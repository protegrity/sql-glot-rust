//! Scope analysis for SQL queries.
//!
//! Provides a [`Scope`] struct that tracks the sources, columns, and
//! relationships within a SQL query. This is the foundation for
//! optimizer passes like qualify_columns, pushdown_predicates,
//! annotate_types, and column lineage analysis.
//!
//! Inspired by Python sqlglot's `optimizer/scope.py`.
//!
//! # Example
//!
//! ```rust
//! use sqlglot_rust::parser::parse;
//! use sqlglot_rust::dialects::Dialect;
//! use sqlglot_rust::optimizer::scope_analysis::{build_scope, ScopeType};
//!
//! let ast = parse("SELECT a, b FROM t WHERE a > 1", Dialect::Ansi).unwrap();
//! let scope = build_scope(&ast);
//! assert_eq!(scope.scope_type, ScopeType::Root);
//! assert!(!scope.columns.is_empty());
//! ```

use std::collections::HashMap;

use crate::ast::*;

// ═══════════════════════════════════════════════════════════════════════
// Scope types
// ═══════════════════════════════════════════════════════════════════════

/// The kind of scope a query fragment lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeType {
    /// The outermost query.
    Root,
    /// A scalar or lateral subquery in a WHERE / SELECT / HAVING expression.
    Subquery,
    /// A derived table (subquery in FROM).
    DerivedTable,
    /// A CTE definition (`WITH name AS (...)`).
    Cte,
    /// One branch of a UNION / INTERSECT / EXCEPT.
    Union,
    /// A user-defined table function (UDTF) / LATERAL.
    Udtf,
}

// ═══════════════════════════════════════════════════════════════════════
// Source — either a table reference or a child scope
// ═══════════════════════════════════════════════════════════════════════

/// A source within a scope. Can be a concrete table or a reference to a
/// child scope (derived table, CTE, etc.).
#[derive(Debug, Clone)]
pub enum Source {
    /// A direct table reference.
    Table(TableRef),
    /// A child scope (derived table, CTE, UDTF).
    Scope(Box<Scope>),
}

// ═══════════════════════════════════════════════════════════════════════
// Column reference — a resolved column encountered in the scope
// ═══════════════════════════════════════════════════════════════════════

/// A column reference encountered during scope analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnRef {
    /// Optional table qualifier.
    pub table: Option<String>,
    /// The column name.
    pub name: String,
}

// ═══════════════════════════════════════════════════════════════════════
// Scope
// ═══════════════════════════════════════════════════════════════════════

/// Represents a single query scope and its relationships.
///
/// A scope is the context created by a query level (a SELECT, each branch
/// of a UNION, each CTE definition, etc.). It tracks what tables are
/// visible, what columns are referenced, and how nested scopes relate.
#[derive(Debug, Clone)]
pub struct Scope {
    /// What kind of scope this is.
    pub scope_type: ScopeType,

    /// Mapping of source name/alias → [`Source`].
    /// For tables this is `alias.unwrap_or(name) → Source::Table(...)`.
    /// For derived tables / CTEs it is `alias → Source::Scope(...)`.
    pub sources: HashMap<String, Source>,

    /// All column references found *directly* in this scope (not in
    /// child subquery scopes).
    pub columns: Vec<ColumnRef>,

    /// Columns that reference an *outer* scope (correlation).
    pub external_columns: Vec<ColumnRef>,

    /// Child scopes created by derived tables (subqueries in FROM).
    pub derived_table_scopes: Vec<Scope>,

    /// Child scopes created by scalar subqueries (in SELECT / WHERE / HAVING).
    pub subquery_scopes: Vec<Scope>,

    /// Child scopes for each branch of a UNION / INTERSECT / EXCEPT.
    pub union_scopes: Vec<Scope>,

    /// Child scopes for CTE definitions.
    pub cte_scopes: Vec<Scope>,

    /// Sources that are actually referenced by columns in SELECT (subset
    /// of `sources`). Keyed the same way as `sources`.
    pub selected_sources: HashMap<String, Source>,

