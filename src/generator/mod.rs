mod sql_generator;

pub use sql_generator::Generator;

use crate::ast::Statement;
use crate::dialects::Dialect;

/// Generate a SQL string from a [`Statement`] AST for the given dialect.
#[must_use]
pub fn generate(statement: &Statement, _dialect: Dialect) -> String {
    let mut generator = Generator::new();
    generator.generate(statement)
}

/// Generate a pretty-printed SQL string from a [`Statement`] AST.
///
/// Produces formatted SQL with newlines and indentation for readability.
#[must_use]
pub fn generate_pretty(statement: &Statement, _dialect: Dialect) -> String {
    let mut generator = Generator::pretty();
    generator.generate(statement)
}
