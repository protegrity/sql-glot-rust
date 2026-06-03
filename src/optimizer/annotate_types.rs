//! Type annotation pass for SQL expressions.
//!
//! Infers and propagates SQL data types across AST nodes using schema metadata.
//! Inspired by Python sqlglot's `annotate_types` optimizer pass.
//!
//! # Overview
//!
//! The pass walks the AST bottom-up, resolving types for:
//! - **Literals**: `42` → `Int`, `'hello'` → `Varchar`, `TRUE` → `Boolean`
//! - **Column references**: looked up from the provided [`Schema`]
//! - **Binary operators**: result type from operand coercion (e.g. `INT + FLOAT → FLOAT`)
//! - **CAST / TRY_CAST**: the target data type
//! - **Functions**: return type based on function signature and argument types
//! - **CASE**: common type across all THEN / ELSE branches
//! - **Aggregates**: `COUNT → BigInt`, `SUM` depends on input, etc.
//! - **Subqueries**: type of the single output column
//!
//! # Example
//!
//! ```rust
//! use sqlglot_rust::optimizer::annotate_types::annotate_types;
//! use sqlglot_rust::schema::{MappingSchema, Schema};
//! use sqlglot_rust::ast::DataType;
//! use sqlglot_rust::{parse, Dialect};
//!
//! let mut schema = MappingSchema::new(Dialect::Ansi);
//! schema.add_table(&["t"], vec![
//!     ("id".to_string(), DataType::Int),
//!     ("name".to_string(), DataType::Varchar(Some(255))),
//! ]).unwrap();
//!
//! let stmt = parse("SELECT id, name FROM t WHERE id > 1", Dialect::Ansi).unwrap();
//! let annotations = annotate_types(&stmt, &schema);
//! // annotations now contains inferred types for every expression node
//! ```

use std::collections::HashMap;

use crate::ast::*;
use crate::schema::Schema;

// ═══════════════════════════════════════════════════════════════════════
// TypeAnnotations — the result of type inference
// ═══════════════════════════════════════════════════════════════════════

/// Stores inferred [`DataType`] annotations for expression nodes in an AST.
///
/// Annotations are keyed by raw pointer identity, so this structure is valid
/// only as long as the underlying AST is not moved, cloned, or dropped.
/// Intended for single-pass analysis over a borrowed AST.
pub struct TypeAnnotations {
    types: HashMap<*const Expr, DataType>,
}

// Raw pointers are not Send/Sync by default, but our usage is safe because
// the pointers are derived from shared references with a known lifetime.
unsafe impl Send for TypeAnnotations {}
unsafe impl Sync for TypeAnnotations {}

impl TypeAnnotations {
    fn new() -> Self {
        Self {
            types: HashMap::new(),
        }
    }

    fn set(&mut self, expr: &Expr, dt: DataType) {
        self.types.insert(expr as *const Expr, dt);
    }

    /// Retrieve the inferred type of an expression, if annotated.
    #[must_use]
    pub fn get_type(&self, expr: &Expr) -> Option<&DataType> {
        self.types.get(&(expr as *const Expr))
    }

    /// Number of annotated nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Returns `true` if no annotations were recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Public entry point
// ═══════════════════════════════════════════════════════════════════════

/// Annotate all expression nodes in a statement with inferred SQL types.
///
/// Walks the AST bottom-up, resolving types from literals, schema column
/// lookups, operator/function signatures, and type coercion rules.
///
/// The returned [`TypeAnnotations`] is valid only while the borrowed `stmt`
/// is alive and unmodified.
#[must_use]
pub fn annotate_types<S: Schema>(stmt: &Statement, schema: &S) -> TypeAnnotations {
    let mut ann = TypeAnnotations::new();
    let mut ctx = AnnotationContext::new(schema);
    annotate_statement(stmt, &mut ctx, &mut ann);
    ann
}

// ═══════════════════════════════════════════════════════════════════════
// Internal context
// ═══════════════════════════════════════════════════════════════════════

/// Carries schema reference and table alias mappings through the walk.
struct AnnotationContext<'s, S: Schema> {
    schema: &'s S,
    /// Maps table alias or name → table path for column type lookups.
    table_aliases: HashMap<String, Vec<String>>,
}

impl<'s, S: Schema> AnnotationContext<'s, S> {
    fn new(schema: &'s S) -> Self {
        Self {
            schema,
            table_aliases: HashMap::new(),
        }
    }

    /// Register a table (by ref) so that columns can be looked up by alias.
    fn register_table(&mut self, table_ref: &TableRef) {
        let path = vec![table_ref.name.clone()];
        let alias = table_ref
            .alias
            .as_deref()
            .unwrap_or(&table_ref.name)
            .to_string();
        self.table_aliases.insert(alias, path);
    }

