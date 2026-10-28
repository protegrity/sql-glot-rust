//! AST Diff — semantic comparison of SQL expression trees.
//!
//! Implements a tree edit distance algorithm inspired by the Change Distiller
//! approach used in Python sqlglot's `diff.py`. Computes a sequence of
//! [`ChangeAction`]s that transform one AST into another.
//!
//! # Example
//!
//! ```rust
//! use sqlglot_rust::{parse, Dialect};
//! use sqlglot_rust::diff::{diff, ChangeAction};
//!
//! let source = parse("SELECT a, b FROM t WHERE a > 1", Dialect::Ansi).unwrap();
//! let target = parse("SELECT a, c FROM t WHERE a > 2", Dialect::Ansi).unwrap();
//! let changes = diff(&source, &target);
//!
//! for change in &changes {
//!     println!("{change:?}");
//! }
//! ```

use std::collections::HashMap;

use crate::ast::*;

/// A change action describing a single difference between two ASTs.
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeAction {
    /// A node present in `source` that was removed.
    Remove(AstNode),
    /// A node inserted into `target` that was not in `source`.
    Insert(AstNode),
    /// A node that is structurally identical in both trees.
    Keep(AstNode, AstNode),
    /// A node that was moved to a different position in the tree.
    Move(AstNode, AstNode),
    /// A node in `source` that was replaced by a different node in `target`.
    Update(AstNode, AstNode),
}

/// A wrapper around an AST node that can represent either statements or
/// expressions, enabling uniform diff output.
#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    Statement(Box<Statement>),
    Expr(Expr),
    SelectItem(SelectItem),
    JoinClause(JoinClause),
    OrderByItem(OrderByItem),
    Cte(Box<Cte>),
    ColumnDef(ColumnDef),
    TableConstraint(TableConstraint),
}

impl std::fmt::Display for AstNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AstNode::Statement(s) => write!(f, "{s:?}"),
            AstNode::Expr(e) => write!(f, "{e:?}"),
            AstNode::SelectItem(si) => write!(f, "{si:?}"),
            AstNode::JoinClause(j) => write!(f, "{j:?}"),
            AstNode::OrderByItem(o) => write!(f, "{o:?}"),
            AstNode::Cte(c) => write!(f, "{c:?}"),
            AstNode::ColumnDef(cd) => write!(f, "{cd:?}"),
            AstNode::TableConstraint(tc) => write!(f, "{tc:?}"),
        }
    }
}

/// Compute the semantic diff between two SQL statements.
///
/// Returns a list of [`ChangeAction`]s describing the minimal set of changes
/// needed to transform `source` into `target`.
#[must_use]
pub fn diff(source: &Statement, target: &Statement) -> Vec<ChangeAction> {
    let mut differ = AstDiffer::new();
    differ.diff_statements(source, target);
    differ.changes
}

/// Internal differ state that accumulates change actions.
struct AstDiffer {
    changes: Vec<ChangeAction>,
}

impl AstDiffer {
    fn new() -> Self {
        Self {
            changes: Vec::new(),
        }
    }

    fn diff_statements(&mut self, source: &Statement, target: &Statement) {
        use Statement::*;

        match (source, target) {
            (Select(s), Select(t)) => self.diff_select(s, t),
            (Insert(s), Insert(t)) => self.diff_insert(s, t),
            (Update(s), Update(t)) => self.diff_update(s, t),
            (Delete(s), Delete(t)) => self.diff_delete(s, t),
            (CreateTable(s), CreateTable(t)) => self.diff_create_table(s, t),
            (DropTable(s), DropTable(t)) => self.diff_drop_table(s, t),
            (SetOperation(s), SetOperation(t)) => self.diff_set_operation(s, t),
            (AlterTable(s), AlterTable(t)) => self.diff_alter_table(s, t),
            (CreateView(s), CreateView(t)) => self.diff_create_view(s, t),
            (Expression(s), Expression(t)) => self.diff_exprs(s, t),
            _ => {
                // Different statement types → remove old, insert new
                self.changes
                    .push(ChangeAction::Remove(AstNode::Statement(Box::new(
                        source.clone(),
                    ))));
                self.changes
                    .push(ChangeAction::Insert(AstNode::Statement(Box::new(
                        target.clone(),
                    ))));
            }
        }
    }

    // ── SELECT ─────────────────────────────────────────────────────────

