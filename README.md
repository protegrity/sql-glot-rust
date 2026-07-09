# sqlglot-rust

A SQL parser, optimizer, and transpiler library written in Rust, inspired by Python's [sqlglot](https://github.com/tobymao/sqlglot).

## Features

- **Parse** SQL strings into a structured AST
- **Generate** SQL from AST nodes
- **Transpile** between 30 SQL dialects
- **C/C++ FFI** — shared & static libraries with a C header for integration from C, C++, or any language with C ABI support
- **CLI** — command-line interface for transpiling, parsing, and formatting SQL
- **Expression Builder API** — fluent builders for programmatic SQL construction
- **Typed function expressions** — 72+ functions across 8 categories with dialect-specific generation
- **LIMIT / TOP / FETCH FIRST** transpilation across dialects
- **Quoted identifier** preservation and cross-dialect conversion (`"id"` ↔ `` `id` `` ↔ `[id]`)
- **Optimize** SQL queries (constant folding, boolean simplification, subquery unnesting, predicate pushdown, column qualification)
- **Scope analysis** — track query scopes, sources, columns, and correlations across nested queries
- **Schema management** — dialect-aware table/column registration and type lookup
- **AST traversal** — walk, find, transform expressions
- **AST Diff** — semantic comparison of SQL statements with structural change detection
- **SQL Execution Engine** — in-memory query execution against Rust data structures for testing and validation
- CTEs, subqueries, set operations (UNION / INTERSECT / EXCEPT)
- Window functions with frames and filters
- CAST, TRY_CAST, EXTRACT, INTERVAL, EXISTS, ANY/ALL/SOME
- Full DDL support (CREATE TABLE, ALTER TABLE, DROP TABLE, CREATE VIEW, etc.)
- Serde serialization for AST nodes

## Supported Dialects

All 30 dialects from Python's sqlglot are supported, with function name mapping,
data type mapping, and ILIKE rewriting between dialects.

### Official Dialects

| Dialect | Description |
| --- | --- |
| ANSI SQL | Base SQL standard |
| Athena | AWS Athena (Presto-based) |
| BigQuery | Google BigQuery |
| ClickHouse | ClickHouse |
| Databricks | Databricks (Spark-based) |
| DuckDB | DuckDB |
| Hive | Apache Hive |
| MySQL | MySQL |
| Oracle | Oracle Database |
| PostgreSQL | PostgreSQL |
| Presto | Presto |
| Redshift | Amazon Redshift (Postgres-based) |
| Snowflake | Snowflake |
| Spark | Apache Spark SQL |
| SQLite | SQLite |
| StarRocks | StarRocks (MySQL-compatible) |
| Trino | Trino (Presto successor) |
| T-SQL | Microsoft SQL Server |

### Community Dialects

| Dialect | Description |
| --- | --- |
| Doris | Apache Doris (MySQL-compatible) |
| Dremio | Dremio |
| Drill | Apache Drill |
| Druid | Apache Druid |
| Exasol | Exasol |
| Fabric | Microsoft Fabric (T-SQL variant) |
| Materialize | Materialize (Postgres-compatible) |
| PRQL | Pipelined Relational Query Language |
| RisingWave | RisingWave (Postgres-compatible) |
| SingleStore | SingleStore (MySQL-compatible) |
| Tableau | Tableau |
| Teradata | Teradata |

### Dialect Transform Rules

The transpiler applies dialect-specific rewrite rules when converting between dialects:

