# sqlglot-rust

A SQL parser, optimizer, and transpiler library written in Rust, inspired by Python's [sqlglot](https://github.com/tobymao/sqlglot).

## Features

- **Parse** SQL strings into a structured AST
- **Generate** SQL from AST nodes
- **Transpile** between 30 SQL dialects
- **LIMIT / TOP / FETCH FIRST** transpilation across dialects
- **Quoted identifier** preservation and cross-dialect conversion (`"id"` ↔ `` `id` `` ↔ `[id]`)
- **Optimize** SQL queries (constant folding, boolean simplification)
- **AST traversal** — walk, find, transform expressions
- CTEs, subqueries, set operations (UNION / INTERSECT / EXCEPT)
- Window functions with frames and filters
- CAST, TRY_CAST, EXTRACT, INTERVAL, EXISTS
- Full DDL support (CREATE TABLE, ALTER TABLE, DROP TABLE, CREATE VIEW, etc.)
- Serde serialization for AST nodes

## Supported Dialects

All 30 dialects from Python's sqlglot are supported, with function name mapping,
data type mapping, and ILIKE rewriting between dialects.

### Official Dialects

| Dialect       | Description                              |
|---------------|------------------------------------------|
| ANSI SQL      | Base SQL standard                        |
| Athena        | AWS Athena (Presto-based)                |
| BigQuery      | Google BigQuery                          |
| ClickHouse    | ClickHouse                               |
| Databricks    | Databricks (Spark-based)                 |
| DuckDB        | DuckDB                                   |
| Hive          | Apache Hive                              |
| MySQL         | MySQL                                    |
| Oracle        | Oracle Database                          |
| PostgreSQL    | PostgreSQL                               |
| Presto        | Presto                                   |
| Redshift      | Amazon Redshift (Postgres-based)         |
| Snowflake     | Snowflake                                |
| Spark         | Apache Spark SQL                         |
| SQLite        | SQLite                                   |
| StarRocks     | StarRocks (MySQL-compatible)             |
| Trino         | Trino (Presto successor)                 |
| T-SQL         | Microsoft SQL Server                     |

### Community Dialects

| Dialect       | Description                              |
|---------------|------------------------------------------|
| Doris         | Apache Doris (MySQL-compatible)          |
| Dremio        | Dremio                                   |
| Drill         | Apache Drill                             |
| Druid         | Apache Druid                             |
| Exasol        | Exasol                                   |
| Fabric        | Microsoft Fabric (T-SQL variant)         |
| Materialize   | Materialize (Postgres-compatible)        |
| PRQL          | Pipelined Relational Query Language      |
| RisingWave    | RisingWave (Postgres-compatible)         |
| SingleStore   | SingleStore (MySQL-compatible)           |
| Tableau       | Tableau                                  |
| Teradata      | Teradata                                 |

### Dialect Transform Rules

The transpiler applies dialect-specific rewrite rules when converting between dialects:

| Rule                       | Example                                         |
|----------------------------|--------------------------------------------------|
| Function name mapping      | `NOW()` → `CURRENT_TIMESTAMP()`, `GETDATE()`    |
| `SUBSTR` ↔ `SUBSTRING`    | Postgres uses `SUBSTRING`, MySQL uses `SUBSTR`   |
| `IFNULL` → `COALESCE`     | MySQL `IFNULL` → ANSI `COALESCE`                |
| `IFNULL` → `ISNULL`       | MySQL `IFNULL` → T-SQL `ISNULL`                 |
| `NVL` → `COALESCE`        | Oracle `NVL` → standard `COALESCE`              |
| `LEN` ↔ `LENGTH`          | T-SQL `LEN` ↔ standard `LENGTH`                 |
| `RAND` ↔ `RANDOM`         | MySQL `RAND` ↔ Postgres `RANDOM`                |
| `ILIKE` → `LOWER`/`LIKE`  | Rewritten for dialects without native ILIKE      |
| `LIMIT` ↔ `TOP`           | `LIMIT 10` → `TOP 10` for T-SQL                 |
| `LIMIT` → `FETCH FIRST`   | `LIMIT 10` → `FETCH FIRST 10 ROWS ONLY` (Oracle)|
| Quoted identifiers         | `"id"` ↔ `` `id` `` ↔ `[id]` per dialect       |
| Data type mapping          | `TEXT` ↔ `STRING`, `INT` → `BIGINT` (BigQuery)  |
| `BYTEA` ↔ `BLOB`          | Postgres `BYTEA` ↔ MySQL `BLOB`                 |

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
sqlglot-rust = "0.1"
```

### Parse and generate SQL

```rust
use sqlglot_rust::{parse, generate, Dialect};