    /// Whether this scope contains correlated references to an outer scope.
    pub is_correlated: bool,

    /// The SQL expression/statement this scope was built from (optional,
    /// kept for diagnostics).
    expression: Option<ScopeExpression>,
}

/// Thin wrapper so we can attach the originating AST node for debugging.
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum ScopeExpression {
    Statement(Statement),
}

impl Scope {
    fn new(scope_type: ScopeType) -> Self {
        Self {
            scope_type,
            sources: HashMap::new(),
            columns: Vec::new(),
            external_columns: Vec::new(),
            derived_table_scopes: Vec::new(),
            subquery_scopes: Vec::new(),
            union_scopes: Vec::new(),
            cte_scopes: Vec::new(),
            selected_sources: HashMap::new(),
            is_correlated: false,
            expression: None,
        }
    }

    /// Names (aliases / table names) of all sources visible in this scope.
    #[must_use]
    pub fn source_names(&self) -> Vec<&str> {
        self.sources.keys().map(String::as_str).collect()
    }

    /// Iterate over all child scopes (derived tables, subqueries, unions,
    /// CTEs) in a flat list.
    #[must_use]
    pub fn child_scopes(&self) -> Vec<&Scope> {
        let mut children: Vec<&Scope> = Vec::new();
        children.extend(self.derived_table_scopes.iter());
        children.extend(self.subquery_scopes.iter());
        children.extend(self.union_scopes.iter());
        children.extend(self.cte_scopes.iter());
        children
    }

