use sqlglot_rust::{generate, parse, Dialect};

#[test]
fn postgres_array_dimensions_and_cardinality_survive_regeneration() {
    for sql in [
        "SELECT ARRAY_LENGTH(ARRAY_AGG(customer_id), 1) FROM customers",
        "SELECT ARRAY_LENGTH(STRING_TO_ARRAY(customer_id, '-'), 1) FROM customers",
        "SELECT ARRAY_LENGTH(ARRAY['a', 'b'], 1) FROM customers",
        "SELECT ARRAY_LENGTH(matrix, 2) FROM customers",
        "SELECT ARRAY_LENGTH(matrix, dimension) FROM customers",
        "SELECT CARDINALITY(ARRAY_AGG(customer_id)) FROM customers",
        "SELECT CARDINALITY(ARRAY[]) FROM customers",
        "SELECT CARDINALITY(matrix) FROM customers",
    ] {
        let ast = parse(sql, Dialect::Postgres).unwrap();
        assert_eq!(generate(&ast, Dialect::Postgres), sql);
    }
}

#[test]
fn unrecognized_array_arities_are_not_silently_truncated() {
    for sql in [
        "SELECT CARDINALITY(arr, 2) FROM t",
        "SELECT ARRAY_SIZE(arr, 2) FROM t",
        "SELECT ARRAY_LENGTH(arr, 1, 2) FROM t",
    ] {
        assert_eq!(generate(&parse(sql, Dialect::Postgres).unwrap(), Dialect::Postgres), sql);
    }
}
