/// Push WHERE predicates into subqueries, derived tables, and JOIN conditions.
///
/// This is a standard query optimization that reduces the data processed by
/// inner queries. The pass splits AND-conjunctions and pushes each conjunct
/// as far down the query tree as safety rules allow.
///
/// ## Supported rewrites
///
/// | Pattern | Rewrite |
/// |---|---|
/// | `SELECT … FROM (SELECT … FROM t) AS s WHERE s.x > 5` | Push `x > 5` into derived table WHERE |
/// | `SELECT … FROM a JOIN b ON … WHERE a.x > 5` | Move `a.x > 5` into JOIN ON (inner joins) |
/// | `WITH c AS (SELECT …) SELECT … FROM c WHERE c.x > 5` | Push into CTE body |
///
/// ## Safety
///
/// The pass does **not** push predicates:
/// - Through LIMIT / OFFSET / FETCH FIRST
/// - Through DISTINCT
/// - Through GROUP BY (unless predicate only references grouped columns)
/// - Through window functions in the SELECT list
/// - When a predicate contains non-deterministic functions (RAND, RANDOM, etc.)
/// - When a predicate contains aggregate functions
/// - When a predicate contains subqueries
use std::collections::HashSet;

use crate::ast::*;

// ═══════════════════════════════════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════════════════════════════════

