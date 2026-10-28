# CREATE TABLE AS (CTAS) with or without properties

## Parsing

`parse()` returns supported CTAS statements as `Statement::CreateTable`.
The contained `CreateTableStatement` exposes the query through `as_select`.

Parsing this Athena Hive CTAS statement records `format` and `external_location`
in `with_properties`, and `SELECT 1 AS id` in `as_select`:

```sql
CREATE TABLE target_table
WITH (
    format = 'PARQUET',
    external_location = 's3://example-bucket/target_table/'
)
AS SELECT 1 AS id
```

This Athena Hive statement creates a table without rows.
The parser records the explicit `WITH NO DATA` clause as `with_data = Some(false)`:

```sql
CREATE TABLE empty_table
AS SELECT 1 AS id
WITH NO DATA
```

Properties use `name = expression` syntax. For CTAS with properties, the query
must parse as `Statement::Select` or `Statement::SetOperation`. The same check
applies when the parser records an explicit `WITH [NO] DATA` clause. For those
statements, other query forms or unsupported clauses such as `OR REPLACE`
make the parser return the full statement as `Statement::Command`.

CTAS also falls back to `Statement::Command` for known query constructs the
parser accepts without representing in the AST, including `DISTINCT ON`,
`FETCH ... WITH TIES`, `TABLESAMPLE`, wildcard exclusions, and `UNION BY NAME`.
This includes such clauses inside CTEs, subqueries, and set-operation branches.
Keeping the full statement preserves the original query instead of emitting
SQL with silently changed results. Callers must check the statement variant;
successful parsing is not a guarantee of structured support or SQL validity.

## SQL generation

`generate()` writes a CTAS AST back to SQL, keeping the property order.
Quoted property names use the target dialect's quoting style. Explicit
`WITH DATA` and `WITH NO DATA` clauses are also preserved.

## Transpilation

`transpile()` converts property expressions using the target dialect's rules.
It does not check whether the target supports the CTAS clauses or property
values, and it does not translate property names. Statements parsed as
`Statement::Command` are written without dialect conversion.

Regression: `cargo test --test test_ctas_properties`.

## AST comparison

`diff()` and `diff_sql()` report a whole-statement update when the CTAS query,
properties, or population clause changes. Query changes include projections,
source tables, filters, and set-operation branches.

## Migrating to 0.11

This release adds public fields to `CreateTableStatement`, so it uses a new
0.x minor version. Existing Rust struct literals must initialize
`with_properties: Vec::new()` and `with_data: None` unless those clauses are
needed. Exhaustive patterns must bind the new fields or include `..`:

```rust
use sqlglot_rust::ast::{CreateTableStatement, Statement};
use sqlglot_rust::{parse, Dialect};

let statement = parse("CREATE TABLE x AS SELECT 1", Dialect::Ansi).unwrap();
if let Statement::CreateTable(CreateTableStatement { as_select, .. }) = statement {
    assert!(as_select.is_some());
}
```

Older JSON ASTs remain readable: missing properties default to an empty vector
and missing population state defaults to `None`. This JSON compatibility does
not make old Rust struct literals source compatible. Empty/omitted values are
not serialized. Consumers of the C API should accept the additional JSON fields;
the exported C function signatures are unchanged.
