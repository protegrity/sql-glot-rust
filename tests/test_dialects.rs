/// Tests ported from Python sqlglot's dialect-specific test files.
///
/// Follows Python's testing patterns:
///   - `validate_identity(sql)` → parse→generate roundtrip (here: `assert_identity`)
///   - `validate(sql, target)` → parse→generate normalization (here: `assert_transpile`)
///   - `validate_all(sql, read={..}, write={..})` → cross-dialect (here: `assert_validate_all`)
///
/// Coverage mirrors the main cross-dialect test categories from:
///   - tests/dialects/test_dialect.py (cast, operators, random, transactions, etc.)
///   - tests/dialects/test_{bigquery,mysql,postgres,duckdb,snowflake,tsql,...}.py
use sqlglot_rust::{transpile, Dialect};

// ═════════════════════════════════════════════════════════════════════════════
// Helpers
// ═════════════════════════════════════════════════════════════════════════════

fn transpile_ok(sql: &str, read: Dialect, write: Dialect) -> String {
    transpile(sql, read, write)
        .unwrap_or_else(|e| panic!("Transpile failed for '{}': {}", sql, e))
}

fn assert_transpile(sql: &str, expected: &str, read: Dialect, write: Dialect) {
    let result = transpile_ok(sql, read, write);
    assert_eq!(result, expected, "\n  SQL:    {}\n  {:?} → {:?}", sql, read, write);
}

/// Verify that SQL roundtrips through a specific dialect pair.
fn assert_identity(sql: &str, dialect: Dialect) {
    let result = transpile_ok(sql, dialect, dialect);
    assert_eq!(result, sql, "\n  Identity failed for {:?}", dialect);
}

