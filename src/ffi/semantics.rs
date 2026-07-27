//! C FFI bindings for schema-aware semantic APIs.

use std::collections::{BTreeMap, HashMap};
use std::os::raw::c_char;
use std::ptr;

use serde::{Deserialize, Serialize};

use super::{cstr_to_option, resolve_dialect, to_c_string};
use crate::ast::{Expr, Statement, TableRef};
use crate::dialects::Dialect;
use crate::optimizer::lineage::{LineageConfig, LineageGraph, LineageNode, lineage};
use crate::optimizer::qualify_columns::qualify_columns;
use crate::optimizer::scope_analysis::{ColumnRef, Scope, ScopeType, Source, build_scope};
use crate::parser::parse_data_type;
use crate::schema::MappingSchema;

#[derive(Serialize)]
struct ColumnRefView<'a> {
    table: &'a Option<String>,
    name: &'a str,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SourceView<'a> {
    Table { table: &'a TableRef },
    Scope { scope: Box<ScopeView<'a>> },
}

#[derive(Serialize)]
struct ScopeView<'a> {
    scope_type: &'static str,
    sources: BTreeMap<&'a str, SourceView<'a>>,
    columns: Vec<ColumnRefView<'a>>,
    external_columns: Vec<ColumnRefView<'a>>,
    derived_table_scopes: Vec<ScopeView<'a>>,
    subquery_scopes: Vec<ScopeView<'a>>,
    union_scopes: Vec<ScopeView<'a>>,
    cte_scopes: Vec<ScopeView<'a>>,
    selected_sources: BTreeMap<&'a str, SourceView<'a>>,
    is_correlated: bool,
}

#[derive(Serialize)]
struct LineageNodeView<'a> {
    name: &'a str,
    expression: &'a Option<Expr>,
    source_name: &'a Option<String>,
    source: &'a Option<Expr>,
    downstream: Vec<LineageNodeView<'a>>,
    alias: &'a Option<String>,
    depth: usize,
}

#[derive(Serialize)]
struct LineageGraphView<'a> {
    node: LineageNodeView<'a>,
    sql: &'a Option<String>,
    dialect: Dialect,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LineageConfigInput {
    dialect: Option<String>,
    trim_qualifiers: Option<bool>,
    sources: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MappingSchemaInput {
    tables: Vec<TableInput>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TableInput {
    path: Vec<String>,
    columns: Vec<ColumnInput>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ColumnInput {
    name: String,
    data_type: String,
}

fn column_view(column: &ColumnRef) -> ColumnRefView<'_> {
    ColumnRefView {
        table: &column.table,
        name: &column.name,
    }
}

fn source_view(source: &Source) -> SourceView<'_> {
    match source {
        Source::Table(table) => SourceView::Table { table },
        Source::Scope(scope) => SourceView::Scope {
            scope: Box::new(scope_view(scope)),
        },
    }
}

fn sources_view(sources: &HashMap<String, Source>) -> BTreeMap<&str, SourceView<'_>> {
    sources
        .iter()
        .map(|(name, source)| (name.as_str(), source_view(source)))
        .collect()
}

fn scope_view(scope: &Scope) -> ScopeView<'_> {
    ScopeView {
        scope_type: match scope.scope_type {
            ScopeType::Root => "root",
            ScopeType::Subquery => "subquery",
            ScopeType::DerivedTable => "derived_table",
            ScopeType::Cte => "cte",
            ScopeType::Union => "union",
            ScopeType::Udtf => "udtf",
        },
        sources: sources_view(&scope.sources),
        columns: scope.columns.iter().map(column_view).collect(),
        external_columns: scope.external_columns.iter().map(column_view).collect(),
        derived_table_scopes: scope.derived_table_scopes.iter().map(scope_view).collect(),
        subquery_scopes: scope.subquery_scopes.iter().map(scope_view).collect(),
        union_scopes: scope.union_scopes.iter().map(scope_view).collect(),
        cte_scopes: scope.cte_scopes.iter().map(scope_view).collect(),
        selected_sources: sources_view(&scope.selected_sources),
        is_correlated: scope.is_correlated,
    }
}

fn lineage_node_view(node: &LineageNode) -> LineageNodeView<'_> {
    LineageNodeView {
        name: &node.name,
        expression: &node.expression,
        source_name: &node.source_name,
        source: &node.source,
        downstream: node.downstream.iter().map(lineage_node_view).collect(),
        alias: &node.alias,
        depth: node.depth,
    }
}

