use serde::{Deserialize, Serialize};

use crate::ast::*;

pub mod plugin;
pub mod time;

/// Supported SQL dialects.
///
/// Mirrors the full set of dialects supported by Python's sqlglot library.
/// Dialects are grouped into **Official** (core, higher-priority maintenance)
/// and **Community** (contributed, fully functional) tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Dialect {
    // ── Core / base ──────────────────────────────────────────────────────
    /// ANSI SQL standard (default / base dialect)
    Ansi,

    // ── Official dialects ────────────────────────────────────────────────
    /// AWS Athena (Presto-based)
    Athena,
    /// Google BigQuery
    BigQuery,
    /// ClickHouse
    ClickHouse,
    /// Databricks (Spark-based)
    Databricks,
    /// DuckDB
    DuckDb,
    /// Apache Hive
    Hive,
    /// MySQL
    Mysql,
    /// Oracle Database
    Oracle,
    /// PostgreSQL
    Postgres,
    /// Presto
    Presto,
    /// Amazon Redshift (Postgres-based)
    Redshift,
    /// Snowflake
    Snowflake,
    /// Apache Spark SQL
    Spark,
    /// SQLite
    Sqlite,
    /// StarRocks (MySQL-compatible)
    StarRocks,
    /// Trino (Presto successor)
    Trino,
    /// Microsoft SQL Server (T-SQL)
    Tsql,

    // ── Community dialects ───────────────────────────────────────────────
    /// Apache Doris (MySQL-compatible)
    Doris,
    /// Dremio
    Dremio,
    /// Apache Drill
    Drill,
    /// Apache Druid
    Druid,
    /// Exasol
    Exasol,
    /// Microsoft Fabric (T-SQL variant)
    Fabric,
    /// Materialize (Postgres-compatible)
    Materialize,
    /// PRQL (Pipelined Relational Query Language)
    Prql,
    /// RisingWave (Postgres-compatible)
    RisingWave,
    /// SingleStore (MySQL-compatible)
    SingleStore,
    /// Tableau
    Tableau,
    /// Teradata
    Teradata,
}

impl std::fmt::Display for Dialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dialect::Ansi => write!(f, "ANSI SQL"),
            Dialect::Athena => write!(f, "Athena"),
            Dialect::BigQuery => write!(f, "BigQuery"),
            Dialect::ClickHouse => write!(f, "ClickHouse"),
            Dialect::Databricks => write!(f, "Databricks"),
            Dialect::DuckDb => write!(f, "DuckDB"),
            Dialect::Hive => write!(f, "Hive"),
            Dialect::Mysql => write!(f, "MySQL"),
            Dialect::Oracle => write!(f, "Oracle"),
            Dialect::Postgres => write!(f, "PostgreSQL"),
            Dialect::Presto => write!(f, "Presto"),
            Dialect::Redshift => write!(f, "Redshift"),
            Dialect::Snowflake => write!(f, "Snowflake"),
            Dialect::Spark => write!(f, "Spark"),
            Dialect::Sqlite => write!(f, "SQLite"),
            Dialect::StarRocks => write!(f, "StarRocks"),
            Dialect::Trino => write!(f, "Trino"),
            Dialect::Tsql => write!(f, "T-SQL"),
            Dialect::Doris => write!(f, "Doris"),
            Dialect::Dremio => write!(f, "Dremio"),
            Dialect::Drill => write!(f, "Drill"),
            Dialect::Druid => write!(f, "Druid"),
            Dialect::Exasol => write!(f, "Exasol"),
            Dialect::Fabric => write!(f, "Fabric"),
            Dialect::Materialize => write!(f, "Materialize"),
            Dialect::Prql => write!(f, "PRQL"),
            Dialect::RisingWave => write!(f, "RisingWave"),
            Dialect::SingleStore => write!(f, "SingleStore"),
            Dialect::Tableau => write!(f, "Tableau"),
            Dialect::Teradata => write!(f, "Teradata"),
        }
    }
}