/// Mirrors Python's `validate_all(sql, write={dialect: expected, ...})`.
///
/// Parses `sql` with `read_dialect` and verifies that generating for each
/// `(write_dialect, expected_sql)` pair produces the expected output.
fn assert_validate_all(sql: &str, read_dialect: Dialect, writes: &[(Dialect, &str)]) {
    for (write_d, expected) in writes {
        let result = transpile_ok(sql, read_dialect, *write_d);
        assert_eq!(
            result, *expected,
            "\n  validate_all:\n    Input: {}\n    Read:  {:?}\n    Write: {:?}",
            sql, read_dialect, write_d
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Dialect identity – all 30 dialects should roundtrip basic SQL
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_all_dialect_identities() {
    let queries = [
        "SELECT 1",
        "SELECT * FROM t",
        "SELECT a, b FROM t WHERE a > 1",
        "SELECT * FROM t ORDER BY a LIMIT 10",
        "SELECT a, COUNT(*) FROM t GROUP BY a",
        "SELECT a FROM t WHERE a IS TRUE",
        "SELECT a FROM t WHERE a IS NOT FALSE",
    ];
    for dialect in Dialect::all() {
        for sql in &queries {
            assert_identity(sql, *dialect);
        }
    }
}

#[test]
fn test_dialect_count() {
    // Ensure we have all 30 Python sqlglot dialects
    assert_eq!(Dialect::all().len(), 30);
}

#[test]
fn test_dialect_from_str() {
    assert_eq!(Dialect::from_str("postgres"), Some(Dialect::Postgres));
    assert_eq!(Dialect::from_str("postgresql"), Some(Dialect::Postgres));
    assert_eq!(Dialect::from_str("BIGQUERY"), Some(Dialect::BigQuery));
    assert_eq!(Dialect::from_str("tsql"), Some(Dialect::Tsql));
    assert_eq!(Dialect::from_str("mssql"), Some(Dialect::Tsql));
    assert_eq!(Dialect::from_str("sqlserver"), Some(Dialect::Tsql));
    assert_eq!(Dialect::from_str("clickhouse"), Some(Dialect::ClickHouse));
    assert_eq!(Dialect::from_str("hive"), Some(Dialect::Hive));
    assert_eq!(Dialect::from_str("spark"), Some(Dialect::Spark));
    assert_eq!(Dialect::from_str("unknown"), None);
}

#[test]
fn test_dialect_support_levels() {
    assert_eq!(Dialect::Postgres.support_level(), "Official");
    assert_eq!(Dialect::BigQuery.support_level(), "Official");
    assert_eq!(Dialect::Tsql.support_level(), "Official");
    assert_eq!(Dialect::Doris.support_level(), "Community");
    assert_eq!(Dialect::Teradata.support_level(), "Community");
    assert_eq!(Dialect::Prql.support_level(), "Community");
}

#[test]
fn test_dialect_display() {
    assert_eq!(format!("{}", Dialect::Ansi), "ANSI SQL");
    assert_eq!(format!("{}", Dialect::Postgres), "PostgreSQL");
    assert_eq!(format!("{}", Dialect::Tsql), "T-SQL");
    assert_eq!(format!("{}", Dialect::ClickHouse), "ClickHouse");
    assert_eq!(format!("{}", Dialect::DuckDb), "DuckDB");
}

// ═════════════════════════════════════════════════════════════════════════════
// Function mapping: SUBSTR ↔ SUBSTRING
// (from Python test_mysql.py, test_postgres.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_substr_postgres_to_mysql() {
    // Postgres SUBSTRING → MySQL SUBSTR
    assert_transpile(
        "SELECT SUBSTRING(x, 1, 3) FROM t",
        "SELECT SUBSTR(x, 1, 3) FROM t",
        Dialect::Postgres,
        Dialect::Mysql,
    );
}

#[test]
fn test_substr_mysql_to_postgres() {
    // MySQL SUBSTR → Postgres SUBSTRING
    assert_transpile(
        "SELECT SUBSTR(x, 1, 3) FROM t",
        "SELECT SUBSTRING(x, 1, 3) FROM t",
        Dialect::Mysql,
        Dialect::Postgres,
    );
}

#[test]
fn test_substr_to_sqlite() {
    assert_transpile(
        "SELECT SUBSTRING(x, 1, 3) FROM t",
        "SELECT SUBSTR(x, 1, 3) FROM t",
        Dialect::Ansi,
        Dialect::Sqlite,
    );
}

#[test]
fn test_substr_from_duckdb() {
    // DuckDB uses SUBSTRING; targeting MySQL should produce SUBSTR
    assert_transpile(
        "SELECT SUBSTRING(x, 1, 3) FROM t",
        "SELECT SUBSTR(x, 1, 3) FROM t",
        Dialect::DuckDb,
        Dialect::Mysql,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Function mapping: NOW → CURRENT_TIMESTAMP
// (from Python test_bigquery.py, test_snowflake.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_now_to_bigquery() {
    assert_transpile(
        "SELECT NOW()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Postgres,
        Dialect::BigQuery,
    );
}

#[test]
fn test_now_to_snowflake() {
    assert_transpile(
        "SELECT NOW()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Postgres,
        Dialect::Snowflake,
    );
}

#[test]
fn test_now_to_ansi() {
    assert_transpile(
        "SELECT NOW()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Postgres,
        Dialect::Ansi,
    );
}

#[test]
fn test_now_to_hive() {
    assert_transpile(
        "SELECT NOW()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Postgres,
        Dialect::Hive,
    );
}

#[test]
fn test_now_to_spark() {
    assert_transpile(
        "SELECT NOW()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Postgres,
        Dialect::Spark,
    );
}

#[test]
fn test_now_to_presto() {
    assert_transpile(
        "SELECT NOW()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Postgres,
        Dialect::Presto,
    );
}

#[test]
fn test_now_to_trino() {
    assert_transpile(
        "SELECT NOW()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Postgres,
        Dialect::Trino,
    );
}

#[test]
fn test_now_to_tsql() {
    // T-SQL uses GETDATE() instead of NOW()
    assert_transpile(
        "SELECT NOW()",
        "SELECT GETDATE()",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_now_to_clickhouse() {
    assert_transpile(
        "SELECT NOW()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Postgres,
        Dialect::ClickHouse,
    );
}

#[test]
fn test_getdate_to_postgres() {
    assert_transpile(
        "SELECT GETDATE()",
        "SELECT NOW()",
        Dialect::Tsql,
        Dialect::Postgres,
    );
}

#[test]
fn test_getdate_to_bigquery() {
    assert_transpile(
        "SELECT GETDATE()",
        "SELECT CURRENT_TIMESTAMP()",
        Dialect::Tsql,
        Dialect::BigQuery,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Function mapping: LEN → LENGTH
// (from Python test_bigquery.py → test_postgres.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_len_to_postgres() {
    assert_transpile(
        "SELECT LEN(name) FROM t",
        "SELECT LENGTH(name) FROM t",
        Dialect::BigQuery,
        Dialect::Postgres,
    );
}

#[test]
fn test_len_to_mysql() {
    assert_transpile(
        "SELECT LEN(x) FROM t",
        "SELECT LENGTH(x) FROM t",
        Dialect::BigQuery,
        Dialect::Mysql,
    );
}

#[test]
fn test_len_to_sqlite() {
    assert_transpile(
        "SELECT LEN(x) FROM t",
        "SELECT LENGTH(x) FROM t",
        Dialect::BigQuery,
        Dialect::Sqlite,
    );
}

#[test]
fn test_len_to_duckdb() {
    assert_transpile(
        "SELECT LEN(x) FROM t",
        "SELECT LENGTH(x) FROM t",
        Dialect::BigQuery,
        Dialect::DuckDb,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Function mapping: IFNULL → COALESCE
// (from Python test_mysql.py → test_postgres.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_ifnull_to_postgres() {
    assert_transpile(
        "SELECT IFNULL(a, b) FROM t",
        "SELECT COALESCE(a, b) FROM t",
        Dialect::Mysql,
        Dialect::Postgres,
    );
}

#[test]
fn test_ifnull_to_ansi() {
    assert_transpile(
        "SELECT IFNULL(a, b) FROM t",
        "SELECT COALESCE(a, b) FROM t",
        Dialect::Mysql,
        Dialect::Ansi,
    );
}

#[test]
fn test_ifnull_to_duckdb() {
    assert_transpile(
        "SELECT IFNULL(a, b) FROM t",
        "SELECT COALESCE(a, b) FROM t",
        Dialect::Mysql,
        Dialect::DuckDb,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// ILIKE → LOWER(x) LIKE LOWER(pattern) for non-ILIKE dialects
// (from Python test_mysql.py, test_sqlite.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_ilike_to_mysql() {
    assert_transpile(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE LOWER(name) LIKE LOWER('%test%')",
        Dialect::Postgres,
        Dialect::Mysql,
    );
}

#[test]
fn test_ilike_to_sqlite() {
    assert_transpile(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE LOWER(name) LIKE LOWER('%test%')",
        Dialect::Postgres,
        Dialect::Sqlite,
    );
}

#[test]
fn test_ilike_preserved_in_postgres() {
    // ILIKE is native to Postgres; should remain as-is
    assert_transpile(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        Dialect::Postgres,
        Dialect::Postgres,
    );
}

#[test]
fn test_ilike_preserved_in_duckdb() {
    // DuckDB supports ILIKE natively
    assert_transpile(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        Dialect::Postgres,
        Dialect::DuckDb,
    );
}

#[test]
fn test_ilike_preserved_in_snowflake() {
    // Snowflake supports ILIKE natively
    assert_transpile(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        Dialect::Postgres,
        Dialect::Snowflake,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Data type mapping: TEXT ↔ STRING
// (from Python test_bigquery.py, test_postgres.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_text_to_string_bigquery() {
    assert_transpile(
        "SELECT CAST(x AS TEXT) FROM t",
        "SELECT CAST(x AS STRING) FROM t",
        Dialect::Postgres,
        Dialect::BigQuery,
    );
}

#[test]
fn test_string_to_text_postgres() {
    assert_transpile(
        "SELECT CAST(x AS STRING) FROM t",
        "SELECT CAST(x AS TEXT) FROM t",
        Dialect::BigQuery,
        Dialect::Postgres,
    );
}

#[test]
fn test_string_to_text_mysql() {
    assert_transpile(
        "SELECT CAST(x AS STRING) FROM t",
        "SELECT CAST(x AS TEXT) FROM t",
        Dialect::BigQuery,
        Dialect::Mysql,
    );
}

#[test]
fn test_string_to_text_sqlite() {
    assert_transpile(
        "SELECT CAST(x AS STRING) FROM t",
        "SELECT CAST(x AS TEXT) FROM t",
        Dialect::BigQuery,
        Dialect::Sqlite,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Data type mapping: INT → BIGINT (BigQuery)
// (from Python test_bigquery.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_int_to_bigint_bigquery() {
    assert_transpile(
        "SELECT CAST(x AS INT) FROM t",
        "SELECT CAST(x AS BIGINT) FROM t",
        Dialect::Postgres,
        Dialect::BigQuery,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Data type mapping: FLOAT → DOUBLE (BigQuery)
// (from Python test_bigquery.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_float_to_double_bigquery() {
    assert_transpile(
        "SELECT CAST(x AS FLOAT) FROM t",
        "SELECT CAST(x AS DOUBLE) FROM t",
        Dialect::Postgres,
        Dialect::BigQuery,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Data type mapping: BYTEA ↔ BLOB
// (from Python test_postgres.py, test_mysql.py)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytea_to_blob_mysql() {
    assert_transpile(
        "SELECT CAST(x AS BYTEA) FROM t",
        "SELECT CAST(x AS BLOB) FROM t",
        Dialect::Postgres,
        Dialect::Mysql,
    );
}

#[test]
fn test_blob_to_bytea_postgres() {
    assert_transpile(
        "SELECT CAST(x AS BLOB) FROM t",
        "SELECT CAST(x AS BYTEA) FROM t",
        Dialect::Mysql,
        Dialect::Postgres,
    );
}

#[test]
fn test_bytea_to_blob_sqlite() {
    assert_transpile(
        "SELECT CAST(x AS BYTEA) FROM t",
        "SELECT CAST(x AS BLOB) FROM t",
        Dialect::Postgres,
        Dialect::Sqlite,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Compound transformations – multiple functions + types in one query
// (from Python dialect tests – complex transpilation)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_compound_function_and_type() {
    // SUBSTR + CAST(x AS TEXT): Postgres → MySQL should map both
    assert_transpile(
        "SELECT SUBSTRING(CAST(x AS TEXT), 1, 3) FROM t",
        "SELECT SUBSTR(CAST(x AS TEXT), 1, 3) FROM t",
        Dialect::Postgres,
        Dialect::Mysql,
    );
}

#[test]
fn test_multiple_functions_in_query() {
    // Multiple function calls that need mapping
    assert_transpile(
        "SELECT LEN(name), IFNULL(email, 'none') FROM users",
        "SELECT LENGTH(name), COALESCE(email, 'none') FROM users",
        Dialect::BigQuery,
        Dialect::Postgres,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Dialect-specific DDL roundtrips
// (from Python dialect tests for CREATE TABLE)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_create_table_identity_each_dialect() {
    let sql = "CREATE TABLE t (id INT, name VARCHAR(100))";
    for dialect in Dialect::all() {
        assert_identity(sql, *dialect);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Edge cases – same-dialect should be no-op
// (from Python test pattern: read={D}, write={D} should roundtrip)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_same_dialect_noop() {
    let queries = [
        "SELECT SUBSTR(x, 1, 3) FROM t",
        "SELECT NOW()",
        "SELECT LEN(x) FROM t",
        "SELECT IFNULL(a, b) FROM t",
    ];
    for sql in &queries {
        // Parse with Ansi, generate with Ansi – function names should be preserved
        assert_identity(sql, Dialect::Ansi);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Dialect transform on INSERT/UPDATE
// (from Python dialect tests)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_transpile_insert_across_dialects() {
    let sql = "INSERT INTO t VALUES (1, 'a')";
    for write_d in Dialect::all() {
        assert_transpile(sql, sql, Dialect::Ansi, *write_d);
    }
}

#[test]
fn test_transpile_update_identity() {
    let sql = "UPDATE t SET a = 1 WHERE b = 2";
    for dialect in [Dialect::Ansi, Dialect::Postgres, Dialect::Mysql] {
        assert_identity(sql, dialect);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// NEW: IFNULL → ISNULL (T-SQL)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_ifnull_to_tsql() {
    assert_transpile(
        "SELECT IFNULL(a, b) FROM t",
        "SELECT ISNULL(a, b) FROM t",
        Dialect::Mysql,
        Dialect::Tsql,
    );
}

#[test]
fn test_isnull_to_postgres() {
    assert_transpile(
        "SELECT ISNULL(a, b) FROM t",
        "SELECT COALESCE(a, b) FROM t",
        Dialect::Tsql,
        Dialect::Postgres,
    );
}

#[test]
fn test_isnull_to_mysql() {
    assert_transpile(
        "SELECT ISNULL(a, b) FROM t",
        "SELECT IFNULL(a, b) FROM t",
        Dialect::Tsql,
        Dialect::Mysql,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// NEW: NVL mapping (Oracle)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_nvl_to_postgres() {
    assert_transpile(
        "SELECT NVL(a, b) FROM t",
        "SELECT COALESCE(a, b) FROM t",
        Dialect::Oracle,
        Dialect::Postgres,
    );
}

#[test]
fn test_nvl_to_mysql() {
    assert_transpile(
        "SELECT NVL(a, b) FROM t",
        "SELECT IFNULL(a, b) FROM t",
        Dialect::Oracle,
        Dialect::Mysql,
    );
}

#[test]
fn test_nvl_to_tsql() {
    assert_transpile(
        "SELECT NVL(a, b) FROM t",
        "SELECT ISNULL(a, b) FROM t",
        Dialect::Oracle,
        Dialect::Tsql,
    );
}

#[test]
fn test_nvl_to_snowflake() {
    // Snowflake supports NVL natively
    assert_transpile(
        "SELECT NVL(a, b) FROM t",
        "SELECT NVL(a, b) FROM t",
        Dialect::Oracle,
        Dialect::Snowflake,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// NEW: RANDOM / RAND cross-dialect mapping
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_random_postgres_to_mysql() {
    assert_transpile(
        "SELECT RANDOM()",
        "SELECT RAND()",
        Dialect::Postgres,
        Dialect::Mysql,
    );
}

#[test]
fn test_rand_mysql_to_postgres() {
    assert_transpile(
        "SELECT RAND()",
        "SELECT RANDOM()",
        Dialect::Mysql,
        Dialect::Postgres,
    );
}

#[test]
fn test_rand_to_duckdb() {
    assert_transpile(
        "SELECT RAND()",
        "SELECT RANDOM()",
        Dialect::Mysql,
        Dialect::DuckDb,
    );
}

#[test]
fn test_rand_to_sqlite() {
    assert_transpile(
        "SELECT RAND()",
        "SELECT RANDOM()",
        Dialect::Mysql,
        Dialect::Sqlite,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// NEW: SUBSTR mapping for Hive/Spark family
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_substring_to_hive() {
    assert_transpile(
        "SELECT SUBSTRING(x, 1, 3) FROM t",
        "SELECT SUBSTR(x, 1, 3) FROM t",
        Dialect::Postgres,
        Dialect::Hive,
    );
}

#[test]
fn test_substring_to_spark() {
    assert_transpile(
        "SELECT SUBSTRING(x, 1, 3) FROM t",
        "SELECT SUBSTR(x, 1, 3) FROM t",
        Dialect::Postgres,
        Dialect::Spark,
    );
}

#[test]
fn test_substring_to_databricks() {
    assert_transpile(
        "SELECT SUBSTRING(x, 1, 3) FROM t",
        "SELECT SUBSTR(x, 1, 3) FROM t",
        Dialect::Postgres,
        Dialect::Databricks,
    );
}

#[test]
fn test_substring_to_oracle() {
    assert_transpile(
        "SELECT SUBSTRING(x, 1, 3) FROM t",
        "SELECT SUBSTR(x, 1, 3) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_substr_to_presto() {
    assert_transpile(
        "SELECT SUBSTR(x, 1, 3) FROM t",
        "SELECT SUBSTRING(x, 1, 3) FROM t",
        Dialect::Mysql,
        Dialect::Presto,
    );
}

#[test]
fn test_substr_to_trino() {
    assert_transpile(
        "SELECT SUBSTR(x, 1, 3) FROM t",
        "SELECT SUBSTRING(x, 1, 3) FROM t",
        Dialect::Mysql,
        Dialect::Trino,
    );
}

#[test]
fn test_substr_to_clickhouse() {
    assert_transpile(
        "SELECT SUBSTR(x, 1, 3) FROM t",
        "SELECT SUBSTRING(x, 1, 3) FROM t",
        Dialect::Mysql,
        Dialect::ClickHouse,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// NEW: LEN ↔ LENGTH for T-SQL
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_length_to_tsql() {
    assert_transpile(
        "SELECT LENGTH(x) FROM t",
        "SELECT LEN(x) FROM t",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_len_tsql_to_postgres() {
    assert_transpile(
        "SELECT LEN(x) FROM t",
        "SELECT LENGTH(x) FROM t",
        Dialect::Tsql,
        Dialect::Postgres,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// NEW: ILIKE to other dialects
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_ilike_to_oracle() {
    assert_transpile(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE LOWER(name) LIKE LOWER('%test%')",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_ilike_to_tsql() {
    assert_transpile(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE LOWER(name) LIKE LOWER('%test%')",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_ilike_to_teradata() {
    assert_transpile(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE LOWER(name) LIKE LOWER('%test%')",
        Dialect::Postgres,
        Dialect::Teradata,
    );
}

#[test]
fn test_ilike_preserved_in_clickhouse() {
    assert_transpile(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        Dialect::Postgres,
        Dialect::ClickHouse,
    );
}

#[test]
fn test_ilike_preserved_in_redshift() {
    assert_transpile(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        Dialect::Postgres,
        Dialect::Redshift,
    );
}

#[test]
fn test_ilike_preserved_in_trino() {
    assert_transpile(
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        "SELECT * FROM t WHERE name ILIKE '%test%'",
        Dialect::Postgres,
        Dialect::Trino,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// NEW: Data type mapping for Hive/Spark family
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_text_to_string_hive() {
    assert_transpile(
        "SELECT CAST(x AS TEXT) FROM t",
        "SELECT CAST(x AS STRING) FROM t",
        Dialect::Postgres,
        Dialect::Hive,
    );
}

#[test]
fn test_text_to_string_spark() {
    assert_transpile(
        "SELECT CAST(x AS TEXT) FROM t",
        "SELECT CAST(x AS STRING) FROM t",
        Dialect::Postgres,
        Dialect::Spark,
    );
}

#[test]
fn test_string_to_text_redshift() {
    assert_transpile(
        "SELECT CAST(x AS STRING) FROM t",
        "SELECT CAST(x AS TEXT) FROM t",
        Dialect::BigQuery,
        Dialect::Redshift,
    );
}

#[test]
fn test_bytea_to_blob_oracle() {
    assert_transpile(
        "SELECT CAST(x AS BYTEA) FROM t",
        "SELECT CAST(x AS BLOB) FROM t",
        Dialect::Postgres,
        Dialect::Oracle,
    );
}

#[test]
fn test_bytea_to_blob_hive() {
    assert_transpile(
        "SELECT CAST(x AS BYTEA) FROM t",
        "SELECT CAST(x AS BLOB) FROM t",
        Dialect::Postgres,
        Dialect::Hive,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// NEW: Postgres-family dialects (Redshift, Materialize, RisingWave)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_postgres_family_blob_to_bytea() {
    // All Postgres-family dialects should map BLOB → BYTEA
    for target in [Dialect::Redshift, Dialect::Materialize, Dialect::RisingWave] {
        assert_transpile(
            "SELECT CAST(x AS BLOB) FROM t",
            "SELECT CAST(x AS BYTEA) FROM t",
            Dialect::Mysql,
            target,
        );
    }
}

#[test]
fn test_postgres_family_string_to_text() {
    for target in [Dialect::Redshift, Dialect::Materialize, Dialect::RisingWave] {
        assert_transpile(
            "SELECT CAST(x AS STRING) FROM t",
            "SELECT CAST(x AS TEXT) FROM t",
            Dialect::BigQuery,
            target,
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// NEW: MySQL-family dialects (Doris, SingleStore, StarRocks)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_mysql_family_substring_to_substr() {
    for target in [Dialect::Doris, Dialect::SingleStore, Dialect::StarRocks] {
        assert_transpile(
            "SELECT SUBSTRING(x, 1, 3) FROM t",
            "SELECT SUBSTR(x, 1, 3) FROM t",
            Dialect::Postgres,
            target,
        );
    }
}

#[test]
fn test_mysql_family_ifnull_preserved() {
    // MySQL family keeps IFNULL
    for target in [Dialect::Doris, Dialect::SingleStore, Dialect::StarRocks] {
        assert_transpile(
            "SELECT IFNULL(a, b) FROM t",
            "SELECT IFNULL(a, b) FROM t",
            Dialect::Mysql,
            target,
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// NEW: T-SQL family (Fabric)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fabric_same_as_tsql() {
    // Fabric is T-SQL variant, should behave the same
    assert_transpile(
        "SELECT NOW()",
        "SELECT GETDATE()",
        Dialect::Postgres,
        Dialect::Fabric,
    );
    assert_transpile(
        "SELECT IFNULL(a, b) FROM t",
        "SELECT ISNULL(a, b) FROM t",
        Dialect::Mysql,
        Dialect::Fabric,
    );
    assert_transpile(
        "SELECT LENGTH(x) FROM t",
        "SELECT LEN(x) FROM t",
        Dialect::Postgres,
        Dialect::Fabric,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// NEW: Compound transforms across all new dialects
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_compound_postgres_to_hive() {
    // SUBSTRING → SUBSTR, TEXT → STRING, NOW → CURRENT_TIMESTAMP
    assert_transpile(
        "SELECT SUBSTRING(CAST(x AS TEXT), 1, 3) FROM t",
        "SELECT SUBSTR(CAST(x AS STRING), 1, 3) FROM t",
        Dialect::Postgres,
        Dialect::Hive,
    );
}

#[test]
fn test_compound_postgres_to_tsql() {
    // LEN stays as LENGTH→LEN, ILIKE→LOWER/LIKE
    assert_transpile(
        "SELECT LENGTH(name) FROM t WHERE name ILIKE '%test%'",
        "SELECT LEN(name) FROM t WHERE LOWER(name) LIKE LOWER('%test%')",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_compound_oracle_to_bigquery() {
    assert_transpile(
        "SELECT NVL(SUBSTR(x, 1, 3), 'default') FROM t",
        "SELECT COALESCE(SUBSTRING(x, 1, 3), 'default') FROM t",
        Dialect::Oracle,
        Dialect::BigQuery,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// validate_all-style tests — mirrors Python's test_dialect.py patterns
// Tests a single canonical SQL generating correctly across many dialects.
// ═════════════════════════════════════════════════════════════════════════════

// ── test_random (from Python test_dialect.py::test_random) ──

#[test]
fn test_validate_all_rand() {
    // Python: RAND() → writes to many dialects
    assert_validate_all(
        "SELECT RAND()",
        Dialect::Mysql,
        &[
            (Dialect::Mysql, "SELECT RAND()"),
            (Dialect::Postgres, "SELECT RANDOM()"),
            (Dialect::DuckDb, "SELECT RANDOM()"),
            (Dialect::Sqlite, "SELECT RANDOM()"),
            (Dialect::BigQuery, "SELECT RAND()"),
            (Dialect::Snowflake, "SELECT RAND()"),
            (Dialect::Hive, "SELECT RAND()"),
            (Dialect::Spark, "SELECT RAND()"),
            (Dialect::Presto, "SELECT RAND()"),
            (Dialect::Trino, "SELECT RAND()"),
            (Dialect::Tsql, "SELECT RAND()"),
            (Dialect::ClickHouse, "SELECT RAND()"),
            (Dialect::Databricks, "SELECT RAND()"),
            (Dialect::Athena, "SELECT RAND()"),
            (Dialect::Doris, "SELECT RAND()"),
            (Dialect::StarRocks, "SELECT RAND()"),
        ],
    );
}

#[test]
fn test_validate_all_random_reads() {
    // Python: read RANDOM() from postgres, write to many
    assert_validate_all(
        "SELECT RANDOM()",
        Dialect::Postgres,
        &[
            (Dialect::Postgres, "SELECT RANDOM()"),
            (Dialect::DuckDb, "SELECT RANDOM()"),
            (Dialect::Sqlite, "SELECT RANDOM()"),
            (Dialect::Mysql, "SELECT RAND()"),
            (Dialect::BigQuery, "SELECT RAND()"),
            (Dialect::Hive, "SELECT RAND()"),
            (Dialect::Presto, "SELECT RAND()"),
            (Dialect::Tsql, "SELECT RAND()"),
        ],
    );
}

// ── test_cast type mappings (from Python test_dialect.py::test_cast) ──

#[test]
fn test_validate_all_cast_text() {
    // Python: CAST(a AS TEXT) → writes to many dialects
    assert_validate_all(
        "SELECT CAST(a AS TEXT)",
        Dialect::Postgres,
        &[
            (Dialect::Postgres, "SELECT CAST(a AS TEXT)"),
            (Dialect::Mysql, "SELECT CAST(a AS TEXT)"),
            (Dialect::Sqlite, "SELECT CAST(a AS TEXT)"),
            (Dialect::BigQuery, "SELECT CAST(a AS STRING)"),
            (Dialect::DuckDb, "SELECT CAST(a AS TEXT)"),
            (Dialect::Hive, "SELECT CAST(a AS STRING)"),
            (Dialect::Spark, "SELECT CAST(a AS STRING)"),
            (Dialect::Databricks, "SELECT CAST(a AS STRING)"),
            (Dialect::Redshift, "SELECT CAST(a AS TEXT)"),
            (Dialect::Materialize, "SELECT CAST(a AS TEXT)"),
        ],
    );
}

#[test]
fn test_validate_all_cast_string_to_text() {
    // Python: CAST(a AS STRING) → writes to many dialects
    assert_validate_all(
        "SELECT CAST(a AS STRING)",
        Dialect::BigQuery,
        &[
            (Dialect::Postgres, "SELECT CAST(a AS TEXT)"),
            (Dialect::Mysql, "SELECT CAST(a AS TEXT)"),
            (Dialect::Sqlite, "SELECT CAST(a AS TEXT)"),
            (Dialect::BigQuery, "SELECT CAST(a AS STRING)"),
            (Dialect::DuckDb, "SELECT CAST(a AS STRING)"),
            (Dialect::Redshift, "SELECT CAST(a AS TEXT)"),
            (Dialect::Materialize, "SELECT CAST(a AS TEXT)"),
            (Dialect::RisingWave, "SELECT CAST(a AS TEXT)"),
            (Dialect::Doris, "SELECT CAST(a AS TEXT)"),
            (Dialect::SingleStore, "SELECT CAST(a AS TEXT)"),
            (Dialect::StarRocks, "SELECT CAST(a AS TEXT)"),
        ],
    );
}

#[test]
fn test_validate_all_cast_bytea() {
    // Python: CAST(x AS BYTEA) → writes to many dialects
    assert_validate_all(
        "SELECT CAST(x AS BYTEA)",
        Dialect::Postgres,
        &[
            (Dialect::Postgres, "SELECT CAST(x AS BYTEA)"),
            (Dialect::Redshift, "SELECT CAST(x AS BYTEA)"),
            (Dialect::Materialize, "SELECT CAST(x AS BYTEA)"),
            (Dialect::Mysql, "SELECT CAST(x AS BLOB)"),
            (Dialect::Sqlite, "SELECT CAST(x AS BLOB)"),
            (Dialect::Oracle, "SELECT CAST(x AS BLOB)"),
            (Dialect::Hive, "SELECT CAST(x AS BLOB)"),
            (Dialect::Doris, "SELECT CAST(x AS BLOB)"),
        ],
    );
}

// ── IFNULL/NVL/ISNULL/COALESCE validate_all ──

#[test]
fn test_validate_all_ifnull_writes() {
    // Python: IFNULL(x, y) → writes to many dialects
    assert_validate_all(
        "SELECT IFNULL(x, y)",
        Dialect::Mysql,
        &[
            // MySQL family keeps IFNULL
            (Dialect::Mysql, "SELECT IFNULL(x, y)"),
            (Dialect::Doris, "SELECT IFNULL(x, y)"),
            (Dialect::SingleStore, "SELECT IFNULL(x, y)"),
            (Dialect::StarRocks, "SELECT IFNULL(x, y)"),
            (Dialect::Sqlite, "SELECT IFNULL(x, y)"),
            // ANSI/Postgres family → COALESCE
            (Dialect::Ansi, "SELECT COALESCE(x, y)"),
            (Dialect::Postgres, "SELECT COALESCE(x, y)"),
            (Dialect::Redshift, "SELECT COALESCE(x, y)"),
            (Dialect::DuckDb, "SELECT COALESCE(x, y)"),
            (Dialect::BigQuery, "SELECT COALESCE(x, y)"),
            (Dialect::Snowflake, "SELECT COALESCE(x, y)"),
            (Dialect::Hive, "SELECT COALESCE(x, y)"),
            (Dialect::Spark, "SELECT COALESCE(x, y)"),
            (Dialect::Presto, "SELECT COALESCE(x, y)"),
            (Dialect::Trino, "SELECT COALESCE(x, y)"),
            (Dialect::ClickHouse, "SELECT COALESCE(x, y)"),
            (Dialect::Oracle, "SELECT COALESCE(x, y)"),
            // T-SQL family → ISNULL
            (Dialect::Tsql, "SELECT ISNULL(x, y)"),
            (Dialect::Fabric, "SELECT ISNULL(x, y)"),
        ],
    );
}

#[test]
fn test_validate_all_nvl_writes() {
    // Python: NVL(x, y) → writes to many dialects
    assert_validate_all(
        "SELECT NVL(x, y)",
        Dialect::Oracle,
        &[
            (Dialect::Oracle, "SELECT NVL(x, y)"),
            (Dialect::Snowflake, "SELECT NVL(x, y)"),
            (Dialect::Postgres, "SELECT COALESCE(x, y)"),
            (Dialect::BigQuery, "SELECT COALESCE(x, y)"),
            (Dialect::DuckDb, "SELECT COALESCE(x, y)"),
            (Dialect::Presto, "SELECT COALESCE(x, y)"),
            (Dialect::Hive, "SELECT COALESCE(x, y)"),
            (Dialect::Mysql, "SELECT IFNULL(x, y)"),
            (Dialect::Sqlite, "SELECT IFNULL(x, y)"),
            (Dialect::Tsql, "SELECT ISNULL(x, y)"),
            (Dialect::Fabric, "SELECT ISNULL(x, y)"),
        ],
    );
}

#[test]
fn test_validate_all_isnull_writes() {
    // Python: ISNULL(x, y) from T-SQL → writes to many dialects
    assert_validate_all(
        "SELECT ISNULL(x, y)",
        Dialect::Tsql,
        &[
            (Dialect::Tsql, "SELECT ISNULL(x, y)"),
            (Dialect::Fabric, "SELECT ISNULL(x, y)"),
            (Dialect::Postgres, "SELECT COALESCE(x, y)"),
            (Dialect::BigQuery, "SELECT COALESCE(x, y)"),
            (Dialect::DuckDb, "SELECT COALESCE(x, y)"),
            (Dialect::Snowflake, "SELECT COALESCE(x, y)"),
            (Dialect::Mysql, "SELECT IFNULL(x, y)"),
            (Dialect::Sqlite, "SELECT IFNULL(x, y)"),
        ],
    );
}

// ── NOW/CURRENT_TIMESTAMP/GETDATE validate_all ──

#[test]
fn test_validate_all_now_writes() {
    // Python: NOW() from Postgres → writes to many dialects
    assert_validate_all(
        "SELECT NOW()",
        Dialect::Postgres,
        &[
            // Keeps NOW
            (Dialect::Postgres, "SELECT NOW()"),
            (Dialect::Mysql, "SELECT NOW()"),
            (Dialect::DuckDb, "SELECT NOW()"),
            (Dialect::Sqlite, "SELECT NOW()"),
            (Dialect::Redshift, "SELECT NOW()"),
            // → CURRENT_TIMESTAMP
            (Dialect::Ansi, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::BigQuery, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Snowflake, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Hive, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Spark, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Databricks, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Presto, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Trino, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Athena, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::ClickHouse, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Oracle, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Exasol, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Teradata, "SELECT CURRENT_TIMESTAMP()"),
            // → GETDATE
            (Dialect::Tsql, "SELECT GETDATE()"),
            (Dialect::Fabric, "SELECT GETDATE()"),
        ],
    );
}

#[test]
fn test_validate_all_getdate_writes() {
    // Python: GETDATE() from T-SQL → writes to many dialects
    assert_validate_all(
        "SELECT GETDATE()",
        Dialect::Tsql,
        &[
            (Dialect::Tsql, "SELECT GETDATE()"),
            (Dialect::Fabric, "SELECT GETDATE()"),
            (Dialect::Postgres, "SELECT NOW()"),
            (Dialect::Mysql, "SELECT NOW()"),
            (Dialect::DuckDb, "SELECT NOW()"),
            (Dialect::Sqlite, "SELECT NOW()"),
            (Dialect::BigQuery, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Snowflake, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Hive, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Presto, "SELECT CURRENT_TIMESTAMP()"),
            (Dialect::Oracle, "SELECT CURRENT_TIMESTAMP()"),
        ],
    );
}

// ── SUBSTR/SUBSTRING validate_all ──

#[test]
fn test_validate_all_substring_writes() {
    // Python: SUBSTRING(x, 1, 3) → writes to many dialects
    assert_validate_all(
        "SELECT SUBSTRING(x, 1, 3)",
        Dialect::Postgres,
        &[
            // SUBSTRING dialects
            (Dialect::Postgres, "SELECT SUBSTRING(x, 1, 3)"),
            (Dialect::Redshift, "SELECT SUBSTRING(x, 1, 3)"),
            (Dialect::DuckDb, "SELECT SUBSTRING(x, 1, 3)"),
            (Dialect::BigQuery, "SELECT SUBSTRING(x, 1, 3)"),
            (Dialect::Snowflake, "SELECT SUBSTRING(x, 1, 3)"),
            (Dialect::Presto, "SELECT SUBSTRING(x, 1, 3)"),
            (Dialect::Trino, "SELECT SUBSTRING(x, 1, 3)"),
            (Dialect::Athena, "SELECT SUBSTRING(x, 1, 3)"),
            (Dialect::ClickHouse, "SELECT SUBSTRING(x, 1, 3)"),
            (Dialect::Ansi, "SELECT SUBSTRING(x, 1, 3)"),
            (Dialect::Materialize, "SELECT SUBSTRING(x, 1, 3)"),
            // SUBSTR dialects
            (Dialect::Mysql, "SELECT SUBSTR(x, 1, 3)"),
            (Dialect::Sqlite, "SELECT SUBSTR(x, 1, 3)"),
            (Dialect::Oracle, "SELECT SUBSTR(x, 1, 3)"),
            (Dialect::Hive, "SELECT SUBSTR(x, 1, 3)"),
            (Dialect::Spark, "SELECT SUBSTR(x, 1, 3)"),
            (Dialect::Databricks, "SELECT SUBSTR(x, 1, 3)"),
            (Dialect::Doris, "SELECT SUBSTR(x, 1, 3)"),
            (Dialect::SingleStore, "SELECT SUBSTR(x, 1, 3)"),
            (Dialect::StarRocks, "SELECT SUBSTR(x, 1, 3)"),
        ],
    );
}

// ── LEN/LENGTH validate_all ──

#[test]
fn test_validate_all_len_writes() {
    // Python: LEN(x) → writes to many dialects
    assert_validate_all(
        "SELECT LEN(x)",
        Dialect::BigQuery,
        &[
            (Dialect::BigQuery, "SELECT LEN(x)"),
            (Dialect::Snowflake, "SELECT LEN(x)"),
            (Dialect::Tsql, "SELECT LEN(x)"),
            (Dialect::Fabric, "SELECT LEN(x)"),
            (Dialect::Postgres, "SELECT LENGTH(x)"),
            (Dialect::Mysql, "SELECT LENGTH(x)"),
            (Dialect::Sqlite, "SELECT LENGTH(x)"),
            (Dialect::DuckDb, "SELECT LENGTH(x)"),
            (Dialect::Oracle, "SELECT LENGTH(x)"),
            (Dialect::Hive, "SELECT LENGTH(x)"),
            (Dialect::Presto, "SELECT LENGTH(x)"),
            (Dialect::ClickHouse, "SELECT LENGTH(x)"),
        ],
    );
}

// ── ILIKE validate_all ──

#[test]
fn test_validate_all_ilike_writes() {
    // Python: x ILIKE '%y' → writes to many dialects
    assert_validate_all(
        "SELECT * FROM t WHERE x ILIKE '%y'",
        Dialect::Postgres,
        &[
            // Native ILIKE support
            (Dialect::Postgres, "SELECT * FROM t WHERE x ILIKE '%y'"),
            (Dialect::DuckDb, "SELECT * FROM t WHERE x ILIKE '%y'"),
            (Dialect::Snowflake, "SELECT * FROM t WHERE x ILIKE '%y'"),
            (Dialect::ClickHouse, "SELECT * FROM t WHERE x ILIKE '%y'"),
            (Dialect::Redshift, "SELECT * FROM t WHERE x ILIKE '%y'"),
            (Dialect::Trino, "SELECT * FROM t WHERE x ILIKE '%y'"),
            (Dialect::Presto, "SELECT * FROM t WHERE x ILIKE '%y'"),
            (Dialect::Spark, "SELECT * FROM t WHERE x ILIKE '%y'"),
            (Dialect::Hive, "SELECT * FROM t WHERE x ILIKE '%y'"),
            // Lowered to LIKE
            (
                Dialect::Mysql,
                "SELECT * FROM t WHERE LOWER(x) LIKE LOWER('%y')",
            ),
            (
                Dialect::Sqlite,
                "SELECT * FROM t WHERE LOWER(x) LIKE LOWER('%y')",
            ),
            (
                Dialect::Oracle,
                "SELECT * FROM t WHERE LOWER(x) LIKE LOWER('%y')",
            ),
            (
                Dialect::Tsql,
                "SELECT * FROM t WHERE LOWER(x) LIKE LOWER('%y')",
            ),
            (
                Dialect::Ansi,
                "SELECT * FROM t WHERE LOWER(x) LIKE LOWER('%y')",
            ),
            (
                Dialect::BigQuery,
                "SELECT * FROM t WHERE LOWER(x) LIKE LOWER('%y')",
            ),
            (
                Dialect::Teradata,
                "SELECT * FROM t WHERE LOWER(x) LIKE LOWER('%y')",
            ),
        ],
    );
}

// ── NOT ILIKE validate_all ──

#[test]
fn test_validate_all_not_ilike() {
    assert_validate_all(
        "SELECT * FROM t WHERE x NOT ILIKE '%y'",
        Dialect::Postgres,
        &[
            (Dialect::Postgres, "SELECT * FROM t WHERE x NOT ILIKE '%y'"),
            (Dialect::DuckDb, "SELECT * FROM t WHERE x NOT ILIKE '%y'"),
            (Dialect::Snowflake, "SELECT * FROM t WHERE x NOT ILIKE '%y'"),
            (
                Dialect::Mysql,
                "SELECT * FROM t WHERE LOWER(x) NOT LIKE LOWER('%y')",
            ),
            (
                Dialect::Oracle,
                "SELECT * FROM t WHERE LOWER(x) NOT LIKE LOWER('%y')",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Per-dialect identity tests (from Python test_<dialect>.py::test_<dialect>)
// ═════════════════════════════════════════════════════════════════════════════

// ── BigQuery (from test_bigquery.py) ──

#[test]
fn test_bigquery_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t WHERE a = 1",
        "SELECT CAST(x AS STRING)",
        "SELECT a, b, c FROM t GROUP BY 1, 2, 3",
        "SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::BigQuery);
    }
}

// ── ClickHouse (from test_clickhouse.py) ──

#[test]
fn test_clickhouse_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t LIMIT 10",
        "SELECT a, COUNT(*) FROM t GROUP BY a",
        "SELECT CAST(x AS INT)",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::ClickHouse);
    }
}

// ── DuckDB (from test_duckdb.py) ──

#[test]
fn test_duckdb_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t WHERE a ILIKE '%x%'",
        "SELECT CAST('2024-01-01' AS DATE)",
        "SELECT a, b FROM t ORDER BY a LIMIT 5",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::DuckDb);
    }
}

// ── Hive (from test_hive.py) ──

#[test]
fn test_hive_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t",
        "SELECT a, b FROM t WHERE a > 1 ORDER BY a LIMIT 10",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Hive);
    }
}

// ── MySQL (from test_mysql.py) ──

#[test]
fn test_mysql_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t WHERE a LIKE '%test%'",
        "SELECT a FROM t GROUP BY a HAVING COUNT(*) > 1",
        "INSERT INTO t VALUES (1, 'a')",
        "UPDATE t SET a = 1 WHERE b = 2",
        "DELETE FROM t WHERE a = 1",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Mysql);
    }
}

// ── Oracle (from test_oracle.py) ──

#[test]
fn test_oracle_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t WHERE ROWNUM <= 10",
        "SELECT a, b FROM t ORDER BY a",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Oracle);
    }
}

// ── PostgreSQL (from test_postgres.py) ──

#[test]
fn test_postgres_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t WHERE a ILIKE '%test%'",
        "SELECT CAST(x AS TEXT)",
        "SELECT * FROM t1 LEFT JOIN t2 ON t1.id = t2.id",
        "CREATE TABLE t (id SERIAL PRIMARY KEY, name VARCHAR(100))",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Postgres);
    }
}

// ── Presto (from test_presto.py) ──

#[test]
fn test_presto_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t LIMIT 10",
        "SELECT CAST(x AS VARCHAR)",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Presto);
    }
}

// ── Redshift (from test_redshift.py) ──

#[test]
fn test_redshift_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t WHERE a ILIKE '%test%'",
        "SELECT a, b FROM t ORDER BY a LIMIT 100",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Redshift);
    }
}

// ── Snowflake (from test_snowflake.py) ──

#[test]
fn test_snowflake_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t WHERE a ILIKE '%test%'",
        "SELECT CAST(x AS VARCHAR)",
        "SELECT a, b FROM t ORDER BY a LIMIT 10",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Snowflake);
    }
}

// ── Spark (from test_spark.py) ──

#[test]
fn test_spark_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t",
        "SELECT a, COUNT(*) FROM t GROUP BY a",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Spark);
    }
}

// ── SQLite (from test_sqlite.py) ──

#[test]
fn test_sqlite_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t WHERE a LIKE '%test%'",
        "SELECT typeof(x)",
        "INSERT INTO t VALUES (1, 'a')",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Sqlite);
    }
}

// ── T-SQL (from test_tsql.py) ──

#[test]
fn test_tsql_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT a, b FROM t WHERE a = 1",
        "SELECT CAST(x AS INT) FROM t",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Tsql);
    }
}

// ── Trino (from test_trino via Presto) ──

#[test]
fn test_trino_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t LIMIT 10",
        "SELECT CAST(x AS VARCHAR)",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Trino);
    }
}

// ── Athena ──

#[test]
fn test_athena_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t WHERE a > 1",
        "SELECT a, b FROM t LIMIT 100",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Athena);
    }
}

// ── Databricks ──

#[test]
fn test_databricks_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t",
        "SELECT a, b FROM t ORDER BY a LIMIT 10",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Databricks);
    }
}

// ── StarRocks ──

#[test]
fn test_starrocks_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t WHERE a > 1",
        "SELECT * FROM t ORDER BY a LIMIT 10",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::StarRocks);
    }
}

