use std::collections::HashMap;

use sqlglot_rust::SqlglotError;
use sqlglot_rust::executor::{Table, Value, execute};

// ═══════════════════════════════════════════════════════════════════════
// Test helpers
// ═══════════════════════════════════════════════════════════════════════

fn sample_tables() -> HashMap<String, Table> {
    let mut tables = HashMap::new();

    tables.insert(
        "employees".to_string(),
        Table::new(
            vec![
                "id".to_string(),
                "name".to_string(),
                "department".to_string(),
                "salary".to_string(),
            ],
            vec![
                vec![
                    Value::Int(1),
                    Value::String("Alice".to_string()),
                    Value::String("Engineering".to_string()),
                    Value::Float(100000.0),
                ],
                vec![
                    Value::Int(2),
                    Value::String("Bob".to_string()),
                    Value::String("Engineering".to_string()),
                    Value::Float(95000.0),
                ],
                vec![
                    Value::Int(3),
                    Value::String("Carol".to_string()),
                    Value::String("Sales".to_string()),
                    Value::Float(80000.0),
                ],
                vec![
                    Value::Int(4),
                    Value::String("Dave".to_string()),
                    Value::String("Sales".to_string()),
                    Value::Float(75000.0),
                ],
                vec![
                    Value::Int(5),
                    Value::String("Eve".to_string()),
                    Value::String("Marketing".to_string()),
                    Value::Float(90000.0),
                ],
            ],
        ),
    );

    tables.insert(
        "departments".to_string(),
        Table::new(
            vec![
                "name".to_string(),
                "budget".to_string(),
                "head_id".to_string(),
            ],
            vec![
                vec![
                    Value::String("Engineering".to_string()),
                    Value::Float(500000.0),
                    Value::Int(1),
                ],
                vec![
                    Value::String("Sales".to_string()),
                    Value::Float(300000.0),
                    Value::Int(3),
                ],
                vec![
                    Value::String("Marketing".to_string()),
                    Value::Float(200000.0),
                    Value::Int(5),
                ],
                vec![
                    Value::String("HR".to_string()),
                    Value::Float(150000.0),
                    Value::Null,
                ],
            ],
        ),
    );

    tables.insert(
        "orders".to_string(),
        Table::new(
            vec![
                "id".to_string(),
                "employee_id".to_string(),
                "amount".to_string(),
            ],
            vec![
                vec![Value::Int(101), Value::Int(1), Value::Float(250.0)],
                vec![Value::Int(102), Value::Int(1), Value::Float(300.0)],
                vec![Value::Int(103), Value::Int(2), Value::Float(150.0)],
                vec![Value::Int(104), Value::Int(3), Value::Float(400.0)],
                vec![Value::Int(105), Value::Int(5), Value::Float(200.0)],
            ],
        ),
    );

    tables
}

#[test]
fn temporal_table_scan_fails_closed() {
    assert!(matches!(
        execute(
            "SELECT * FROM employees AT(OFFSET => -60)",
            &sample_tables(),
        ),
        Err(SqlglotError::UnsupportedDialectFeature(_))
    ));
}

// ═══════════════════════════════════════════════════════════════════════
// Basic SELECT
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_select_all_columns() {
    let tables = sample_tables();
    let result = execute("SELECT * FROM employees", &tables).unwrap();
    assert_eq!(result.row_count(), 5);
    assert_eq!(result.column_count(), 4);
}

#[test]
fn test_select_specific_columns() {
    let tables = sample_tables();
    let result = execute("SELECT name, salary FROM employees", &tables).unwrap();
    assert_eq!(result.row_count(), 5);
    assert_eq!(result.column_count(), 2);
    assert_eq!(result.columns[0], "name");
    assert_eq!(result.columns[1], "salary");
}

#[test]
fn test_select_with_alias() {
    let tables = sample_tables();
    let result = execute("SELECT name AS employee_name FROM employees", &tables).unwrap();
    assert_eq!(result.columns[0], "employee_name");
}

#[test]
fn test_select_expression() {
    let tables = sample_tables();
    let result = execute("SELECT 1 + 2", &tables).unwrap();
    assert_eq!(result.row_count(), 1);
    assert_eq!(result.rows[0][0], Value::Int(3));
}

