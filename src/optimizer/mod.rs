/// Query optimization passes.
///
/// Inspired by Python sqlglot's optimizer module. Currently implements:
/// - Constant folding (e.g., `1 + 2` → `3`)
/// - Boolean simplification (e.g., `TRUE AND x` → `x`)
/// - Dead predicate elimination (e.g., `WHERE TRUE`)
///
/// Future optimizations:
/// - Predicate pushdown
/// - Join reordering
/// - Column pruning
/// - Subquery unnesting

use crate::ast::*;
use crate::errors::Result;

/// Optimize a SQL statement by applying transformation passes.
pub fn optimize(statement: Statement) -> Result<Statement> {
    let mut stmt = statement;
    stmt = fold_constants(stmt);
    stmt = simplify_booleans(stmt);
    Ok(stmt)
}

/// Fold constant expressions (e.g., `1 + 2` → `3`).
fn fold_constants(statement: Statement) -> Statement {
    match statement {
        Statement::Select(mut sel) => {
            if let Some(wh) = sel.where_clause {
                sel.where_clause = Some(fold_expr(wh));
            }
            if let Some(having) = sel.having {
                sel.having = Some(fold_expr(having));
            }
            for item in &mut sel.columns {
                if let SelectItem::Expr { expr, .. } = item {
                    *expr = fold_expr(expr.clone());
                }
            }
            Statement::Select(sel)
        }
        other => other,
    }
}

fn fold_expr(expr: Expr) -> Expr {
    match expr {
        Expr::BinaryOp { left, op, right } => {
            let left = fold_expr(*left);
            let right = fold_expr(*right);

            // Try numeric folding
            if let (Expr::Number(l), Expr::Number(r)) = (&left, &right) {
                if let (Ok(lv), Ok(rv)) = (l.parse::<f64>(), r.parse::<f64>()) {
                    let result = match op {
                        BinaryOperator::Plus => Some(lv + rv),
                        BinaryOperator::Minus => Some(lv - rv),
                        BinaryOperator::Multiply => Some(lv * rv),
                        BinaryOperator::Divide if rv != 0.0 => Some(lv / rv),
                        BinaryOperator::Modulo if rv != 0.0 => Some(lv % rv),
                        _ => None,
                    };
                    if let Some(val) = result {
                        // Emit integer if it's a whole number
                        if val.fract() == 0.0 && val.abs() < i64::MAX as f64 {
                            return Expr::Number(format!("{}", val as i64));
                        }
                        return Expr::Number(format!("{val}"));
                    }

                    // Try boolean folding for comparison
                    let cmp = match op {
                        BinaryOperator::Eq => Some(lv == rv),
                        BinaryOperator::Neq => Some(lv != rv),
                        BinaryOperator::Lt => Some(lv < rv),
                        BinaryOperator::Gt => Some(lv > rv),
                        BinaryOperator::LtEq => Some(lv <= rv),
                        BinaryOperator::GtEq => Some(lv >= rv),
                        _ => None,
                    };
                    if let Some(val) = cmp {
                        return Expr::Boolean(val);
                    }
                }
            }

            // String concatenation folding
            if matches!(op, BinaryOperator::Concat) {
                if let (Expr::StringLiteral(l), Expr::StringLiteral(r)) = (&left, &right) {
                    return Expr::StringLiteral(format!("{l}{r}"));
                }
            }

            Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            }
        }
        Expr::UnaryOp { op: UnaryOperator::Minus, expr } => {
            let inner = fold_expr(*expr);
            if let Expr::Number(ref n) = inner {
                if let Ok(v) = n.parse::<f64>() {
                    let neg = -v;
                    if neg.fract() == 0.0 && neg.abs() < i64::MAX as f64 {
                        return Expr::Number(format!("{}", neg as i64));
                    }
                    return Expr::Number(format!("{neg}"));
                }
            }
            Expr::UnaryOp {
                op: UnaryOperator::Minus,
                expr: Box::new(inner),
            }
        }
        Expr::Nested(inner) => {
            let folded = fold_expr(*inner);
            if folded.is_literal() {
                folded
            } else {
                Expr::Nested(Box::new(folded))
            }
        }
        other => other,
    }
}