    fn diff_select(&mut self, source: &SelectStatement, target: &SelectStatement) {
        // CTEs
        self.diff_ctes(&source.ctes, &target.ctes);

        // DISTINCT
        if source.distinct != target.distinct {
            if target.distinct {
                self.changes
                    .push(ChangeAction::Insert(AstNode::Expr(Expr::Column {
                        table: None,
                        name: "DISTINCT".to_string(),
                        quote_style: QuoteStyle::None,
                        table_quote_style: QuoteStyle::None,
                    })));
            } else {
                self.changes
                    .push(ChangeAction::Remove(AstNode::Expr(Expr::Column {
                        table: None,
                        name: "DISTINCT".to_string(),
                        quote_style: QuoteStyle::None,
                        table_quote_style: QuoteStyle::None,
                    })));
            }
        }

        // SELECT columns (ordered)
        self.diff_select_items(&source.columns, &target.columns);

        // FROM
        match (&source.from, &target.from) {
            (Some(sf), Some(tf)) => self.diff_table_sources(&sf.source, &tf.source),
            (None, Some(tf)) => self.insert_table_source(&tf.source),
            (Some(sf), None) => self.remove_table_source(&sf.source),
            (None, None) => {}
        }

        // JOINs
        self.diff_joins(&source.joins, &target.joins);

        // WHERE
        self.diff_optional_exprs(&source.where_clause, &target.where_clause);

        // GROUP BY
        self.diff_expr_lists(&source.group_by, &target.group_by);

        // HAVING
        self.diff_optional_exprs(&source.having, &target.having);

        // ORDER BY
        self.diff_order_by(&source.order_by, &target.order_by);

        // LIMIT
        self.diff_optional_exprs(&source.limit, &target.limit);

        // OFFSET
        self.diff_optional_exprs(&source.offset, &target.offset);

        // QUALIFY
        self.diff_optional_exprs(&source.qualify, &target.qualify);
    }

    // ── INSERT ─────────────────────────────────────────────────────────

    fn diff_insert(&mut self, source: &InsertStatement, target: &InsertStatement) {
        if source.table != target.table {
            self.changes.push(ChangeAction::Update(
                AstNode::Expr(table_ref_to_expr(&source.table)),
                AstNode::Expr(table_ref_to_expr(&target.table)),
            ));
        }

        // Column list
        self.diff_string_lists(&source.columns, &target.columns);

        // Source
        match (&source.source, &target.source) {
            (InsertSource::Values(sv), InsertSource::Values(tv)) => {
                for (i, (sr, tr)) in sv.iter().zip(tv.iter()).enumerate() {
                    self.diff_expr_lists(sr, tr);
                    let _ = i;
                }
                for extra in sv.iter().skip(tv.len()) {
                    for e in extra {
                        self.changes
                            .push(ChangeAction::Remove(AstNode::Expr(e.clone())));
                    }
                }
                for extra in tv.iter().skip(sv.len()) {
                    for e in extra {
                        self.changes
                            .push(ChangeAction::Insert(AstNode::Expr(e.clone())));
                    }
                }
            }
            (InsertSource::Query(sq), InsertSource::Query(tq)) => {
                self.diff_statements(sq, tq);
            }
            _ => {
                self.changes
                    .push(ChangeAction::Remove(AstNode::Statement(Box::new(
                        Statement::Insert(source.clone()),
                    ))));
                self.changes
                    .push(ChangeAction::Insert(AstNode::Statement(Box::new(
                        Statement::Insert(target.clone()),
                    ))));
            }
        }
    }

    // ── UPDATE ─────────────────────────────────────────────────────────

    fn diff_update(&mut self, source: &UpdateStatement, target: &UpdateStatement) {
        if source.table != target.table {
            self.changes.push(ChangeAction::Update(
                AstNode::Expr(table_ref_to_expr(&source.table)),
                AstNode::Expr(table_ref_to_expr(&target.table)),
            ));
        }

        // Assignments (ordered by column name matching)
        let source_map: HashMap<&str, &Expr> = source
            .assignments
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect();
        let target_map: HashMap<&str, &Expr> = target
            .assignments
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect();

        for (col, src_val) in &source_map {
            if let Some(tgt_val) = target_map.get(col) {
                self.diff_exprs(src_val, tgt_val);
            } else {
                self.changes
                    .push(ChangeAction::Remove(AstNode::Expr((*src_val).clone())));
            }
        }
        for (col, tgt_val) in &target_map {
            if !source_map.contains_key(col) {
                self.changes
                    .push(ChangeAction::Insert(AstNode::Expr((*tgt_val).clone())));
            }
        }

        self.diff_optional_exprs(&source.where_clause, &target.where_clause);
    }

    // ── DELETE ─────────────────────────────────────────────────────────

    fn diff_delete(&mut self, source: &DeleteStatement, target: &DeleteStatement) {
        if source.table != target.table {
            self.changes.push(ChangeAction::Update(
                AstNode::Expr(table_ref_to_expr(&source.table)),
                AstNode::Expr(table_ref_to_expr(&target.table)),
            ));
        }
        self.diff_optional_exprs(&source.where_clause, &target.where_clause);
    }

    // ── CREATE TABLE ───────────────────────────────────────────────────

