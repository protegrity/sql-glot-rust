use crate::ast::*;

/// SQL code generator that converts an AST into a SQL string.
///
/// Supports all statement and expression types defined in the AST,
/// including CTEs, subqueries, UNION/INTERSECT/EXCEPT, CAST, window
/// functions, EXISTS, EXTRACT, INTERVAL, and more.
pub struct Generator {
    output: String,
    /// When true, emit formatted SQL with indentation and newlines.
    pretty: bool,
    indent: usize,
}

impl Generator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            output: String::new(),
            pretty: false,
            indent: 0,
        }
    }

    /// Create a generator that produces formatted SQL.
    #[must_use]
    pub fn pretty() -> Self {
        Self {
            output: String::new(),
            pretty: true,
            indent: 0,
        }
    }

    /// Generate SQL from a statement.
    #[must_use]
    pub fn generate(&mut self, statement: &Statement) -> String {
        self.output.clear();
        self.gen_statement(statement);
        self.output.clone()
    }

    /// Generate SQL for an expression (static helper for `Expr::sql()`).
    #[must_use]
    pub fn expr_to_sql(expr: &Expr) -> String {
        let mut g = Self::new();
        g.gen_expr(expr);
        g.output
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    /// Emit a newline followed by current indentation (pretty mode only).
    fn newline(&mut self) {
        if self.pretty {
            self.output.push('\n');
            for _ in 0..self.indent {
                self.output.push_str("  ");
            }
        }
    }

    /// In pretty mode: newline + indent. In compact mode: a single space.
    fn sep(&mut self) {
        if self.pretty {
            self.newline();
        } else {
            self.output.push(' ');
        }
    }

    fn indent_up(&mut self) {
        self.indent += 1;
    }

    fn indent_down(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    fn write_keyword(&mut self, s: &str) {
        self.write(s);
    }

    /// Write an identifier with the given quoting style.
    fn write_quoted(&mut self, name: &str, style: QuoteStyle) {
        match style {
            QuoteStyle::None => self.write(name),
            QuoteStyle::DoubleQuote => {
                self.write("\"");
                self.write(&name.replace('"', "\"\""));
                self.write("\"");
            }
            QuoteStyle::Backtick => {
                self.write("`");
                self.write(&name.replace('`', "``"));
                self.write("`");
            }
            QuoteStyle::Bracket => {
                self.write("[");
                self.write(&name.replace(']', "]]"));
                self.write("]");
            }
        }
    }

    // ══════════════════════════════════════════════════════════════
    // Statements
    // ══════════════════════════════════════════════════════════════

    fn gen_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Select(s) => self.gen_select(s),
            Statement::Insert(s) => self.gen_insert(s),
            Statement::Update(s) => self.gen_update(s),
            Statement::Delete(s) => self.gen_delete(s),
            Statement::CreateTable(s) => self.gen_create_table(s),
            Statement::DropTable(s) => self.gen_drop_table(s),
            Statement::SetOperation(s) => self.gen_set_operation(s),
            Statement::AlterTable(s) => self.gen_alter_table(s),
            Statement::CreateView(s) => self.gen_create_view(s),
            Statement::DropView(s) => self.gen_drop_view(s),
            Statement::Truncate(s) => self.gen_truncate(s),
            Statement::Transaction(s) => self.gen_transaction(s),
            Statement::Explain(s) => self.gen_explain(s),
            Statement::Use(s) => self.gen_use(s),
            Statement::Expression(e) => self.gen_expr(e),
        }
    }

    // ── SELECT ──────────────────────────────────────────────────

    fn gen_select(&mut self, sel: &SelectStatement) {
        // CTEs
        if !sel.ctes.is_empty() {
            self.gen_ctes(&sel.ctes);
            self.sep();
        }

        self.write_keyword("SELECT");
        if sel.distinct {
            self.write(" ");
            self.write_keyword("DISTINCT");
        }
        if let Some(top) = &sel.top {
            self.write(" ");
            self.write_keyword("TOP ");
            self.gen_expr(top);
        }

        // columns
        if self.pretty {
            self.indent_up();
            for (i, item) in sel.columns.iter().enumerate() {
                self.newline();
                self.gen_select_item(item);
                if i < sel.columns.len() - 1 {
                    self.write(",");
                }
            }
            self.indent_down();
        } else {
            self.write(" ");
            for (i, item) in sel.columns.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.gen_select_item(item);
            }
        }

        if let Some(from) = &sel.from {
            self.sep();
            self.write_keyword("FROM");
            if self.pretty {
                self.indent_up();
                self.newline();
                self.gen_table_source(&from.source);
                self.indent_down();
            } else {
                self.write(" ");
                self.gen_table_source(&from.source);
            }
        }

        for join in &sel.joins {
            self.gen_join(join);
        }

        if let Some(wh) = &sel.where_clause {
            self.sep();
            self.write_keyword("WHERE");
            if self.pretty {
                self.indent_up();
                self.newline();
                self.gen_expr(wh);
                self.indent_down();
            } else {
                self.write(" ");
                self.gen_expr(wh);
            }
        }

        if !sel.group_by.is_empty() {
            self.sep();
            self.write_keyword("GROUP BY");
            if self.pretty {
                self.indent_up();
                self.newline();
                self.gen_expr_list(&sel.group_by);
                self.indent_down();
            } else {
                self.write(" ");
                self.gen_expr_list(&sel.group_by);
            }
        }

        if let Some(having) = &sel.having {
            self.sep();
            self.write_keyword("HAVING");
            if self.pretty {
                self.indent_up();
                self.newline();
                self.gen_expr(having);
                self.indent_down();
            } else {
                self.write(" ");
                self.gen_expr(having);
            }
        }

        if let Some(qualify) = &sel.qualify {
            self.sep();
            self.write_keyword("QUALIFY");
            if self.pretty {
                self.indent_up();
                self.newline();
                self.gen_expr(qualify);
                self.indent_down();
            } else {
                self.write(" ");
                self.gen_expr(qualify);
            }
        }

        if !sel.window_definitions.is_empty() {
            self.sep();
            self.write_keyword("WINDOW ");
            for (i, wd) in sel.window_definitions.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&wd.name);
                self.write(" AS (");
                self.gen_window_spec(&wd.spec);
                self.write(")");
            }
        }

        self.gen_order_by(&sel.order_by);

        if let Some(limit) = &sel.limit {
            self.sep();
            self.write_keyword("LIMIT ");
            self.gen_expr(limit);
        }

        if let Some(offset) = &sel.offset {
            self.sep();
            self.write_keyword("OFFSET ");
            self.gen_expr(offset);
        }

        if let Some(fetch) = &sel.fetch_first {
            self.sep();
            self.write_keyword("FETCH FIRST ");
            self.gen_expr(fetch);
            self.write(" ");
            self.write_keyword("ROWS ONLY");
        }
    }

    fn gen_ctes(&mut self, ctes: &[Cte]) {
        self.write_keyword("WITH ");
        if ctes.iter().any(|c| c.recursive) {
            self.write_keyword("RECURSIVE ");
        }
        for (i, cte) in ctes.iter().enumerate() {
            if i > 0 {
                self.write(",");
                self.sep();
            }
            self.write(&cte.name);
            if !cte.columns.is_empty() {
                self.write("(");
                self.write(&cte.columns.join(", "));
                self.write(")");
            }
            self.write(" ");
            self.write_keyword("AS ");
            if let Some(true) = cte.materialized {
                self.write_keyword("MATERIALIZED ");
            } else if let Some(false) = cte.materialized {
                self.write_keyword("NOT MATERIALIZED ");
            }
            self.write("(");
            if self.pretty {
                self.indent_up();
                self.newline();
                self.gen_statement(&cte.query);
                self.indent_down();
                self.newline();
            } else {
                self.gen_statement(&cte.query);
            }
            self.write(")");
        }
    }

    fn gen_select_item(&mut self, item: &SelectItem) {
        match item {
            SelectItem::Wildcard => self.write("*"),
            SelectItem::QualifiedWildcard { table } => {
                self.write(table);
                self.write(".*");
            }
            SelectItem::Expr { expr, alias } => {
                self.gen_expr(expr);
                if let Some(alias) = alias {
                    self.write(" ");
                    self.write_keyword("AS ");
                    self.write(alias);
                }
            }
        }
    }

    fn gen_table_source(&mut self, source: &TableSource) {
        match source {
            TableSource::Table(table_ref) => self.gen_table_ref(table_ref),
            TableSource::Subquery { query, alias } => {
                self.write("(");
                self.gen_statement(query);
                self.write(")");
                if let Some(alias) = alias {
                    self.write(" ");
                    self.write_keyword("AS ");
                    self.write(alias);
                }
            }
            TableSource::TableFunction { name, args, alias } => {
                self.write(name);
                self.write("(");
                self.gen_expr_list(args);
                self.write(")");
                if let Some(alias) = alias {
                    self.write(" ");
                    self.write_keyword("AS ");
                    self.write(alias);
                }
            }
            TableSource::Lateral { source } => {
                self.write_keyword("LATERAL ");
                self.gen_table_source(source);
            }
            TableSource::Unnest { expr, alias, with_offset } => {
                self.write_keyword("UNNEST(");
                self.gen_expr(expr);
                self.write(")");
                if let Some(alias) = alias {
                    self.write(" ");
                    self.write_keyword("AS ");
                    self.write(alias);
                }
                if *with_offset {
                    self.write(" ");
                    self.write_keyword("WITH OFFSET");
                }
            }
        }
    }

    fn gen_table_ref(&mut self, table: &TableRef) {
        if let Some(catalog) = &table.catalog {
            self.write(catalog);
            self.write(".");
        }
        if let Some(schema) = &table.schema {
            self.write(schema);
            self.write(".");
        }
        self.write_quoted(&table.name, table.name_quote_style);
        if let Some(alias) = &table.alias {
            self.write(" ");
            self.write_keyword("AS ");
            self.write(alias);
        }
    }

    fn gen_join(&mut self, join: &JoinClause) {
        let join_kw = match join.join_type {
            JoinType::Inner => "INNER JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
            JoinType::Full => "FULL JOIN",
            JoinType::Cross => "CROSS JOIN",
            JoinType::Natural => "NATURAL JOIN",
            JoinType::Lateral => "LATERAL JOIN",
        };
        self.sep();
        self.write_keyword(join_kw);
        if self.pretty {
            self.indent_up();
            self.newline();
            self.gen_table_source(&join.table);
        } else {
            self.write(" ");
            self.gen_table_source(&join.table);
        }
        if let Some(on) = &join.on {
            if self.pretty {
                self.newline();
            } else {
                self.write(" ");
            }
            self.write_keyword("ON ");
            self.gen_expr(on);
        }
        if !join.using.is_empty() {
            if self.pretty {
                self.newline();
            } else {
                self.write(" ");
            }
            self.write_keyword("USING (");
            self.write(&join.using.join(", "));
            self.write(")");
        }
        if self.pretty {
            self.indent_down();
        }
    }

    fn gen_order_by(&mut self, items: &[OrderByItem]) {
        if items.is_empty() {
            return;
        }
        self.sep();
        self.write_keyword("ORDER BY");
        if self.pretty {
            self.indent_up();
            self.newline();
        } else {
            self.write(" ");
        }
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.gen_expr(&item.expr);
            if !item.ascending {
                self.write(" ");
                self.write_keyword("DESC");
            }
            if let Some(nulls_first) = item.nulls_first {
                if nulls_first {
                    self.write(" ");
                    self.write_keyword("NULLS FIRST");
                } else {
                    self.write(" ");
                    self.write_keyword("NULLS LAST");
                }
            }
        }
        if self.pretty {
            self.indent_down();
        }
    }

    // ── Set operations ──────────────────────────────────────────

    fn gen_set_operation(&mut self, sop: &SetOperationStatement) {
        self.gen_statement(&sop.left);
        let op_kw = match sop.op {
            SetOperationType::Union => "UNION",
            SetOperationType::Intersect => "INTERSECT",
            SetOperationType::Except => "EXCEPT",
        };
        self.sep();
        self.write_keyword(op_kw);
        if sop.all {
            self.write(" ");
            self.write_keyword("ALL");
        }
        self.sep();
        self.gen_statement(&sop.right);

        self.gen_order_by(&sop.order_by);

        if let Some(limit) = &sop.limit {
            self.sep();
            self.write_keyword("LIMIT ");
            self.gen_expr(limit);
        }
        if let Some(offset) = &sop.offset {
            self.sep();
            self.write_keyword("OFFSET ");
            self.gen_expr(offset);
        }
    }

    // ── INSERT ──────────────────────────────────────────────────

    fn gen_insert(&mut self, ins: &InsertStatement) {
        self.write_keyword("INSERT INTO ");
        self.gen_table_ref(&ins.table);

        if !ins.columns.is_empty() {
            self.write(" (");
            self.write(&ins.columns.join(", "));
            self.write(")");
        }

        match &ins.source {
            InsertSource::Values(rows) => {
                self.sep();
                self.write_keyword("VALUES");
                if self.pretty {
                    self.indent_up();
                    for (i, row) in rows.iter().enumerate() {
                        self.newline();
                        self.write("(");
                        self.gen_expr_list(row);
                        self.write(")");
                        if i < rows.len() - 1 {
                            self.write(",");
                        }
                    }
                    self.indent_down();
                } else {
                    self.write(" ");
                    for (i, row) in rows.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write("(");
                        self.gen_expr_list(row);
                        self.write(")");
                    }
                }
            }
            InsertSource::Query(query) => {
                self.sep();
                self.gen_statement(query);
            }
            InsertSource::Default => {
                self.sep();
                self.write_keyword("DEFAULT VALUES");
            }
        }

        if let Some(on_conflict) = &ins.on_conflict {
            self.sep();
            self.write_keyword("ON CONFLICT");
            if !on_conflict.columns.is_empty() {
                self.write(" (");
                self.write(&on_conflict.columns.join(", "));
                self.write(")");
            }
            match &on_conflict.action {
                ConflictAction::DoNothing => {
                    self.write(" ");
                    self.write_keyword("DO NOTHING");
                }
                ConflictAction::DoUpdate(assignments) => {
                    self.write(" ");
                    self.write_keyword("DO UPDATE SET ");
                    for (i, (col, val)) in assignments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write(col);
                        self.write(" = ");
                        self.gen_expr(val);
                    }
                }
            }
        }

        if !ins.returning.is_empty() {
            self.sep();
            self.write_keyword("RETURNING ");
            for (i, item) in ins.returning.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.gen_select_item(item);
            }
        }
    }

    // ── UPDATE ──────────────────────────────────────────────────

    fn gen_update(&mut self, upd: &UpdateStatement) {
        self.write_keyword("UPDATE ");
        self.gen_table_ref(&upd.table);
        self.sep();
        self.write_keyword("SET");

        if self.pretty {
            self.indent_up();
            for (i, (col, val)) in upd.assignments.iter().enumerate() {
                self.newline();
                self.write(col);
                self.write(" = ");
                self.gen_expr(val);
                if i < upd.assignments.len() - 1 {
                    self.write(",");
                }
            }
            self.indent_down();
        } else {
            self.write(" ");
            for (i, (col, val)) in upd.assignments.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(col);
                self.write(" = ");
                self.gen_expr(val);
            }
        }

        if let Some(from) = &upd.from {
            self.sep();
            self.write_keyword("FROM ");
            self.gen_table_source(&from.source);
        }

        if let Some(wh) = &upd.where_clause {
            self.sep();
            self.write_keyword("WHERE");
            if self.pretty {
                self.indent_up();
                self.newline();
                self.gen_expr(wh);
                self.indent_down();
            } else {
                self.write(" ");
                self.gen_expr(wh);
            }
        }

        if !upd.returning.is_empty() {
            self.sep();
            self.write_keyword("RETURNING ");
            for (i, item) in upd.returning.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.gen_select_item(item);
            }
        }
    }

    // ── DELETE ──────────────────────────────────────────────────

    fn gen_delete(&mut self, del: &DeleteStatement) {
        self.write_keyword("DELETE FROM ");
        self.gen_table_ref(&del.table);

        if let Some(using) = &del.using {
            self.sep();
            self.write_keyword("USING ");
            self.gen_table_source(&using.source);
        }

        if let Some(wh) = &del.where_clause {
            self.sep();
            self.write_keyword("WHERE");
            if self.pretty {
                self.indent_up();
                self.newline();
                self.gen_expr(wh);
                self.indent_down();
            } else {
                self.write(" ");
                self.gen_expr(wh);
            }
        }

        if !del.returning.is_empty() {
            self.sep();
            self.write_keyword("RETURNING ");
            for (i, item) in del.returning.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.gen_select_item(item);
            }
        }
    }

    // ── CREATE TABLE ────────────────────────────────────────────

    fn gen_create_table(&mut self, ct: &CreateTableStatement) {
        self.write_keyword("CREATE ");
        if ct.temporary {
            self.write_keyword("TEMPORARY ");
        }
        self.write_keyword("TABLE ");
        if ct.if_not_exists {
            self.write_keyword("IF NOT EXISTS ");
        }
        self.gen_table_ref(&ct.table);

        if let Some(as_select) = &ct.as_select {
            self.write(" ");
            self.write_keyword("AS ");
            self.gen_statement(as_select);
            return;
        }

        self.write(" (");

        if self.pretty {
            self.indent_up();
            for (i, col) in ct.columns.iter().enumerate() {
                self.newline();
                self.gen_column_def(col);
                if i < ct.columns.len() - 1 || !ct.constraints.is_empty() {
                    self.write(",");
                }
            }
            for (i, constraint) in ct.constraints.iter().enumerate() {
                self.newline();
                self.gen_table_constraint(constraint);
                if i < ct.constraints.len() - 1 {
                    self.write(",");
                }
            }
            self.indent_down();
            self.newline();
        } else {
            for (i, col) in ct.columns.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.gen_column_def(col);
            }
            for (i, constraint) in ct.constraints.iter().enumerate() {
                if i + ct.columns.len() > 0 {
                    self.write(", ");
                }
                self.gen_table_constraint(constraint);
            }
        }

        self.write(")");
    }

    fn gen_column_def(&mut self, col: &ColumnDef) {
        self.write(&col.name);
        self.write(" ");
        self.gen_data_type(&col.data_type);

        if col.primary_key {
            self.write(" ");
            self.write_keyword("PRIMARY KEY");
        }
        if col.unique {
            self.write(" ");
            self.write_keyword("UNIQUE");
        }
        if col.auto_increment {
            self.write(" ");
            self.write_keyword("AUTOINCREMENT");
        }

        match col.nullable {
            Some(false) => {
                self.write(" ");
                self.write_keyword("NOT NULL");
            }
            Some(true) => {
                self.write(" ");
                self.write_keyword("NULL");
            }
            None => {}
        }

        if let Some(default) = &col.default {
            self.write(" ");
            self.write_keyword("DEFAULT ");
            self.gen_expr(default);
        }

        if let Some(collation) = &col.collation {
            self.write(" ");
            self.write_keyword("COLLATE ");
            self.write(collation);
        }

        if let Some(comment) = &col.comment {
            self.write(" ");
            self.write_keyword("COMMENT '");
            self.write(&comment.replace('\'', "''"));
            self.write("'");
        }
    }

    fn gen_table_constraint(&mut self, constraint: &TableConstraint) {
        match constraint {
            TableConstraint::PrimaryKey { name, columns } => {
                if let Some(name) = name {
                    self.write_keyword("CONSTRAINT ");
                    self.write(name);
                    self.write(" ");
                }
                self.write_keyword("PRIMARY KEY (");
                self.write(&columns.join(", "));
                self.write(")");
            }
            TableConstraint::Unique { name, columns } => {
                if let Some(name) = name {
                    self.write_keyword("CONSTRAINT ");
                    self.write(name);
                    self.write(" ");
                }
                self.write_keyword("UNIQUE (");
                self.write(&columns.join(", "));
                self.write(")");
            }
            TableConstraint::ForeignKey {
                name,
                columns,
                ref_table,
                ref_columns,
                on_delete,
                on_update,
            } => {
                if let Some(name) = name {
                    self.write_keyword("CONSTRAINT ");
                    self.write(name);
                    self.write(" ");
                }
                self.write_keyword("FOREIGN KEY (");
                self.write(&columns.join(", "));
                self.write(") ");
                self.write_keyword("REFERENCES ");
                self.gen_table_ref(ref_table);
                self.write(" (");
                self.write(&ref_columns.join(", "));
                self.write(")");
                if let Some(action) = on_delete {
                    self.write(" ");
                    self.write_keyword("ON DELETE ");
                    self.gen_referential_action(action);
                }
                if let Some(action) = on_update {
                    self.write(" ");
                    self.write_keyword("ON UPDATE ");
                    self.gen_referential_action(action);
                }
            }
            TableConstraint::Check { name, expr } => {
                if let Some(name) = name {
                    self.write_keyword("CONSTRAINT ");
                    self.write(name);
                    self.write(" ");
                }
                self.write_keyword("CHECK (");
                self.gen_expr(expr);
                self.write(")");
            }
        }
    }

    fn gen_referential_action(&mut self, action: &ReferentialAction) {
        match action {
            ReferentialAction::Cascade => self.write_keyword("CASCADE"),
            ReferentialAction::Restrict => self.write_keyword("RESTRICT"),
            ReferentialAction::NoAction => self.write_keyword("NO ACTION"),
            ReferentialAction::SetNull => self.write_keyword("SET NULL"),
            ReferentialAction::SetDefault => self.write_keyword("SET DEFAULT"),
        }
    }

    // ── DROP TABLE ──────────────────────────────────────────────

    fn gen_drop_table(&mut self, dt: &DropTableStatement) {
        self.write_keyword("DROP TABLE ");
        if dt.if_exists {
            self.write_keyword("IF EXISTS ");
        }
        self.gen_table_ref(&dt.table);
        if dt.cascade {
            self.write(" ");
            self.write_keyword("CASCADE");
        }
    }

    // ── ALTER TABLE ─────────────────────────────────────────────

    fn gen_alter_table(&mut self, alt: &AlterTableStatement) {
        self.write_keyword("ALTER TABLE ");
        self.gen_table_ref(&alt.table);

        for (i, action) in alt.actions.iter().enumerate() {
            if i > 0 {
                self.write(",");
            }
            self.write(" ");
            match action {
                AlterTableAction::AddColumn(col) => {
                    self.write_keyword("ADD COLUMN ");
                    self.gen_column_def(col);
                }
                AlterTableAction::DropColumn { name, if_exists } => {
                    self.write_keyword("DROP COLUMN ");
                    if *if_exists {
                        self.write_keyword("IF EXISTS ");
                    }
                    self.write(name);
                }
                AlterTableAction::RenameColumn { old_name, new_name } => {
                    self.write_keyword("RENAME COLUMN ");
                    self.write(old_name);
                    self.write(" ");
                    self.write_keyword("TO ");
                    self.write(new_name);
                }
                AlterTableAction::AlterColumnType { name, data_type } => {
                    self.write_keyword("ALTER COLUMN ");
                    self.write(name);
                    self.write(" ");
                    self.write_keyword("TYPE ");
                    self.gen_data_type(data_type);
                }
                AlterTableAction::AddConstraint(constraint) => {
                    self.write_keyword("ADD ");
                    self.gen_table_constraint(constraint);
                }
                AlterTableAction::DropConstraint { name } => {
                    self.write_keyword("DROP CONSTRAINT ");
                    self.write(name);
                }
                AlterTableAction::RenameTable { new_name } => {
                    self.write_keyword("RENAME TO ");
                    self.write(new_name);
                }
            }
        }
    }

    // ── CREATE / DROP VIEW ──────────────────────────────────────

    fn gen_create_view(&mut self, cv: &CreateViewStatement) {
        self.write_keyword("CREATE ");
        if cv.or_replace {
            self.write_keyword("OR REPLACE ");
        }
        if cv.materialized {
            self.write_keyword("MATERIALIZED ");
        }
        self.write_keyword("VIEW ");
        if cv.if_not_exists {
            self.write_keyword("IF NOT EXISTS ");
        }
        self.gen_table_ref(&cv.name);

        if !cv.columns.is_empty() {
            self.write(" (");
            self.write(&cv.columns.join(", "));
            self.write(")");
        }

        self.write(" ");
        self.write_keyword("AS ");
        self.gen_statement(&cv.query);
    }

    fn gen_drop_view(&mut self, dv: &DropViewStatement) {
        self.write_keyword("DROP ");
        if dv.materialized {
            self.write_keyword("MATERIALIZED ");
        }
        self.write_keyword("VIEW ");
        if dv.if_exists {
            self.write_keyword("IF EXISTS ");
        }
        self.gen_table_ref(&dv.name);
    }

    // ── TRUNCATE ────────────────────────────────────────────────

    fn gen_truncate(&mut self, t: &TruncateStatement) {
        self.write_keyword("TRUNCATE TABLE ");
        self.gen_table_ref(&t.table);
    }

    // ── Transaction ─────────────────────────────────────────────

    fn gen_transaction(&mut self, t: &TransactionStatement) {
        match t {
            TransactionStatement::Begin => self.write_keyword("BEGIN"),
            TransactionStatement::Commit => self.write_keyword("COMMIT"),
            TransactionStatement::Rollback => self.write_keyword("ROLLBACK"),
            TransactionStatement::Savepoint(name) => {
                self.write_keyword("SAVEPOINT ");
                self.write(name);
            }
            TransactionStatement::ReleaseSavepoint(name) => {
                self.write_keyword("RELEASE SAVEPOINT ");
                self.write(name);
            }
            TransactionStatement::RollbackTo(name) => {
                self.write_keyword("ROLLBACK TO SAVEPOINT ");
                self.write(name);
            }
        }
    }

    // ── EXPLAIN ─────────────────────────────────────────────────

    fn gen_explain(&mut self, e: &ExplainStatement) {
        self.write_keyword("EXPLAIN ");
        if e.analyze {
            self.write_keyword("ANALYZE ");
        }
        self.gen_statement(&e.statement);
    }

    // ── USE ─────────────────────────────────────────────────────

    fn gen_use(&mut self, u: &UseStatement) {
        self.write_keyword("USE ");
        self.write(&u.name);
    }

    // ══════════════════════════════════════════════════════════════
    // Data types
    // ══════════════════════════════════════════════════════════════

    fn gen_data_type(&mut self, dt: &DataType) {
        match dt {
            DataType::TinyInt => self.write("TINYINT"),
            DataType::SmallInt => self.write("SMALLINT"),
            DataType::Int => self.write("INT"),
            DataType::BigInt => self.write("BIGINT"),
            DataType::Float => self.write("FLOAT"),
            DataType::Double => self.write("DOUBLE"),
            DataType::Real => self.write("REAL"),
            DataType::Decimal { precision, scale } | DataType::Numeric { precision, scale } => {
                self.write(if matches!(dt, DataType::Numeric { .. }) { "NUMERIC" } else { "DECIMAL" });
                if let Some(p) = precision {
                    self.write(&format!("({p}"));
                    if let Some(s) = scale {
                        self.write(&format!(", {s}"));
                    }
                    self.write(")");
                }
            }
            DataType::Varchar(len) => {
                self.write("VARCHAR");
                if let Some(n) = len {
                    self.write(&format!("({n})"));
                }
            }
            DataType::Char(len) => {
                self.write("CHAR");
                if let Some(n) = len {
                    self.write(&format!("({n})"));
                }
            }
            DataType::Text => self.write("TEXT"),
            DataType::String => self.write("STRING"),
            DataType::Binary(len) => {
                self.write("BINARY");
                if let Some(n) = len {
                    self.write(&format!("({n})"));
                }
            }
            DataType::Varbinary(len) => {
                self.write("VARBINARY");
                if let Some(n) = len {
                    self.write(&format!("({n})"));
                }
            }
            DataType::Boolean => self.write("BOOLEAN"),
            DataType::Date => self.write("DATE"),
            DataType::Time { precision } => {
                self.write("TIME");
                if let Some(p) = precision {
                    self.write(&format!("({p})"));
                }
            }
            DataType::Timestamp { precision, with_tz } => {
                self.write("TIMESTAMP");
                if let Some(p) = precision {
                    self.write(&format!("({p})"));
                }
                if *with_tz {
                    self.write(" WITH TIME ZONE");
                }
            }
            DataType::Interval => self.write("INTERVAL"),
            DataType::DateTime => self.write("DATETIME"),
            DataType::Blob => self.write("BLOB"),
            DataType::Bytea => self.write("BYTEA"),
            DataType::Bytes => self.write("BYTES"),
            DataType::Json => self.write("JSON"),
            DataType::Jsonb => self.write("JSONB"),
            DataType::Uuid => self.write("UUID"),
            DataType::Array(inner) => {
                self.write("ARRAY");
                if let Some(inner) = inner {
                    self.write("<");
                    self.gen_data_type(inner);
                    self.write(">");
                }
            }
            DataType::Map { key, value } => {
                self.write("MAP<");
                self.gen_data_type(key);
                self.write(", ");
                self.gen_data_type(value);
                self.write(">");
            }
            DataType::Struct(fields) => {
                self.write("STRUCT<");
                for (i, (name, dt)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(name);
                    self.write(" ");
                    self.gen_data_type(dt);
                }
                self.write(">");
            }
            DataType::Tuple(types) => {
                self.write("TUPLE(");
                for (i, dt) in types.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.gen_data_type(dt);
                }
                self.write(")");
            }
            DataType::Null => self.write("NULL"),
            DataType::Variant => self.write("VARIANT"),
            DataType::Object => self.write("OBJECT"),
            DataType::Xml => self.write("XML"),
            DataType::Inet => self.write("INET"),
            DataType::Cidr => self.write("CIDR"),
            DataType::Macaddr => self.write("MACADDR"),
            DataType::Bit(len) => {
                self.write("BIT");
                if let Some(n) = len {
                    self.write(&format!("({n})"));
                }
            }
            DataType::Money => self.write("MONEY"),
            DataType::Serial => self.write("SERIAL"),
            DataType::BigSerial => self.write("BIGSERIAL"),
            DataType::SmallSerial => self.write("SMALLSERIAL"),
            DataType::Regclass => self.write("REGCLASS"),
            DataType::Regtype => self.write("REGTYPE"),
            DataType::Hstore => self.write("HSTORE"),
            DataType::Geography => self.write("GEOGRAPHY"),
            DataType::Geometry => self.write("GEOMETRY"),
            DataType::Super => self.write("SUPER"),
            DataType::Unknown(name) => self.write(name),
        }
    }

    // ══════════════════════════════════════════════════════════════
    // Expressions
    // ══════════════════════════════════════════════════════════════

    fn gen_expr_list(&mut self, exprs: &[Expr]) {
        for (i, expr) in exprs.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.gen_expr(expr);
        }
    }

    fn gen_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Column { table, name, quote_style, table_quote_style } => {
                if let Some(t) = table {
                    self.write_quoted(t, *table_quote_style);
                    self.write(".");
                }
                self.write_quoted(name, *quote_style);
            }
            Expr::Number(n) => self.write(n),
            Expr::StringLiteral(s) => {
                self.write("'");
                self.write(&s.replace('\'', "''"));
                self.write("'");
            }
            Expr::Boolean(b) => self.write(if *b { "TRUE" } else { "FALSE" }),
            Expr::Null => self.write("NULL"),
            Expr::Default => self.write_keyword("DEFAULT"),
            Expr::Wildcard | Expr::Star => self.write("*"),

            Expr::BinaryOp { left, op, right } => {
                self.gen_expr(left);
                let op_str = match op {
                    BinaryOperator::Plus => " + ",
                    BinaryOperator::Minus => " - ",
                    BinaryOperator::Multiply => " * ",
                    BinaryOperator::Divide => " / ",
                    BinaryOperator::Modulo => " % ",
                    BinaryOperator::Eq => " = ",
                    BinaryOperator::Neq => " <> ",
                    BinaryOperator::Lt => " < ",
                    BinaryOperator::Gt => " > ",
                    BinaryOperator::LtEq => " <= ",
                    BinaryOperator::GtEq => " >= ",
                    BinaryOperator::And => " AND ",
                    BinaryOperator::Or => " OR ",
                    BinaryOperator::Xor => " XOR ",
                    BinaryOperator::Concat => " || ",
                    BinaryOperator::BitwiseAnd => " & ",
                    BinaryOperator::BitwiseOr => " | ",
                    BinaryOperator::BitwiseXor => " ^ ",
                    BinaryOperator::ShiftLeft => " << ",
                    BinaryOperator::ShiftRight => " >> ",
                    BinaryOperator::Arrow => " -> ",
                    BinaryOperator::DoubleArrow => " ->> ",
                };
                self.write(op_str);
                self.gen_expr(right);
            }
            Expr::UnaryOp { op, expr } => {
                let op_str = match op {
                    UnaryOperator::Not => "NOT ",
                    UnaryOperator::Minus => "-",
                    UnaryOperator::Plus => "+",
                    UnaryOperator::BitwiseNot => "~",
                };
                self.write(op_str);
                self.gen_expr(expr);
            }
            Expr::Function { name, args, distinct, filter, over } => {
                self.write(name);
                self.write("(");
                if *distinct {
                    self.write_keyword("DISTINCT ");
                }
                self.gen_expr_list(args);
                self.write(")");

                if let Some(filter_expr) = filter {
                    self.write(" ");
                    self.write_keyword("FILTER (WHERE ");
                    self.gen_expr(filter_expr);
                    self.write(")");
                }
                if let Some(spec) = over {
                    self.write(" ");
                    self.write_keyword("OVER ");
                    if let Some(wref) = &spec.window_ref {
                        if spec.partition_by.is_empty() && spec.order_by.is_empty() && spec.frame.is_none() {
                            self.write(wref);
                        } else {
                            self.write("(");
                            self.gen_window_spec(spec);
                            self.write(")");
                        }
                    } else {
                        self.write("(");
                        self.gen_window_spec(spec);
                        self.write(")");
                    }
                }
            }
            Expr::Between { expr, low, high, negated } => {
                self.gen_expr(expr);
                if *negated {
                    self.write(" ");
                    self.write_keyword("NOT");
                }
                self.write(" ");
                self.write_keyword("BETWEEN ");
                self.gen_expr(low);
                self.write(" ");
                self.write_keyword("AND ");
                self.gen_expr(high);
            }
            Expr::InList { expr, list, negated } => {
                self.gen_expr(expr);
                if *negated {
                    self.write(" ");
                    self.write_keyword("NOT");
                }
                self.write(" ");
                self.write_keyword("IN (");
                self.gen_expr_list(list);
                self.write(")");
            }
            Expr::InSubquery { expr, subquery, negated } => {
                self.gen_expr(expr);
                if *negated {
                    self.write(" ");
                    self.write_keyword("NOT");
                }
                self.write(" ");
                self.write_keyword("IN (");
                self.gen_statement(subquery);
                self.write(")");
            }
            Expr::IsNull { expr, negated } => {
                self.gen_expr(expr);
                if *negated {
                    self.write(" ");
                    self.write_keyword("IS NOT NULL");
                } else {
                    self.write(" ");
                    self.write_keyword("IS NULL");
                }
            }
            Expr::Like { expr, pattern, negated, escape } => {
                self.gen_expr(expr);
                if *negated {
                    self.write(" ");
                    self.write_keyword("NOT");
                }
                self.write(" ");
                self.write_keyword("LIKE ");
                self.gen_expr(pattern);
                if let Some(esc) = escape {
                    self.write(" ");
                    self.write_keyword("ESCAPE ");
                    self.gen_expr(esc);
                }
            }
            Expr::ILike { expr, pattern, negated, escape } => {
                self.gen_expr(expr);
                if *negated {
                    self.write(" ");
                    self.write_keyword("NOT");
                }
                self.write(" ");
                self.write_keyword("ILIKE ");
                self.gen_expr(pattern);
                if let Some(esc) = escape {
                    self.write(" ");
                    self.write_keyword("ESCAPE ");
                    self.gen_expr(esc);
                }
            }
            Expr::Case { operand, when_clauses, else_clause } => {
                self.write_keyword("CASE");
                if let Some(op) = operand {
                    self.write(" ");
                    self.gen_expr(op);
                }
                for (cond, result) in when_clauses {
                    self.write(" ");
                    self.write_keyword("WHEN ");
                    self.gen_expr(cond);
                    self.write(" ");
                    self.write_keyword("THEN ");
                    self.gen_expr(result);
                }
                if let Some(el) = else_clause {
                    self.write(" ");
                    self.write_keyword("ELSE ");
                    self.gen_expr(el);
                }
                self.write(" ");
                self.write_keyword("END");
            }
            Expr::Nested(inner) => {
                self.write("(");
                self.gen_expr(inner);
                self.write(")");
            }
            Expr::Subquery(query) => {
                self.write("(");
                self.gen_statement(query);
                self.write(")");
            }
            Expr::Exists { subquery, negated } => {
                if *negated {
                    self.write_keyword("NOT ");
                }
                self.write_keyword("EXISTS (");
                self.gen_statement(subquery);
                self.write(")");
            }
            Expr::Cast { expr, data_type } => {
                self.write_keyword("CAST(");
                self.gen_expr(expr);
                self.write(" ");
                self.write_keyword("AS ");
                self.gen_data_type(data_type);
                self.write(")");
            }
            Expr::TryCast { expr, data_type } => {
                self.write_keyword("TRY_CAST(");
                self.gen_expr(expr);
                self.write(" ");
                self.write_keyword("AS ");
                self.gen_data_type(data_type);
                self.write(")");
            }
            Expr::Extract { field, expr } => {
                self.write_keyword("EXTRACT(");
                self.gen_datetime_field(field);
                self.write(" ");
                self.write_keyword("FROM ");
                self.gen_expr(expr);
                self.write(")");
            }
            Expr::Interval { value, unit } => {
                self.write_keyword("INTERVAL ");
                self.gen_expr(value);
                if let Some(unit) = unit {
                    self.write(" ");
                    self.gen_datetime_field(unit);
                }
            }
            Expr::ArrayLiteral(items) => {
                self.write_keyword("ARRAY[");
                self.gen_expr_list(items);
                self.write("]");
            }
            Expr::Tuple(items) => {
                self.write("(");
                self.gen_expr_list(items);
                self.write(")");
            }
            Expr::Coalesce(items) => {
                self.write_keyword("COALESCE(");
                self.gen_expr_list(items);
                self.write(")");
            }
            Expr::If { condition, true_val, false_val } => {
                self.write_keyword("IF(");
                self.gen_expr(condition);
                self.write(", ");
                self.gen_expr(true_val);
                if let Some(fv) = false_val {
                    self.write(", ");
                    self.gen_expr(fv);
                }
                self.write(")");
            }
            Expr::NullIf { expr, r#else } => {
                self.write_keyword("NULLIF(");
                self.gen_expr(expr);
                self.write(", ");
                self.gen_expr(r#else);
                self.write(")");
            }
            Expr::Collate { expr, collation } => {
                self.gen_expr(expr);
                self.write(" ");
                self.write_keyword("COLLATE ");
                self.write(collation);
            }
            Expr::Parameter(p) => self.write(p),
            Expr::TypeExpr(dt) => self.gen_data_type(dt),
            Expr::QualifiedWildcard { table } => {
                self.write(table);
                self.write(".*");
            }
            Expr::Alias { expr, name } => {
                self.gen_expr(expr);
                self.write(" ");
                self.write_keyword("AS ");
                self.write(name);
            }
            Expr::ArrayIndex { expr, index } => {
                self.gen_expr(expr);
                self.write("[");
                self.gen_expr(index);
                self.write("]");
            }
            Expr::JsonAccess { expr, path, as_text } => {
                self.gen_expr(expr);
                if *as_text {
                    self.write("->>");
                } else {
                    self.write("->");
                }
                self.gen_expr(path);
            }
            Expr::Lambda { params, body } => {
                if params.len() == 1 {
                    self.write(&params[0]);
                } else {
                    self.write("(");
                    self.write(&params.join(", "));
                    self.write(")");
                }
                self.write(" -> ");
                self.gen_expr(body);
            }
        }
    }

    fn gen_window_spec(&mut self, spec: &WindowSpec) {
        if let Some(wref) = &spec.window_ref {
            self.write(wref);
            if !spec.partition_by.is_empty() || !spec.order_by.is_empty() || spec.frame.is_some() {
                self.write(" ");
            }
        }
        if !spec.partition_by.is_empty() {
            self.write_keyword("PARTITION BY ");
            self.gen_expr_list(&spec.partition_by);
        }
        if !spec.order_by.is_empty() {
            if !spec.partition_by.is_empty() {
                self.write(" ");
            }
            self.write_keyword("ORDER BY ");
            for (i, item) in spec.order_by.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.gen_expr(&item.expr);
                if !item.ascending {
                    self.write(" ");
                    self.write_keyword("DESC");
                }
                if let Some(nulls_first) = item.nulls_first {
                    if nulls_first {
                        self.write(" ");
                        self.write_keyword("NULLS FIRST");
                    } else {
                        self.write(" ");
                        self.write_keyword("NULLS LAST");
                    }
                }
            }
        }
        if let Some(frame) = &spec.frame {
            self.write(" ");
            self.gen_window_frame(frame);
        }
    }

    fn gen_window_frame(&mut self, frame: &WindowFrame) {
        match frame.kind {
            WindowFrameKind::Rows => self.write_keyword("ROWS "),
            WindowFrameKind::Range => self.write_keyword("RANGE "),
            WindowFrameKind::Groups => self.write_keyword("GROUPS "),
        }
        if let Some(end) = &frame.end {
            self.write_keyword("BETWEEN ");
            self.gen_window_frame_bound(&frame.start);
            self.write(" ");
            self.write_keyword("AND ");
            self.gen_window_frame_bound(end);
        } else {
            self.gen_window_frame_bound(&frame.start);
        }
    }

    fn gen_window_frame_bound(&mut self, bound: &WindowFrameBound) {
        match bound {
            WindowFrameBound::CurrentRow => self.write_keyword("CURRENT ROW"),
            WindowFrameBound::Preceding(None) => self.write_keyword("UNBOUNDED PRECEDING"),
            WindowFrameBound::Preceding(Some(n)) => {
                self.gen_expr(n);
                self.write(" ");
                self.write_keyword("PRECEDING");
            }
            WindowFrameBound::Following(None) => self.write_keyword("UNBOUNDED FOLLOWING"),
            WindowFrameBound::Following(Some(n)) => {
                self.gen_expr(n);
                self.write(" ");
                self.write_keyword("FOLLOWING");
            }
        }
    }

    fn gen_datetime_field(&mut self, field: &DateTimeField) {
        let name = match field {
            DateTimeField::Year => "YEAR",
            DateTimeField::Quarter => "QUARTER",
            DateTimeField::Month => "MONTH",
            DateTimeField::Week => "WEEK",
            DateTimeField::Day => "DAY",
            DateTimeField::DayOfWeek => "DOW",
            DateTimeField::DayOfYear => "DOY",
            DateTimeField::Hour => "HOUR",
            DateTimeField::Minute => "MINUTE",
            DateTimeField::Second => "SECOND",
            DateTimeField::Millisecond => "MILLISECOND",
            DateTimeField::Microsecond => "MICROSECOND",
            DateTimeField::Nanosecond => "NANOSECOND",
            DateTimeField::Epoch => "EPOCH",
            DateTimeField::Timezone => "TIMEZONE",
            DateTimeField::TimezoneHour => "TIMEZONE_HOUR",
            DateTimeField::TimezoneMinute => "TIMEZONE_MINUTE",
        };
        self.write(name);
    }
}

