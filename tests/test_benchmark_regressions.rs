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
