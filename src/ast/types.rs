use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════
// Identifier quoting style
// ═══════════════════════════════════════════════════════════════════════

/// How an identifier (column, table, alias) was quoted in the source SQL.
///
/// Used to preserve and transform quoting across dialects (e.g. backtick
/// for MySQL/BigQuery → double-quote for PostgreSQL → bracket for T-SQL).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum QuoteStyle {
    /// Bare / unquoted identifier
    #[default]
    None,
    /// `"identifier"` — ANSI SQL, PostgreSQL, Oracle, Snowflake, etc.
    DoubleQuote,
    /// `` `identifier` `` — MySQL, BigQuery, Hive, Spark, etc.
    Backtick,
    /// `[identifier]` — T-SQL / SQL Server
    Bracket,
}

impl QuoteStyle {
    /// Returns the canonical quoting style for the given dialect.
    #[must_use]
    pub fn for_dialect(dialect: crate::dialects::Dialect) -> Self {
        use crate::dialects::Dialect;
        match dialect {
            Dialect::Tsql | Dialect::Fabric => QuoteStyle::Bracket,
            Dialect::Mysql | Dialect::BigQuery | Dialect::Hive | Dialect::Spark
            | Dialect::Databricks | Dialect::Doris | Dialect::SingleStore
            | Dialect::StarRocks => QuoteStyle::Backtick,
            // ANSI, Postgres, Oracle, Snowflake, Presto, Trino, etc.
            _ => QuoteStyle::DoubleQuote,
        }
    }

    /// Returns `true` when the identifier carries explicit quoting.
    #[must_use]
    pub fn is_quoted(self) -> bool {
        !matches!(self, QuoteStyle::None)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Top-level statement types
// ═══════════════════════════════════════════════════════════════════════

/// A fully parsed SQL statement.
///
/// Corresponds to the top-level node in sqlglot's expression tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    CreateTable(CreateTableStatement),
    DropTable(DropTableStatement),
    /// UNION / INTERSECT / EXCEPT between queries
    SetOperation(SetOperationStatement),
    /// ALTER TABLE ...
    AlterTable(AlterTableStatement),
    /// CREATE VIEW ...
    CreateView(CreateViewStatement),
    /// DROP VIEW ...
    DropView(DropViewStatement),
    /// TRUNCATE TABLE ...
    Truncate(TruncateStatement),
    /// BEGIN / COMMIT / ROLLBACK
    Transaction(TransactionStatement),
    /// EXPLAIN <statement>
    Explain(ExplainStatement),
    /// USE database
    Use(UseStatement),
    /// Raw / passthrough expression (for expressions that don't fit a specific statement type)
    Expression(Expr),
}

// ═══════════════════════════════════════════════════════════════════════
// SELECT
// ═══════════════════════════════════════════════════════════════════════

/// A SELECT statement, including CTEs.
///
/// Aligned with sqlglot's `Select` expression which wraps `With`, `From`,
/// `Where`, `Group`, `Having`, `Order`, `Limit`, `Offset`, `Window`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectStatement {
    /// Common Table Expressions (WITH clause)
    pub ctes: Vec<Cte>,
    pub distinct: bool,
    /// TOP N (TSQL-style)
    pub top: Option<Box<Expr>>,
    pub columns: Vec<SelectItem>,
    pub from: Option<FromClause>,
    pub joins: Vec<JoinClause>,
    pub where_clause: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub order_by: Vec<OrderByItem>,
    pub limit: Option<Expr>,
    pub offset: Option<Expr>,
    /// Oracle-style FETCH FIRST n ROWS ONLY
    pub fetch_first: Option<Expr>,
    /// QUALIFY clause (BigQuery, Snowflake)
    pub qualify: Option<Expr>,
    /// Named WINDOW definitions
    pub window_definitions: Vec<WindowDefinition>,
}