    fn diff_create_table(&mut self, source: &CreateTableStatement, target: &CreateTableStatement) {
        if source.with_properties != target.with_properties
            || source.with_data != target.with_data
            || source.as_select != target.as_select
        {
            self.changes.push(ChangeAction::Update(
                AstNode::Statement(Box::new(Statement::CreateTable(source.clone()))),
                AstNode::Statement(Box::new(Statement::CreateTable(target.clone()))),
            ));
            return;
        }

        if source.table != target.table {
            self.changes.push(ChangeAction::Update(
                AstNode::Expr(table_ref_to_expr(&source.table)),
                AstNode::Expr(table_ref_to_expr(&target.table)),
            ));
        }

        // Column definitions (match by name)
        let source_cols: HashMap<&str, &ColumnDef> = source
            .columns
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let target_cols: HashMap<&str, &ColumnDef> = target
            .columns
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();

        for (name, src_col) in &source_cols {
            if let Some(tgt_col) = target_cols.get(name) {
                if src_col != tgt_col {
                    self.changes.push(ChangeAction::Update(
                        AstNode::ColumnDef((*src_col).clone()),
                        AstNode::ColumnDef((*tgt_col).clone()),
                    ));
                } else {
                    self.changes.push(ChangeAction::Keep(
                        AstNode::ColumnDef((*src_col).clone()),
                        AstNode::ColumnDef((*tgt_col).clone()),
                    ));
                }
            } else {
                self.changes
                    .push(ChangeAction::Remove(AstNode::ColumnDef((*src_col).clone())));
            }
        }
        for (name, tgt_col) in &target_cols {
            if !source_cols.contains_key(name) {
                self.changes
                    .push(ChangeAction::Insert(AstNode::ColumnDef((*tgt_col).clone())));
            }
        }

        // Constraints
        self.diff_constraints(&source.constraints, &target.constraints);
    }

    // ── DROP TABLE ─────────────────────────────────────────────────────

    fn diff_drop_table(&mut self, source: &DropTableStatement, target: &DropTableStatement) {
        if source != target {
            self.changes.push(ChangeAction::Update(
                AstNode::Statement(Box::new(Statement::DropTable(source.clone()))),
                AstNode::Statement(Box::new(Statement::DropTable(target.clone()))),
            ));
        } else {
            self.changes.push(ChangeAction::Keep(
                AstNode::Statement(Box::new(Statement::DropTable(source.clone()))),
                AstNode::Statement(Box::new(Statement::DropTable(target.clone()))),
            ));
        }
    }

    // ── SET OPERATION ──────────────────────────────────────────────────

    fn diff_set_operation(
        &mut self,
        source: &SetOperationStatement,
        target: &SetOperationStatement,
    ) {
        if source.op != target.op || source.all != target.all {
            self.changes.push(ChangeAction::Update(
                AstNode::Statement(Box::new(Statement::SetOperation(source.clone()))),
                AstNode::Statement(Box::new(Statement::SetOperation(target.clone()))),
            ));
            return;
        }
        self.diff_statements(&source.left, &target.left);
        self.diff_statements(&source.right, &target.right);
        self.diff_order_by(&source.order_by, &target.order_by);
        self.diff_optional_exprs(&source.limit, &target.limit);
        self.diff_optional_exprs(&source.offset, &target.offset);
    }

    // ── ALTER TABLE ────────────────────────────────────────────────────

    fn diff_alter_table(&mut self, source: &AlterTableStatement, target: &AlterTableStatement) {
        if source.table != target.table {
            self.changes.push(ChangeAction::Update(
                AstNode::Expr(table_ref_to_expr(&source.table)),
                AstNode::Expr(table_ref_to_expr(&target.table)),
            ));
        }
        // Actions compared for equality
        if source.actions != target.actions {
            self.changes.push(ChangeAction::Update(
                AstNode::Statement(Box::new(Statement::AlterTable(source.clone()))),
                AstNode::Statement(Box::new(Statement::AlterTable(target.clone()))),
            ));
        }
    }

    // ── CREATE VIEW ────────────────────────────────────────────────────

    fn diff_create_view(&mut self, source: &CreateViewStatement, target: &CreateViewStatement) {
        if source.name != target.name {
            self.changes.push(ChangeAction::Update(
                AstNode::Expr(table_ref_to_expr(&source.name)),
                AstNode::Expr(table_ref_to_expr(&target.name)),
            ));
        }
        self.diff_statements(&source.query, &target.query);
    }

    // ── Shared helpers ─────────────────────────────────────────────────