| Rule | Example |
| --- | --- |
| Function name mapping | `NOW()` → `CURRENT_TIMESTAMP()`, `GETDATE()` |
| `SUBSTR` ↔ `SUBSTRING` | Postgres uses `SUBSTRING`, MySQL uses `SUBSTR` |
| `CEIL` → `CEILING` | T-SQL uses `CEILING` |
| `POW` → `POWER` | T-SQL/Oracle use `POWER` |
| `DATE_TRUNC` → `DATETRUNC`/`TRUNC` | T-SQL uses `DATETRUNC`, Oracle uses `TRUNC` |
| `ARRAY_AGG` → `LIST`/`COLLECT_LIST` | DuckDB uses `LIST`, Hive/Spark use `COLLECT_LIST` |
| `HEX`/`UNHEX` → `TO_HEX`/`FROM_HEX` | Presto/Trino naming convention |
| `IFNULL` → `COALESCE` | MySQL `IFNULL` → ANSI `COALESCE` |
| `IFNULL` → `ISNULL` | MySQL `IFNULL` → T-SQL `ISNULL` |
| `NVL` → `COALESCE` | Oracle `NVL` → standard `COALESCE` |
| `LEN` ↔ `LENGTH` | T-SQL `LEN` ↔ standard `LENGTH` |
| `RAND` ↔ `RANDOM` | MySQL `RAND` ↔ Postgres `RANDOM` |
| `ILIKE` → `LOWER`/`LIKE` | Rewritten for dialects without native ILIKE |
| `LIMIT` ↔ `TOP` | `LIMIT 10` → `TOP 10` for T-SQL |
| `LIMIT` → `FETCH FIRST` | `LIMIT 10` → `FETCH FIRST 10 ROWS ONLY` (Oracle) |
| Quoted identifiers | `"id"` ↔ `` `id` `` ↔ `[id]` per dialect |
| Data type mapping | `TEXT` ↔ `STRING`, `INT` → `BIGINT` (BigQuery) |
| `BYTEA` ↔ `BLOB` | Postgres `BYTEA` ↔ MySQL `BLOB` |

## Recent Parser Improvements and Benchmark Snapshot

This repository has recently added tolerance and compatibility fixes across
multiple dialects, including:

- ClickHouse: richer dotted/typed access parsing (`expr.:Type`, `expr.^field`, `.null` field paths), plus broader support for alias-heavy query forms.
- DuckDB: alias-first projection support (`alias: expr`) and DESCRIBE-in-subquery tolerance.
- PostgreSQL: `BIT VARYING(n)` cast tolerance, JSON key/value argument parsing (`'k' : v`), richer `ON CONFLICT (...)` target parsing, and broader geometric/operator-sequence tolerance.
- MySQL: improved UPDATE/DELETE tolerance around ORDER BY, LIMIT, PARTITION, and insert alias edge cases.
- T-SQL / ANSI extensions: tolerance for `WITH XMLNAMESPACES (...)` and additional parser guardrails for mixed corpora.

Benchmark reference:

