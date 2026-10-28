#!/usr/bin/env python3
"""Check issue #39 against SQLite and, optionally, a real MySQL server.

Build: cargo build --locked --features cli --bin sqlglot
Run: python3 scripts/operator_precedence.py --binary target/debug/sqlglot \
       --mysql-command mysql --host=127.0.0.1 --user=root
Only scalar SELECTs are executed. The MySQL connection must use default SQL
mode (without PIPES_AS_CONCAT); its version and mode are printed for evidence.
"""
import argparse
import itertools
import json
import sqlite3
import subprocess
from decimal import Decimal
from pathlib import Path

OPERATORS = {
    "Plus": "+", "Minus": "-", "Multiply": "*", "Divide": "/",
    "Modulo": "%", "Concat": "||", "BitwiseAnd": "&", "BitwiseOr": "|",
    "BitwiseXor": "^", "ShiftLeft": "<<", "ShiftRight": ">>", "Or": "OR", "And": "AND",
}


def ast_sql(expr):
    if "BinaryOp" in expr:
        node = expr["BinaryOp"]
        return f"({ast_sql(node['left'])} {OPERATORS[node['op']]} {ast_sql(node['right'])})"
    if "Nested" in expr:
        return f"({ast_sql(expr['Nested'])})"
    if "Number" in expr:
        return expr["Number"]
    if "StringLiteral" in expr:
        return "'" + expr["StringLiteral"].replace("'", "''") + "'"
    if expr == "Null":
        return "NULL"
    raise AssertionError(f"Unexpected test AST: {expr}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--mysql-command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    binary = str(args.binary.resolve())
    sqlite = sqlite3.connect(":memory:")
    print(f"SQLite {sqlite3.sqlite_version}")

    def mysql(sql):
        result = subprocess.run(
            args.mysql_command + ["--batch", "--raw", "--skip-column-names"],
            input=sql, text=True, capture_output=True, check=True,
        )
        return [line.split("\t") for line in result.stdout.splitlines()]

    def cli(command, sql, read, write=None, optimize=False):
        cmd = [binary, command, "--read", read]
        if write:
            cmd += ["--write", write]
        if optimize:
            cmd += ["--optimize"]
        return subprocess.run(cmd, input=sql, text=True, capture_output=True, check=True).stdout.strip()

    def same_sqlite(a, b):
        return a == b and type(a) is type(b)

    dialects = ["sqlite"]
    if args.mysql_command:
        info = mysql("SELECT VERSION(), @@sql_mode;")[0]
        print(f"MySQL {info[0]}, sql_mode={info[1]}")
        assert "PIPES_AS_CONCAT" not in info[1], "This test requires default pipe semantics"
        dialects.append("mysql")
    else:
        print("MySQL skipped: supply --mysql-command to validate both engines")

    for dialect in dialects:
        ops = ["*", "/", "%", "+", "-", "&", "|", "<<", ">>"]
        ops.append("||" if dialect == "sqlite" else "^")
        pairs = list(itertools.product(ops, repeat=2))
        sources = [f"SELECT 17 {a} 3 {b} 1" for a, b in pairs]
        sources += (["SELECT 1 | 2 & 4", "SELECT 1 || 2 * 3", "SELECT '1' || '2' * 3"]
                    if dialect == "sqlite" else
                    ["SELECT 2 ^ 3 * 4", "SELECT 2 * 3 ^ 4", "SELECT 0 || 1 * 0", "SELECT 0 || 1 & 2 && 3 | 4"])
        batch = ";\n".join(sources)
        trees = json.loads(cli("parse", batch, dialect))
        emitted = cli("transpile", batch, dialect, dialect).split(";\n")
        optimized = cli("transpile", batch, dialect, dialect, True).split(";\n")
        assert len(trees) == len(emitted) == len(optimized) == len(sources)
        mysql_probes = []
        for i, (source, tree, output, folded) in enumerate(zip(sources, trees, emitted, optimized)):
            expr = tree["Select"]["columns"][0]["Expr"]["expr"]
            queries = [source, "SELECT " + ast_sql(expr), output]
            # Integer division / decimal precision in constant folding is a
            # separate issue. Keep it out of optimized-result assertions here.
            if i >= len(pairs) or "/" not in pairs[i]:
                queries.append(folded)
            if dialect == "sqlite":
                values = [sqlite.execute(q).fetchone()[0] for q in queries]
                assert all(same_sqlite(values[0], v) for v in values[1:]), (queries, values)
            else:
                mysql_probes.append("SELECT " + ", ".join("(" + q.removeprefix("SELECT ") + ")" for q in queries) + ";")
        if dialect == "mysql":
            values = mysql("\n".join(mysql_probes))
            assert len(values) == len(sources)
            for source, row in zip(sources, values):
                assert len(row) in (3, 4) and all(v == row[0] for v in row[1:]), (source, row)
        print(f"{dialect}: 100 operator pairs + {len(sources) - 100} focused cases preserve engine values; "
              f"{len(sources) - 19} cases also checked after optimization")

    if args.mysql_command:
        for read, write, source in [
            ("mysql", "sqlite", "SELECT 1 | 2 & 4"),
            ("sqlite", "mysql", "SELECT 1 | 2 & 4"),
            ("sqlite", "mysql", "SELECT 1 || 2 * 3"),
            ("mysql", "sqlite", "SELECT 0 || 1 * 0"),
        ]:
            for optimize in [False, True]:
                output = cli("transpile", source, read, write, optimize)
                before = (sqlite.execute(source).fetchone()[0] if read == "sqlite" else mysql(source)[0][0])
                after = (sqlite.execute(output).fetchone()[0] if write == "sqlite" else mysql(output)[0][0])
                assert Decimal(str(before)) == Decimal(str(after)), (source, output, before, after)
        print("8 cross-dialect checks passed, including optimized output")
    print("All requested engine checks passed")


if __name__ == "__main__":
    main()
