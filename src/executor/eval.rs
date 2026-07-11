// Expression evaluation for the SQL execution engine.

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::errors::{Result, SqlglotError};

use super::engine::ExecutionContext;
use super::{ResultSet, RowContext, Tables, Value};

// ═══════════════════════════════════════════════════════════════════════
// Public helpers
// ═══════════════════════════════════════════════════════════════════════

/// Evaluate an expression against a single row.
pub(crate) fn eval_expr(
    expr: &Expr,
    row: &RowContext,
    tables: &Tables,
    ctes: &HashMap<String, ResultSet>,
) -> Result<Value> {
    eval_expr_impl(expr, row, None, tables, ctes)
}

/// Evaluate an expression in group context (for aggregation).
pub(crate) fn eval_expr_group(
    expr: &Expr,
    rows: &[RowContext],
    tables: &Tables,
    ctes: &HashMap<String, ResultSet>,
) -> Result<Value> {
    let first = rows.first().cloned().unwrap_or_else(RowContext::empty);
    eval_expr_impl(expr, &first, Some(rows), tables, ctes)
}

/// Returns `true` if the expression tree contains an aggregate call.
pub(crate) fn expr_contains_aggregate(expr: &Expr) -> bool {
    match expr {
        Expr::Function { name, .. } => is_aggregate_name(name),
        Expr::TypedFunction { func, .. } => is_typed_aggregate(func),
        Expr::Alias { expr, .. } => expr_contains_aggregate(expr),
        Expr::BinaryOp { left, right, .. } => {
            expr_contains_aggregate(left) || expr_contains_aggregate(right)
        }
        Expr::UnaryOp { expr, .. } => expr_contains_aggregate(expr),
        Expr::Nested(inner) => expr_contains_aggregate(inner),
        Expr::Cast { expr, .. } | Expr::TryCast { expr, .. } => expr_contains_aggregate(expr),
        Expr::Case {
            operand,
            when_clauses,
            else_clause,
        } => {
            operand.as_ref().is_some_and(|o| expr_contains_aggregate(o))
                || when_clauses
                    .iter()
                    .any(|(w, t)| expr_contains_aggregate(w) || expr_contains_aggregate(t))
                || else_clause
                    .as_ref()
                    .is_some_and(|e| expr_contains_aggregate(e))
        }
        _ => false,
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Core evaluation
// ═══════════════════════════════════════════════════════════════════════

fn eval_expr_impl(
    expr: &Expr,
    row: &RowContext,
    group: Option<&[RowContext]>,
    tables: &Tables,
    ctes: &HashMap<String, ResultSet>,
) -> Result<Value> {
    match expr {
        // ── Literals & references ────────────────────────────────────
        Expr::Column { table, name, .. } => {
            let val = if let Some(t) = table {
                row.get_qualified(t, name)
            } else {
                row.get(name)
            };
            Ok(val.cloned().unwrap_or(Value::Null))
        }

        Expr::Number(s) => {
            if s.contains('.') {
                Ok(Value::Float(s.parse().map_err(|_| {
                    SqlglotError::Internal(format!("Invalid number: {s}"))
                })?))
            } else {
                Ok(Value::Int(s.parse().map_err(|_| {
                    SqlglotError::Internal(format!("Invalid integer: {s}"))
                })?))
            }
        }

        Expr::StringLiteral(s) | Expr::NationalStringLiteral(s) => Ok(Value::String(s.clone())),
        Expr::Boolean(b) => Ok(Value::Boolean(*b)),
        Expr::Null => Ok(Value::Null),
        Expr::Star | Expr::Wildcard => Ok(Value::Null),

        Expr::Nested(inner) | Expr::Alias { expr: inner, .. } => {
            eval_expr_impl(inner, row, group, tables, ctes)
        }

        // ── Binary / unary ───────────────────────────────────────────
        Expr::BinaryOp { left, op, right } => {
            let l = eval_expr_impl(left, row, group, tables, ctes)?;
            // Short-circuit AND / OR.
            match op {
                BinaryOperator::And => {
                    if !l.is_truthy() {
                        return Ok(Value::Boolean(false));
                    }
                    let r = eval_expr_impl(right, row, group, tables, ctes)?;
                    return Ok(Value::Boolean(r.is_truthy()));
                }
                BinaryOperator::Or => {
                    if l.is_truthy() {
                        return Ok(Value::Boolean(true));
                    }
                    let r = eval_expr_impl(right, row, group, tables, ctes)?;
                    return Ok(Value::Boolean(r.is_truthy()));
                }
                _ => {}
            }
            let r = eval_expr_impl(right, row, group, tables, ctes)?;
            eval_binary_op(&l, op, &r)
        }

        Expr::UnaryOp { op, expr } => {
            let v = eval_expr_impl(expr, row, group, tables, ctes)?;
            eval_unary_op(op, &v)
        }

        // ── Generic functions ────────────────────────────────────────
        Expr::Function {
            name,
            args,
            distinct,
            ..
        } => {
            if is_aggregate_name(name) {
                if let Some(g) = group {
                    return eval_aggregate_fn(name, args, *distinct, g, tables, ctes);
                }
            }
            eval_scalar_fn(name, args, row, group, tables, ctes)
        }

        // ── Typed functions ──────────────────────────────────────────
        Expr::TypedFunction { func, .. } => {
            if is_typed_aggregate(func) {
                if let Some(g) = group {
                    return eval_typed_aggregate(func, g, tables, ctes);
                }
            }
            eval_typed_fn(func, row, group, tables, ctes)
        }

        // ── Comparisons / predicates ─────────────────────────────────
        Expr::Between {
            expr,
            low,
            high,
            negated,
        } => {
            let v = eval_expr_impl(expr, row, group, tables, ctes)?;
            let lo = eval_expr_impl(low, row, group, tables, ctes)?;
            let hi = eval_expr_impl(high, row, group, tables, ctes)?;
            let in_range = v
                .partial_cmp(&lo)
                .is_some_and(|c| c != std::cmp::Ordering::Less)
                && v.partial_cmp(&hi)
                    .is_some_and(|c| c != std::cmp::Ordering::Greater);
            Ok(Value::Boolean(if *negated { !in_range } else { in_range }))
        }

        Expr::InList {
            expr,
            list,
            negated,
        } => {
            let v = eval_expr_impl(expr, row, group, tables, ctes)?;
            let found = list
                .iter()
                .any(|item| eval_expr_impl(item, row, group, tables, ctes).ok() == Some(v.clone()));
            Ok(Value::Boolean(if *negated { !found } else { found }))
        }

        Expr::InSubquery {
            expr,
            subquery,
            negated,
        } => {
            let v = eval_expr_impl(expr, row, group, tables, ctes)?;
            let result = execute_subquery(subquery, tables, ctes)?;
            let found = result.rows.iter().any(|r| !r.is_empty() && r[0] == v);
            Ok(Value::Boolean(if *negated { !found } else { found }))
        }

        Expr::IsNull { expr, negated } => {
            let v = eval_expr_impl(expr, row, group, tables, ctes)?;
            let is_null = v.is_null();
            Ok(Value::Boolean(if *negated { !is_null } else { is_null }))
        }

        Expr::IsBool {
            expr,
            value,
            negated,
        } => {
            let v = eval_expr_impl(expr, row, group, tables, ctes)?;
            let matches = matches!(&v, Value::Boolean(b) if b == value);
            Ok(Value::Boolean(if *negated { !matches } else { matches }))
        }

        Expr::Like {
            expr,
            pattern,
            negated,
            ..
        } => {
            let v = eval_expr_impl(expr, row, group, tables, ctes)?;
            let p = eval_expr_impl(pattern, row, group, tables, ctes)?;
            let matches = like_match(&v.to_string_val(), &p.to_string_val(), true);
            Ok(Value::Boolean(if *negated { !matches } else { matches }))
        }

        Expr::ILike {
            expr,
            pattern,
            negated,
            ..
        } => {
            let v = eval_expr_impl(expr, row, group, tables, ctes)?;
            let p = eval_expr_impl(pattern, row, group, tables, ctes)?;
            let matches = like_match(&v.to_string_val(), &p.to_string_val(), false);
            Ok(Value::Boolean(if *negated { !matches } else { matches }))
        }

        // ── Control flow ─────────────────────────────────────────────
        Expr::Case {
            operand,
            when_clauses,
            else_clause,
        } => {
            if let Some(op) = operand {
                let op_val = eval_expr_impl(op, row, group, tables, ctes)?;
                for (when, then) in when_clauses {
                    let when_val = eval_expr_impl(when, row, group, tables, ctes)?;
                    if op_val == when_val {
                        return eval_expr_impl(then, row, group, tables, ctes);
                    }
                }
            } else {
                for (when, then) in when_clauses {
                    let when_val = eval_expr_impl(when, row, group, tables, ctes)?;
                    if when_val.is_truthy() {
                        return eval_expr_impl(then, row, group, tables, ctes);
                    }
                }
            }
            if let Some(else_expr) = else_clause {
                eval_expr_impl(else_expr, row, group, tables, ctes)
            } else {
                Ok(Value::Null)
            }
        }

        Expr::Coalesce(exprs) => {
            for e in exprs {
                let v = eval_expr_impl(e, row, group, tables, ctes)?;
                if !v.is_null() {
                    return Ok(v);
                }
            }
            Ok(Value::Null)
        }

        Expr::If {
            condition,
            true_val,
            false_val,
        } => {
            let cond = eval_expr_impl(condition, row, group, tables, ctes)?;
            if cond.is_truthy() {
                eval_expr_impl(true_val, row, group, tables, ctes)
            } else if let Some(fv) = false_val {
                eval_expr_impl(fv, row, group, tables, ctes)
            } else {
                Ok(Value::Null)
            }
        }

        Expr::NullIf { expr, r#else } => {
            let v = eval_expr_impl(expr, row, group, tables, ctes)?;
            let e = eval_expr_impl(r#else, row, group, tables, ctes)?;
            if v == e { Ok(Value::Null) } else { Ok(v) }
        }

        // ── Cast ─────────────────────────────────────────────────────
        Expr::Cast { expr, data_type } => {
            let v = eval_expr_impl(expr, row, group, tables, ctes)?;
            cast_value(&v, data_type)
        }

        Expr::TryCast { expr, data_type } => {
            let v = eval_expr_impl(expr, row, group, tables, ctes)?;
            Ok(cast_value(&v, data_type).unwrap_or(Value::Null))
        }

        // ── Subqueries ───────────────────────────────────────────────
        Expr::Subquery(stmt) => {
            let result = execute_subquery(stmt, tables, ctes)?;
            Ok(result
                .rows
                .first()
                .and_then(|r| r.first().cloned())
                .unwrap_or(Value::Null))
        }

        Expr::Exists { subquery, negated } => {
            let result = execute_subquery(subquery, tables, ctes)?;
            let exists = !result.rows.is_empty();
            Ok(Value::Boolean(if *negated { !exists } else { exists }))
        }

        // ── Collections ──────────────────────────────────────────────
        Expr::Tuple(exprs) => {
            if exprs.len() == 1 {
                eval_expr_impl(&exprs[0], row, group, tables, ctes)
            } else {
                let vals: Vec<String> = exprs
                    .iter()
                    .map(|e| eval_expr_impl(e, row, group, tables, ctes).map(|v| v.to_string()))
                    .collect::<Result<_>>()?;
                Ok(Value::String(format!("({})", vals.join(", "))))
            }
        }

        Expr::ArrayLiteral(exprs) => {
            let vals: Vec<String> = exprs
                .iter()
                .map(|e| eval_expr_impl(e, row, group, tables, ctes).map(|v| v.to_string()))
                .collect::<Result<_>>()?;
            Ok(Value::String(format!("[{}]", vals.join(", "))))
        }

        // ── EXTRACT ──────────────────────────────────────────────────
        Expr::Extract { field, expr } => {
            let v = eval_expr_impl(expr, row, group, tables, ctes)?;
            eval_extract(field, &v)
        }

        _ => Err(SqlglotError::Internal(format!(
            "Unsupported expression in executor: {expr:?}"
        ))),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Binary / unary operators
// ═══════════════════════════════════════════════════════════════════════

fn eval_binary_op(left: &Value, op: &BinaryOperator, right: &Value) -> Result<Value> {
    // NULL propagation.
    if left.is_null() || right.is_null() {
        return match op {
            BinaryOperator::Eq
            | BinaryOperator::Neq
            | BinaryOperator::Lt
            | BinaryOperator::Gt
            | BinaryOperator::LtEq
            | BinaryOperator::GtEq => Ok(Value::Null),
            BinaryOperator::And => {
                if matches!(left, Value::Boolean(false)) || matches!(right, Value::Boolean(false)) {
                    Ok(Value::Boolean(false))
                } else {
                    Ok(Value::Null)
                }
            }
            BinaryOperator::Or => {
                if left.is_truthy() || right.is_truthy() {
                    Ok(Value::Boolean(true))
                } else {
                    Ok(Value::Null)
                }
            }
            _ => Ok(Value::Null),
        };
    }

    match op {
        BinaryOperator::Eq => Ok(Value::Boolean(left == right)),
        BinaryOperator::Neq => Ok(Value::Boolean(left != right)),
        BinaryOperator::Lt => Ok(Value::Boolean(
            left.partial_cmp(right)
                .is_some_and(|c| c == std::cmp::Ordering::Less),
        )),
        BinaryOperator::Gt => Ok(Value::Boolean(
            left.partial_cmp(right)
                .is_some_and(|c| c == std::cmp::Ordering::Greater),
        )),
        BinaryOperator::LtEq => Ok(Value::Boolean(
            left.partial_cmp(right)
                .is_some_and(|c| c != std::cmp::Ordering::Greater),
        )),
        BinaryOperator::GtEq => Ok(Value::Boolean(
            left.partial_cmp(right)
                .is_some_and(|c| c != std::cmp::Ordering::Less),
        )),

        BinaryOperator::Plus => eval_arithmetic(left, right, |a, b| a + b, |a, b| a + b),
        BinaryOperator::Minus => eval_arithmetic(left, right, |a, b| a - b, |a, b| a - b),
        BinaryOperator::Multiply => eval_arithmetic(left, right, |a, b| a * b, |a, b| a * b),
        BinaryOperator::Divide => {
            if let (Some(a), Some(b)) = (left.to_f64(), right.to_f64()) {
                if b == 0.0 {
                    return Err(SqlglotError::Internal("Division by zero".to_string()));
                }
                if matches!(left, Value::Int(_)) && matches!(right, Value::Int(_)) {
                    Ok(Value::Int(left.to_i64().unwrap() / right.to_i64().unwrap()))
                } else {
                    Ok(Value::Float(a / b))
                }
            } else {
                Ok(Value::Null)
            }
        }
        BinaryOperator::Modulo => {
            if let (Some(a), Some(b)) = (left.to_i64(), right.to_i64()) {
                if b == 0 {
                    return Err(SqlglotError::Internal("Modulo by zero".to_string()));
                }
                Ok(Value::Int(a % b))
            } else if let (Some(a), Some(b)) = (left.to_f64(), right.to_f64()) {
                if b == 0.0 {
                    return Err(SqlglotError::Internal("Modulo by zero".to_string()));
                }
                Ok(Value::Float(a % b))
            } else {
                Ok(Value::Null)
            }
        }
        BinaryOperator::Concat => Ok(Value::String(format!(
            "{}{}",
            left.to_string_val(),
            right.to_string_val()
        ))),

        BinaryOperator::And => Ok(Value::Boolean(left.is_truthy() && right.is_truthy())),
        BinaryOperator::Or => Ok(Value::Boolean(left.is_truthy() || right.is_truthy())),

        _ => Err(SqlglotError::Internal(format!(
            "Unsupported binary operator: {op:?}"
        ))),
    }
}

fn eval_arithmetic(
    left: &Value,
    right: &Value,
    int_op: impl Fn(i64, i64) -> i64,
    float_op: impl Fn(f64, f64) -> f64,
) -> Result<Value> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a, *b))),
        _ => {
            if let (Some(a), Some(b)) = (left.to_f64(), right.to_f64()) {
                Ok(Value::Float(float_op(a, b)))
            } else {
                Ok(Value::Null)
            }
        }
    }
}

