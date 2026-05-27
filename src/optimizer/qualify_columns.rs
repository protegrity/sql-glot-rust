//! Qualify columns optimizer pass.
//!
//! Resolves column references by:
//! - Expanding `SELECT *` to explicit column lists from schema
//! - Expanding `SELECT t.*` to columns from table `t`
//! - Adding table qualifiers to unqualified columns (e.g., `col` → `t.col`)
//! - Validating column existence against the schema
//! - Resolving columns through CTEs and derived tables

use std::collections::HashMap;

use crate::ast::*;
use crate::dialects::Dialect;
use crate::schema::{Schema, normalize_identifier};

/// Qualify columns in a statement using the provided schema.
///
/// This adds table qualifiers to unqualified column references and expands
/// wildcard selects (`*`, `t.*`) into explicit column lists.
pub fn qualify_columns<S: Schema>(statement: Statement, schema: &S) -> Statement {
    let dialect = schema.dialect();
    match statement {
        Statement::Select(sel) => {
            let qualified = qualify_select(sel, schema, dialect, &HashMap::new());
            Statement::Select(qualified)
        }
        Statement::SetOperation(mut set_op) => {
            set_op.left = Box::new(qualify_columns(*set_op.left, schema));
            set_op.right = Box::new(qualify_columns(*set_op.right, schema));
            Statement::SetOperation(set_op)
        }
        other => other,
    }
}

/// Metadata about columns available from a source (table, derived table, CTE).
#[derive(Debug, Clone)]
struct SourceColumns {
    /// Column names in definition order.
    columns: Vec<String>,
}

/// Build a mapping of source name/alias → available columns for a SELECT scope.
fn resolve_source_columns<S: Schema>(
    sel: &SelectStatement,
    schema: &S,
    dialect: Dialect,
    cte_columns: &HashMap<String, Vec<String>>,
) -> HashMap<String, SourceColumns> {
    let mut source_map: HashMap<String, SourceColumns> = HashMap::new();

    // Process FROM source
    if let Some(from) = &sel.from {
        collect_source_columns(&from.source, schema, dialect, cte_columns, &mut source_map);
    }

    // Process JOINs
    for join in &sel.joins {
        collect_source_columns(&join.table, schema, dialect, cte_columns, &mut source_map);
    }

    source_map
}

/// Collect columns from a single table source.
fn collect_source_columns<S: Schema>(
    source: &TableSource,
    schema: &S,
    dialect: Dialect,
    cte_columns: &HashMap<String, Vec<String>>,
    source_map: &mut HashMap<String, SourceColumns>,
) {
    match source {
        TableSource::Table(table_ref) => {
            let key = table_ref
                .alias
                .as_deref()
                .unwrap_or(&table_ref.name)
                .to_string();
            let norm_key = normalize_identifier(&key, dialect);

            // Check if this is a CTE reference
            let norm_name = normalize_identifier(&table_ref.name, dialect);
            if let Some(cols) = cte_columns.get(&norm_name) {
                source_map.insert(
                    norm_key,
                    SourceColumns {
                        columns: cols.clone(),
                    },
                );
                return;
            }

            // Build the table path for schema lookup
            let path = build_table_path(table_ref, dialect);
            let path_refs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();

            if let Ok(cols) = schema.column_names(&path_refs) {
                source_map.insert(norm_key, SourceColumns { columns: cols });
            }
        }
        TableSource::Subquery { query, alias, .. } => {
            if let Some(alias) = alias {
                let norm_alias = normalize_identifier(alias, dialect);
                let cols = extract_output_columns(query, schema, dialect, cte_columns);
                source_map.insert(norm_alias, SourceColumns { columns: cols });
            }
        }
        TableSource::Lateral { source: inner } => {
            collect_source_columns(inner, schema, dialect, cte_columns, source_map);
        }
        TableSource::Pivot { source, alias, .. } | TableSource::Unpivot { source, alias, .. } => {
            collect_source_columns(source, schema, dialect, cte_columns, source_map);
            if let Some(alias) = alias {
                let norm_alias = normalize_identifier(alias, dialect);
                source_map.insert(norm_alias, SourceColumns { columns: vec![] });
            }
        }
        TableSource::Unnest { alias, .. } => {
            if let Some(alias) = alias {
                let norm_alias = normalize_identifier(alias, dialect);
                // Unnest typically produces unnamed columns; skip
                source_map.insert(norm_alias, SourceColumns { columns: vec![] });
            }
        }
        TableSource::TableFunction { alias, .. } => {
            if let Some(alias) = alias {
                let norm_alias = normalize_identifier(alias, dialect);
                source_map.insert(norm_alias, SourceColumns { columns: vec![] });
            }
        }
    }
}