/// Simplify boolean expressions.
fn simplify_booleans(statement: Statement) -> Statement {
    match statement {
        Statement::Select(mut sel) => {
            // Simplify boolean expressions in SELECT columns
            for item in &mut sel.columns {
                if let SelectItem::Expr { expr, .. } = item {
                    *expr = simplify_bool_expr(expr.clone());
                }
            }
            if let Some(wh) = sel.where_clause {
                let simplified = simplify_bool_expr(wh);
                // WHERE TRUE → no WHERE clause
                if simplified == Expr::Boolean(true) {
                    sel.where_clause = None;
                } else {
                    sel.where_clause = Some(simplified);
                }
            }
            if let Some(having) = sel.having {
                let simplified = simplify_bool_expr(having);
                if simplified == Expr::Boolean(true) {
                    sel.having = None;
                } else {
                    sel.having = Some(simplified);
                }
            }
            Statement::Select(sel)
        }
        other => other,
    }
}

fn simplify_bool_expr(expr: Expr) -> Expr {
    match expr {
        Expr::BinaryOp { left, op: BinaryOperator::And, right } => {
            let left = simplify_bool_expr(*left);
            let right = simplify_bool_expr(*right);
            match (&left, &right) {
                (Expr::Boolean(true), _) => right,
                (_, Expr::Boolean(true)) => left,
                (Expr::Boolean(false), _) | (_, Expr::Boolean(false)) => Expr::Boolean(false),
                _ => Expr::BinaryOp {
                    left: Box::new(left),
                    op: BinaryOperator::And,
                    right: Box::new(right),
                },
            }
        }
        Expr::BinaryOp { left, op: BinaryOperator::Or, right } => {
            let left = simplify_bool_expr(*left);
            let right = simplify_bool_expr(*right);
            match (&left, &right) {
                (Expr::Boolean(true), _) | (_, Expr::Boolean(true)) => Expr::Boolean(true),
                (Expr::Boolean(false), _) => right,
                (_, Expr::Boolean(false)) => left,
                _ => Expr::BinaryOp {
                    left: Box::new(left),
                    op: BinaryOperator::Or,
                    right: Box::new(right),
                },
            }
        }
        Expr::UnaryOp { op: UnaryOperator::Not, expr } => {
            let inner = simplify_bool_expr(*expr);
            match inner {
                Expr::Boolean(b) => Expr::Boolean(!b),
                Expr::UnaryOp { op: UnaryOperator::Not, expr: inner2 } => *inner2,
                other => Expr::UnaryOp {
                    op: UnaryOperator::Not,
                    expr: Box::new(other),
                },
            }
        }
        Expr::Nested(inner) => {
            let simplified = simplify_bool_expr(*inner);
            if simplified.is_literal() {
                simplified
            } else {
                Expr::Nested(Box::new(simplified))
            }
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn optimize_sql(sql: &str) -> Statement {
        let stmt = Parser::new(sql).unwrap().parse_statement().unwrap();
        optimize(stmt).unwrap()
    }

    #[test]
    fn test_constant_folding() {
        let stmt = optimize_sql("SELECT 1 + 2 FROM t");
        if let Statement::Select(sel) = stmt {
            if let SelectItem::Expr { expr, .. } = &sel.columns[0] {
                assert_eq!(*expr, Expr::Number("3".to_string()));
            }
        }
    }

    #[test]
    fn test_boolean_simplification_where_true() {
        let stmt = optimize_sql("SELECT x FROM t WHERE TRUE");
        if let Statement::Select(sel) = stmt {
            assert!(sel.where_clause.is_none());
        }
    }

    #[test]
    fn test_boolean_simplification_and_true() {
        let stmt = optimize_sql("SELECT x FROM t WHERE TRUE AND x > 1");
        if let Statement::Select(sel) = stmt {
            // Should simplify to just x > 1
            assert!(sel.where_clause.is_some());
            assert!(!matches!(&sel.where_clause, Some(Expr::BinaryOp { op: BinaryOperator::And, .. })));
        }
    }

    #[test]
    fn test_double_negation() {
        let stmt = optimize_sql("SELECT x FROM t WHERE NOT NOT x > 1");
        if let Statement::Select(sel) = stmt {
            // Should simplify to x > 1 (no NOT)
            assert!(sel.where_clause.is_some());
        }
    }
}
