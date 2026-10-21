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
applies when the parser records an explicit `WITH [NO] DATA` clause. Other
query forms, or unsupported clauses such as `OR REPLACE`, make the parser
return the full statement as `Statement::Command`.

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