    /// Look up the type of a column, resolving through table aliases.
    fn resolve_column_type(&self, table: Option<&str>, column: &str) -> Option<DataType> {
        if let Some(tbl) = table {
            // Qualified column — look up via alias map
            if let Some(path) = self.table_aliases.get(tbl) {
                let path_refs: Vec<&str> = path.iter().map(String::as_str).collect();
                return self.schema.get_column_type(&path_refs, column).ok();
            }
            // Try the table name directly
            return self.schema.get_column_type(&[tbl], column).ok();
        }
        // Unqualified — search all registered tables
        for path in self.table_aliases.values() {
            let path_refs: Vec<&str> = path.iter().map(String::as_str).collect();
            if let Ok(dt) = self.schema.get_column_type(&path_refs, column) {
                return Some(dt);
            }
        }
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Statement-level annotation
// ═══════════════════════════════════════════════════════════════════════

fn annotate_statement<S: Schema>(
    stmt: &Statement,
    ctx: &mut AnnotationContext<S>,
    ann: &mut TypeAnnotations,
) {
    match stmt {
        Statement::Select(sel) => annotate_select(sel, ctx, ann),
        Statement::SetOperation(set_op) => {
            annotate_statement(&set_op.left, ctx, ann);
            annotate_statement(&set_op.right, ctx, ann);
        }
        Statement::Insert(ins) => {
            if let InsertSource::Query(q) = &ins.source {
                annotate_statement(q, ctx, ann);
            }
            for row in match &ins.source {
                InsertSource::Values(rows) => rows.as_slice(),
                _ => &[],
            } {
                for expr in row {
                    annotate_expr(expr, ctx, ann);
                }
            }
        }
        Statement::Update(upd) => {
            for (_, expr) in &upd.assignments {
                annotate_expr(expr, ctx, ann);
            }
            if let Some(wh) = &upd.where_clause {
                annotate_expr(wh, ctx, ann);
            }
        }
        Statement::Delete(del) => {
            if let Some(wh) = &del.where_clause {
                annotate_expr(wh, ctx, ann);
            }
        }
        Statement::Expression(expr) => {
            annotate_expr(expr, ctx, ann);
        }
        Statement::Explain(expl) => {
            annotate_statement(&expl.statement, ctx, ann);
        }
        // DDL / transaction / other statements — no expression types to annotate
        _ => {}
    }
}

fn annotate_select<S: Schema>(
    sel: &SelectStatement,
    ctx: &mut AnnotationContext<S>,
    ann: &mut TypeAnnotations,
) {
    // 1. Register CTEs
    for cte in &sel.ctes {
        annotate_statement(&cte.query, ctx, ann);
    }

    // 2. Register FROM sources
    if let Some(from) = &sel.from {
        register_table_source(&from.source, ctx);
    }
    for join in &sel.joins {
        register_table_source(&join.table, ctx);
    }

    // 3. Annotate WHERE clause
    if let Some(wh) = &sel.where_clause {
        annotate_expr(wh, ctx, ann);
    }

    // 4. Annotate SELECT columns
    for item in &sel.columns {
        if let SelectItem::Expr { expr, .. } = item {
            annotate_expr(expr, ctx, ann);
        }
    }

    // 5. Annotate GROUP BY
    for expr in &sel.group_by {
        annotate_expr(expr, ctx, ann);
    }

    // 6. Annotate HAVING
    if let Some(having) = &sel.having {
        annotate_expr(having, ctx, ann);
    }

    // 7. Annotate ORDER BY
    for ob in &sel.order_by {
        annotate_expr(&ob.expr, ctx, ann);
    }

    // 8. Annotate LIMIT / OFFSET
    if let Some(limit) = &sel.limit {
        annotate_expr(limit, ctx, ann);
    }
    if let Some(offset) = &sel.offset {
        annotate_expr(offset, ctx, ann);
    }
    if let Some(fetch) = &sel.fetch_first {
        annotate_expr(fetch, ctx, ann);
    }

    // 9. Annotate QUALIFY
    if let Some(qualify) = &sel.qualify {
        annotate_expr(qualify, ctx, ann);
    }

    // 10. Annotate JOIN ON conditions
    for join in &sel.joins {
        if let Some(on) = &join.on {
            annotate_expr(on, ctx, ann);
        }
    }
}

fn register_table_source<S: Schema>(source: &TableSource, ctx: &mut AnnotationContext<S>) {
    match source {
        TableSource::Table(tref) => ctx.register_table(tref),
        TableSource::Subquery { alias, .. } => {
            // Subqueries as sources don't have schema entries to register.
            // Their output column types would come from recursive annotation.
            let _ = alias;
        }
        TableSource::TableFunction { alias, .. } => {
            let _ = alias;
        }
        TableSource::Lateral { source } => register_table_source(source, ctx),
        TableSource::Pivot { source, .. } | TableSource::Unpivot { source, .. } => {
            register_table_source(source, ctx);
        }
        TableSource::Unnest { .. } => {}
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Expression-level annotation (bottom-up)
// ═══════════════════════════════════════════════════════════════════════

fn annotate_expr<S: Schema>(expr: &Expr, ctx: &AnnotationContext<S>, ann: &mut TypeAnnotations) {
    // First annotate children, then determine this node's type.
    annotate_children(expr, ctx, ann);

    let dt = infer_type(expr, ctx, ann);
    if let Some(t) = dt {
        ann.set(expr, t);
    }
}

/// Recursively annotate child expressions before the parent.
fn annotate_children<S: Schema>(
    expr: &Expr,
    ctx: &AnnotationContext<S>,
    ann: &mut TypeAnnotations,
) {
    match expr {
        Expr::BinaryOp { left, right, .. } => {
            annotate_expr(left, ctx, ann);
            annotate_expr(right, ctx, ann);
        }
        Expr::UnaryOp { expr: inner, .. } => annotate_expr(inner, ctx, ann),
        Expr::Function { args, filter, .. } => {
            for arg in args {
                annotate_expr(arg, ctx, ann);
            }
            if let Some(f) = filter {
                annotate_expr(f, ctx, ann);
            }
        }
        Expr::Between {
            expr: e, low, high, ..
        } => {
            annotate_expr(e, ctx, ann);
            annotate_expr(low, ctx, ann);
            annotate_expr(high, ctx, ann);
        }
        Expr::InList { expr: e, list, .. } => {
            annotate_expr(e, ctx, ann);
            for item in list {
                annotate_expr(item, ctx, ann);
            }
        }
        Expr::InSubquery {
            expr: e, subquery, ..
        } => {
            annotate_expr(e, ctx, ann);
            let mut sub_ctx = AnnotationContext::new(ctx.schema);
            annotate_statement(subquery, &mut sub_ctx, ann);
        }
        Expr::IsNull { expr: e, .. } | Expr::IsBool { expr: e, .. } => {
            annotate_expr(e, ctx, ann);
        }
        Expr::Like {
            expr: e,
            pattern,
            escape,
            ..
        }
        | Expr::ILike {
            expr: e,
            pattern,
            escape,
            ..
        } => {
            annotate_expr(e, ctx, ann);
            annotate_expr(pattern, ctx, ann);
            if let Some(esc) = escape {
                annotate_expr(esc, ctx, ann);
            }
        }
        Expr::Case {
            operand,
            when_clauses,
            else_clause,
        } => {
            if let Some(op) = operand {
                annotate_expr(op, ctx, ann);
            }
            for (cond, result) in when_clauses {
                annotate_expr(cond, ctx, ann);
                annotate_expr(result, ctx, ann);
            }
            if let Some(el) = else_clause {
                annotate_expr(el, ctx, ann);
            }
        }
        Expr::Nested(inner) => annotate_expr(inner, ctx, ann),
        Expr::Cast { expr: e, .. } | Expr::TryCast { expr: e, .. } => {
            annotate_expr(e, ctx, ann);
        }
        Expr::Extract { expr: e, .. } => annotate_expr(e, ctx, ann),
        Expr::Interval { value, .. } => annotate_expr(value, ctx, ann),
        Expr::ArrayLiteral(items) | Expr::Tuple(items) | Expr::Coalesce(items) => {
            for item in items {
                annotate_expr(item, ctx, ann);
            }
        }
        Expr::If {
            condition,
            true_val,
            false_val,
        } => {
            annotate_expr(condition, ctx, ann);
            annotate_expr(true_val, ctx, ann);
            if let Some(fv) = false_val {
                annotate_expr(fv, ctx, ann);
            }
        }
        Expr::NullIf { expr: e, r#else } => {
            annotate_expr(e, ctx, ann);
            annotate_expr(r#else, ctx, ann);
        }
        Expr::Collate { expr: e, .. } => annotate_expr(e, ctx, ann),
        Expr::Alias { expr: e, .. } => annotate_expr(e, ctx, ann),
        Expr::ArrayIndex { expr: e, index } => {
            annotate_expr(e, ctx, ann);
            annotate_expr(index, ctx, ann);
        }
        Expr::JsonAccess { expr: e, path, .. } => {
            annotate_expr(e, ctx, ann);
            annotate_expr(path, ctx, ann);
        }
        Expr::Lambda { body, .. } => annotate_expr(body, ctx, ann),
        Expr::AnyOp { expr: e, right, .. } | Expr::AllOp { expr: e, right, .. } => {
            annotate_expr(e, ctx, ann);
            annotate_expr(right, ctx, ann);
        }
        Expr::Subquery(sub) => {
            let mut sub_ctx = AnnotationContext::new(ctx.schema);
            annotate_statement(sub, &mut sub_ctx, ann);
        }
        Expr::Exists { subquery, .. } => {
            let mut sub_ctx = AnnotationContext::new(ctx.schema);
            annotate_statement(subquery, &mut sub_ctx, ann);
        }
        Expr::TypedFunction { func, filter, .. } => {
            annotate_typed_function_children(func, ctx, ann);
            if let Some(f) = filter {
                annotate_expr(f, ctx, ann);
            }
        }
        Expr::Cube { exprs } | Expr::Rollup { exprs } | Expr::GroupingSets { sets: exprs } => {
            for item in exprs {
                annotate_expr(item, ctx, ann);
            }
        }
        // Leaf nodes — no children to annotate
        Expr::Column { .. }
        | Expr::Number(_)
        | Expr::StringLiteral(_)
        | Expr::NationalStringLiteral(_)
        | Expr::Boolean(_)
        | Expr::Null
        | Expr::Wildcard
        | Expr::Star
        | Expr::Parameter(_)
        | Expr::TypeExpr(_)
        | Expr::QualifiedWildcard { .. }
        | Expr::Default
        | Expr::Commented { .. }
        | Expr::SimilarTo { .. } => {}
    }
}

/// Annotate children of a TypedFunction.
fn annotate_typed_function_children<S: Schema>(
    func: &TypedFunction,
    ctx: &AnnotationContext<S>,
    ann: &mut TypeAnnotations,
) {
    // Use walk_children to visit all child expressions and annotate each
    func.walk_children(&mut |child| {
        annotate_expr(child, ctx, ann);
        true
    });
}

// ═══════════════════════════════════════════════════════════════════════
// Type inference for a single expression node
// ═══════════════════════════════════════════════════════════════════════

fn infer_type<S: Schema>(
    expr: &Expr,
    ctx: &AnnotationContext<S>,
    ann: &TypeAnnotations,
) -> Option<DataType> {
    match expr {
        // ── Literals ───────────────────────────────────────────────────
        Expr::Number(s) => Some(infer_number_type(s)),
        Expr::StringLiteral(_) | Expr::NationalStringLiteral(_) => Some(DataType::Varchar(None)),
        Expr::Boolean(_) => Some(DataType::Boolean),
        Expr::Null => Some(DataType::Null),

        // ── Column reference ──────────────────────────────────────────
        Expr::Column { table, name, .. } => ctx.resolve_column_type(table.as_deref(), name),

        // ── Binary operators ──────────────────────────────────────────
        Expr::BinaryOp { left, op, right } => {
            infer_binary_op_type(op, ann.get_type(left), ann.get_type(right))
        }

        // ── Unary operators ───────────────────────────────────────────
        Expr::UnaryOp { op, expr: inner } => match op {
            UnaryOperator::Not => Some(DataType::Boolean),
            UnaryOperator::Minus | UnaryOperator::Plus => ann.get_type(inner).cloned(),
            UnaryOperator::BitwiseNot => ann.get_type(inner).cloned(),
        },

        // ── CAST / TRY_CAST ──────────────────────────────────────────
        Expr::Cast { data_type, .. } | Expr::TryCast { data_type, .. } => Some(data_type.clone()),

        // ── CASE expression ──────────────────────────────────────────
        Expr::Case {
            when_clauses,
            else_clause,
            ..
        } => {
            let mut result_types: Vec<&DataType> = Vec::new();
            for (_, result) in when_clauses {
                if let Some(t) = ann.get_type(result) {
                    result_types.push(t);
                }
            }
            if let Some(el) = else_clause {
                if let Some(t) = ann.get_type(el.as_ref()) {
                    result_types.push(t);
                }
            }
            common_type(&result_types)
        }

        // ── IF expression ────────────────────────────────────────────
        Expr::If {
            true_val,
            false_val,
            ..
        } => {
            let mut types = Vec::new();
            if let Some(t) = ann.get_type(true_val) {
                types.push(t);
            }
            if let Some(fv) = false_val {
                if let Some(t) = ann.get_type(fv.as_ref()) {
                    types.push(t);
                }
            }
            common_type(&types)
        }

        // ── COALESCE ─────────────────────────────────────────────────
        Expr::Coalesce(items) => {
            let types: Vec<&DataType> = items.iter().filter_map(|e| ann.get_type(e)).collect();
            common_type(&types)
        }

        // ── NULLIF ───────────────────────────────────────────────────
        Expr::NullIf { expr: e, .. } => ann.get_type(e.as_ref()).cloned(),

        // ── Generic function ─────────────────────────────────────────
        Expr::Function { name, args, .. } => infer_generic_function_type(name, args, ctx, ann),

        // ── Typed functions ──────────────────────────────────────────
        Expr::TypedFunction { func, .. } => infer_typed_function_type(func, ann),

        // ── Subquery (scalar) ────────────────────────────────────────
        Expr::Subquery(sub) => infer_subquery_type(sub, ann),

        // ── EXISTS → Boolean ─────────────────────────────────────────
        Expr::Exists { .. } => Some(DataType::Boolean),

        // ── Boolean predicates ───────────────────────────────────────
        Expr::Between { .. }
        | Expr::InList { .. }
        | Expr::InSubquery { .. }
        | Expr::IsNull { .. }
        | Expr::IsBool { .. }
        | Expr::Like { .. }
        | Expr::ILike { .. }
        | Expr::AnyOp { .. }
        | Expr::AllOp { .. } => Some(DataType::Boolean),

        // ── EXTRACT → numeric ────────────────────────────────────────
        Expr::Extract { .. } => Some(DataType::Int),

        // ── INTERVAL → Interval ──────────────────────────────────────
        Expr::Interval { .. } => Some(DataType::Interval),

        // ── Array literal ────────────────────────────────────────────
        Expr::ArrayLiteral(items) => {
            let elem_types: Vec<&DataType> = items.iter().filter_map(|e| ann.get_type(e)).collect();
            let elem = common_type(&elem_types);
            Some(DataType::Array(elem.map(Box::new)))
        }

        // ── Tuple ────────────────────────────────────────────────────
        Expr::Tuple(items) => {
            let types: Vec<DataType> = items
                .iter()
                .map(|e| ann.get_type(e).cloned().unwrap_or(DataType::Null))
                .collect();
            Some(DataType::Tuple(types))
        }

        // ── Array index → element type ───────────────────────────────
        Expr::ArrayIndex { expr: e, .. } => match ann.get_type(e.as_ref()) {
            Some(DataType::Array(Some(elem))) => Some(elem.as_ref().clone()),
            _ => None,
        },

        // ── JSON access ──────────────────────────────────────────────
        Expr::JsonAccess { as_text, .. } => {
            if *as_text {
                Some(DataType::Text)
            } else {
                Some(DataType::Json)
            }
        }

        // ── Nested / Alias — pass through ────────────────────────────
        Expr::Nested(inner) => ann.get_type(inner.as_ref()).cloned(),
        Expr::Alias { expr: e, .. } => ann.get_type(e.as_ref()).cloned(),

        // ── Collate → Varchar ────────────────────────────────────────
        Expr::Collate { .. } => Some(DataType::Varchar(None)),

        // ── TypeExpr ─────────────────────────────────────────────────
        Expr::TypeExpr(dt) => Some(dt.clone()),

        // ── Others — no type ─────────────────────────────────────────
        Expr::Wildcard
        | Expr::Star
        | Expr::QualifiedWildcard { .. }
        | Expr::Parameter(_)
        | Expr::Lambda { .. }
        | Expr::Default
        | Expr::Cube { .. }
        | Expr::Rollup { .. }
        | Expr::GroupingSets { .. }
        | Expr::SimilarTo { .. }
        | Expr::Commented { .. } => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Number type inference
// ═══════════════════════════════════════════════════════════════════════

fn infer_number_type(s: &str) -> DataType {
    if s.contains('.') || s.contains('e') || s.contains('E') {
        DataType::Double
    } else if let Ok(v) = s.parse::<i64>() {
        if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
            DataType::Int
        } else {
            DataType::BigInt
        }
    } else {
        // Very large numbers or special formats
        DataType::BigInt
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Binary operator type inference
// ═══════════════════════════════════════════════════════════════════════

fn infer_binary_op_type(
    op: &BinaryOperator,
    left: Option<&DataType>,
    right: Option<&DataType>,
) -> Option<DataType> {
    use BinaryOperator::*;
    match op {
        // Comparison operators → Boolean
        Eq | Neq | Lt | Gt | LtEq | GtEq | AtArrow | ArrowAt => Some(DataType::Boolean),

        // Logical operators → Boolean
        And | Or | Xor => Some(DataType::Boolean),

        // String concatenation → Varchar
        Concat => Some(DataType::Varchar(None)),

        // Arithmetic → coerce operand types
        Plus | Minus | Multiply | Divide | Modulo => match (left, right) {
            (Some(l), Some(r)) => Some(coerce_numeric(l, r)),
            (Some(l), None) => Some(l.clone()),
            (None, Some(r)) => Some(r.clone()),
            (None, None) => None,
        },

        // Bitwise → integer type
        BitwiseAnd | BitwiseOr | BitwiseXor | ShiftLeft | ShiftRight => match (left, right) {
            (Some(l), Some(r)) => Some(coerce_numeric(l, r)),
            (Some(l), None) => Some(l.clone()),
            (None, Some(r)) => Some(r.clone()),
            (None, None) => Some(DataType::Int),
        },

        // JSON operators
        Arrow => Some(DataType::Json),
        DoubleArrow => Some(DataType::Text),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Generic (untyped) function return type inference
// ═══════════════════════════════════════════════════════════════════════

fn infer_generic_function_type<S: Schema>(
    name: &str,
    args: &[Expr],
    ctx: &AnnotationContext<S>,
    ann: &TypeAnnotations,
) -> Option<DataType> {
    let upper = name.to_uppercase();
    match upper.as_str() {
        // Aggregate functions
        "COUNT" | "COUNT_BIG" => Some(DataType::BigInt),
        "SUM" => args
            .first()
            .and_then(|a| ann.get_type(a))
            .map(|t| coerce_sum_type(t)),
        "AVG" => Some(DataType::Double),
        "MIN" | "MAX" => args.first().and_then(|a| ann.get_type(a)).cloned(),
        "VARIANCE" | "VAR_SAMP" | "VAR_POP" | "STDDEV" | "STDDEV_SAMP" | "STDDEV_POP" => {
            Some(DataType::Double)
        }
        "APPROX_COUNT_DISTINCT" | "APPROX_DISTINCT" => Some(DataType::BigInt),

        // String functions
        "CONCAT" | "UPPER" | "LOWER" | "TRIM" | "LTRIM" | "RTRIM" | "LPAD" | "RPAD" | "REPLACE"
        | "REVERSE" | "SUBSTRING" | "SUBSTR" | "LEFT" | "RIGHT" | "INITCAP" | "REPEAT"
        | "TRANSLATE" | "FORMAT" | "CONCAT_WS" | "SPACE" | "REPLICATE" => {
            Some(DataType::Varchar(None))
        }
        "LENGTH" | "LEN" | "CHAR_LENGTH" | "CHARACTER_LENGTH" | "OCTET_LENGTH" | "BIT_LENGTH" => {
            Some(DataType::Int)
        }
        "POSITION" | "STRPOS" | "LOCATE" | "INSTR" | "CHARINDEX" => Some(DataType::Int),
        "ASCII" => Some(DataType::Int),
        "CHR" | "CHAR" => Some(DataType::Varchar(Some(1))),

        // Math functions
        "ABS" | "CEIL" | "CEILING" | "FLOOR" => args.first().and_then(|a| ann.get_type(a)).cloned(),
        "ROUND" | "TRUNCATE" | "TRUNC" => args.first().and_then(|a| ann.get_type(a)).cloned(),
        "SQRT" | "LN" | "LOG" | "LOG2" | "LOG10" | "EXP" | "POWER" | "POW" | "ACOS" | "ASIN"
        | "ATAN" | "ATAN2" | "COS" | "SIN" | "TAN" | "COT" | "DEGREES" | "RADIANS" | "PI"
        | "SIGN" => Some(DataType::Double),
        "MOD" => {
            match (
                args.first().and_then(|a| ann.get_type(a)),
                args.get(1).and_then(|a| ann.get_type(a)),
            ) {
                (Some(l), Some(r)) => Some(coerce_numeric(l, r)),
                (Some(l), _) => Some(l.clone()),
                (_, Some(r)) => Some(r.clone()),
                _ => Some(DataType::Int),
            }
        }
        "GREATEST" | "LEAST" => {
            let types: Vec<&DataType> = args.iter().filter_map(|a| ann.get_type(a)).collect();
            common_type(&types)
        }
        "RANDOM" | "RAND" => Some(DataType::Double),

        // Date/Time functions
        "CURRENT_DATE" | "CURDATE" | "TODAY" => Some(DataType::Date),
        "CURRENT_TIMESTAMP" | "NOW" | "GETDATE" | "SYSDATE" | "SYSTIMESTAMP" | "LOCALTIMESTAMP" => {
            Some(DataType::Timestamp {
                precision: None,
                with_tz: false,
            })
        }
        "CURRENT_TIME" | "CURTIME" => Some(DataType::Time { precision: None }),
        "DATE" | "TO_DATE" | "DATE_TRUNC" | "DATE_ADD" | "DATE_SUB" | "DATEADD" | "DATESUB"
        | "ADDDATE" | "SUBDATE" => Some(DataType::Date),
        "TIMESTAMP" | "TO_TIMESTAMP" => Some(DataType::Timestamp {
            precision: None,
            with_tz: false,
        }),
        "YEAR" | "MONTH" | "DAY" | "DAYOFWEEK" | "DAYOFYEAR" | "HOUR" | "MINUTE" | "SECOND"
        | "QUARTER" | "WEEK" | "EXTRACT" | "DATEDIFF" | "TIMESTAMPDIFF" | "MONTHS_BETWEEN" => {
            Some(DataType::Int)
        }

        // Type conversion
        "CAST" | "TRY_CAST" | "SAFE_CAST" | "CONVERT" => None, // handled by Expr::Cast

        // Boolean functions
        "COALESCE" => {
            let types: Vec<&DataType> = args.iter().filter_map(|a| ann.get_type(a)).collect();
            common_type(&types)
        }
        "NULLIF" => args.first().and_then(|a| ann.get_type(a)).cloned(),
        "IF" | "IIF" => {
            // IF(cond, true_val, false_val) — type from true_val
            args.get(1).and_then(|a| ann.get_type(a)).cloned()
        }
        "IFNULL" | "NVL" | "ISNULL" => {
            let types: Vec<&DataType> = args.iter().filter_map(|a| ann.get_type(a)).collect();
            common_type(&types)
        }

        // JSON functions
        "JSON_EXTRACT" | "JSON_QUERY" | "GET_JSON_OBJECT" => Some(DataType::Json),
        "JSON_EXTRACT_SCALAR" | "JSON_VALUE" | "JSON_EXTRACT_PATH_TEXT" => {
            Some(DataType::Varchar(None))
        }
        "TO_JSON" | "JSON_OBJECT" | "JSON_ARRAY" | "JSON_BUILD_OBJECT" | "JSON_BUILD_ARRAY" => {
            Some(DataType::Json)
        }
        "PARSE_JSON" | "JSON_PARSE" | "JSON" => Some(DataType::Json),

        // Array functions
        "ARRAY_AGG" | "COLLECT_LIST" | "COLLECT_SET" => {
            let elem = args.first().and_then(|a| ann.get_type(a)).cloned();
            Some(DataType::Array(elem.map(Box::new)))
        }
        "ARRAY_LENGTH" | "ARRAY_SIZE" | "CARDINALITY" => Some(DataType::Int),
        "ARRAY" | "ARRAY_CONSTRUCT" => {
            let types: Vec<&DataType> = args.iter().filter_map(|a| ann.get_type(a)).collect();
            let elem = common_type(&types);
            Some(DataType::Array(elem.map(Box::new)))
        }
        "ARRAY_CONTAINS" | "ARRAY_POSITION" => Some(DataType::Boolean),

        // Window ranking
        "ROW_NUMBER" | "RANK" | "DENSE_RANK" | "NTILE" | "CUME_DIST" | "PERCENT_RANK" => {
            Some(DataType::BigInt)
        }

        // Hash / crypto
        "MD5" | "SHA1" | "SHA" | "SHA2" | "SHA256" | "SHA512" => Some(DataType::Varchar(None)),
        "HEX" | "TO_HEX" => Some(DataType::Varchar(None)),
        "UNHEX" | "FROM_HEX" => Some(DataType::Varbinary(None)),
        "CRC32" | "HASH" => Some(DataType::BigInt),

        // Type checking
        "TYPEOF" | "TYPE_OF" => Some(DataType::Varchar(None)),

        // UDFs — check schema
        _ => ctx.schema.get_udf_type(&upper).cloned(),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TypedFunction return type inference
// ═══════════════════════════════════════════════════════════════════════

fn infer_typed_function_type(func: &TypedFunction, ann: &TypeAnnotations) -> Option<DataType> {
    match func {
        // ── Date/Time → Date or Timestamp ────────────────────────────
        TypedFunction::DateAdd { .. }
        | TypedFunction::DateSub { .. }
        | TypedFunction::DateTrunc { .. }
        | TypedFunction::TsOrDsToDate { .. } => Some(DataType::Date),
        TypedFunction::DateDiff { .. } => Some(DataType::Int),
        TypedFunction::CurrentDate => Some(DataType::Date),
        TypedFunction::CurrentTime => Some(DataType::Time { precision: None }),
        TypedFunction::CurrentTimestamp => Some(DataType::Timestamp {
            precision: None,
            with_tz: false,
        }),
        TypedFunction::StrToTime { .. } => Some(DataType::Timestamp {
            precision: None,
            with_tz: false,
        }),
        TypedFunction::TimeToStr { .. } => Some(DataType::Varchar(None)),
        TypedFunction::Year { .. } | TypedFunction::Month { .. } | TypedFunction::Day { .. } => {
            Some(DataType::Int)
        }

        // ── String → Varchar ─────────────────────────────────────────
        TypedFunction::Trim { .. }
        | TypedFunction::Substring { .. }
        | TypedFunction::Upper { .. }
        | TypedFunction::Lower { .. }
        | TypedFunction::Initcap { .. }
        | TypedFunction::Replace { .. }
        | TypedFunction::Reverse { .. }
        | TypedFunction::Left { .. }
        | TypedFunction::Right { .. }
        | TypedFunction::Lpad { .. }
        | TypedFunction::Rpad { .. }
        | TypedFunction::ConcatWs { .. } => Some(DataType::Varchar(None)),
        TypedFunction::Length { .. } => Some(DataType::Int),
        TypedFunction::RegexpLike { .. } => Some(DataType::Boolean),
        TypedFunction::RegexpExtract { .. } => Some(DataType::Varchar(None)),
        TypedFunction::RegexpReplace { .. } => Some(DataType::Varchar(None)),
        TypedFunction::Split { .. } => {
            Some(DataType::Array(Some(Box::new(DataType::Varchar(None)))))
        }

        // ── Aggregates ───────────────────────────────────────────────
        TypedFunction::Count { .. } => Some(DataType::BigInt),
        TypedFunction::Sum { expr, .. } => ann.get_type(expr.as_ref()).map(|t| coerce_sum_type(t)),
        TypedFunction::Avg { .. } => Some(DataType::Double),
        TypedFunction::Min { expr } | TypedFunction::Max { expr } => {
            ann.get_type(expr.as_ref()).cloned()
        }
        TypedFunction::ArrayAgg { expr, .. } => {
            let elem = ann.get_type(expr.as_ref()).cloned();
            Some(DataType::Array(elem.map(Box::new)))
        }
        TypedFunction::ApproxDistinct { .. } => Some(DataType::BigInt),
        TypedFunction::Variance { .. } | TypedFunction::Stddev { .. } => Some(DataType::Double),
        TypedFunction::GroupConcat { .. } => Some(DataType::Varchar(None)),

        // ── Array ────────────────────────────────────────────────────
        TypedFunction::ArrayConcat { arrays } => {
            // Type is the same as input arrays
            arrays.first().and_then(|a| ann.get_type(a)).cloned()
        }
        TypedFunction::ArrayContains { .. } => Some(DataType::Boolean),
        TypedFunction::ArraySize { .. } => Some(DataType::Int),
        TypedFunction::Explode { expr } => {
            // Unwrap array element type
            match ann.get_type(expr.as_ref()) {
                Some(DataType::Array(Some(elem))) => Some(elem.as_ref().clone()),
                _ => None,
            }
        }
        TypedFunction::GenerateSeries { .. } => Some(DataType::Int),
        TypedFunction::Flatten { expr } => ann.get_type(expr.as_ref()).cloned(),

        // ── JSON ─────────────────────────────────────────────────────
        TypedFunction::JSONExtract { .. } => Some(DataType::Json),
        TypedFunction::JSONExtractScalar { .. } => Some(DataType::Varchar(None)),
        TypedFunction::ParseJSON { .. } | TypedFunction::JSONFormat { .. } => Some(DataType::Json),

        // ── Window ───────────────────────────────────────────────────
        TypedFunction::RowNumber | TypedFunction::Rank | TypedFunction::DenseRank => {
            Some(DataType::BigInt)
        }
        TypedFunction::NTile { .. } => Some(DataType::BigInt),
        TypedFunction::Lead { expr, .. }
        | TypedFunction::Lag { expr, .. }
        | TypedFunction::FirstValue { expr }
        | TypedFunction::LastValue { expr } => ann.get_type(expr.as_ref()).cloned(),

        // ── Math ─────────────────────────────────────────────────────
        TypedFunction::Abs { expr }
        | TypedFunction::Ceil { expr }
        | TypedFunction::Floor { expr } => ann.get_type(expr.as_ref()).cloned(),
        TypedFunction::Round { expr, .. } => ann.get_type(expr.as_ref()).cloned(),
        TypedFunction::Log { .. }
        | TypedFunction::Ln { .. }
        | TypedFunction::Pow { .. }
        | TypedFunction::Sqrt { .. } => Some(DataType::Double),
        TypedFunction::Greatest { exprs } | TypedFunction::Least { exprs } => {
            let types: Vec<&DataType> = exprs.iter().filter_map(|e| ann.get_type(e)).collect();
            common_type(&types)
        }
        TypedFunction::Mod { left, right } => {
            match (ann.get_type(left.as_ref()), ann.get_type(right.as_ref())) {
                (Some(l), Some(r)) => Some(coerce_numeric(l, r)),
                (Some(l), _) => Some(l.clone()),
                (_, Some(r)) => Some(r.clone()),
                _ => Some(DataType::Int),
            }
        }

        // ── Conversion ───────────────────────────────────────────────
        TypedFunction::Hex { .. } | TypedFunction::Md5 { .. } | TypedFunction::Sha { .. } => {
            Some(DataType::Varchar(None))
        }
        TypedFunction::Sha2 { .. } => Some(DataType::Varchar(None)),
        TypedFunction::Unhex { .. } => Some(DataType::Varbinary(None)),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Subquery type inference
// ═══════════════════════════════════════════════════════════════════════

fn infer_subquery_type(sub: &Statement, ann: &TypeAnnotations) -> Option<DataType> {
    // The type of a scalar subquery is the type of its single output column
    if let Statement::Select(sel) = sub {
        if let Some(SelectItem::Expr { expr, .. }) = sel.columns.first() {
            return ann.get_type(expr).cloned();
        }
    }
    None
}

// ═══════════════════════════════════════════════════════════════════════
// Type coercion helpers
// ═══════════════════════════════════════════════════════════════════════

/// Numeric type widening precedence (higher = wider).
fn numeric_precedence(dt: &DataType) -> u8 {
    match dt {
        DataType::Boolean => 1,
        DataType::TinyInt => 2,
        DataType::SmallInt => 3,
        DataType::Int | DataType::Serial => 4,
        DataType::BigInt | DataType::BigSerial => 5,
        DataType::Real | DataType::Float => 6,
        DataType::Double => 7,
        DataType::Decimal { .. } | DataType::Numeric { .. } => 8,
        _ => 0,
    }
}

/// Coerce two numeric types to their common wider type.
fn coerce_numeric(left: &DataType, right: &DataType) -> DataType {
    let lp = numeric_precedence(left);
    let rp = numeric_precedence(right);
    if lp == 0 && rp == 0 {
        // Neither is numeric — fall back to left
        return left.clone();
    }
    if lp >= rp {
        left.clone()
    } else {
        right.clone()
    }
}

/// Determine the return type of SUM based on input type.
fn coerce_sum_type(input: &DataType) -> DataType {
    match input {
        DataType::TinyInt | DataType::SmallInt | DataType::Int | DataType::BigInt => {
            DataType::BigInt
        }
        DataType::Float | DataType::Real => DataType::Double,
        DataType::Double => DataType::Double,
        DataType::Decimal { precision, scale } => DataType::Decimal {
            precision: *precision,
            scale: *scale,
        },
        DataType::Numeric { precision, scale } => DataType::Numeric {
            precision: *precision,
            scale: *scale,
        },
        _ => DataType::BigInt,
    }
}

/// Find the common (widest) type among a set of types.
fn common_type(types: &[&DataType]) -> Option<DataType> {
    if types.is_empty() {
        return None;
    }
    let mut result = types[0];
    for t in &types[1..] {
        // Skip NULL — it doesn't contribute to the common type
        if **t == DataType::Null {
            continue;
        }
        if *result == DataType::Null {
            result = t;
            continue;
        }
        // If both are numeric, pick the wider one
        let lp = numeric_precedence(result);
        let rp = numeric_precedence(t);
        if lp > 0 && rp > 0 {
            if rp > lp {
                result = t;
            }
            continue;
        }
        // If both are string-like, prefer VARCHAR
        if is_string_type(result) && is_string_type(t) {
            result = if matches!(result, DataType::Text) || matches!(t, DataType::Text) {
                if matches!(result, DataType::Text) {
                    result
                } else {
                    t
                }
            } else {
                result // keep first
            };
            continue;
        }
        // Otherwise keep the first non-null type
    }
    Some(result.clone())
}

fn is_string_type(dt: &DataType) -> bool {
    matches!(
        dt,
        DataType::Varchar(_) | DataType::Char(_) | DataType::Text | DataType::String
    )
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialects::Dialect;
    use crate::parser::Parser;
    use crate::schema::{MappingSchema, Schema};

    fn setup_schema() -> MappingSchema {
        let mut schema = MappingSchema::new(Dialect::Ansi);
        schema
            .add_table(
                &["users"],
                vec![
                    ("id".to_string(), DataType::Int),
                    ("name".to_string(), DataType::Varchar(Some(255))),
                    ("age".to_string(), DataType::Int),
                    ("salary".to_string(), DataType::Double),
                    ("active".to_string(), DataType::Boolean),
                    (
                        "created_at".to_string(),
                        DataType::Timestamp {
                            precision: None,
                            with_tz: false,
                        },
                    ),
                ],
            )
            .unwrap();
        schema
            .add_table(
                &["orders"],
                vec![
                    ("id".to_string(), DataType::Int),
                    ("user_id".to_string(), DataType::Int),
                    (
                        "amount".to_string(),
                        DataType::Decimal {
                            precision: Some(10),
                            scale: Some(2),
                        },
                    ),
                    ("status".to_string(), DataType::Varchar(Some(50))),
                ],
            )
            .unwrap();
        schema
    }

    fn parse_and_annotate(sql: &str, schema: &MappingSchema) -> (Statement, TypeAnnotations) {
        let stmt = Parser::new(sql).unwrap().parse_statement().unwrap();
        let ann = annotate_types(&stmt, schema);
        (stmt, ann)
    }

    /// Helper: get the type of the first SELECT column
    fn first_col_type(stmt: &Statement, ann: &TypeAnnotations) -> Option<DataType> {
        if let Statement::Select(sel) = stmt {
            if let Some(SelectItem::Expr { expr, .. }) = sel.columns.first() {
                return ann.get_type(expr).cloned();
            }
        }
        None
    }

    // ── Literal type inference ────────────────────────────────────────

    #[test]
    fn test_number_literal_int() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT 42", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Int));
    }

    #[test]
    fn test_number_literal_big_int() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT 9999999999", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::BigInt));
    }

    #[test]
    fn test_number_literal_double() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT 3.14", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Double));
    }

    #[test]
    fn test_string_literal() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT 'hello'", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Varchar(None)));
    }

    #[test]
    fn test_boolean_literal() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT TRUE", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Boolean));
    }

    #[test]
    fn test_null_literal() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT NULL", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Null));
    }

    // ── Column reference type lookup ─────────────────────────────────

    #[test]
    fn test_column_type_from_schema() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT id FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Int));
    }

    #[test]
    fn test_qualified_column_type() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT users.name FROM users", &schema);
        assert_eq!(
            first_col_type(&stmt, &ann),
            Some(DataType::Varchar(Some(255)))
        );
    }

    #[test]
    fn test_aliased_table_column_type() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT u.salary FROM users AS u", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Double));
    }

    // ── Binary operator type inference ───────────────────────────────

    #[test]
    fn test_int_plus_int() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT id + age FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Int));
    }

