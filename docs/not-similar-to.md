# NOT SIMILAR TO comparisons (PSQ-4135)

`expression NOT SIMILAR TO pattern [ESCAPE character]` parses as a negated
`SimilarTo` expression, including inside AND/OR conditions. Its pattern and
escape survive parse/generate round trips. The generic NOT IN/LIKE/ILIKE/BETWEEN
branch consumes NOT only for those operators, allowing the existing SIMILAR TO
and REGEXP/RLIKE handlers to run. Malformed comparisons remain parse errors.

Regression: `cargo test --test test_not_similar_to` (observed failing before the
parser change). Full validation: `cargo test --features cli --lib --tests`.
