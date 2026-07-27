mod sql_parser;

pub use sql_parser::Parser;

use crate::ast::{DataType, Statement};
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

/// Parse a standalone SQL data type using the same grammar as casts and
/// column definitions.
///
/// # Errors
///
/// Returns a [`SqlglotError`](crate::errors::SqlglotError) if the input is not
/// a valid data type or contains trailing tokens.
pub fn parse_data_type(sql: &str, dialect: Dialect) -> Result<DataType> {
    let mut parser = Parser::new_with_bracket_identifiers(sql, is_tsql_family(dialect))?;
    parser.parse_data_type_expression()
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

#[cfg(test)]
mod tests {
    use super::parse_data_type;
    use crate::ast::DataType;
    use crate::dialects::Dialect;

    #[test]
    fn parses_parameterized_data_types_without_losing_information() {
        assert_eq!(
            parse_data_type("VARCHAR(255)", Dialect::Postgres).unwrap(),
            DataType::Varchar(Some(255))
        );
        assert_eq!(
            parse_data_type("DECIMAL(18, 4)", Dialect::Postgres).unwrap(),
            DataType::Decimal {
                precision: Some(18),
                scale: Some(4),
            }
        );
        assert_eq!(
            parse_data_type("TIMESTAMP(6) WITH TIME ZONE", Dialect::Postgres).unwrap(),
            DataType::Timestamp {
                precision: Some(6),
                with_tz: true,
            }
        );
    }

    #[test]
    fn rejects_trailing_tokens() {
        assert!(parse_data_type("INTEGER nonsense", Dialect::Ansi).is_err());
    }
}