/// A Common Table Expression: `name [(col1, col2)] AS [NOT] MATERIALIZED (query)`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cte {
    pub name: String,
    pub columns: Vec<String>,
    pub query: Box<Statement>,
    pub materialized: Option<bool>,
    pub recursive: bool,
}

/// Named WINDOW definition: `window_name AS (window_spec)`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowDefinition {
    pub name: String,
    pub spec: WindowSpec,
}

// ═══════════════════════════════════════════════════════════════════════
// Set operations (UNION, INTERSECT, EXCEPT)
// ═══════════════════════════════════════════════════════════════════════

/// UNION / INTERSECT / EXCEPT between two or more queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetOperationStatement {
    pub op: SetOperationType,
    pub all: bool,
    pub left: Box<Statement>,
    pub right: Box<Statement>,
    pub order_by: Vec<OrderByItem>,
    pub limit: Option<Expr>,
    pub offset: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetOperationType {
    Union,
    Intersect,
    Except,
}

// ═══════════════════════════════════════════════════════════════════════
// SELECT items and FROM
// ═══════════════════════════════════════════════════════════════════════

/// An item in a SELECT list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelectItem {
    /// `*`
    Wildcard,
    /// `table.*`
    QualifiedWildcard { table: String },
    /// An expression with optional alias: `expr AS alias`
    Expr { expr: Expr, alias: Option<String> },
}

/// A FROM clause, now supporting subqueries and multiple tables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FromClause {
    pub source: TableSource,
}

/// A table source can be a table reference, subquery, or table function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableSource {
    Table(TableRef),
    Subquery {
        query: Box<Statement>,
        alias: Option<String>,
    },
    TableFunction {
        name: String,
        args: Vec<Expr>,
        alias: Option<String>,
    },
    /// LATERAL subquery or function
    Lateral {
        source: Box<TableSource>,
    },
    /// UNNEST(array_expr)
    Unnest {
        expr: Box<Expr>,
        alias: Option<String>,
        with_offset: bool,
    },
}

/// A reference to a table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableRef {
    pub catalog: Option<String>,
    pub schema: Option<String>,
    pub name: String,
    pub alias: Option<String>,
    /// How the table name was quoted in the source SQL.
    #[serde(default)]
    pub name_quote_style: QuoteStyle,
}

/// A JOIN clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: TableSource,
    pub on: Option<Expr>,
    pub using: Vec<String>,
}

/// The type of JOIN.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
    /// NATURAL JOIN
    Natural,
    /// LATERAL JOIN
    Lateral,
}

/// An ORDER BY item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByItem {
    pub expr: Expr,
    pub ascending: bool,
    /// NULLS FIRST / NULLS LAST
    pub nulls_first: Option<bool>,
}

// ═══════════════════════════════════════════════════════════════════════
// Expressions (the core of the AST)
// ═══════════════════════════════════════════════════════════════════════