- [SQL AST Benchmark](https://sql-ast-benchmark.luca.phd/)

### Acceptance Report (latest run)

| dialect | total | accepted | rejected | accept% | panics |
| --- | ---: | ---: | ---: | ---: | ---: |
| postgresql | 29402 | 28089 | 1313 | 95.53% | 0 |
| sqlite | 12119 | 12103 | 16 | 99.87% | 0 |
| mysql | 30220 | 29925 | 295 | 99.02% | 0 |
| clickhouse | 92268 | 91148 | 1120 | 98.79% | 0 |
| duckdb | 41148 | 40029 | 1119 | 97.28% | 0 |
| hive | 41294 | 40774 | 520 | 98.74% | 0 |
| spark_sql | 14464 | 14180 | 284 | 98.04% | 0 |
| trino | 71 | 70 | 1 | 98.59% | 0 |
| tsql | 14782 | 13521 | 1261 | 91.47% | 0 |
| oracle | 21648 | 21608 | 40 | 99.82% | 0 |
| bigquery | 224 | 222 | 2 | 99.11% | 0 |
| redshift | 2992 | 2964 | 28 | 99.06% | 0 |
| multi | 10962 | 9637 | 1325 | 87.91% | 0 |
| **TOTAL** | **311594** | **304270** | **7324** | **97.65%** | **0** |

### Brief Summary of Not-Yet-Supported Query Kinds

The remaining rejects are concentrated in a few recurring categories:

- Non-SQL or shell-like corpus lines mixed into SQL datasets (for example raw text, script fragments, malformed separators).
- Dialect-specific meta commands or client commands (for example backslash-style command lines in PostgreSQL corpora).
- Template or parameterized placeholders that are not concrete SQL syntax until preprocessed.
- Highly dialect-specific operator families and parser corner-cases in advanced analytical or geometric expressions.
- Intentionally malformed or truncated statements in test corpora (unterminated strings/comments, unexpected EOF).

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
sqlglot-rust = "0.10.11"
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

### Build SQL with the Expression Builder API

```rust
use sqlglot_rust::builder::{select, condition, column, literal};
use sqlglot_rust::{generate, Dialect};

fn main() {
    // Fluent SELECT builder
    let query = select(&["u.id", "u.name", "COUNT(o.id) AS order_count"])
        .from("users")
        .left_join("orders", "u.id = o.user_id")
        .where_clause("u.active = true")
        .and_where("o.created_at > '2024-01-01'")
        .group_by(&["u.id", "u.name"])
        .having("COUNT(o.id) > 0")
        .order_by(&["order_count DESC"])
        .limit(10)
        .build();

    let sql = generate(&query, Dialect::Postgres);
    // => "SELECT u.id, u.name, COUNT(o.id) AS order_count FROM users LEFT JOIN orders ON u.id = o.user_id WHERE u.active = true AND o.created_at > '2024-01-01' GROUP BY u.id, u.name HAVING COUNT(o.id) > 0 ORDER BY order_count DESC LIMIT 10"

    // Condition builder
    let cond = condition("x = 1")
        .and("y = 2")
        .or("z = 3")
        .build();

    // Expression factory functions
    let col = column("name", Some("users"));  // users.name
    let num = literal(42);                     // 42
}
```

### Supported Statements

- `SELECT` (with JOINs, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET, TOP, FETCH FIRST, QUALIFY)
- `INSERT INTO ... VALUES` / `INSERT INTO ... SELECT`
- `UPDATE ... SET ... WHERE` (with RETURNING)
- `DELETE FROM ... WHERE` (with USING)
- `MERGE INTO ... USING ... WHEN MATCHED / WHEN NOT MATCHED` (with dialect extensions)
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
- Typed function expressions (72+ functions: date/time, string, aggregate, array, JSON, window, math, conversion)
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

## CLI

The CLI is built with the `cli` feature flag and provides three commands:

### Install

```bash
cargo install sqlglot-rust --features cli
```

### Transpile

Transpile SQL between dialects:

```bash
# From stdin
echo "SELECT CAST(x AS INT) FROM t" | sqlglot transpile --read mysql --write postgres

# With pretty-printing
echo "SELECT a, b FROM t WHERE x > 1" | sqlglot transpile --pretty

# With optimizer
echo "SELECT * FROM t WHERE 1 = 1 AND a > 5" | sqlglot transpile --optimize

# From file to file
sqlglot transpile --read mysql --write postgres --input query.sql --output result.sql
```

### Parse

Parse SQL and output the AST as JSON:

```bash
echo "SELECT a FROM t" | sqlglot parse --pretty
```

### Format

Pretty-print SQL:

```bash
echo "select a,b from t where x>1" | sqlglot format
```

### Options

| Option | Description |
| --- | --- |
| `--read <dialect>` | Source dialect (default: ansi) |
| `--write <dialect>` | Target dialect (default: ansi) |
| `--pretty` | Pretty-print output |
| `--input <file>` | Read from file instead of stdin |
| `--output <file>` | Write to file instead of stdout |
| `--optimize` | Run optimizer before generation (transpile only) |

## C/C++ FFI

sqlglot-rust can be built as a shared library (`.so` / `.dylib` / `.dll`) or a static library (`.a`) for use from C, C++, or any language that supports the C ABI.

### Build the FFI libraries

```bash
# Build for the current host
make ffi

# Build for a specific target
make ffi-macos-arm64    # aarch64-apple-darwin
make ffi-macos-amd64    # x86_64-apple-darwin
make ffi-linux-amd64    # x86_64-unknown-linux-gnu
make ffi-linux-arm64    # aarch64-unknown-linux-gnu

# Build all four targets
make ffi-all
```

Output goes to `target/ffi/` with the C header at `target/ffi/include/sqlglot.h`.

### C API

```c
#include "sqlglot.h"

const char *sqlglot_version(void);
char *sqlglot_parse(const char *sql, const char *dialect);
char *sqlglot_transpile(const char *sql, const char *from_dialect, const char *to_dialect);
char *sqlglot_generate(const char *ast_json, const char *dialect);
void  sqlglot_free(char *ptr);   /* must be called on every non-NULL return */
```

### C usage example

```c
#include <stdio.h>
#include "sqlglot.h"

int main(void) {
    char *result = sqlglot_transpile("SELECT * FROM t LIMIT 10", "mysql", "tsql");
    if (result) {
        printf("%s\n", result);   /* → SELECT TOP 10 * FROM t */
        sqlglot_free(result);
    }
    return 0;
}
```

See [`examples/ffi_example.c`](examples/ffi_example.c) and [`examples/ffi_example.cpp`](examples/ffi_example.cpp) for complete examples.

### Link against the library

```bash
# macOS
gcc example.c -Itarget/ffi/include -Ltarget/release -lsqlglot_rust -o example

# Linux
gcc example.c -Itarget/ffi/include -Ltarget/release -lsqlglot_rust -lpthread -ldl -lm -o example
LD_LIBRARY_PATH=target/release ./example
```

## Documentation

- **[Installation](docs/installation.md)** — Dependency setup and verification
- **[Developer Guide](docs/developer-guide.md)** — Parsing, generating, transpiling, AST traversal, optimization, and serialization with full code examples
- **[API Reference](docs/reference.md)** — Complete type catalog, function signatures, dialect tables, and error variants

## Architecture

```text
src/
├── ast/          # AST node definitions (~40 expression types, 15 statement types)
├── bin/          # CLI binary (sqlglot) — feature-gated behind "cli"
├── builder/      # Expression builder API (fluent SQL construction)
├── dialects/     # 30 dialect definitions with transform rules
├── diff/         # AST diff / semantic SQL comparison
├── errors/       # Error types
├── executor/     # In-memory SQL execution engine
├── ffi.rs        # C-compatible FFI bindings (extern "C" API)
├── generator/    # SQL code generator
├── lib.rs        # Public API (parse, generate, transpile)
├── optimizer/    # Query optimization and scope analysis
├── parser/       # Recursive-descent SQL parser
├── planner/      # Logical query planner (execution plan DAG)
├── schema/       # Schema management (MappingSchema, dialect-aware lookups)
└── tokens/       # Token types (~200+ variants) and tokenizer
```

## Development

```bash
# Build
cargo build

# Build with CLI
cargo build --features cli

# Run tests (500+ tests)
cargo test

# Run CLI tests
cargo test --features cli --test test_cli

# Run benchmarks
cargo bench

# Lint
cargo clippy

# Format
cargo fmt
```

A `Makefile` is provided for convenience:

```bash
make build          # cargo build
make test           # cargo test
make lint           # cargo clippy
make fmt            # cargo fmt
make sbom           # Generate SPDX SBOM (see below)
make bump-version   # Update version across all files (see below)
make ffi            # Build FFI libraries for the current host
make ffi-all        # Build FFI libraries for all 4 targets (macOS/Linux × arm64/amd64)
make all            # build + sbom
```

## SBOM (Software Bill of Materials)

The project supports generating an SBOM in [SPDX](https://spdx.dev/) 2.3 JSON format using [`cargo-sbom`](https://crates.io/crates/cargo-sbom).

### Prerequisites

Install `cargo-sbom`:

```bash
cargo install cargo-sbom
```

### Generate the SBOM

```bash
make sbom
```

This writes the SBOM to `target/sbom/sqlglot-rust.spdx.json`. You can also run the command directly:

```bash
cargo sbom --output-format spdx_json_2_3 > target/sbom/sqlglot-rust.spdx.json
```

The generated SBOM includes all dependency packages with license information, download locations, and [Package URLs (PURLs)](https://github.com/package-url/purl-spec).

## Updating the Version

Use the `bump-version` Makefile target to update the version consistently across
all configuration and documentation files:

```bash
make bump-version VERSION=1.0.0
```

This updates:

- `Cargo.toml` — package version
- `README.md` — dependency snippet
- `docs/installation.md` — dependency snippet
- `Cargo.lock` — regenerated automatically

The `VERSION` parameter is required and must be a full semantic version (e.g. `1.0.0`, `0.10.1`).

## Acknowledgements

This project is inspired by and derived from [sqlglot](https://github.com/tobymao/sqlglot) by Toby Mao, licensed under the MIT License. The original sqlglot license is included in this repository as required by the MIT License terms. A copy of the original license can also be found [in the polyglot repository](https://github.com/tobilg/polyglot/tree/main/licenses).

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

This project includes code and concepts derived from [sqlglot](https://github.com/tobymao/sqlglot), which is also licensed under the MIT License. The original copyright notice and permission notice are reproduced below:

```text
MIT License

Copyright (c) 2023 Toby Mao

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
