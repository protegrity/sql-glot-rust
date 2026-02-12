use crate::errors::{Result, SqlglotError};
use crate::tokens::{Token, TokenType};

/// SQL tokenizer that converts a SQL string into a stream of tokens.
///
/// Tracks line and column numbers for error reporting. Supports:
/// - Single-line comments (`--`)
/// - Block comments (`/* ... */`)
/// - Quoted identifiers (`"..."` and backtick)
/// - String literals with escape handling
/// - Multi-character operators (`<=`, `>=`, `<>`, `!=`, `||`, `::`, `->`, `->>`)
pub struct Tokenizer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    /// Whether to preserve comments as tokens.
    pub preserve_comments: bool,
}

impl Tokenizer {
    /// Create a new tokenizer for the given SQL input.
    #[must_use]
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            preserve_comments: false,
        }
    }

    /// Create a tokenizer that preserves comment tokens.
    #[must_use]
    pub fn with_comments(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            preserve_comments: true,
        }
    }

    /// Tokenize the entire input and return a vector of tokens.
    ///
    /// Whitespace tokens are skipped. Comments are optionally preserved.
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            match token.token_type {
                TokenType::Eof => {
                    tokens.push(token);
                    break;
                }
                TokenType::Whitespace => continue,
                TokenType::LineComment | TokenType::BlockComment => {
                    if self.preserve_comments {
                        tokens.push(token);
                    }
                }
                _ => tokens.push(token),
            }
        }
        Ok(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.input.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn make_token(&self, token_type: TokenType, value: impl Into<String>, start: usize, start_line: usize, start_col: usize) -> Token {
        Token::with_location(token_type, value, start, start_line, start_col)
    }

    fn next_token(&mut self) -> Result<Token> {
        // Skip whitespace
        while self.peek().is_some_and(|c| c.is_whitespace()) {
            self.advance();
        }

        let start = self.pos;
        let start_line = self.line;
        let start_col = self.col;

        let Some(ch) = self.advance() else {
            return Ok(self.make_token(TokenType::Eof, "", start, start_line, start_col));
        };

        match ch {
            // ── Punctuation ─────────────────────────────────────────
            '(' => Ok(self.make_token(TokenType::LParen, "(", start, start_line, start_col)),
            ')' => Ok(self.make_token(TokenType::RParen, ")", start, start_line, start_col)),
            '[' => {
                // Check if this is a bracket-quoted identifier (T-SQL style: [identifier])
                // Only treat as quoted identifier if the content between [ and ] looks like
                // an identifier (starts with a letter or underscore, no commas inside).
                let mut looks_like_ident = false;
                if let Some(first_inner) = self.peek()
                    && (first_inner.is_ascii_alphabetic() || first_inner == '_')
                {
                    let mut scan = self.pos;
                    while scan < self.input.len() {
                        if self.input[scan] == ']' {
                            looks_like_ident = scan > self.pos;
                            break;
                        }
                        if self.input[scan] == ',' || self.input[scan] == '\n' {
                            break;
                        }
                        scan += 1;
                    }
                }
                if looks_like_ident {
                    self.read_quoted_identifier(start, start_line, start_col, '[')
                } else {
                    Ok(self.make_token(TokenType::LBracket, "[", start, start_line, start_col))
                }
            }
            ']' => Ok(self.make_token(TokenType::RBracket, "]", start, start_line, start_col)),
            '{' => Ok(self.make_token(TokenType::LBrace, "{", start, start_line, start_col)),
            '}' => Ok(self.make_token(TokenType::RBrace, "}", start, start_line, start_col)),
            ',' => Ok(self.make_token(TokenType::Comma, ",", start, start_line, start_col)),
            ';' => Ok(self.make_token(TokenType::Semicolon, ";", start, start_line, start_col)),
            '.' => Ok(self.make_token(TokenType::Dot, ".", start, start_line, start_col)),
            '+' => Ok(self.make_token(TokenType::Plus, "+", start, start_line, start_col)),
            '~' => Ok(self.make_token(TokenType::BitwiseNot, "~", start, start_line, start_col)),
            '@' => Ok(self.make_token(TokenType::AtSign, "@", start, start_line, start_col)),
            '=' => Ok(self.make_token(TokenType::Eq, "=", start, start_line, start_col)),
            '*' => Ok(self.make_token(TokenType::Star, "*", start, start_line, start_col)),
            '%' => Ok(self.make_token(TokenType::Percent2, "%", start, start_line, start_col)),
            '^' => Ok(self.make_token(TokenType::BitwiseXor, "^", start, start_line, start_col)),

            // ── Colon ───────────────────────────────────────────────
            ':' => {
                if self.peek() == Some(':') {
                    self.advance();
                    Ok(self.make_token(TokenType::DoubleColon, "::", start, start_line, start_col))
                } else {
                    Ok(self.make_token(TokenType::Colon, ":", start, start_line, start_col))
                }
            }

            // ── Minus / line comment / arrow ────────────────────────
            '-' => {
                if self.peek() == Some('-') {
                    self.advance();
                    let mut value = String::from("--");
                    while self.peek().is_some_and(|c| c != '\n') {
                        value.push(self.advance().unwrap());
                    }
                    Ok(self.make_token(TokenType::LineComment, value, start, start_line, start_col))
                } else if self.peek() == Some('>') {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        Ok(self.make_token(TokenType::DoubleArrow, "->>", start, start_line, start_col))
                    } else {
                        Ok(self.make_token(TokenType::Arrow, "->", start, start_line, start_col))
                    }
                } else {
                    Ok(self.make_token(TokenType::Minus, "-", start, start_line, start_col))
                }
            }

            // ── Slash / block comment ───────────────────────────────
            '/' => {
                if self.peek() == Some('*') {
                    self.advance();
                    let mut value = String::from("/*");
                    let mut depth = 1;
                    while depth > 0 {
                        match self.advance() {
                            Some('*') if self.peek() == Some('/') => {
                                self.advance();
                                depth -= 1;
                                value.push_str("*/");
                            }
                            Some('/') if self.peek() == Some('*') => {
                                self.advance();
                                depth += 1;
                                value.push_str("/*");
                            }
                            Some(c) => value.push(c),
                            None => {
                                return Err(SqlglotError::TokenizerError {
                                    message: "Unterminated block comment".into(),
                                    position: start,
                                });
                            }
                        }
                    }
                    Ok(self.make_token(TokenType::BlockComment, value, start, start_line, start_col))
                } else {
                    Ok(self.make_token(TokenType::Slash, "/", start, start_line, start_col))
                }
            }

            // ── Less-than variants ──────────────────────────────────
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(self.make_token(TokenType::LtEq, "<=", start, start_line, start_col))
                } else if self.peek() == Some('>') {
                    self.advance();
                    Ok(self.make_token(TokenType::Neq, "<>", start, start_line, start_col))
                } else if self.peek() == Some('<') {
                    self.advance();
                    Ok(self.make_token(TokenType::ShiftLeft, "<<", start, start_line, start_col))
                } else {
                    Ok(self.make_token(TokenType::Lt, "<", start, start_line, start_col))
                }
            }

            // ── Greater-than variants ───────────────────────────────
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(self.make_token(TokenType::GtEq, ">=", start, start_line, start_col))
                } else if self.peek() == Some('>') {
                    self.advance();
                    Ok(self.make_token(TokenType::ShiftRight, ">>", start, start_line, start_col))
                } else {
                    Ok(self.make_token(TokenType::Gt, ">", start, start_line, start_col))
                }
            }

            // ── Bang ────────────────────────────────────────────────
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(self.make_token(TokenType::Neq, "!=", start, start_line, start_col))
                } else {
                    Err(SqlglotError::TokenizerError {
                        message: format!("Unexpected character: {ch}"),
                        position: start,
                    })
                }
            }

            // ── Pipe / BitwiseOr / Concat ───────────────────────────
            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    Ok(self.make_token(TokenType::Concat, "||", start, start_line, start_col))
                } else {
                    Ok(self.make_token(TokenType::BitwiseOr, "|", start, start_line, start_col))
                }
            }

            // ── Ampersand ───────────────────────────────────────────
            '&' => Ok(self.make_token(TokenType::BitwiseAnd, "&", start, start_line, start_col)),

            // ── Hash ────────────────────────────────────────────────
            '#' => {
                if self.peek() == Some('>') {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        Ok(self.make_token(TokenType::HashDoubleArrow, "#>>", start, start_line, start_col))
                    } else {
                        Ok(self.make_token(TokenType::HashArrow, "#>", start, start_line, start_col))
                    }
                } else {
                    let mut value = String::from("#");
                    while self.peek().is_some_and(|c| c != '\n') {
                        value.push(self.advance().unwrap());
                    }
                    Ok(self.make_token(TokenType::LineComment, value, start, start_line, start_col))
                }
            }

            // ── String literals ─────────────────────────────────────
            '\'' => self.read_string(start, start_line, start_col),

            // ── Numbers ─────────────────────────────────────────────
            c if c.is_ascii_digit() => self.read_number(start, start_line, start_col, c),

            // ── Identifiers and keywords ────────────────────────────
            c if c.is_ascii_alphabetic() || c == '_' => self.read_identifier(start, start_line, start_col, c),

            // ── Quoted identifiers (double quote) ───────────────────
            '"' => self.read_quoted_identifier(start, start_line, start_col, '"'),

            // ── Backtick identifiers (MySQL, BigQuery) ──────────────
            '`' => self.read_quoted_identifier(start, start_line, start_col, '`'),

            // ── Parameter markers ───────────────────────────────────
            '$' => {
                if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    let mut value = String::from("$");
                    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        value.push(self.advance().unwrap());
                    }
                    Ok(self.make_token(TokenType::Parameter, value, start, start_line, start_col))
                } else {
                    Ok(self.make_token(TokenType::Parameter, "$", start, start_line, start_col))
                }
            }

            '?' => Ok(self.make_token(TokenType::Parameter, "?", start, start_line, start_col)),

            _ => Err(SqlglotError::TokenizerError {
                message: format!("Unexpected character: {ch}"),
                position: start,
            }),
        }
    }

    fn read_string(&mut self, start: usize, start_line: usize, start_col: usize) -> Result<Token> {
        let mut value = String::new();
        loop {
            match self.advance() {
                Some('\'') => {
                    if self.peek() == Some('\'') {
                        self.advance();
                        value.push('\'');
                    } else {
                        return Ok(self.make_token(TokenType::String, value, start, start_line, start_col));
                    }
                }
                Some('\\') => {
                    match self.peek() {
                        Some('\\') => {
                            self.advance();
                            value.push('\\');
                        }
                        Some('n') => {
                            self.advance();
                            value.push('\n');
                        }
                        Some('t') => {
                            self.advance();
                            value.push('\t');
                        }
                        Some('r') => {
                            self.advance();
                            value.push('\r');
                        }
                        _ => {
                            value.push('\\');
                        }
                    }
                }
                Some(c) => value.push(c),
                None => {
                    return Err(SqlglotError::TokenizerError {
                        message: "Unterminated string literal".into(),
                        position: start,
                    });
                }
            }
        }
    }

    fn read_number(&mut self, start: usize, start_line: usize, start_col: usize, first: char) -> Result<Token> {
        let mut value = String::new();
        value.push(first);

        if first == '0' && self.peek().is_some_and(|c| c == 'x' || c == 'X') {
            value.push(self.advance().unwrap());
            while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                value.push(self.advance().unwrap());
            }
            return Ok(self.make_token(TokenType::HexString, value, start, start_line, start_col));
        }

        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            value.push(self.advance().unwrap());
        }

        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            value.push(self.advance().unwrap());
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                value.push(self.advance().unwrap());
            }
        }

        if self.peek().is_some_and(|c| c == 'e' || c == 'E') {
            value.push(self.advance().unwrap());
            if self.peek().is_some_and(|c| c == '+' || c == '-') {
                value.push(self.advance().unwrap());
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                value.push(self.advance().unwrap());
            }
        }

        Ok(self.make_token(TokenType::Number, value, start, start_line, start_col))
    }

    fn read_identifier(&mut self, start: usize, start_line: usize, start_col: usize, first: char) -> Result<Token> {
        let mut value = String::new();
        value.push(first);
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            value.push(self.advance().unwrap());
        }

        let token_type = Self::keyword_type(&value);
        Ok(self.make_token(token_type, value, start, start_line, start_col))
    }

    /// Map a word to its keyword token type, or `Identifier` if not a keyword.
    fn keyword_type(word: &str) -> TokenType {
        match word.to_uppercase().as_str() {
            "SELECT" => TokenType::Select,
            "FROM" => TokenType::From,
            "WHERE" => TokenType::Where,
            "AND" => TokenType::And,
            "OR" => TokenType::Or,
            "NOT" => TokenType::Not,
            "AS" => TokenType::As,
            "JOIN" => TokenType::Join,
            "INNER" => TokenType::Inner,
            "LEFT" => TokenType::Left,
            "RIGHT" => TokenType::Right,
            "FULL" => TokenType::Full,
            "OUTER" => TokenType::Outer,
            "CROSS" => TokenType::Cross,
            "ON" => TokenType::On,
            "INSERT" => TokenType::Insert,
            "INTO" => TokenType::Into,
            "VALUES" => TokenType::Values,
            "UPDATE" => TokenType::Update,
            "SET" => TokenType::Set,
            "DELETE" => TokenType::Delete,
            "CREATE" => TokenType::Create,
            "TABLE" => TokenType::Table,
            "DROP" => TokenType::Drop,
            "ALTER" => TokenType::Alter,
            "INDEX" => TokenType::Index,
            "IF" => TokenType::If,
            "EXISTS" => TokenType::Exists,
            "IN" => TokenType::In,
            "IS" => TokenType::Is,
            "NULL" => TokenType::Null,
            "LIKE" => TokenType::Like,
            "ILIKE" => TokenType::ILike,
            "ESCAPE" => TokenType::Escape,
            "BETWEEN" => TokenType::Between,
            "CASE" => TokenType::Case,
            "WHEN" => TokenType::When,
            "THEN" => TokenType::Then,
            "ELSE" => TokenType::Else,
            "END" => TokenType::End,
            "ORDER" => TokenType::Order,
            "BY" => TokenType::By,
            "ASC" => TokenType::Asc,
            "DESC" => TokenType::Desc,
            "GROUP" => TokenType::Group,
            "HAVING" => TokenType::Having,
            "LIMIT" => TokenType::Limit,
            "OFFSET" => TokenType::Offset,
            "UNION" => TokenType::Union,
            "ALL" => TokenType::All,
            "DISTINCT" => TokenType::Distinct,
            "TRUE" => TokenType::True,
            "FALSE" => TokenType::False,
            "INTERSECT" => TokenType::Intersect,
            "EXCEPT" => TokenType::Except,
            "WITH" => TokenType::With,
            "RECURSIVE" => TokenType::Recursive,
            "ANY" => TokenType::Any,
            "SOME" => TokenType::Some,
            "CAST" => TokenType::Cast,
            "OVER" => TokenType::Over,
            "PARTITION" => TokenType::Partition,
            "WINDOW" => TokenType::Window,
            "ROWS" => TokenType::Rows,
            "RANGE" => TokenType::Range,
            "UNBOUNDED" => TokenType::Unbounded,
            "PRECEDING" => TokenType::Preceding,
            "FOLLOWING" => TokenType::Following,
            "FILTER" => TokenType::Filter,
            "INT" => TokenType::Int,
            "INTEGER" => TokenType::Integer,
            "BIGINT" => TokenType::BigInt,
            "SMALLINT" => TokenType::SmallInt,
            "TINYINT" => TokenType::TinyInt,
            "FLOAT" => TokenType::Float,
            "DOUBLE" => TokenType::Double,
            "DECIMAL" => TokenType::Decimal,
            "NUMERIC" => TokenType::Numeric,
            "REAL" => TokenType::Real,
            "VARCHAR" => TokenType::Varchar,
            "CHAR" | "CHARACTER" => TokenType::Char,
            "TEXT" => TokenType::Text,
            "BOOLEAN" | "BOOL" => TokenType::Boolean,
            "DATE" => TokenType::Date,
            "TIMESTAMP" => TokenType::Timestamp,
            "TIMESTAMPTZ" => TokenType::TimestampTz,
            "TIME" => TokenType::Time,
            "INTERVAL" => TokenType::Interval,
            "BLOB" => TokenType::Blob,
            "BYTEA" => TokenType::Bytea,
            "JSON" => TokenType::Json,
            "JSONB" => TokenType::Jsonb,
            "UUID" => TokenType::Uuid,
            "ARRAY" => TokenType::Array,
            "MAP" => TokenType::Map,
            "STRUCT" => TokenType::Struct,
            "PRIMARY" => TokenType::Primary,
            "KEY" => TokenType::Key,
            "FOREIGN" => TokenType::Foreign,
            "REFERENCES" => TokenType::References,
            "UNIQUE" => TokenType::Unique,
            "CHECK" => TokenType::Check,
            "DEFAULT" => TokenType::Default,
            "CONSTRAINT" => TokenType::Constraint,
            "AUTO_INCREMENT" | "AUTOINCREMENT" => TokenType::AutoIncrement,
            "CASCADE" => TokenType::Cascade,
            "RESTRICT" => TokenType::Restrict,
            "RETURNING" => TokenType::Returning,
            "CONFLICT" => TokenType::Conflict,
            "DO" => TokenType::Do,
            "NOTHING" => TokenType::Nothing,
            "REPLACE" => TokenType::Replace,
            "IGNORE" => TokenType::Ignore,
            "MERGE" => TokenType::Merge,
            "MATCHED" => TokenType::Matched,
            "USING" => TokenType::Using,
            "TRUNCATE" => TokenType::Truncate,
            "SCHEMA" => TokenType::Schema,
            "DATABASE" => TokenType::Database,
            "VIEW" => TokenType::View,
            "MATERIALIZED" => TokenType::Materialized,
            "TEMPORARY" => TokenType::Temporary,
            "TEMP" => TokenType::Temp,
            "BEGIN" => TokenType::Begin,
            "COMMIT" => TokenType::Commit,
            "ROLLBACK" => TokenType::Rollback,
            "SAVEPOINT" => TokenType::Savepoint,
            "TRANSACTION" => TokenType::Transaction,
            "EXPLAIN" => TokenType::Explain,
            "ANALYZE" => TokenType::Analyze,
            "SHOW" => TokenType::Show,
            "USE" => TokenType::Use,
            "GRANT" => TokenType::Grant,
            "REVOKE" => TokenType::Revoke,
            "LATERAL" => TokenType::Lateral,
            "UNNEST" => TokenType::Unnest,
            "PIVOT" => TokenType::Pivot,
            "UNPIVOT" => TokenType::Unpivot,
            "TABLESAMPLE" => TokenType::Tablesample,
            "FETCH" => TokenType::Fetch,
            "FIRST" => TokenType::First,
            "NEXT" => TokenType::Next,
            "ONLY" => TokenType::Only,
            "NULLS" => TokenType::Nulls,
            "RESPECT" => TokenType::Respect,
            "TOP" => TokenType::Top,
            "COLLATE" => TokenType::Collate,
            "QUALIFY" => TokenType::Qualify,
            "XOR" => TokenType::Xor,
            "EXTRACT" => TokenType::Extract,
            "EPOCH" => TokenType::Epoch,
            "YEAR" => TokenType::Year,
            "MONTH" => TokenType::Month,
            "DAY" => TokenType::Day,
            "HOUR" => TokenType::Hour,
            "MINUTE" => TokenType::Minute,
            "SECOND" => TokenType::Second,
            _ => TokenType::Identifier,
        }
    }

    fn read_quoted_identifier(&mut self, start: usize, start_line: usize, start_col: usize, quote: char) -> Result<Token> {
        let end_char = if quote == '[' { ']' } else { quote };
        let mut value = String::new();
        loop {
            match self.advance() {
                Some(c) if c == end_char => {
                    if self.peek() == Some(end_char) && end_char != ']' {
                        self.advance();
                        value.push(end_char);
                    } else {
                        return Ok(Token::with_quote(
                            TokenType::Identifier,
                            value,
                            start,
                            start_line,
                            start_col,
                            quote,
                        ));
                    }
                }
                Some(c) => value.push(c),
                None => {
                    return Err(SqlglotError::TokenizerError {
                        message: format!("Unterminated quoted identifier (expected {end_char})"),
                        position: start,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple_select() {
        let mut tokenizer = Tokenizer::new("SELECT a, b FROM t");
        let tokens = tokenizer.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Select);
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].value, "a");
        assert_eq!(tokens[2].token_type, TokenType::Comma);
        assert_eq!(tokens[3].token_type, TokenType::Identifier);
        assert_eq!(tokens[3].value, "b");
        assert_eq!(tokens[4].token_type, TokenType::From);
        assert_eq!(tokens[5].token_type, TokenType::Identifier);
        assert_eq!(tokens[5].value, "t");
        assert_eq!(tokens[6].token_type, TokenType::Eof);
    }

    #[test]
    fn test_tokenize_string_literal() {
        let mut tokenizer = Tokenizer::new("'hello world'");
        let tokens = tokenizer.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::String);
        assert_eq!(tokens[0].value, "hello world");
    }

    #[test]
    fn test_tokenize_operators() {
        let mut tokenizer = Tokenizer::new("a >= 1 AND b != 2");
        let tokens = tokenizer.tokenize().unwrap();
        assert_eq!(tokens[1].token_type, TokenType::GtEq);
        assert_eq!(tokens[3].token_type, TokenType::And);
        assert_eq!(tokens[5].token_type, TokenType::Neq);
    }

    #[test]
    fn test_tokenize_number() {
        let mut tokenizer = Tokenizer::new("123.45");
        let tokens = tokenizer.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Number);
        assert_eq!(tokens[0].value, "123.45");
    }

    #[test]
    fn test_tokenize_line_comment() {
        let mut tok = Tokenizer::with_comments("SELECT 1 -- comment\nFROM t");
        let tokens = tok.tokenize().unwrap();
        assert!(tokens.iter().any(|t| t.token_type == TokenType::LineComment));
    }

    #[test]
    fn test_tokenize_block_comment() {
        let mut tok = Tokenizer::with_comments("SELECT /* hello */ 1");
        let tokens = tok.tokenize().unwrap();
        assert!(tokens.iter().any(|t| t.token_type == TokenType::BlockComment));
    }

    #[test]
    fn test_tokenize_cte_keywords() {
        let mut tok = Tokenizer::new("WITH cte AS (SELECT 1) SELECT * FROM cte");
        let tokens = tok.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::With);
        assert_eq!(tokens[2].token_type, TokenType::As);
    }

    #[test]
    fn test_tokenize_double_colon() {
        let mut tok = Tokenizer::new("x::int");
        let tokens = tok.tokenize().unwrap();
        assert_eq!(tokens[1].token_type, TokenType::DoubleColon);
    }

    #[test]
    fn test_tokenize_cast() {
        let mut tok = Tokenizer::new("CAST(x AS INT)");
        let tokens = tok.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Cast);
    }

    #[test]
    fn test_tokenize_window() {
        let mut tok = Tokenizer::new("ROW_NUMBER() OVER (PARTITION BY id ORDER BY name)");
        let tokens = tok.tokenize().unwrap();
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Over));
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Partition));
    }

    #[test]
    fn test_line_tracking() {
        let mut tok = Tokenizer::new("SELECT\n  1");
        let tokens = tok.tokenize().unwrap();
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[1].line, 2);
    }

    #[test]
    fn test_tokenize_union_intersect_except() {
        let mut tok = Tokenizer::new("UNION INTERSECT EXCEPT");
        let tokens = tok.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Union);
        assert_eq!(tokens[1].token_type, TokenType::Intersect);
        assert_eq!(tokens[2].token_type, TokenType::Except);
    }
}
