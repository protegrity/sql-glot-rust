use serde::{Deserialize, Serialize};

use crate::ast::*;

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
fn is_tsql_family(d: Dialect) -> bool {
    matches!(d, Dialect::Tsql | Dialect::Fabric)
}

/// Dialects that natively support ILIKE.
fn supports_ilike(d: Dialect) -> bool {
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
        }
        Statement::Update(upd) => {
            for (_, val) in &mut upd.assignments {
                *val = transform_expr(val.clone(), target);
            }
            if let Some(wh) = &mut upd.where_clause {
                *wh = transform_expr(wh.clone(), target);
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
        // ILIKE → LOWER(expr) LIKE LOWER(pattern) for non-supporting dialects
        Expr::ILike {
            expr,
            pattern,
            negated,
            escape,
        } if !supports_ilike(target) => {
            Expr::Like {
                expr: Box::new(Expr::Function {
                    name: "LOWER".to_string(),
                    args: vec![transform_expr(*expr, target)],
                    distinct: false,
                    filter: None,
                    over: None,
                }),
                pattern: Box::new(Expr::Function {
                    name: "LOWER".to_string(),
                    args: vec![transform_expr(*pattern, target)],
                    distinct: false,
                    filter: None,
                    over: None,
                }),
                negated,
                escape,
            }
        }
        // Map data types in CAST
        Expr::Cast { expr, data_type } => Expr::Cast {
            expr: Box::new(transform_expr(*expr, target)),
            data_type: map_data_type(data_type, target),
        },
        // Recurse into binary ops
        Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
            left: Box::new(transform_expr(*left, target)),
            op,
            right: Box::new(transform_expr(*right, target)),
        },
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
// Function name mapping
// ═══════════════════════════════════════════════════════════════════════════

/// Map function names between dialects.
fn map_function_name(name: &str, target: Dialect) -> String {
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
            if matches!(target, Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb) {
                name.to_string()
            } else {
                "RAND".to_string()
            }
        }
        "RAND" => {
            if matches!(target, Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb) {
                "RANDOM".to_string()
            } else {
                name.to_string()
            }
        }

        // Everything else – preserve original name
        _ => name.to_string(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Data-type mapping
// ═══════════════════════════════════════════════════════════════════════════

/// Map data types between dialects.
fn map_data_type(dt: DataType, target: Dialect) -> DataType {
    match (dt, target) {
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
            args: args.into_iter().map(|a| transform_quotes(a, target)).collect(),
            distinct,
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
        TableSource::Unnest { .. } => {}
    }
}
