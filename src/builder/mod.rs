//! # Expression Builder API
//!
//! Fluent builder API for programmatic SQL construction and manipulation.
//!
//! This module provides ergonomic builders inspired by Python sqlglot's
//! builder API, allowing construction of SQL expressions without manual
//! AST enum construction.
//!
//! ## Quick Start
//!
//! ```rust
//! use sqlglot_rust::builder::{select, column, literal, condition};
//! use sqlglot_rust::{Dialect, generate};
//!
//! // Build a SELECT query fluently
//! let query = select(&["a", "b"])
//!     .from("users")
//!     .where_clause("active = true")
//!     .order_by(&["created_at"])
//!     .limit(10)
//!     .build();
//!
//! let sql = generate(&query, Dialect::Postgres);
//! ```
//!
//! ## Condition Builder
//!
//! ```rust
//! use sqlglot_rust::builder::condition;
//!
//! // Build complex conditions
//! let cond = condition("x = 1")
//!     .and("y = 2")
//!     .or("z = 3")
//!     .build();
//! ```
//!
//! ## Expression Factory Functions
//!
//! ```rust
//! use sqlglot_rust::builder::{column, table, literal, cast, and_all, or_all};
//! use sqlglot_rust::ast::DataType;
//!
//! // Create expressions directly
//! let col = column("name", Some("users"));
//! let tbl = table("users", Some("public"));
//! let num = literal(42);
//! let casted = cast(column("id", None), DataType::BigInt);
//! ```

use crate::ast::{
    BinaryOperator, DataType, Expr, FromClause, JoinClause, JoinType, OrderByItem, QuoteStyle,
    SelectItem, SelectStatement, Statement, TableRef, TableSource,
};
use crate::dialects::Dialect;
use crate::parser::parse;

// ═══════════════════════════════════════════════════════════════════════
// Expression Factory Functions
// ═══════════════════════════════════════════════════════════════════════

/// Create a column expression.
///
/// # Arguments
/// * `name` - Column name
/// * `table` - Optional table qualifier
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::column;
///
/// let col = column("id", None);
/// let qualified = column("name", Some("users"));
/// ```
#[must_use]
pub fn column(name: &str, table: Option<&str>) -> Expr {
    Expr::Column {
        table: table.map(String::from),
        name: name.to_string(),
        quote_style: QuoteStyle::None,
        table_quote_style: QuoteStyle::None,
    }
}

/// Create a table reference.
///
/// # Arguments
/// * `name` - Table name
/// * `schema` - Optional schema qualifier
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::table;
///
/// let tbl = table("users", None);
/// let qualified = table("orders", Some("public"));
/// ```
#[must_use]
pub fn table(name: &str, schema: Option<&str>) -> TableRef {
    TableRef {
        catalog: None,
        schema: schema.map(String::from),
        name: name.to_string(),
        alias: None,
        name_quote_style: QuoteStyle::None,
        alias_quote_style: QuoteStyle::None,
    }
}

/// Create a fully qualified table reference with catalog.
///
/// # Arguments
/// * `name` - Table name
/// * `schema` - Optional schema qualifier
/// * `catalog` - Optional catalog qualifier
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::table_full;
///
/// let tbl = table_full("users", Some("public"), Some("mydb"));
/// ```
#[must_use]
pub fn table_full(name: &str, schema: Option<&str>, catalog: Option<&str>) -> TableRef {
    TableRef {
        catalog: catalog.map(String::from),
        schema: schema.map(String::from),
        name: name.to_string(),
        alias: None,
        name_quote_style: QuoteStyle::None,
        alias_quote_style: QuoteStyle::None,
    }
}

/// Create an integer literal expression.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::literal;
///
/// let num = literal(42);
/// ```
#[must_use]
pub fn literal<T: ToString>(value: T) -> Expr {
    Expr::Number(value.to_string())
}

/// Create a string literal expression.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::string_literal;
///
/// let s = string_literal("hello");
/// ```
#[must_use]
pub fn string_literal(value: &str) -> Expr {
    Expr::StringLiteral(value.to_string())
}

/// Create a boolean literal expression.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::boolean;
///
/// let t = boolean(true);
/// let f = boolean(false);
/// ```
#[must_use]
pub fn boolean(value: bool) -> Expr {
    Expr::Boolean(value)
}

/// Create a NULL literal expression.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::null;
///
/// let n = null();
/// ```
#[must_use]
pub fn null() -> Expr {
    Expr::Null
}

/// Create a CAST expression.
///
/// # Arguments
/// * `expr` - Expression to cast
/// * `data_type` - Target data type
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::{cast, column};
/// use sqlglot_rust::ast::DataType;
///
/// let casted = cast(column("id", None), DataType::BigInt);
/// ```
#[must_use]
pub fn cast(expr: Expr, data_type: DataType) -> Expr {
    Expr::Cast {
        expr: Box::new(expr),
        data_type,
    }
}

/// Combine multiple conditions with AND.
///
/// # Arguments
/// * `conditions` - Iterator of expressions to combine
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::{and_all, column};
/// use sqlglot_rust::ast::{Expr, BinaryOperator};
///
/// let cond1 = Expr::BinaryOp {
///     left: Box::new(column("x", None)),
///     op: BinaryOperator::Gt,
///     right: Box::new(Expr::Number("1".to_string())),
/// };
/// let cond2 = Expr::BinaryOp {
///     left: Box::new(column("y", None)),
///     op: BinaryOperator::Lt,
///     right: Box::new(Expr::Number("10".to_string())),
/// };
///
/// let combined = and_all(vec![cond1, cond2]);
/// ```
#[must_use]
pub fn and_all<I>(conditions: I) -> Option<Expr>
where
    I: IntoIterator<Item = Expr>,
{
    let mut iter = conditions.into_iter();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, cond| Expr::BinaryOp {
        left: Box::new(acc),
        op: BinaryOperator::And,
        right: Box::new(cond),
    }))
}