    #[test]
    fn test_int_plus_double() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT id + salary FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Double));
    }

    #[test]
    fn test_comparison_returns_boolean() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT id > 5 FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Boolean));
    }

    #[test]
    fn test_and_returns_boolean() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT id > 5 AND age < 30 FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Boolean));
    }

    // ── CAST type inference ──────────────────────────────────────────

    #[test]
    fn test_cast_type() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT CAST(id AS BIGINT) FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::BigInt));
    }

    #[test]
    fn test_cast_to_varchar() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT CAST(id AS VARCHAR) FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Varchar(None)));
    }

    // ── CASE expression ──────────────────────────────────────────────

    #[test]
    fn test_case_expression_type() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate(
            "SELECT CASE WHEN id > 1 THEN salary ELSE 0.0 END FROM users",
            &schema,
        );
        let t = first_col_type(&stmt, &ann);
        assert!(
            matches!(t, Some(DataType::Double)),
            "Expected Double, got {t:?}"
        );
    }

    // ── Function return types ────────────────────────────────────────

    #[test]
    fn test_count_returns_bigint() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT COUNT(*) FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::BigInt));
    }

    #[test]
    fn test_sum_returns_bigint_for_int() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT SUM(id) FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::BigInt));
    }

    #[test]
    fn test_avg_returns_double() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT AVG(age) FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Double));
    }

    #[test]
    fn test_min_preserves_type() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT MIN(salary) FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Double));
    }

    #[test]
    fn test_upper_returns_varchar() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT UPPER(name) FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Varchar(None)));
    }

    #[test]
    fn test_length_returns_int() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT LENGTH(name) FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Int));
    }

    // ── Predicate types ──────────────────────────────────────────────

    #[test]
    fn test_between_returns_boolean() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT age BETWEEN 18 AND 65 FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Boolean));
    }

    #[test]
    fn test_in_list_returns_boolean() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT id IN (1, 2, 3) FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Boolean));
    }

    #[test]
    fn test_is_null_returns_boolean() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT name IS NULL FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Boolean));
    }

    #[test]
    fn test_like_returns_boolean() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT name LIKE '%test%' FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Boolean));
    }

    // ── Exists ───────────────────────────────────────────────────────

    #[test]
    fn test_exists_returns_boolean() {
        let schema = setup_schema();
        let (stmt, ann) =
            parse_and_annotate("SELECT EXISTS (SELECT 1 FROM orders) FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Boolean));
    }

    // ── Nested expressions ───────────────────────────────────────────

    #[test]
    fn test_nested_expression_propagation() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT (id + age) * salary FROM users", &schema);
        let t = first_col_type(&stmt, &ann);
        // INT + INT = INT, INT * DOUBLE = DOUBLE
        assert!(
            matches!(t, Some(DataType::Double)),
            "Expected Double, got {t:?}"
        );
    }

    // ── EXTRACT ──────────────────────────────────────────────────────

    #[test]
    fn test_extract_returns_int() {
        let schema = setup_schema();
        let (stmt, ann) =
            parse_and_annotate("SELECT EXTRACT(YEAR FROM created_at) FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Int));
    }

    // ── Multiple columns ─────────────────────────────────────────────

    #[test]
    fn test_multiple_columns_annotated() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT id, name, salary FROM users", &schema);
        if let Statement::Select(sel) = &stmt {
            assert_eq!(sel.columns.len(), 3);
            // id → Int
            if let SelectItem::Expr { expr, .. } = &sel.columns[0] {
                assert_eq!(ann.get_type(expr), Some(&DataType::Int));
            }
            // name → Varchar(255)
            if let SelectItem::Expr { expr, .. } = &sel.columns[1] {
                assert_eq!(ann.get_type(expr), Some(&DataType::Varchar(Some(255))));
            }
            // salary → Double
            if let SelectItem::Expr { expr, .. } = &sel.columns[2] {
                assert_eq!(ann.get_type(expr), Some(&DataType::Double));
            }
        }
    }

    // ── WHERE clause annotation ──────────────────────────────────────

    #[test]
    fn test_where_clause_annotated() {
        let schema = setup_schema();
        // Don't move stmt after annotation — raw pointers for inline fields
        // (like where_clause: Option<Expr>) are invalidated on move.
        let stmt = Parser::new("SELECT id FROM users WHERE age > 21")
            .unwrap()
            .parse_statement()
            .unwrap();
        let ann = annotate_types(&stmt, &schema);
        if let Statement::Select(sel) = &stmt {
            if let Some(wh) = &sel.where_clause {
                assert_eq!(ann.get_type(wh), Some(&DataType::Boolean));
            }
        }
    }

    // ── Coercion rules ──────────────────────────────────────────────

    #[test]
    fn test_int_and_bigint_coercion() {
        assert_eq!(
            coerce_numeric(&DataType::Int, &DataType::BigInt),
            DataType::BigInt
        );
    }

    #[test]
    fn test_float_and_double_coercion() {
        assert_eq!(
            coerce_numeric(&DataType::Float, &DataType::Double),
            DataType::Double
        );
    }

    #[test]
    fn test_int_and_double_coercion() {
        assert_eq!(
            coerce_numeric(&DataType::Int, &DataType::Double),
            DataType::Double
        );
    }

    // ── Common type ─────────────────────────────────────────────────

    #[test]
    fn test_common_type_nulls_skipped() {
        let types = vec![&DataType::Null, &DataType::Int, &DataType::Null];
        assert_eq!(common_type(&types), Some(DataType::Int));
    }

    #[test]
    fn test_common_type_numeric_widening() {
        let types = vec![&DataType::Int, &DataType::Double, &DataType::Float];
        assert_eq!(common_type(&types), Some(DataType::Double));
    }

    #[test]
    fn test_common_type_empty() {
        let types: Vec<&DataType> = vec![];
        assert_eq!(common_type(&types), None);
    }

    // ── UDF type support ─────────────────────────────────────────────

    #[test]
    fn test_udf_return_type() {
        let mut schema = setup_schema();
        schema.add_udf("my_func", DataType::Varchar(None));
        let (stmt, ann) = parse_and_annotate("SELECT my_func(id) FROM users", &schema);
        assert_eq!(first_col_type(&stmt, &ann), Some(DataType::Varchar(None)));
    }

    // ── Annotation count ─────────────────────────────────────────────

    #[test]
    fn test_annotations_not_empty() {
        let schema = setup_schema();
        let (_, ann) = parse_and_annotate("SELECT id, name FROM users WHERE age > 21", &schema);
        assert!(!ann.is_empty());
        // Should have at least the SELECT columns and WHERE predicate
        assert!(ann.len() >= 3);
    }

    // ── SUM of DECIMAL preserves precision ───────────────────────────

    #[test]
    fn test_sum_decimal_preserves_type() {
        let schema = setup_schema();
        let (stmt, ann) = parse_and_annotate("SELECT SUM(amount) FROM orders", &schema);
        assert_eq!(
            first_col_type(&stmt, &ann),
            Some(DataType::Decimal {
                precision: Some(10),
                scale: Some(2)
            })
        );
    }
}
