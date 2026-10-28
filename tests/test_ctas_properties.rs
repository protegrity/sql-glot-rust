use sqlglot_rust::ast::Statement;
use sqlglot_rust::dialects::Dialect;
use sqlglot_rust::generator::generate;
use sqlglot_rust::optimizer::lineage::{LineageConfig, lineage};
use sqlglot_rust::parser::parse;
use sqlglot_rust::schema::MappingSchema;

fn assert_ctas(sql: &str, query: &str, dialect: Dialect) -> Statement {
    let statement = parse(sql, dialect).unwrap();
    let Statement::CreateTable(table) = &statement else {
        panic!("expected CREATE TABLE, got {statement:?}");
    };
    assert_eq!(
        table.as_select.as_deref(),
        Some(&parse(query, dialect).unwrap())
    );
    let generated = generate(&statement, dialect);
    assert_eq!(parse(&generated, dialect).unwrap(), statement);
    statement
}

fn assert_group_by_modifier_keeps_following_clauses(modifier: &str, dialect: Dialect) {
    use sqlglot_rust::ast::{BinaryOperator, Expr, OrderByItem, QuoteStyle};

    let sql =
        format!("SELECT id FROM t GROUP BY id WITH {modifier} HAVING id > 0 ORDER BY id DESC");
    let Statement::Select(query) = parse(&sql, dialect).unwrap() else {
        panic!("expected SELECT");
    };
    let column = Expr::Column {
        table: None,
        name: "id".into(),
        quote_style: QuoteStyle::None,
        table_quote_style: QuoteStyle::None,
    };
    assert_eq!(query.group_by, vec![column.clone()]);
    assert_eq!(
        query.having,
        Some(Expr::BinaryOp {
            left: Box::new(column.clone()),
            op: BinaryOperator::Gt,
            right: Box::new(Expr::Number("0".into())),
        })
    );
    assert_eq!(
        query.order_by,
        vec![OrderByItem {
            expr: column,
            ascending: false,
            nulls_first: None,
        }]
    );
}

mod ansi {
    use super::*;

    // Parser constructors

    #[test]
    fn constructors_expose_ctas_query() {
        use sqlglot_rust::parser::Parser;

        let sql = "CREATE TABLE x AS (SELECT 1 AS id) WITH NO DATA";
        for mut parser in [
            Parser::new(sql).unwrap(),
            Parser::new_with_comments(sql).unwrap(),
            Parser::new_with_bracket_identifiers(sql, true).unwrap(),
            Parser::new_with_comments_and_bracket_identifiers(sql, true).unwrap(),
        ] {
            let Statement::CreateTable(table) = parser.parse_statement().unwrap() else {
                panic!("expected CREATE TABLE");
            };
            assert!(table.as_select.is_some());
            assert_eq!(table.with_data, Some(false));
        }
    }

    // Population clauses

    #[test]
    fn population_comments_survive_query_parentheses() {
        for query in ["SELECT 1 AS id", "SELECT 1 AS id UNION ALL SELECT 2 AS id"] {
            let sql = format!("CREATE TABLE x AS ({query}) WITH NO /* population */ DATA");
            let statement = sqlglot_rust::parse_with_comments(&sql, Dialect::Ansi).unwrap();
            assert!(
                matches!(&statement, Statement::CreateTable(table) if table.with_data == Some(false))
            );
            let generated = generate(&statement, Dialect::Ansi);
            assert!(generated.contains("/* population */"));
            assert_eq!(
                sqlglot_rust::parse_with_comments(&generated, Dialect::Ansi).unwrap(),
                statement
            );
        }
    }

    #[test]
    fn population_clause_generation_keeps_query_parentheses() {
        let sql = "CREATE TABLE x AS (SELECT id FROM t) WITH DATA";
        let statement = assert_ctas(sql, "SELECT id FROM t", Dialect::Ansi);
        assert_eq!(generate(&statement, Dialect::Ansi), sql);
    }
}

mod athena {
    use super::*;
    use sqlglot_rust::ast::Expr;

    // Defaults

