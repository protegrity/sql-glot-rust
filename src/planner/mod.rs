//! Logical query planner.
//!
//! Generates a logical execution plan (a DAG of [`Step`]s) from an
//! optimized SQL AST. Inspired by Python sqlglot's `planner.py`.
//!
//! The planner sits between the optimizer and the executor: the optimizer
//! rewrites the AST, then the planner produces a plan that an execution
//! engine can follow.
//!
//! # Example
//!
//! ```rust
//! use sqlglot_rust::parser::parse;
//! use sqlglot_rust::dialects::Dialect;
//! use sqlglot_rust::planner::{plan, Plan};
//!
//! let ast = parse("SELECT a, b FROM t WHERE a > 1 ORDER BY b", Dialect::Ansi).unwrap();
//! let p = plan(&ast).unwrap();
//! println!("{}", p.to_mermaid());
//! ```

use std::fmt;

use crate::ast::*;
use crate::errors::{Result, SqlglotError};

// ═══════════════════════════════════════════════════════════════════════
// Step ID
// ═══════════════════════════════════════════════════════════════════════

/// Opaque identifier for a step within a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StepId(usize);

impl fmt::Display for StepId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "step_{}", self.0)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Column projection
// ═══════════════════════════════════════════════════════════════════════

/// A projected column in a plan step.
#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    /// The expression being projected.
    pub expr: Expr,
    /// Output alias (if any).
    pub alias: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════
// Plan step types
// ═══════════════════════════════════════════════════════════════════════

/// A single step in the logical execution plan.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Full table scan with optional filter pushdown.
    Scan {
        /// Fully-qualified table name.
        table: String,
        /// Alias for the table (if any).
        alias: Option<String>,
        /// Projected columns.
        projections: Vec<Projection>,
        /// Predicate pushed down to the scan.
        predicate: Option<Expr>,
        /// IDs of steps this step depends on (always empty for a scan).
        dependencies: Vec<StepId>,
    },
    /// Filter (WHERE / HAVING) applied to its input.
    Filter {
        /// The filter predicate.
        predicate: Expr,
        /// Projected columns.
        projections: Vec<Projection>,
        /// The single input step.
        dependencies: Vec<StepId>,
    },
    /// Projection (SELECT list evaluation).
    Project {
        /// Output projections.
        projections: Vec<Projection>,
        /// The single input step.
        dependencies: Vec<StepId>,
    },
    /// Aggregation (GROUP BY + aggregate functions).
    Aggregate {
        /// GROUP BY keys.
        group_by: Vec<Expr>,
        /// Aggregate expressions (COUNT, SUM, etc.).
        aggregations: Vec<Projection>,
        /// Projected output columns.
        projections: Vec<Projection>,
        /// The single input step.
        dependencies: Vec<StepId>,
    },
    /// Sort (ORDER BY).
    Sort {
        /// Order-by items.
        order_by: Vec<OrderByItem>,
        /// Projected columns (pass-through).
        projections: Vec<Projection>,
        /// The single input step.
        dependencies: Vec<StepId>,
    },
    /// Join two inputs.
    Join {
        /// Type of join.
        join_type: JoinType,
        /// Join condition (ON clause).
        condition: Option<Expr>,
        /// USING columns (if specified instead of ON).
        using_columns: Vec<String>,
        /// Projected columns.
        projections: Vec<Projection>,
        /// Two input steps: [left, right].
        dependencies: Vec<StepId>,
    },
    /// LIMIT / OFFSET.
    Limit {
        /// Row limit.
        limit: Option<Expr>,
        /// Row offset.
        offset: Option<Expr>,
        /// Projected columns (pass-through).
        projections: Vec<Projection>,
        /// The single input step.
        dependencies: Vec<StepId>,
    },
    /// UNION / INTERSECT / EXCEPT.
    SetOperation {
        /// The kind of set operation.
        op: SetOperationType,
        /// Whether ALL (no deduplication).
        all: bool,
        /// Projected columns from the combined result.
        projections: Vec<Projection>,
        /// Two input steps: [left, right].
        dependencies: Vec<StepId>,
    },
    /// DISTINCT elimination.
    Distinct {
        /// Projected columns.
        projections: Vec<Projection>,
        /// The single input step.
        dependencies: Vec<StepId>,
    },
}

