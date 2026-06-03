//! Regressions surfaced by sql-ast-benchmark (CR-012).
//!
//! Each test pins one gap from the benchmark report so future tokenizer /
//! parser changes don't silently regress acceptance on real-world corpora.

use sqlglot_rust::{Dialect, parse};

// ── Gap 1 — Unicode identifier characters ───────────────────────────────

#[test]
fn unicode_identifier_latin1() {
    parse("SELECT regionalliga_süd FROM t", Dialect::Postgres)
        .expect("Latin-1 letters must be accepted in identifiers");
}

#[test]
fn unicode_identifier_superscript() {
    parse("SELECT area_in_1000km² FROM t", Dialect::Sqlite)
        .expect("Unicode digits / superscripts in identifier tail must tokenize");
}

#[test]
fn unicode_identifier_curly_quote_continuation() {
    // Trailing underscore form from the SQLite corpus — leading char is ASCII.
    parse(
        "SELECT area_in_1000km²__1930_ FROM table_11654169_1",
        Dialect::Sqlite,
    )
    .expect("Mixed ASCII / superscript identifier must tokenize");
}

// ── Gap 3 — `$` inside identifiers ──────────────────────────────────────

#[test]
fn dollar_in_identifier() {
    parse("SELECT purse__$__ FROM t", Dialect::Sqlite)
        .expect("`$` is allowed mid-identifier in SQLite/MySQL/Oracle/T-SQL");
}

#[test]
fn dollar_parameter_still_works() {
    // `$1` at the start of a token must still be a parameter, not an identifier.
    parse("SELECT $1 FROM t", Dialect::Postgres).expect("`$1` is a parameter marker");
}

// ── Gap 2 (partial) — `SELECT ALL` quantifier ───────────────────────────

#[test]
fn select_all_quantifier() {
    parse("SELECT ALL col1 FROM t", Dialect::DuckDb)
        .expect("`SELECT ALL` (SQL:2003 §7.12) must be accepted");
}

#[test]
fn select_all_with_unary_plus() {
    // Real DuckDB corpus example: `SELECT ALL + tab.col / tab.col`.
    parse(
        "SELECT ALL + tab2.col1 / tab2.col1 FROM tab2 GROUP BY col1",
        Dialect::DuckDb,
    )
    .expect("`SELECT ALL` followed by a unary-plus expression must be accepted");
}

// ── Gap 9 — qualified LHS in `UPDATE … SET` ─────────────────────────────

#[test]
fn oracle_update_qualified_set_lhs() {
    parse(
        "UPDATE customers c SET c.email = 'x' WHERE c.id = 1",
        Dialect::Oracle,
    )
    .expect("Qualified `alias.col` on LHS of UPDATE … SET must be accepted");
}

#[test]
fn oracle_update_qualified_set_multi_assignment() {
    parse(
        "UPDATE customers c SET c.date_of_birth = '02-MAR-53', c.marital_status = 'single' WHERE c.customer_id = 102",
        Dialect::Oracle,
    )
    .expect("Multiple qualified assignments must round-trip");
}

// ── Gap 8 — Postgres `@>` / `<@` containment operators ─────────────────

#[test]
fn pg_array_contains() {
    parse(
        "SELECT * FROM t WHERE tags @> ARRAY['a', 'b']",
        Dialect::Postgres,
    )
    .expect("`@>` must parse as a binary operator");
}

#[test]
fn pg_array_contained_by() {
    parse(
        "SELECT * FROM t WHERE ARRAY['a'] <@ tags",
        Dialect::Postgres,
    )
    .expect("`<@` must parse as a binary operator");
}

// ── Gap 6 — aggregate ORDER BY / WITHIN GROUP ──────────────────────────

#[test]
fn array_agg_order_by() {
    parse(
        "SELECT array_agg(name ORDER BY id DESC) FROM t",
        Dialect::Postgres,
    )
    .expect("ORDER BY inside an aggregate arg list must be accepted");
}

