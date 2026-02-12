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
}

impl Parser {
    /// Create a new parser from a SQL string.
    pub fn new(sql: &str) -> Result<Self> {
        let mut tokenizer = Tokenizer::new(sql);
        let tokens = tokenizer.tokenize()?;
        Ok(Self { tokens, pos: 0 })
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

    /// Match a keyword by string value (for multi-word context-sensitive keywords).
    fn match_keyword(&mut self, keyword: &str) -> bool {
        if self.check_keyword(keyword) {
            self.advance();
            true
        } else {
            false
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
        let stmt = self.parse_statement_inner()?;
        // Consume trailing semicolons
        while self.match_token(TokenType::Semicolon) {}
        Ok(stmt)
    }

    fn parse_statement_inner(&mut self) -> Result<Statement> {
        match self.peek_type() {
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
        }
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
        let name = self.expect_name()?;

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

        // TOP N (SQL Server style)
        let top = if self.match_token(TokenType::Top) {
            Some(Box::new(self.parse_expr()?))
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
            self.parse_expr_list()?
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
            Some(self.parse_expr()?)
        } else {
            None
        };

        // FETCH FIRST|NEXT n ROWS ONLY (Oracle / ANSI SQL:2008)
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
            op,
            all,
            left: Box::new(left),
            right: Box::new(right),
            order_by: vec![],
            limit: None,
            offset: None,
        });

        // Parse trailing ORDER BY / LIMIT / OFFSET that applies to the whole set operation
        if matches!(self.peek_type(), TokenType::Union | TokenType::Intersect | TokenType::Except) {
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

        let alias = self.parse_optional_alias()?;

        Ok(SelectItem::Expr { expr, alias })
    }

    fn parse_optional_alias(&mut self) -> Result<Option<String>> {
        if self.match_token(TokenType::As) {
            return Ok(Some(self.expect_name()?));
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
            ) {
                return Ok(Some(self.advance().value.clone()));
            }
        }
        Ok(None)
    }

    fn parse_table_source(&mut self) -> Result<TableSource> {
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
            let alias = self.parse_optional_alias()?;
            let with_offset = self.match_keyword("WITH") && self.match_keyword("OFFSET");
            return Ok(TableSource::Unnest {
                expr: Box::new(expr),
                alias,
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
                let alias = self.parse_optional_alias()?;
                return Ok(TableSource::Subquery {
                    query: Box::new(query),
                    alias,
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
            let alias = self.parse_optional_alias()?;
            return Ok(TableSource::TableFunction {
                name: table_ref.name,
                args,
                alias,
            });
        }

        Ok(TableSource::Table(table_ref))
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

        let alias = self.parse_optional_alias()?;

        Ok(TableRef {
            catalog,
            schema,
            name,
            alias,
            name_quote_style: name_qs,
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
        })
    }

    fn parse_joins(&mut self) -> Result<Vec<JoinClause>> {
        let mut joins = Vec::new();
        loop {
            let join_type = match self.peek_type() {
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
        } else if matches!(self.peek_type(), TokenType::Select | TokenType::With | TokenType::LParen) {
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
            let col = self.expect_name()?;
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
            table,
            using,
            where_clause,
            returning,
        })
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
            return self.parse_create_view(or_replace, materialized).map(Statement::CreateView);
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
            if matches!(self.peek_type(), TokenType::Primary | TokenType::Unique | TokenType::Foreign | TokenType::Check | TokenType::Constraint) {
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
            if_not_exists,
            temporary,
            table,
            columns,
            constraints,
            as_select: None,
        }))
    }

    fn parse_create_view(&mut self, or_replace: bool, materialized: bool) -> Result<CreateViewStatement> {
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

            let on_delete = if self.match_token(TokenType::On) && self.match_token(TokenType::Delete) {
                Some(self.parse_referential_action()?)
            } else {
                None
            };
            let on_update = if self.match_token(TokenType::On) && self.match_token(TokenType::Update) {
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
            TokenType::Int | TokenType::Integer => { self.advance(); Ok(DataType::Int) }
            TokenType::BigInt => { self.advance(); Ok(DataType::BigInt) }
            TokenType::SmallInt => { self.advance(); Ok(DataType::SmallInt) }
            TokenType::TinyInt => { self.advance(); Ok(DataType::TinyInt) }
            TokenType::Float => { self.advance(); Ok(DataType::Float) }
            TokenType::Double => {
                self.advance();
                let _ = self.match_keyword("PRECISION");
                Ok(DataType::Double)
            }
            TokenType::Real => { self.advance(); Ok(DataType::Real) }
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
            TokenType::Text => { self.advance(); Ok(DataType::Text) }
            TokenType::Boolean => { self.advance(); Ok(DataType::Boolean) }
            TokenType::Date => { self.advance(); Ok(DataType::Date) }
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
                Ok(DataType::Timestamp { precision, with_tz: true })
            }
            TokenType::Time => {
                self.advance();
                let precision = self.parse_single_type_param()?;
                Ok(DataType::Time { precision })
            }
            TokenType::Interval => { self.advance(); Ok(DataType::Interval) }
            TokenType::Blob => { self.advance(); Ok(DataType::Blob) }
            TokenType::Bytea => { self.advance(); Ok(DataType::Bytea) }
            TokenType::Json => { self.advance(); Ok(DataType::Json) }
            TokenType::Jsonb => { self.advance(); Ok(DataType::Jsonb) }
            TokenType::Uuid => { self.advance(); Ok(DataType::Uuid) }
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
                    "BINARY" => { let len = self.parse_single_type_param()?; Ok(DataType::Binary(len)) }
                    "VARBINARY" => { let len = self.parse_single_type_param()?; Ok(DataType::Varbinary(len)) }
                    "DATETIME" => Ok(DataType::DateTime),
                    "BYTES" => Ok(DataType::Bytes),
                    "VARIANT" => Ok(DataType::Variant),
                    "OBJECT" => Ok(DataType::Object),
                    "XML" => Ok(DataType::Xml),
                    "INET" => Ok(DataType::Inet),
                    "CIDR" => Ok(DataType::Cidr),
                    "MACADDR" => Ok(DataType::Macaddr),
                    "BIT" => { let len = self.parse_single_type_param()?; Ok(DataType::Bit(len)) }
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
        type_result
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

        Ok(AlterTableStatement { table, actions })
    }

    fn parse_alter_action(&mut self) -> Result<AlterTableAction> {
        if self.match_keyword("ADD") {
            if matches!(self.peek_type(), TokenType::Constraint | TokenType::Primary | TokenType::Unique | TokenType::Foreign | TokenType::Check) {
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
        Ok(TruncateStatement { table })
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
            analyze,
            statement: Box::new(statement),
        })
    }

    // ── USE ─────────────────────────────────────────────────────────

    fn parse_use(&mut self) -> Result<UseStatement> {
        self.expect(TokenType::Use)?;
        let name = self.expect_name()?;
        Ok(UseStatement { name })
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
                let right = self.parse_addition()?;
                left = Expr::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            } else if self.peek_type() == &TokenType::Is {
                self.advance();
                let negated = self.match_token(TokenType::Not);
                self.expect(TokenType::Null)?;
                left = Expr::IsNull {
                    expr: Box::new(left),
                    negated,
                };
            } else if matches!(
                self.peek_type(),
                TokenType::Not | TokenType::In | TokenType::Like | TokenType::ILike | TokenType::Between
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
                let negated = self.pos > 0
                    && self.tokens[self.pos - 1].token_type == TokenType::Not;

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
            if let Expr::Function { name, args, distinct, filter, .. } = expr {
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
                expr = Expr::Function {
                    name,
                    args,
                    distinct,
                    filter,
                    over: Some(spec),
                };
            }
        }

        // FILTER (WHERE ...) for aggregate functions
        if self.match_token(TokenType::Filter) {
            if let Expr::Function { name, args, distinct, over, .. } = expr {
                self.expect(TokenType::LParen)?;
                self.expect(TokenType::Where)?;
                let filter_expr = self.parse_expr()?;
                self.expect(TokenType::RParen)?;
                expr = Expr::Function {
                    name,
                    args,
                    distinct,
                    filter: Some(Box::new(filter_expr)),
                    over,
                };
            }
        }

        Ok(expr)
    }

