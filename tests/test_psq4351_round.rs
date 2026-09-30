use sqlglot_rust::{Dialect, transpile};

#[test]
fn round_default_scale_is_supplied_only_for_tsql() {
    let sql = "SELECT ROUND(revenue / 50) FROM sales";
    assert_eq!(transpile(sql, Dialect::Postgres, Dialect::Tsql).unwrap(), "SELECT ROUND(revenue / 50, 0) FROM sales");
    assert_eq!(transpile(sql, Dialect::Postgres, Dialect::Postgres).unwrap(), sql);
}

#[test]
fn explicit_negative_scale_is_preserved() {
    let sql = "SELECT ROUND(revenue, -2) FROM sales";
    assert_eq!(transpile(sql, Dialect::Postgres, Dialect::Tsql).unwrap(), sql);
}