impl Default for Generator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn roundtrip(sql: &str) -> String {
        let stmt = Parser::new(sql).unwrap().parse_statement().unwrap();
        let mut g = Generator::new();
        g.generate(&stmt)
    }

    #[test]
    fn test_select_roundtrip() {
        assert_eq!(roundtrip("SELECT a, b FROM t"), "SELECT a, b FROM t");
    }

    #[test]
    fn test_select_where() {
        assert_eq!(
            roundtrip("SELECT x FROM t WHERE x > 10"),
            "SELECT x FROM t WHERE x > 10"
        );
    }

    #[test]
    fn test_select_wildcard() {
        assert_eq!(roundtrip("SELECT * FROM users"), "SELECT * FROM users");
    }

    #[test]
    fn test_insert_values() {
        assert_eq!(
            roundtrip("INSERT INTO t (a, b) VALUES (1, 'hello')"),
            "INSERT INTO t (a, b) VALUES (1, 'hello')"
        );
    }

    #[test]
    fn test_delete() {
        assert_eq!(
            roundtrip("DELETE FROM users WHERE id = 1"),
            "DELETE FROM users WHERE id = 1"
        );
    }

    #[test]
    fn test_join() {
        assert_eq!(
            roundtrip("SELECT a.id, b.name FROM a INNER JOIN b ON a.id = b.a_id"),
            "SELECT a.id, b.name FROM a INNER JOIN b ON a.id = b.a_id"
        );
    }

    #[test]
    fn test_create_table() {
        assert_eq!(
            roundtrip("CREATE TABLE users (id INT NOT NULL, name VARCHAR(255), email TEXT)"),
            "CREATE TABLE users (id INT NOT NULL, name VARCHAR(255), email TEXT)"
        );
    }

    #[test]
    fn test_cte_roundtrip() {
        let sql = "WITH cte AS (SELECT 1 AS x) SELECT x FROM cte";
        assert_eq!(roundtrip(sql), "WITH cte AS (SELECT 1 AS x) SELECT x FROM cte");
    }

    #[test]
    fn test_union_roundtrip() {
        let sql = "SELECT 1 UNION ALL SELECT 2";
        assert_eq!(roundtrip(sql), "SELECT 1 UNION ALL SELECT 2");
    }

    #[test]
    fn test_cast_roundtrip() {
        assert_eq!(
            roundtrip("SELECT CAST(x AS INT) FROM t"),
            "SELECT CAST(x AS INT) FROM t"
        );
    }

    #[test]
    fn test_exists_roundtrip() {
        assert_eq!(
            roundtrip("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM t2)"),
            "SELECT * FROM t WHERE EXISTS (SELECT 1 FROM t2)"
        );
    }

    #[test]
    fn test_extract_roundtrip() {
        assert_eq!(
            roundtrip("SELECT EXTRACT(YEAR FROM created_at) FROM t"),
            "SELECT EXTRACT(YEAR FROM created_at) FROM t"
        );
    }

    #[test]
    fn test_window_function_roundtrip() {
        assert_eq!(
            roundtrip("SELECT ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) FROM emp"),
            "SELECT ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) FROM emp"
        );
    }

    #[test]
    fn test_subquery_from_roundtrip() {
        assert_eq!(
            roundtrip("SELECT * FROM (SELECT 1 AS x) AS sub"),
            "SELECT * FROM (SELECT 1 AS x) AS sub"
        );
    }

    #[test]
    fn test_in_subquery_roundtrip() {
        assert_eq!(
            roundtrip("SELECT * FROM t WHERE id IN (SELECT id FROM t2)"),
            "SELECT * FROM t WHERE id IN (SELECT id FROM t2)"
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Pretty-print tests
    // ═══════════════════════════════════════════════════════════════

    fn pretty_print(sql: &str) -> String {
        let stmt = Parser::new(sql).unwrap().parse_statement().unwrap();
        let mut g = Generator::pretty();
        g.generate(&stmt)
    }

    #[test]
    fn test_pretty_simple_select() {
        assert_eq!(
            pretty_print("SELECT a, b, c FROM t"),
            "SELECT\n  a,\n  b,\n  c\nFROM\n  t"
        );
    }

    #[test]
    fn test_pretty_select_where() {
        assert_eq!(
            pretty_print("SELECT a FROM t WHERE a > 1"),
            "SELECT\n  a\nFROM\n  t\nWHERE\n  a > 1"
        );
    }

    #[test]
    fn test_pretty_select_group_by_having() {
        assert_eq!(
            pretty_print("SELECT a, COUNT(*) FROM t GROUP BY a HAVING COUNT(*) > 1"),
            "SELECT\n  a,\n  COUNT(*)\nFROM\n  t\nGROUP BY\n  a\nHAVING\n  COUNT(*) > 1"
        );
    }

    #[test]
    fn test_pretty_select_order_by_limit() {
        assert_eq!(
            pretty_print("SELECT a FROM t ORDER BY a DESC LIMIT 10"),
            "SELECT\n  a\nFROM\n  t\nORDER BY\n  a DESC\nLIMIT 10"
        );
    }

    #[test]
    fn test_pretty_join() {
        assert_eq!(
            pretty_print("SELECT a.id, b.name FROM a INNER JOIN b ON a.id = b.a_id"),
            "SELECT\n  a.id,\n  b.name\nFROM\n  a\nINNER JOIN\n  b\n  ON a.id = b.a_id"
        );
    }

    #[test]
    fn test_pretty_cte() {
        assert_eq!(
            pretty_print("WITH cte AS (SELECT 1 AS x) SELECT x FROM cte"),
            "WITH cte AS (\n  SELECT\n    1 AS x\n)\nSELECT\n  x\nFROM\n  cte"
        );
    }

    #[test]
    fn test_pretty_union() {
        assert_eq!(
            pretty_print("SELECT 1 UNION ALL SELECT 2"),
            "SELECT\n  1\nUNION ALL\nSELECT\n  2"
        );
    }

    #[test]
    fn test_pretty_insert() {
        assert_eq!(
            pretty_print("INSERT INTO t (a, b) VALUES (1, 'hello'), (2, 'world')"),
            "INSERT INTO t (a, b)\nVALUES\n  (1, 'hello'),\n  (2, 'world')"
        );
    }

    #[test]
    fn test_pretty_update() {
        assert_eq!(
            pretty_print("UPDATE t SET a = 1, b = 2 WHERE c = 3"),
            "UPDATE t\nSET\n  a = 1,\n  b = 2\nWHERE\n  c = 3"
        );
    }

    #[test]
    fn test_pretty_delete() {
        assert_eq!(
            pretty_print("DELETE FROM t WHERE id = 1"),
            "DELETE FROM t\nWHERE\n  id = 1"
        );
    }

    #[test]
    fn test_pretty_create_table() {
        assert_eq!(
            pretty_print("CREATE TABLE t (id INT NOT NULL, name VARCHAR(255), email TEXT)"),
            "CREATE TABLE t (\n  id INT NOT NULL,\n  name VARCHAR(255),\n  email TEXT\n)"
        );
    }

    #[test]
    fn test_pretty_complex_query() {
        let sql = "SELECT a, SUM(b) FROM t1 INNER JOIN t2 ON t1.id = t2.id WHERE t1.x > 1 GROUP BY a HAVING SUM(b) > 10 ORDER BY a LIMIT 100";
        let expected = "SELECT\n  a,\n  SUM(b)\nFROM\n  t1\nINNER JOIN\n  t2\n  ON t1.id = t2.id\nWHERE\n  t1.x > 1\nGROUP BY\n  a\nHAVING\n  SUM(b) > 10\nORDER BY\n  a\nLIMIT 100";
        assert_eq!(pretty_print(sql), expected);
    }

    #[test]
    fn test_pretty_select_distinct() {
        assert_eq!(
            pretty_print("SELECT DISTINCT a, b FROM t"),
            "SELECT DISTINCT\n  a,\n  b\nFROM\n  t"
        );
    }
}
