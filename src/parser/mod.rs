mod sql_parser;

pub use sql_parser::Parser;

use crate::ast::Statement;
use crate::dialects::Dialect;
use crate::errors::Result;

/// Parse a SQL string into a [`Statement`] AST using the given dialect.
///
/// # Errors
///
/// Returns a [`SqlglotError`](crate::errors::SqlglotError) if the input
/// is not valid SQL.
pub fn parse(sql: &str, _dialect: Dialect) -> Result<Statement> {
    let mut parser = Parser::new(sql)?;
    parser.parse_statement()
}

/// Parse a SQL string containing multiple statements separated by semicolons.
///
/// # Errors
///
/// Returns a [`SqlglotError`](crate::errors::SqlglotError) if parsing fails.
pub fn parse_statements(sql: &str, _dialect: Dialect) -> Result<Vec<Statement>> {
    let mut parser = Parser::new(sql)?;
    parser.parse_statements()
}
