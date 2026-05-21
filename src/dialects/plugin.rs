use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::ast::{DataType, Expr, QuoteStyle, Statement};

/// Trait that external code can implement to define a custom SQL dialect.
///
/// All methods have default implementations that return `None`, meaning
/// "no custom behaviour — fall through to the built-in logic". Implementors
/// only need to override the methods they care about.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` because the global registry is
/// shared across threads.
///
/// # Example
///
/// ```rust
/// use sqlglot_rust::dialects::plugin::{DialectPlugin, DialectRegistry};
/// use sqlglot_rust::ast::{DataType, Expr, QuoteStyle, Statement};
///
/// struct MyDialect;
///
/// impl DialectPlugin for MyDialect {
///     fn name(&self) -> &str { "mydialect" }
///
///     fn map_function_name(&self, name: &str) -> Option<String> {
///         match name.to_uppercase().as_str() {
///             "MY_FUNC" => Some("BUILTIN_FUNC".to_string()),
///             _ => None,
///         }
///     }
///
///     fn quote_style(&self) -> Option<QuoteStyle> {
///         Some(QuoteStyle::Backtick)
///     }
/// }
///
/// // Register once, then use via DialectRef::Custom("mydialect")
/// DialectRegistry::global().register(MyDialect);
/// ```
pub trait DialectPlugin: Send + Sync {
    /// Canonical lower-case name for this dialect (e.g. `"mydialect"`).
    fn name(&self) -> &str;

    // ── Tokenizer rules ──────────────────────────────────────────────

    /// Preferred quoting style for identifiers.
    fn quote_style(&self) -> Option<QuoteStyle> {
        None
    }

    // ── Parser rules ─────────────────────────────────────────────────

    /// Whether this dialect natively supports `ILIKE`.
    fn supports_ilike(&self) -> Option<bool> {
        None
    }

    // ── Generator / transform rules ──────────────────────────────────

    /// Map a function name for this dialect.
    ///
    /// Return `Some(new_name)` to override, or `None` to keep the original.
    fn map_function_name(&self, name: &str) -> Option<String> {
        let _ = name;
        None
    }

    /// Map a data type for this dialect.
    ///
    /// Return `Some(new_type)` to override, or `None` to keep the original.
    fn map_data_type(&self, data_type: &DataType) -> Option<DataType> {
        let _ = data_type;
        None
    }

    /// Transform an entire expression for this dialect.
    ///
    /// Return `Some(new_expr)` to replace the expression, or `None` to
    /// fall through to the default transformation logic.
    fn transform_expr(&self, expr: &Expr) -> Option<Expr> {
        let _ = expr;
        None
    }

    /// Transform a complete statement for this dialect.
    ///
    /// Return `Some(new_stmt)` to replace the statement, or `None` to
    /// fall through to the default transformation logic.
    fn transform_statement(&self, statement: &Statement) -> Option<Statement> {
        let _ = statement;
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Global registry
// ═══════════════════════════════════════════════════════════════════════════

/// Thread-safe registry for custom dialect plugins.
///
/// Access the singleton with [`DialectRegistry::global()`].
pub struct DialectRegistry {
    dialects: Mutex<HashMap<String, Arc<dyn DialectPlugin>>>,
}

impl DialectRegistry {
    /// Returns the global registry singleton.
    pub fn global() -> &'static DialectRegistry {
        static INSTANCE: OnceLock<DialectRegistry> = OnceLock::new();
        INSTANCE.get_or_init(|| DialectRegistry {
            dialects: Mutex::new(HashMap::new()),
        })
    }

    /// Register a custom dialect plugin.
    ///
    /// If a plugin with the same name already exists it is replaced.
    pub fn register<P: DialectPlugin + 'static>(&self, plugin: P) {
        let name = plugin.name().to_lowercase();
        let mut map = self
            .dialects
            .lock()
            .expect("dialect registry lock poisoned");
        map.insert(name, Arc::new(plugin));
    }

