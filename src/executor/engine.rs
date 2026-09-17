// Query execution engine.

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::errors::{Result, SqlglotError};

use super::eval;
use super::{ResultSet, RowContext, Table, Tables, Value};

// ═══════════════════════════════════════════════════════════════════════
// ExecutionContext
// ═══════════════════════════════════════════════════════════════════════

pub(crate) struct ExecutionContext<'a> {
    tables: &'a Tables,
    ctes: HashMap<String, ResultSet>,
}

impl<'a> ExecutionContext<'a> {
    pub fn new(tables: &'a Tables) -> Self {
        Self {
            tables,
            ctes: HashMap::new(),
        }
    }

    pub fn with_ctes(tables: &'a Tables, ctes: HashMap<String, ResultSet>) -> Self {
        Self { tables, ctes }
    }

    // ── Top-level dispatch ───────────────────────────────────────────

    pub fn execute(&mut self, stmt: &Statement) -> Result<ResultSet> {
        match stmt {
            Statement::Select(select) => self.execute_select(select),
            Statement::SetOperation(set_op) => self.execute_set_operation(set_op),
            Statement::Expression(expr) => {
                let row = RowContext::empty();
                let val = eval::eval_expr(expr, &row, self.tables, &self.ctes)?;
                Ok(ResultSet::new(vec!["result".to_string()], vec![vec![val]]))
            }
            _ => Err(SqlglotError::Internal(
                "Only SELECT and SET OPERATION statements can be executed".to_string(),
            )),
        }
    }

    // ── SELECT ───────────────────────────────────────────────────────

    fn execute_select(&mut self, select: &SelectStatement) -> Result<ResultSet> {
        // 1. Register CTEs.
        for cte in &select.ctes {
            let cte_result = self.execute(&cte.query)?;
            self.ctes.insert(cte.name.to_lowercase(), cte_result);
        }

        // 2. Resolve FROM clause.
        let mut rows = if let Some(from) = &select.from {
            self.resolve_table_source(&from.source)?
        } else {
            vec![RowContext::empty()]
        };

        // 3. Apply JOINs.
        for join in &select.joins {
            rows = self.execute_join(&rows, join)?;
        }

        // 4. WHERE filter.
        if let Some(where_expr) = &select.where_clause {
            rows = self.filter_rows(&rows, where_expr)?;
        }

        // 5. GROUP BY / aggregation.
        let has_group_by = !select.group_by.is_empty();
        let has_aggregates = select.columns.iter().any(|item| match item {
            SelectItem::Expr { expr, .. } => eval::expr_contains_aggregate(expr),
            _ => false,
        });

        let (result_columns, result_rows) = if has_group_by || has_aggregates {
            let groups = if has_group_by {
                self.group_rows(&rows, &select.group_by)?
            } else {
                vec![rows.clone()]
            };

            // HAVING.
            let filtered_groups = if let Some(having) = &select.having {
                groups
                    .into_iter()
                    .filter(|group| {
                        eval::eval_expr_group(having, group, self.tables, &self.ctes)
                            .map(|v| v.is_truthy())
                            .unwrap_or(false)
                    })
                    .collect()
            } else {
                groups
            };

            self.evaluate_select_items_grouped(&select.columns, &filtered_groups)?
        } else {
            self.evaluate_select_items(&select.columns, &rows)?
        };

        let mut result = ResultSet::new(result_columns, result_rows);

        // 6. DISTINCT.
        if select.distinct {
            let mut seen = HashSet::new();
            result.rows.retain(|row| seen.insert(row.clone()));
        }

        // 7. ORDER BY.
        if !select.order_by.is_empty() {
            self.sort_result(&mut result, &select.order_by)?;
        }

        // 8. LIMIT / OFFSET.
        if select.offset.is_some() || select.limit.is_some() {
            self.apply_limit_offset(&mut result, &select.limit, &select.offset)?;
        }

        Ok(result)
    }

    // ── FROM resolution ──────────────────────────────────────────────