fn lineage_graph_view(graph: &LineageGraph) -> LineageGraphView<'_> {
    LineageGraphView {
        node: lineage_node_view(&graph.node),
        sql: &graph.sql,
        dialect: graph.dialect,
    }
}

fn parse_statement(json: &str) -> Option<Statement> {
    serde_json::from_str(json).ok()
}

fn mapping_schema(json: &str, dialect: Dialect) -> Option<MappingSchema> {
    let input: MappingSchemaInput = serde_json::from_str(json).ok()?;
    let mut schema = MappingSchema::new(dialect);

    for table in input.tables {
        let path: Vec<&str> = table.path.iter().map(String::as_str).collect();
        let typed_columns = table
            .columns
            .into_iter()
            .map(|column| {
                parse_data_type(&column.data_type, dialect)
                    .ok()
                    .map(|data_type| (column.name, data_type))
            })
            .collect::<Option<Vec<_>>>()?;
        schema.replace_table(&path, typed_columns).ok()?;
    }

    Some(schema)
}

fn serialize<T: Serialize>(value: &T) -> *mut c_char {
    serde_json::to_string(value)
        .map(to_c_string)
        .unwrap_or(ptr::null_mut())
}

/// Qualify a JSON-serialized SQLGlot statement using a JSON schema mapping.
///
/// Returns a heap-allocated JSON statement, or `NULL` for invalid input.
///
/// # Safety
///
/// Each non-null argument must point to a readable NUL-terminated string that
/// remains valid for the duration of the call. Invalid UTF-8 or invalid input
/// returns `NULL`. Free a non-null return value with [`super::sqlglot_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sqlglot_qualify_columns(
    ast_json: *const c_char,
    schema_json: *const c_char,
    dialect: *const c_char,
) -> *mut c_char {
    let Some(statement) = (unsafe { cstr_to_option(ast_json) }).and_then(parse_statement) else {
        return ptr::null_mut();
    };
    let dialect = resolve_dialect(unsafe { cstr_to_option(dialect) });
    let Some(schema) =
        (unsafe { cstr_to_option(schema_json) }).and_then(|json| mapping_schema(json, dialect))
    else {
        return ptr::null_mut();
    };

    serialize(&qualify_columns(statement, &schema))
}

/// Build a scope tree from a JSON-serialized SQLGlot statement.
///
/// Returns a heap-allocated JSON scope, or `NULL` for invalid input.
///
/// # Safety
///
/// A non-null `ast_json` must point to a readable NUL-terminated string that
/// remains valid for the duration of the call. Invalid UTF-8 or invalid input
/// returns `NULL`. Free a non-null return value with [`super::sqlglot_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sqlglot_build_scope(ast_json: *const c_char) -> *mut c_char {
    let Some(statement) = (unsafe { cstr_to_option(ast_json) }).and_then(parse_statement) else {
        return ptr::null_mut();
    };

    serialize(&scope_view(&build_scope(&statement)))
}

/// Build requested output-column lineage from JSON-serialized SQLGlot inputs.
///
/// Returns a heap-allocated JSON lineage graph, or `NULL` for invalid input or
/// a lineage error.
///
/// # Safety
///
/// Each non-null argument must point to a readable NUL-terminated string that
/// remains valid for the duration of the call. Invalid UTF-8 or invalid input
/// returns `NULL`. Free a non-null return value with [`super::sqlglot_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sqlglot_lineage(
    column: *const c_char,
    ast_json: *const c_char,
    schema_json: *const c_char,
    config_json: *const c_char,
) -> *mut c_char {
    let Some(column) = (unsafe { cstr_to_option(column) }) else {
        return ptr::null_mut();
    };
    let Some(statement) = (unsafe { cstr_to_option(ast_json) }).and_then(parse_statement) else {
        return ptr::null_mut();
    };
    let Some(config) = (unsafe { cstr_to_option(config_json) })
        .and_then(|json| serde_json::from_str::<LineageConfigInput>(json).ok())
    else {
        return ptr::null_mut();
    };
    let dialect = resolve_dialect(config.dialect.as_deref());
    let Some(schema) =
        (unsafe { cstr_to_option(schema_json) }).and_then(|json| mapping_schema(json, dialect))
    else {
        return ptr::null_mut();
    };
    let lineage_config = LineageConfig::new(dialect)
        .with_sources(config.sources.unwrap_or_default())
        .with_trim_qualifiers(config.trim_qualifiers.unwrap_or(true));
    let Ok(graph) = lineage(column, &statement, &schema, &lineage_config) else {
        return ptr::null_mut();
    };

    serialize(&lineage_graph_view(&graph))
}