impl Step {
    /// Returns the list of step IDs this step depends on.
    #[must_use]
    pub fn dependencies(&self) -> &[StepId] {
        match self {
            Step::Scan { dependencies, .. }
            | Step::Filter { dependencies, .. }
            | Step::Project { dependencies, .. }
            | Step::Aggregate { dependencies, .. }
            | Step::Sort { dependencies, .. }
            | Step::Join { dependencies, .. }
            | Step::Limit { dependencies, .. }
            | Step::SetOperation { dependencies, .. }
            | Step::Distinct { dependencies, .. } => dependencies,
        }
    }

    /// Returns the projected columns of this step.
    #[must_use]
    pub fn projections(&self) -> &[Projection] {
        match self {
            Step::Scan { projections, .. }
            | Step::Filter { projections, .. }
            | Step::Project { projections, .. }
            | Step::Aggregate { projections, .. }
            | Step::Sort { projections, .. }
            | Step::Join { projections, .. }
            | Step::Limit { projections, .. }
            | Step::SetOperation { projections, .. }
            | Step::Distinct { projections, .. } => projections,
        }
    }

    /// A short human-readable label for the step type.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Step::Scan { .. } => "Scan",
            Step::Filter { .. } => "Filter",
            Step::Project { .. } => "Project",
            Step::Aggregate { .. } => "Aggregate",
            Step::Sort { .. } => "Sort",
            Step::Join { .. } => "Join",
            Step::Limit { .. } => "Limit",
            Step::SetOperation { .. } => "SetOperation",
            Step::Distinct { .. } => "Distinct",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Plan
// ═══════════════════════════════════════════════════════════════════════

/// A logical execution plan — a directed acyclic graph (DAG) of steps.
///
/// Steps are stored in topological order: a step's dependencies always
/// have a smaller [`StepId`] than the step itself.
#[derive(Debug, Clone)]
pub struct Plan {
    /// All steps in topological order.
    steps: Vec<Step>,
    /// The "root" step that produces the final result.
    root: StepId,
}

impl Plan {
    /// Returns the root step ID.
    #[must_use]
    pub fn root(&self) -> StepId {
        self.root
    }

    /// Returns a reference to all steps.
    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Looks up a step by its ID.
    #[must_use]
    pub fn get(&self, id: StepId) -> Option<&Step> {
        self.steps.get(id.0)
    }

    /// Number of steps in the plan.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the plan has zero steps.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Render the plan as a Mermaid flowchart.
    #[must_use]
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("graph TD\n");
        for (i, step) in self.steps.iter().enumerate() {
            let id = StepId(i);
            let label = step_label(step);
            out.push_str(&format!("    {id}[\"{label}\"]\n"));
            for dep in step.dependencies() {
                out.push_str(&format!("    {dep} --> {id}\n"));
            }
        }
        out
    }

    /// Render the plan as a DOT (Graphviz) digraph.
    #[must_use]
    pub fn to_dot(&self) -> String {
        let mut out = String::from("digraph plan {\n    rankdir=BT;\n");
        for (i, step) in self.steps.iter().enumerate() {
            let id = StepId(i);
            let label = step_label(step);
            out.push_str(&format!("    {id} [label=\"{label}\"];\n"));
            for dep in step.dependencies() {
                out.push_str(&format!("    {dep} -> {id};\n"));
            }
        }
        out.push_str("}\n");
        out
    }
}

impl fmt::Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, step) in self.steps.iter().enumerate() {
            let id = StepId(i);
            let root_marker = if id == self.root { " (root)" } else { "" };
            writeln!(f, "{id}{root_marker}: {}", step_label(step))?;
            for dep in step.dependencies() {
                writeln!(f, "  <- {dep}")?;
            }
        }
        Ok(())
    }
}