/// Build a normalized table path for schema lookup.
fn build_table_path(table_ref: &TableRef, dialect: Dialect) -> Vec<String> {
    let mut path = Vec::new();
    if let Some(cat) = &table_ref.catalog {
        path.push(normalize_identifier(cat, dialect));
    }
    if let Some(sch) = &table_ref.schema {
        path.push(normalize_identifier(sch, dialect));
    }
    path.push(normalize_identifier(&table_ref.name, dialect));
    path
}

/// Extract output column names from a subquery statement.
fn extract_output_columns<S: Schema>(
    stmt: &Statement,
    schema: &S,
    dialect: Dialect,
    cte_columns: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    match stmt {
        Statement::Select(sel) => {
            let inner_sources = resolve_source_columns(sel, schema, dialect, cte_columns);
            let mut cols = Vec::new();
            for item in &sel.columns {
                match item {
                    SelectItem::Wildcard => {
                        // Expand * from all sources (in definition order)
                        for_each_source_ordered(sel, dialect, &inner_sources, |sc| {
                            cols.extend(sc.columns.iter().cloned());
                        });
                    }
                    SelectItem::QualifiedWildcard { table } => {
                        let norm_table = normalize_identifier(table, dialect);
                        if let Some(sc) = inner_sources.get(&norm_table) {
                            cols.extend(sc.columns.iter().cloned());
                        }
                    }
                    SelectItem::Expr { alias, expr, .. } => {
                        if let Some(alias) = alias {
                            cols.push(alias.clone());
                        } else {
                            cols.push(expr_output_name(expr));
                        }
                    }
                }
            }
            cols
        }
        Statement::SetOperation(set_op) => {
            // Output columns come from the left branch
            extract_output_columns(&set_op.left, schema, dialect, cte_columns)
        }
        _ => vec![],
    }
}

/// Get the output name of an expression (column name, function name, or a placeholder).
fn expr_output_name(expr: &Expr) -> String {
    match expr {
        Expr::Column { name, .. } => name.clone(),
        Expr::Function { name, .. } => name.clone(),
        Expr::TypedFunction { .. } => "_col".to_string(),
        _ => "_col".to_string(),
    }
}

/// Iterate source columns in FROM/JOIN order for deterministic wildcard expansion.
fn for_each_source_ordered<F>(
    sel: &SelectStatement,
    dialect: Dialect,
    source_map: &HashMap<String, SourceColumns>,
    mut callback: F,
) where
    F: FnMut(&SourceColumns),
{
    // FROM source first
    if let Some(from) = &sel.from {
        let key = source_key_for(&from.source, dialect);
        if let Some(sc) = source_map.get(&key) {
            callback(sc);
        }
    }
    // Then JOINs in order
    for join in &sel.joins {
        let key = source_key_for(&join.table, dialect);
        if let Some(sc) = source_map.get(&key) {
            callback(sc);
        }
    }
}

