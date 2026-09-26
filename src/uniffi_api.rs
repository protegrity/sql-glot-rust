//! UniFFI bindings for Swift, Kotlin, and Python.
//!
//! Enabled with the `uniffi` feature. Generate bindings from the built
//! library with the `uniffi-bindgen` binary (see `make uniffi-bindings`).
//!
//! The AST is exposed through [`SqlStatement`], an opaque handle whose full
//! structure is available as JSON using the same serde schema as the C FFI.

use std::sync::Arc;

use crate::ast::Statement;
use crate::dialects::Dialect;
use crate::errors::SqlglotError as CoreError;

/// Error raised by the foreign-language API.
#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum SqlglotError {
    /// The SQL text could not be tokenized.
    #[error("tokenizer error at position {position}: {detail}")]
    Tokenizer { detail: String, position: u64 },
    /// The SQL text could not be parsed.
    #[error("parser error: {detail}")]
    Parser { detail: String },
    /// The statement uses a construct the target dialect cannot express.
    #[error("unsupported feature for dialect: {detail}")]
    UnsupportedDialectFeature { detail: String },
    /// The supplied AST JSON does not describe a valid statement.
    #[error("invalid AST JSON: {detail}")]
    InvalidAst { detail: String },
    /// An unexpected internal failure.
    #[error("internal error: {detail}")]
    Internal { detail: String },
}

impl From<CoreError> for SqlglotError {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::TokenizerError { message, position } => Self::Tokenizer {
                detail: message,
                position: position as u64,
            },
            CoreError::ParserError { message } => Self::Parser { detail: message },
            unexpected @ CoreError::UnexpectedToken { .. } => Self::Parser {
                detail: unexpected.to_string(),
            },
            CoreError::UnsupportedDialectFeature(detail) => {
                Self::UnsupportedDialectFeature { detail }
            }
            CoreError::Internal(detail) => Self::Internal { detail },
        }
    }
}

/// A parsed SQL statement.
#[derive(Debug, uniffi::Object)]
pub struct SqlStatement {
    inner: Statement,
}

impl SqlStatement {
    fn wrap(inner: Statement) -> Arc<Self> {
        Arc::new(Self { inner })
    }

    /// The underlying Rust AST.
    #[must_use]
    pub fn statement(&self) -> &Statement {
        &self.inner
    }
}

#[uniffi::export]
impl SqlStatement {
    /// Rebuild a statement from AST JSON produced by [`SqlStatement::to_json`].
    #[uniffi::constructor]
    pub fn from_json(json: String) -> Result<Arc<Self>, SqlglotError> {
        serde_json::from_str(&json)
            .map(Self::wrap)
            .map_err(|error| SqlglotError::InvalidAst {
                detail: error.to_string(),
            })
    }

    /// Serialize the full AST to JSON.
    pub fn to_json(&self) -> Result<String, SqlglotError> {
        serde_json::to_string(&self.inner).map_err(|error| SqlglotError::Internal {
            detail: error.to_string(),
        })
    }

    /// Render the statement as SQL for `dialect`.
    ///
    /// This renders the AST as-is. Use [`transpile`] to also apply the
    /// source-to-target dialect rewrites (e.g. `LIMIT` to `TOP`).
    pub fn generate(&self, dialect: Dialect) -> Result<String, SqlglotError> {
        crate::validate_dialect_support(&self.inner, dialect)?;
        Ok(crate::generate(&self.inner, dialect))
    }

    /// Render the statement as pretty-printed SQL for `dialect`.
    pub fn generate_pretty(&self, dialect: Dialect) -> Result<String, SqlglotError> {
        crate::validate_dialect_support(&self.inner, dialect)?;
        Ok(crate::generate_pretty(&self.inner, dialect))
    }
}

/// Parse a single SQL statement.
#[uniffi::export]
pub fn parse(sql: String, dialect: Dialect) -> Result<Arc<SqlStatement>, SqlglotError> {
    Ok(SqlStatement::wrap(crate::parser::parse(&sql, dialect)?))
}

/// Parse semicolon-separated SQL statements.
#[uniffi::export]
pub fn parse_statements(
    sql: String,
    dialect: Dialect,
) -> Result<Vec<Arc<SqlStatement>>, SqlglotError> {
    Ok(crate::parser::parse_statements(&sql, dialect)?
        .into_iter()
        .map(SqlStatement::wrap)
        .collect())
}

/// Transpile a single SQL statement from one dialect to another.
#[uniffi::export]
pub fn transpile(
    sql: String,
    read_dialect: Dialect,
    write_dialect: Dialect,
) -> Result<String, SqlglotError> {
    Ok(crate::transpile(&sql, read_dialect, write_dialect)?)
}

/// The library version.
#[uniffi::export]
#[must_use]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::{Token, TokenType};

    #[test]
    fn core_errors_map_to_structured_foreign_errors() {
        assert_eq!(
            SqlglotError::from(CoreError::TokenizerError {
                message: "bad".into(),
                position: 7,
            }),
            SqlglotError::Tokenizer {
                detail: "bad".into(),
                position: 7,
            }
        );
        assert_eq!(
            SqlglotError::from(CoreError::ParserError {
                message: "oops".into()
            }),
            SqlglotError::Parser {
                detail: "oops".into()
            }
        );
        assert_eq!(
            SqlglotError::from(CoreError::UnsupportedDialectFeature("arr".into())),
            SqlglotError::UnsupportedDialectFeature {
                detail: "arr".into()
            }
        );
        assert_eq!(
            SqlglotError::from(CoreError::Internal("boom".into())),
            SqlglotError::Internal {
                detail: "boom".into()
            }
        );
    }

    #[test]
    fn unexpected_token_keeps_the_token_in_the_parser_detail() {
        let token = Token::new(TokenType::Comma, ",", 3);
        let SqlglotError::Parser { detail } =
            SqlglotError::from(CoreError::UnexpectedToken { token })
        else {
            panic!("UnexpectedToken must map to Parser");
        };
        assert!(detail.starts_with("Unexpected token:"), "{detail}");
        assert!(detail.contains("Comma"), "{detail}");
    }

    #[test]
    fn version_matches_the_crate() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
