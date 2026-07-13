# Developer Guide

A hands-on guide to using **sqlglot-rust** in your application — parsing SQL,
generating output, transpiling across dialects, inspecting and transforming
the AST, optimizing queries, and serializing results.

> **See also:** [Installation](installation.md) · [Reference](reference.md)

---

## Table of Contents

- [Parsing SQL](#parsing-sql)
  - [Single Statement](#single-statement)
  - [Multiple Statements](#multiple-statements)
  - [Parsing with Comments](#parsing-with-comments)
  - [What Can Be Parsed](#what-can-be-parsed)
- [Generating SQL](#generating-sql)
  - [Compact Output](#compact-output)
  - [Pretty Printing](#pretty-printing)
- [Transpiling Between Dialects](#transpiling-between-dialects)
  - [Single Statement Transpilation](#single-statement-transpilation)
  - [Multi-Statement Transpilation](#multi-statement-transpilation)
  - [Transpiling with Comments](#transpiling-with-comments)
  - [Function Mapping Examples](#function-mapping-examples)
  - [Data Type Mapping Examples](#data-type-mapping-examples)
  - [Time Format Mapping](#time-format-mapping)
  - [ILIKE Rewriting](#ilike-rewriting)
  - [Identifier Quoting](#identifier-quoting)
  - [LIMIT / TOP / FETCH FIRST](#limit--top--fetch-first)
- [Custom Dialect Plugins](#custom-dialect-plugins)
  - [Implementing a Custom Dialect](#implementing-a-custom-dialect)
  - [Registering and Using a Custom Dialect](#registering-and-using-a-custom-dialect)
  - [DialectRef](#dialectref)
  - [resolve_dialect](#resolve_dialect)
  - [Extension Points](#extension-points)
  - [Registry API](#registry-api)
- [Working with the AST](#working-with-the-ast)
  - [Matching Statement Types](#matching-statement-types)
  - [Inspecting a SELECT](#inspecting-a-select)
  - [Constructing Expressions](#constructing-expressions)
- [Expression Builder API](#expression-builder-api)
  - [SELECT Builder](#select-builder)
  - [Condition Builder](#condition-builder)
  - [Expression Factory Functions](#expression-factory-functions)
  - [Comparison and Arithmetic Helpers](#comparison-and-arithmetic-helpers)
  - [Statement Mutation Methods](#statement-mutation-methods)
- [Traversal and Search](#traversal-and-search)
  - [Walking the Tree](#walking-the-tree)
  - [Finding Nodes](#finding-nodes)
  - [Finding Tables and Columns](#finding-tables-and-columns)
- [Transforming the AST](#transforming-the-ast)
  - [Doubling Numeric Literals](#doubling-numeric-literals)
  - [Renaming Columns](#renaming-columns)
- [Schema Management](#schema-management)
  - [Creating a Schema](#creating-a-schema)
  - [Querying the Schema](#querying-the-schema)
  - [Dialect-Aware Normalization](#dialect-aware-normalization)
  - [Building Schemas from Maps](#building-schemas-from-maps)
- [Query Optimization](#query-optimization)
  - [Constant Folding](#constant-folding)
  - [Boolean Simplification](#boolean-simplification)
  - [Subquery Unnesting](#subquery-unnesting)
  - [Predicate Pushdown](#predicate-pushdown)
  - [Qualify Columns](#qualify-columns)
  - [Scope Analysis](#scope-analysis)
  - [Type Annotation (annotate_types)](#type-annotation-annotate_types)
  - [Column Lineage](#column-lineage)
- [Query Planner](#query-planner)
  - [Building a Plan](#building-a-plan)
  - [Inspecting Steps](#inspecting-steps)
  - [Visualization](#visualization)
- [Serialization (JSON Round-Tripping)](#serialization-json-round-tripping)
- [C/C++ FFI Bindings](#cc-ffi-bindings)
  - [Building the FFI Libraries](#building-the-ffi-libraries)
  - [C Example](#c-example)
  - [C++ Example with RAII](#c-example-with-raii)
  - [Linking](#linking)
- [Error Handling](#error-handling)
- [SBOM Generation](#sbom-generation)
- [Updating the Version](#updating-the-version)

---

## Parsing SQL

### Single Statement

Use `parse` to convert a SQL string into a `Statement` AST node.

```rust
use sqlglot_rust::{parse, Dialect};

let stmt = parse("SELECT 1 + 2", Dialect::Ansi).unwrap();
```

**Input:**

```sql
SELECT 1 + 2
```

**Output (`stmt`):**

```text
Statement::Select(SelectStatement {
    columns: [Expr { expr: BinaryOp { left: Number("1"), op: Plus, right: Number("2") }, alias: None }],
    ...
})
```

### Multiple Statements

`parse_statements` splits on semicolons and returns a `Vec<Statement>`:

```rust
use sqlglot_rust::parser::parse_statements;
use sqlglot_rust::Dialect;

let stmts = parse_statements(
    "SELECT 1; SELECT 2; INSERT INTO t VALUES (1)",
    Dialect::Ansi,
).unwrap();
assert_eq!(stmts.len(), 3);
```

**Input:**

```sql
SELECT 1; SELECT 2; INSERT INTO t VALUES (1)
```

**Output:** A `Vec` of three `Statement` values — two `Select` and one `Insert`.

### Parsing with Comments

By default, `parse` strips SQL comments. Use `parse_with_comments` or
`parse_statements_with_comments` to preserve them on the AST.

```rust
use sqlglot_rust::{parse_with_comments, generate, Dialect, Statement};

let stmt = parse_with_comments(
    "-- fetch active users\nSELECT * FROM users WHERE active = TRUE",
    Dialect::Ansi,
).unwrap();

// Comments are attached to the statement struct
if let Statement::Select(ref s) = stmt {
    assert_eq!(s.comments, vec!["-- fetch active users"]);
}

// generate() emits attached comments before the statement
let sql = generate(&stmt, Dialect::Ansi);
assert!(sql.starts_with("-- fetch active users"));
```

Supported comment syntaxes:

| Syntax | Example | Recognized By |
| --- | --- | --- |
| Line comment | `-- text` | All dialects |
| Block comment | `/* text */` | All dialects |
| Hash comment | `# text` | MySQL, Doris, SingleStore, StarRocks |

When transpiling to a dialect that does not support `#` comments, they are
automatically converted to `--` style.

For multiple statements:

```rust
use sqlglot_rust::{parse_statements_with_comments, Dialect};

let stmts = parse_statements_with_comments(
    "-- first\nSELECT 1; -- second\nSELECT 2",
    Dialect::Ansi,
).unwrap();
assert_eq!(stmts.len(), 2);
```

### What Can Be Parsed

| Category | Examples |
| --- | --- |
| **SELECT** | `SELECT`, `JOIN`, `WHERE`, `GROUP BY`, `HAVING`, `ORDER BY`, `LIMIT`, `OFFSET`, `FETCH FIRST`, `DISTINCT`, `TOP`, `QUALIFY`, `GROUPING SETS`, `CUBE`, `ROLLUP`, window functions |
| **CTEs** | `WITH ... AS (...)`, `WITH RECURSIVE ...` |
| **Set Operations** | `UNION`, `UNION ALL`, `INTERSECT`, `EXCEPT` |
| **DML** | `INSERT INTO ... VALUES`, `INSERT INTO ... SELECT`, `UPDATE ... SET`, `DELETE FROM` |
| **DDL** | `CREATE TABLE`, `CREATE TABLE ... AS SELECT`, `ALTER TABLE` (add/drop/rename column, add/drop constraint), `DROP TABLE`, `CREATE VIEW`, `DROP VIEW`, `TRUNCATE` |
| **Transaction** | `BEGIN`, `COMMIT`, `ROLLBACK`, `SAVEPOINT`, `RELEASE SAVEPOINT`, `ROLLBACK TO` |
| **Other** | `EXPLAIN [ANALYZE]`, `USE database` |
| **Expressions** | Binary/unary ops, `BETWEEN`, `IN`, `ANY`/`ALL`/`SOME`, `LIKE`, `ILIKE`, `CASE`, `CAST`, `TRY_CAST`, `EXTRACT`, `EXISTS`, `COALESCE`, `IF`, `NULLIF`, `INTERVAL`, `GROUPING()`, window functions, subqueries, array literals, JSON access (`->`, `->>`), parameters (`$1`, `?`, `:name`), lambdas |

---

## Generating SQL

### Compact Output

`generate` produces a single-line SQL string from an AST:

```rust
use sqlglot_rust::{parse, generate, Dialect};

let ast = parse("SELECT a FROM t WHERE a > 1", Dialect::Ansi).unwrap();
let sql = generate(&ast, Dialect::Ansi);
assert_eq!(sql, "SELECT a FROM t WHERE a > 1");
```

**Input (AST):** The parsed tree of `SELECT a FROM t WHERE a > 1`

**Output (String):**

```sql
SELECT a FROM t WHERE a > 1
```

### Pretty Printing

`generate_pretty` adds line breaks and indentation:

```rust
use sqlglot_rust::{parse, generate_pretty, Dialect};

let ast = parse(
    "SELECT u.id, u.name FROM users AS u INNER JOIN orders AS o ON u.id = o.user_id WHERE o.total > 100 ORDER BY u.name LIMIT 10",
    Dialect::Ansi,
).unwrap();
let pretty = generate_pretty(&ast, Dialect::Ansi);
println!("{pretty}");
```

**Input:**

```sql
SELECT u.id, u.name FROM users AS u INNER JOIN orders AS o ON u.id = o.user_id WHERE o.total > 100 ORDER BY u.name LIMIT 10
```

**Output:**

```sql
SELECT
  u.id,
  u.name
FROM users AS u
INNER JOIN orders AS o
  ON u.id = o.user_id
WHERE
  o.total > 100
ORDER BY
  u.name
LIMIT 10
```

#### Pretty-printing with CTEs

```rust
let ast = parse(
    "WITH active AS (SELECT * FROM users WHERE active = TRUE) SELECT a.name, COUNT(*) FROM active AS a INNER JOIN orders AS o ON a.id = o.user_id GROUP BY a.name HAVING COUNT(*) > 5 ORDER BY a.name",
    Dialect::Ansi,
).unwrap();
println!("{}", generate_pretty(&ast, Dialect::Ansi));
```

**Output:**

```sql
WITH active AS (
  SELECT
    *
  FROM users
  WHERE
    active = TRUE
)
SELECT
  a.name,
  COUNT(*)
FROM active AS a
INNER JOIN orders AS o
  ON a.id = o.user_id
GROUP BY
  a.name
HAVING
  COUNT(*) > 5
ORDER BY
  a.name
```

---

## Transpiling Between Dialects

`transpile` parses with one dialect and generates with another, applying all
dialect-specific transformations automatically.

### Single Statement Transpilation

```rust
use sqlglot_rust::{transpile, Dialect};

let result = transpile(
    "SELECT NOW()",
    Dialect::Postgres,   // read dialect
    Dialect::BigQuery,   // write dialect
).unwrap();
assert_eq!(result, "SELECT CURRENT_TIMESTAMP()");
```

**Input:**

| | |
| --- | --- |
| SQL | `SELECT NOW()` |
| Read dialect | PostgreSQL |
| Write dialect | BigQuery |

**Output:**

```sql
SELECT CURRENT_TIMESTAMP()
```

### Multi-Statement Transpilation

```rust
use sqlglot_rust::transpile_statements;
use sqlglot_rust::Dialect;

let results = transpile_statements(
    "SELECT SUBSTR(x, 1, 3); SELECT NOW()",
    Dialect::Mysql,
    Dialect::Postgres,
).unwrap();
```

**Input:**

```sql
SELECT SUBSTR(x, 1, 3); SELECT NOW()
```

**Output (`results`):**

| Index | SQL                         |
| ----- | --------------------------- |
| `[0]` | `SELECT SUBSTRING(x, 1, 3)` |
| `[1]` | `SELECT NOW()`              |

### Transpiling with Comments

Use `transpile_with_comments` to preserve comments through a dialect conversion
in one call:

```rust
use sqlglot_rust::{transpile_with_comments, Dialect};

let sql = transpile_with_comments(
    "-- user lookup\nSELECT NOW()",
    Dialect::Postgres,
    Dialect::Tsql,
).unwrap();
assert!(sql.contains("-- user lookup"));
assert!(sql.contains("GETDATE()"));
```

MySQL `#` comments are normalized to `--` when the target dialect does not
support hash comments.

### Function Mapping Examples

Below are common function transformations. Each row shows a transpile call
with its input, source dialect, target dialect, and the resulting SQL.

Recognized functions are parsed into **Typed Function Expressions** (see
[TypedFunction Enum](reference.md#typedfunction-enum) in the reference) which
enables the generator to emit the correct syntax for each target dialect.

| Input SQL | From | To | Output SQL |
| --- | --- | --- | --- |
| `SELECT NOW()` | PostgreSQL | BigQuery | `SELECT CURRENT_TIMESTAMP()` |
| `SELECT NOW()` | PostgreSQL | T-SQL | `SELECT GETDATE()` |
| `SELECT NOW()` | PostgreSQL | Snowflake | `SELECT CURRENT_TIMESTAMP()` |
| `SELECT GETDATE()` | T-SQL | PostgreSQL | `SELECT NOW()` |
| `SELECT GETDATE()` | T-SQL | BigQuery | `SELECT CURRENT_TIMESTAMP()` |
| `SELECT SUBSTRING(x, 1, 3) FROM t` | PostgreSQL | MySQL | `SELECT SUBSTR(x, 1, 3) FROM t` |
| `SELECT SUBSTR(x, 1, 3) FROM t` | MySQL | PostgreSQL | `SELECT SUBSTRING(x, 1, 3) FROM t` |
| `SELECT LEN(name) FROM t` | BigQuery | PostgreSQL | `SELECT LENGTH(name) FROM t` |
| `SELECT LEN(x) FROM t` | BigQuery | MySQL | `SELECT LENGTH(x) FROM t` |
| `SELECT CEIL(x)` | ANSI | T-SQL | `SELECT CEILING(x)` |
| `SELECT POW(x, 2)` | MySQL | T-SQL | `SELECT POWER(x, 2)` |
| `SELECT DATE_TRUNC('month', d)` | PostgreSQL | T-SQL | `SELECT DATETRUNC(month, d)` |
| `SELECT ARRAY_AGG(x)` | PostgreSQL | DuckDB | `SELECT LIST(x)` |
| `SELECT HEX(x)` | MySQL | Presto | `SELECT TO_HEX(x)` |
| `SELECT IFNULL(a, b) FROM t` | MySQL | PostgreSQL | `SELECT COALESCE(a, b) FROM t` |
| `SELECT IFNULL(a, b) FROM t` | MySQL | T-SQL | `SELECT ISNULL(a, b) FROM t` |

### Typed Function Expressions

When the parser encounters a recognized function name (72+ functions across 8
categories), it creates `Expr::TypedFunction` instead of the generic
`Expr::Function`. This provides:

- **Dialect-specific SQL generation** — the generator emits the correct function
  name and argument order for each target dialect without string-based rewriting
- **Semantic argument types** — each function variant carries typed fields
  (e.g., `Substring { expr, start, length }`) rather than a generic argument list
- **Proper AST traversal** — `walk()` and `transform()` recurse into typed
  function children

Unrecognized functions fall back to the generic `Expr::Function` with
string-based name and argument list, preserving backward compatibility.

```rust
use sqlglot_rust::{parse, Dialect, ast::types::Expr};

let stmt = parse("SELECT SUBSTRING(name, 1, 3) FROM users", Dialect::Ansi).unwrap();

// The parser creates a TypedFunction variant:
// Expr::TypedFunction {
//     func: TypedFunction::Substring { expr: Column("name"), start: 1, length: Some(3) },
//     filter: None,
//     over: None,
// }
```

**Categories** (72 variants):
Date/Time (12), String (17), Aggregate (9), Array (6), JSON (4),
Window (8), Math (11), Conversion (5)

See the [TypedFunction Enum](reference.md#typedfunction-enum) in the API reference
for the complete variant list and dialect-specific generation rules.

### Data Type Mapping Examples

CAST expressions are rewritten to use the target dialect's preferred type names.

| Input SQL | From | To | Output SQL |
| --- | --- | --- | --- |
| `SELECT CAST(x AS TEXT) FROM t` | PostgreSQL | BigQuery | `SELECT CAST(x AS STRING) FROM t` |
| `SELECT CAST(x AS STRING) FROM t` | BigQuery | PostgreSQL | `SELECT CAST(x AS TEXT) FROM t` |
| `SELECT CAST(x AS STRING) FROM t` | BigQuery | MySQL | `SELECT CAST(x AS TEXT) FROM t` |
| `SELECT CAST(x AS INT) FROM t` | PostgreSQL | BigQuery | `SELECT CAST(x AS BIGINT) FROM t` |
| `SELECT CAST(x AS FLOAT) FROM t` | PostgreSQL | BigQuery | `SELECT CAST(x AS DOUBLE) FROM t` |
| `SELECT CAST(x AS BYTEA) FROM t` | PostgreSQL | MySQL | `SELECT CAST(x AS BLOB) FROM t` |

### Time Format Mapping

Date/time formatting functions (`DATE_FORMAT`, `TO_CHAR`, `STRFTIME`, etc.) use format
strings that vary by dialect. During transpilation, both the function name AND the format
specifiers are automatically converted:

```rust
use sqlglot_rust::{transpile, Dialect};

// MySQL → PostgreSQL
let result = transpile(
    "SELECT DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s')",
    Dialect::Mysql,
    Dialect::Postgres
).unwrap();
// => "SELECT TO_CHAR(created_at, 'YYYY-MM-DD HH24:MI:SS')"

// PostgreSQL → Spark (uses Java DateTimeFormatter patterns)
let result = transpile(
    "SELECT TO_CHAR(dt, 'YYYY-MM-DD HH24:MI:SS')",
    Dialect::Postgres,
    Dialect::Spark
).unwrap();
// => "SELECT DATE_FORMAT(dt, 'yyyy-MM-dd HH:mm:ss')"

// MySQL → BigQuery
let result = transpile(
    "SELECT DATE_FORMAT(created_at, '%Y-%m-%d')",
    Dialect::Mysql,
    Dialect::BigQuery
).unwrap();
// => "SELECT FORMAT_TIMESTAMP(created_at, '%Y-%m-%d')"
```

**Format Style Families:**

| Family | Dialects | Year | Month | Day | Hour (24h) | Minute | Second |
| --- | --- | --- | --- | --- | --- | --- | --- |
| strftime | SQLite, BigQuery, DuckDB | `%Y` | `%m` | `%d` | `%H` | `%M` | `%S` |
| MySQL | MySQL, Doris, SingleStore, StarRocks | `%Y` | `%m` | `%d` | `%H` | `%i` | `%s` |
| Postgres | PostgreSQL, Oracle, Redshift | `YYYY` | `MM` | `DD` | `HH24` | `MI` | `SS` |
| Java | Spark, Hive, Databricks, Presto, Trino | `yyyy` | `MM` | `dd` | `HH` | `mm` | `ss` |

**Direct Format Conversion:**

You can also convert format strings directly without parsing SQL:

```rust
use sqlglot_rust::{format_time, format_time_dialect, TimeFormatStyle, Dialect};

// Convert between format styles
let pg_format = format_time("%Y-%m-%d", TimeFormatStyle::Strftime, TimeFormatStyle::Postgres);
assert_eq!(pg_format, "YYYY-MM-DD");

// Or use dialect-to-dialect conversion
let spark_format = format_time_dialect("YYYY-MM-DD", Dialect::Postgres, Dialect::Spark);
assert_eq!(spark_format, "yyyy-MM-dd");
```

### ILIKE Rewriting

Dialects without native `ILIKE` support get a `LOWER()` rewrite:

```rust
// Preserved in dialects that support ILIKE natively
transpile("SELECT * FROM t WHERE name ILIKE '%test%'", Dialect::Postgres, Dialect::Postgres);
// => "SELECT * FROM t WHERE name ILIKE '%test%'"

transpile("SELECT * FROM t WHERE name ILIKE '%test%'", Dialect::Postgres, Dialect::DuckDb);
// => "SELECT * FROM t WHERE name ILIKE '%test%'"

transpile("SELECT * FROM t WHERE name ILIKE '%test%'", Dialect::Postgres, Dialect::Snowflake);
// => "SELECT * FROM t WHERE name ILIKE '%test%'"

// Rewritten for dialects without ILIKE
transpile("SELECT * FROM t WHERE name ILIKE '%test%'", Dialect::Postgres, Dialect::Mysql);
// => "SELECT * FROM t WHERE LOWER(name) LIKE LOWER('%test%')"

transpile("SELECT * FROM t WHERE name ILIKE '%test%'", Dialect::Postgres, Dialect::Sqlite);
// => "SELECT * FROM t WHERE LOWER(name) LIKE LOWER('%test%')"
```

### Identifier Quoting

Quoted identifiers are automatically converted to the target dialect's style:

```sql
-- PostgreSQL input:  SELECT "my column" FROM "my table"
-- MySQL output:      SELECT `my column` FROM `my table`
-- T-SQL output:      SELECT [my column] FROM [my table]
```

| Dialect Family | Quote Style | Example |
| --- | --- | --- |
| PostgreSQL, Snowflake, Oracle, ANSI | Double-quote | `"my_column"` |
| MySQL, BigQuery, Hive, Spark | Backtick | `` `my_column` `` |
| T-SQL, Fabric | Bracket | `[my_column]` |

### LIMIT / TOP / FETCH FIRST

Row-limit syntax is translated between dialect conventions:

| Input | From | To | Output |
| --- | --- | --- | --- |
| `SELECT * FROM t LIMIT 10` | ANSI | T-SQL | `SELECT TOP 10 * FROM t` |
| `SELECT * FROM t LIMIT 10` | ANSI | Oracle | `SELECT * FROM t FETCH FIRST 10 ROWS ONLY` |
| `SELECT TOP 10 * FROM t` | T-SQL | ANSI | `SELECT * FROM t LIMIT 10` |

---

## Custom Dialect Plugins

The plugin system allows external code to define and register custom SQL
dialects without modifying the library source. This is useful for proprietary
SQL extensions, internal database engines, or experimental dialects.

### Implementing a Custom Dialect

Implement the `DialectPlugin` trait — only override the methods you need:

```rust
use sqlglot_rust::DialectPlugin;
use sqlglot_rust::ast::{DataType, Expr, QuoteStyle};

struct AcmeDialect;

impl DialectPlugin for AcmeDialect {
    fn name(&self) -> &str { "acme" }

    fn quote_style(&self) -> Option<QuoteStyle> {
        Some(QuoteStyle::Backtick)
    }

    fn supports_ilike(&self) -> Option<bool> {
        Some(true)
    }

    fn map_function_name(&self, name: &str) -> Option<String> {
        match name.to_uppercase().as_str() {
            "NOW" => Some("ACME_TIMESTAMP".to_string()),
            "LENGTH" => Some("ACME_LEN".to_string()),
            _ => None, // fall through to default
        }
    }

    fn map_data_type(&self, dt: &DataType) -> Option<DataType> {
        match dt {
            DataType::Text => Some(DataType::Varchar(Some(65535))),
            _ => None,
        }
    }
}
```

### Registering and Using a Custom Dialect

```rust
use sqlglot_rust::{register_dialect, DialectRef, Dialect};
use sqlglot_rust::dialects::plugin::transpile_ext;

// Register the custom dialect globally (once)
register_dialect(AcmeDialect);

// Use it in the transpile pipeline via DialectRef
let result = transpile_ext(
    "SELECT NOW(), LENGTH(name) FROM users",
    &DialectRef::from(Dialect::Ansi),   // read as ANSI
    &DialectRef::custom("acme"),        // write as custom "acme"
).unwrap();
assert_eq!(result, "SELECT ACME_TIMESTAMP(), ACME_LEN(name) FROM users");
```

### DialectRef

`DialectRef` is a unified handle for both built-in and custom dialects:

| Variant | Description |
| --- | --- |
| `DialectRef::BuiltIn(Dialect::Postgres)` | References a built-in dialect |
| `DialectRef::custom("acme")` | References a registered custom dialect |

You can convert any `Dialect` into a `DialectRef` with `DialectRef::from(dialect)`.

### resolve_dialect

Look up a dialect by name, checking built-in dialects first, then the custom registry:

```rust
use sqlglot_rust::{resolve_dialect, DialectRef, Dialect};

assert_eq!(resolve_dialect("postgres"), Some(DialectRef::from(Dialect::Postgres)));
assert_eq!(resolve_dialect("acme"), Some(DialectRef::custom("acme")));
assert_eq!(resolve_dialect("unknown"), None);
```

### Extension Points

| Method | Purpose | Default |
| --- | --- | --- |
| `name()` | Canonical dialect name (required) | — |
| `quote_style()` | Identifier quoting convention | `None` (double-quote) |
| `supports_ilike()` | Native ILIKE support | `None` (false) |
| `map_function_name(name)` | Rename functions | `None` (keep original) |
| `map_data_type(dt)` | Remap data types | `None` (keep original) |
| `transform_expr(expr)` | Custom expression transform | `None` (fallthrough) |
| `transform_statement(stmt)` | Custom statement transform | `None` (fallthrough) |

### Registry API

| Function | Description |
| --- | --- |
| `register_dialect(plugin)` | Register a plugin globally |
| `DialectRegistry::global().get(name)` | Look up a plugin by name |
| `DialectRegistry::global().unregister(name)` | Remove a plugin |
| `DialectRegistry::global().registered_names()` | List all registered custom dialect names |

---

## Working with the AST

### Matching Statement Types

After parsing, match on the `Statement` enum to handle each SQL command:

```rust
use sqlglot_rust::{parse, Dialect, Statement};

let stmt = parse("SELECT a, b FROM users", Dialect::Ansi).unwrap();

match &stmt {
    Statement::Select(s) => {
        println!("Columns: {}", s.columns.len());  // => 2
        println!("Has WHERE: {}", s.where_clause.is_some());  // => false
    }
    Statement::Insert(i) => { println!("Insert into: {}", i.table.name); }
    Statement::Update(u) => { println!("Update: {}", u.table.name); }
    Statement::Delete(d) => { println!("Delete from: {}", d.table.name); }
    Statement::CreateTable(c) => { println!("Create: {}", c.table.name); }
    Statement::DropTable(d)   => { println!("Drop: {}", d.table.name); }
    Statement::AlterTable(a)  => { println!("Alter: {}", a.table.name); }
    Statement::SetOperation(s) => { println!("Set op: {:?}", s.op); }
    Statement::Transaction(t) => { println!("Transaction: {:?}", t); }
    _ => {}
}
```

**Input:** `SELECT a, b FROM users`

**Output:**

```text
Columns: 2
Has WHERE: false
```

### Inspecting a SELECT

Access individual clauses of a `SelectStatement`:

```rust
use sqlglot_rust::{parse, Dialect, Statement};
use sqlglot_rust::ast::SelectItem;

let stmt = parse(
    "SELECT u.name, COUNT(*) FROM users AS u WHERE u.active = TRUE GROUP BY u.name ORDER BY u.name LIMIT 10",
    Dialect::Ansi,
).unwrap();

if let Statement::Select(s) = &stmt {
    // Columns
    println!("Select list has {} items", s.columns.len());  // => 2

    // FROM table
    if let Some(from) = &s.from {
        println!("From: {:?}", from.source);  // => Table { name: "users", alias: Some("u") }
    }

    // WHERE
    if let Some(w) = &s.where_clause {
        println!("Where: {}", w.sql());  // => u.active = TRUE
    }

    // GROUP BY
    println!("Group by: {} exprs", s.group_by.len());  // => 1

    // ORDER BY
    println!("Order by: {} items", s.order_by.len());  // => 1

    // LIMIT
    if let Some(l) = &s.limit {
        println!("Limit: {}", l.sql());  // => 10
    }
}
```

**Input:** `SELECT u.name, COUNT(*) FROM users AS u WHERE u.active = TRUE GROUP BY u.name ORDER BY u.name LIMIT 10`

**Output:**

```text
Select list has 2 items
From: Table(TableRef { name: "users", alias: Some("u"), ... })
Where: u.active = TRUE
Group by: 1 exprs
Order by: 1 items
Limit: 10
```

### Constructing Expressions

Build AST nodes programmatically:

```rust
use sqlglot_rust::Expr;
use sqlglot_rust::ast::BinaryOperator;

// Build: price * quantity
let expr = Expr::BinaryOp {
    left: Box::new(Expr::Column {
        name: "price".to_string(),
        table: None,
        quote_style: Default::default(),
        table_quote_style: Default::default(),
    }),
    op: BinaryOperator::Multiply,
    right: Box::new(Expr::Column {
        name: "quantity".to_string(),
        table: None,
        quote_style: Default::default(),
        table_quote_style: Default::default(),
    }),
};

println!("{}", expr.sql());
```

**Output:**

```text
price * quantity
```

---

## Expression Builder API

The Expression Builder API provides a fluent, ergonomic way to construct SQL
queries programmatically without manually assembling AST enum variants.

### SELECT Builder

Build SELECT queries using method chaining:

```rust
use sqlglot_rust::builder::{select, select_all, select_distinct};
use sqlglot_rust::{generate, Dialect};

// Basic SELECT
let query = select(&["a", "b", "c"])
    .from("users")
    .where_clause("active = true")
    .order_by(&["created_at DESC"])
    .limit(10)
    .build();

let sql = generate(&query, Dialect::Postgres);
// => SELECT a, b, c FROM users WHERE active = true ORDER BY created_at DESC LIMIT 10

// SELECT with JOINs
let query = select(&["u.name", "COUNT(o.id) AS order_count"])
    .from("users")
    .left_join("orders", "u.id = o.user_id")
    .where_clause("u.active = true")
    .group_by(&["u.name"])
    .having("COUNT(o.id) > 0")
    .build();

// SELECT DISTINCT
let query = select_distinct(&["category"])
    .from("products")
    .build();

// SELECT *
let query = select_all()
    .from("users")
    .build();
```

**SelectBuilder Methods:**

| Method | Description |
| --- | --- |
| `columns(&[&str])` | Add columns to SELECT list |
| `column_expr(Expr, Option<&str>)` | Add expression with optional alias |
| `all()` | Add wildcard (*) |
| `all_from(&str)` | Add qualified wildcard (table.*) |
| `distinct()` | Enable DISTINCT |
| `from(&str)` | Set FROM table |
| `from_table(TableRef)` | Set FROM with TableRef |
| `from_subquery(Statement, &str)` | Set FROM subquery with alias |
| `join(&str, &str)` | INNER JOIN table ON condition |
| `left_join(&str, &str)` | LEFT JOIN |
| `right_join(&str, &str)` | RIGHT JOIN |
| `full_join(&str, &str)` | FULL JOIN |
| `cross_join(&str)` | CROSS JOIN |
| `where_clause(&str)` | Set WHERE condition |
| `and_where(&str)` | Add to WHERE with AND |
| `or_where(&str)` | Add to WHERE with OR |
| `group_by(&[&str])` | Set GROUP BY |
| `having(&str)` | Set HAVING clause |
| `order_by(&[&str])` | Set ORDER BY |
| `limit(i64)` | Set LIMIT |
| `offset(i64)` | Set OFFSET |
| `top(i64)` | Set TOP (T-SQL) |
| `qualify(&str)` | Set QUALIFY (BigQuery/Snowflake) |
| `build()` | Build Statement |
| `build_select()` | Build SelectStatement |

### Condition Builder

Build complex conditions with AND/OR/NOT:

```rust
use sqlglot_rust::builder::condition;

// Build: x = 1 AND y = 2 OR z = 3
let cond = condition("x = 1")
    .and("y = 2")
    .or("z = 3")
    .build();

// Negate a condition
let negated = condition("status = 'active'")
    .not()
    .build();
```

### Expression Factory Functions

Create expressions without manual AST construction:

```rust
use sqlglot_rust::builder::*;
use sqlglot_rust::ast::DataType;

// Column reference
let col = column("name", Some("users"));  // users.name

// Table reference
let tbl = table("users", Some("public"));  // public.users

// Literals
let num = literal(42);
let s = string_literal("hello");
let b = boolean(true);
let n = null();

// CAST expression
let casted = cast(column("id", None), DataType::BigInt);

// Functions
let count = func("COUNT", vec![column("id", None)]);
let count_distinct = func_distinct("COUNT", vec![column("user_id", None)]);

// Logical combinations
let cond1 = eq(column("x", None), literal(1));
let cond2 = eq(column("y", None), literal(2));
let combined_and = and_all(vec![cond1.clone(), cond2.clone()]);
let combined_or = or_all(vec![cond1, cond2]);

// Subqueries and EXISTS
let inner = select(&["id"]).from("users").where_clause("active = true").build();
let sub = subquery(inner.clone());
let exists_check = exists(inner, false);

// Aliases
let aliased = alias(column("first_name", None), "name");

// Wildcards
let star_expr = star();
let qualified = qualified_star("users");
```

### Comparison and Arithmetic Helpers

Shorthand functions for common operations:

```rust
use sqlglot_rust::builder::*;

// Comparisons
let e = eq(column("a", None), literal(1));      // a = 1
let e = neq(column("a", None), literal(1));     // a <> 1
let e = lt(column("a", None), literal(10));     // a < 10
let e = lte(column("a", None), literal(10));    // a <= 10
let e = gt(column("a", None), literal(0));      // a > 0
let e = gte(column("a", None), literal(0));     // a >= 0

// NULL checks
let e = is_null(column("deleted_at", None));        // deleted_at IS NULL
let e = is_not_null(column("created_at", None));    // created_at IS NOT NULL

// BETWEEN
let e = between(column("age", None), literal(18), literal(65));

// IN list
let e = in_list(column("status", None), vec![
    string_literal("active"),
    string_literal("pending"),
]);

// IN subquery
let sub = select(&["id"]).from("active_users").build();
let e = in_subquery(column("user_id", None), sub);

// LIKE
let e = like(column("name", None), string_literal("%john%"));

// Arithmetic
let e = add(column("price", None), literal(10));    // price + 10
let e = sub(column("total", None), literal(5));     // total - 5
let e = mul(column("qty", None), column("price", None));  // qty * price
let e = div(column("amount", None), literal(100));  // amount / 100

// NOT
let e = not(eq(column("active", None), boolean(true)));
```

### Statement Mutation Methods

Modify existing statements after construction:

```rust
use sqlglot_rust::builder::select;
use sqlglot_rust::ast::JoinType;

let mut stmt = select(&["a"]).from("table1").build_select();

// Add more columns
stmt.add_select("b");
stmt.add_select("c AS column_c");

// Add WHERE conditions (combined with AND)
stmt.add_where("x > 1");
stmt.add_where("y < 10");

// Add JOINs
stmt.add_join("table2", "table1.id = table2.t1_id", JoinType::Inner);
stmt.add_join("table3", "table2.id = table3.t2_id", JoinType::Left);

// Wrap as subquery
let subquery_source = stmt.as_subquery("sub");
```

### Parsing Helpers

Parse SQL fragments into expressions:

```rust
use sqlglot_rust::builder::{parse_expr, parse_condition};

// Parse an expression
let expr = parse_expr("x + y * 2").unwrap();

// Parse a condition
let cond = parse_condition("a > 1 AND b < 10").unwrap();
```

---

## Traversal and Search

### Walking the Tree

`walk` visits every node in the expression tree depth-first. The visitor
callback returns `true` to recurse into children, `false` to skip them.

```rust
use sqlglot_rust::{parse, Dialect, Expr, Statement};
use sqlglot_rust::ast::SelectItem;

let stmt = parse("SELECT a + b * 2", Dialect::Ansi).unwrap();
if let Statement::Select(s) = &stmt {
    if let SelectItem::Expr { expr, .. } = &s.columns[0] {
        let mut col_names = Vec::new();
        expr.walk(&mut |e| {
            if let Expr::Column { name, .. } = e {
                col_names.push(name.clone());
            }
            true
        });
        println!("Columns found: {:?}", col_names);
    }
}
```

**Input:** `SELECT a + b * 2`

**Output:**

```text
Columns found: ["a", "b"]
```

### Finding Nodes

#### `find` — First match

Returns the first node matching a predicate, or `None`:

```rust
let found = expr.find(&|e| matches!(e, Expr::Number(_)));
```

**Input (expr from):** `SELECT a + 1`

**Output:**

```text
Some(Number("1"))
```

#### `find_all` — All matches

Returns every node matching a predicate:

```rust
let numbers = expr.find_all(&|e| matches!(e, Expr::Number(_)));
```

**Input (expr from):** `SELECT 1 + 2 + 3`

**Output:**

```text
[Number("1"), Number("2"), Number("3")]  // len() == 3
```

### Finding Tables and Columns

Free functions in the `ast` module collect common references:

```rust
use sqlglot_rust::{parse, Dialect};
use sqlglot_rust::ast::{find_columns, find_tables};

// --- find_tables ---
let stmt = parse(
    "SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id",
    Dialect::Ansi,
).unwrap();
let tables = find_tables(&stmt);
println!("{:?}", tables.iter().map(|t| &t.name).collect::<Vec<_>>());
```

**Input:** `SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id`

**Output:**

```text
["t1", "t2"]
```

```rust
// --- find_columns ---
use sqlglot_rust::{parse, Dialect, Expr, Statement};
use sqlglot_rust::ast::{find_columns, SelectItem};

let stmt = parse("SELECT a + b + c", Dialect::Ansi).unwrap();
if let Statement::Select(s) = &stmt {
    if let SelectItem::Expr { expr, .. } = &s.columns[0] {
        let cols = find_columns(expr);
        println!("Column count: {}", cols.len());
    }
}
```

**Input:** `SELECT a + b + c`

**Output:**

```text
Column count: 3
```

---

## Transforming the AST

`transform` creates a new expression tree by applying a function bottom-up to
every node. The original tree is consumed and each node passes through the
function, which can return it unchanged or provide a replacement.

### Doubling Numeric Literals

```rust
use sqlglot_rust::{parse, generate, Expr, Dialect, Statement};
use sqlglot_rust::ast::SelectItem;

let stmt = parse("SELECT 1 + 2", Dialect::Ansi).unwrap();

let new_stmt = if let Statement::Select(mut s) = stmt {
    if let SelectItem::Expr { expr, .. } = &mut s.columns[0] {
        *expr = expr.clone().transform(&|e| {
            if let Expr::Number(n) = &e {
                let val: f64 = n.parse().unwrap();
                Expr::Number(((val * 2.0) as i64).to_string())
            } else {
                e
            }
        });
    }
    Statement::Select(s)
} else {
    stmt
};

let sql = generate(&new_stmt, Dialect::Ansi);
println!("{sql}");
```

**Input:** `SELECT 1 + 2`

**Output:**

```text
SELECT 2 + 4
```

### Renaming Columns

```rust
let transformed = expr.transform(&|e| {
    if let Expr::Column { name, table, quote_style, table_quote_style } = &e {
        if name == "old_col" {
            return Expr::Column {
                name: "new_col".to_string(),
                table: table.clone(),
                quote_style: *quote_style,
                table_quote_style: *table_quote_style,
            };
        }
    }
    e
});
println!("{}", transformed.sql());
```

**Input (expr from):** `SELECT old_col + b`

**Output:**

```text
new_col + b
```

---

## Query Optimization

The optimizer applies safe, semantics-preserving rewrites to a `Statement`.

```rust
use sqlglot_rust::{parse, generate, Dialect};
use sqlglot_rust::optimizer::optimize;

let stmt = parse("SELECT 1 + 2 * 3", Dialect::Ansi).unwrap();
let optimized = optimize(stmt).unwrap();
let sql = generate(&optimized, Dialect::Ansi);
assert_eq!(sql, "SELECT 7");
```

### Constant Folding

Compile-time–evaluable expressions are reduced to their result:

| Input | Output |
| --- | --- |
| `SELECT 1 + 2` | `SELECT 3` |
| `SELECT 10 - 3` | `SELECT 7` |
| `SELECT 3 * 4` | `SELECT 12` |
| `SELECT 10 / 2` | `SELECT 5` |
| `SELECT 10 % 3` | `SELECT 1` |
| `SELECT 1 + 2 * 3` | `SELECT 7` |
| `SELECT 'hello' \|\| ' ' \|\| 'world'` | `SELECT 'hello world'` |
| `SELECT 1 < 2` | `SELECT TRUE` |
| `SELECT 1 > 2` | `SELECT FALSE` |
| `SELECT 1 = 1` | `SELECT TRUE` |
| `SELECT 1 <> 1` | `SELECT FALSE` |
| `SELECT 1 <= 1` | `SELECT TRUE` |
| `SELECT 2 <= 1` | `SELECT FALSE` |

### Boolean Simplification

Tautologies and contradictions are eliminated:

| Input | Output | Rule |
| --- | --- | --- |
| `SELECT TRUE AND x` | `SELECT x` | AND identity |
| `SELECT FALSE AND x` | `SELECT FALSE` | AND annihilation |
| `SELECT TRUE OR x` | `SELECT TRUE` | OR annihilation |
| `SELECT FALSE OR x` | `SELECT x` | OR identity |
| `SELECT NOT NOT x` | `SELECT x` | Double negation |
| `SELECT NOT TRUE` | `SELECT FALSE` | NOT constant |
| `SELECT NOT FALSE` | `SELECT TRUE` | NOT constant |
| `SELECT a FROM t WHERE TRUE` | `SELECT a FROM t` | WHERE TRUE removal |

### Subquery Unnesting

Correlated subqueries in WHERE clauses are rewritten into equivalent JOINs,
which most query engines execute more efficiently.

```rust
use sqlglot_rust::{parse, generate, Dialect};
use sqlglot_rust::optimizer::optimize;

let stmt = parse(
    "SELECT a.id FROM a WHERE EXISTS (SELECT 1 FROM b WHERE b.id = a.id)",
    Dialect::Ansi,
).unwrap();
let optimized = optimize(stmt).unwrap();
let sql = generate(&optimized, Dialect::Ansi);
// EXISTS is rewritten to INNER JOIN
assert!(sql.contains("INNER JOIN"));
```

| Input | Output | Rule |
| --- | --- | --- |
| `WHERE EXISTS (SELECT … WHERE b.id = a.id)` | `INNER JOIN (SELECT DISTINCT …) ON …` | Semi-join |
| `WHERE NOT EXISTS (SELECT … WHERE b.id = a.id)` | `LEFT JOIN … WHERE _u.col IS NULL` | Anti-join |
| `WHERE x IN (SELECT col FROM …)` | `INNER JOIN (SELECT DISTINCT …)` | Semi-join |
| `WHERE x NOT IN (SELECT col FROM …)` | `LEFT JOIN … WHERE _u.col IS NULL` | Anti-join |

The pass is conservative and leaves the query unchanged when:

- The subquery has no equality correlation predicates.
- The subquery uses non-equality correlations (e.g., `b.val < a.val`) that would require LATERAL/APPLY.
- The subquery is inside a function in the SELECT list (e.g., `COALESCE(...)`) rather than in a WHERE predicate.

You can also call the pass directly:

```rust
use sqlglot_rust::optimizer::unnest_subqueries::unnest_subqueries;

let unnested = unnest_subqueries(stmt);
```

### Predicate Pushdown

The `pushdown_predicates` pass moves WHERE conditions from outer queries into
derived tables and JOIN conditions where possible, reducing the data processed
by inner queries.

```rust
use sqlglot_rust::{parse, generate, Dialect};
use sqlglot_rust::pushdown_predicates;

let stmt = parse(
    "SELECT sub.id FROM (SELECT id, name FROM t) AS sub WHERE sub.id > 5",
    Dialect::Ansi,
).unwrap();
let pushed = pushdown_predicates(stmt);
let sql = generate(&pushed, Dialect::Ansi);
assert_eq!(sql, "SELECT sub.id FROM (SELECT id, name FROM t WHERE id > 5) AS sub");
```

| Input | Output | Rule |
| --- | --- | --- |
| `SELECT … FROM (SELECT … FROM t) AS s WHERE s.x > 5` | `SELECT … FROM (SELECT … FROM t WHERE x > 5) AS s` | Push into derived table |
| `SELECT … FROM a JOIN b ON … WHERE b.x > 10` | `SELECT … FROM a JOIN b ON … AND b.x > 10` | Push into JOIN ON |
| `WHERE a.x > 5 AND b.y = 10` | Splits: each conjunct pushed independently | AND splitting |

The pass does **not** push predicates when:

- The target has `LIMIT`, `OFFSET`, or `FETCH FIRST` (would change result set).
- The target has `DISTINCT` (pushdown could change deduplication behavior).
- The target's SELECT list contains window functions.
- The predicate contains aggregate functions, window functions, or subqueries.
- The predicate uses non-deterministic functions (`RAND`, `RANDOM`, etc.).
- The predicate references columns from multiple sources (cross-table join predicates).
- The JOIN is `LEFT`, `RIGHT`, or `FULL` (pushdown changes outer join semantics).

### Qualify Columns

`qualify_columns` resolves column references against a schema,
adding table qualifiers to unqualified columns and expanding wildcards.

```rust
use sqlglot_rust::{parse, generate, Dialect};
use sqlglot_rust::optimizer::qualify_columns::qualify_columns;
use sqlglot_rust::schema::MappingSchema;

let schema = MappingSchema::new()
    .with_table(vec!["users"], vec!["id", "name", "email"])
    .with_table(vec!["orders"], vec!["id", "user_id", "amount"]);

let stmt = parse("SELECT * FROM users", Dialect::Ansi).unwrap();
let qualified = qualify_columns(stmt, &schema);
assert_eq!(
    generate(&qualified, Dialect::Ansi),
    "SELECT users.id, users.name, users.email FROM users"
);
```

| Transformation | Before | After |
| --- | --- | --- |
| Qualify column | `SELECT col FROM t` | `SELECT t.col FROM t` |
| Expand `*` | `SELECT * FROM t` | `SELECT t.id, t.name FROM t` |
| Expand `t.*` | `SELECT t.* FROM t` | `SELECT t.id, t.name FROM t` |
| Qualify WHERE | `WHERE col = 1` | `WHERE t.col = 1` |
| Qualify JOIN ON | `ON id = fk` | `ON a.id = b.fk` |
| CTE resolution | `WITH cte AS (...) SELECT col FROM cte` | `SELECT cte.col FROM cte` |
| Subquery in WHERE | `WHERE id IN (SELECT fk FROM t2)` | `WHERE t.id IN (SELECT t2.fk FROM t2)` |

Ambiguous columns (found in multiple sources) are left unqualified.
The pass resolves columns through CTEs, derived tables, and nested subqueries.

### Scope Analysis

Scope analysis builds a tree of `Scope` objects from a parsed SQL statement,
tracking sources, column references, and correlations at each level. It is the
foundation for qualify_columns, pushdown_predicates, annotate_types, and
column lineage.

```rust
use sqlglot_rust::{parse, Dialect, build_scope};
use sqlglot_rust::optimizer::scope_analysis::ScopeType;

let ast = parse(
    "WITH cte AS (SELECT id FROM t) \
     SELECT cte.id FROM cte WHERE EXISTS (SELECT 1 FROM s WHERE s.fk = cte.id)",
    Dialect::Ansi,
).unwrap();
let scope = build_scope(&ast);

// Root scope sees cte as a source
assert!(scope.sources.contains_key("cte"));
assert_eq!(scope.cte_scopes.len(), 1);
assert_eq!(scope.subquery_scopes.len(), 1);

// The EXISTS subquery is correlated — it references outer table cte
let sub = &scope.subquery_scopes[0];
assert!(sub.is_correlated);
assert!(sub.external_columns.iter().any(|c| c.table.as_deref() == Some("cte")));
```

| Concept | Description |
| --- | --- |
| **Root scope** | The outermost SELECT |
| **CTE scope** | Each `WITH name AS (...)` body |
| **Derived-table scope** | Each subquery in FROM |
| **Subquery scope** | Each scalar, EXISTS, or IN subquery |
| **Union scope** | Each branch of UNION / INTERSECT / EXCEPT |
| **Correlation** | A column in a child scope referencing an outer source |
| **`find_all_in_scope`** | Filter columns within a single scope (does not descend into children) |

You can walk the full scope tree:

```rust
scope.walk(&mut |s| {
    println!("{:?}: {} sources", s.scope_type, s.sources.len());
});
```

### Type Annotation (annotate_types)

The `annotate_types` pass infers SQL data types for every expression node in the AST
by combining schema column types, literal inference, function signatures, operator
coercion rules, and CASE/COALESCE resolution.

```rust
use sqlglot_rust::{parse, Dialect, annotate_types};
use sqlglot_rust::ast::DataType;
use sqlglot_rust::schema::{MappingSchema, Schema};

let mut schema = MappingSchema::new(Dialect::Ansi);
schema.add_table(&["employees"], vec![
    ("id".to_string(), DataType::Int),
    ("name".to_string(), DataType::Varchar(Some(100))),
    ("salary".to_string(), DataType::Double),
]).unwrap();

let stmt = parse("SELECT id, salary * 1.1, COUNT(*) FROM employees WHERE id > 5 GROUP BY id, salary", Dialect::Ansi).unwrap();
let ann = annotate_types(&stmt, &schema);

// Access types for individual expression nodes
if let sqlglot_rust::Statement::Select(sel) = &stmt {
    if let Some(sqlglot_rust::ast::SelectItem::Expr { expr, .. }) = sel.columns.first() {
        assert_eq!(ann.get_type(expr), Some(&DataType::Int)); // id → Int
    }
}
```

| Concept | Description |
| --- | --- |
| **Literal inference** | `42` → `Int`, `3.14` → `Double`, `'text'` → `Varchar` |
| **Column lookup** | Resolved from schema via table alias mapping |
| **Operator coercion** | `Int + Double` → `Double`, comparisons → `Boolean` |
| **Function signatures** | `COUNT(*)` → `BigInt`, `UPPER(x)` → `Varchar`, etc. |
| **CASE/COALESCE** | Common (widest) type across branches |
| **CAST** | Target data type |
| **UDFs** | Resolved from `schema.add_udf(name, return_type)` |

> **Note:** The returned `TypeAnnotations` uses raw pointer references, so the
> `Statement` must not be moved after annotation. Always access the statement
> by reference while using annotations.

### Column Lineage

Column lineage tracking traces how data flows from source columns through query
transformations to output columns. This is essential for data governance, impact
analysis, and understanding complex query pipelines.

```rust
use sqlglot_rust::{parse, Dialect};
use sqlglot_rust::optimizer::lineage::{lineage, lineage_sql, LineageConfig};
use sqlglot_rust::schema::MappingSchema;

// Set up schema
let mut schema = MappingSchema::new(Dialect::Ansi);
schema.add_table(&["employees"], vec![
    ("id".to_string(), sqlglot_rust::ast::DataType::Int),
    ("name".to_string(), sqlglot_rust::ast::DataType::Varchar(Some(100))),
    ("salary".to_string(), sqlglot_rust::ast::DataType::Double),
]).unwrap();

// Build lineage for a specific output column
let sql = "SELECT id, salary * 1.1 AS adjusted_salary FROM employees";
let config = LineageConfig::new(Dialect::Ansi);

let graph = lineage_sql("adjusted_salary", sql, &schema, &config).unwrap();

// The root node is the output column
assert_eq!(graph.node.name, "adjusted_salary");

// Get all source tables in the lineage
let sources = graph.source_tables();
assert!(sources.contains(&"employees".to_string()));

// Generate DOT format for visualization
let dot = graph.to_dot();
println!("{}", dot);
```

**Lineage Graph Output Example:**

For `SELECT a + b AS sum FROM t`, the lineage graph for column `sum` would be:

```text
LineageNode {
    name: "sum",
    downstream: [
        LineageNode { name: "a", source_name: Some("t") },
        LineageNode { name: "b", source_name: Some("t") },
    ]
}
```

| Feature | Description |
| --- | --- |
| **Simple columns** | `SELECT a FROM t` → traces `a` to `t.a` |
| **Aliased expressions** | `SELECT a + b AS sum FROM t` → traces `sum` to both `t.a` and `t.b` |
| **CTEs** | Traces through CTE definitions to underlying sources |
| **Derived tables** | Traces through subqueries in FROM clause |
| **JOINs** | Tracks columns from multiple joined tables |
| **UNIONs** | Creates branches for each UNION operand |
| **Functions** | `SUM(a)` → traces to the column arguments |
| **CASE expressions** | Traces all branches of CASE expressions |

**Visualization:**

```rust
// Generate DOT format (for Graphviz)
let dot = graph.to_dot();
// Output: digraph lineage { rankdir=BT; n0 [label="sum"]; n1 [label="t.a"]; ... }

// Generate Mermaid diagram
let mermaid = graph.to_mermaid();
// Output: flowchart BT\n  n0["sum"]\n  n1["t.a"]\n  ...
```

**Walking the Lineage:**

```rust
// Walk all nodes
graph.node.walk(&mut |node| {
    println!("Node: {} (source: {:?})", node.name, node.source_name);
});

// Iterate with standard iterator
for node in graph.node.iter() {
    println!("{}: depth {}", node.name, node.depth);
}

// Get leaf nodes (source columns)
let source_cols = graph.source_columns();
```

**External Sources:**

For multi-query lineage (e.g., analyzing views), you can provide external source
definitions:

```rust
use std::collections::HashMap;

let mut sources = HashMap::new();
sources.insert("view1".to_string(), "SELECT a, b FROM base_table".to_string());

let config = LineageConfig::new(Dialect::Ansi)
    .with_sources(sources)
    .with_trim_qualifiers(false); // Keep table qualifiers in output names

let graph = lineage_sql("a", "SELECT a FROM view1", &schema, &config).unwrap();
```

---

## Query Planner

The planner module generates a logical execution plan from a parsed (and optionally optimized) SQL AST. The plan is a directed acyclic graph (DAG) of steps that represents how a query should be executed.

### Building a Plan

```rust
use sqlglot_rust::{parse, Dialect};
use sqlglot_rust::planner::plan;

let ast = parse("SELECT a, b FROM t WHERE a > 1 ORDER BY b", Dialect::Ansi).unwrap();
let p = plan(&ast).unwrap();

println!("Plan has {} steps", p.len());
println!("{p}");
```

For best results, optimize the AST first:

```rust
use sqlglot_rust::{parse, Dialect};
use sqlglot_rust::optimizer::optimize;
use sqlglot_rust::planner::plan;

let ast = parse("SELECT a FROM t WHERE 1 = 1 AND a > 0", Dialect::Ansi).unwrap();
let optimized = optimize(ast).unwrap();
let p = plan(&optimized).unwrap();
```

### Inspecting Steps

Each step in the plan has a type, dependencies, and projections:

```rust
use sqlglot_rust::{parse, Dialect};
use sqlglot_rust::planner::{plan, Step};

let ast = parse(
    "SELECT a, SUM(b) FROM t JOIN u ON t.id = u.id GROUP BY a",
    Dialect::Ansi,
).unwrap();
let p = plan(&ast).unwrap();

for (i, step) in p.steps().iter().enumerate() {
    println!("Step {i}: {} (depends on {:?})", step.kind(), step.dependencies());
}
```

Step types: `Scan`, `Filter`, `Project`, `Aggregate`, `Sort`, `Join`, `Limit`, `SetOperation`, `Distinct`.

### Visualization

Plans can be rendered as Mermaid or Graphviz DOT:

```rust
use sqlglot_rust::{parse, Dialect};
use sqlglot_rust::planner::plan;

let ast = parse("SELECT a FROM t WHERE a > 1", Dialect::Ansi).unwrap();
let p = plan(&ast).unwrap();

// Mermaid flowchart (embed in Markdown docs)
println!("{}", p.to_mermaid());

// Graphviz DOT (render with `dot -Tpng`)
println!("{}", p.to_dot());
```

---

## Schema Management

The schema module provides a `Schema` trait and `MappingSchema` implementation
for registering table metadata and performing dialect-aware lookups. This is the
foundation for type annotation, column qualification, and lineage analysis.

### Creating a Schema

```rust
use sqlglot_rust::schema::{MappingSchema, Schema};
use sqlglot_rust::ast::DataType;
use sqlglot_rust::Dialect;

let mut schema = MappingSchema::new(Dialect::Postgres);

// Register a table with column definitions
schema.add_table(
    &["public", "users"],
    vec![
        ("id".to_string(), DataType::Int),
        ("name".to_string(), DataType::Varchar(Some(255))),
        ("email".to_string(), DataType::Text),
    ],
).unwrap();

// 3-level path: catalog.database.table
schema.add_table(
    &["prod", "analytics", "events"],
    vec![
        ("event_id".to_string(), DataType::BigInt),
        ("payload".to_string(), DataType::Json),
    ],
).unwrap();
```

### Querying the Schema

```rust
// Get column names in definition order
let cols = schema.column_names(&["public", "users"]).unwrap();
assert_eq!(cols, vec!["id", "name", "email"]);

// Look up a column's type
let dt = schema.get_column_type(&["public", "users"], "id").unwrap();
assert_eq!(dt, DataType::Int);

// Check column existence
assert!(schema.has_column(&["public", "users"], "email"));
assert!(!schema.has_column(&["public", "users"], "age"));

// Short-path lookup — searches all catalogs/databases
assert!(schema.has_column(&["users"], "id"));
```

### Dialect-Aware Normalization

Identifier lookups are normalized per the schema's dialect:

```rust
// Postgres: case-insensitive — these all match
let mut pg = MappingSchema::new(Dialect::Postgres);
pg.add_table(&["Users"], vec![("ID".to_string(), DataType::Int)]).unwrap();
assert!(pg.has_column(&["users"], "id"));   // matches
assert!(pg.has_column(&["USERS"], "ID"));   // matches

// BigQuery: case-sensitive — exact match required
let mut bq = MappingSchema::new(Dialect::BigQuery);
bq.add_table(&["Users"], vec![("ID".to_string(), DataType::Int)]).unwrap();
assert!(bq.has_column(&["Users"], "ID"));    // matches
assert!(!bq.has_column(&["users"], "id"));   // does not match
```

### Building Schemas from Maps

The `ensure_schema` helper builds a `MappingSchema` from nested `HashMap`s:

```rust
use std::collections::HashMap;
use sqlglot_rust::schema::ensure_schema;
use sqlglot_rust::ast::DataType;
use sqlglot_rust::Dialect;

let mut tables = HashMap::new();
let mut cols = HashMap::new();
cols.insert("id".to_string(), DataType::Int);
cols.insert("name".to_string(), DataType::Text);
tables.insert("users".to_string(), cols);

let schema = ensure_schema(tables, Dialect::Postgres);
assert!(schema.has_column(&["users"], "id"));
```

---

## Serialization (JSON Round-Tripping)

All AST types derive `serde::Serialize` and `serde::Deserialize`, so you can
convert any `Statement` to JSON and back:

```rust
use sqlglot_rust::{parse, generate, Dialect, Statement};

// Parse
let ast = parse("SELECT id, name FROM users WHERE active = TRUE", Dialect::Ansi).unwrap();

// Serialize to JSON
let json = serde_json::to_string_pretty(&ast).unwrap();
println!("{json}");

// Deserialize back
let restored: Statement = serde_json::from_str(&json).unwrap();
let sql = generate(&restored, Dialect::Ansi);
assert_eq!(sql, "SELECT id, name FROM users WHERE active = TRUE");
```

**Input:** `SELECT id, name FROM users WHERE active = TRUE`

**Output (JSON, abbreviated):**

```json
{
  "Select": {
    "ctes": [],
    "distinct": false,
    "top": null,
    "columns": [
      { "Expr": { "expr": { "Column": { "name": "id", "table": null } }, "alias": null } },
      { "Expr": { "expr": { "Column": { "name": "name", "table": null } }, "alias": null } }
    ],
    "from": { "source": { "Table": { "name": "users" } } },
    "where_clause": { "BinaryOp": { "left": { "Column": { "name": "active" } }, "op": "Eq", "right": { "Boolean": true } } },
    ...
  }
}
```

**Use cases:**

- Store parsed ASTs in databases or caches
- Send ASTs across service boundaries (microservices, WASM)
- Build language-server-style tooling
- Debugging — inspect the exact parse tree

---

## C/C++ FFI Bindings

sqlglot-rust ships with a C-compatible FFI layer so the library can be consumed
from C, C++, or any language that supports the C ABI.

### Building the FFI Libraries

```bash
# Build for the current host — produces header + static/shared libs
make ffi

# Or build for a specific cross-compilation target
make ffi-macos-arm64   # aarch64-apple-darwin
make ffi-linux-amd64   # x86_64-unknown-linux-gnu

# Build all four targets (macOS + Linux × arm64 + amd64)
make ffi-all
```

Output:

```text
target/ffi/
├── include/
│   └── sqlglot.h           # Auto-generated C header
└── lib/
    ├── libsqlglot_rust.a       # Static library
    └── libsqlglot_rust.dylib   # Shared library (or .so on Linux)
```

### C Example

```c
#include <stdio.h>
#include "sqlglot.h"

int main(void) {
    printf("version: %s\n", sqlglot_version());

    /* Transpile MySQL → PostgreSQL */
    char *result = sqlglot_transpile(
        "SELECT NOW(), IFNULL(a, b) FROM t LIMIT 10",
        "mysql",
        "postgres"
    );
    if (result) {
        printf("transpiled: %s\n", result);
        sqlglot_free(result);   /* MUST free every non-NULL return */
    }

    /* Parse SQL to JSON AST */
    char *json = sqlglot_parse("SELECT a FROM t", "ansi");
    if (json) {
        printf("AST: %s\n", json);

        char *pretty = sqlglot_generate_pretty(json, "ansi");
        if (pretty) {
            printf("formatted: %s\n", pretty);
            sqlglot_free(pretty);
        }

        sqlglot_free(json);
    }

    return 0;
}
```

Build and run:

```bash
# macOS
gcc example.c -Itarget/ffi/include -Ltarget/release -lsqlglot_rust -o example
./example

# Linux
gcc example.c -Itarget/ffi/include -Ltarget/release -lsqlglot_rust -lpthread -ldl -lm -o example
LD_LIBRARY_PATH=target/release ./example
```

### C++ Example with RAII

Use a `unique_ptr` with a custom deleter so strings are freed automatically:

```cpp
#include <cstdio>
#include <memory>
#include <optional>
#include <string>

extern "C" {
#include "sqlglot.h"
}

struct SqlglotDeleter {
    void operator()(char *p) const noexcept { sqlglot_free(p); }
};
using SqlglotString = std::unique_ptr<char, SqlglotDeleter>;

std::optional<std::string> transpile(const char *sql,
                                     const char *from,
                                     const char *to) {
    SqlglotString result(sqlglot_transpile(sql, from, to));
    if (!result) return std::nullopt;
    return std::string(result.get());
}

int main() {
    auto sql = transpile("SELECT * FROM t LIMIT 5", "mysql", "tsql");
    if (sql) std::printf("result: %s\n", sql->c_str());
    return 0;
}
```

Build:

```bash
g++ -std=c++17 example.cpp -Itarget/ffi/include -Ltarget/release -lsqlglot_rust -o example
```

### Linking

**Static linking** (no runtime dependency):

```bash
gcc example.c -Itarget/ffi/include target/ffi/lib/libsqlglot_rust.a -lpthread -ldl -lm -o example
```

**Dynamic linking** (smaller binary, requires `.so`/`.dylib` at runtime):

```bash
gcc example.c -Itarget/ffi/include -Ltarget/ffi/lib -lsqlglot_rust -o example
LD_LIBRARY_PATH=target/ffi/lib ./example    # Linux
DYLD_LIBRARY_PATH=target/ffi/lib ./example  # macOS
```

See [`examples/ffi_example.c`](../examples/ffi_example.c) and
[`examples/ffi_example.cpp`](../examples/ffi_example.cpp) for complete working
examples.

---

## Error Handling

All fallible operations return `Result<T, SqlglotError>`. Errors implement
`Display` and `std::error::Error` (via `thiserror`), making them compatible
with `anyhow`, `eyre`, and the `?` operator.

```rust
use sqlglot_rust::{parse, Dialect, SqlglotError};

match parse("SELECT FROM", Dialect::Ansi) {
    Ok(stmt) => println!("Parsed: {:?}", stmt),
    Err(e)   => eprintln!("Error: {e}"),
}
```

**Input:** `SELECT FROM` (missing select list)

**Output:**

```text
Error: Parser error: ...
```

### Matching Error Variants

```rust
use sqlglot_rust::{parse, Dialect, SqlglotError};

let err = parse("SELECT @@@", Dialect::Ansi).unwrap_err();

match &err {
    SqlglotError::TokenizerError { message, position } => {
        println!("Tokenizer error at byte {position}: {message}");
    }
    SqlglotError::ParserError { message } => {
        println!("Parse error: {message}");
    }
    SqlglotError::UnexpectedToken { token } => {
        println!("Unexpected token: {:?}", token);
    }
    SqlglotError::UnsupportedDialectFeature(msg) => {
        println!("Unsupported: {msg}");
    }
    SqlglotError::Internal(msg) => {
        println!("Internal error: {msg}");
    }
}
```

### Using with `anyhow`

```rust
use anyhow::Result;
use sqlglot_rust::{parse, generate, Dialect};

fn process_query(sql: &str) -> Result<String> {
    let ast = parse(sql, Dialect::Ansi)?;    // SqlglotError → anyhow::Error
    Ok(generate(&ast, Dialect::Ansi))
}
```

---

## SBOM Generation

The project supports generating a Software Bill of Materials (SBOM) in
[SPDX](https://spdx.dev/) 2.3 JSON format. This documents every dependency
shipped with the library — useful for license compliance, supply-chain
security audits, and vulnerability tracking.

### Prerequisites

Install [`cargo-sbom`](https://crates.io/crates/cargo-sbom) (one-time setup):

```bash
cargo install cargo-sbom
```

> `cargo-sbom` is a standalone CLI tool. It is **not** a project dependency and
> does not appear in the generated SBOM.

### Generating the SBOM

Use the provided Makefile target:

```bash
make sbom
```

Or run the command directly:

```bash
cargo sbom --output-format spdx_json_2_3 > target/sbom/sqlglot-rust.spdx.json
```

**Output location:** `target/sbom/sqlglot-rust.spdx.json`

### What the SBOM Contains

The generated SPDX document includes:

| Field | Description |
| --- | --- |
| Package name and version | For every direct and transitive dependency |
| License (concluded) | SPDX license expression per package |
| Download location | Registry URL for the crate |
| Package URL (PURL) | `pkg:cargo/<name>@<version>` identifier |
| File checksums | SHA-1 hash of `Cargo.lock` |
| Creation info | Timestamp and tool version |

### Example: Inspecting the SBOM

List all packages:

```bash
jq -r '.packages[].name' target/sbom/sqlglot-rust.spdx.json
```

Count dependencies:

```bash
jq '.packages | length' target/sbom/sqlglot-rust.spdx.json
```

Extract licenses:

```bash
jq -r '.packages[] | "\(.name): \(.licenseConcluded)"' target/sbom/sqlglot-rust.spdx.json
```

---

## Updating the Version

The project provides a Makefile target to update the version consistently across
all configuration and documentation files:

```bash
make bump-version VERSION=1.0.0
```

### What It Updates

| File | Field |
| --- | --- |
| `Cargo.toml` | `version = "..."` |
| `README.md` | `sqlglot-rust = "..."` dependency snippet |
| `docs/installation.md` | `sqlglot-rust = "..."` dependency snippet |
| `Cargo.lock` | Regenerated via `cargo generate-lockfile` |

### Guidelines

- Always use a **full semantic version**: `MAJOR.MINOR.PATCH` (e.g. `1.0.0`, `0.10.1`).
- Run `make bump-version` **before** committing a release so all references stay in sync.
- The `VERSION` parameter is required — omitting it produces an error with usage instructions.

### Example

```bash
$ make bump-version VERSION=1.2.0
Bumping version to 1.2.0...
Version updated to 1.2.0
```

---

## Next Steps

- **[Reference](reference.md)** — Complete API tables, type catalogs, all 30
  dialects, `Expr` variants, `DataType` variants, and operator enums.
- **[Installation](installation.md)** — Dependency setup and verification.