    #[test]
    fn explicit_defaults_remain_explicit() {
        let required =
            "table_type = 'ICEBERG', is_external = false, location = 's3://example-bucket/output'";
        let query = "SELECT id FROM t";
        let implicit = assert_ctas(
            &format!("CREATE TABLE x WITH ({required}) AS {query}"),
            query,
            Dialect::Athena,
        );
        let explicit = assert_ctas(
            &format!(
                "CREATE TABLE x WITH ({required}, format = 'PARQUET', write_compression = 'ZSTD') AS {query}"
            ),
            query,
            Dialect::Athena,
        );
        let (Statement::CreateTable(implicit), Statement::CreateTable(explicit)) =
            (implicit, explicit)
        else {
            panic!("expected CREATE TABLE");
        };
        assert_eq!(implicit.with_properties.len(), 3);
        assert_eq!(explicit.with_properties.len(), 5);
        assert_eq!(explicit.with_properties[3].name, "format");
        assert_eq!(explicit.with_properties[4].name, "write_compression");
    }

    // Query bodies

    #[test]
    fn properties_expose_query() {
        let statement = assert_ctas(
            "CREATE TABLE x WITH (format = 'PARQUET') AS SELECT user_id FROM t",
            "SELECT user_id FROM t",
            Dialect::Athena,
        );
        assert!(generate(&statement, Dialect::Athena).contains("format = 'PARQUET'"));
    }

    #[test]
    fn cte_prefixed_query_is_preserved() {
        assert_ctas(
            "CREATE TABLE x WITH (format = 'AVRO') AS WITH c AS (SELECT user_id FROM t) SELECT user_id FROM c",
            "WITH c AS (SELECT user_id FROM t) SELECT user_id FROM c",
            Dialect::Athena,
        );
    }

    #[test]
    fn union_body_is_preserved() {
        assert_ctas(
            "CREATE TABLE x WITH (format = 'ORC') AS SELECT user_id FROM t UNION ALL SELECT user_id FROM u",
            "SELECT user_id FROM t UNION ALL SELECT user_id FROM u",
            Dialect::Athena,
        );
    }

    #[test]
    fn iceberg_ctas_exposes_query_and_lineage() {
        let sql = "CREATE TABLE x WITH (\
            is_external = false, \
            table_type = 'ICEBERG', \
            location = 's3://example-bucket/output'\
        ) AS SELECT user_id, NTILE(2) OVER (ORDER BY user_id) AS shard_id FROM t";
        let statement = parse(sql, Dialect::Athena).unwrap();
        let Statement::CreateTable(table) = statement else {
            panic!("expected CREATE TABLE");
        };
        let query = table.as_select.unwrap();
        let graph = lineage(
            "user_id",
            &query,
            &MappingSchema::new(Dialect::Athena),
            &LineageConfig::new(Dialect::Athena),
        )
        .unwrap();
        assert_eq!(graph.node.name, "user_id");
        let bare_query = parse(
            "SELECT user_id, NTILE(2) OVER (ORDER BY user_id) AS shard_id FROM t",
            Dialect::Athena,
        )
        .unwrap();
        assert_eq!(*query, bare_query);
    }

    #[test]
    fn query_alias_is_normalized() {
        let query = "SELECT count(*) total FROM t";
        let statement = assert_ctas(
            &format!("CREATE TABLE x WITH (format = 'PARQUET') AS {query}"),
            query,
            Dialect::Athena,
        );
        assert!(generate(&statement, Dialect::Athena).contains("COUNT(*) AS total"));
    }

    // Property names and values

    #[test]
    fn quoted_property_names_roundtrip() {
        let statement = assert_ctas(
            "CREATE TABLE x WITH (\"format\" = 'TEXTFILE') AS SELECT user_id FROM t",
            "SELECT user_id FROM t",
            Dialect::Athena,
        );
        assert!(generate(&statement, Dialect::Athena).contains("\"format\" = 'TEXTFILE'"));
    }

    #[test]
    fn unicode_property_value_is_preserved() {
        let sql = "CREATE TABLE x WITH (external_location = 's3://example-bucket/\u{0442}\u{0435}\u{0441}\u{0442}/') AS SELECT user_id FROM t";
        let Statement::CreateTable(table) =
            assert_ctas(sql, "SELECT user_id FROM t", Dialect::Athena)
        else {
            panic!("expected CREATE TABLE");
        };
        assert_eq!(
            table.with_properties[0].value,
            Expr::StringLiteral("s3://example-bucket/\u{0442}\u{0435}\u{0441}\u{0442}/".into())
        );
    }

