//! Transpile SQL between dialects.
//!
//! Run with: `cargo run --example transpile`

use sqlglot_rust::{Dialect, transpile};

fn main() {
    let examples = [
        // Function mapping
        (
            "SELECT NOW()",
            Dialect::Postgres,
            Dialect::Tsql,
            "NOW → GETDATE",
        ),
        (
            "SELECT IFNULL(a, b) FROM t",
            Dialect::Mysql,
            Dialect::Postgres,
            "IFNULL → COALESCE",
        ),
        // LIMIT → TOP
        (
            "SELECT * FROM t LIMIT 5",
            Dialect::Mysql,
            Dialect::Tsql,
            "LIMIT → TOP",
        ),
        // Identifier quoting
        (
            "SELECT \"user\" FROM t",
            Dialect::Postgres,
            Dialect::Mysql,
            "Double-quote → backtick",
        ),
        // ILIKE rewriting
        (
            "SELECT * FROM t WHERE name ILIKE '%test%'",
            Dialect::Postgres,
            Dialect::Mysql,
            "ILIKE → LOWER/LIKE",
        ),
        // Data types
        (
            "CREATE TABLE t (id SERIAL)",
            Dialect::Postgres,
            Dialect::Mysql,
            "SERIAL → INT AUTO_INCREMENT",
        ),
    ];

    for (sql, from, to, label) in examples {
        let result = transpile(sql, from, to).unwrap();
        println!("[{label}]");
        println!("  {from:?} → {to:?}");
        println!("  IN:  {sql}");
        println!("  OUT: {result}\n");
    }
}