/// Get the source key (alias or name) for a table source.
fn source_key_for(source: &TableSource, dialect: Dialect) -> String {
    match source {
        TableSource::Table(tr) => {
            let name = tr.alias.as_deref().unwrap_or(&tr.name);
            normalize_identifier(name, dialect)
        }
        TableSource::Subquery { alias, .. } => alias
            .as_deref()
            .map(|a| normalize_identifier(a, dialect))
            .unwrap_or_default(),
        TableSource::Lateral { source } => source_key_for(source, dialect),
        TableSource::Pivot { source, alias, .. } | TableSource::Unpivot { source, alias, .. } => {
            if let Some(a) = alias {
                normalize_identifier(a, dialect)
            } else {
                source_key_for(source, dialect)
            }
        }
        TableSource::Unnest { alias, .. } | TableSource::TableFunction { alias, .. } => alias
            .as_deref()
            .map(|a| normalize_identifier(a, dialect))
            .unwrap_or_default(),
    }
}

/// Qualify a SELECT statement: expand wildcards and qualify column references.
fn qualify_select<S: Schema>(
    mut sel: SelectStatement,
    schema: &S,
    dialect: Dialect,
    outer_cte_columns: &HashMap<String, Vec<String>>,
) -> SelectStatement {
    // ── 1. Build CTE column map (CTEs defined in this SELECT + inherited) ──
    let mut cte_columns = outer_cte_columns.clone();
    for cte in &sel.ctes {
        let cols = if !cte.columns.is_empty() {
            // Explicit CTE column list: WITH cte(a, b) AS (...)
            cte.columns.clone()
        } else {
            extract_output_columns(&cte.query, schema, dialect, &cte_columns)
        };
        let norm_name = normalize_identifier(&cte.name, dialect);
        cte_columns.insert(norm_name, cols);
    }

    // ── 2. Recursively qualify CTE bodies ────────────────────────────
    sel.ctes = sel
        .ctes
        .into_iter()
        .map(|mut cte| {
            cte.query = Box::new(qualify_columns(*cte.query, schema));
            cte
        })
        .collect();

    // ── 3. Recursively qualify derived tables and join subqueries ─────
    if let Some(ref mut from) = sel.from {
        qualify_table_source(&mut from.source, schema, dialect, &cte_columns);
    }
    for join in &mut sel.joins {
        qualify_table_source(&mut join.table, schema, dialect, &cte_columns);
    }

    // ── 4. Resolve source columns for this scope ─────────────────────
    let source_map = resolve_source_columns(&sel, schema, dialect, &cte_columns);

    // ── 5. Expand wildcards in SELECT list ────────────────────────────
    let mut new_columns = Vec::new();
    let old_columns = std::mem::take(&mut sel.columns);
    for item in old_columns {
        match item {
            SelectItem::Wildcard => {
                // Expand to all columns from all sources in order
                for_each_source_ordered(&sel, dialect, &source_map, |sc| {
                    for col_name in &sc.columns {
                        new_columns.push(SelectItem::Expr {
                            expr: Expr::Column {
                                table: None,
                                name: col_name.clone(),
                                quote_style: QuoteStyle::None,
                                table_quote_style: QuoteStyle::None,
                            },
                            alias: None,
                            alias_quote_style: QuoteStyle::None,
                        });
                    }
                });
            }
            SelectItem::QualifiedWildcard { table } => {
                let norm_table = normalize_identifier(&table, dialect);
                if let Some(sc) = source_map.get(&norm_table) {
                    for col_name in &sc.columns {
                        new_columns.push(SelectItem::Expr {
                            expr: Expr::Column {
                                table: Some(table.clone()),
                                name: col_name.clone(),
                                quote_style: QuoteStyle::None,
                                table_quote_style: QuoteStyle::None,
                            },
                            alias: None,
                            alias_quote_style: QuoteStyle::None,
                        });
                    }
                } else {
                    // Unknown table — preserve as-is
                    new_columns.push(SelectItem::QualifiedWildcard { table });
                }
            }
            SelectItem::Expr {
                expr,
                alias,
                alias_quote_style,
                ..
            } => {
                let qualified_expr = qualify_expr(expr, &source_map, schema, dialect, &cte_columns);
                new_columns.push(SelectItem::Expr {
                    expr: qualified_expr,
                    alias,
                    alias_quote_style,
                });
            }
        }
    }
    sel.columns = new_columns;

    // ── 6. Qualify expressions in WHERE, GROUP BY, HAVING, ORDER BY ──
    if let Some(wh) = sel.where_clause {
        sel.where_clause = Some(qualify_expr(wh, &source_map, schema, dialect, &cte_columns));
    }
    sel.group_by = sel
        .group_by
        .into_iter()
        .map(|e| qualify_expr(e, &source_map, schema, dialect, &cte_columns))
        .collect();
    if let Some(having) = sel.having {
        sel.having = Some(qualify_expr(
            having,
            &source_map,
            schema,
            dialect,
            &cte_columns,
        ));
    }
    sel.order_by = sel
        .order_by
        .into_iter()
        .map(|mut item| {
            item.expr = qualify_expr(item.expr, &source_map, schema, dialect, &cte_columns);
            item
        })
        .collect();
    if let Some(qualify) = sel.qualify {
        sel.qualify = Some(qualify_expr(
            qualify,
            &source_map,
            schema,
            dialect,
            &cte_columns,
        ));
    }

    // ── 7. Qualify JOIN ON expressions ────────────────────────────────
    for join in &mut sel.joins {
        if let Some(on) = join.on.take() {
            join.on = Some(qualify_expr(on, &source_map, schema, dialect, &cte_columns));
        }
    }

    sel
}

