//! Shared source/target precedence for the dialects covered by issue #39.
//! Higher numbers bind more tightly. Keep other dialects on their existing
//! paths until their full operator rules are covered by conformance tests.
use crate::{Dialect, ast::BinaryOperator as Op};

pub(crate) fn arithmetic_precedence(dialect: Dialect, op: &Op) -> Option<u8> {
    // https://www.sqlite.org/lang_expr.html#operators_and_parse_affecting_attributes
    // https://dev.mysql.com/doc/refman/8.4/en/operator-precedence.html
    match dialect {
        Dialect::Sqlite => match op {
            Op::BitwiseOr | Op::BitwiseAnd | Op::ShiftLeft | Op::ShiftRight => Some(10),
            Op::Plus | Op::Minus => Some(20),
            Op::Multiply | Op::Divide | Op::Modulo => Some(30),
            Op::Concat => Some(40),
            // Retain the previously accepted extension; SQLite itself has no ^.
            Op::BitwiseXor => Some(20),
            _ => None,
        },
        Dialect::Mysql => match op {
            Op::BitwiseOr => Some(10),
            Op::BitwiseAnd => Some(20),
            Op::ShiftLeft | Op::ShiftRight => Some(30),
            Op::Plus | Op::Minus => Some(40),
            Op::Multiply | Op::Divide | Op::Modulo => Some(50),
            Op::BitwiseXor => Some(60),
            // Default SQL mode: || is logical OR, handled outside arithmetic.
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn binary_precedence(dialect: Dialect, op: &Op) -> Option<u8> {
    arithmetic_precedence(dialect, op).or(match op {
        Op::Or => Some(1),
        Op::Xor => Some(2),
        Op::And => Some(3),
        Op::Eq | Op::Neq | Op::Lt | Op::Gt | Op::LtEq | Op::GtEq => Some(5),
        _ => None,
    })
}