    fn diff_exprs(&mut self, source: &Expr, target: &Expr) {
        if source == target {
            self.changes.push(ChangeAction::Keep(
                AstNode::Expr(source.clone()),
                AstNode::Expr(target.clone()),
            ));
            return;
        }

        // Same top-level variant → recurse into children
        match (source, target) {
            (
                Expr::BinaryOp {
                    left: sl,
                    op: sop,
                    right: sr,
                },
                Expr::BinaryOp {
                    left: tl,
                    op: top,
                    right: tr,
                },
            ) => {
                if sop != top {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                } else {
                    self.diff_exprs(sl, tl);
                    self.diff_exprs(sr, tr);
                }
            }
            (Expr::UnaryOp { op: sop, expr: se }, Expr::UnaryOp { op: top, expr: te }) => {
                if sop != top {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                } else {
                    self.diff_exprs(se, te);
                }
            }
            (
                Expr::Function {
                    name: sn,
                    args: sa,
                    distinct: sd,
                    ..
                },
                Expr::Function {
                    name: tn,
                    args: ta,
                    distinct: td,
                    ..
                },
            ) => {
                if sn != tn || sd != td {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                } else {
                    self.diff_expr_lists(sa, ta);
                }
            }
            (
                Expr::Cast {
                    expr: se,
                    data_type: sd,
                },
                Expr::Cast {
                    expr: te,
                    data_type: td,
                },
            ) => {
                if sd != td {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                } else {
                    self.diff_exprs(se, te);
                }
            }
            (
                Expr::Case {
                    operand: so,
                    when_clauses: sw,
                    else_clause: se,
                },
                Expr::Case {
                    operand: to,
                    when_clauses: tw,
                    else_clause: te,
                },
            ) => {
                self.diff_optional_boxed_exprs(so, to);
                // when clauses — ordered
                for (i, ((sc, sr), (tc, tr))) in sw.iter().zip(tw.iter()).enumerate() {
                    self.diff_exprs(sc, tc);
                    self.diff_exprs(sr, tr);
                    let _ = i;
                }
                for (sc, sr) in sw.iter().skip(tw.len()) {
                    self.changes
                        .push(ChangeAction::Remove(AstNode::Expr(sc.clone())));
                    self.changes
                        .push(ChangeAction::Remove(AstNode::Expr(sr.clone())));
                }
                for (tc, tr) in tw.iter().skip(sw.len()) {
                    self.changes
                        .push(ChangeAction::Insert(AstNode::Expr(tc.clone())));
                    self.changes
                        .push(ChangeAction::Insert(AstNode::Expr(tr.clone())));
                }
                self.diff_optional_boxed_exprs(se, te);
            }
            (Expr::Nested(se), Expr::Nested(te)) => self.diff_exprs(se, te),
            (
                Expr::Between {
                    expr: se,
                    low: sl,
                    high: sh,
                    negated: sn,
                },
                Expr::Between {
                    expr: te,
                    low: tl,
                    high: th,
                    negated: tn,
                },
            ) => {
                if sn != tn {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                } else {
                    self.diff_exprs(se, te);
                    self.diff_exprs(sl, tl);
                    self.diff_exprs(sh, th);
                }
            }
            (
                Expr::InList {
                    expr: se,
                    list: sl,
                    negated: sn,
                },
                Expr::InList {
                    expr: te,
                    list: tl,
                    negated: tn,
                },
            ) => {
                if sn != tn {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                } else {
                    self.diff_exprs(se, te);
                    self.diff_expr_lists(sl, tl);
                }
            }
            (
                Expr::InSubquery {
                    expr: se,
                    subquery: ss,
                    negated: sn,
                },
                Expr::InSubquery {
                    expr: te,
                    subquery: ts,
                    negated: tn,
                },
            ) => {
                if sn != tn {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                } else {
                    self.diff_exprs(se, te);
                    self.diff_statements(ss, ts);
                }
            }
            (
                Expr::IsNull {
                    expr: se,
                    negated: sn,
                },
                Expr::IsNull {
                    expr: te,
                    negated: tn,
                },
            ) => {
                if sn != tn {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                } else {
                    self.diff_exprs(se, te);
                }
            }
            (
                Expr::Like {
                    expr: se,
                    pattern: sp,
                    negated: sn,
                    ..
                },
                Expr::Like {
                    expr: te,
                    pattern: tp,
                    negated: tn,
                    ..
                },
            )
            | (
                Expr::ILike {
                    expr: se,
                    pattern: sp,
                    negated: sn,
                    ..
                },
                Expr::ILike {
                    expr: te,
                    pattern: tp,
                    negated: tn,
                    ..
                },
            ) => {
                if sn != tn {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                } else {
                    self.diff_exprs(se, te);
                    self.diff_exprs(sp, tp);
                }
            }
            (Expr::Subquery(ss), Expr::Subquery(ts)) => self.diff_statements(ss, ts),
            (
                Expr::Exists {
                    subquery: ss,
                    negated: sn,
                },
                Expr::Exists {
                    subquery: ts,
                    negated: tn,
                },
            ) => {
                if sn != tn {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                } else {
                    self.diff_statements(ss, ts);
                }
            }
            (Expr::Alias { expr: se, name: sn }, Expr::Alias { expr: te, name: tn }) => {
                if sn != tn {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                } else {
                    self.diff_exprs(se, te);
                }
            }
            (Expr::Coalesce(sa), Expr::Coalesce(ta)) => self.diff_expr_lists(sa, ta),
            (Expr::ArrayLiteral(sa), Expr::ArrayLiteral(ta)) => self.diff_expr_lists(sa, ta),
            (Expr::Tuple(sa), Expr::Tuple(ta)) => self.diff_expr_lists(sa, ta),
            (Expr::TypedFunction { func: sf, .. }, Expr::TypedFunction { func: tf, .. }) => {
                if std::mem::discriminant(sf) == std::mem::discriminant(tf) && source == target {
                    self.changes.push(ChangeAction::Keep(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                } else {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(source.clone()),
                        AstNode::Expr(target.clone()),
                    ));
                }
            }
            // Different variant types → leaf-level update
            _ => {
                self.changes.push(ChangeAction::Update(
                    AstNode::Expr(source.clone()),
                    AstNode::Expr(target.clone()),
                ));
            }
        }
    }