/// Recursively qualify columns inside subquery table sources.
fn qualify_table_source<S: Schema>(
    source: &mut TableSource,
    schema: &S,
    dialect: Dialect,
    cte_columns: &HashMap<String, Vec<String>>,
) {
    match source {
        TableSource::Subquery { query, .. } => {
            *query = Box::new(qualify_columns_inner(
                *query.clone(),
                schema,
                dialect,
                cte_columns,
            ));
        }
        TableSource::Lateral { source: inner } => {
            qualify_table_source(inner, schema, dialect, cte_columns);
        }
        TableSource::Pivot { source, .. } | TableSource::Unpivot { source, .. } => {
            qualify_table_source(source, schema, dialect, cte_columns);
        }
        _ => {}
    }
}

/// Inner qualify entry point that passes CTE context.
fn qualify_columns_inner<S: Schema>(
    statement: Statement,
    schema: &S,
    dialect: Dialect,
    cte_columns: &HashMap<String, Vec<String>>,
) -> Statement {
    match statement {
        Statement::Select(sel) => {
            Statement::Select(qualify_select(sel, schema, dialect, cte_columns))
        }
        Statement::SetOperation(mut set_op) => {
            set_op.left = Box::new(qualify_columns_inner(
                *set_op.left,
                schema,
                dialect,
                cte_columns,
            ));
            set_op.right = Box::new(qualify_columns_inner(
                *set_op.right,
                schema,
                dialect,
                cte_columns,
            ));
            Statement::SetOperation(set_op)
        }
        other => other,
    }
}

/// Qualify column references in an expression by adding table qualifiers.
/// Also recursively qualifies any subqueries found inside the expression.
fn qualify_expr<S: Schema>(
    expr: Expr,
    source_map: &HashMap<String, SourceColumns>,
    schema: &S,
    dialect: Dialect,
    cte_columns: &HashMap<String, Vec<String>>,
) -> Expr {
    expr.transform(&|e| match e {
        Expr::Column {
            table: None,
            name,
            quote_style,
            table_quote_style,
        } => {
            let norm_name = normalize_identifier(&name, dialect);
            // Find which source contains this column
            let resolved_source = resolve_column(&norm_name, source_map);
            if let Some(source_name) = resolved_source {
                Expr::Column {
                    table: Some(source_name),
                    name,
                    quote_style,
                    table_quote_style,
                }
            } else {
                // Column not found in any source — leave unqualified
                // (could be an alias reference, positional, etc.)
                Expr::Column {
                    table: None,
                    name,
                    quote_style,
                    table_quote_style,
                }
            }
        }
        // Recursively qualify subqueries inside expressions
        Expr::InSubquery {
            expr,
            subquery,
            negated,
        } => Expr::InSubquery {
            expr,
            subquery: Box::new(qualify_columns_inner(
                *subquery,
                schema,
                dialect,
                cte_columns,
            )),
            negated,
        },
        Expr::Subquery(stmt) => Expr::Subquery(Box::new(qualify_columns_inner(
            *stmt,
            schema,
            dialect,
            cte_columns,
        ))),
        Expr::Exists { subquery, negated } => Expr::Exists {
            subquery: Box::new(qualify_columns_inner(
                *subquery,
                schema,
                dialect,
                cte_columns,
            )),
            negated,
        },
        other => other,
    })
}

