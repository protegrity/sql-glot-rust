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
    // CR-027: for the T-SQL family, rewrite any correlated `GROUP BY` whose
    // key list is a pure outer reference (SQL Server code 164). This walks the
    // whole statement tree — including subqueries — because the offending
    // grouping is typically nested inside a correlated scalar subquery that the
    // shallow `transform_statement` pass never visits.
    if is_tsql_family(to) {
        fix_group_by_outer_refs_stmt(&mut stmt);
    }
    // CR-031 (PSQ-3306/3307/3309): the shallow `transform_statement` pass only
    // rewrites the outermost query block, so the LIMIT/OFFSET lowering, the
    // no-op nested `ORDER BY` removal, and the aggregate `FILTER` lowering are
    // skipped inside derived tables, CTEs, subqueries, and set-operation
    // branches. This walk applies them at every level. It runs only for the
    // T-SQL family and Oracle — the sole targets that need these rewrites — so
    // every other dialect pair is left byte-for-byte unchanged.
    if is_tsql_family(to) || matches!(to, Dialect::Oracle) {
        transform_nested_blocks(&mut stmt, to, false);
    }
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

// ═══════════════════════════════════════════════════════════════════════════
// CR-027: correlated GROUP BY on a pure outer reference (T-SQL code 164)
// ═══════════════════════════════════════════════════════════════════════════
//
// PostgreSQL allows a correlated subquery's `GROUP BY` list to consist entirely
// of columns from the *outer* query (a "pure outer reference"); SQL Server
// rejects it at parse time:
//
//   Msg 164 — Each GROUP BY expression must contain at least one column that is
//             not an outer reference.
//
// Within a correlated subquery such a key is a per-row constant, so the grouping
// is redundant. The fix makes each `GROUP BY` key contain a *local* column:
//
//   1. Preferred — substitute the correlated inner column. If the subquery's
//      WHERE has an equality `<local col> = <the outer ref>`, replace the outer
//      key with `<local col>` (`c.customer_id` -> `t.customer_id`). This is
//      byte-for-byte equivalent to PostgreSQL, including NULL-for-empty groups.
//   2. Fallback — drop the redundant pure-outer key. Removing a constant key
//      never changes group membership when other local keys remain.
//   3. If the list becomes empty, the whole `GROUP BY` is dropped (empty Vec).
//
// The walk descends into subqueries because the offending grouping is usually
// nested inside a correlated scalar subquery, which the shallow
// `transform_statement` pass never visits. It only touches `GROUP BY` content;
// every other part of the query is left byte-for-byte unchanged.

/// Recursively apply the CR-027 `GROUP BY` outer-reference fix to every
/// SELECT reachable from `stmt` (including nested subqueries).
fn fix_group_by_outer_refs_stmt(stmt: &mut Statement) {
    match stmt {
        Statement::Select(sel) => fix_group_by_outer_refs_select(sel),
        Statement::SetOperation(setop) => {
            fix_group_by_outer_refs_stmt(&mut setop.left);
            fix_group_by_outer_refs_stmt(&mut setop.right);
        }
        Statement::Insert(ins) => {
            if let InsertSource::Query(query) = &mut ins.source {
                fix_group_by_outer_refs_stmt(query);
            }
        }
        Statement::CreateTable(ct) => {
            if let Some(as_select) = &mut ct.as_select {
                fix_group_by_outer_refs_stmt(as_select);
            }
        }
        Statement::CreateView(view) => fix_group_by_outer_refs_stmt(&mut view.query),
        Statement::Explain(explain) => fix_group_by_outer_refs_stmt(&mut explain.statement),
        _ => {}
    }
}

