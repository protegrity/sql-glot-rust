/// Tests for double-quoted identifier alias preservation (CR-005).
///
/// Verifies that the parser preserves QuoteStyle on aliases and the generator
/// emits them with proper quoting, preventing silent miscompiles when
/// re-emitted SQL is executed on case-sensitive databases (PostgreSQL, Oracle, etc.).
use sqlglot_rust::{Dialect, generate, parse, transpile};

/// Parse SQL → generate SQL, assert output == input (identity round-trip).
fn validate_identity(sql: &str, dialect: Dialect) {
    let ast = parse(sql, dialect).unwrap_or_else(|e| panic!("Parse failed for '{}': {}", sql, e));
    let output = generate(&ast, dialect);
    assert_eq!(output, sql, "\n  Identity roundtrip failed");
}

/// Parse SQL → generate SQL, assert output == expected.
fn validate(sql: &str, expected: &str, dialect: Dialect) {
    let ast = parse(sql, dialect).unwrap_or_else(|e| panic!("Parse failed for '{}': {}", sql, e));
    let output = generate(&ast, dialect);
    assert_eq!(output, expected, "\n  Input: {}", sql);
}

// ═══════════════════════════════════════════════════════════════════════
// Basic double-quoted alias round-trip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_double_quoted_select_alias_roundtrip() {
    validate_identity(r#"SELECT 7 AS "Mixed""#, Dialect::Postgres);
}

#[test]
fn test_double_quoted_alias_in_subquery() {
    validate_identity(
        r#"SELECT "Mixed" FROM (SELECT 7 AS "Mixed") AS t"#,
        Dialect::Postgres,
    );
}

#[test]
fn test_double_quoted_alias_case_sensitive() {
    // This is the exact reproduction case from CR-005
    validate(
        r#"SELECT "Mixed" FROM (SELECT 7 AS "Mixed") t"#,
        r#"SELECT "Mixed" FROM (SELECT 7 AS "Mixed") AS t"#,
        Dialect::Postgres,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// CTE with double-quoted name
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_cte_quoted_name_roundtrip() {
    validate_identity(
        r#"WITH "MyCte" AS (SELECT 1 AS "Val") SELECT "Val" FROM "MyCte""#,
        Dialect::Postgres,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Reserved-word alias
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_reserved_word_quoted_alias() {
    validate(
        r#"SELECT count(*) AS "order" FROM orders"#,
        r#"SELECT COUNT(*) AS "order" FROM orders"#,
        Dialect::Postgres,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mixed quoting (some quoted, some bare)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_mixed_quoted_and_unquoted_aliases() {
    validate_identity(
        r#"SELECT a AS "First", b AS second FROM t"#,
        Dialect::Postgres,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Table alias with double quotes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_table_alias_quoted_roundtrip() {
    validate_identity(
        r#"SELECT "t"."Col" FROM my_table AS "t""#,
        Dialect::Postgres,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Subquery alias with double quotes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_subquery_alias_quoted_roundtrip() {
    validate_identity(
        r#"SELECT * FROM (SELECT 1) AS "sub query""#,
        Dialect::Postgres,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Cross-dialect transpile: PG → MySQL
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_cross_dialect_pg_to_mysql_alias() {
    let result = transpile(r#"SELECT 7 AS "Mixed""#, Dialect::Postgres, Dialect::Mysql).unwrap();
    assert_eq!(result, "SELECT 7 AS `Mixed`");
}

// ═══════════════════════════════════════════════════════════════════════
// Cross-dialect transpile: PG → T-SQL
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_cross_dialect_pg_to_tsql_alias() {
    let result = transpile(r#"SELECT 7 AS "Mixed""#, Dialect::Postgres, Dialect::Tsql).unwrap();
    assert_eq!(result, "SELECT 7 AS [Mixed]");
}

// ═══════════════════════════════════════════════════════════════════════
// Unquoted aliases (no regression)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_unquoted_aliases_unchanged() {
    validate_identity("SELECT 1 AS foo, 2 AS bar FROM t AS x", Dialect::Ansi);
}

// ═══════════════════════════════════════════════════════════════════════
// Backtick alias preservation (MySQL)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_backtick_alias_roundtrip_mysql() {
    validate_identity("SELECT 1 AS `Mixed`", Dialect::Mysql);
}

#[test]
fn test_backtick_to_doublequote_mysql_to_pg() {
    let result = transpile("SELECT 1 AS `Mixed`", Dialect::Mysql, Dialect::Postgres).unwrap();
    assert_eq!(result, r#"SELECT 1 AS "Mixed""#);
}

// ═══════════════════════════════════════════════════════════════════════
// Implicit (no AS keyword) quoted alias
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_implicit_quoted_alias() {
    // Some SQL allows: SELECT 1 "MyAlias" (without AS keyword)
    validate(
        r#"SELECT 1 "MyAlias""#,
        r#"SELECT 1 AS "MyAlias""#,
        Dialect::Postgres,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Single-quoted (string literal) alias - SQLite / MySQL / Snowflake
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_single_quoted_column_alias_normalizes_to_double() {
    validate(
        r#"SELECT record_id AS 'Record Id' FROM deals"#,
        r#"SELECT record_id AS "Record Id" FROM deals"#,
        Dialect::Sqlite,
    );
}

#[test]
fn test_single_quoted_table_alias_normalizes_to_double() {
    validate(
        r#"SELECT a FROM deals AS 't'"#,
        r#"SELECT a FROM deals AS "t""#,
        Dialect::Sqlite,
    );
}

#[test]
fn test_single_quoted_alias_with_quoted_column_ref() {
    validate(
        r#"SELECT "deals"."record_id" AS 'Record Id' FROM "deals""#,
        r#"SELECT "deals"."record_id" AS "Record Id" FROM "deals""#,
        Dialect::Sqlite,
    );
}

#[test]
fn test_single_quoted_alias_with_escaped_quote() {
    validate(
        r#"SELECT 1 AS 'It''s'"#,
        r#"SELECT 1 AS "It's""#,
        Dialect::Sqlite,
    );
}

#[test]
fn test_single_quoted_alias_transpile_to_mysql() {
    let result = transpile(r#"SELECT 7 AS 'Mixed'"#, Dialect::Sqlite, Dialect::Mysql).unwrap();
    assert_eq!(result, "SELECT 7 AS `Mixed`");
}

#[test]
fn test_string_literal_expression_not_treated_as_alias() {
    // A trailing string with no AS stays a string expression, never an alias.
    validate_identity(r#"SELECT 'a', 'b' FROM t"#, Dialect::Sqlite);
}
