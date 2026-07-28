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

/// Validate that a transformed AST doesn't contain constructs unsupported by the target dialect.
fn validate_dialect_support(stmt: &Statement, target: Dialect) -> errors::Result<()> {
    use crate::ast::Expr;
    use crate::dialects::Dialect::{Fabric, Tsql};

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
    Ok(generate(&transformed, write_dialect))
}
