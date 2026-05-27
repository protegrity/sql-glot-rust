//! Walk, find, and transform expressions in the AST.
//!
//! Run with: `cargo run --example ast_traversal`

use sqlglot_rust::{Dialect, Expr, Statement, generate, parse};

fn main() {
    let sql = "SELECT a, b + 1, UPPER(name) FROM users WHERE age > 21 AND status = 'active'";
    let ast = parse(sql, Dialect::Ansi).unwrap();

    if let Statement::Select(ref s) = ast {
        let where_expr = s.where_clause.as_ref().unwrap();

        // Walk: collect all column names referenced in WHERE
        println!("=== Columns in WHERE ===");
        let mut columns = Vec::new();
        where_expr.walk(&mut |expr| {
            if let Expr::Column { name, .. } = expr {
                columns.push(name.clone());
            }
            true
        });
        println!("{columns:?}\n");

        // Find: locate first string literal
        println!("=== First string literal ===");
        if let Some(lit) = where_expr.find(&|e| matches!(e, Expr::StringLiteral(_))) {
            println!("{lit:?}\n");
        }

        // Find all: every literal in the query
        println!("=== All literals in WHERE ===");
        let literals = where_expr.find_all(&|e| e.is_literal());
        for lit in &literals {
            println!("  {}", lit.sql());
        }
    }

    // Transform: rename column "a" → "id"
    println!("\n=== Transform: rename column a → id ===");
    if let Statement::Select(s) = ast {
        let transformed = Statement::Select(sqlglot_rust::ast::SelectStatement {
            columns: s
                .columns
                .into_iter()
                .map(|item| match item {
                    sqlglot_rust::ast::SelectItem::Expr {
                        expr,
                        alias,
                        alias_quote_style,
                    } => sqlglot_rust::ast::SelectItem::Expr {
                        expr: expr.transform(&|e| match e {
                            Expr::Column {
                                name,
                                table,
                                quote_style,
                                table_quote_style,
                            } if name == "a" => Expr::Column {
                                name: "id".to_string(),
                                table,
                                quote_style,
                                table_quote_style,
                            },
                            other => other,
                        }),
                        alias,
                        alias_quote_style,
                    },
                    other => other,
                })
                .collect(),
            ..s
        });
        println!("{}", generate(&transformed, Dialect::Ansi));
    }
}
