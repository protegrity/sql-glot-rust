/// Tests ported from Python sqlglot's `test_expressions.py`.
///
/// Covers AST construction, traversal (walk, find, find_all, transform),
/// helper methods (find_columns, find_tables, sql()), and equality.
use sqlglot_rust::ast::{SelectItem, find_columns, find_tables};
use sqlglot_rust::{generate, parse, Dialect, Expr, Statement};

// ═════════════════════════════════════════════════════════════════════════════
// AST traversal helpers
// (from Python test_expressions.py::test_find / test_walk / test_transform)
// ═════════════════════════════════════════════════════════════════════════════

fn parse_expr(sql: &str) -> Expr {
    let stmt = parse(sql, Dialect::Ansi).unwrap();
    match stmt {
        Statement::Select(s) => match &s.columns[0] {
            SelectItem::Expr { expr, .. } => expr.clone(),
            SelectItem::Wildcard => Expr::Star,
            SelectItem::QualifiedWildcard { table } => Expr::QualifiedWildcard {
                table: table.clone(),
            },
        },
        Statement::Expression(e) => e,
        _ => panic!("Expected SELECT or expression"),
    }
}

fn parse_stmt(sql: &str) -> Statement {
    parse(sql, Dialect::Ansi).unwrap()
}

#[test]
fn test_find_columns() {
    // find_columns operates on an Expr, so we test it on a WHERE clause expression
    let expr = parse_expr("SELECT a + b + c");
    let cols = find_columns(&expr);
    assert_eq!(cols.len(), 3);
}

#[test]
fn test_find_tables() {
    let stmt = parse_stmt("SELECT a FROM t1 INNER JOIN t2 ON t1.id = t2.id");
    let tables = find_tables(&stmt);
    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0].name, "t1");
    assert_eq!(tables[1].name, "t2");
}

#[test]
fn test_find_columns_qualified() {
    let expr = parse_expr("SELECT t.a");
    let cols = find_columns(&expr);
    // The entire `t.a` is a Column { name: "a", table: Some("t") }
    assert_eq!(cols.len(), 1);
}

// ═════════════════════════════════════════════════════════════════════════════
// Expression properties
// (from Python test_expressions.py::test_column / test_literal)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_column_is_column() {
    let expr = parse_expr("SELECT a");
    assert!(expr.is_column());
}

#[test]
fn test_literal_is_literal() {
    assert!(parse_expr("SELECT 1").is_literal());
    assert!(parse_expr("SELECT 'hello'").is_literal());
    assert!(parse_expr("SELECT 1.5").is_literal());
}

#[test]
fn test_boolean_is_literal() {
    assert!(parse_expr("SELECT TRUE").is_literal());
    assert!(parse_expr("SELECT FALSE").is_literal());
}

#[test]
fn test_null_properties() {
    let expr = parse_expr("SELECT NULL");
    assert!(matches!(expr, Expr::Null));
}

// ═════════════════════════════════════════════════════════════════════════════
// Expression walk
// (from Python test_expressions.py::test_walk)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_walk_collects_all_nodes() {
    let expr = parse_expr("SELECT 1 + 2 * 3");
    let mut nodes = Vec::new();
    expr.walk(&mut |e| {
        nodes.push(e.clone());
        true // continue visiting children
    });
    // Should have visited: BinaryOp(+), 1, BinaryOp(*), 2, 3
    assert!(nodes.len() >= 5);
}

#[test]
fn test_walk_column_references() {
    let expr = parse_expr("SELECT a + b + c");
    let mut col_names = Vec::new();
    expr.walk(&mut |e| {
        if let Expr::Column { name, .. } = e {
            col_names.push(name.clone());
        }
        true
    });
    assert_eq!(col_names.len(), 3);
    assert!(col_names.contains(&"a".to_string()));
    assert!(col_names.contains(&"b".to_string()));
    assert!(col_names.contains(&"c".to_string()));
}

// ═════════════════════════════════════════════════════════════════════════════
// Expression find
// (from Python test_expressions.py::test_find)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_find_number() {
    let expr = parse_expr("SELECT a + 1");
    let found = expr.find(&|e| matches!(e, Expr::Number(_)));
    assert!(found.is_some());
    if let Some(Expr::Number(n)) = found {
        assert_eq!(n, "1");
    }
}

#[test]
fn test_find_not_present() {
    let expr = parse_expr("SELECT a + b");
    let found = expr.find(&|e| matches!(e, Expr::Number(_)));
    assert!(found.is_none());
}

#[test]
fn test_find_all_numbers() {
    let expr = parse_expr("SELECT 1 + 2 + 3");
    let found = expr.find_all(&|e| matches!(e, Expr::Number(_)));
    assert_eq!(found.len(), 3);
}

// ═════════════════════════════════════════════════════════════════════════════
// Expression transform
// (from Python test_expressions.py::test_transform)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_transform_double_numbers() {
    let expr = parse_expr("SELECT 1 + 2");
    let transformed = expr.transform(&|e| {
        if let Expr::Number(n) = &e {
            let val: f64 = n.parse().unwrap();
            Expr::Number(((val * 2.0) as i64).to_string())
        } else {
            e
        }
    });
    let sql = transformed.sql();
    assert_eq!(sql, "2 + 4");
}

