use sqlglot_rust::{Dialect, generate, parse};

fn main() {
    // Simulate what normalize_table_schema does: parse as TSQL, generate as TSQL
    let sqls = vec![
        "SELECT 1 AS t, 0 AS f",
        "SELECT CASE WHEN 1 = 1 THEN 1 ELSE 0 END AS flag",
    ];
    for sql in &sqls {
        let stmt = parse(sql, Dialect::Tsql).unwrap();
        let result = generate(&stmt, Dialect::Tsql);
        println!("IN:  {}", sql);
        println!("OUT: {}", result);
        println!();
    }
}
