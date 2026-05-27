/// Tests for SQL comment attachment to AST nodes.
///
/// Covers comment types (single-line --, block /* */, MySQL #), comment
/// preservation through parse -> generate roundtrip, and comment survival
/// through AST transformations.
use sqlglot_rust::generator::Generator;
use sqlglot_rust::{
    Dialect, Expr, Statement, generate, generate_pretty, parse_statements_with_comments,
    parse_with_comments,
};

// ═════════════════════════════════════════════════════════════════════════════
// Helpers
// ═════════════════════════════════════════════════════════════════════════════

fn roundtrip(sql: &str) -> String {
    let stmt = parse_with_comments(sql, Dialect::Ansi).unwrap();
    generate(&stmt, Dialect::Ansi)
}

fn roundtrip_dialect(sql: &str, read: Dialect, write: Dialect) -> String {
    let stmt = parse_with_comments(sql, read).unwrap();
    generate(&stmt, write)
}

fn get_comments(sql: &str) -> Vec<String> {
    let stmt = parse_with_comments(sql, Dialect::Ansi).unwrap();
    match stmt {
        Statement::Select(s) => s.comments,
        Statement::Insert(s) => s.comments,
        Statement::Update(s) => s.comments,
        Statement::Delete(s) => s.comments,
        Statement::CreateTable(s) => s.comments,
        _ => vec![],
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Single-line comment tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_line_comment_before_select() {
    let sql = "-- this is a comment\nSELECT a FROM t";
    let comments = get_comments(sql);
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0], "-- this is a comment");
}

#[test]
fn test_line_comment_roundtrip() {
    let sql = "-- query for users\nSELECT a FROM t";
    let result = roundtrip(sql);
    assert!(result.contains("-- query for users"));
    assert!(result.contains("SELECT a FROM t"));
}

#[test]
fn test_multiple_line_comments() {
    let sql = "-- first comment\n-- second comment\nSELECT a FROM t";
    let comments = get_comments(sql);
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0], "-- first comment");
    assert_eq!(comments[1], "-- second comment");
}

// ═════════════════════════════════════════════════════════════════════════════
// Block comment tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_block_comment_before_select() {
    let sql = "/* this is a block comment */ SELECT a FROM t";
    let comments = get_comments(sql);
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0], "/* this is a block comment */");
}

#[test]
fn test_block_comment_roundtrip() {
    let sql = "/* block */ SELECT a FROM t";
    let result = roundtrip(sql);
    assert!(result.contains("/* block */"));
    assert!(result.contains("SELECT a FROM t"));
}

// ═════════════════════════════════════════════════════════════════════════════
// MySQL hash comment tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_hash_comment_parsed() {
    let sql = "# mysql comment\nSELECT a FROM t";
    let comments = get_comments(sql);
    assert_eq!(comments.len(), 1);
    assert!(comments[0].starts_with('#'));
}

#[test]
fn test_hash_comment_converted_to_line_comment_on_transpile() {
    // When targeting non-MySQL dialects, # should become --
    let sql = "# mysql comment\nSELECT a FROM t";
    let result = roundtrip_dialect(sql, Dialect::Mysql, Dialect::Postgres);
    assert!(
        result.contains("-- mysql comment"),
        "Expected -- comment but got: {result}"
    );
    assert!(!result.contains('#'));
}

#[test]
fn test_hash_comment_preserved_for_mysql_target() {
    let sql = "# mysql comment\nSELECT a FROM t";
    let result = roundtrip_dialect(sql, Dialect::Mysql, Dialect::Mysql);
    assert!(result.contains("# mysql comment"));
}

// ═════════════════════════════════════════════════════════════════════════════
// Comment-free parsing (backwards compatibility)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_default_parse_ignores_comments() {
    // The default parse() should not include comments
    let sql = "-- comment\nSELECT a FROM t";
    let stmt = sqlglot_rust::parse(sql, Dialect::Ansi).unwrap();
    match stmt {
        Statement::Select(s) => assert!(s.comments.is_empty()),
        _ => panic!("Expected SELECT"),
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Comment preservation through transformations
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_comments_survive_generate() {
    let sql = "-- important query\nSELECT a, b FROM t WHERE a > 1";
    let stmt = parse_with_comments(sql, Dialect::Ansi).unwrap();
    let output = generate(&stmt, Dialect::Ansi);
    assert!(output.contains("-- important query"));
    assert!(output.contains("SELECT a, b FROM t WHERE a > 1"));
}

#[test]
fn test_comments_survive_pretty_generate() {
    let sql = "-- important query\nSELECT a, b FROM t WHERE a > 1";
    let stmt = parse_with_comments(sql, Dialect::Ansi).unwrap();
    let output = generate_pretty(&stmt, Dialect::Ansi);
    assert!(output.contains("-- important query"));
    assert!(output.contains("SELECT"));
}

// ═════════════════════════════════════════════════════════════════════════════
// Multiple statement comment tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_multi_statement_comments() {
    let sql = "-- first\nSELECT 1; -- second\nSELECT 2";
    let stmts = parse_statements_with_comments(sql, Dialect::Ansi).unwrap();
    assert_eq!(stmts.len(), 2);
    match &stmts[0] {
        Statement::Select(s) => {
            assert_eq!(s.comments.len(), 1);
            assert_eq!(s.comments[0], "-- first");
        }
        _ => panic!("Expected SELECT"),
    }
    match &stmts[1] {
        Statement::Select(s) => {
            assert_eq!(s.comments.len(), 1);
            assert_eq!(s.comments[0], "-- second");
        }
        _ => panic!("Expected SELECT"),
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// DML statement comments
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_insert_comment() {
    let sql = "-- insert data\nINSERT INTO t (a) VALUES (1)";
    let comments = get_comments(sql);
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0], "-- insert data");
}

