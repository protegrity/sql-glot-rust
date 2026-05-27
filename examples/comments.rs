//! Parse and preserve SQL comments through transpilation.
//!
//! Run with: `cargo run --example comments`

use sqlglot_rust::{Dialect, Statement, generate, parse_with_comments, transpile_with_comments};

fn main() {
    // Parse with comments preserved
    let sql = "-- fetch active users\n/* priority: high */\nSELECT id, name FROM users WHERE active = TRUE";
    println!("=== Input ===");
    println!("{sql}\n");

    let stmt = parse_with_comments(sql, Dialect::Ansi).unwrap();
    if let Statement::Select(ref s) = stmt {
        println!("=== Attached comments ===");
        for c in &s.comments {
            println!("  {c}");
        }
    }

    // Roundtrip: comments survive generate
    println!("\n=== Generated output ===");
    println!("{}\n", generate(&stmt, Dialect::Ansi));

    // Transpile with comments: MySQL → PostgreSQL
    let mysql_sql = "# get user count\nSELECT IFNULL(name, 'unknown') FROM users";
    println!("=== MySQL → PostgreSQL (with comments) ===");
    println!("IN:  {mysql_sql}");
    let result = transpile_with_comments(mysql_sql, Dialect::Mysql, Dialect::Postgres).unwrap();
    println!("OUT: {result}");
    println!("(# comment converted to -- for PostgreSQL)");
}
