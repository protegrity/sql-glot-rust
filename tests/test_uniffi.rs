#![cfg(feature = "uniffi")]
//! Integration tests for the UniFFI facade, exercised through its Rust API.
//! Generated-binding tests live in `tests/uniffi/` (run `make uniffi-test`).

use sqlglot_rust::Dialect;
use sqlglot_rust::uniffi_api::{
    SqlStatement, SqlglotError, parse, parse_statements, transpile, version,
};

#[test]
fn parse_then_generate_applies_target_quoting() {
    let statement = parse("SELECT `select` FROM `events`".into(), Dialect::Mysql).unwrap();
    assert_eq!(
        statement.generate(Dialect::Tsql).unwrap(),
        "SELECT [select] FROM [events]"
    );
}

#[test]
fn statement_wraps_the_same_ast_as_the_core_parser() {
    let sql = "SELECT a, b FROM t WHERE a > 1";
    let statement = parse(sql.into(), Dialect::Postgres).unwrap();
    assert_eq!(
        statement.statement(),
        &sqlglot_rust::parse(sql, Dialect::Postgres).unwrap()
    );
}

#[test]
fn generate_pretty_is_dialect_aware() {
    let statement = parse("SELECT a, b FROM t WHERE a > 1".into(), Dialect::Postgres).unwrap();
    assert_eq!(
        statement.generate_pretty(Dialect::Postgres).unwrap(),
        "SELECT\n  a,\n  b\nFROM\n  t\nWHERE\n  a > 1"
    );
}

#[test]
fn transpile_applies_dialect_rewrites_that_generate_does_not() {
    let sql = "SELECT * FROM t LIMIT 10";
    assert_eq!(
        transpile(sql.into(), Dialect::Mysql, Dialect::Tsql).unwrap(),
        "SELECT TOP 10 * FROM t"
    );
    assert_eq!(
        parse(sql.into(), Dialect::Mysql)
            .unwrap()
            .generate(Dialect::Tsql)
            .unwrap(),
        "SELECT * FROM t LIMIT 10"
    );
}

#[test]
fn parse_statements_returns_one_handle_per_statement() {
    let statements = parse_statements("SELECT 1; SELECT 2".into(), Dialect::Ansi).unwrap();
    let generated: Vec<String> = statements
        .iter()
        .map(|statement| statement.generate(Dialect::Ansi).unwrap())
        .collect();
    assert_eq!(generated, ["SELECT 1", "SELECT 2"]);
}

#[test]
fn ast_json_roundtrips_and_edits_are_honoured() {
    let statement = parse("SELECT a, b FROM t WHERE a > 1".into(), Dialect::Postgres).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&statement.to_json().unwrap()).unwrap();
    assert_eq!(json["Select"]["columns"].as_array().unwrap().len(), 2);
    assert_eq!(json["Select"]["from"]["source"]["Table"]["name"], "t");

    let unchanged = SqlStatement::from_json(json.to_string()).unwrap();
    assert_eq!(unchanged.statement(), statement.statement());

    json["Select"]["from"]["source"]["Table"]["name"] = "renamed".into();
    let edited = SqlStatement::from_json(json.to_string()).unwrap();
    assert_eq!(
        edited.generate(Dialect::Postgres).unwrap(),
        "SELECT a, b FROM renamed WHERE a > 1"
    );
}

#[test]
fn ast_json_matches_the_c_ffi_schema() {
    let statement = parse("SELECT 1".into(), Dialect::Ansi).unwrap();
    assert_eq!(
        statement.to_json().unwrap(),
        serde_json::to_string(statement.statement()).unwrap()
    );
}

#[test]
fn parse_errors_are_structured() {
    assert!(matches!(
        parse("SELECT FROM WHERE".into(), Dialect::Ansi),
        Err(SqlglotError::Parser { .. })
    ));
    assert_eq!(
        parse("SELECT 'abc".into(), Dialect::Ansi).unwrap_err(),
        SqlglotError::Tokenizer {
            detail: "Unterminated string literal".into(),
            position: 7,
        }
    );
    assert!(matches!(
        parse_statements("SELECT 1; SELECT FROM".into(), Dialect::Ansi),
        Err(SqlglotError::Parser { .. })
    ));
    assert!(matches!(
        transpile("SELECT FROM".into(), Dialect::Ansi, Dialect::Tsql),
        Err(SqlglotError::Parser { .. })
    ));
}

#[test]
fn generation_rejects_constructs_the_target_cannot_express() {
    let statement = parse("SELECT ARRAY[1, 2]".into(), Dialect::Postgres).unwrap();
    let expected = SqlglotError::UnsupportedDialectFeature {
        detail: "ARRAY constructor has no T-SQL equivalent".into(),
    };
    assert_eq!(statement.generate(Dialect::Tsql).unwrap_err(), expected);
    assert_eq!(
        statement.generate_pretty(Dialect::Tsql).unwrap_err(),
        expected
    );
    assert_eq!(
        statement.generate(Dialect::Postgres).unwrap(),
        "SELECT ARRAY[1, 2]"
    );
}

#[test]
fn invalid_ast_json_is_rejected() {
    for json in ["", "not json", r#"{"Nope": 1}"#, r#"{"Select": {}}"#] {
        assert!(
            matches!(
                SqlStatement::from_json(json.into()),
                Err(SqlglotError::InvalidAst { .. })
            ),
            "{json:?} must be rejected"
        );
    }
}

#[test]
fn version_is_the_crate_version() {
    assert_eq!(version(), env!("CARGO_PKG_VERSION"));
}