    fn resolve_table_source(&self, source: &TableSource) -> Result<Vec<RowContext>> {
        match source {
            TableSource::Table(table_ref) => {
                if table_ref.temporal.is_some() {
                    return Err(SqlglotError::UnsupportedDialectFeature(
                        "table temporal clauses are not supported by the in-memory executor"
                            .to_string(),
                    ));
                }
                let table_name = table_ref.name.to_lowercase();
                let alias = table_ref
                    .alias
                    .as_ref()
                    .unwrap_or(&table_ref.name)
                    .to_lowercase();

                // CTEs first.
                if let Some(cte_result) = self.ctes.get(&table_name) {
                    return Ok(Self::result_to_rows(cte_result, &alias));
                }

                // Physical tables.
                let table = self
                    .tables
                    .get(&table_name)
                    .or_else(|| self.tables.get(&table_ref.name))
                    .ok_or_else(|| {
                        SqlglotError::Internal(format!("Table not found: {}", table_ref.name))
                    })?;

                Ok(Self::table_to_rows(table, &alias))
            }

            TableSource::Subquery { query, alias, .. } => {
                let result =
                    ExecutionContext::with_ctes(self.tables, self.ctes.clone()).execute(query)?;
                let alias_name = alias.as_deref().unwrap_or("subquery");
                Ok(Self::result_to_rows(&result, alias_name))
            }

            _ => Err(SqlglotError::Internal(format!(
                "Unsupported table source type: {source:?}"
            ))),
        }
    }

    fn table_to_rows(table: &Table, alias: &str) -> Vec<RowContext> {
        let columns: Vec<String> = table
            .columns
            .iter()
            .map(|c| format!("{}.{}", alias, c.to_lowercase()))
            .collect();

        table
            .rows
            .iter()
            .map(|row| RowContext::new(columns.clone(), row.clone()))
            .collect()
    }

    fn result_to_rows(result: &ResultSet, alias: &str) -> Vec<RowContext> {
        let columns: Vec<String> = result
            .columns
            .iter()
            .map(|c| format!("{}.{}", alias, c.to_lowercase()))
            .collect();

        result
            .rows
            .iter()
            .map(|row| RowContext::new(columns.clone(), row.clone()))
            .collect()
    }

    // ── JOINs ────────────────────────────────────────────────────────

    fn execute_join(&self, left_rows: &[RowContext], join: &JoinClause) -> Result<Vec<RowContext>> {
        let right_rows = self.resolve_table_source(&join.table)?;

        match join.join_type {
            JoinType::Inner => self.inner_join(left_rows, &right_rows, &join.on, &join.using),
            JoinType::Left => self.left_join(left_rows, &right_rows, &join.on, &join.using),
            JoinType::Right => self.right_join(left_rows, &right_rows, &join.on, &join.using),
            JoinType::Full => self.full_join(left_rows, &right_rows, &join.on, &join.using),
            JoinType::Cross => self.cross_join(left_rows, &right_rows),
            JoinType::Natural => self.natural_join(left_rows, &right_rows),
            _ => Err(SqlglotError::Internal(format!(
                "Unsupported join type: {:?}",
                join.join_type
            ))),
        }
    }

    fn cross_join(&self, left: &[RowContext], right: &[RowContext]) -> Result<Vec<RowContext>> {
        let mut result = Vec::with_capacity(left.len() * right.len());
        for l in left {
            for r in right {
                result.push(l.merge(r));
            }
        }
        Ok(result)
    }

    fn inner_join(
        &self,
        left: &[RowContext],
        right: &[RowContext],
        on: &Option<Expr>,
        using: &[String],
    ) -> Result<Vec<RowContext>> {
        let mut result = Vec::new();
        for l in left {
            for r in right {
                let merged = l.merge(r);
                if self.join_matches(&merged, on, using)? {
                    result.push(merged);
                }
            }
        }
        Ok(result)
    }

    fn left_join(
        &self,
        left: &[RowContext],
        right: &[RowContext],
        on: &Option<Expr>,
        using: &[String],
    ) -> Result<Vec<RowContext>> {
        let right_columns = right
            .first()
            .map(|r| &r.columns)
            .cloned()
            .unwrap_or_default();
        let mut result = Vec::new();

        for l in left {
            let mut matched = false;
            for r in right {
                let merged = l.merge(r);
                if self.join_matches(&merged, on, using)? {
                    result.push(merged);
                    matched = true;
                }
            }
            if !matched {
                result.push(l.merge(&RowContext::null_row(&right_columns)));
            }
        }
        Ok(result)
    }

    fn right_join(
        &self,
        left: &[RowContext],
        right: &[RowContext],
        on: &Option<Expr>,
        using: &[String],
    ) -> Result<Vec<RowContext>> {
        let left_columns = left
            .first()
            .map(|l| &l.columns)
            .cloned()
            .unwrap_or_default();
        let mut result = Vec::new();

        for r in right {
            let mut matched = false;
            for l in left {
                let merged = l.merge(r);
                if self.join_matches(&merged, on, using)? {
                    result.push(merged);
                    matched = true;
                }
            }
            if !matched {
                result.push(RowContext::null_row(&left_columns).merge(r));
            }
        }
        Ok(result)
    }

