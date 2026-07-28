use serde::Deserialize;
use sqlglot_rust::ast::DataTypeKind;
use sqlglot_rust::{Dialect, parse_data_type, parse_data_type_with_udt};

#[derive(Deserialize)]
struct FixtureHeader {
    source: String,
    commit: String,
    cases: usize,
}

#[derive(Deserialize)]
struct Case {
    dialect: String,
    sql: String,
    expected: Option<String>,
}

fn kind_name(kind: DataTypeKind) -> String {
    serde_json::to_value(kind)
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn matches_python_type_parsing_fixture() {
    let mut lines = include_str!("fixtures/python_type_parity.jsonl").lines();
    let header: FixtureHeader = serde_json::from_str(lines.next().unwrap()).unwrap();
    assert_eq!(header.source, "tobymao/sqlglot");
    assert_eq!(header.commit.len(), 40);
    assert_eq!(header.cases, 3_681);
    let cases: Vec<Case> = lines
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(cases.len(), header.cases);

    let mut failures = Vec::new();
    for case in cases {
        let dialect = Dialect::from_str(&case.dialect).unwrap();
        let actual = if dialect.supports_user_defined_types() {
            parse_data_type_with_udt(&case.sql, dialect)
        } else {
            parse_data_type(&case.sql, dialect)
        };
        match (actual, case.expected) {
            (Ok(data_type), Some(expected)) => {
                let actual = kind_name(data_type.kind());
                if actual != expected {
                    failures.push(format!(
                        "{}: {:?} parsed as {actual}, expected {expected}",
                        case.dialect, case.sql
                    ));
                }
            }
            (Err(_), None) => {}
            (Ok(data_type), None) => failures.push(format!(
                "{}: {:?} parsed as {}, expected an error",
                case.dialect,
                case.sql,
                kind_name(data_type.kind())
            )),
            (Err(error), Some(expected)) => failures.push(format!(
                "{}: {:?} failed with {error}, expected {expected}",
                case.dialect, case.sql
            )),
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn matches_python_user_defined_type_policy() {
    let supported = [
        Dialect::Ansi,
        Dialect::Athena,
        Dialect::Druid,
        Dialect::DuckDb,
        Dialect::Fabric,
        Dialect::Materialize,
        Dialect::Oracle,
        Dialect::Postgres,
        Dialect::Presto,
        Dialect::Prql,
        Dialect::RisingWave,
        Dialect::Sqlite,
        Dialect::Tableau,
        Dialect::Teradata,
        Dialect::Tsql,
    ];

    for dialect in Dialect::all() {
        assert_eq!(
            dialect.supports_user_defined_types(),
            supported.contains(dialect),
            "unexpected user-defined type policy for {dialect:?}"
        );
    }
}