    /// Look up a custom dialect by name (case-insensitive).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn DialectPlugin>> {
        let map = self
            .dialects
            .lock()
            .expect("dialect registry lock poisoned");
        map.get(&name.to_lowercase()).cloned()
    }

    /// Remove a custom dialect plugin by name.
    ///
    /// Returns `true` if the dialect was found and removed.
    pub fn unregister(&self, name: &str) -> bool {
        let mut map = self
            .dialects
            .lock()
            .expect("dialect registry lock poisoned");
        map.remove(&name.to_lowercase()).is_some()
    }

    /// Returns the names of all registered custom dialects.
    #[must_use]
    pub fn registered_names(&self) -> Vec<String> {
        let map = self
            .dialects
            .lock()
            .expect("dialect registry lock poisoned");
        map.keys().cloned().collect()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DialectRef — unified built-in + custom dialect handle
// ═══════════════════════════════════════════════════════════════════════════

use crate::dialects::Dialect;

/// A reference to either a built-in [`Dialect`] or a custom dialect
/// registered via the plugin system.
///
/// This is the primary handle that plugin-aware API functions accept.
///
/// # Example
///
/// ```rust
/// use sqlglot_rust::dialects::plugin::DialectRef;
/// use sqlglot_rust::Dialect;
///
/// let builtin = DialectRef::from(Dialect::Postgres);
/// let custom  = DialectRef::custom("mydialect");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DialectRef {
    /// A built-in dialect variant.
    BuiltIn(Dialect),
    /// A custom dialect identified by its registered name.
    Custom(String),
}

impl DialectRef {
    /// Create a `DialectRef` for a custom dialect by name.
    #[must_use]
    pub fn custom(name: &str) -> Self {
        DialectRef::Custom(name.to_lowercase())
    }

    /// Try to resolve this reference to a built-in dialect.
    #[must_use]
    pub fn as_builtin(&self) -> Option<Dialect> {
        match self {
            DialectRef::BuiltIn(d) => Some(*d),
            DialectRef::Custom(_) => None,
        }
    }

    /// Try to resolve this reference to a custom plugin.
    #[must_use]
    pub fn as_plugin(&self) -> Option<Arc<dyn DialectPlugin>> {
        match self {
            DialectRef::Custom(name) => DialectRegistry::global().get(name),
            DialectRef::BuiltIn(_) => None,
        }
    }

    /// Get the quote style for this dialect reference.
    #[must_use]
    pub fn quote_style(&self) -> QuoteStyle {
        match self {
            DialectRef::BuiltIn(d) => QuoteStyle::for_dialect(*d),
            DialectRef::Custom(name) => DialectRegistry::global()
                .get(name)
                .and_then(|p| p.quote_style())
                .unwrap_or(QuoteStyle::DoubleQuote),
        }
    }

    /// Check if this dialect supports ILIKE natively.
    #[must_use]
    pub fn supports_ilike(&self) -> bool {
        match self {
            DialectRef::BuiltIn(d) => super::supports_ilike_builtin(*d),
            DialectRef::Custom(name) => DialectRegistry::global()
                .get(name)
                .and_then(|p| p.supports_ilike())
                .unwrap_or(false),
        }
    }

    /// Map a function name using this dialect's rules.
    #[must_use]
    pub fn map_function_name(&self, name: &str) -> String {
        match self {
            DialectRef::BuiltIn(d) => super::map_function_name(name, *d),
            DialectRef::Custom(cname) => DialectRegistry::global()
                .get(cname)
                .and_then(|p| p.map_function_name(name))
                .unwrap_or_else(|| name.to_string()),
        }
    }

    /// Map a data type using this dialect's rules.
    #[must_use]
    pub fn map_data_type(&self, dt: &DataType) -> DataType {
        match self {
            DialectRef::BuiltIn(d) => super::map_data_type(dt.clone(), *d),
            DialectRef::Custom(name) => DialectRegistry::global()
                .get(name)
                .and_then(|p| p.map_data_type(dt))
                .unwrap_or_else(|| dt.clone()),
        }
    }
}

impl From<Dialect> for DialectRef {
    fn from(d: Dialect) -> Self {
        DialectRef::BuiltIn(d)
    }
}