    fn full_join(
        &self,
        left: &[RowContext],
        right: &[RowContext],
        on: &Option<Expr>,
        using: &[String],
    ) -> Result<Vec<RowContext>> {
        let left_columns = left
            .first()
            .map(|l| &l.columns)
            .cloned()
            .unwrap_or_default();
        let right_columns = right
            .first()
            .map(|r| &r.columns)
            .cloned()
            .unwrap_or_default();
        let mut result = Vec::new();
        let mut right_matched = vec![false; right.len()];

        for l in left {
            let mut matched = false;
            for (j, r) in right.iter().enumerate() {
                let merged = l.merge(r);
                if self.join_matches(&merged, on, using)? {
                    result.push(merged);
                    matched = true;
                    right_matched[j] = true;
                }
            }
            if !matched {
                result.push(l.merge(&RowContext::null_row(&right_columns)));
            }
        }

        for (j, r) in right.iter().enumerate() {
            if !right_matched[j] {
                result.push(RowContext::null_row(&left_columns).merge(r));
            }
        }

        Ok(result)
    }

    fn natural_join(&self, left: &[RowContext], right: &[RowContext]) -> Result<Vec<RowContext>> {
        let left_cols: Vec<String> = left
            .first()
            .map(|l| {
                l.columns
                    .iter()
                    .map(|c| {
                        c.rsplit_once('.')
                            .map(|(_, name)| name.to_string())
                            .unwrap_or_else(|| c.clone())
                    })
                    .collect()
            })
            .unwrap_or_default();

        let right_cols: Vec<String> = right
            .first()
            .map(|r| {
                r.columns
                    .iter()
                    .map(|c| {
                        c.rsplit_once('.')
                            .map(|(_, name)| name.to_string())
                            .unwrap_or_else(|| c.clone())
                    })
                    .collect()
            })
            .unwrap_or_default();

        let common: Vec<String> = left_cols
            .iter()
            .filter(|c| right_cols.contains(c))
            .cloned()
            .collect();

        self.inner_join(left, right, &None, &common)
    }

    fn join_matches(
        &self,
        merged: &RowContext,
        on: &Option<Expr>,
        using: &[String],
    ) -> Result<bool> {
        if let Some(on_expr) = on {
            let val = eval::eval_expr(on_expr, merged, self.tables, &self.ctes)?;
            Ok(val.is_truthy())
        } else if !using.is_empty() {
            for col_name in using {
                let col_lower = col_name.to_lowercase();
                let mut vals = Vec::new();
                for (i, col) in merged.columns.iter().enumerate() {
                    let unqualified = col.rsplit_once('.').map(|(_, n)| n).unwrap_or(col);
                    if unqualified.to_lowercase() == col_lower {
                        vals.push(&merged.values[i]);
                    }
                }
                if vals.len() >= 2 && vals[0] != vals[1] {
                    return Ok(false);
                }
            }
            Ok(true)
        } else {
            Ok(true) // no condition ⇒ cross join semantics
        }
    }

    // ── WHERE filter ─────────────────────────────────────────────────

    fn filter_rows(&self, rows: &[RowContext], predicate: &Expr) -> Result<Vec<RowContext>> {
        let mut result = Vec::new();
        for row in rows {
            let val = eval::eval_expr(predicate, row, self.tables, &self.ctes)?;
            if val.is_truthy() {
                result.push(row.clone());
            }
        }
        Ok(result)
    }

    // ── GROUP BY ─────────────────────────────────────────────────────

    fn group_rows(&self, rows: &[RowContext], group_by: &[Expr]) -> Result<Vec<Vec<RowContext>>> {
        let mut groups: HashMap<Vec<Value>, Vec<RowContext>> = HashMap::new();
        let mut order = Vec::new();

        for row in rows {
            let key: Vec<Value> = group_by
                .iter()
                .map(|expr| eval::eval_expr(expr, row, self.tables, &self.ctes))
                .collect::<Result<_>>()?;

            if !groups.contains_key(&key) {
                order.push(key.clone());
            }
            groups.entry(key).or_default().push(row.clone());
        }

        Ok(order
            .into_iter()
            .map(|key| groups.remove(&key).unwrap())
            .collect())
    }

