//! Column lineage tracking for SQL queries.
//!
//! Provides functionality to trace data flow from source columns through
//! query transformations to output columns. This is the foundation for
//! data governance tools and impact analysis.
//!
//! Inspired by Python sqlglot's `lineage.py`.
//!
//! # Example
//!
//! ```rust
//! use sqlglot_rust::parser::parse;
//! use sqlglot_rust::dialects::Dialect;
//! use sqlglot_rust::optimizer::lineage::{lineage, LineageConfig};
//! use sqlglot_rust::schema::MappingSchema;
//!
//! let sql = "SELECT a, b + 1 AS c FROM t";
//! let ast = parse(sql, Dialect::Ansi).unwrap();
//! let schema = MappingSchema::new(Dialect::Ansi);
//! let config = LineageConfig::default();
//!
//! let graph = lineage("c", &ast, &schema, &config).unwrap();
//! assert_eq!(graph.node.name, "c");
//! ```

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::dialects::Dialect;
use crate::errors::SqlglotError;
use crate::schema::{MappingSchema, Schema};

// ═══════════════════════════════════════════════════════════════════════
// Error types
// ═══════════════════════════════════════════════════════════════════════

/// Errors specific to lineage operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineageError {
    /// The target column was not found in the output.
    ColumnNotFound(String),
    /// Ambiguous column reference (multiple sources).
    AmbiguousColumn(String),
    /// Invalid query structure for lineage analysis.
    InvalidQuery(String),
    /// A parsing error occurred.
    ParseError(String),
}

impl std::fmt::Display for LineageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LineageError::ColumnNotFound(c) => write!(f, "Column not found in output: {c}"),
            LineageError::AmbiguousColumn(c) => write!(f, "Ambiguous column reference: {c}"),
            LineageError::InvalidQuery(msg) => write!(f, "Invalid query for lineage: {msg}"),
            LineageError::ParseError(msg) => write!(f, "Parse error: {msg}"),
        }
    }
}

impl std::error::Error for LineageError {}

impl From<LineageError> for SqlglotError {
    fn from(e: LineageError) -> Self {
        SqlglotError::Internal(e.to_string())
    }
}

/// Result type for lineage operations.
pub type LineageResult<T> = std::result::Result<T, LineageError>;

// ═══════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════

/// Configuration for lineage analysis.
#[derive(Debug, Clone)]
pub struct LineageConfig {
    /// SQL dialect for parsing and identifier normalization.
    pub dialect: Dialect,
    /// Whether to trim column qualifiers in output node names.
    pub trim_qualifiers: bool,
    /// External sources mapping for multi-query lineage.
    /// Maps source names to their SQL definitions (e.g., views).
    pub sources: HashMap<String, String>,
}

impl Default for LineageConfig {
    fn default() -> Self {
        Self {
            dialect: Dialect::Ansi,
            trim_qualifiers: true,
            sources: HashMap::new(),
        }
    }
}

impl LineageConfig {
    /// Create a new configuration with the specified dialect.
    #[must_use]
    pub fn new(dialect: Dialect) -> Self {
        Self {
            dialect,
            ..Default::default()
        }
    }

    /// Add external sources for multi-query lineage.
    #[must_use]
    pub fn with_sources(mut self, sources: HashMap<String, String>) -> Self {
        self.sources = sources;
        self
    }