    /// Walk through all scopes in the tree (pre-order).
    pub fn walk<F>(&self, visitor: &mut F)
    where
        F: FnMut(&Scope),
    {
        visitor(self);
        for child in self.child_scopes() {
            child.walk(visitor);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════════════════════════════════

/// Build the scope tree from a parsed SQL statement.
///
/// The returned [`Scope`] is the root scope; child scopes (CTEs, derived
/// tables, subqueries, unions) are reachable via the corresponding fields.
#[must_use]
pub fn build_scope(statement: &Statement) -> Scope {
    let mut scope = Scope::new(ScopeType::Root);
    scope.expression = Some(ScopeExpression::Statement(statement.clone()));
    build_scope_inner(statement, &mut scope, ScopeType::Root);
    resolve_selected_sources(&mut scope);
    detect_correlation(&mut scope, &[]);
    scope
}

/// Find all expressions in a scope that match a predicate, respecting
/// scope boundaries (does not descend into child scopes).
///
/// This is the equivalent of Python sqlglot's `find_all_in_scope`.
#[must_use]
pub fn find_all_in_scope<'a, F>(scope: &'a Scope, predicate: &F) -> Vec<&'a ColumnRef>
where
    F: Fn(&ColumnRef) -> bool,
{
    scope.columns.iter().filter(|c| predicate(c)).collect()
}

// ═══════════════════════════════════════════════════════════════════════
// Internal scope-building logic
// ═══════════════════════════════════════════════════════════════════════

fn build_scope_inner(statement: &Statement, scope: &mut Scope, _scope_type: ScopeType) {
    match statement {
        Statement::Select(sel) => build_select_scope(sel, scope),
        Statement::SetOperation(set_op) => build_set_operation_scope(set_op, scope),
        Statement::CreateView(cv) => {
            // Treat the view body as a root-like scope
            build_scope_inner(&cv.query, scope, ScopeType::Root);
        }
        Statement::Insert(ins) => {
            if let InsertSource::Query(q) = &ins.source {
                let mut sub = Scope::new(ScopeType::Subquery);
                build_scope_inner(q, &mut sub, ScopeType::Subquery);
                resolve_selected_sources(&mut sub);
                scope.subquery_scopes.push(sub);
            }
        }
        Statement::Delete(del) => {
            // Collect columns from WHERE
            if let Some(wh) = &del.where_clause {
                collect_columns_from_expr(wh, scope);
            }
        }
        Statement::Update(upd) => {
            // Collect columns from assignments and WHERE
            for (_col, expr) in &upd.assignments {
                collect_columns_from_expr(expr, scope);
            }
            if let Some(wh) = &upd.where_clause {
                collect_columns_from_expr(wh, scope);
            }
        }
        Statement::Explain(expl) => {
            build_scope_inner(&expl.statement, scope, _scope_type);
        }
        // Other statement types don't introduce meaningful scopes
        _ => {}
    }
}

/// Build scope information for a SELECT statement.
fn build_select_scope(sel: &SelectStatement, scope: &mut Scope) {
    // ── 1. Process CTEs ──────────────────────────────────────────────
    for cte in &sel.ctes {
        let mut cte_scope = Scope::new(ScopeType::Cte);
        cte_scope.expression = Some(ScopeExpression::Statement(*cte.query.clone()));
        build_scope_inner(&cte.query, &mut cte_scope, ScopeType::Cte);
        resolve_selected_sources(&mut cte_scope);

        // The CTE name is visible as a source in the outer scope
        scope
            .sources
            .insert(cte.name.clone(), Source::Scope(Box::new(cte_scope.clone())));
        scope.cte_scopes.push(cte_scope);
    }

    // ── 2. Process FROM source ───────────────────────────────────────
    if let Some(from) = &sel.from {
        process_table_source(&from.source, scope);
    }

    // ── 3. Process JOINs ─────────────────────────────────────────────
    for join in &sel.joins {
        process_table_source(&join.table, scope);
        if let Some(on) = &join.on {
            collect_columns_from_expr(on, scope);
        }
    }

    // ── 4. Process SELECT columns ────────────────────────────────────
    for item in &sel.columns {
        match item {
            SelectItem::Expr { expr, .. } => {
                collect_columns_from_expr(expr, scope);
                collect_subqueries_from_expr(expr, scope);
            }
            SelectItem::QualifiedWildcard { table } => {
                // table.* — record as a column reference so selected_sources
                // picks it up
                scope.columns.push(ColumnRef {
                    table: Some(table.clone()),
                    name: "*".to_string(),
                });
            }
            SelectItem::Wildcard => {}
        }
    }

    // ── 5. Process WHERE ─────────────────────────────────────────────
    if let Some(wh) = &sel.where_clause {
        collect_columns_from_expr(wh, scope);
        collect_subqueries_from_expr(wh, scope);
    }

    // ── 6. Process GROUP BY ──────────────────────────────────────────
    for expr in &sel.group_by {
        collect_columns_from_expr(expr, scope);
    }

    // ── 7. Process HAVING ────────────────────────────────────────────
    if let Some(having) = &sel.having {
        collect_columns_from_expr(having, scope);
        collect_subqueries_from_expr(having, scope);
    }

    // ── 8. Process ORDER BY ──────────────────────────────────────────
    for item in &sel.order_by {
        collect_columns_from_expr(&item.expr, scope);
    }

    // ── 9. Process QUALIFY ───────────────────────────────────────────
    if let Some(qualify) = &sel.qualify {
        collect_columns_from_expr(qualify, scope);
        collect_subqueries_from_expr(qualify, scope);
    }
}

/// Build scope for UNION / INTERSECT / EXCEPT.
fn build_set_operation_scope(set_op: &SetOperationStatement, scope: &mut Scope) {
    // Each branch gets its own Union scope
    let mut left_scope = Scope::new(ScopeType::Union);
    build_scope_inner(&set_op.left, &mut left_scope, ScopeType::Union);
    resolve_selected_sources(&mut left_scope);
    scope.union_scopes.push(left_scope);

    let mut right_scope = Scope::new(ScopeType::Union);
    build_scope_inner(&set_op.right, &mut right_scope, ScopeType::Union);
    resolve_selected_sources(&mut right_scope);
    scope.union_scopes.push(right_scope);

    // ORDER BY and LIMIT on the set operation itself
    for item in &set_op.order_by {
        collect_columns_from_expr(&item.expr, scope);
    }
}

/// Register a table source in the scope and recurse into derived tables.
fn process_table_source(source: &TableSource, scope: &mut Scope) {
    match source {
        TableSource::Table(table_ref) => {
            let key = table_ref
                .alias
                .as_deref()
                .unwrap_or(&table_ref.name)
                .to_string();
            let source = match (
                table_ref.catalog.is_none() && table_ref.schema.is_none(),
                scope.sources.get(&table_ref.name),
            ) {
                (true, Some(Source::Scope(cte_scope))) => Source::Scope(cte_scope.clone()),
                _ => Source::Table(table_ref.clone()),
            };
            scope.sources.insert(key, source);
        }
        TableSource::Subquery { query, alias, .. } => {
            let mut dt_scope = Scope::new(ScopeType::DerivedTable);
            dt_scope.expression = Some(ScopeExpression::Statement(*query.clone()));
            build_scope_inner(query, &mut dt_scope, ScopeType::DerivedTable);
            resolve_selected_sources(&mut dt_scope);

            if let Some(alias) = alias {
                scope
                    .sources
                    .insert(alias.clone(), Source::Scope(Box::new(dt_scope.clone())));
            }
            scope.derived_table_scopes.push(dt_scope);
        }
        TableSource::TableFunction { alias, .. } => {
            if let Some(alias) = alias {
                // UDTF — register the alias but we can't descend further
                scope.sources.insert(
                    alias.clone(),
                    Source::Table(TableRef {
                        catalog: None,
                        schema: None,
                        name: alias.clone(),
                        alias: None,
                        name_quote_style: QuoteStyle::None,
                        alias_quote_style: QuoteStyle::None,
                    }),
                );
            }
        }
        TableSource::Lateral { source } => {
            process_table_source(source, scope);
        }
        TableSource::Pivot { source, alias, .. } | TableSource::Unpivot { source, alias, .. } => {
            process_table_source(source, scope);
            if let Some(alias) = alias {
                scope.sources.insert(
                    alias.clone(),
                    Source::Table(TableRef {
                        catalog: None,
                        schema: None,
                        name: alias.clone(),
                        alias: None,
                        name_quote_style: QuoteStyle::None,
                        alias_quote_style: QuoteStyle::None,
                    }),
                );
            }
        }
        TableSource::Unnest { alias, .. } => {
            if let Some(alias) = alias {
                scope.sources.insert(
                    alias.clone(),
                    Source::Table(TableRef {
                        catalog: None,
                        schema: None,
                        name: alias.clone(),
                        alias: None,
                        name_quote_style: QuoteStyle::None,
                        alias_quote_style: QuoteStyle::None,
                    }),
                );
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Column collection — stays within scope boundaries
// ═══════════════════════════════════════════════════════════════════════

/// Collect column references from an expression, stopping at subquery
/// boundaries (those create their own scopes).
fn collect_columns_from_expr(expr: &Expr, scope: &mut Scope) {
    expr.walk(&mut |e| {
        match e {
            Expr::Column { table, name, .. } => {
                scope.columns.push(ColumnRef {
                    table: table.clone(),
                    name: name.clone(),
                });
                true
            }
            // Don't descend into subqueries — they create their own scope
            Expr::Subquery(_) | Expr::Exists { .. } | Expr::InSubquery { .. } => false,
            _ => true,
        }
    });
}

/// Collect scalar and EXISTS subqueries from an expression as child scopes.
fn collect_subqueries_from_expr(expr: &Expr, scope: &mut Scope) {
    match expr {
        Expr::Subquery(stmt) => {
            let mut sub = Scope::new(ScopeType::Subquery);
            sub.expression = Some(ScopeExpression::Statement(*stmt.clone()));
            build_scope_inner(stmt, &mut sub, ScopeType::Subquery);
            resolve_selected_sources(&mut sub);
            scope.subquery_scopes.push(sub);
        }
        Expr::Exists { subquery, .. } => {
            let mut sub = Scope::new(ScopeType::Subquery);
            sub.expression = Some(ScopeExpression::Statement(*subquery.clone()));
            build_scope_inner(subquery, &mut sub, ScopeType::Subquery);
            resolve_selected_sources(&mut sub);
            scope.subquery_scopes.push(sub);
        }
        Expr::InSubquery {
            expr: left,
            subquery,
            ..
        } => {
            // The left-hand expression belongs to the *current* scope
            collect_columns_from_expr(left, scope);

            let mut sub = Scope::new(ScopeType::Subquery);
            sub.expression = Some(ScopeExpression::Statement(*subquery.clone()));
            build_scope_inner(subquery, &mut sub, ScopeType::Subquery);
            resolve_selected_sources(&mut sub);
            scope.subquery_scopes.push(sub);
        }
        _ => {
            // Walk children to find nested subqueries
            walk_expr_for_subqueries(expr, scope);
        }
    }
}

/// Walk child expressions looking for subqueries, without double-processing
/// the top-level node.
fn walk_expr_for_subqueries(expr: &Expr, scope: &mut Scope) {
    match expr {
        Expr::BinaryOp { left, right, .. } => {
            collect_subqueries_from_expr(left, scope);
            collect_subqueries_from_expr(right, scope);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            collect_subqueries_from_expr(inner, scope);
        }
        Expr::Function { args, filter, .. } => {
            for arg in args {
                collect_subqueries_from_expr(arg, scope);
            }
            if let Some(f) = filter {
                collect_subqueries_from_expr(f, scope);
            }
        }
        Expr::Nested(inner) => {
            collect_subqueries_from_expr(inner, scope);
        }
        Expr::Case {
            operand,
            when_clauses,
            else_clause,
        } => {
            if let Some(op) = operand {
                collect_subqueries_from_expr(op, scope);
            }
            for (cond, result) in when_clauses {
                collect_subqueries_from_expr(cond, scope);
                collect_subqueries_from_expr(result, scope);
            }
            if let Some(el) = else_clause {
                collect_subqueries_from_expr(el, scope);
            }
        }
        Expr::Between {
            expr: inner,
            low,
            high,
            ..
        } => {
            collect_subqueries_from_expr(inner, scope);
            collect_subqueries_from_expr(low, scope);
            collect_subqueries_from_expr(high, scope);
        }
        Expr::InList {
            expr: inner, list, ..
        } => {
            collect_subqueries_from_expr(inner, scope);
            for item in list {
                collect_subqueries_from_expr(item, scope);
            }
        }
        Expr::Cast { expr: inner, .. } | Expr::TryCast { expr: inner, .. } => {
            collect_subqueries_from_expr(inner, scope);
        }
        Expr::Coalesce(items) | Expr::ArrayLiteral(items) | Expr::Tuple(items) => {
            for item in items {
                collect_subqueries_from_expr(item, scope);
            }
        }
        Expr::If {
            condition,
            true_val,
            false_val,
        } => {
            collect_subqueries_from_expr(condition, scope);
            collect_subqueries_from_expr(true_val, scope);
            if let Some(fv) = false_val {
                collect_subqueries_from_expr(fv, scope);
            }
        }
        Expr::IsNull { expr: inner, .. } | Expr::IsBool { expr: inner, .. } => {
            collect_subqueries_from_expr(inner, scope);
        }
        Expr::Like {
            expr: inner,
            pattern,
            ..
        }
        | Expr::ILike {
            expr: inner,
            pattern,
            ..
        } => {
            collect_subqueries_from_expr(inner, scope);
            collect_subqueries_from_expr(pattern, scope);
        }
        Expr::Alias { expr: inner, .. } | Expr::Collate { expr: inner, .. } => {
            collect_subqueries_from_expr(inner, scope);
        }
        Expr::NullIf {
            expr: inner,
            r#else,
        } => {
            collect_subqueries_from_expr(inner, scope);
            collect_subqueries_from_expr(r#else, scope);
        }
        Expr::AnyOp {
            expr: inner, right, ..
        }
        | Expr::AllOp {
            expr: inner, right, ..
        } => {
            collect_subqueries_from_expr(inner, scope);
            collect_subqueries_from_expr(right, scope);
        }
        Expr::ArrayIndex { expr: inner, index } => {
            collect_subqueries_from_expr(inner, scope);
            collect_subqueries_from_expr(index, scope);
        }
        Expr::JsonAccess {
            expr: inner, path, ..
        } => {
            collect_subqueries_from_expr(inner, scope);
            collect_subqueries_from_expr(path, scope);
        }
        Expr::Lambda { body, .. } => {
            collect_subqueries_from_expr(body, scope);
        }
        Expr::Extract { expr: inner, .. } | Expr::Interval { value: inner, .. } => {
            collect_subqueries_from_expr(inner, scope);
        }
        Expr::Cube { exprs } | Expr::Rollup { exprs } | Expr::GroupingSets { sets: exprs } => {
            for item in exprs {
                collect_subqueries_from_expr(item, scope);
            }
        }
        // Leaf nodes — nothing to do
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Selected-source resolution
// ═══════════════════════════════════════════════════════════════════════

/// Populate `selected_sources` by checking which sources are actually
/// referenced by the scope's column list.
#[allow(clippy::collapsible_if)]
fn resolve_selected_sources(scope: &mut Scope) {
    for col in &scope.columns {
        if let Some(table) = &col.table {
            if let Some(source) = scope.sources.get(table) {
                scope
                    .selected_sources
                    .entry(table.clone())
                    .or_insert_with(|| source.clone());
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Correlation detection
// ═══════════════════════════════════════════════════════════════════════

/// Detect correlated references: columns in child scopes that reference
/// tables from an outer scope. Populates `external_columns` and
/// `is_correlated` on each child scope.
fn detect_correlation(scope: &mut Scope, outer_source_names: &[String]) {
    // Source names visible in this scope
    let mut visible: Vec<String> = outer_source_names.to_vec();
    visible.extend(scope.sources.keys().cloned());

    // Process each category of child scopes
    detect_correlation_in_children(&mut scope.subquery_scopes, &visible);
    detect_correlation_in_children(&mut scope.derived_table_scopes, &visible);
    detect_correlation_in_children(&mut scope.union_scopes, &visible);
    detect_correlation_in_children(&mut scope.cte_scopes, &visible);
}

#[allow(clippy::collapsible_if)]
fn detect_correlation_in_children(children: &mut [Scope], outer_names: &[String]) {
    for child in children.iter_mut() {
        // A column in the child is external (correlated) if its table
        // qualifier matches an outer source but NOT a source in the child.
        for col in &child.columns {
            if let Some(table) = &col.table {
                if outer_names.contains(table) && !child.sources.contains_key(table) {
                    child.external_columns.push(col.clone());
                    child.is_correlated = true;
                }
            }
        }

        // Recurse into the child's own children
        detect_correlation(child, outer_names);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialects::Dialect;
    use crate::parser::parse;

    /// Helper: parse SQL and build scope in one step.
    fn scope_for(sql: &str) -> Scope {
        let ast = parse(sql, Dialect::Ansi).unwrap();
        build_scope(&ast)
    }

    // ── Basic SELECT ─────────────────────────────────────────────────

    #[test]
    fn test_simple_select() {
        let scope = scope_for("SELECT a, b FROM t WHERE a > 1");
        assert_eq!(scope.scope_type, ScopeType::Root);
        assert!(scope.sources.contains_key("t"));
        // Should have columns a, b, a (from WHERE)
        assert!(scope.columns.len() >= 2);
        assert!(scope.external_columns.is_empty());
        assert!(!scope.is_correlated);
    }

    #[test]
    fn test_aliased_table() {
        let scope = scope_for("SELECT t1.x FROM my_table t1");
        assert!(scope.sources.contains_key("t1"));
        assert!(!scope.sources.contains_key("my_table"));
    }

    // ── JOINs ────────────────────────────────────────────────────────

    #[test]
    fn test_join_sources() {
        let scope = scope_for("SELECT a.id, b.val FROM alpha a JOIN beta b ON a.id = b.id");
        assert!(scope.sources.contains_key("a"));
        assert!(scope.sources.contains_key("b"));
        // ON condition columns
        let on_cols: Vec<_> = scope.columns.iter().filter(|c| c.name == "id").collect();
        assert!(on_cols.len() >= 2); // a.id, b.id from ON + maybe SELECT
    }

    // ── Derived tables ───────────────────────────────────────────────

    #[test]
    fn test_derived_table() {
        let scope = scope_for("SELECT sub.x FROM (SELECT a AS x FROM t) sub");
        assert!(scope.sources.contains_key("sub"));
        assert_eq!(scope.derived_table_scopes.len(), 1);

        let dt = &scope.derived_table_scopes[0];
        assert_eq!(dt.scope_type, ScopeType::DerivedTable);
        assert!(dt.sources.contains_key("t"));
    }

    // ── CTEs ─────────────────────────────────────────────────────────

    #[test]
    fn test_cte_scope() {
        let scope = scope_for("WITH cte AS (SELECT id FROM t) SELECT cte.id FROM cte");
        assert!(scope.sources.contains_key("cte"));
        assert!(matches!(scope.sources["cte"], Source::Scope(_)));
        assert_eq!(scope.cte_scopes.len(), 1);

        let cte = &scope.cte_scopes[0];
        assert_eq!(cte.scope_type, ScopeType::Cte);
        assert!(cte.sources.contains_key("t"));
    }

    #[test]
    fn test_aliased_cte_source() {
        let scope = scope_for("WITH cte AS (SELECT id FROM t) SELECT c.id FROM cte AS c");
        assert!(matches!(scope.sources["cte"], Source::Scope(_)));
        assert!(matches!(scope.sources["c"], Source::Scope(_)));
        assert!(scope.selected_sources.contains_key("c"));
    }

    #[test]
    fn test_qualified_table_does_not_resolve_to_cte() {
        let scope =
            scope_for("WITH cards AS (SELECT id FROM t) SELECT public.cards.id FROM public.cards");
        assert!(matches!(scope.sources["cards"], Source::Table(_)));
    }

    #[test]
    fn test_multiple_ctes() {
        let scope = scope_for(
            "WITH a AS (SELECT 1 AS x), b AS (SELECT 2 AS y) \
             SELECT a.x, b.y FROM a, b",
        );
        assert_eq!(scope.cte_scopes.len(), 2);
        assert!(scope.sources.contains_key("a"));
        assert!(scope.sources.contains_key("b"));
    }

    // ── UNION / INTERSECT / EXCEPT ───────────────────────────────────

    #[test]
    fn test_union_scopes() {
        let scope = scope_for("SELECT a FROM t1 UNION ALL SELECT b FROM t2");
        assert_eq!(scope.union_scopes.len(), 2);

        let left = &scope.union_scopes[0];
        assert_eq!(left.scope_type, ScopeType::Union);
        assert!(left.sources.contains_key("t1"));

        let right = &scope.union_scopes[1];
        assert!(right.sources.contains_key("t2"));
    }

    // ── Scalar subqueries ────────────────────────────────────────────

    #[test]
    fn test_scalar_subquery() {
        let scope = scope_for("SELECT (SELECT MAX(x) FROM t2) AS mx FROM t1");
        assert_eq!(scope.subquery_scopes.len(), 1);
        let sub = &scope.subquery_scopes[0];
        assert_eq!(sub.scope_type, ScopeType::Subquery);
        assert!(sub.sources.contains_key("t2"));
    }

    // ── EXISTS ───────────────────────────────────────────────────────

    #[test]
    fn test_exists_subquery() {
        let scope =
            scope_for("SELECT a FROM t1 WHERE EXISTS (SELECT 1 FROM t2 WHERE t2.id = t1.id)");
        assert_eq!(scope.subquery_scopes.len(), 1);
        let sub = &scope.subquery_scopes[0];
        assert!(sub.sources.contains_key("t2"));
        // t1.id inside the subquery references outer scope ⇒ correlated
        assert!(sub.is_correlated);
        assert!(!sub.external_columns.is_empty());
        let ext = &sub.external_columns[0];
        assert_eq!(ext.table.as_deref(), Some("t1"));
        assert_eq!(ext.name, "id");
    }

    // ── IN subquery ──────────────────────────────────────────────────

    #[test]
    fn test_in_subquery() {
        let scope = scope_for("SELECT a FROM t1 WHERE a IN (SELECT b FROM t2)");
        assert_eq!(scope.subquery_scopes.len(), 1);
        let sub = &scope.subquery_scopes[0];
        assert!(sub.sources.contains_key("t2"));
        // Not correlated — no outer reference
        assert!(!sub.is_correlated);
    }

    // ── Correlated subquery ──────────────────────────────────────────

    #[test]
    fn test_correlated_subquery() {
        let scope =
            scope_for("SELECT a FROM t1 WHERE a = (SELECT MAX(b) FROM t2 WHERE t2.fk = t1.id)");
        assert_eq!(scope.subquery_scopes.len(), 1);
        let sub = &scope.subquery_scopes[0];
        assert!(sub.is_correlated);
        assert!(
            sub.external_columns
                .iter()
                .any(|c| c.table.as_deref() == Some("t1"))
        );
    }

    // ── Nested subqueries ────────────────────────────────────────────

    #[test]
    fn test_nested_subqueries() {
        let scope = scope_for(
            "SELECT a FROM t1 WHERE a IN (SELECT b FROM t2 WHERE b > (SELECT MIN(c) FROM t3))",
        );
        // t1's scope should have 1 subquery (the IN subquery)
        assert_eq!(scope.subquery_scopes.len(), 1);

        let in_sub = &scope.subquery_scopes[0];
        assert!(in_sub.sources.contains_key("t2"));
        // The nested scalar subquery inside the IN subquery
        assert_eq!(in_sub.subquery_scopes.len(), 1);
        let inner = &in_sub.subquery_scopes[0];
        assert!(inner.sources.contains_key("t3"));
    }

    // ── Selected sources ─────────────────────────────────────────────

    #[test]
    fn test_selected_sources() {
        let scope = scope_for("SELECT a.x FROM alpha a JOIN beta b ON a.id = b.id");
        // Column a.x references source "a", so it should be in selected_sources
        assert!(scope.selected_sources.contains_key("a"));
    }

    // ── find_all_in_scope ────────────────────────────────────────────

    #[test]
    fn test_find_all_in_scope() {
        let scope = scope_for("SELECT t.a, t.b, s.c FROM t JOIN s ON t.id = s.id");
        let t_cols = find_all_in_scope(&scope, &|c| c.table.as_deref() == Some("t"));
        // t.a, t.b, t.id
        assert!(t_cols.len() >= 3);
    }

    // ── Scope walk ───────────────────────────────────────────────────

    #[test]
    fn test_scope_walk() {
        let scope = scope_for(
            "WITH cte AS (SELECT 1 AS a) \
             SELECT * FROM cte WHERE EXISTS (SELECT 1 FROM t)",
        );
        let mut count = 0;
        scope.walk(&mut |_| count += 1);
        // root + cte + exists subquery = 3
        assert!(count >= 3);
    }

    // ── Qualified wildcard ───────────────────────────────────────────

    #[test]
    fn test_qualified_wildcard() {
        let scope = scope_for("SELECT t.* FROM t");
        assert!(
            scope
                .columns
                .iter()
                .any(|c| c.table.as_deref() == Some("t") && c.name == "*")
        );
        assert!(scope.selected_sources.contains_key("t"));
    }
}