#[test]
fn test_select_string_literal() {
    let tables = sample_tables();
    let result = execute("SELECT 'hello'", &tables).unwrap();
    assert_eq!(result.rows[0][0], Value::String("hello".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════
// WHERE filtering
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_where_equals() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM employees WHERE department = 'Engineering'",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 2);
}

#[test]
fn test_where_greater_than() {
    let tables = sample_tables();
    let result = execute("SELECT name FROM employees WHERE salary > 90000", &tables).unwrap();
    assert_eq!(result.row_count(), 2); // Alice (100k), Bob (95k)
}

#[test]
fn test_where_and() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM employees WHERE department = 'Engineering' AND salary > 96000",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 1);
    assert_eq!(result.rows[0][0], Value::String("Alice".to_string()));
}

#[test]
fn test_where_or() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM employees WHERE department = 'Marketing' OR department = 'Sales'",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 3); // Carol, Dave, Eve
}

#[test]
fn test_where_between() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM employees WHERE salary BETWEEN 80000 AND 95000",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 3); // Bob, Carol, Eve
}

#[test]
fn test_where_in_list() {
    let tables = sample_tables();
    let result = execute("SELECT name FROM employees WHERE id IN (1, 3, 5)", &tables).unwrap();
    assert_eq!(result.row_count(), 3);
}

#[test]
fn test_where_is_null() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM departments WHERE head_id IS NULL",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 1);
    assert_eq!(result.rows[0][0], Value::String("HR".to_string()));
}

#[test]
fn test_where_is_not_null() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM departments WHERE head_id IS NOT NULL",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 3);
}

#[test]
fn test_where_like() {
    let tables = sample_tables();
    let result = execute("SELECT name FROM employees WHERE name LIKE 'A%'", &tables).unwrap();
    assert_eq!(result.row_count(), 1);
    assert_eq!(result.rows[0][0], Value::String("Alice".to_string()));
}

#[test]
fn test_where_not_equals() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM employees WHERE department <> 'Engineering'",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 3);
}

// ═══════════════════════════════════════════════════════════════════════
// JOINs
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_inner_join() {
    let tables = sample_tables();
    let result = execute(
        "SELECT e.name, o.amount FROM employees e INNER JOIN orders o ON e.id = o.employee_id",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 5); // 5 orders, all matching an employee
}

#[test]
fn test_left_join() {
    let tables = sample_tables();
    let result = execute(
        "SELECT e.name, o.amount FROM employees e LEFT JOIN orders o ON e.id = o.employee_id",
        &tables,
    )
    .unwrap();
    // Alice(2), Bob(1), Carol(1), Dave(0→NULL), Eve(1) = 6 rows
    assert_eq!(result.row_count(), 6);
}

#[test]
fn test_right_join() {
    let tables = sample_tables();
    let result = execute(
        "SELECT e.name, d.name AS dept FROM employees e RIGHT JOIN departments d ON e.department = d.name",
        &tables,
    )
    .unwrap();
    // Engineering(2), Sales(2), Marketing(1), HR(0→NULL) = 6 rows
    assert_eq!(result.row_count(), 6);
}

#[test]
fn test_full_join() {
    let _tables = sample_tables();
    // Use small tables for clarity.
    let mut t = HashMap::new();
    t.insert(
        "a".to_string(),
        Table::from_rows(
            vec!["id", "val"],
            vec![
                vec![Value::Int(1), Value::String("a1".to_string())],
                vec![Value::Int(2), Value::String("a2".to_string())],
            ],
        ),
    );
    t.insert(
        "b".to_string(),
        Table::from_rows(
            vec!["id", "val"],
            vec![
                vec![Value::Int(2), Value::String("b2".to_string())],
                vec![Value::Int(3), Value::String("b3".to_string())],
            ],
        ),
    );
    let result = execute("SELECT a.val, b.val FROM a FULL JOIN b ON a.id = b.id", &t).unwrap();
    assert_eq!(result.row_count(), 3); // (a1,NULL), (a2,b2), (NULL,b3)
}

#[test]
fn test_cross_join() {
    let mut t = HashMap::new();
    t.insert(
        "a".to_string(),
        Table::from_rows(vec!["x"], vec![vec![Value::Int(1)], vec![Value::Int(2)]]),
    );
    t.insert(
        "b".to_string(),
        Table::from_rows(
            vec!["y"],
            vec![
                vec![Value::String("a".to_string())],
                vec![Value::String("b".to_string())],
            ],
        ),
    );
    let result = execute("SELECT x, y FROM a CROSS JOIN b", &t).unwrap();
    assert_eq!(result.row_count(), 4);
}