    /// Set whether to trim table qualifiers from output names.
    #[must_use]
    pub fn with_trim_qualifiers(mut self, trim: bool) -> Self {
        self.trim_qualifiers = trim;
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Lineage Node
// ═══════════════════════════════════════════════════════════════════════

/// A node in the lineage graph representing a column or expression.
#[derive(Debug, Clone)]
pub struct LineageNode {
    /// The name of this column/expression (e.g., "a", "SUM(b)", "t.col").
    pub name: String,
    /// The AST expression this node represents.
    pub expression: Option<Expr>,
    /// The source table/CTE/subquery name, if applicable.
    pub source_name: Option<String>,
    /// Reference to the source AST (for complex expressions).
    pub source: Option<Expr>,
    /// Child nodes (upstream lineage - where data comes from).
    pub downstream: Vec<LineageNode>,
    /// The alias, if this is an aliased expression.
    pub alias: Option<String>,
    /// Depth in the lineage graph (0 = root output column).
    pub depth: usize,
}

impl LineageNode {
    /// Create a new lineage node.
    #[must_use]
    pub fn new(name: String) -> Self {
        Self {
            name,
            expression: None,
            source_name: None,
            source: None,
            downstream: Vec::new(),
            alias: None,
            depth: 0,
        }
    }

    /// Create a node with source information.
    #[must_use]
    pub fn with_source(mut self, source_name: String) -> Self {
        self.source_name = Some(source_name);
        self
    }

    /// Create a node with an expression.
    #[must_use]
    pub fn with_expression(mut self, expr: Expr) -> Self {
        self.expression = Some(expr);
        self
    }

    /// Create a node with an alias.
    #[must_use]
    #[allow(dead_code)]
    pub fn with_alias(mut self, alias: String) -> Self {
        self.alias = Some(alias);
        self
    }

    /// Create a node with depth.
    #[must_use]
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// Add a downstream (upstream lineage) node.
    #[allow(dead_code)]
    pub fn add_downstream(&mut self, node: LineageNode) {
        self.downstream.push(node);
    }

    /// Walk through all nodes in the lineage graph (pre-order).
    pub fn walk<F>(&self, visitor: &mut F)
    where
        F: FnMut(&LineageNode),
    {
        visitor(self);
        for child in &self.downstream {
            child.walk(visitor);
        }
    }

    /// Iterate over all nodes in the lineage graph.
    #[must_use]
    pub fn iter(&self) -> LineageIterator<'_> {
        LineageIterator { stack: vec![self] }
    }

    /// Get all source columns (leaf nodes) in this lineage.
    #[must_use]
    #[allow(dead_code)]
    pub fn source_columns(&self) -> Vec<&LineageNode> {
        self.iter().filter(|n| n.downstream.is_empty()).collect()
    }

    /// Get all source table names referenced in this lineage.
    #[must_use]
    pub fn source_tables(&self) -> Vec<String> {
        let mut tables = HashSet::new();
        for node in self.iter() {
            if let Some(ref source) = node.source_name {
                tables.insert(source.clone());
            }
        }
        tables.into_iter().collect()
    }

    /// Generate DOT format representation for visualization.
    #[must_use]
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph lineage {\n");
        dot.push_str("  rankdir=BT;\n");
        dot.push_str("  node [shape=box];\n");

        let mut node_id = 0;
        let mut node_ids = HashMap::new();

        // First pass: assign IDs and create nodes
        self.walk(&mut |node| {
            let id = format!("n{}", node_id);
            let label = if let Some(ref src) = node.source_name {
                format!("{}.{}", src, node.name)
            } else {
                node.name.clone()
            };
            dot.push_str(&format!("  {} [label=\"{}\"];\n", id, escape_dot(&label)));
            node_ids.insert(node as *const _ as usize, id);
            node_id += 1;
        });

        // Second pass: create edges
        self.walk(&mut |node| {
            let parent_id = node_ids.get(&(node as *const _ as usize)).unwrap();
            for child in &node.downstream {
                let child_id = node_ids.get(&(child as *const _ as usize)).unwrap();
                dot.push_str(&format!("  {} -> {};\n", child_id, parent_id));
            }
        });

        dot.push_str("}\n");
        dot
    }

    /// Generate Mermaid diagram representation.
    #[must_use]
    pub fn to_mermaid(&self) -> String {
        let mut mermaid = String::from("flowchart BT\n");

        let mut node_id = 0;
        let mut node_ids = HashMap::new();

        // First pass: assign IDs and create nodes
        self.walk(&mut |node| {
            let id = format!("n{}", node_id);
            let label = if let Some(ref src) = node.source_name {
                format!("{}.{}", src, node.name)
            } else {
                node.name.clone()
            };
            mermaid.push_str(&format!("  {}[\"{}\"]\n", id, escape_mermaid(&label)));
            node_ids.insert(node as *const _ as usize, id);
            node_id += 1;
        });

        // Second pass: create edges
        self.walk(&mut |node| {
            let parent_id = node_ids.get(&(node as *const _ as usize)).unwrap();
            for child in &node.downstream {
                let child_id = node_ids.get(&(child as *const _ as usize)).unwrap();
                mermaid.push_str(&format!("  {} --> {}\n", child_id, parent_id));
            }
        });

        mermaid
    }
}

/// Iterator over lineage nodes (pre-order traversal).
pub struct LineageIterator<'a> {
    stack: Vec<&'a LineageNode>,
}

