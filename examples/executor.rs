//! Execute SQL queries against in-memory tables.
//!
//! Run with: `cargo run --example executor`

use sqlglot_rust::executor::{Table, Tables, Value, execute};
use std::collections::HashMap;

fn main() {
    let mut tables: Tables = HashMap::new();

    tables.insert(
        "employees".into(),
        Table::from_rows(
            vec!["id", "name", "department", "salary"],
            vec![
                vec![
                    Value::Int(1),
                    Value::String("Alice".into()),
                    Value::String("Engineering".into()),
                    Value::Float(120000.0),
                ],
                vec![
                    Value::Int(2),
                    Value::String("Bob".into()),
                    Value::String("Engineering".into()),
                    Value::Float(110000.0),
                ],
                vec![
                    Value::Int(3),
                    Value::String("Carol".into()),
                    Value::String("Marketing".into()),
                    Value::Float(95000.0),
                ],
                vec![
                    Value::Int(4),
                    Value::String("Dave".into()),
                    Value::String("Marketing".into()),
                    Value::Float(88000.0),
                ],
                vec![
                    Value::Int(5),
                    Value::String("Eve".into()),
                    Value::String("Engineering".into()),
                    Value::Float(130000.0),
                ],
            ],
        ),
    );

    // Aggregate query
    println!("=== Average salary by department ===");
    let result = execute(
        "SELECT department, COUNT(*) AS cnt, AVG(salary) AS avg_sal FROM employees GROUP BY department ORDER BY avg_sal DESC",
        &tables,
    ).unwrap();
    for col in &result.columns {
        print!("{col:<15}");
    }
    println!();
    for row in &result.rows {
        for val in row {
            let s = val.to_string();
            print!("{s:<15}");
        }
        println!();
    }

    // Filtered query
    println!("\n=== Engineers earning > 115k ===");
    let result = execute(
        "SELECT name, salary FROM employees WHERE department = 'Engineering' AND salary > 115000 ORDER BY salary DESC",
        &tables,
    ).unwrap();
    for row in &result.rows {
        println!("  {} — ${}", row[0], row[1]);
    }
}