/// An expression in SQL.
///
/// This enum is aligned with sqlglot's Expression class hierarchy.
/// Key additions over the basic implementation:
/// - Subquery, Exists, Cast, Extract, Window functions
/// - TypedString, Interval, Array/Struct constructors
/// - Postgres-style casting (`::`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// A column reference, possibly qualified: `[catalog.][schema.]table.column`
    Column {
        table: Option<String>,
        name: String,
        /// How the column name was quoted in the source SQL.
        #[serde(default)]
        quote_style: QuoteStyle,
        /// How the table qualifier was quoted, if present.
        #[serde(default)]
        table_quote_style: QuoteStyle,
    },
    /// A numeric literal.
    Number(String),
    /// A string literal.
    StringLiteral(String),
    /// A boolean literal.
    Boolean(bool),
    /// NULL literal.
    Null,
    /// A binary operation: `left op right`
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOperator,
        right: Box<Expr>,
    },
    /// A unary operation: `op expr`
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expr>,
    },
    /// A function call: `name(args...)` with optional DISTINCT, ORDER BY, etc.
    Function {
        name: String,
        args: Vec<Expr>,
        distinct: bool,
        /// FILTER (WHERE expr) clause on aggregate
        filter: Option<Box<Expr>>,
        /// OVER window specification for window functions
        over: Option<WindowSpec>,
    },
    /// `expr BETWEEN low AND high`
    Between {
        expr: Box<Expr>,
        low: Box<Expr>,
        high: Box<Expr>,
        negated: bool,
    },
    /// `expr IN (list...)` or `expr IN (subquery)`
    InList {
        expr: Box<Expr>,
        list: Vec<Expr>,
        negated: bool,
    },
    /// `expr IN (SELECT ...)`
    InSubquery {
        expr: Box<Expr>,
        subquery: Box<Statement>,
        negated: bool,
    },
    /// `expr IS [NOT] NULL`
    IsNull {
        expr: Box<Expr>,
        negated: bool,
    },
    /// `expr [NOT] LIKE pattern [ESCAPE escape_char]`
    Like {
        expr: Box<Expr>,
        pattern: Box<Expr>,
        negated: bool,
        escape: Option<Box<Expr>>,
    },
    /// `expr [NOT] ILIKE pattern [ESCAPE escape_char]` (case-insensitive LIKE)
    ILike {
        expr: Box<Expr>,
        pattern: Box<Expr>,
        negated: bool,
        escape: Option<Box<Expr>>,
    },
    /// `CASE [operand] WHEN ... THEN ... ELSE ... END`
    Case {
        operand: Option<Box<Expr>>,
        when_clauses: Vec<(Expr, Expr)>,
        else_clause: Option<Box<Expr>>,
    },
    /// A parenthesized sub-expression.
    Nested(Box<Expr>),
    /// A wildcard `*` used in contexts like `COUNT(*)`.
    Wildcard,
    /// A scalar subquery: `(SELECT ...)`
    Subquery(Box<Statement>),
    /// `EXISTS (SELECT ...)`
    Exists {
        subquery: Box<Statement>,
        negated: bool,
    },
    /// `CAST(expr AS type)` or `expr::type` (PostgreSQL)
    Cast {
        expr: Box<Expr>,
        data_type: DataType,
    },
    /// `TRY_CAST(expr AS type)`
    TryCast {
        expr: Box<Expr>,
        data_type: DataType,
    },
    /// `EXTRACT(field FROM expr)`
    Extract {
        field: DateTimeField,
        expr: Box<Expr>,
    },
    /// `INTERVAL 'value' unit`
    Interval {
        value: Box<Expr>,
        unit: Option<DateTimeField>,
    },
    /// Array literal: `ARRAY[1, 2, 3]` or `[1, 2, 3]`
    ArrayLiteral(Vec<Expr>),
    /// Struct literal / row constructor: `(1, 'a', true)`
    Tuple(Vec<Expr>),
    /// `COALESCE(a, b, c)`
    Coalesce(Vec<Expr>),
    /// `IF(condition, true_val, false_val)` (MySQL, BigQuery)
    If {
        condition: Box<Expr>,
        true_val: Box<Expr>,
        false_val: Option<Box<Expr>>,
    },
    /// `NULLIF(a, b)`
    NullIf {
        expr: Box<Expr>,
        r#else: Box<Expr>,
    },
    /// `expr COLLATE collation`
    Collate {
        expr: Box<Expr>,
        collation: String,
    },
    /// Parameter / placeholder: `$1`, `?`, `:name`
    Parameter(String),
    /// A type expression used in DDL contexts or CAST
    TypeExpr(DataType),
    /// `table.*` in expression context
    QualifiedWildcard { table: String },
    /// Star expression `*`
    Star,
    /// Alias expression: `expr AS name`
    Alias {
        expr: Box<Expr>,
        name: String,
    },
    /// Array access: `expr[index]`
    ArrayIndex {
        expr: Box<Expr>,
        index: Box<Expr>,
    },
    /// JSON access: `expr->key` or `expr->>key`
    JsonAccess {
        expr: Box<Expr>,
        path: Box<Expr>,
        /// false = ->, true = ->>
        as_text: bool,
    },
    /// Lambda expression: `x -> x + 1`
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },
    /// `DEFAULT` keyword in INSERT/UPDATE contexts
    Default,
}

