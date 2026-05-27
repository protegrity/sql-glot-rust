//! Build SQL queries programmatically with the fluent builder API.
//!
//! Run with: `cargo run --example builder`

use sqlglot_rust::builder::*;
use sqlglot_rust::{Dialect, generate};

fn main() {
    // Simple SELECT
    let stmt = select(&["id", "name", "email"])
        .from("users")
        .where_clause("active = TRUE")
        .order_by(&["name"])
        .limit(10)
        .build();
    println!("=== Simple SELECT ===");
    println!("{}\n", generate(&stmt, Dialect::Ansi));

    // SELECT with JOIN
    let stmt = select(&["u.name", "o.total"])
        .from("users u")
        .join("orders o", "o.user_id = u.id")
        .where_clause("o.total > 100")
        .order_by(&["o.total"])
        .build();
    println!("=== JOIN ===");
    println!("{}\n", generate(&stmt, Dialect::Ansi));

    // Expression-level construction
    let id_col = column("id", None);
    let salary = column("salary", None);
    let threshold = literal(50000);
    let condition = gt(salary, threshold);
    println!("=== Expression ===");
    println!("Column: {}", id_col.sql());
    println!("Condition: {}\n", condition.sql());

    // SELECT with subquery
    let stmt = select(&["name", "salary"])
        .from("employees")
        .where_clause("department_id IN (SELECT id FROM departments WHERE active = TRUE)")
        .build();
    println!("=== Subquery in WHERE ===");
    println!("{}", generate(&stmt, Dialect::Ansi));
}