    #[test]
    fn query_keywords_in_property_values_are_preserved() {
        let Statement::CreateTable(table) = assert_ctas(
            "CREATE TABLE x WITH (external_location = 's3://example-bucket/AS SELECT/') AS SELECT id FROM t",
            "SELECT id FROM t",
            Dialect::Athena,
        ) else {
            panic!("expected CREATE TABLE");
        };
        assert_eq!(
            table.with_properties[0].value,
            Expr::StringLiteral("s3://example-bucket/AS SELECT/".into())
        );
    }

    // Population clauses

    #[test]
    fn population_clauses_preserve_explicit_and_omitted_values() {
        for properties in ["", " WITH (format = 'ORC')"] {
            for (suffix, expected) in [
                ("", None),
                (" WITH DATA", Some(true)),
                (" wItH nO dAtA", Some(false)),
            ] {
                let sql = format!("create table x{properties} as select 1 as id{suffix}");
                let statement = assert_ctas(&sql, "SELECT 1 AS id", Dialect::Athena);
                let Statement::CreateTable(table) = &statement else {
                    unreachable!();
                };
                assert_eq!(table.with_data, expected);
                assert!(
                    generate(&statement, Dialect::Athena).ends_with(&suffix.to_ascii_uppercase())
                );
            }
        }
    }

    #[test]
    fn parenthesized_query_preserves_population_clause() {
        let statement = assert_ctas(
            "CREATE TABLE x WITH (format = 'ORC') AS ((SELECT 1 AS id)) WITH NO DATA",
            "SELECT 1 AS id",
            Dialect::Athena,
        );
        assert!(generate(&statement, Dialect::Athena).ends_with(" WITH NO DATA"));
    }

    #[test]
    fn grouped_query_leaves_population_clause_for_ctas() {
        let statement = assert_ctas(
            "CREATE TABLE x WITH (format = 'ORC') AS SELECT id FROM t GROUP BY id WITH NO DATA",
            "SELECT id FROM t GROUP BY id",
            Dialect::Athena,
        );
        assert!(generate(&statement, Dialect::Athena).ends_with(" WITH NO DATA"));
    }

    // Syntax boundaries

    #[test]
    fn malformed_properties_return_command_or_error() {
        for sql in [
            "CREATE TABLE x WITH () AS SELECT user_id FROM t",
            "CREATE TABLE x WITH (format = 'ORC',) AS SELECT user_id FROM t",
            "CREATE TABLE x WITH (format = 'ORC') (id INT)",
            "CREATE TABLE x WITH (format = 'ORC') SELECT user_id FROM t",
            "CREATE TABLE x WITH (format = 'ORC')) AS SELECT user_id FROM t",
            "CREATE TABLE x WITH (format =) AS SELECT user_id FROM t",
            "CREATE TABLE x WITH (format = 'ORC' AS SELECT user_id FROM t",
        ] {
            assert!(!matches!(
                parse(sql, Dialect::Athena),
                Ok(Statement::CreateTable(_))
            ));
        }
    }

    #[test]
    fn malformed_data_clauses_return_command_or_error() {
        for suffix in [
            " WITH",
            " WITH NO",
            " WITH \"NO\" DATA",
            " WITH NO \"DATA\"",
            " WITH 'DATA'",
        ] {
            let sql = format!("CREATE TABLE x WITH (format = 'ORC') AS SELECT 1 AS id{suffix}");
            assert!(!matches!(
                parse(&sql, Dialect::Athena),
                Ok(Statement::CreateTable(_))
            ));
        }
    }

    // Comments and statement streams

    #[test]
    fn leading_comment_is_preserved() {
        let statement = sqlglot_rust::parse_with_comments(
            "/* destination */ CREATE TABLE x WITH (format = 'PARQUET') AS SELECT 1 AS id",
            Dialect::Athena,
        )
        .unwrap();
        let Statement::CreateTable(table) = &statement else {
            panic!("expected CREATE TABLE");
        };
        assert_eq!(table.with_properties.len(), 1);
        assert!(table.as_select.is_some());
        assert!(generate(&statement, Dialect::Athena).contains("destination"));
    }

