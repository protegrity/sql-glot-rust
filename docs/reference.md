# API Reference

Complete type and function reference for **sqlglot-rust**.

> **See also:** [Installation](installation.md) · [Developer Guide](developer-guide.md)

---

## Table of Contents

- [Top-Level Functions](#top-level-functions)
- [Expression Builder API](#expression-builder-api)
  - [Factory Functions](#factory-functions)
  - [Comparison Functions](#comparison-functions)
  - [Arithmetic Functions](#arithmetic-functions)
  - [SelectBuilder](#selectbuilder)
  - [ConditionBuilder](#conditionbuilder)
- [Statement Enum](#statement-enum)
  - [SelectStatement](#selectstatement)
  - [InsertStatement](#insertstatement)
  - [UpdateStatement](#updatestatement)
  - [DeleteStatement](#deletestatement)
  - [MergeStatement](#mergestatement)
  - [CreateTableStatement](#createtablestatement)
  - [AlterTableStatement](#altertablestatement)
  - [Other Statements](#other-statements)
- [Expr Enum](#expr-enum)
  - [Expr Variants](#expr-variants)
  - [Expr Methods](#expr-methods)
- [Supporting Types](#supporting-types)
  - [SelectItem](#selectitem)
  - [FromClause / TableSource](#fromclause--tablesource)
  - [TableRef](#tableref)
  - [JoinClause / JoinType](#joinclause--jointype)
  - [OrderByItem](#orderbyitem)
  - [Cte](#cte)
  - [WindowSpec / WindowFrame](#windowspec--windowframe)
  - [InsertSource / OnConflict](#insertsource--onconflict)
  - [ColumnDef](#columndef)
  - [TableConstraint](#tableconstraint)
  - [AlterTableAction](#altertableaction)
- [Operators](#operators)
  - [BinaryOperator](#binaryoperator)
  - [UnaryOperator](#unaryoperator)
- [DataType Enum](#datatype-enum)
- [DateTimeField Enum](#datetimefield-enum)
- [QuoteStyle Enum](#quotestyle-enum)
- [CommentType Enum](#commenttype-enum)
- [Dialect Enum](#dialect-enum)
  - [Dialect List](#dialect-list)
  - [Dialect Methods](#dialect-methods)
  - [Dialect Aliases (from_str)](#dialect-aliases-from_str)
  - [Function Mapping Matrix](#function-mapping-matrix)
  - [Data Type Mapping Matrix](#data-type-mapping-matrix)
  - [Identifier Quote Styles by Dialect](#identifier-quote-styles-by-dialect)
  - [Time Format Mapping](#time-format-mapping)
- [Custom Dialect Plugins](#custom-dialect-plugins)
  - [DialectPlugin Trait](#dialectplugin-trait)
  - [DialectRegistry](#dialectregistry)
  - [DialectRef Enum](#dialectref-enum)
  - [Plugin-Aware Functions](#plugin-aware-functions)
- [Error Types](#error-types)
- [Free Functions (ast module)](#free-functions-ast-module)
- [Schema System](#schema-system)
  - [Schema Trait](#schema-trait)
  - [MappingSchema](#mappingschema)
  - [Helper Functions](#helper-functions)
  - [SchemaError](#schemaerror)
- [Optimizer](#optimizer)
  - [Scope Analysis](#scope-analysis)
  - [Annotate Types](#annotate-types)
  - [Column Lineage](#column-lineage)
- [Query Planner](#query-planner)
  - [Plan / StepId](#plan--stepid)
  - [Step Enum](#step-enum)
  - [Projection](#projection)
  - [Visualization (Mermaid / DOT)](#visualization-mermaid--dot)
- [AST Diff](#ast-diff)
  - [ChangeAction Enum](#changeaction-enum)
  - [AstNode Enum](#astnode-enum)
- [C/C++ FFI API](#cc-ffi-api)
  - [Functions](#functions)
  - [Memory Management](#memory-management)
  - [Build Targets](#build-targets)
  - [Output Artefacts](#output-artefacts)
- [SQL Execution Engine](#sql-execution-engine)
  - [Value Enum](#value-enum)
  - [Table / Tables](#table--tables)
  - [ResultSet](#resultset)
  - [execute / execute_statement](#execute--execute_statement)

---

## Top-Level Functions

All re-exported from the crate root (`use sqlglot_rust::*`).

| Function | Signature | Returns | Description |
| --- | --- | --- | --- |
| `parse` | `(sql: &str, dialect: Dialect) -> Result<Statement>` | `Statement` | Parse a single SQL statement into an AST |
| `parse_statements` | `(sql: &str, dialect: Dialect) -> Result<Vec<Statement>>` | `Vec<Statement>` | Parse multiple semicolon-separated statements |
| `generate` | `(stmt: &Statement, dialect: Dialect) -> String` | `String` | Generate compact single-line SQL |
| `generate_pretty` | `(stmt: &Statement, dialect: Dialect) -> String` | `String` | Generate formatted SQL with indentation |
| `transpile` | `(sql: &str, read: Dialect, write: Dialect) -> Result<String>` | `String` | Parse → transform → generate in one call |
| `transpile_statements` | `(sql: &str, read: Dialect, write: Dialect) -> Result<Vec<String>>` | `Vec<String>` | Transpile multiple statements |
| `parse_with_comments` | `(sql: &str, dialect: Dialect) -> Result<Statement>` | `Statement` | Parse a single statement preserving SQL comments on AST nodes |
| `parse_statements_with_comments` | `(sql: &str, dialect: Dialect) -> Result<Vec<Statement>>` | `Vec<Statement>` | Parse multiple statements preserving SQL comments |
| `transpile_with_comments` | `(sql: &str, read: Dialect, write: Dialect) -> Result<String>` | `String` | Transpile preserving SQL comments |

**`parse_statements`** is accessed via `sqlglot_rust::parser::parse_statements`.

### Examples

```rust
use sqlglot_rust::{parse, generate, generate_pretty, transpile, Dialect};

// parse + generate roundtrip
let ast = parse("SELECT 1", Dialect::Ansi).unwrap();
assert_eq!(generate(&ast, Dialect::Ansi), "SELECT 1");

// pretty output
let pretty = generate_pretty(&ast, Dialect::Ansi);
// => "SELECT\n  1"

// transpile
let out = transpile("SELECT NOW()", Dialect::Postgres, Dialect::Tsql).unwrap();
assert_eq!(out, "SELECT GETDATE()");
```

---

## Expression Builder API

Fluent API for programmatic SQL construction. All functions are re-exported from `sqlglot_rust::builder`.

### Factory Functions

| Function | Signature | Returns | Description |
| --- | --- | --- | --- |
| `column` | `(name: &str, table: Option<&str>) -> Expr` | `Expr::Column` | Create column reference |
| `table` | `(name: &str, schema: Option<&str>) -> TableRef` | `TableRef` | Create table reference |
| `table_full` | `(name: &str, schema: Option<&str>, catalog: Option<&str>) -> TableRef` | `TableRef` | Create fully-qualified table reference |
| `literal` | `<T: ToString>(value: T) -> Expr` | `Expr::Number` | Create numeric literal |
| `string_literal` | `(value: &str) -> Expr` | `Expr::StringLiteral` | Create string literal |
| `boolean` | `(value: bool) -> Expr` | `Expr::Boolean` | Create boolean literal |
| `null` | `() -> Expr` | `Expr::Null` | Create NULL literal |
| `cast` | `(expr: Expr, data_type: DataType) -> Expr` | `Expr::Cast` | Create CAST expression |
| `func` | `(name: &str, args: Vec<Expr>) -> Expr` | `Expr::Function` | Create function call |
| `func_distinct` | `(name: &str, args: Vec<Expr>) -> Expr` | `Expr::Function` | Create function call with DISTINCT |
| `star` | `() -> Expr` | `Expr::Star` | Create wildcard (*) |
| `qualified_star` | `(table: &str) -> Expr` | `Expr::QualifiedWildcard` | Create table.* wildcard |
| `subquery` | `(stmt: Statement) -> Expr` | `Expr::Subquery` | Create scalar subquery |
| `exists` | `(stmt: Statement, negated: bool) -> Expr` | `Expr::Exists` | Create EXISTS expression |
| `alias` | `(expr: Expr, name: &str) -> Expr` | `Expr::Alias` | Create aliased expression |
| `not` | `(expr: Expr) -> Expr` | `Expr::UnaryOp` | Negate expression with NOT |
| `and_all` | `<I: IntoIterator<Item=Expr>>(conditions: I) -> Option<Expr>` | `Option<Expr>` | Combine with AND |
| `or_all` | `<I: IntoIterator<Item=Expr>>(conditions: I) -> Option<Expr>` | `Option<Expr>` | Combine with OR |

### Comparison Functions

| Function | Signature | Description |
| --- | --- | --- |
| `eq` | `(left: Expr, right: Expr) -> Expr` | Equality (=) |
| `neq` | `(left: Expr, right: Expr) -> Expr` | Inequality (<>) |
| `lt` | `(left: Expr, right: Expr) -> Expr` | Less than (<) |
| `lte` | `(left: Expr, right: Expr) -> Expr` | Less than or equal (<=) |
| `gt` | `(left: Expr, right: Expr) -> Expr` | Greater than (>) |
| `gte` | `(left: Expr, right: Expr) -> Expr` | Greater than or equal (>=) |
| `is_null` | `(expr: Expr) -> Expr` | IS NULL check |
| `is_not_null` | `(expr: Expr) -> Expr` | IS NOT NULL check |
| `between` | `(expr: Expr, low: Expr, high: Expr) -> Expr` | BETWEEN expression |
| `in_list` | `(expr: Expr, list: Vec<Expr>) -> Expr` | IN list expression |
| `not_in_list` | `(expr: Expr, list: Vec<Expr>) -> Expr` | NOT IN list expression |
| `in_subquery` | `(expr: Expr, query: Statement) -> Expr` | IN subquery expression |
| `like` | `(expr: Expr, pattern: Expr) -> Expr` | LIKE expression |

### Arithmetic Functions

| Function | Signature | Description |
| --- | --- | --- |
| `add` | `(left: Expr, right: Expr) -> Expr` | Addition (+) |
| `sub` | `(left: Expr, right: Expr) -> Expr` | Subtraction (-) |
| `mul` | `(left: Expr, right: Expr) -> Expr` | Multiplication (*) |
| `div` | `(left: Expr, right: Expr) -> Expr` | Division (/) |

### Parse Functions

| Function | Signature | Description |
| --- | --- | --- |
| `parse_expr` | `(sql: &str) -> Option<Expr>` | Parse expression (ANSI dialect) |
| `parse_expr_dialect` | `(sql: &str, dialect: Dialect) -> Option<Expr>` | Parse expression with dialect |
| `parse_condition` | `(sql: &str) -> Option<Expr>` | Parse WHERE condition (ANSI) |
| `parse_condition_dialect` | `(sql: &str, dialect: Dialect) -> Option<Expr>` | Parse condition with dialect |

### SelectBuilder

```rust
pub struct SelectBuilder { ... }
```

Create via `select(&[&str])`, `select_all()`, or `select_distinct(&[&str])`.

| Method | Returns | Description |
| --- | --- | --- |
| `dialect(Dialect)` | `Self` | Set parsing dialect |
| `columns(&[&str])` | `Self` | Add columns to SELECT |
| `column_expr(Expr, Option<&str>)` | `Self` | Add expression with alias |
| `all()` | `Self` | Add * wildcard |
| `all_from(&str)` | `Self` | Add table.* wildcard |
| `distinct()` | `Self` | Enable DISTINCT |
| `from(&str)` | `Self` | Set FROM table |
| `from_table(TableRef)` | `Self` | Set FROM with TableRef |
| `from_subquery(Statement, &str)` | `Self` | Set FROM subquery |
| `join(&str, &str)` | `Self` | INNER JOIN |
| `left_join(&str, &str)` | `Self` | LEFT JOIN |
| `right_join(&str, &str)` | `Self` | RIGHT JOIN |
| `full_join(&str, &str)` | `Self` | FULL JOIN |
| `cross_join(&str)` | `Self` | CROSS JOIN |
| `join_using(&str, &[&str], JoinType)` | `Self` | JOIN with USING |
| `join_subquery(Statement, &str, &str, JoinType)` | `Self` | JOIN subquery |
| `where_clause(&str)` | `Self` | Set WHERE |
| `where_expr(Expr)` | `Self` | Set WHERE from Expr |
| `and_where(&str)` | `Self` | AND to WHERE |
| `or_where(&str)` | `Self` | OR to WHERE |
| `group_by(&[&str])` | `Self` | Set GROUP BY |
| `add_group_by(&str)` | `Self` | Add to GROUP BY |
| `having(&str)` | `Self` | Set HAVING |
| `order_by(&[&str])` | `Self` | Set ORDER BY |
| `add_order_by(&str)` | `Self` | Add to ORDER BY |
| `add_order_by_expr(Expr, bool, Option<bool>)` | `Self` | Add ORDER BY with options |
| `limit(i64)` | `Self` | Set LIMIT |
| `limit_expr(Expr)` | `Self` | Set LIMIT from Expr |
| `offset(i64)` | `Self` | Set OFFSET |
| `offset_expr(Expr)` | `Self` | Set OFFSET from Expr |
| `top(i64)` | `Self` | Set TOP (T-SQL) |
| `qualify(&str)` | `Self` | Set QUALIFY |
| `build()` | `Statement` | Build final Statement |
| `build_select()` | `SelectStatement` | Build SelectStatement |

### ConditionBuilder

```rust
pub struct ConditionBuilder { ... }
```

Create via `condition(&str)` or `condition_dialect(&str, Dialect)`.

| Method | Returns | Description |
| --- | --- | --- |
| `and(&str)` | `Self` | Add AND condition |
| `and_expr(Option<Expr>)` | `Self` | Add AND with Expr |
| `or(&str)` | `Self` | Add OR condition |
| `or_expr(Option<Expr>)` | `Self` | Add OR with Expr |
| `not()` | `Self` | Negate entire condition |
| `build()` | `Option<Expr>` | Build final Expr |

### SelectStatement Mutation Methods

Methods added to `SelectStatement` for in-place modification:

| Method | Signature | Description |
| --- | --- | --- |
| `add_select` | `(&mut self, expr: &str)` | Add column to SELECT |
| `add_select_dialect` | `(&mut self, expr: &str, dialect: Dialect)` | Add with dialect |
| `add_select_expr` | `(&mut self, expr: Expr, alias: Option<&str>)` | Add Expr to SELECT |
| `add_where` | `(&mut self, condition: &str)` | AND to WHERE |
| `add_where_dialect` | `(&mut self, condition: &str, dialect: Dialect)` | AND with dialect |
| `add_where_expr` | `(&mut self, expr: Expr)` | AND Expr to WHERE |
| `add_join` | `(&mut self, table: &str, on: &str, join_type: JoinType)` | Add JOIN |
| `add_join_dialect` | `(&mut self, table: &str, on: &str, join_type: JoinType, dialect: Dialect)` | Add with dialect |
| `add_join_subquery` | `(&mut self, query: Statement, alias: &str, on: &str, join_type: JoinType)` | JOIN subquery |
| `as_subquery` | `(self, alias: &str) -> TableSource` | Wrap as subquery |

---

## Statement Enum

```rust
pub enum Statement {
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    Merge(MergeStatement),
    CreateTable(CreateTableStatement),
    DropTable(DropTableStatement),
    SetOperation(SetOperationStatement),
    AlterTable(AlterTableStatement),
    CreateView(CreateViewStatement),
    DropView(DropViewStatement),
    Truncate(TruncateStatement),
    Transaction(TransactionStatement),
    Explain(ExplainStatement),
    Use(UseStatement),
    Expression(Expr),
}
```

Derives: `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`.

### SelectStatement

```rust
pub struct SelectStatement {
    pub comments: Vec<String>,
    pub ctes: Vec<Cte>,
    pub distinct: bool,
    pub top: Option<Box<Expr>>,
    pub columns: Vec<SelectItem>,
    pub from: Option<FromClause>,
    pub joins: Vec<JoinClause>,
    pub where_clause: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub order_by: Vec<OrderByItem>,
    pub limit: Option<Expr>,
    pub offset: Option<Expr>,
    pub fetch_first: Option<Expr>,
    pub qualify: Option<Expr>,
    pub window_definitions: Vec<WindowDefinition>,
}
```

| Field | Type | Description |
| --- | --- | --- |
| `comments` | `Vec<String>` | Attached SQL comments (populated by `parse_with_comments`) |
| `ctes` | `Vec<Cte>` | Common Table Expressions (`WITH` clause) |
| `distinct` | `bool` | `SELECT DISTINCT` flag |
| `top` | `Option<Box<Expr>>` | T-SQL `TOP N` expression |
| `columns` | `Vec<SelectItem>` | Select list items |
| `from` | `Option<FromClause>` | Primary `FROM` source |
| `joins` | `Vec<JoinClause>` | All `JOIN` clauses |
| `where_clause` | `Option<Expr>` | `WHERE` predicate |
| `group_by` | `Vec<Expr>` | `GROUP BY` expressions |
| `having` | `Option<Expr>` | `HAVING` predicate |
| `order_by` | `Vec<OrderByItem>` | `ORDER BY` items |
| `limit` | `Option<Expr>` | `LIMIT` value |
| `offset` | `Option<Expr>` | `OFFSET` value |
| `fetch_first` | `Option<Expr>` | Oracle `FETCH FIRST N ROWS ONLY` |
| `qualify` | `Option<Expr>` | BigQuery/Snowflake `QUALIFY` clause |
| `window_definitions` | `Vec<WindowDefinition>` | Named `WINDOW` specs |

**Example:**

```rust
use sqlglot_rust::{parse, Dialect, Statement};

let stmt = parse(
    "WITH cte AS (SELECT 1 AS x) SELECT DISTINCT x FROM cte WHERE x > 0 ORDER BY x LIMIT 5",
    Dialect::Ansi,
).unwrap();

if let Statement::Select(s) = &stmt {
    assert_eq!(s.ctes.len(), 1);         // 1 CTE
    assert!(s.distinct);                   // DISTINCT
    assert_eq!(s.columns.len(), 1);        // 1 column
    assert!(s.where_clause.is_some());     // has WHERE
    assert_eq!(s.order_by.len(), 1);       // 1 ORDER BY
    assert!(s.limit.is_some());            // has LIMIT
}
```

### InsertStatement

```rust
pub struct InsertStatement {
    pub table: TableRef,
    pub columns: Vec<String>,
    pub source: InsertSource,
    pub on_conflict: Option<OnConflict>,
    pub returning: Vec<SelectItem>,
}
```

| Field | Description |
| --- | --- |
| `table` | Target table |
| `columns` | Column list (may be empty) |
| `source` | `Values(rows)`, `Query(select)`, or `Default` |
| `on_conflict` | `ON CONFLICT` / `ON DUPLICATE KEY` clause |
| `returning` | `RETURNING` clause items |

**Example:**

```rust
let stmt = parse("INSERT INTO users (name, age) VALUES ('Alice', 30)", Dialect::Ansi).unwrap();
// => Statement::Insert(InsertStatement { table: "users", columns: ["name","age"], source: Values(...) })
```

### UpdateStatement

```rust
pub struct UpdateStatement {
    pub table: TableRef,
    pub assignments: Vec<(String, Expr)>,
    pub from: Option<FromClause>,
    pub where_clause: Option<Expr>,
    pub returning: Vec<SelectItem>,
}
```

**Example:**

```rust
let stmt = parse("UPDATE users SET name = 'Bob' WHERE id = 1", Dialect::Ansi).unwrap();
// => Statement::Update(UpdateStatement { table: "users", assignments: [("name", StringLiteral("Bob"))], ... })
```

### DeleteStatement

```rust
pub struct DeleteStatement {
    pub table: TableRef,
    pub using: Option<FromClause>,
    pub where_clause: Option<Expr>,
    pub returning: Vec<SelectItem>,
}
```

**Example:**

```rust
let stmt = parse("DELETE FROM users WHERE id = 1", Dialect::Ansi).unwrap();
// => Statement::Delete(DeleteStatement { table: "users", where_clause: Some(...) })
```

### MergeStatement

```rust
pub struct MergeStatement {
    pub target: TableRef,
    pub source: TableSource,
    pub on: Expr,
    pub clauses: Vec<MergeClause>,
    pub output: Vec<SelectItem>,
}

pub struct MergeClause {
    pub kind: MergeClauseKind,
    pub condition: Option<Expr>,
    pub action: MergeAction,
}

pub enum MergeClauseKind {
    Matched,
    NotMatched,
    NotMatchedBySource,
}

pub enum MergeAction {
    Update(Vec<(String, Expr)>),
    Insert { columns: Vec<String>, values: Vec<Expr> },
    InsertRow,
    Delete,
}
```

| Field | Description |
| --- | --- |
| `target` | Target table for the MERGE |
| `source` | Source table/subquery (`USING` clause) |
| `on` | Join condition (`ON` clause) |
| `clauses` | One or more `WHEN MATCHED` / `WHEN NOT MATCHED` clauses |
| `output` | T-SQL `OUTPUT` clause items |

**Supported dialect extensions:**

- `WHEN NOT MATCHED BY SOURCE` (T-SQL)
- `INSERT ROW` (BigQuery)
- `OUTPUT` clause (T-SQL)
- Multiple `WHEN` clauses with `AND` conditions

**Example:**

```rust
use sqlglot_rust::{parse, generate, transpile, Dialect};

// Parse a standard MERGE statement
let stmt = parse(
    "MERGE INTO target AS t USING source AS s ON t.id = s.id \
     WHEN MATCHED THEN UPDATE SET t.name = s.name \
     WHEN NOT MATCHED THEN INSERT (id, name) VALUES (s.id, s.name)",
    Dialect::Ansi,
).unwrap();

// Roundtrip back to SQL
let sql = generate(&stmt, Dialect::Ansi);

// Transpile to another dialect
let snowflake_sql = transpile(
    "MERGE INTO dst USING src ON dst.id = src.id WHEN MATCHED THEN DELETE",
    Dialect::Ansi,
    Dialect::Snowflake,
).unwrap();
```

### CreateTableStatement

```rust
pub struct CreateTableStatement {
    pub if_not_exists: bool,
    pub temporary: bool,
    pub table: TableRef,
    pub columns: Vec<ColumnDef>,
    pub constraints: Vec<TableConstraint>,
    pub as_select: Option<Box<Statement>>,
}
```

**Example:**

```rust
let stmt = parse(
    "CREATE TABLE IF NOT EXISTS items (id INT NOT NULL, name VARCHAR(100), price DECIMAL(10, 2))",
    Dialect::Ansi,
).unwrap();
// => Statement::CreateTable(CreateTableStatement { if_not_exists: true, columns: [3 ColumnDefs], ... })
```

### AlterTableStatement

```rust
pub struct AlterTableStatement {
    pub table: TableRef,
    pub actions: Vec<AlterTableAction>,
}
```

### Other Statements

| Type | SQL |
| --- | --- |
| `DropTableStatement` | `DROP TABLE [IF EXISTS] name [CASCADE]` |
| `SetOperationStatement` | `query UNION [ALL] query`, `INTERSECT`, `EXCEPT` |
| `CreateViewStatement` | `CREATE [OR REPLACE] [MATERIALIZED] VIEW ...` |
| `DropViewStatement` | `DROP [MATERIALIZED] VIEW [IF EXISTS] name` |
| `TruncateStatement` | `TRUNCATE TABLE name` |
| `TransactionStatement` | `BEGIN`, `COMMIT`, `ROLLBACK`, `SAVEPOINT x`, `RELEASE SAVEPOINT x`, `ROLLBACK TO x` |
| `ExplainStatement` | `EXPLAIN [ANALYZE] statement` |
| `UseStatement` | `USE database_name` |

---

## Expr Enum

### Expr Variants

| Variant | Fields | SQL Example |
| --- | --- | --- |
| `Column` | `{ name, table?, quote_style, table_quote_style }` | `t.col`, `"col"` |
| `Number` | `(String)` | `42`, `3.14` |
| `StringLiteral` | `(String)` | `'hello'` |
| `Boolean` | `(bool)` | `TRUE`, `FALSE` |
| `Null` | — | `NULL` |
| `BinaryOp` | `{ left, op, right }` | `a + b`, `x AND y` |
| `UnaryOp` | `{ op, expr }` | `-x`, `NOT a`, `~b` |
| `Function` | `{ name, args, distinct, filter?, over? }` | `COUNT(DISTINCT x)`, `SUM(a) OVER (...)` |
| `TypedFunction` | `{ func: TypedFunction, filter?, over? }` | `SUBSTRING(x, 1, 3)`, `DATE_TRUNC('month', d)` |
| `Between` | `{ expr, low, high, negated }` | `x BETWEEN 1 AND 10` |
| `InList` | `{ expr, list, negated }` | `x IN (1, 2, 3)` |
| `InSubquery` | `{ expr, subquery, negated }` | `x IN (SELECT ...)` |
| `AnyOp` | `{ expr, op, right }` | `x = ANY(ARRAY[1, 2])`, `x = ANY(SELECT ...)` |
| `AllOp` | `{ expr, op, right }` | `x > ALL(SELECT ...)` |
| `IsNull` | `{ expr, negated }` | `x IS NULL`, `x IS NOT NULL` |
| `Like` | `{ expr, pattern, negated, escape? }` | `name LIKE '%test%'` |
| `ILike` | `{ expr, pattern, negated, escape? }` | `name ILIKE '%test%'` |
| `Case` | `{ operand?, when_clauses, else_clause? }` | `CASE WHEN ... THEN ... END` |
| `Nested` | `(Box<Expr>)` | `(a + b)` |
| `Wildcard` | — | `*` (in function args) |
| `Star` | — | `*` (select list) |
| `QualifiedWildcard` | `{ table }` | `t.*` |
| `Subquery` | `(Box<Statement>)` | `(SELECT 1)` |
| `Exists` | `{ subquery, negated }` | `EXISTS (SELECT ...)` |
| `Cast` | `{ expr, data_type }` | `CAST(x AS INT)` |
| `TryCast` | `{ expr, data_type }` | `TRY_CAST(x AS INT)` |
| `Extract` | `{ field, expr }` | `EXTRACT(YEAR FROM d)` |
| `Interval` | `{ value, unit? }` | `INTERVAL '1' DAY` |
| `ArrayLiteral` | `(Vec<Expr>)` | `ARRAY[1, 2, 3]` |
| `Tuple` | `(Vec<Expr>)` | `(1, 'a', true)` |
| `Coalesce` | `(Vec<Expr>)` | `COALESCE(a, b, c)` |
| `If` | `{ condition, true_val, false_val? }` | `IF(a > 0, a, 0)` |
| `NullIf` | `{ expr, else }` | `NULLIF(a, b)` |
| `Collate` | `{ expr, collation }` | `col COLLATE utf8` |
| `Parameter` | `(String)` | `$1`, `?`, `:name` |
| `TypeExpr` | `(DataType)` | Used in DDL contexts |
| `Alias` | `{ expr, name }` | `expr AS alias` |
| `ArrayIndex` | `{ expr, index }` | `arr[0]` |
| `JsonAccess` | `{ expr, path, as_text }` | `data->'key'`, `data->>'key'` |
| `Lambda` | `{ params, body }` | `x -> x + 1` |
| `Default` | — | `DEFAULT` |
| `Cube` | `{ exprs }` | `CUBE(a, b)` |
| `Rollup` | `{ exprs }` | `ROLLUP(a, b)` |
| `GroupingSets` | `{ sets }` | `GROUPING SETS((a, b), (a), ())` |
| `Commented` | `{ expr, comments }` | Expression with attached SQL comments |

### Expr Methods

| Method | Signature | Returns | Description |
| --- | --- | --- | --- |
| `walk` | `(&self, visitor: &mut F)` where `F: FnMut(&Expr) -> bool` | `()` | Depth-first traversal. Visitor returns `true` to recurse. |
| `find` | `(&self, predicate: &F) -> Option<&Expr>` where `F: Fn(&Expr) -> bool` | `Option<&Expr>` | First matching node |
| `find_all` | `(&self, predicate: &F) -> Vec<&Expr>` where `F: Fn(&Expr) -> bool` | `Vec<&Expr>` | All matching nodes |
| `transform` | `(self, func: &F) -> Expr` where `F: Fn(Expr) -> Expr` | `Expr` | Bottom-up tree rewrite (consumes self) |
| `is_column` | `(&self) -> bool` | `bool` | `true` if `Column` variant |
| `is_literal` | `(&self) -> bool` | `bool` | `true` if `Number`, `StringLiteral`, or `Boolean` |
| `sql` | `(&self) -> String` | `String` | Quick SQL output (ANSI dialect) |

**Examples:**

```rust
use sqlglot_rust::Expr;

// sql()
assert_eq!(Expr::Number("42".into()).sql(), "42");
assert_eq!(Expr::StringLiteral("hello".into()).sql(), "'hello'");
assert_eq!(Expr::Boolean(true).sql(), "TRUE");
assert_eq!(Expr::Null.sql(), "NULL");

// is_column / is_literal
assert!(Expr::Column { name: "a".into(), table: None,
    quote_style: Default::default(), table_quote_style: Default::default() }.is_column());
assert!(Expr::Number("1".into()).is_literal());
assert!(Expr::Boolean(false).is_literal());
assert!(!Expr::Null.is_literal());
```

---

## TypedFunction Enum

Typed function variants enable per-function transpilation rules and dialect-specific code generation.
Each variant carries semantically typed arguments rather than a generic `Vec<Expr>`.

When a recognized function name is parsed (e.g., `SUBSTRING`, `NOW`, `COUNT`), the parser creates
`Expr::TypedFunction { func, filter, over }` instead of a generic `Expr::Function`. This allows
the generator to emit the correct SQL for each target dialect without relying on string-based
function renaming.

### TypedFunction Variants

#### Date/Time

| Variant | Fields | SQL |
| --- | --- | --- |
| `DateAdd` | `{ expr, interval, unit? }` | `DATE_ADD(d, INTERVAL 1 DAY)` |
| `DateDiff` | `{ start, end, unit? }` | `DATE_DIFF(d1, d2)` |
| `DateTrunc` | `{ unit, expr }` | `DATE_TRUNC('month', d)` |
| `DateSub` | `{ expr, interval, unit? }` | `DATE_SUB(d, INTERVAL 1 DAY)` |
| `CurrentDate` | — | `CURRENT_DATE` |
| `CurrentTimestamp` | — | `CURRENT_TIMESTAMP` / `NOW()` / `GETDATE()` |
| `StrToTime` | `{ expr, format }` | `STR_TO_TIME(s, fmt)` |
| `TimeToStr` | `{ expr, format }` | `TIME_TO_STR(t, fmt)` |
| `TsOrDsToDate` | `{ expr }` | `TS_OR_DS_TO_DATE(expr)` |
| `Year` | `{ expr }` | `YEAR(d)` |
| `Month` | `{ expr }` | `MONTH(d)` |
| `Day` | `{ expr }` | `DAY(d)` |

#### String

| Variant | Fields | SQL |
| --- | --- | --- |
| `Trim` | `{ expr, trim_type, trim_chars? }` | `TRIM(LEADING 'x' FROM s)` |
| `Substring` | `{ expr, start, length? }` | `SUBSTRING(s, 1, 3)` / `SUBSTR(s, 1, 3)` |
| `Upper` | `{ expr }` | `UPPER(s)` |
| `Lower` | `{ expr }` | `LOWER(s)` |
| `RegexpLike` | `{ expr, pattern, flags? }` | `REGEXP_LIKE(s, '^A')` |
| `RegexpExtract` | `{ expr, pattern, group_index? }` | `REGEXP_EXTRACT(s, '(\\d+)')` |
| `RegexpReplace` | `{ expr, pattern, replacement, flags? }` | `REGEXP_REPLACE(s, '\\d', 'X')` |
| `ConcatWs` | `{ separator, exprs }` | `CONCAT_WS(',', a, b, c)` |
| `Split` | `{ expr, delimiter }` | `SPLIT(s, ',')` |
| `Initcap` | `{ expr }` | `INITCAP(s)` |
| `Length` | `{ expr }` | `LENGTH(s)` / `LEN(s)` |
| `Replace` | `{ expr, from, to }` | `REPLACE(s, 'old', 'new')` |
| `Reverse` | `{ expr }` | `REVERSE(s)` |
| `Left` | `{ expr, n }` | `LEFT(s, 3)` |
| `Right` | `{ expr, n }` | `RIGHT(s, 3)` |
| `Lpad` | `{ expr, length, pad? }` | `LPAD(s, 10, '0')` |
| `Rpad` | `{ expr, length, pad? }` | `RPAD(s, 10, '0')` |

#### Aggregate

| Variant | Fields | SQL |
| --- | --- | --- |
| `Count` | `{ expr, distinct }` | `COUNT(*)`, `COUNT(DISTINCT x)` |
| `Sum` | `{ expr, distinct }` | `SUM(x)`, `SUM(DISTINCT x)` |
| `Avg` | `{ expr, distinct }` | `AVG(x)` |
| `Min` | `{ expr }` | `MIN(x)` |
| `Max` | `{ expr }` | `MAX(x)` |
| `ArrayAgg` | `{ expr, distinct }` | `ARRAY_AGG(x)` / `LIST(x)` / `COLLECT_LIST(x)` |
| `ApproxDistinct` | `{ expr }` | `APPROX_DISTINCT(x)` |
| `Variance` | `{ expr }` | `VARIANCE(x)` / `VAR_SAMP(x)` |
| `VariancePop` | `{ expr }` | `VAR_POP(x)` (T-SQL `VARP(x)`) |
| `Stddev` | `{ expr }` | `STDDEV(x)` / `STDDEV_SAMP(x)` (T-SQL `STDEV(x)`) |
| `StddevPop` | `{ expr }` | `STDDEV_POP(x)` (T-SQL `STDEVP(x)`) |

#### Array

| Variant | Fields | SQL |
| --- | --- | --- |
| `ArrayConcat` | `{ arrays }` | `ARRAY_CONCAT(a, b)` |
| `ArrayContains` | `{ array, element }` | `ARRAY_CONTAINS(arr, 1)` |
| `ArraySize` | `{ expr }` | `ARRAY_SIZE(arr)` / `ARRAY_LENGTH(arr)` |
| `Explode` | `{ expr }` | `EXPLODE(arr)` |
| `GenerateSeries` | `{ start, stop, step? }` | `GENERATE_SERIES(1, 10)` |
| `Flatten` | `{ expr }` | `FLATTEN(arr)` |

#### JSON

| Variant | Fields | SQL |
| --- | --- | --- |
| `JSONExtract` | `{ expr, path }` | `JSON_EXTRACT(doc, '$.key')` |
| `JSONExtractScalar` | `{ expr, path }` | `JSON_EXTRACT_SCALAR(doc, '$.key')` |
| `ParseJSON` | `{ expr }` | `PARSE_JSON(s)` |
| `JSONFormat` | `{ expr }` | `JSON_FORMAT(obj)` / `TO_JSON(obj)` |

#### Window

| Variant | Fields | SQL |
| --- | --- | --- |
| `RowNumber` | — | `ROW_NUMBER()` |
| `Rank` | — | `RANK()` |
| `DenseRank` | — | `DENSE_RANK()` |
| `NTile` | `{ n }` | `NTILE(4)` |
| `Lead` | `{ expr, offset?, default? }` | `LEAD(x, 1, 0)` |
| `Lag` | `{ expr, offset?, default? }` | `LAG(x, 1, 0)` |
| `FirstValue` | `{ expr }` | `FIRST_VALUE(x)` |
| `LastValue` | `{ expr }` | `LAST_VALUE(x)` |

#### Math

| Variant | Fields | SQL |
| --- | --- | --- |
| `Abs` | `{ expr }` | `ABS(x)` |
| `Ceil` | `{ expr }` | `CEIL(x)` / `CEILING(x)` |
| `Floor` | `{ expr }` | `FLOOR(x)` |
| `Round` | `{ expr, decimals? }` | `ROUND(x, 2)` |
| `Log` | `{ expr, base? }` | `LOG(x)` |
| `Ln` | `{ expr }` | `LN(x)` |
| `Pow` | `{ base, exponent }` | `POW(x, 2)` / `POWER(x, 2)` |
| `Sqrt` | `{ expr }` | `SQRT(x)` |
| `Greatest` | `{ exprs }` | `GREATEST(a, b, c)` |
| `Least` | `{ exprs }` | `LEAST(a, b, c)` |
| `Mod` | `{ left, right }` | `MOD(a, b)` |

#### Conversion

| Variant | Fields | SQL |
| --- | --- | --- |
| `Hex` | `{ expr }` | `HEX(x)` / `TO_HEX(x)` |
| `Unhex` | `{ expr }` | `UNHEX(x)` / `FROM_HEX(x)` |
| `Md5` | `{ expr }` | `MD5(x)` |
| `Sha` | `{ expr }` | `SHA(x)` / `SHA1(x)` |
| `Sha2` | `{ expr, bit_length }` | `SHA2(x, 256)` |

### TrimType Enum

```rust
pub enum TrimType {
    Leading,
    Trailing,
    Both,
}
```

### Dialect-Specific Function Generation

The generator emits different SQL for each `TypedFunction` variant depending on the target dialect:

| TypedFunction | ANSI / Default | T-SQL | MySQL | Oracle | Hive/Spark | Presto/Trino |
| --- | --- | --- | --- | --- | --- | --- |
| `CurrentTimestamp` | `CURRENT_TIMESTAMP()` | `GETDATE()` | `NOW()` | `CURRENT_TIMESTAMP()` | `CURRENT_TIMESTAMP()` | `CURRENT_TIMESTAMP()` |
| `Substring` | `SUBSTRING(x, a, b)` | `SUBSTRING(x, a, b)` | `SUBSTR(x, a, b)` | `SUBSTR(x, a, b)` | `SUBSTR(x, a, b)` | `SUBSTRING(x, a, b)` |
| `Length` | `LENGTH(x)` | `LEN(x)` | `LENGTH(x)` | `LENGTH(x)` | `LENGTH(x)` | `LENGTH(x)` |
| `DateTrunc` | `DATE_TRUNC('m', d)` | `DATETRUNC(m, d)` | `DATE_TRUNC('m', d)` | `TRUNC(d, 'm')` | `DATE_TRUNC('m', d)` | `DATE_TRUNC('m', d)` |
| `Ceil` | `CEIL(x)` | `CEILING(x)` | `CEIL(x)` | `CEIL(x)` | `CEIL(x)` | `CEIL(x)` |
| `Pow` | `POW(x, n)` | `POWER(x, n)` | `POW(x, n)` | `POWER(x, n)` | `POW(x, n)` | `POW(x, n)` |
| `ArrayAgg` | `ARRAY_AGG(x)` | `ARRAY_AGG(x)` | `ARRAY_AGG(x)` | `ARRAY_AGG(x)` | `COLLECT_LIST(x)` | `ARRAY_AGG(x)` |
| `Hex` | `HEX(x)` | `HEX(x)` | `HEX(x)` | `HEX(x)` | `HEX(x)` | `TO_HEX(x)` |
| `Unhex` | `UNHEX(x)` | `UNHEX(x)` | `UNHEX(x)` | `UNHEX(x)` | `UNHEX(x)` | `FROM_HEX(x)` |
| `Sha` | `SHA(x)` | `SHA(x)` | `SHA(x)` | `SHA(x)` | `SHA(x)` | `SHA1(x)` |
| `JSONFormat` | `JSON_FORMAT(x)` | `JSON_FORMAT(x)` | `JSON_FORMAT(x)` | `JSON_FORMAT(x)` | `JSON_FORMAT(x)` | `TO_JSON(x)` |

---

## Supporting Types

### SelectItem

```rust
pub enum SelectItem {
    Wildcard,                              // *
    QualifiedWildcard { table: String },   // t.*
    Expr { expr: Expr, alias: Option<String> },  // expr [AS alias]
}
```

### FromClause / TableSource

```rust
pub struct FromClause {
    pub source: TableSource,
}

pub enum TableSource {
    Table(TableRef),
    Subquery { query: Box<Statement>, alias: Option<String> },
    TableFunction { name: String, args: Vec<Expr>, alias: Option<String> },
    Lateral { source: Box<TableSource> },
    Unnest { expr: Box<Expr>, alias: Option<String>, with_offset: bool },
    Pivot { source: Box<TableSource>, aggregate: Box<Expr>, for_column: String, in_values: Vec<PivotValue>, alias: Option<String> },
    Unpivot { source: Box<TableSource>, value_column: String, for_column: String, in_columns: Vec<PivotValue>, alias: Option<String> },
}

pub struct PivotValue {
    pub value: Expr,
    pub alias: Option<String>,
}
```

**PIVOT** rotates rows into columns using an aggregate function:

```sql
SELECT * FROM sales PIVOT (SUM(amount) FOR quarter IN ('Q1', 'Q2', 'Q3')) AS pvt
```

**UNPIVOT** rotates columns back into rows:

```sql
SELECT * FROM quarterly UNPIVOT (amount FOR quarter IN (Q1, Q2, Q3, Q4)) AS unpvt
```

Pivot values/columns may include aliases (`'Q1' AS q1`). Supported in T-SQL, Oracle, Snowflake, BigQuery, Spark, and other dialects.

### TableRef

```rust
pub struct TableRef {
    pub catalog: Option<String>,
    pub schema: Option<String>,
    pub name: String,
    pub alias: Option<String>,
    pub name_quote_style: QuoteStyle,
}
```

### JoinClause / JoinType

```rust
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: TableSource,
    pub on: Option<Expr>,
    pub using: Vec<String>,
}

pub enum JoinType {
    Inner, Left, Right, Full, Cross, Natural, Lateral,
}
```

### OrderByItem

```rust
pub struct OrderByItem {
    pub expr: Expr,
    pub ascending: bool,
    pub nulls_first: Option<bool>,
}
```

### Cte

```rust
pub struct Cte {
    pub name: String,
    pub columns: Vec<String>,
    pub query: Box<Statement>,
    pub materialized: Option<bool>,
    pub recursive: bool,
}
```

### WindowSpec / WindowFrame

```rust
pub struct WindowSpec {
    pub window_ref: Option<String>,
    pub partition_by: Vec<Expr>,
    pub order_by: Vec<OrderByItem>,
    pub frame: Option<WindowFrame>,
}

pub struct WindowFrame {
    pub kind: WindowFrameKind,       // Rows | Range | Groups
    pub start: WindowFrameBound,
    pub end: Option<WindowFrameBound>,
}

pub enum WindowFrameBound {
    CurrentRow,
    Preceding(Option<Box<Expr>>),   // None = UNBOUNDED PRECEDING
    Following(Option<Box<Expr>>),   // None = UNBOUNDED FOLLOWING
}
```

### InsertSource / OnConflict

```rust
pub enum InsertSource {
    Values(Vec<Vec<Expr>>),
    Query(Box<Statement>),
    Default,
}

pub struct OnConflict {
    pub columns: Vec<String>,
    pub action: ConflictAction,
}

pub enum ConflictAction {
    DoNothing,
    DoUpdate(Vec<(String, Expr)>),
}
```

### ColumnDef

```rust
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub nullable: Option<bool>,
    pub default: Option<Expr>,
    pub primary_key: bool,
    pub unique: bool,
    pub auto_increment: bool,
    pub collation: Option<String>,
    pub comment: Option<String>,
}
```

### TableConstraint

```rust
pub enum TableConstraint {
    PrimaryKey { name: Option<String>, columns: Vec<String> },
    Unique { name: Option<String>, columns: Vec<String> },
    ForeignKey {
        name: Option<String>,
        columns: Vec<String>,
        ref_table: TableRef,
        ref_columns: Vec<String>,
        on_delete: Option<ReferentialAction>,
        on_update: Option<ReferentialAction>,
    },
    Check { name: Option<String>, expr: Expr },
}

pub enum ReferentialAction {
    Cascade, Restrict, NoAction, SetNull, SetDefault,
}
```

### AlterTableAction

```rust
pub enum AlterTableAction {
    AddColumn(ColumnDef),
    DropColumn { name: String, if_exists: bool },
    RenameColumn { old_name: String, new_name: String },
    AlterColumnType { name: String, data_type: DataType },
    AddConstraint(TableConstraint),
    DropConstraint { name: String },
    RenameTable { new_name: String },
}
```

---

## Operators

### BinaryOperator

| Variant | SQL |
| --- | --- |
| `Plus` | `+` |
| `Minus` | `-` |
| `Multiply` | `*` |
| `Divide` | `/` |
| `Modulo` | `%` |
| `Eq` | `=` |
| `Neq` | `<>` |
| `Lt` | `<` |
| `Gt` | `>` |
| `LtEq` | `<=` |
| `GtEq` | `>=` |
| `And` | `AND` |
| `Or` | `OR` |
| `Xor` | `XOR` |
| `Concat` | `\|\|` |
| `BitwiseAnd` | `&` |
| `BitwiseOr` | `\|` |
| `BitwiseXor` | `^` |
| `ShiftLeft` | `<<` |
| `ShiftRight` | `>>` |
| `Arrow` | `->` |
| `DoubleArrow` | `->>` |

### UnaryOperator

| Variant | SQL |
| --- | --- |
| `Not` | `NOT` |
| `Minus` | `-` (negation) |
| `Plus` | `+` (positive) |
| `BitwiseNot` | `~` |

---

## DataType Enum

50+ SQL types organized by category.

### Numeric

| Variant | Parameters | Example SQL |
| --- | --- | --- |
| `TinyInt` | — | `TINYINT` |
| `SmallInt` | — | `SMALLINT` |
| `Int` | — | `INT` |
| `BigInt` | — | `BIGINT` |
| `Float` | — | `FLOAT` |
| `Double` | — | `DOUBLE` |
| `Decimal` | `precision: Option<u32>, scale: Option<u32>` | `DECIMAL(10, 2)` |
| `Numeric` | `precision: Option<u32>, scale: Option<u32>` | `NUMERIC(5)` |
| `Real` | — | `REAL` |
| `Serial` | — | `SERIAL` |
| `BigSerial` | — | `BIGSERIAL` |
| `SmallSerial` | — | `SMALLSERIAL` |
| `Money` | — | `MONEY` |

### String Types

| Variant | Parameters | Example SQL |
| --- | --- | --- |
| `Varchar` | `Option<u32>` | `VARCHAR(255)` |
| `Char` | `Option<u32>` | `CHAR(10)` |
| `Text` | — | `TEXT` |
| `String` | — | `STRING` |

### Binary

| Variant | Parameters | Example SQL |
| --- | --- | --- |
| `Binary` | `Option<u32>` | `BINARY(16)` |
| `Varbinary` | `Option<u32>` | `VARBINARY(256)` |
| `Blob` | — | `BLOB` |
| `Bytea` | — | `BYTEA` |
| `Bytes` | — | `BYTES` |
| `Bit` | `Option<u32>` | `BIT(8)` |

### Boolean

| Variant | Example SQL |
| --- | --- |
| `Boolean` | `BOOLEAN` |

### Date / Time

| Variant | Parameters | Example SQL |
| --- | --- | --- |
| `Date` | — | `DATE` |
| `Time` | `precision: Option<u32>` | `TIME(3)` |
| `Timestamp` | `precision: Option<u32>, with_tz: bool` | `TIMESTAMP`, `TIMESTAMPTZ` |
| `DateTime` | — | `DATETIME` |
| `Interval` | — | `INTERVAL` |

### JSON Types

| Variant | Example SQL |
| --- | --- |
| `Json` | `JSON` |
| `Jsonb` | `JSONB` |

### UUID

| Variant | Example SQL |
| --- | --- |
| `Uuid` | `UUID` |

### Complex / Composite

| Variant | Parameters | Example SQL |
| --- | --- | --- |
| `Array` | `Option<Box<DataType>>` | `ARRAY<INT>`, `INT[]` |
| `Map` | `key: Box<DataType>, value: Box<DataType>` | `MAP<STRING, INT>` |
| `Struct` | `Vec<(String, DataType)>` | `STRUCT<name STRING, age INT>` |
| `Tuple` | `Vec<DataType>` | `TUPLE(INT, STRING)` |

### PostgreSQL-Specific

| Variant | Example SQL |
| --- | --- |
| `Inet` | `INET` |
| `Cidr` | `CIDR` |
| `Macaddr` | `MACADDR` |
| `Regclass` | `REGCLASS` |
| `Regtype` | `REGTYPE` |
| `Hstore` | `HSTORE` |
| `Geography` | `GEOGRAPHY` |
| `Geometry` | `GEOMETRY` |

### Vendor-Specific

| Variant | Dialects | Example SQL |
| --- | --- | --- |
| `Variant` | Snowflake | `VARIANT` |
| `Object` | Snowflake | `OBJECT` |
| `Super` | Redshift | `SUPER` |
| `Xml` | T-SQL, Oracle | `XML` |

### Special

| Variant | Description |
| --- | --- |
| `Null` | The NULL type |
| `Unknown(String)` | Fallback for unrecognized type names |

---

## DateTimeField Enum

Used by `EXTRACT(field FROM expr)` and `INTERVAL`:

| Variant | SQL |
| --- | --- |
| `Year` | `YEAR` |
| `Quarter` | `QUARTER` |
| `Month` | `MONTH` |
| `Week` | `WEEK` |
| `Day` | `DAY` |
| `DayOfWeek` | `DOW` |
| `DayOfYear` | `DOY` |
| `Hour` | `HOUR` |
| `Minute` | `MINUTE` |
| `Second` | `SECOND` |
| `Millisecond` | `MILLISECOND` |
| `Microsecond` | `MICROSECOND` |
| `Nanosecond` | `NANOSECOND` |
| `Epoch` | `EPOCH` |
| `Timezone` | `TIMEZONE` |
| `TimezoneHour` | `TIMEZONE_HOUR` |
| `TimezoneMinute` | `TIMEZONE_MINUTE` |

**Example:**

```rust
use sqlglot_rust::{parse, generate, Dialect};

let ast = parse("SELECT EXTRACT(YEAR FROM hire_date) FROM employees", Dialect::Ansi).unwrap();
let sql = generate(&ast, Dialect::Ansi);
assert_eq!(sql, "SELECT EXTRACT(YEAR FROM hire_date) FROM employees");
```

---

## CommentType Enum

```rust
pub enum CommentType {
    Line,   // -- comment
    Block,  // /* comment */
    Hash,   // # comment (MySQL)
}
```

Used internally to classify comment tokens. Comments are stored as raw strings
(including delimiters) on the `comments: Vec<String>` field of each statement
struct and on `Expr::Commented`.

---

## QuoteStyle Enum

```rust
pub enum QuoteStyle {
    None,         // bare identifier
    DoubleQuote,  // "identifier"
    Backtick,     // `identifier`
    Bracket,      // [identifier]
}
```

### Methods

| Method | Signature | Description |
| --- | --- | --- |
| `for_dialect` | `(dialect: Dialect) -> QuoteStyle` | Canonical quote style for a dialect |
| `is_quoted` | `(self) -> bool` | `true` unless `None` |

### Quote Style by Dialect

| Style | Dialects |
| --- | --- |
| `DoubleQuote` | ANSI, PostgreSQL, Oracle, Snowflake, Presto, Trino, Redshift, Athena, DuckDB, and others |
| `Backtick` | MySQL, BigQuery, Hive, Spark, Databricks, Doris, SingleStore, StarRocks |
| `Bracket` | T-SQL, Fabric |

**Example:**

```rust
use sqlglot_rust::{QuoteStyle, Dialect};

assert_eq!(QuoteStyle::for_dialect(Dialect::Mysql), QuoteStyle::Backtick);
assert_eq!(QuoteStyle::for_dialect(Dialect::Postgres), QuoteStyle::DoubleQuote);
assert_eq!(QuoteStyle::for_dialect(Dialect::Tsql), QuoteStyle::Bracket);
assert!(QuoteStyle::Backtick.is_quoted());
assert!(!QuoteStyle::None.is_quoted());
```

---

## Dialect Enum

### Dialect List

30 dialects in two support tiers:

| # | Variant | Display Name | Support |
| --- | --- | --- | --- |
| 1 | `Ansi` | ANSI SQL | Official |
| 2 | `Athena` | Athena | Official |
| 3 | `BigQuery` | BigQuery | Official |
| 4 | `ClickHouse` | ClickHouse | Official |
| 5 | `Databricks` | Databricks | Official |
| 6 | `DuckDb` | DuckDB | Official |
| 7 | `Hive` | Hive | Official |
| 8 | `Mysql` | MySQL | Official |
| 9 | `Oracle` | Oracle | Official |
| 10 | `Postgres` | PostgreSQL | Official |
| 11 | `Presto` | Presto | Official |
| 12 | `Redshift` | Redshift | Official |
| 13 | `Snowflake` | Snowflake | Official |
| 14 | `Spark` | Spark | Official |
| 15 | `Sqlite` | SQLite | Official |
| 16 | `StarRocks` | StarRocks | Official |
| 17 | `Trino` | Trino | Official |
| 18 | `Tsql` | T-SQL | Official |
| 19 | `Doris` | Doris | Community |
| 20 | `Dremio` | Dremio | Community |
| 21 | `Drill` | Drill | Community |
| 22 | `Druid` | Druid | Community |
| 23 | `Exasol` | Exasol | Community |
| 24 | `Fabric` | Fabric | Community |
| 25 | `Materialize` | Materialize | Community |
| 26 | `Prql` | PRQL | Community |
| 27 | `RisingWave` | RisingWave | Community |
| 28 | `SingleStore` | SingleStore | Community |
| 29 | `Tableau` | Tableau | Community |
| 30 | `Teradata` | Teradata | Community |

### Dialect Methods

| Method | Signature | Description |
| --- | --- | --- |
| `all()` | `-> &'static [Dialect]` | All 30 dialect variants |
| `from_str(s)` | `(s: &str) -> Option<Dialect>` | Case-insensitive lookup with aliases |
| `support_level()` | `(&self) -> &'static str` | `"Official"` or `"Community"` |
| `Display` trait | `format!("{}", dialect)` | Human-readable name |

### Dialect Aliases (from_str)

| Input String(s) | Resolved Dialect |
| --- | --- |
| `postgres`, `postgresql` | `Dialect::Postgres` |
| `tsql`, `mssql`, `sqlserver` | `Dialect::Tsql` |
| `bigquery` | `Dialect::BigQuery` |
| `clickhouse` | `Dialect::ClickHouse` |
| `hive` | `Dialect::Hive` |
| `spark` | `Dialect::Spark` |
| `mysql` | `Dialect::Mysql` |
| `sqlite` | `Dialect::Sqlite` |
| `duckdb` | `Dialect::DuckDb` |

(All lookups are case-insensitive.)

**Example:**

```rust
use sqlglot_rust::Dialect;

assert_eq!(Dialect::from_str("postgresql"), Some(Dialect::Postgres));
assert_eq!(Dialect::from_str("mssql"), Some(Dialect::Tsql));
assert_eq!(Dialect::from_str("BIGQUERY"), Some(Dialect::BigQuery));
assert_eq!(Dialect::from_str("unknown"), None);

assert_eq!(Dialect::Postgres.support_level(), "Official");
assert_eq!(Dialect::Teradata.support_level(), "Community");

assert_eq!(format!("{}", Dialect::DuckDb), "DuckDB");
assert_eq!(Dialect::all().len(), 30);
```

### Function Mapping Matrix

Transformations applied automatically during transpilation via typed function expressions:

| Source Function | Target Dialects | Mapped To |
| --- | --- | --- |
| `NOW()` | BigQuery, Snowflake, ANSI, Hive, Spark, Presto, Trino, ClickHouse | `CURRENT_TIMESTAMP()` |
| `NOW()` | T-SQL | `GETDATE()` |
| `GETDATE()` | PostgreSQL, DuckDB, SQLite, Redshift | `NOW()` |
| `GETDATE()` | BigQuery, ANSI | `CURRENT_TIMESTAMP()` |
| `SUBSTRING(x, a, b)` | MySQL, SQLite, Oracle, Hive, Spark, Databricks | `SUBSTR(x, a, b)` |
| `SUBSTR(x, a, b)` | PostgreSQL, DuckDB, ANSI, Presto, Trino | `SUBSTRING(x, a, b)` |
| `LEN(x)` | PostgreSQL, MySQL, SQLite, DuckDB, ANSI | `LENGTH(x)` |
| `LENGTH(x)` | T-SQL | `LEN(x)` |
| `CEIL(x)` | T-SQL | `CEILING(x)` |
| `POW(x, n)` | T-SQL, Oracle | `POWER(x, n)` |
| `DATE_TRUNC(u, d)` | T-SQL | `DATETRUNC(u, d)` |
| `DATE_TRUNC(u, d)` | Oracle | `TRUNC(d, u)` |
| `ARRAY_AGG(x)` | DuckDB | `LIST(x)` |
| `ARRAY_AGG(x)` | Hive, Spark, Databricks | `COLLECT_LIST(x)` |
| `HEX(x)` | Presto, Trino | `TO_HEX(x)` |
| `UNHEX(x)` | Presto, Trino | `FROM_HEX(x)` |
| `SHA(x)` | Presto, Trino | `SHA1(x)` |
| `JSON_FORMAT(x)` | Presto, Trino | `TO_JSON(x)` |
| `IFNULL(a, b)` | PostgreSQL, ANSI, DuckDB | `COALESCE(a, b)` |
| `IFNULL(a, b)` | T-SQL | `ISNULL(a, b)` |

### Data Type Mapping Matrix

| Source Type | Target Dialect | Mapped Type |
| --- | --- | --- |
| `TEXT` | BigQuery | `STRING` |
| `STRING` | PostgreSQL, MySQL, SQLite | `TEXT` |
| `INT` | BigQuery | `BIGINT` |
| `FLOAT` | BigQuery | `DOUBLE` |
| `BYTEA` | MySQL, SQLite, BigQuery | `BLOB` |
| `BLOB` | PostgreSQL | `BYTEA` |

### Identifier Quote Styles by Dialect

| Quote Style | Dialects |
| --- | --- |
| `"double"` | ANSI, PostgreSQL, Oracle, Snowflake, Presto, Trino, Redshift, Athena, DuckDB |
| `` `backtick` `` | MySQL, BigQuery, Hive, Spark, Databricks, Doris, SingleStore, StarRocks |
| `[bracket]` | T-SQL, Fabric |

### Time Format Mapping

Date/time format strings are automatically converted during transpilation. Each dialect family uses different format specifiers:

| Style | Dialects | Year | Month | Day | Hour (24h) | Minute | Second |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **strftime** | SQLite, BigQuery, DuckDB | `%Y` | `%m` | `%d` | `%H` | `%M` | `%S` |
| **MySQL** | MySQL, Doris, SingleStore, StarRocks | `%Y` | `%m` | `%d` | `%H` | `%i` | `%s` |
| **Postgres** | PostgreSQL, Oracle, Redshift | `YYYY` | `MM` | `DD` | `HH24` | `MI` | `SS` |
| **Snowflake** | Snowflake | `YYYY` | `MM` | `DD` | `HH24` | `MI` | `SS` |
| **Java** | Spark, Hive, Databricks, Presto, Trino | `yyyy` | `MM` | `dd` | `HH` | `mm` | `ss` |

**Example Transpilation:**

```rust
use sqlglot_rust::{transpile, Dialect};

// MySQL → PostgreSQL: format strings are converted automatically
let result = transpile(
    "SELECT DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s')",
    Dialect::Mysql,
    Dialect::Postgres
).unwrap();
assert_eq!(result, "SELECT TO_CHAR(created_at, 'YYYY-MM-DD HH24:MI:SS')");

// PostgreSQL → Spark
let result = transpile(
    "SELECT TO_CHAR(dt, 'YYYY-MM-DD HH24:MI:SS')",
    Dialect::Postgres,
    Dialect::Spark
).unwrap();
assert_eq!(result, "SELECT DATE_FORMAT(dt, 'yyyy-MM-dd HH:mm:ss')");
```

**Direct Format Conversion:**

```rust
use sqlglot_rust::{format_time, format_time_dialect, TimeFormatStyle, Dialect};

// Convert format strings directly
let pg_format = format_time("%Y-%m-%d", TimeFormatStyle::Strftime, TimeFormatStyle::Postgres);
assert_eq!(pg_format, "YYYY-MM-DD");

// Or use dialect-to-dialect conversion
let spark_format = format_time_dialect("YYYY-MM-DD HH24:MI:SS", Dialect::Postgres, Dialect::Spark);
assert_eq!(spark_format, "yyyy-MM-dd HH:mm:ss");
```

**Format Function Mapping:**

| Source Function | Postgres | MySQL | BigQuery | Spark | T-SQL |
| --- | --- | --- | --- | --- | --- |
| Time → String | `TO_CHAR()` | `DATE_FORMAT()` | `FORMAT_TIMESTAMP()` | `DATE_FORMAT()` | `FORMAT()` |
| String → Time | `TO_TIMESTAMP()` | `STR_TO_DATE()` | `PARSE_TIMESTAMP()` | `TO_TIMESTAMP()` | `CONVERT()` |

**T-SQL Style Codes:**

T-SQL's `CONVERT` function uses numeric style codes instead of format patterns. The `TsqlStyleCode` enum provides mappings:

| Code | Format | Example |
| --- | --- | --- |
| 101 | mm/dd/yyyy (USA) | 03/17/2026 |
| 102 | yyyy.mm.dd (ANSI) | 2026.03.17 |
| 103 | dd/mm/yyyy (British) | 17/03/2026 |
| 120 | yyyy-mm-dd hh:mi:ss (ODBC) | 2026-03-17 22:15:30 |
| 126 | yyyy-mm-ddThh:mi:ss.mmm (ISO8601) | 2026-03-17T22:15:30.123 |

---

## Custom Dialect Plugins

The plugin system (`sqlglot_rust::dialects::plugin`) provides extensibility for
custom SQL dialects without modifying the library source.

### DialectPlugin Trait

```rust
pub trait DialectPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn quote_style(&self) -> Option<QuoteStyle>;
    fn supports_ilike(&self) -> Option<bool>;
    fn map_function_name(&self, name: &str) -> Option<String>;
    fn map_data_type(&self, data_type: &DataType) -> Option<DataType>;
    fn transform_expr(&self, expr: &Expr) -> Option<Expr>;
    fn transform_statement(&self, statement: &Statement) -> Option<Statement>;
}
```

All methods except `name()` have default implementations that return `None`
(fall through to built-in logic). Implementations must be `Send + Sync`.

| Method | Return | Meaning of `None` |
| --- | --- | --- |
| `name()` | `&str` | *(required)* |
| `quote_style()` | `Option<QuoteStyle>` | Use double-quote |
| `supports_ilike()` | `Option<bool>` | Assume false |
| `map_function_name(name)` | `Option<String>` | Keep original name |
| `map_data_type(dt)` | `Option<DataType>` | Keep original type |
| `transform_expr(expr)` | `Option<Expr>` | Use default transform |
| `transform_statement(stmt)` | `Option<Statement>` | Use default transform |

### DialectRegistry

Thread-safe global singleton for custom dialect registration.

```rust
pub struct DialectRegistry { /* ... */ }

impl DialectRegistry {
    pub fn global() -> &'static DialectRegistry;
    pub fn register<P: DialectPlugin + 'static>(&self, plugin: P);
    pub fn get(&self, name: &str) -> Option<Arc<dyn DialectPlugin>>;
    pub fn unregister(&self, name: &str) -> bool;
    pub fn registered_names(&self) -> Vec<String>;
}
```

| Method | Description |
| --- | --- |
| `global()` | Returns the singleton registry |
| `register(plugin)` | Registers a plugin (replaces if name exists) |
| `get(name)` | Case-insensitive lookup |
| `unregister(name)` | Remove by name; returns `true` if found |
| `registered_names()` | All registered custom dialect names |

### DialectRef Enum

Unified handle for built-in and custom dialects:

```rust
pub enum DialectRef {
    BuiltIn(Dialect),
    Custom(String),
}
```

| Constructor | Description |
| --- | --- |
| `DialectRef::from(Dialect::Postgres)` | Built-in dialect |
| `DialectRef::custom("acme")` | Custom dialect (case-insensitive) |

| Method | Signature | Description |
| --- | --- | --- |
| `as_builtin()` | `-> Option<Dialect>` | Extract built-in variant |
| `as_plugin()` | `-> Option<Arc<dyn DialectPlugin>>` | Look up custom plugin |
| `quote_style()` | `-> QuoteStyle` | Resolved quote style |
| `supports_ilike()` | `-> bool` | Resolved ILIKE support |
| `map_function_name(name)` | `-> String` | Resolved function name |
| `map_data_type(dt)` | `-> DataType` | Resolved data type |

### Plugin-Aware Functions

| Function | Signature | Description |
| --- | --- | --- |
| `register_dialect(plugin)` | `<P: DialectPlugin + 'static>(P)` | Convenience registration |
| `resolve_dialect(name)` | `(&str) -> Option<DialectRef>` | Resolve name to built-in or custom |
| `transpile_ext(sql, read, write)` | `(&str, &DialectRef, &DialectRef) -> Result<String>` | Plugin-aware transpile |
| `transpile_statements_ext(sql, read, write)` | `(&str, &DialectRef, &DialectRef) -> Result<Vec<String>>` | Plugin-aware multi-statement transpile |

---

## Error Types

```rust
pub enum SqlglotError {
    TokenizerError { message: String, position: usize },
    ParserError { message: String },
    UnexpectedToken { token: Token },
    UnsupportedDialectFeature(String),
    Internal(String),
}
```

| Variant | When It Occurs | Key Fields |
| --- | --- | --- |
| `TokenizerError` | Invalid character or unterminated string | `message`, `position` (byte offset) |
| `ParserError` | Grammar / syntax error | `message` |
| `UnexpectedToken` | Parser hit an unexpected token | `token` (full `Token` struct) |
| `UnsupportedDialectFeature` | Feature unavailable in target dialect | Description string |
| `Internal` | Library bug | Description string |

All variants implement `std::fmt::Display` and `std::error::Error` via `thiserror`.

**Example:**

```rust
use sqlglot_rust::{parse, Dialect, SqlglotError};

let err = parse("SELECT ???", Dialect::Ansi).unwrap_err();
match err {
    SqlglotError::ParserError { message } => println!("Parse: {message}"),
    SqlglotError::TokenizerError { message, position } => {
        println!("Token error at byte {position}: {message}")
    }
    other => println!("{other}"),
}
```

---

## Free Functions (ast module)

Accessed via `use sqlglot_rust::ast::*`.

| Function | Signature | Returns | Description |
| --- | --- | --- | --- |
| `find_columns` | `(expr: &Expr) -> Vec<&Expr>` | `Vec<&Expr>` | All `Column` nodes in an expression tree |
| `find_tables` | `(stmt: &Statement) -> Vec<&TableRef>` | `Vec<&TableRef>` | All `TableRef` nodes in a statement |

**Example:**

```rust
use sqlglot_rust::{parse, Dialect};
use sqlglot_rust::ast::find_tables;

let stmt = parse(
    "SELECT a FROM t1 INNER JOIN t2 ON t1.id = t2.id",
    Dialect::Ansi,
).unwrap();
let tables = find_tables(&stmt);
assert_eq!(tables.len(), 2);
assert_eq!(tables[0].name, "t1");
assert_eq!(tables[1].name, "t2");
```

---

## Schema System

Accessed via `use sqlglot_rust::schema::*`.

The schema module provides dialect-aware table and column metadata management, serving as the foundation for type annotation, column qualification, and lineage analysis.

### Schema Trait

| Method | Signature | Returns | Description |
| --- | --- | --- | --- |
| `add_table` | `(&mut self, table_path: &[&str], columns: Vec<(String, DataType)>)` | `Result<(), SchemaError>` | Register a table with column definitions |
| `column_names` | `(&self, table_path: &[&str])` | `Result<Vec<String>, SchemaError>` | Get column names in definition order |
| `get_column_type` | `(&self, table_path: &[&str], column: &str)` | `Result<DataType, SchemaError>` | Get the data type of a column |
| `has_column` | `(&self, table_path: &[&str], column: &str)` | `bool` | Check if a column exists |
| `dialect` | `(&self)` | `Dialect` | Get the configured dialect |

### MappingSchema

In-memory implementation of `Schema` with 3-level nesting: `catalog → database → table → column → type`.

| Method | Signature | Returns | Description |
| --- | --- | --- | --- |
| `new` | `(dialect: Dialect)` | `MappingSchema` | Create an empty schema for the given dialect |
| `replace_table` | `(&mut self, table_path: &[&str], columns: Vec<(String, DataType)>)` | `Result<(), SchemaError>` | Add or overwrite a table |
| `remove_table` | `(&mut self, table_path: &[&str])` | `Result<bool, SchemaError>` | Remove a table (returns true if existed) |
| `add_udf` | `(&mut self, name: &str, return_type: DataType)` | `()` | Register a UDF with its return type |
| `get_udf_type` | `(&self, name: &str)` | `Option<&DataType>` | Look up a UDF's return type |
| `table_names` | `(&self)` | `Vec<(String, String, String)>` | List all registered tables |

**Table path formats:**

| Path | Example | Resolved as |
| --- | --- | --- |
| `&["table"]` | `&["users"]` | `"" → "" → "users"` |
| `&["db", "table"]` | `&["public", "users"]` | `"" → "public" → "users"` |
| `&["catalog", "db", "table"]` | `&["prod", "public", "users"]` | `"prod" → "public" → "users"` |

**Identifier normalization:** Unquoted identifiers are lowercased for case-insensitive dialects (Postgres, MySQL, Snowflake, etc.) and kept as-is for case-sensitive dialects (BigQuery, Hive, Spark, Databricks).

### Helper Functions

| Function | Signature | Returns | Description |
| --- | --- | --- | --- |
| `ensure_schema` | `(tables: HashMap<String, HashMap<String, DataType>>, dialect: Dialect)` | `MappingSchema` | Build schema from `table → column → type` map |
| `ensure_schema_nested` | `(catalog_map: CatalogMap, dialect: Dialect)` | `MappingSchema` | Build schema from `catalog → db → table → column → type` map |
| `normalize_identifier` | `(name: &str, dialect: Dialect)` | `String` | Normalize an identifier per dialect rules |
| `is_case_sensitive_dialect` | `(dialect: Dialect)` | `bool` | Check if a dialect is case-sensitive |

### SchemaError

| Variant | Description |
| --- | --- |
| `TableNotFound(String)` | The referenced table was not found |
| `ColumnNotFound { table, column }` | The column was not found in the table |
| `DuplicateTable(String)` | Table is already registered (use `replace_table` instead) |

`SchemaError` implements `From<SchemaError> for SqlglotError`.

---

## Optimizer

Accessed via `use sqlglot_rust::optimizer::optimize`.

| Function | Signature | Returns | Description |
| --- | --- | --- | --- |
| `optimize` | `(stmt: Statement) -> Result<Statement>` | `Statement` | Apply all optimization passes |
| `build_scope` | `(stmt: &Statement) -> Scope` | `Scope` | Build a scope tree from a parsed statement |
| `find_all_in_scope` | `(scope: &Scope, predicate: &F) -> Vec<&ColumnRef>` | `Vec<&ColumnRef>` | Find columns matching a predicate within a single scope |
| `annotate_types` | `(stmt: &Statement, schema: &S) -> TypeAnnotations` | `TypeAnnotations` | Infer SQL types for all expression nodes |

### Optimization Passes

| Pass | Description | Example |
| --- | --- | --- |
| **Constant Folding** | Evaluate compile-time expressions | `1 + 2` → `3` |
| **Boolean Simplification** | Eliminate tautologies / contradictions | `TRUE AND x` → `x` |
| **Dead Predicate Elimination** | Remove trivially-true WHERE clauses | `WHERE TRUE` → removed |
| **Subquery Unnesting** | Decorrelate subqueries into JOINs | `WHERE EXISTS (… WHERE b.id = a.id)` → `INNER JOIN` |
| **Qualify Columns** | Resolve and qualify column references | `SELECT col FROM t` → `SELECT t.col FROM t` |
| **Annotate Types** | Infer SQL types for all AST expression nodes | `Column(id)` → `Int`, `COUNT(*)` → `BigInt` |

### Subquery Unnesting Details

`unnest_subqueries` rewrites correlated subqueries in WHERE clauses into JOINs.

Accessed via `use sqlglot_rust::optimizer::unnest_subqueries::unnest_subqueries`.

| Function | Signature | Returns | Description |
| --- | --- | --- | --- |
| `unnest_subqueries` | `(stmt: Statement) -> Statement` | `Statement` | Decorrelate WHERE subqueries into JOINs |

| Pattern | Rewrite |
| --- | --- |
| `WHERE EXISTS (SELECT … WHERE b.id = a.id)` | `INNER JOIN (SELECT DISTINCT …) ON a.id = _u0.id` |
| `WHERE NOT EXISTS (…)` | `LEFT JOIN … WHERE _u0._sentinel IS NULL` |
| `WHERE x IN (SELECT col FROM …)` | `INNER JOIN (SELECT DISTINCT col AS _col0 …)` |
| `WHERE x NOT IN (SELECT col FROM …)` | `LEFT JOIN … WHERE _u0._col0 IS NULL` |

The pass bails out (no-op) when:

- No equality correlation exists in the subquery.
- Non-equality correlations are present (e.g., `<`, `>`) — would need LATERAL.
- The subquery is embedded in a SELECT-list function (e.g., `COALESCE`).

**Example:**

```rust
use sqlglot_rust::{parse, generate, Dialect};
use sqlglot_rust::optimizer::optimize;

let stmt = parse("SELECT 1 + 2 * 3", Dialect::Ansi).unwrap();
let opt = optimize(stmt).unwrap();
assert_eq!(generate(&opt, Dialect::Ansi), "SELECT 7");

let stmt = parse("SELECT a FROM t WHERE TRUE AND x > 1", Dialect::Ansi).unwrap();
let opt = optimize(stmt).unwrap();
assert_eq!(generate(&opt, Dialect::Ansi), "SELECT a FROM t WHERE x > 1");
```

### Qualify Columns

`qualify_columns` resolves column references against the schema, adds table qualifiers to
unqualified columns, and expands wildcard selects (`*`, `t.*`) into explicit column lists.

Accessed via `use sqlglot_rust::optimizer::qualify_columns::qualify_columns`.

| Function | Signature | Returns | Description |
| --- | --- | --- | --- |
| `qualify_columns` | `(stmt: Statement, schema: &S) -> Statement` | `Statement` | Qualify column references and expand wildcards |

The schema parameter must implement the `Schema` trait. The most common implementation
is `MappingSchema` (see [Schema System](#schema-system)).

| Transformation | Before | After |
| --- | --- | --- |
| Qualify unqualified column | `SELECT col FROM t` | `SELECT t.col FROM t` |
| Expand `*` | `SELECT * FROM t` | `SELECT t.id, t.name FROM t` |
| Expand `t.*` | `SELECT t.* FROM t` | `SELECT t.id, t.name FROM t` |
| Qualify WHERE / GROUP BY / ORDER BY | `WHERE col = 1` | `WHERE t.col = 1` |
| Qualify JOIN ON | `ON id = other_id` | `ON a.id = b.other_id` |
| CTE column resolution | `WITH cte AS (...) SELECT col FROM cte` | `WITH cte AS (...) SELECT cte.col FROM cte` |
| Derived table columns | `SELECT col FROM (SELECT ...) AS sub` | `SELECT sub.col FROM (SELECT ...) AS sub` |
| Subquery in WHERE | `WHERE id IN (SELECT fk FROM t2)` | `WHERE t.id IN (SELECT t2.fk FROM t2)` |

Ambiguous columns (present in multiple sources) are left unqualified.

**Example:**

```rust
use sqlglot_rust::{parse, generate, Dialect};
use sqlglot_rust::optimizer::qualify_columns::qualify_columns;
use sqlglot_rust::schema::MappingSchema;

let schema = MappingSchema::new()
    .with_table(vec!["users"], vec!["id", "name", "email"])
    .with_table(vec!["orders"], vec!["id", "user_id", "amount"]);

let stmt = parse("SELECT name FROM users WHERE id = 1", Dialect::Ansi).unwrap();
let qualified = qualify_columns(stmt, &schema);
assert_eq!(
    generate(&qualified, Dialect::Ansi),
    "SELECT users.name FROM users WHERE users.id = 1"
);

// Expand wildcard
let stmt = parse("SELECT * FROM users", Dialect::Ansi).unwrap();
let qualified = qualify_columns(stmt, &schema);
assert_eq!(
    generate(&qualified, Dialect::Ansi),
    "SELECT users.id, users.name, users.email FROM users"
);
```

### Scope Analysis

Scope analysis tracks the sources, columns, and inter-scope relationships
in a query tree. It is the foundation for qualify_columns, pushdown_predicates,
annotate_types, and column lineage analysis.

Accessed via `use sqlglot_rust::optimizer::scope_analysis::{build_scope, find_all_in_scope, Scope, ScopeType}`
or the crate-level re-exports `use sqlglot_rust::{build_scope, find_all_in_scope, Scope, ScopeType}`.

#### ScopeType

| Variant | Description |
| --- | --- |
| `Root` | The outermost query |
| `Subquery` | A scalar or lateral subquery (WHERE / SELECT / HAVING) |
| `DerivedTable` | A subquery in FROM |
| `Cte` | A CTE definition (`WITH name AS (...)`) |
| `Union` | One branch of a UNION / INTERSECT / EXCEPT |
| `Udtf` | A user-defined table function / LATERAL |

#### Scope Fields

| Field | Type | Description |
| --- | --- | --- |
| `scope_type` | `ScopeType` | Kind of scope |
| `sources` | `HashMap<String, Source>` | Source name/alias → table or child scope |
| `columns` | `Vec<ColumnRef>` | Column references in this scope (not child scopes) |
| `external_columns` | `Vec<ColumnRef>` | Columns referencing an outer scope (correlations) |
| `derived_table_scopes` | `Vec<Scope>` | Child scopes from subqueries in FROM |
| `subquery_scopes` | `Vec<Scope>` | Child scopes from scalar/EXISTS/IN subqueries |
| `union_scopes` | `Vec<Scope>` | Child scopes for UNION/INTERSECT/EXCEPT branches |
| `cte_scopes` | `Vec<Scope>` | Child scopes for CTE definitions |
| `selected_sources` | `HashMap<String, Source>` | Sources referenced by SELECT columns |
| `is_correlated` | `bool` | Whether scope references outer columns |

#### Example

```rust
use sqlglot_rust::{parse, Dialect, build_scope, find_all_in_scope};
use sqlglot_rust::optimizer::scope_analysis::ScopeType;

let ast = parse(
    "SELECT a FROM t1 WHERE EXISTS (SELECT 1 FROM t2 WHERE t2.id = t1.id)",
    Dialect::Ansi,
).unwrap();
let scope = build_scope(&ast);

assert_eq!(scope.scope_type, ScopeType::Root);
assert!(scope.sources.contains_key("t1"));
assert_eq!(scope.subquery_scopes.len(), 1);

let sub = &scope.subquery_scopes[0];
assert!(sub.is_correlated);
assert!(sub.external_columns.iter().any(|c| c.table.as_deref() == Some("t1")));

// Find all columns referencing t1 in the root scope
let t1_cols = find_all_in_scope(&scope, &|c| c.table.as_deref() == Some("t1"));
assert!(!t1_cols.is_empty());
```

### Annotate Types

`annotate_types` infers and propagates SQL data types across all AST expression nodes
using schema metadata. It is the foundation for type-aware transpilation and validation.

Accessed via `use sqlglot_rust::optimizer::annotate_types::{annotate_types, TypeAnnotations}`
or the crate-level re-exports `use sqlglot_rust::{annotate_types, TypeAnnotations}`.

| Function | Signature | Returns | Description |
| --- | --- | --- | --- |
| `annotate_types` | `(stmt: &Statement, schema: &S) -> TypeAnnotations` | `TypeAnnotations` | Annotate all expressions with inferred types |

#### TypeAnnotations

| Method | Signature | Returns | Description |
| --- | --- | --- | --- |
| `get_type` | `(&self, expr: &Expr) -> Option<&DataType>` | `Option<&DataType>` | Get the inferred type of an expression node |
| `len` | `(&self) -> usize` | `usize` | Number of annotated nodes |
| `is_empty` | `(&self) -> bool` | `bool` | Whether any annotations exist |

> **Important:** `TypeAnnotations` stores raw pointer references. The statement must not be
> moved or dropped while the annotations are in use. Work with the statement by reference
> after calling `annotate_types`.

#### Type Propagation Rules

| Expression | Inferred Type | Rule |
| --- | --- | --- |
| `42` (integer literal) | `Int` / `BigInt` | Fits `i32` → `Int`, otherwise `BigInt` |
| `3.14` (decimal literal) | `Double` | Contains `.`, `e`, or `E` |
| `'hello'` (string literal) | `Varchar` | All string literals |
| `TRUE` / `FALSE` | `Boolean` | Boolean literals |
| `NULL` | `Null` | Null literal |
| `Column(name)` | schema type | Looked up from `Schema` |
| `a + b` (arithmetic) | coerced type | Wider of operand types |
| `a > b` (comparison) | `Boolean` | All comparison operators |
| `a AND b` (logical) | `Boolean` | Logical operators |
| `a \|\| b` (concat) | `Varchar` | String concatenation |
| `CAST(x AS T)` | `T` | Target type |
| `CASE WHEN ... THEN a ELSE b END` | common type | Widest of all THEN/ELSE branches |
| `COUNT(*)` | `BigInt` | Always BigInt |
| `SUM(int_col)` | `BigInt` | Integer inputs → BigInt |
| `SUM(decimal_col)` | `Decimal` | Preserves precision/scale |
| `AVG(x)` | `Double` | Always Double |
| `MIN(x)` / `MAX(x)` | input type | Same as argument type |
| `UPPER(x)` / `LOWER(x)` | `Varchar` | String functions |
| `LENGTH(x)` | `Int` | Returns integer |
| `EXISTS (...)` | `Boolean` | Existence check |
| `x BETWEEN a AND b` | `Boolean` | Boolean predicate |
| `x IN (...)` | `Boolean` | Boolean predicate |
| `x IS NULL` | `Boolean` | Boolean predicate |
| `EXTRACT(YEAR FROM x)` | `Int` | Date part extraction |
| `ROW_NUMBER()` / `RANK()` | `BigInt` | Window ranking |
| `LEAD(x)` / `LAG(x)` | input type | Same as argument |
| `JSON_EXTRACT(...)` | `Json` | JSON type |
| `JSON_EXTRACT_SCALAR(...)` | `Varchar` | Text extraction |
| UDF | registered type | From `schema.add_udf(name, type)` |

#### Numeric Coercion Precedence

Wider types supersede narrower ones in arithmetic expressions:

```text
Boolean < TinyInt < SmallInt < Int < BigInt < Float/Real < Double < Decimal/Numeric
```

#### Annotate Types Example

```rust
use sqlglot_rust::{parse, Dialect, annotate_types};
use sqlglot_rust::ast::DataType;
use sqlglot_rust::schema::{MappingSchema, Schema};

let mut schema = MappingSchema::new(Dialect::Ansi);
schema.add_table(&["users"], vec![
    ("id".to_string(), DataType::Int),
    ("name".to_string(), DataType::Varchar(Some(255))),
    ("salary".to_string(), DataType::Double),
]).unwrap();

let stmt = parse("SELECT id, name, salary * 1.1 FROM users WHERE id > 5", Dialect::Ansi).unwrap();
let ann = annotate_types(&stmt, &schema);

// Query types from the annotations
if let sqlglot_rust::Statement::Select(sel) = &stmt {
    for col in &sel.columns {
        if let sqlglot_rust::ast::SelectItem::Expr { expr, .. } = col {
            if let Some(dt) = ann.get_type(expr) {
                println!("Column type: {:?}", dt);
            }
        }
    }
}
```

### Column Lineage

Column lineage tracking traces data flow from source columns through query
transformations to output columns. Essential for data governance, impact
analysis, and compliance.

Accessed via `use sqlglot_rust::optimizer::lineage::{lineage, lineage_sql, LineageConfig, LineageError, LineageGraph, LineageNode}`
or the crate-level re-exports `use sqlglot_rust::{lineage, lineage_sql, LineageConfig, LineageError, LineageGraph, LineageNode}`.

| Function | Signature | Returns | Description |
| --- | --- | --- | --- |
| `lineage` | `(column: &str, stmt: &Statement, schema: &MappingSchema, config: &LineageConfig) -> LineageResult<LineageGraph>` | `LineageGraph` | Build lineage for a column in a parsed statement |
| `lineage_sql` | `(column: &str, sql: &str, schema: &MappingSchema, config: &LineageConfig) -> LineageResult<LineageGraph>` | `LineageGraph` | Parse SQL and build lineage in one call |

#### LineageConfig

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `dialect` | `Dialect` | `Ansi` | SQL dialect for parsing and identifier normalization |
| `trim_qualifiers` | `bool` | `true` | Remove table qualifiers from output node names |
| `sources` | `HashMap<String, String>` | `{}` | External source definitions (view name → SQL) |

| Method | Signature | Returns | Description |
| --- | --- | --- | --- |
| `new` | `(dialect: Dialect) -> LineageConfig` | `LineageConfig` | Create config with specified dialect |
| `with_sources` | `(self, sources: HashMap<String, String>) -> LineageConfig` | `LineageConfig` | Add external source definitions |
| `with_trim_qualifiers` | `(self, trim: bool) -> LineageConfig` | `LineageConfig` | Set qualifier trimming |

#### LineageNode

| Field | Type | Description |
| --- | --- | --- |
| `name` | `String` | Column/expression name (e.g., "a", "SUM(b)") |
| `expression` | `Option<Expr>` | The AST expression this node represents |
| `source_name` | `Option<String>` | Source table/CTE/subquery name |
| `source` | `Option<Expr>` | Reference to source AST |
| `downstream` | `Vec<LineageNode>` | Child nodes (upstream lineage sources) |
| `alias` | `Option<String>` | Alias if this is an aliased expression |
| `depth` | `usize` | Depth in the lineage graph (0 = root) |

| Method | Signature | Returns | Description |
| --- | --- | --- | --- |
| `new` | `(name: String) -> LineageNode` | `LineageNode` | Create a new lineage node |
| `with_source` | `(self, source_name: String) -> LineageNode` | `LineageNode` | Set source name |
| `with_expression` | `(self, expr: Expr) -> LineageNode` | `LineageNode` | Set expression |
| `with_depth` | `(self, depth: usize) -> LineageNode` | `LineageNode` | Set depth |
| `walk` | `(&self, visitor: &mut F)` | `()` | Pre-order traversal of all nodes |
| `iter` | `(&self) -> LineageIterator` | `LineageIterator` | Iterate over all nodes |
| `source_columns` | `(&self) -> Vec<&LineageNode>` | `Vec<&LineageNode>` | Get leaf nodes (source columns) |
| `source_tables` | `(&self) -> Vec<String>` | `Vec<String>` | Get all source table names |
| `to_dot` | `(&self) -> String` | `String` | Generate DOT format for Graphviz |
| `to_mermaid` | `(&self) -> String` | `String` | Generate Mermaid diagram |

#### LineageGraph

| Field | Type | Description |
| --- | --- | --- |
| `node` | `LineageNode` | Root node (the target output column) |
| `sql` | `Option<String>` | Original SQL (if using `lineage_sql`) |
| `dialect` | `Dialect` | Dialect used for analysis |

| Method | Signature | Returns | Description |
| --- | --- | --- | --- |
| `new` | `(node: LineageNode, dialect: Dialect) -> LineageGraph` | `LineageGraph` | Create a new graph |
| `source_tables` | `(&self) -> Vec<String>` | `Vec<String>` | Get all source table names |
| `source_columns` | `(&self) -> Vec<&LineageNode>` | `Vec<&LineageNode>` | Get leaf nodes |
| `to_dot` | `(&self) -> String` | `String` | Generate DOT format |
| `to_mermaid` | `(&self) -> String` | `String` | Generate Mermaid diagram |

#### LineageError

| Variant | Description |
| --- | --- |
| `ColumnNotFound(String)` | Target column not found in output columns |
| `AmbiguousColumn(String)` | Ambiguous column reference (multiple sources) |
| `InvalidQuery(String)` | Query structure not supported for lineage |
| `ParseError(String)` | SQL parsing failed |

#### Lineage Example

```rust
use sqlglot_rust::{parse, Dialect};
use sqlglot_rust::optimizer::lineage::{lineage_sql, LineageConfig};
use sqlglot_rust::schema::MappingSchema;
use sqlglot_rust::ast::DataType;

let mut schema = MappingSchema::new(Dialect::Ansi);
schema.add_table(&["orders"], vec![
    ("id".to_string(), DataType::Int),
    ("user_id".to_string(), DataType::Int),
    ("amount".to_string(), DataType::Double),
]).unwrap();

let config = LineageConfig::new(Dialect::Ansi);
let sql = "WITH totals AS (SELECT user_id, SUM(amount) AS total FROM orders GROUP BY user_id) \
           SELECT user_id, total FROM totals";

let graph = lineage_sql("total", sql, &schema, &config).unwrap();

// Root node is the output column
assert_eq!(graph.node.name, "total");

// Get source tables
let sources = graph.source_tables();
assert!(sources.contains(&"orders".to_string()));

// Generate visualization
println!("{}", graph.to_mermaid());
```

## Query Planner

The planner module generates a logical execution plan (a DAG of steps) from a parsed SQL AST. This sits between the optimizer and the executor, providing a structured representation of how a query should be executed.

### Plan / StepId

```rust
use sqlglot_rust::planner::{plan, Plan, StepId};
use sqlglot_rust::{parse, Dialect};

let ast = parse("SELECT a, b FROM t WHERE a > 1 ORDER BY b", Dialect::Ansi).unwrap();
let p = plan(&ast).unwrap();

// Inspect the plan
println!("Steps: {}", p.len());
println!("Root: {:?}", p.root());
println!("{p}"); // Display shows all steps and dependencies
```

| Method | Return Type | Description |
| -------- | ------------- | ------------- |
| `plan(&Statement)` | `Result<Plan>` | Build a plan from a parsed statement |
| `Plan::root()` | `StepId` | The root step that produces the final result |
| `Plan::steps()` | `&[Step]` | All steps in topological order |
| `Plan::get(StepId)` | `Option<&Step>` | Look up a step by ID |
| `Plan::len()` | `usize` | Number of steps |
| `Plan::is_empty()` | `bool` | Whether the plan has zero steps |
| `Plan::to_mermaid()` | `String` | Render as Mermaid flowchart |
| `Plan::to_dot()` | `String` | Render as DOT (Graphviz) digraph |

### Step Enum

Each step in the plan represents a logical operation.

| Variant | Description | Dependencies |
| --------- | ------------- | -------------- |
| `Scan` | Full table scan with optional filter pushdown | None (leaf) |
| `Filter` | WHERE / HAVING predicate evaluation | 1 input |
| `Project` | SELECT list evaluation | 1 input |
| `Aggregate` | GROUP BY + aggregate functions | 1 input |
| `Sort` | ORDER BY | 1 input |
| `Join` | Join two inputs (INNER, LEFT, RIGHT, FULL, CROSS) | 2 inputs |
| `Limit` | LIMIT / OFFSET | 1 input |
| `SetOperation` | UNION / INTERSECT / EXCEPT | 2 inputs |
| `Distinct` | DISTINCT elimination | 1 input |

All steps have:

- `dependencies()` → `&[StepId]` — IDs of input steps
- `projections()` → `&[Projection]` — output column projections
- `kind()` → `&str` — human-readable step type name

### Projection

```rust
pub struct Projection {
    pub expr: Expr,           // The expression being projected
    pub alias: Option<String>, // Output alias (if any)
}
```

### Visualization (Mermaid / DOT)

```rust
use sqlglot_rust::planner::plan;
use sqlglot_rust::{parse, Dialect};

let ast = parse(
    "SELECT a, SUM(b) FROM t JOIN u ON t.id = u.id WHERE a > 0 GROUP BY a ORDER BY a",
    Dialect::Ansi,
).unwrap();
let p = plan(&ast).unwrap();

// Mermaid flowchart (for docs, GitHub, etc.)
println!("{}", p.to_mermaid());
// graph TD
//     step_0["Scan(t)"]
//     step_1["Scan(u)"]
//     step_0 --> step_2
//     step_1 --> step_2
//     step_2["Join(Inner)"]
//     ...

// DOT / Graphviz digraph
println!("{}", p.to_dot());
// digraph plan {
//     rankdir=BT;
//     step_0 [label="Scan(t)"];
//     ...
// }
```

## AST Diff

Semantic comparison of SQL expression trees. Computes structured differences between two
parsed AST statements using a tree edit distance algorithm inspired by the Change Distiller
approach from Python sqlglot's `diff.py`.

Accessed via `use sqlglot_rust::diff::{diff, diff_sql, ChangeAction, AstNode}` or the
re-exported `use sqlglot_rust::{diff_ast, diff_sql, ChangeAction, AstNode}`.

| Function | Signature | Returns | Description |
| --- | --- | --- | --- |
| `diff` | `(source: &Statement, target: &Statement) -> Vec<ChangeAction>` | `Vec<ChangeAction>` | Compute semantic diff between two parsed ASTs |
| `diff_sql` | `(source_sql: &str, target_sql: &str, dialect: Dialect) -> Result<Vec<ChangeAction>>` | `Result<Vec<ChangeAction>>` | Parse two SQL strings and compute their diff |

### ChangeAction Enum

| Variant | Fields | Description |
| --- | --- | --- |
| `Remove` | `(AstNode)` | A node present in source that was removed |
| `Insert` | `(AstNode)` | A node inserted into target that was not in source |
| `Keep` | `(AstNode, AstNode)` | A node structurally identical in both trees |
| `Move` | `(AstNode, AstNode)` | A node moved to a different position in the tree |
| `Update` | `(AstNode, AstNode)` | A node in source replaced by a different node in target |

### AstNode Enum

Wraps AST nodes of different types for uniform diff output.

| Variant | Description |
| --- | --- |
| `Statement(Box<Statement>)` | A full SQL statement |
| `Expr(Expr)` | An expression node |
| `SelectItem(SelectItem)` | A SELECT list item |
| `JoinClause(JoinClause)` | A JOIN clause |
| `OrderByItem(OrderByItem)` | An ORDER BY item |
| `Cte(Box<Cte>)` | A Common Table Expression |
| `ColumnDef(ColumnDef)` | A column definition (DDL) |
| `TableConstraint(TableConstraint)` | A table constraint (DDL) |

**Example:**

```rust
use sqlglot_rust::{parse, Dialect};
use sqlglot_rust::diff::{diff, diff_sql, ChangeAction};

// Diff two parsed ASTs
let source = parse("SELECT a, b FROM t WHERE a > 1", Dialect::Ansi).unwrap();
let target = parse("SELECT a, c FROM t WHERE a > 2", Dialect::Ansi).unwrap();
let changes = diff(&source, &target);

for change in &changes {
    match change {
        ChangeAction::Keep(s, _t) => println!("  kept: {s}"),
        ChangeAction::Insert(n) => println!("+ insert: {n}"),
        ChangeAction::Remove(n) => println!("- remove: {n}"),
        ChangeAction::Update(s, t) => println!("~ update: {s} -> {t}"),
        ChangeAction::Move(s, t) => println!("⇄ move: {s} -> {t}"),
    }
}

// Or diff directly from SQL strings
let changes = diff_sql(
    "SELECT a FROM t",
    "SELECT a, b FROM t",
    Dialect::Ansi,
).unwrap();
assert!(changes.iter().any(|c| matches!(c, ChangeAction::Insert(_))));
```

---

## C/C++ FFI API

The library exposes a C-compatible API via `extern "C"` functions, allowing sqlglot-rust to be used from C, C++, Python (ctypes/cffi), Go (cgo), or any language that supports the C calling convention.

### Header

The C header is generated by `cbindgen` and placed at `target/ffi/include/sqlglot.h` (via `make ffi` or `make ffi-header`).

### Functions

| Function | Signature | Description |
| --- | --- | --- |
| `sqlglot_parse` | `char *sqlglot_parse(const char *sql, const char *dialect)` | Parse SQL into a JSON-serialised AST. Returns `NULL` on error. |
| `sqlglot_transpile` | `char *sqlglot_transpile(const char *sql, const char *from_dialect, const char *to_dialect)` | Transpile a single SQL statement between dialects. Returns `NULL` on error. |
| `sqlglot_generate` | `char *sqlglot_generate(const char *ast_json, const char *dialect)` | Generate SQL from a JSON-serialised `Statement`. Returns `NULL` on error. |
| `sqlglot_version` | `const char *sqlglot_version(void)` | Return the library version as a static string. **Do not free.** |
| `sqlglot_free` | `void sqlglot_free(char *ptr)` | Free a string returned by any `sqlglot_*` function. `NULL`-safe. |

### Memory Management

- Every non-`NULL` `char *` returned by `sqlglot_parse`, `sqlglot_transpile`, or `sqlglot_generate` **must** be freed by calling `sqlglot_free`.
- `sqlglot_version` returns a pointer to static memory — **do not** free it.
- Passing `NULL` to `sqlglot_free` is a safe no-op.

### Dialect Parameter

All `dialect` / `from_dialect` / `to_dialect` parameters accept a null-terminated string matching one of the [dialect names](#dialect-aliases-from_str) (case-insensitive). Pass `NULL` for ANSI SQL.

### Build Targets

| Make Target | Rust Target Triple | Platform |
| --- | --- | --- |
| `make ffi` | (host) | Current machine |
| `make ffi-macos-arm64` | `aarch64-apple-darwin` | macOS ARM64 |
| `make ffi-macos-amd64` | `x86_64-apple-darwin` | macOS x86-64 |
| `make ffi-linux-amd64` | `x86_64-unknown-linux-gnu` | Linux x86-64 |
| `make ffi-linux-arm64` | `aarch64-unknown-linux-gnu` | Linux ARM64 |
| `make ffi-all` | all four | All platforms |

### Output Artefacts

| File | Description |
| --- | --- |
| `target/ffi/include/sqlglot.h` | Auto-generated C header |
| `target/ffi/lib/libsqlglot_rust.a` | Static library |
| `target/ffi/lib/libsqlglot_rust.so` / `.dylib` | Shared (dynamic) library |

---

## SQL Execution Engine

The executor module provides an in-memory SQL execution engine that can run queries against Rust data structures. This is useful for testing SQL transformations, validating optimizer output, and unit-testing SQL pipelines without a real database.

### Value Enum

Represents a single cell value in the execution engine.

| Variant | Rust Type | Description |
| --------- | ----------- | ------------- |
| `Null` | — | SQL NULL |
| `Boolean(bool)` | `bool` | TRUE / FALSE |
| `Int(i64)` | `i64` | Integer numbers |
| `Float(f64)` | `f64` | Floating-point numbers |
| `String(String)` | `String` | Text values |

`Value` implements `Display`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`, `Clone`, and `Serialize`.

### Table / Tables

```rust
use sqlglot_rust::executor::{Table, Tables};

// A Table is a named collection of rows
let table = Table {
    columns: vec!["id".into(), "name".into(), "salary".into()],
    rows: vec![
        vec![Value::Int(1), Value::String("Alice".into()), Value::Float(100000.0)],
        vec![Value::Int(2), Value::String("Bob".into()),   Value::Float(95000.0)],
    ],
};

// Tables is a HashMap<String, Table>
let mut tables: Tables = std::collections::HashMap::new();
tables.insert("employees".to_string(), table);
```

### ResultSet

Returned by `execute` and `execute_statement`. Provides access to column names and rows.

| Method | Return Type | Description |
| -------- | ------------- | ------------- |
| `columns()` | `&[String]` | Column names in the result |
| `rows()` | `&[Vec<Value>]` | All result rows |
| `row_count()` | `usize` | Number of rows |
| `column_count()` | `usize` | Number of columns |

### execute / execute_statement

```rust
use sqlglot_rust::executor::{execute, Value, Table, Tables};
use std::collections::HashMap;

let mut tables: Tables = HashMap::new();
tables.insert("employees".into(), Table {
    columns: vec!["name".into(), "department".into(), "salary".into()],
    rows: vec![
        vec![Value::String("Alice".into()), Value::String("Engineering".into()), Value::Float(100000.0)],
        vec![Value::String("Bob".into()),   Value::String("Engineering".into()), Value::Float(95000.0)],
        vec![Value::String("Carol".into()), Value::String("Marketing".into()),   Value::Float(80000.0)],
    ],
});

// Execute a SQL string directly
let result = execute(
    "SELECT name, salary FROM employees WHERE department = 'Engineering' ORDER BY salary DESC",
    &tables,
).unwrap();

assert_eq!(result.row_count(), 2);
assert_eq!(result.rows()[0][0], Value::String("Alice".into()));
```

#### Supported SQL Features

| Feature | Details |
| --------- | --------- |
| **SELECT** | Column references, aliases, `*`, expressions, `DISTINCT` |
| **WHERE** | Comparison operators, `AND`/`OR`/`NOT`, `IN`, `BETWEEN`, `LIKE`, `IS NULL` |
| **JOIN** | `INNER`, `LEFT`, `RIGHT`, `FULL`, `CROSS`, `NATURAL` |
| **GROUP BY / HAVING** | Grouping with aggregate functions |
| **ORDER BY** | ASC/DESC, NULLS FIRST/LAST, positional references |
| **LIMIT / OFFSET** | Row limiting |
| **Aggregates** | `COUNT`, `SUM`, `AVG`, `MIN`, `MAX` (including `COUNT(*)`, `COUNT(DISTINCT)`) |
| **Scalar functions** | `UPPER`, `LOWER`, `LENGTH`, `CONCAT`, `ABS`, `CEIL`, `FLOOR`, `ROUND`, `SQRT`, `POWER`, `COALESCE`, `REPLACE`, `SUBSTRING`, `TRIM`, `LEFT`, `RIGHT`, `LPAD`, `RPAD`, `MOD`, `LN`, `LOG` |
| **Subqueries** | Scalar subqueries, `IN (SELECT ...)`, `EXISTS (SELECT ...)` |
| **CTEs** | `WITH ... AS (SELECT ...)` common table expressions |
| **Set operations** | `UNION`, `UNION ALL`, `INTERSECT`, `EXCEPT` |
| **CASE** | `CASE WHEN ... THEN ... ELSE ... END` |
| **CAST** | Type conversions between INT, FLOAT, STRING, BOOLEAN |
| **EXTRACT** | `EXTRACT(field FROM value)` for date-part extraction |
