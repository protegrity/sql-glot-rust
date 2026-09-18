use sqlglot_rust::{generate, parse, Dialect, Expr, Statement};

#[test]
fn negated_similar_to_preserves_pattern_escape_and_negation() {
    for dialect in [Dialect::Postgres, Dialect::Ansi, Dialect::Redshift] {
        for suffix in ["", " ESCAPE '!'", " AND active = true", " OR active = false"] {
            let sql = format!("SELECT nationality FROM customers WHERE nationality NOT SIMILAR TO '[0-9]+'{suffix}");
            let statement = parse(&sql, dialect).unwrap_or_else(|e| panic!("{sql}: {e}"));
            let positive = parse(&sql.replace(" NOT SIMILAR", " SIMILAR"), dialect).unwrap();
            assert_ne!(statement, positive, "negation must not be discarded");
            let output = generate(&statement, dialect);
            assert_eq!(parse(&output, dialect).unwrap(), statement);
            if suffix.is_empty() || suffix.starts_with(" ESCAPE") {
                let Statement::Select(select) = statement else { panic!("expected SELECT") };
                let Some(Expr::SimilarTo { negated, escape, .. }) = select.where_clause else { panic!("expected SIMILAR TO") };
                assert!(negated);
                assert_eq!(escape.is_some(), !suffix.is_empty());
            }
        }
    }
}

#[test]
fn existing_negated_comparisons_still_round_trip_and_invalid_not_is_rejected() {
    for predicate in ["x NOT IN (1, 2)", "x NOT LIKE 'a%'", "x NOT ILIKE 'a%'", "x NOT BETWEEN 1 AND 2", "NOT (x SIMILAR TO 'a%')", "x NOT REGEXP 'a'", "x NOT RLIKE 'a'"] {
        let statement = parse(&format!("SELECT x FROM t WHERE {predicate}"), Dialect::Postgres).unwrap();
        let output = generate(&statement, Dialect::Postgres);
        assert_eq!(parse(&output, Dialect::Postgres).unwrap(), statement);
    }
    for predicate in ["x NOT SIMILAR 'a'", "x NOT SIMILAR TO", "x NOT = 1"] {
        assert!(parse(&format!("SELECT x FROM t WHERE {predicate}"), Dialect::Postgres).is_err());
    }
}
