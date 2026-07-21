use crate::errors::{Result, SqlglotError};
use crate::tokens::{Token, TokenType};

/// Identifier-start predicate. Accepts ASCII `_` plus any Unicode letter,
/// matching SQL:2003 §5.2 (PostgreSQL/MySQL/SQLite/Oracle/ClickHouse all
/// accept Unicode letters in regular identifiers).
#[inline]
fn is_identifier_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

/// Identifier-continue predicate. Accepts Unicode alphanumerics, `_`, `$`,
/// and additionally any non-ASCII printable character that is not a quote,
/// bracket, or operator delimiter. This permits identifiers like `n°`, `±x`,
/// or `tag€` that appear in some real-world corpora (auto-generated column
/// names, scientific tables) — every major engine accepts these inside
/// quoted identifiers and most accept them unquoted in tail position.
#[inline]
fn is_identifier_continue(c: char) -> bool {
    if c == '_' || c == '$' || c.is_alphanumeric() {
        return true;
    }
    if c.is_ascii() || c.is_whitespace() || c.is_control() {
        return false;
    }
    // Non-ASCII printable: reject only characters that play a structural
    // role in SQL syntax. Everything else (degree/euro/math symbols,
    // sub/superscripts, fraction slash) folds into the identifier tail.
    !matches!(
        c,
        '\u{00AB}' | '\u{00BB}' // « »
        | '\u{2018}' | '\u{2019}' // ‘ ’
        | '\u{201C}' | '\u{201D}' // “ ”
    )
}

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
    /// Last non-whitespace / non-comment token type emitted. Used by the
    /// `[` handler to disambiguate bracket-quoted identifiers from array
    /// subscripts.
    prev_token_type: Option<TokenType>,
    /// When `true`, `[...]` is always read as a bracket-quoted identifier
    /// (T-SQL / Fabric semantics), never as an array literal or subscript.
    /// These dialects use `[` solely for delimited identifiers and have no
    /// array syntax, so the context-free subscript heuristic would otherwise
    /// misparse `SELECT TOP 1 [col]` and implicit aliases like `x [col]`.
    brackets_are_identifiers: bool,
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
            prev_token_type: None,
            brackets_are_identifiers: false,
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
            prev_token_type: None,
            brackets_are_identifiers: false,
        }
    }

    /// Configure whether `[...]` is always tokenized as a bracket-quoted
    /// identifier (T-SQL / Fabric semantics) rather than being disambiguated
    /// against array subscript syntax. Returns `self` for builder-style use.
    #[must_use]
    pub fn with_bracket_identifiers(mut self, on: bool) -> Self {
        self.brackets_are_identifiers = on;
        self
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
                _ => {
                    self.prev_token_type = Some(token.token_type.clone());
                    tokens.push(token);
                }
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

    fn make_token(
        &self,
        token_type: TokenType,
        value: impl Into<String>,
        start: usize,
        start_line: usize,
        start_col: usize,
    ) -> Token {
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
                // In bracket-quoting dialects (T-SQL / Fabric) `[` is used
                // exclusively for delimited identifiers and there is no array
                // literal or subscript syntax, so `[...]` is unconditionally a
                // quoted identifier. This avoids the context-free heuristic
                // below misreading e.g. `SELECT TOP 1000 [col]` (number before
                // `[`) or `x [col]` (implicit alias) as an array subscript.
                if self.brackets_are_identifiers {
                    return self.read_quoted_identifier(start, start_line, start_col, '[');
                }
                // Decide between two readings of `[`:
                //   1. Bracket-quoted identifier (T-SQL / SQLite style): `[name]`,
                //      `[#]`, `[1]`, `[User Link]`. Inner content may be anything
                //      except `]` or newline.
                //   2. Array subscript / element selector: `arr[1]`, `arr[1:5]`.
                //
                // Disambiguate on the previously emitted token: array subscript
                // requires a subscriptable value on its left (closing paren /
                // closing bracket / identifier / string / number). After
                // statement-start, `AS`, `(`, `,`, operators, `BY`, etc. the
                // bracket can only be a quoted identifier.
                let prev_is_subscriptable = matches!(
                    self.prev_token_type,
                    Some(
                        TokenType::Identifier
                            | TokenType::RParen
                            | TokenType::RBracket
                            | TokenType::String
                            | TokenType::Number
                            // Type keywords commonly preceding array modifier `TYPE[N]`
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
                            | TokenType::Bool
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
                );

                let mut looks_like_ident = false;
                // Always try bracketed-ident interpretation when there is a
                // space inside before `]` (e.g. `id [User Link]` — implicit
                // alias). Real array subscripts never contain a literal space.
                let mut has_space_inside = false;
                let mut has_operator_inside = false;
                if prev_is_subscriptable {
                    let mut scan = self.pos;
                    while scan < self.input.len() {
                        let c = self.input[scan];
                        if c == ']' {
                            break;
                        }
                        if c == '\n' || c == '[' || c == ',' {
                            break;
                        }
                        if c == ' ' || c == '\t' {
                            has_space_inside = true;
                        }
                        if matches!(
                            c,
                            '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' | '&' | '|' | '^'
                        ) {
                            has_operator_inside = true;
                        }
                        scan += 1;
                    }
                }
                if !prev_is_subscriptable || (has_space_inside && !has_operator_inside) {
                    let mut scan = self.pos;
                    let mut saw_quote = false;
                    while scan < self.input.len() {
                        let c = self.input[scan];
                        if c == ']' {
                            // For ARRAY/typed subscripts, a `'` inside means
                            // it's a string literal cast (`array['lit'::T]`),
                            // not a bracket identifier. For non-subscriptable
                            // contexts (TSQL `[user's name]`), accept quotes.
                            looks_like_ident =
                                scan > self.pos && (!prev_is_subscriptable || !saw_quote);
                            break;
                        }
                        // `,` rules out `ARRAY[1,2,3]` style literals.
                        if c == '\n' || c == '[' || c == ',' {
                            break;
                        }
                        if c == '\'' {
                            saw_quote = true;
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
            '{' => {
                // ClickHouse parameter / typed placeholder `{name:Type}`.
                // The name is identifier-like; the type may itself contain
                // parens (e.g. `{ids:Array(UInt64)}`). Scan until the
                // matching `}` and emit a single Parameter token; fall back
                // to a plain `LBrace` otherwise.
                if self.peek().is_some_and(is_identifier_start) {
                    let mut i = 1usize;
                    while self.peek_at(i).is_some_and(|c| is_identifier_continue(c)) {
                        i += 1;
                    }
                    if self.peek_at(i) == Some(':') {
                        let mut value = String::from('{');
                        let mut depth = 0usize;
                        loop {
                            match self.peek() {
                                None => break,
                                Some('{') => {
                                    depth += 1;
                                    value.push('{');
                                    self.advance();
                                }
                                Some('}') => {
                                    if depth == 0 {
                                        value.push('}');
                                        self.advance();
                                        return Ok(self.make_token(
                                            TokenType::Parameter,
                                            value,
                                            start,
                                            start_line,
                                            start_col,
                                        ));
                                    }
                                    depth -= 1;
                                    value.push('}');
                                    self.advance();
                                }
                                Some(c) => {
                                    value.push(c);
                                    self.advance();
                                }
                            }
                        }
                        return Err(SqlglotError::TokenizerError {
                            message: "Unterminated parameter placeholder".into(),
                            position: start,
                        });
                    }
                }
                Ok(self.make_token(TokenType::LBrace, "{", start, start_line, start_col))
            }
            '}' => Ok(self.make_token(TokenType::RBrace, "}", start, start_line, start_col)),
            ',' => Ok(self.make_token(TokenType::Comma, ",", start, start_line, start_col)),
            ';' => Ok(self.make_token(TokenType::Semicolon, ";", start, start_line, start_col)),
            '.' => Ok(self.make_token(TokenType::Dot, ".", start, start_line, start_col)),
            '+' => Ok(self.make_token(TokenType::Plus, "+", start, start_line, start_col)),
            '~' => Ok(self.make_token(TokenType::BitwiseNot, "~", start, start_line, start_col)),
            '@' => {
                if self.peek() == Some('>') {
                    self.advance();
                    Ok(self.make_token(TokenType::AtArrow, "@>", start, start_line, start_col))
                } else {
                    Ok(self.make_token(TokenType::AtSign, "@", start, start_line, start_col))
                }
            }
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
                    Ok(
                        self.make_token(
                            TokenType::LineComment,
                            value,
                            start,
                            start_line,
                            start_col,
                        ),
                    )
                } else if self.peek() == Some('>') {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        Ok(self.make_token(
                            TokenType::DoubleArrow,
                            "->>",
                            start,
                            start_line,
                            start_col,
                        ))
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
                    Ok(self.make_token(
                        TokenType::BlockComment,
                        value,
                        start,
                        start_line,
                        start_col,
                    ))
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
                } else if self.peek() == Some('@') {
                    self.advance();
                    Ok(self.make_token(TokenType::ArrowAt, "<@", start, start_line, start_col))
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
                        Ok(self.make_token(
                            TokenType::HashDoubleArrow,
                            "#>>",
                            start,
                            start_line,
                            start_col,
                        ))
                    } else {
                        Ok(self.make_token(
                            TokenType::HashArrow,
                            "#>",
                            start,
                            start_line,
                            start_col,
                        ))
                    }
                } else if self.peek() == Some('#') {
                    // `##name##` — StackExchange Data Explorer style template
                    // placeholder. Surface as a regular identifier so the
                    // surrounding query parses. If we can't find a matching
                    // closing `##` on the same line, fall through to the
                    // line-comment behavior below.
                    let save_pos = self.pos;
                    let save_line = self.line;
                    let save_col = self.col;
                    self.advance(); // consume second `#`
                    let inner_start = self.pos;
                    let mut found_close = false;
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        if c == '#' && self.peek_at(1) == Some('#') {
                            found_close = true;
                            break;
                        }
                        self.advance();
                    }
                    if found_close {
                        let value: String = self.input[inner_start..self.pos].iter().collect();
                        self.advance(); // first closing `#`
                        self.advance(); // second closing `#`
                        return Ok(Token::with_quote(
                            TokenType::Identifier,
                            value,
                            start,
                            start_line,
                            start_col,
                            '#',
                        ));
                    }
                    // Rewind and fall through to line-comment handling.
                    self.pos = save_pos;
                    self.line = save_line;
                    self.col = save_col;
                    let mut value = String::from("#");
                    while self.peek().is_some_and(|c| c != '\n') {
                        value.push(self.advance().unwrap());
                    }
                    Ok(
                        self.make_token(
                            TokenType::LineComment,
                            value,
                            start,
                            start_line,
                            start_col,
                        ),
                    )
                } else if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    // DuckDB `#N` positional column reference. Emit as a
                    // Parameter so it parses inside expressions / ORDER BY.
                    let mut value = String::from("#");
                    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        value.push(self.advance().unwrap());
                    }
                    Ok(self.make_token(TokenType::Parameter, value, start, start_line, start_col))
                } else {
                    let mut value = String::from("#");
                    while self.peek().is_some_and(|c| c != '\n') {
                        value.push(self.advance().unwrap());
                    }
                    Ok(
                        self.make_token(
                            TokenType::LineComment,
                            value,
                            start,
                            start_line,
                            start_col,
                        ),
                    )
                }
            }

            // ── String literals ─────────────────────────────────────
            '\'' => self.read_string(start, start_line, start_col),

            // ── Numbers ─────────────────────────────────────────────
            c if c.is_ascii_digit() => self.read_number(start, start_line, start_col, c),

            // ── Identifiers and keywords ────────────────────────────
            c if is_identifier_start(c) => self.read_identifier(start, start_line, start_col, c),

            // ── Quoted identifiers (double quote) ───────────────────
            '"' => self.read_quoted_identifier(start, start_line, start_col, '"'),

            // ── Backtick identifiers (MySQL, BigQuery) ──────────────
            '`' => self.read_quoted_identifier(start, start_line, start_col, '`'),

            // ── Parameter markers ───────────────────────────────────
            '$' => {
                // PostgreSQL dollar-quoted string literal: `$$body$$` or
                // `$tag$body$tag$`. The tag is an optional identifier. We
                // detect the opening sequence and scan to the matching
                // closing sequence; the body may contain any characters.
                if self.peek() == Some('$') {
                    self.advance(); // closing $ of opening $$
                    let mut value = String::new();
                    while let Some(c) = self.peek() {
                        if c == '$' && self.peek_at(1) == Some('$') {
                            self.advance();
                            self.advance();
                            return Ok(self.make_token(
                                TokenType::String,
                                value,
                                start,
                                start_line,
                                start_col,
                            ));
                        }
                        value.push(self.advance().unwrap());
                    }
                    // Unterminated — fall back to the captured body as String.
                    return Ok(self.make_token(
                        TokenType::String,
                        value,
                        start,
                        start_line,
                        start_col,
                    ));
                }
                // Speculative `$tag$ … $tag$` form. Only treat as a
                // dollar-quote if the tokens after the tag actually form
                // a valid closing sequence; otherwise fall through to
                // the identifier / parameter handling below.
                if self.peek().is_some_and(is_identifier_start) {
                    let save_pos = self.pos;
                    let save_line = self.line;
                    let save_col = self.col;
                    let mut tag = String::new();
                    while self.peek().is_some_and(is_identifier_continue) {
                        tag.push(self.advance().unwrap());
                    }
                    if self.peek() == Some('$') {
                        self.advance();
                        // Look ahead for matching `$tag$` close.
                        let mut value = String::new();
                        let mut closed = false;
                        while let Some(c) = self.peek() {
                            if c == '$' {
                                // Test for the closing tag.
                                let mut matched = true;
                                for (i, ch) in tag.chars().enumerate() {
                                    if self.peek_at(i + 1) != Some(ch) {
                                        matched = false;
                                        break;
                                    }
                                }
                                if matched && self.peek_at(tag.len() + 1) == Some('$') {
                                    // Consume `$tag$`.
                                    for _ in 0..(tag.len() + 2) {
                                        self.advance();
                                    }
                                    closed = true;
                                    break;
                                }
                            }
                            value.push(self.advance().unwrap());
                        }
                        if closed {
                            return Ok(self.make_token(
                                TokenType::String,
                                value,
                                start,
                                start_line,
                                start_col,
                            ));
                        }
                    }
                    // Not a dollar-quote; rewind and fall through to the
                    // identifier path.
                    self.pos = save_pos;
                    self.line = save_line;
                    self.col = save_col;
                }
                if self.peek() == Some('{') {
                    // `${name}` template variable (DuckDB / shell-style). Consume
                    // through the closing `}` and emit as a single Parameter token.
                    let mut value = String::from("$");
                    value.push(self.advance().unwrap()); // '{'
                    while let Some(c) = self.peek() {
                        value.push(self.advance().unwrap());
                        if c == '}' {
                            break;
                        }
                    }
                    Ok(self.make_token(TokenType::Parameter, value, start, start_line, start_col))
                } else if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    let mut value = String::from("$");
                    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        value.push(self.advance().unwrap());
                    }
                    Ok(self.make_token(TokenType::Parameter, value, start, start_line, start_col))
                } else if self.peek().is_some_and(is_identifier_start) {
                    // `$alias` / `$_`: identifier with a leading `$`. Appears
                    // in auto-generated column names (e.g. `purse__$__`) and as
                    // SELECT aliases (`AS $__`). PostgreSQL prepared-statement
                    // parameters (`$1`, `$2`) keep the digits-only fast path
                    // above; the `$<digit>` form cannot start an identifier so
                    // there is no ambiguity.
                    let mut value = String::from("$");
                    while self.peek().is_some_and(is_identifier_continue) {
                        value.push(self.advance().unwrap());
                    }
                    Ok(self.make_token(TokenType::Identifier, value, start, start_line, start_col))
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
                        return Ok(self.make_token(
                            TokenType::String,
                            value,
                            start,
                            start_line,
                            start_col,
                        ));
                    }
                }
                Some('\\') => match self.peek() {
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
                    Some('\'') => {
                        self.advance();
                        value.push('\'');
                    }
                    Some('"') => {
                        self.advance();
                        value.push('"');
                    }
                    Some('0') => {
                        self.advance();
                        value.push('\0');
                    }
                    Some('b') => {
                        self.advance();
                        value.push('\u{0008}');
                    }
                    Some('f') => {
                        self.advance();
                        value.push('\u{000C}');
                    }
                    Some('v') => {
                        self.advance();
                        value.push('\u{000B}');
                    }
                    Some('a') => {
                        self.advance();
                        value.push('\u{0007}');
                    }
                    Some(c) if c.is_ascii_alphanumeric() || c == '?' => {
                        // Tolerate other escape sequences (e.g. ClickHouse
                        // \xAA, \uXXXX, \?) by consuming the introducer
                        // and keeping the literal character in the string.
                        self.advance();
                        value.push('\\');
                        value.push(c);
                    }
                    _ => {
                        value.push('\\');
                    }
                },
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

    fn read_number(
        &mut self,
        start: usize,
        start_line: usize,
        start_col: usize,
        first: char,
    ) -> Result<Token> {
        let mut value = String::new();
        value.push(first);

        if first == '0' && self.peek().is_some_and(|c| c == 'x' || c == 'X') {
            value.push(self.advance().unwrap());
            while self
                .peek()
                .is_some_and(|c| c.is_ascii_hexdigit() || c == '_')
            {
                value.push(self.advance().unwrap());
            }
            // Optional binary-exponent suffix `pN` / `PN` for hex floats
            // (`0x1p-1022`, `0x123p4`).
            if self.peek().is_some_and(|c| c == 'p' || c == 'P') {
                value.push(self.advance().unwrap());
                if self.peek().is_some_and(|c| c == '+' || c == '-') {
                    value.push(self.advance().unwrap());
                }
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    value.push(self.advance().unwrap());
                }
            }
            return Ok(self.make_token(TokenType::HexString, value, start, start_line, start_col));
        }

        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
            value.push(self.advance().unwrap());
        }

        if self.peek() == Some('.')
            && (self.peek_at(1).is_some_and(|c| c.is_ascii_digit())
                || !self.peek_at(1).is_some_and(is_identifier_start))
        {
            value.push(self.advance().unwrap());
            while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
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

        // ClickHouse / Hive allow identifiers that start with digits
        // (`03720_test_alter`, `1_table`). If the run of digits is butted
        // directly against an identifier-continue character, treat the
        // whole span as an identifier.
        if !value.contains('.')
            && !value.contains('e')
            && !value.contains('E')
            && self.peek().is_some_and(is_identifier_continue)
        {
            while self.peek().is_some_and(is_identifier_continue) {
                value.push(self.advance().unwrap());
            }
            let token_type = Self::keyword_type(&value);
            return Ok(self.make_token(token_type, value, start, start_line, start_col));
        }

        Ok(self.make_token(TokenType::Number, value, start, start_line, start_col))
    }

    fn read_identifier(
        &mut self,
        start: usize,
        start_line: usize,
        start_col: usize,
        first: char,
    ) -> Result<Token> {
        let mut value = String::new();
        value.push(first);
        while self.peek().is_some_and(is_identifier_continue) {
            // Don't swallow a `$` that starts a template variable
            // (`${name}`) or a numbered parameter (`$1`) — those need to
            // tokenize as their own Parameter token.
            if self.peek() == Some('$') {
                let next = self.peek_at(1);
                if matches!(next, Some('{')) || next.is_some_and(|c| c.is_ascii_digit()) {
                    break;
                }
            }
            value.push(self.advance().unwrap());
        }

        // Phase 1 support: treat N'...' / n'...' as a string literal token.
        // This unblocks Oracle/TSQL national string parsing without AST changes.
        if value.len() == 1
            && value
                .as_bytes()
                .first()
                .is_some_and(|b| b.eq_ignore_ascii_case(&b'n'))
            && self.peek() == Some('\'')
        {
            self.advance(); // consume opening quote
            let mut token = self.read_string(start, start_line, start_col)?;
            token.token_type = TokenType::NationalString;
            return Ok(token);
        }

        // PostgreSQL / SQL standard string-literal prefixes:
        //   E'...'  — escape string (backslash escapes processed)
        //   B'...'  — bit string
        //   X'...'  — hex / byte string
        //   U&'...' — Unicode escape string (we accept the prefix and string;
        //             the trailing `UESCAPE 'x'` clause is parser-side noise)
        // Each prefix tokenizes as a single-char identifier; merge with the
        // following `'...'` literal into a String token so the SQL parses.
        if value.len() == 1
            && value
                .as_bytes()
                .first()
                .is_some_and(|b| matches!(b.to_ascii_uppercase(), b'E' | b'B' | b'X'))
            && self.peek() == Some('\'')
        {
            self.advance();
            return self.read_string(start, start_line, start_col);
        }
        // U&'...' — Unicode escape literal.
        if value.len() == 1
            && value
                .as_bytes()
                .first()
                .is_some_and(|b| b.eq_ignore_ascii_case(&b'u'))
            && self.peek() == Some('&')
            && self.peek_at(1) == Some('\'')
        {
            self.advance(); // &
            self.advance(); // '
            return self.read_string(start, start_line, start_col);
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
            "CUBE" => TokenType::Cube,
            "ROLLUP" => TokenType::Rollup,
            "GROUPING" => TokenType::Grouping,
            "SETS" => TokenType::Sets,
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

    fn read_quoted_identifier(
        &mut self,
        start: usize,
        start_line: usize,
        start_col: usize,
        quote: char,
    ) -> Result<Token> {
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
        assert!(
            tokens
                .iter()
                .any(|t| t.token_type == TokenType::LineComment)
        );
    }

    #[test]
    fn test_tokenize_block_comment() {
        let mut tok = Tokenizer::with_comments("SELECT /* hello */ 1");
        let tokens = tok.tokenize().unwrap();
        assert!(
            tokens
                .iter()
                .any(|t| t.token_type == TokenType::BlockComment)
        );
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

    #[test]
    fn test_tokenize_n_prefixed_string_literal_uppercase() {
        let mut tok = Tokenizer::new("N'Hello'");
        let tokens = tok.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::NationalString);
        assert_eq!(tokens[0].value, "Hello");
    }

    #[test]
    fn test_tokenize_n_prefixed_string_literal_lowercase() {
        let mut tok = Tokenizer::new("n'hello'");
        let tokens = tok.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::NationalString);
        assert_eq!(tokens[0].value, "hello");
    }

    #[test]
    fn test_tokenize_n_prefixed_string_literal_escaped_quote() {
        let mut tok = Tokenizer::new("N'can''t stop'");
        let tokens = tok.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::NationalString);
        assert_eq!(tokens[0].value, "can't stop");
    }

    #[test]
    fn test_tokenize_n_prefixed_string_literal_unicode() {
        let mut tok = Tokenizer::new("N'テスト'");
        let tokens = tok.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::NationalString);
        assert_eq!(tokens[0].value, "テスト");
    }

    #[test]
    fn test_tokenize_identifier_n_without_quote() {
        let mut tok = Tokenizer::new("SELECT N FROM t");
        let tokens = tok.tokenize().unwrap();
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].value, "N");
    }

    #[test]
    fn test_tokenize_identifier_name_starting_with_n() {
        let mut tok = Tokenizer::new("SELECT NAME FROM t");
        let tokens = tok.tokenize().unwrap();
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].value, "NAME");
    }
}