#[test]
fn test_update_comment() {
    let sql = "/* update */ UPDATE t SET a = 1";
    let stmt = parse_with_comments(sql, Dialect::Ansi).unwrap();
    match stmt {
        Statement::Update(u) => {
            assert_eq!(u.comments.len(), 1);
            assert_eq!(u.comments[0], "/* update */");
        }
        _ => panic!("Expected UPDATE"),
    }
}

#[test]
fn test_delete_comment() {
    let sql = "-- remove rows\nDELETE FROM t WHERE a = 1";
    let stmt = parse_with_comments(sql, Dialect::Ansi).unwrap();
    match stmt {
        Statement::Delete(d) => {
            assert_eq!(d.comments.len(), 1);
            assert_eq!(d.comments[0], "-- remove rows");
        }
        _ => panic!("Expected DELETE"),
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// DDL statement comments
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_create_table_comment() {
    let sql = "-- create users table\nCREATE TABLE users (id INT)";
    let stmt = parse_with_comments(sql, Dialect::Ansi).unwrap();
    match stmt {
        Statement::CreateTable(ct) => {
            assert_eq!(ct.comments.len(), 1);
            assert_eq!(ct.comments[0], "-- create users table");
        }
        _ => panic!("Expected CREATE TABLE"),
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Expression-level comment (Commented variant)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_commented_expr_walk() {
    let commented = Expr::Commented {
        expr: Box::new(Expr::Column {
            table: None,
            name: "a".into(),
            quote_style: sqlglot_rust::QuoteStyle::None,
            table_quote_style: sqlglot_rust::QuoteStyle::None,
        }),
        comments: vec!["-- test".into()],
    };
    let mut found = false;
    commented.walk(&mut |e| {
        if matches!(e, Expr::Column { name, .. } if name == "a") {
            found = true;
        }
        true
    });
    assert!(
        found,
        "Walk should traverse through Commented to inner expr"
    );
}

#[test]
fn test_commented_expr_transform() {
    let commented = Expr::Commented {
        expr: Box::new(Expr::Number("1".into())),
        comments: vec!["-- original".into()],
    };
    let transformed = commented.transform(&|e| match e {
        Expr::Number(n) if n == "1" => Expr::Number("42".into()),
        other => other,
    });
    match transformed {
        Expr::Commented { expr, comments } => {
            assert_eq!(*expr, Expr::Number("42".into()));
            assert_eq!(comments, vec!["-- original"]);
        }
        _ => panic!("Expected Commented variant to be preserved"),
    }
}

#[test]
fn test_commented_expr_find() {
    let commented = Expr::Commented {
        expr: Box::new(Expr::Column {
            table: None,
            name: "x".into(),
            quote_style: sqlglot_rust::QuoteStyle::None,
            table_quote_style: sqlglot_rust::QuoteStyle::None,
        }),
        comments: vec!["-- note".into()],
    };
    let result = commented.find(&|e| matches!(e, Expr::Column { name, .. } if name == "x"));
    assert!(
        result.is_some(),
        "find should locate column inside Commented"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Comment generation
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_generate_commented_expr() {
    let commented = Expr::Commented {
        expr: Box::new(Expr::Column {
            table: None,
            name: "a".into(),
            quote_style: sqlglot_rust::QuoteStyle::None,
            table_quote_style: sqlglot_rust::QuoteStyle::None,
        }),
        comments: vec!["/* hint */".into()],
    };
    let sql = Generator::expr_to_sql(&commented);
    assert_eq!(sql, "/* hint */ a");
}

// ═════════════════════════════════════════════════════════════════════════════
// Mixed comment types
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_mixed_comment_types() {
    let sql = "-- line\n/* block */ SELECT a FROM t";
    let comments = get_comments(sql);
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0], "-- line");
    assert_eq!(comments[1], "/* block */");
}

// ═════════════════════════════════════════════════════════════════════════════
// Edge cases
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_no_comments() {
    let sql = "SELECT a FROM t";
    let comments = get_comments(sql);
    assert!(comments.is_empty());
}

#[test]
fn test_empty_block_comment() {
    let sql = "/**/ SELECT a FROM t";
    let comments = get_comments(sql);
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0], "/**/");
}

#[test]
fn test_comment_with_special_characters() {
    let sql = "-- comment with 'quotes' and \"double\" and $pecial\nSELECT 1";
    let comments = get_comments(sql);
    assert_eq!(comments.len(), 1);
    assert!(comments[0].contains("'quotes'"));
}