/// Descend into a SELECT's nested statements, then rewrite its own `GROUP BY`.
fn fix_group_by_outer_refs_select(sel: &mut SelectStatement) {
    // Recurse into nested statements first (CTEs, derived tables, and any
    // subqueries embedded in projections / predicates).
    for cte in &mut sel.ctes {
        fix_group_by_outer_refs_stmt(&mut cte.query);
    }
    if let Some(from) = &mut sel.from {
        fix_group_by_outer_refs_table_source(&mut from.source);
    }
    for join in &mut sel.joins {
        fix_group_by_outer_refs_table_source(&mut join.table);
        if let Some(on) = &mut join.on {
            fix_group_by_outer_refs_in_expr(on);
        }
    }
    for item in &mut sel.columns {
        if let SelectItem::Expr { expr, .. } = item {
            fix_group_by_outer_refs_in_expr(expr);
        }
    }
    if let Some(where_clause) = &mut sel.where_clause {
        fix_group_by_outer_refs_in_expr(where_clause);
    }
    if let Some(having) = &mut sel.having {
        fix_group_by_outer_refs_in_expr(having);
    }
    if let Some(qualify) = &mut sel.qualify {
        fix_group_by_outer_refs_in_expr(qualify);
    }
    for order in &mut sel.order_by {
        fix_group_by_outer_refs_in_expr(&mut order.expr);
    }

    // Rewrite this node's GROUP BY.
    if sel.group_by.is_empty() {
        return;
    }
    let local = local_source_names(sel);
    let group_by = std::mem::take(&mut sel.group_by);
    let mut kept = Vec::with_capacity(group_by.len());
    for key in group_by {
        if is_pure_outer_reference(&key, &local) {
            // 1) substitute the correlated inner column from a WHERE equality
            if let Some(inner) = correlated_inner_column(sel.where_clause.as_ref(), &key, &local) {
                kept.push(inner);
            }
            // 2) else drop the redundant pure-outer key
        } else {
            kept.push(key);
        }
    }
    // 3) an emptied GROUP BY (empty Vec) is rendered as no clause at all
    sel.group_by = kept;
}

/// Recurse into a table source, descending into derived-table subqueries.
fn fix_group_by_outer_refs_table_source(source: &mut TableSource) {
    match source {
        TableSource::Subquery { query, .. } => fix_group_by_outer_refs_stmt(query),
        TableSource::Lateral { source } => fix_group_by_outer_refs_table_source(source),
        TableSource::Pivot { source, .. } | TableSource::Unpivot { source, .. } => {
            fix_group_by_outer_refs_table_source(source);
        }
        TableSource::Table(_) | TableSource::TableFunction { .. } | TableSource::Unnest { .. } => {}
    }
}

/// Recurse into every subquery embedded anywhere in `expr`. Structural
/// recursion through composite expressions is handled by [`Expr::transform`];
/// this only needs to intercept the three statement-bearing variants.
fn fix_group_by_outer_refs_in_expr(expr: &mut Expr) {
    let taken = std::mem::replace(expr, Expr::Null);
    *expr = taken.transform(&|e| match e {
        Expr::Subquery(mut query) => {
            fix_group_by_outer_refs_stmt(&mut query);
            Expr::Subquery(query)
        }
        Expr::Exists {
            mut subquery,
            negated,
        } => {
            fix_group_by_outer_refs_stmt(&mut subquery);
            Expr::Exists { subquery, negated }
        }
        Expr::InSubquery {
            expr,
            mut subquery,
            negated,
        } => {
            fix_group_by_outer_refs_stmt(&mut subquery);
            Expr::InSubquery {
                expr,
                subquery,
                negated,
            }
        }
        other => other,
    });
}

/// The set of table names and aliases introduced by this SELECT's own `FROM`
/// and `JOIN` clauses (lowercased for case-insensitive comparison). A column
/// qualifier not in this set is an outer/correlated reference.
fn local_source_names(sel: &SelectStatement) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    if let Some(from) = &sel.from {
        collect_local_source_names(&from.source, &mut set);
    }
    for join in &sel.joins {
        collect_local_source_names(&join.table, &mut set);
    }
    set
}

