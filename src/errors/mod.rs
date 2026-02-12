use thiserror::Error;

use crate::tokens::Token;

/// Errors that may occur during SQL parsing, generation, or optimization.
#[derive(Debug, Error)]
pub enum SqlglotError {
    /// An error occurred during tokenization.
    #[error("Tokenizer error at position {position}: {message}")]
    TokenizerError { message: String, position: usize },

    /// An error occurred during parsing.
    #[error("Parser error: {message}")]
    ParserError { message: String },

    /// An unexpected token was encountered.
    #[error("Unexpected token: {token:?}")]
    UnexpectedToken { token: Token },

    /// An unsupported feature was encountered for the target dialect.
    #[error("Unsupported feature for dialect: {0}")]
    UnsupportedDialectFeature(String),

    /// A generic internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience alias for results that can fail with [`SqlglotError`].
pub type Result<T> = std::result::Result<T, SqlglotError>;
