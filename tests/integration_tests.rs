use sqlglot_rust::schema::{MappingSchema, Schema, SchemaError, ensure_schema};
use sqlglot_rust::{Dialect, generate, parse, transpile};

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
    if let sqlglot_rust::Statement::Select(sel) = &ast
        && let Some(wh) = &sel.where_clause
    {
        let cols = sqlglot_rust::ast::find_columns(wh);
        assert_eq!(cols.len(), 1);
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

// ═══════════════════════════════════════════════════════════════════════
// Schema system integration tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_schema_from_create_table() {
    // Parse a CREATE TABLE and use it to populate a schema
    let sql =
        "CREATE TABLE products (id INT, name VARCHAR(100), price DECIMAL(10, 2), active BOOLEAN)";
    let ast = parse(sql, Dialect::Ansi).unwrap();

    let mut schema = MappingSchema::new(Dialect::Ansi);

    if let sqlglot_rust::ast::Statement::CreateTable(ct) = &ast {
        let columns: Vec<(String, sqlglot_rust::ast::DataType)> = ct
            .columns
            .iter()
            .map(|col| (col.name.clone(), col.data_type.clone()))
            .collect();
        schema.add_table(&[&ct.table.name], columns).unwrap();
    }

    assert_eq!(
        schema.column_names(&["products"]).unwrap(),
        vec!["id", "name", "price", "active"]
    );
    assert_eq!(
        schema.get_column_type(&["products"], "price").unwrap(),
        sqlglot_rust::ast::DataType::Decimal {
            precision: Some(10),
            scale: Some(2)
        }
    );
    assert!(schema.has_column(&["products"], "active"));
    assert!(!schema.has_column(&["products"], "nonexistent"));
}

#[test]
fn test_schema_validates_query_columns() {
    // Build a schema and verify that query columns can be validated
    let mut schema = MappingSchema::new(Dialect::Postgres);
    schema
        .add_table(
            &["users"],
            vec![
                ("id".into(), sqlglot_rust::ast::DataType::Int),
                ("name".into(), sqlglot_rust::ast::DataType::Text),
                ("email".into(), sqlglot_rust::ast::DataType::Text),
            ],
        )
        .unwrap();

    let ast = parse(
        "SELECT id, name, email FROM users WHERE id > 10",
        Dialect::Postgres,
    )
    .unwrap();
    if let sqlglot_rust::ast::Statement::Select(sel) = &ast {
        // Verify all selected columns exist in schema
        for item in &sel.columns {
            if let sqlglot_rust::ast::SelectItem::Expr {
                expr: sqlglot_rust::ast::Expr::Column { name, .. },
                ..
            } = item
            {
                assert!(
                    schema.has_column(&["users"], name),
                    "Column {name} should exist in schema"
                );
            }
        }
    }
}

#[test]
fn test_schema_cross_dialect_normalization() {
    use std::collections::HashMap;

    // Build schema with ensure_schema helper
    let mut tables = HashMap::new();
    let mut cols = HashMap::new();
    cols.insert("UserId".to_string(), sqlglot_rust::ast::DataType::Int);
    cols.insert("UserName".to_string(), sqlglot_rust::ast::DataType::Text);
    tables.insert("UserAccounts".to_string(), cols);

    // Postgres: case-insensitive
    let pg_schema = ensure_schema(tables.clone(), Dialect::Postgres);
    assert!(pg_schema.has_column(&["useraccounts"], "userid"));
    assert!(pg_schema.has_column(&["USERACCOUNTS"], "USERNAME"));

    // BigQuery: case-sensitive
    let bq_schema = ensure_schema(tables, Dialect::BigQuery);
    assert!(bq_schema.has_column(&["UserAccounts"], "UserId"));
    assert!(!bq_schema.has_column(&["useraccounts"], "userid"));
}

#[test]
fn test_schema_duplicate_and_replace() {
    let mut schema = MappingSchema::new(Dialect::Ansi);
    schema
        .add_table(&["t"], vec![("a".into(), sqlglot_rust::ast::DataType::Int)])
        .unwrap();

    // Duplicate should fail
    let result = schema.add_table(
        &["t"],
        vec![("b".into(), sqlglot_rust::ast::DataType::Text)],
    );
    assert!(matches!(result, Err(SchemaError::DuplicateTable(_))));

    // Replace should succeed
    schema
        .replace_table(
            &["t"],
            vec![("b".into(), sqlglot_rust::ast::DataType::Text)],
        )
        .unwrap();
    assert!(schema.has_column(&["t"], "b"));
    assert!(!schema.has_column(&["t"], "a"));
}