impl std::fmt::Display for DialectRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialectRef::BuiltIn(d) => write!(f, "{d}"),
            DialectRef::Custom(name) => write!(f, "Custom({name})"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Plugin-aware transform
// ═══════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════
// Plugin-aware transform
// ═══════════════════════════════════════════════════════════════════════════

use crate::ast::TypedFunction;

/// Return the canonical SQL function name for a TypedFunction variant.
fn typed_function_canonical_name(func: &TypedFunction) -> &'static str {
    match func {
        TypedFunction::DateAdd { .. } => "DATE_ADD",
        TypedFunction::DateDiff { .. } => "DATE_DIFF",
        TypedFunction::DateTrunc { .. } => "DATE_TRUNC",
        TypedFunction::DateSub { .. } => "DATE_SUB",
        TypedFunction::CurrentDate => "CURRENT_DATE",
        TypedFunction::CurrentTimestamp => "NOW",
        TypedFunction::StrToTime { .. } => "STR_TO_TIME",
        TypedFunction::TimeToStr { .. } => "TIME_TO_STR",
        TypedFunction::TsOrDsToDate { .. } => "TS_OR_DS_TO_DATE",
        TypedFunction::Year { .. } => "YEAR",
        TypedFunction::Month { .. } => "MONTH",
        TypedFunction::Day { .. } => "DAY",
        TypedFunction::Trim { .. } => "TRIM",
        TypedFunction::Substring { .. } => "SUBSTRING",
        TypedFunction::Upper { .. } => "UPPER",
        TypedFunction::Lower { .. } => "LOWER",
        TypedFunction::RegexpLike { .. } => "REGEXP_LIKE",
        TypedFunction::RegexpExtract { .. } => "REGEXP_EXTRACT",
        TypedFunction::RegexpReplace { .. } => "REGEXP_REPLACE",
        TypedFunction::ConcatWs { .. } => "CONCAT_WS",
        TypedFunction::Split { .. } => "SPLIT",
        TypedFunction::Initcap { .. } => "INITCAP",
        TypedFunction::Length { .. } => "LENGTH",
        TypedFunction::Replace { .. } => "REPLACE",
        TypedFunction::Reverse { .. } => "REVERSE",
        TypedFunction::Left { .. } => "LEFT",
        TypedFunction::Right { .. } => "RIGHT",
        TypedFunction::Lpad { .. } => "LPAD",
        TypedFunction::Rpad { .. } => "RPAD",
        TypedFunction::Count { .. } => "COUNT",
        TypedFunction::Sum { .. } => "SUM",
        TypedFunction::Avg { .. } => "AVG",
        TypedFunction::Min { .. } => "MIN",
        TypedFunction::Max { .. } => "MAX",
        TypedFunction::ArrayAgg { .. } => "ARRAY_AGG",
        TypedFunction::ApproxDistinct { .. } => "APPROX_DISTINCT",
        TypedFunction::Variance { .. } => "VARIANCE",
        TypedFunction::Stddev { .. } => "STDDEV",
        TypedFunction::GroupConcat { .. } => "GROUP_CONCAT",
        TypedFunction::ArrayConcat { .. } => "ARRAY_CONCAT",
        TypedFunction::ArrayContains { .. } => "ARRAY_CONTAINS",
        TypedFunction::ArraySize { .. } => "ARRAY_SIZE",
        TypedFunction::Explode { .. } => "EXPLODE",
        TypedFunction::GenerateSeries { .. } => "GENERATE_SERIES",
        TypedFunction::Flatten { .. } => "FLATTEN",
        TypedFunction::JSONExtract { .. } => "JSON_EXTRACT",
        TypedFunction::JSONExtractScalar { .. } => "JSON_EXTRACT_SCALAR",
        TypedFunction::ParseJSON { .. } => "PARSE_JSON",
        TypedFunction::JSONFormat { .. } => "JSON_FORMAT",
        TypedFunction::RowNumber => "ROW_NUMBER",
        TypedFunction::Rank => "RANK",
        TypedFunction::DenseRank => "DENSE_RANK",
        TypedFunction::NTile { .. } => "NTILE",
        TypedFunction::Lead { .. } => "LEAD",
        TypedFunction::Lag { .. } => "LAG",
        TypedFunction::FirstValue { .. } => "FIRST_VALUE",
        TypedFunction::LastValue { .. } => "LAST_VALUE",
        TypedFunction::Abs { .. } => "ABS",
        TypedFunction::Ceil { .. } => "CEIL",
        TypedFunction::Floor { .. } => "FLOOR",
        TypedFunction::Round { .. } => "ROUND",
        TypedFunction::Log { .. } => "LOG",
        TypedFunction::Ln { .. } => "LN",
        TypedFunction::Pow { .. } => "POW",
        TypedFunction::Sqrt { .. } => "SQRT",
        TypedFunction::Greatest { .. } => "GREATEST",
        TypedFunction::Least { .. } => "LEAST",
        TypedFunction::Mod { .. } => "MOD",
        TypedFunction::Hex { .. } => "HEX",
        TypedFunction::Unhex { .. } => "UNHEX",
        TypedFunction::Md5 { .. } => "MD5",
        TypedFunction::Sha { .. } => "SHA",
        TypedFunction::Sha2 { .. } => "SHA2",
    }
}

