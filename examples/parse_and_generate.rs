//! Parse SQL into an AST and generate it back.
//!
//! Run with: `cargo run --example parse_and_generate`

use sqlglot_rust::{Dialect, Statement, generate, generate_pretty, parse};

fn main() {
    let sql = "SELECT id, name, salary * 1.1 AS adjusted FROM employees WHERE department = 'Engineering' ORDER BY salary DESC LIMIT 10";

    // Parse into AST
    let ast = parse(sql, Dialect::Ansi).unwrap();

    // Compact output
    println!("=== Compact ===");
    println!("{}\n", generate(&ast, Dialect::Ansi));

    // Pretty-printed output
    println!("=== Pretty ===");
    println!("{}\n", generate_pretty(&ast, Dialect::Ansi));

    // Inspect the AST
    println!("=== AST Debug ===");
    if let Statement::Select(ref s) = ast {
        println!("Columns: {}", s.columns.len());
        println!("Has WHERE: {}", s.where_clause.is_some());
        println!("ORDER BY items: {}", s.order_by.len());
        println!("LIMIT: {:?}", s.limit);
    }
}