    #[test]
    fn property_comments_are_preserved() {
        let sql = "CREATE TABLE x WITH /* options */ (\
            /* name */ format /* assignment */ = /* value */ 'PARQUET' /* separator */,\
            external_location = 's3://example-bucket/output/' /* end */\
        ) /* query */ AS SELECT 1 AS id";
        let statement = sqlglot_rust::parse_with_comments(sql, Dialect::Athena).unwrap();
        let Statement::CreateTable(table) = &statement else {
            panic!("expected CREATE TABLE");
        };
        assert_eq!(table.with_properties.len(), 2);
        assert!(table.as_select.is_some());
        let generated = generate(&statement, Dialect::Athena);
        for comment in [
            "options",
            "name",
            "assignment",
            "value",
            "separator",
            "end",
            "query",
        ] {
            assert!(generated.contains(&format!("/* {comment} */")));
        }
        assert_eq!(
            sqlglot_rust::parse_with_comments(&generated, Dialect::Athena).unwrap(),
            statement
        );
    }

    #[test]
    fn comments_between_ctas_clauses_are_preserved() {
        for (properties, suffix) in [
            (" /* table */ WITH (format = 'PARQUET')", ""),
            (" WITH (format = 'PARQUET')", " /* query */"),
            ("", " /* query */ WITH NO DATA"),
            ("", " WITH /* with */ DATA"),
            ("", " WITH /* with */ NO DATA"),
            ("", " WITH NO /* no */ DATA"),
            ("", " WITH NO DATA /* end */"),
            (" WITH (format = 'PARQUET')", " WITH DATA -- end"),
        ] {
            let sql = format!("/* leading */ CREATE TABLE x{properties} AS SELECT 1 AS id{suffix}");
            let statement = sqlglot_rust::parse_with_comments(&sql, Dialect::Athena).unwrap();
            let Statement::CreateTable(table) = &statement else {
                panic!("expected CREATE TABLE for {sql}");
            };
            assert!(table.as_select.is_some());
            let generated = generate(&statement, Dialect::Athena);
            for word in ["leading", "table", "query", "with", "no", "end"] {
                if sql.contains(&format!("/* {word} */")) || sql.contains(&format!("-- {word}")) {
                    assert!(generated.contains(word), "missing {word} in {generated}");
                }
            }
            assert_eq!(
                sqlglot_rust::parse_with_comments(&generated, Dialect::Athena).unwrap(),
                statement,
                "{generated}"
            );
        }
    }

    #[test]
    fn statement_stream_keeps_ctas_boundaries() {
        let statements = sqlglot_rust::parser::parse_statements(
            "CREATE TABLE x WITH (format = 'PARQUET') AS SELECT 1 AS id WITH NO DATA; SELECT 2 AS id",
            Dialect::Athena,
        )
        .unwrap();
        assert_eq!(statements.len(), 2);
        let Statement::CreateTable(table) = &statements[0] else {
            panic!("expected CREATE TABLE");
        };
        assert_eq!(table.with_data, Some(false));
        assert!(matches!(statements[1], Statement::Select(_)));
    }
}

mod clickhouse {
    use super::*;

    // GROUP BY modifiers

    #[test]
    fn group_by_modifiers_keep_following_clauses() {
        for modifier in ["CUBE", "TOTALS"] {
            assert_group_by_modifier_keeps_following_clauses(modifier, Dialect::ClickHouse);
        }
    }
}

mod mysql {
    use super::*;

    // GROUP BY modifiers

    #[test]
    fn rollup_keeps_following_clauses() {
        assert_group_by_modifier_keeps_following_clauses("ROLLUP", Dialect::Mysql);
    }
}

mod postgres {
    use super::*;

    // Storage parameters

    #[test]
    fn storage_parameter_exposes_query() {
        assert_ctas(
            "CREATE TABLE x WITH (fillfactor = 70) AS SELECT id FROM t WITH NO DATA",
            "SELECT id FROM t",
            Dialect::Postgres,
        );
    }
}

mod presto {
    use super::*;

    // Property names

    #[test]
    fn quoted_format_property_exposes_query() {
        assert_ctas(
            "CREATE TABLE x WITH (\"write.format.default\" = 'ORC') AS SELECT user_id FROM t",
            "SELECT user_id FROM t",
            Dialect::Presto,
        );
    }
}

