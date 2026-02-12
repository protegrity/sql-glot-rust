/// Tests ported from Python sqlglot's `test_optimizer.py`.
///
/// Covers constant folding (arithmetic, string concat, comparisons) and
/// boolean simplification (AND/OR with TRUE/FALSE, double NOT, WHERE TRUE).
use sqlglot_rust::{generate, parse, Dialect};
use sqlglot_rust::optimizer::optimize;

/// Parse → optimise → generate, compare with expected output.
fn validate_optimized(input: &str, expected: &str) {
    let stmt = parse(input, Dialect::Ansi)
        .unwrap_or_else(|e| panic!("Parse failed for '{}': {}", input, e));
    let optimized = optimize(stmt).unwrap();
    let output = generate(&optimized, Dialect::Ansi);
    assert_eq!(output, expected, "\n  Input: {}", input);
}

/// Ensures the optimized output equals the original (i.e. no-op optimization).
fn validate_no_op(sql: &str) {
    validate_optimized(sql, sql);
}

// ═════════════════════════════════════════════════════════════════════════════
// Constant Folding – Arithmetic
// (from Python test_optimizer.py::test_simplify)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fold_addition() {
    validate_optimized("SELECT 1 + 2", "SELECT 3");
    validate_optimized("SELECT 10 + 20 + 30", "SELECT 60");
}

#[test]
fn test_fold_subtraction() {
    validate_optimized("SELECT 10 - 3", "SELECT 7");
}

#[test]
fn test_fold_multiplication() {
    validate_optimized("SELECT 3 * 4", "SELECT 12");
    validate_optimized("SELECT 2 * 3 * 4", "SELECT 24");
}

#[test]
fn test_fold_division() {
    validate_optimized("SELECT 10 / 2", "SELECT 5");
}

#[test]
fn test_fold_modulo() {
    validate_optimized("SELECT 10 % 3", "SELECT 1");
}

#[test]
fn test_fold_mixed_arithmetic() {
    validate_optimized("SELECT 1 + 2 * 3", "SELECT 7");
}

// ═════════════════════════════════════════════════════════════════════════════
// Constant Folding – String Concatenation
// (from Python test_optimizer.py::test_simplify)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fold_string_concat() {
    validate_optimized("SELECT 'hello' || ' ' || 'world'", "SELECT 'hello world'");
    validate_optimized("SELECT 'foo' || 'bar'", "SELECT 'foobar'");
}

// ═════════════════════════════════════════════════════════════════════════════
// Constant Folding – Comparisons
// (from Python test_optimizer.py::test_simplify)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fold_comparisons() {
    validate_optimized("SELECT 1 < 2", "SELECT TRUE");
    validate_optimized("SELECT 1 > 2", "SELECT FALSE");
    validate_optimized("SELECT 1 = 1", "SELECT TRUE");
    validate_optimized("SELECT 1 <> 1", "SELECT FALSE");
    validate_optimized("SELECT 1 <= 1", "SELECT TRUE");
    validate_optimized("SELECT 1 >= 1", "SELECT TRUE");
    validate_optimized("SELECT 2 <= 1", "SELECT FALSE");
    validate_optimized("SELECT 0 >= 1", "SELECT FALSE");
}

// ═════════════════════════════════════════════════════════════════════════════
// Boolean Simplification – AND / OR with TRUE / FALSE
// (from Python test_optimizer.py::test_simplify)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_simplify_and_true() {
    validate_optimized("SELECT TRUE AND x", "SELECT x");
    validate_optimized("SELECT x AND TRUE", "SELECT x");
}

#[test]
fn test_simplify_and_false() {
    validate_optimized("SELECT FALSE AND x", "SELECT FALSE");
    validate_optimized("SELECT x AND FALSE", "SELECT FALSE");
}

#[test]
fn test_simplify_or_true() {
    validate_optimized("SELECT TRUE OR x", "SELECT TRUE");
    validate_optimized("SELECT x OR TRUE", "SELECT TRUE");
}

#[test]
fn test_simplify_or_false() {
    validate_optimized("SELECT FALSE OR x", "SELECT x");
    validate_optimized("SELECT x OR FALSE", "SELECT x");
}

// ═════════════════════════════════════════════════════════════════════════════
// Boolean Simplification – NOT
// (from Python test_optimizer.py::test_simplify)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_simplify_double_not() {
    validate_optimized("SELECT NOT NOT x", "SELECT x");
}

// ═════════════════════════════════════════════════════════════════════════════
// Boolean Simplification – WHERE TRUE elimination
// (from Python test_optimizer.py::test_simplify)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_simplify_where_true() {
    validate_optimized(
        "SELECT x FROM t WHERE TRUE",
        "SELECT x FROM t",
    );
}

#[test]
fn test_simplify_where_true_and_condition() {
    validate_optimized(
        "SELECT x FROM t WHERE TRUE AND a > 1",
        "SELECT x FROM t WHERE a > 1",
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// No-op cases – optimizer should leave these unchanged
// (from Python test_optimizer.py checking idempotence)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_no_op_plain_select() {
    validate_no_op("SELECT a FROM t WHERE a > 1");
}

#[test]
fn test_no_op_column_only() {
    validate_no_op("SELECT a + b FROM t");
}

#[test]
fn test_no_op_complex_query() {
    validate_no_op(
        "SELECT a, b FROM t INNER JOIN t2 ON t.id = t2.id WHERE t.x > 0 ORDER BY a",
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combined constant folding + boolean simplification
// (from Python test_optimizer.py multi-pass tests)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_combined_fold_and_simplify() {
    // 1 + 2 folds to 3, 3 > 0 folds to TRUE, TRUE AND x → x
    validate_optimized("SELECT 1 + 2 > 0 AND x", "SELECT x");
}

#[test]
fn test_fold_in_where() {
    validate_optimized(
        "SELECT a FROM t WHERE 1 + 1 = 2",
        "SELECT a FROM t",  // WHERE TRUE → removed
    );
}

#[test]
fn test_fold_leaves_non_const_untouched() {
    // Cannot fold a + 1 because a is a column reference
    validate_no_op("SELECT a + 1 FROM t");
}

// ═════════════════════════════════════════════════════════════════════════════
// Optimizer preserves correct structure
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_optimizer_preserves_joins() {
    validate_no_op(
        "SELECT a.x FROM a INNER JOIN b ON a.id = b.id",
    );
}

#[test]
fn test_optimizer_preserves_group_by() {
    validate_no_op(
        "SELECT a, COUNT(*) FROM t GROUP BY a",
    );
}

#[test]
fn test_optimizer_preserves_cte() {
    validate_no_op(
        "WITH cte AS (SELECT 1 AS x) SELECT * FROM cte",
    );
}
