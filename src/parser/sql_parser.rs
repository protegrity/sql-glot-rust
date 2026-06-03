use crate::ast::*;
use crate::errors::{Result, SqlglotError};
use crate::tokens::{Token, TokenType, Tokenizer};

/// Convert a token's `quote_char` into a `QuoteStyle`.
fn quote_style_from_char(c: char) -> QuoteStyle {
    match c {
        '"' => QuoteStyle::DoubleQuote,
        '`' => QuoteStyle::Backtick,
        '[' => QuoteStyle::Bracket,
        _ => QuoteStyle::None,
    }
}

/// A recursive-descent SQL parser.
///
/// Supports CTEs (WITH), subqueries, UNION/INTERSECT/EXCEPT, CAST,
/// window functions (OVER), EXISTS, EXTRACT, INTERVAL, and more.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Whether to preserve comments during parsing.
    #[allow(dead_code)]
    preserve_comments: bool,
    /// Accumulated comments pending attachment to the next AST node.
    pending_comments: Vec<String>,
}

impl Parser {
    /// Create a new parser from a SQL string.
    pub fn new(sql: &str) -> Result<Self> {
        let mut tokenizer = Tokenizer::new(sql);
        let tokens = tokenizer.tokenize()?;
        Ok(Self {
            tokens,
            pos: 0,
            preserve_comments: false,
            pending_comments: Vec::new(),
        })
    }

    /// Create a new parser that preserves SQL comments in the AST.
    pub fn new_with_comments(sql: &str) -> Result<Self> {
        let mut tokenizer = Tokenizer::with_comments(sql);
        let tokens = tokenizer.tokenize()?;
        Ok(Self {
            tokens,
            pos: 0,
            preserve_comments: true,
            pending_comments: Vec::new(),
        })
    }

    // ── Comment helpers ────────────────────────────────────────────

    /// Consume any comment tokens at the current position, accumulating
    /// their text into `pending_comments`.
    fn collect_comments(&mut self) {
        while self.pos < self.tokens.len() {
            match self.tokens[self.pos].token_type {
                TokenType::LineComment | TokenType::BlockComment => {
                    let token = &self.tokens[self.pos];
                    self.pending_comments.push(token.value.clone());
                    self.pos += 1;
                }
                _ => break,
            }
        }
    }