mod teradata {
    use super::*;

    // Population clauses

    #[test]
    fn population_clause_generation_keeps_query_parentheses() {
        let sql = "CREATE TABLE x AS (SELECT id FROM t) WITH NO DATA";
        let statement = assert_ctas(sql, "SELECT id FROM t", Dialect::Teradata);
        assert_eq!(generate(&statement, Dialect::Teradata), sql);
    }
}

mod trino {
    use super::*;

    // Property expressions

    #[test]
    fn array_and_arithmetic_properties_roundtrip() {
        let statement = assert_ctas(
            "CREATE TABLE x WITH (partitioning = ARRAY['bucket(user_id, 8)', 'month(ts)'], max_commit_retry = 1 + 2) AS SELECT user_id, ts FROM t",
            "SELECT user_id, ts FROM t",
            Dialect::Trino,
        );
        let generated = generate(&statement, Dialect::Trino);
        assert!(generated.contains("ARRAY['bucket(user_id, 8)', 'month(ts)']"));
        assert!(generated.contains("max_commit_retry = 1 + 2"));
    }

    // Query bodies

    #[test]
    fn values_and_table_query_bodies_remain_commands() {
        for query in ["VALUES (1)", "TABLE source"] {
            let statement = parse(&format!("CREATE TABLE x AS {query}"), Dialect::Trino).unwrap();
            let Statement::CreateTable(table) = statement else {
                panic!("expected CREATE TABLE");
            };
            assert_eq!(
                table.as_select.as_deref(),
                Some(&parse(query, Dialect::Trino).unwrap())
            );
            assert!(table.with_properties.is_empty());
            assert_eq!(table.with_data, None);
        }
    }

    // Unsupported CTAS forms

    #[test]
    fn unsupported_ctas_returns_command() {
        for sql in [
            "CREATE OR REPLACE TABLE x WITH (format = 'PARQUET') AS SELECT id FROM t",
            "CREATE TABLE x WITH (format = 'PARQUET') AS TABLE source",
            "CREATE TABLE x AS (TABLE source) WITH NO DATA",
            "CREATE TABLE x WITH (format = 'PARQUET') AS SET value = 1",
        ] {
            assert!(matches!(
                parse(sql, Dialect::Trino).unwrap(),
                Statement::Command(_)
            ));
        }
    }
}

mod integration {
    use super::*;

    #[test]
    fn unmodeled_query_clauses_preserve_the_complete_ctas() {
        for (dialect, sql) in [
            (
                Dialect::Postgres,
                "CREATE TABLE x WITH (fillfactor = 70) AS SELECT DISTINCT ON (id) id, value FROM t ORDER BY id, value DESC",
            ),
            (
                Dialect::Trino,
                "CREATE TABLE x WITH (format = 'ORC') AS SELECT id FROM t ORDER BY id FETCH FIRST 1 ROWS WITH TIES",
            ),
            (
                Dialect::Trino,
                "CREATE TABLE x WITH (format = 'ORC') AS SELECT id FROM t TABLESAMPLE BERNOULLI (10)",
            ),
            (
                Dialect::Postgres,
                "CREATE TABLE x AS SELECT DISTINCT ON (id) id FROM t WITH NO DATA",
            ),
            (
                Dialect::DuckDb,
                "CREATE TABLE x WITH (format = 'ORC') AS SELECT * EXCLUDE (secret) FROM t",
            ),
            (
                Dialect::DuckDb,
                "CREATE TABLE x WITH (format = 'ORC') AS SELECT a FROM t UNION BY NAME SELECT b FROM u",
            ),
            (
                Dialect::Mysql,
                "CREATE TABLE x AS SELECT id FROM t GROUP BY id WITH ROLLUP",
            ),
        ] {
            let statement = parse(sql, dialect).unwrap();
            assert!(
                matches!(statement, Statement::Command(_)),
                "{sql}: {statement:?}"
            );
            assert_eq!(generate(&statement, dialect), sql);
            assert_eq!(sqlglot_rust::transpile(sql, dialect, dialect).unwrap(), sql);
        }
    }

