#!/usr/bin/env python3

import argparse
import json
import subprocess
import sys
from pathlib import Path


DIALECTS = [
    "",
    "athena",
    "bigquery",
    "clickhouse",
    "databricks",
    "doris",
    "dremio",
    "drill",
    "druid",
    "duckdb",
    "exasol",
    "fabric",
    "hive",
    "materialize",
    "mysql",
    "oracle",
    "postgres",
    "presto",
    "prql",
    "redshift",
    "risingwave",
    "singlestore",
    "snowflake",
    "spark",
    "sqlite",
    "starrocks",
    "tableau",
    "teradata",
    "trino",
    "tsql",
]

GRAMMAR_CASES = {
    "": [
        "DECIMAL(38, 9)",
        "CHARACTER VARYING(255)",
        "DOUBLE PRECISION",
        "TIMESTAMP(6) WITH TIME ZONE",
        "TIMESTAMP WITHOUT TIME ZONE",
        "TIME WITH TIME ZONE",
        "INTERVAL YEAR TO MONTH",
        "ARRAY<MAP<VARCHAR, DECIMAL(38, 9)>>",
        "STRUCT<id INT, values ARRAY<VARCHAR>>",
        "app.custom_type",
        '"custom type"',
    ],
    "bigquery": [
        "ARRAY<STRUCT<id INT64, payload JSON>>",
        "NUMERIC(38, 9)",
        "BIGNUMERIC(76, 38)",
        "RANGE<DATE>",
        "custom_type",
    ],
    "clickhouse": [
        "Nullable(LowCardinality(String))",
        "Tuple(a Int32, b Array(Nullable(UInt64)))",
        "Enum8('open' = 1, 'closed' = 2)",
        "AggregateFunction(sum, UInt64)",
        "SimpleAggregateFunction(sum, UInt64)",
        "DateTime64(6, 'UTC')",
        "Decimal256(76, 20)",
        "JSON(payload String, SKIP metadata)",
    ],
    "duckdb": [
        "STRUCT(id BIGINT, tags VARCHAR[])",
        "MAP(VARCHAR, INTEGER)",
        "UNION(number INTEGER, text VARCHAR)",
        "TIMESTAMP_NS",
        "custom_type[]",
    ],
    "oracle": [
        "VARCHAR2(255 CHAR)",
        "TIMESTAMP(9) WITH LOCAL TIME ZONE",
        "INTERVAL DAY(3) TO SECOND(6)",
        "APP.CUSTOM_TYPE",
    ],
    "postgres": [
        "NUMERIC(38, 9)[]",
        "TIMESTAMP(6) WITH TIME ZONE",
        "BIT VARYING(128)",
        "INT ARRAY[3]",
        "INTEGER[][]",
        "app.custom_type",
        "app.custom_type[][]",
        'app."custom type"',
    ],
    "snowflake": [
        "VECTOR(FLOAT, 768)",
        "NUMBER(38, 0)",
        "TIMESTAMP_TZ(9)",
        "FILE",
    ],
    "trino": [
        "ROW(id BIGINT, payload MAP(VARCHAR, ARRAY(DECIMAL(38, 9))))",
        "TIMESTAMP(6) WITH TIME ZONE",
        "IPADDRESS",
    ],
    "tsql": [
        "NVARCHAR(MAX)",
        "DATETIMEOFFSET(7)",
        "DECIMAL(38, 9)",
        "TIMESTAMP",
        "[custom type]",
    ],
}


def arguments():
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    for name, help_text in (
        ("generate", "generate the Python SQLGlot parity fixture"),
        ("verify", "verify the committed parity fixture"),
    ):
        command = commands.add_parser(name, help=help_text)
        command.add_argument(
            "--sqlglot",
            required=True,
            type=Path,
            help="path to the pinned Python SQLGlot checkout",
        )
        command.add_argument(
            "--fixture",
            required=True,
            type=Path,
            help="path to the parity JSON Lines fixture",
        )
    return parser.parse_args()


def expression_kind(expression, exp):
    if isinstance(expression, exp.DataType):
        if isinstance(expression.this, exp.DType):
            return expression.this.name
        if isinstance(expression.this, exp.Interval):
            return "INTERVAL"
        return "USERDEFINED"
    if isinstance(expression, (exp.PseudoType, exp.ObjectIdentifier)):
        return "USERDEFINED"
    return None


def generate_fixture(sqlglot):
    sys.path.insert(0, str(sqlglot))
    from sqlglot import Dialect, exp

    cases = []
    for name in DIALECTS:
        dialect = Dialect.get_or_raise(name)
        spellings = [
            spelling
            for spelling, token in dialect.tokenizer_class.KEYWORDS.items()
            if token in dialect.parser_class.TYPE_TOKENS
        ]
        spellings.extend(GRAMMAR_CASES.get(name, []))

        for sql in sorted(set(spellings)):
            try:
                expression = exp.DataType.from_str(
                    sql,
                    dialect=dialect,
                    udt=dialect.SUPPORTS_USER_DEFINED_TYPES,
                )
                expected = expression_kind(expression, exp)
            except Exception:
                expected = None

            cases.append(
                {
                    "dialect": name or "ansi",
                    "sql": sql,
                    "expected": expected,
                }
            )

    commit = subprocess.check_output(
        ["git", "-C", sqlglot, "rev-parse", "HEAD"],
        text=True,
    ).strip()
    records = [
        {
            "source": "tobymao/sqlglot",
            "commit": commit,
            "cases": len(cases),
        }
    ]
    records.extend(cases)
    return "".join(
        f"{json.dumps(record, separators=(',', ':'), ensure_ascii=False)}\n"
        for record in records
    )


def main():
    args = arguments()
    fixture = generate_fixture(args.sqlglot)

    if args.command == "generate":
        args.fixture.parent.mkdir(parents=True, exist_ok=True)
        args.fixture.write_text(fixture)
    elif not args.fixture.exists() or args.fixture.read_text() != fixture:
        raise SystemExit(
            f"{args.fixture} does not match the generated Python SQLGlot parity fixture"
        )


if __name__ == "__main__":
    main()