impl Dialect {
    /// Returns the support tier for this dialect.
    #[must_use]
    pub fn support_level(&self) -> &'static str {
        match self {
            Dialect::Ansi
            | Dialect::Athena
            | Dialect::BigQuery
            | Dialect::ClickHouse
            | Dialect::Databricks
            | Dialect::DuckDb
            | Dialect::Hive
            | Dialect::Mysql
            | Dialect::Oracle
            | Dialect::Postgres
            | Dialect::Presto
            | Dialect::Redshift
            | Dialect::Snowflake
            | Dialect::Spark
            | Dialect::Sqlite
            | Dialect::StarRocks
            | Dialect::Trino
            | Dialect::Tsql => "Official",

            Dialect::Doris
            | Dialect::Dremio
            | Dialect::Drill
            | Dialect::Druid
            | Dialect::Exasol
            | Dialect::Fabric
            | Dialect::Materialize
            | Dialect::Prql
            | Dialect::RisingWave
            | Dialect::SingleStore
            | Dialect::Tableau
            | Dialect::Teradata => "Community",
        }
    }

    /// Returns all dialect variants.
    #[must_use]
    pub fn all() -> &'static [Dialect] {
        &[
            Dialect::Ansi,
            Dialect::Athena,
            Dialect::BigQuery,
            Dialect::ClickHouse,
            Dialect::Databricks,
            Dialect::Doris,
            Dialect::Dremio,
            Dialect::Drill,
            Dialect::Druid,
            Dialect::DuckDb,
            Dialect::Exasol,
            Dialect::Fabric,
            Dialect::Hive,
            Dialect::Materialize,
            Dialect::Mysql,
            Dialect::Oracle,
            Dialect::Postgres,
            Dialect::Presto,
            Dialect::Prql,
            Dialect::Redshift,
            Dialect::RisingWave,
            Dialect::SingleStore,
            Dialect::Snowflake,
            Dialect::Spark,
            Dialect::Sqlite,
            Dialect::StarRocks,
            Dialect::Tableau,
            Dialect::Teradata,
            Dialect::Trino,
            Dialect::Tsql,
        ]
    }

    /// Parse a dialect name (case-insensitive) into a `Dialect`.
    pub fn from_str(s: &str) -> Option<Dialect> {
        match s.to_lowercase().as_str() {
            "" | "ansi" => Some(Dialect::Ansi),
            "athena" => Some(Dialect::Athena),
            "bigquery" => Some(Dialect::BigQuery),
            "clickhouse" => Some(Dialect::ClickHouse),
            "databricks" => Some(Dialect::Databricks),
            "doris" => Some(Dialect::Doris),
            "dremio" => Some(Dialect::Dremio),
            "drill" => Some(Dialect::Drill),
            "druid" => Some(Dialect::Druid),
            "duckdb" => Some(Dialect::DuckDb),
            "exasol" => Some(Dialect::Exasol),
            "fabric" => Some(Dialect::Fabric),
            "hive" => Some(Dialect::Hive),
            "materialize" => Some(Dialect::Materialize),
            "mysql" => Some(Dialect::Mysql),
            "oracle" => Some(Dialect::Oracle),
            "postgres" | "postgresql" => Some(Dialect::Postgres),
            "presto" => Some(Dialect::Presto),
            "prql" => Some(Dialect::Prql),
            "redshift" => Some(Dialect::Redshift),
            "risingwave" => Some(Dialect::RisingWave),
            "singlestore" => Some(Dialect::SingleStore),
            "snowflake" => Some(Dialect::Snowflake),
            "spark" => Some(Dialect::Spark),
            "sqlite" => Some(Dialect::Sqlite),
            "starrocks" => Some(Dialect::StarRocks),
            "tableau" => Some(Dialect::Tableau),
            "teradata" => Some(Dialect::Teradata),
            "trino" => Some(Dialect::Trino),
            "tsql" | "mssql" | "sqlserver" => Some(Dialect::Tsql),
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Dialect families — helpers for grouping similar dialects
// ═══════════════════════════════════════════════════════════════════════════

/// Dialects in the MySQL family (use SUBSTR, IFNULL, similar type system).
fn is_mysql_family(d: Dialect) -> bool {
    matches!(
        d,
        Dialect::Mysql | Dialect::Doris | Dialect::SingleStore | Dialect::StarRocks
    )
}

/// Dialects in the Postgres family (support ILIKE, BYTEA, SUBSTRING).
fn is_postgres_family(d: Dialect) -> bool {
    matches!(
        d,
        Dialect::Postgres | Dialect::Redshift | Dialect::Materialize | Dialect::RisingWave
    )
}

/// Dialects in the Presto family (ANSI-like, VARCHAR oriented).
fn is_presto_family(d: Dialect) -> bool {
    matches!(d, Dialect::Presto | Dialect::Trino | Dialect::Athena)
}

/// Dialects in the Hive/Spark family (use STRING type, SUBSTR).
fn is_hive_family(d: Dialect) -> bool {
    matches!(d, Dialect::Hive | Dialect::Spark | Dialect::Databricks)
}

/// Dialects in the T-SQL family.
pub(crate) fn is_tsql_family(d: Dialect) -> bool {
    matches!(d, Dialect::Tsql | Dialect::Fabric)
}

/// Returns `true` when `name` (case-insensitive) is a T-SQL reserved keyword
/// that must be quoted with square brackets when used as an identifier
/// (e.g. as a column alias, table alias, or column name).
///
/// Sourced from Microsoft's documented T-SQL reserved words (current and
/// future). Covers both the ANSI/ODBC reserved set and SQL Server's
/// dialect-specific reservations. Not exhaustive for every contextual
/// keyword — focuses on words that, when emitted unquoted as aliases, will
/// cause MSSQL syntax error 156.
#[must_use]
pub(crate) fn is_tsql_reserved(name: &str) -> bool {
    // Reserved word set — keep in sorted order for binary_search.
    // Source: docs.microsoft.com "Reserved Keywords (Transact-SQL)" and
    // "ODBC Reserved Keywords", plus ANSI/ISO future reserved words.
    const RESERVED: &[&str] = &[
        "ABSOLUTE",
        "ACTION",
        "ADA",
        "ADD",
        "ALL",
        "ALLOCATE",
        "ALTER",
        "AND",
        "ANY",
        "ARE",
        "AS",
        "ASC",
        "ASSERTION",
        "AT",
        "AUTHORIZATION",
        "AVG",
        "BACKUP",
        "BEGIN",
        "BETWEEN",
        "BIT",
        "BIT_LENGTH",
        "BOTH",
        "BREAK",
        "BROWSE",
        "BULK",
        "BY",
        "CASCADE",
        "CASCADED",
        "CASE",
        "CAST",
        "CATALOG",
        "CHAR",
        "CHARACTER",
        "CHARACTER_LENGTH",
        "CHAR_LENGTH",
        "CHECK",
        "CHECKPOINT",
        "CLOSE",
        "CLUSTERED",
        "COALESCE",
        "COLLATE",
        "COLLATION",
        "COLUMN",
        "COMMIT",
        "COMPUTE",
        "CONNECT",
        "CONNECTION",
        "CONSTRAINT",
        "CONSTRAINTS",
        "CONTAINS",
        "CONTAINSTABLE",
        "CONTINUE",
        "CONVERT",
        "CORRESPONDING",
        "COUNT",
        "CREATE",
        "CROSS",
        "CURRENT",
        "CURRENT_DATE",
        "CURRENT_TIME",
        "CURRENT_TIMESTAMP",
        "CURRENT_USER",
        "CURSOR",
        "DATABASE",
        "DATE",
        "DBCC",
        "DEALLOCATE",
        "DEC",
        "DECIMAL",
        "DECLARE",
        "DEFAULT",
        "DEFERRABLE",
        "DEFERRED",
        "DELETE",
        "DENY",
        "DESC",
        "DESCRIBE",
        "DESCRIPTOR",
        "DIAGNOSTICS",
        "DISCONNECT",
        "DISK",
        "DISTINCT",
        "DISTRIBUTED",
        "DOMAIN",
        "DOUBLE",
        "DROP",
        "DUMP",
        "ELSE",
        "END",
        "ERRLVL",
        "ESCAPE",
        "EXCEPT",
        "EXCEPTION",
        "EXEC",
        "EXECUTE",
        "EXISTS",
        "EXIT",
        "EXTERNAL",
        "EXTRACT",
        "FETCH",
        "FILE",
        "FILLFACTOR",
        "FLOAT",
        "FOR",
        "FOREIGN",
        "FORTRAN",
        "FOUND",
        "FREETEXT",
        "FREETEXTTABLE",
        "FROM",
        "FULL",
        "FUNCTION",
        "GET",
        "GLOBAL",
        "GO",
        "GOTO",
        "GRANT",
        "GROUP",
        "HAVING",
        "HOLDLOCK",
        "HOUR",
        "IDENTITY",
        "IDENTITYCOL",
        "IDENTITY_INSERT",
        "IF",
        "IMMEDIATE",
        "IN",
        "INCLUDE",
        "INDEX",
        "INDICATOR",
        "INITIALLY",
        "INNER",
        "INPUT",
        "INSENSITIVE",
        "INSERT",
        "INT",
        "INTEGER",
        "INTERSECT",
        "INTERVAL",
        "INTO",
        "IS",
        "ISOLATION",
        "JOIN",
        "KEY",
        "KILL",
        "LANGUAGE",
        "LAST",
        "LEADING",
        "LEFT",
        "LEVEL",
        "LIKE",
        "LINENO",
        "LOAD",
        "LOCAL",
        "LOWER",
        "MATCH",
        "MAX",
        "MERGE",
        "MIN",
        "MINUTE",
        "MODULE",
        "MONTH",
        "NAMES",
        "NATIONAL",
        "NATURAL",
        "NCHAR",
        "NEXT",
        "NO",
        "NOCHECK",
        "NONCLUSTERED",
        "NONE",
        "NOT",
        "NULL",
        "NULLIF",
        "NUMERIC",
        "OCTET_LENGTH",
        "OF",
        "OFF",
        "OFFSETS",
        "ON",
        "ONLY",
        "OPEN",
        "OPENDATASOURCE",
        "OPENQUERY",
        "OPENROWSET",
        "OPENXML",
        "OPTION",
        "OR",
        "ORDER",
        "OUTER",
        "OUTPUT",
        "OVER",
        "OVERLAPS",
        "PAD",
        "PARTIAL",
        "PASCAL",
        "PERCENT",
        "PIVOT",
        "PLAN",
        "POSITION",
        "PRECISION",
        "PREPARE",
        "PRESERVE",
        "PRIMARY",
        "PRINT",
        "PRIOR",
        "PRIVILEGES",
        "PROC",
        "PROCEDURE",
        "PUBLIC",
        "RAISERROR",
        "READ",
        "READTEXT",
        "REAL",
        "RECONFIGURE",
        "REFERENCES",
        "RELATIVE",
        "REPLICATION",
        "RESTORE",
        "RESTRICT",
        "RETURN",
        "REVERT",
        "REVOKE",
        "RIGHT",
        "ROLLBACK",
        "ROWCOUNT",
        "ROWGUIDCOL",
        "ROWS",
        "RULE",
        "SAVE",
        "SCHEMA",
        "SCROLL",
        "SECOND",
        "SECTION",
        "SECURITYAUDIT",
        "SELECT",
        "SEMANTICKEYPHRASETABLE",
        "SEMANTICSIMILARITYDETAILSTABLE",
        "SEMANTICSIMILARITYTABLE",
        "SESSION",
        "SESSION_USER",
        "SET",
        "SETUSER",
        "SHUTDOWN",
        "SIZE",
        "SMALLINT",
        "SOME",
        "SPACE",
        "SQL",
        "SQLCA",
        "SQLCODE",
        "SQLERROR",
        "SQLSTATE",
        "SQLWARNING",
        "STATISTICS",
        "SUBSTRING",
        "SUM",
        "SYSTEM_USER",
        "TABLE",
        "TABLESAMPLE",
        "TEMPORARY",
        "TEXTSIZE",
        "THEN",
        "TIME",
        "TIMESTAMP",
        "TIMEZONE_HOUR",
        "TIMEZONE_MINUTE",
        "TO",
        "TOP",
        "TRAILING",
        "TRAN",
        "TRANSACTION",
        "TRANSLATE",
        "TRANSLATION",
        "TRIGGER",
        "TRIM",
        "TRUE",
        "TRUNCATE",
        "TRY_CONVERT",
        "TSEQUAL",
        "UNION",
        "UNIQUE",
        "UNKNOWN",
        "UNPIVOT",
        "UPDATE",
        "UPDATETEXT",
        "UPPER",
        "USAGE",
        "USE",
        "USER",
        "USING",
        "VALUE",
        "VALUES",
        "VARCHAR",
        "VARYING",
        "VIEW",
        "WAITFOR",
        "WHEN",
        "WHENEVER",
        "WHERE",
        "WHILE",
        "WITH",
        "WITHIN GROUP",
        "WORK",
        "WRITE",
        "WRITETEXT",
        "YEAR",
        "ZONE",
    ];

    // Cheap upper-case comparison without allocation for ASCII identifiers.
    if name.is_empty() || name.len() > 32 {
        return false;
    }
    let mut buf = [0u8; 32];
    for (i, b) in name.as_bytes().iter().enumerate() {
        buf[i] = b.to_ascii_uppercase();
    }
    let upper = match std::str::from_utf8(&buf[..name.len()]) {
        Ok(s) => s,
        Err(_) => return false,
    };
    RESERVED.binary_search(&upper).is_ok()
}

/// Dialects that natively support ILIKE.
pub(crate) fn supports_ilike_builtin(d: Dialect) -> bool {
    matches!(
        d,
        Dialect::Postgres
            | Dialect::Redshift
            | Dialect::Materialize
            | Dialect::RisingWave
            | Dialect::DuckDb
            | Dialect::Snowflake
            | Dialect::ClickHouse
            | Dialect::Trino
            | Dialect::Presto
            | Dialect::Athena
            | Dialect::Databricks
            | Dialect::Spark
            | Dialect::Hive
            | Dialect::StarRocks
            | Dialect::Exasol
            | Dialect::Druid
            | Dialect::Dremio
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// Statement / expression transforms
// ═══════════════════════════════════════════════════════════════════════════

/// Transform a statement from one dialect to another.
///
/// This applies dialect-specific rewrite rules such as:
/// - Type mapping (e.g., `TEXT` → `STRING` for BigQuery)
/// - Function name mapping (e.g., `NOW()` → `CURRENT_TIMESTAMP()`)
/// - ILIKE → LIKE with LOWER() wrapping for dialects that don't support ILIKE
#[must_use]
pub fn transform(statement: &Statement, from: Dialect, to: Dialect) -> Statement {
    if from == to {
        return statement.clone();
    }
    let mut stmt = statement.clone();
    transform_statement(&mut stmt, to);
    stmt
}

fn transform_statement(statement: &mut Statement, target: Dialect) {
    match statement {
        Statement::Select(sel) => {
            // Transform LIMIT / TOP / FETCH FIRST for the target dialect
            transform_limit(sel, target);
            // Transform identifier quoting for the target dialect
            transform_quotes_in_select(sel, target);

            for item in &mut sel.columns {
                if let SelectItem::Expr { expr, .. } = item {
                    *expr = transform_expr(expr.clone(), target);
                }
            }
            if let Some(wh) = &mut sel.where_clause {
                *wh = transform_expr(wh.clone(), target);
            }
            for gb in &mut sel.group_by {
                *gb = transform_expr(gb.clone(), target);
            }
            if let Some(having) = &mut sel.having {
                *having = transform_expr(having.clone(), target);
            }
        }
        Statement::Insert(ins) => {
            if let InsertSource::Values(rows) = &mut ins.source {
                for row in rows {
                    for val in row {
                        *val = transform_expr(val.clone(), target);
                    }
                }
            }
            // Transform RETURNING expressions
            for item in &mut ins.returning {
                if let SelectItem::Expr { expr, .. } = item {
                    *expr = transform_expr(expr.clone(), target);
                }
            }
        }
        Statement::Update(upd) => {
            for (_, val) in &mut upd.assignments {
                *val = transform_expr(val.clone(), target);
            }
            if let Some(wh) = &mut upd.where_clause {
                *wh = transform_expr(wh.clone(), target);
            }
            // Transform RETURNING expressions
            for item in &mut upd.returning {
                if let SelectItem::Expr { expr, .. } = item {
                    *expr = transform_expr(expr.clone(), target);
                }
            }
        }
        Statement::Delete(del) => {
            if let Some(wh) = &mut del.where_clause {
                *wh = transform_expr(wh.clone(), target);
            }
            // Transform RETURNING expressions
            for item in &mut del.returning {
                if let SelectItem::Expr { expr, .. } = item {
                    *expr = transform_expr(expr.clone(), target);
                }
            }
        }
        // DDL: map data types in CREATE TABLE column definitions
        Statement::CreateTable(ct) => {
            for col in &mut ct.columns {
                col.data_type = map_data_type(col.data_type.clone(), target);
                if let Some(default) = &mut col.default {
                    *default = transform_expr(default.clone(), target);
                }
            }
            // Transform constraints (CHECK expressions)
            for constraint in &mut ct.constraints {
                if let TableConstraint::Check { expr, .. } = constraint {
                    *expr = transform_expr(expr.clone(), target);
                }
            }
            // Transform AS SELECT subquery
            if let Some(as_select) = &mut ct.as_select {
                transform_statement(as_select, target);
            }
        }
        // DDL: map data types in ALTER TABLE ADD COLUMN
        Statement::AlterTable(alt) => {
            for action in &mut alt.actions {
                match action {
                    AlterTableAction::AddColumn(col) => {
                        col.data_type = map_data_type(col.data_type.clone(), target);
                        if let Some(default) = &mut col.default {
                            *default = transform_expr(default.clone(), target);
                        }
                    }
                    AlterTableAction::AlterColumnType { data_type, .. } => {
                        *data_type = map_data_type(data_type.clone(), target);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// Transform an expression for the target dialect.
fn transform_expr(expr: Expr, target: Dialect) -> Expr {
    match expr {
        // Map function names across dialects
        Expr::Function {
            name,
            args,
            distinct,
            filter,
            over,
        } => {
            let new_name = map_function_name(&name, target);
            let new_args: Vec<Expr> = args
                .into_iter()
                .map(|a| transform_expr(a, target))
                .collect();
            Expr::Function {
                name: new_name,
                args: new_args,
                distinct,
                filter: filter.map(|f| Box::new(transform_expr(*f, target))),
                over,
            }
        }
        // Recurse into typed function child expressions, with special handling
        // for date/time formatting functions that need format string conversion
        Expr::TypedFunction { func, filter, over } => {
            let transformed_func = transform_typed_function(func, target);
            Expr::TypedFunction {
                func: transformed_func,
                filter: filter.map(|f| Box::new(transform_expr(*f, target))),
                over,
            }
        }
        // ILIKE → LOWER(expr) LIKE LOWER(pattern) for non-supporting dialects
        Expr::ILike {
            expr,
            pattern,
            negated,
            escape,
        } if !supports_ilike_builtin(target) => Expr::Like {
            expr: Box::new(Expr::TypedFunction {
                func: TypedFunction::Lower {
                    expr: Box::new(transform_expr(*expr, target)),
                },
                filter: None,
                over: None,
            }),
            pattern: Box::new(Expr::TypedFunction {
                func: TypedFunction::Lower {
                    expr: Box::new(transform_expr(*pattern, target)),
                },
                filter: None,
                over: None,
            }),
            negated,
            escape,
        },
        // SIMILAR TO → LIKE for T-SQL (lossy: regex features dropped)
        Expr::SimilarTo {
            expr,
            pattern,
            negated,
            escape,
        } if is_tsql_family(target) => {
            let transformed_pattern = transform_expr(*pattern, target);
            let simplified = simplify_similar_to_pattern(&transformed_pattern);
            Expr::Like {
                expr: Box::new(transform_expr(*expr, target)),
                pattern: Box::new(simplified),
                negated,
                escape,
            }
        }
        // Map data types in CAST
        Expr::Cast { expr, data_type } => Expr::Cast {
            expr: Box::new(transform_expr(*expr, target)),
            data_type: map_data_type(data_type, target),
        },
        // Recurse into binary ops, with T-SQL specific transforms
        Expr::BinaryOp { left, op, right } => {
            // Change 3: || → CONCAT() for T-SQL
            // Collect args BEFORE recursive transform to flatten the full chain
            if op == BinaryOperator::Concat && is_tsql_family(target) {
                let mut args = Vec::new();
                collect_concat_args(
                    &Expr::BinaryOp {
                        left,
                        op: BinaryOperator::Concat,
                        right,
                    },
                    &mut args,
                );
                // Now transform each collected arg
                let args = args
                    .into_iter()
                    .map(|a| transform_expr(a, target))
                    .collect();
                return Expr::Function {
                    name: "CONCAT".to_string(),
                    args,
                    distinct: false,
                    filter: None,
                    over: None,
                };
            }

            let left_transformed = transform_expr(*left, target);
            let right_transformed = transform_expr(*right, target);

            // Change 6: expr ± INTERVAL → DATEADD() for T-SQL
            if is_tsql_family(target) && matches!(op, BinaryOperator::Plus | BinaryOperator::Minus)
            {
                if let Some(dateadd) =
                    try_transform_interval_arithmetic(&left_transformed, &op, &right_transformed)
                {
                    return dateadd;
                }
            }

            Expr::BinaryOp {
                left: Box::new(left_transformed),
                op,
                right: Box::new(right_transformed),
            }
        }
        Expr::UnaryOp { op, expr } => Expr::UnaryOp {
            op,
            expr: Box::new(transform_expr(*expr, target)),
        },
        Expr::Nested(inner) => Expr::Nested(Box::new(transform_expr(*inner, target))),
        // Transform quoting on column references
        Expr::Column {
            table,
            name,
            quote_style,
            table_quote_style,
        } => {
            let new_qs = if quote_style.is_quoted() {
                QuoteStyle::for_dialect(target)
            } else {
                QuoteStyle::None
            };
            let new_tqs = if table_quote_style.is_quoted() {
                QuoteStyle::for_dialect(target)
            } else {
                QuoteStyle::None
            };
            Expr::Column {
                table,
                name,
                quote_style: new_qs,
                table_quote_style: new_tqs,
            }
        }
        // Everything else stays the same
        other => other,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Typed function transformation with format string conversion
// ═══════════════════════════════════════════════════════════════════════════

/// Transform a TypedFunction, including date/time format string conversion.
///
/// For TimeToStr and StrToTime functions, this converts the format string
/// from the source dialect's convention to the target dialect's convention.
fn transform_typed_function(func: TypedFunction, target: Dialect) -> TypedFunction {
    match func {
        TypedFunction::TimeToStr { expr, format } => {
            let transformed_expr = Box::new(transform_expr(*expr, target));
            let transformed_format = transform_format_expr(*format, target);
            TypedFunction::TimeToStr {
                expr: transformed_expr,
                format: Box::new(transformed_format),
            }
        }
        TypedFunction::StrToTime { expr, format } => {
            let transformed_expr = Box::new(transform_expr(*expr, target));
            let transformed_format = transform_format_expr(*format, target);
            TypedFunction::StrToTime {
                expr: transformed_expr,
                format: Box::new(transformed_format),
            }
        }
        // For all other typed functions, just transform child expressions
        other => other.transform_children(&|e| transform_expr(e, target)),
    }
}

/// Transform a format string expression for the target dialect.
///
/// If the expression is a string literal, convert the format specifiers.
/// Otherwise, just recursively transform child expressions.
fn transform_format_expr(expr: Expr, target: Dialect) -> Expr {
    // We need to know the source dialect to convert properly.
    // Since we don't have access to the source dialect here, we use heuristics
    // to detect the format style based on the format string content.
    match &expr {
        Expr::StringLiteral(s) | Expr::NationalStringLiteral(s) => {
            let detected_source = detect_format_style(s);
            let target_style = time::TimeFormatStyle::for_dialect(target);

            // Only convert if styles differ
            if detected_source != target_style {
                let converted = time::format_time(s, detected_source, target_style);
                match expr {
                    Expr::NationalStringLiteral(_) => Expr::NationalStringLiteral(converted),
                    _ => Expr::StringLiteral(converted),
                }
            } else {
                expr
            }
        }
        _ => transform_expr(expr, target),
    }
}

/// Detect the format style from a format string based on its content.
fn detect_format_style(format_str: &str) -> time::TimeFormatStyle {
    // Check for style-specific patterns
    if format_str.contains('%') {
        // strftime-style format
        if format_str.contains("%i") {
            // MySQL uses %i for minutes
            time::TimeFormatStyle::Mysql
        } else {
            // Generic strftime (SQLite, BigQuery, etc.)
            time::TimeFormatStyle::Strftime
        }
    } else if format_str.contains("YYYY") || format_str.contains("yyyy") {
        // Check for Java vs Postgres/Snowflake
        if format_str.contains("HH24") || format_str.contains("MI") || format_str.contains("SS") {
            // Postgres/Oracle style
            time::TimeFormatStyle::Postgres
        } else if format_str.contains("mm") && format_str.contains("ss") {
            // Java style (lowercase seconds and minutes)
            time::TimeFormatStyle::Java
        } else if format_str.contains("FF") {
            // Snowflake fractional seconds
            time::TimeFormatStyle::Snowflake
        } else if format_str.contains("MM") && format_str.contains("DD") {
            // Could be Postgres or Snowflake - default to Postgres
            time::TimeFormatStyle::Postgres
        } else {
            // Default to Java for ambiguous cases with lowercase patterns
            time::TimeFormatStyle::Java
        }
    } else {
        // Unknown format - default to strftime
        time::TimeFormatStyle::Strftime
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Function name mapping
// ═══════════════════════════════════════════════════════════════════════════

/// Map function names between dialects.
pub(crate) fn map_function_name(name: &str, target: Dialect) -> String {
    let upper = name.to_uppercase();
    match upper.as_str() {
        // ── NOW / CURRENT_TIMESTAMP / GETDATE ────────────────────────────
        "NOW" => {
            if is_tsql_family(target) {
                "GETDATE".to_string()
            } else if matches!(
                target,
                Dialect::Ansi
                    | Dialect::BigQuery
                    | Dialect::Snowflake
                    | Dialect::Oracle
                    | Dialect::ClickHouse
                    | Dialect::Exasol
                    | Dialect::Teradata
                    | Dialect::Druid
                    | Dialect::Dremio
                    | Dialect::Tableau
            ) || is_presto_family(target)
                || is_hive_family(target)
            {
                "CURRENT_TIMESTAMP".to_string()
            } else {
                // Postgres, MySQL, SQLite, DuckDB, Redshift, etc. – keep NOW
                name.to_string()
            }
        }
        "GETDATE" => {
            if is_tsql_family(target) {
                name.to_string()
            } else if is_postgres_family(target)
                || matches!(target, Dialect::Mysql | Dialect::DuckDb | Dialect::Sqlite)
            {
                "NOW".to_string()
            } else {
                "CURRENT_TIMESTAMP".to_string()
            }
        }

        // ── LEN / LENGTH ─────────────────────────────────────────────────
        "LEN" => {
            if is_tsql_family(target) || matches!(target, Dialect::BigQuery | Dialect::Snowflake) {
                name.to_string()
            } else {
                "LENGTH".to_string()
            }
        }
        "LENGTH" if is_tsql_family(target) => "LEN".to_string(),

        // ── SUBSTR / SUBSTRING ───────────────────────────────────────────
        "SUBSTR" => {
            if is_mysql_family(target)
                || matches!(target, Dialect::Sqlite | Dialect::Oracle)
                || is_hive_family(target)
            {
                "SUBSTR".to_string()
            } else {
                "SUBSTRING".to_string()
            }
        }
        "SUBSTRING" => {
            if is_mysql_family(target)
                || matches!(target, Dialect::Sqlite | Dialect::Oracle)
                || is_hive_family(target)
            {
                "SUBSTR".to_string()
            } else {
                name.to_string()
            }
        }

        // ── IFNULL / COALESCE / ISNULL ───────────────────────────────────
        "IFNULL" => {
            if is_tsql_family(target) {
                "ISNULL".to_string()
            } else if is_mysql_family(target) || matches!(target, Dialect::Sqlite) {
                // MySQL family + SQLite natively support IFNULL
                name.to_string()
            } else {
                "COALESCE".to_string()
            }
        }
        "ISNULL" => {
            if is_tsql_family(target) {
                name.to_string()
            } else if is_mysql_family(target) || matches!(target, Dialect::Sqlite) {
                "IFNULL".to_string()
            } else {
                "COALESCE".to_string()
            }
        }

        // ── NVL → COALESCE (Oracle to others) ───────────────────────────
        "NVL" => {
            if matches!(target, Dialect::Oracle | Dialect::Snowflake) {
                name.to_string()
            } else if is_mysql_family(target) || matches!(target, Dialect::Sqlite) {
                "IFNULL".to_string()
            } else if is_tsql_family(target) {
                "ISNULL".to_string()
            } else {
                "COALESCE".to_string()
            }
        }

        // ── RANDOM / RAND ────────────────────────────────────────────────
        "RANDOM" => {
            if matches!(
                target,
                Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb
            ) {
                name.to_string()
            } else {
                "RAND".to_string()
            }
        }
        "RAND" => {
            if matches!(
                target,
                Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb
            ) {
                "RANDOM".to_string()
            } else {
                name.to_string()
            }
        }

        // ── POSITION / CHARINDEX ─────────────────────────────────────────
        "POSITION" if is_tsql_family(target) => "CHARINDEX".to_string(),
        "CHARINDEX" if is_postgres_family(target) => "POSITION".to_string(),

        // Everything else – preserve original name
        _ => name.to_string(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Data-type mapping
// ═══════════════════════════════════════════════════════════════════════════

/// Map data types between dialects.
pub(crate) fn map_data_type(dt: DataType, target: Dialect) -> DataType {
    match (dt, target) {
        // ── T-SQL type mappings ─────────────────────────────────────────
        (DataType::Text, t) if is_tsql_family(t) => {
            DataType::Varchar(None) // NVARCHAR(MAX) emitted by generator via Unknown
        }
        (DataType::Boolean, t) if is_tsql_family(t) => DataType::Bit(None),
        (DataType::Bytea, t) if is_tsql_family(t) => DataType::Varbinary(None),
        (DataType::Json, t) if is_tsql_family(t) => DataType::Varchar(None),
        (DataType::Jsonb, t) if is_tsql_family(t) => DataType::Varchar(None),
        (DataType::Uuid, t) if is_tsql_family(t) => {
            DataType::Unknown("UNIQUEIDENTIFIER".to_string())
        }
        (DataType::Serial, t) if is_tsql_family(t) => DataType::Int,
        (DataType::BigSerial, t) if is_tsql_family(t) => DataType::BigInt,
        (DataType::SmallSerial, t) if is_tsql_family(t) => DataType::SmallInt,
        (DataType::Timestamp { .. }, t) if is_tsql_family(t) => {
            DataType::Unknown("DATETIME2".to_string())
        }
        (DataType::Real, t) if is_tsql_family(t) => DataType::Real,

        // ── TEXT / STRING ────────────────────────────────────────────────
        // TEXT → STRING for BigQuery, Hive, Spark, Databricks
        (DataType::Text, t) if matches!(t, Dialect::BigQuery) || is_hive_family(t) => {
            DataType::String
        }
        // STRING → TEXT for Postgres family, MySQL family, SQLite
        (DataType::String, t)
            if is_postgres_family(t) || is_mysql_family(t) || matches!(t, Dialect::Sqlite) =>
        {
            DataType::Text
        }

        // ── INT → BIGINT (BigQuery) ─────────────────────────────────────
        (DataType::Int, Dialect::BigQuery) => DataType::BigInt,

        // ── FLOAT → DOUBLE (BigQuery) ───────────────────────────────────
        (DataType::Float, Dialect::BigQuery) => DataType::Double,

        // ── BYTEA ↔ BLOB ────────────────────────────────────────────────
        (DataType::Bytea, t)
            if is_mysql_family(t)
                || matches!(t, Dialect::Sqlite | Dialect::Oracle)
                || is_hive_family(t) =>
        {
            DataType::Blob
        }
        (DataType::Blob, t) if is_postgres_family(t) => DataType::Bytea,
        (DataType::Varbinary(_), t) if is_postgres_family(t) => DataType::Bytea,

        // ── BOOLEAN → BOOL ──────────────────────────────────────────────
        (DataType::Boolean, Dialect::Mysql) => DataType::Boolean,

        // Everything else is unchanged
        (dt, _) => dt,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// LIMIT / TOP / FETCH FIRST transform
// ═══════════════════════════════════════════════════════════════════════════

/// Transform LIMIT / TOP / FETCH FIRST between dialects.
///
/// - T-SQL family:  `LIMIT n` → `TOP n` (OFFSET + FETCH handled separately)
/// - Oracle:        `LIMIT n` → `FETCH FIRST n ROWS ONLY`
/// - All others:    `TOP n` / `FETCH FIRST n` → `LIMIT n`
fn transform_limit(sel: &mut SelectStatement, target: Dialect) {
    if is_tsql_family(target) {
        // Move LIMIT → TOP for T-SQL (only when there's no OFFSET)
        if let Some(limit) = sel.limit.take() {
            if sel.offset.is_none() {
                sel.top = Some(Box::new(limit));
            } else {
                // T-SQL with OFFSET uses OFFSET n ROWS FETCH NEXT m ROWS ONLY
                sel.fetch_first = Some(limit);
                // T-SQL OFFSET/FETCH requires ORDER BY. Add ORDER BY (SELECT NULL) if absent.
                if sel.order_by.is_empty() {
                    sel.order_by = vec![OrderByItem {
                        expr: Expr::Subquery(Box::new(Statement::Select(SelectStatement {
                            comments: Vec::new(),
                            ctes: Vec::new(),
                            distinct: false,
                            top: None,
                            columns: vec![SelectItem::Expr {
                                expr: Expr::Null,
                                alias: None,
                                alias_quote_style: QuoteStyle::None,
                            }],
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
                        }))),
                        ascending: true,
                        nulls_first: None,
                    }];
                }
            }
        }
        // Also move fetch_first → top when no offset
        if sel.offset.is_none() {
            if let Some(fetch) = sel.fetch_first.take() {
                sel.top = Some(Box::new(fetch));
            }
        }
    } else if matches!(target, Dialect::Oracle) {
        // Oracle prefers FETCH FIRST n ROWS ONLY (SQL:2008 syntax)
        if let Some(limit) = sel.limit.take() {
            sel.fetch_first = Some(limit);
        }
        if let Some(top) = sel.top.take() {
            sel.fetch_first = Some(*top);
        }
    } else {
        // All other dialects: normalize to LIMIT
        if let Some(top) = sel.top.take() {
            if sel.limit.is_none() {
                sel.limit = Some(*top);
            }
        }
        if let Some(fetch) = sel.fetch_first.take() {
            if sel.limit.is_none() {
                sel.limit = Some(fetch);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Quoted-identifier transform
// ═══════════════════════════════════════════════════════════════════════════

/// Convert any quoted identifiers in expressions to the target dialect's
/// quoting convention.
fn transform_quotes(expr: Expr, target: Dialect) -> Expr {
    match expr {
        Expr::Column {
            table,
            name,
            quote_style,
            table_quote_style,
        } => {
            let new_qs = if quote_style.is_quoted() {
                QuoteStyle::for_dialect(target)
            } else {
                QuoteStyle::None
            };
            let new_tqs = if table_quote_style.is_quoted() {
                QuoteStyle::for_dialect(target)
            } else {
                QuoteStyle::None
            };
            Expr::Column {
                table,
                name,
                quote_style: new_qs,
                table_quote_style: new_tqs,
            }
        }
        // Recurse into sub-expressions
        Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
            left: Box::new(transform_quotes(*left, target)),
            op,
            right: Box::new(transform_quotes(*right, target)),
        },
        Expr::UnaryOp { op, expr } => Expr::UnaryOp {
            op,
            expr: Box::new(transform_quotes(*expr, target)),
        },
        Expr::Function {
            name,
            args,
            distinct,
            filter,
            over,
        } => Expr::Function {
            name,
            args: args
                .into_iter()
                .map(|a| transform_quotes(a, target))
                .collect(),
            distinct,
            filter: filter.map(|f| Box::new(transform_quotes(*f, target))),
            over,
        },
        Expr::TypedFunction { func, filter, over } => Expr::TypedFunction {
            func: func.transform_children(&|e| transform_quotes(e, target)),
            filter: filter.map(|f| Box::new(transform_quotes(*f, target))),
            over,
        },
        Expr::Nested(inner) => Expr::Nested(Box::new(transform_quotes(*inner, target))),
        Expr::Alias { expr, name } => Expr::Alias {
            expr: Box::new(transform_quotes(*expr, target)),
            name,
        },
        other => other,
    }
}

/// Transform quoting for all identifier-bearing nodes inside a SELECT.
fn transform_quotes_in_select(sel: &mut SelectStatement, target: Dialect) {
    // Columns in the select list
    for item in &mut sel.columns {
        if let SelectItem::Expr { expr, .. } = item {
            *expr = transform_quotes(expr.clone(), target);
        }
    }
    // WHERE
    if let Some(wh) = &mut sel.where_clause {
        *wh = transform_quotes(wh.clone(), target);
    }
    // GROUP BY
    for gb in &mut sel.group_by {
        *gb = transform_quotes(gb.clone(), target);
    }
    // HAVING
    if let Some(having) = &mut sel.having {
        *having = transform_quotes(having.clone(), target);
    }
    // ORDER BY
    for ob in &mut sel.order_by {
        ob.expr = transform_quotes(ob.expr.clone(), target);
    }
    // Table refs (FROM, JOINs)
    if let Some(from) = &mut sel.from {
        transform_quotes_in_table_source(&mut from.source, target);
    }
    for join in &mut sel.joins {
        transform_quotes_in_table_source(&mut join.table, target);
        if let Some(on) = &mut join.on {
            *on = transform_quotes(on.clone(), target);
        }
    }
}

fn transform_quotes_in_table_source(source: &mut TableSource, target: Dialect) {
    match source {
        TableSource::Table(tref) => {
            if tref.name_quote_style.is_quoted() {
                tref.name_quote_style = QuoteStyle::for_dialect(target);
            }
        }
        TableSource::Subquery { .. } => {}
        TableSource::TableFunction { .. } => {}
        TableSource::Lateral { source } => transform_quotes_in_table_source(source, target),
        TableSource::Pivot { source, .. } | TableSource::Unpivot { source, .. } => {
            transform_quotes_in_table_source(source, target);
        }
        TableSource::Unnest { .. } => {}
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Concat operator transform (Change 3: || → CONCAT() for T-SQL)
// ═══════════════════════════════════════════════════════════════════════════

/// Collect all operands from a chain of `||` (Concat) operations into a flat list.
fn collect_concat_args(expr: &Expr, args: &mut Vec<Expr>) {
    match expr {
        Expr::BinaryOp {
            left,
            op: BinaryOperator::Concat,
            right,
        } => {
            collect_concat_args(left, args);
            collect_concat_args(right, args);
        }
        other => args.push(other.clone()),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Interval arithmetic transform (Change 6: expr ± INTERVAL → DATEADD())
// ═══════════════════════════════════════════════════════════════════════════

/// Try to transform `expr ± INTERVAL 'n unit'` into `DATEADD(unit, ±n, expr)` for T-SQL.
/// Returns `Some(transformed_expr)` if the right side is an interval, `None` otherwise.
fn try_transform_interval_arithmetic(
    left: &Expr,
    op: &BinaryOperator,
    right: &Expr,
) -> Option<Expr> {
    // Check right side is an interval
    if let Expr::Interval { value, unit } = right {
        if let Some((count, unit_name)) = parse_interval_value(value, unit) {
            let final_count = if matches!(op, BinaryOperator::Minus) {
                -count
            } else {
                count
            };
            return Some(Expr::Function {
                name: "DATEADD".to_string(),
                args: vec![
                    // Use a Column expr for the datepart keyword (unquoted identifier)
                    Expr::Column {
                        table: None,
                        name: unit_name,
                        quote_style: QuoteStyle::None,
                        table_quote_style: QuoteStyle::None,
                    },
                    Expr::Number(final_count.to_string()),
                    left.clone(),
                ],
                distinct: false,
                filter: None,
                over: None,
            });
        }
    }

    // Check left side is an interval (less common: INTERVAL '7 days' + col)
    if let Expr::Interval { value, unit } = left {
        if matches!(op, BinaryOperator::Plus) {
            if let Some((count, unit_name)) = parse_interval_value(value, unit) {
                return Some(Expr::Function {
                    name: "DATEADD".to_string(),
                    args: vec![
                        Expr::Column {
                            table: None,
                            name: unit_name,
                            quote_style: QuoteStyle::None,
                            table_quote_style: QuoteStyle::None,
                        },
                        Expr::Number(count.to_string()),
                        right.clone(),
                    ],
                    distinct: false,
                    filter: None,
                    over: None,
                });
            }
        }
    }

    None
}

/// Parse an interval value expression and optional unit into (count, T-SQL datepart name).
fn parse_interval_value(value: &Expr, unit: &Option<DateTimeField>) -> Option<(i64, String)> {
    // Case 1: INTERVAL '7 days' (value is a string literal containing "7 days")
    if let Expr::StringLiteral(s) = value {
        let parts: Vec<&str> = s.trim().split_whitespace().collect();
        if parts.len() == 2 {
            let count: i64 = parts[0].parse().ok()?;
            let unit_name = normalize_interval_unit(parts[1])?;
            return Some((count, unit_name));
        }
        if parts.len() == 1 {
            // Just a number in the string, unit must come from the `unit` field
            let count: i64 = parts[0].parse().ok()?;
            if let Some(u) = unit {
                let unit_name = datetime_field_to_tsql(u)?;
                return Some((count, unit_name));
            }
        }
    }

    // Case 2: INTERVAL 7 DAY (value is a number, unit is DateTimeField)
    if let Expr::Number(n) = value {
        let count: i64 = n.parse().ok()?;
        if let Some(u) = unit {
            let unit_name = datetime_field_to_tsql(u)?;
            return Some((count, unit_name));
        }
    }

    None
}

/// Normalize an interval unit string to a T-SQL DATEADD part name.
fn normalize_interval_unit(unit: &str) -> Option<String> {
    let lower = unit.to_lowercase();
    let normalized = lower.trim_end_matches('s');
    match normalized {
        "year" => Some("YEAR".to_string()),
        "month" => Some("MONTH".to_string()),
        "week" => Some("WEEK".to_string()),
        "day" => Some("DAY".to_string()),
        "hour" => Some("HOUR".to_string()),
        "minute" => Some("MINUTE".to_string()),
        "second" => Some("SECOND".to_string()),
        "millisecond" => Some("MILLISECOND".to_string()),
        "microsecond" => Some("MICROSECOND".to_string()),
        _ => None,
    }
}

/// Convert a DateTimeField to T-SQL DATEADD unit name.
fn datetime_field_to_tsql(field: &DateTimeField) -> Option<String> {
    match field {
        DateTimeField::Year => Some("YEAR".to_string()),
        DateTimeField::Quarter => Some("QUARTER".to_string()),
        DateTimeField::Month => Some("MONTH".to_string()),
        DateTimeField::Week => Some("WEEK".to_string()),
        DateTimeField::Day => Some("DAY".to_string()),
        DateTimeField::Hour => Some("HOUR".to_string()),
        DateTimeField::Minute => Some("MINUTE".to_string()),
        DateTimeField::Second => Some("SECOND".to_string()),
        DateTimeField::Millisecond => Some("MILLISECOND".to_string()),
        DateTimeField::Microsecond => Some("MICROSECOND".to_string()),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SIMILAR TO → LIKE pattern simplification (Change 9)
// ═══════════════════════════════════════════════════════════════════════════

/// Simplify a SIMILAR TO pattern for use with LIKE.
/// Strips regex features (|, (), +, *) that T-SQL LIKE doesn't support.
fn simplify_similar_to_pattern(pattern: &Expr) -> Expr {
    if let Expr::StringLiteral(s) = pattern {
        let simplified = s.replace('|', "%").replace('(', "").replace(')', "");
        Expr::StringLiteral(simplified)
    } else {
        pattern.clone()
    }
}
