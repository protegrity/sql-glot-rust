mod tokenizer;

pub use tokenizer::Tokenizer;

use serde::{Deserialize, Serialize};

/// The type of a SQL token.
///
/// Modeled after Python sqlglot's comprehensive token type system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenType {
    // ── Literals ────────────────────────────────────────────────────
    Number,
    String,
    Identifier,
    BitString,
    HexString,
    Parameter, // $1, :name, ?

    // ── Keywords ────────────────────────────────────────────────────
    Select,
    From,
    Where,
    And,
    Or,
    Not,
    As,
    Join,
    Inner,
    Left,
    Right,
    Full,
    Outer,
    Cross,
    On,
    Insert,
    Into,
    Values,
    Update,
    Set,
    Delete,
    Create,
    Table,
    Drop,
    Alter,
    Index,
    If,
    Exists,
    In,
    Is,
    Null,
    Like,
    ILike,
    Between,
    Case,
    When,
    Then,
    Else,
    End,
    Order,
    By,
    Asc,
    Desc,
    Group,
    Having,
    Limit,
    Offset,
    Union,
    All,
    Distinct,
    True,
    False,

    // Set operations
    Intersect,
    Except,

    // CTEs
    With,
    Recursive,

    // Subqueries / misc
    Any,
    Some,

    // Casting
    Cast,

    // Window functions
    Over,
    Partition,
    Window,
    Rows,
    Range,
    Unbounded,
    Preceding,
    Following,
    CurrentRow,
    Filter,

    // Data types as keywords
    Int,
    Integer,
    BigInt,
    SmallInt,
    TinyInt,
    Float,
    Double,
    Decimal,
    Numeric,
    Real,
    Varchar,
    Char,
    Text,
    Boolean,
    Bool,
    Date,
    Timestamp,
    TimestampTz,
    Time,
    Interval,
    Blob,
    Bytea,
    Json,
    Jsonb,
    Uuid,
    Array,
    Map,
    Struct,

    // Constraints & DDL
    Primary,
    Key,
    Foreign,
    References,
    Unique,
    Check,
    Default,
    Constraint,
    AutoIncrement,
    NotNull,
    Cascade,
    Restrict,
    NoAction,
    SetNull,
    SetDefault,

    // Additional DML
    Returning,
    Conflict,
    Do,
    Nothing,
    Replace,
    Ignore,
    Merge,
    Matched,
    Using,
    Truncate,

    // Schema
    Schema,
    Database,
    View,
    Materialized,
    Temporary,
    Temp,

    // Transaction
    Begin,
    Commit,
    Rollback,
    Savepoint,
    Transaction,

    // Misc keywords
    Explain,
    Analyze,
    Describe,
    Show,
    Use,
    Grant,
    Revoke,
    Lateral,
    Unnest,
    Pivot,
    Unpivot,
    Tablesample,
    Fetch,
    First,
    Next,
    Only,
    Percent,
    WithTies,
    Nulls,
    Respect,
    Top,
    Collate,
    Comment,
    Isnull,
    Notnull,
    Escape,

    // Existence checks
    Qualify,

    // Logical
    Xor,

    // Special expressions
    Extract,
    Epoch,
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,

    // ── Operators ───────────────────────────────────────────────────
    Plus,
    Minus,
    Star,
    Slash,
    Percent2,     // % as modulo operator
    Eq,
    Neq,          // <> or !=
    Lt,
    Gt,
    LtEq,
    GtEq,
    Concat,       // ||
    BitwiseAnd,   // &
    BitwiseOr,    // |
    BitwiseXor,   // ^
    BitwiseNot,   // ~
    ShiftLeft,    // <<
    ShiftRight,   // >>
    DoubleColon,  // :: (Postgres cast)
    Arrow,        // ->
    DoubleArrow,  // ->>
    HashArrow,    // #>
    HashDoubleArrow, // #>>
    AtSign,       // @
    Scope,        // ::

    // ── Punctuation ────────────────────────────────────────────────
    LParen,
    RParen,
    LBracket,     // [
    RBracket,     // ]
    LBrace,       // {
    RBrace,       // }
    Comma,
    Semicolon,
    Dot,
    Colon,
    DoubleColon2, // duplicated for compat -- use DoubleColon

    // ── Special ────────────────────────────────────────────────────
    Whitespace,
    LineComment,
    BlockComment,
    Eof,
}

/// A token produced by the tokenizer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub token_type: TokenType,
    pub value: String,
    pub line: usize,
    pub col: usize,
    pub position: usize,
    /// For quoted identifiers: the delimiter character ('"', '`', or '[').
    /// '\0' means unquoted.
    #[serde(default)]
    pub quote_char: char,
}

impl Token {
    #[must_use]
    pub fn new(token_type: TokenType, value: impl Into<String>, position: usize) -> Self {
        Self {
            token_type,
            value: value.into(),
            line: 0,
            col: 0,
            position,
            quote_char: '\0',
        }
    }

    #[must_use]
    pub fn with_location(
        token_type: TokenType,
        value: impl Into<String>,
        position: usize,
        line: usize,
        col: usize,
    ) -> Self {
        Self {
            token_type,
            value: value.into(),
            line,
            col,
            position,
            quote_char: '\0',
        }
    }

    #[must_use]
    pub fn with_quote(
        token_type: TokenType,
        value: impl Into<String>,
        position: usize,
        line: usize,
        col: usize,
        quote_char: char,
    ) -> Self {
        Self {
            token_type,
            value: value.into(),
            line,
            col,
            position,
            quote_char,
        }
    }
}