    #[test]
    fn nested_unmodeled_clauses_preserve_the_complete_ctas() {
        for query in [
            "WITH c AS (SELECT DISTINCT ON (id) id FROM t) SELECT id FROM c",
            "SELECT id FROM (SELECT DISTINCT ON (id) id FROM t) AS c",
            "SELECT id FROM t UNION ALL SELECT DISTINCT ON (id) id FROM u",
            "SELECT id FROM t WHERE id IN (SELECT DISTINCT ON (id) id FROM u)",
        ] {
            let sql = format!("CREATE TABLE x WITH (fillfactor = 70) AS {query}");
            let statement = parse(&sql, Dialect::Postgres).unwrap();
            assert!(matches!(statement, Statement::Command(_)), "{sql}");
            assert_eq!(generate(&statement, Dialect::Postgres), sql);
        }
    }

    #[test]
    fn unmodeled_syntax_does_not_poison_the_next_ctas() {
        let sql = "CREATE TABLE x WITH (fillfactor = 70) AS SELECT DISTINCT ON (id) id FROM t; CREATE TABLE y WITH (fillfactor = 70) AS SELECT id FROM t WITH NO DATA";
        let statements = sqlglot_rust::parser::parse_statements(sql, Dialect::Postgres).unwrap();
        assert_eq!(statements.len(), 2);
        assert!(matches!(statements[0], Statement::Command(_)));
        assert!(
            matches!(&statements[1], Statement::CreateTable(table) if table.with_data == Some(false))
        );
    }

    #[test]
    fn ctas_query_changes_are_visible_in_ast_diff() {
        use sqlglot_rust::diff::{ChangeAction, diff};
        for (before_query, after_query) in [
            ("SELECT a FROM t", "SELECT b FROM t"),
            ("SELECT a FROM t", "SELECT a FROM other"),
            ("SELECT a FROM t WHERE tenant_id = 1", "SELECT a FROM t"),
            (
                "SELECT a FROM t UNION ALL SELECT a FROM u",
                "SELECT a FROM t UNION ALL SELECT b FROM u",
            ),
        ] {
            for properties in ["", " WITH (format = 'ORC')"] {
                let before = parse(
                    &format!("CREATE TABLE x{properties} AS {before_query}"),
                    Dialect::Athena,
                )
                .unwrap();
                let after = parse(
                    &format!("CREATE TABLE x{properties} AS {after_query}"),
                    Dialect::Athena,
                )
                .unwrap();
                assert!(
                    diff(&before, &after)
                        .iter()
                        .any(|change| matches!(change, ChangeAction::Update(_, _)))
                );
                assert!(diff(&before, &before).is_empty());
            }
        }
    }

    // Statement streams

    #[test]
    fn bare_ctas_leaves_next_with_statement_in_stream() {
        for name in ["c", "data", "no"] {
            let sql = format!(
                "CREATE TABLE x AS SELECT 1 AS id WITH {name} AS (SELECT 2 AS v) SELECT v FROM {name}"
            );
            let statements = sqlglot_rust::parser::parse_statements(&sql, Dialect::Ansi).unwrap();
            assert_eq!(statements.len(), 2, "{sql}");
            assert!(matches!(statements[0], Statement::CreateTable(_)));
            assert!(matches!(statements[1], Statement::Select(_)));
        }
    }

    // AST compatibility

    #[test]
    fn schema_only_create_table_has_no_query() {
        let Statement::CreateTable(table) =
            parse("CREATE TABLE x (id INT)", Dialect::Athena).unwrap()
        else {
            panic!("expected schema-only table");
        };
        assert!(table.as_select.is_none());
    }

    #[test]
    fn unrelated_create_keeps_generic_command() {
        assert!(matches!(
            parse("CREATE FUNCTION f RETURNS INT AS SELECT 1", Dialect::Athena).unwrap(),
            Statement::Command(_)
        ));
    }

    #[test]
    fn direct_ctas_lineage_still_requires_unwrapping() {
        let statement = parse("CREATE TABLE x AS SELECT user_id FROM t", Dialect::Athena).unwrap();
        assert!(
            lineage(
                "user_id",
                &statement,
                &MappingSchema::new(Dialect::Athena),
                &LineageConfig::new(Dialect::Athena)
            )
            .is_err()
        );
    }

    // Serialization