#[test]
fn string_agg_within_group() {
    parse(
        "SELECT string_agg(name, ', ') WITHIN GROUP (ORDER BY id) FROM t",
        Dialect::Postgres,
    )
    .expect("WITHIN GROUP (ORDER BY ...) on aggregates must be accepted");
}

#[test]
fn percentile_cont_within_group() {
    parse(
        "SELECT percentile_cont(0.5) WITHIN GROUP (ORDER BY salary) FROM emp",
        Dialect::Postgres,
    )
    .expect("Ordered-set aggregate percentile_cont must be accepted");
}

// ── Gap 4 — raw-tail command statements ────────────────────────────────

#[test]
fn pg_set_command() {
    parse("SET search_path TO public, pg_catalog", Dialect::Postgres)
        .expect("`SET` must be accepted as a Command statement");
}

#[test]
fn pg_show_command() {
    parse("SHOW search_path", Dialect::Postgres).expect("`SHOW` must be accepted");
}

#[test]
fn pg_analyze_command() {
    parse("ANALYZE customers", Dialect::Postgres).expect("`ANALYZE` (standalone) must be accepted");
}

#[test]
fn tsql_go_separator() {
    parse("GO", Dialect::Tsql).expect("T-SQL batch separator `GO` must be accepted");
}

#[test]
fn pg_load_extension() {
    parse("LOAD 'plpgsql'", Dialect::Postgres).expect("`LOAD` must be accepted as a Command");
}

#[test]
fn pg_comment_on_table() {
    parse(
        "COMMENT ON TABLE customers IS 'all customers'",
        Dialect::Postgres,
    )
    .expect("`COMMENT ON ...` must be accepted as a Command");
}

#[test]
fn pg_create_operator_fallback() {
    parse(
        "CREATE OPERATOR === (LEFTARG = int, RIGHTARG = int, PROCEDURE = int4eq)",
        Dialect::Postgres,
    )
    .expect("Unknown `CREATE …` forms must fall back to a Command");
}

#[test]
fn sqlite_pragma() {
    parse("PRAGMA foreign_keys = ON", Dialect::Sqlite).expect("SQLite `PRAGMA` must be accepted");
}

// ── Gap 5 — vendor-specific ALTER TABLE tails ──────────────────────────

#[test]
fn mysql_alter_table_collate() {
    parse(
        "ALTER TABLE t CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci",
        Dialect::Mysql,
    )
    .expect("MySQL `ALTER TABLE … CONVERT TO CHARACTER SET …` must round-trip");
}

#[test]
fn hive_alter_partition_compact() {
    parse(
        "ALTER TABLE t PARTITION (dt='2024-01-01') COMPACT 'major'",
        Dialect::Hive,
    )
    .expect("Hive `ALTER TABLE … PARTITION … COMPACT …` must round-trip");
}

// ── Gap 7 — CREATE TABLE AS VALUES ─────────────────────────────────────

#[test]
fn ctas_values() {
    parse(
        "CREATE TABLE t AS VALUES (1, 'a'), (2, 'b')",
        Dialect::Postgres,
    )
    .expect("`CREATE TABLE … AS VALUES …` must round-trip");
}

// ── Gap 2 (full) — non-reserved keywords as identifiers ────────────────

#[test]
fn range_as_identifier() {
    parse("SELECT range FROM mountain", Dialect::Sqlite)
        .expect("`range` must be usable as a column identifier");
}

#[test]
fn conflict_as_identifier() {
    parse("SELECT conflict FROM t", Dialect::Sqlite)
        .expect("`conflict` must be usable as a column identifier");
}

#[test]
fn comment_as_identifier() {
    parse("SELECT COMMENT FROM hr.departments", Dialect::Oracle)
        .expect("`COMMENT` must be usable as a column identifier");
}

#[test]
fn show_as_identifier() {
    parse("SELECT show FROM t", Dialect::DuckDb)
        .expect("`show` must be usable as a column identifier");
}