// ═══════════════════════════════════════════════════════════════════════
// Window specification
// ═══════════════════════════════════════════════════════════════════════

/// Window specification for window functions: OVER (PARTITION BY ... ORDER BY ... frame)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowSpec {
    /// Reference to a named window
    pub window_ref: Option<String>,
    pub partition_by: Vec<Expr>,
    pub order_by: Vec<OrderByItem>,
    pub frame: Option<WindowFrame>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowFrame {
    pub kind: WindowFrameKind,
    pub start: WindowFrameBound,
    pub end: Option<WindowFrameBound>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowFrameKind {
    Rows,
    Range,
    Groups,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindowFrameBound {
    CurrentRow,
    Preceding(Option<Box<Expr>>),  // None = UNBOUNDED PRECEDING
    Following(Option<Box<Expr>>),  // None = UNBOUNDED FOLLOWING
}

// ═══════════════════════════════════════════════════════════════════════
// Date/time fields (for EXTRACT, INTERVAL)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateTimeField {
    Year,
    Quarter,
    Month,
    Week,
    Day,
    DayOfWeek,
    DayOfYear,
    Hour,
    Minute,
    Second,
    Millisecond,
    Microsecond,
    Nanosecond,
    Epoch,
    Timezone,
    TimezoneHour,
    TimezoneMinute,
}

// ═══════════════════════════════════════════════════════════════════════
// Operators
// ═══════════════════════════════════════════════════════════════════════

/// Binary operators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOperator {
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Eq,
    Neq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    Xor,
    Concat,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
    /// `->` JSON access operator
    Arrow,
    /// `->>` JSON text access
    DoubleArrow,
}

/// Unary operators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Minus,
    Plus,
    BitwiseNot,
}

// ═══════════════════════════════════════════════════════════════════════
// DML statements
// ═══════════════════════════════════════════════════════════════════════

/// An INSERT statement, now supporting INSERT ... SELECT and ON CONFLICT.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsertStatement {
    pub table: TableRef,
    pub columns: Vec<String>,
    pub source: InsertSource,
    /// ON CONFLICT / ON DUPLICATE KEY
    pub on_conflict: Option<OnConflict>,
    /// RETURNING clause
    pub returning: Vec<SelectItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InsertSource {
    Values(Vec<Vec<Expr>>),
    Query(Box<Statement>),
    Default,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnConflict {
    pub columns: Vec<String>,
    pub action: ConflictAction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConflictAction {
    DoNothing,
    DoUpdate(Vec<(String, Expr)>),
}

/// An UPDATE statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateStatement {
    pub table: TableRef,
    pub assignments: Vec<(String, Expr)>,
    pub from: Option<FromClause>,
    pub where_clause: Option<Expr>,
    pub returning: Vec<SelectItem>,
}

/// A DELETE statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeleteStatement {
    pub table: TableRef,
    pub using: Option<FromClause>,
    pub where_clause: Option<Expr>,
    pub returning: Vec<SelectItem>,
}

// ═══════════════════════════════════════════════════════════════════════
// DDL statements
// ═══════════════════════════════════════════════════════════════════════

/// A CREATE TABLE statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTableStatement {
    pub if_not_exists: bool,
    pub temporary: bool,
    pub table: TableRef,
    pub columns: Vec<ColumnDef>,
    pub constraints: Vec<TableConstraint>,
    /// CREATE TABLE ... AS SELECT ...
    pub as_select: Option<Box<Statement>>,
}

