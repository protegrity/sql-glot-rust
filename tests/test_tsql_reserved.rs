//! CR-009 regression tests.
//!
//! Covers two related issues when transpiling Postgres → TSQL:
//!
//!  1. Column aliases that collide with T-SQL reserved keywords must be
//!     wrapped in square brackets, otherwise MSSQL rejects the query
//!     with error 156.
//!  2. The ANSI niladic temporal expressions `CURRENT_TIME`, `CURRENT_DATE`
//!     and `CURRENT_TIMESTAMP` (no parentheses) must be translated to
//!     TSQL-compatible forms.

use sqlglot_rust::{Dialect, transpile};

// ── Fix 1: reserved-keyword alias quoting ─────────────────────────────────

#[test]
fn alias_reserved_keyword_quoted_tsql() {
    let result = transpile(
        "SELECT NOW() AS current_time",
        Dialect::Postgres,
        Dialect::Tsql,
    )
    .unwrap();
    assert!(
        result.contains("[current_time]"),
        "Reserved keyword alias should be quoted: {result}"
    );
    assert!(result.contains("GETDATE()"));
}

#[test]
fn alias_reserved_keyword_time_quoted_tsql() {
    let result = transpile(
        "SELECT col AS time FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    )
    .unwrap();
    assert!(
        result.contains("[time]"),
        "TIME alias must be bracket-quoted for TSQL: {result}"
    );
}

#[test]
fn alias_reserved_keyword_user_quoted_tsql() {
    let result = transpile(
        "SELECT col AS user FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    )
    .unwrap();
    assert!(
        result.contains("[user]"),
        "USER alias must be bracket-quoted for TSQL: {result}"
    );
}

#[test]
fn non_reserved_alias_unquoted_tsql() {
    let result = transpile("SELECT 1 AS my_alias", Dialect::Postgres, Dialect::Tsql).unwrap();
    assert!(
        !result.contains("[my_alias]"),
        "Non-reserved aliases must not be quoted: {result}"
    );
    assert!(result.contains("AS my_alias"));
}

#[test]
fn reserved_alias_not_quoted_for_postgres() {
    // Same alias is fine for Postgres → no transformation.
    let result = transpile(
        "SELECT NOW() AS current_time",
        Dialect::Postgres,
        Dialect::Postgres,
    )
    .unwrap();
    assert!(
        !result.contains("[current_time]"),
        "Postgres target must not bracket-quote aliases: {result}"
    );
}

#[test]
fn reserved_table_alias_quoted_tsql() {
    let result = transpile(
        "SELECT t.x FROM tbl AS time WHERE t.x > 0",
        Dialect::Postgres,
        Dialect::Tsql,
    )
    .unwrap();
    // The alias `time` on the table must be bracketed.
    assert!(
        result.contains("[time]"),
        "Reserved table alias must be bracket-quoted: {result}"
    );
}

// ── Fix 2: CURRENT_TIME / CURRENT_DATE / CURRENT_TIMESTAMP expressions ───

#[test]
fn current_time_expr_to_tsql() {
    let result = transpile("SELECT CURRENT_TIME", Dialect::Postgres, Dialect::Tsql).unwrap();
    let upper = result.to_uppercase();
    assert!(
        upper.contains("GETDATE") && upper.contains("TIME"),
        "CURRENT_TIME should become CAST(GETDATE() AS TIME): {result}"
    );
}

#[test]
fn current_time_aliased_to_tsql() {
    let result = transpile(
        "SELECT CURRENT_TIME AS ct",
        Dialect::Postgres,
        Dialect::Tsql,
    )
    .unwrap();
    let upper = result.to_uppercase();
    assert!(
        upper.contains("GETDATE"),
        "Should translate CURRENT_TIME: {result}"
    );
    assert!(upper.contains("AS CT"), "Alias preserved: {result}");
}

#[test]
fn current_date_expr_to_tsql() {
    let result = transpile("SELECT CURRENT_DATE", Dialect::Postgres, Dialect::Tsql).unwrap();
    let upper = result.to_uppercase();
    assert!(
        upper.contains("GETDATE") && upper.contains("DATE"),
        "CURRENT_DATE should become CAST(GETDATE() AS DATE): {result}"
    );
}

#[test]
fn current_timestamp_bare_expr_to_tsql() {
    let result = transpile("SELECT CURRENT_TIMESTAMP", Dialect::Postgres, Dialect::Tsql).unwrap();
    let upper = result.to_uppercase();
    assert!(
        upper.contains("GETDATE()"),
        "Bare CURRENT_TIMESTAMP should become GETDATE(): {result}"
    );
}

#[test]
fn current_time_preserved_for_postgres() {
    let result = transpile("SELECT CURRENT_TIME", Dialect::Postgres, Dialect::Postgres).unwrap();
    let upper = result.to_uppercase();
    assert!(
        upper.contains("CURRENT_TIME"),
        "Postgres should keep CURRENT_TIME: {result}"
    );
}

#[test]
fn current_time_mysql_becomes_function_call() {
    let result = transpile("SELECT CURRENT_TIME", Dialect::Postgres, Dialect::Mysql).unwrap();
    let upper = result.to_uppercase();
    assert!(
        upper.contains("CURRENT_TIME()"),
        "MySQL should emit CURRENT_TIME(): {result}"
    );
}