    #[test]
    fn older_serialized_tables_remain_readable() {
        let statement = parse("CREATE TABLE x AS SELECT user_id FROM t", Dialect::Athena).unwrap();
        let encoded = serde_json::to_value(&statement).unwrap();
        assert!(encoded["CreateTable"].get("with_properties").is_none());
        assert!(encoded["CreateTable"].get("with_data").is_none());
        assert_eq!(
            serde_json::from_value::<Statement>(encoded).unwrap(),
            statement
        );
    }

    #[test]
    fn properties_survive_json_roundtrip() {
        let statement = parse(
            "CREATE TABLE x WITH (format = 'PARQUET', write_compression = 'SNAPPY') AS SELECT id FROM t WITH NO DATA",
            Dialect::Athena,
        )
        .unwrap();
        let json = serde_json::to_string(&statement).unwrap();
        assert_eq!(serde_json::from_str::<Statement>(&json).unwrap(), statement);
    }

    // AST comparisons

    #[test]
    fn property_only_changes_are_visible_in_ast_diff() {
        use sqlglot_rust::diff::{ChangeAction, diff};

        let before = parse(
            "CREATE TABLE x WITH (format = 'ORC') AS SELECT id FROM t",
            Dialect::Athena,
        )
        .unwrap();
        let after = parse(
            "CREATE TABLE x WITH (format = 'AVRO') AS SELECT id FROM t",
            Dialect::Athena,
        )
        .unwrap();
        assert!(
            diff(&before, &after)
                .iter()
                .any(|change| matches!(change, ChangeAction::Update(_, _)))
        );
    }

    #[test]
    fn ctas_data_clause_changes_are_visible_in_ast_diff() {
        use sqlglot_rust::diff::{ChangeAction, diff};

        let before = parse(
            "CREATE TABLE x AS SELECT 1 AS id WITH DATA",
            Dialect::Athena,
        )
        .unwrap();
        let after = parse(
            "CREATE TABLE x AS SELECT 1 AS id WITH NO DATA",
            Dialect::Athena,
        )
        .unwrap();
        assert!(
            diff(&before, &after)
                .iter()
                .any(|change| matches!(change, ChangeAction::Update(_, _)))
        );
    }

    // Property expression traversal

    #[test]
    fn property_values_use_builtin_transforms() {
        use sqlglot_rust::ast::SelectItem;

        let mut statement = parse(
            "CREATE TABLE x WITH (bucketed_by = ARRAY['id'], bucket_count = 3) AS SELECT id FROM t",
            Dialect::Athena,
        )
        .unwrap();
        let Statement::Select(select) = parse("SELECT LENGTH('abc')", Dialect::Athena).unwrap()
        else {
            panic!("expected SELECT");
        };
        let SelectItem::Expr { expr, .. } = select.columns.into_iter().next().unwrap() else {
            panic!("expected expression");
        };
        let Statement::CreateTable(table) = &mut statement else {
            panic!("expected CREATE TABLE");
        };
        // Synthetic expression for AST traversal
        table.with_properties[1].value = expr;
        let transformed =
            sqlglot_rust::dialects::transform(&statement, Dialect::Athena, Dialect::Tsql);
        let output = generate(&transformed, Dialect::Tsql);
        assert!(output.contains("bucket_count = LEN('abc')"), "{output}");
    }

    #[test]
    fn property_values_use_plugin_transforms() {
        use sqlglot_rust::ast::Expr;
        use sqlglot_rust::dialects::plugin::{
            DialectPlugin, DialectRef, register_dialect, transpile_ext,
        };

        struct PropertyDialect;

        impl DialectPlugin for PropertyDialect {
            fn name(&self) -> &str {
                "property_test"
            }

            fn transform_expr(&self, expr: &Expr) -> Option<Expr> {
                if expr == &Expr::Number("8".into()) {
                    Some(Expr::Number("16".into()))
                } else {
                    None
                }
            }
        }

        let sql =
            "CREATE TABLE x WITH (bucketed_by = ARRAY['id'], bucket_count = 8) AS SELECT id FROM t";
        register_dialect(PropertyDialect);
        let target = DialectRef::custom("property_test");
        let output = transpile_ext(sql, &DialectRef::BuiltIn(Dialect::Athena), &target).unwrap();
        assert!(output.contains("bucket_count = 16"), "{output}");
    }
}