    /// Diff two ordered expression lists (e.g., SELECT columns, function args).
    fn diff_expr_lists(&mut self, source: &[Expr], target: &[Expr]) {
        // Use longest common subsequence for ordered diff
        let lcs = compute_lcs(source, target);
        let mut si = 0;
        let mut ti = 0;
        let mut li = 0;

        while si < source.len() || ti < target.len() {
            if li < lcs.len() {
                let (lcs_si, lcs_ti) = lcs[li];

                // Remove items before the next LCS match in source
                while si < lcs_si {
                    self.changes
                        .push(ChangeAction::Remove(AstNode::Expr(source[si].clone())));
                    si += 1;
                }
                // Insert items before the next LCS match in target
                while ti < lcs_ti {
                    self.changes
                        .push(ChangeAction::Insert(AstNode::Expr(target[ti].clone())));
                    ti += 1;
                }
                // Matched pair — recurse to find deeper changes
                self.diff_exprs(&source[si], &target[ti]);
                si += 1;
                ti += 1;
                li += 1;
            } else {
                // Remaining source items are removed
                while si < source.len() {
                    self.changes
                        .push(ChangeAction::Remove(AstNode::Expr(source[si].clone())));
                    si += 1;
                }
                // Remaining target items are inserted
                while ti < target.len() {
                    self.changes
                        .push(ChangeAction::Insert(AstNode::Expr(target[ti].clone())));
                    ti += 1;
                }
            }
        }
    }

    fn diff_select_items(&mut self, source: &[SelectItem], target: &[SelectItem]) {
        let min_len = source.len().min(target.len());
        for i in 0..min_len {
            if source[i] == target[i] {
                self.changes.push(ChangeAction::Keep(
                    AstNode::SelectItem(source[i].clone()),
                    AstNode::SelectItem(target[i].clone()),
                ));
            } else {
                match (&source[i], &target[i]) {
                    (
                        SelectItem::Expr {
                            expr: se,
                            alias: sa,
                            ..
                        },
                        SelectItem::Expr {
                            expr: te,
                            alias: ta,
                            ..
                        },
                    ) => {
                        if sa != ta {
                            self.changes.push(ChangeAction::Update(
                                AstNode::SelectItem(source[i].clone()),
                                AstNode::SelectItem(target[i].clone()),
                            ));
                        } else {
                            self.diff_exprs(se, te);
                        }
                    }
                    _ => {
                        self.changes.push(ChangeAction::Update(
                            AstNode::SelectItem(source[i].clone()),
                            AstNode::SelectItem(target[i].clone()),
                        ));
                    }
                }
            }
        }
        for item in source.iter().skip(min_len) {
            self.changes
                .push(ChangeAction::Remove(AstNode::SelectItem(item.clone())));
        }
        for item in target.iter().skip(min_len) {
            self.changes
                .push(ChangeAction::Insert(AstNode::SelectItem(item.clone())));
        }
    }

    fn diff_optional_exprs(&mut self, source: &Option<Expr>, target: &Option<Expr>) {
        match (source, target) {
            (Some(s), Some(t)) => self.diff_exprs(s, t),
            (None, Some(t)) => self
                .changes
                .push(ChangeAction::Insert(AstNode::Expr(t.clone()))),
            (Some(s), None) => self
                .changes
                .push(ChangeAction::Remove(AstNode::Expr(s.clone()))),
            (None, None) => {}
        }
    }

    fn diff_optional_boxed_exprs(
        &mut self,
        source: &Option<Box<Expr>>,
        target: &Option<Box<Expr>>,
    ) {
        match (source, target) {
            (Some(s), Some(t)) => self.diff_exprs(s, t),
            (None, Some(t)) => self
                .changes
                .push(ChangeAction::Insert(AstNode::Expr((**t).clone()))),
            (Some(s), None) => self
                .changes
                .push(ChangeAction::Remove(AstNode::Expr((**s).clone()))),
            (None, None) => {}
        }
    }

    fn diff_ctes(&mut self, source: &[Cte], target: &[Cte]) {
        // Match CTEs by name
        let source_map: HashMap<&str, &Cte> = source.iter().map(|c| (c.name.as_str(), c)).collect();
        let target_map: HashMap<&str, &Cte> = target.iter().map(|c| (c.name.as_str(), c)).collect();

        for (name, sc) in &source_map {
            if let Some(tc) = target_map.get(name) {
                if sc == tc {
                    self.changes.push(ChangeAction::Keep(
                        AstNode::Cte(Box::new((*sc).clone())),
                        AstNode::Cte(Box::new((*tc).clone())),
                    ));
                } else {
                    self.diff_statements(&sc.query, &tc.query);
                }
            } else {
                self.changes
                    .push(ChangeAction::Remove(AstNode::Cte(Box::new((*sc).clone()))));
            }
        }
        for (name, tc) in &target_map {
            if !source_map.contains_key(name) {
                self.changes
                    .push(ChangeAction::Insert(AstNode::Cte(Box::new((*tc).clone()))));
            }
        }
    }