/// Produce a concise label for visualization.
fn step_label(step: &Step) -> String {
    match step {
        Step::Scan {
            table,
            alias,
            predicate,
            ..
        } => {
            let name = alias.as_deref().unwrap_or(table.as_str());
            if predicate.is_some() {
                format!("Scan({name} + filter)")
            } else {
                format!("Scan({name})")
            }
        }
        Step::Filter { .. } => "Filter".to_string(),
        Step::Project { projections, .. } => {
            let cols: Vec<_> = projections
                .iter()
                .map(|p| {
                    p.alias
                        .as_deref()
                        .unwrap_or_else(|| expr_short_name(&p.expr))
                })
                .collect();
            if cols.len() <= 4 {
                format!("Project({})", cols.join(", "))
            } else {
                format!("Project({} cols)", cols.len())
            }
        }
        Step::Aggregate { group_by, .. } => {
            if group_by.is_empty() {
                "Aggregate(scalar)".to_string()
            } else {
                format!("Aggregate({} keys)", group_by.len())
            }
        }
        Step::Sort { order_by, .. } => format!("Sort({} keys)", order_by.len()),
        Step::Join { join_type, .. } => format!("Join({join_type:?})"),
        Step::Limit { limit, offset, .. } => {
            let mut parts = Vec::new();
            if limit.is_some() {
                parts.push("limit");
            }
            if offset.is_some() {
                parts.push("offset");
            }
            format!("Limit({})", parts.join("+"))
        }
        Step::SetOperation { op, all, .. } => {
            let all_str = if *all { " ALL" } else { "" };
            format!("{op:?}{all_str}")
        }
        Step::Distinct { .. } => "Distinct".to_string(),
    }
}