    // ── SELECT item evaluation ───────────────────────────────────────

    fn evaluate_select_items(
        &self,
        items: &[SelectItem],
        rows: &[RowContext],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>)> {
        let sample_row = rows.first().cloned().unwrap_or_else(RowContext::empty);
        let column_names = self.compute_column_names(items, &sample_row);

        let mut result_rows = Vec::new();
        for row in rows {
            let mut values = Vec::new();
            for item in items {
                match item {
                    SelectItem::Wildcard => values.extend(row.values.iter().cloned()),
                    SelectItem::QualifiedWildcard { table } => {
                        let prefix = format!("{}.", table.to_lowercase());
                        for (i, col) in row.columns.iter().enumerate() {
                            if col.to_lowercase().starts_with(&prefix) {
                                values.push(row.values[i].clone());
                            }
                        }
                    }
                    SelectItem::Expr { expr, .. } => {
                        values.push(eval::eval_expr(expr, row, self.tables, &self.ctes)?);
                    }
                }
            }
            result_rows.push(values);
        }

        Ok((column_names, result_rows))
    }

    fn evaluate_select_items_grouped(
        &self,
        items: &[SelectItem],
        groups: &[Vec<RowContext>],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>)> {
        let first_group = groups.first().map(|g| g.as_slice()).unwrap_or(&[]);
        let sample_row = first_group
            .first()
            .cloned()
            .unwrap_or_else(RowContext::empty);
        let column_names = self.compute_column_names(items, &sample_row);

        let mut result_rows = Vec::new();
        for group in groups {
            let first_row = group.first().cloned().unwrap_or_else(RowContext::empty);
            let mut values = Vec::new();

            for item in items {
                match item {
                    SelectItem::Wildcard => values.extend(first_row.values.iter().cloned()),
                    SelectItem::QualifiedWildcard { table } => {
                        let prefix = format!("{}.", table.to_lowercase());
                        for (i, col) in first_row.columns.iter().enumerate() {
                            if col.to_lowercase().starts_with(&prefix) {
                                values.push(first_row.values[i].clone());
                            }
                        }
                    }
                    SelectItem::Expr { expr, .. } => {
                        values.push(eval::eval_expr_group(expr, group, self.tables, &self.ctes)?);
                    }
                }
            }
            result_rows.push(values);
        }

        Ok((column_names, result_rows))
    }

    fn compute_column_names(&self, items: &[SelectItem], sample: &RowContext) -> Vec<String> {
        let mut names = Vec::new();
        for item in items {
            match item {
                SelectItem::Wildcard => {
                    for col in &sample.columns {
                        let name = col.rsplit_once('.').map(|(_, n)| n).unwrap_or(col);
                        names.push(name.to_string());
                    }
                }
                SelectItem::QualifiedWildcard { table } => {
                    let prefix = format!("{}.", table.to_lowercase());
                    for col in &sample.columns {
                        if col.to_lowercase().starts_with(&prefix) {
                            let name = col.rsplit_once('.').map(|(_, n)| n).unwrap_or(col);
                            names.push(name.to_string());
                        }
                    }
                }
                SelectItem::Expr { expr, alias, .. } => {
                    names.push(alias.clone().unwrap_or_else(|| expr_to_name(expr)));
                }
            }
        }
        names
    }

    // ── ORDER BY ─────────────────────────────────────────────────────

    fn sort_result(&self, result: &mut ResultSet, order_by: &[OrderByItem]) -> Result<()> {
        let row_contexts: Vec<RowContext> = result
            .rows
            .iter()
            .map(|row| RowContext::new(result.columns.clone(), row.clone()))
            .collect();

        let mut indices: Vec<usize> = (0..result.rows.len()).collect();

        // Pre-evaluate sort keys.
        let mut sort_keys: Vec<Vec<Value>> = Vec::new();
        for row_ctx in &row_contexts {
            let keys: Vec<Value> = order_by
                .iter()
                .map(|item| {
                    // Support ORDER BY <position>.
                    if let Expr::Number(n) = &item.expr {
                        if let Ok(pos) = n.parse::<usize>() {
                            if pos > 0 && pos <= row_ctx.values.len() {
                                return Ok(row_ctx.values[pos - 1].clone());
                            }
                        }
                    }
                    eval::eval_expr(&item.expr, row_ctx, self.tables, &self.ctes)
                })
                .collect::<Result<_>>()?;
            sort_keys.push(keys);
        }

        indices.sort_by(|&a, &b| {
            for (i, item) in order_by.iter().enumerate() {
                let va = &sort_keys[a][i];
                let vb = &sort_keys[b][i];
                let cmp = va.partial_cmp(vb).unwrap_or(std::cmp::Ordering::Equal);
                let cmp = if item.ascending { cmp } else { cmp.reverse() };
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
            }
            std::cmp::Ordering::Equal
        });

        result.rows = indices
            .into_iter()
            .map(|i| result.rows[i].clone())
            .collect();

        Ok(())
    }