/// Extract the argument expressions from a TypedFunction (in positional order).
fn typed_function_args(func: &TypedFunction) -> Vec<Expr> {
    match func {
        TypedFunction::CurrentDate | TypedFunction::CurrentTimestamp => vec![],
        TypedFunction::RowNumber | TypedFunction::Rank | TypedFunction::DenseRank => vec![],
        TypedFunction::Length { expr }
        | TypedFunction::Upper { expr }
        | TypedFunction::Lower { expr }
        | TypedFunction::Initcap { expr }
        | TypedFunction::Reverse { expr }
        | TypedFunction::Abs { expr }
        | TypedFunction::Ceil { expr }
        | TypedFunction::Floor { expr }
        | TypedFunction::Ln { expr }
        | TypedFunction::Sqrt { expr }
        | TypedFunction::Explode { expr }
        | TypedFunction::Flatten { expr }
        | TypedFunction::ArraySize { expr }
        | TypedFunction::ParseJSON { expr }
        | TypedFunction::JSONFormat { expr }
        | TypedFunction::Hex { expr }
        | TypedFunction::Unhex { expr }
        | TypedFunction::Md5 { expr }
        | TypedFunction::Sha { expr }
        | TypedFunction::TsOrDsToDate { expr }
        | TypedFunction::Year { expr }
        | TypedFunction::Month { expr }
        | TypedFunction::Day { expr }
        | TypedFunction::ApproxDistinct { expr }
        | TypedFunction::Variance { expr }
        | TypedFunction::Stddev { expr }
        | TypedFunction::FirstValue { expr }
        | TypedFunction::LastValue { expr } => vec![*expr.clone()],
        TypedFunction::DateTrunc { unit, expr } => {
            vec![Expr::StringLiteral(format!("{unit:?}")), *expr.clone()]
        }
        TypedFunction::DateAdd { expr, interval, .. }
        | TypedFunction::DateSub { expr, interval, .. } => {
            vec![*expr.clone(), *interval.clone()]
        }
        TypedFunction::DateDiff { start, end, .. } => vec![*start.clone(), *end.clone()],
        TypedFunction::StrToTime { expr, format } | TypedFunction::TimeToStr { expr, format } => {
            vec![*expr.clone(), *format.clone()]
        }
        TypedFunction::Trim { expr, .. } => vec![*expr.clone()],
        TypedFunction::Substring {
            expr,
            start,
            length,
        } => {
            let mut args = vec![*expr.clone(), *start.clone()];
            if let Some(len) = length {
                args.push(*len.clone());
            }
            args
        }
        TypedFunction::RegexpLike {
            expr,
            pattern,
            flags,
        } => {
            let mut args = vec![*expr.clone(), *pattern.clone()];
            if let Some(f) = flags {
                args.push(*f.clone());
            }
            args
        }
        TypedFunction::RegexpExtract {
            expr,
            pattern,
            group_index,
        } => {
            let mut args = vec![*expr.clone(), *pattern.clone()];
            if let Some(g) = group_index {
                args.push(*g.clone());
            }
            args
        }
        TypedFunction::RegexpReplace {
            expr,
            pattern,
            replacement,
            flags,
        } => {
            let mut args = vec![*expr.clone(), *pattern.clone(), *replacement.clone()];
            if let Some(f) = flags {
                args.push(*f.clone());
            }
            args
        }
        TypedFunction::ConcatWs { separator, exprs } => {
            let mut args = vec![*separator.clone()];
            args.extend(exprs.iter().cloned());
            args
        }
        TypedFunction::Split { expr, delimiter } => vec![*expr.clone(), *delimiter.clone()],
        TypedFunction::Replace { expr, from, to } => {
            vec![*expr.clone(), *from.clone(), *to.clone()]
        }
        TypedFunction::Left { expr, n } | TypedFunction::Right { expr, n } => {
            vec![*expr.clone(), *n.clone()]
        }
        TypedFunction::Lpad { expr, length, pad } | TypedFunction::Rpad { expr, length, pad } => {
            let mut args = vec![*expr.clone(), *length.clone()];
            if let Some(p) = pad {
                args.push(*p.clone());
            }
            args
        }
        TypedFunction::Count { expr, .. }
        | TypedFunction::Sum { expr, .. }
        | TypedFunction::Avg { expr, .. }
        | TypedFunction::Min { expr }
        | TypedFunction::Max { expr }
        | TypedFunction::ArrayAgg { expr, .. } => vec![*expr.clone()],
        TypedFunction::ArrayConcat { arrays } => arrays.clone(),
        TypedFunction::ArrayContains { array, element } => {
            vec![*array.clone(), *element.clone()]
        }
        TypedFunction::GenerateSeries { start, stop, step } => {
            let mut args = vec![*start.clone(), *stop.clone()];
            if let Some(s) = step {
                args.push(*s.clone());
            }
            args
        }
        TypedFunction::JSONExtract { expr, path }
        | TypedFunction::JSONExtractScalar { expr, path } => {
            vec![*expr.clone(), *path.clone()]
        }
        TypedFunction::NTile { n } => vec![*n.clone()],
        TypedFunction::Lead {
            expr,
            offset,
            default,
        }
        | TypedFunction::Lag {
            expr,
            offset,
            default,
        } => {
            let mut args = vec![*expr.clone()];
            if let Some(o) = offset {
                args.push(*o.clone());
            }
            if let Some(d) = default {
                args.push(*d.clone());
            }
            args
        }
        TypedFunction::Round { expr, decimals } => {
            let mut args = vec![*expr.clone()];
            if let Some(d) = decimals {
                args.push(*d.clone());
            }
            args
        }
        TypedFunction::Log { expr, base } => {
            let mut args = vec![*expr.clone()];
            if let Some(b) = base {
                args.push(*b.clone());
            }
            args
        }
        TypedFunction::Pow { base, exponent } => vec![*base.clone(), *exponent.clone()],
        TypedFunction::Greatest { exprs } | TypedFunction::Least { exprs } => exprs.clone(),
        TypedFunction::Mod { left, right } => vec![*left.clone(), *right.clone()],
        TypedFunction::Sha2 { expr, bit_length } => vec![*expr.clone(), *bit_length.clone()],
        TypedFunction::GroupConcat {
            exprs, separator, ..
        } => {
            let mut args = exprs.clone();
            if let Some(s) = separator {
                args.push(*s.clone());
            }
            args
        }
    }
}