// ═══════════════════════════════════════════════════════════════════════
// Aggregation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_count_star() {
    let tables = sample_tables();
    let result = execute("SELECT COUNT(*) FROM employees", &tables).unwrap();
    assert_eq!(result.row_count(), 1);
    assert_eq!(result.rows[0][0], Value::Int(5));
}

#[test]
fn test_sum() {
    let tables = sample_tables();
    let result = execute("SELECT SUM(salary) FROM employees", &tables).unwrap();
    assert_eq!(result.rows[0][0], Value::Float(440000.0));
}

#[test]
fn test_avg() {
    let tables = sample_tables();
    let result = execute("SELECT AVG(salary) FROM employees", &tables).unwrap();
    assert_eq!(result.rows[0][0], Value::Float(88000.0));
}

#[test]
fn test_min_max() {
    let tables = sample_tables();
    let result = execute("SELECT MIN(salary), MAX(salary) FROM employees", &tables).unwrap();
    assert_eq!(result.rows[0][0], Value::Float(75000.0));
    assert_eq!(result.rows[0][1], Value::Float(100000.0));
}

#[test]
fn test_group_by() {
    let tables = sample_tables();
    let result = execute(
        "SELECT department, COUNT(*) AS cnt FROM employees GROUP BY department ORDER BY department",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 3);
    // Engineering: 2, Marketing: 1, Sales: 2
    assert_eq!(result.rows[0][0], Value::String("Engineering".to_string()));
    assert_eq!(result.rows[0][1], Value::Int(2));
    assert_eq!(result.rows[1][0], Value::String("Marketing".to_string()));
    assert_eq!(result.rows[1][1], Value::Int(1));
    assert_eq!(result.rows[2][0], Value::String("Sales".to_string()));
    assert_eq!(result.rows[2][1], Value::Int(2));
}

#[test]
fn test_group_by_sum() {
    let tables = sample_tables();
    let result = execute(
        "SELECT department, SUM(salary) AS total FROM employees GROUP BY department ORDER BY total",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 3);
    // Marketing: 90k, Sales: 155k, Engineering: 195k
    assert_eq!(result.rows[0][0], Value::String("Marketing".to_string()));
    assert_eq!(result.rows[0][1], Value::Float(90000.0));
}

#[test]
fn test_having() {
    let tables = sample_tables();
    let result = execute(
        "SELECT department, COUNT(*) AS cnt FROM employees GROUP BY department HAVING COUNT(*) > 1",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 2); // Engineering(2), Sales(2)
}

// ═══════════════════════════════════════════════════════════════════════
// ORDER BY
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_order_by_asc() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name, salary FROM employees ORDER BY salary",
        &tables,
    )
    .unwrap();
    assert_eq!(result.rows[0][0], Value::String("Dave".to_string())); // 75k
    assert_eq!(result.rows[4][0], Value::String("Alice".to_string())); // 100k
}

#[test]
fn test_order_by_desc() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name, salary FROM employees ORDER BY salary DESC",
        &tables,
    )
    .unwrap();
    assert_eq!(result.rows[0][0], Value::String("Alice".to_string())); // 100k
    assert_eq!(result.rows[4][0], Value::String("Dave".to_string())); // 75k
}

// ═══════════════════════════════════════════════════════════════════════
// LIMIT / OFFSET
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_limit() {
    let tables = sample_tables();
    let result = execute("SELECT name FROM employees LIMIT 3", &tables).unwrap();
    assert_eq!(result.row_count(), 3);
}

#[test]
fn test_limit_offset() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM employees ORDER BY id LIMIT 2 OFFSET 2",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 2);
    assert_eq!(result.rows[0][0], Value::String("Carol".to_string()));
    assert_eq!(result.rows[1][0], Value::String("Dave".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════
// DISTINCT
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_distinct() {
    let tables = sample_tables();
    let result = execute(
        "SELECT DISTINCT department FROM employees ORDER BY department",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 3);
}

// ═══════════════════════════════════════════════════════════════════════
// Subqueries
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_subquery_in_where() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM employees WHERE salary > (SELECT AVG(salary) FROM employees)",
        &tables,
    )
    .unwrap();
    // AVG = 88k, so Alice(100k) and Bob(95k) and Eve(90k)
    assert_eq!(result.row_count(), 3);
}

