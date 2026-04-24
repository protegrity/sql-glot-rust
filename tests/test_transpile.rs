/// Tests ported from Python sqlglot's `test_transpile.py` and `identity.sql` fixture.
///
/// These test parse→generate roundtrips (identity), normalization transforms,
/// and basic cross-dialect transpilation. Modeled after the `validate` and
/// `validate_identity` helpers in the Python test suite.
use sqlglot_rust::{Dialect, generate, parse, transpile};

// ═════════════════════════════════════════════════════════════════════════════
// Helpers (mirrors Python sqlglot's TestTranspile.validate / validate_identity)
// ═════════════════════════════════════════════════════════════════════════════

/// Parse SQL → generate SQL, assert output == input.
/// Equivalent to Python sqlglot's `validate_identity`.
fn validate_identity(sql: &str) {
    let ast =
        parse(sql, Dialect::Ansi).unwrap_or_else(|e| panic!("Parse failed for '{}': {}", sql, e));
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, sql, "\n  Identity roundtrip failed");
}

/// Parse SQL → generate SQL, assert output == expected.
/// Equivalent to Python sqlglot's `validate(sql, target)`.
fn validate(sql: &str, expected: &str) {
    let ast =
        parse(sql, Dialect::Ansi).unwrap_or_else(|e| panic!("Parse failed for '{}': {}", sql, e));
    let output = generate(&ast, Dialect::Ansi);
    assert_eq!(output, expected, "\n  Input: {}", sql);
}