/// Combine multiple conditions with OR.
///
/// # Arguments
/// * `conditions` - Iterator of expressions to combine
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::{or_all, column};
/// use sqlglot_rust::ast::{Expr, BinaryOperator};
///
/// let cond1 = Expr::BinaryOp {
///     left: Box::new(column("status", None)),
///     op: BinaryOperator::Eq,
///     right: Box::new(Expr::StringLiteral("active".to_string())),
/// };
/// let cond2 = Expr::BinaryOp {
///     left: Box::new(column("status", None)),
///     op: BinaryOperator::Eq,
///     right: Box::new(Expr::StringLiteral("pending".to_string())),
/// };
///
/// let combined = or_all(vec![cond1, cond2]);
/// ```
#[must_use]
pub fn or_all<I>(conditions: I) -> Option<Expr>
where
    I: IntoIterator<Item = Expr>,
{
    let mut iter = conditions.into_iter();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, cond| Expr::BinaryOp {
        left: Box::new(acc),
        op: BinaryOperator::Or,
        right: Box::new(cond),
    }))
}

/// Negate an expression with NOT.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::{not, column};
///
/// let negated = not(column("active", None));
/// ```
#[must_use]
pub fn not(expr: Expr) -> Expr {
    Expr::UnaryOp {
        op: crate::ast::UnaryOperator::Not,
        expr: Box::new(expr),
    }
}

/// Create a function call expression.
///
/// # Arguments
/// * `name` - Function name
/// * `args` - Function arguments
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::{func, column};
///
/// let count = func("COUNT", vec![column("id", None)]);
/// let coalesce = func("COALESCE", vec![column("name", None), sqlglot_rust::builder::string_literal("N/A")]);
/// ```
#[must_use]
pub fn func(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Function {
        name: name.to_string(),
        args,
        distinct: false,
        filter: None,
        over: None,
        order_by: vec![],
        within_group: false,
    }
}

/// Create a function call with DISTINCT.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::{func_distinct, column};
///
/// let count_distinct = func_distinct("COUNT", vec![column("user_id", None)]);
/// ```
#[must_use]
pub fn func_distinct(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Function {
        name: name.to_string(),
        args,
        distinct: true,
        filter: None,
        over: None,
        order_by: vec![],
        within_group: false,
    }
}

/// Create a wildcard (*) expression.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::star;
///
/// let all = star();
/// ```
#[must_use]
pub fn star() -> Expr {
    Expr::Star
}

/// Create a qualified wildcard (table.*) expression.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::qualified_star;
///
/// let all_users = qualified_star("users");
/// ```
#[must_use]
pub fn qualified_star(table: &str) -> Expr {
    Expr::QualifiedWildcard {
        table: table.to_string(),
    }
}

/// Create a subquery expression.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::{subquery, select};
///
/// let inner = select(&["id"]).from("users").build();
/// let sub = subquery(inner);
/// ```
#[must_use]
pub fn subquery(statement: Statement) -> Expr {
    Expr::Subquery(Box::new(statement))
}

/// Create an EXISTS expression.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::{exists, select};
///
/// let inner = select(&["1"]).from("users").where_clause("id = 1").build();
/// let check = exists(inner, false);
/// ```
#[must_use]
pub fn exists(statement: Statement, negated: bool) -> Expr {
    Expr::Exists {
        subquery: Box::new(statement),
        negated,
    }
}

/// Create an aliased expression.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::{alias, column};
///
/// let aliased = alias(column("first_name", None), "name");
/// ```
#[must_use]
pub fn alias(expr: Expr, name: &str) -> Expr {
    Expr::Alias {
        expr: Box::new(expr),
        name: name.to_string(),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Comparison helpers
// ═══════════════════════════════════════════════════════════════════════

/// Create an equality comparison (=).
#[must_use]
pub fn eq(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Eq,
        right: Box::new(right),
    }
}

/// Create an inequality comparison (<>).
#[must_use]
pub fn neq(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Neq,
        right: Box::new(right),
    }
}

/// Create a less-than comparison (<).
#[must_use]
pub fn lt(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Lt,
        right: Box::new(right),
    }
}

/// Create a less-than-or-equal comparison (<=).
#[must_use]
pub fn lte(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::LtEq,
        right: Box::new(right),
    }
}

/// Create a greater-than comparison (>).
#[must_use]
pub fn gt(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Gt,
        right: Box::new(right),
    }
}

/// Create a greater-than-or-equal comparison (>=).
#[must_use]
pub fn gte(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::GtEq,
        right: Box::new(right),
    }
}

/// Create an IS NULL check.
#[must_use]
pub fn is_null(expr: Expr) -> Expr {
    Expr::IsNull {
        expr: Box::new(expr),
        negated: false,
    }
}

/// Create an IS NOT NULL check.
#[must_use]
pub fn is_not_null(expr: Expr) -> Expr {
    Expr::IsNull {
        expr: Box::new(expr),
        negated: true,
    }
}

/// Create a BETWEEN expression.
#[must_use]
pub fn between(expr: Expr, low: Expr, high: Expr) -> Expr {
    Expr::Between {
        expr: Box::new(expr),
        low: Box::new(low),
        high: Box::new(high),
        negated: false,
    }
}

/// Create an IN list expression.
#[must_use]
pub fn in_list(expr: Expr, list: Vec<Expr>) -> Expr {
    Expr::InList {
        expr: Box::new(expr),
        list,
        negated: false,
    }
}