fn main() {
    // Parse a SQL query
    let ast = parse("SELECT a, b FROM t WHERE a > 1", Dialect::Ansi).unwrap();

    // Generate SQL (roundtrip)
    let sql = generate(&ast, Dialect::Ansi);
    assert_eq!(sql, "SELECT a, b FROM t WHERE a > 1");
}
```

### Transpile between dialects

```rust
use sqlglot_rust::{transpile, Dialect};

fn main() {
    // Postgres → BigQuery: NOW() becomes CURRENT_TIMESTAMP()
    let sql = transpile(
        "SELECT NOW(), SUBSTRING(name, 1, 3) FROM users",
        Dialect::Postgres,
        Dialect::BigQuery,
    ).unwrap();
    // → "SELECT CURRENT_TIMESTAMP(), SUBSTRING(name, 1, 3) FROM users"

    // Oracle → T-SQL: NVL becomes ISNULL
    let sql = transpile(
        "SELECT NVL(a, b) FROM t",
        Dialect::Oracle,
        Dialect::Tsql,
    ).unwrap();
    // → "SELECT ISNULL(a, b) FROM t"
}
```

### Parse a dialect name from a string

```rust
use sqlglot_rust::Dialect;

let d = Dialect::from_str("postgres").unwrap();
assert_eq!(d, Dialect::Postgres);

// Multiple aliases are supported
assert_eq!(Dialect::from_str("tsql"), Some(Dialect::Tsql));
assert_eq!(Dialect::from_str("mssql"), Some(Dialect::Tsql));
assert_eq!(Dialect::from_str("sqlserver"), Some(Dialect::Tsql));
```

### Supported Statements

- `SELECT` (with JOINs, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET, TOP, FETCH FIRST, QUALIFY)
- `INSERT INTO ... VALUES` / `INSERT INTO ... SELECT`
- `UPDATE ... SET ... WHERE` (with RETURNING)
- `DELETE FROM ... WHERE` (with USING)
- `CREATE TABLE` (with constraints, IF NOT EXISTS, TEMPORARY, AS SELECT)
- `DROP TABLE` (with IF EXISTS)
- `ALTER TABLE` (ADD COLUMN, DROP COLUMN, RENAME COLUMN, RENAME TO, ALTER TYPE)
- `CREATE VIEW` / `DROP VIEW`
- `TRUNCATE TABLE`
- `BEGIN` / `COMMIT` / `ROLLBACK`
- `EXPLAIN`
- `USE`

### Expression Support

- Column references (qualified and unqualified) with quote-style metadata
- Numeric, string, boolean, and NULL literals
- Binary operators (`+`, `-`, `*`, `/`, `%`, `=`, `<>`, `<`, `>`, `<=`, `>=`, `AND`, `OR`, `||`)
- Unary operators (`NOT`, `-`, `+`)
- Bitwise operators (`&`, `|`, `^`, `<<`, `>>`)
- Function calls (with DISTINCT, FILTER, OVER support)
- `BETWEEN`, `IN`, `IS NULL`, `LIKE`, `ILIKE`
- `CASE ... WHEN ... THEN ... ELSE ... END`
- `CAST`, `TRY_CAST`, PostgreSQL `::` cast
- `EXTRACT(field FROM expr)` for all date/time fields
- `INTERVAL` expressions
- `EXISTS`, `IN (subquery)`
- Array literals (`ARRAY[1, 2, 3]`)
- Window functions with frames (`ROWS`, `RANGE`, `GROUPS`)
- Common Table Expressions (WITH / WITH RECURSIVE)
- Set operations (UNION, INTERSECT, EXCEPT with ALL)
- Parenthesized sub-expressions and subqueries

## Architecture

```
src/
├── ast/          # AST node definitions (~40 expression types, 15 statement types)
├── tokens/       # Token types (~200+ variants) and tokenizer
├── parser/       # Recursive-descent SQL parser
├── generator/    # SQL code generator
├── dialects/     # 30 dialect definitions with transform rules
├── optimizer/    # Query optimization (constant folding, boolean simplification)
├── errors/       # Error types
└── lib.rs        # Public API (parse, generate, transpile)
```

## Development

```bash
# Build
cargo build

# Run tests (244+ tests)
cargo test

# Run benchmarks
cargo bench

# Lint
cargo clippy

# Format
cargo fmt
```

## Acknowledgements

Inspired by [sqlglot](https://github.com/tobymao/sqlglot) by Toby Mao.

## License

MIT
