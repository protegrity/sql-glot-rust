use sqlglot_rust::{generate, parse, transpile, Dialect};

// ═══════════════════════════════════════════════════════════════════════
// Basic roundtrip tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_roundtrip_select() {
    let sql = "SELECT a, b FROM t WHERE a > 1";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_insert() {
    let sql = "INSERT INTO users (name, age) VALUES ('Alice', 30)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_update() {
    let sql = "UPDATE users SET name = 'Bob' WHERE id = 1";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_delete() {
    let sql = "DELETE FROM users WHERE id = 1";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_create_table() {
    let sql = "CREATE TABLE IF NOT EXISTS items (id INT NOT NULL, name VARCHAR(100), price DECIMAL(10, 2))";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_drop_table() {
    let sql = "DROP TABLE IF EXISTS users";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_complex_query() {
    let sql = "SELECT u.id, u.name FROM users AS u INNER JOIN orders AS o ON u.id = o.user_id WHERE o.total > 100 ORDER BY u.name LIMIT 10";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

// ═══════════════════════════════════════════════════════════════════════
// Serialization
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_ast_serialization() {
    let ast = parse("SELECT 1", Dialect::Ansi).unwrap();
    let json = serde_json::to_string(&ast).unwrap();
    let deserialized: sqlglot_rust::Statement = serde_json::from_str(&json).unwrap();
    let output = generate(&deserialized, Dialect::Ansi);
    assert_eq!(output, "SELECT 1");
}

// ═══════════════════════════════════════════════════════════════════════
// CTEs
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_roundtrip_cte() {
    let sql = "WITH cte AS (SELECT 1 AS x) SELECT x FROM cte";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_recursive_cte() {
    let sql = "WITH RECURSIVE nums AS (SELECT 1 AS n) SELECT n FROM nums";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

// ═══════════════════════════════════════════════════════════════════════
// Set operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_roundtrip_union() {
    let sql = "SELECT 1 UNION ALL SELECT 2";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_intersect() {
    let sql = "SELECT a FROM t1 INTERSECT SELECT a FROM t2";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

// ═══════════════════════════════════════════════════════════════════════
// CAST, EXTRACT, EXISTS, Window functions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_roundtrip_cast() {
    let sql = "SELECT CAST(x AS INT) FROM t";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_extract() {
    let sql = "SELECT EXTRACT(YEAR FROM created_at) FROM t";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_exists() {
    let sql = "SELECT * FROM t WHERE EXISTS (SELECT 1 FROM t2)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_window_function() {
    let sql = "SELECT ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) FROM emp";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

// ═══════════════════════════════════════════════════════════════════════
// Subqueries
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_roundtrip_subquery_from() {
    let sql = "SELECT * FROM (SELECT 1 AS x) AS sub";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_in_subquery() {
    let sql = "SELECT * FROM t WHERE id IN (SELECT id FROM t2)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

// ═══════════════════════════════════════════════════════════════════════
// DDL
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_roundtrip_create_view() {
    let sql = "CREATE VIEW v AS SELECT * FROM t";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_drop_view() {
    let sql = "DROP VIEW IF EXISTS v";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

// ═══════════════════════════════════════════════════════════════════════
// Transaction statements
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_roundtrip_begin() {
    let ast = parse("BEGIN", Dialect::Ansi).unwrap();
    assert_eq!(generate(&ast, Dialect::Ansi), "BEGIN");
}

#[test]
fn test_roundtrip_commit() {
    let ast = parse("COMMIT", Dialect::Ansi).unwrap();
    assert_eq!(generate(&ast, Dialect::Ansi), "COMMIT");
}

// ═══════════════════════════════════════════════════════════════════════
// Transpile
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_transpile_identity() {
    let sql = "SELECT a FROM t WHERE a > 1";
    let result = transpile(sql, Dialect::Ansi, Dialect::Ansi).unwrap();
    assert_eq!(result, sql);
}

#[test]
fn test_transpile_function_mapping() {
    // SUBSTR → SUBSTRING when targeting Postgres
    let result = transpile(
        "SELECT SUBSTR(name, 1, 3) FROM users",
        Dialect::Mysql,
        Dialect::Postgres,
    )
    .unwrap();
    assert!(result.contains("SUBSTRING"));
}

// ═══════════════════════════════════════════════════════════════════════
// INSERT ... SELECT
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_roundtrip_insert_select() {
    let sql = "INSERT INTO t SELECT * FROM s";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

// ═══════════════════════════════════════════════════════════════════════
// ON CONFLICT
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_roundtrip_on_conflict_nothing() {
    let sql = "INSERT INTO t (id) VALUES (1) ON CONFLICT (id) DO NOTHING";
    let ast = parse(sql, Dialect::Postgres).unwrap();
    let output = generate(&ast, Dialect::Postgres);
    assert_eq!(output, sql);
}

// ═══════════════════════════════════════════════════════════════════════
// Optimizer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_optimizer_constant_folding() {
    let ast = parse("SELECT 1 + 2 FROM t", Dialect::Ansi).unwrap();
    let optimized = sqlglot_rust::optimizer::optimize(ast).unwrap();
    let output = generate(&optimized, Dialect::Ansi);
    assert_eq!(output, "SELECT 3 FROM t");
}

// ═══════════════════════════════════════════════════════════════════════
// AST traversal
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_find_columns() {
    let ast = parse("SELECT a, b, c FROM t WHERE a > 1", Dialect::Ansi).unwrap();
    if let sqlglot_rust::Statement::Select(sel) = &ast {
        if let Some(wh) = &sel.where_clause {
            let cols = sqlglot_rust::ast::find_columns(wh);
            assert_eq!(cols.len(), 1);
        }
    }
}

#[test]
fn test_find_tables() {
    let ast = parse(
        "SELECT * FROM users INNER JOIN orders ON users.id = orders.user_id",
        Dialect::Ansi,
    )
    .unwrap();
    let tables = sqlglot_rust::ast::find_tables(&ast);
    assert_eq!(tables.len(), 2);
}