/// Short name for an expression (used in labels).
fn expr_short_name(expr: &Expr) -> &str {
    match expr {
        Expr::Column { name, .. } => name.as_str(),
        Expr::Wildcard => "*",
        _ => "expr",
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Plan builder
// ═══════════════════════════════════════════════════════════════════════

/// Build a logical execution plan from a parsed SQL statement.
///
/// The statement should ideally be optimized first (via
/// [`crate::optimizer::optimize`]) for the best plan quality, but this
/// is not required.
///
/// # Errors
///
/// Returns [`SqlglotError`] when the statement cannot be planned (e.g.,
/// DDL statements, unsupported constructs).
pub fn plan(statement: &Statement) -> Result<Plan> {
    let mut builder = PlanBuilder::new();
    let _root = builder.plan_statement(statement)?;
    Ok(builder.build())
}

/// Internal builder that accumulates steps.
struct PlanBuilder {
    steps: Vec<Step>,
}

impl PlanBuilder {
    fn new() -> Self {
        Self { steps: Vec::new() }
    }

    fn add_step(&mut self, step: Step) -> StepId {
        let id = StepId(self.steps.len());
        self.steps.push(step);
        id
    }

    fn build(self) -> Plan {
        let root = if self.steps.is_empty() {
            StepId(0)
        } else {
            StepId(self.steps.len() - 1)
        };
        Plan {
            steps: self.steps,
            root,
        }
    }

    // ───────────────────────────────────────────────────────────────
    // Statement dispatch
    // ───────────────────────────────────────────────────────────────

    fn plan_statement(&mut self, stmt: &Statement) -> Result<StepId> {
        match stmt {
            Statement::Select(sel) => self.plan_select(sel),
            Statement::SetOperation(set_op) => self.plan_set_operation(set_op),
            _ => Err(SqlglotError::Internal(format!(
                "Planner does not support {:?} statements",
                std::mem::discriminant(stmt)
            ))),
        }
    }

    // ───────────────────────────────────────────────────────────────
    // SELECT
    // ───────────────────────────────────────────────────────────────

    fn plan_select(&mut self, sel: &SelectStatement) -> Result<StepId> {
        // 1. Resolve FROM source(s)
        let mut current = if let Some(from) = &sel.from {
            self.plan_table_source(&from.source)?
        } else {
            // No FROM — single-row virtual scan (e.g., SELECT 1+2)
            self.add_step(Step::Scan {
                table: String::new(),
                alias: None,
                projections: vec![],
                predicate: None,
                dependencies: vec![],
            })
        };

        // 2. JOINs
        for join in &sel.joins {
            let right = self.plan_table_source(&join.table)?;
            let projections = vec![]; // pass-through
            current = self.add_step(Step::Join {
                join_type: join.join_type.clone(),
                condition: join.on.clone(),
                using_columns: join.using.clone(),
                projections,
                dependencies: vec![current, right],
            });
        }

        // 3. WHERE
        if let Some(pred) = &sel.where_clause {
            current = self.add_step(Step::Filter {
                predicate: pred.clone(),
                projections: vec![],
                dependencies: vec![current],
            });
        }

        // 4. GROUP BY / Aggregation
        if !sel.group_by.is_empty() || has_aggregates(&sel.columns) {
            let aggregations = extract_aggregates(&sel.columns);
            current = self.add_step(Step::Aggregate {
                group_by: sel.group_by.clone(),
                aggregations,
                projections: vec![],
                dependencies: vec![current],
            });
        }

        // 5. HAVING
        if let Some(having) = &sel.having {
            current = self.add_step(Step::Filter {
                predicate: having.clone(),
                projections: vec![],
                dependencies: vec![current],
            });
        }

        // 6. DISTINCT
        if sel.distinct {
            current = self.add_step(Step::Distinct {
                projections: vec![],
                dependencies: vec![current],
            });
        }

        // 7. ORDER BY
        if !sel.order_by.is_empty() {
            current = self.add_step(Step::Sort {
                order_by: sel.order_by.clone(),
                projections: vec![],
                dependencies: vec![current],
            });
        }

        // 8. LIMIT / OFFSET
        if sel.limit.is_some() || sel.offset.is_some() || sel.fetch_first.is_some() {
            let limit = sel.limit.clone().or_else(|| sel.fetch_first.clone());
            current = self.add_step(Step::Limit {
                limit,
                offset: sel.offset.clone(),
                projections: vec![],
                dependencies: vec![current],
            });
        }

        // 9. Project (SELECT columns)
        let projections = select_items_to_projections(&sel.columns);
        if !projections.is_empty() {
            current = self.add_step(Step::Project {
                projections,
                dependencies: vec![current],
            });
        }

        Ok(current)
    }

    // ───────────────────────────────────────────────────────────────
    // Table sources
    // ───────────────────────────────────────────────────────────────

    fn plan_table_source(&mut self, source: &TableSource) -> Result<StepId> {
        match source {
            TableSource::Table(tref) => {
                let table = fully_qualified_name(tref);
                Ok(self.add_step(Step::Scan {
                    table,
                    alias: tref.alias.clone(),
                    projections: vec![],
                    predicate: None,
                    dependencies: vec![],
                }))
            }
            TableSource::Subquery {
                query, alias: _, ..
            } => self.plan_statement(query),
            TableSource::Lateral { source } => self.plan_table_source(source),
            TableSource::TableFunction {
                name, args, alias, ..
            } => Ok(self.add_step(Step::Scan {
                table: name.clone(),
                alias: alias.clone(),
                projections: args
                    .iter()
                    .map(|a| Projection {
                        expr: a.clone(),
                        alias: None,
                    })
                    .collect(),
                predicate: None,
                dependencies: vec![],
            })),
            TableSource::Unnest { expr, alias, .. } => Ok(self.add_step(Step::Scan {
                table: "UNNEST".to_string(),
                alias: alias.clone(),
                projections: vec![Projection {
                    expr: *expr.clone(),
                    alias: None,
                }],
                predicate: None,
                dependencies: vec![],
            })),
            TableSource::Pivot { source, alias, .. }
            | TableSource::Unpivot { source, alias, .. } => {
                // Plan the underlying source; the pivot/unpivot is treated
                // as a virtual scan wrapping it.
                let inner = self.plan_table_source(source)?;
                // For simplicity, wrap pivot/unpivot into a project.
                Ok(self.add_step(Step::Project {
                    projections: vec![Projection {
                        expr: Expr::Wildcard,
                        alias: alias.clone(),
                    }],
                    dependencies: vec![inner],
                }))
            }
        }
    }

    // ───────────────────────────────────────────────────────────────
    // Set operations
    // ───────────────────────────────────────────────────────────────

    fn plan_set_operation(&mut self, set_op: &SetOperationStatement) -> Result<StepId> {
        let left = self.plan_statement(&set_op.left)?;
        let right = self.plan_statement(&set_op.right)?;

        let mut current = self.add_step(Step::SetOperation {
            op: set_op.op.clone(),
            all: set_op.all,
            projections: vec![],
            dependencies: vec![left, right],
        });

        if !set_op.order_by.is_empty() {
            current = self.add_step(Step::Sort {
                order_by: set_op.order_by.clone(),
                projections: vec![],
                dependencies: vec![current],
            });
        }

        if set_op.limit.is_some() || set_op.offset.is_some() {
            current = self.add_step(Step::Limit {
                limit: set_op.limit.clone(),
                offset: set_op.offset.clone(),
                projections: vec![],
                dependencies: vec![current],
            });
        }

        Ok(current)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════

/// Build a fully qualified table name from a [`TableRef`].
fn fully_qualified_name(tref: &TableRef) -> String {
    let mut parts = Vec::new();
    if let Some(catalog) = &tref.catalog {
        parts.push(catalog.as_str());
    }
    if let Some(schema) = &tref.schema {
        parts.push(schema.as_str());
    }
    parts.push(tref.name.as_str());
    parts.join(".")
}

/// Convert SELECT items to projections.
fn select_items_to_projections(items: &[SelectItem]) -> Vec<Projection> {
    items
        .iter()
        .map(|item| match item {
            SelectItem::Wildcard => Projection {
                expr: Expr::Wildcard,
                alias: None,
            },
            SelectItem::QualifiedWildcard { table } => Projection {
                expr: Expr::QualifiedWildcard {
                    table: table.clone(),
                },
                alias: None,
            },
            SelectItem::Expr { expr, alias, .. } => Projection {
                expr: expr.clone(),
                alias: alias.clone(),
            },
        })
        .collect()
}

/// Check whether any SELECT items contain aggregate functions.
fn has_aggregates(items: &[SelectItem]) -> bool {
    items.iter().any(|item| match item {
        SelectItem::Expr { expr, .. } => expr_has_aggregate(expr),
        _ => false,
    })
}

/// Recursively check whether an expression contains an aggregate function.
fn expr_has_aggregate(expr: &Expr) -> bool {
    match expr {
        Expr::Function { name, .. } => is_aggregate_name(name),
        Expr::TypedFunction { func, .. } => typed_function_is_aggregate(func),
        Expr::BinaryOp { left, right, .. } => expr_has_aggregate(left) || expr_has_aggregate(right),
        Expr::UnaryOp { expr, .. } => expr_has_aggregate(expr),
        Expr::Cast { expr, .. } | Expr::TryCast { expr, .. } => expr_has_aggregate(expr),
        Expr::Case {
            operand,
            when_clauses,
            else_clause,
        } => {
            operand.as_ref().is_some_and(|e| expr_has_aggregate(e))
                || when_clauses
                    .iter()
                    .any(|(cond, result)| expr_has_aggregate(cond) || expr_has_aggregate(result))
                || else_clause.as_ref().is_some_and(|e| expr_has_aggregate(e))
        }
        Expr::Alias { expr, .. } => expr_has_aggregate(expr),
        _ => false,
    }
}

/// Well-known aggregate function names.
fn is_aggregate_name(name: &str) -> bool {
    matches!(
        name.to_uppercase().as_str(),
        "COUNT"
            | "SUM"
            | "AVG"
            | "MIN"
            | "MAX"
            | "GROUP_CONCAT"
            | "STRING_AGG"
            | "ARRAY_AGG"
            | "LISTAGG"
            | "COLLECT_LIST"
            | "COLLECT_SET"
            | "ANY_VALUE"
            | "APPROX_COUNT_DISTINCT"
            | "PERCENTILE_CONT"
            | "PERCENTILE_DISC"
            | "STDDEV"
            | "STDDEV_POP"
            | "STDDEV_SAMP"
            | "VARIANCE"
            | "VAR_POP"
            | "VAR_SAMP"
            | "CORR"
            | "COVAR_POP"
            | "COVAR_SAMP"
            | "FIRST_VALUE"
            | "LAST_VALUE"
            | "NTH_VALUE"
            | "BIT_AND"
            | "BIT_OR"
            | "BIT_XOR"
            | "BOOL_AND"
            | "BOOL_OR"
            | "EVERY"
    )
}

/// Check whether a TypedFunction variant is an aggregate.
fn typed_function_is_aggregate(func: &TypedFunction) -> bool {
    matches!(
        func,
        TypedFunction::Count { .. }
            | TypedFunction::Sum { .. }
            | TypedFunction::Avg { .. }
            | TypedFunction::Min { .. }
            | TypedFunction::Max { .. }
            | TypedFunction::ArrayAgg { .. }
            | TypedFunction::ApproxDistinct { .. }
            | TypedFunction::Variance { .. }
            | TypedFunction::Stddev { .. }
            | TypedFunction::GroupConcat { .. }
    )
}

/// Extract aggregation projections from SELECT items.
fn extract_aggregates(items: &[SelectItem]) -> Vec<Projection> {
    let mut aggs = Vec::new();
    for item in items {
        if let SelectItem::Expr { expr, alias, .. } = item {
            collect_aggregates(expr, alias, &mut aggs);
        }
    }
    aggs
}

fn collect_aggregates(expr: &Expr, alias: &Option<String>, out: &mut Vec<Projection>) {
    match expr {
        Expr::Function { name, .. } if is_aggregate_name(name) => {
            out.push(Projection {
                expr: expr.clone(),
                alias: alias.clone(),
            });
        }
        Expr::TypedFunction { func, .. } if typed_function_is_aggregate(func) => {
            out.push(Projection {
                expr: expr.clone(),
                alias: alias.clone(),
            });
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_aggregates(left, &None, out);
            collect_aggregates(right, &None, out);
        }
        Expr::Alias { expr: inner, name } => {
            collect_aggregates(inner, &Some(name.clone()), out);
        }
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialects::Dialect;
    use crate::parser::parse;

    #[test]
    fn test_simple_select() {
        let ast = parse("SELECT a, b FROM t", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        assert!(p.len() >= 2); // Scan + Project
        assert_eq!(p.get(p.root()).unwrap().kind(), "Project");
    }

    #[test]
    fn test_select_with_where() {
        let ast = parse("SELECT a FROM t WHERE a > 1", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        // Scan -> Filter -> Project
        let kinds: Vec<_> = p.steps().iter().map(|s| s.kind()).collect();
        assert!(kinds.contains(&"Scan"));
        assert!(kinds.contains(&"Filter"));
        assert!(kinds.contains(&"Project"));
    }

    #[test]
    fn test_select_with_order_by() {
        let ast = parse("SELECT a FROM t ORDER BY a", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        let kinds: Vec<_> = p.steps().iter().map(|s| s.kind()).collect();
        assert!(kinds.contains(&"Sort"));
    }

    #[test]
    fn test_select_with_group_by() {
        let ast = parse("SELECT a, COUNT(*) FROM t GROUP BY a", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        let kinds: Vec<_> = p.steps().iter().map(|s| s.kind()).collect();
        assert!(kinds.contains(&"Aggregate"));
    }

    #[test]
    fn test_select_with_having() {
        let ast = parse(
            "SELECT a, COUNT(*) FROM t GROUP BY a HAVING COUNT(*) > 1",
            Dialect::Ansi,
        )
        .unwrap();
        let p = plan(&ast).unwrap();
        let kinds: Vec<_> = p.steps().iter().map(|s| s.kind()).collect();
        // Should have Aggregate and a Filter for HAVING
        assert!(kinds.contains(&"Aggregate"));
        assert!(kinds.contains(&"Filter"));
    }

    #[test]
    fn test_join() {
        let ast = parse("SELECT a.x FROM a JOIN b ON a.id = b.id", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        let kinds: Vec<_> = p.steps().iter().map(|s| s.kind()).collect();
        assert!(kinds.contains(&"Join"));
    }

    #[test]
    fn test_multiple_joins() {
        let ast = parse(
            "SELECT a.x FROM a JOIN b ON a.id = b.id JOIN c ON b.id = c.id",
            Dialect::Ansi,
        )
        .unwrap();
        let p = plan(&ast).unwrap();
        let join_count = p.steps().iter().filter(|s| s.kind() == "Join").count();
        assert_eq!(join_count, 2);
    }

    #[test]
    fn test_union() {
        let ast = parse("SELECT a FROM t1 UNION ALL SELECT b FROM t2", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        let kinds: Vec<_> = p.steps().iter().map(|s| s.kind()).collect();
        assert!(kinds.contains(&"SetOperation"));
    }

    #[test]
    fn test_limit_offset() {
        let ast = parse("SELECT a FROM t LIMIT 10 OFFSET 5", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        let kinds: Vec<_> = p.steps().iter().map(|s| s.kind()).collect();
        assert!(kinds.contains(&"Limit"));
    }

    #[test]
    fn test_distinct() {
        let ast = parse("SELECT DISTINCT a FROM t", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        let kinds: Vec<_> = p.steps().iter().map(|s| s.kind()).collect();
        assert!(kinds.contains(&"Distinct"));
    }

    #[test]
    fn test_subquery_in_from() {
        let ast = parse("SELECT x FROM (SELECT a AS x FROM t) sub", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        // Inner scan + inner project + outer project
        assert!(p.len() >= 3);
    }

    #[test]
    fn test_complex_query() {
        let ast = parse(
            "SELECT a, SUM(b) AS total FROM t WHERE c > 0 GROUP BY a HAVING SUM(b) > 10 ORDER BY total DESC LIMIT 5",
            Dialect::Ansi,
        ).unwrap();
        let p = plan(&ast).unwrap();
        let kinds: Vec<_> = p.steps().iter().map(|s| s.kind()).collect();
        assert!(kinds.contains(&"Scan"));
        assert!(kinds.contains(&"Filter")); // WHERE and HAVING
        assert!(kinds.contains(&"Aggregate"));
        assert!(kinds.contains(&"Sort"));
        assert!(kinds.contains(&"Limit"));
        assert!(kinds.contains(&"Project"));
    }

    #[test]
    fn test_dag_dependencies() {
        let ast = parse("SELECT a FROM t1 JOIN t2 ON t1.id = t2.id", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        // Every step's dependencies should reference valid earlier steps
        for (i, step) in p.steps().iter().enumerate() {
            for dep in step.dependencies() {
                assert!(dep.0 < i, "step {i} depends on {dep} which is not earlier");
            }
        }
    }

    #[test]
    fn test_mermaid_output() {
        let ast = parse("SELECT a FROM t WHERE a > 1", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        let mermaid = p.to_mermaid();
        assert!(mermaid.starts_with("graph TD"));
        assert!(mermaid.contains("Scan"));
    }

    #[test]
    fn test_dot_output() {
        let ast = parse("SELECT a FROM t WHERE a > 1", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        let dot = p.to_dot();
        assert!(dot.starts_with("digraph plan"));
        assert!(dot.contains("Scan"));
    }

    #[test]
    fn test_display() {
        let ast = parse("SELECT a FROM t", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        let display = format!("{p}");
        assert!(display.contains("(root)"));
    }

    #[test]
    fn test_ddl_rejected() {
        let ast = parse("CREATE TABLE t (a INT)", Dialect::Ansi).unwrap();
        assert!(plan(&ast).is_err());
    }

    #[test]
    fn test_no_from_select() {
        let ast = parse("SELECT 1 + 2", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        assert!(!p.is_empty());
    }

    #[test]
    fn test_left_join() {
        let ast = parse(
            "SELECT a.x FROM a LEFT JOIN b ON a.id = b.id",
            Dialect::Ansi,
        )
        .unwrap();
        let p = plan(&ast).unwrap();
        let join_step = p.steps().iter().find(|s| s.kind() == "Join").unwrap();
        if let Step::Join { join_type, .. } = join_step {
            assert_eq!(*join_type, JoinType::Left);
        } else {
            panic!("expected Join step");
        }
    }

    #[test]
    fn test_cross_join() {
        let ast = parse("SELECT a.x FROM a CROSS JOIN b", Dialect::Ansi).unwrap();
        let p = plan(&ast).unwrap();
        let join_step = p.steps().iter().find(|s| s.kind() == "Join").unwrap();
        if let Step::Join { join_type, .. } = join_step {
            assert_eq!(*join_type, JoinType::Cross);
        } else {
            panic!("expected Join step");
        }
    }

    #[test]
    fn test_union_with_order_limit() {
        let ast = parse(
            "SELECT a FROM t1 UNION SELECT b FROM t2 ORDER BY 1 LIMIT 10",
            Dialect::Ansi,
        )
        .unwrap();
        let p = plan(&ast).unwrap();
        let kinds: Vec<_> = p.steps().iter().map(|s| s.kind()).collect();
        assert!(kinds.contains(&"SetOperation"));
        assert!(kinds.contains(&"Sort"));
        assert!(kinds.contains(&"Limit"));
    }
}