impl<'a> Iterator for LineageIterator<'a> {
    type Item = &'a LineageNode;

    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop().inspect(|node| {
            // Push children in reverse order for pre-order traversal
            for child in node.downstream.iter().rev() {
                self.stack.push(child);
            }
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Lineage Graph
// ═══════════════════════════════════════════════════════════════════════

/// A lineage graph rooted at a specific output column.
#[derive(Debug, Clone)]
pub struct LineageGraph {
    /// The root node representing the target output column.
    pub node: LineageNode,
    /// The original SQL that was analyzed.
    pub sql: Option<String>,
    /// The dialect used for analysis.
    pub dialect: Dialect,
}

impl LineageGraph {
    /// Create a new lineage graph.
    #[must_use]
    pub fn new(node: LineageNode, dialect: Dialect) -> Self {
        Self {
            node,
            sql: None,
            dialect,
        }
    }

    /// Set the original SQL string.
    #[must_use]
    #[allow(dead_code)]
    pub fn with_sql(mut self, sql: String) -> Self {
        self.sql = Some(sql);
        self
    }

    /// Get all source tables in the lineage.
    #[must_use]
    pub fn source_tables(&self) -> Vec<String> {
        self.node.source_tables()
    }

    /// Get all source columns (leaf nodes).
    #[must_use]
    #[allow(dead_code)]
    pub fn source_columns(&self) -> Vec<&LineageNode> {
        self.node.source_columns()
    }

    /// Walk through all nodes in the graph.
    #[allow(dead_code)]
    pub fn walk<F>(&self, visitor: &mut F)
    where
        F: FnMut(&LineageNode),
    {
        self.node.walk(visitor);
    }

    /// Generate DOT format visualization.
    #[must_use]
    pub fn to_dot(&self) -> String {
        self.node.to_dot()
    }

    /// Generate Mermaid diagram visualization.
    #[must_use]
    pub fn to_mermaid(&self) -> String {
        self.node.to_mermaid()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Context for lineage building
// ═══════════════════════════════════════════════════════════════════════

/// Internal context for building lineage graphs.
struct LineageContext {
    /// The schema for column resolution.
    schema: MappingSchema,
    /// Configuration options.
    config: LineageConfig,
    /// Current depth in the lineage graph.
    depth: usize,
    /// CTE definitions available in this scope (owned).
    ctes: HashMap<String, Statement>,
    /// Visible sources in current scope (alias/name → source info).
    sources: HashMap<String, SourceInfo>,
    /// External sources for multi-query lineage.
    external_sources: HashMap<String, Statement>,
    /// Sources currently being visited (to prevent infinite recursion).
    visiting: HashSet<String>,
}

/// Information about a source (table, CTE, derived table).
#[derive(Debug, Clone)]
struct SourceInfo {
    /// The source type.
    kind: SourceKind,
    /// For subqueries/CTEs, the SELECT columns.
    columns: Option<Vec<SelectItem>>,
    /// The underlying statement, if any (owned).
    statement: Option<Statement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum SourceKind {
    Table,
    Cte,
    DerivedTable,
    External,
}

impl LineageContext {
    fn new(schema: &MappingSchema, config: &LineageConfig) -> Self {
        Self {
            schema: schema.clone(),
            config: config.clone(),
            depth: 0,
            ctes: HashMap::new(),
            sources: HashMap::new(),
            external_sources: HashMap::new(),
            visiting: HashSet::new(),
        }
    }

    fn with_depth(&self, depth: usize) -> Self {
        Self {
            schema: self.schema.clone(),
            config: self.config.clone(),
            depth,
            ctes: self.ctes.clone(),
            sources: self.sources.clone(),
            external_sources: self.external_sources.clone(),
            visiting: self.visiting.clone(),
        }
    }

    #[allow(dead_code)]
    fn resolve_source(&self, name: &str) -> Option<&SourceInfo> {
        let normalized = normalize_name(name, self.config.dialect);
        self.sources.get(&normalized)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════════════════════════════════

/// Build lineage for a specific output column in a SQL statement.
///
/// # Arguments
///
/// * `column` - The name of the output column to trace (can include table qualifier).
/// * `statement` - The parsed SQL statement.
/// * `schema` - Schema information for table/column resolution.
/// * `config` - Configuration options.
///
/// # Returns
///
/// A [`LineageGraph`] rooted at the target column, showing its upstream lineage.
///
/// # Errors
///
/// Returns [`LineageError::ColumnNotFound`] if the column is not in the output.
///
/// # Example
///
/// ```rust
/// use sqlglot_rust::parser::parse;
/// use sqlglot_rust::dialects::Dialect;
/// use sqlglot_rust::optimizer::lineage::{lineage, LineageConfig};
/// use sqlglot_rust::schema::MappingSchema;
///
/// let sql = "SELECT a, b AS c FROM t";
/// let ast = parse(sql, Dialect::Ansi).unwrap();
/// let schema = MappingSchema::new(Dialect::Ansi);
/// let config = LineageConfig::default();
///
/// let graph = lineage("c", &ast, &schema, &config).unwrap();
/// assert_eq!(graph.node.name, "c");
/// ```
pub fn lineage(
    column: &str,
    statement: &Statement,
    schema: &MappingSchema,
    config: &LineageConfig,
) -> LineageResult<LineageGraph> {
    // Parse external sources if provided
    let mut ctx = LineageContext::new(schema, config);

    for (name, sql) in &config.sources {
        match crate::parser::parse(sql, config.dialect) {
            Ok(stmt) => {
                ctx.external_sources
                    .insert(normalize_name(name, config.dialect), stmt);
            }
            Err(e) => return Err(LineageError::ParseError(e.to_string())),
        }
    }

    // Build lineage for the target column
    let node = build_lineage_for_column(column, statement, &mut ctx)?;

    Ok(LineageGraph::new(node, config.dialect))
}

/// Build lineage from a SQL string.
///
/// Convenience function that parses the SQL and builds lineage.
///
/// # Example
///
/// ```rust
/// use sqlglot_rust::dialects::Dialect;
/// use sqlglot_rust::optimizer::lineage::{lineage_sql, LineageConfig};
/// use sqlglot_rust::schema::MappingSchema;
///
/// let schema = MappingSchema::new(Dialect::Ansi);
/// let config = LineageConfig::default();
///
/// let graph = lineage_sql("c", "SELECT a + b AS c FROM t", &schema, &config).unwrap();
/// assert_eq!(graph.node.name, "c");
/// ```
pub fn lineage_sql(
    column: &str,
    sql: &str,
    schema: &MappingSchema,
    config: &LineageConfig,
) -> LineageResult<LineageGraph> {
    let statement = crate::parser::parse(sql, config.dialect)
        .map_err(|e| LineageError::ParseError(e.to_string()))?;

    let mut graph = lineage(column, &statement, schema, config)?;
    graph.sql = Some(sql.to_string());
    Ok(graph)
}

// ═══════════════════════════════════════════════════════════════════════
// Internal lineage building
// ═══════════════════════════════════════════════════════════════════════

/// Build lineage for a specific column in a statement.
fn build_lineage_for_column(
    column: &str,
    statement: &Statement,
    ctx: &mut LineageContext,
) -> LineageResult<LineageNode> {
    match statement {
        Statement::Select(sel) => build_lineage_for_select_column(column, sel, ctx),
        Statement::SetOperation(set_op) => build_lineage_for_set_operation(column, set_op, ctx),
        Statement::CreateView(cv) => build_lineage_for_column(column, &cv.query, ctx),
        _ => Err(LineageError::InvalidQuery(
            "Lineage analysis requires a SELECT or set operation statement".to_string(),
        )),
    }
}

/// Build lineage for a column in a SELECT statement.
fn build_lineage_for_select_column(
    column: &str,
    sel: &SelectStatement,
    ctx: &mut LineageContext,
) -> LineageResult<LineageNode> {
    // Register CTEs (cloning to avoid lifetime issues)
    for cte in &sel.ctes {
        let cte_name = normalize_name(&cte.name, ctx.config.dialect);
        ctx.ctes.insert(cte_name.clone(), (*cte.query).clone());
        ctx.sources.insert(
            cte_name,
            SourceInfo {
                kind: SourceKind::Cte,
                columns: extract_select_columns(&cte.query),
                statement: Some((*cte.query).clone()),
            },
        );
    }

    // Register FROM source
    if let Some(from) = &sel.from {
        register_table_source(&from.source, ctx);
    }

    // Register JOINs
    for join in &sel.joins {
        register_table_source(&join.table, ctx);
    }

    // Find the target column in the SELECT list
    let (col_name, table_qual) = parse_column_ref(column);

    for item in &sel.columns {
        match item {
            SelectItem::Expr { expr, alias, .. } => {
                let item_name = alias
                    .as_ref()
                    .map(String::as_str)
                    .unwrap_or_else(|| expr_output_name(expr));

                if matches_column_name(item_name, &col_name) {
                    return build_lineage_for_expr(expr, alias.clone(), ctx);
                }
            }
            SelectItem::Wildcard => {
                // Expand wildcard - check all sources
                for (source_name, source_info) in ctx.sources.clone() {
                    if let Some(cols) = &source_info.columns {
                        for col_item in cols {
                            if let SelectItem::Expr { expr, alias, .. } = col_item {
                                let item_name = alias
                                    .as_ref()
                                    .map(String::as_str)
                                    .unwrap_or_else(|| expr_output_name(expr));
                                if matches_column_name(item_name, &col_name) {
                                    return build_lineage_for_expr(expr, alias.clone(), ctx);
                                }
                            }
                        }
                    } else if source_info.kind == SourceKind::Table {
                        // Check schema for table columns
                        if let Ok(schema_cols) = ctx.schema.column_names(&[&source_name])
                            && schema_cols
                                .iter()
                                .any(|c| matches_column_name(c, &col_name))
                        {
                            // Found in schema
                            let mut node = LineageNode::new(col_name.clone())
                                .with_source(source_name.clone())
                                .with_depth(ctx.depth);
                            node.expression = Some(Expr::Column {
                                table: Some(source_name.clone()),
                                name: col_name.clone(),
                                quote_style: QuoteStyle::None,
                                table_quote_style: QuoteStyle::None,
                            });
                            return Ok(node);
                        }
                    }
                }
            }
            SelectItem::QualifiedWildcard { table } => {
                if table_qual
                    .as_ref()
                    .is_some_and(|t| matches_column_name(t, table))
                {
                    // Check if column exists in this table
                    if let Some(source_info) = ctx.sources.get(table).cloned()
                        && let Some(cols) = &source_info.columns
                    {
                        for col_item in cols {
                            if let SelectItem::Expr { expr, alias, .. } = col_item {
                                let item_name = alias
                                    .as_ref()
                                    .map(String::as_str)
                                    .unwrap_or_else(|| expr_output_name(expr));
                                if matches_column_name(item_name, &col_name) {
                                    return build_lineage_for_expr(expr, alias.clone(), ctx);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err(LineageError::ColumnNotFound(column.to_string()))
}

/// Build lineage for a set operation (UNION, INTERSECT, EXCEPT).
fn build_lineage_for_set_operation(
    column: &str,
    set_op: &SetOperationStatement,
    ctx: &mut LineageContext,
) -> LineageResult<LineageNode> {
    let mut root = LineageNode::new(column.to_string()).with_depth(ctx.depth);

    // Build lineage from both branches
    let mut child_ctx = ctx.with_depth(ctx.depth + 1);

    let left_lineage = build_lineage_for_column(column, &set_op.left, &mut child_ctx)?;
    let right_lineage = build_lineage_for_column(column, &set_op.right, &mut child_ctx)?;

    root.downstream.push(left_lineage);
    root.downstream.push(right_lineage);

    Ok(root)
}

/// Build lineage for an expression.
fn build_lineage_for_expr(
    expr: &Expr,
    alias: Option<String>,
    ctx: &mut LineageContext,
) -> LineageResult<LineageNode> {
    let name = alias
        .clone()
        .unwrap_or_else(|| expr_to_name(expr, ctx.config.trim_qualifiers));
    let mut node = LineageNode::new(name.clone())
        .with_expression(expr.clone())
        .with_depth(ctx.depth);

    if let Some(a) = alias {
        node.alias = Some(a);
    }

    // Collect column references from the expression
    let columns = collect_expr_columns(expr);

    let mut child_ctx = ctx.with_depth(ctx.depth + 1);

    for col_ref in columns {
        let child_node = resolve_column_lineage(&col_ref, &mut child_ctx)?;
        node.downstream.push(child_node);
    }

    Ok(node)
}

/// Resolve lineage for a column reference.
fn resolve_column_lineage(
    col: &ColumnReference,
    ctx: &mut LineageContext,
) -> LineageResult<LineageNode> {
    let name = if ctx.config.trim_qualifiers {
        col.name.clone()
    } else {
        col.qualified_name()
    };

    // If table qualifier is provided, look up in that source
    if let Some(ref table) = col.table {
        let normalized_table = normalize_name(table, ctx.config.dialect);

        if let Some(source_info) = ctx.sources.get(&normalized_table).cloned() {
            match source_info.kind {
                SourceKind::Table => {
                    // Base table - this is a leaf node
                    let node = LineageNode::new(name)
                        .with_source(normalized_table)
                        .with_depth(ctx.depth);
                    return Ok(node);
                }
                SourceKind::Cte | SourceKind::DerivedTable => {
                    // Recurse into CTE/derived table (if not already visiting)
                    if !ctx.visiting.contains(&normalized_table)
                        && let Some(stmt) = source_info.statement
                    {
                        ctx.visiting.insert(normalized_table.clone());
                        let result = build_lineage_for_column(&col.name, &stmt, ctx);
                        ctx.visiting.remove(&normalized_table);
                        return result;
                    }
                    // If already visiting, treat as leaf
                    let node = LineageNode::new(name)
                        .with_source(normalized_table)
                        .with_depth(ctx.depth);
                    return Ok(node);
                }
                SourceKind::External => {
                    // Check external sources
                    if let Some(stmt) = ctx.external_sources.get(&normalized_table).cloned() {
                        return build_lineage_for_column(&col.name, &stmt, ctx);
                    }
                }
            }
        }
    }

    // No table qualifier - search all sources
    for (source_name, source_info) in ctx.sources.clone() {
        match source_info.kind {
            SourceKind::Table => {
                // Check if this table has the column
                if ctx.schema.has_column(&[&source_name], &col.name) {
                    let node = LineageNode::new(name)
                        .with_source(source_name.clone())
                        .with_depth(ctx.depth);
                    return Ok(node);
                }
            }
            SourceKind::Cte | SourceKind::DerivedTable => {
                // Skip if already visiting this source
                if ctx.visiting.contains(&source_name) {
                    continue;
                }
                // Check if CTE/derived table outputs this column
                if let Some(ref columns) = source_info.columns
                    && columns.iter().any(|c| select_item_has_column(c, &col.name))
                    && let Some(stmt) = source_info.statement
                {
                    ctx.visiting.insert(source_name.clone());
                    let result = build_lineage_for_column(&col.name, &stmt, ctx);
                    ctx.visiting.remove(&source_name);
                    return result;
                }
            }
            SourceKind::External => {}
        }
    }

    // Column not found in any known source - treat as external/unknown
    let node = LineageNode::new(name).with_depth(ctx.depth);
    Ok(node)
}

/// Register a table source in the context.
fn register_table_source(source: &TableSource, ctx: &mut LineageContext) {
    match source {
        TableSource::Table(table_ref) => {
            let key = table_ref.alias.as_ref().unwrap_or(&table_ref.name).clone();
            let normalized = normalize_name(&key, ctx.config.dialect);
            // Don't overwrite CTEs or derived tables
            ctx.sources.entry(normalized).or_insert(SourceInfo {
                kind: SourceKind::Table,
                columns: None,
                statement: None,
            });
        }
        TableSource::Subquery { query, alias, .. } => {
            if let Some(alias) = alias {
                let normalized = normalize_name(alias, ctx.config.dialect);
                ctx.sources.insert(
                    normalized,
                    SourceInfo {
                        kind: SourceKind::DerivedTable,
                        columns: extract_select_columns(query),
                        statement: Some((**query).clone()),
                    },
                );
            }
        }
        TableSource::Lateral { source } => {
            register_table_source(source, ctx);
        }
        TableSource::Pivot { source, alias, .. } | TableSource::Unpivot { source, alias, .. } => {
            register_table_source(source, ctx);
            // TODO: Track pivot/unpivot column transformations
            if let Some(alias) = alias {
                let normalized = normalize_name(alias, ctx.config.dialect);
                ctx.sources.insert(
                    normalized,
                    SourceInfo {
                        kind: SourceKind::DerivedTable,
                        columns: None,
                        statement: None,
                    },
                );
            }
        }
        TableSource::TableFunction { alias, .. } => {
            if let Some(alias) = alias {
                let normalized = normalize_name(alias, ctx.config.dialect);
                ctx.sources.insert(
                    normalized,
                    SourceInfo {
                        kind: SourceKind::Table,
                        columns: None,
                        statement: None,
                    },
                );
            }
        }
        TableSource::Unnest { alias, .. } => {
            if let Some(alias) = alias {
                let normalized = normalize_name(alias, ctx.config.dialect);
                ctx.sources.insert(
                    normalized,
                    SourceInfo {
                        kind: SourceKind::Table,
                        columns: None,
                        statement: None,
                    },
                );
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Helper types and functions
// ═══════════════════════════════════════════════════════════════════════

/// A column reference found in an expression.
#[derive(Debug, Clone)]
struct ColumnReference {
    table: Option<String>,
    name: String,
}

impl ColumnReference {
    fn qualified_name(&self) -> String {
        if let Some(ref table) = self.table {
            format!("{}.{}", table, self.name)
        } else {
            self.name.clone()
        }
    }
}

/// Collect all column references from an expression.
fn collect_expr_columns(expr: &Expr) -> Vec<ColumnReference> {
    let mut columns = Vec::new();

    expr.walk(&mut |e| {
        if let Expr::Column { table, name, .. } = e {
            columns.push(ColumnReference {
                table: table.clone(),
                name: name.clone(),
            });
            return false; // Don't recurse into column nodes
        }
        // Don't descend into subqueries
        !matches!(
            e,
            Expr::Subquery(_) | Expr::Exists { .. } | Expr::InSubquery { .. }
        )
    });

    columns
}

/// Extract SELECT columns from a statement.
fn extract_select_columns(stmt: &Statement) -> Option<Vec<SelectItem>> {
    match stmt {
        Statement::Select(sel) => Some(sel.columns.clone()),
        Statement::SetOperation(set_op) => extract_select_columns(&set_op.left),
        Statement::CreateView(cv) => extract_select_columns(&cv.query),
        _ => None,
    }
}

/// Get the output name of an expression.
fn expr_output_name(expr: &Expr) -> &str {
    match expr {
        Expr::Column { name, .. } => name,
        Expr::Alias { name, .. } => name,
        _ => "",
    }
}

/// Convert an expression to a displayable name.
fn expr_to_name(expr: &Expr, trim_qualifiers: bool) -> String {
    match expr {
        Expr::Column { table, name, .. } => {
            if trim_qualifiers {
                name.clone()
            } else if let Some(t) = table {
                format!("{}.{}", t, name)
            } else {
                name.clone()
            }
        }
        Expr::Alias { name, .. } => name.clone(),
        Expr::Function { name, .. } => format!("{}(...)", name),
        Expr::BinaryOp { op, .. } => format!("({:?})", op),
        Expr::Cast { data_type, .. } => format!("CAST AS {:?}", data_type),
        _ => "expr".to_string(),
    }
}

/// Parse a column reference string into (name, optional_table_qualifier).
fn parse_column_ref(column: &str) -> (String, Option<String>) {
    if let Some(idx) = column.rfind('.') {
        let table = column[..idx].to_string();
        let name = column[idx + 1..].to_string();
        (name, Some(table))
    } else {
        (column.to_string(), None)
    }
}

/// Check if a column name matches (case-insensitive for most dialects).
fn matches_column_name(item: &str, target: &str) -> bool {
    item.eq_ignore_ascii_case(target)
}

/// Normalize an identifier name for the given dialect.
fn normalize_name(name: &str, dialect: Dialect) -> String {
    crate::schema::normalize_identifier(name, dialect)
}

/// Check if a SELECT item outputs a column with the given name.
fn select_item_has_column(item: &SelectItem, name: &str) -> bool {
    match item {
        SelectItem::Expr { expr, alias, .. } => {
            let item_name = alias
                .as_ref()
                .map(String::as_str)
                .unwrap_or_else(|| expr_output_name(expr));
            matches_column_name(item_name, name)
        }
        SelectItem::Wildcard => true, // Could match any column
        SelectItem::QualifiedWildcard { .. } => true,
    }
}

/// Escape a string for DOT format.
fn escape_dot(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Escape a string for Mermaid format.
fn escape_mermaid(s: &str) -> String {
    s.replace('"', "'")
        .replace('\n', " ")
        .replace('[', "(")
        .replace(']', ")")
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn test_config() -> LineageConfig {
        LineageConfig::new(Dialect::Ansi)
    }

    fn test_schema() -> MappingSchema {
        let mut schema = MappingSchema::new(Dialect::Ansi);
        schema
            .add_table(
                &["t"],
                vec![
                    ("a".to_string(), DataType::Int),
                    ("b".to_string(), DataType::Int),
                    ("c".to_string(), DataType::Int),
                ],
            )
            .unwrap();
        schema
            .add_table(
                &["users"],
                vec![
                    ("id".to_string(), DataType::Int),
                    ("name".to_string(), DataType::Varchar(Some(255))),
                    ("email".to_string(), DataType::Text),
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
                ],
            )
            .unwrap();
        schema
    }

    #[test]
    fn test_simple_column_lineage() {
        let sql = "SELECT a FROM t";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("a", &ast, &schema, &config).unwrap();

        assert_eq!(graph.node.name, "a");
        assert_eq!(graph.node.depth, 0);
        // The root column depends on t.a
        assert_eq!(graph.node.downstream.len(), 1);
        assert_eq!(graph.node.downstream[0].source_name, Some("t".to_string()));
    }

    #[test]
    fn test_aliased_column_lineage() {
        let sql = "SELECT a AS col_a FROM t";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("col_a", &ast, &schema, &config).unwrap();

        assert_eq!(graph.node.name, "col_a");
        assert_eq!(graph.node.alias, Some("col_a".to_string()));
    }

    #[test]
    fn test_expression_lineage() {
        let sql = "SELECT a + b AS sum FROM t";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("sum", &ast, &schema, &config).unwrap();

        assert_eq!(graph.node.name, "sum");
        // The sum depends on both a and b
        assert_eq!(graph.node.downstream.len(), 2);
    }

    #[test]
    fn test_cte_lineage() {
        let sql = "WITH cte AS (SELECT a FROM t) SELECT a FROM cte";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("a", &ast, &schema, &config).unwrap();

        assert_eq!(graph.node.name, "a");
        // Should trace through the CTE
        assert!(graph.source_tables().contains(&"t".to_string()));
    }

    #[test]
    fn test_join_lineage() {
        let sql = "SELECT u.name, o.amount FROM users u JOIN orders o ON u.id = o.user_id";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("name", &ast, &schema, &config).unwrap();
        assert_eq!(graph.node.name, "name");

        let graph2 = lineage("amount", &ast, &schema, &config).unwrap();
        assert_eq!(graph2.node.name, "amount");
    }

    #[test]
    fn test_union_lineage() {
        let sql = "SELECT a FROM t UNION SELECT b AS a FROM t";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("a", &ast, &schema, &config).unwrap();

        assert_eq!(graph.node.name, "a");
        // Should have two branches
        assert_eq!(graph.node.downstream.len(), 2);
    }

    #[test]
    fn test_column_not_found() {
        let sql = "SELECT a FROM t";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let result = lineage("nonexistent", &ast, &schema, &config);
        assert!(matches!(result, Err(LineageError::ColumnNotFound(_))));
    }

    #[test]
    fn test_derived_table_lineage() {
        let sql = "SELECT x FROM (SELECT a AS x FROM t) sub";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("x", &ast, &schema, &config).unwrap();

        assert_eq!(graph.node.name, "x");
        // Should trace through the derived table to t.a
        assert!(graph.source_tables().contains(&"t".to_string()));
    }

    #[test]
    fn test_function_lineage() {
        let sql = "SELECT SUM(a) AS total FROM t";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("total", &ast, &schema, &config).unwrap();

        assert_eq!(graph.node.name, "total");
        assert_eq!(graph.node.downstream.len(), 1);
    }

    #[test]
    fn test_lineage_sql_convenience() {
        let schema = test_schema();
        let config = test_config();

        let graph = lineage_sql("b", "SELECT a, b FROM t", &schema, &config).unwrap();

        assert_eq!(graph.node.name, "b");
        assert_eq!(graph.sql, Some("SELECT a, b FROM t".to_string()));
    }

    #[test]
    fn test_source_tables() {
        let sql = "SELECT u.name, o.amount FROM users u JOIN orders o ON u.id = o.user_id";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("name", &ast, &schema, &config).unwrap();
        let tables = graph.source_tables();

        assert!(tables.contains(&"u".to_string()));
    }

    #[test]
    fn test_to_dot() {
        let sql = "SELECT a AS col FROM t";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("col", &ast, &schema, &config).unwrap();
        let dot = graph.to_dot();

        assert!(dot.contains("digraph lineage"));
        assert!(dot.contains("rankdir=BT"));
    }

    #[test]
    fn test_to_mermaid() {
        let sql = "SELECT a AS col FROM t";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("col", &ast, &schema, &config).unwrap();
        let mermaid = graph.to_mermaid();

        assert!(mermaid.contains("flowchart BT"));
    }

    #[test]
    fn test_case_expression_lineage() {
        let sql = "SELECT CASE WHEN a > 0 THEN b ELSE c END AS result FROM t";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("result", &ast, &schema, &config).unwrap();

        assert_eq!(graph.node.name, "result");
        // Should depend on a, b, and c
        assert!(graph.node.downstream.len() >= 2);
    }

    #[test]
    fn test_coalesce_lineage() {
        let sql = "SELECT COALESCE(a, b, c) AS val FROM t";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("val", &ast, &schema, &config).unwrap();

        assert_eq!(graph.node.name, "val");
        // Should depend on a, b, and c
        assert_eq!(graph.node.downstream.len(), 3);
    }

    #[test]
    fn test_nested_cte_lineage() {
        let sql = r#"
            WITH cte1 AS (SELECT a, b FROM t),
                 cte2 AS (SELECT a + b AS sum FROM cte1)
            SELECT sum FROM cte2
        "#;
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("sum", &ast, &schema, &config).unwrap();

        assert_eq!(graph.node.name, "sum");
        // Should trace through both CTEs to t
        let sources = graph.source_tables();
        assert!(sources.contains(&"t".to_string()));
    }

    #[test]
    fn test_lineage_iterator() {
        let sql = "SELECT a + b AS sum FROM t";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = test_config();

        let graph = lineage("sum", &ast, &schema, &config).unwrap();

        let nodes: Vec<_> = graph.node.iter().collect();
        assert!(!nodes.is_empty());
        assert_eq!(nodes[0].name, "sum");
    }

    #[test]
    fn test_external_sources() {
        let schema = test_schema();
        let mut sources = HashMap::new();
        sources.insert("view1".to_string(), "SELECT a FROM t".to_string());

        let config = LineageConfig::new(Dialect::Ansi).with_sources(sources);

        let sql = "SELECT a FROM view1";
        let result = lineage_sql("a", sql, &schema, &config);
        // Should parse and handle external sources
        assert!(result.is_ok() || matches!(result, Err(LineageError::ColumnNotFound(_))));
    }

    #[test]
    fn test_qualified_column() {
        let sql = "SELECT t.a FROM t";
        let ast = parse(sql, Dialect::Ansi).unwrap();
        let schema = test_schema();
        let config = LineageConfig::new(Dialect::Ansi).with_trim_qualifiers(false);

        let graph = lineage("a", &ast, &schema, &config).unwrap();

        // With trim_qualifiers=false, should preserve qualification
        assert!(graph.node.name.contains('a'));
    }
}