// ── Teradata ──

#[test]
fn test_teradata_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t",
        "SELECT a, b FROM t WHERE a = 1",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Teradata);
    }
}

// ── Exasol ──

#[test]
fn test_exasol_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t",
        "SELECT a, COUNT(*) FROM t GROUP BY a",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Exasol);
    }
}

// ── Materialize ──

#[test]
fn test_materialize_identity() {
    let sqls = [
        "SELECT 1",
        "SELECT * FROM t WHERE a ILIKE '%x%'",
        "SELECT CAST(a AS INT) FROM t",
    ];
    for sql in &sqls {
        assert_identity(sql, Dialect::Materialize);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Cross-dialect DDL tests (from Python test_dialect.py::test_ddl)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_create_table_type_mapping_postgres_to_bigquery() {
    assert_transpile(
        "CREATE TABLE t (id INT, name TEXT, data BYTEA)",
        "CREATE TABLE t (id BIGINT, name STRING, data BYTEA)",
        Dialect::Postgres,
        Dialect::BigQuery,
    );
}

#[test]
fn test_create_table_type_mapping_postgres_to_hive() {
    assert_transpile(
        "CREATE TABLE t (id INT, name TEXT, data BYTEA)",
        "CREATE TABLE t (id INT, name STRING, data BLOB)",
        Dialect::Postgres,
        Dialect::Hive,
    );
}

#[test]
fn test_create_table_type_mapping_bigquery_to_postgres() {
    assert_transpile(
        "CREATE TABLE t (id INT, name STRING)",
        "CREATE TABLE t (id INT, name TEXT)",
        Dialect::BigQuery,
        Dialect::Postgres,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Cross-dialect INSERT/UPDATE with function transforms
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_insert_with_function_transform() {
    assert_transpile(
        "INSERT INTO t VALUES (NOW(), 'data')",
        "INSERT INTO t VALUES (CURRENT_TIMESTAMP(), 'data')",
        Dialect::Postgres,
        Dialect::BigQuery,
    );
}

#[test]
fn test_update_with_function_transform() {
    assert_transpile(
        "UPDATE t SET a = NOW() WHERE b = 1",
        "UPDATE t SET a = GETDATE() WHERE b = 1",
        Dialect::Postgres,
        Dialect::Tsql,
    );
}

#[test]
fn test_update_with_null_coalesce_transform() {
    assert_transpile(
        "UPDATE t SET a = NVL(b, 0) WHERE c = 1",
        "UPDATE t SET a = COALESCE(b, 0) WHERE c = 1",
        Dialect::Oracle,
        Dialect::Postgres,
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Compound multi-function validate_all
// (matches Python's complex validate_all with many functions in one query)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_validate_all_compound_query() {
    // Complex query with SUBSTR + IFNULL + type cast
    assert_validate_all(
        "SELECT IFNULL(SUBSTR(CAST(x AS TEXT), 1, 3), 'none') FROM t",
        Dialect::Mysql,
        &[
            (
                Dialect::Mysql,
                "SELECT IFNULL(SUBSTR(CAST(x AS TEXT), 1, 3), 'none') FROM t",
            ),
            (
                Dialect::Postgres,
                "SELECT COALESCE(SUBSTRING(CAST(x AS TEXT), 1, 3), 'none') FROM t",
            ),
            (
                Dialect::BigQuery,
                "SELECT COALESCE(SUBSTRING(CAST(x AS STRING), 1, 3), 'none') FROM t",
            ),
            (
                Dialect::Hive,
                "SELECT COALESCE(SUBSTR(CAST(x AS STRING), 1, 3), 'none') FROM t",
            ),
            (
                Dialect::Tsql,
                "SELECT ISNULL(SUBSTRING(CAST(x AS TEXT), 1, 3), 'none') FROM t",
            ),
            (
                Dialect::Oracle,
                "SELECT COALESCE(SUBSTR(CAST(x AS TEXT), 1, 3), 'none') FROM t",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Transaction identity per dialect (from Python test_dialect.py::test_transactions)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_transaction_identity_all_dialects() {
    for dialect in Dialect::all() {
        assert_identity("BEGIN", *dialect);
        assert_identity("COMMIT", *dialect);
        assert_identity("ROLLBACK", *dialect);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// DDL identity per dialect
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_drop_table_identity_all_dialects() {
    for dialect in Dialect::all() {
        assert_identity("DROP TABLE t", *dialect);
        assert_identity("DROP TABLE IF EXISTS t", *dialect);
    }
}

#[test]
fn test_truncate_identity_all_dialects() {
    for dialect in Dialect::all() {
        assert_identity("TRUNCATE TABLE t", *dialect);
    }
}

#[test]
fn test_alter_table_identity_all_dialects() {
    for dialect in Dialect::all() {
        assert_identity("ALTER TABLE t ADD COLUMN c INT", *dialect);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Complex SELECT identity per dialect
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_complex_select_identity_all_dialects() {
    let sqls = [
        "SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id WHERE t1.a > 1 ORDER BY t1.a LIMIT 10",
        "SELECT a, SUM(b) FROM t GROUP BY a HAVING SUM(b) > 100",
        "WITH cte AS (SELECT * FROM t) SELECT * FROM cte",
        "SELECT * FROM t1 UNION ALL SELECT * FROM t2",
        "SELECT CASE WHEN a > 1 THEN 'yes' ELSE 'no' END FROM t",
        "SELECT a FROM t WHERE a BETWEEN 1 AND 10",
        "SELECT a FROM t WHERE a IN (1, 2, 3)",
        "SELECT a FROM t WHERE a IS NULL",
        "SELECT CAST(a AS INT) FROM t",
        "SELECT EXTRACT(YEAR FROM d) FROM t",
    ];
    for dialect in Dialect::all() {
        for sql in &sqls {
            assert_identity(sql, *dialect);
        }
    }
}