/// Transform a statement from one dialect to another, supporting custom
/// dialect plugins.
///
/// For built-in → built-in transforms this delegates to the existing
/// [`super::transform`]. When either side is a custom dialect the plugin's
/// transform hooks are applied.
#[must_use]
pub fn transform(statement: &Statement, from: &DialectRef, to: &DialectRef) -> Statement {
    // Fast path: both built-in → use existing logic
    if let (DialectRef::BuiltIn(f), DialectRef::BuiltIn(t)) = (from, to) {
        return super::transform(statement, *f, *t);
    }

    // If the target is a custom dialect with a full statement transform, try that first.
    if let Some(plugin) = to.as_plugin()
        && let Some(transformed) = plugin.transform_statement(statement)
    {
        return transformed;
    }

    // Otherwise apply expression-level transforms via DialectRef helpers.
    let mut stmt = statement.clone();
    transform_statement_plugin(&mut stmt, to);
    stmt
}

/// Recursively transform a statement using plugin-aware rules.
fn transform_statement_plugin(statement: &mut Statement, target: &DialectRef) {
    match statement {
        Statement::Select(sel) => {
            for item in &mut sel.columns {
                if let crate::ast::SelectItem::Expr { expr, .. } = item {
                    *expr = transform_expr_plugin(expr.clone(), target);
                }
            }
            if let Some(wh) = &mut sel.where_clause {
                *wh = transform_expr_plugin(wh.clone(), target);
            }
            for gb in &mut sel.group_by {
                *gb = transform_expr_plugin(gb.clone(), target);
            }
            if let Some(having) = &mut sel.having {
                *having = transform_expr_plugin(having.clone(), target);
            }
        }
        Statement::CreateTable(ct) => {
            for col in &mut ct.columns {
                col.data_type = target.map_data_type(&col.data_type);
                if let Some(default) = &mut col.default {
                    *default = transform_expr_plugin(default.clone(), target);
                }
            }
        }
        _ => {}
    }
}

