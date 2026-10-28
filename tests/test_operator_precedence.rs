//! Regression coverage for #39. Expected tiers come from the database manuals,
//! independently of the parser/generator implementation.
use sqlglot_rust::ast::{BinaryOperator as Op, SelectItem};
use sqlglot_rust::parser::parse_statements;
use sqlglot_rust::{
    Dialect, Expr, Statement, generate, generate_pretty, optimizer, parse,
    parse_statements_with_comments, parse_with_comments, transpile,
};

fn expression(stmt: Statement) -> Expr {
    match stmt {
        Statement::Select(s) => match s.columns.into_iter().next().unwrap() {
            SelectItem::Expr { expr, .. } => expr,
            _ => panic!("expected expression"),
        },
        Statement::Expression(expr) => expr,
        _ => panic!("expected SELECT"),
    }
}

fn normalized(expr: Expr) -> Expr {
    expr.transform(&|e| match e {
        Expr::Nested(inner) | Expr::Commented { expr: inner, .. } => *inner,
        other => other,
    })
}

fn number(n: i32) -> Expr {
    Expr::Number(n.to_string())
}

fn binary(left: Expr, op: &Op, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: op.clone(),
        right: Box::new(right),
    }
}

fn tiers(dialect: Dialect) -> Vec<(&'static str, Op, u8)> {
    match dialect {
        Dialect::Sqlite => vec![
            ("||", Op::Concat, 4),
            ("*", Op::Multiply, 3),
            ("/", Op::Divide, 3),
            ("%", Op::Modulo, 3),
            ("+", Op::Plus, 2),
            ("-", Op::Minus, 2),
            ("&", Op::BitwiseAnd, 1),
            ("|", Op::BitwiseOr, 1),
            ("<<", Op::ShiftLeft, 1),
            (">>", Op::ShiftRight, 1),
        ],
        Dialect::Mysql => vec![
            ("^", Op::BitwiseXor, 6),
            ("*", Op::Multiply, 5),
            ("/", Op::Divide, 5),
            ("%", Op::Modulo, 5),
            ("+", Op::Plus, 4),
            ("-", Op::Minus, 4),
            ("<<", Op::ShiftLeft, 3),
            (">>", Op::ShiftRight, 3),
            ("&", Op::BitwiseAnd, 2),
            ("|", Op::BitwiseOr, 1),
        ],
        _ => unreachable!(),
    }
}

#[test]
fn every_operator_pair_has_the_documented_tree() {
    for dialect in [Dialect::Sqlite, Dialect::Mysql] {
        let ops = tiers(dialect);
        for (a, op_a, prec_a) in &ops {
            for (b, op_b, prec_b) in &ops {
                let left = binary(binary(number(17), op_a, number(3)), op_b, number(1));
                let right = binary(number(17), op_a, binary(number(3), op_b, number(1)));
                let expected = if prec_a >= prec_b { &left } else { &right };
                for (sql, tree) in [
                    (format!("SELECT 17 {a} 3 {b} 1"), expected),
                    (format!("SELECT (17 {a} 3) {b} 1"), &left),
                    (format!("SELECT 17 {a} (3 {b} 1)"), &right),
                ] {
                    assert_eq!(
                        &normalized(expression(parse(&sql, dialect).unwrap())),
                        tree,
                        "{dialect}: {sql}"
                    );
                }
            }
        }
    }
}