/// Table-level constraints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableConstraint {
    PrimaryKey {
        name: Option<String>,
        columns: Vec<String>,
    },
    Unique {
        name: Option<String>,
        columns: Vec<String>,
    },
    ForeignKey {
        name: Option<String>,
        columns: Vec<String>,
        ref_table: TableRef,
        ref_columns: Vec<String>,
        on_delete: Option<ReferentialAction>,
        on_update: Option<ReferentialAction>,
    },
    Check {
        name: Option<String>,
        expr: Expr,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferentialAction {
    Cascade,
    Restrict,
    NoAction,
    SetNull,
    SetDefault,
}

/// A column definition in CREATE TABLE.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub nullable: Option<bool>,
    pub default: Option<Expr>,
    pub primary_key: bool,
    pub unique: bool,
    pub auto_increment: bool,
    pub collation: Option<String>,
    pub comment: Option<String>,
}

/// ALTER TABLE statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterTableStatement {
    pub table: TableRef,
    pub actions: Vec<AlterTableAction>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlterTableAction {
    AddColumn(ColumnDef),
    DropColumn { name: String, if_exists: bool },
    RenameColumn { old_name: String, new_name: String },
    AlterColumnType { name: String, data_type: DataType },
    AddConstraint(TableConstraint),
    DropConstraint { name: String },
    RenameTable { new_name: String },
}

/// CREATE VIEW statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateViewStatement {
    pub name: TableRef,
    pub columns: Vec<String>,
    pub query: Box<Statement>,
    pub or_replace: bool,
    pub materialized: bool,
    pub if_not_exists: bool,
}

/// DROP VIEW statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropViewStatement {
    pub name: TableRef,
    pub if_exists: bool,
    pub materialized: bool,
}

/// TRUNCATE TABLE statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TruncateStatement {
    pub table: TableRef,
}

/// Transaction control statements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransactionStatement {
    Begin,
    Commit,
    Rollback,
    Savepoint(String),
    ReleaseSavepoint(String),
    RollbackTo(String),
}

/// EXPLAIN statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplainStatement {
    pub analyze: bool,
    pub statement: Box<Statement>,
}

/// USE database statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UseStatement {
    pub name: String,
}

/// A DROP TABLE statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropTableStatement {
    pub if_exists: bool,
    pub table: TableRef,
    pub cascade: bool,
}

// ═══════════════════════════════════════════════════════════════════════
// Data types
// ═══════════════════════════════════════════════════════════════════════

/// SQL data types. Significantly expanded to match sqlglot's DataType.Type enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    // Numeric
    TinyInt,
    SmallInt,
    Int,
    BigInt,
    Float,
    Double,
    Decimal { precision: Option<u32>, scale: Option<u32> },
    Numeric { precision: Option<u32>, scale: Option<u32> },
    Real,

    // String
    Varchar(Option<u32>),
    Char(Option<u32>),
    Text,
    String,
    Binary(Option<u32>),
    Varbinary(Option<u32>),

    // Boolean
    Boolean,

    // Date/Time
    Date,
    Time { precision: Option<u32> },
    Timestamp { precision: Option<u32>, with_tz: bool },
    Interval,
    DateTime,

    // Binary
    Blob,
    Bytea,
    Bytes,

    // JSON
    Json,
    Jsonb,

    // UUID
    Uuid,

    // Complex types
    Array(Option<Box<DataType>>),
    Map { key: Box<DataType>, value: Box<DataType> },
    Struct(Vec<(String, DataType)>),
    Tuple(Vec<DataType>),

    // Special
    Null,
    Unknown(String),
    Variant,
    Object,
    Xml,
    Inet,
    Cidr,
    Macaddr,
    Bit(Option<u32>),
    Money,
    Serial,
    BigSerial,
    SmallSerial,
    Regclass,
    Regtype,
    Hstore,
    Geography,
    Geometry,
    Super,
}

// ═══════════════════════════════════════════════════════════════════════
// Expression tree traversal helpers
// ═══════════════════════════════════════════════════════════════════════