    // ── LIMIT / OFFSET ──────────────────────────────────────────────

    fn apply_limit_offset(
        &self,
        result: &mut ResultSet,
        limit: &Option<Expr>,
        offset: &Option<Expr>,
    ) -> Result<()> {
        let offset_val = if let Some(off_expr) = offset {
            let row = RowContext::empty();
            eval::eval_expr(off_expr, &row, self.tables, &self.ctes)?
                .to_i64()
                .unwrap_or(0) as usize
        } else {
            0
        };

        let limit_val = if let Some(lim_expr) = limit {
            let row = RowContext::empty();
            Some(
                eval::eval_expr(lim_expr, &row, self.tables, &self.ctes)?
                    .to_i64()
                    .unwrap_or(0) as usize,
            )
        } else {
            None
        };

        let total = result.rows.len();
        let start = offset_val.min(total);
        let end = if let Some(lim) = limit_val {
            (start + lim).min(total)
        } else {
            total
        };

        result.rows = result.rows[start..end].to_vec();

        Ok(())
    }

    // ── Set operations ───────────────────────────────────────────────

    fn execute_set_operation(&mut self, set_op: &SetOperationStatement) -> Result<ResultSet> {
        let left_result = self.execute(&set_op.left)?;
        let right_result = self.execute(&set_op.right)?;

        let columns = left_result.columns.clone();

        let rows = match set_op.op {
            SetOperationType::Union => {
                let mut all_rows = left_result.rows;
                all_rows.extend(right_result.rows);
                if !set_op.all {
                    let mut seen = HashSet::new();
                    all_rows.retain(|row| seen.insert(row.clone()));
                }
                all_rows
            }
            SetOperationType::Intersect => {
                let right_set: HashSet<Vec<Value>> = right_result.rows.into_iter().collect();
                let mut result: Vec<Vec<Value>> = left_result
                    .rows
                    .into_iter()
                    .filter(|row| right_set.contains(row))
                    .collect();
                if !set_op.all {
                    let mut seen = HashSet::new();
                    result.retain(|row| seen.insert(row.clone()));
                }
                result
            }
            SetOperationType::Except => {
                let right_set: HashSet<Vec<Value>> = right_result.rows.into_iter().collect();
                let mut result: Vec<Vec<Value>> = left_result
                    .rows
                    .into_iter()
                    .filter(|row| !right_set.contains(row))
                    .collect();
                if !set_op.all {
                    let mut seen = HashSet::new();
                    result.retain(|row| seen.insert(row.clone()));
                }
                result
            }
        };

        let mut result = ResultSet::new(columns, rows);

        if !set_op.order_by.is_empty() {
            self.sort_result(&mut result, &set_op.order_by)?;
        }

        if set_op.limit.is_some() || set_op.offset.is_some() {
            self.apply_limit_offset(&mut result, &set_op.limit, &set_op.offset)?;
        }

        Ok(result)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════

fn expr_to_name(expr: &Expr) -> String {
    match expr {
        Expr::Column { name, table, .. } => {
            if let Some(t) = table {
                format!("{t}.{name}")
            } else {
                name.clone()
            }
        }
        Expr::Alias { name, .. } => name.clone(),
        Expr::Function { name, .. } => name.clone(),
        Expr::TypedFunction { func, .. } => match func {
            TypedFunction::Count { .. } => "count".to_string(),
            TypedFunction::Sum { .. } => "sum".to_string(),
            TypedFunction::Avg { .. } => "avg".to_string(),
            TypedFunction::Min { .. } => "min".to_string(),
            TypedFunction::Max { .. } => "max".to_string(),
            TypedFunction::Upper { .. } => "upper".to_string(),
            TypedFunction::Lower { .. } => "lower".to_string(),
            TypedFunction::Length { .. } => "length".to_string(),
            _ => "?func?".to_string(),
        },
        Expr::Star | Expr::Wildcard => "*".to_string(),
        _ => "?column?".to_string(),
    }
}
