# sqlglot-rust 0.11.0

CTAS statements can now represent ordered `WITH (name = expression)` properties
and explicit `WITH DATA` / `WITH NO DATA` clauses in the AST. Parsing, SQL
generation, expression traversal, dialect conversion, optimization, and AST
comparison handle these fields.

## Compatibility and migration

This release adds public fields to `CreateTableStatement`. Rust callers that
construct this struct must add `with_properties: Vec::new()` and `with_data: None`
when no clauses are needed. Exhaustive destructuring must account for the new
fields or use `..`. See the [CTAS guide and migration example](ctas-with-properties.md).

Older JSON ASTs remain readable through defaulted fields. C function signatures
are unchanged; JSON consumers must accept the additional fields.

## Correctness fixes

- Preserve the full CTAS as `Statement::Command` for known query constructs
  without an equivalent AST representation, including `DISTINCT ON`,
  `FETCH ... WITH TIES`, `TABLESAMPLE`, wildcard exclusions, and `UNION BY NAME`.
  Detection covers nested CTEs, subqueries, and set-operation branches.
- Report CTAS query-only changes in AST diffs, including changes to projections,
  source tables, filters, and set-operation branches.
- Keep property order, quoted names, population clauses, and older JSON input
  compatibility covered by regression tests.

`Command` statements preserve SQL text and do not undergo dialect conversion.
Parsing success does not establish SQL validity, dialect support, or execution
equivalence. Property names and values are not validated against a database.

## Maintenance

Repository formatting and existing Clippy warnings are addressed. Four narrowly
scoped, documented Clippy exceptions retain existing public API shapes. A new
quality workflow checks formatting, denies Clippy warnings, runs tests with and
without CLI features, and exercises CTAS benchmark cases. Lint checks use Rust
1.89.0 for reproducibility; platform build workflows continue to use stable Rust.

The API reference and documentation navigation now include the CTAS fields and
migration guidance. Dependency versions are unchanged.
