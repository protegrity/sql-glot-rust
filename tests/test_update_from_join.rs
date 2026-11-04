use sqlglot_rust::{Dialect, generate, parse};

#[test]
fn psq5302_update_from_join_roundtrip() {
    for sql in [
        "UPDATE c SET c.nationality = 'PT' FROM customers c JOIN customers s ON c.customer_id = s.customer_id WHERE s.customer_id = 'CUST-000001'",
        "UPDATE c SET c.email = 'join.update@example.com', c.nationality = 'PT' FROM customers c JOIN (SELECT 'QAW02-003' AS k) s ON c.customer_id = s.k",
        "UPDATE c SET nationality = 'PT' FROM customers c LEFT JOIN customers s ON c.customer_id = s.customer_id WHERE s.customer_id IS NULL",
    ] {
        for dialect in [Dialect::Postgres, Dialect::Tsql] {
            let statement = parse(sql, dialect).expect("UPDATE FROM JOIN must parse");
            let rendered = generate(&statement, dialect);
            assert!(rendered.contains("JOIN"), "{rendered}");
            assert_eq!(parse(&rendered, dialect).unwrap(), statement);
        }
    }
}