/// Apply predicate pushdown to a statement.
///
/// Returns the statement unchanged if no predicates can be pushed down.
pub fn pushdown_predicates(statement: Statement) -> Statement {
    match statement {
        Statement::Select(sel) => Statement::Select(pushdown_select(sel)),
        other => other,
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Core logic
// ═══════════════════════════════════════════════════════════════════════

fn pushdown_select(mut sel: SelectStatement) -> SelectStatement {
    // First, recursively pushdown in any nested derived tables and CTEs,
    // regardless of whether *this* level has a WHERE clause.
    if let Some(from) = &mut sel.from {
        recurse_into_source(&mut from.source);
    }
    for join in &mut sel.joins {
        recurse_into_source(&mut join.table);
    }
    for cte in &mut sel.ctes {
        *cte.query = pushdown_predicates(*cte.query.clone());
    }

    // Now try to push this level's WHERE predicates down.
    let where_clause = match sel.where_clause.take() {
        Some(w) => w,
        None => return sel,
    };

    let predicates = split_conjunction(where_clause);
    let mut remaining: Vec<Expr> = Vec::new();

    for pred in predicates {
        if !is_pushable(&pred) {
            remaining.push(pred);
            continue;
        }

        let tables = referenced_tables(&pred);

        // Try pushing into FROM derived table
        let mut pushed = false;
        if let Some(from) = &mut sel.from {
            pushed = try_push_into_source(&mut from.source, &pred, &tables);
        }

        // Try pushing into JOIN ON conditions (inner joins only)
        if !pushed {
            for join in &mut sel.joins {
                if try_push_into_join(join, &pred, &tables) {
                    pushed = true;
                    break;
                }
            }
        }

        if !pushed {
            remaining.push(pred);
        }
    }

    sel.where_clause = conjoin(remaining);
    sel
}

// ═══════════════════════════════════════════════════════════════════════
// Push into derived tables (subqueries in FROM)
// ═══════════════════════════════════════════════════════════════════════

/// Try to push a predicate into a table source. Returns true if pushed.
fn try_push_into_source(source: &mut TableSource, pred: &Expr, tables: &HashSet<String>) -> bool {
    match source {
        TableSource::Subquery { query, alias, .. } => {
            let alias_name = match alias {
                Some(a) => a.clone(),
                None => return false,
            };

            // Predicate must reference only this derived table alias
            if tables.is_empty() || !tables.iter().all(|t| t == &alias_name) {
                return false;
            }

            // Check the inner query is a simple SELECT we can push into
            let inner_sel = match query.as_mut() {
                Statement::Select(sel) => sel,
                _ => return false,
            };

            if !is_pushdown_safe_target(inner_sel) {
                return false;
            }

            // Rewrite column references: strip the outer alias qualifier
            // and map to inner column names using the SELECT list.
            let rewritten = rewrite_predicate_for_derived_table(pred, &alias_name, inner_sel);
            let rewritten = match rewritten {
                Some(r) => r,
                None => return false,
            };

            // Push into the inner WHERE
            inner_sel.where_clause = match inner_sel.where_clause.take() {
                Some(existing) => Some(Expr::BinaryOp {
                    left: Box::new(existing),
                    op: BinaryOperator::And,
                    right: Box::new(rewritten),
                }),
                None => Some(rewritten),
            };

            true
        }
        _ => false,
    }
}

/// Try to push a predicate into a JOIN's ON condition.
/// Only safe for INNER and CROSS joins.
fn try_push_into_join(join: &mut JoinClause, pred: &Expr, tables: &HashSet<String>) -> bool {
    // Only push into inner joins — pushing into LEFT/RIGHT/FULL
    // changes semantics.
    if !matches!(join.join_type, JoinType::Inner | JoinType::Cross) {
        return false;
    }

    // Get the table name/alias of this join's source
    let join_table = source_alias(&join.table);
    let join_table = match join_table {
        Some(t) => t,
        None => return false,
    };

    // Predicate must reference only the join's table
    if tables.is_empty() || tables.len() != 1 || !tables.contains(&join_table) {
        return false;
    }

    // Also try pushing into a derived-table join source
    if matches!(join.table, TableSource::Subquery { .. })
        && try_push_into_source(&mut join.table, pred, tables)
    {
        return true;
    }

    // Push into the ON condition
    join.on = match join.on.take() {
        Some(existing) => Some(Expr::BinaryOp {
            left: Box::new(existing),
            op: BinaryOperator::And,
            right: Box::new(pred.clone()),
        }),
        None => Some(pred.clone()),
    };

    true
}

// ═══════════════════════════════════════════════════════════════════════
// Recursion into nested structures
// ═══════════════════════════════════════════════════════════════════════

/// Recurse into a table source to pushdown predicates in nested derived tables.
fn recurse_into_source(source: &mut TableSource) {
    match source {
        TableSource::Subquery { query, .. } => {
            *query = Box::new(pushdown_predicates(*query.clone()));
        }
        TableSource::Lateral { source } => {
            recurse_into_source(source);
        }
        TableSource::Pivot { source, .. } | TableSource::Unpivot { source, .. } => {
            recurse_into_source(source);
        }
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Predicate analysis helpers
// ═══════════════════════════════════════════════════════════════════════

/// Split an expression on AND into a flat list of conjuncts.
fn split_conjunction(expr: Expr) -> Vec<Expr> {
    match expr {
        Expr::BinaryOp {
            left,
            op: BinaryOperator::And,
            right,
        } => {
            let mut result = split_conjunction(*left);
            result.extend(split_conjunction(*right));
            result
        }
        Expr::Nested(inner) => {
            // Only unwrap if the nested expression is itself an AND
            if matches!(
                inner.as_ref(),
                Expr::BinaryOp {
                    op: BinaryOperator::And,
                    ..
                }
            ) {
                split_conjunction(*inner)
            } else {
                vec![Expr::Nested(inner)]
            }
        }
        other => vec![other],
    }
}

/// Rejoin a list of predicates with AND. Returns None if empty.
fn conjoin(predicates: Vec<Expr>) -> Option<Expr> {
    predicates.into_iter().reduce(|a, b| Expr::BinaryOp {
        left: Box::new(a),
        op: BinaryOperator::And,
        right: Box::new(b),
    })
}

/// Collect all table qualifiers referenced by column expressions.
fn referenced_tables(expr: &Expr) -> HashSet<String> {
    let mut tables = HashSet::new();
    expr.walk(&mut |e| {
        if let Expr::Column { table: Some(t), .. } = e {
            tables.insert(t.clone());
        }
        true
    });
    tables
}

/// Check whether a predicate is safe to push down.
///
/// Returns false for predicates containing:
/// - Aggregate functions
/// - Window functions
/// - Non-deterministic functions
/// - Subqueries
fn is_pushable(expr: &Expr) -> bool {
    let mut safe = true;
    expr.walk(&mut |e| {
        if !safe {
            return false;
        }
        match e {
            // Subqueries should not be pushed
            Expr::Subquery(_) | Expr::Exists { .. } | Expr::InSubquery { .. } => {
                safe = false;
                false
            }
            // Aggregate functions
            Expr::Function { name, .. } if is_aggregate_function(name) => {
                safe = false;
                false
            }
            // Window functions (have OVER clause)
            Expr::Function { over: Some(_), .. } | Expr::TypedFunction { over: Some(_), .. } => {
                safe = false;
                false
            }
            // Non-deterministic functions
            Expr::Function { name, .. } if is_nondeterministic(name) => {
                safe = false;
                false
            }
            Expr::TypedFunction {
                func: TypedFunction::CurrentTimestamp,
                ..
            } => {
                safe = false;
                false
            }
            _ => true,
        }
    });
    safe
}

/// Check whether the target SELECT is safe for predicate pushdown.
///
/// We cannot push through LIMIT, OFFSET, DISTINCT, or window functions.
fn is_pushdown_safe_target(sel: &SelectStatement) -> bool {
    if sel.limit.is_some() || sel.offset.is_some() || sel.fetch_first.is_some() || sel.distinct {
        return false;
    }
    // Check for window functions in SELECT list
    for item in &sel.columns {
        if let SelectItem::Expr { expr, .. } = item {
            if contains_window_function(expr) {
                return false;
            }
        }
    }
    true
}

/// Check whether an expression contains a window function.
fn contains_window_function(expr: &Expr) -> bool {
    let mut has_window = false;
    expr.walk(&mut |e| {
        if has_window {
            return false;
        }
        match e {
            Expr::Function { over: Some(_), .. } | Expr::TypedFunction { over: Some(_), .. } => {
                has_window = true;
                false
            }
            _ => true,
        }
    });
    has_window
}

fn is_aggregate_function(name: &str) -> bool {
    matches!(
        name.to_uppercase().as_str(),
        "COUNT"
            | "SUM"
            | "AVG"
            | "MIN"
            | "MAX"
            | "GROUP_CONCAT"
            | "STRING_AGG"
            | "ARRAY_AGG"
            | "LISTAGG"
            | "STDDEV"
            | "STDDEV_POP"
            | "STDDEV_SAMP"
            | "VARIANCE"
            | "VAR_POP"
            | "VAR_SAMP"
            | "EVERY"
            | "ANY_VALUE"
            | "SOME"
            | "BIT_AND"
            | "BIT_OR"
            | "BIT_XOR"
            | "BOOL_AND"
            | "BOOL_OR"
            | "CORR"
            | "COVAR_POP"
            | "COVAR_SAMP"
            | "REGR_SLOPE"
            | "REGR_INTERCEPT"
            | "PERCENTILE_CONT"
            | "PERCENTILE_DISC"
            | "APPROX_COUNT_DISTINCT"
            | "HLL_COUNT"
            | "APPROX_DISTINCT"
    )
}

fn is_nondeterministic(name: &str) -> bool {
    matches!(
        name.to_uppercase().as_str(),
        "RAND" | "RANDOM" | "UUID" | "NEWID" | "GEN_RANDOM_UUID" | "SYSDATE" | "SYSTIMESTAMP"
    )
}

// ═══════════════════════════════════════════════════════════════════════
// Column remapping for derived-table pushdown
// ═══════════════════════════════════════════════════════════════════════

/// Rewrite a predicate so its column references match the inner derived
/// table's namespace. For example, given:
///
/// ```sql
/// SELECT * FROM (SELECT id, name FROM t) AS sub WHERE sub.name = 'foo'
/// ```
///
/// The predicate `sub.name = 'foo'` becomes `name = 'foo'` (or `t.name = 'foo'`
/// if the inner query qualifies columns).
///
/// Returns `None` if the rewrite cannot be performed (e.g., the column
/// isn't exposed by the derived table's SELECT list).
fn rewrite_predicate_for_derived_table(
    pred: &Expr,
    outer_alias: &str,
    inner_sel: &SelectStatement,
) -> Option<Expr> {
    // Build a mapping: outer column name → inner expression
    let column_map = build_column_map(inner_sel);

    // Check that all columns referenced in the predicate can be mapped
    let mut can_rewrite = true;
    pred.walk(&mut |e| {
        if !can_rewrite {
            return false;
        }
        if let Expr::Column {
            table: Some(t),
            name,
            ..
        } = e
        {
            if t == outer_alias && !column_map.contains_key(name.as_str()) {
                can_rewrite = false;
            }
        }
        can_rewrite
    });

    if !can_rewrite {
        return None;
    }

    // If inner SELECT has GROUP BY, only allow pushing predicates that
    // reference grouped columns (pre-aggregation filters).
    if !inner_sel.group_by.is_empty() {
        let grouped_names: HashSet<String> = inner_sel
            .group_by
            .iter()
            .filter_map(|e| match e {
                Expr::Column { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect();

        let mut all_grouped = true;
        pred.walk(&mut |e| {
            if !all_grouped {
                return false;
            }
            if let Expr::Column {
                table: Some(t),
                name,
                ..
            } = e
            {
                if t == outer_alias {
                    if let Some(inner_expr) = column_map.get(name.as_str()) {
                        let inner_name = match inner_expr {
                            Expr::Column { name: n, .. } => n.clone(),
                            _ => name.clone(),
                        };
                        if !grouped_names.contains(&inner_name) {
                            all_grouped = false;
                        }
                    }
                }
            }
            all_grouped
        });

        if !all_grouped {
            return None;
        }
    }

    // Perform the rewrite
    let rewritten = pred.clone().transform(&|e| match e {
        Expr::Column {
            table: Some(ref t),
            ref name,
            ..
        } if t == outer_alias => {
            if let Some(inner_expr) = column_map.get(name.as_str()) {
                inner_expr.clone()
            } else {
                e
            }
        }
        other => other,
    });

    Some(rewritten)
}

/// Build a mapping from output column name → inner expression for a SELECT.
///
/// For `SELECT id, name AS n, x + 1 AS calc FROM t`:
/// - "id" → Column { name: "id", ... }
/// - "n" → Column { name: "name", ... }
/// - "calc" → BinaryOp(x + 1)
fn build_column_map(sel: &SelectStatement) -> std::collections::HashMap<&str, Expr> {
    let mut map = std::collections::HashMap::new();

    for item in &sel.columns {
        match item {
            SelectItem::Expr {
                expr:
                    Expr::Column {
                        name,
                        table,
                        quote_style,
                        table_quote_style,
                    },
                alias,
                ..
            } => {
                let output_name = alias.as_deref().unwrap_or(name.as_str());
                map.insert(
                    output_name,
                    Expr::Column {
                        table: table.clone(),
                        name: name.clone(),
                        quote_style: *quote_style,
                        table_quote_style: *table_quote_style,
                    },
                );
            }
            SelectItem::Expr { expr, alias, .. } => {
                if let Some(alias) = alias {
                    map.insert(alias.as_str(), expr.clone());
                }
            }
            SelectItem::Wildcard | SelectItem::QualifiedWildcard { .. } => {
                // With *, we can't easily build a column map without schema
                // info. Bail  — the predicate columns just need to not
                // have a table qualifier to match.
            }
        }
    }

    map
}

/// Get the alias or table name for a table source.
fn source_alias(source: &TableSource) -> Option<String> {
    match source {
        TableSource::Table(t) => Some(t.alias.clone().unwrap_or_else(|| t.name.clone())),
        TableSource::Subquery { alias, .. } => alias.clone(),
        TableSource::TableFunction { alias, .. } => alias.clone(),
        TableSource::Unnest { alias, .. } => alias.clone(),
        TableSource::Lateral { source } => source_alias(source),
        TableSource::Pivot { alias, .. } | TableSource::Unpivot { alias, .. } => alias.clone(),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_conjunction_single() {
        let expr = Expr::Boolean(true);
        let parts = split_conjunction(expr);
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_split_conjunction_and() {
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Boolean(true)),
            op: BinaryOperator::And,
            right: Box::new(Expr::Boolean(false)),
        };
        let parts = split_conjunction(expr);
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn test_split_conjunction_nested_and() {
        // (a AND b) AND c
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Column {
                    table: None,
                    name: "a".into(),
                    quote_style: QuoteStyle::None,
                    table_quote_style: QuoteStyle::None,
                }),
                op: BinaryOperator::And,
                right: Box::new(Expr::Column {
                    table: None,
                    name: "b".into(),
                    quote_style: QuoteStyle::None,
                    table_quote_style: QuoteStyle::None,
                }),
            }),
            op: BinaryOperator::And,
            right: Box::new(Expr::Column {
                table: None,
                name: "c".into(),
                quote_style: QuoteStyle::None,
                table_quote_style: QuoteStyle::None,
            }),
        };
        let parts = split_conjunction(expr);
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_conjoin_empty() {
        assert!(conjoin(vec![]).is_none());
    }

    #[test]
    fn test_conjoin_single() {
        let r = conjoin(vec![Expr::Boolean(true)]);
        assert_eq!(r, Some(Expr::Boolean(true)));
    }

    #[test]
    fn test_is_pushable_simple_comparison() {
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Column {
                table: Some("t".into()),
                name: "x".into(),
                quote_style: QuoteStyle::None,
                table_quote_style: QuoteStyle::None,
            }),
            op: BinaryOperator::Gt,
            right: Box::new(Expr::Number("5".into())),
        };
        assert!(is_pushable(&expr));
    }

    #[test]
    fn test_is_pushable_rejects_aggregate() {
        let expr = Expr::Function {
            name: "COUNT".into(),
            args: vec![Expr::Star],
            distinct: false,
            filter: None,
            over: None,
            order_by: vec![],
            within_group: false,
        };
        assert!(!is_pushable(&expr));
    }

    #[test]
    fn test_is_pushable_rejects_window() {
        let expr = Expr::Function {
            name: "ROW_NUMBER".into(),
            args: vec![],
            distinct: false,
            filter: None,
            over: Some(WindowSpec {
                window_ref: None,
                partition_by: vec![],
                order_by: vec![],
                frame: None,
            }),
            order_by: vec![],
            within_group: false,
        };
        assert!(!is_pushable(&expr));
    }

    #[test]
    fn test_is_pushable_rejects_subquery() {
        let expr = Expr::Exists {
            subquery: Box::new(Statement::Select(SelectStatement {
                comments: vec![],
                ctes: vec![],
                distinct: false,
                top: None,
                columns: vec![],
                from: None,
                joins: vec![],
                where_clause: None,
                group_by: vec![],
                having: None,
                order_by: vec![],
                limit: None,
                offset: None,
                fetch_first: None,
                qualify: None,
                window_definitions: vec![],
                query_options: None,
            })),
            negated: false,
        };
        assert!(!is_pushable(&expr));
    }

    #[test]
    fn test_referenced_tables() {
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Column {
                table: Some("a".into()),
                name: "x".into(),
                quote_style: QuoteStyle::None,
                table_quote_style: QuoteStyle::None,
            }),
            op: BinaryOperator::Eq,
            right: Box::new(Expr::Column {
                table: Some("b".into()),
                name: "y".into(),
                quote_style: QuoteStyle::None,
                table_quote_style: QuoteStyle::None,
            }),
        };
        let tables = referenced_tables(&expr);
        assert_eq!(tables.len(), 2);
        assert!(tables.contains("a"));
        assert!(tables.contains("b"));
    }
}
