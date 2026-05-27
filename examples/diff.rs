//! Compare two SQL queries and show semantic differences.
//!
//! Run with: `cargo run --example diff`

use sqlglot_rust::Dialect;
use sqlglot_rust::diff::{ChangeAction, diff_sql};

fn main() {
    let original = "SELECT a, b FROM t WHERE a > 1 ORDER BY a";
    let modified = "SELECT a, c, d FROM t WHERE a > 5 ORDER BY a DESC";

    println!("Original: {original}");
    println!("Modified: {modified}\n");

    let changes = diff_sql(original, modified, Dialect::Ansi).unwrap();

    println!("=== Changes ({} total) ===", changes.len());
    for change in &changes {
        match change {
            ChangeAction::Remove(node) => println!("  - REMOVE: {node:?}"),
            ChangeAction::Insert(node) => println!("  + INSERT: {node:?}"),
            ChangeAction::Keep(node, _) => println!("    KEEP:   {node:?}"),
            ChangeAction::Move(node, _) => println!("  ~ MOVE:   {node:?}"),
            ChangeAction::Update(old, new) => {
                println!("  * UPDATE: {old:?}");
                println!("         → {new:?}");
            }
        }
    }
}
