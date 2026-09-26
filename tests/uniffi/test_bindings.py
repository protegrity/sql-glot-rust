"""Exercises the generated Python bindings against the compiled library."""

import json
import os
import unittest

import sqlglot_rust as sg


class GeneratedPythonBindings(unittest.TestCase):
    def test_loads_the_library_under_test(self):
        self.assertEqual(sg.version(), os.environ["SQLGLOT_EXPECTED_VERSION"])

    def test_exposes_every_dialect(self):
        self.assertEqual(len(sg.Dialect), 30)

    def test_parse_then_generate(self):
        statement = sg.parse("SELECT `select` FROM `events`", sg.Dialect.MYSQL)
        self.assertEqual(statement.generate(sg.Dialect.TSQL), "SELECT [select] FROM [events]")

    def test_generate_pretty(self):
        statement = sg.parse("SELECT a, b FROM t WHERE a > 1", sg.Dialect.POSTGRES)
        self.assertEqual(
            statement.generate_pretty(sg.Dialect.POSTGRES),
            "SELECT\n  a,\n  b\nFROM\n  t\nWHERE\n  a > 1",
        )

    def test_transpile(self):
        self.assertEqual(
            sg.transpile("SELECT * FROM t LIMIT 10", sg.Dialect.MYSQL, sg.Dialect.TSQL),
            "SELECT TOP 10 * FROM t",
        )

    def test_parse_statements(self):
        statements = sg.parse_statements("SELECT 1; SELECT 2", sg.Dialect.ANSI)
        self.assertEqual([s.generate(sg.Dialect.ANSI) for s in statements], ["SELECT 1", "SELECT 2"])

    def test_ast_access_and_edit(self):
        statement = sg.parse("SELECT a, b FROM t WHERE a > 1", sg.Dialect.POSTGRES)
        ast = json.loads(statement.to_json())
        self.assertEqual(len(ast["Select"]["columns"]), 2)
        self.assertEqual(ast["Select"]["from"]["source"]["Table"]["name"], "t")

        ast["Select"]["from"]["source"]["Table"]["name"] = "renamed"
        edited = sg.SqlStatement.from_json(json.dumps(ast))
        self.assertEqual(edited.generate(sg.Dialect.POSTGRES), "SELECT a, b FROM renamed WHERE a > 1")

    def test_parser_error(self):
        with self.assertRaises(sg.SqlglotError.Parser):
            sg.parse("SELECT FROM WHERE", sg.Dialect.ANSI)

    def test_tokenizer_error_carries_position(self):
        with self.assertRaises(sg.SqlglotError.Tokenizer) as raised:
            sg.parse("SELECT 'abc", sg.Dialect.ANSI)
        self.assertEqual(raised.exception.detail, "Unterminated string literal")
        self.assertEqual(raised.exception.position, 7)

    def test_unsupported_dialect_feature(self):
        statement = sg.parse("SELECT ARRAY[1, 2]", sg.Dialect.POSTGRES)
        with self.assertRaises(sg.SqlglotError.UnsupportedDialectFeature) as raised:
            statement.generate(sg.Dialect.TSQL)
        self.assertEqual(raised.exception.detail, "ARRAY constructor has no T-SQL equivalent")

    def test_invalid_ast(self):
        with self.assertRaises(sg.SqlglotError.InvalidAst):
            sg.SqlStatement.from_json('{"Nope": 1}')


if __name__ == "__main__":
    unittest.main(verbosity=2)
