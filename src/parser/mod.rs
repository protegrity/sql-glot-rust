mod sql_parser;

pub use sql_parser::Parser;

use crate::ast::Statement;
use crate::dialects::{Dialect, is_tsql_family};
use crate::errors::Result;

/// Parse a SQL string into a [`Statement`] AST using the given dialect.
///
/// # Errors
///
/// Returns a [`SqlglotError`](crate::errors::SqlglotError) if the input
/// is not valid SQL.
pub fn parse(sql: &str, dialect: Dialect) -> Result<Statement> {
    let mut parser = Parser::new_with_bracket_identifiers(sql, is_tsql_family(dialect))?;
    parser.parse_statement()
}

/// Parse a SQL string into a [`Statement`] AST, preserving SQL comments.
///
/// Comments are attached to the nearest AST node and survive through
/// transformation and generation.
///
/// # Errors
///
/// Returns a [`SqlglotError`](crate::errors::SqlglotError) if the input
/// is not valid SQL.
pub fn parse_with_comments(sql: &str, dialect: Dialect) -> Result<Statement> {
    let mut parser =
        Parser::new_with_comments_and_bracket_identifiers(sql, is_tsql_family(dialect))?;
    parser.parse_statement()
}

/// Parse a SQL string containing multiple statements separated by semicolons.
///
/// # Errors
///
/// Returns a [`SqlglotError`](crate::errors::SqlglotError) if parsing fails.
pub fn parse_statements(sql: &str, dialect: Dialect) -> Result<Vec<Statement>> {
    let mut parser = Parser::new_with_bracket_identifiers(sql, is_tsql_family(dialect))?;
    parser.parse_statements()
}

/// Parse multiple semicolon-separated SQL statements, preserving comments.
///
/// # Errors
///
/// Returns a [`SqlglotError`](crate::errors::SqlglotError) if parsing fails.
pub fn parse_statements_with_comments(sql: &str, dialect: Dialect) -> Result<Vec<Statement>> {
    let mut parser =
        Parser::new_with_comments_and_bracket_identifiers(sql, is_tsql_family(dialect))?;
    parser.parse_statements()
}