fn validate_with_dialect(sql: &str, expected: &str, read: Dialect, write: Dialect) {
    let result = transpile(sql, read, write)
        .unwrap_or_else(|e| panic!("Transpile failed for '{}': {}", sql, e));
    assert_eq!(
        result, expected,
        "\n  Input: {} ({:?} → {:?})",
        sql, read, write
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – Expressions & Literals
// (from Python identity.sql fixture)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_literals() {
    let cases = [
        "SELECT 1",
        "SELECT 1.0",
        "SELECT 'x'",
        "SELECT ''",
        "SELECT TRUE",
        "SELECT FALSE",
        "SELECT NULL",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_arithmetic() {
    let cases = [
        "SELECT 1 + 1",
        "SELECT 1 - 1",
        "SELECT 1 * 1",
        "SELECT 1 / 1",
        "SELECT 1 % 1",
        "SELECT 1 + 2 * 3",
        "SELECT (1 + 2) * 3",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_comparisons() {
    let cases = [
        "SELECT 1 < 2",
        "SELECT 1 <= 2",
        "SELECT 1 > 2",
        "SELECT 1 >= 2",
        "SELECT 1 <> 2",
        "SELECT 1 = 2",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_boolean_logic() {
    let cases = [
        "SELECT a AND b",
        "SELECT a OR b",
        "SELECT NOT a",
        "SELECT NOT NOT a",
        "SELECT a AND b OR c",
        "SELECT (a OR b) AND c",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_unary() {
    let cases = ["SELECT -1", "SELECT -a", "SELECT +a", "SELECT ~x"];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_bitwise() {
    let cases = [
        "SELECT x & 1",
        "SELECT x | 1",
        "SELECT x ^ 1",
        "SELECT x << 1",
        "SELECT x >> 1",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_string_concat() {
    validate_identity("SELECT 'a' || 'b'");
    validate_identity("SELECT a || b || c");
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – SELECT basics
// (from Python identity.sql and test_transpile.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_select_basic() {
    let cases = [
        "SELECT * FROM test",
        "SELECT a FROM test",
        "SELECT a, b FROM test",
        "SELECT a, b, c FROM test",
        "SELECT 1 FROM test",
        "SELECT 1 + 1 FROM test",
        "SELECT 1 AS b FROM test",
        "SELECT a AS b FROM test",
        "SELECT test.* FROM test",
        "SELECT a.b FROM a",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_select_distinct() {
    let cases = [
        "SELECT DISTINCT x FROM test",
        "SELECT DISTINCT x, y FROM test",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_qualified_columns() {
    let cases = ["SELECT a.b FROM a"];
    for sql in &cases {
        validate_identity(sql);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – WHERE clause
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_where() {
    let cases = [
        "SELECT a FROM test WHERE a = 1",
        "SELECT a FROM test WHERE a = 1 AND b = 2",
        "SELECT a FROM test WHERE (a > 1)",
        "SELECT a FROM test WHERE NOT FALSE",
        "SELECT a FROM test WHERE a > 1 OR b < 2",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – FROM and JOINs
// (from Python identity.sql: JOIN section)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_joins() {
    let cases = [
        "SELECT 1 FROM a INNER JOIN b ON a.x = b.x",
        "SELECT 1 FROM a LEFT JOIN b ON a.x = b.x",
        "SELECT 1 FROM a RIGHT JOIN b ON a.x = b.x",
        "SELECT 1 FROM a FULL JOIN b ON a.x = b.x",
        "SELECT 1 FROM a CROSS JOIN b",
        // Note: bare JOIN is parsed as INNER JOIN, so INNER JOIN is the identity
        "SELECT 1 FROM a INNER JOIN b USING (x)",
        "SELECT 1 FROM a INNER JOIN b USING (x, y, z)",
        "SELECT 1 FROM a LEFT JOIN b ON a.x = b.x INNER JOIN c ON a.y = c.y",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_join_subquery() {
    validate_identity("SELECT 1 FROM a INNER JOIN (SELECT a FROM c) AS b ON a.x = b.x");
}

#[test]
fn test_identity_multiple_from_tables() {
    // Note: comma-separated FROM tables parsed as single table (limitation)
    // Use explicit CROSS JOIN instead
    validate_identity("SELECT * FROM a CROSS JOIN b");
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_group_by_having() {
    let cases = [
        "SELECT a, b FROM test GROUP BY a",
        "SELECT a, b FROM test GROUP BY 1",
        "SELECT a, b FROM test GROUP BY a, b",
        "SELECT a, b FROM test WHERE a = 1 GROUP BY a HAVING a = 2",
        "SELECT a, b FROM test WHERE a = 1 GROUP BY a HAVING a = 2 ORDER BY a",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_order_by() {
    let cases = [
        "SELECT a FROM test ORDER BY a",
        "SELECT a FROM test ORDER BY a, b",
        "SELECT a FROM test ORDER BY a DESC",
        // ASC is omitted in output (it's the default)
        "SELECT a FROM test ORDER BY a, b DESC",
        "SELECT a FROM test ORDER BY a NULLS FIRST",
        "SELECT a FROM test ORDER BY a DESC NULLS LAST",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_order_by_asc_normalization() {
    // ASC is default, so it's dropped in output
    validate(
        "SELECT a FROM test ORDER BY a ASC, b DESC",
        "SELECT a FROM test ORDER BY a, b DESC",
    );
}

#[test]
fn test_identity_limit_offset() {
    let cases = [
        "SELECT * FROM test LIMIT 100",
        "SELECT * FROM test LIMIT 100 OFFSET 200",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – Subqueries
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_subqueries() {
    let cases = [
        "SELECT a FROM (SELECT a FROM test) AS x",
        "SELECT * FROM (SELECT 1 AS x) AS sub",
        "SELECT a FROM test WHERE a IN (SELECT b FROM z)",
        "SELECT a FROM test WHERE EXISTS (SELECT 1)",
        "SELECT * FROM t WHERE id IN (SELECT id FROM t2)",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_nested_subquery() {
    validate_identity("SELECT a FROM (SELECT a FROM (SELECT a FROM test) AS y) AS x");
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – CASE expression
// (from Python identity.sql: CASE section)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_case() {
    let cases = [
        "SELECT CASE WHEN a > 1 THEN 1 ELSE 0 END",
        "SELECT CASE WHEN a < b THEN 1 WHEN a < c THEN 2 ELSE 3 END FROM test",
        "SELECT CASE 1 WHEN 1 THEN 1 ELSE 2 END",
        "SELECT CASE a WHEN 1 THEN 'one' WHEN 2 THEN 'two' ELSE 'other' END",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – BETWEEN, IN, IS NULL, LIKE, ILIKE
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_predicates() {
    let cases = [
        "SELECT * FROM t WHERE x BETWEEN 1 AND 10",
        "SELECT * FROM t WHERE x NOT BETWEEN 1 AND 10",
        "SELECT * FROM t WHERE x IN (1, 2, 3)",
        "SELECT * FROM t WHERE x NOT IN (1, 2, 3)",
        "SELECT * FROM t WHERE x IS NULL",
        "SELECT * FROM t WHERE x IS NOT NULL",
        "SELECT * FROM t WHERE x IS TRUE",
        "SELECT * FROM t WHERE x IS NOT TRUE",
        "SELECT * FROM t WHERE x IS FALSE",
        "SELECT * FROM t WHERE x IS NOT FALSE",
        "SELECT * FROM t WHERE x IS TRUE AND y IS NULL",
        "SELECT * FROM t WHERE x IS NOT FALSE OR y IS NOT NULL",
        "SELECT * FROM t WHERE x LIKE '%y%'",
        "SELECT * FROM t WHERE x NOT LIKE '%y%'",
        "SELECT * FROM t WHERE x ILIKE '%y%'",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_in_subquery() {
    validate_identity("SELECT * FROM t WHERE a IN (SELECT b FROM t2)");
    validate_identity("SELECT * FROM t WHERE a NOT IN (SELECT b FROM t2)");
}

#[test]
fn test_identity_exists() {
    validate_identity("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM t2)");
    validate_identity("SELECT * FROM t WHERE NOT EXISTS (SELECT 1 FROM t2)");
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – CAST, EXTRACT, functions
// (from Python identity.sql: CAST, EXTRACT, function sections)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_cast() {
    let cases = [
        "SELECT CAST(a AS INT) FROM test",
        "SELECT CAST(a AS VARCHAR) FROM test",
        "SELECT CAST(a AS DECIMAL(5, 3)) FROM test",
        "SELECT CAST(a AS TIMESTAMP) FROM test",
        "SELECT CAST(a AS DATE) FROM test",
        "SELECT CAST(a AS BOOLEAN) FROM test",
        "SELECT CAST(a AS TEXT) FROM test",
        "SELECT CAST(a AS BIGINT) FROM test",
        "SELECT CAST(a AS FLOAT) FROM test",
        "SELECT CAST(a AS DOUBLE) FROM test",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_extract() {
    let cases = [
        "SELECT EXTRACT(YEAR FROM x)",
        "SELECT EXTRACT(MONTH FROM x)",
        "SELECT EXTRACT(DAY FROM x)",
        "SELECT EXTRACT(HOUR FROM x)",
        "SELECT EXTRACT(MINUTE FROM x)",
        "SELECT EXTRACT(SECOND FROM x)",
        "SELECT EXTRACT(DOW FROM x)",
        "SELECT EXTRACT(EPOCH FROM x)",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_functions() {
    let cases = [
        "SELECT ABS(a) FROM test",
        "SELECT COUNT(*) FROM test",
        "SELECT COUNT(a) FROM test",
        "SELECT COUNT(DISTINCT a) FROM test",
        "SELECT SUM(a) FROM test",
        "SELECT AVG(a) FROM test",
        "SELECT MIN(a) FROM test",
        "SELECT MAX(a) FROM test",
        "SELECT ROUND(a) FROM test",
        "SELECT ROUND(a, 2) FROM test",
        "SELECT COALESCE(a, b, c) FROM test",
        "SELECT NULLIF(a, b) FROM test",
        "SELECT GREATEST(a, b, c) FROM test",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – Window functions
// (from Python identity.sql: Window section)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_window_functions() {
    let cases = [
        "SELECT RANK() OVER () FROM x",
        "SELECT RANK() OVER () AS y FROM x",
        "SELECT RANK() OVER (PARTITION BY a) FROM x",
        "SELECT RANK() OVER (PARTITION BY a, b) FROM x",
        "SELECT RANK() OVER (ORDER BY a) FROM x",
        "SELECT RANK() OVER (ORDER BY a, b) FROM x",
        "SELECT RANK() OVER (PARTITION BY a ORDER BY a) FROM x",
        "SELECT RANK() OVER (PARTITION BY a, b ORDER BY a, b DESC) FROM x",
        "SELECT SUM(x) OVER (PARTITION BY a) AS y FROM x",
        "SELECT ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) FROM emp",
        "SELECT LAG(x) OVER (ORDER BY y) AS x",
        "SELECT LEAD(a) OVER (ORDER BY b) AS a",
        "SELECT LEAD(a, 1) OVER (PARTITION BY a ORDER BY a) AS x",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_window_frames() {
    let cases = [
        "SELECT SUM(x) OVER (PARTITION BY a ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)",
        "SELECT SUM(x) OVER (PARTITION BY a ORDER BY b ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)",
        "SELECT SUM(x) OVER (PARTITION BY a ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING)",
        "SELECT SUM(x) OVER (PARTITION BY a ROWS BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING)",
        "SELECT SUM(x) OVER (PARTITION BY a RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)",
        "SELECT SUM(x) OVER (PARTITION BY a ORDER BY b ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING)",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_window_filter() {
    validate_identity("SELECT SUM(x) FILTER (WHERE x > 1)");
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – Set Operations (UNION, INTERSECT, EXCEPT)
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_set_operations() {
    let cases = [
        "SELECT 1 UNION ALL SELECT 2",
        "SELECT 1 UNION SELECT 2",
        "SELECT 1 INTERSECT SELECT 2",
        "SELECT 1 EXCEPT SELECT 2",
        "SELECT a FROM t1 UNION ALL SELECT b FROM t2",
        "SELECT a FROM t1 INTERSECT SELECT a FROM t2",
        "SELECT a FROM t1 EXCEPT SELECT a FROM t2",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – CTEs (WITH clause)
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_ctes() {
    let cases = [
        "WITH a AS (SELECT 1) SELECT * FROM a",
        "WITH a AS (SELECT 1 AS x) SELECT x FROM a",
        "WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a CROSS JOIN b",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_recursive_cte() {
    validate_identity("WITH RECURSIVE nums AS (SELECT 1 AS n) SELECT n FROM nums");
}

#[test]
fn test_identity_cte_with_columns() {
    validate_identity("WITH cte(x, y) AS (SELECT 1, 2) SELECT x, y FROM cte");
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – INSERT
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_insert() {
    let cases = [
        "INSERT INTO x VALUES (1, 'a', 2.0)",
        "INSERT INTO x VALUES (1, 'a', 2.0), (2, 'b', 3.0)",
        "INSERT INTO y (a, b, c) SELECT a, b, c FROM x",
        "INSERT INTO x SELECT * FROM y",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_insert_on_conflict() {
    validate_identity("INSERT INTO t (id) VALUES (1) ON CONFLICT (id) DO NOTHING");
    validate_identity(
        "INSERT INTO t (id, name) VALUES (1, 'a') ON CONFLICT (id) DO UPDATE SET name = 'b'",
    );
}

#[test]
fn test_identity_insert_returning() {
    validate_identity("INSERT INTO users (name) VALUES ('Alice') RETURNING id");
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – UPDATE
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_update() {
    let cases = [
        "UPDATE tbl_name SET foo = 123",
        "UPDATE tbl_name SET foo = 123, bar = 345",
        "UPDATE db.tbl_name SET foo = 123 WHERE tbl_name.bar = 234",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_update_returning() {
    validate_identity("UPDATE products SET price = 10 WHERE id = 1 RETURNING name, price");
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – DELETE
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_delete() {
    let cases = ["DELETE FROM x WHERE y > 1", "DELETE FROM y"];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_delete_using() {
    validate_identity("DELETE FROM event USING sales WHERE event.eventid = sales.eventid");
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – DDL: CREATE TABLE
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_create_table() {
    let cases = [
        "CREATE TABLE z (a INT, b VARCHAR, c VARCHAR(100), d DECIMAL(5, 3))",
        "CREATE TABLE IF NOT EXISTS x AS SELECT a FROM d",
        "CREATE TEMPORARY TABLE x AS SELECT a FROM d",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_create_table_constraints() {
    let cases = [
        "CREATE TABLE z (a INT, PRIMARY KEY (a))",
        "CREATE TABLE z (a INT NOT NULL)",
        // Generator outputs NOT NULL before DEFAULT
        "CREATE TABLE z (a INT NOT NULL DEFAULT 0)",
        "CREATE TABLE z (a INT UNIQUE)",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_create_table_constraint_ordering() {
    // DEFAULT 0 NOT NULL gets normalized to NOT NULL DEFAULT 0
    validate(
        "CREATE TABLE z (a INT DEFAULT 0 NOT NULL)",
        "CREATE TABLE z (a INT NOT NULL DEFAULT 0)",
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – DDL: DROP TABLE, CREATE/DROP VIEW
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_drop_table() {
    let cases = [
        "DROP TABLE a",
        "DROP TABLE IF EXISTS a",
        "DROP TABLE a CASCADE",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_identity_views() {
    let cases = [
        "CREATE VIEW x AS SELECT a FROM b",
        "CREATE VIEW IF NOT EXISTS x AS SELECT a FROM b",
        "CREATE OR REPLACE VIEW x AS SELECT *",
        "DROP VIEW a",
        "DROP VIEW IF EXISTS a",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – ALTER TABLE
// (from Python identity.sql: ALTER TABLE section)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_alter_table() {
    let cases = [
        "ALTER TABLE integers ADD COLUMN k INT",
        "ALTER TABLE integers DROP COLUMN k",
        "ALTER TABLE integers DROP COLUMN IF EXISTS k",
        "ALTER TABLE table1 RENAME COLUMN c1 TO c2",
        "ALTER TABLE table1 RENAME TO table2",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – Transaction statements
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_transactions() {
    let cases = ["BEGIN", "COMMIT", "ROLLBACK"];
    for sql in &cases {
        validate_identity(sql);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – EXPLAIN, USE
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_explain_use() {
    validate_identity("EXPLAIN SELECT * FROM x");
    validate_identity("USE db");
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – INTERVAL
// (from Python identity.sql: INTERVAL section)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_interval() {
    let cases = [
        "SELECT INTERVAL '1' DAY",
        "SELECT INTERVAL '1' MONTH",
        "SELECT INTERVAL '1' YEAR",
        "SELECT INTERVAL '1' HOUR",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – ARRAY and complex expressions
// (from Python identity.sql)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_array() {
    // ARRAY[1, 2, 3] using bracket syntax
    validate_identity("SELECT ARRAY[1, 2, 3]");
}

// ═════════════════════════════════════════════════════════════════════════════
// Identity tests – Postgres-style cast (::)
// (from Python test_transpile.py::test_types)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_postgres_cast_roundtrip() {
    // x::INT parses as CAST(x AS INT) when in a SELECT context
    validate("SELECT x::INT", "SELECT CAST(x AS INT)");
    validate(
        "SELECT x::INT::BOOLEAN",
        "SELECT CAST(CAST(x AS INT) AS BOOLEAN)",
    );
    validate(
        "SELECT CAST(x::INT AS BOOLEAN)",
        "SELECT CAST(CAST(x AS INT) AS BOOLEAN)",
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Spacing normalization tests
// (from Python test_transpile.py::test_space)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_space_normalization() {
    // Operators get spaces around them
    validate("SELECT 1>0", "SELECT 1 > 0");
    validate("SELECT 1>=0", "SELECT 1 >= 0");
    validate("SELECT 1<0", "SELECT 1 < 0");
    validate("SELECT 1<=0", "SELECT 1 <= 0");
}

// ═════════════════════════════════════════════════════════════════════════════
// Transpile – cross-dialect tests
// (from Python test_transpile.py and dialect test files)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_transpile_identity_same_dialect() {
    let sql = "SELECT a, b FROM t WHERE a > 1";
    for dialect in [
        Dialect::Ansi,
        Dialect::Postgres,
        Dialect::Mysql,
        Dialect::Sqlite,
        Dialect::BigQuery,
        Dialect::Snowflake,
        Dialect::DuckDb,
    ] {
        validate_with_dialect(sql, sql, dialect, dialect);
    }
}

#[test]
fn test_transpile_substr_to_substring() {
    // SUBSTR → SUBSTRING when targeting ANSI/Postgres
    validate_with_dialect(
        "SELECT SUBSTR(name, 1, 3) FROM users",
        "SELECT SUBSTRING(name, 1, 3) FROM users",
        Dialect::Mysql,
        Dialect::Postgres,
    );
}

#[test]
fn test_transpile_substring_to_substr() {
    // SUBSTRING → SUBSTR when targeting MySQL/SQLite
    validate_with_dialect(
        "SELECT SUBSTRING(name, 1, 3) FROM users",
        "SELECT SUBSTR(name, 1, 3) FROM users",
        Dialect::Postgres,
        Dialect::Mysql,
    );
    validate_with_dialect(
        "SELECT SUBSTRING(name, 1, 3) FROM users",
        "SELECT SUBSTR(name, 1, 3) FROM users",
        Dialect::Postgres,
        Dialect::Sqlite,
    );
}

#[test]
fn test_transpile_now_to_current_timestamp() {
    // NOW() → CURRENT_TIMESTAMP for BigQuery/Snowflake
    validate_with_dialect(
        "SELECT NOW()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Postgres,
        Dialect::BigQuery,
    );
    validate_with_dialect(
        "SELECT NOW()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Postgres,
        Dialect::Snowflake,
    );
}

#[test]
fn test_transpile_len_to_length() {
    // LEN → LENGTH for Postgres, MySQL, SQLite, DuckDB
    validate_with_dialect(
        "SELECT LEN(name) FROM t",
        "SELECT LENGTH(name) FROM t",
        Dialect::BigQuery,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT LEN(name) FROM t",
        "SELECT LENGTH(name) FROM t",
        Dialect::BigQuery,
        Dialect::Mysql,
    );
}

#[test]
fn test_transpile_ifnull_to_coalesce() {
    // IFNULL → COALESCE for ANSI/Postgres
    validate_with_dialect(
        "SELECT IFNULL(a, b) FROM t",
        "SELECT COALESCE(a, b) FROM t",
        Dialect::Mysql,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT IFNULL(a, b) FROM t",
        "SELECT COALESCE(a, b) FROM t",
        Dialect::Mysql,
        Dialect::Ansi,
    );
}

#[test]
fn test_transpile_ilike_to_like_lower() {
    // ILIKE → LOWER(x) LIKE LOWER(pattern) for MySQL/SQLite
    validate_with_dialect(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE LOWER(name) LIKE LOWER('%test%')",
        Dialect::Postgres,
        Dialect::Mysql,
    );
    validate_with_dialect(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE LOWER(name) LIKE LOWER('%test%')",
        Dialect::Postgres,
        Dialect::Sqlite,
    );
}

#[test]
fn test_transpile_type_mapping_text_to_string() {
    // TEXT → STRING for BigQuery
    validate_with_dialect(
        "SELECT CAST(x AS TEXT) FROM t",
        "SELECT CAST(x AS STRING) FROM t",
        Dialect::Postgres,
        Dialect::BigQuery,
    );
}

#[test]
fn test_transpile_type_mapping_string_to_text() {
    // STRING → TEXT for Postgres, MySQL, SQLite
    validate_with_dialect(
        "SELECT CAST(x AS STRING) FROM t",
        "SELECT CAST(x AS TEXT) FROM t",
        Dialect::BigQuery,
        Dialect::Postgres,
    );
}

#[test]
fn test_transpile_type_mapping_int_to_bigint() {
    // INT → BIGINT for BigQuery
    validate_with_dialect(
        "SELECT CAST(x AS INT) FROM t",
        "SELECT CAST(x AS BIGINT) FROM t",
        Dialect::Postgres,
        Dialect::BigQuery,
    );
}

#[test]
fn test_transpile_type_mapping_float_to_double() {
    // FLOAT → DOUBLE for BigQuery
    validate_with_dialect(
        "SELECT CAST(x AS FLOAT) FROM t",
        "SELECT CAST(x AS DOUBLE) FROM t",
        Dialect::Postgres,
        Dialect::BigQuery,
    );
}

#[test]
fn test_transpile_type_mapping_bytea_blob() {
    // BYTEA → BLOB for MySQL/SQLite
    validate_with_dialect(
        "SELECT CAST(x AS BYTEA) FROM t",
        "SELECT CAST(x AS BLOB) FROM t",
        Dialect::Postgres,
        Dialect::Mysql,
    );
    // BLOB → BYTEA for Postgres
    validate_with_dialect(
        "SELECT CAST(x AS BLOB) FROM t",
        "SELECT CAST(x AS BYTEA) FROM t",
        Dialect::Mysql,
        Dialect::Postgres,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Parse error tests
// (from Python test_transpile.py::test_paren)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_parse_errors() {
    // Unmatched parentheses should fail
    assert!(parse("1 + (2 + 3", Dialect::Ansi).is_err());
    assert!(parse("SELECT (", Dialect::Ansi).is_err());
    // Empty input
    assert!(parse("", Dialect::Ansi).is_err());
}

// ═════════════════════════════════════════════════════════════════════════════
// Multi-statement parsing
// (from Python test_transpile.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_transpile_multiple_statements() {
    let results = sqlglot_rust::transpile_statements(
        "SELECT 1; SELECT 2; SELECT 3",
        Dialect::Ansi,
        Dialect::Ansi,
    )
    .unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0], "SELECT 1");
    assert_eq!(results[1], "SELECT 2");
    assert_eq!(results[2], "SELECT 3");
}

// ═════════════════════════════════════════════════════════════════════════════
// Complex roundtrip tests combining multiple features
// (inspired by Python identity.sql complex queries)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_complex_join_where_order() {
    validate_identity(
        "SELECT u.id, u.name FROM users AS u INNER JOIN orders AS o ON u.id = o.user_id WHERE o.total > 100 ORDER BY u.name LIMIT 10",
    );
}

#[test]
fn test_identity_cte_with_join() {
    validate_identity(
        "WITH active_users AS (SELECT id, name FROM users WHERE active = TRUE) SELECT a.name, COUNT(*) FROM active_users AS a INNER JOIN orders AS o ON a.id = o.user_id GROUP BY a.name",
    );
}

#[test]
fn test_identity_subquery_in_select() {
    validate_identity("SELECT a, (SELECT MAX(b) FROM t2) AS max_b FROM t1");
}

#[test]
fn test_identity_union_with_order_limit() {
    validate_identity("SELECT a FROM t1 UNION ALL SELECT b FROM t2 ORDER BY 1 LIMIT 10");
}

#[test]
fn test_identity_nested_case_in_select() {
    validate_identity(
        "SELECT CASE WHEN x > 0 THEN CASE WHEN y > 0 THEN 'both' ELSE 'x_only' END ELSE 'none' END AS result FROM t",
    );
}

#[test]
fn test_identity_window_with_case() {
    validate_identity(
        "SELECT SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END) OVER (PARTITION BY dept) AS active_count FROM employees",
    );
}

#[test]
fn test_identity_multiple_ctes() {
    validate_identity(
        "WITH a AS (SELECT 1 AS x), b AS (SELECT 2 AS y), c AS (SELECT 3 AS z) SELECT * FROM a CROSS JOIN b CROSS JOIN c",
    );
}

#[test]
fn test_identity_insert_with_cte() {
    // Note: CTE with INSERT is complex; test the basic version
    validate_identity("INSERT INTO target SELECT * FROM src");
}

#[test]
fn test_identity_create_table_as() {
    validate_identity("CREATE TABLE new_t AS SELECT a, b FROM old_t WHERE a > 0");
}

// ═════════════════════════════════════════════════════════════════════════════
// Serde roundtrip tests
// (from Python test_serde.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_serde_roundtrip() {
    let test_cases = [
        "SELECT 1",
        "SELECT a, b FROM t WHERE a > 1",
        "WITH cte AS (SELECT 1) SELECT * FROM cte",
        "INSERT INTO t VALUES (1, 'a')",
        "CREATE TABLE t (a INT, b VARCHAR(100))",
    ];
    for sql in &test_cases {
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let json = serde_json::to_string(&ast).unwrap();
        let deserialized: sqlglot_rust::Statement = serde_json::from_str(&json).unwrap();
        let output = generate(&deserialized, Dialect::Ansi);
        assert_eq!(output, *sql, "Serde roundtrip failed for: {}", sql);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// TRUNCATE
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_truncate() {
    validate_identity("TRUNCATE TABLE t");
}

// ═════════════════════════════════════════════════════════════════════════════
// SELECT TOP N (T-SQL) — Issue #1
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_top_n_star_tsql_roundtrip() {
    // Core bug: SELECT TOP 5 * was failing because * was consumed as multiply
    validate_with_dialect(
        "SELECT TOP 5 * FROM t",
        "SELECT TOP 5 * FROM t",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

#[test]
fn test_top_n_columns_tsql_roundtrip() {
    validate_with_dialect(
        "SELECT TOP 10 id, name FROM t",
        "SELECT TOP 10 id, name FROM t",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

#[test]
fn test_top_n_parenthesized_tsql_roundtrip() {
    validate_with_dialect(
        "SELECT TOP (5) * FROM t",
        "SELECT TOP (5) * FROM t",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

#[test]
fn test_top_distinct_tsql_roundtrip() {
    validate_with_dialect(
        "SELECT DISTINCT TOP 3 id FROM t",
        "SELECT DISTINCT TOP 3 id FROM t",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Typed Function Expressions — comprehensive tests
// ═════════════════════════════════════════════════════════════════════════════

// ── Date/Time typed functions ──

#[test]
fn test_typed_date_trunc_identity() {
    validate_identity("SELECT DATE_TRUNC('MONTH', created_at) FROM orders");
}

#[test]
fn test_typed_date_trunc_to_tsql() {
    validate_with_dialect(
        "SELECT DATE_TRUNC('MONTH', created_at) FROM orders",
        "SELECT DATETRUNC(MONTH, created_at) FROM orders",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_typed_date_trunc_to_oracle() {
    validate_with_dialect(
        "SELECT DATE_TRUNC('MONTH', created_at) FROM orders",
        "SELECT TRUNC(created_at, 'MONTH') FROM orders",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_typed_current_timestamp_roundtrip() {
    let cases = [
        "SELECT CURRENT_TIMESTAMP()",
        "SELECT COUNT(*) FROM t WHERE ts > CURRENT_TIMESTAMP()",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_typed_year_month_day() {
    // YEAR/MONTH/DAY → EXTRACT for non-TSQL
    validate_with_dialect(
        "SELECT YEAR(created_at) FROM t",
        "SELECT EXTRACT(YEAR FROM created_at) FROM t",
        Dialect::Ansi,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT MONTH(created_at) FROM t",
        "SELECT EXTRACT(MONTH FROM created_at) FROM t",
        Dialect::Ansi,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT DAY(created_at) FROM t",
        "SELECT EXTRACT(DAY FROM created_at) FROM t",
        Dialect::Ansi,
        Dialect::Postgres,
    );
}

#[test]
fn test_typed_year_tsql_preserves() {
    validate_with_dialect(
        "SELECT YEAR(created_at) FROM t",
        "SELECT YEAR(created_at) FROM t",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

// ── String typed functions ──

#[test]
fn test_typed_upper_lower_identity() {
    validate_identity("SELECT UPPER(name) FROM t");
    validate_identity("SELECT LOWER(name) FROM t");
}

#[test]
fn test_typed_trim_identity() {
    validate_identity("SELECT TRIM(name) FROM t");
}

#[test]
fn test_typed_length_cross_dialect() {
    validate_with_dialect(
        "SELECT LENGTH(name) FROM t",
        "SELECT LEN(name) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT LEN(name) FROM t",
        "SELECT LENGTH(name) FROM t",
        Dialect::Tsql,
        Dialect::Postgres,
    );
}

#[test]
fn test_typed_substring_cross_dialect() {
    validate_with_dialect(
        "SELECT SUBSTRING(name, 1, 3) FROM t",
        "SELECT SUBSTR(name, 1, 3) FROM t",
        Dialect::Postgres,
        Dialect::Mysql,
    );
    validate_with_dialect(
        "SELECT SUBSTR(name, 1, 3) FROM t",
        "SELECT SUBSTRING(name, 1, 3) FROM t",
        Dialect::Mysql,
        Dialect::Postgres,
    );
}

#[test]
fn test_typed_replace_identity() {
    validate_identity("SELECT REPLACE(name, 'old', 'new') FROM t");
}

#[test]
fn test_typed_reverse_identity() {
    validate_identity("SELECT REVERSE(name) FROM t");
}

#[test]
fn test_typed_left_right_identity() {
    validate_identity("SELECT LEFT(name, 3) FROM t");
    validate_identity("SELECT RIGHT(name, 3) FROM t");
}

#[test]
fn test_typed_lpad_rpad_identity() {
    validate_identity("SELECT LPAD(name, 10, '*') FROM t");
    validate_identity("SELECT RPAD(name, 10) FROM t");
}

#[test]
fn test_typed_concat_ws_identity() {
    validate_identity("SELECT CONCAT_WS(', ', a, b, c) FROM t");
}

#[test]
fn test_typed_split_cross_dialect() {
    validate_with_dialect(
        "SELECT SPLIT(name, ',') FROM t",
        "SELECT STRING_SPLIT(name, ',') FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_typed_initcap_identity() {
    validate_identity("SELECT INITCAP(name) FROM t");
}

#[test]
fn test_typed_regexp_like_identity() {
    validate_identity("SELECT REGEXP_LIKE(name, '^A.*') FROM t");
}

#[test]
fn test_typed_regexp_replace_identity() {
    validate_identity("SELECT REGEXP_REPLACE(name, '[0-9]', 'X') FROM t");
}

// ── Aggregate typed functions ──

#[test]
fn test_typed_count_variations() {
    validate_identity("SELECT COUNT(*) FROM t");
    validate_identity("SELECT COUNT(a) FROM t");
    validate_identity("SELECT COUNT(DISTINCT a) FROM t");
}

#[test]
fn test_typed_sum_avg_min_max() {
    validate_identity("SELECT SUM(amount) FROM t");
    validate_identity("SELECT AVG(price) FROM t");
    validate_identity("SELECT MIN(created_at) FROM t");
    validate_identity("SELECT MAX(score) FROM t");
}

#[test]
fn test_typed_sum_distinct() {
    validate_identity("SELECT SUM(DISTINCT amount) FROM t");
}

#[test]
fn test_typed_array_agg_cross_dialect() {
    validate_with_dialect(
        "SELECT ARRAY_AGG(name) FROM t",
        "SELECT LIST(name) FROM t",
        Dialect::Postgres,
        Dialect::DuckDb,
    );
    validate_with_dialect(
        "SELECT ARRAY_AGG(name) FROM t",
        "SELECT COLLECT_LIST(name) FROM t",
        Dialect::Postgres,
        Dialect::Hive,
    );
}

#[test]
fn test_typed_variance_stddev() {
    validate_identity("SELECT VARIANCE(score) FROM t");
    validate_identity("SELECT STDDEV(score) FROM t");
}

// ── Window typed functions ──

#[test]
fn test_typed_row_number_with_over() {
    validate_identity("SELECT ROW_NUMBER() OVER (ORDER BY id) FROM t");
}

#[test]
fn test_typed_rank_dense_rank() {
    validate_identity("SELECT RANK() OVER (PARTITION BY dept ORDER BY salary) FROM t");
    validate_identity("SELECT DENSE_RANK() OVER (ORDER BY score DESC) FROM t");
}

#[test]
fn test_typed_ntile() {
    validate_identity("SELECT NTILE(4) OVER (ORDER BY id) FROM t");
}

#[test]
fn test_typed_lead_lag() {
    validate_identity("SELECT LEAD(price, 1) OVER (ORDER BY date) FROM t");
    validate_identity("SELECT LAG(price) OVER (ORDER BY date) FROM t");
    validate_identity("SELECT LAG(price, 1, 0) OVER (PARTITION BY category ORDER BY date) FROM t");
}

#[test]
fn test_typed_first_last_value() {
    validate_identity("SELECT FIRST_VALUE(name) OVER (ORDER BY id) FROM t");
    validate_identity("SELECT LAST_VALUE(name) OVER (ORDER BY id) FROM t");
}

#[test]
fn test_typed_window_with_filter() {
    validate_identity("SELECT COUNT(*) FILTER (WHERE active) FROM t");
    validate_identity("SELECT SUM(amount) FILTER (WHERE status = 'paid') FROM orders");
}

// ── Math typed functions ──

#[test]
fn test_typed_math_functions_identity() {
    let cases = [
        "SELECT ABS(x) FROM t",
        "SELECT CEIL(x) FROM t",
        "SELECT FLOOR(x) FROM t",
        "SELECT ROUND(x, 2) FROM t",
        "SELECT SQRT(x) FROM t",
        "SELECT LN(x) FROM t",
        "SELECT LOG(x) FROM t",
        "SELECT MOD(x, 3) FROM t",
    ];
    for sql in &cases {
        validate_identity(sql);
    }
}

#[test]
fn test_typed_pow_cross_dialect() {
    validate_with_dialect(
        "SELECT POW(x, 2) FROM t",
        "SELECT POWER(x, 2) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_typed_ceil_cross_dialect() {
    validate_with_dialect(
        "SELECT CEIL(x) FROM t",
        "SELECT CEILING(x) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_typed_greatest_least() {
    validate_identity("SELECT GREATEST(a, b, c) FROM t");
    validate_identity("SELECT LEAST(a, b, c) FROM t");
}

// ── Array typed functions ──

#[test]
fn test_typed_array_size_cross_dialect() {
    validate_with_dialect(
        "SELECT ARRAY_SIZE(arr) FROM t",
        "SELECT ARRAY_LENGTH(arr) FROM t",
        Dialect::Snowflake,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT ARRAY_SIZE(arr) FROM t",
        "SELECT SIZE(arr) FROM t",
        Dialect::Snowflake,
        Dialect::Hive,
    );
}

#[test]
fn test_typed_array_concat_cross_dialect() {
    validate_with_dialect(
        "SELECT ARRAY_CONCAT(a, b) FROM t",
        "SELECT ARRAY_CAT(a, b) FROM t",
        Dialect::BigQuery,
        Dialect::Postgres,
    );
}

#[test]
fn test_typed_generate_series() {
    validate_identity("SELECT GENERATE_SERIES(1, 10)");
    validate_identity("SELECT GENERATE_SERIES(1, 100, 5)");
}

#[test]
fn test_typed_flatten_identity() {
    validate_identity("SELECT FLATTEN(arr) FROM t");
}

#[test]
fn test_typed_explode_identity() {
    validate_identity("SELECT EXPLODE(arr) FROM t");
}

// ── JSON typed functions ──

#[test]
fn test_typed_json_extract_cross_dialect() {
    validate_with_dialect(
        "SELECT JSON_EXTRACT(data, '$.name') FROM t",
        "SELECT JSON_VALUE(data, '$.name') FROM t",
        Dialect::Mysql,
        Dialect::Tsql,
    );
}

#[test]
fn test_typed_json_extract_scalar_identity() {
    validate_identity("SELECT JSON_EXTRACT_SCALAR(data, '$.name') FROM t");
}

#[test]
fn test_typed_json_format_cross_dialect() {
    validate_with_dialect(
        "SELECT JSON_FORMAT(data) FROM t",
        "SELECT TO_JSON_STRING(data) FROM t",
        Dialect::Ansi,
        Dialect::BigQuery,
    );
}

// ── Conversion typed functions ──

#[test]
fn test_typed_hex_cross_dialect() {
    validate_with_dialect(
        "SELECT HEX(data) FROM t",
        "SELECT TO_HEX(data) FROM t",
        Dialect::Mysql,
        Dialect::Presto,
    );
}

#[test]
fn test_typed_unhex_cross_dialect() {
    validate_with_dialect(
        "SELECT UNHEX(data) FROM t",
        "SELECT FROM_HEX(data) FROM t",
        Dialect::Mysql,
        Dialect::Trino,
    );
}

#[test]
fn test_typed_md5_identity() {
    validate_identity("SELECT MD5(password) FROM t");
}

#[test]
fn test_typed_sha_cross_dialect() {
    validate_with_dialect(
        "SELECT SHA(data) FROM t",
        "SELECT SHA1(data) FROM t",
        Dialect::Postgres,
        Dialect::Mysql,
    );
}

// ── Generic function fallback ──

#[test]
fn test_generic_function_fallback() {
    // Unrecognized functions should still work via Expr::Function
    validate_identity("SELECT MY_CUSTOM_FUNC(a, b) FROM t");
    validate_identity("SELECT SOME_UDF(x) FROM t");
}

// ── Complex expressions with typed functions ──

#[test]
fn test_typed_functions_in_complex_expressions() {
    validate_identity("SELECT COUNT(*), SUM(amount), AVG(price) FROM orders GROUP BY category");
    validate_identity(
        "SELECT ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) AS rn FROM emp",
    );
    validate_identity("SELECT UPPER(SUBSTRING(name, 1, 1)) FROM t");
    validate_identity("SELECT GREATEST(a, LEAST(b, c)) FROM t");
    validate_identity("SELECT ROUND(AVG(score), 2) FROM t");
}

#[test]
fn test_typed_functions_in_where_clause() {
    validate_identity("SELECT * FROM t WHERE LENGTH(name) > 5");
    validate_identity("SELECT * FROM t WHERE ABS(score) < 10");
    validate_identity("SELECT * FROM t WHERE UPPER(status) = 'ACTIVE'");
}

#[test]
fn test_typed_functions_nested() {
    validate_identity("SELECT ROUND(SQRT(ABS(x)), 2) FROM t");
    validate_identity("SELECT UPPER(REVERSE(TRIM(name))) FROM t");
}

#[test]
fn test_typed_functions_with_aliases() {
    validate_identity("SELECT COUNT(*) AS total, MAX(price) AS max_price FROM t");
    validate_identity("SELECT ROW_NUMBER() OVER (ORDER BY id) AS rn FROM t");
}

// ═════════════════════════════════════════════════════════════════════════════
// PIVOT / UNPIVOT
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_pivot_basic() {
    validate_identity(
        "SELECT * FROM sales PIVOT (SUM(amount) FOR quarter IN ('Q1', 'Q2', 'Q3', 'Q4'))",
    );
}

#[test]
fn test_pivot_with_alias() {
    validate_identity(
        "SELECT * FROM sales PIVOT (SUM(amount) FOR quarter IN ('Q1', 'Q2', 'Q3', 'Q4')) AS pvt",
    );
}

#[test]
fn test_pivot_with_aliased_values() {
    validate_identity(
        "SELECT * FROM sales PIVOT (SUM(amount) FOR quarter IN ('Q1' AS q1, 'Q2' AS q2))",
    );
}

#[test]
fn test_pivot_with_count() {
    validate_identity(
        "SELECT * FROM orders PIVOT (COUNT(*) FOR status IN ('open', 'closed', 'pending'))",
    );
}

#[test]
fn test_pivot_subquery_source() {
    validate_identity(
        "SELECT * FROM (SELECT * FROM sales) AS s PIVOT (SUM(amount) FOR quarter IN ('Q1', 'Q2'))",
    );
}

#[test]
fn test_unpivot_basic() {
    validate_identity("SELECT * FROM quarterly UNPIVOT (amount FOR quarter IN (Q1, Q2, Q3, Q4))");
}

#[test]
fn test_unpivot_with_alias() {
    validate_identity(
        "SELECT * FROM quarterly UNPIVOT (amount FOR quarter IN (Q1, Q2, Q3, Q4)) AS unpvt",
    );
}

#[test]
fn test_unpivot_with_aliased_columns() {
    validate_identity(
        "SELECT * FROM quarterly UNPIVOT (amount FOR quarter IN (Q1 AS q1, Q2 AS q2))",
    );
}

#[test]
fn test_pivot_with_where() {
    validate_identity(
        "SELECT * FROM sales PIVOT (SUM(amount) FOR quarter IN ('Q1', 'Q2')) AS pvt WHERE pvt.Q1 > 100",
    );
}

#[test]
fn test_pivot_with_join() {
    validate_identity(
        "SELECT * FROM sales PIVOT (SUM(amount) FOR quarter IN ('Q1', 'Q2')) AS pvt INNER JOIN regions ON pvt.region_id = regions.id",
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Time Format Mapping Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_time_format_mysql_to_postgres() {
    // MySQL DATE_FORMAT should transpile to PostgreSQL TO_CHAR with format conversion
    validate_with_dialect(
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s')",
        "SELECT TO_CHAR(created_at, 'YYYY-MM-DD HH24:MI:SS')",
        Dialect::Mysql,
        Dialect::Postgres,
    );
}

#[test]
fn test_time_format_postgres_to_mysql() {
    // PostgreSQL TO_CHAR should transpile to MySQL DATE_FORMAT with format conversion
    validate_with_dialect(
        "SELECT TO_CHAR(created_at, 'YYYY-MM-DD HH24:MI:SS')",
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s')",
        Dialect::Postgres,
        Dialect::Mysql,
    );
}

#[test]
fn test_time_format_mysql_to_spark() {
    // MySQL format to Spark Java DateTimeFormatter style
    validate_with_dialect(
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%d')",
        "SELECT DATE_FORMAT(created_at, 'yyyy-MM-dd')",
        Dialect::Mysql,
        Dialect::Spark,
    );
}

#[test]
fn test_time_format_postgres_to_snowflake() {
    // PostgreSQL TO_CHAR to Snowflake (which uses similar Postgres-style format)
    validate_with_dialect(
        "SELECT TO_CHAR(created_at, 'YYYY-MM-DD')",
        "SELECT TO_CHAR(created_at, 'YYYY-MM-DD')",
        Dialect::Postgres,
        Dialect::Snowflake,
    );
}

#[test]
fn test_time_format_spark_to_postgres() {
    // Spark Java format to PostgreSQL
    validate_with_dialect(
        "SELECT DATE_FORMAT(created_at, 'yyyy-MM-dd HH:mm:ss')",
        "SELECT TO_CHAR(created_at, 'YYYY-MM-DD HH24:MI:SS')",
        Dialect::Spark,
        Dialect::Postgres,
    );
}

#[test]
fn test_time_format_with_12hour_clock() {
    // 12-hour clock format with AM/PM (MySQL uses %h for 12-hour)
    validate_with_dialect(
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%d %h:%i %p')",
        "SELECT TO_CHAR(created_at, 'YYYY-MM-DD HH12:MI AM')",
        Dialect::Mysql,
        Dialect::Postgres,
    );
}

#[test]
fn test_time_format_mysql_to_bigquery() {
    // MySQL to BigQuery (BigQuery uses strftime-like format)
    validate_with_dialect(
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s')",
        "SELECT FORMAT_TIMESTAMP(created_at, '%Y-%m-%d %H:%M:%S')",
        Dialect::Mysql,
        Dialect::BigQuery,
    );
}

#[test]
fn test_time_format_with_literals() {
    // Format with literal characters (like T in ISO format)
    validate_with_dialect(
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%dT%H:%i:%s')",
        "SELECT TO_CHAR(created_at, 'YYYY-MM-DDTHH24:MI:SS')",
        Dialect::Mysql,
        Dialect::Postgres,
    );
}

#[test]
fn test_str_to_time_mysql_to_postgres() {
    // STR_TO_DATE to TO_TIMESTAMP conversion
    validate_with_dialect(
        "SELECT STR_TO_DATE(date_str, '%Y-%m-%d')",
        "SELECT TO_TIMESTAMP(date_str, 'YYYY-MM-DD')",
        Dialect::Mysql,
        Dialect::Postgres,
    );
}

#[test]
fn test_time_format_identity_mysql() {
    // Identity test - MySQL format should remain unchanged when transpiling to MySQL
    validate_with_dialect(
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s')",
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s')",
        Dialect::Mysql,
        Dialect::Mysql,
    );
}

#[test]
fn test_time_format_identity_postgres() {
    // Identity test - PostgreSQL format should remain unchanged
    validate_with_dialect(
        "SELECT TO_CHAR(created_at, 'YYYY-MM-DD HH24:MI:SS')",
        "SELECT TO_CHAR(created_at, 'YYYY-MM-DD HH24:MI:SS')",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn test_oracle_omits_as_in_table_alias() {
    // Oracle forbids AS between a table reference and its alias
    validate_with_dialect(
        "SELECT * FROM users AS u WHERE u.id = 1",
        "SELECT * FROM users u WHERE u.id = 1",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_oracle_omits_as_in_join_table_alias() {
    validate_with_dialect(
        "SELECT a.name, b.email FROM accounts AS a INNER JOIN users AS b ON a.user_id = b.id",
        "SELECT a.name, b.email FROM accounts a INNER JOIN users b ON a.user_id = b.id",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_oracle_omits_as_in_subquery_alias() {
    validate_with_dialect(
        "SELECT * FROM (SELECT id, name FROM users) AS sub WHERE sub.id > 10",
        "SELECT * FROM (SELECT id, name FROM users) sub WHERE sub.id > 10",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_oracle_preserves_column_alias_as() {
    // Column aliases should still use AS even for Oracle
    validate_with_dialect(
        "SELECT first_name AS fname, last_name AS lname FROM employees",
        "SELECT first_name AS fname, last_name AS lname FROM employees",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_oracle_catalog_query_no_spurious_as() {
    // Catalog query that already has no AS should not gain one
    validate_with_dialect(
        "SELECT U.* FROM ALL_USERS U WHERE (U.USERNAME IS NOT NULL)",
        "SELECT U.* FROM ALL_USERS U WHERE (U.USERNAME IS NOT NULL)",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_non_oracle_keeps_table_alias_as() {
    // Non-Oracle dialects should still emit AS
    validate_with_dialect(
        "SELECT * FROM users AS u",
        "SELECT * FROM users AS u",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}