    fn diff_joins(&mut self, source: &[JoinClause], target: &[JoinClause]) {
        let min_len = source.len().min(target.len());
        for i in 0..min_len {
            if source[i] == target[i] {
                self.changes.push(ChangeAction::Keep(
                    AstNode::JoinClause(source[i].clone()),
                    AstNode::JoinClause(target[i].clone()),
                ));
            } else if source[i].join_type == target[i].join_type {
                // Same join type, diff the contents
                self.diff_table_sources(&source[i].table, &target[i].table);
                self.diff_optional_exprs(&source[i].on, &target[i].on);
            } else {
                self.changes.push(ChangeAction::Update(
                    AstNode::JoinClause(source[i].clone()),
                    AstNode::JoinClause(target[i].clone()),
                ));
            }
        }
        for item in source.iter().skip(min_len) {
            self.changes
                .push(ChangeAction::Remove(AstNode::JoinClause(item.clone())));
        }
        for item in target.iter().skip(min_len) {
            self.changes
                .push(ChangeAction::Insert(AstNode::JoinClause(item.clone())));
        }
    }

    fn diff_order_by(&mut self, source: &[OrderByItem], target: &[OrderByItem]) {
        let min_len = source.len().min(target.len());
        for i in 0..min_len {
            if source[i] == target[i] {
                self.changes.push(ChangeAction::Keep(
                    AstNode::OrderByItem(source[i].clone()),
                    AstNode::OrderByItem(target[i].clone()),
                ));
            } else if source[i].ascending == target[i].ascending
                && source[i].nulls_first == target[i].nulls_first
            {
                self.diff_exprs(&source[i].expr, &target[i].expr);
            } else {
                self.changes.push(ChangeAction::Update(
                    AstNode::OrderByItem(source[i].clone()),
                    AstNode::OrderByItem(target[i].clone()),
                ));
            }
        }
        for item in source.iter().skip(min_len) {
            self.changes
                .push(ChangeAction::Remove(AstNode::OrderByItem(item.clone())));
        }
        for item in target.iter().skip(min_len) {
            self.changes
                .push(ChangeAction::Insert(AstNode::OrderByItem(item.clone())));
        }
    }

    fn diff_table_sources(&mut self, source: &TableSource, target: &TableSource) {
        if source == target {
            return;
        }
        match (source, target) {
            (TableSource::Table(st), TableSource::Table(tt)) => {
                if st != tt {
                    self.changes.push(ChangeAction::Update(
                        AstNode::Expr(table_ref_to_expr(st)),
                        AstNode::Expr(table_ref_to_expr(tt)),
                    ));
                }
            }
            (TableSource::Subquery { query: sq, .. }, TableSource::Subquery { query: tq, .. }) => {
                self.diff_statements(sq, tq);
            }
            _ => {
                // Different source types
                self.remove_table_source(source);
                self.insert_table_source(target);
            }
        }
    }

    fn insert_table_source(&mut self, source: &TableSource) {
        match source {
            TableSource::Table(t) => {
                self.changes
                    .push(ChangeAction::Insert(AstNode::Expr(table_ref_to_expr(t))));
            }
            TableSource::Subquery { query, .. } => {
                self.changes
                    .push(ChangeAction::Insert(AstNode::Statement(Box::new(
                        (**query).clone(),
                    ))));
            }
            other => {
                self.changes
                    .push(ChangeAction::Insert(AstNode::Expr(Expr::StringLiteral(
                        format!("{other:?}"),
                    ))));
            }
        }
    }

    fn remove_table_source(&mut self, source: &TableSource) {
        match source {
            TableSource::Table(t) => {
                self.changes
                    .push(ChangeAction::Remove(AstNode::Expr(table_ref_to_expr(t))));
            }
            TableSource::Subquery { query, .. } => {
                self.changes
                    .push(ChangeAction::Remove(AstNode::Statement(Box::new(
                        (**query).clone(),
                    ))));
            }
            other => {
                self.changes
                    .push(ChangeAction::Remove(AstNode::Expr(Expr::StringLiteral(
                        format!("{other:?}"),
                    ))));
            }
        }
    }

    fn diff_constraints(&mut self, source: &[TableConstraint], target: &[TableConstraint]) {
        // Simple positional diff for constraints
        let min_len = source.len().min(target.len());
        for i in 0..min_len {
            if source[i] == target[i] {
                self.changes.push(ChangeAction::Keep(
                    AstNode::TableConstraint(source[i].clone()),
                    AstNode::TableConstraint(target[i].clone()),
                ));
            } else {
                self.changes.push(ChangeAction::Update(
                    AstNode::TableConstraint(source[i].clone()),
                    AstNode::TableConstraint(target[i].clone()),
                ));
            }
        }
        for item in source.iter().skip(min_len) {
            self.changes
                .push(ChangeAction::Remove(AstNode::TableConstraint(item.clone())));
        }
        for item in target.iter().skip(min_len) {
            self.changes
                .push(ChangeAction::Insert(AstNode::TableConstraint(item.clone())));
        }
    }