#[test]
fn test_schema_udf_types() {
    let mut schema = MappingSchema::new(Dialect::Ansi);
    schema.add_udf("calculate_score", sqlglot_rust::ast::DataType::Double);
    schema.add_udf("format_name", sqlglot_rust::ast::DataType::Text);

    assert_eq!(
        schema.get_udf_type("calculate_score").unwrap(),
        &sqlglot_rust::ast::DataType::Double,
    );
    // Case-insensitive lookup in ANSI
    assert_eq!(
        schema.get_udf_type("CALCULATE_SCORE").unwrap(),
        &sqlglot_rust::ast::DataType::Double,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// GROUPING SETS, CUBE, ROLLUP tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_roundtrip_group_by_cube() {
    let sql = "SELECT a, b, SUM(c) FROM t GROUP BY CUBE(a, b)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_group_by_rollup() {
    let sql = "SELECT a, b, SUM(c) FROM t GROUP BY ROLLUP(a, b)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_group_by_grouping_sets() {
    let sql = "SELECT a, b, SUM(c) FROM t GROUP BY GROUPING SETS((a, b), (a), ())";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_group_by_cube_single_column() {
    let sql = "SELECT a, COUNT(*) FROM t GROUP BY CUBE(a)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_group_by_rollup_three_columns() {
    let sql = "SELECT a, b, c, SUM(d) FROM t GROUP BY ROLLUP(a, b, c)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_grouping_sets_with_empty_set() {
    let sql = "SELECT a, SUM(b) FROM t GROUP BY GROUPING SETS((a), ())";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_nested_grouping_sets_with_rollup() {
    let sql = "SELECT a, b, c, SUM(d) FROM t GROUP BY GROUPING SETS(ROLLUP(a, b), CUBE(c))";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_mixed_group_by_and_cube() {
    let sql = "SELECT a, b, c, SUM(d) FROM t GROUP BY a, CUBE(b, c)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_mixed_group_by_and_rollup() {
    let sql = "SELECT a, b, c, SUM(d) FROM t GROUP BY a, ROLLUP(b, c)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_grouping_function() {
    let sql = "SELECT a, b, GROUPING(a), SUM(c) FROM t GROUP BY CUBE(a, b)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_grouping_function_multiple_args() {
    let sql = "SELECT a, b, GROUPING(a, b), SUM(c) FROM t GROUP BY CUBE(a, b)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_cube_with_having() {
    let sql = "SELECT a, b, SUM(c) FROM t GROUP BY CUBE(a, b) HAVING SUM(c) > 10";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_rollup_with_order_by() {
    let sql = "SELECT a, b, SUM(c) FROM t GROUP BY ROLLUP(a, b) ORDER BY a";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_transpile_cube_across_dialects() {
    let sql = "SELECT a, b, SUM(c) FROM t GROUP BY CUBE(a, b)";
    for dialect in &[
        Dialect::Postgres,
        Dialect::Snowflake,
        Dialect::BigQuery,
        Dialect::Spark,
    ] {
        let result = transpile(sql, Dialect::Ansi, *dialect).unwrap();
        assert!(
            result.contains("CUBE"),
            "CUBE should be present in {:?} output: {}",
            dialect,
            result
        );
    }
}

#[test]
fn test_transpile_rollup_across_dialects() {
    let sql = "SELECT a, b, SUM(c) FROM t GROUP BY ROLLUP(a, b)";
    for dialect in &[
        Dialect::Postgres,
        Dialect::Snowflake,
        Dialect::BigQuery,
        Dialect::Spark,
    ] {
        let result = transpile(sql, Dialect::Ansi, *dialect).unwrap();
        assert!(
            result.contains("ROLLUP"),
            "ROLLUP should be present in {:?} output: {}",
            dialect,
            result
        );
    }
}

#[test]
fn test_transpile_grouping_sets_across_dialects() {
    let sql = "SELECT a, b, SUM(c) FROM t GROUP BY GROUPING SETS((a, b), (a), ())";
    for dialect in &[
        Dialect::Postgres,
        Dialect::Snowflake,
        Dialect::BigQuery,
        Dialect::Spark,
    ] {
        let result = transpile(sql, Dialect::Ansi, *dialect).unwrap();
        assert!(
            result.contains("GROUPING SETS"),
            "GROUPING SETS should be present in {:?} output: {}",
            dialect,
            result
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// MERGE statement tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_roundtrip_merge_basic() {
    let sql = "MERGE INTO target AS t USING source AS s ON t.id = s.id WHEN MATCHED THEN UPDATE SET t.name = s.name WHEN NOT MATCHED THEN INSERT (id, name) VALUES (s.id, s.name)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_merge_with_delete() {
    let sql = "MERGE INTO target USING source ON target.id = source.id WHEN MATCHED THEN DELETE";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_merge_multiple_when_clauses() {
    let sql = "MERGE INTO inventory AS inv USING shipments AS s ON inv.product_id = s.product_id WHEN MATCHED AND s.quantity > 0 THEN UPDATE SET inv.quantity = inv.quantity + s.quantity WHEN MATCHED AND s.quantity = 0 THEN DELETE WHEN NOT MATCHED THEN INSERT (product_id, quantity) VALUES (s.product_id, s.quantity)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_merge_not_matched_by_source() {
    // T-SQL extension: WHEN NOT MATCHED BY SOURCE
    let sql = "MERGE INTO target USING source ON target.id = source.id WHEN NOT MATCHED BY SOURCE THEN DELETE";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_merge_insert_row() {
    // BigQuery extension: INSERT ROW
    let sql =
        "MERGE INTO target USING source ON target.id = source.id WHEN NOT MATCHED THEN INSERT ROW";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_merge_subquery_source() {
    let sql = "MERGE INTO target USING (SELECT id, name FROM staging) AS s ON target.id = s.id WHEN MATCHED THEN UPDATE SET target.name = s.name";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_roundtrip_merge_without_into() {
    // MERGE without INTO keyword (both forms should work)
    let sql = "MERGE INTO target USING source ON target.id = source.id WHEN MATCHED THEN DELETE";
    let ast_with = parse(sql, Dialect::Ansi).unwrap();
    let sql_without = "MERGE target USING source ON target.id = source.id WHEN MATCHED THEN DELETE";
    let ast_without = parse(sql_without, Dialect::Ansi).unwrap();
    // Both parse the same and generate with INTO
    let output_with = generate(&ast_with, Dialect::Ansi);
    let output_without = generate(&ast_without, Dialect::Ansi);
    assert_eq!(output_with, output_without);
}

#[test]
fn test_merge_ast_structure() {
    let sql = "MERGE INTO dst USING src ON dst.id = src.id WHEN MATCHED THEN UPDATE SET dst.val = src.val WHEN NOT MATCHED THEN INSERT (id, val) VALUES (src.id, src.val)";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    match &ast {
        sqlglot_rust::Statement::Merge(m) => {
            assert_eq!(m.target.name, "dst");
            assert_eq!(m.clauses.len(), 2);
            assert_eq!(m.clauses[0].kind, sqlglot_rust::MergeClauseKind::Matched);
            assert_eq!(m.clauses[1].kind, sqlglot_rust::MergeClauseKind::NotMatched);
        }
        other => panic!("Expected Merge statement, got {:?}", other),
    }
}

#[test]
fn test_merge_serialization() {
    let sql = "MERGE INTO target USING source ON target.id = source.id WHEN MATCHED THEN UPDATE SET target.name = source.name";
    let ast = parse(sql, Dialect::Ansi).unwrap();
    let json = serde_json::to_string(&ast).unwrap();
    let deserialized: sqlglot_rust::Statement = serde_json::from_str(&json).unwrap();
    let output = generate(&deserialized, Dialect::Ansi);
    assert_eq!(output, sql);
}

#[test]
fn test_merge_transpile_across_dialects() {
    let sql = "MERGE INTO target USING source ON target.id = source.id WHEN MATCHED THEN UPDATE SET target.name = source.name WHEN NOT MATCHED THEN INSERT (id, name) VALUES (source.id, source.name)";
    for dialect in &[
        Dialect::Ansi,
        Dialect::Snowflake,
        Dialect::BigQuery,
        Dialect::Tsql,
        Dialect::Spark,
        Dialect::Databricks,
        Dialect::Postgres,
    ] {
        let result = transpile(sql, Dialect::Ansi, *dialect)
            .unwrap_or_else(|e| panic!("Transpile to {:?} failed: {}", dialect, e));
        assert!(
            result.contains("MERGE INTO"),
            "MERGE INTO should be present in {:?} output: {}",
            dialect,
            result
        );
    }
}