fn eval_unary_op(op: &UnaryOperator, val: &Value) -> Result<Value> {
    match op {
        UnaryOperator::Not => Ok(Value::Boolean(!val.is_truthy())),
        UnaryOperator::Minus => match val {
            Value::Int(i) => Ok(Value::Int(-i)),
            Value::Float(f) => Ok(Value::Float(-f)),
            Value::Null => Ok(Value::Null),
            _ => Err(SqlglotError::Internal(format!("Cannot negate {val:?}"))),
        },
        UnaryOperator::Plus => Ok(val.clone()),
        UnaryOperator::BitwiseNot => match val {
            Value::Int(i) => Ok(Value::Int(!i)),
            Value::Null => Ok(Value::Null),
            _ => Err(SqlglotError::Internal(format!(
                "Cannot bitwise-not {val:?}"
            ))),
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Aggregate helpers
// ═══════════════════════════════════════════════════════════════════════

fn is_aggregate_name(name: &str) -> bool {
    matches!(
        name.to_uppercase().as_str(),
        "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" | "ARRAY_AGG" | "GROUP_CONCAT"
    )
}

fn is_typed_aggregate(func: &TypedFunction) -> bool {
    matches!(
        func,
        TypedFunction::Count { .. }
            | TypedFunction::Sum { .. }
            | TypedFunction::Avg { .. }
            | TypedFunction::Min { .. }
            | TypedFunction::Max { .. }
            | TypedFunction::ArrayAgg { .. }
            | TypedFunction::Variance { .. }
            | TypedFunction::VariancePop { .. }
            | TypedFunction::Stddev { .. }
            | TypedFunction::StddevPop { .. }
            | TypedFunction::GroupConcat { .. }
    )
}

fn eval_aggregate_fn(
    name: &str,
    args: &[Expr],
    distinct: bool,
    group: &[RowContext],
    tables: &Tables,
    ctes: &HashMap<String, ResultSet>,
) -> Result<Value> {
    match name.to_uppercase().as_str() {
        "COUNT" => {
            if args.len() == 1 && matches!(&args[0], Expr::Star | Expr::Wildcard) {
                return Ok(Value::Int(group.len() as i64));
            }
            let mut count = 0i64;
            let mut seen = HashSet::new();
            for row in group {
                let val = eval_expr(&args[0], row, tables, ctes)?;
                if !val.is_null() {
                    if distinct {
                        if seen.insert(val) {
                            count += 1;
                        }
                    } else {
                        count += 1;
                    }
                }
            }
            Ok(Value::Int(count))
        }
        "SUM" => {
            let mut sum = 0.0_f64;
            let mut has_val = false;
            let mut all_int = true;
            let mut seen = HashSet::new();
            for row in group {
                let val = eval_expr(&args[0], row, tables, ctes)?;
                if !val.is_null() {
                    if distinct && !seen.insert(val.clone()) {
                        continue;
                    }
                    if !matches!(val, Value::Int(_)) {
                        all_int = false;
                    }
                    if let Some(f) = val.to_f64() {
                        sum += f;
                        has_val = true;
                    }
                }
            }
            if !has_val {
                return Ok(Value::Null);
            }
            if all_int {
                Ok(Value::Int(sum as i64))
            } else {
                Ok(Value::Float(sum))
            }
        }
        "AVG" => {
            let mut sum = 0.0_f64;
            let mut count = 0i64;
            let mut seen = HashSet::new();
            for row in group {
                let val = eval_expr(&args[0], row, tables, ctes)?;
                if !val.is_null() {
                    if distinct && !seen.insert(val.clone()) {
                        continue;
                    }
                    if let Some(f) = val.to_f64() {
                        sum += f;
                        count += 1;
                    }
                }
            }
            if count == 0 {
                Ok(Value::Null)
            } else {
                Ok(Value::Float(sum / count as f64))
            }
        }
        "MIN" => {
            let mut min: Option<Value> = None;
            for row in group {
                let val = eval_expr(&args[0], row, tables, ctes)?;
                if !val.is_null() {
                    min = Some(match min {
                        Some(m)
                            if val
                                .partial_cmp(&m)
                                .is_some_and(|c| c == std::cmp::Ordering::Less) =>
                        {
                            val
                        }
                        Some(m) => m,
                        None => val,
                    });
                }
            }
            Ok(min.unwrap_or(Value::Null))
        }
        "MAX" => {
            let mut max: Option<Value> = None;
            for row in group {
                let val = eval_expr(&args[0], row, tables, ctes)?;
                if !val.is_null() {
                    max = Some(match max {
                        Some(m)
                            if val
                                .partial_cmp(&m)
                                .is_some_and(|c| c == std::cmp::Ordering::Greater) =>
                        {
                            val
                        }
                        Some(m) => m,
                        None => val,
                    });
                }
            }
            Ok(max.unwrap_or(Value::Null))
        }
        _ => Err(SqlglotError::Internal(format!(
            "Unsupported aggregate function: {name}"
        ))),
    }
}

fn eval_typed_aggregate(
    func: &TypedFunction,
    group: &[RowContext],
    tables: &Tables,
    ctes: &HashMap<String, ResultSet>,
) -> Result<Value> {
    match func {
        TypedFunction::Count { expr, distinct } => {
            if matches!(**expr, Expr::Star | Expr::Wildcard) {
                return Ok(Value::Int(group.len() as i64));
            }
            let mut count = 0i64;
            let mut seen = HashSet::new();
            for row in group {
                let val = eval_expr(expr, row, tables, ctes)?;
                if !val.is_null() {
                    if *distinct {
                        if seen.insert(val) {
                            count += 1;
                        }
                    } else {
                        count += 1;
                    }
                }
            }
            Ok(Value::Int(count))
        }
        TypedFunction::Sum { expr, distinct } => {
            let mut sum = 0.0_f64;
            let mut has_val = false;
            let mut all_int = true;
            let mut seen = HashSet::new();
            for row in group {
                let val = eval_expr(expr, row, tables, ctes)?;
                if !val.is_null() {
                    if *distinct && !seen.insert(val.clone()) {
                        continue;
                    }
                    if !matches!(val, Value::Int(_)) {
                        all_int = false;
                    }
                    if let Some(f) = val.to_f64() {
                        sum += f;
                        has_val = true;
                    }
                }
            }
            if !has_val {
                return Ok(Value::Null);
            }
            if all_int {
                Ok(Value::Int(sum as i64))
            } else {
                Ok(Value::Float(sum))
            }
        }
        TypedFunction::Avg { expr, distinct } => {
            let mut sum = 0.0_f64;
            let mut count = 0i64;
            let mut seen = HashSet::new();
            for row in group {
                let val = eval_expr(expr, row, tables, ctes)?;
                if !val.is_null() {
                    if *distinct && !seen.insert(val.clone()) {
                        continue;
                    }
                    if let Some(f) = val.to_f64() {
                        sum += f;
                        count += 1;
                    }
                }
            }
            if count == 0 {
                Ok(Value::Null)
            } else {
                Ok(Value::Float(sum / count as f64))
            }
        }
        TypedFunction::Min { expr } => {
            let mut min: Option<Value> = None;
            for row in group {
                let val = eval_expr(expr, row, tables, ctes)?;
                if !val.is_null() {
                    min = Some(match min {
                        Some(m)
                            if val
                                .partial_cmp(&m)
                                .is_some_and(|c| c == std::cmp::Ordering::Less) =>
                        {
                            val
                        }
                        Some(m) => m,
                        None => val,
                    });
                }
            }
            Ok(min.unwrap_or(Value::Null))
        }
        TypedFunction::Max { expr } => {
            let mut max: Option<Value> = None;
            for row in group {
                let val = eval_expr(expr, row, tables, ctes)?;
                if !val.is_null() {
                    max = Some(match max {
                        Some(m)
                            if val
                                .partial_cmp(&m)
                                .is_some_and(|c| c == std::cmp::Ordering::Greater) =>
                        {
                            val
                        }
                        Some(m) => m,
                        None => val,
                    });
                }
            }
            Ok(max.unwrap_or(Value::Null))
        }
        _ => Err(SqlglotError::Internal(format!(
            "Unsupported typed aggregate: {func:?}"
        ))),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Scalar functions
// ═══════════════════════════════════════════════════════════════════════

fn eval_scalar_fn(
    name: &str,
    args: &[Expr],
    row: &RowContext,
    group: Option<&[RowContext]>,
    tables: &Tables,
    ctes: &HashMap<String, ResultSet>,
) -> Result<Value> {
    let evaluated: Vec<Value> = args
        .iter()
        .map(|a| eval_expr_impl(a, row, group, tables, ctes))
        .collect::<Result<_>>()?;

    match name.to_uppercase().as_str() {
        "UPPER" => Ok(Value::String(
            evaluated
                .first()
                .map_or(String::new(), |v| v.to_string_val().to_uppercase()),
        )),
        "LOWER" => Ok(Value::String(
            evaluated
                .first()
                .map_or(String::new(), |v| v.to_string_val().to_lowercase()),
        )),
        "LENGTH" | "LEN" | "CHAR_LENGTH" | "CHARACTER_LENGTH" => Ok(Value::Int(
            evaluated
                .first()
                .map_or(0, |v| v.to_string_val().len() as i64),
        )),
        "CONCAT" => {
            let s: String = evaluated.iter().map(|v| v.to_string_val()).collect();
            Ok(Value::String(s))
        }
        "ABS" => match evaluated.first() {
            Some(Value::Int(i)) => Ok(Value::Int(i.abs())),
            Some(Value::Float(f)) => Ok(Value::Float(f.abs())),
            Some(Value::Null) | None => Ok(Value::Null),
            _ => Err(SqlglotError::Internal(
                "ABS requires numeric argument".to_string(),
            )),
        },
        "CEIL" | "CEILING" => Ok(evaluated
            .first()
            .and_then(|v| v.to_f64())
            .map_or(Value::Null, |f| Value::Int(f.ceil() as i64))),
        "FLOOR" => Ok(evaluated
            .first()
            .and_then(|v| v.to_f64())
            .map_or(Value::Null, |f| Value::Int(f.floor() as i64))),
        "ROUND" => {
            let val = evaluated.first().and_then(|v| v.to_f64());
            let decimals = evaluated.get(1).and_then(|v| v.to_i64()).unwrap_or(0);
            match val {
                Some(f) => {
                    let factor = 10_f64.powi(decimals as i32);
                    Ok(Value::Float((f * factor).round() / factor))
                }
                None => Ok(Value::Null),
            }
        }
        "SQRT" => Ok(evaluated
            .first()
            .and_then(|v| v.to_f64())
            .map_or(Value::Null, |f| Value::Float(f.sqrt()))),
        "POWER" | "POW" => {
            match (
                evaluated.first().and_then(|v| v.to_f64()),
                evaluated.get(1).and_then(|v| v.to_f64()),
            ) {
                (Some(base), Some(exp)) => Ok(Value::Float(base.powf(exp))),
                _ => Ok(Value::Null),
            }
        }
        "COALESCE" => {
            for v in &evaluated {
                if !v.is_null() {
                    return Ok(v.clone());
                }
            }
            Ok(Value::Null)
        }
        "NULLIF" => {
            if evaluated.len() >= 2 && evaluated[0] == evaluated[1] {
                Ok(Value::Null)
            } else {
                Ok(evaluated.into_iter().next().unwrap_or(Value::Null))
            }
        }
        "GREATEST" => Ok(find_extreme(&evaluated, true)),
        "LEAST" => Ok(find_extreme(&evaluated, false)),
        "REPLACE" => {
            if evaluated.len() >= 3 {
                let s = evaluated[0].to_string_val();
                let from = evaluated[1].to_string_val();
                let to = evaluated[2].to_string_val();
                Ok(Value::String(s.replace(&from, &to)))
            } else {
                Ok(Value::Null)
            }
        }
        "SUBSTRING" | "SUBSTR" => {
            let s = evaluated
                .first()
                .map(|v| v.to_string_val())
                .unwrap_or_default();
            let start = evaluated.get(1).and_then(|v| v.to_i64()).unwrap_or(1) as usize;
            let start_idx = start.saturating_sub(1);
            if let Some(len) = evaluated.get(2).and_then(|v| v.to_i64()) {
                Ok(Value::String(
                    s.chars().skip(start_idx).take(len as usize).collect(),
                ))
            } else {
                Ok(Value::String(s.chars().skip(start_idx).collect()))
            }
        }
        "TRIM" => Ok(Value::String(
            evaluated
                .first()
                .map_or(String::new(), |v| v.to_string_val().trim().to_string()),
        )),
        "LTRIM" => Ok(Value::String(
            evaluated.first().map_or(String::new(), |v| {
                v.to_string_val().trim_start().to_string()
            }),
        )),
        "RTRIM" => Ok(Value::String(
            evaluated
                .first()
                .map_or(String::new(), |v| v.to_string_val().trim_end().to_string()),
        )),
        "LEFT" => {
            let s = evaluated
                .first()
                .map(|v| v.to_string_val())
                .unwrap_or_default();
            let n = evaluated.get(1).and_then(|v| v.to_i64()).unwrap_or(0) as usize;
            Ok(Value::String(s.chars().take(n).collect()))
        }
        "RIGHT" => {
            let s = evaluated
                .first()
                .map(|v| v.to_string_val())
                .unwrap_or_default();
            let n = evaluated.get(1).and_then(|v| v.to_i64()).unwrap_or(0) as usize;
            let len = s.chars().count();
            Ok(Value::String(
                s.chars().skip(len.saturating_sub(n)).collect(),
            ))
        }
        "REVERSE" => Ok(Value::String(
            evaluated
                .first()
                .map_or(String::new(), |v| v.to_string_val().chars().rev().collect()),
        )),
        "LPAD" => {
            let s = evaluated
                .first()
                .map(|v| v.to_string_val())
                .unwrap_or_default();
            let target = evaluated.get(1).and_then(|v| v.to_i64()).unwrap_or(0) as usize;
            let pad = evaluated
                .get(2)
                .map(|v| v.to_string_val())
                .unwrap_or_else(|| " ".to_string());
            Ok(Value::String(pad_string(&s, target, &pad, true)))
        }
        "RPAD" => {
            let s = evaluated
                .first()
                .map(|v| v.to_string_val())
                .unwrap_or_default();
            let target = evaluated.get(1).and_then(|v| v.to_i64()).unwrap_or(0) as usize;
            let pad = evaluated
                .get(2)
                .map(|v| v.to_string_val())
                .unwrap_or_else(|| " ".to_string());
            Ok(Value::String(pad_string(&s, target, &pad, false)))
        }
        "MOD" => {
            match (
                evaluated.first().and_then(|v| v.to_i64()),
                evaluated.get(1).and_then(|v| v.to_i64()),
            ) {
                (Some(a), Some(b)) => {
                    if b == 0 {
                        return Err(SqlglotError::Internal("Modulo by zero".to_string()));
                    }
                    Ok(Value::Int(a % b))
                }
                _ => Ok(Value::Null),
            }
        }
        "LN" => Ok(evaluated
            .first()
            .and_then(|v| v.to_f64())
            .map_or(Value::Null, |f| Value::Float(f.ln()))),
        "LOG" | "LOG10" => {
            let val = evaluated.first().and_then(|v| v.to_f64());
            let base = evaluated.get(1).and_then(|v| v.to_f64()).unwrap_or(10.0);
            Ok(val.map_or(Value::Null, |f| Value::Float(f.log(base))))
        }
        "LOG2" => Ok(evaluated
            .first()
            .and_then(|v| v.to_f64())
            .map_or(Value::Null, |f| Value::Float(f.log2()))),
        _ => Err(SqlglotError::Internal(format!("Unknown function: {name}"))),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Typed scalar functions
// ═══════════════════════════════════════════════════════════════════════

fn eval_typed_fn(
    func: &TypedFunction,
    row: &RowContext,
    group: Option<&[RowContext]>,
    tables: &Tables,
    ctes: &HashMap<String, ResultSet>,
) -> Result<Value> {
    // Helper closure.
    let ev = |e: &Expr| eval_expr_impl(e, row, group, tables, ctes);

    match func {
        TypedFunction::Upper { expr } => {
            Ok(Value::String(ev(expr)?.to_string_val().to_uppercase()))
        }
        TypedFunction::Lower { expr } => {
            Ok(Value::String(ev(expr)?.to_string_val().to_lowercase()))
        }
        TypedFunction::Length { expr } => Ok(Value::Int(ev(expr)?.to_string_val().len() as i64)),
        TypedFunction::Reverse { expr } => Ok(Value::String(
            ev(expr)?.to_string_val().chars().rev().collect(),
        )),
        TypedFunction::Replace { expr, from, to } => {
            let s = ev(expr)?.to_string_val();
            let f = ev(from)?.to_string_val();
            let t = ev(to)?.to_string_val();
            Ok(Value::String(s.replace(&f, &t)))
        }
        TypedFunction::Substring {
            expr,
            start,
            length,
        } => {
            let s = ev(expr)?.to_string_val();
            let st = ev(start)?.to_i64().unwrap_or(1) as usize;
            let start_idx = st.saturating_sub(1);
            if let Some(len_expr) = length {
                let len = ev(len_expr)?.to_i64().unwrap_or(0) as usize;
                Ok(Value::String(s.chars().skip(start_idx).take(len).collect()))
            } else {
                Ok(Value::String(s.chars().skip(start_idx).collect()))
            }
        }
        TypedFunction::Trim { expr, .. } => {
            Ok(Value::String(ev(expr)?.to_string_val().trim().to_string()))
        }
        TypedFunction::Left { expr, n } => {
            let s = ev(expr)?.to_string_val();
            let count = ev(n)?.to_i64().unwrap_or(0) as usize;
            Ok(Value::String(s.chars().take(count).collect()))
        }
        TypedFunction::Right { expr, n } => {
            let s = ev(expr)?.to_string_val();
            let count = ev(n)?.to_i64().unwrap_or(0) as usize;
            let len = s.chars().count();
            Ok(Value::String(
                s.chars().skip(len.saturating_sub(count)).collect(),
            ))
        }
        TypedFunction::Abs { expr } => match ev(expr)? {
            Value::Int(i) => Ok(Value::Int(i.abs())),
            Value::Float(f) => Ok(Value::Float(f.abs())),
            Value::Null => Ok(Value::Null),
            v => Err(SqlglotError::Internal(format!(
                "ABS requires numeric: {v:?}"
            ))),
        },
        TypedFunction::Ceil { expr } => Ok(ev(expr)?
            .to_f64()
            .map_or(Value::Null, |f| Value::Int(f.ceil() as i64))),
        TypedFunction::Floor { expr } => Ok(ev(expr)?
            .to_f64()
            .map_or(Value::Null, |f| Value::Int(f.floor() as i64))),
        TypedFunction::Round { expr, decimals } => {
            let d = decimals
                .as_ref()
                .map(|de| ev(de))
                .transpose()?
                .and_then(|v| v.to_i64())
                .unwrap_or(0);
            match ev(expr)?.to_f64() {
                Some(f) => {
                    let factor = 10_f64.powi(d as i32);
                    Ok(Value::Float((f * factor).round() / factor))
                }
                None => Ok(Value::Null),
            }
        }
        TypedFunction::Sqrt { expr } => Ok(ev(expr)?
            .to_f64()
            .map_or(Value::Null, |f| Value::Float(f.sqrt()))),
        TypedFunction::Pow { base, exponent } => {
            let b = ev(base)?.to_f64();
            let e = ev(exponent)?.to_f64();
            match (b, e) {
                (Some(b), Some(e)) => Ok(Value::Float(b.powf(e))),
                _ => Ok(Value::Null),
            }
        }
        TypedFunction::Ln { expr } => Ok(ev(expr)?
            .to_f64()
            .map_or(Value::Null, |f| Value::Float(f.ln()))),
        TypedFunction::Log { expr, base } => {
            let val = ev(expr)?.to_f64();
            let b = base
                .as_ref()
                .map(|be| ev(be))
                .transpose()?
                .and_then(|v| v.to_f64())
                .unwrap_or(10.0);
            Ok(val.map_or(Value::Null, |f| Value::Float(f.log(b))))
        }
        TypedFunction::Mod { left, right } => {
            let l = ev(left)?.to_i64();
            let r = ev(right)?.to_i64();
            match (l, r) {
                (Some(a), Some(b)) => {
                    if b == 0 {
                        return Err(SqlglotError::Internal("Modulo by zero".to_string()));
                    }
                    Ok(Value::Int(a % b))
                }
                _ => Ok(Value::Null),
            }
        }
        TypedFunction::Greatest { exprs } => {
            let vals: Vec<Value> = exprs.iter().map(|e| ev(e)).collect::<Result<_>>()?;
            Ok(find_extreme(&vals, true))
        }
        TypedFunction::Least { exprs } => {
            let vals: Vec<Value> = exprs.iter().map(|e| ev(e)).collect::<Result<_>>()?;
            Ok(find_extreme(&vals, false))
        }
        TypedFunction::ConcatWs { separator, exprs } => {
            let sep = ev(separator)?.to_string_val();
            let vals: Vec<String> = exprs
                .iter()
                .map(|e| ev(e).map(|v| v.to_string_val()))
                .collect::<Result<_>>()?;
            Ok(Value::String(vals.join(&sep)))
        }
        TypedFunction::Initcap { expr } => {
            let s = ev(expr)?.to_string_val();
            let result: String = s
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => {
                            format!("{}{}", c.to_uppercase(), chars.as_str().to_lowercase())
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            Ok(Value::String(result))
        }
        TypedFunction::Lpad { expr, length, pad } => {
            let s = ev(expr)?.to_string_val();
            let target = ev(length)?.to_i64().unwrap_or(0) as usize;
            let p = pad
                .as_ref()
                .map(|pe| ev(pe).map(|v| v.to_string_val()))
                .transpose()?
                .unwrap_or_else(|| " ".to_string());
            Ok(Value::String(pad_string(&s, target, &p, true)))
        }
        TypedFunction::Rpad { expr, length, pad } => {
            let s = ev(expr)?.to_string_val();
            let target = ev(length)?.to_i64().unwrap_or(0) as usize;
            let p = pad
                .as_ref()
                .map(|pe| ev(pe).map(|v| v.to_string_val()))
                .transpose()?
                .unwrap_or_else(|| " ".to_string());
            Ok(Value::String(pad_string(&s, target, &p, false)))
        }

        // Aggregates in non-group context ⇒ NULL.
        TypedFunction::Count { .. }
        | TypedFunction::Sum { .. }
        | TypedFunction::Avg { .. }
        | TypedFunction::Min { .. }
        | TypedFunction::Max { .. } => Ok(Value::Null),

        _ => Err(SqlglotError::Internal(format!(
            "Unsupported typed function in executor: {func:?}"
        ))),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// LIKE pattern matching
// ═══════════════════════════════════════════════════════════════════════

fn like_match(value: &str, pattern: &str, case_sensitive: bool) -> bool {
    let (v, p) = if case_sensitive {
        (value.to_string(), pattern.to_string())
    } else {
        (value.to_lowercase(), pattern.to_lowercase())
    };
    like_match_impl(v.as_bytes(), p.as_bytes())
}

fn like_match_impl(value: &[u8], pattern: &[u8]) -> bool {
    if pattern.is_empty() {
        return value.is_empty();
    }
    if pattern[0] == b'%' {
        let mut p = 1;
        while p < pattern.len() && pattern[p] == b'%' {
            p += 1;
        }
        for i in 0..=value.len() {
            if like_match_impl(&value[i..], &pattern[p..]) {
                return true;
            }
        }
        false
    } else if value.is_empty() {
        false
    } else if pattern[0] == b'_' || pattern[0] == value[0] {
        like_match_impl(&value[1..], &pattern[1..])
    } else {
        false
    }
}

// ═══════════════════════════════════════════════════════════════════════
// CAST
// ═══════════════════════════════════════════════════════════════════════

fn cast_value(val: &Value, data_type: &DataType) -> Result<Value> {
    if val.is_null() {
        return Ok(Value::Null);
    }
    match data_type {
        DataType::Int | DataType::BigInt | DataType::SmallInt | DataType::TinyInt => val
            .to_i64()
            .map(Value::Int)
            .ok_or_else(|| SqlglotError::Internal(format!("Cannot cast {val:?} to integer"))),
        DataType::Float
        | DataType::Double
        | DataType::Real
        | DataType::Decimal { .. }
        | DataType::Numeric { .. } => val
            .to_f64()
            .map(Value::Float)
            .ok_or_else(|| SqlglotError::Internal(format!("Cannot cast {val:?} to float"))),
        DataType::Varchar(_)
        | DataType::Char(_)
        | DataType::NChar(_)
        | DataType::NVarchar(_)
        | DataType::VarcharMax
        | DataType::NvarcharMax
        | DataType::Varchar2(_)
        | DataType::NVarchar2(_)
        | DataType::Text
        | DataType::String => Ok(Value::String(val.to_string_val())),
        DataType::Boolean => Ok(Value::Boolean(val.is_truthy())),
        _ => Ok(Value::String(val.to_string_val())),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// EXTRACT
// ═══════════════════════════════════════════════════════════════════════

fn eval_extract(field: &DateTimeField, val: &Value) -> Result<Value> {
    let s = val.to_string_val();
    let parts: Vec<&str> = s.split(&['-', ' ', ':', 'T'][..]).collect();

    match field {
        DateTimeField::Year => Ok(parts
            .first()
            .and_then(|p| p.parse::<i64>().ok())
            .map_or(Value::Null, Value::Int)),
        DateTimeField::Month => Ok(parts
            .get(1)
            .and_then(|p| p.parse::<i64>().ok())
            .map_or(Value::Null, Value::Int)),
        DateTimeField::Day => Ok(parts
            .get(2)
            .and_then(|p| p.parse::<i64>().ok())
            .map_or(Value::Null, Value::Int)),
        DateTimeField::Hour => Ok(parts
            .get(3)
            .and_then(|p| p.parse::<i64>().ok())
            .map_or(Value::Null, Value::Int)),
        DateTimeField::Minute => Ok(parts
            .get(4)
            .and_then(|p| p.parse::<i64>().ok())
            .map_or(Value::Null, Value::Int)),
        DateTimeField::Second => Ok(parts
            .get(5)
            .and_then(|p| p.parse::<i64>().ok())
            .map_or(Value::Null, Value::Int)),
        _ => Err(SqlglotError::Internal(format!(
            "Unsupported EXTRACT field: {field:?}"
        ))),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Subquery execution
// ═══════════════════════════════════════════════════════════════════════

fn execute_subquery(
    stmt: &Statement,
    tables: &Tables,
    ctes: &HashMap<String, ResultSet>,
) -> Result<ResultSet> {
    let mut ctx = ExecutionContext::with_ctes(tables, ctes.clone());
    ctx.execute(stmt)
}

// ═══════════════════════════════════════════════════════════════════════
// Misc helpers
// ═══════════════════════════════════════════════════════════════════════

fn find_extreme(vals: &[Value], greatest: bool) -> Value {
    let mut best: Option<Value> = None;
    for v in vals {
        if v.is_null() {
            continue;
        }
        best = Some(match best {
            Some(b) => {
                let cmp = v.partial_cmp(&b);
                if greatest {
                    if cmp.is_some_and(|c| c == std::cmp::Ordering::Greater) {
                        v.clone()
                    } else {
                        b
                    }
                } else if cmp.is_some_and(|c| c == std::cmp::Ordering::Less) {
                    v.clone()
                } else {
                    b
                }
            }
            None => v.clone(),
        });
    }
    best.unwrap_or(Value::Null)
}

fn pad_string(s: &str, target: usize, pad: &str, left: bool) -> String {
    if s.len() >= target {
        return s.chars().take(target).collect();
    }
    let needed = target - s.len();
    let padding: String = pad.chars().cycle().take(needed).collect();
    if left {
        format!("{padding}{s}")
    } else {
        format!("{s}{padding}")
    }
}