fn collect_local_source_names(src: &TableSource, set: &mut std::collections::HashSet<String>) {
    match src {
        TableSource::Table(tref) => {
            set.insert(tref.name.to_lowercase());
            if let Some(alias) = &tref.alias {
                set.insert(alias.to_lowercase());
            }
        }
        TableSource::Subquery { alias, .. }
        | TableSource::TableFunction { alias, .. }
        | TableSource::Unnest { alias, .. } => {
            if let Some(alias) = alias {
                set.insert(alias.to_lowercase());
            }
        }
        TableSource::Lateral { source } => collect_local_source_names(source, set),
        TableSource::Pivot { source, alias, .. } | TableSource::Unpivot { source, alias, .. } => {
            collect_local_source_names(source, set);
            if let Some(alias) = alias {
                set.insert(alias.to_lowercase());
            }
        }
    }
}

/// A `GROUP BY` key is a "pure outer reference" iff it contains at least one
/// column, every column is qualified, and every qualifier is non-local.
/// Unqualified columns are conservatively treated as local, so any unqualified
/// column disqualifies the key (it is never rewritten).
fn is_pure_outer_reference(expr: &Expr, local: &std::collections::HashSet<String>) -> bool {
    let columns = expr.find_all(&|e| matches!(e, Expr::Column { .. }));
    if columns.is_empty() {
        return false;
    }
    let mut has_qualified = false;
    for col in &columns {
        match col {
            Expr::Column { table: Some(t), .. } => {
                if local.contains(&t.to_lowercase()) {
                    return false;
                }
                has_qualified = true;
            }
            // Unqualified column — treat as local; the key is not pure-outer.
            Expr::Column { table: None, .. } => return false,
            _ => {}
        }
    }
    has_qualified
}

/// If the subquery's WHERE contains a top-level equality `<local col> = <outer
/// key>`, return the local column to substitute for the outer key. Recurses
/// through `AND` and parentheses only (top-level conjuncts).
fn correlated_inner_column(
    where_clause: Option<&Expr>,
    outer_key: &Expr,
    local: &std::collections::HashSet<String>,
) -> Option<Expr> {
    find_correlating_local_column(where_clause?, outer_key, local)
}