/// Transform an expression using plugin-aware rules.
fn transform_expr_plugin(expr: Expr, target: &DialectRef) -> Expr {
    // Let the plugin have first shot at transforming the whole expression
    if let Some(plugin) = target.as_plugin()
        && let Some(transformed) = plugin.transform_expr(&expr)
    {
        return transformed;
    }

    match expr {
        // For TypedFunction, check if the plugin wants to rename the function.
        // If so, convert it to a plain Expr::Function with the new name.
        Expr::TypedFunction { func, filter, over } => {
            if let DialectRef::Custom(_) = target {
                let canonical = typed_function_canonical_name(&func);
                let new_name = target.map_function_name(canonical);
                if new_name != canonical {
                    // Plugin wants to rename this function — convert to Expr::Function
                    let args = typed_function_args(&func)
                        .into_iter()
                        .map(|a| transform_expr_plugin(a, target))
                        .collect();
                    return Expr::Function {
                        name: new_name,
                        args,
                        distinct: false,
                        filter: filter.map(|f| Box::new(transform_expr_plugin(*f, target))),
                        over,
                    };
                }
            }
            // No rename — recurse into children
            Expr::TypedFunction {
                func: func.transform_children(&|e| transform_expr_plugin(e, target)),
                filter: filter.map(|f| Box::new(transform_expr_plugin(*f, target))),
                over,
            }
        }
        Expr::Function {
            name,
            args,
            distinct,
            filter,
            over,
        } => {
            let new_name = target.map_function_name(&name);
            let new_args: Vec<Expr> = args
                .into_iter()
                .map(|a| transform_expr_plugin(a, target))
                .collect();
            Expr::Function {
                name: new_name,
                args: new_args,
                distinct,
                filter: filter.map(|f| Box::new(transform_expr_plugin(*f, target))),
                over,
            }
        }
        Expr::Cast { expr, data_type } => Expr::Cast {
            expr: Box::new(transform_expr_plugin(*expr, target)),
            data_type: target.map_data_type(&data_type),
        },
        Expr::ILike {
            expr,
            pattern,
            negated,
            escape,
        } if !target.supports_ilike() => Expr::Like {
            expr: Box::new(Expr::TypedFunction {
                func: crate::ast::TypedFunction::Lower {
                    expr: Box::new(transform_expr_plugin(*expr, target)),
                },
                filter: None,
                over: None,
            }),
            pattern: Box::new(Expr::TypedFunction {
                func: crate::ast::TypedFunction::Lower {
                    expr: Box::new(transform_expr_plugin(*pattern, target)),
                },
                filter: None,
                over: None,
            }),
            negated,
            escape,
        },
        Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
            left: Box::new(transform_expr_plugin(*left, target)),
            op,
            right: Box::new(transform_expr_plugin(*right, target)),
        },
        Expr::UnaryOp { op, expr } => Expr::UnaryOp {
            op,
            expr: Box::new(transform_expr_plugin(*expr, target)),
        },
        Expr::Nested(inner) => Expr::Nested(Box::new(transform_expr_plugin(*inner, target))),
        Expr::Column {
            table,
            name,
            quote_style,
            table_quote_style,
        } => {
            let new_qs = if quote_style.is_quoted() {
                target.quote_style()
            } else {
                QuoteStyle::None
            };
            let new_tqs = if table_quote_style.is_quoted() {
                target.quote_style()
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
        other => other,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Plugin-aware top-level API
// ═══════════════════════════════════════════════════════════════════════════

use crate::errors;

/// Transpile a SQL string using [`DialectRef`], supporting custom plugins.
///
/// # Example
///
/// ```rust
/// use sqlglot_rust::dialects::plugin::{DialectRef, transpile_ext};
/// use sqlglot_rust::Dialect;
///
/// let result = transpile_ext(
///     "SELECT NOW()",
///     &DialectRef::from(Dialect::Postgres),
///     &DialectRef::from(Dialect::Mysql),
/// ).unwrap();
/// ```
///
/// # Errors
///
/// Returns a [`SqlglotError`](crate::errors::SqlglotError) if parsing fails.
pub fn transpile_ext(
    sql: &str,
    read_dialect: &DialectRef,
    write_dialect: &DialectRef,
) -> errors::Result<String> {
    // Parse using the read dialect (fall back to Ansi for custom dialects)
    let parse_dialect = read_dialect.as_builtin().unwrap_or(Dialect::Ansi);
    let ast = crate::parser::parse(sql, parse_dialect)?;
    let transformed = transform(&ast, read_dialect, write_dialect);
    let gen_dialect = write_dialect.as_builtin().unwrap_or(Dialect::Ansi);
    Ok(crate::generator::generate(&transformed, gen_dialect))
}

/// Transpile multiple statements using [`DialectRef`], supporting custom plugins.
///
/// # Errors
///
/// Returns a [`SqlglotError`](crate::errors::SqlglotError) if parsing fails.
pub fn transpile_statements_ext(
    sql: &str,
    read_dialect: &DialectRef,
    write_dialect: &DialectRef,
) -> errors::Result<Vec<String>> {
    let parse_dialect = read_dialect.as_builtin().unwrap_or(Dialect::Ansi);
    let stmts = crate::parser::parse_statements(sql, parse_dialect)?;
    let gen_dialect = write_dialect.as_builtin().unwrap_or(Dialect::Ansi);
    let mut results = Vec::with_capacity(stmts.len());
    for stmt in &stmts {
        let transformed = transform(stmt, read_dialect, write_dialect);
        results.push(crate::generator::generate(&transformed, gen_dialect));
    }
    Ok(results)
}

// ═══════════════════════════════════════════════════════════════════════════
// Convenience registration functions
// ═══════════════════════════════════════════════════════════════════════════

/// Register a custom dialect plugin in the global registry.
///
/// This is a convenience wrapper around [`DialectRegistry::global().register()`].
pub fn register_dialect<P: DialectPlugin + 'static>(plugin: P) {
    DialectRegistry::global().register(plugin);
}

/// Look up a dialect by name, returning either a built-in or custom [`DialectRef`].
///
/// Checks built-in dialects first, then the custom plugin registry.
#[must_use]
pub fn resolve_dialect(name: &str) -> Option<DialectRef> {
    // Try built-in first
    if let Some(d) = Dialect::from_str(name) {
        return Some(DialectRef::BuiltIn(d));
    }
    // Then try plugin registry
    if DialectRegistry::global().get(name).is_some() {
        return Some(DialectRef::Custom(name.to_lowercase()));
    }
    None
}
