//! In-memory SQL execution engine for testing and validation.
//!
//! Provides the ability to run SQL queries against Rust data structures,
//! supporting SELECT with WHERE, GROUP BY, HAVING, ORDER BY, LIMIT/OFFSET,
//! JOINs, aggregate functions, subqueries, CTEs, and set operations.
//!
//! # Example
//!
//! ```
//! use std::collections::HashMap;
//! use sqlglot_rust::executor::{execute, Table, Value};
//!
//! let mut tables = HashMap::new();
//! tables.insert("t".to_string(), Table::new(
//!     vec!["a".to_string(), "b".to_string()],
//!     vec![
//!         vec![Value::Int(1), Value::String("x".to_string())],
//!         vec![Value::Int(2), Value::String("y".to_string())],
//!     ],
//! ));
//! let result = execute("SELECT a, b FROM t WHERE a > 1", &tables).unwrap();
//! assert_eq!(result.row_count(), 1);
//! ```

mod engine;
mod eval;

use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::ast::Statement;
use crate::dialects::Dialect;
use crate::errors::Result;
use crate::parser;

// ═══════════════════════════════════════════════════════════════════════
// Value
// ═══════════════════════════════════════════════════════════════════════

/// A SQL value that can be stored in a table cell or produced by
/// expression evaluation.
#[derive(Debug, Clone)]
pub enum Value {
    /// SQL NULL.
    Null,
    /// A boolean value.
    Boolean(bool),
    /// A 64-bit integer.
    Int(i64),
    /// A 64-bit floating-point number.
    Float(f64),
    /// A string value.
    String(String),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::String(a), Value::String(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Value::Null => {}
            Value::Boolean(b) => b.hash(state),
            Value::Int(i) => i.hash(state),
            Value::Float(f) => f.to_bits().hash(state),
            Value::String(s) => s.hash(state),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "NULL"),
            Value::Boolean(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(v) => {
                if v.fract() == 0.0 && v.abs() < 1e15 {
                    write!(f, "{v:.1}")
                } else {
                    write!(f, "{v}")
                }
            }
            Value::String(s) => write!(f, "{s}"),
        }
    }
}

impl Value {
    /// Returns `true` if this value is NULL.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns `true` if this value is truthy (non-NULL, non-zero,
    /// non-empty).
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Boolean(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
        }
    }

    /// Try to convert to `f64`.
    #[must_use]
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            Value::String(s) => s.parse().ok(),
            Value::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            Value::Null => None,
        }
    }

    /// Try to convert to `i64`.
    #[must_use]
    pub fn to_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(*f as i64),
            Value::String(s) => s.parse().ok(),
            Value::Boolean(b) => Some(i64::from(*b)),
            Value::Null => None,
        }
    }

    /// Convert to a `String` representation (empty string for NULL).
    #[must_use]
    pub fn to_string_val(&self) -> String {
        match self {
            Value::Null => String::new(),
            other => other.to_string(),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Null, Value::Null) => Some(std::cmp::Ordering::Equal),
            (Value::Null, _) => Some(std::cmp::Ordering::Less),
            (_, Value::Null) => Some(std::cmp::Ordering::Greater),
            (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)),
            (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
            (Value::Boolean(a), Value::Boolean(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Table
// ═══════════════════════════════════════════════════════════════════════

/// An in-memory table with named columns and rows of values.
#[derive(Debug, Clone)]
pub struct Table {
    /// Column names.
    pub columns: Vec<String>,
    /// Row data. Each inner `Vec` has one entry per column.
    pub rows: Vec<Vec<Value>>,
}

impl Table {
    /// Create a new table from owned column names and rows.
    pub fn new(columns: Vec<String>, rows: Vec<Vec<Value>>) -> Self {
        Self { columns, rows }
    }

    /// Create a table from string-slice column names.
    pub fn from_rows(columns: Vec<&str>, rows: Vec<Vec<Value>>) -> Self {
        Self {
            columns: columns.into_iter().map(String::from).collect(),
            rows,
        }
    }
}

/// A mapping of table names to in-memory tables.
pub type Tables = HashMap<String, Table>;

// ═══════════════════════════════════════════════════════════════════════
// ResultSet
// ═══════════════════════════════════════════════════════════════════════

/// The result of executing a SQL query.
#[derive(Debug, Clone)]
pub struct ResultSet {
    /// Column names in the result.
    pub columns: Vec<String>,
    /// Row data.
    pub rows: Vec<Vec<Value>>,
}

impl ResultSet {
    /// Create a new result set.
    #[must_use]
    pub fn new(columns: Vec<String>, rows: Vec<Vec<Value>>) -> Self {
        Self { columns, rows }
    }

    /// Create an empty result set.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            columns: vec![],
            rows: vec![],
        }
    }

    /// Number of rows.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Number of columns.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }
}

impl fmt::Display for ResultSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.columns.is_empty() {
            return write!(f, "(empty)");
        }
        // Compute column widths.
        let mut widths: Vec<usize> = self.columns.iter().map(|c| c.len()).collect();
        for row in &self.rows {
            for (i, val) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(val.to_string().len());
                }
            }
        }
        // Header.
        let header: Vec<String> = self
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{:width$}", c, width = widths[i]))
            .collect();
        writeln!(f, "{}", header.join(" | "))?;
        let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
        writeln!(f, "{}", sep.join("-+-"))?;
        // Rows.
        for row in &self.rows {
            let cells: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(i, v)| format!("{:width$}", v, width = widths.get(i).copied().unwrap_or(0)))
                .collect();
            writeln!(f, "{}", cells.join(" | "))?;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
