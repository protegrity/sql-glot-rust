use sqlglot_rust::{Dialect, generate_pretty, parse};

#[test]
fn pretty_generation_formats_sql() {
    let ast = parse("SELECT a, b FROM events WHERE active = true", Dialect::Ansi).unwrap();

    let generated = generate_pretty(&ast, Dialect::Ansi);

    assert!(generated.contains("SELECT\n"));
    assert!(generated.contains("\nFROM\n"));
    assert!(generated.contains("\nWHERE\n"));
}

#[test]
fn pretty_generation_preserves_the_target_dialect() {
    let ast = parse("SELECT `select` FROM `events`", Dialect::Mysql).unwrap();

    let generated = generate_pretty(&ast, Dialect::Tsql);

    assert!(generated.contains("[select]"));
    assert!(generated.contains("[events]"));
}