/// Find which source owns a column name.
/// If exactly one source has it, return that source's name.
/// If multiple sources have it or none do, return None.
fn resolve_column(
    norm_col_name: &str,
    source_map: &HashMap<String, SourceColumns>,
) -> Option<String> {
    let mut matches: Vec<&str> = Vec::new();
    for (source_name, sc) in source_map {
        if sc
            .columns
            .iter()
            .any(|c| c.eq_ignore_ascii_case(norm_col_name))
        {
            matches.push(source_name);
        }
    }
    if matches.len() == 1 {
        Some(matches[0].to_string())
    } else {
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::generate;
    use crate::parser::parse;
    use crate::schema::MappingSchema;

    fn make_schema() -> MappingSchema {
        let mut schema = MappingSchema::new(Dialect::Ansi);
        schema
            .add_table(
                &["users"],
                vec![
                    ("id".to_string(), DataType::Int),
                    ("name".to_string(), DataType::Varchar(Some(255))),
                    ("email".to_string(), DataType::Text),
                ],
            )
            .unwrap();
        schema
            .add_table(
                &["orders"],
                vec![
                    ("id".to_string(), DataType::Int),
                    ("user_id".to_string(), DataType::Int),
                    (
                        "amount".to_string(),
                        DataType::Decimal {
                            precision: Some(10),
                            scale: Some(2),
                        },
                    ),
                    ("status".to_string(), DataType::Varchar(Some(50))),
                ],
            )
            .unwrap();
        schema
            .add_table(
                &["products"],
                vec![
                    ("id".to_string(), DataType::Int),
                    ("name".to_string(), DataType::Varchar(Some(255))),
                    (
                        "price".to_string(),
                        DataType::Decimal {
                            precision: Some(10),
                            scale: Some(2),
                        },
                    ),
                ],
            )
            .unwrap();
        schema
    }

    fn qualify(sql: &str, schema: &MappingSchema) -> String {
        let stmt = parse(sql, Dialect::Ansi).unwrap();
        let qualified = qualify_columns(stmt, schema);
        generate(&qualified, Dialect::Ansi)
    }

    #[test]
    fn test_expand_star() {
        let schema = make_schema();
        assert_eq!(
            qualify("SELECT * FROM users", &schema),
            "SELECT id, name, email FROM users"
        );
    }

    #[test]
    fn test_expand_qualified_wildcard() {
        let schema = make_schema();
        assert_eq!(
            qualify("SELECT users.* FROM users", &schema),
            "SELECT users.id, users.name, users.email FROM users"
        );
    }

    #[test]
    fn test_expand_star_with_alias() {
        let schema = make_schema();
        assert_eq!(
            qualify("SELECT * FROM users AS u", &schema),
            "SELECT id, name, email FROM users AS u"
        );
    }

    #[test]
    fn test_expand_qualified_wildcard_alias() {
        let schema = make_schema();
        assert_eq!(
            qualify("SELECT u.* FROM users AS u", &schema),
            "SELECT u.id, u.name, u.email FROM users AS u"
        );
    }

    #[test]
    fn test_qualify_unqualified_single_table() {
        let schema = make_schema();
        assert_eq!(
            qualify("SELECT id, name FROM users", &schema),
            "SELECT users.id, users.name FROM users"
        );
    }

    #[test]
    fn test_qualify_unqualified_single_table_alias() {
        let schema = make_schema();
        assert_eq!(
            qualify("SELECT id, name FROM users AS u", &schema),
            "SELECT u.id, u.name FROM users AS u"
        );
    }

    #[test]
    fn test_qualify_already_qualified() {
        let schema = make_schema();
        assert_eq!(
            qualify("SELECT users.id, users.name FROM users", &schema),
            "SELECT users.id, users.name FROM users"
        );
    }

    #[test]
    fn test_qualify_join_unambiguous() {
        let schema = make_schema();
        assert_eq!(
            qualify(
                "SELECT name, amount FROM users JOIN orders ON users.id = orders.user_id",
                &schema
            ),
            "SELECT users.name, orders.amount FROM users INNER JOIN orders ON users.id = orders.user_id"
        );
    }

    #[test]
    fn test_qualify_join_ambiguous_left_unqualified() {
        // 'id' exists in both users and orders — should remain unqualified
        let schema = make_schema();
        let result = qualify(
            "SELECT id FROM users JOIN orders ON users.id = orders.user_id",
            &schema,
        );
        // Ambiguous — stays unqualified
        assert_eq!(
            result,
            "SELECT id FROM users INNER JOIN orders ON users.id = orders.user_id"
        );
    }

    #[test]
    fn test_qualify_where_clause() {
        let schema = make_schema();
        assert_eq!(
            qualify(
                "SELECT name FROM users WHERE email = 'test@test.com'",
                &schema
            ),
            "SELECT users.name FROM users WHERE users.email = 'test@test.com'"
        );
    }

    #[test]
    fn test_qualify_order_by() {
        let schema = make_schema();
        assert_eq!(
            qualify("SELECT name FROM users ORDER BY email", &schema),
            "SELECT users.name FROM users ORDER BY users.email"
        );
    }

    #[test]
    fn test_qualify_group_by_having() {
        let schema = make_schema();
        assert_eq!(
            qualify(
                "SELECT status, COUNT(*) FROM orders GROUP BY status HAVING COUNT(*) > 1",
                &schema
            ),
            "SELECT orders.status, COUNT(*) FROM orders GROUP BY orders.status HAVING COUNT(*) > 1"
        );
    }

    #[test]
    fn test_expand_star_join() {
        let schema = make_schema();
        let result = qualify(
            "SELECT * FROM users JOIN orders ON users.id = orders.user_id",
            &schema,
        );
        assert_eq!(
            result,
            "SELECT id, name, email, id, user_id, amount, status FROM users INNER JOIN orders ON users.id = orders.user_id"
        );
    }

    #[test]
    fn test_cte_column_resolution() {
        let schema = make_schema();
        let result = qualify(
            "WITH active AS (SELECT id, name FROM users) SELECT id, name FROM active",
            &schema,
        );
        assert_eq!(
            result,
            "WITH active AS (SELECT users.id, users.name FROM users) SELECT active.id, active.name FROM active"
        );
    }

    #[test]
    fn test_derived_table_column_resolution() {
        let schema = make_schema();
        let result = qualify(
            "SELECT id FROM (SELECT id, name FROM users) AS sub",
            &schema,
        );
        assert_eq!(
            result,
            "SELECT sub.id FROM (SELECT users.id, users.name FROM users) AS sub"
        );
    }

    #[test]
    fn test_preserve_expression_aliases() {
        let schema = make_schema();
        assert_eq!(
            qualify("SELECT name AS user_name FROM users", &schema),
            "SELECT users.name AS user_name FROM users"
        );
    }

    #[test]
    fn test_qualify_join_on() {
        let schema = make_schema();
        // 'id' is ambiguous (in both users and orders) so stays unqualified
        // 'user_id' is unique to orders so gets qualified
        assert_eq!(
            qualify(
                "SELECT name FROM users JOIN orders ON id = user_id",
                &schema
            ),
            "SELECT users.name FROM users INNER JOIN orders ON id = orders.user_id"
        );
    }

    #[test]
    fn test_no_schema_columns_passthrough() {
        // Table not in schema — columns pass through unchanged
        let schema = make_schema();
        assert_eq!(
            qualify("SELECT x, y FROM unknown_table", &schema),
            "SELECT x, y FROM unknown_table"
        );
    }
}