    fn diff_string_lists(&mut self, source: &[String], target: &[String]) {
        for s in source {
            if !target.contains(s) {
                self.changes
                    .push(ChangeAction::Remove(AstNode::Expr(Expr::Column {
                        table: None,
                        name: s.clone(),
                        quote_style: QuoteStyle::None,
                        table_quote_style: QuoteStyle::None,
                    })));
            }
        }
        for t in target {
            if !source.contains(t) {
                self.changes
                    .push(ChangeAction::Insert(AstNode::Expr(Expr::Column {
                        table: None,
                        name: t.clone(),
                        quote_style: QuoteStyle::None,
                        table_quote_style: QuoteStyle::None,
                    })));
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// LCS — Longest Common Subsequence for ordered diff
// ═══════════════════════════════════════════════════════════════════════

/// Compute the longest common subsequence of two expression slices,
/// returning pairs of (source_index, target_index).
fn compute_lcs(source: &[Expr], target: &[Expr]) -> Vec<(usize, usize)> {
    let m = source.len();
    let n = target.len();
    if m == 0 || n == 0 {
        return Vec::new();
    }

    // Build DP table
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if source[i - 1] == target[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to find the actual subsequence indices
    let mut result = Vec::new();
    let mut i = m;
    let mut j = n;
    while i > 0 && j > 0 {
        if source[i - 1] == target[j - 1] {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

/// Convert a `TableRef` to an `Expr::Column` for uniform representation.
fn table_ref_to_expr(table: &TableRef) -> Expr {
    let full_name = match (&table.catalog, &table.schema) {
        (Some(c), Some(s)) => format!("{c}.{s}.{}", table.name),
        (None, Some(s)) => format!("{s}.{}", table.name),
        _ => table.name.clone(),
    };
    Expr::Column {
        table: table.schema.clone(),
        name: full_name,
        quote_style: table.name_quote_style,
        table_quote_style: QuoteStyle::None,
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Convenience: diff from SQL strings
// ═══════════════════════════════════════════════════════════════════════

/// Parse two SQL strings and compute their diff.
///
/// # Errors
///
/// Returns a [`SqlglotError`](crate::errors::SqlglotError) if either
/// string fails to parse.
pub fn diff_sql(
    source_sql: &str,
    target_sql: &str,
    dialect: crate::dialects::Dialect,
) -> crate::errors::Result<Vec<ChangeAction>> {
    let source = crate::parser::parse(source_sql, dialect)?;
    let target = crate::parser::parse(target_sql, dialect)?;
    Ok(diff(&source, &target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialects::Dialect;
    use crate::parser::parse;

    fn count_by_action(changes: &[ChangeAction]) -> (usize, usize, usize, usize, usize) {
        let mut keeps = 0;
        let mut inserts = 0;
        let mut removes = 0;
        let mut updates = 0;
        let mut moves = 0;
        for c in changes {
            match c {
                ChangeAction::Keep(..) => keeps += 1,
                ChangeAction::Insert(..) => inserts += 1,
                ChangeAction::Remove(..) => removes += 1,
                ChangeAction::Update(..) => updates += 1,
                ChangeAction::Move(..) => moves += 1,
            }
        }
        (keeps, inserts, removes, updates, moves)
    }

    #[test]
    fn test_identical_queries_are_all_keep() {
        let sql = "SELECT a, b FROM t WHERE a > 1";
        let source = parse(sql, Dialect::Ansi).unwrap();
        let target = parse(sql, Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (keeps, inserts, removes, updates, _moves) = count_by_action(&changes);
        assert!(keeps > 0, "should have keep actions");
        assert_eq!(inserts, 0, "no inserts for identical queries");
        assert_eq!(removes, 0, "no removes for identical queries");
        assert_eq!(updates, 0, "no updates for identical queries");
    }

    #[test]
    fn test_column_added() {
        let source = parse("SELECT a FROM t", Dialect::Ansi).unwrap();
        let target = parse("SELECT a, b FROM t", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (keeps, inserts, removes, _updates, _moves) = count_by_action(&changes);
        assert!(keeps > 0);
        assert!(inserts > 0, "should have insert for new column b");
        assert_eq!(removes, 0);
    }

    #[test]
    fn test_column_removed() {
        let source = parse("SELECT a, b FROM t", Dialect::Ansi).unwrap();
        let target = parse("SELECT a FROM t", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (keeps, _inserts, removes, _updates, _moves) = count_by_action(&changes);
        assert!(keeps > 0);
        assert!(removes > 0, "should have remove for column b");
    }

    #[test]
    fn test_column_changed() {
        let source = parse("SELECT a, b FROM t", Dialect::Ansi).unwrap();
        let target = parse("SELECT a, c FROM t", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (_keeps, _inserts, _removes, updates, _moves) = count_by_action(&changes);
        assert!(updates > 0, "should have update for b -> c");
    }

    #[test]
    fn test_where_clause_added() {
        let source = parse("SELECT a FROM t", Dialect::Ansi).unwrap();
        let target = parse("SELECT a FROM t WHERE a > 1", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (_keeps, inserts, _removes, _updates, _moves) = count_by_action(&changes);
        assert!(inserts > 0, "should have insert for WHERE clause");
    }

    #[test]
    fn test_where_clause_removed() {
        let source = parse("SELECT a FROM t WHERE a > 1", Dialect::Ansi).unwrap();
        let target = parse("SELECT a FROM t", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (_keeps, _inserts, removes, _updates, _moves) = count_by_action(&changes);
        assert!(removes > 0, "should have remove for WHERE clause");
    }

    #[test]
    fn test_where_clause_updated() {
        let source = parse("SELECT a FROM t WHERE a > 1", Dialect::Ansi).unwrap();
        let target = parse("SELECT a FROM t WHERE a > 2", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (_keeps, _inserts, _removes, updates, _moves) = count_by_action(&changes);
        assert!(updates > 0, "should have update for WHERE value change");
    }

    #[test]
    fn test_table_changed() {
        let source = parse("SELECT a FROM t1", Dialect::Ansi).unwrap();
        let target = parse("SELECT a FROM t2", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (_keeps, _inserts, _removes, updates, _moves) = count_by_action(&changes);
        assert!(updates > 0, "should have update for table change");
    }

    #[test]
    fn test_join_added() {
        let source = parse("SELECT a FROM t1", Dialect::Ansi).unwrap();
        let target = parse("SELECT a FROM t1 JOIN t2 ON t1.id = t2.id", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (_keeps, inserts, _removes, _updates, _moves) = count_by_action(&changes);
        assert!(inserts > 0, "should have insert for JOIN");
    }

    #[test]
    fn test_order_by_changed() {
        let source = parse("SELECT a FROM t ORDER BY a ASC", Dialect::Ansi).unwrap();
        let target = parse("SELECT a FROM t ORDER BY a DESC", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (_keeps, _inserts, _removes, updates, _moves) = count_by_action(&changes);
        assert!(updates > 0, "should have update for ORDER BY direction");
    }

    #[test]
    fn test_complex_nested_query() {
        let source = parse(
            "SELECT a, b FROM t1 WHERE a IN (SELECT x FROM t2 WHERE x > 0)",
            Dialect::Ansi,
        )
        .unwrap();
        let target = parse(
            "SELECT a, c FROM t1 WHERE a IN (SELECT x FROM t2 WHERE x > 5)",
            Dialect::Ansi,
        )
        .unwrap();
        let changes = diff(&source, &target);
        let (keeps, _inserts, _removes, updates, _moves) = count_by_action(&changes);
        assert!(keeps > 0, "unchanged parts should be kept");
        assert!(updates > 0, "changed parts should be updated (b->c, 0->5)");
    }

    #[test]
    fn test_different_statement_types() {
        let source = parse("SELECT a FROM t", Dialect::Ansi).unwrap();
        let target = parse("CREATE TABLE t (a INT)", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (_keeps, inserts, removes, _updates, _moves) = count_by_action(&changes);
        assert!(removes > 0, "source should be removed");
        assert!(inserts > 0, "target should be inserted");
    }

    #[test]
    fn test_cte_added() {
        let source = parse("SELECT a FROM t", Dialect::Ansi).unwrap();
        let target = parse("WITH cte AS (SELECT 1 AS x) SELECT a FROM t", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (_keeps, inserts, _removes, _updates, _moves) = count_by_action(&changes);
        assert!(inserts > 0, "should have insert for CTE");
    }

    #[test]
    fn test_limit_changed() {
        let source = parse("SELECT a FROM t LIMIT 10", Dialect::Ansi).unwrap();
        let target = parse("SELECT a FROM t LIMIT 20", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (_keeps, _inserts, _removes, updates, _moves) = count_by_action(&changes);
        assert!(updates > 0, "should have update for LIMIT change");
    }

    #[test]
    fn test_group_by_added() {
        let source = parse("SELECT a, COUNT(*) FROM t", Dialect::Ansi).unwrap();
        let target = parse("SELECT a, COUNT(*) FROM t GROUP BY a", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (_keeps, inserts, _removes, _updates, _moves) = count_by_action(&changes);
        assert!(inserts > 0, "should have insert for GROUP BY");
    }

    #[test]
    fn test_diff_sql_convenience() {
        let changes = diff_sql("SELECT a FROM t", "SELECT a, b FROM t", Dialect::Ansi).unwrap();
        let (_keeps, inserts, _removes, _updates, _moves) = count_by_action(&changes);
        assert!(inserts > 0);
    }

    #[test]
    fn test_having_added() {
        let source = parse("SELECT a, COUNT(*) FROM t GROUP BY a", Dialect::Ansi).unwrap();
        let target = parse(
            "SELECT a, COUNT(*) FROM t GROUP BY a HAVING COUNT(*) > 1",
            Dialect::Ansi,
        )
        .unwrap();
        let changes = diff(&source, &target);
        let (_keeps, inserts, _removes, _updates, _moves) = count_by_action(&changes);
        assert!(inserts > 0, "should have insert for HAVING");
    }

    #[test]
    fn test_create_table_column_diff() {
        let source = parse("CREATE TABLE t (a INT, b TEXT)", Dialect::Ansi).unwrap();
        let target = parse("CREATE TABLE t (a INT, c TEXT)", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (_keeps, inserts, removes, _updates, _moves) = count_by_action(&changes);
        assert!(removes > 0, "should remove column b");
        assert!(inserts > 0, "should insert column c");
    }

    #[test]
    fn test_union_diff() {
        let source = parse("SELECT a FROM t1 UNION SELECT b FROM t2", Dialect::Ansi).unwrap();
        let target = parse("SELECT a FROM t1 UNION SELECT c FROM t2", Dialect::Ansi).unwrap();
        let changes = diff(&source, &target);
        let (keeps, _inserts, _removes, updates, _moves) = count_by_action(&changes);
        assert!(keeps > 0);
        assert!(updates > 0, "should have update for b -> c");
    }
}
