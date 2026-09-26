//! # sqlglot-rust
//!
//! A SQL parser, optimizer, and transpiler library written in Rust,
//! inspired by Python's [sqlglot](https://github.com/tobymao/sqlglot).
//!
//! ## Features
//!
//! - Parse SQL strings into a structured AST
//! - Generate SQL from AST nodes
//! - Transpile between SQL dialects (30 dialects including MySQL, PostgreSQL, BigQuery, Snowflake, DuckDB, Hive, Spark, Presto, Trino, T-SQL, Oracle, ClickHouse, Redshift, and more)
//! - Optimize SQL queries
//! - CTEs, subqueries, UNION/INTERSECT/EXCEPT
//! - Window functions, CAST, EXTRACT, EXISTS
//! - Pretty-print SQL output
//! - AST traversal (walk, find, transform)
//! - AST diff for semantic SQL comparison
//!
//! ## Quick Start
//!
//! ```rust
//! use sqlglot_rust::{parse, generate, transpile, Dialect};
//!
//! // Parse a SQL query
//! let ast = parse("SELECT a, b FROM t WHERE a > 1", Dialect::Ansi).unwrap();
//!
//! // Generate SQL for a specific dialect
//! let sql = generate(&ast, Dialect::Postgres);
//! assert_eq!(sql, "SELECT a, b FROM t WHERE a > 1");
//!
//! // One-step transpile between dialects
//! let result = transpile("SELECT a, b FROM t", Dialect::Ansi, Dialect::Postgres).unwrap();
//! ```

pub mod ast;
pub mod builder;
pub mod dialects;
pub mod diff;
pub mod errors;
pub mod executor;
pub mod ffi;
pub mod generator;
pub mod optimizer;
pub mod parser;
pub mod planner;
pub mod schema;
pub mod tokens;
#[cfg(feature = "uniffi")]
pub mod uniffi_api;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub use ast::{CommentType, Expr, MergeClauseKind, QuoteStyle, Statement};
pub use builder::{
    ConditionBuilder,
    SelectBuilder,
    // Arithmetic helpers
    add,
    alias,
    and_all,
    between,
    boolean,
    // Operators and expressions
    cast,
    // Expression factory functions
    column,
    // Builders
    condition,
    condition_dialect,
    div,
    // Comparison helpers
    eq,
    exists,
    func,
    func_distinct,
    gt,
    gte,
    in_list,
    in_subquery,
    is_not_null,
    is_null,
    like,
    literal,
    lt,
    lte,
    mul,
    neq,
    not,
    not_in_list,
    null,
    or_all,
    parse_condition,
    parse_condition_dialect,
    // Parse helpers
    parse_expr,
    parse_expr_dialect,
    qualified_star,
    select,
    select_all,
    select_distinct,
    // Other helpers
    star,
    string_literal,
    sub,
    subquery,
    table,
    table_full,
};
pub use dialects::Dialect;
pub use dialects::plugin::{
    DialectPlugin, DialectRef, DialectRegistry, register_dialect, resolve_dialect, transpile_ext,
    transpile_statements_ext,
};
pub use dialects::time::{
    FormatConversionResult, TimeFormatStyle, TsqlStyleCode, format_time, format_time_dialect,
    format_time_with_warnings,
};
pub use diff::{AstNode, ChangeAction, diff as diff_ast, diff_sql};
pub use errors::SqlglotError;
pub use generator::{generate, generate_pretty};
pub use optimizer::annotate_types::{TypeAnnotations, annotate_types};
pub use optimizer::lineage::{
    LineageConfig, LineageError, LineageGraph, LineageNode, lineage, lineage_sql,
};
pub use optimizer::pushdown_predicates::pushdown_predicates;
pub use optimizer::scope_analysis::{Scope, ScopeType, build_scope, find_all_in_scope};
pub use parser::{
    parse, parse_data_type, parse_data_type_with_udt, parse_statements_with_comments,
    parse_with_comments,
};
pub use planner::{Plan, Projection, Step, StepId, plan};

