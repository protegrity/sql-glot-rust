use sqlglot_rust::{parse, generate, Dialect};
use sqlglot_rust::tokens::Tokenizer;

#[test]
fn regex_operators_preserve_pattern_flags_negation_and_precedence() {
    for (op, function) in [
        ("~", "REGEXP_LIKE(name, '^A')"),
        ("~*", "REGEXP_LIKE(name, '^A', 'i')"),
        ("!~", "NOT REGEXP_LIKE(name, '^A')"),
        ("!~*", "NOT REGEXP_LIKE(name, '^A', 'i')"),
    ] {
        for tail in ["", " AND active = TRUE", " OR id = 2"] {
            let sql = format!("SELECT name FROM people WHERE name {op} '^A'{tail}");
            let ast = parse(&sql, Dialect::Postgres).unwrap();
            let expected = parse(&format!("SELECT name FROM people WHERE {function}{tail}"), Dialect::Postgres).unwrap();
            assert_eq!(ast, expected, "{sql}");
            assert_eq!(parse(&generate(&ast, Dialect::Postgres), Dialect::Postgres).unwrap(), ast);
        }
    }
}

#[test]
fn literals_comments_unicode_and_bitwise_not_remain_distinct() {
    let sql = "SELECT 'é !~*' AS label, \"!~\" FROM people WHERE (name !~* 'A''B') /* !~ */";
    let expected = "SELECT 'é !~*' AS label, \"!~\" FROM people WHERE (NOT REGEXP_LIKE(name, 'A''B', 'i')) /* !~ */";
    assert_eq!(parse(sql, Dialect::Postgres).unwrap(), parse(expected, Dialect::Postgres).unwrap());
    assert!(parse("SELECT ~flags FROM settings", Dialect::Postgres).is_ok());
    assert!(Tokenizer::new("SELECT !name").tokenize().is_err());
    assert!(parse("SELECT name FROM people WHERE name !~", Dialect::Postgres).is_err());
}