#[test]
fn test_in_subquery() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM employees WHERE id IN (SELECT employee_id FROM orders)",
        &tables,
    )
    .unwrap();
    // Orders have employee_ids: 1, 1, 2, 3, 5
    assert_eq!(result.row_count(), 4); // Alice, Bob, Carol, Eve
}

#[test]
fn test_exists_subquery() {
    let tables = sample_tables();
    // Uncorrelated EXISTS – the subquery returns rows, so all outer rows pass.
    let result = execute(
        "SELECT name FROM employees WHERE EXISTS (SELECT 1 FROM orders WHERE amount > 100)",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 5);

    // Subquery returns no rows → NOT EXISTS is true for every outer row.
    let result = execute(
        "SELECT name FROM employees WHERE NOT EXISTS (SELECT 1 FROM orders WHERE amount > 1000)",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 5);
}

#[test]
fn test_subquery_in_from() {
    let tables = sample_tables();
    let result = execute(
        "SELECT sub.name FROM (SELECT name, salary FROM employees WHERE salary > 90000) AS sub",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 2); // Alice(100k), Bob(95k)
}

// ═══════════════════════════════════════════════════════════════════════
// CTEs
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_cte_basic() {
    let tables = sample_tables();
    let result = execute(
        "WITH high_earners AS (SELECT name, salary FROM employees WHERE salary > 90000) \
         SELECT name FROM high_earners",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 2); // Alice, Bob
}

#[test]
fn test_cte_multiple() {
    let tables = sample_tables();
    let result = execute(
        "WITH eng AS (SELECT * FROM employees WHERE department = 'Engineering'), \
              sales AS (SELECT * FROM employees WHERE department = 'Sales') \
         SELECT COUNT(*) FROM eng",
        &tables,
    )
    .unwrap();
    assert_eq!(result.rows[0][0], Value::Int(2));
}

// ═══════════════════════════════════════════════════════════════════════
// Set operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_union() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM employees WHERE department = 'Engineering' \
         UNION \
         SELECT name FROM employees WHERE salary >= 90000",
        &tables,
    )
    .unwrap();
    // Engineering: Alice, Bob; salary >= 90k: Alice, Bob, Eve → UNION: Alice, Bob, Eve
    assert_eq!(result.row_count(), 3);
}

#[test]
fn test_union_all() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM employees WHERE department = 'Engineering' \
         UNION ALL \
         SELECT name FROM employees WHERE salary >= 90000",
        &tables,
    )
    .unwrap();
    // 2 + 3 = 5 (duplicates kept)
    assert_eq!(result.row_count(), 5);
}

#[test]
fn test_intersect() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM employees WHERE department = 'Engineering' \
         INTERSECT \
         SELECT name FROM employees WHERE salary > 90000",
        &tables,
    )
    .unwrap();
    // Both Engineering AND salary>90k: Alice, Bob
    assert_eq!(result.row_count(), 2);
}

#[test]
fn test_except() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name FROM employees WHERE salary > 80000 \
         EXCEPT \
         SELECT name FROM employees WHERE department = 'Engineering'",
        &tables,
    )
    .unwrap();
    // salary>80k: Alice, Bob, Carol, Eve minus Engineering: Alice, Bob → Carol, Eve
    // Wait: Carol salary = 80k, so salary > 80k doesn't include Carol
    // salary > 80k: Alice(100k), Bob(95k), Eve(90k) minus Engineering: Alice, Bob
    // → Eve
    assert_eq!(result.row_count(), 1);
}

// ═══════════════════════════════════════════════════════════════════════
// Built-in functions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_upper_lower() {
    let tables = sample_tables();
    let result = execute(
        "SELECT UPPER(name), LOWER(name) FROM employees WHERE id = 1",
        &tables,
    )
    .unwrap();
    assert_eq!(result.rows[0][0], Value::String("ALICE".to_string()));
    assert_eq!(result.rows[0][1], Value::String("alice".to_string()));
}