fn find_correlating_local_column(
    cond: &Expr,
    outer_key: &Expr,
    local: &std::collections::HashSet<String>,
) -> Option<Expr> {
    match cond {
        Expr::BinaryOp {
            left,
            op: BinaryOperator::And,
            right,
        } => find_correlating_local_column(left, outer_key, local)
            .or_else(|| find_correlating_local_column(right, outer_key, local)),
        Expr::Nested(inner) => find_correlating_local_column(inner, outer_key, local),
        Expr::BinaryOp {
            left,
            op: BinaryOperator::Eq,
            right,
        } => {
            if columns_match(left, outer_key) && is_local_qualified_column(right, local) {
                Some((**right).clone())
            } else if columns_match(right, outer_key) && is_local_qualified_column(left, local) {
                Some((**left).clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_local_qualified_column(expr: &Expr, local: &std::collections::HashSet<String>) -> bool {
    matches!(expr, Expr::Column { table: Some(t), .. } if local.contains(&t.to_lowercase()))
}

/// Compare two column references by qualifier + name, case-insensitively and
/// ignoring quote style. Non-column expressions never match.
fn columns_match(a: &Expr, b: &Expr) -> bool {
    match (a, b) {
        (
            Expr::Column {
                table: ta,
                name: na,
                ..
            },
            Expr::Column {
                table: tb,
                name: nb,
                ..
            },
        ) => {
            na.eq_ignore_ascii_case(nb)
                && match (ta, tb) {
                    (Some(x), Some(y)) => x.eq_ignore_ascii_case(y),
                    (None, None) => true,
                    _ => false,
                }
        }
        _ => false,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CR-031 (PSQ-3306 / PSQ-3307 / PSQ-3309): nested-block dialect transforms
// ═══════════════════════════════════════════════════════════════════════════
//
// `transform_statement` only rewrites the outermost query block, so three
// PostgreSQL→backend conversions are silently skipped when the relevant clause
// is nested inside a derived table, CTE, subquery, or set-operation branch:
//
//   1. PSQ-3306 — `LIMIT … OFFSET …` is not lowered to the target's `TOP` /
//      `OFFSET … ROWS FETCH …` form, so T-SQL and Oracle reject the invalid
//      `LIMIT … OFFSET … ROWS` hybrid the generator emits from the untouched
//      PostgreSQL shape.
//   2. PSQ-3307 — a *no-op* `ORDER BY` (one with no `TOP`/`LIMIT`/`OFFSET`/
//      `FETCH`) left in a derived table / subquery is rejected by SQL Server
//      (error 1033). It cannot affect the outer result, so it is dropped for
//      the T-SQL family; PostgreSQL and Oracle accept it and are left alone.
//   3. PSQ-3309 (secondary) — an aggregate `FILTER (WHERE p)` is emitted
//      verbatim; T-SQL and Oracle have no `FILTER` clause and reject it, so it
//      is lowered to the equivalent `agg(CASE WHEN p THEN … END)`.
//
// The walk mirrors the CR-027 `fix_group_by_outer_refs_stmt` deep walk (the
// crate's established pattern for reaching nested query blocks). `is_subquery`
// is `false` for the top-level statement — whose outermost SELECT was already
// handled by `transform_statement` — and becomes `true` as soon as the walk
// descends into any nested block, which is where the LIMIT/OFFSET and ORDER BY
// rewrites apply. The FILTER lowering applies at every level.

/// Recursively apply the CR-031 nested-block transforms to every query block
/// reachable from `stmt`.
fn transform_nested_blocks(stmt: &mut Statement, target: Dialect, is_subquery: bool) {
    match stmt {
        Statement::Select(sel) => transform_nested_select(sel, target, is_subquery),
        Statement::SetOperation(setop) => {
            transform_nested_blocks(&mut setop.left, target, true);
            transform_nested_blocks(&mut setop.right, target, true);
        }
        Statement::Insert(ins) => {
            if let InsertSource::Query(query) = &mut ins.source {
                transform_nested_blocks(query, target, true);
            }
        }
        Statement::CreateTable(ct) => {
            if let Some(as_select) = &mut ct.as_select {
                transform_nested_blocks(as_select, target, true);
            }
        }
        Statement::CreateView(view) => transform_nested_blocks(&mut view.query, target, true),
        Statement::Explain(explain) => {
            transform_nested_blocks(&mut explain.statement, target, true);
        }
        _ => {}
    }
}

/// Descend into a SELECT's nested statements, then apply the three CR-031
/// rewrites to this node.
fn transform_nested_select(sel: &mut SelectStatement, target: Dialect, is_subquery: bool) {
    // Recurse into nested statements first (CTEs, derived tables, JOINs, and any
    // subqueries embedded in projections / predicates / ORDER BY).
    for cte in &mut sel.ctes {
        transform_nested_blocks(&mut cte.query, target, true);
    }
    if let Some(from) = &mut sel.from {
        transform_nested_table_source(&mut from.source, target);
    }
    for join in &mut sel.joins {
        transform_nested_table_source(&mut join.table, target);
        if let Some(on) = &mut join.on {
            transform_nested_in_expr(on, target);
        }
    }
    for item in &mut sel.columns {
        if let SelectItem::Expr { expr, .. } = item {
            transform_nested_in_expr(expr, target);
        }
    }
    if let Some(where_clause) = &mut sel.where_clause {
        transform_nested_in_expr(where_clause, target);
    }
    for gb in &mut sel.group_by {
        transform_nested_in_expr(gb, target);
    }
    if let Some(having) = &mut sel.having {
        transform_nested_in_expr(having, target);
    }
    if let Some(qualify) = &mut sel.qualify {
        transform_nested_in_expr(qualify, target);
    }
    for order in &mut sel.order_by {
        transform_nested_in_expr(&mut order.expr, target);
    }

    // Fix 1 (PSQ-3306): lower LIMIT / OFFSET for a *nested* SELECT. The
    // outermost SELECT was already handled by `transform_statement`, so only
    // nested blocks are touched here — avoiding a double application.
    if is_subquery {
        transform_limit(sel, target);
    }

    // Fix 2 (PSQ-3307): drop a no-op `ORDER BY` in a nested T-SQL block. This
    // runs *after* `transform_limit`, so a paginated subquery — now carrying
    // `TOP` or `OFFSET`/`FETCH` — keeps its `ORDER BY`; only an `ORDER BY` with
    // no row-limiting clause (which cannot affect the outer result) is removed.
    if is_subquery
        && is_tsql_family(target)
        && !sel.order_by.is_empty()
        && sel.top.is_none()
        && sel.limit.is_none()
        && sel.offset.is_none()
        && sel.fetch_first.is_none()
    {
        sel.order_by.clear();
    }

    // Fix 3 (PSQ-3309, secondary): lower aggregate `FILTER (WHERE p)` for the
    // T-SQL family and Oracle at every level (the outermost SELECT included,
    // since `transform_expr` does not lower `FILTER`).
    lower_agg_filter_in_select(sel);
}

/// Recurse into a table source, descending into derived-table subqueries.
fn transform_nested_table_source(source: &mut TableSource, target: Dialect) {
    match source {
        TableSource::Subquery { query, .. } => transform_nested_blocks(query, target, true),
        TableSource::Lateral { source } => transform_nested_table_source(source, target),
        TableSource::Pivot { source, .. } | TableSource::Unpivot { source, .. } => {
            transform_nested_table_source(source, target);
        }
        TableSource::Table(_) | TableSource::TableFunction { .. } | TableSource::Unnest { .. } => {}
    }
}

/// Recurse into every subquery embedded anywhere in `expr`. Structural
/// recursion through composite expressions is handled by [`Expr::transform`];
/// this only needs to intercept the three statement-bearing variants (the same
/// technique as the CR-027 walk).
fn transform_nested_in_expr(expr: &mut Expr, target: Dialect) {
    let taken = std::mem::replace(expr, Expr::Null);
    *expr = taken.transform(&|e| match e {
        Expr::Subquery(mut query) => {
            transform_nested_blocks(&mut query, target, true);
            Expr::Subquery(query)
        }
        Expr::Exists {
            mut subquery,
            negated,
        } => {
            transform_nested_blocks(&mut subquery, target, true);
            Expr::Exists { subquery, negated }
        }
        Expr::InSubquery {
            expr,
            mut subquery,
            negated,
        } => {
            transform_nested_blocks(&mut subquery, target, true);
            Expr::InSubquery {
                expr,
                subquery,
                negated,
            }
        }
        other => other,
    });
}

// ── Fix 3: aggregate FILTER (WHERE p) → agg(CASE WHEN p THEN … END) ──────────

/// Lower every aggregate `FILTER (WHERE p)` in this SELECT's own expressions
/// (projection, `HAVING`, `QUALIFY`, `ORDER BY`) to the `CASE`-wrapped form.
/// Subqueries are visited by the surrounding walk, so [`Expr::transform`]
/// (which does not cross into `Box<Statement>`) is exactly the right traversal.
fn lower_agg_filter_in_select(sel: &mut SelectStatement) {
    for item in &mut sel.columns {
        if let SelectItem::Expr { expr, .. } = item {
            lower_agg_filter_in_expr(expr);
        }
    }
    if let Some(having) = &mut sel.having {
        lower_agg_filter_in_expr(having);
    }
    if let Some(qualify) = &mut sel.qualify {
        lower_agg_filter_in_expr(qualify);
    }
    for order in &mut sel.order_by {
        lower_agg_filter_in_expr(&mut order.expr);
    }
}

fn lower_agg_filter_in_expr(expr: &mut Expr) {
    let taken = std::mem::replace(expr, Expr::Null);
    *expr = taken.transform(&lower_agg_filter_node);
}

/// Rewrite a single aggregate node that carries a `filter` into the
/// `CASE`-wrapped, filter-free form. Nodes without a `filter` — and typed
/// aggregates the wrapper does not recognise — are returned unchanged (the
/// latter keep their `filter` rather than silently dropping it).
fn lower_agg_filter_node(e: Expr) -> Expr {
    match e {
        Expr::Function {
            name,
            args,
            distinct,
            filter: Some(pred),
            over,
            order_by,
            within_group,
        } => Expr::Function {
            name,
            args: case_wrap_agg_args(args, &pred),
            distinct,
            filter: None,
            over,
            order_by,
            within_group,
        },
        Expr::TypedFunction {
            func,
            filter: Some(pred),
            over,
        } => match case_wrap_typed_agg(func, &pred) {
            Ok(func) => Expr::TypedFunction {
                func,
                filter: None,
                over,
            },
            Err(func) => Expr::TypedFunction {
                func,
                filter: Some(pred),
                over,
            },
        },
        other => other,
    }
}

/// Build `CASE WHEN <pred> THEN <then> END`.
fn case_when(pred: &Expr, then: Expr) -> Expr {
    Expr::Case {
        operand: None,
        when_clauses: vec![(pred.clone(), then)],
        else_clause: None,
    }
}

/// Wrap the value argument of a generic aggregate in a filtering `CASE`.
/// `COUNT(*)` (a `Wildcard` or empty argument list) becomes
/// `COUNT(CASE WHEN p THEN 1 END)`; any other single argument `x` becomes
/// `agg(CASE WHEN p THEN x END)`. For a multi-argument aggregate such as
/// `STRING_AGG(x, ',')` only the first argument — the value being aggregated —
/// is wrapped, matching the standard conditional-aggregation lowering.
fn case_wrap_agg_args(mut args: Vec<Expr>, pred: &Expr) -> Vec<Expr> {
    if args.is_empty() {
        return vec![case_when(pred, Expr::Number("1".to_string()))];
    }
    if matches!(args[0], Expr::Wildcard) {
        args[0] = case_when(pred, Expr::Number("1".to_string()));
    } else {
        let first = std::mem::replace(&mut args[0], Expr::Null);
        args[0] = case_when(pred, first);
    }
    args
}

/// Wrap the argument of a typed aggregate in a filtering `CASE`. `COUNT(*)`
/// becomes `COUNT(CASE WHEN p THEN 1 END)`; every other single-argument
/// aggregate `agg(x)` becomes `agg(CASE WHEN p THEN x END)`. Returns `Err`
/// (with the function unchanged) for typed functions that are not
/// single-argument aggregates, so their `filter` is preserved rather than
/// dropped.
fn case_wrap_typed_agg(func: TypedFunction, pred: &Expr) -> Result<TypedFunction, TypedFunction> {
    let wrap = |expr: Box<Expr>| Box::new(case_when(pred, *expr));
    Ok(match func {
        TypedFunction::Count { expr, distinct } => TypedFunction::Count {
            expr: if matches!(*expr, Expr::Wildcard) {
                Box::new(case_when(pred, Expr::Number("1".to_string())))
            } else {
                wrap(expr)
            },
            distinct,
        },
        TypedFunction::Sum { expr, distinct } => TypedFunction::Sum {
            expr: wrap(expr),
            distinct,
        },
        TypedFunction::Avg { expr, distinct } => TypedFunction::Avg {
            expr: wrap(expr),
            distinct,
        },
        TypedFunction::Min { expr } => TypedFunction::Min { expr: wrap(expr) },
        TypedFunction::Max { expr } => TypedFunction::Max { expr: wrap(expr) },
        TypedFunction::ArrayAgg { expr, distinct } => TypedFunction::ArrayAgg {
            expr: wrap(expr),
            distinct,
        },
        TypedFunction::ApproxDistinct { expr } => {
            TypedFunction::ApproxDistinct { expr: wrap(expr) }
        }
        TypedFunction::Variance { expr } => TypedFunction::Variance { expr: wrap(expr) },
        TypedFunction::VariancePop { expr } => TypedFunction::VariancePop { expr: wrap(expr) },
        TypedFunction::Stddev { expr } => TypedFunction::Stddev { expr: wrap(expr) },
        TypedFunction::StddevPop { expr } => TypedFunction::StddevPop { expr: wrap(expr) },
        other => return Err(other),
    })
}

/// Returns `true` when `expr` is already a zero-guarded divisor — either an
/// `Expr::NullIf { .., r#else: 0 }` node or a `NULLIF(<x>, 0)` function call
/// (the parser lowers `NULLIF(...)` to `Expr::Function`). Used by the CR-019
/// safe-divide transform to avoid emitting a redundant
/// `NULLIF(NULLIF(<x>, 0), 0)` when the source already guarded the divisor.
fn is_zero_guarded_divisor(expr: &Expr) -> bool {
    let is_zero = |e: &Expr| matches!(e, Expr::Number(n) if n == "0");
    match expr {
        Expr::NullIf { r#else, .. } => is_zero(r#else),
        Expr::Function { name, args, .. } => {
            name.eq_ignore_ascii_case("NULLIF") && args.len() == 2 && is_zero(&args[1])
        }
        _ => false,
    }
}

/// CR-029 (PSQ-3267): returns `true` when `expr` is a compile-time numeric
/// literal — optionally wrapped in redundant parentheses or a leading unary
/// sign (`0`, `(0)`, `-0`, `+5`). The CR-019 safe-divide wrap must NOT be
/// applied to a literal divisor: a literal `0` is an unconditional
/// divide-by-zero that must error on the backend (matching PostgreSQL and
/// Oracle, which raise a hard error), and a literal non-zero divisor can never
/// be `0`, so `NULLIF(<lit>, 0)` would be dead code. Only non-literal divisors
/// (columns, expressions, `CAST`, subqueries) keep the guard — exactly the
/// PSQ-2758 / q74 shape CR-019 was written for.
fn is_numeric_literal(expr: &Expr) -> bool {
    match expr {
        Expr::Number(_) => true,
        Expr::Nested(inner) => is_numeric_literal(inner),
        Expr::UnaryOp {
            op: UnaryOperator::Minus | UnaryOperator::Plus,
            expr,
        } => is_numeric_literal(expr),
        _ => false,
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
            order_by,
            within_group,
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
                order_by,
                within_group,
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
                    order_by: vec![],
                    within_group: false,
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

            // Change 7 (CR-019): a / b → a / NULLIF(b, 0) for the T-SQL family,
            // so a zero divisor yields NULL instead of raising "Divide by zero"
            // (code 8134). SQL Server does not short-circuit the ANDed WHERE qual
            // list the way PostgreSQL does, so a guard such as `WHERE b <> 0`
            // cannot be relied on to protect the division — the guard must live
            // in the expression itself. NULLIF(b, 0) returns b unchanged whenever
            // b <> 0, so non-zero divisors are behaviorally unaffected. Modulo is
            // included because `x % 0` raises the same 8134 error class. A divisor
            // already written as NULLIF(<x>, 0) is left as-is (no double wrap).
            //
            // CR-029 (PSQ-3267): a divisor that is a compile-time numeric literal
            // is also left as-is. Wrapping a literal `0` would convert a genuine,
            // unconditional divide-by-zero (which PostgreSQL and Oracle both raise
            // as a hard error) into a silent NULL on MSSQL, masking SQL Server
            // code 8134; wrapping a literal non-zero divisor is dead code. Only
            // non-literal divisors can be undecidably zero at transpile time, so
            // only they need the guard.
            if is_tsql_family(target)
                && matches!(op, BinaryOperator::Divide | BinaryOperator::Modulo)
                && !is_zero_guarded_divisor(&right_transformed)
                && !is_numeric_literal(&right_transformed)
            {
                return Expr::BinaryOp {
                    left: Box::new(left_transformed),
                    op,
                    right: Box::new(Expr::NullIf {
                        expr: Box::new(right_transformed),
                        r#else: Box::new(Expr::Number("0".to_string())),
                    }),
                };
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
            // TEXT is a large-object string; emit VARCHAR(MAX) so the value is
            // not truncated to the MSSQL CAST default length of 30.
            DataType::VarcharMax
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
                            query_options: None,
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
            order_by,
            within_group,
        } => Expr::Function {
            name,
            args: args
                .into_iter()
                .map(|a| transform_quotes(a, target))
                .collect(),
            distinct,
            filter: filter.map(|f| Box::new(transform_quotes(*f, target))),
            over,
            order_by,
            within_group,
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
                order_by: vec![],
                within_group: false,
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
                    order_by: vec![],
                    within_group: false,
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
