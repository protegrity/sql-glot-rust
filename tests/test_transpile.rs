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
fn test_national_string_literal_oracle_roundtrip() {
    validate_with_dialect(
        "SELECT N'Hello' FROM DUAL",
        "SELECT N'Hello' FROM DUAL",
        Dialect::Oracle,
        Dialect::Oracle,
    );
}

#[test]
fn test_national_string_literal_tsql_roundtrip() {
    validate_with_dialect(
        "SELECT N'Hello'",
        "SELECT N'Hello'",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

#[test]
fn test_national_string_literal_oracle_to_postgres() {
    validate_with_dialect(
        "SELECT N'Hello' FROM DUAL",
        "SELECT 'Hello' FROM DUAL",
        Dialect::Oracle,
        Dialect::Postgres,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-007: Auto-promote Unicode StringLiteral to N'...' for TSQL/Oracle
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_string_literal_ascii_tsql_no_prefix() {
    // ASCII-only strings should NOT get N prefix
    validate_with_dialect(
        "SELECT 'Hello'",
        "SELECT 'Hello'",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_string_literal_unicode_tsql_gets_n_prefix() {
    // Non-ASCII strings MUST get N prefix for TSQL
    validate_with_dialect(
        "SELECT '世界'",
        "SELECT N'世界'",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_string_literal_emoji_tsql_gets_n_prefix() {
    validate_with_dialect(
        "SELECT '🎉 party'",
        "SELECT N'🎉 party'",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_string_literal_accented_tsql_gets_n_prefix() {
    validate_with_dialect(
        "SELECT 'café'",
        "SELECT N'café'",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_string_literal_unicode_oracle_gets_n_prefix() {
    validate_with_dialect(
        "SELECT 'テスト' FROM DUAL",
        "SELECT N'テスト' FROM DUAL",
        Dialect::Oracle,
        Dialect::Oracle,
    );
}

#[test]
fn test_string_literal_unicode_postgres_no_prefix() {
    // PostgreSQL target should NOT add N prefix
    validate_with_dialect(
        "SELECT '世界'",
        "SELECT '世界'",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn test_string_literal_unicode_in_where_tsql() {
    validate_with_dialect(
        "SELECT * FROM t WHERE name = '日本'",
        "SELECT * FROM t WHERE name = N'日本'",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_string_literal_unicode_in_insert_tsql() {
    validate_with_dialect(
        "INSERT INTO t (col) VALUES ('données')",
        "INSERT INTO t (col) VALUES (N'données')",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_string_literal_escaped_quote_with_unicode_tsql() {
    validate_with_dialect(
        "SELECT '日本''s best'",
        "SELECT N'日本''s best'",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_national_string_literal_still_works_with_cr007() {
    // Existing NationalStringLiteral behavior unchanged
    validate_with_dialect(
        "SELECT N'Hello'",
        "SELECT N'Hello'",
        Dialect::Tsql,
        Dialect::Tsql,
    );
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
    assert_eq!(
        transpile("SELECT * FROM a, b", Dialect::Ansi, Dialect::Ansi).unwrap(),
        "SELECT * FROM a CROSS JOIN b"
    );
    assert_eq!(
        transpile("SELECT * FROM a, b, c", Dialect::Ansi, Dialect::Ansi).unwrap(),
        "SELECT * FROM a CROSS JOIN b CROSS JOIN c"
    );
    assert_eq!(
        transpile(
            "SELECT * FROM a, b WHERE a.x = b.y",
            Dialect::Ansi,
            Dialect::Ansi,
        )
        .unwrap(),
        "SELECT * FROM a CROSS JOIN b WHERE a.x = b.y"
    );
    validate_identity("SELECT * FROM a CROSS JOIN b");
}

#[test]
fn test_mysql_group_concat_to_sqlite() {
    assert_eq!(
        transpile(
            "SELECT GROUP_CONCAT(v SEPARATOR '|') FROM gc",
            Dialect::Mysql,
            Dialect::Sqlite,
        )
        .unwrap(),
        "SELECT GROUP_CONCAT(v, '|') FROM gc"
    );
    // SQLite has no ORDER BY support inside GROUP_CONCAT; the order is
    // intentionally dropped on output.
    assert_eq!(
        transpile(
            "SELECT GROUP_CONCAT(v ORDER BY v SEPARATOR '|') FROM gc",
            Dialect::Mysql,
            Dialect::Sqlite,
        )
        .unwrap(),
        "SELECT GROUP_CONCAT(v, '|') FROM gc"
    );
}

#[test]
fn test_mysql_group_concat_identity() {
    // Round-trip MySQL → MySQL must preserve DISTINCT, ORDER BY, and SEPARATOR.
    assert_eq!(
        transpile(
            "SELECT GROUP_CONCAT(v SEPARATOR '|') FROM gc",
            Dialect::Mysql,
            Dialect::Mysql,
        )
        .unwrap(),
        "SELECT GROUP_CONCAT(v SEPARATOR '|') FROM gc"
    );
    assert_eq!(
        transpile(
            "SELECT GROUP_CONCAT(DISTINCT v ORDER BY v DESC SEPARATOR ',') FROM gc",
            Dialect::Mysql,
            Dialect::Mysql,
        )
        .unwrap(),
        "SELECT GROUP_CONCAT(DISTINCT v ORDER BY v DESC SEPARATOR ',') FROM gc"
    );
}

#[test]
fn test_mysql_group_concat_to_postgres() {
    assert_eq!(
        transpile(
            "SELECT GROUP_CONCAT(v SEPARATOR '|') FROM gc",
            Dialect::Mysql,
            Dialect::Postgres,
        )
        .unwrap(),
        "SELECT STRING_AGG(v, '|') FROM gc"
    );
    assert_eq!(
        transpile(
            "SELECT GROUP_CONCAT(v ORDER BY v DESC SEPARATOR '|') FROM gc",
            Dialect::Mysql,
            Dialect::Postgres,
        )
        .unwrap(),
        "SELECT STRING_AGG(v, '|' ORDER BY v DESC) FROM gc"
    );
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
        "SELECT x::TEXT FROM t",
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
        "SELECT x::BYTEA FROM t",
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

// ── CR-024: extended TRIM([LEADING|TRAILING|BOTH] [chars] FROM expr) ──────────
// The ANSI/PostgreSQL extended TRIM grammar was transpiled to an invalid 2-arg
// comma call `TRIM(expr, chars)` with the direction silently dropped. It must
// route into a TypedFunction::Trim and emit valid, direction-preserving SQL.

#[test]
fn cr024_pg_to_tsql_trim_both() {
    // Reported case: BOTH + chars → ANSI `chars FROM expr` (SQL Server 2022).
    validate_with_dialect(
        "SELECT TRIM(BOTH ' ' FROM '  hi  ')",
        "SELECT TRIM(' ' FROM '  hi  ')",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT TRIM(BOTH 'x' FROM 'xxhixx')",
        "SELECT TRIM('x' FROM 'xxhixx')",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr024_pg_to_tsql_trim_direction_preserved() {
    validate_with_dialect(
        "SELECT TRIM(LEADING ' ' FROM '  hi  ')",
        "SELECT TRIM(LEADING ' ' FROM '  hi  ')",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT TRIM(TRAILING ' ' FROM '  hi  ')",
        "SELECT TRIM(TRAILING ' ' FROM '  hi  ')",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr025_pg_to_tsql_trim_direction_no_chars_native_ltrim() {
    // CR-025: T-SQL rejects a direction keyword with no characters
    // (`TRIM(LEADING FROM x)` -> Msg 156). Emit the native LTRIM instead.
    validate_with_dialect(
        "SELECT TRIM(LEADING FROM '  hi  ')",
        "SELECT LTRIM('  hi  ')",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr024_pg_to_tsql_trim_comma_form_preserves_chars() {
    // Comma form `TRIM(expr, chars)` must keep chars (previously dropped).
    validate_with_dialect(
        "SELECT TRIM(name, 'x') FROM t",
        "SELECT TRIM('x' FROM name) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr025_ltrim_rtrim_native_tsql() {
    // CR-025: PostgreSQL LTRIM/RTRIM map to native T-SQL LTRIM/RTRIM. The
    // earlier `TRIM(LEADING FROM x)` form is rejected by SQL Server (Msg 156).
    validate_with_dialect(
        "SELECT LTRIM(name) FROM t",
        "SELECT LTRIM(name) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT RTRIM(name) FROM t",
        "SELECT RTRIM(name) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    // Control: with a character set the ANSI FROM form is still used (valid on
    // SQL Server 2022), and BOTH with no chars is a bare TRIM.
    validate_with_dialect(
        "SELECT TRIM(LEADING ' ' FROM name) FROM t",
        "SELECT TRIM(LEADING ' ' FROM name) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT TRIM('  hi  ')",
        "SELECT TRIM('  hi  ')",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    // Other ANSI targets keep the FROM form (valid there).
    validate_with_dialect(
        "SELECT TRIM(LEADING FROM name) FROM t",
        "SELECT TRIM(LEADING FROM name) FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn cr024_pg_to_snowflake_trim_comma_form() {
    // Comma-form dialects must not regress to the unsupported FROM grammar;
    // direction is preserved via LTRIM/RTRIM with the chars as a 2nd argument.
    validate_with_dialect(
        "SELECT TRIM(BOTH ' ' FROM '  hi  ')",
        "SELECT TRIM('  hi  ', ' ')",
        Dialect::Postgres,
        Dialect::Snowflake,
    );
    validate_with_dialect(
        "SELECT TRIM(LEADING ' ' FROM '  hi  ')",
        "SELECT LTRIM('  hi  ', ' ')",
        Dialect::Postgres,
        Dialect::Snowflake,
    );
    validate_with_dialect(
        "SELECT TRIM(TRAILING ' ' FROM '  hi  ')",
        "SELECT RTRIM('  hi  ', ' ')",
        Dialect::Postgres,
        Dialect::Snowflake,
    );
}

#[test]
fn cr024_snowflake_trim_comma_form_roundtrip() {
    validate_with_dialect(
        "SELECT TRIM(name, 'x') FROM t",
        "SELECT TRIM(name, 'x') FROM t",
        Dialect::Snowflake,
        Dialect::Snowflake,
    );
    validate_with_dialect(
        "SELECT LTRIM(name) FROM t",
        "SELECT LTRIM(name) FROM t",
        Dialect::Snowflake,
        Dialect::Snowflake,
    );
}

#[test]
fn issue_30_ltrim_rtrim_preserve_chars_ansi() {
    for (sql, expected) in [
        (
            "SELECT LTRIM(a, 'x') FROM t",
            "SELECT TRIM(LEADING 'x' FROM a) FROM t",
        ),
        (
            "SELECT RTRIM(a, 'x') FROM t",
            "SELECT TRIM(TRAILING 'x' FROM a) FROM t",
        ),
        (
            "SELECT TRIM(a, 'x') FROM t",
            "SELECT TRIM('x' FROM a) FROM t",
        ),
        (
            "SELECT LTRIM(a) FROM t",
            "SELECT TRIM(LEADING FROM a) FROM t",
        ),
        (
            "SELECT RTRIM(a) FROM t",
            "SELECT TRIM(TRAILING FROM a) FROM t",
        ),
        ("SELECT TRIM(a) FROM t", "SELECT TRIM(a) FROM t"),
    ] {
        validate_with_dialect(sql, expected, Dialect::Postgres, Dialect::Postgres);
    }
}

#[test]
fn issue_30_ltrim_rtrim_preserve_chars_comma_form() {
    for sql in [
        "SELECT LTRIM(a, 'x') FROM t",
        "SELECT RTRIM(a, 'x') FROM t",
        "SELECT TRIM(a, 'x') FROM t",
        "SELECT LTRIM(a) FROM t",
        "SELECT RTRIM(a) FROM t",
        "SELECT TRIM(a) FROM t",
    ] {
        validate_with_dialect(sql, sql, Dialect::Snowflake, Dialect::Snowflake);
    }
}

#[test]
fn issue_30_ltrim_rtrim_preserve_chars_oracle() {
    for (sql, expected) in [
        (
            "SELECT LTRIM(a, 'x') FROM t",
            "SELECT TRIM(LEADING 'x' FROM a) FROM t",
        ),
        (
            "SELECT RTRIM(a, 'x') FROM t",
            "SELECT TRIM(TRAILING 'x' FROM a) FROM t",
        ),
        (
            "SELECT TRIM(a, 'x') FROM t",
            "SELECT TRIM('x' FROM a) FROM t",
        ),
        (
            "SELECT LTRIM(a) FROM t",
            "SELECT TRIM(LEADING FROM a) FROM t",
        ),
        (
            "SELECT RTRIM(a) FROM t",
            "SELECT TRIM(TRAILING FROM a) FROM t",
        ),
        ("SELECT TRIM(a) FROM t", "SELECT TRIM(a) FROM t"),
    ] {
        validate_with_dialect(sql, expected, Dialect::Oracle, Dialect::Oracle);
    }
}

#[test]
fn issue_30_ltrim_rtrim_preserve_chars_tsql() {
    // Directional TRIM with custom characters requires SQL Server 2022 and
    // database compatibility level 160. No-character calls use native LTRIM/RTRIM.
    for (sql, expected) in [
        (
            "SELECT LTRIM(a, 'x') FROM t",
            "SELECT TRIM(LEADING 'x' FROM a) FROM t",
        ),
        (
            "SELECT RTRIM(a, 'x') FROM t",
            "SELECT TRIM(TRAILING 'x' FROM a) FROM t",
        ),
        (
            "SELECT TRIM(a, 'x') FROM t",
            "SELECT TRIM('x' FROM a) FROM t",
        ),
        ("SELECT LTRIM(a) FROM t", "SELECT LTRIM(a) FROM t"),
        ("SELECT RTRIM(a) FROM t", "SELECT RTRIM(a) FROM t"),
        ("SELECT TRIM(a) FROM t", "SELECT TRIM(a) FROM t"),
    ] {
        validate_with_dialect(sql, expected, Dialect::Tsql, Dialect::Tsql);
    }
}

#[test]
fn cr024_ansi_trim_roundtrip_controls() {
    // ANSI FROM-form round-trips unchanged on PostgreSQL, Oracle, MySQL.
    validate_with_dialect(
        "SELECT TRIM(LEADING ' ' FROM name) FROM t",
        "SELECT TRIM(LEADING ' ' FROM name) FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT TRIM(LEADING ' ' FROM name) FROM t",
        "SELECT TRIM(LEADING ' ' FROM name) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
    validate_with_dialect(
        "SELECT TRIM(BOTH ' ' FROM name) FROM t",
        "SELECT TRIM(' ' FROM name) FROM t",
        Dialect::Postgres,
        Dialect::Mysql,
    );
}

#[test]
fn cr024_pg_to_tsql_trim_single_arg_control() {
    // Single-arg form was always correct; it must remain unchanged.
    validate_with_dialect(
        "SELECT TRIM('  hi  ')",
        "SELECT TRIM('  hi  ')",
        Dialect::Postgres,
        Dialect::Tsql,
    );
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

// ── CR-017: statistical aggregates lowered to T-SQL spellings ──

#[test]
fn test_pg_to_tsql_stddev() {
    // STDDEV / STDDEV_SAMP (sample) -> T-SQL STDEV
    validate_with_dialect(
        "SELECT STDDEV(x) FROM t",
        "SELECT STDEV(x) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT STDDEV_SAMP(x) FROM t",
        "SELECT STDEV(x) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_stddev_pop() {
    validate_with_dialect(
        "SELECT STDDEV_POP(x) FROM t",
        "SELECT STDEVP(x) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_var_pop() {
    validate_with_dialect(
        "SELECT VAR_POP(x) FROM t",
        "SELECT VARP(x) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_variance_controls() {
    // VARIANCE / VAR_SAMP (sample) -> T-SQL VAR (already correct; guard against regression)
    validate_with_dialect(
        "SELECT VARIANCE(x) FROM t",
        "SELECT VAR(x) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT VAR_SAMP(x) FROM t",
        "SELECT VAR(x) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_stat_aggregates_non_tsql_passthrough() {
    // Non-T-SQL dialects keep the ANSI/Postgres spellings (population variants
    // previously fell through as generic functions -> identical result).
    validate_with_dialect(
        "SELECT STDDEV_POP(x) FROM t",
        "SELECT STDDEV_POP(x) FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT VAR_POP(x) FROM t",
        "SELECT VAR_POP(x) FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
    // Oracle natively supports STDDEV / STDDEV_POP -> left unchanged.
    validate_with_dialect(
        "SELECT STDDEV(x) FROM t",
        "SELECT STDDEV(x) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
    validate_with_dialect(
        "SELECT STDDEV_POP(x) FROM t",
        "SELECT STDDEV_POP(x) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
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

// ═════════════════════════════════════════════════════════════════════════════
// CR-003: ANSI typed string literals (DATE 'x', TIMESTAMP 'x', TIME 'x')
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_date_literal_roundtrip_oracle() {
    validate_with_dialect(
        "SELECT DATE '2024-01-01' FROM DUAL",
        "SELECT DATE '2024-01-01' FROM DUAL",
        Dialect::Oracle,
        Dialect::Oracle,
    );
}

#[test]
fn test_timestamp_literal_roundtrip_postgres() {
    validate_with_dialect(
        "SELECT TIMESTAMP '2024-06-15 10:30:00'",
        "SELECT TIMESTAMP '2024-06-15 10:30:00'",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn test_date_literal_in_where_clause() {
    validate_with_dialect(
        "SELECT * FROM orders WHERE order_date > DATE '2024-01-01'",
        "SELECT * FROM orders WHERE order_date > DATE '2024-01-01'",
        Dialect::Oracle,
        Dialect::Oracle,
    );
}

#[test]
fn test_timestamp_literal_in_between() {
    validate_with_dialect(
        "SELECT * FROM events WHERE ts BETWEEN TIMESTAMP '2024-01-01 00:00:00' AND TIMESTAMP '2024-12-31 23:59:59'",
        "SELECT * FROM events WHERE ts BETWEEN TIMESTAMP '2024-01-01 00:00:00' AND TIMESTAMP '2024-12-31 23:59:59'",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_date_literal_cross_dialect_postgres_to_oracle() {
    validate_with_dialect(
        "SELECT * FROM t WHERE created_at >= DATE '2024-06-01'",
        "SELECT * FROM t WHERE created_at >= DATE '2024-06-01'",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_date_literal_oracle_to_mysql() {
    validate_with_dialect(
        "SELECT DATE '2024-01-01' FROM DUAL",
        "SELECT CAST('2024-01-01' AS DATE) FROM DUAL",
        Dialect::Oracle,
        Dialect::Mysql,
    );
}

#[test]
fn test_date_literal_in_insert() {
    validate_with_dialect(
        "INSERT INTO t (id, created) VALUES (1, DATE '2024-01-01')",
        "INSERT INTO t (id, created) VALUES (1, DATE '2024-01-01')",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_multiple_date_literals() {
    validate_with_dialect(
        "SELECT * FROM t WHERE d BETWEEN DATE '2024-01-01' AND DATE '2024-12-31'",
        "SELECT * FROM t WHERE d BETWEEN DATE '2024-01-01' AND DATE '2024-12-31'",
        Dialect::Oracle,
        Dialect::Oracle,
    );
}

#[test]
fn test_time_literal() {
    validate_with_dialect(
        "SELECT * FROM t WHERE start_time > TIME '10:30:00'",
        "SELECT * FROM t WHERE start_time > TIME '10:30:00'",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn test_date_as_column_name() {
    // DATE without a following string literal must still parse as a column
    validate_with_dialect(
        "SELECT date FROM t",
        "SELECT date FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn test_date_as_function_call() {
    // DATE(...) must still parse as a function call
    validate_with_dialect(
        "SELECT DATE(timestamp_col) FROM t",
        "SELECT DATE(timestamp_col) FROM t",
        Dialect::BigQuery,
        Dialect::BigQuery,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-004: PostgreSQL → T-SQL Dialect Translation Gaps
// ═════════════════════════════════════════════════════════════════════════════

// ── Change 1: Boolean Literal → 1/0 ─────────────────────────────────────────

#[test]
fn test_pg_to_tsql_boolean_true() {
    validate_with_dialect("SELECT TRUE", "SELECT 1", Dialect::Postgres, Dialect::Tsql);
}

#[test]
fn test_pg_to_tsql_boolean_false() {
    validate_with_dialect("SELECT FALSE", "SELECT 0", Dialect::Postgres, Dialect::Tsql);
}

#[test]
fn test_pg_to_tsql_boolean_in_where() {
    validate_with_dialect(
        "SELECT * FROM t WHERE active = TRUE",
        "SELECT * FROM t WHERE active = 1",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Change 2: EXTRACT → DATEPART ────────────────────────────────────────────

#[test]
fn test_pg_to_tsql_extract_year() {
    validate_with_dialect(
        "SELECT EXTRACT(YEAR FROM d) FROM t",
        "SELECT DATEPART(YEAR, d) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_extract_month() {
    validate_with_dialect(
        "SELECT EXTRACT(MONTH FROM created_at) FROM t",
        "SELECT DATEPART(MONTH, created_at) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_extract_epoch() {
    validate_with_dialect(
        "SELECT EXTRACT(EPOCH FROM ts) FROM t",
        "SELECT DATEDIFF(SECOND, '1970-01-01', ts) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_extract_dow() {
    // PG DOW = 0(Sun)..6(Sat). Preserve numbering independent of @@DATEFIRST
    // (T-SQL DATEPART(weekday, ..) is 1..7 and @@DATEFIRST-dependent).
    validate_with_dialect(
        "SELECT EXTRACT(DOW FROM created_at) FROM t",
        "SELECT (DATEPART(WEEKDAY, created_at) + @@DATEFIRST - 1) % 7 FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_extract_doy() {
    validate_with_dialect(
        "SELECT EXTRACT(DOY FROM created_at) FROM t",
        "SELECT DATEPART(DAYOFYEAR, created_at) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_extract_week_is_iso() {
    // PG EXTRACT(WEEK ..) is ISO-8601; map to T-SQL ISO_WEEK (not plain `week`).
    validate_with_dialect(
        "SELECT EXTRACT(WEEK FROM created_at) FROM t",
        "SELECT DATEPART(ISO_WEEK, created_at) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_extract_quarter() {
    validate_with_dialect(
        "SELECT EXTRACT(QUARTER FROM created_at) FROM t",
        "SELECT DATEPART(QUARTER, created_at) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── CR-020 (PSQ-2759): PG EXTRACT(QUARTER/WEEK/DOY/DOW) → Oracle ────────────
// Oracle EXTRACT rejects these Postgres-only fields (ORA-00907). Rewrite to
// TO_CHAR/date arithmetic, wrapped in TO_NUMBER to preserve the numeric type.

#[test]
fn test_pg_to_oracle_extract_quarter() {
    validate_with_dialect(
        "SELECT EXTRACT(QUARTER FROM created_at) FROM t",
        "SELECT TO_NUMBER(TO_CHAR(created_at, 'Q')) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_extract_week_is_iso() {
    // PG EXTRACT(WEEK) is ISO-8601 → 'IW' (not 'WW', which is Jan-1-based).
    validate_with_dialect(
        "SELECT EXTRACT(WEEK FROM created_at) FROM t",
        "SELECT TO_NUMBER(TO_CHAR(created_at, 'IW')) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_extract_doy() {
    validate_with_dialect(
        "SELECT EXTRACT(DOY FROM created_at) FROM t",
        "SELECT TO_NUMBER(TO_CHAR(created_at, 'DDD')) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_extract_dow() {
    // PG DOW = 0(Sun)..6(Sat). Fixed-anchor MOD keeps numbering NLS-independent.
    validate_with_dialect(
        "SELECT EXTRACT(DOW FROM created_at) FROM t",
        "SELECT MOD(TRUNC(created_at) - DATE '1970-01-04', 7) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_extract_year_native_unchanged() {
    // Native Oracle EXTRACT fields must stay as-is.
    validate_with_dialect(
        "SELECT EXTRACT(YEAR FROM d) FROM t",
        "SELECT EXTRACT(YEAR FROM d) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_extract_month_native_unchanged() {
    validate_with_dialect(
        "SELECT EXTRACT(MONTH FROM d) FROM t",
        "SELECT EXTRACT(MONTH FROM d) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_pg_extract_dow_identity_unchanged() {
    // Generic (non-Oracle, non-T-SQL) path must be untouched by the Oracle branch.
    validate_with_dialect(
        "SELECT EXTRACT(DOW FROM x)",
        "SELECT EXTRACT(DOW FROM x)",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn test_pg_to_tsql_extract_dow_still_datepart() {
    // CR-016 T-SQL path must be untouched by the Oracle branch.
    validate_with_dialect(
        "SELECT EXTRACT(DOW FROM x)",
        "SELECT (DATEPART(WEEKDAY, x) + @@DATEFIRST - 1) % 7",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── PSQ-2848: portable Oracle generation (Oracle 21c and earlier) ───────────
// Oracle <= 21c rejects FROM-less SELECT (ORA-00923) and boolean literals
// (ORA-00904/00920). Emit `FROM DUAL` for FROM-less selects and lower booleans
// to 1/0 (operand position) or 1 = 1 / 1 = 0 (bare condition position).

#[test]
fn test_pg_to_oracle_fromless_select_gets_dual() {
    validate_with_dialect(
        "SELECT 1",
        "SELECT 1 FROM DUAL",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_scalar_subquery_gets_dual() {
    validate_with_dialect(
        "SELECT (SELECT 1) FROM t",
        "SELECT (SELECT 1 FROM DUAL) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_nested_scalar_subquery_gets_dual() {
    // Acceptance: FROM-less scalar subqueries render with FROM DUAL when nested.
    validate_with_dialect(
        "SELECT (SELECT (SELECT 1)) FROM t",
        "SELECT (SELECT (SELECT 1 FROM DUAL) FROM DUAL) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_derived_fromless_select_gets_dual() {
    validate_with_dialect(
        "SELECT * FROM (SELECT 1) x",
        "SELECT * FROM (SELECT 1 FROM DUAL) x",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_exists_fromless_select_gets_dual() {
    validate_with_dialect(
        "SELECT * FROM t WHERE EXISTS (SELECT 1)",
        "SELECT * FROM t WHERE EXISTS (SELECT 1 FROM DUAL)",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_setop_branches_each_get_dual() {
    validate_with_dialect(
        "SELECT 1 UNION SELECT 2",
        "SELECT 1 FROM DUAL UNION SELECT 2 FROM DUAL",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_select_with_from_unchanged() {
    // Control: a SELECT that already has a FROM must not gain a second source.
    validate_with_dialect(
        "SELECT a FROM t",
        "SELECT a FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_oracle_identity_select_from_dual() {
    // Control: an explicit FROM DUAL round-trips without duplication.
    validate_with_dialect(
        "SELECT 1 FROM DUAL",
        "SELECT 1 FROM DUAL",
        Dialect::Oracle,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_boolean_operand_true() {
    validate_with_dialect(
        "SELECT * FROM t WHERE col = TRUE",
        "SELECT * FROM t WHERE col = 1",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_boolean_operand_false() {
    validate_with_dialect(
        "SELECT * FROM t WHERE col = FALSE",
        "SELECT * FROM t WHERE col = 0",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_boolean_projection_gets_dual() {
    // Both changes at once: TRUE -> 1 in projection, plus FROM DUAL.
    validate_with_dialect(
        "SELECT TRUE",
        "SELECT 1 FROM DUAL",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_bare_boolean_condition_true() {
    // Bare boolean in condition position: `WHERE 1` is invalid Oracle, so 1 = 1.
    validate_with_dialect(
        "SELECT * FROM t WHERE TRUE",
        "SELECT * FROM t WHERE 1 = 1",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_bare_boolean_condition_false() {
    validate_with_dialect(
        "SELECT * FROM t WHERE FALSE",
        "SELECT * FROM t WHERE 1 = 0",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_bare_boolean_in_connective() {
    // Bare boolean nested under a logical connective is lowered too.
    validate_with_dialect(
        "SELECT a FROM t WHERE TRUE AND x > 1",
        "SELECT a FROM t WHERE 1 = 1 AND x > 1",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_pg_fromless_select_unchanged() {
    // Control: the generic (non-Oracle) path never appends FROM DUAL.
    validate_with_dialect("SELECT 1", "SELECT 1", Dialect::Postgres, Dialect::Postgres);
}

#[test]
fn test_pg_to_pg_boolean_unchanged() {
    // Control: booleans stay TRUE/FALSE on Postgres.
    validate_with_dialect(
        "SELECT * FROM t WHERE col = TRUE",
        "SELECT * FROM t WHERE col = TRUE",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn test_pg_to_tsql_fromless_select_no_dual() {
    // Control: FROM DUAL is Oracle-only; T-SQL must not gain it.
    validate_with_dialect("SELECT 1", "SELECT 1", Dialect::Postgres, Dialect::Tsql);
}

#[test]
fn test_pg_to_tsql_boolean_operand_still_one() {
    // Control: the CR-008/CR-018 T-SQL boolean path is untouched by PSQ-2848.
    validate_with_dialect(
        "SELECT * FROM t WHERE col = TRUE",
        "SELECT * FROM t WHERE col = 1",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Change 3: LIMIT/OFFSET → OFFSET/FETCH ──────────────────────────────────

#[test]
fn test_pg_to_tsql_limit_offset() {
    validate_with_dialect(
        "SELECT * FROM t ORDER BY a LIMIT 10 OFFSET 5",
        "SELECT * FROM t ORDER BY a OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_limit_offset_no_order_by() {
    // Should add ORDER BY (SELECT NULL) when none present
    validate_with_dialect(
        "SELECT * FROM t LIMIT 10 OFFSET 5",
        "SELECT * FROM t ORDER BY (SELECT NULL) OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Change 4: Data Type Mapping ─────────────────────────────────────────────

#[test]
fn test_pg_to_tsql_text_type() {
    // PostgreSQL TEXT is a large-object string → VARCHAR(MAX) (CR-023).
    // Previously emitted a bare VARCHAR, which MSSQL treats as VARCHAR(30)
    // in a CAST, silently truncating the value.
    validate_with_dialect(
        "SELECT CAST(x AS TEXT) FROM t",
        "SELECT CAST(x AS VARCHAR(MAX)) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_boolean_type() {
    validate_with_dialect(
        "SELECT CAST(x AS BOOLEAN) FROM t",
        "SELECT CAST(x AS BIT) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_bytea_type() {
    validate_with_dialect(
        "SELECT CAST(x AS BYTEA) FROM t",
        "SELECT CAST(x AS VARBINARY(MAX)) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_tsql_to_pg_varbinary_type() {
    validate_with_dialect(
        "SELECT CAST(x AS VARBINARY(100)) FROM t",
        "SELECT x::BYTEA FROM t",
        Dialect::Tsql,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT CAST(x AS VARBINARY(MAX)) FROM t",
        "SELECT x::BYTEA FROM t",
        Dialect::Tsql,
        Dialect::Postgres,
    );
}

#[test]
fn test_pg_to_tsql_serial_type() {
    validate_with_dialect(
        "CREATE TABLE t (id SERIAL)",
        "CREATE TABLE t (id INT)",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_bigserial_type() {
    validate_with_dialect(
        "CREATE TABLE t (id BIGSERIAL)",
        "CREATE TABLE t (id BIGINT)",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_timestamp_type() {
    validate_with_dialect(
        "SELECT CAST(x AS TIMESTAMP) FROM t",
        "SELECT CAST(x AS DATETIME2) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Change 5: String Concatenation → CONCAT() ──────────────────────────────

#[test]
fn test_pg_to_tsql_concat_operator() {
    validate_with_dialect(
        "SELECT a || b FROM t",
        "SELECT CONCAT(a, b) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_concat_chain() {
    validate_with_dialect(
        "SELECT a || b || c FROM t",
        "SELECT CONCAT(a, b, c) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Change 6: INTERVAL Arithmetic → DATEADD ────────────────────────────────

#[test]
fn test_pg_to_tsql_interval_add() {
    validate_with_dialect(
        "SELECT ts + INTERVAL '7' DAY FROM t",
        "SELECT DATEADD(DAY, 7, ts) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_interval_subtract() {
    validate_with_dialect(
        "SELECT ts - INTERVAL '1' HOUR FROM t",
        "SELECT DATEADD(HOUR, -1, ts) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Change 7: RETURNING → OUTPUT ────────────────────────────────────────────

#[test]
fn test_pg_to_tsql_insert_returning() {
    validate_with_dialect(
        "INSERT INTO t (a, b) VALUES (1, 2) RETURNING id",
        "INSERT INTO t (a, b) OUTPUT INSERTED.id VALUES (1, 2)",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_pg_to_tsql_delete_returning() {
    validate_with_dialect(
        "DELETE FROM t WHERE id = 1 RETURNING *",
        "DELETE FROM t OUTPUT DELETED.* WHERE id = 1",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── PSQ-2414: DELETE WHERE clause expression transformation ─────────────────

#[test]
fn test_pg_to_oracle_delete_where_transform() {
    // Verify that transform_expr is applied to DELETE WHERE clause.
    // SUBSTRING → SUBSTR for Oracle.
    validate_with_dialect(
        "DELETE FROM t WHERE SUBSTRING(status, 1, 3) = 'act'",
        "DELETE FROM t WHERE SUBSTR(status, 1, 3) = 'act'",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_pg_to_oracle_delete_simple_equality() {
    // Simple equality DELETE should pass through correctly.
    validate_with_dialect(
        "DELETE FROM customers WHERE customer_id = 'TEST_DEL'",
        "DELETE FROM customers WHERE customer_id = 'TEST_DEL'",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

// ── Change 8: POSITION → CHARINDEX ─────────────────────────────────────────

#[test]
fn test_pg_to_tsql_position() {
    validate_with_dialect(
        "SELECT POSITION(substr, name) FROM t",
        "SELECT CHARINDEX(substr, name) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Change 9: SIMILAR TO → LIKE ────────────────────────────────────────────

#[test]
fn test_pg_to_tsql_similar_to() {
    validate_with_dialect(
        "SELECT * FROM t WHERE name SIMILAR TO '%test%'",
        "SELECT * FROM t WHERE name LIKE '%test%'",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Change 10: ARRAY → Error ────────────────────────────────────────────────

#[test]
fn test_pg_to_tsql_array_errors() {
    let result = transpile(
        "SELECT ARRAY[1, 2, 3] FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    assert!(result.is_err(), "ARRAY should be unsupported for T-SQL");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("ARRAY"),
        "Error should mention ARRAY: {}",
        err
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-014: Parenthesised set operation as a derived table
// `FROM ((SELECT …) EXCEPT|UNION|INTERSECT (SELECT …)) alias`
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn cr014_parse_paren_setop_derived_table() {
    // Each set-op branch is individually parenthesised — must parse.
    for op in ["EXCEPT", "UNION", "UNION ALL", "INTERSECT"] {
        let sql = format!("SELECT count(*) FROM ((SELECT 1) {op} (SELECT 2)) x");
        assert!(parse(&sql, Dialect::Postgres).is_ok(), "must parse: {sql}");
    }
}

#[test]
fn cr014_parse_chained_except_derived_table() {
    // TPC-DS q87 shape: chained EXCEPT of parenthesised branches.
    let sql = "SELECT count(*) FROM ((SELECT 1 AS a) EXCEPT (SELECT 2 AS a) \
               EXCEPT (SELECT 3 AS a)) cool_cust";
    assert!(
        parse(sql, Dialect::Postgres).is_ok(),
        "chained EXCEPT must parse"
    );
}

#[test]
fn cr014_controls_still_parse() {
    // Redundant nesting and no-branch-parens set-op were already OK.
    assert!(parse("SELECT count(*) FROM ((SELECT 1)) x", Dialect::Postgres).is_ok());
    assert!(
        parse(
            "SELECT count(*) FROM (SELECT 1 EXCEPT SELECT 2) x",
            Dialect::Postgres
        )
        .is_ok()
    );
}

#[test]
fn cr014_transpile_paren_setop_pg_to_tsql() {
    let out = transpile(
        "SELECT count(*) FROM ((SELECT 1) EXCEPT (SELECT 2)) x",
        Dialect::Postgres,
        Dialect::Tsql,
    )
    .unwrap();
    let u = out.to_uppercase();
    assert!(u.contains("EXCEPT"), "set op preserved: {out}");
    assert!(
        u.contains(" X") || u.ends_with('X'),
        "alias preserved: {out}"
    );
}

#[test]
fn cr014_transpile_paren_setop_pg_identity() {
    // PG → PG round-trip must succeed (no parser error) for all four operators.
    for op in ["EXCEPT", "UNION", "UNION ALL", "INTERSECT"] {
        let sql = format!("SELECT count(*) FROM ((SELECT 1) {op} (SELECT 2)) x");
        assert!(
            transpile(&sql, Dialect::Postgres, Dialect::Postgres).is_ok(),
            "{sql}"
        );
    }
}

// ── CR-018: bare boolean in a condition position wrapped to `= 1` for T-SQL ──
//
// SQL Server has no native boolean type, so a bare boolean expression in a
// search-condition position (WHERE / HAVING / QUALIFY / JOIN … ON / searched
// CASE WHEN / AND / OR / NOT) is rejected with error 4145. The generator wraps
// such a bare boolean as `<expr> = 1` for the T-SQL family only.

#[test]
fn cr018_where_bare_boolean_pg_to_tsql() {
    validate_with_dialect(
        "SELECT 1 FROM t WHERE b",
        "SELECT 1 FROM t WHERE b = 1",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr018_where_not_bare_boolean() {
    // `NOT b` → `NOT b = 1`, which T-SQL parses as `NOT (b = 1)` (`=` binds
    // tighter than `NOT`) — identical 3-valued logic to PostgreSQL `NOT b`.
    validate_with_dialect(
        "SELECT 1 FROM t WHERE NOT b",
        "SELECT 1 FROM t WHERE NOT b = 1",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr018_where_and_mixed_with_predicate() {
    // Only the bare operand is wrapped; the existing IS NOT NULL predicate stays.
    validate_with_dialect(
        "SELECT 1 FROM t WHERE b AND x IS NOT NULL",
        "SELECT 1 FROM t WHERE b = 1 AND x IS NOT NULL",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr018_where_or_both_bare() {
    validate_with_dialect(
        "SELECT 1 FROM t WHERE b OR c",
        "SELECT 1 FROM t WHERE b = 1 OR c = 1",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr018_searched_case_when_bare_boolean() {
    // The smoking-gun case from the ticket: bare boolean in a searched CASE WHEN.
    let out = transpile(
        "SELECT SUM(CASE WHEN b THEN 1 ELSE 0 END) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    )
    .unwrap();
    assert!(out.contains("WHEN b = 1 THEN"), "got: {out}");
    assert!(!out.contains("WHEN b THEN"), "must not stay bare: {out}");
}

#[test]
fn cr018_join_on_bare_boolean() {
    let out = transpile(
        "SELECT 1 FROM t1 JOIN t2 ON t2.b",
        Dialect::Postgres,
        Dialect::Tsql,
    )
    .unwrap();
    assert!(out.contains("ON t2.b = 1"), "got: {out}");
}

#[test]
fn cr018_where_boolean_literals() {
    // Bare TRUE / FALSE in a condition position → `1 = 1` / `1 = 0`, not the
    // invalid bare `1` / `0` produced by the value-context Boolean arm.
    validate_with_dialect(
        "SELECT 1 FROM t WHERE TRUE",
        "SELECT 1 FROM t WHERE 1 = 1",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT 1 FROM t WHERE FALSE",
        "SELECT 1 FROM t WHERE 1 = 0",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── CR-018 controls: predicates already valid must NOT be double-wrapped ──

#[test]
fn cr018_control_existing_predicate_not_double_wrapped() {
    validate_with_dialect(
        "SELECT 1 FROM t WHERE b = 1",
        "SELECT 1 FROM t WHERE b = 1",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT 1 FROM t WHERE x IS NULL",
        "SELECT 1 FROM t WHERE x IS NULL",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT 1 FROM t WHERE x BETWEEN 1 AND 9",
        "SELECT 1 FROM t WHERE x BETWEEN 1 AND 9",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr018_control_simple_case_when_value_not_wrapped() {
    // A simple CASE (`CASE <operand> WHEN <value>`) compares each WHEN value
    // against the operand — it is NOT a condition position and must be left
    // alone (wrapping it would produce the broken `WHEN 1 = 1`).
    let out = transpile(
        "SELECT CASE status WHEN 1 THEN 'a' ELSE 'b' END FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    )
    .unwrap();
    assert!(out.contains("WHEN 1 THEN"), "got: {out}");
    assert!(!out.contains("1 = 1"), "simple CASE wrongly wrapped: {out}");
}

#[test]
fn cr018_control_non_tsql_passthrough() {
    // Non-T-SQL targets must be byte-for-byte unchanged (the helper delegates
    // straight to gen_expr off the T-SQL family).
    validate_with_dialect(
        "SELECT 1 FROM t WHERE b",
        "SELECT 1 FROM t WHERE b",
        Dialect::Postgres,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT 1 FROM t WHERE TRUE",
        "SELECT 1 FROM t WHERE TRUE",
        Dialect::Postgres,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT SUM(CASE WHEN b THEN 1 ELSE 0 END) FROM t",
        "SELECT SUM(CASE WHEN b THEN 1 ELSE 0 END) FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

// ── CR-019: safe divide for T-SQL — `a / b` → `a / NULLIF(b, 0)` ──
//
// SQL Server does not short-circuit the ANDed WHERE qual list the way
// PostgreSQL does, so a divisor that is zero only for rows a guard was meant
// to exclude is still evaluated, raising "Divide by zero error" (code 8134).
// The transform wraps the divisor in NULLIF(divisor, 0) for the T-SQL family
// so a zero divisor becomes NULL (and the surrounding comparison drops the
// row) instead of erroring. NULLIF(b, 0) == b whenever b <> 0.

#[test]
fn cr019_divide_pg_to_tsql() {
    validate_with_dialect(
        "SELECT a / b FROM t",
        "SELECT a / NULLIF(b, 0) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr019_divide_cast_divisor() {
    validate_with_dialect(
        "SELECT a / CAST(b AS DECIMAL(18, 4)) FROM t",
        "SELECT a / NULLIF(CAST(b AS DECIMAL(18, 4)), 0) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr019_modulo_pg_to_tsql() {
    // `x % 0` raises the same 8134 error class, so modulo is guarded too.
    validate_with_dialect(
        "SELECT a % b FROM t",
        "SELECT a % NULLIF(b, 0) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr019_divide_parenthesized_dividend() {
    // Only the divisor is guarded; the parenthesized dividend is untouched.
    validate_with_dialect(
        "SELECT (x + y) / z FROM t",
        "SELECT (x + y) / NULLIF(z, 0) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr019_q74_outer_predicate() {
    // The reported q74 shape: the guarded division in the outer WHERE.
    let out = transpile(
        "SELECT count(*) FROM (SELECT c.id cid, SUM(t.y1) y1, SUM(t.y2) y2 \
         FROM t GROUP BY c.id) v \
         WHERE v.y1 > 0 AND v.y2 > 0 AND v.y2 / CAST(v.y1 AS DECIMAL(18, 4)) > 1.0",
        Dialect::Postgres,
        Dialect::Tsql,
    )
    .unwrap();
    assert!(
        out.contains("v.y2 / NULLIF(CAST(v.y1 AS DECIMAL(18, 4)), 0)"),
        "divisor must be zero-guarded: {out}"
    );
}

#[test]
fn cr019_already_guarded_divisor_not_double_wrapped() {
    // A divisor already written as NULLIF(<x>, 0) in the source must not be
    // wrapped again into NULLIF(NULLIF(<x>, 0), 0).
    validate_with_dialect(
        "SELECT a / NULLIF(b, 0) FROM t",
        "SELECT a / NULLIF(b, 0) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr019_control_non_tsql_passthrough() {
    // Non-T-SQL targets must be byte-for-byte unchanged — no NULLIF injected.
    validate_with_dialect(
        "SELECT a / b FROM t",
        "SELECT a / b FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT a % b FROM t",
        "SELECT a % b FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn cr019_control_other_operators_untouched() {
    // Only Divide / Modulo are guarded; other arithmetic operators are left
    // as-is for the T-SQL family.
    validate_with_dialect(
        "SELECT a - b FROM t",
        "SELECT a - b FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT a * b FROM t",
        "SELECT a * b FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-029 (PSQ-3267): the CR-019 safe-divide wrap must NOT be applied to a
// divisor that is a compile-time numeric literal. Wrapping a literal `0`
// (`SELECT 1/0`) turns a genuine, unconditional divide-by-zero — which
// PostgreSQL and Oracle both raise as a hard error — into a silent NULL on
// MSSQL, masking SQL Server code 8134. Wrapping a literal non-zero divisor
// (`SELECT 10/2`) is dead code. Only non-literal divisors (columns, CAST,
// expressions, subqueries) can be undecidably zero at transpile time, so only
// they keep the NULLIF guard — exactly the PSQ-2758 / q74 shape.
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn cr029_literal_divide_by_zero_not_wrapped() {
    // A genuine constant divide-by-zero must be forwarded verbatim so MSSQL
    // raises code 8134 (consistent with PostgreSQL / Oracle), not masked to NULL.
    validate_with_dialect(
        "SELECT 1 / 0",
        "SELECT 1 / 0",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr029_literal_divide_by_zero_nonunit_dividend() {
    validate_with_dialect(
        "SELECT 7 / 0",
        "SELECT 7 / 0",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr029_literal_modulo_by_zero_not_wrapped() {
    // `x % 0` raises the same 8134 class; a literal modulo must not be masked.
    validate_with_dialect(
        "SELECT 1 % 0",
        "SELECT 1 % 0",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr029_literal_nonzero_divisor_not_wrapped() {
    // A literal non-zero divisor can never be 0, so the NULLIF was dead code.
    validate_with_dialect(
        "SELECT 10 / 2",
        "SELECT 10 / 2",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr029_parenthesized_literal_divisor_not_wrapped() {
    // A literal wrapped in redundant parentheses is still a literal — no guard.
    let out = transpile("SELECT a / (0) FROM t", Dialect::Postgres, Dialect::Tsql).unwrap();
    assert!(
        !out.contains("NULLIF"),
        "parenthesized literal divisor must not be zero-guarded: {out}"
    );
}

#[test]
fn cr029_signed_literal_divisor_not_wrapped() {
    // A leading unary sign on a literal is still a literal (exercises the
    // UnaryOp arm of is_numeric_literal); a non-zero literal needs no guard.
    let out = transpile("SELECT a / -1 FROM t", Dialect::Postgres, Dialect::Tsql).unwrap();
    assert!(
        !out.contains("NULLIF"),
        "signed literal divisor must not be zero-guarded: {out}"
    );
}

#[test]
fn cr029_control_column_divisor_still_guarded() {
    // A non-literal (column) divisor keeps the CR-019 guard — PSQ-2758 preserved.
    validate_with_dialect(
        "SELECT a / b FROM t",
        "SELECT a / NULLIF(b, 0) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr029_control_cast_divisor_still_guarded() {
    // A CAST divisor is non-literal — the q74 shape must remain guarded.
    validate_with_dialect(
        "SELECT a / CAST(b AS DECIMAL(18, 4)) FROM t",
        "SELECT a / NULLIF(CAST(b AS DECIMAL(18, 4)), 0) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr029_control_q74_outer_predicate_still_guarded() {
    // The reported q74 outer predicate (non-literal CAST divisor) stays guarded.
    let out = transpile(
        "SELECT count(*) FROM (SELECT c.id cid, SUM(t.y1) y1, SUM(t.y2) y2 \
         FROM t GROUP BY c.id) v \
         WHERE v.y1 > 0 AND v.y2 > 0 AND v.y2 / CAST(v.y1 AS DECIMAL(18, 4)) > 1.0",
        Dialect::Postgres,
        Dialect::Tsql,
    )
    .unwrap();
    assert!(
        out.contains("v.y2 / NULLIF(CAST(v.y1 AS DECIMAL(18, 4)), 0)"),
        "non-literal q74 divisor must stay zero-guarded: {out}"
    );
}

#[test]
fn cr029_control_non_tsql_passthrough() {
    // Non-T-SQL targets are unaffected: PostgreSQL already errors on 1/0.
    validate_with_dialect(
        "SELECT 1 / 0",
        "SELECT 1 / 0",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-027 (PSQ-3243): a correlated subquery whose GROUP BY is a *pure outer
// reference* (e.g. `GROUP BY c.customer_id` where `c` is the outer table) is
// legal in PostgreSQL but rejected by SQL Server with code 164 ("Each GROUP BY
// expression must contain at least one column that is not an outer reference").
// The fix substitutes the correlated inner column from the WHERE equality
// (`c.customer_id` -> `t.customer_id`, byte-exact PG equivalence) or, when no
// correlating equality exists, drops the redundant pure-outer key.
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn cr027_scalar_subquery_substitutes_inner_column() {
    // The reported shape: substitute the correlated inner column so the GROUP BY
    // key is local (semantics-exact — NULL for empty correlated groups).
    validate_with_dialect(
        "SELECT c.customer_id, (SELECT COUNT(*) FROM transactions t \
         WHERE t.customer_id = c.customer_id GROUP BY c.customer_id) AS cnt FROM customers c",
        "SELECT c.customer_id, (SELECT COUNT(*) FROM transactions AS t \
         WHERE t.customer_id = c.customer_id GROUP BY t.customer_id) AS cnt FROM customers AS c",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr027_no_correlating_equality_drops_group_by() {
    // No `<local> = <outer ref>` equality to substitute, and the pure-outer key
    // is the only one, so the redundant GROUP BY is dropped entirely.
    validate_with_dialect(
        "SELECT c.customer_id, (SELECT COUNT(*) FROM transactions t \
         WHERE t.flag = 1 GROUP BY c.customer_id) AS cnt FROM customers c",
        "SELECT c.customer_id, (SELECT COUNT(*) FROM transactions AS t \
         WHERE t.flag = 1) AS cnt FROM customers AS c",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr027_mixed_keys_substitute_outer_keep_local() {
    // Mixed GROUP BY: the pure-outer key is substituted, the local key kept.
    validate_with_dialect(
        "SELECT c.customer_id, (SELECT COUNT(*) FROM transactions t \
         WHERE t.customer_id = c.customer_id GROUP BY c.customer_id, t.status) AS cnt FROM customers c",
        "SELECT c.customer_id, (SELECT COUNT(*) FROM transactions AS t \
         WHERE t.customer_id = c.customer_id GROUP BY t.customer_id, t.status) AS cnt FROM customers AS c",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr027_mixed_keys_no_equality_drops_outer_keep_local() {
    // Mixed GROUP BY with no correlating equality: drop the pure-outer key,
    // keep the local key. The remaining local key keeps the grouping valid.
    validate_with_dialect(
        "SELECT c.customer_id, (SELECT COUNT(*) FROM transactions t \
         WHERE t.flag = 1 GROUP BY c.customer_id, t.status) AS cnt FROM customers c",
        "SELECT c.customer_id, (SELECT COUNT(*) FROM transactions AS t \
         WHERE t.flag = 1 GROUP BY t.status) AS cnt FROM customers AS c",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr027_reversed_equality_substitutes_inner_column() {
    // The correlating equality may be written `c.id = t.customer_id` (outer on
    // the left); the local column is still substituted.
    validate_with_dialect(
        "SELECT c.id, (SELECT COUNT(*) FROM transactions t \
         WHERE c.id = t.customer_id GROUP BY c.id) AS cnt FROM customers c",
        "SELECT c.id, (SELECT COUNT(*) FROM transactions AS t \
         WHERE c.id = t.customer_id GROUP BY t.customer_id) AS cnt FROM customers AS c",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr027_exists_subquery_pure_outer_group_by() {
    // The rewrite reaches subqueries in the WHERE clause (EXISTS), not just the
    // projection list.
    validate_with_dialect(
        "SELECT c.id FROM customers c WHERE EXISTS (SELECT 1 FROM transactions t \
         WHERE t.customer_id = c.id GROUP BY c.id)",
        "SELECT c.id FROM customers AS c WHERE EXISTS (SELECT 1 FROM transactions AS t \
         WHERE t.customer_id = c.id GROUP BY t.customer_id)",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr027_unqualified_group_by_key_untouched() {
    // An unqualified GROUP BY key is conservatively treated as local and never
    // rewritten (it does not trip code 164).
    validate_with_dialect(
        "SELECT c.id, (SELECT COUNT(*) FROM transactions t \
         WHERE t.customer_id = c.id GROUP BY customer_id) AS cnt FROM customers c",
        "SELECT c.id, (SELECT COUNT(*) FROM transactions AS t \
         WHERE t.customer_id = c.id GROUP BY customer_id) AS cnt FROM customers AS c",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr027_control_inner_column_group_by_unchanged() {
    // A GROUP BY that already references a local column is valid T-SQL and must
    // be left byte-for-byte unchanged.
    validate_with_dialect(
        "SELECT c.customer_id, (SELECT COUNT(*) FROM transactions t \
         WHERE t.customer_id = c.customer_id GROUP BY t.customer_id) AS cnt FROM customers c",
        "SELECT c.customer_id, (SELECT COUNT(*) FROM transactions AS t \
         WHERE t.customer_id = c.customer_id GROUP BY t.customer_id) AS cnt FROM customers AS c",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr027_control_real_outer_query_grouping_unchanged() {
    // Top-level grouping by `c.city` where `c` IS local (joined here) must not
    // be treated as an outer reference.
    validate_with_dialect(
        "SELECT c.city, COUNT(t.id) FROM customers c \
         JOIN transactions t ON t.customer_id = c.id GROUP BY c.city",
        "SELECT c.city, COUNT(t.id) FROM customers AS c \
         INNER JOIN transactions AS t ON t.customer_id = c.id GROUP BY c.city",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr027_control_pg_to_pg_passthrough() {
    // PostgreSQL accepts a pure-outer GROUP BY, so PG -> PG must not fire.
    validate_with_dialect(
        "SELECT c.customer_id, (SELECT COUNT(*) FROM transactions t \
         WHERE t.customer_id = c.customer_id GROUP BY c.customer_id) AS cnt FROM customers c",
        "SELECT c.customer_id, (SELECT COUNT(*) FROM transactions AS t \
         WHERE t.customer_id = c.customer_id GROUP BY c.customer_id) AS cnt FROM customers AS c",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn cr027_control_uncorrelated_derived_table_unchanged() {
    // A non-correlated derived table grouping by its own local column is
    // untouched (regression guard for the FROM-subquery descent).
    validate_with_dialect(
        "SELECT x FROM (SELECT COUNT(*) AS x FROM transactions t GROUP BY t.status) d",
        "SELECT x FROM (SELECT COUNT(*) AS x FROM transactions AS t GROUP BY t.status) AS d",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-023 (PSQ-3005): T-SQL NCHAR / NVARCHAR length and VARCHAR(MAX) /
// NVARCHAR(MAX) must round-trip. Previously the length / MAX modifier was
// dropped, so MSSQL applied its CAST default length of 30 (padding NCHAR,
// truncating VARCHAR/NVARCHAR) — silent data corruption.
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn cr023_nchar_length_preserved() {
    validate_with_dialect(
        "SELECT CAST(x AS NCHAR(10)) FROM t",
        "SELECT CAST(x AS NCHAR(10)) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr023_nchar_small_length_preserved() {
    // Disproves the reported "3x padding" theory: the length is preserved
    // verbatim, not multiplied.
    validate_with_dialect(
        "SELECT CAST(x AS NCHAR(2)) FROM t",
        "SELECT CAST(x AS NCHAR(2)) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr023_nvarchar_length_preserved() {
    validate_with_dialect(
        "SELECT CAST(x AS NVARCHAR(10)) FROM t",
        "SELECT CAST(x AS NVARCHAR(10)) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr023_varchar_max_roundtrip() {
    validate_with_dialect(
        "SELECT CAST(x AS VARCHAR(MAX)) FROM t",
        "SELECT CAST(x AS VARCHAR(MAX)) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr023_nvarchar_max_roundtrip() {
    validate_with_dialect(
        "SELECT CAST(x AS NVARCHAR(MAX)) FROM t",
        "SELECT CAST(x AS NVARCHAR(MAX)) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr023_pg_text_to_tsql_varchar_max() {
    // PostgreSQL TEXT is a large-object string; it must map to VARCHAR(MAX),
    // not a bare VARCHAR (which MSSQL treats as VARCHAR(30) in a CAST).
    validate_with_dialect(
        "SELECT CAST(x AS TEXT) FROM t",
        "SELECT CAST(x AS VARCHAR(MAX)) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Controls: finite CHAR / VARCHAR and VARBINARY(MAX) unchanged ─────────────

#[test]
fn cr023_control_char_length_preserved() {
    validate_with_dialect(
        "SELECT CAST(x AS CHAR(10)) FROM t",
        "SELECT CAST(x AS CHAR(10)) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr023_control_varchar_finite_preserved() {
    validate_with_dialect(
        "SELECT CAST(x AS VARCHAR(100)) FROM t",
        "SELECT CAST(x AS VARCHAR(100)) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr023_control_varbinary_max_preserved() {
    // CR-008 behaviour must be preserved: VARBINARY(MAX) still round-trips.
    validate_with_dialect(
        "SELECT CAST(x AS VARBINARY(MAX)) FROM t",
        "SELECT CAST(x AS VARBINARY(MAX)) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Controls: national types degrade to non-national on PostgreSQL ───────────

#[test]
fn cr023_nchar_to_postgres_char() {
    // National → non-national mapping; PostgreSQL emits the `::` cast form.
    validate_with_dialect(
        "SELECT CAST(x AS NCHAR(10)) FROM t",
        "SELECT x::CHAR(10) FROM t",
        Dialect::Tsql,
        Dialect::Postgres,
    );
}

#[test]
fn cr023_nvarchar_max_to_postgres_text() {
    validate_with_dialect(
        "SELECT CAST(x AS NVARCHAR(MAX)) FROM t",
        "SELECT x::TEXT FROM t",
        Dialect::Tsql,
        Dialect::Postgres,
    );
}

#[test]
fn cr023_tsql_identity_nvarchar_max() {
    // Same-dialect round-trip (parse T-SQL → generate T-SQL).
    validate_with_dialect(
        "SELECT CAST(x AS NVARCHAR(MAX)) FROM t",
        "SELECT CAST(x AS NVARCHAR(MAX)) FROM t",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-026 (PSQ-3201): Oracle VARCHAR2 / NVARCHAR2 length must round-trip.
// Previously the length was dropped, so `CAST(x AS VARCHAR2(10))` transpiled to
// a bare `CAST(x AS VARCHAR2)`, which Oracle rejects with ORA-00906 (a length
// is mandatory) — the query failed with SQL_ERROR through the gateway. CR-023
// fixed the same class for the T-SQL family but scoped Oracle out; this closes
// the Oracle gap.
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn cr026_varchar2_length_preserved() {
    // The exact PSQ-3201 repro: a PG→Oracle cast must keep the length.
    validate_with_dialect(
        "SELECT CAST(123 AS VARCHAR2(10)) FROM dual",
        "SELECT CAST(123 AS VARCHAR2(10)) FROM dual",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr026_varchar2_small_length_preserved() {
    // The length is preserved verbatim, not defaulted or dropped.
    validate_with_dialect(
        "SELECT CAST(x AS VARCHAR2(4)) FROM t",
        "SELECT CAST(x AS VARCHAR2(4)) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr026_nvarchar2_length_preserved() {
    validate_with_dialect(
        "SELECT CAST(x AS NVARCHAR2(20)) FROM t",
        "SELECT CAST(x AS NVARCHAR2(20)) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr026_varchar2_char_qualifier_length_preserved() {
    // Oracle `VARCHAR2(n CHAR)` — the numeric length is preserved; the
    // CHAR length-semantics qualifier is tolerated (dropped).
    validate_with_dialect(
        "SELECT CAST(x AS VARCHAR2(10 CHAR)) FROM t",
        "SELECT CAST(x AS VARCHAR2(10)) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr026_varchar2_byte_qualifier_length_preserved() {
    validate_with_dialect(
        "SELECT CAST(x AS VARCHAR2(10 BYTE)) FROM t",
        "SELECT CAST(x AS VARCHAR2(10)) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr026_oracle_identity_varchar2() {
    // Same-dialect round-trip (parse Oracle → generate Oracle).
    validate_with_dialect(
        "SELECT CAST(x AS VARCHAR2(10)) FROM t",
        "SELECT CAST(x AS VARCHAR2(10)) FROM t",
        Dialect::Oracle,
        Dialect::Oracle,
    );
}

// ── Controls: VARCHAR2 / NVARCHAR2 degrade to portable VARCHAR off Oracle ─────

#[test]
fn cr026_varchar2_to_postgres_varchar() {
    // PostgreSQL has no VARCHAR2; it degrades to VARCHAR with the length kept
    // (PostgreSQL emits the `::` cast form).
    validate_with_dialect(
        "SELECT CAST(x AS VARCHAR2(10)) FROM t",
        "SELECT x::VARCHAR(10) FROM t",
        Dialect::Oracle,
        Dialect::Postgres,
    );
}

#[test]
fn cr026_nvarchar2_to_postgres_varchar() {
    validate_with_dialect(
        "SELECT CAST(x AS NVARCHAR2(20)) FROM t",
        "SELECT x::VARCHAR(20) FROM t",
        Dialect::Oracle,
        Dialect::Postgres,
    );
}

#[test]
fn cr026_nvarchar2_to_tsql_nvarchar() {
    // National → national mapping onto the T-SQL family.
    validate_with_dialect(
        "SELECT CAST(x AS NVARCHAR2(20)) FROM t",
        "SELECT CAST(x AS NVARCHAR(20)) FROM t",
        Dialect::Oracle,
        Dialect::Tsql,
    );
}

#[test]
fn cr026_control_varchar_finite_preserved() {
    // Plain VARCHAR(n) is unaffected — Oracle accepts it verbatim.
    validate_with_dialect(
        "SELECT CAST(x AS VARCHAR(100)) FROM t",
        "SELECT CAST(x AS VARCHAR(100)) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-028 (PSQ-3264): T-SQL statement-tail `OPTION ( ... )` query hint.
// The clause (most importantly `OPTION (MAXRECURSION n)` for recursive CTEs)
// must survive a parse→generate round-trip and be emitted only for the T-SQL
// family; every other target drops it (no portable equivalent).
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn cr028_pg_to_tsql_maxrecursion_preserved() {
    // The reported defect: the gateway parses client SQL as Postgres and
    // generates T-SQL; the recursive-CTE OPTION clause must be preserved so
    // MSSQL does not fall back to its default MAXRECURSION 100.
    validate_with_dialect(
        "WITH r (n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM r WHERE n < 200) \
         SELECT count(*) FROM r OPTION (MAXRECURSION 200)",
        "WITH r(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM r WHERE n < 200) \
         SELECT COUNT(*) FROM r OPTION (MAXRECURSION 200)",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr028_tsql_to_tsql_maxrecursion_roundtrip() {
    validate_with_dialect(
        "WITH r (n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM r WHERE n < 200) \
         SELECT count(*) FROM r OPTION (MAXRECURSION 200)",
        "WITH r(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM r WHERE n < 200) \
         SELECT COUNT(*) FROM r OPTION (MAXRECURSION 200)",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

#[test]
fn cr028_maxrecursion_zero_unlimited() {
    // MAXRECURSION 0 means "unlimited" — must be preserved verbatim.
    validate_with_dialect(
        "SELECT count(*) FROM r OPTION (MAXRECURSION 0)",
        "SELECT COUNT(*) FROM r OPTION (MAXRECURSION 0)",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr028_multi_hint_preserved() {
    // Multiple comma-separated hints round-trip with correct spacing.
    validate_with_dialect(
        "SELECT count(*) FROM r OPTION (MAXRECURSION 100, RECOMPILE)",
        "SELECT COUNT(*) FROM r OPTION (MAXRECURSION 100, RECOMPILE)",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr028_trailing_semicolon_handled() {
    // A trailing `;` after the OPTION clause is consumed; the clause survives.
    validate_with_dialect(
        "SELECT a FROM t OPTION (MAXRECURSION 5);",
        "SELECT a FROM t OPTION (MAXRECURSION 5)",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr028_top_level_setop_option_preserved() {
    // OPTION at the tail of a top-level set operation attaches to the
    // SetOperation node (not just plain SELECT).
    validate_with_dialect(
        "SELECT 1 AS n UNION ALL SELECT 2 OPTION (MAXRECURSION 0)",
        "SELECT 1 AS n UNION ALL SELECT 2 OPTION (MAXRECURSION 0)",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr028_control_pg_to_pg_drops_option() {
    // PostgreSQL has no OPTION clause, so a PG target must drop it.
    validate_with_dialect(
        "SELECT count(*) FROM r OPTION (MAXRECURSION 200)",
        "SELECT COUNT(*) FROM r",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn cr028_control_tsql_to_oracle_drops_option() {
    // Oracle has no portable MAXRECURSION equivalent — the clause is dropped.
    // (`FROM DUAL` is unrelated CR-021 behavior for FROM-less selects.)
    validate_with_dialect(
        "SELECT 1 UNION ALL SELECT 2 OPTION (MAXRECURSION 0)",
        "SELECT 1 FROM DUAL UNION ALL SELECT 2 FROM DUAL",
        Dialect::Tsql,
        Dialect::Oracle,
    );
}

#[test]
fn cr028_control_no_false_match_in_list() {
    // A trailing `IN (...)` list must not be mistaken for an OPTION clause.
    validate_with_dialect(
        "SELECT a FROM t WHERE a IN (1, 2, 3)",
        "SELECT a FROM t WHERE a IN (1, 2, 3)",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr028_control_no_false_match_string_literal() {
    // A string literal 'OPTION' is a String token, never matched as the hint.
    validate_with_dialect(
        "SELECT 'OPTION' AS x FROM t",
        "SELECT 'OPTION' AS x FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr028_control_no_false_match_derived_table() {
    // A statement ending in a derived table (subquery) carries no OPTION.
    validate_with_dialect(
        "SELECT x FROM (SELECT 1 AS x) d",
        "SELECT x FROM (SELECT 1 AS x) AS d",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-030 (PSQ-3261): T-SQL / Fabric use `[...]` exclusively for delimited
// identifiers and have no array literal or subscript syntax. The tokenizer's
// context-free `[` disambiguation used to treat `[ident]` as an array subscript
// whenever the previous token was a "subscriptable" value (a number, `)`,
// string, etc.). So re-parsing generated T-SQL like `SELECT TOP 1000 [col]`
// (number `1000` before `[`) misparsed `[col]` as an ARRAY constructor,
// emitting `ARRAY[col]` / `->` — the stray `-` MSSQL rejected on Power BI Load.
// The fix makes `parse()` honor the dialect: for the T-SQL family `[...]` is
// always tokenized as a bracket-quoted identifier. Non-bracket dialects keep
// full array-subscript semantics (see the control tests below).
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn cr030_top_qualified_bracket_projection_roundtrip() {
    // The exact reported Power BI Load query (T-SQL → T-SQL schema-injection
    // round-trip). Must preserve the bracketed identifiers, never become ARRAY.
    validate_with_dialect(
        "SELECT TOP 1000 [rows].[customer_id] FROM (SELECT customer_id FROM gateway.customers) AS [rows]",
        "SELECT TOP 1000 [rows].[customer_id] FROM (SELECT customer_id FROM gateway.customers) AS [rows]",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

#[test]
fn cr030_top_qualified_bracket_pair() {
    // `TOP n` followed by a qualified bracketed identifier `[a].[b]` — the case
    // that previously produced `ARRAY[a]->'b'`.
    validate_with_dialect(
        "SELECT TOP 1000 [a].[b] FROM t",
        "SELECT TOP 1000 [a].[b] FROM t",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

#[test]
fn cr030_top_single_bracket_projection() {
    // `TOP n` followed by a single bracketed identifier — previously parsed as
    // a bare `ARRAY[c]` column ("Invalid column name 'ARRAY'").
    validate_with_dialect(
        "SELECT TOP 1000 [c] FROM t",
        "SELECT TOP 1000 [c] FROM t",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

#[test]
fn cr030_top_parenthesized_before_bracket() {
    // `TOP (n)` leaves `)` (RParen) as the previous token, another subscriptable
    // value that used to trip the heuristic.
    validate_with_dialect(
        "SELECT TOP (10) [c] FROM t",
        "SELECT TOP (10) [c] FROM t",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

#[test]
fn cr030_implicit_bracket_alias_after_number() {
    // Implicit bracket alias after a numeric projection (`5 [b]`) — the same
    // Number-before-`[` trigger; must be an alias, not `5[b]` array indexing.
    validate_with_dialect(
        "SELECT 5 [b] FROM t",
        "SELECT 5 AS [b] FROM t",
        Dialect::Tsql,
        Dialect::Tsql,
    );
}

#[test]
fn cr030_fabric_top_single_bracket_projection() {
    // Fabric is in the T-SQL family and gets the same bracket-identifier rule.
    validate_with_dialect(
        "SELECT TOP 1000 [c] FROM t",
        "SELECT TOP 1000 [c] FROM t",
        Dialect::Fabric,
        Dialect::Fabric,
    );
}

#[test]
fn cr030_tsql_to_postgres_top_bracket() {
    // The bracketed identifiers must survive as identifiers cross-dialect:
    // T-SQL `TOP` → PostgreSQL `LIMIT`, brackets → double quotes.
    validate_with_dialect(
        "SELECT TOP 1000 [rows].[customer_id] FROM (SELECT customer_id FROM gateway.customers) AS [rows]",
        "SELECT \"rows\".\"customer_id\" FROM (SELECT customer_id FROM gateway.customers) AS \"rows\" LIMIT 1000",
        Dialect::Tsql,
        Dialect::Postgres,
    );
}

#[test]
fn cr030_postgres_to_tsql_powerbi_load_shape() {
    // The Npgsql-generated Power BI Load query (PostgreSQL) → T-SQL. This is the
    // shape that, once re-parsed as T-SQL for schema injection, used to corrupt.
    validate_with_dialect(
        "SELECT \"rows\".\"customer_id\" FROM (SELECT * FROM \"gateway\".\"customers\") \"rows\" LIMIT 1000",
        "SELECT TOP 1000 [rows].[customer_id] FROM (SELECT * FROM gateway.[customers]) AS [rows]",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr030_control_postgres_array_subscript_preserved() {
    // Control: non-bracket dialects keep array-subscript semantics unchanged.
    validate_with_dialect(
        "SELECT arr[1] FROM t",
        "SELECT arr[1] FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn cr030_control_postgres_array_literal_preserved() {
    // Control: a genuine PostgreSQL ARRAY literal is untouched.
    validate_with_dialect(
        "SELECT ARRAY[1, 2, 3] FROM t",
        "SELECT ARRAY[1, 2, 3] FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn cr030_control_clickhouse_array_subscript_preserved() {
    // Control: ClickHouse array subscript semantics are unaffected.
    validate_with_dialect(
        "SELECT arr[1] FROM t",
        "SELECT arr[1] FROM t",
        Dialect::ClickHouse,
        Dialect::ClickHouse,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-031 (PSQ-3306 / PSQ-3307 / PSQ-3309): dialect transforms inside nested
// query blocks. `transform_statement` only rewrote the outermost SELECT, so
// LIMIT/OFFSET lowering, no-op nested ORDER BY removal, and aggregate FILTER
// lowering were skipped inside derived tables, CTEs, subqueries, and set-op
// branches. The CR-031 walk applies them at every level for the T-SQL family
// and Oracle. (Doc filed as "CR-030" upstream, but that number shipped for the
// bracket-identifier fix in v0.10.20, so these artifacts use CR-031.)
// ═════════════════════════════════════════════════════════════════════════════

// ── Fix 1 (PSQ-3306): nested LIMIT / OFFSET ─────────────────────────────────

#[test]
fn cr031_nested_limit_offset_tsql() {
    // A paginated derived table lowers to OFFSET ... ROWS FETCH NEXT ... ROWS ONLY.
    validate_with_dialect(
        "SELECT count(*) FROM (SELECT id FROM t ORDER BY id LIMIT 100 OFFSET 100) x",
        "SELECT COUNT(*) FROM (SELECT id FROM t ORDER BY id OFFSET 100 ROWS FETCH NEXT 100 ROWS ONLY) AS x",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_nested_limit_offset_oracle() {
    validate_with_dialect(
        "SELECT count(*) FROM (SELECT id FROM t ORDER BY id LIMIT 100 OFFSET 100) x",
        "SELECT COUNT(*) FROM (SELECT id FROM t ORDER BY id OFFSET 100 ROWS FETCH FIRST 100 ROWS ONLY) x",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr031_nested_limit_only_tsql() {
    // LIMIT with no OFFSET lowers to TOP inside the derived table.
    validate_with_dialect(
        "SELECT count(*) FROM (SELECT id FROM t ORDER BY id LIMIT 100) x",
        "SELECT COUNT(*) FROM (SELECT TOP 100 id FROM t ORDER BY id) AS x",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_nested_limit_only_oracle() {
    // Oracle uses FETCH FIRST (no TOP) for a LIMIT-only nested block.
    validate_with_dialect(
        "SELECT count(*) FROM (SELECT id FROM t LIMIT 5) x",
        "SELECT COUNT(*) FROM (SELECT id FROM t FETCH FIRST 5 ROWS ONLY) x",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr031_scalar_subquery_limit_tsql() {
    validate_with_dialect(
        "SELECT a, (SELECT count(*) FROM u LIMIT 5) FROM t",
        "SELECT a, (SELECT TOP 5 COUNT(*) FROM u) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_in_subquery_limit_tsql() {
    validate_with_dialect(
        "SELECT id FROM t WHERE id IN (SELECT id FROM u LIMIT 5)",
        "SELECT id FROM t WHERE id IN (SELECT TOP 5 id FROM u)",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_exists_subquery_limit_tsql() {
    validate_with_dialect(
        "SELECT id FROM t WHERE EXISTS (SELECT 1 FROM u LIMIT 5)",
        "SELECT id FROM t WHERE EXISTS (SELECT TOP 5 1 FROM u)",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_setop_branches_limit_tsql() {
    // Both UNION branches are rewritten (a top-level set-op is never visited by
    // the shallow transform_statement pass at all).
    validate_with_dialect(
        "SELECT id FROM t LIMIT 5 UNION SELECT id FROM u LIMIT 5",
        "SELECT TOP 5 id FROM t UNION SELECT TOP 5 id FROM u",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_cte_body_limit_tsql() {
    validate_with_dialect(
        "WITH c AS (SELECT id FROM t ORDER BY id LIMIT 5) SELECT * FROM c",
        "WITH c AS (SELECT TOP 5 id FROM t ORDER BY id) SELECT * FROM c",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Fix 2 (PSQ-3307): no-op nested ORDER BY ─────────────────────────────────

#[test]
fn cr031_nested_orderby_no_limit_stripped_tsql() {
    // An ORDER BY with no row-limiting clause cannot affect the outer result and
    // is rejected by SQL Server in a derived table (error 1033) — so it is dropped.
    validate_with_dialect(
        "SELECT count(*) FROM (SELECT id FROM t WHERE c = 'x' ORDER BY id) x",
        "SELECT COUNT(*) FROM (SELECT id FROM t WHERE c = 'x') AS x",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_nested_orderby_no_limit_preserved_oracle() {
    // Oracle accepts a nested ORDER BY, so it must be left untouched.
    validate_with_dialect(
        "SELECT count(*) FROM (SELECT id FROM t WHERE c = 'x' ORDER BY id) x",
        "SELECT COUNT(*) FROM (SELECT id FROM t WHERE c = 'x' ORDER BY id) x",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr031_setop_branch_orderby_stripped_tsql() {
    validate_with_dialect(
        "SELECT id FROM t ORDER BY id UNION SELECT id FROM u ORDER BY id",
        "SELECT id FROM t UNION SELECT id FROM u",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Fix 3 (PSQ-3309, secondary): aggregate FILTER (WHERE p) lowering ─────────

#[test]
fn cr031_count_star_filter_tsql() {
    validate_with_dialect(
        "SELECT COUNT(*) FILTER (WHERE c = 'US') FROM t",
        "SELECT COUNT(CASE WHEN c = 'US' THEN 1 END) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_count_star_filter_oracle() {
    validate_with_dialect(
        "SELECT COUNT(*) FILTER (WHERE c = 'US') FROM t",
        "SELECT COUNT(CASE WHEN c = 'US' THEN 1 END) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr031_count_expr_filter_tsql() {
    validate_with_dialect(
        "SELECT COUNT(x) FILTER (WHERE c = 'US') FROM t",
        "SELECT COUNT(CASE WHEN c = 'US' THEN x END) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_sum_filter_oracle() {
    validate_with_dialect(
        "SELECT SUM(x) FILTER (WHERE c = 'US') FROM t",
        "SELECT SUM(CASE WHEN c = 'US' THEN x END) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr031_distinct_filter_tsql() {
    // DISTINCT is preserved through the CASE lowering.
    validate_with_dialect(
        "SELECT COUNT(DISTINCT x) FILTER (WHERE c = 'US') FROM t",
        "SELECT COUNT(DISTINCT CASE WHEN c = 'US' THEN x END) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_filter_in_having_tsql() {
    validate_with_dialect(
        "SELECT dept FROM t GROUP BY dept HAVING COUNT(*) FILTER (WHERE c = 'US') > 5",
        "SELECT dept FROM t GROUP BY dept HAVING COUNT(CASE WHEN c = 'US' THEN 1 END) > 5",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_nested_filter_derived_table_tsql() {
    // FILTER lowering also reaches aggregates inside a derived table.
    validate_with_dialect(
        "SELECT * FROM (SELECT COUNT(*) FILTER (WHERE c = 'US') AS n FROM t) z",
        "SELECT * FROM (SELECT COUNT(CASE WHEN c = 'US' THEN 1 END) AS n FROM t) AS z",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_filter_preserved_postgres() {
    // PostgreSQL supports FILTER — it must be preserved.
    validate_with_dialect(
        "SELECT COUNT(*) FILTER (WHERE c = 'US') FROM t",
        "SELECT COUNT(*) FILTER (WHERE c = 'US') FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

// ── Controls: top-level behaviour and non-target dialects unchanged ─────────

#[test]
fn cr031_control_top_level_orderby_preserved_tsql() {
    // A top-level ORDER BY is meaningful and must never be stripped.
    validate_with_dialect(
        "SELECT id FROM t ORDER BY id",
        "SELECT id FROM t ORDER BY id",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_control_top_level_limit_offset_unchanged_tsql() {
    // Regression guard for CR-010 / CR-013: top-level LIMIT/OFFSET is unchanged.
    validate_with_dialect(
        "SELECT customer_id FROM customers ORDER BY customer_id LIMIT 100 OFFSET 100",
        "SELECT customer_id FROM customers ORDER BY customer_id OFFSET 100 ROWS FETCH NEXT 100 ROWS ONLY",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn cr031_control_non_target_dialect_filter_preserved() {
    // A dialect that supports FILTER (DuckDB) is outside the CR-031 gate and is
    // left byte-for-byte unchanged.
    validate_with_dialect(
        "SELECT COUNT(*) FILTER (WHERE c = 'US') FROM t",
        "SELECT COUNT(*) FILTER (WHERE c = 'US') FROM t",
        Dialect::Postgres,
        Dialect::DuckDb,
    );
}

#[test]
fn cr031_control_non_target_dialect_nested_limit_untouched() {
    // Nested LIMIT is preserved for a non-target dialect (DuckDB supports it).
    validate_with_dialect(
        "SELECT n FROM (SELECT id AS n FROM t LIMIT 5) z",
        "SELECT n FROM (SELECT id AS n FROM t LIMIT 5) AS z",
        Dialect::Postgres,
        Dialect::DuckDb,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-032 (PSQ-3340 / PSQ-3341): Oracle CURRENT_TIMESTAMP, CAST data types,
// interval literals, and AT TIME ZONE
// ═════════════════════════════════════════════════════════════════════════════

// ── Change 1: CURRENT_TIMESTAMP must not carry empty parens on Oracle ────────

#[test]
fn cr032_oracle_now_emits_bare_current_timestamp() {
    // `CURRENT_TIMESTAMP()` is parsed by Oracle as the precision-qualified form
    // with a missing argument → ORA-30088.
    validate_with_dialect(
        "SELECT now()",
        "SELECT CURRENT_TIMESTAMP FROM DUAL",
        Dialect::Postgres,
        Dialect::Oracle,
    );
    validate_with_dialect(
        "SELECT CURRENT_TIMESTAMP",
        "SELECT CURRENT_TIMESTAMP FROM DUAL",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_oracle_current_timestamp_roundtrip() {
    validate_with_dialect(
        "SELECT CURRENT_TIMESTAMP FROM dual",
        "SELECT CURRENT_TIMESTAMP FROM dual",
        Dialect::Oracle,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_oracle_current_date_and_time_controls() {
    // The sibling arms already emitted the paren-less form; they stay untouched.
    validate_with_dialect(
        "SELECT CURRENT_DATE, CURRENT_TIME",
        "SELECT CURRENT_DATE, CURRENT_TIME FROM DUAL",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_control_current_timestamp_other_dialects_unchanged() {
    validate_with_dialect(
        "SELECT now()",
        "SELECT GETDATE()",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT now()",
        "SELECT NOW()",
        Dialect::Postgres,
        Dialect::Postgres,
    );
    // Snowflake and BigQuery accept (and canonically use) the parenthesised form.
    validate_with_dialect(
        "SELECT now()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Postgres,
        Dialect::Snowflake,
    );
    validate_with_dialect(
        "SELECT now()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Postgres,
        Dialect::BigQuery,
    );
}

// ── Change 2: gen_data_type Oracle arms ─────────────────────────────────────

#[test]
fn cr032_oracle_bigint_maps_to_number_19() {
    // ORA-00902 "invalid datatype" for BIGINT.
    validate_with_dialect(
        "SELECT SUM(CAST(id AS bigint)) FROM transactions",
        "SELECT SUM(CAST(id AS NUMBER(19))) FROM transactions",
        Dialect::Postgres,
        Dialect::Oracle,
    );
    // The `::` spelling lowers to the same Cast node.
    validate_with_dialect(
        "SELECT id::bigint FROM transactions",
        "SELECT CAST(id AS NUMBER(19)) FROM transactions",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_oracle_tinyint_maps_to_number_3() {
    validate_with_dialect(
        "SELECT CAST(x AS tinyint) FROM t",
        "SELECT CAST(x AS NUMBER(3)) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_oracle_double_maps_to_binary_double() {
    // ORA-02000 "missing PRECISION keyword" for a bare DOUBLE.
    validate_with_dialect(
        "SELECT CAST(x AS double) FROM t",
        "SELECT CAST(x AS BINARY_DOUBLE) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_oracle_text_and_string_map_to_varchar2_4000() {
    // CLOB is rejected inside a CAST (ORA-22849), so the large-string types map
    // to the widest non-extended VARCHAR2 instead.
    validate_with_dialect(
        "SELECT CAST(x AS text) FROM t",
        "SELECT CAST(x AS VARCHAR2(4000)) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
    validate_with_dialect(
        "SELECT CAST(x AS string) FROM t",
        "SELECT CAST(x AS VARCHAR2(4000)) FROM t",
        Dialect::Hive,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_oracle_lengthless_varchar_maps_to_varchar2_4000() {
    // ORA-00906 "missing left parenthesis" for a bare VARCHAR.
    validate_with_dialect(
        "SELECT CAST(x AS varchar) FROM t",
        "SELECT CAST(x AS VARCHAR2(4000)) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_oracle_varchar_max_maps_to_varchar2() {
    validate_with_dialect(
        "SELECT CAST(x AS varchar(max)) FROM t",
        "SELECT CAST(x AS VARCHAR2(4000)) FROM t",
        Dialect::Tsql,
        Dialect::Oracle,
    );
    validate_with_dialect(
        "SELECT CAST(x AS nvarchar(max)) FROM t",
        "SELECT CAST(x AS NVARCHAR2(2000)) FROM t",
        Dialect::Tsql,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_control_oracle_accepted_types_untouched() {
    // Types Oracle already accepts are deliberately left alone.
    for (src, expected) in [
        ("smallint", "SMALLINT"),
        ("int", "INT"),
        ("real", "REAL"),
        ("float", "FLOAT"),
        ("char(3)", "CHAR(3)"),
        ("varchar(10)", "VARCHAR(10)"),
    ] {
        validate_with_dialect(
            &format!("SELECT CAST(x AS {src}) FROM t"),
            &format!("SELECT CAST(x AS {expected}) FROM t"),
            Dialect::Postgres,
            Dialect::Oracle,
        );
    }
}

#[test]
fn cr032_control_oracle_varchar2_length_preserved() {
    // CR-026 regression control.
    validate_with_dialect(
        "SELECT CAST(x AS varchar2(10)) FROM t",
        "SELECT CAST(x AS VARCHAR2(10)) FROM t",
        Dialect::Oracle,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_control_non_oracle_data_types_untouched() {
    validate_with_dialect(
        "SELECT CAST(id AS bigint) FROM t",
        "SELECT id::BIGINT FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT CAST(id AS bigint) FROM t",
        "SELECT CAST(id AS BIGINT) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT CAST(x AS text) FROM t",
        "SELECT CAST(x AS VARCHAR(MAX)) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

// ── Change 3: PG-form interval literals → Oracle functional intervals ───────

#[test]
fn cr032_oracle_interval_day_uses_numtodsinterval() {
    // ORA-30089 for `INTERVAL '30 day'`; the ANSI form `INTERVAL '30' DAY` is
    // valid but precision-limited, so the functional form is used instead.
    validate_with_dialect(
        "SELECT COUNT(*) FROM transactions WHERE created_at > now() - INTERVAL '30 day'",
        "SELECT COUNT(*) FROM transactions WHERE created_at > CURRENT_TIMESTAMP - NUMTODSINTERVAL(30, 'DAY')",
        Dialect::Postgres,
        Dialect::Oracle,
    );
    // Plural spelling normalizes identically.
    validate_with_dialect(
        "SELECT ts - INTERVAL '30 days' FROM t",
        "SELECT ts - NUMTODSINTERVAL(30, 'DAY') FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_oracle_interval_large_magnitude_avoids_ora_01873() {
    // Oracle's default DAY leading precision is 2 digits, so the ANSI form
    // `INTERVAL '36500' DAY` raises ORA-01873. NUMTODSINTERVAL has no such cap.
    validate_with_dialect(
        "SELECT COUNT(*) FROM transactions WHERE created_at > now() - INTERVAL '36500 day'",
        "SELECT COUNT(*) FROM transactions WHERE created_at > CURRENT_TIMESTAMP - NUMTODSINTERVAL(36500, 'DAY')",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_oracle_interval_time_units_use_numtodsinterval() {
    for (unit, expected) in [
        ("3 hour", "NUMTODSINTERVAL(3, 'HOUR')"),
        ("15 minute", "NUMTODSINTERVAL(15, 'MINUTE')"),
        ("45 second", "NUMTODSINTERVAL(45, 'SECOND')"),
        // A week is a day-to-second interval in Oracle; the count is scaled.
        ("2 week", "NUMTODSINTERVAL(14, 'DAY')"),
    ] {
        validate_with_dialect(
            &format!("SELECT ts - INTERVAL '{unit}' FROM t"),
            &format!("SELECT ts - {expected} FROM t"),
            Dialect::Postgres,
            Dialect::Oracle,
        );
    }
}

#[test]
fn cr032_oracle_interval_year_month_use_numtoyminterval() {
    validate_with_dialect(
        "SELECT ts - INTERVAL '2 month' FROM t",
        "SELECT ts - NUMTOYMINTERVAL(2, 'MONTH') FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
    validate_with_dialect(
        "SELECT ts - INTERVAL '1 year' FROM t",
        "SELECT ts - NUMTOYMINTERVAL(1, 'YEAR') FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_oracle_interval_ansi_numeric_form_is_lowered() {
    // `INTERVAL '7' DAY` (value + unit field) reaches the same lowering.
    validate_with_dialect(
        "SELECT ts + INTERVAL '7' DAY FROM t",
        "SELECT ts + NUMTODSINTERVAL(7, 'DAY') FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
    // QUARTER has no Oracle interval unit; it is scaled to months.
    validate_with_dialect(
        "SELECT ts + INTERVAL '1' QUARTER FROM t",
        "SELECT ts + NUMTOYMINTERVAL(3, 'MONTH') FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_oracle_unsupported_interval_unit_is_not_rewritten() {
    // Sub-second units have no exact integer NUMTODSINTERVAL form. They are
    // forwarded unchanged so Oracle rejects the statement rather than silently
    // applying a different offset.
    validate_with_dialect(
        "SELECT ts - INTERVAL '500 millisecond' FROM t",
        "SELECT ts - INTERVAL '500 millisecond' FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_control_oracle_standalone_interval_literal_untouched() {
    // Only interval operands of `+`/`-` are lowered; a bare literal round-trips.
    validate_with_dialect(
        "SELECT INTERVAL '1' DAY FROM dual",
        "SELECT INTERVAL '1' DAY FROM dual",
        Dialect::Oracle,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_control_interval_other_dialects_untouched() {
    // The T-SQL DATEADD lowering (CR-006 "Change 6") is unchanged.
    validate_with_dialect(
        "SELECT ts - INTERVAL '1' HOUR FROM t",
        "SELECT DATEADD(HOUR, -1, ts) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT ts - INTERVAL '30 day' FROM t",
        "SELECT ts - INTERVAL '30 day' FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

// ── Change 4: AT TIME ZONE is modelled instead of discarded ─────────────────

#[test]
fn cr032_at_time_zone_preserved_postgres_roundtrip() {
    // Previously the whole suffix was parsed and thrown away, silently changing
    // the result set on every target — including Postgres → Postgres.
    validate_with_dialect(
        "SELECT COUNT(*) FROM t WHERE created_at AT TIME ZONE 'UTC' < now()",
        "SELECT COUNT(*) FROM t WHERE created_at AT TIME ZONE 'UTC' < NOW()",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn cr032_at_time_zone_preserved_for_tsql_and_duckdb() {
    validate_with_dialect(
        "SELECT ts AT TIME ZONE 'UTC' FROM t",
        "SELECT ts AT TIME ZONE 'UTC' FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
    validate_with_dialect(
        "SELECT ts AT TIME ZONE 'UTC' FROM t",
        "SELECT ts AT TIME ZONE 'UTC' FROM t",
        Dialect::Postgres,
        Dialect::DuckDb,
    );
}

#[test]
fn cr032_at_time_zone_oracle_uses_from_tz() {
    // Oracle has no AT TIME ZONE operator.
    validate_with_dialect(
        "SELECT COUNT(*) FROM t WHERE created_at AT TIME ZONE 'UTC' < now()",
        "SELECT COUNT(*) FROM t WHERE FROM_TZ(CAST(created_at AS TIMESTAMP), 'UTC') < CURRENT_TIMESTAMP",
        Dialect::Postgres,
        Dialect::Oracle,
    );
    validate_with_dialect(
        "SELECT CAST(ts AT TIME ZONE 'UTC' AS DATE) FROM t",
        "SELECT CAST(FROM_TZ(CAST(ts AS TIMESTAMP), 'UTC') AS DATE) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn cr032_at_time_zone_non_literal_zone_and_chaining() {
    validate_with_dialect(
        "SELECT ts AT TIME ZONE tz_col FROM t",
        "SELECT ts AT TIME ZONE tz_col FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
    validate_with_dialect(
        "SELECT ts AT TIME ZONE 'UTC' AT TIME ZONE 'EST' FROM t",
        "SELECT ts AT TIME ZONE 'UTC' AT TIME ZONE 'EST' FROM t",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}