/// Validate that an AST doesn't contain constructs unsupported by the target dialect.
///
/// # Errors
///
/// Returns [`SqlglotError::UnsupportedDialectFeature`] when generation would
/// otherwise emit syntax without a proven equivalent in `target`.
pub fn validate_dialect_support(stmt: &Statement, target: Dialect) -> errors::Result<()> {
    use crate::ast::{Expr, SelectItem, TableSource, TableTemporalClause};
    use crate::dialects::Dialect::{Fabric, Tsql};

    fn temporal_supported(temporal: &TableTemporalClause, target: Dialect) -> bool {
        match temporal {
            TableTemporalClause::Snowflake { .. } => matches!(target, Dialect::Snowflake),
            TableTemporalClause::SystemTime(ast::SystemTimeSpec::AsOf(_)) => {
                matches!(target, Dialect::Tsql | Dialect::BigQuery | Dialect::Hive)
            }
            TableTemporalClause::SystemTime(_) => matches!(target, Dialect::Tsql),
            TableTemporalClause::OracleFlashback(_) => matches!(target, Dialect::Oracle),
            TableTemporalClause::VersionAsOf { .. } => {
                matches!(target, Dialect::Databricks | Dialect::Spark)
            }
            TableTemporalClause::SystemVersionAsOf { .. } => matches!(target, Dialect::Hive),
        }
    }

    fn unsupported_direct_table(table: &ast::TableRef) -> Option<String> {
        table
            .temporal
            .as_ref()
            .map(|temporal| format!("{temporal:?} on a non-query table reference"))
    }

    fn unsupported_in_temporal(temporal: &TableTemporalClause, target: Dialect) -> Option<String> {
        let check = |expr: &Expr| unsupported_in_expr(expr, target);
        match temporal {
            TableTemporalClause::Snowflake { expression, .. }
            | TableTemporalClause::VersionAsOf { expression, .. }
            | TableTemporalClause::SystemVersionAsOf { expression } => check(expression),
            TableTemporalClause::SystemTime(spec) => match spec {
                ast::SystemTimeSpec::AsOf(expression) => check(expression),
                ast::SystemTimeSpec::FromTo { start, end }
                | ast::SystemTimeSpec::BetweenAnd { start, end }
                | ast::SystemTimeSpec::ContainedIn { start, end } => {
                    check(start).or_else(|| check(end))
                }
                ast::SystemTimeSpec::All => None,
            },
            TableTemporalClause::OracleFlashback(spec) => match spec {
                ast::OracleFlashbackSpec::AsOf { expression, .. } => check(expression),
                ast::OracleFlashbackSpec::VersionsBetween { start, end, .. } => {
                    check(start).or_else(|| check(end))
                }
            },
        }
    }

    fn unsupported_in_source(source: &TableSource, target: Dialect) -> Option<String> {
        match source {
            TableSource::Table(table) => table.temporal.as_ref().and_then(|temporal| {
                if temporal_supported(temporal, target) {
                    unsupported_in_temporal(temporal, target)
                } else {
                    Some(format!("{temporal:?}"))
                }
            }),
            TableSource::Subquery { query, .. } => unsupported_temporal(query, target),
            TableSource::Lateral { source }
            | TableSource::Pivot { source, .. }
            | TableSource::Unpivot { source, .. } => unsupported_in_source(source, target),
            TableSource::TableFunction { .. } | TableSource::Unnest { .. } => None,
        }
    }

    fn unsupported_in_expr(expr: &Expr, target: Dialect) -> Option<String> {
        let mut unsupported = None;
        expr.walk(&mut |node| {
            let nested = match node {
                Expr::Subquery(query)
                | Expr::Exists {
                    subquery: query, ..
                } => Some(query.as_ref()),
                Expr::InSubquery { subquery, .. } => Some(subquery.as_ref()),
                _ => None,
            };
            if let Some(query) = nested
                && let Some(found) = unsupported_temporal(query, target)
            {
                unsupported = Some(found);
                return false;
            }
            unsupported.is_none()
        });
        unsupported
    }

    fn unsupported_in_items(items: &[SelectItem], target: Dialect) -> Option<String> {
        items.iter().find_map(|item| match item {
            SelectItem::Expr { expr, .. } => unsupported_in_expr(expr, target),
            SelectItem::Wildcard | SelectItem::QualifiedWildcard { .. } => None,
        })
    }

    fn unsupported_in_select(sel: &ast::SelectStatement, target: Dialect) -> Option<String> {
        for cte in &sel.ctes {
            if let Some(found) = unsupported_temporal(&cte.query, target) {
                return Some(found);
            }
        }
        if let Some(from) = &sel.from
            && let Some(found) = unsupported_in_source(&from.source, target)
        {
            return Some(found);
        }
        for join in &sel.joins {
            if let Some(found) = unsupported_in_source(&join.table, target) {
                return Some(found);
            }
            if let Some(on) = &join.on
                && let Some(found) = unsupported_in_expr(on, target)
            {
                return Some(found);
            }
        }
        if let Some(found) = unsupported_in_items(&sel.columns, target) {
            return Some(found);
        }
        let optional_exprs = [
            sel.top.as_deref(),
            sel.where_clause.as_ref(),
            sel.having.as_ref(),
            sel.limit.as_ref(),
            sel.offset.as_ref(),
            sel.fetch_first.as_ref(),
            sel.qualify.as_ref(),
        ];
        for expr in optional_exprs.into_iter().flatten() {
            if let Some(found) = unsupported_in_expr(expr, target) {
                return Some(found);
            }
        }
        for expr in &sel.group_by {
            if let Some(found) = unsupported_in_expr(expr, target) {
                return Some(found);
            }
        }
        for item in &sel.order_by {
            if let Some(found) = unsupported_in_expr(&item.expr, target) {
                return Some(found);
            }
        }
        None
    }

    fn unsupported_temporal(stmt: &Statement, target: Dialect) -> Option<String> {
        match stmt {
            Statement::Select(sel) => unsupported_in_select(sel, target),
            Statement::SetOperation(set) => unsupported_temporal(&set.left, target)
                .or_else(|| unsupported_temporal(&set.right, target)),
            Statement::Insert(insert) => unsupported_direct_table(&insert.table)
                .or_else(|| match &insert.source {
                    ast::InsertSource::Values(rows) => rows
                        .iter()
                        .flatten()
                        .find_map(|expr| unsupported_in_expr(expr, target)),
                    ast::InsertSource::Query(query) => unsupported_temporal(query, target),
                    ast::InsertSource::Default => None,
                })
                .or_else(|| match &insert.on_conflict {
                    Some(ast::OnConflict {
                        action: ast::ConflictAction::DoUpdate(assignments),
                        ..
                    }) => assignments
                        .iter()
                        .find_map(|(_, expr)| unsupported_in_expr(expr, target)),
                    _ => None,
                })
                .or_else(|| unsupported_in_items(&insert.returning, target)),
            Statement::Update(update) => unsupported_direct_table(&update.table)
                .or_else(|| {
                    update
                        .assignments
                        .iter()
                        .find_map(|(_, expr)| unsupported_in_expr(expr, target))
                })
                .or_else(|| {
                    update
                        .from
                        .as_ref()
                        .and_then(|from| unsupported_in_source(&from.source, target))
                })
                .or_else(|| {
                    update
                        .where_clause
                        .as_ref()
                        .and_then(|expr| unsupported_in_expr(expr, target))
                })
                .or_else(|| unsupported_in_items(&update.returning, target)),
            Statement::Delete(delete) => unsupported_direct_table(&delete.table)
                .or_else(|| {
                    delete
                        .using
                        .as_ref()
                        .and_then(|using| unsupported_in_source(&using.source, target))
                })
                .or_else(|| {
                    delete
                        .where_clause
                        .as_ref()
                        .and_then(|expr| unsupported_in_expr(expr, target))
                })
                .or_else(|| unsupported_in_items(&delete.returning, target)),
            Statement::Merge(merge) => unsupported_direct_table(&merge.target)
                .or_else(|| unsupported_in_source(&merge.source, target))
                .or_else(|| unsupported_in_expr(&merge.on, target))
                .or_else(|| {
                    merge.clauses.iter().find_map(|clause| {
                        clause
                            .condition
                            .as_ref()
                            .and_then(|expr| unsupported_in_expr(expr, target))
                            .or_else(|| match &clause.action {
                                ast::MergeAction::Update(assignments) => assignments
                                    .iter()
                                    .find_map(|(_, expr)| unsupported_in_expr(expr, target)),
                                ast::MergeAction::Insert { values, .. } => values
                                    .iter()
                                    .find_map(|expr| unsupported_in_expr(expr, target)),
                                ast::MergeAction::InsertRow | ast::MergeAction::Delete => None,
                            })
                    })
                })
                .or_else(|| unsupported_in_items(&merge.output, target)),
            Statement::CreateTable(create) => {
                unsupported_direct_table(&create.table).or_else(|| {
                    create
                        .as_select
                        .as_ref()
                        .and_then(|query| unsupported_temporal(query, target))
                })
            }
            Statement::DropTable(drop) => unsupported_direct_table(&drop.table),
            Statement::AlterTable(alter) => unsupported_direct_table(&alter.table),
            Statement::CreateView(create) => unsupported_direct_table(&create.name)
                .or_else(|| unsupported_temporal(&create.query, target)),
            Statement::DropView(drop) => unsupported_direct_table(&drop.name),
            Statement::Truncate(truncate) => unsupported_direct_table(&truncate.table),
            Statement::Explain(explain) => unsupported_temporal(&explain.statement, target),
            Statement::Expression(expr) => unsupported_in_expr(expr, target),
            _ => None,
        }
    }

    if let Some(temporal) = unsupported_temporal(stmt, target) {
        return Err(errors::SqlglotError::UnsupportedDialectFeature(format!(
            "table temporal clause {temporal} has no proven equivalent in {target}"
        )));
    }

    if !matches!(target, Tsql | Fabric) {
        return Ok(());
    }

    // Walk the AST looking for unsupported constructs
    let mut found_array = false;
    fn check_expr(expr: &Expr, found: &mut bool) {
        match expr {
            Expr::ArrayLiteral(_) => {
                *found = true;
            }
            _ => {
                expr.walk(&mut |e| {
                    if matches!(e, Expr::ArrayLiteral(_)) {
                        *found = true;
                        false
                    } else {
                        true
                    }
                });
            }
        }
    }

    match stmt {
        Statement::Select(sel) => {
            for item in &sel.columns {
                if let ast::SelectItem::Expr { expr, .. } = item {
                    check_expr(expr, &mut found_array);
                }
            }
            if let Some(wh) = &sel.where_clause {
                check_expr(wh, &mut found_array);
            }
        }
        Statement::Expression(expr) => check_expr(expr, &mut found_array),
        _ => {}
    }

    if found_array {
        return Err(errors::SqlglotError::UnsupportedDialectFeature(
            "ARRAY constructor has no T-SQL equivalent".to_string(),
        ));
    }

    Ok(())
}