#[test]
fn generated_sql_preserves_both_child_groupings_without_nested_nodes() {
    for dialect in [Dialect::Sqlite, Dialect::Mysql] {
        let ops = tiers(dialect);
        for (_, op_a, _) in &ops {
            for (_, op_b, _) in &ops {
                for tree in [
                    binary(binary(number(17), op_a, number(3)), op_b, number(1)),
                    binary(number(17), op_a, binary(number(3), op_b, number(1))),
                ] {
                    let stmt = Statement::Expression(tree.clone());
                    for sql in [generate(&stmt, dialect), generate_pretty(&stmt, dialect)] {
                        assert_eq!(
                            normalized(expression(
                                parse(&format!("SELECT {sql}"), dialect).unwrap()
                            )),
                            tree,
                            "{dialect}: {sql}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn all_parse_entry_points_keep_the_source_dialect() {
    for (dialect, sql, expected) in [
        (
            Dialect::Sqlite,
            "SELECT 1 | 2 & 4",
            binary(
                binary(number(1), &Op::BitwiseOr, number(2)),
                &Op::BitwiseAnd,
                number(4),
            ),
        ),
        (
            Dialect::Sqlite,
            "SELECT 1 || 2 * 3",
            binary(
                binary(number(1), &Op::Concat, number(2)),
                &Op::Multiply,
                number(3),
            ),
        ),
        (
            Dialect::Mysql,
            "SELECT 2 ^ 3 * 4",
            binary(
                binary(number(2), &Op::BitwiseXor, number(3)),
                &Op::Multiply,
                number(4),
            ),
        ),
    ] {
        let with_comment = format!("/* issue 39 */ {sql}");
        let mut statements = vec![
            parse(sql, dialect).unwrap(),
            parse_with_comments(&with_comment, dialect).unwrap(),
        ];
        statements.extend(parse_statements(&format!("{sql}; {sql}"), dialect).unwrap());
        statements.extend(
            parse_statements_with_comments(&format!("{with_comment}; {sql}"), dialect).unwrap(),
        );
        for stmt in statements {
            assert_eq!(normalized(expression(stmt)), expected);
        }
    }
}

#[test]
fn optimization_preserves_the_reported_expressions() {
    for (dialect, sql) in [
        (Dialect::Sqlite, "SELECT 1 | 2 & 4"),
        (Dialect::Sqlite, "SELECT 1 || 2 * 3"),
        (Dialect::Mysql, "SELECT 2 ^ 3 * 4"),
        (Dialect::Mysql, "SELECT 2 * 3 ^ 4"),
    ] {
        let stmt = parse(sql, dialect).unwrap();
        let expected = normalized(expression(stmt.clone()));
        let output = generate(&optimizer::optimize(stmt).unwrap(), dialect);
        assert_eq!(
            normalized(expression(parse(&output, dialect).unwrap())),
            expected,
            "{output}"
        );
    }
}

#[test]
fn cross_dialect_bitwise_grouping_is_preserved() {
    assert_eq!(
        transpile("SELECT 1 | 2 & 4", Dialect::Mysql, Dialect::Sqlite).unwrap(),
        "SELECT 1 | (2 & 4)"
    );
    assert_eq!(
        transpile("SELECT 1 | 2 & 4", Dialect::Sqlite, Dialect::Mysql).unwrap(),
        "SELECT (1 | 2) & 4"
    );
}

#[test]
fn unary_comparison_and_nested_query_boundaries_are_preserved() {
    for dialect in [Dialect::Sqlite, Dialect::Mysql] {
        for sql in [
            "SELECT -2 * 3 + 1",
            "SELECT ~2 & 3 + 1",
            "SELECT 1 + 2 > 2 AND 4 | 1 = 5",
            "SELECT (SELECT 1 | 2 & 4) FROM (SELECT 2 ^ 3 * 4) AS q",
        ] {
            let stmt = parse(sql, dialect).unwrap();
            let output = generate(&stmt, dialect);
            assert_eq!(parse(&output, dialect).unwrap(), stmt, "{dialect}: {sql}");
        }
    }
}

#[test]
fn mysql_default_mode_distinguishes_pipes_from_concatenation() {
    let pipes = expression(parse("SELECT 0 || 1 * 0", Dialect::Mysql).unwrap());
    assert_eq!(
        pipes,
        binary(
            number(0),
            &Op::Or,
            binary(number(1), &Op::Multiply, number(0))
        )
    );
    assert_eq!(
        transpile("SELECT 'a' || 'b'", Dialect::Sqlite, Dialect::Mysql).unwrap(),
        "SELECT CONCAT('a', 'b')"
    );
    assert_eq!(
        transpile("SELECT 0 || 1 * 0", Dialect::Mysql, Dialect::Sqlite).unwrap(),
        "SELECT 0 OR 1 * 0"
    );
}

#[test]
fn other_dialects_keep_their_existing_operator_handling() {
    let sql = "SELECT 1 | 2 & 4";
    let legacy = binary(
        number(1),
        &Op::BitwiseOr,
        binary(number(2), &Op::BitwiseAnd, number(4)),
    );
    for dialect in [
        Dialect::Ansi,
        Dialect::Postgres,
        Dialect::Tsql,
        Dialect::Oracle,
    ] {
        assert_eq!(expression(parse(sql, dialect).unwrap()), legacy);
    }
    for sql in [
        "SELECT a ^@ b",
        "SELECT a <<| b",
        "SELECT a >>| b",
        "SELECT a &< b",
        "SELECT a && b",
    ] {
        let stmt = parse(sql, Dialect::Postgres).unwrap();
        assert_eq!(
            parse(&generate(&stmt, Dialect::Postgres), Dialect::Postgres).unwrap(),
            stmt
        );
    }
}

#[test]
fn mysql_double_ampersand_stays_at_logical_precedence() {
    let sql = "SELECT 0 || 1 & 2 && 3 | 4";
    let expected = binary(
        number(0),
        &Op::Or,
        binary(
            binary(number(1), &Op::BitwiseAnd, number(2)),
            &Op::And,
            binary(number(3), &Op::BitwiseOr, number(4)),
        ),
    );
    assert_eq!(expression(parse(sql, Dialect::Mysql).unwrap()), expected);
}