    fn parse_window_spec(&mut self) -> Result<WindowSpec> {
        let window_ref = if self.is_name_token() && !matches!(self.peek_type(), TokenType::Partition | TokenType::Order | TokenType::Rows | TokenType::Range) {
            let saved = self.pos;
            let name = self.expect_name()?;
            // Check if it's actually a keyword we need
            if matches!(self.peek_type(), TokenType::RParen | TokenType::Partition | TokenType::Order | TokenType::Rows | TokenType::Range) {
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
            TokenType::Not if {
                let next_pos = self.pos + 1;
                next_pos < self.tokens.len() && self.tokens[next_pos].token_type == TokenType::Exists
            } => {
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

                // Function call: name(...)
                if self.peek_type() == &TokenType::LParen {
                    self.advance();

                    // Special: COUNT(*), COUNT(DISTINCT x)
                    let distinct = self.match_token(TokenType::Distinct);

                    let args = if self.peek_type() == &TokenType::RParen {
                        vec![]
                    } else if self.peek_type() == &TokenType::Star {
                        self.advance();
                        vec![Expr::Wildcard]
                    } else {
                        self.parse_expr_list()?
                    };
                    self.expect(TokenType::RParen)?;

                    Ok(Expr::Function {
                        name,
                        args,
                        distinct,
                        filter: None,
                        over: None,
                    })
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
        let stmt =
            Parser::new("SELECT a.id, b.name FROM a INNER JOIN b ON a.id = b.a_id")
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
        let stmt = Parser::new(
            "WITH cte AS (SELECT 1 AS x) SELECT x FROM cte",
        )
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
        let stmt = Parser::new(
            "SELECT * FROM (SELECT 1 AS x) AS sub",
        )
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
        let stmt = Parser::new(
            "SELECT * FROM t WHERE EXISTS (SELECT 1 FROM t2)",
        )
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
                    if let Expr::Function { over, .. } = expr {
                        assert!(over.is_some());
                    } else {
                        panic!("Expected function");
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
        let stmt = Parser::new(
            "CREATE TABLE t (id INT PRIMARY KEY, name VARCHAR(100) NOT NULL UNIQUE)",
        )
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