/// Transpile a SQL string from one dialect to another.
///
/// This is the primary high-level API, corresponding to Python sqlglot's
/// `sqlglot.transpile()`.
///
/// # Example
///
/// ```rust
/// use sqlglot_rust::{transpile, Dialect};
///
/// let result = transpile(
///     "SELECT CAST(x AS INT) FROM t",
///     Dialect::Ansi,
///     Dialect::Postgres,
/// ).unwrap();
/// ```
///
/// # Errors
///
/// Returns a [`SqlglotError`] if parsing fails.
pub fn transpile(
    sql: &str,
    read_dialect: Dialect,
    write_dialect: Dialect,
) -> errors::Result<String> {
    let ast = parse(sql, read_dialect)?;
    let transformed = dialects::transform(&ast, read_dialect, write_dialect);
    validate_dialect_support(&transformed, write_dialect)?;
    Ok(generate(&transformed, write_dialect))
}

/// Transpile a SQL string, returning multiple statements if the input
/// contains semicolons.
///
/// # Errors
///
/// Returns a [`SqlglotError`] if parsing fails.
pub fn transpile_statements(
    sql: &str,
    read_dialect: Dialect,
    write_dialect: Dialect,
) -> errors::Result<Vec<String>> {
    let stmts = parser::parse_statements(sql, read_dialect)?;
    let mut results = Vec::with_capacity(stmts.len());
    for stmt in &stmts {
        let transformed = dialects::transform(stmt, read_dialect, write_dialect);
        validate_dialect_support(&transformed, write_dialect)?;
        results.push(generate(&transformed, write_dialect));
    }
    Ok(results)
}

/// Transpile a SQL string preserving comments through the pipeline.
///
/// # Errors
///
/// Returns a [`SqlglotError`] if parsing fails.
pub fn transpile_with_comments(
    sql: &str,
    read_dialect: Dialect,
    write_dialect: Dialect,
) -> errors::Result<String> {
    let ast = parse_with_comments(sql, read_dialect)?;
    let transformed = dialects::transform(&ast, read_dialect, write_dialect);
    validate_dialect_support(&transformed, write_dialect)?;
    Ok(generate(&transformed, write_dialect))
}