#[test]
fn test_transform_rename_columns() {
    let expr = parse_expr("SELECT a + b");
    let transformed = expr.transform(&|e| {
        if let Expr::Column { name, table, quote_style, table_quote_style } = &e {
            if name == "a" {
                Expr::Column {
                    name: "x".to_string(),
                    table: table.clone(),
                    quote_style: *quote_style,
                    table_quote_style: *table_quote_style,
                }
            } else {
                e
            }
        } else {
            e
        }
    });
    let sql = transformed.sql();
    assert_eq!(sql, "x + b");
}

// ═════════════════════════════════════════════════════════════════════════════
// Expr::sql() method
// (from Python test_expressions.py::test_sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_expr_sql() {
    assert_eq!(Expr::Number("42".to_string()).sql(), "42");
    assert_eq!(Expr::StringLiteral("hello".to_string()).sql(), "'hello'");
    assert_eq!(Expr::Boolean(true).sql(), "TRUE");
    assert_eq!(Expr::Boolean(false).sql(), "FALSE");
    assert_eq!(Expr::Null.sql(), "NULL");
}

#[test]
fn test_expr_sql_binary_op() {
    let expr = parse_expr("SELECT 1 + 2");
    assert_eq!(expr.sql(), "1 + 2");
}

#[test]
fn test_expr_sql_function() {
    let expr = parse_expr("SELECT COUNT(*)");
    assert_eq!(expr.sql(), "COUNT(*)");
}

// ═════════════════════════════════════════════════════════════════════════════
// Statement equality via SQL output
// (from Python test_expressions.py::test_eq)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_statement_equality_via_output() {
    let s1 = parse("SELECT a FROM b", Dialect::Ansi).unwrap();
    let s2 = parse("SELECT  a  FROM  b", Dialect::Ansi).unwrap();
    let out1 = generate(&s1, Dialect::Ansi);
    let out2 = generate(&s2, Dialect::Ansi);
    assert_eq!(out1, out2);
}

// ═════════════════════════════════════════════════════════════════════════════
// Nested expression handling
// (from Python test_expressions.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_nested_expression() {
    let stmt = parse_stmt("SELECT (a + b) * c FROM t");
    let out = generate(&stmt, Dialect::Ansi);
    assert_eq!(out, "SELECT (a + b) * c FROM t");
}

#[test]
fn test_deeply_nested_expression() {
    let stmt = parse_stmt("SELECT ((a + b) * (c - d)) / e FROM t");
    let out = generate(&stmt, Dialect::Ansi);
    assert_eq!(out, "SELECT ((a + b) * (c - d)) / e FROM t");
}

// ═════════════════════════════════════════════════════════════════════════════
// Alias handling
// (from Python test_expressions.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_alias_expression() {
    let stmt = parse_stmt("SELECT a + b AS sum_ab FROM t");
    let out = generate(&stmt, Dialect::Ansi);
    assert_eq!(out, "SELECT a + b AS sum_ab FROM t");
}

#[test]
fn test_alias_subquery() {
    let stmt = parse_stmt("SELECT * FROM (SELECT 1) AS sub");
    let out = generate(&stmt, Dialect::Ansi);
    assert_eq!(out, "SELECT * FROM (SELECT 1) AS sub");
}

#[test]
fn test_alias_table() {
    let stmt = parse_stmt("SELECT t1.a FROM table1 AS t1");
    let out = generate(&stmt, Dialect::Ansi);
    assert_eq!(out, "SELECT t1.a FROM table1 AS t1");
}

// ═════════════════════════════════════════════════════════════════════════════
// Complex AST tests
// (inspired by Python test_expressions.py edge cases)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_function_with_distinct() {
    let stmt = parse_stmt("SELECT COUNT(DISTINCT a) FROM t");
    let out = generate(&stmt, Dialect::Ansi);
    assert_eq!(out, "SELECT COUNT(DISTINCT a) FROM t");
}

#[test]
fn test_coalesce_expression() {
    let stmt = parse_stmt("SELECT COALESCE(a, b, 0) FROM t");
    let out = generate(&stmt, Dialect::Ansi);
    assert_eq!(out, "SELECT COALESCE(a, b, 0) FROM t");
}

#[test]
fn test_between_in_where() {
    let stmt = parse_stmt("SELECT * FROM t WHERE a BETWEEN 1 AND 10");
    let out = generate(&stmt, Dialect::Ansi);
    assert_eq!(out, "SELECT * FROM t WHERE a BETWEEN 1 AND 10");
}

#[test]
fn test_cast_expression() {
    let stmt = parse_stmt("SELECT CAST(a AS VARCHAR) FROM t");
    let out = generate(&stmt, Dialect::Ansi);
    assert_eq!(out, "SELECT CAST(a AS VARCHAR) FROM t");
}

#[test]
fn test_case_expression_full() {
    let stmt = parse_stmt(
        "SELECT CASE x WHEN 1 THEN 'a' WHEN 2 THEN 'b' ELSE 'c' END FROM t",
    );
    let out = generate(&stmt, Dialect::Ansi);
    assert_eq!(
        out,
        "SELECT CASE x WHEN 1 THEN 'a' WHEN 2 THEN 'b' ELSE 'c' END FROM t"
    );
}