    /// Take all pending comments, leaving the buffer empty.
    fn take_comments(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_comments)
    }

    // ── Token helpers ──────────────────────────────────────────────

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek_type(&self) -> &TokenType {
        &self.peek().token_type
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        token
    }

    fn expect(&mut self, expected: TokenType) -> Result<Token> {
        let token = self.peek().clone();
        if token.token_type == expected {
            self.advance();
            Ok(token)
        } else {
            Err(SqlglotError::ParserError {
                message: format!(
                    "Expected {expected:?}, got {:?} ('{}') at line {} col {}",
                    token.token_type, token.value, token.line, token.col
                ),
            })
        }
    }

    fn match_token(&mut self, expected: TokenType) -> bool {
        if self.peek().token_type == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Check if the current token's uppercased value matches a keyword string.
    fn check_keyword(&self, keyword: &str) -> bool {
        self.peek().value.to_uppercase() == keyword
    }

    /// Check if the token at `current + offset` matches a keyword string.
    fn check_keyword_offset(&self, keyword: &str, offset: usize) -> bool {
        let idx = self.pos + offset;
        if idx < self.tokens.len() {
            self.tokens[idx].value.to_uppercase() == keyword
        } else {
            false
        }
    }

    /// Match a keyword by string value (for multi-word context-sensitive keywords).
    fn match_keyword(&mut self, keyword: &str) -> bool {
        if self.check_keyword(keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Expect a keyword by string value, returning an error if not found.
    fn expect_keyword(&mut self, keyword: &str) -> Result<()> {
        if self.check_keyword(keyword) {
            self.advance();
            Ok(())
        } else {
            let token = self.peek().clone();
            Err(SqlglotError::ParserError {
                message: format!(
                    "Expected keyword '{keyword}', got '{value}' at line {line} col {col}",
                    value = token.value,
                    line = token.line,
                    col = token.col
                ),
            })
        }
    }

    /// Helper to check if current token is an identifier or keyword that can serve as a name.
    fn is_name_token(&self) -> bool {
        matches!(
            self.peek_type(),
            TokenType::Identifier
                | TokenType::Year
                | TokenType::Month
                | TokenType::Day
                | TokenType::Hour
                | TokenType::Minute
                | TokenType::Second
                | TokenType::Key
                | TokenType::Filter
                | TokenType::First
                | TokenType::Next
                | TokenType::Only
                | TokenType::Respect
                | TokenType::Epoch
                | TokenType::Schema
                | TokenType::Database
                | TokenType::View
                | TokenType::Collate
                | TokenType::Comment
                | TokenType::Left
                | TokenType::Right
                | TokenType::Replace
                | TokenType::Cube
                | TokenType::Rollup
                | TokenType::Grouping
                | TokenType::Pivot
                | TokenType::Unpivot
                | TokenType::Sets
        )
    }

    /// Consume a name token (identifier or unreserved keyword used as identifier).
    fn expect_name(&mut self) -> Result<String> {
        let (name, _) = self.expect_name_with_quote()?;
        Ok(name)
    }

    /// Like `expect_name` but also returns the quote style of the token.
    fn expect_name_with_quote(&mut self) -> Result<(String, QuoteStyle)> {
        if self.is_name_token() {
            let token = self.advance().clone();
            let qs = quote_style_from_char(token.quote_char);
            return Ok((token.value.clone(), qs));
        }
        // Also accept any keyword-like identifier
        let token = self.peek().clone();
        if matches!(
            token.token_type,
            TokenType::Identifier
                | TokenType::Int
                | TokenType::Integer
                | TokenType::BigInt
                | TokenType::SmallInt
                | TokenType::TinyInt
                | TokenType::Float
                | TokenType::Double
                | TokenType::Decimal
                | TokenType::Numeric
                | TokenType::Real
                | TokenType::Varchar
                | TokenType::Char
                | TokenType::Text
                | TokenType::Boolean
                | TokenType::Date
                | TokenType::Timestamp
                | TokenType::TimestampTz
                | TokenType::Time
                | TokenType::Interval
                | TokenType::Blob
                | TokenType::Bytea
                | TokenType::Json
                | TokenType::Jsonb
                | TokenType::Uuid
                | TokenType::Array
                | TokenType::Map
                | TokenType::Struct
        ) {
            let t = self.advance().clone();
            let qs = quote_style_from_char(t.quote_char);
            Ok((t.value.clone(), qs))
        } else {
            Err(SqlglotError::ParserError {
                message: format!(
                    "Expected identifier, got {:?} ('{}') at line {} col {}",
                    token.token_type, token.value, token.line, token.col
                ),
            })
        }
    }

    // ── Top-level parsing ──────────────────────────────────────────

    /// Parse a single SQL statement.
    pub fn parse_statement(&mut self) -> Result<Statement> {
        self.collect_comments();
        let stmt = self.parse_statement_inner()?;
        // Consume trailing semicolons
        while self.match_token(TokenType::Semicolon) {}
        Ok(stmt)
    }

    fn parse_statement_inner(&mut self) -> Result<Statement> {
        self.collect_comments();
        let comments = self.take_comments();
        let mut stmt = match self.peek_type() {
            TokenType::With => self.parse_with_statement(),
            TokenType::Select => {
                let select = self.parse_select_body(vec![])?;
                self.maybe_parse_set_operation(Statement::Select(select))
            }
            TokenType::LParen => {
                // Could be a parenthesized SELECT
                let saved_pos = self.pos;
                self.advance(); // consume '('
                if matches!(self.peek_type(), TokenType::Select | TokenType::With) {
                    let inner = self.parse_statement_inner()?;
                    self.expect(TokenType::RParen)?;
                    self.maybe_parse_set_operation(inner)
                } else {
                    self.pos = saved_pos;
                    Err(SqlglotError::ParserError {
                        message: "Expected statement".into(),
                    })
                }
            }
            TokenType::Insert => self.parse_insert().map(Statement::Insert),
            TokenType::Update => self.parse_update().map(Statement::Update),
            TokenType::Delete => self.parse_delete().map(Statement::Delete),
            TokenType::Merge => self.parse_merge().map(Statement::Merge),
            TokenType::Create => self.parse_create(),
            TokenType::Drop => self.parse_drop(),
            TokenType::Alter => self.parse_alter_table().map(Statement::AlterTable),
            TokenType::Truncate => self.parse_truncate().map(Statement::Truncate),
            TokenType::Begin | TokenType::Commit | TokenType::Rollback | TokenType::Savepoint => {
                self.parse_transaction().map(Statement::Transaction)
            }
            TokenType::Explain => self.parse_explain().map(Statement::Explain),
            TokenType::Use => self.parse_use().map(Statement::Use),
            _ => Err(SqlglotError::UnexpectedToken {
                token: self.peek().clone(),
            }),
        }?;
        if !comments.is_empty() {
            attach_comments_to_statement(&mut stmt, comments);
        }
        Ok(stmt)
    }

    /// Parse multiple statements separated by semicolons.
    pub fn parse_statements(&mut self) -> Result<Vec<Statement>> {
        let mut stmts = Vec::new();
        while !matches!(self.peek_type(), TokenType::Eof) {
            while self.match_token(TokenType::Semicolon) {}
            if matches!(self.peek_type(), TokenType::Eof) {
                break;
            }
            stmts.push(self.parse_statement()?);
        }
        Ok(stmts)
    }

    // ── WITH / CTE parsing ─────────────────────────────────────────

    fn parse_with_statement(&mut self) -> Result<Statement> {
        self.expect(TokenType::With)?;
        let recursive = self.match_token(TokenType::Recursive);
        let mut ctes = vec![self.parse_cte(recursive)?];
        while self.match_token(TokenType::Comma) {
            ctes.push(self.parse_cte(recursive)?);
        }

        // Now parse the main query
        match self.peek_type() {
            TokenType::Select => {
                let select = self.parse_select_body(ctes)?;
                self.maybe_parse_set_operation(Statement::Select(select))
            }
            TokenType::Insert => {
                // WITH ... INSERT is supported in some dialects
                let ins = self.parse_insert()?;
                // Attach CTEs if needed (simplification)
                let _ = ctes; // CTEs with INSERT - we'll handle this later
                Ok(Statement::Insert(ins))
            }
            _ => Err(SqlglotError::ParserError {
                message: "Expected SELECT or INSERT after WITH clause".into(),
            }),
        }
    }

    fn parse_cte(&mut self, recursive: bool) -> Result<Cte> {
        let (name, name_quote_style) = self.expect_name_with_quote()?;

        let columns = if self.match_token(TokenType::LParen) {
            let mut cols = vec![self.expect_name()?];
            while self.match_token(TokenType::Comma) {
                cols.push(self.expect_name()?);
            }
            self.expect(TokenType::RParen)?;
            cols
        } else {
            vec![]
        };

        self.expect(TokenType::As)?;

        let materialized = if self.match_keyword("MATERIALIZED") {
            Some(true)
        } else if self.check_keyword("NOT") {
            let saved = self.pos;
            self.advance();
            if self.match_keyword("MATERIALIZED") {
                Some(false)
            } else {
                self.pos = saved;
                None
            }
        } else {
            None
        };

        self.expect(TokenType::LParen)?;
        let query = self.parse_statement_inner()?;
        self.expect(TokenType::RParen)?;

        Ok(Cte {
            name,
            name_quote_style,
            columns,
            query: Box::new(query),
            materialized,
            recursive,
        })
    }

    // ── SELECT ──────────────────────────────────────────────────────

    fn parse_select_body(&mut self, ctes: Vec<Cte>) -> Result<SelectStatement> {
        self.expect(TokenType::Select)?;

        let distinct = self.match_token(TokenType::Distinct);
        // SQL-standard `SELECT ALL` quantifier (§7.12). Equivalent to omitting
        // the quantifier; consume it so it does not get mis-parsed as a column.
        if !distinct {
            let _ = self.match_token(TokenType::All);
        }

        // TOP N (SQL Server style)
        // Use parse_primary() instead of parse_expr() to prevent the parser
        // from consuming `*` (SELECT all columns) as a multiplication operator.
        // This correctly handles: TOP 5, TOP 100, TOP (expr), TOP (@var)
        let top = if self.match_token(TokenType::Top) {
            Some(Box::new(self.parse_primary()?))
        } else {
            None
        };

        let columns = self.parse_select_items()?;

        let from = if self.match_token(TokenType::From) {
            Some(FromClause {
                source: self.parse_table_source()?,
            })
        } else {
            None
        };

        let joins = self.parse_joins()?;

        let where_clause = if self.match_token(TokenType::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let group_by = if self.match_token(TokenType::Group) {
            self.expect(TokenType::By)?;
            self.parse_group_by_list()?
        } else {
            vec![]
        };

        let having = if self.match_token(TokenType::Having) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let qualify = if self.match_token(TokenType::Qualify) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        // Named WINDOW definitions
        let window_definitions = if self.match_token(TokenType::Window) {
            self.parse_window_definitions()?
        } else {
            vec![]
        };

        let order_by = if self.match_token(TokenType::Order) {
            self.expect(TokenType::By)?;
            self.parse_order_by_items()?
        } else {
            vec![]
        };

        let limit = if self.match_token(TokenType::Limit) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let offset = if self.match_token(TokenType::Offset) {
            let expr = self.parse_expr()?;
            // T-SQL / ANSI SQL:2008 form: OFFSET n ROWS [FETCH …].
            // Consume the optional ROWS/ROW keyword so FETCH can match next.
            let _ = self.match_token(TokenType::Rows) || self.match_keyword("ROW");
            Some(expr)
        } else {
            None
        };

        // FETCH FIRST|NEXT n ROWS ONLY (Oracle / ANSI SQL:2008 / T-SQL)
        let fetch_first = if self.match_token(TokenType::Fetch) {
            // consume FIRST or NEXT
            let _ = self.match_token(TokenType::First) || self.match_token(TokenType::Next);
            let count = self.parse_expr()?;
            // consume ROWS or ROW
            let _ = self.match_keyword("ROWS") || self.match_keyword("ROW");
            // consume ONLY
            let _ = self.match_token(TokenType::Only);
            Some(count)
        } else {
            None
        };

        Ok(SelectStatement {
            comments: vec![],
            ctes,
            distinct,
            top,
            columns,
            from,
            joins,
            where_clause,
            group_by,
            having,
            order_by,
            limit,
            offset,
            fetch_first,
            qualify,
            window_definitions,
        })
    }

    fn parse_window_definitions(&mut self) -> Result<Vec<WindowDefinition>> {
        let mut defs = Vec::new();
        loop {
            let name = self.expect_name()?;
            self.expect(TokenType::As)?;
            self.expect(TokenType::LParen)?;
            let spec = self.parse_window_spec()?;
            self.expect(TokenType::RParen)?;
            defs.push(WindowDefinition { name, spec });
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(defs)
    }

    /// Check if we should parse a set operation (UNION / INTERSECT / EXCEPT)
    fn maybe_parse_set_operation(&mut self, left: Statement) -> Result<Statement> {
        let op = match self.peek_type() {
            TokenType::Union => SetOperationType::Union,
            TokenType::Intersect => SetOperationType::Intersect,
            TokenType::Except => SetOperationType::Except,
            _ => return Ok(left),
        };
        self.advance();

        let all = self.match_token(TokenType::All);
        let _ = self.match_token(TokenType::Distinct); // UNION DISTINCT

        let right = self.parse_statement_inner()?;

        // Check for further set operations chaining
        let combined = Statement::SetOperation(SetOperationStatement {
            comments: vec![],
            op,
            all,
            left: Box::new(left),
            right: Box::new(right),
            order_by: vec![],
            limit: None,
            offset: None,
        });

        // Parse trailing ORDER BY / LIMIT / OFFSET that applies to the whole set operation
        if matches!(
            self.peek_type(),
            TokenType::Union | TokenType::Intersect | TokenType::Except
        ) {
            self.maybe_parse_set_operation(combined)
        } else {
            // Check for global ORDER BY / LIMIT
            if let Statement::SetOperation(mut sop) = combined {
                if self.match_token(TokenType::Order) {
                    self.expect(TokenType::By)?;
                    sop.order_by = self.parse_order_by_items()?;
                }
                if self.match_token(TokenType::Limit) {
                    sop.limit = Some(self.parse_expr()?);
                }
                if self.match_token(TokenType::Offset) {
                    sop.offset = Some(self.parse_expr()?);
                }
                Ok(Statement::SetOperation(sop))
            } else {
                Ok(combined)
            }
        }
    }

    fn parse_select_items(&mut self) -> Result<Vec<SelectItem>> {
        let mut items = vec![self.parse_select_item()?];
        while self.match_token(TokenType::Comma) {
            items.push(self.parse_select_item()?);
        }
        Ok(items)
    }

    fn parse_select_item(&mut self) -> Result<SelectItem> {
        if self.peek().token_type == TokenType::Star {
            self.advance();
            return Ok(SelectItem::Wildcard);
        }

        let expr = self.parse_expr()?;

        // Check for table.* pattern
        if let Expr::QualifiedWildcard { ref table } = expr {
            return Ok(SelectItem::QualifiedWildcard {
                table: table.clone(),
            });
        }

        let (alias, alias_quote_style) = match self.parse_optional_alias()? {
            Some((name, qs)) => (Some(name), qs),
            None => (None, QuoteStyle::None),
        };

        Ok(SelectItem::Expr {
            expr,
            alias,
            alias_quote_style,
        })
    }

    fn parse_optional_alias(&mut self) -> Result<Option<(String, QuoteStyle)>> {
        if self.match_token(TokenType::As) {
            return Ok(Some(self.expect_name_with_quote()?));
        }
        // Implicit alias
        if self.is_name_token() {
            let peeked_upper = self.peek().value.to_uppercase();
            if !matches!(
                peeked_upper.as_str(),
                "FROM"
                    | "WHERE"
                    | "GROUP"
                    | "ORDER"
                    | "LIMIT"
                    | "HAVING"
                    | "UNION"
                    | "INTERSECT"
                    | "EXCEPT"
                    | "JOIN"
                    | "INNER"
                    | "LEFT"
                    | "RIGHT"
                    | "FULL"
                    | "CROSS"
                    | "ON"
                    | "WINDOW"
                    | "QUALIFY"
                    | "INTO"
                    | "SET"
                    | "RETURNING"
                    | "PIVOT"
                    | "UNPIVOT"
            ) {
                let token = self.advance().clone();
                let qs = quote_style_from_char(token.quote_char);
                return Ok(Some((token.value.clone(), qs)));
            }
        }
        Ok(None)
    }

    fn parse_table_source(&mut self) -> Result<TableSource> {
        let source = self.parse_base_table_source()?;
        // Check for trailing PIVOT / UNPIVOT
        self.parse_pivot_or_unpivot(source)
    }

    fn parse_base_table_source(&mut self) -> Result<TableSource> {
        // LATERAL
        if self.match_token(TokenType::Lateral) {
            let source = self.parse_table_source()?;
            return Ok(TableSource::Lateral {
                source: Box::new(source),
            });
        }

        // UNNEST(expr)
        if self.match_token(TokenType::Unnest) {
            self.expect(TokenType::LParen)?;
            let expr = self.parse_expr()?;
            self.expect(TokenType::RParen)?;
            let (alias, alias_quote_style) = match self.parse_optional_alias()? {
                Some((name, qs)) => (Some(name), qs),
                None => (None, QuoteStyle::None),
            };
            let with_offset = self.match_keyword("WITH") && self.match_keyword("OFFSET");
            return Ok(TableSource::Unnest {
                expr: Box::new(expr),
                alias,
                alias_quote_style,
                with_offset,
            });
        }

        // Subquery: (SELECT ...)
        if self.peek_type() == &TokenType::LParen {
            let saved = self.pos;
            self.advance();
            if matches!(self.peek_type(), TokenType::Select | TokenType::With) {
                let query = self.parse_statement_inner()?;
                self.expect(TokenType::RParen)?;
                let (alias, alias_quote_style) = match self.parse_optional_alias()? {
                    Some((name, qs)) => (Some(name), qs),
                    None => (None, QuoteStyle::None),
                };
                return Ok(TableSource::Subquery {
                    query: Box::new(query),
                    alias,
                    alias_quote_style,
                });
            }
            self.pos = saved;
        }

        // Regular table reference (possibly with function syntax)
        let table_ref = self.parse_table_ref()?;

        // Check if it's actually a table function: name(args...)
        if self.peek_type() == &TokenType::LParen && table_ref.schema.is_none() {
            self.advance();
            let args = if self.peek_type() != &TokenType::RParen {
                self.parse_expr_list()?
            } else {
                vec![]
            };
            self.expect(TokenType::RParen)?;
            let (alias, alias_quote_style) = match self.parse_optional_alias()? {
                Some((name, qs)) => (Some(name), qs),
                None => (None, QuoteStyle::None),
            };
            return Ok(TableSource::TableFunction {
                name: table_ref.name,
                args,
                alias,
                alias_quote_style,
            });
        }

        Ok(TableSource::Table(table_ref))
    }

    /// After parsing a base table source, check if PIVOT or UNPIVOT follows.
    fn parse_pivot_or_unpivot(&mut self, source: TableSource) -> Result<TableSource> {
        if self.match_token(TokenType::Pivot) {
            self.expect(TokenType::LParen)?;
            let aggregate = self.parse_expr()?;
            self.expect_keyword("FOR")?;
            let for_column = self.expect_name()?;
            self.expect(TokenType::In)?;
            self.expect(TokenType::LParen)?;
            let in_values = self.parse_pivot_values()?;
            self.expect(TokenType::RParen)?;
            self.expect(TokenType::RParen)?;
            let (alias, alias_quote_style) = match self.parse_optional_alias()? {
                Some((name, qs)) => (Some(name), qs),
                None => (None, QuoteStyle::None),
            };
            return Ok(TableSource::Pivot {
                source: Box::new(source),
                aggregate: Box::new(aggregate),
                for_column,
                in_values,
                alias,
                alias_quote_style,
            });
        }
        if self.match_token(TokenType::Unpivot) {
            self.expect(TokenType::LParen)?;
            let value_column = self.expect_name()?;
            self.expect_keyword("FOR")?;
            let for_column = self.expect_name()?;
            self.expect(TokenType::In)?;
            self.expect(TokenType::LParen)?;
            let in_columns = self.parse_pivot_values()?;
            self.expect(TokenType::RParen)?;
            self.expect(TokenType::RParen)?;
            let (alias, alias_quote_style) = match self.parse_optional_alias()? {
                Some((name, qs)) => (Some(name), qs),
                None => (None, QuoteStyle::None),
            };
            return Ok(TableSource::Unpivot {
                source: Box::new(source),
                value_column,
                for_column,
                in_columns,
                alias,
                alias_quote_style,
            });
        }
        Ok(source)
    }

    /// Parse comma-separated pivot values, each optionally aliased.
    fn parse_pivot_values(&mut self) -> Result<Vec<PivotValue>> {
        let mut values = Vec::new();
        loop {
            let value = self.parse_expr()?;
            let (alias, alias_quote_style) = match self.parse_optional_alias()? {
                Some((name, qs)) => (Some(name), qs),
                None => (None, QuoteStyle::None),
            };
            values.push(PivotValue {
                value,
                alias,
                alias_quote_style,
            });
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(values)
    }

    fn parse_table_ref(&mut self) -> Result<TableRef> {
        let (first, first_qs) = self.expect_name_with_quote()?;

        // Check for schema.table or catalog.schema.table
        let (catalog, schema, name, name_qs) = if self.match_token(TokenType::Dot) {
            let (second, second_qs) = self.expect_name_with_quote()?;
            if self.match_token(TokenType::Dot) {
                let (third, third_qs) = self.expect_name_with_quote()?;
                (Some(first), Some(second), third, third_qs)
            } else {
                (None, Some(first), second, second_qs)
            }
        } else {
            (None, None, first, first_qs)
        };

        let (alias, alias_quote_style) = match self.parse_optional_alias()? {
            Some((name, qs)) => (Some(name), qs),
            None => (None, QuoteStyle::None),
        };

        Ok(TableRef {
            catalog,
            schema,
            name,
            alias,
            name_quote_style: name_qs,
            alias_quote_style,
        })
    }

    /// Like `parse_table_ref` but does not consume an alias.
    fn parse_table_ref_no_alias(&mut self) -> Result<TableRef> {
        let (first, first_qs) = self.expect_name_with_quote()?;

        let (catalog, schema, name, name_qs) = if self.match_token(TokenType::Dot) {
            let (second, second_qs) = self.expect_name_with_quote()?;
            if self.match_token(TokenType::Dot) {
                let (third, third_qs) = self.expect_name_with_quote()?;
                (Some(first), Some(second), third, third_qs)
            } else {
                (None, Some(first), second, second_qs)
            }
        } else {
            (None, None, first, first_qs)
        };

        Ok(TableRef {
            catalog,
            schema,
            name,
            alias: None,
            name_quote_style: name_qs,
            alias_quote_style: QuoteStyle::None,
        })
    }

    fn parse_joins(&mut self) -> Result<Vec<JoinClause>> {
        let mut joins = Vec::new();
        loop {
            let join_type = match self.peek_type() {
                // `FROM a, b` is treated as `FROM a CROSS JOIN b`. Note the
                // SQL standard gives comma a lower precedence than explicit
                // JOIN operators (so `FROM a, b JOIN c ON ...` should be
                // `a CROSS JOIN (b JOIN c ...)`), but we flatten everything
                // into a left-deep chain. Column resolution still works for
                // the common cases since the join order is associative when
                // ON-clauses only reference adjacent tables.
                TokenType::Comma => {
                    self.advance();
                    JoinType::Cross
                }
                TokenType::Join => {
                    self.advance();
                    JoinType::Inner
                }
                TokenType::Inner => {
                    self.advance();
                    self.expect(TokenType::Join)?;
                    JoinType::Inner
                }
                TokenType::Left => {
                    self.advance();
                    let _ = self.match_token(TokenType::Outer);
                    self.expect(TokenType::Join)?;
                    JoinType::Left
                }
                TokenType::Right => {
                    self.advance();
                    let _ = self.match_token(TokenType::Outer);
                    self.expect(TokenType::Join)?;
                    JoinType::Right
                }
                TokenType::Full => {
                    self.advance();
                    let _ = self.match_token(TokenType::Outer);
                    self.expect(TokenType::Join)?;
                    JoinType::Full
                }
                TokenType::Cross => {
                    self.advance();
                    self.expect(TokenType::Join)?;
                    JoinType::Cross
                }
                _ => break,
            };

            let table = self.parse_table_source()?;
            let mut on = None;
            let mut using = vec![];

            if self.match_token(TokenType::On) {
                on = Some(self.parse_expr()?);
            } else if self.match_token(TokenType::Using) {
                self.expect(TokenType::LParen)?;
                using = vec![self.expect_name()?];
                while self.match_token(TokenType::Comma) {
                    using.push(self.expect_name()?);
                }
                self.expect(TokenType::RParen)?;
            }

            joins.push(JoinClause {
                join_type,
                table,
                on,
                using,
            });
        }
        Ok(joins)
    }

    fn parse_order_by_items(&mut self) -> Result<Vec<OrderByItem>> {
        let mut items = Vec::new();
        loop {
            let expr = self.parse_expr()?;
            let ascending = if self.match_token(TokenType::Desc) {
                false
            } else {
                let _ = self.match_token(TokenType::Asc);
                true
            };

            let nulls_first = if self.match_token(TokenType::Nulls) {
                if self.match_token(TokenType::First) {
                    Some(true)
                } else {
                    self.expect(TokenType::Identifier)?; // LAST
                    Some(false)
                }
            } else {
                None
            };

            items.push(OrderByItem {
                expr,
                ascending,
                nulls_first,
            });
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(items)
    }

    fn parse_expr_list(&mut self) -> Result<Vec<Expr>> {
        let mut exprs = vec![self.parse_expr()?];
        while self.match_token(TokenType::Comma) {
            exprs.push(self.parse_expr()?);
        }
        Ok(exprs)
    }

    /// Parse a GROUP BY list, which may contain regular expressions,
    /// CUBE(...), ROLLUP(...), and GROUPING SETS(...).
    fn parse_group_by_list(&mut self) -> Result<Vec<Expr>> {
        let mut items = vec![self.parse_group_by_item()?];
        while self.match_token(TokenType::Comma) {
            items.push(self.parse_group_by_item()?);
        }
        Ok(items)
    }

    /// Parse a single GROUP BY item: a CUBE, ROLLUP, GROUPING SETS, or regular expression.
    fn parse_group_by_item(&mut self) -> Result<Expr> {
        match self.peek_type() {
            TokenType::Cube => {
                self.advance();
                self.expect(TokenType::LParen)?;
                let exprs = if self.peek_type() == &TokenType::RParen {
                    vec![]
                } else {
                    self.parse_group_by_element_list()?
                };
                self.expect(TokenType::RParen)?;
                Ok(Expr::Cube { exprs })
            }
            TokenType::Rollup => {
                self.advance();
                self.expect(TokenType::LParen)?;
                let exprs = if self.peek_type() == &TokenType::RParen {
                    vec![]
                } else {
                    self.parse_group_by_element_list()?
                };
                self.expect(TokenType::RParen)?;
                Ok(Expr::Rollup { exprs })
            }
            TokenType::Grouping => {
                // Could be GROUPING SETS or GROUPING() function
                let saved = self.pos;
                self.advance();
                if self.peek_type() == &TokenType::Sets {
                    // GROUPING SETS (...)
                    self.advance();
                    self.expect(TokenType::LParen)?;
                    let sets = self.parse_grouping_sets_elements()?;
                    self.expect(TokenType::RParen)?;
                    Ok(Expr::GroupingSets { sets })
                } else {
                    // It's the GROUPING() function, backtrack and parse as expression
                    self.pos = saved;
                    self.parse_expr()
                }
            }
            _ => self.parse_expr(),
        }
    }

    /// Parse elements inside CUBE(...) or ROLLUP(...).
    /// Each element can be a single expression or a parenthesized tuple of expressions.
    fn parse_group_by_element_list(&mut self) -> Result<Vec<Expr>> {
        let mut items = vec![self.parse_group_by_element()?];
        while self.match_token(TokenType::Comma) {
            items.push(self.parse_group_by_element()?);
        }
        Ok(items)
    }

    /// Parse a single element inside CUBE/ROLLUP: either `expr` or `(expr, expr, ...)`.
    fn parse_group_by_element(&mut self) -> Result<Expr> {
        if self.peek_type() == &TokenType::LParen {
            self.advance();
            let exprs = self.parse_expr_list()?;
            self.expect(TokenType::RParen)?;
            if exprs.len() == 1 {
                Ok(Expr::Nested(Box::new(exprs.into_iter().next().unwrap())))
            } else {
                Ok(Expr::Tuple(exprs))
            }
        } else {
            self.parse_expr()
        }
    }

    /// Parse elements inside GROUPING SETS (...).
    /// Each element can be: (), (expr, ...), CUBE(...), ROLLUP(...), or a single expr.
    fn parse_grouping_sets_elements(&mut self) -> Result<Vec<Expr>> {
        let mut items = vec![self.parse_grouping_sets_element()?];
        while self.match_token(TokenType::Comma) {
            items.push(self.parse_grouping_sets_element()?);
        }
        Ok(items)
    }

    /// Parse a single GROUPING SETS element.
    fn parse_grouping_sets_element(&mut self) -> Result<Expr> {
        match self.peek_type() {
            TokenType::Cube => {
                self.advance();
                self.expect(TokenType::LParen)?;
                let exprs = if self.peek_type() == &TokenType::RParen {
                    vec![]
                } else {
                    self.parse_group_by_element_list()?
                };
                self.expect(TokenType::RParen)?;
                Ok(Expr::Cube { exprs })
            }
            TokenType::Rollup => {
                self.advance();
                self.expect(TokenType::LParen)?;
                let exprs = if self.peek_type() == &TokenType::RParen {
                    vec![]
                } else {
                    self.parse_group_by_element_list()?
                };
                self.expect(TokenType::RParen)?;
                Ok(Expr::Rollup { exprs })
            }
            TokenType::LParen => {
                self.advance();
                if self.peek_type() == &TokenType::RParen {
                    // Empty grouping set: ()
                    self.advance();
                    Ok(Expr::Tuple(vec![]))
                } else {
                    let exprs = self.parse_expr_list()?;
                    self.expect(TokenType::RParen)?;
                    if exprs.len() == 1 {
                        Ok(Expr::Nested(Box::new(exprs.into_iter().next().unwrap())))
                    } else {
                        Ok(Expr::Tuple(exprs))
                    }
                }
            }
            _ => self.parse_expr(),
        }
    }

    // ── INSERT ──────────────────────────────────────────────────────

    fn parse_insert(&mut self) -> Result<InsertStatement> {
        self.expect(TokenType::Insert)?;
        let _ = self.match_token(TokenType::Into);
        let table = self.parse_table_ref()?;

        let columns = if self.match_token(TokenType::LParen) {
            let mut cols = vec![self.expect_name()?];
            while self.match_token(TokenType::Comma) {
                cols.push(self.expect_name()?);
            }
            self.expect(TokenType::RParen)?;
            cols
        } else {
            vec![]
        };

        let source = if self.match_token(TokenType::Values) {
            let mut rows = Vec::new();
            loop {
                self.expect(TokenType::LParen)?;
                let row = self.parse_expr_list()?;
                self.expect(TokenType::RParen)?;
                rows.push(row);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            InsertSource::Values(rows)
        } else if matches!(
            self.peek_type(),
            TokenType::Select | TokenType::With | TokenType::LParen
        ) {
            InsertSource::Query(Box::new(self.parse_statement_inner()?))
        } else if self.match_token(TokenType::Default) {
            self.expect(TokenType::Values)?;
            InsertSource::Default
        } else {
            return Err(SqlglotError::ParserError {
                message: "Expected VALUES, SELECT, or DEFAULT VALUES after INSERT".into(),
            });
        };

        // ON CONFLICT
        let on_conflict = if self.match_token(TokenType::On) {
            if self.match_token(TokenType::Conflict) {
                let columns = if self.match_token(TokenType::LParen) {
                    let mut cols = vec![self.expect_name()?];
                    while self.match_token(TokenType::Comma) {
                        cols.push(self.expect_name()?);
                    }
                    self.expect(TokenType::RParen)?;
                    cols
                } else {
                    vec![]
                };
                self.expect(TokenType::Do)?;
                let action = if self.match_token(TokenType::Nothing) {
                    ConflictAction::DoNothing
                } else {
                    self.expect(TokenType::Update)?;
                    self.expect(TokenType::Set)?;
                    let mut assignments = Vec::new();
                    loop {
                        let col = self.expect_name()?;
                        self.expect(TokenType::Eq)?;
                        let val = self.parse_expr()?;
                        assignments.push((col, val));
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    ConflictAction::DoUpdate(assignments)
                };
                Some(OnConflict { columns, action })
            } else {
                None
            }
        } else {
            None
        };

        let returning = if self.match_token(TokenType::Returning) {
            self.parse_select_items()?
        } else {
            vec![]
        };

        Ok(InsertStatement {
            comments: vec![],
            table,
            columns,
            source,
            on_conflict,
            returning,
        })
    }

    // ── UPDATE ──────────────────────────────────────────────────────

    fn parse_update(&mut self) -> Result<UpdateStatement> {
        self.expect(TokenType::Update)?;
        let table = self.parse_table_ref()?;
        self.expect(TokenType::Set)?;

        let mut assignments = Vec::new();
        loop {
            // Accept qualified LHS like `alias.col` (Oracle, T-SQL idiom).
            let mut col = self.expect_name()?;
            while self.match_token(TokenType::Dot) {
                col.push('.');
                col.push_str(&self.expect_name()?);
            }
            self.expect(TokenType::Eq)?;
            let val = self.parse_expr()?;
            assignments.push((col, val));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        let from = if self.match_token(TokenType::From) {
            Some(FromClause {
                source: self.parse_table_source()?,
            })
        } else {
            None
        };

        let where_clause = if self.match_token(TokenType::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let returning = if self.match_token(TokenType::Returning) {
            self.parse_select_items()?
        } else {
            vec![]
        };

        Ok(UpdateStatement {
            comments: vec![],
            table,
            assignments,
            from,
            where_clause,
            returning,
        })
    }

    // ── DELETE ──────────────────────────────────────────────────────

    fn parse_delete(&mut self) -> Result<DeleteStatement> {
        self.expect(TokenType::Delete)?;
        self.expect(TokenType::From)?;
        let table = self.parse_table_ref()?;

        let using = if self.match_token(TokenType::Using) {
            Some(FromClause {
                source: self.parse_table_source()?,
            })
        } else {
            None
        };

        let where_clause = if self.match_token(TokenType::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let returning = if self.match_token(TokenType::Returning) {
            self.parse_select_items()?
        } else {
            vec![]
        };

        Ok(DeleteStatement {
            comments: vec![],
            table,
            using,
            where_clause,
            returning,
        })
    }

    // ── MERGE ───────────────────────────────────────────────────────

    fn parse_merge(&mut self) -> Result<MergeStatement> {
        self.expect(TokenType::Merge)?;
        let _ = self.match_token(TokenType::Into);
        let target = self.parse_table_ref()?;

        self.expect(TokenType::Using)?;
        let source = self.parse_table_source()?;

        self.expect(TokenType::On)?;
        let on = self.parse_expr()?;

        let mut clauses = Vec::new();
        while self.match_token(TokenType::When) {
            clauses.push(self.parse_merge_clause()?);
        }

        if clauses.is_empty() {
            return Err(SqlglotError::ParserError {
                message: "MERGE requires at least one WHEN clause".into(),
            });
        }

        // OUTPUT clause (T-SQL extension)
        let output = if self.match_keyword("OUTPUT") {
            self.parse_select_items()?
        } else {
            vec![]
        };

        Ok(MergeStatement {
            comments: vec![],
            target,
            source,
            on,
            clauses,
            output,
        })
    }

    fn parse_merge_clause(&mut self) -> Result<MergeClause> {
        let kind = if self.match_token(TokenType::Not) {
            self.expect(TokenType::Matched)?;
            if self.match_keyword("BY") {
                if self.match_keyword("SOURCE") {
                    MergeClauseKind::NotMatchedBySource
                } else {
                    // BY TARGET is the default / explicit form
                    let _ = self.match_keyword("TARGET");
                    MergeClauseKind::NotMatched
                }
            } else {
                MergeClauseKind::NotMatched
            }
        } else {
            self.expect(TokenType::Matched)?;
            MergeClauseKind::Matched
        };

        let condition = if self.match_token(TokenType::And) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(TokenType::Then)?;

        let action = self.parse_merge_action(&kind)?;

        Ok(MergeClause {
            kind,
            condition,
            action,
        })
    }

    fn parse_merge_action(&mut self, kind: &MergeClauseKind) -> Result<MergeAction> {
        if self.match_token(TokenType::Update) {
            self.expect(TokenType::Set)?;
            let mut assignments = Vec::new();
            loop {
                let mut col = self.expect_name()?;
                // Support dotted column names like target.col
                while self.match_token(TokenType::Dot) {
                    col.push('.');
                    col.push_str(&self.expect_name()?);
                }
                self.expect(TokenType::Eq)?;
                let val = self.parse_expr()?;
                assignments.push((col, val));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            Ok(MergeAction::Update(assignments))
        } else if self.match_token(TokenType::Insert) {
            // INSERT ROW (BigQuery)
            if self.match_keyword("ROW") {
                return Ok(MergeAction::InsertRow);
            }

            let columns = if self.match_token(TokenType::LParen) {
                let mut cols = vec![self.expect_name()?];
                while self.match_token(TokenType::Comma) {
                    cols.push(self.expect_name()?);
                }
                self.expect(TokenType::RParen)?;
                cols
            } else {
                vec![]
            };

            self.expect(TokenType::Values)?;
            self.expect(TokenType::LParen)?;
            let values = self.parse_expr_list()?;
            self.expect(TokenType::RParen)?;

            Ok(MergeAction::Insert { columns, values })
        } else if self.match_token(TokenType::Delete) {
            Ok(MergeAction::Delete)
        } else {
            Err(SqlglotError::ParserError {
                message: format!(
                    "Expected UPDATE, INSERT, or DELETE after WHEN {} THEN",
                    match kind {
                        MergeClauseKind::Matched => "MATCHED",
                        MergeClauseKind::NotMatched => "NOT MATCHED",
                        MergeClauseKind::NotMatchedBySource => "NOT MATCHED BY SOURCE",
                    }
                ),
            })
        }
    }

    // ── CREATE ──────────────────────────────────────────────────────

    fn parse_create(&mut self) -> Result<Statement> {
        self.expect(TokenType::Create)?;

        let or_replace = if self.check_keyword("OR") {
            self.advance();
            self.expect(TokenType::Replace)?;
            true
        } else {
            false
        };

        let temporary = self.match_token(TokenType::Temporary) || self.match_token(TokenType::Temp);

        let materialized = self.match_token(TokenType::Materialized);

        if self.match_token(TokenType::View) {
            return self
                .parse_create_view(or_replace, materialized)
                .map(Statement::CreateView);
        }

        self.expect(TokenType::Table)?;

        let if_not_exists = if self.match_token(TokenType::If) {
            self.expect(TokenType::Not)?;
            self.expect(TokenType::Exists)?;
            true
        } else {
            false
        };

        let table = self.parse_table_ref_no_alias()?;

        // CREATE TABLE ... AS SELECT ...
        if self.match_token(TokenType::As) {
            let query = self.parse_statement_inner()?;
            return Ok(Statement::CreateTable(CreateTableStatement {
                comments: vec![],
                if_not_exists,
                temporary,
                table,
                columns: vec![],
                constraints: vec![],
                as_select: Some(Box::new(query)),
            }));
        }

        self.expect(TokenType::LParen)?;

        let mut columns = Vec::new();
        let mut constraints = Vec::new();

        loop {
            // Check for table-level constraints
            if matches!(
                self.peek_type(),
                TokenType::Primary
                    | TokenType::Unique
                    | TokenType::Foreign
                    | TokenType::Check
                    | TokenType::Constraint
            ) {
                constraints.push(self.parse_table_constraint()?);
            } else if self.peek_type() != &TokenType::RParen {
                columns.push(self.parse_column_def()?);
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect(TokenType::RParen)?;

        Ok(Statement::CreateTable(CreateTableStatement {
            comments: vec![],
            if_not_exists,
            temporary,
            table,
            columns,
            constraints,
            as_select: None,
        }))
    }

    fn parse_create_view(
        &mut self,
        or_replace: bool,
        materialized: bool,
    ) -> Result<CreateViewStatement> {
        let if_not_exists = if self.match_token(TokenType::If) {
            self.expect(TokenType::Not)?;
            self.expect(TokenType::Exists)?;
            true
        } else {
            false
        };

        // Parse name without alias (so AS is not consumed as an alias)
        let name = self.parse_table_ref_no_alias()?;

        let columns = if self.match_token(TokenType::LParen) {
            let mut cols = vec![self.expect_name()?];
            while self.match_token(TokenType::Comma) {
                cols.push(self.expect_name()?);
            }
            self.expect(TokenType::RParen)?;
            cols
        } else {
            vec![]
        };

        self.expect(TokenType::As)?;
        let query = self.parse_statement_inner()?;

        Ok(CreateViewStatement {
            comments: vec![],
            name,
            columns,
            query: Box::new(query),
            or_replace,
            materialized,
            if_not_exists,
        })
    }

    fn parse_table_constraint(&mut self) -> Result<TableConstraint> {
        let name = if self.match_token(TokenType::Constraint) {
            Some(self.expect_name()?)
        } else {
            None
        };

        if self.match_token(TokenType::Primary) {
            self.expect(TokenType::Key)?;
            self.expect(TokenType::LParen)?;
            let columns = self.parse_name_list()?;
            self.expect(TokenType::RParen)?;
            Ok(TableConstraint::PrimaryKey { name, columns })
        } else if self.match_token(TokenType::Unique) {
            self.expect(TokenType::LParen)?;
            let columns = self.parse_name_list()?;
            self.expect(TokenType::RParen)?;
            Ok(TableConstraint::Unique { name, columns })
        } else if self.match_token(TokenType::Foreign) {
            self.expect(TokenType::Key)?;
            self.expect(TokenType::LParen)?;
            let columns = self.parse_name_list()?;
            self.expect(TokenType::RParen)?;
            self.expect(TokenType::References)?;
            let ref_table = self.parse_table_ref()?;
            self.expect(TokenType::LParen)?;
            let ref_columns = self.parse_name_list()?;
            self.expect(TokenType::RParen)?;

            let on_delete =
                if self.match_token(TokenType::On) && self.match_token(TokenType::Delete) {
                    Some(self.parse_referential_action()?)
                } else {
                    None
                };
            let on_update =
                if self.match_token(TokenType::On) && self.match_token(TokenType::Update) {
                    Some(self.parse_referential_action()?)
                } else {
                    None
                };

            Ok(TableConstraint::ForeignKey {
                name,
                columns,
                ref_table,
                ref_columns,
                on_delete,
                on_update,
            })
        } else if self.match_token(TokenType::Check) {
            self.expect(TokenType::LParen)?;
            let expr = self.parse_expr()?;
            self.expect(TokenType::RParen)?;
            Ok(TableConstraint::Check { name, expr })
        } else {
            Err(SqlglotError::ParserError {
                message: "Expected constraint type".into(),
            })
        }
    }

    fn parse_referential_action(&mut self) -> Result<ReferentialAction> {
        if self.match_token(TokenType::Cascade) {
            Ok(ReferentialAction::Cascade)
        } else if self.match_token(TokenType::Restrict) {
            Ok(ReferentialAction::Restrict)
        } else if self.match_token(TokenType::Set) {
            if self.match_token(TokenType::Null) {
                Ok(ReferentialAction::SetNull)
            } else if self.match_token(TokenType::Default) {
                Ok(ReferentialAction::SetDefault)
            } else {
                Err(SqlglotError::ParserError {
                    message: "Expected NULL or DEFAULT after SET".into(),
                })
            }
        } else if self.check_keyword("NO") {
            self.advance();
            self.expect(TokenType::Identifier)?; // ACTION
            Ok(ReferentialAction::NoAction)
        } else {
            Err(SqlglotError::ParserError {
                message: "Expected referential action (CASCADE, RESTRICT, SET NULL, SET DEFAULT, NO ACTION)".into(),
            })
        }
    }

    fn parse_name_list(&mut self) -> Result<Vec<String>> {
        let mut names = vec![self.expect_name()?];
        while self.match_token(TokenType::Comma) {
            names.push(self.expect_name()?);
        }
        Ok(names)
    }

    fn parse_column_def(&mut self) -> Result<ColumnDef> {
        let name = self.expect_name()?;
        let data_type = self.parse_data_type()?;

        let mut nullable = None;
        let mut default = None;
        let mut primary_key = false;
        let mut unique = false;
        let mut auto_increment = false;
        let mut collation = None;
        let mut comment = None;

        loop {
            if self.match_token(TokenType::Not) {
                self.expect(TokenType::Null)?;
                nullable = Some(false);
            } else if self.peek_type() == &TokenType::Null {
                self.advance();
                nullable = Some(true);
            } else if self.match_token(TokenType::Default) {
                default = Some(self.parse_expr()?);
            } else if self.match_token(TokenType::Primary) {
                self.expect(TokenType::Key)?;
                primary_key = true;
            } else if self.match_token(TokenType::Unique) {
                unique = true;
            } else if self.match_token(TokenType::AutoIncrement) {
                auto_increment = true;
            } else if self.match_token(TokenType::Collate) {
                collation = Some(self.expect_name()?);
            } else if self.match_token(TokenType::Comment) {
                let tok = self.expect(TokenType::String)?;
                comment = Some(tok.value);
            } else if self.match_token(TokenType::References) {
                // Inline foreign key — skip for now
                let _ = self.parse_table_ref()?;
                if self.match_token(TokenType::LParen) {
                    while !self.match_token(TokenType::RParen) {
                        self.advance();
                    }
                }
            } else {
                break;
            }
        }

        Ok(ColumnDef {
            name,
            data_type,
            nullable,
            default,
            primary_key,
            unique,
            auto_increment,
            collation,
            comment,
        })
    }

    fn parse_data_type(&mut self) -> Result<DataType> {
        let token = self.peek().clone();
        let type_result = match &token.token_type {
            TokenType::Int | TokenType::Integer => {
                self.advance();
                Ok(DataType::Int)
            }
            TokenType::BigInt => {
                self.advance();
                Ok(DataType::BigInt)
            }
            TokenType::SmallInt => {
                self.advance();
                Ok(DataType::SmallInt)
            }
            TokenType::TinyInt => {
                self.advance();
                Ok(DataType::TinyInt)
            }
            TokenType::Float => {
                self.advance();
                Ok(DataType::Float)
            }
            TokenType::Double => {
                self.advance();
                let _ = self.match_keyword("PRECISION");
                Ok(DataType::Double)
            }
            TokenType::Real => {
                self.advance();
                Ok(DataType::Real)
            }
            TokenType::Decimal | TokenType::Numeric => {
                let is_numeric = token.token_type == TokenType::Numeric;
                self.advance();
                let (precision, scale) = self.parse_type_params()?;
                if is_numeric {
                    Ok(DataType::Numeric { precision, scale })
                } else {
                    Ok(DataType::Decimal { precision, scale })
                }
            }
            TokenType::Varchar => {
                self.advance();
                let len = self.parse_single_type_param()?;
                Ok(DataType::Varchar(len))
            }
            TokenType::Char => {
                self.advance();
                let len = self.parse_single_type_param()?;
                Ok(DataType::Char(len))
            }
            TokenType::Text => {
                self.advance();
                Ok(DataType::Text)
            }
            TokenType::Boolean => {
                self.advance();
                Ok(DataType::Boolean)
            }
            TokenType::Date => {
                self.advance();
                Ok(DataType::Date)
            }
            TokenType::Timestamp => {
                self.advance();
                let precision = self.parse_single_type_param()?;
                let with_tz = if self.match_keyword("WITH") {
                    let _ = self.match_keyword("TIME");
                    let _ = self.match_keyword("ZONE");
                    true
                } else if self.match_keyword("WITHOUT") {
                    let _ = self.match_keyword("TIME");
                    let _ = self.match_keyword("ZONE");
                    false
                } else {
                    false
                };
                Ok(DataType::Timestamp { precision, with_tz })
            }
            TokenType::TimestampTz => {
                self.advance();
                let precision = self.parse_single_type_param()?;
                Ok(DataType::Timestamp {
                    precision,
                    with_tz: true,
                })
            }
            TokenType::Time => {
                self.advance();
                let precision = self.parse_single_type_param()?;
                Ok(DataType::Time { precision })
            }
            TokenType::Interval => {
                self.advance();
                Ok(DataType::Interval)
            }
            TokenType::Blob => {
                self.advance();
                Ok(DataType::Blob)
            }
            TokenType::Bytea => {
                self.advance();
                Ok(DataType::Bytea)
            }
            TokenType::Json => {
                self.advance();
                Ok(DataType::Json)
            }
            TokenType::Jsonb => {
                self.advance();
                Ok(DataType::Jsonb)
            }
            TokenType::Uuid => {
                self.advance();
                Ok(DataType::Uuid)
            }
            TokenType::Array => {
                self.advance();
                if self.match_token(TokenType::Lt) {
                    let inner = self.parse_data_type()?;
                    self.expect(TokenType::Gt)?;
                    Ok(DataType::Array(Some(Box::new(inner))))
                } else {
                    Ok(DataType::Array(None))
                }
            }
            TokenType::Identifier => {
                let name = token.value.to_uppercase();
                self.advance();
                match name.as_str() {
                    "STRING" => Ok(DataType::String),
                    "BINARY" => {
                        let len = self.parse_single_type_param()?;
                        Ok(DataType::Binary(len))
                    }
                    "VARBINARY" => {
                        let len = self.parse_single_type_param()?;
                        Ok(DataType::Varbinary(len))
                    }
                    "DATETIME" => Ok(DataType::DateTime),
                    "BYTES" => Ok(DataType::Bytes),
                    "VARIANT" => Ok(DataType::Variant),
                    "OBJECT" => Ok(DataType::Object),
                    "XML" => Ok(DataType::Xml),
                    "INET" => Ok(DataType::Inet),
                    "CIDR" => Ok(DataType::Cidr),
                    "MACADDR" => Ok(DataType::Macaddr),
                    "BIT" => {
                        let len = self.parse_single_type_param()?;
                        Ok(DataType::Bit(len))
                    }
                    "MONEY" => Ok(DataType::Money),
                    "SERIAL" => Ok(DataType::Serial),
                    "BIGSERIAL" => Ok(DataType::BigSerial),
                    "SMALLSERIAL" => Ok(DataType::SmallSerial),
                    "REGCLASS" => Ok(DataType::Regclass),
                    "REGTYPE" => Ok(DataType::Regtype),
                    "HSTORE" => Ok(DataType::Hstore),
                    "GEOGRAPHY" => Ok(DataType::Geography),
                    "GEOMETRY" => Ok(DataType::Geometry),
                    "SUPER" => Ok(DataType::Super),
                    _ => Ok(DataType::Unknown(name)),
                }
            }
            _ => Err(SqlglotError::ParserError {
                message: format!("Expected data type, got {:?}", token.token_type),
            }),
        };

        // PostgreSQL opt_array_bounds: typename[], typename[N], typename[][]...
        let mut dt = type_result?;
        while self.match_token(TokenType::LBracket) {
            // Consume optional integer bound (PostgreSQL ignores it but accepts it)
            let _ = self.match_token(TokenType::Number);
            self.expect(TokenType::RBracket)?;
            dt = DataType::Array(Some(Box::new(dt)));
        }
        Ok(dt)
    }

    fn parse_type_params(&mut self) -> Result<(Option<u32>, Option<u32>)> {
        if self.match_token(TokenType::LParen) {
            let p: Option<u32> = self.expect(TokenType::Number)?.value.parse().ok();
            let s = if self.match_token(TokenType::Comma) {
                self.expect(TokenType::Number)?.value.parse().ok()
            } else {
                None
            };
            self.expect(TokenType::RParen)?;
            Ok((p, s))
        } else {
            Ok((None, None))
        }
    }

    fn parse_single_type_param(&mut self) -> Result<Option<u32>> {
        if self.match_token(TokenType::LParen) {
            // Handle TSQL MAX keyword (e.g. VARBINARY(MAX), VARCHAR(MAX))
            if self.check_keyword("MAX") {
                self.advance(); // consume MAX
                self.expect(TokenType::RParen)?;
                return Ok(None);
            }
            let n: Option<u32> = self.expect(TokenType::Number)?.value.parse().ok();
            self.expect(TokenType::RParen)?;
            Ok(n)
        } else {
            Ok(None)
        }
    }

    // ── DROP ────────────────────────────────────────────────────────

    fn parse_drop(&mut self) -> Result<Statement> {
        self.expect(TokenType::Drop)?;

        if self.match_token(TokenType::Materialized) {
            self.expect(TokenType::View)?;
            let if_exists = if self.match_token(TokenType::If) {
                self.expect(TokenType::Exists)?;
                true
            } else {
                false
            };
            let name = self.parse_table_ref()?;
            return Ok(Statement::DropView(DropViewStatement {
                comments: vec![],
                name,
                if_exists,
                materialized: true,
            }));
        }

        if self.match_token(TokenType::View) {
            let if_exists = if self.match_token(TokenType::If) {
                self.expect(TokenType::Exists)?;
                true
            } else {
                false
            };
            let name = self.parse_table_ref()?;
            return Ok(Statement::DropView(DropViewStatement {
                comments: vec![],
                name,
                if_exists,
                materialized: false,
            }));
        }

        self.expect(TokenType::Table)?;

        let if_exists = if self.match_token(TokenType::If) {
            self.expect(TokenType::Exists)?;
            true
        } else {
            false
        };

        let table = self.parse_table_ref()?;
        let cascade = self.match_token(TokenType::Cascade);

        Ok(Statement::DropTable(DropTableStatement {
            comments: vec![],
            if_exists,
            table,
            cascade,
        }))
    }

    // ── ALTER TABLE ─────────────────────────────────────────────────

    fn parse_alter_table(&mut self) -> Result<AlterTableStatement> {
        self.expect(TokenType::Alter)?;
        self.expect(TokenType::Table)?;
        let table = self.parse_table_ref_no_alias()?;

        let mut actions = Vec::new();
        loop {
            let action = self.parse_alter_action()?;
            actions.push(action);
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(AlterTableStatement {
            comments: vec![],
            table,
            actions,
        })
    }

    fn parse_alter_action(&mut self) -> Result<AlterTableAction> {
        if self.match_keyword("ADD") {
            if matches!(
                self.peek_type(),
                TokenType::Constraint
                    | TokenType::Primary
                    | TokenType::Unique
                    | TokenType::Foreign
                    | TokenType::Check
            ) {
                let constraint = self.parse_table_constraint()?;
                Ok(AlterTableAction::AddConstraint(constraint))
            } else {
                let _ = self.match_keyword("COLUMN");
                let col = self.parse_column_def()?;
                Ok(AlterTableAction::AddColumn(col))
            }
        } else if self.match_token(TokenType::Drop) {
            let _ = self.match_keyword("COLUMN");
            let if_exists = if self.match_token(TokenType::If) {
                self.expect(TokenType::Exists)?;
                true
            } else {
                false
            };
            let name = self.expect_name()?;
            Ok(AlterTableAction::DropColumn { name, if_exists })
        } else if self.match_keyword("RENAME") {
            if self.match_keyword("COLUMN") {
                let old_name = self.expect_name()?;
                self.expect(TokenType::Identifier)?; // TO
                let new_name = self.expect_name()?;
                Ok(AlterTableAction::RenameColumn { old_name, new_name })
            } else if self.match_keyword("TO") {
                let new_name = self.expect_name()?;
                Ok(AlterTableAction::RenameTable { new_name })
            } else {
                Err(SqlglotError::ParserError {
                    message: "Expected COLUMN or TO after RENAME".into(),
                })
            }
        } else {
            Err(SqlglotError::ParserError {
                message: "Expected ADD, DROP, or RENAME in ALTER TABLE".into(),
            })
        }
    }

    // ── TRUNCATE ────────────────────────────────────────────────────

    fn parse_truncate(&mut self) -> Result<TruncateStatement> {
        self.expect(TokenType::Truncate)?;
        let _ = self.match_token(TokenType::Table);
        let table = self.parse_table_ref()?;
        Ok(TruncateStatement {
            comments: vec![],
            table,
        })
    }

    // ── Transaction ─────────────────────────────────────────────────

    fn parse_transaction(&mut self) -> Result<TransactionStatement> {
        match self.peek_type() {
            TokenType::Begin => {
                self.advance();
                let _ = self.match_token(TokenType::Transaction);
                Ok(TransactionStatement::Begin)
            }
            TokenType::Commit => {
                self.advance();
                let _ = self.match_token(TokenType::Transaction);
                Ok(TransactionStatement::Commit)
            }
            TokenType::Rollback => {
                self.advance();
                let _ = self.match_token(TokenType::Transaction);
                if self.match_keyword("TO") {
                    let _ = self.match_token(TokenType::Savepoint);
                    let name = self.expect_name()?;
                    Ok(TransactionStatement::RollbackTo(name))
                } else {
                    Ok(TransactionStatement::Rollback)
                }
            }
            TokenType::Savepoint => {
                self.advance();
                let name = self.expect_name()?;
                Ok(TransactionStatement::Savepoint(name))
            }
            _ => Err(SqlglotError::ParserError {
                message: "Expected transaction statement".into(),
            }),
        }
    }

    // ── EXPLAIN ─────────────────────────────────────────────────────

    fn parse_explain(&mut self) -> Result<ExplainStatement> {
        self.expect(TokenType::Explain)?;
        let analyze = self.match_token(TokenType::Analyze);
        let statement = self.parse_statement_inner()?;
        Ok(ExplainStatement {
            comments: vec![],
            analyze,
            statement: Box::new(statement),
        })
    }

    // ── USE ─────────────────────────────────────────────────────────

    fn parse_use(&mut self) -> Result<UseStatement> {
        self.expect(TokenType::Use)?;
        let name = self.expect_name()?;
        Ok(UseStatement {
            comments: vec![],
            name,
        })
    }

    // ══════════════════════════════════════════════════════════════
    // Expression parsing (precedence climbing)
    // ══════════════════════════════════════════════════════════════

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<Expr> {
        let mut left = self.parse_and_expr()?;
        while self.match_token(TokenType::Or) {
            let right = self.parse_and_expr()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<Expr> {
        let mut left = self.parse_not_expr()?;
        while self.match_token(TokenType::And) {
            let right = self.parse_not_expr()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_not_expr(&mut self) -> Result<Expr> {
        if self.match_token(TokenType::Not) {
            let expr = self.parse_not_expr()?;
            Ok(Expr::UnaryOp {
                op: UnaryOperator::Not,
                expr: Box::new(expr),
            })
        } else {
            self.parse_comparison()
        }
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let mut left = self.parse_addition()?;

        loop {
            let op = match self.peek_type() {
                TokenType::Eq => Some(BinaryOperator::Eq),
                TokenType::Neq => Some(BinaryOperator::Neq),
                TokenType::Lt => Some(BinaryOperator::Lt),
                TokenType::Gt => Some(BinaryOperator::Gt),
                TokenType::LtEq => Some(BinaryOperator::LtEq),
                TokenType::GtEq => Some(BinaryOperator::GtEq),
                _ => None,
            };

            if let Some(op) = op {
                self.advance();
                if matches!(self.peek_type(), TokenType::Any | TokenType::Some) {
                    self.advance();
                    self.expect(TokenType::LParen)?;
                    let right = if matches!(self.peek_type(), TokenType::Select | TokenType::With) {
                        Expr::Subquery(Box::new(self.parse_statement_inner()?))
                    } else {
                        self.parse_expr()?
                    };
                    self.expect(TokenType::RParen)?;
                    left = Expr::AnyOp {
                        expr: Box::new(left),
                        op,
                        right: Box::new(right),
                    };
                } else if self.peek_type() == &TokenType::All {
                    self.advance();
                    self.expect(TokenType::LParen)?;
                    let right = if matches!(self.peek_type(), TokenType::Select | TokenType::With) {
                        Expr::Subquery(Box::new(self.parse_statement_inner()?))
                    } else {
                        self.parse_expr()?
                    };
                    self.expect(TokenType::RParen)?;
                    left = Expr::AllOp {
                        expr: Box::new(left),
                        op,
                        right: Box::new(right),
                    };
                } else {
                    let right = self.parse_addition()?;
                    left = Expr::BinaryOp {
                        left: Box::new(left),
                        op,
                        right: Box::new(right),
                    };
                }
            } else if self.peek_type() == &TokenType::Is {
                self.advance();
                let negated = self.match_token(TokenType::Not);
                if self.match_token(TokenType::True) {
                    left = Expr::IsBool {
                        expr: Box::new(left),
                        value: true,
                        negated,
                    };
                } else if self.match_token(TokenType::False) {
                    left = Expr::IsBool {
                        expr: Box::new(left),
                        value: false,
                        negated,
                    };
                } else {
                    self.expect(TokenType::Null)?;
                    left = Expr::IsNull {
                        expr: Box::new(left),
                        negated,
                    };
                }
            } else if matches!(
                self.peek_type(),
                TokenType::Not
                    | TokenType::In
                    | TokenType::Like
                    | TokenType::ILike
                    | TokenType::Between
            ) {
                // Peek ahead: if NOT, only consume it if followed by IN/LIKE/ILIKE/BETWEEN
                if self.peek_type() == &TokenType::Not {
                    let saved_pos = self.pos;
                    self.advance(); // consume NOT
                    if !matches!(
                        self.peek_type(),
                        TokenType::In | TokenType::Like | TokenType::ILike | TokenType::Between
                    ) {
                        // NOT is not part of a comparison predicate — restore position
                        self.pos = saved_pos;
                        break;
                    }
                    // NOT was consumed, negated = true
                }
                let negated =
                    self.pos > 0 && self.tokens[self.pos - 1].token_type == TokenType::Not;

                if self.match_token(TokenType::In) {
                    self.expect(TokenType::LParen)?;
                    // Check for subquery
                    if matches!(self.peek_type(), TokenType::Select | TokenType::With) {
                        let subquery = self.parse_statement_inner()?;
                        self.expect(TokenType::RParen)?;
                        left = Expr::InSubquery {
                            expr: Box::new(left),
                            subquery: Box::new(subquery),
                            negated,
                        };
                    } else {
                        let list = self.parse_expr_list()?;
                        self.expect(TokenType::RParen)?;
                        left = Expr::InList {
                            expr: Box::new(left),
                            list,
                            negated,
                        };
                    }
                } else if self.match_token(TokenType::Like) {
                    let pattern = self.parse_addition()?;
                    let escape = if self.match_token(TokenType::Escape) {
                        Some(Box::new(self.parse_primary()?))
                    } else {
                        None
                    };
                    left = Expr::Like {
                        expr: Box::new(left),
                        pattern: Box::new(pattern),
                        negated,
                        escape,
                    };
                } else if self.match_token(TokenType::ILike) {
                    let pattern = self.parse_addition()?;
                    let escape = if self.match_token(TokenType::Escape) {
                        Some(Box::new(self.parse_primary()?))
                    } else {
                        None
                    };
                    left = Expr::ILike {
                        expr: Box::new(left),
                        pattern: Box::new(pattern),
                        negated,
                        escape,
                    };
                } else if self.match_token(TokenType::Between) {
                    let low = self.parse_addition()?;
                    self.expect(TokenType::And)?;
                    let high = self.parse_addition()?;
                    left = Expr::Between {
                        expr: Box::new(left),
                        low: Box::new(low),
                        high: Box::new(high),
                        negated,
                    };
                } else {
                    break;
                }
            } else if self.check_keyword("SIMILAR") {
                // SIMILAR TO pattern [ESCAPE escape_char]
                self.advance(); // consume SIMILAR
                self.expect_keyword("TO")?;
                let pattern = self.parse_addition()?;
                let escape = if self.match_token(TokenType::Escape) {
                    Some(Box::new(self.parse_primary()?))
                } else {
                    None
                };
                left = Expr::SimilarTo {
                    expr: Box::new(left),
                    pattern: Box::new(pattern),
                    negated: false,
                    escape,
                };
            } else if self.peek_type() == &TokenType::Not && self.check_keyword_offset("SIMILAR", 1)
            {
                // NOT SIMILAR TO pattern [ESCAPE escape_char]
                self.advance(); // consume NOT
                self.advance(); // consume SIMILAR
                self.expect_keyword("TO")?;
                let pattern = self.parse_addition()?;
                let escape = if self.match_token(TokenType::Escape) {
                    Some(Box::new(self.parse_primary()?))
                } else {
                    None
                };
                left = Expr::SimilarTo {
                    expr: Box::new(left),
                    pattern: Box::new(pattern),
                    negated: true,
                    escape,
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expr> {
        let mut left = self.parse_multiplication()?;
        loop {
            let op = match self.peek_type() {
                TokenType::Plus => Some(BinaryOperator::Plus),
                TokenType::Minus => Some(BinaryOperator::Minus),
                TokenType::Concat => Some(BinaryOperator::Concat),
                TokenType::BitwiseOr => Some(BinaryOperator::BitwiseOr),
                TokenType::BitwiseXor => Some(BinaryOperator::BitwiseXor),
                TokenType::ShiftLeft => Some(BinaryOperator::ShiftLeft),
                TokenType::ShiftRight => Some(BinaryOperator::ShiftRight),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_multiplication()?;
                left = Expr::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_type() {
                TokenType::Star => Some(BinaryOperator::Multiply),
                TokenType::Slash => Some(BinaryOperator::Divide),
                TokenType::Percent2 => Some(BinaryOperator::Modulo),
                TokenType::BitwiseAnd => Some(BinaryOperator::BitwiseAnd),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_unary()?;
                left = Expr::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        match self.peek_type() {
            TokenType::Minus => {
                self.advance();
                let expr = self.parse_postfix()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOperator::Minus,
                    expr: Box::new(expr),
                })
            }
            TokenType::Plus => {
                self.advance();
                let expr = self.parse_postfix()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOperator::Plus,
                    expr: Box::new(expr),
                })
            }
            TokenType::BitwiseNot => {
                self.advance();
                let expr = self.parse_postfix()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOperator::BitwiseNot,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    /// Parse postfix operators: `::type`, `[index]`, `->`, `->>`
    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(TokenType::DoubleColon) {
                // PostgreSQL-style cast: expr::type
                let data_type = self.parse_data_type()?;
                expr = Expr::Cast {
                    expr: Box::new(expr),
                    data_type,
                };
            } else if self.match_token(TokenType::LBracket) {
                // Array index: expr[index]
                let index = self.parse_expr()?;
                self.expect(TokenType::RBracket)?;
                expr = Expr::ArrayIndex {
                    expr: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.match_token(TokenType::Arrow) {
                let path = self.parse_primary()?;
                expr = Expr::JsonAccess {
                    expr: Box::new(expr),
                    path: Box::new(path),
                    as_text: false,
                };
            } else if self.match_token(TokenType::DoubleArrow) {
                let path = self.parse_primary()?;
                expr = Expr::JsonAccess {
                    expr: Box::new(expr),
                    path: Box::new(path),
                    as_text: true,
                };
            } else {
                break;
            }
        }

        // Check for window function: expr OVER (...)
        if self.match_token(TokenType::Over) {
            let spec = if self.match_token(TokenType::LParen) {
                let ws = self.parse_window_spec()?;
                self.expect(TokenType::RParen)?;
                ws
            } else {
                // Named window reference
                let wref = self.expect_name()?;
                WindowSpec {
                    window_ref: Some(wref),
                    partition_by: vec![],
                    order_by: vec![],
                    frame: None,
                }
            };
            match expr {
                Expr::Function {
                    name,
                    args,
                    distinct,
                    filter,
                    ..
                } => {
                    expr = Expr::Function {
                        name,
                        args,
                        distinct,
                        filter,
                        over: Some(spec),
                    };
                }
                Expr::TypedFunction { func, filter, .. } => {
                    expr = Expr::TypedFunction {
                        func,
                        filter,
                        over: Some(spec),
                    };
                }
                _ => {}
            }
        }

        // FILTER (WHERE ...) for aggregate functions
        if self.match_token(TokenType::Filter) {
            self.expect(TokenType::LParen)?;
            self.expect(TokenType::Where)?;
            let filter_expr = self.parse_expr()?;
            self.expect(TokenType::RParen)?;
            match expr {
                Expr::Function {
                    name,
                    args,
                    distinct,
                    over,
                    ..
                } => {
                    expr = Expr::Function {
                        name,
                        args,
                        distinct,
                        filter: Some(Box::new(filter_expr)),
                        over,
                    };
                }
                Expr::TypedFunction { func, over, .. } => {
                    expr = Expr::TypedFunction {
                        func,
                        filter: Some(Box::new(filter_expr)),
                        over,
                    };
                }
                _ => {}
            }
        }

        Ok(expr)
    }

    fn parse_window_spec(&mut self) -> Result<WindowSpec> {
        let window_ref = if self.is_name_token()
            && !matches!(
                self.peek_type(),
                TokenType::Partition | TokenType::Order | TokenType::Rows | TokenType::Range
            ) {
            let saved = self.pos;
            let name = self.expect_name()?;
            // Check if it's actually a keyword we need
            if matches!(
                self.peek_type(),
                TokenType::RParen
                    | TokenType::Partition
                    | TokenType::Order
                    | TokenType::Rows
                    | TokenType::Range
            ) {
                Some(name)
            } else {
                self.pos = saved;
                None
            }
        } else {
            None
        };

        let partition_by = if self.match_token(TokenType::Partition) {
            self.expect(TokenType::By)?;
            self.parse_expr_list()?
        } else {
            vec![]
        };

        let order_by = if self.match_token(TokenType::Order) {
            self.expect(TokenType::By)?;
            self.parse_order_by_items()?
        } else {
            vec![]
        };

        let frame = if matches!(self.peek_type(), TokenType::Rows | TokenType::Range) {
            Some(self.parse_window_frame()?)
        } else {
            None
        };

        Ok(WindowSpec {
            window_ref,
            partition_by,
            order_by,
            frame,
        })
    }

    fn parse_window_frame(&mut self) -> Result<WindowFrame> {
        let kind = if self.match_token(TokenType::Rows) {
            WindowFrameKind::Rows
        } else if self.match_token(TokenType::Range) {
            WindowFrameKind::Range
        } else {
            WindowFrameKind::Rows
        };

        if self.match_keyword("BETWEEN") {
            let start = self.parse_window_frame_bound()?;
            self.expect(TokenType::And)?;
            let end = self.parse_window_frame_bound()?;
            Ok(WindowFrame {
                kind,
                start,
                end: Some(end),
            })
        } else {
            let start = self.parse_window_frame_bound()?;
            Ok(WindowFrame {
                kind,
                start,
                end: None,
            })
        }
    }

    fn parse_window_frame_bound(&mut self) -> Result<WindowFrameBound> {
        if self.check_keyword("CURRENT") {
            self.advance();
            let _ = self.match_keyword("ROW");
            Ok(WindowFrameBound::CurrentRow)
        } else if self.match_token(TokenType::Unbounded) {
            if self.match_token(TokenType::Preceding) {
                Ok(WindowFrameBound::Preceding(None))
            } else {
                self.expect(TokenType::Following)?;
                Ok(WindowFrameBound::Following(None))
            }
        } else {
            let n = self.parse_expr()?;
            if self.match_token(TokenType::Preceding) {
                Ok(WindowFrameBound::Preceding(Some(Box::new(n))))
            } else {
                self.expect(TokenType::Following)?;
                Ok(WindowFrameBound::Following(Some(Box::new(n))))
            }
        }
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        let token = self.peek().clone();

        match &token.token_type {
            TokenType::Number => {
                self.advance();
                Ok(Expr::Number(token.value))
            }
            TokenType::String => {
                self.advance();
                Ok(Expr::StringLiteral(token.value))
            }
            TokenType::NationalString => {
                self.advance();
                Ok(Expr::NationalStringLiteral(token.value))
            }
            TokenType::True => {
                self.advance();
                Ok(Expr::Boolean(true))
            }
            TokenType::False => {
                self.advance();
                Ok(Expr::Boolean(false))
            }
            TokenType::Null => {
                self.advance();
                Ok(Expr::Null)
            }
            TokenType::Default => {
                self.advance();
                Ok(Expr::Default)
            }
            TokenType::Star => {
                self.advance();
                Ok(Expr::Wildcard)
            }
            TokenType::Parameter => {
                self.advance();
                Ok(Expr::Parameter(token.value))
            }

            // ── CAST ────────────────────────────────────────────────
            TokenType::Cast => {
                self.advance();
                self.expect(TokenType::LParen)?;
                let expr = self.parse_expr()?;
                self.expect(TokenType::As)?;
                let data_type = self.parse_data_type()?;
                self.expect(TokenType::RParen)?;
                Ok(Expr::Cast {
                    expr: Box::new(expr),
                    data_type,
                })
            }

            // ── EXTRACT ─────────────────────────────────────────────
            TokenType::Extract => {
                self.advance();
                self.expect(TokenType::LParen)?;
                let field = self.parse_datetime_field()?;
                self.expect(TokenType::From)?;
                let expr = self.parse_expr()?;
                self.expect(TokenType::RParen)?;
                Ok(Expr::Extract {
                    field,
                    expr: Box::new(expr),
                })
            }

            // ── CASE ────────────────────────────────────────────────
            TokenType::Case => self.parse_case_expr(),

            // ── EXISTS ──────────────────────────────────────────────
            TokenType::Exists => {
                self.advance();
                self.expect(TokenType::LParen)?;
                let subquery = self.parse_statement_inner()?;
                self.expect(TokenType::RParen)?;
                Ok(Expr::Exists {
                    subquery: Box::new(subquery),
                    negated: false,
                })
            }

            // ── NOT EXISTS ──────────────────────────────────────────
            TokenType::Not
                if {
                    let next_pos = self.pos + 1;
                    next_pos < self.tokens.len()
                        && self.tokens[next_pos].token_type == TokenType::Exists
                } =>
            {
                self.advance(); // NOT
                self.advance(); // EXISTS
                self.expect(TokenType::LParen)?;
                let subquery = self.parse_statement_inner()?;
                self.expect(TokenType::RParen)?;
                Ok(Expr::Exists {
                    subquery: Box::new(subquery),
                    negated: true,
                })
            }

            // ── INTERVAL ────────────────────────────────────────────
            TokenType::Interval => {
                self.advance();
                let value = self.parse_primary()?;
                let unit = self.try_parse_datetime_field();
                Ok(Expr::Interval {
                    value: Box::new(value),
                    unit,
                })
            }

            // ── Parenthesized expression or subquery ────────────────
            TokenType::LParen => {
                self.advance();
                // Check for subquery
                if matches!(self.peek_type(), TokenType::Select | TokenType::With) {
                    let subquery = self.parse_statement_inner()?;
                    self.expect(TokenType::RParen)?;
                    Ok(Expr::Subquery(Box::new(subquery)))
                } else {
                    let expr = self.parse_expr()?;
                    // Tuple: (a, b, c)
                    if self.match_token(TokenType::Comma) {
                        let mut items = vec![expr];
                        items.push(self.parse_expr()?);
                        while self.match_token(TokenType::Comma) {
                            items.push(self.parse_expr()?);
                        }
                        self.expect(TokenType::RParen)?;
                        Ok(Expr::Tuple(items))
                    } else {
                        self.expect(TokenType::RParen)?;
                        Ok(Expr::Nested(Box::new(expr)))
                    }
                }
            }

            // ── Array literal: ARRAY[...] ──────────────────────────
            TokenType::Array => {
                self.advance();
                if self.match_token(TokenType::LBracket) {
                    let items = if self.peek_type() != &TokenType::RBracket {
                        self.parse_expr_list()?
                    } else {
                        vec![]
                    };
                    self.expect(TokenType::RBracket)?;
                    Ok(Expr::ArrayLiteral(items))
                } else if self.match_token(TokenType::LParen) {
                    // ARRAY(SELECT ...)
                    let subquery = self.parse_statement_inner()?;
                    self.expect(TokenType::RParen)?;
                    Ok(Expr::Subquery(Box::new(subquery)))
                } else {
                    Ok(Expr::Column {
                        table: None,
                        name: "ARRAY".to_string(),
                        quote_style: QuoteStyle::None,
                        table_quote_style: QuoteStyle::None,
                    })
                }
            }

            // ── Bracket array literal: [...] ────────────────────────
            TokenType::LBracket => {
                self.advance();
                let items = if self.peek_type() != &TokenType::RBracket {
                    self.parse_expr_list()?
                } else {
                    vec![]
                };
                self.expect(TokenType::RBracket)?;
                Ok(Expr::ArrayLiteral(items))
            }

            // ── Identifier: column ref, function call, or qualified name ─
            _ if self.is_name_token() || self.is_data_type_token() => {
                let name_token = self.advance().clone();
                let name = name_token.value.clone();
                let name_qs = quote_style_from_char(name_token.quote_char);

                // ── ANSI typed string literals: DATE 'x', TIMESTAMP 'x', TIME 'x' ──
                if matches!(
                    name_token.token_type,
                    TokenType::Date
                        | TokenType::Timestamp
                        | TokenType::TimestampTz
                        | TokenType::Time
                ) && self.peek_type() == &TokenType::String
                {
                    let value_token = self.advance().clone();
                    let data_type = match name_token.token_type {
                        TokenType::Date => DataType::Date,
                        TokenType::Timestamp => DataType::Timestamp {
                            precision: None,
                            with_tz: false,
                        },
                        TokenType::TimestampTz => DataType::Timestamp {
                            precision: None,
                            with_tz: true,
                        },
                        TokenType::Time => DataType::Time { precision: None },
                        _ => unreachable!(),
                    };
                    return Ok(Expr::Cast {
                        expr: Box::new(Expr::StringLiteral(value_token.value)),
                        data_type,
                    });
                }

                // ── Bare niladic temporal keywords: CURRENT_TIME, CURRENT_DATE,
                //    CURRENT_TIMESTAMP, LOCALTIMESTAMP (no parens) ──
                // ANSI SQL allows these without parentheses. Materialize them
                // as typed functions so the generator can emit dialect-specific
                // forms (e.g. TSQL requires CAST(GETDATE() AS TIME) rather than
                // a bare CURRENT_TIME reserved word).
                if name_qs == QuoteStyle::None && self.peek_type() != &TokenType::LParen {
                    let upper = name.to_ascii_uppercase();
                    let typed = match upper.as_str() {
                        "CURRENT_DATE" => Some(TypedFunction::CurrentDate),
                        "CURRENT_TIME" => Some(TypedFunction::CurrentTime),
                        "CURRENT_TIMESTAMP" | "LOCALTIMESTAMP" => {
                            Some(TypedFunction::CurrentTimestamp)
                        }
                        _ => None,
                    };
                    if let Some(tf) = typed {
                        return Ok(Expr::TypedFunction {
                            func: tf,
                            filter: None,
                            over: None,
                        });
                    }
                }

                // Function call: name(...)
                if self.peek_type() == &TokenType::LParen {
                    self.advance();

                    // Special: COUNT(*), COUNT(DISTINCT x)
                    let distinct = self.match_token(TokenType::Distinct);

                    // MySQL's GROUP_CONCAT has bespoke grammar
                    // (ORDER BY ..., SEPARATOR ...) — parse it into a typed
                    // expression so the structure is preserved across dialects.
                    if name.eq_ignore_ascii_case("GROUP_CONCAT") {
                        let expr = self.parse_group_concat_call(distinct)?;
                        self.expect(TokenType::RParen)?;
                        return Ok(expr);
                    }

                    let args = if self.peek_type() == &TokenType::RParen {
                        vec![]
                    } else if self.peek_type() == &TokenType::Star {
                        self.advance();
                        vec![Expr::Wildcard]
                    } else {
                        self.parse_expr_list()?
                    };
                    self.expect(TokenType::RParen)?;

                    // Try to construct a typed function variant
                    if let Some(typed) = Self::try_typed_function(&name, args.clone(), distinct) {
                        Ok(typed)
                    } else {
                        Ok(Expr::Function {
                            name,
                            args,
                            distinct,
                            filter: None,
                            over: None,
                        })
                    }
                }
                // Qualified column: table.column or table.*
                else if self.match_token(TokenType::Dot) {
                    if self.peek_type() == &TokenType::Star {
                        self.advance();
                        Ok(Expr::QualifiedWildcard { table: name })
                    } else {
                        let (col, col_qs) = self.expect_name_with_quote()?;
                        Ok(Expr::Column {
                            table: Some(name),
                            name: col,
                            quote_style: col_qs,
                            table_quote_style: name_qs,
                        })
                    }
                } else {
                    Ok(Expr::Column {
                        table: None,
                        name,
                        quote_style: name_qs,
                        table_quote_style: QuoteStyle::None,
                    })
                }
            }

            _ => Err(SqlglotError::UnexpectedToken { token }),
        }
    }

    fn is_data_type_token(&self) -> bool {
        matches!(
            self.peek_type(),
            TokenType::Int
                | TokenType::Integer
                | TokenType::BigInt
                | TokenType::SmallInt
                | TokenType::TinyInt
                | TokenType::Float
                | TokenType::Double
                | TokenType::Decimal
                | TokenType::Numeric
                | TokenType::Real
                | TokenType::Varchar
                | TokenType::Char
                | TokenType::Text
                | TokenType::Boolean
                | TokenType::Date
                | TokenType::Timestamp
                | TokenType::TimestampTz
                | TokenType::Time
                | TokenType::Interval
                | TokenType::Blob
                | TokenType::Bytea
                | TokenType::Json
                | TokenType::Jsonb
                | TokenType::Uuid
                | TokenType::Array
                | TokenType::Map
                | TokenType::Struct
        )
    }

    fn parse_datetime_field(&mut self) -> Result<DateTimeField> {
        let token = self.peek().clone();
        let field = match &token.token_type {
            TokenType::Year => DateTimeField::Year,
            TokenType::Month => DateTimeField::Month,
            TokenType::Day => DateTimeField::Day,
            TokenType::Hour => DateTimeField::Hour,
            TokenType::Minute => DateTimeField::Minute,
            TokenType::Second => DateTimeField::Second,
            TokenType::Epoch => DateTimeField::Epoch,
            _ => {
                let name = token.value.to_uppercase();
                match name.as_str() {
                    "YEAR" => DateTimeField::Year,
                    "QUARTER" => DateTimeField::Quarter,
                    "MONTH" => DateTimeField::Month,
                    "WEEK" => DateTimeField::Week,
                    "DAY" => DateTimeField::Day,
                    "DOW" | "DAYOFWEEK" => DateTimeField::DayOfWeek,
                    "DOY" | "DAYOFYEAR" => DateTimeField::DayOfYear,
                    "HOUR" => DateTimeField::Hour,
                    "MINUTE" => DateTimeField::Minute,
                    "SECOND" => DateTimeField::Second,
                    "MILLISECOND" => DateTimeField::Millisecond,
                    "MICROSECOND" => DateTimeField::Microsecond,
                    "NANOSECOND" => DateTimeField::Nanosecond,
                    "EPOCH" => DateTimeField::Epoch,
                    "TIMEZONE" => DateTimeField::Timezone,
                    "TIMEZONE_HOUR" => DateTimeField::TimezoneHour,
                    "TIMEZONE_MINUTE" => DateTimeField::TimezoneMinute,
                    _ => {
                        return Err(SqlglotError::ParserError {
                            message: format!("Unknown datetime field: {name}"),
                        });
                    }
                }
            }
        };
        self.advance();
        Ok(field)
    }

    fn try_parse_datetime_field(&mut self) -> Option<DateTimeField> {
        let saved = self.pos;
        match self.parse_datetime_field() {
            Ok(field) => Some(field),
            Err(_) => {
                self.pos = saved;
                None
            }
        }
    }

    /// Parse the inside of `GROUP_CONCAT(...)` (caller has already consumed
    /// the `(` and optional `DISTINCT`). Returns a typed `GroupConcat`
    /// expression. Does NOT consume the trailing `)`.
    fn parse_group_concat_call(&mut self, distinct: bool) -> Result<Expr> {
        let mut exprs: Vec<Expr> = Vec::new();
        let mut order_by: Vec<OrderByItem> = Vec::new();
        let mut separator: Option<Box<Expr>> = None;

        if self.peek_type() != &TokenType::RParen {
            exprs.push(self.parse_expr()?);
            while self.peek_type() == &TokenType::Comma {
                // ORDER BY / SEPARATOR are alternative terminators, not args.
                // Peek one past the comma to disambiguate `f(a, b)` from
                // `f(a, b ORDER BY ...)` — but comma here always introduces
                // another positional arg, so just keep consuming.
                self.advance();
                exprs.push(self.parse_expr()?);
            }

            if self.match_token(TokenType::Order) {
                self.expect(TokenType::By)?;
                order_by = self.parse_order_by_items()?;
            }

            if self.match_keyword("SEPARATOR") {
                separator = Some(Box::new(self.parse_expr()?));
            }
        }

        Ok(Expr::TypedFunction {
            func: TypedFunction::GroupConcat {
                exprs,
                separator,
                order_by,
                distinct,
            },
            filter: None,
            over: None,
        })
    }

    /// Try to construct a typed function expression from a parsed function call.
    /// Returns `None` if the function name is not recognized, falling back to
    /// the generic `Expr::Function`.
    fn try_typed_function(name: &str, args: Vec<Expr>, distinct: bool) -> Option<Expr> {
        let upper = name.to_uppercase();
        let tf = match upper.as_str() {
            // ── Date/Time ──────────────────────────────────────────
            "DATE_ADD" | "DATEADD" | "TIMESTAMPADD" => {
                let mut it = args.into_iter();
                let first = it.next()?;
                let second = it.next()?;
                let third = it.next();
                // Handle DATEADD(unit, interval, expr) — TSQL/Snowflake arg order
                if upper == "DATEADD" {
                    if let Some(third_arg) = third {
                        // 3-arg: DATEADD(unit, interval, expr)
                        let unit = Self::expr_to_datetime_field(&first);
                        TypedFunction::DateAdd {
                            expr: Box::new(third_arg),
                            interval: Box::new(second),
                            unit,
                        }
                    } else {
                        TypedFunction::DateAdd {
                            expr: Box::new(first),
                            interval: Box::new(second),
                            unit: None,
                        }
                    }
                } else {
                    // DATE_ADD(expr, interval [, unit])
                    let unit = third.as_ref().and_then(Self::expr_to_datetime_field);
                    TypedFunction::DateAdd {
                        expr: Box::new(first),
                        interval: Box::new(second),
                        unit,
                    }
                }
            }
            "DATE_DIFF" | "DATEDIFF" | "TIMESTAMPDIFF" => {
                let mut it = args.into_iter();
                let first = it.next()?;
                let second = it.next()?;
                let third = it.next();
                if let Some(third_arg) = third {
                    if upper == "DATEDIFF" {
                        // DATEDIFF(unit, start, end) — TSQL/Snowflake
                        let unit = Self::expr_to_datetime_field(&first);
                        TypedFunction::DateDiff {
                            start: Box::new(second),
                            end: Box::new(third_arg),
                            unit,
                        }
                    } else {
                        let unit = Self::expr_to_datetime_field(&third_arg);
                        TypedFunction::DateDiff {
                            start: Box::new(first),
                            end: Box::new(second),
                            unit,
                        }
                    }
                } else {
                    TypedFunction::DateDiff {
                        start: Box::new(first),
                        end: Box::new(second),
                        unit: None,
                    }
                }
            }
            "DATE_TRUNC" | "DATETRUNC" => {
                let mut it = args.into_iter();
                let first = it.next()?;
                let second = it.next()?;
                // DATE_TRUNC('unit', expr) or DATE_TRUNC(unit, expr)
                let (unit, expr) = if let Some(u) = Self::expr_to_datetime_field(&first) {
                    (u, second)
                } else if let Some(u) = Self::expr_to_datetime_field(&second) {
                    (u, first)
                } else {
                    // Default: first = unit string, second = expr
                    return None;
                };
                TypedFunction::DateTrunc {
                    unit,
                    expr: Box::new(expr),
                }
            }
            "DATE_SUB" | "DATESUB" => {
                let mut it = args.into_iter();
                let first = it.next()?;
                let second = it.next()?;
                let third = it.next();
                let unit = third.as_ref().and_then(Self::expr_to_datetime_field);
                TypedFunction::DateSub {
                    expr: Box::new(first),
                    interval: Box::new(second),
                    unit,
                }
            }
            "CURRENT_DATE" => TypedFunction::CurrentDate,
            "CURRENT_TIME" | "CURTIME" => TypedFunction::CurrentTime,
            "CURRENT_TIMESTAMP" | "NOW" | "GETDATE" | "SYSDATE" => TypedFunction::CurrentTimestamp,
            "STR_TO_TIME" | "STR_TO_DATE" | "TO_TIMESTAMP" | "PARSE_TIMESTAMP"
            | "PARSE_DATETIME" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let format = it.next()?;
                TypedFunction::StrToTime {
                    expr: Box::new(expr),
                    format: Box::new(format),
                }
            }
            "TIME_TO_STR" | "DATE_FORMAT" | "FORMAT_TIMESTAMP" | "FORMAT_DATETIME" | "TO_CHAR" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let format = it.next()?;
                TypedFunction::TimeToStr {
                    expr: Box::new(expr),
                    format: Box::new(format),
                }
            }
            "TS_OR_DS_TO_DATE" => {
                let mut it = args.into_iter();
                TypedFunction::TsOrDsToDate {
                    expr: Box::new(it.next()?),
                }
            }
            "YEAR" => {
                let mut it = args.into_iter();
                TypedFunction::Year {
                    expr: Box::new(it.next()?),
                }
            }
            "MONTH" => {
                let mut it = args.into_iter();
                TypedFunction::Month {
                    expr: Box::new(it.next()?),
                }
            }
            "DAY" | "DAYOFMONTH" => {
                let mut it = args.into_iter();
                TypedFunction::Day {
                    expr: Box::new(it.next()?),
                }
            }

            // ── String ─────────────────────────────────────────────
            "TRIM" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                TypedFunction::Trim {
                    expr: Box::new(expr),
                    trim_type: TrimType::Both,
                    trim_chars: None,
                }
            }
            "LTRIM" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                TypedFunction::Trim {
                    expr: Box::new(expr),
                    trim_type: TrimType::Leading,
                    trim_chars: None,
                }
            }
            "RTRIM" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                TypedFunction::Trim {
                    expr: Box::new(expr),
                    trim_type: TrimType::Trailing,
                    trim_chars: None,
                }
            }
            "SUBSTRING" | "SUBSTR" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let start = it.next()?;
                let length = it.next();
                TypedFunction::Substring {
                    expr: Box::new(expr),
                    start: Box::new(start),
                    length: length.map(Box::new),
                }
            }
            "UPPER" | "UCASE" => {
                let mut it = args.into_iter();
                TypedFunction::Upper {
                    expr: Box::new(it.next()?),
                }
            }
            "LOWER" | "LCASE" => {
                let mut it = args.into_iter();
                TypedFunction::Lower {
                    expr: Box::new(it.next()?),
                }
            }
            "REGEXP_LIKE" | "RLIKE" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let pattern = it.next()?;
                let flags = it.next();
                TypedFunction::RegexpLike {
                    expr: Box::new(expr),
                    pattern: Box::new(pattern),
                    flags: flags.map(Box::new),
                }
            }
            "REGEXP_EXTRACT" | "REGEXP_SUBSTR" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let pattern = it.next()?;
                let group_index = it.next();
                TypedFunction::RegexpExtract {
                    expr: Box::new(expr),
                    pattern: Box::new(pattern),
                    group_index: group_index.map(Box::new),
                }
            }
            "REGEXP_REPLACE" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let pattern = it.next()?;
                let replacement = it.next()?;
                let flags = it.next();
                TypedFunction::RegexpReplace {
                    expr: Box::new(expr),
                    pattern: Box::new(pattern),
                    replacement: Box::new(replacement),
                    flags: flags.map(Box::new),
                }
            }
            "CONCAT_WS" => {
                let mut it = args.into_iter();
                let separator = it.next()?;
                let exprs: Vec<Expr> = it.collect();
                TypedFunction::ConcatWs {
                    separator: Box::new(separator),
                    exprs,
                }
            }
            "SPLIT" | "STRING_SPLIT" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let delimiter = it.next()?;
                TypedFunction::Split {
                    expr: Box::new(expr),
                    delimiter: Box::new(delimiter),
                }
            }
            "INITCAP" => {
                let mut it = args.into_iter();
                TypedFunction::Initcap {
                    expr: Box::new(it.next()?),
                }
            }
            "LENGTH" | "LEN" | "CHAR_LENGTH" | "CHARACTER_LENGTH" => {
                let mut it = args.into_iter();
                TypedFunction::Length {
                    expr: Box::new(it.next()?),
                }
            }
            "REPLACE" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let from = it.next()?;
                let to = it.next()?;
                TypedFunction::Replace {
                    expr: Box::new(expr),
                    from: Box::new(from),
                    to: Box::new(to),
                }
            }
            "REVERSE" => {
                let mut it = args.into_iter();
                TypedFunction::Reverse {
                    expr: Box::new(it.next()?),
                }
            }
            "LEFT" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let n = it.next()?;
                TypedFunction::Left {
                    expr: Box::new(expr),
                    n: Box::new(n),
                }
            }
            "RIGHT" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let n = it.next()?;
                TypedFunction::Right {
                    expr: Box::new(expr),
                    n: Box::new(n),
                }
            }
            "LPAD" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let length = it.next()?;
                let pad = it.next();
                TypedFunction::Lpad {
                    expr: Box::new(expr),
                    length: Box::new(length),
                    pad: pad.map(Box::new),
                }
            }
            "RPAD" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let length = it.next()?;
                let pad = it.next();
                TypedFunction::Rpad {
                    expr: Box::new(expr),
                    length: Box::new(length),
                    pad: pad.map(Box::new),
                }
            }

            // ── Aggregate ──────────────────────────────────────────
            "COUNT" => {
                let mut it = args.into_iter();
                let expr = it.next().unwrap_or(Expr::Wildcard);
                TypedFunction::Count {
                    expr: Box::new(expr),
                    distinct,
                }
            }
            "SUM" => {
                let mut it = args.into_iter();
                TypedFunction::Sum {
                    expr: Box::new(it.next()?),
                    distinct,
                }
            }
            "AVG" => {
                let mut it = args.into_iter();
                TypedFunction::Avg {
                    expr: Box::new(it.next()?),
                    distinct,
                }
            }
            "MIN" => {
                let mut it = args.into_iter();
                TypedFunction::Min {
                    expr: Box::new(it.next()?),
                }
            }
            "MAX" => {
                let mut it = args.into_iter();
                TypedFunction::Max {
                    expr: Box::new(it.next()?),
                }
            }
            "ARRAY_AGG" | "LIST" | "COLLECT_LIST" => {
                let mut it = args.into_iter();
                TypedFunction::ArrayAgg {
                    expr: Box::new(it.next()?),
                    distinct,
                }
            }
            "APPROX_DISTINCT" | "APPROX_COUNT_DISTINCT" => {
                let mut it = args.into_iter();
                TypedFunction::ApproxDistinct {
                    expr: Box::new(it.next()?),
                }
            }
            "VARIANCE" | "VAR_SAMP" | "VAR" => {
                let mut it = args.into_iter();
                TypedFunction::Variance {
                    expr: Box::new(it.next()?),
                }
            }
            "STDDEV" | "STDDEV_SAMP" => {
                let mut it = args.into_iter();
                TypedFunction::Stddev {
                    expr: Box::new(it.next()?),
                }
            }

            // ── Array ──────────────────────────────────────────────
            "ARRAY_CONCAT" | "ARRAY_CAT" => TypedFunction::ArrayConcat { arrays: args },
            "ARRAY_CONTAINS" => {
                let mut it = args.into_iter();
                let array = it.next()?;
                let element = it.next()?;
                TypedFunction::ArrayContains {
                    array: Box::new(array),
                    element: Box::new(element),
                }
            }
            "ARRAY_SIZE" | "ARRAY_LENGTH" | "CARDINALITY" => {
                let mut it = args.into_iter();
                TypedFunction::ArraySize {
                    expr: Box::new(it.next()?),
                }
            }
            "EXPLODE" => {
                let mut it = args.into_iter();
                TypedFunction::Explode {
                    expr: Box::new(it.next()?),
                }
            }
            "GENERATE_SERIES" | "SEQUENCE" => {
                let mut it = args.into_iter();
                let start = it.next()?;
                let stop = it.next()?;
                let step = it.next();
                TypedFunction::GenerateSeries {
                    start: Box::new(start),
                    stop: Box::new(stop),
                    step: step.map(Box::new),
                }
            }
            "FLATTEN" => {
                let mut it = args.into_iter();
                TypedFunction::Flatten {
                    expr: Box::new(it.next()?),
                }
            }

            // ── JSON ───────────────────────────────────────────────
            "JSON_EXTRACT" | "JSON_VALUE" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let path = it.next()?;
                TypedFunction::JSONExtract {
                    expr: Box::new(expr),
                    path: Box::new(path),
                }
            }
            "JSON_EXTRACT_SCALAR" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let path = it.next()?;
                TypedFunction::JSONExtractScalar {
                    expr: Box::new(expr),
                    path: Box::new(path),
                }
            }
            "PARSE_JSON" | "JSON_PARSE" => {
                let mut it = args.into_iter();
                TypedFunction::ParseJSON {
                    expr: Box::new(it.next()?),
                }
            }
            "JSON_FORMAT" | "TO_JSON" | "TO_JSON_STRING" => {
                let mut it = args.into_iter();
                TypedFunction::JSONFormat {
                    expr: Box::new(it.next()?),
                }
            }

            // ── Window ─────────────────────────────────────────────
            "ROW_NUMBER" => TypedFunction::RowNumber,
            "RANK" => TypedFunction::Rank,
            "DENSE_RANK" => TypedFunction::DenseRank,
            "NTILE" => {
                let mut it = args.into_iter();
                TypedFunction::NTile {
                    n: Box::new(it.next()?),
                }
            }
            "LEAD" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let offset = it.next();
                let default = it.next();
                TypedFunction::Lead {
                    expr: Box::new(expr),
                    offset: offset.map(Box::new),
                    default: default.map(Box::new),
                }
            }
            "LAG" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let offset = it.next();
                let default = it.next();
                TypedFunction::Lag {
                    expr: Box::new(expr),
                    offset: offset.map(Box::new),
                    default: default.map(Box::new),
                }
            }
            "FIRST_VALUE" => {
                let mut it = args.into_iter();
                TypedFunction::FirstValue {
                    expr: Box::new(it.next()?),
                }
            }
            "LAST_VALUE" => {
                let mut it = args.into_iter();
                TypedFunction::LastValue {
                    expr: Box::new(it.next()?),
                }
            }

            // ── Math ───────────────────────────────────────────────
            "ABS" => {
                let mut it = args.into_iter();
                TypedFunction::Abs {
                    expr: Box::new(it.next()?),
                }
            }
            "CEIL" | "CEILING" => {
                let mut it = args.into_iter();
                TypedFunction::Ceil {
                    expr: Box::new(it.next()?),
                }
            }
            "FLOOR" => {
                let mut it = args.into_iter();
                TypedFunction::Floor {
                    expr: Box::new(it.next()?),
                }
            }
            "ROUND" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let decimals = it.next();
                TypedFunction::Round {
                    expr: Box::new(expr),
                    decimals: decimals.map(Box::new),
                }
            }
            "LOG" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let base = it.next();
                TypedFunction::Log {
                    expr: Box::new(expr),
                    base: base.map(Box::new),
                }
            }
            "LN" => {
                let mut it = args.into_iter();
                TypedFunction::Ln {
                    expr: Box::new(it.next()?),
                }
            }
            "POW" | "POWER" => {
                let mut it = args.into_iter();
                let base = it.next()?;
                let exponent = it.next()?;
                TypedFunction::Pow {
                    base: Box::new(base),
                    exponent: Box::new(exponent),
                }
            }
            "SQRT" => {
                let mut it = args.into_iter();
                TypedFunction::Sqrt {
                    expr: Box::new(it.next()?),
                }
            }
            "GREATEST" => TypedFunction::Greatest { exprs: args },
            "LEAST" => TypedFunction::Least { exprs: args },
            "MOD" => {
                let mut it = args.into_iter();
                let left = it.next()?;
                let right = it.next()?;
                TypedFunction::Mod {
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }

            // ── Conversion ─────────────────────────────────────────
            "HEX" | "TO_HEX" => {
                let mut it = args.into_iter();
                TypedFunction::Hex {
                    expr: Box::new(it.next()?),
                }
            }
            "UNHEX" | "FROM_HEX" => {
                let mut it = args.into_iter();
                TypedFunction::Unhex {
                    expr: Box::new(it.next()?),
                }
            }
            "MD5" => {
                let mut it = args.into_iter();
                TypedFunction::Md5 {
                    expr: Box::new(it.next()?),
                }
            }
            "SHA" | "SHA1" => {
                let mut it = args.into_iter();
                TypedFunction::Sha {
                    expr: Box::new(it.next()?),
                }
            }
            "SHA2" | "SHA256" | "SHA512" => {
                let mut it = args.into_iter();
                let expr = it.next()?;
                let bit_length = it.next().unwrap_or(Expr::Number("256".to_string()));
                TypedFunction::Sha2 {
                    expr: Box::new(expr),
                    bit_length: Box::new(bit_length),
                }
            }

            // Not a recognized typed function
            _ => return None,
        };

        Some(Expr::TypedFunction {
            func: tf,
            filter: None,
            over: None,
        })
    }

    /// Try to extract a DateTimeField from a column-name expression.
    fn expr_to_datetime_field(expr: &Expr) -> Option<DateTimeField> {
        match expr {
            Expr::Column {
                name, table: None, ..
            } => match name.to_uppercase().as_str() {
                "YEAR" => Some(DateTimeField::Year),
                "QUARTER" => Some(DateTimeField::Quarter),
                "MONTH" => Some(DateTimeField::Month),
                "WEEK" => Some(DateTimeField::Week),
                "DAY" => Some(DateTimeField::Day),
                "HOUR" => Some(DateTimeField::Hour),
                "MINUTE" => Some(DateTimeField::Minute),
                "SECOND" => Some(DateTimeField::Second),
                "MILLISECOND" => Some(DateTimeField::Millisecond),
                "MICROSECOND" => Some(DateTimeField::Microsecond),
                _ => None,
            },
            Expr::StringLiteral(s) | Expr::NationalStringLiteral(s) => {
                match s.to_uppercase().as_str() {
                    "YEAR" => Some(DateTimeField::Year),
                    "QUARTER" => Some(DateTimeField::Quarter),
                    "MONTH" => Some(DateTimeField::Month),
                    "WEEK" => Some(DateTimeField::Week),
                    "DAY" => Some(DateTimeField::Day),
                    "HOUR" => Some(DateTimeField::Hour),
                    "MINUTE" => Some(DateTimeField::Minute),
                    "SECOND" => Some(DateTimeField::Second),
                    "MILLISECOND" => Some(DateTimeField::Millisecond),
                    "MICROSECOND" => Some(DateTimeField::Microsecond),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn parse_case_expr(&mut self) -> Result<Expr> {
        self.expect(TokenType::Case)?;

        let operand = if self.peek_type() != &TokenType::When {
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };

        let mut when_clauses = Vec::new();
        while self.match_token(TokenType::When) {
            let condition = self.parse_expr()?;
            self.expect(TokenType::Then)?;
            let result = self.parse_expr()?;
            when_clauses.push((condition, result));
        }

        let else_clause = if self.match_token(TokenType::Else) {
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };

        self.expect(TokenType::End)?;

        Ok(Expr::Case {
            operand,
            when_clauses,
            else_clause,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_select() {
        let stmt = Parser::new("SELECT a, b FROM t")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Select(sel) => {
                assert_eq!(sel.columns.len(), 2);
                assert!(sel.from.is_some());
            }
            _ => panic!("Expected SELECT"),
        }
    }

    #[test]
    fn test_parse_select_with_where() {
        let stmt = Parser::new("SELECT x FROM t WHERE x > 10")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Select(sel) => assert!(sel.where_clause.is_some()),
            _ => panic!("Expected SELECT"),
        }
    }

    #[test]
    fn test_parse_select_wildcard() {
        let stmt = Parser::new("SELECT * FROM users")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Select(sel) => {
                assert_eq!(sel.columns.len(), 1);
                assert!(matches!(sel.columns[0], SelectItem::Wildcard));
            }
            _ => panic!("Expected SELECT"),
        }
    }

    #[test]
    fn test_parse_insert() {
        let stmt = Parser::new("INSERT INTO t (a, b) VALUES (1, 'hello')")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Insert(ins) => {
                assert_eq!(ins.table.name, "t");
                assert_eq!(ins.columns, vec!["a", "b"]);
                match &ins.source {
                    InsertSource::Values(rows) => {
                        assert_eq!(rows.len(), 1);
                        assert_eq!(rows[0].len(), 2);
                    }
                    _ => panic!("Expected VALUES"),
                }
            }
            _ => panic!("Expected INSERT"),
        }
    }

    #[test]
    fn test_parse_delete() {
        let stmt = Parser::new("DELETE FROM users WHERE id = 1")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Delete(del) => {
                assert_eq!(del.table.name, "users");
                assert!(del.where_clause.is_some());
            }
            _ => panic!("Expected DELETE"),
        }
    }

    #[test]
    fn test_parse_join() {
        let stmt = Parser::new("SELECT a.id, b.name FROM a INNER JOIN b ON a.id = b.a_id")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Select(sel) => {
                assert_eq!(sel.joins.len(), 1);
                assert_eq!(sel.joins[0].join_type, JoinType::Inner);
            }
            _ => panic!("Expected SELECT"),
        }
    }

    #[test]
    fn test_parse_cte() {
        let stmt = Parser::new("WITH cte AS (SELECT 1 AS x) SELECT x FROM cte")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Select(sel) => {
                assert_eq!(sel.ctes.len(), 1);
                assert_eq!(sel.ctes[0].name, "cte");
            }
            _ => panic!("Expected SELECT"),
        }
    }

    #[test]
    fn test_parse_union() {
        let stmt = Parser::new("SELECT 1 UNION ALL SELECT 2")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::SetOperation(sop) => {
                assert_eq!(sop.op, SetOperationType::Union);
                assert!(sop.all);
            }
            _ => panic!("Expected SetOperation"),
        }
    }

    #[test]
    fn test_parse_cast() {
        let stmt = Parser::new("SELECT CAST(x AS INT) FROM t")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Select(sel) => {
                if let SelectItem::Expr { expr, .. } = &sel.columns[0] {
                    assert!(matches!(expr, Expr::Cast { .. }));
                }
            }
            _ => panic!("Expected SELECT"),
        }
    }

    #[test]
    fn test_parse_subquery() {
        let stmt = Parser::new("SELECT * FROM (SELECT 1 AS x) AS sub")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Select(sel) => {
                if let Some(from) = &sel.from {
                    assert!(matches!(from.source, TableSource::Subquery { .. }));
                }
            }
            _ => panic!("Expected SELECT"),
        }
    }

    #[test]
    fn test_parse_exists() {
        let stmt = Parser::new("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM t2)")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Select(sel) => {
                assert!(sel.where_clause.is_some());
            }
            _ => panic!("Expected SELECT"),
        }
    }

    #[test]
    fn test_parse_window_function() {
        let stmt = Parser::new(
            "SELECT ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) FROM emp",
        )
        .unwrap()
        .parse_statement()
        .unwrap();
        match stmt {
            Statement::Select(sel) => {
                if let SelectItem::Expr { expr, .. } = &sel.columns[0] {
                    match expr {
                        Expr::TypedFunction { over, .. } => {
                            assert!(over.is_some());
                        }
                        Expr::Function { over, .. } => {
                            assert!(over.is_some());
                        }
                        _ => panic!("Expected function"),
                    }
                }
            }
            _ => panic!("Expected SELECT"),
        }
    }

    #[test]
    fn test_parse_multiple_statements() {
        let stmts = Parser::new("SELECT 1; SELECT 2;")
            .unwrap()
            .parse_statements()
            .unwrap();
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn test_parse_insert_select() {
        let stmt = Parser::new("INSERT INTO t SELECT * FROM s")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Insert(ins) => {
                assert!(matches!(ins.source, InsertSource::Query(_)));
            }
            _ => panic!("Expected INSERT"),
        }
    }

    #[test]
    fn test_parse_create_table_constraints() {
        let stmt =
            Parser::new("CREATE TABLE t (id INT PRIMARY KEY, name VARCHAR(100) NOT NULL UNIQUE)")
                .unwrap()
                .parse_statement()
                .unwrap();
        match stmt {
            Statement::CreateTable(ct) => {
                assert_eq!(ct.columns.len(), 2);
                assert!(ct.columns[0].primary_key);
                assert!(ct.columns[1].unique);
            }
            _ => panic!("Expected CREATE TABLE"),
        }
    }

    #[test]
    fn test_parse_extract() {
        let stmt = Parser::new("SELECT EXTRACT(YEAR FROM created_at) FROM t")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Select(sel) => {
                if let SelectItem::Expr { expr, .. } = &sel.columns[0] {
                    assert!(matches!(expr, Expr::Extract { .. }));
                }
            }
            _ => panic!("Expected SELECT"),
        }
    }

    #[test]
    fn test_parse_postgres_cast() {
        let stmt = Parser::new("SELECT x::int FROM t")
            .unwrap()
            .parse_statement()
            .unwrap();
        match stmt {
            Statement::Select(sel) => {
                if let SelectItem::Expr { expr, .. } = &sel.columns[0] {
                    assert!(matches!(expr, Expr::Cast { .. }));
                }
            }
            _ => panic!("Expected SELECT"),
        }
    }
}

/// Attach comments to the appropriate field on a parsed statement.
fn attach_comments_to_statement(stmt: &mut Statement, comments: Vec<String>) {
    match stmt {
        Statement::Select(s) => s.comments = comments,
        Statement::Insert(s) => s.comments = comments,
        Statement::Update(s) => s.comments = comments,
        Statement::Delete(s) => s.comments = comments,
        Statement::CreateTable(s) => s.comments = comments,
        Statement::DropTable(s) => s.comments = comments,
        Statement::SetOperation(s) => s.comments = comments,
        Statement::AlterTable(s) => s.comments = comments,
        Statement::CreateView(s) => s.comments = comments,
        Statement::DropView(s) => s.comments = comments,
        Statement::Truncate(s) => s.comments = comments,
        Statement::Explain(s) => s.comments = comments,
        Statement::Use(s) => s.comments = comments,
        Statement::Merge(s) => s.comments = comments,
        // Transaction and Expression don't have comment fields
        Statement::Transaction(_) | Statement::Expression(_) => {}
    }
}