// RowContext (internal)
// ═══════════════════════════════════════════════════════════════════════

/// Internal row context used for expression evaluation.
///
/// Columns are stored as `"table_alias.column_name"` (lowercased) so
/// that both qualified and unqualified look-ups work.
#[derive(Debug, Clone)]
pub(crate) struct RowContext {
    pub columns: Vec<String>,
    pub values: Vec<Value>,
}

impl RowContext {
    pub fn empty() -> Self {
        Self {
            columns: vec![],
            values: vec![],
        }
    }

    pub fn new(columns: Vec<String>, values: Vec<Value>) -> Self {
        Self { columns, values }
    }

    /// Look up a value by unqualified column name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        let name_lower = name.to_lowercase();
        // Exact match first.
        for (i, col) in self.columns.iter().enumerate() {
            if col.to_lowercase() == name_lower {
                return Some(&self.values[i]);
            }
        }
        // Strip table qualifier.
        for (i, col) in self.columns.iter().enumerate() {
            let col_lower = col.to_lowercase();
            if let Some((_, suffix)) = col_lower.rsplit_once('.')
                && suffix == name_lower
            {
                return Some(&self.values[i]);
            }
        }
        None
    }

    /// Look up a value by qualified column name (`table.column`).
    pub fn get_qualified(&self, table: &str, name: &str) -> Option<&Value> {
        let qualified = format!("{}.{}", table, name).to_lowercase();
        for (i, col) in self.columns.iter().enumerate() {
            if col.to_lowercase() == qualified {
                return Some(&self.values[i]);
            }
        }
        self.get(name)
    }

    /// Merge two row contexts (used for JOINs).
    pub fn merge(&self, other: &RowContext) -> RowContext {
        let mut columns = self.columns.clone();
        let mut values = self.values.clone();
        columns.extend(other.columns.iter().cloned());
        values.extend(other.values.iter().cloned());
        RowContext { columns, values }
    }

    /// Create a NULL-filled context with the given column names.
    pub fn null_row(columns: &[String]) -> RowContext {
        RowContext {
            columns: columns.to_vec(),
            values: vec![Value::Null; columns.len()],
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════════════════════════════════

/// Execute a SQL query string against the provided tables.
///
/// Parses the query as ANSI SQL, then runs it in-memory against
/// `tables`.
pub fn execute(sql: &str, tables: &Tables) -> Result<ResultSet> {
    let stmt = parser::parse(sql, Dialect::Ansi)?;
    execute_statement(&stmt, tables)
}

/// Execute a pre-parsed [`Statement`] against the provided tables.
pub fn execute_statement(stmt: &Statement, tables: &Tables) -> Result<ResultSet> {
    let mut ctx = engine::ExecutionContext::new(tables);
    ctx.execute(stmt)
}