impl Expr {
    /// Recursively walk this expression tree, calling `visitor` on each node.
    /// If `visitor` returns `false`, children of that node are not visited.
    pub fn walk<F>(&self, visitor: &mut F)
    where
        F: FnMut(&Expr) -> bool,
    {
        if !visitor(self) {
            return;
        }
        match self {
            Expr::BinaryOp { left, right, .. } => {
                left.walk(visitor);
                right.walk(visitor);
            }
            Expr::UnaryOp { expr, .. } => expr.walk(visitor),
            Expr::Function { args, filter, .. } => {
                for arg in args {
                    arg.walk(visitor);
                }
                if let Some(f) = filter {
                    f.walk(visitor);
                }
            }
            Expr::Between { expr, low, high, .. } => {
                expr.walk(visitor);
                low.walk(visitor);
                high.walk(visitor);
            }
            Expr::InList { expr, list, .. } => {
                expr.walk(visitor);
                for item in list {
                    item.walk(visitor);
                }
            }
            Expr::InSubquery { expr, .. } => {
                expr.walk(visitor);
            }
            Expr::IsNull { expr, .. } => expr.walk(visitor),
            Expr::Like { expr, pattern, .. } | Expr::ILike { expr, pattern, .. } => {
                expr.walk(visitor);
                pattern.walk(visitor);
            }
            Expr::Case {
                operand,
                when_clauses,
                else_clause,
            } => {
                if let Some(op) = operand {
                    op.walk(visitor);
                }
                for (cond, result) in when_clauses {
                    cond.walk(visitor);
                    result.walk(visitor);
                }
                if let Some(el) = else_clause {
                    el.walk(visitor);
                }
            }
            Expr::Nested(inner) => inner.walk(visitor),
            Expr::Cast { expr, .. } | Expr::TryCast { expr, .. } => expr.walk(visitor),
            Expr::Extract { expr, .. } => expr.walk(visitor),
            Expr::Interval { value, .. } => value.walk(visitor),
            Expr::ArrayLiteral(items) | Expr::Tuple(items) | Expr::Coalesce(items) => {
                for item in items {
                    item.walk(visitor);
                }
            }
            Expr::If { condition, true_val, false_val } => {
                condition.walk(visitor);
                true_val.walk(visitor);
                if let Some(fv) = false_val {
                    fv.walk(visitor);
                }
            }
            Expr::NullIf { expr, r#else } => {
                expr.walk(visitor);
                r#else.walk(visitor);
            }
            Expr::Collate { expr, .. } => expr.walk(visitor),
            Expr::Alias { expr, .. } => expr.walk(visitor),
            Expr::ArrayIndex { expr, index } => {
                expr.walk(visitor);
                index.walk(visitor);
            }
            Expr::JsonAccess { expr, path, .. } => {
                expr.walk(visitor);
                path.walk(visitor);
            }
            Expr::Lambda { body, .. } => body.walk(visitor),
            // Leaf nodes
            Expr::Column { .. }
            | Expr::Number(_)
            | Expr::StringLiteral(_)
            | Expr::Boolean(_)
            | Expr::Null
            | Expr::Wildcard
            | Expr::Star
            | Expr::Parameter(_)
            | Expr::TypeExpr(_)
            | Expr::QualifiedWildcard { .. }
            | Expr::Default
            | Expr::Subquery(_)
            | Expr::Exists { .. } => {}
        }
    }

    /// Find the first expression matching the predicate.
    #[must_use]
    pub fn find<F>(&self, predicate: &F) -> Option<&Expr>
    where
        F: Fn(&Expr) -> bool,
    {
        let mut result = None;
        self.walk(&mut |expr| {
            if result.is_some() {
                return false;
            }
            if predicate(expr) {
                result = Some(expr as *const Expr);
                false
            } else {
                true
            }
        });
        // SAFETY: the pointer is valid as long as self is alive
        result.map(|p| unsafe { &*p })
    }