#[test]
fn test_coalesce() {
    let tables = sample_tables();
    let result = execute(
        "SELECT COALESCE(head_id, 0) FROM departments WHERE name = 'HR'",
        &tables,
    )
    .unwrap();
    assert_eq!(result.rows[0][0], Value::Int(0));
}

#[test]
fn test_case_expression() {
    let tables = sample_tables();
    let result = execute(
        "SELECT name, CASE WHEN salary > 90000 THEN 'high' ELSE 'normal' END AS level \
         FROM employees ORDER BY id",
        &tables,
    )
    .unwrap();
    assert_eq!(result.rows[0][1], Value::String("high".to_string())); // Alice
    assert_eq!(result.rows[1][1], Value::String("high".to_string())); // Bob (95k)
    assert_eq!(result.rows[2][1], Value::String("normal".to_string())); // Carol
}

#[test]
fn test_arithmetic_expressions() {
    let tables = sample_tables();
    let result = execute("SELECT salary * 2 FROM employees WHERE id = 1", &tables).unwrap();
    // 100000.0 * 2 = 200000.0
    assert_eq!(result.rows[0][0], Value::Float(200000.0));
}

#[test]
fn test_cast() {
    let tables = sample_tables();
    let result = execute(
        "SELECT CAST(salary AS INT) FROM employees WHERE id = 1",
        &tables,
    )
    .unwrap();
    assert_eq!(result.rows[0][0], Value::Int(100000));
}

// ═══════════════════════════════════════════════════════════════════════
// Type coercion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_int_float_comparison() {
    let mut tables = HashMap::new();
    tables.insert(
        "t".to_string(),
        Table::from_rows(
            vec!["a"],
            vec![
                vec![Value::Int(1)],
                vec![Value::Int(2)],
                vec![Value::Int(3)],
            ],
        ),
    );
    let result = execute("SELECT a FROM t WHERE a > 1.5", &tables).unwrap();
    assert_eq!(result.row_count(), 2); // 2, 3
}

// ═══════════════════════════════════════════════════════════════════════
// Edge cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_empty_table() {
    let mut tables = HashMap::new();
    tables.insert(
        "empty".to_string(),
        Table::from_rows(vec!["a", "b"], vec![]),
    );
    let result = execute("SELECT * FROM empty", &tables).unwrap();
    assert_eq!(result.row_count(), 0);
}

#[test]
fn test_count_empty_table() {
    let mut tables = HashMap::new();
    tables.insert("empty".to_string(), Table::from_rows(vec!["a"], vec![]));
    let result = execute("SELECT COUNT(*) FROM empty", &tables).unwrap();
    assert_eq!(result.rows[0][0], Value::Int(0));
}

#[test]
fn test_aggregate_with_null() {
    let mut tables = HashMap::new();
    tables.insert(
        "t".to_string(),
        Table::from_rows(
            vec!["a"],
            vec![vec![Value::Int(1)], vec![Value::Null], vec![Value::Int(3)]],
        ),
    );

    let result = execute("SELECT SUM(a) FROM t", &tables).unwrap();
    assert_eq!(result.rows[0][0], Value::Int(4));

    let result = execute("SELECT COUNT(a) FROM t", &tables).unwrap();
    assert_eq!(result.rows[0][0], Value::Int(2)); // NULL not counted

    let result = execute("SELECT COUNT(*) FROM t", &tables).unwrap();
    assert_eq!(result.rows[0][0], Value::Int(3)); // COUNT(*) counts NULLs
}

#[test]
fn test_qualified_column_in_join() {
    let tables = sample_tables();
    let result = execute(
        "SELECT employees.name, departments.budget \
         FROM employees \
         INNER JOIN departments ON employees.department = departments.name \
         WHERE employees.id = 1",
        &tables,
    )
    .unwrap();
    assert_eq!(result.row_count(), 1);
    assert_eq!(result.rows[0][0], Value::String("Alice".to_string()));
    assert_eq!(result.rows[0][1], Value::Float(500000.0));
}

// ═══════════════════════════════════════════════════════════════════════
// ResultSet display
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_result_set_display() {
    let tables = sample_tables();
    let result = execute("SELECT name, salary FROM employees WHERE id = 1", &tables).unwrap();
    let display = format!("{result}");
    assert!(display.contains("Alice"));
    assert!(display.contains("name"));
    assert!(display.contains("salary"));
}