/// Create a NOT IN list expression.
#[must_use]
pub fn not_in_list(expr: Expr, list: Vec<Expr>) -> Expr {
    Expr::InList {
        expr: Box::new(expr),
        list,
        negated: true,
    }
}

/// Create an IN subquery expression.
#[must_use]
pub fn in_subquery(expr: Expr, query: Statement) -> Expr {
    Expr::InSubquery {
        expr: Box::new(expr),
        subquery: Box::new(query),
        negated: false,
    }
}

/// Create a LIKE expression.
#[must_use]
pub fn like(expr: Expr, pattern: Expr) -> Expr {
    Expr::Like {
        expr: Box::new(expr),
        pattern: Box::new(pattern),
        negated: false,
        escape: None,
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Arithmetic helpers
// ═══════════════════════════════════════════════════════════════════════

/// Create an addition expression (+).
#[must_use]
pub fn add(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Plus,
        right: Box::new(right),
    }
}

/// Create a subtraction expression (-).
#[must_use]
pub fn sub(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Minus,
        right: Box::new(right),
    }
}

/// Create a multiplication expression (*).
#[must_use]
pub fn mul(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Multiply,
        right: Box::new(right),
    }
}

/// Create a division expression (/).
#[must_use]
pub fn div(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Divide,
        right: Box::new(right),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Parse helpers
// ═══════════════════════════════════════════════════════════════════════

/// Parse an expression string into an Expr.
///
/// Uses ANSI SQL dialect by default. Returns None if parsing fails.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::parse_expr;
///
/// let expr = parse_expr("x + 1").unwrap();
/// let cond = parse_expr("a = 1 AND b = 2").unwrap();
/// ```
#[must_use]
pub fn parse_expr(sql: &str) -> Option<Expr> {
    parse_expr_dialect(sql, Dialect::Ansi)
}

/// Parse an expression string into an Expr with a specific dialect.
#[must_use]
pub fn parse_expr_dialect(sql: &str, dialect: Dialect) -> Option<Expr> {
    // Parse as a SELECT to extract the expression
    let query = format!("SELECT {sql}");
    match parse(&query, dialect) {
        Ok(Statement::Select(select)) => {
            if let Some(SelectItem::Expr { expr, .. }) = select.columns.first() {
                Some(expr.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Parse a condition string for use in WHERE clauses.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::parse_condition;
///
/// let cond = parse_condition("x > 1 AND y < 10").unwrap();
/// ```
#[must_use]
pub fn parse_condition(sql: &str) -> Option<Expr> {
    parse_condition_dialect(sql, Dialect::Ansi)
}

/// Parse a condition string with a specific dialect.
#[must_use]
pub fn parse_condition_dialect(sql: &str, dialect: Dialect) -> Option<Expr> {
    // Parse as a SELECT with WHERE to extract the condition
    let query = format!("SELECT 1 WHERE {sql}");
    match parse(&query, dialect) {
        Ok(Statement::Select(select)) => select.where_clause,
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Condition Builder
// ═══════════════════════════════════════════════════════════════════════

/// Builder for combining conditions with AND/OR/NOT.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::condition;
///
/// let cond = condition("x = 1")
///     .and("y = 2")
///     .or("z = 3")
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct ConditionBuilder {
    expr: Option<Expr>,
    dialect: Dialect,
}

impl ConditionBuilder {
    /// Create a new condition builder from a string.
    #[must_use]
    pub fn new(condition: &str) -> Self {
        Self::new_with_dialect(condition, Dialect::Ansi)
    }

    /// Create a new condition builder with a specific dialect.
    #[must_use]
    pub fn new_with_dialect(condition: &str, dialect: Dialect) -> Self {
        Self {
            expr: parse_condition_dialect(condition, dialect),
            dialect,
        }
    }

    /// Create a new condition builder from an expression.
    #[must_use]
    pub fn from_expr(expr: Expr) -> Self {
        Self {
            expr: Some(expr),
            dialect: Dialect::Ansi,
        }
    }

    /// Add an AND condition.
    #[must_use]
    pub fn and(self, condition: &str) -> Self {
        let dialect = self.dialect;
        self.and_expr(parse_condition_dialect(condition, dialect))
    }

    /// Add an AND condition from an expression.
    #[must_use]
    pub fn and_expr(self, other: Option<Expr>) -> Self {
        let expr = match (self.expr, other) {
            (Some(left), Some(right)) => Some(Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::And,
                right: Box::new(right),
            }),
            (Some(e), None) | (None, Some(e)) => Some(e),
            (None, None) => None,
        };
        Self {
            expr,
            dialect: self.dialect,
        }
    }

    /// Add an OR condition.
    #[must_use]
    pub fn or(self, condition: &str) -> Self {
        let dialect = self.dialect;
        self.or_expr(parse_condition_dialect(condition, dialect))
    }

    /// Add an OR condition from an expression.
    #[must_use]
    pub fn or_expr(self, other: Option<Expr>) -> Self {
        let expr = match (self.expr, other) {
            (Some(left), Some(right)) => Some(Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::Or,
                right: Box::new(right),
            }),
            (Some(e), None) | (None, Some(e)) => Some(e),
            (None, None) => None,
        };
        Self {
            expr,
            dialect: self.dialect,
        }
    }

    /// Negate the current condition with NOT.
    #[must_use]
    pub fn not(self) -> Self {
        let expr = self.expr.map(|e| Expr::UnaryOp {
            op: crate::ast::UnaryOperator::Not,
            expr: Box::new(e),
        });
        Self {
            expr,
            dialect: self.dialect,
        }
    }

    /// Build the final expression.
    #[must_use]
    pub fn build(self) -> Option<Expr> {
        self.expr
    }
}

/// Create a new condition builder.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::condition;
///
/// let cond = condition("x = 1").and("y = 2").build();
/// ```
#[must_use]
pub fn condition(cond: &str) -> ConditionBuilder {
    ConditionBuilder::new(cond)
}

/// Create a condition builder with a specific dialect.
#[must_use]
pub fn condition_dialect(cond: &str, dialect: Dialect) -> ConditionBuilder {
    ConditionBuilder::new_with_dialect(cond, dialect)
}

// ═══════════════════════════════════════════════════════════════════════
// SELECT Builder
// ═══════════════════════════════════════════════════════════════════════

/// Fluent builder for SELECT statements.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::select;
///
/// let query = select(&["a", "b"])
///     .from("users")
///     .where_clause("active = true")
///     .order_by(&["created_at DESC"])
///     .limit(10)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct SelectBuilder {
    statement: SelectStatement,
    dialect: Dialect,
}

impl Default for SelectBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectBuilder {
    /// Create a new empty SELECT builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            statement: SelectStatement {
                comments: Vec::new(),
                ctes: Vec::new(),
                distinct: false,
                top: None,
                columns: Vec::new(),
                from: None,
                joins: Vec::new(),
                where_clause: None,
                group_by: Vec::new(),
                having: None,
                order_by: Vec::new(),
                limit: None,
                offset: None,
                fetch_first: None,
                qualify: None,
                window_definitions: Vec::new(),
                query_options: None,
            },
            dialect: Dialect::Ansi,
        }
    }

    /// Set the dialect for parsing string inputs.
    #[must_use]
    pub fn dialect(mut self, dialect: Dialect) -> Self {
        self.dialect = dialect;
        self
    }

    /// Add columns to the SELECT list from strings.
    ///
    /// Each string is parsed as an expression.
    #[must_use]
    pub fn columns(mut self, cols: &[&str]) -> Self {
        for col in cols {
            if let Some(expr) = parse_expr_dialect(col, self.dialect) {
                self.statement.columns.push(SelectItem::Expr {
                    expr,
                    alias: None,
                    alias_quote_style: QuoteStyle::None,
                });
            }
        }
        self
    }

    /// Add a single column expression.
    #[must_use]
    pub fn column_expr(mut self, expr: Expr, alias: Option<&str>) -> Self {
        self.statement.columns.push(SelectItem::Expr {
            expr,
            alias: alias.map(String::from),
            alias_quote_style: QuoteStyle::None,
        });
        self
    }

    /// Add a wildcard (*) to the SELECT list.
    #[must_use]
    pub fn all(mut self) -> Self {
        self.statement.columns.push(SelectItem::Wildcard);
        self
    }

    /// Add a qualified wildcard (table.*) to the SELECT list.
    #[must_use]
    pub fn all_from(mut self, table: &str) -> Self {
        self.statement.columns.push(SelectItem::QualifiedWildcard {
            table: table.to_string(),
        });
        self
    }

    /// Set distinct mode.
    #[must_use]
    pub fn distinct(mut self) -> Self {
        self.statement.distinct = true;
        self
    }

    /// Set the FROM clause to a table name.
    #[must_use]
    pub fn from(mut self, table_name: &str) -> Self {
        self.statement.from = Some(FromClause {
            source: TableSource::Table(table(table_name, None)),
        });
        self
    }

    /// Set the FROM clause to a table reference.
    #[must_use]
    pub fn from_table(mut self, table_ref: TableRef) -> Self {
        self.statement.from = Some(FromClause {
            source: TableSource::Table(table_ref),
        });
        self
    }

    /// Set the FROM clause to a subquery.
    #[must_use]
    pub fn from_subquery(mut self, query: Statement, alias: &str) -> Self {
        self.statement.from = Some(FromClause {
            source: TableSource::Subquery {
                query: Box::new(query),
                alias: Some(alias.to_string()),
                alias_quote_style: QuoteStyle::None,
            },
        });
        self
    }

    /// Add a JOIN clause.
    #[must_use]
    pub fn join(self, table_name: &str, on: &str) -> Self {
        self.join_type(table_name, on, JoinType::Inner)
    }

    /// Add a LEFT JOIN clause.
    #[must_use]
    pub fn left_join(self, table_name: &str, on: &str) -> Self {
        self.join_type(table_name, on, JoinType::Left)
    }

    /// Add a RIGHT JOIN clause.
    #[must_use]
    pub fn right_join(self, table_name: &str, on: &str) -> Self {
        self.join_type(table_name, on, JoinType::Right)
    }

    /// Add a FULL JOIN clause.
    #[must_use]
    pub fn full_join(self, table_name: &str, on: &str) -> Self {
        self.join_type(table_name, on, JoinType::Full)
    }

    /// Add a CROSS JOIN clause.
    #[must_use]
    pub fn cross_join(mut self, table_name: &str) -> Self {
        self.statement.joins.push(JoinClause {
            join_type: JoinType::Cross,
            table: TableSource::Table(table(table_name, None)),
            on: None,
            using: Vec::new(),
        });
        self
    }

    /// Add a JOIN with a specific type.
    #[must_use]
    fn join_type(mut self, table_name: &str, on: &str, join_type: JoinType) -> Self {
        let on_expr = parse_condition_dialect(on, self.dialect);
        self.statement.joins.push(JoinClause {
            join_type,
            table: TableSource::Table(table(table_name, None)),
            on: on_expr,
            using: Vec::new(),
        });
        self
    }

    /// Add a JOIN with USING clause.
    #[must_use]
    pub fn join_using(mut self, table_name: &str, columns: &[&str], join_type: JoinType) -> Self {
        self.statement.joins.push(JoinClause {
            join_type,
            table: TableSource::Table(table(table_name, None)),
            on: None,
            using: columns.iter().map(|s| s.to_string()).collect(),
        });
        self
    }

    /// Add a JOIN with a subquery.
    #[must_use]
    pub fn join_subquery(
        mut self,
        query: Statement,
        alias: &str,
        on: &str,
        join_type: JoinType,
    ) -> Self {
        let on_expr = parse_condition_dialect(on, self.dialect);
        self.statement.joins.push(JoinClause {
            join_type,
            table: TableSource::Subquery {
                query: Box::new(query),
                alias: Some(alias.to_string()),
                alias_quote_style: QuoteStyle::None,
            },
            on: on_expr,
            using: Vec::new(),
        });
        self
    }

    /// Set the WHERE clause from a string.
    #[must_use]
    pub fn where_clause(mut self, condition: &str) -> Self {
        self.statement.where_clause = parse_condition_dialect(condition, self.dialect);
        self
    }

    /// Set the WHERE clause from an expression.
    #[must_use]
    pub fn where_expr(mut self, expr: Expr) -> Self {
        self.statement.where_clause = Some(expr);
        self
    }

    /// Add to the WHERE clause with AND.
    #[must_use]
    pub fn and_where(mut self, condition: &str) -> Self {
        let new_cond = parse_condition_dialect(condition, self.dialect);
        self.statement.where_clause = match (self.statement.where_clause, new_cond) {
            (Some(existing), Some(new)) => Some(Expr::BinaryOp {
                left: Box::new(existing),
                op: BinaryOperator::And,
                right: Box::new(new),
            }),
            (Some(e), None) | (None, Some(e)) => Some(e),
            (None, None) => None,
        };
        self
    }

    /// Add to the WHERE clause with OR.
    #[must_use]
    pub fn or_where(mut self, condition: &str) -> Self {
        let new_cond = parse_condition_dialect(condition, self.dialect);
        self.statement.where_clause = match (self.statement.where_clause, new_cond) {
            (Some(existing), Some(new)) => Some(Expr::BinaryOp {
                left: Box::new(existing),
                op: BinaryOperator::Or,
                right: Box::new(new),
            }),
            (Some(e), None) | (None, Some(e)) => Some(e),
            (None, None) => None,
        };
        self
    }

    /// Set the GROUP BY clause.
    #[must_use]
    pub fn group_by(mut self, exprs: &[&str]) -> Self {
        self.statement.group_by = exprs
            .iter()
            .filter_map(|e| parse_expr_dialect(e, self.dialect))
            .collect();
        self
    }

    /// Add a GROUP BY expression.
    #[must_use]
    pub fn add_group_by(mut self, expr: &str) -> Self {
        if let Some(e) = parse_expr_dialect(expr, self.dialect) {
            self.statement.group_by.push(e);
        }
        self
    }

    /// Set the HAVING clause.
    #[must_use]
    pub fn having(mut self, condition: &str) -> Self {
        self.statement.having = parse_condition_dialect(condition, self.dialect);
        self
    }

    /// Set the ORDER BY clause.
    #[must_use]
    pub fn order_by(mut self, exprs: &[&str]) -> Self {
        self.statement.order_by = exprs
            .iter()
            .filter_map(|e| parse_order_by_item(e, self.dialect))
            .collect();
        self
    }

    /// Add an ORDER BY item.
    #[must_use]
    pub fn add_order_by(mut self, expr: &str) -> Self {
        if let Some(item) = parse_order_by_item(expr, self.dialect) {
            self.statement.order_by.push(item);
        }
        self
    }

    /// Add an ORDER BY item with explicit direction.
    #[must_use]
    pub fn add_order_by_expr(
        mut self,
        expr: Expr,
        ascending: bool,
        nulls_first: Option<bool>,
    ) -> Self {
        self.statement.order_by.push(OrderByItem {
            expr,
            ascending,
            nulls_first,
        });
        self
    }

    /// Set the LIMIT clause.
    #[must_use]
    pub fn limit(mut self, n: i64) -> Self {
        self.statement.limit = Some(Expr::Number(n.to_string()));
        self
    }

    /// Set the LIMIT clause from an expression.
    #[must_use]
    pub fn limit_expr(mut self, expr: Expr) -> Self {
        self.statement.limit = Some(expr);
        self
    }

    /// Set the OFFSET clause.
    #[must_use]
    pub fn offset(mut self, n: i64) -> Self {
        self.statement.offset = Some(Expr::Number(n.to_string()));
        self
    }

    /// Set the OFFSET clause from an expression.
    #[must_use]
    pub fn offset_expr(mut self, expr: Expr) -> Self {
        self.statement.offset = Some(expr);
        self
    }

    /// Set TOP N (T-SQL style).
    #[must_use]
    pub fn top(mut self, n: i64) -> Self {
        self.statement.top = Some(Box::new(Expr::Number(n.to_string())));
        self
    }

    /// Set the QUALIFY clause (BigQuery, Snowflake).
    #[must_use]
    pub fn qualify(mut self, condition: &str) -> Self {
        self.statement.qualify = parse_condition_dialect(condition, self.dialect);
        self
    }

    /// Build the final SELECT statement.
    #[must_use]
    pub fn build(self) -> Statement {
        Statement::Select(self.statement)
    }

    /// Build and return the inner SelectStatement.
    #[must_use]
    pub fn build_select(self) -> SelectStatement {
        self.statement
    }
}

/// Create a new SELECT builder with columns.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::select;
///
/// let query = select(&["a", "b", "c"]).from("table_name").build();
/// ```
#[must_use]
pub fn select(columns: &[&str]) -> SelectBuilder {
    SelectBuilder::new().columns(columns)
}

/// Create a SELECT * query.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::select_all;
///
/// let query = select_all().from("users").build();
/// ```
#[must_use]
pub fn select_all() -> SelectBuilder {
    SelectBuilder::new().all()
}

/// Create a SELECT DISTINCT builder.
///
/// # Examples
///
/// ```rust
/// use sqlglot_rust::builder::select_distinct;
///
/// let query = select_distinct(&["category"]).from("products").build();
/// ```
#[must_use]
pub fn select_distinct(columns: &[&str]) -> SelectBuilder {
    SelectBuilder::new().distinct().columns(columns)
}

// ═══════════════════════════════════════════════════════════════════════
// Statement Mutation Methods
// ═══════════════════════════════════════════════════════════════════════

impl SelectStatement {
    /// Add a column to the SELECT list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlglot_rust::builder::select;
    ///
    /// let mut stmt = select(&["a"]).from("t").build_select();
    /// stmt.add_select("b");
    /// ```
    pub fn add_select(&mut self, expr_str: &str) {
        self.add_select_dialect(expr_str, Dialect::Ansi);
    }

    /// Add a column with dialect-specific parsing.
    pub fn add_select_dialect(&mut self, expr_str: &str, dialect: Dialect) {
        if let Some(expr) = parse_expr_dialect(expr_str, dialect) {
            self.columns.push(SelectItem::Expr {
                expr,
                alias: None,
                alias_quote_style: QuoteStyle::None,
            });
        }
    }

    /// Add an expression to the SELECT list.
    pub fn add_select_expr(&mut self, expr: Expr, alias: Option<&str>) {
        self.columns.push(SelectItem::Expr {
            expr,
            alias: alias.map(String::from),
            alias_quote_style: QuoteStyle::None,
        });
    }

    /// Add a condition to the WHERE clause (AND).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlglot_rust::builder::select;
    ///
    /// let mut stmt = select(&["a"]).from("t").build_select();
    /// stmt.add_where("x > 1");
    /// stmt.add_where("y < 10");
    /// ```
    pub fn add_where(&mut self, condition: &str) {
        self.add_where_dialect(condition, Dialect::Ansi);
    }

    /// Add a WHERE condition with dialect-specific parsing.
    pub fn add_where_dialect(&mut self, condition: &str, dialect: Dialect) {
        let new_cond = parse_condition_dialect(condition, dialect);
        self.where_clause = match (self.where_clause.take(), new_cond) {
            (Some(existing), Some(new)) => Some(Expr::BinaryOp {
                left: Box::new(existing),
                op: BinaryOperator::And,
                right: Box::new(new),
            }),
            (Some(e), None) | (None, Some(e)) => Some(e),
            (None, None) => None,
        };
    }

    /// Add an expression to the WHERE clause (AND).
    pub fn add_where_expr(&mut self, expr: Expr) {
        self.where_clause = match self.where_clause.take() {
            Some(existing) => Some(Expr::BinaryOp {
                left: Box::new(existing),
                op: BinaryOperator::And,
                right: Box::new(expr),
            }),
            None => Some(expr),
        };
    }

    /// Add a JOIN clause.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlglot_rust::builder::select;
    /// use sqlglot_rust::ast::JoinType;
    ///
    /// let mut stmt = select(&["*"]).from("users").build_select();
    /// stmt.add_join("orders", "users.id = orders.user_id", JoinType::Left);
    /// ```
    pub fn add_join(&mut self, table_name: &str, on: &str, join_type: JoinType) {
        self.add_join_dialect(table_name, on, join_type, Dialect::Ansi);
    }

    /// Add a JOIN with dialect-specific parsing.
    pub fn add_join_dialect(
        &mut self,
        table_name: &str,
        on: &str,
        join_type: JoinType,
        dialect: Dialect,
    ) {
        let on_expr = parse_condition_dialect(on, dialect);
        self.joins.push(JoinClause {
            join_type,
            table: TableSource::Table(table(table_name, None)),
            on: on_expr,
            using: Vec::new(),
        });
    }

    /// Add a JOIN with a subquery.
    pub fn add_join_subquery(
        &mut self,
        query: Statement,
        alias: &str,
        on: &str,
        join_type: JoinType,
    ) {
        self.add_join_subquery_dialect(query, alias, on, join_type, Dialect::Ansi);
    }

    /// Add a JOIN with a subquery and dialect-specific parsing.
    pub fn add_join_subquery_dialect(
        &mut self,
        query: Statement,
        alias: &str,
        on: &str,
        join_type: JoinType,
        dialect: Dialect,
    ) {
        let on_expr = parse_condition_dialect(on, dialect);
        self.joins.push(JoinClause {
            join_type,
            table: TableSource::Subquery {
                query: Box::new(query),
                alias: Some(alias.to_string()),
                alias_quote_style: QuoteStyle::None,
            },
            on: on_expr,
            using: Vec::new(),
        });
    }

    /// Wrap this SELECT as a subquery with an alias.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlglot_rust::builder::select;
    ///
    /// let inner = select(&["id", "name"]).from("users").build_select();
    /// let subq = inner.as_subquery("u");
    /// ```
    #[must_use]
    pub fn as_subquery(self, alias: &str) -> TableSource {
        TableSource::Subquery {
            query: Box::new(Statement::Select(self)),
            alias: Some(alias.to_string()),
            alias_quote_style: QuoteStyle::None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Helper functions
// ═══════════════════════════════════════════════════════════════════════

/// Parse an ORDER BY item string like "col ASC" or "col DESC NULLS FIRST"
fn parse_order_by_item(s: &str, dialect: Dialect) -> Option<OrderByItem> {
    let s = s.trim();
    let upper = s.to_uppercase();

    // Check for NULLS FIRST/LAST
    let nulls_first = if upper.contains("NULLS FIRST") {
        Some(true)
    } else if upper.contains("NULLS LAST") {
        Some(false)
    } else {
        None
    };

    // Remove NULLS clause for parsing
    let s = s
        .replace("NULLS FIRST", "")
        .replace("NULLS LAST", "")
        .replace("nulls first", "")
        .replace("nulls last", "");
    let s = s.trim();

    // Check for ASC/DESC
    let (expr_str, ascending) = if s.to_uppercase().ends_with(" DESC") {
        (&s[..s.len() - 5], false)
    } else if s.to_uppercase().ends_with(" ASC") {
        (&s[..s.len() - 4], true)
    } else {
        (s, true)
    };

    parse_expr_dialect(expr_str.trim(), dialect).map(|expr| OrderByItem {
        expr,
        ascending,
        nulls_first,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate;

    #[test]
    fn test_column() {
        let col = column("name", None);
        assert!(
            matches!(col, Expr::Column { name, table, .. } if name == "name" && table.is_none())
        );

        let qualified = column("id", Some("users"));
        assert!(matches!(qualified, Expr::Column { name, table, .. }
            if name == "id" && table == Some("users".to_string())));
    }

    #[test]
    fn test_table() {
        let tbl = table("users", None);
        assert_eq!(tbl.name, "users");
        assert!(tbl.schema.is_none());

        let qualified = table("orders", Some("public"));
        assert_eq!(qualified.name, "orders");
        assert_eq!(qualified.schema, Some("public".to_string()));
    }

    #[test]
    fn test_literals() {
        assert!(matches!(literal(42), Expr::Number(n) if n == "42"));
        assert!(matches!(string_literal("hello"), Expr::StringLiteral(s) if s == "hello"));
        assert!(matches!(boolean(true), Expr::Boolean(true)));
        assert!(matches!(null(), Expr::Null));
    }

    #[test]
    fn test_cast() {
        let col = column("id", None);
        let casted = cast(col, DataType::BigInt);
        assert!(matches!(
            casted,
            Expr::Cast {
                data_type: DataType::BigInt,
                ..
            }
        ));
    }

    #[test]
    fn test_and_all() {
        let cond1 = eq(column("x", None), literal(1));
        let cond2 = eq(column("y", None), literal(2));

        let combined = and_all(vec![cond1, cond2]).unwrap();
        assert!(matches!(
            combined,
            Expr::BinaryOp {
                op: BinaryOperator::And,
                ..
            }
        ));

        // Empty returns None
        assert!(and_all(Vec::<Expr>::new()).is_none());
    }

    #[test]
    fn test_or_all() {
        let cond1 = eq(column("x", None), literal(1));
        let cond2 = eq(column("y", None), literal(2));

        let combined = or_all(vec![cond1, cond2]).unwrap();
        assert!(matches!(
            combined,
            Expr::BinaryOp {
                op: BinaryOperator::Or,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_expr() {
        let expr = parse_expr("x + 1").unwrap();
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOperator::Plus,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_condition() {
        let cond = parse_condition("x > 1 AND y < 10").unwrap();
        assert!(matches!(
            cond,
            Expr::BinaryOp {
                op: BinaryOperator::And,
                ..
            }
        ));
    }

    #[test]
    fn test_condition_builder() {
        let cond = condition("x = 1").and("y = 2").or("z = 3").build();
        assert!(cond.is_some());
    }

    #[test]
    fn test_condition_builder_not() {
        let cond = condition("x = 1").not().build().unwrap();
        assert!(matches!(
            cond,
            Expr::UnaryOp {
                op: crate::ast::UnaryOperator::Not,
                ..
            }
        ));
    }

    #[test]
    fn test_select_builder_basic() {
        let query = select(&["a", "b"]).from("users").build();
        let sql = generate(&query, Dialect::Ansi);
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("a"));
        assert!(sql.contains("b"));
        assert!(sql.contains("FROM users"));
    }

    #[test]
    fn test_select_builder_where() {
        let query = select(&["*"])
            .from("users")
            .where_clause("active = true")
            .build();
        let sql = generate(&query, Dialect::Ansi);
        assert!(sql.contains("WHERE"));
    }

    #[test]
    fn test_select_builder_join() {
        let query = select(&["u.name", "o.total"])
            .from("users")
            .join("orders", "users.id = orders.user_id")
            .build();
        let sql = generate(&query, Dialect::Ansi);
        assert!(sql.contains("JOIN"));
    }

    #[test]
    fn test_select_builder_group_by() {
        let query = select(&["category", "COUNT(*)"])
            .from("products")
            .group_by(&["category"])
            .having("COUNT(*) > 5")
            .build();
        let sql = generate(&query, Dialect::Ansi);
        assert!(sql.contains("GROUP BY"));
        assert!(sql.contains("HAVING"));
    }

    #[test]
    fn test_select_builder_order_limit() {
        let query = select(&["*"])
            .from("users")
            .order_by(&["created_at DESC"])
            .limit(10)
            .offset(5)
            .build();
        let sql = generate(&query, Dialect::Ansi);
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("LIMIT 10"));
        assert!(sql.contains("OFFSET 5"));
    }

    #[test]
    fn test_select_builder_distinct() {
        let query = select_distinct(&["category"]).from("products").build();
        let sql = generate(&query, Dialect::Ansi);
        assert!(sql.contains("SELECT DISTINCT"));
    }

    #[test]
    fn test_select_all() {
        let query = select_all().from("users").build();
        let sql = generate(&query, Dialect::Ansi);
        assert!(sql.contains("SELECT *"));
    }

    #[test]
    fn test_mutation_add_select() {
        let mut stmt = select(&["a"]).from("t").build_select();
        stmt.add_select("b");
        assert_eq!(stmt.columns.len(), 2);
    }

    #[test]
    fn test_mutation_add_where() {
        let mut stmt = select(&["*"]).from("t").build_select();
        stmt.add_where("x > 1");
        stmt.add_where("y < 10");
        // Should be combined with AND
        assert!(stmt.where_clause.is_some());
    }

    #[test]
    fn test_mutation_add_join() {
        let mut stmt = select(&["*"]).from("users").build_select();
        stmt.add_join("orders", "users.id = orders.user_id", JoinType::Inner);
        assert_eq!(stmt.joins.len(), 1);
    }

    #[test]
    fn test_as_subquery() {
        let inner = select(&["id"]).from("users").build_select();
        let source = inner.as_subquery("u");
        assert!(matches!(source, TableSource::Subquery { alias: Some(a), .. } if a == "u"));
    }

    #[test]
    fn test_comparison_helpers() {
        let e = eq(column("a", None), literal(1));
        assert!(matches!(
            e,
            Expr::BinaryOp {
                op: BinaryOperator::Eq,
                ..
            }
        ));

        let e = neq(column("a", None), literal(1));
        assert!(matches!(
            e,
            Expr::BinaryOp {
                op: BinaryOperator::Neq,
                ..
            }
        ));

        let e = lt(column("a", None), literal(1));
        assert!(matches!(
            e,
            Expr::BinaryOp {
                op: BinaryOperator::Lt,
                ..
            }
        ));

        let e = gt(column("a", None), literal(1));
        assert!(matches!(
            e,
            Expr::BinaryOp {
                op: BinaryOperator::Gt,
                ..
            }
        ));
    }

    #[test]
    fn test_arithmetic_helpers() {
        let e = add(column("a", None), literal(1));
        assert!(matches!(
            e,
            Expr::BinaryOp {
                op: BinaryOperator::Plus,
                ..
            }
        ));

        let e = mul(column("a", None), literal(2));
        assert!(matches!(
            e,
            Expr::BinaryOp {
                op: BinaryOperator::Multiply,
                ..
            }
        ));
    }

    #[test]
    fn test_is_null_helpers() {
        let e = is_null(column("a", None));
        assert!(matches!(e, Expr::IsNull { negated: false, .. }));

        let e = is_not_null(column("a", None));
        assert!(matches!(e, Expr::IsNull { negated: true, .. }));
    }

    #[test]
    fn test_between() {
        let e = between(column("x", None), literal(1), literal(10));
        assert!(matches!(e, Expr::Between { negated: false, .. }));
    }

    #[test]
    fn test_in_list() {
        let e = in_list(
            column("status", None),
            vec![string_literal("active"), string_literal("pending")],
        );
        assert!(matches!(e, Expr::InList { negated: false, .. }));
    }

    #[test]
    fn test_like() {
        let e = like(column("name", None), string_literal("%John%"));
        assert!(matches!(e, Expr::Like { negated: false, .. }));
    }

    #[test]
    fn test_func() {
        let f = func("UPPER", vec![column("name", None)]);
        assert!(matches!(f, Expr::Function { name, distinct: false, .. } if name == "UPPER"));
    }

    #[test]
    fn test_func_distinct() {
        let f = func_distinct("COUNT", vec![column("id", None)]);
        assert!(matches!(f, Expr::Function { name, distinct: true, .. } if name == "COUNT"));
    }

    #[test]
    fn test_alias() {
        let e = alias(column("first_name", None), "name");
        assert!(matches!(e, Expr::Alias { name, .. } if name == "name"));
    }

    #[test]
    fn test_subquery_and_exists() {
        let inner = select(&["1"]).from("users").where_clause("id = 1").build();
        let sub = subquery(inner.clone());
        assert!(matches!(sub, Expr::Subquery(_)));

        let ex = exists(inner, false);
        assert!(matches!(ex, Expr::Exists { negated: false, .. }));
    }

    #[test]
    fn test_star_and_qualified_star() {
        let s = star();
        assert!(matches!(s, Expr::Star));

        let qs = qualified_star("users");
        assert!(matches!(qs, Expr::QualifiedWildcard { table } if table == "users"));
    }

    #[test]
    fn test_complex_query() {
        // Build a more complex query
        let query = select(&["u.id", "u.name", "COUNT(o.id) AS order_count"])
            .from("users")
            .join("orders", "u.id = o.user_id")
            .where_clause("u.active = true")
            .and_where("o.created_at > '2024-01-01'")
            .group_by(&["u.id", "u.name"])
            .having("COUNT(o.id) > 0")
            .order_by(&["order_count DESC"])
            .limit(10)
            .build();

        let sql = generate(&query, Dialect::Postgres);
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("JOIN"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("GROUP BY"));
        assert!(sql.contains("HAVING"));
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("LIMIT"));
    }

    #[test]
    fn test_subquery_in_from() {
        let inner = select(&["id", "name"])
            .from("users")
            .where_clause("active = true")
            .build();
        let outer = select(&["*"]).from_subquery(inner, "active_users").build();

        let sql = generate(&outer, Dialect::Ansi);
        assert!(sql.contains("FROM (SELECT"));
        assert!(sql.contains(") AS active_users"));
    }
}