    /// Find all expressions matching the predicate.
    #[must_use]
    pub fn find_all<F>(&self, predicate: &F) -> Vec<&Expr>
    where
        F: Fn(&Expr) -> bool,
    {
        let mut results: Vec<*const Expr> = Vec::new();
        self.walk(&mut |expr| {
            if predicate(expr) {
                results.push(expr as *const Expr);
            }
            true
        });
        results.into_iter().map(|p| unsafe { &*p }).collect()
    }

    /// Transform this expression tree by applying a function to each node.
    /// The function can return a new expression to replace the current one.
    #[must_use]
    pub fn transform<F>(self, func: &F) -> Expr
    where
        F: Fn(Expr) -> Expr,
    {
        let transformed = match self {
            Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
                left: Box::new(left.transform(func)),
                op,
                right: Box::new(right.transform(func)),
            },
            Expr::UnaryOp { op, expr } => Expr::UnaryOp {
                op,
                expr: Box::new(expr.transform(func)),
            },
            Expr::Function { name, args, distinct, filter, over } => Expr::Function {
                name,
                args: args.into_iter().map(|a| a.transform(func)).collect(),
                distinct,
                filter: filter.map(|f| Box::new(f.transform(func))),
                over,
            },
            Expr::Nested(inner) => Expr::Nested(Box::new(inner.transform(func))),
            Expr::Cast { expr, data_type } => Expr::Cast {
                expr: Box::new(expr.transform(func)),
                data_type,
            },
            Expr::Between { expr, low, high, negated } => Expr::Between {
                expr: Box::new(expr.transform(func)),
                low: Box::new(low.transform(func)),
                high: Box::new(high.transform(func)),
                negated,
            },
            Expr::Case { operand, when_clauses, else_clause } => Expr::Case {
                operand: operand.map(|o| Box::new(o.transform(func))),
                when_clauses: when_clauses
                    .into_iter()
                    .map(|(c, r)| (c.transform(func), r.transform(func)))
                    .collect(),
                else_clause: else_clause.map(|e| Box::new(e.transform(func))),
            },
            other => other,
        };
        func(transformed)
    }

    /// Check whether this expression is a column reference.
    #[must_use]
    pub fn is_column(&self) -> bool {
        matches!(self, Expr::Column { .. })
    }

    /// Check whether this expression is a literal value (number, string, bool, null).
    #[must_use]
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            Expr::Number(_) | Expr::StringLiteral(_) | Expr::Boolean(_) | Expr::Null
        )
    }

    /// Get the SQL representation of this expression for display purposes.
    /// For full generation, use the Generator.
    #[must_use]
    pub fn sql(&self) -> String {
        use crate::generator::Generator;
        Generator::expr_to_sql(self)
    }
}

/// Helper: collect all column references from an expression.
#[must_use]
pub fn find_columns(expr: &Expr) -> Vec<&Expr> {
    expr.find_all(&|e| matches!(e, Expr::Column { .. }))
}

/// Helper: collect all table references from a statement.
#[must_use]
pub fn find_tables(statement: &Statement) -> Vec<&TableRef> {
    match statement {
        Statement::Select(sel) => {
            let mut tables = Vec::new();
            if let Some(from) = &sel.from {
                collect_table_refs_from_source(&from.source, &mut tables);
            }
            for join in &sel.joins {
                collect_table_refs_from_source(&join.table, &mut tables);
            }
            tables
        }
        Statement::Insert(ins) => vec![&ins.table],
        Statement::Update(upd) => vec![&upd.table],
        Statement::Delete(del) => vec![&del.table],
        Statement::CreateTable(ct) => vec![&ct.table],
        Statement::DropTable(dt) => vec![&dt.table],
        _ => vec![],
    }
}

fn collect_table_refs_from_source<'a>(source: &'a TableSource, tables: &mut Vec<&'a TableRef>) {
    match source {
        TableSource::Table(table_ref) => tables.push(table_ref),
        TableSource::Subquery { .. } => {}
        TableSource::TableFunction { .. } => {}
        TableSource::Lateral { source } => collect_table_refs_from_source(source, tables),
        TableSource::Unnest { .. } => {}
    }
}
