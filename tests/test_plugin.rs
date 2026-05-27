use sqlglot_rust::Dialect;
/// Tests for the Custom Dialect Plugin System (PQO-199).
///
/// Verifies:
/// - DialectPlugin trait implementation
/// - DialectRegistry registration and lookup
/// - DialectRef built-in and custom variants
/// - Plugin-aware transpile pipeline (transpile_ext)
/// - Custom function and type mapping
/// - Custom ILIKE support flag
/// - Custom quote style
/// - Full statement-level transform hooks
/// - resolve_dialect helper
use sqlglot_rust::ast::{DataType, Expr, QuoteStyle, Statement};
use sqlglot_rust::dialects::plugin::{
    DialectPlugin, DialectRef, DialectRegistry, register_dialect, resolve_dialect, transpile_ext,
    transpile_statements_ext,
};

// ═════════════════════════════════════════════════════════════════════════════
// Test dialect implementations
// ═════════════════════════════════════════════════════════════════════════════

/// A minimal custom dialect that only overrides function names.
struct FuncDialect;

impl DialectPlugin for FuncDialect {
    fn name(&self) -> &str {
        "funcdialect"
    }

    fn map_function_name(&self, name: &str) -> Option<String> {
        match name.to_uppercase().as_str() {
            "NOW" => Some("GET_CURRENT_TS".to_string()),
            "LENGTH" => Some("CHAR_COUNT".to_string()),
            _ => None,
        }
    }
}

/// A custom dialect that overrides data type mapping.
struct TypeDialect;

impl DialectPlugin for TypeDialect {
    fn name(&self) -> &str {
        "typedialect"
    }

    fn map_data_type(&self, dt: &DataType) -> Option<DataType> {
        match dt {
            DataType::Text => Some(DataType::Varchar(Some(65535))),
            DataType::Int => Some(DataType::BigInt),
            _ => None,
        }
    }
}

/// A custom dialect that uses backtick quoting and supports ILIKE.
struct QuoteDialect;

impl DialectPlugin for QuoteDialect {
    fn name(&self) -> &str {
        "quotedialect"
    }

    fn quote_style(&self) -> Option<QuoteStyle> {
        Some(QuoteStyle::Backtick)
    }

    fn supports_ilike(&self) -> Option<bool> {
        Some(true)
    }
}

/// A custom dialect that provides expression-level transforms.
struct ExprDialect;

impl DialectPlugin for ExprDialect {
    fn name(&self) -> &str {
        "exprdialect"
    }

    fn transform_expr(&self, expr: &Expr) -> Option<Expr> {
        // Replace any Number literal "0" with "FALSE"
        if let Expr::Number(n) = expr
            && n == "0"
        {
            return Some(Expr::Boolean(false));
        }
        None
    }
}

/// A custom dialect that provides a full statement-level transform.
struct StmtDialect;

impl DialectPlugin for StmtDialect {
    fn name(&self) -> &str {
        "stmtdialect"
    }

    fn transform_statement(&self, statement: &Statement) -> Option<Statement> {
        // If the statement is a SELECT with a single column "x", add a WHERE TRUE
        if let Statement::Select(sel) = statement
            && sel.columns.len() == 1
            && sel.where_clause.is_none()
        {
            let mut new_sel = sel.clone();
            new_sel.where_clause = Some(Expr::Boolean(true));
            return Some(Statement::Select(new_sel));
        }
        None
    }
}

/// A custom dialect with both function and type mappings.
struct FullDialect;

impl DialectPlugin for FullDialect {
    fn name(&self) -> &str {
        "fulldialect"
    }

    fn quote_style(&self) -> Option<QuoteStyle> {
        Some(QuoteStyle::Bracket)
    }

    fn supports_ilike(&self) -> Option<bool> {
        Some(false)
    }

    fn map_function_name(&self, name: &str) -> Option<String> {
        match name.to_uppercase().as_str() {
            "COALESCE" => Some("NVL2".to_string()),
            _ => None,
        }
    }

    fn map_data_type(&self, dt: &DataType) -> Option<DataType> {
        match dt {
            DataType::Boolean => Some(DataType::Int),
            _ => None,
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Registry tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_register_and_lookup() {
    let registry = DialectRegistry::global();
    registry.register(FuncDialect);

    let plugin = registry.get("funcdialect");
    assert!(plugin.is_some());
    assert_eq!(plugin.unwrap().name(), "funcdialect");
}

#[test]
fn test_case_insensitive_lookup() {
    let registry = DialectRegistry::global();
    registry.register(FuncDialect);

    assert!(registry.get("FuncDialect").is_some());
    assert!(registry.get("FUNCDIALECT").is_some());
    assert!(registry.get("funcdialect").is_some());
}

#[test]
fn test_lookup_nonexistent() {
    let registry = DialectRegistry::global();
    assert!(registry.get("nonexistent_dialect_xyz").is_none());
}

#[test]
fn test_unregister() {
    let registry = DialectRegistry::global();

    // Use a unique name to avoid conflicts with other tests
    struct TempDialect;
    impl DialectPlugin for TempDialect {
        fn name(&self) -> &str {
            "tempdialect_unregister_test"
        }
    }

    registry.register(TempDialect);
    assert!(registry.get("tempdialect_unregister_test").is_some());

    let removed = registry.unregister("tempdialect_unregister_test");
    assert!(removed);
    assert!(registry.get("tempdialect_unregister_test").is_none());

    // Unregistering again returns false
    assert!(!registry.unregister("tempdialect_unregister_test"));
}

#[test]
fn test_registered_names() {
    let registry = DialectRegistry::global();

    struct NameTestDialect;
    impl DialectPlugin for NameTestDialect {
        fn name(&self) -> &str {
            "nametest_dialect"
        }
    }

    registry.register(NameTestDialect);
    let names = registry.registered_names();
    assert!(names.contains(&"nametest_dialect".to_string()));
}

#[test]
fn test_register_replaces_existing() {
    let registry = DialectRegistry::global();

    struct V1;
    impl DialectPlugin for V1 {
        fn name(&self) -> &str {
            "versioned_dialect"
        }
        fn map_function_name(&self, name: &str) -> Option<String> {
            if name == "F" {
                Some("V1".to_string())
            } else {
                None
            }
        }
    }
    struct V2;
    impl DialectPlugin for V2 {
        fn name(&self) -> &str {
            "versioned_dialect"
        }
        fn map_function_name(&self, name: &str) -> Option<String> {
            if name == "F" {
                Some("V2".to_string())
            } else {
                None
            }
        }
    }

    registry.register(V1);
    let p1 = registry.get("versioned_dialect").unwrap();
    assert_eq!(p1.map_function_name("F"), Some("V1".to_string()));

    registry.register(V2);
    let p2 = registry.get("versioned_dialect").unwrap();
    assert_eq!(p2.map_function_name("F"), Some("V2".to_string()));
}

// ═════════════════════════════════════════════════════════════════════════════
// DialectRef tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dialect_ref_builtin() {
    let dr = DialectRef::from(Dialect::Postgres);
    assert_eq!(dr.as_builtin(), Some(Dialect::Postgres));
    assert!(dr.as_plugin().is_none());
}

#[test]
fn test_dialect_ref_custom() {
    register_dialect(FuncDialect);
    let dr = DialectRef::custom("funcdialect");
    assert!(dr.as_builtin().is_none());
    assert!(dr.as_plugin().is_some());
}

#[test]
fn test_dialect_ref_display() {
    let builtin = DialectRef::from(Dialect::Mysql);
    assert_eq!(format!("{builtin}"), "MySQL");

    let custom = DialectRef::custom("mydb");
    assert_eq!(format!("{custom}"), "Custom(mydb)");
}

#[test]
fn test_dialect_ref_equality() {
    assert_eq!(
        DialectRef::from(Dialect::Postgres),
        DialectRef::from(Dialect::Postgres)
    );
    assert_ne!(
        DialectRef::from(Dialect::Postgres),
        DialectRef::from(Dialect::Mysql)
    );
    assert_eq!(DialectRef::custom("x"), DialectRef::custom("x"));
    assert_ne!(DialectRef::custom("x"), DialectRef::from(Dialect::Postgres));
}

// ═════════════════════════════════════════════════════════════════════════════
// Custom function mapping via transpile_ext
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_custom_function_mapping() {
    register_dialect(FuncDialect);

    let result = transpile_ext(
        "SELECT NOW()",
        &DialectRef::from(Dialect::Ansi),
        &DialectRef::custom("funcdialect"),
    )
    .unwrap();

    assert_eq!(result, "SELECT GET_CURRENT_TS()");
}

#[test]
fn test_custom_function_mapping_length() {
    register_dialect(FuncDialect);

    let result = transpile_ext(
        "SELECT LENGTH(name) FROM users",
        &DialectRef::from(Dialect::Ansi),
        &DialectRef::custom("funcdialect"),
    )
    .unwrap();

    assert_eq!(result, "SELECT CHAR_COUNT(name) FROM users");
}

#[test]
fn test_custom_function_passthrough() {
    register_dialect(FuncDialect);

    // Functions not in the custom map should pass through unchanged
    let result = transpile_ext(
        "SELECT UPPER(name) FROM users",
        &DialectRef::from(Dialect::Ansi),
        &DialectRef::custom("funcdialect"),
    )
    .unwrap();

    assert_eq!(result, "SELECT UPPER(name) FROM users");
}

// ═════════════════════════════════════════════════════════════════════════════
// Custom type mapping via transpile_ext
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_custom_type_mapping() {
    register_dialect(TypeDialect);

    let result = transpile_ext(
        "SELECT CAST(x AS TEXT)",
        &DialectRef::from(Dialect::Ansi),
        &DialectRef::custom("typedialect"),
    )
    .unwrap();

    assert_eq!(result, "SELECT CAST(x AS VARCHAR(65535))");
}

#[test]
fn test_custom_type_mapping_int_to_bigint() {
    register_dialect(TypeDialect);

    let result = transpile_ext(
        "SELECT CAST(x AS INT)",
        &DialectRef::from(Dialect::Ansi),
        &DialectRef::custom("typedialect"),
    )
    .unwrap();

    assert_eq!(result, "SELECT CAST(x AS BIGINT)");
}

// ═════════════════════════════════════════════════════════════════════════════
// Custom quote style
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_custom_quote_style() {
    register_dialect(QuoteDialect);

    let dr = DialectRef::custom("quotedialect");
    assert_eq!(dr.quote_style(), QuoteStyle::Backtick);
}

#[test]
fn test_custom_supports_ilike() {
    register_dialect(QuoteDialect);

    let dr = DialectRef::custom("quotedialect");
    assert!(dr.supports_ilike());
}

// ═════════════════════════════════════════════════════════════════════════════
// Built-in → built-in via transpile_ext (passthrough)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_builtin_to_builtin_transpile_ext() {
    let result = transpile_ext(
        "SELECT NOW()",
        &DialectRef::from(Dialect::Postgres),
        &DialectRef::from(Dialect::Tsql),
    )
    .unwrap();

    assert_eq!(result, "SELECT GETDATE()");
}

#[test]
fn test_builtin_identity_transpile_ext() {
    let sql = "SELECT a, b FROM t WHERE a > 1";
    let result = transpile_ext(
        sql,
        &DialectRef::from(Dialect::Ansi),
        &DialectRef::from(Dialect::Ansi),
    )
    .unwrap();

    assert_eq!(result, sql);
}

// ═════════════════════════════════════════════════════════════════════════════
// transpile_statements_ext
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_transpile_statements_ext() {
    register_dialect(FuncDialect);

    let result = transpile_statements_ext(
        "SELECT NOW(); SELECT LENGTH(x)",
        &DialectRef::from(Dialect::Ansi),
        &DialectRef::custom("funcdialect"),
    )
    .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0], "SELECT GET_CURRENT_TS()");
    assert_eq!(result[1], "SELECT CHAR_COUNT(x)");
}

// ═════════════════════════════════════════════════════════════════════════════
// Expression-level transform hook
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_custom_expr_transform() {
    register_dialect(ExprDialect);

    let result = transpile_ext(
        "SELECT 0 FROM t",
        &DialectRef::from(Dialect::Ansi),
        &DialectRef::custom("exprdialect"),
    )
    .unwrap();

    assert_eq!(result, "SELECT FALSE FROM t");
}

// ═════════════════════════════════════════════════════════════════════════════
// Statement-level transform hook
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_custom_statement_transform() {
    register_dialect(StmtDialect);

    let result = transpile_ext(
        "SELECT x FROM t",
        &DialectRef::from(Dialect::Ansi),
        &DialectRef::custom("stmtdialect"),
    )
    .unwrap();

    assert_eq!(result, "SELECT x FROM t WHERE TRUE");
}

#[test]
fn test_custom_statement_transform_multi_column_fallthrough() {
    register_dialect(StmtDialect);

    // More than one column → statement transform returns None → falls through
    let result = transpile_ext(
        "SELECT x, y FROM t",
        &DialectRef::from(Dialect::Ansi),
        &DialectRef::custom("stmtdialect"),
    )
    .unwrap();

    // Should not add WHERE TRUE (multi-column fallthrough)
    assert_eq!(result, "SELECT x, y FROM t");
}

// ═════════════════════════════════════════════════════════════════════════════
// Full-featured custom dialect
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_full_dialect_function_mapping() {
    register_dialect(FullDialect);

    let result = transpile_ext(
        "SELECT COALESCE(a, b) FROM t",
        &DialectRef::from(Dialect::Ansi),
        &DialectRef::custom("fulldialect"),
    )
    .unwrap();

    assert_eq!(result, "SELECT NVL2(a, b) FROM t");
}

#[test]
fn test_full_dialect_type_mapping() {
    register_dialect(FullDialect);

    let result = transpile_ext(
        "SELECT CAST(x AS BOOLEAN)",
        &DialectRef::from(Dialect::Ansi),
        &DialectRef::custom("fulldialect"),
    )
    .unwrap();

    assert_eq!(result, "SELECT CAST(x AS INT)");
}

// ═════════════════════════════════════════════════════════════════════════════
// resolve_dialect helper
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_resolve_builtin_dialect() {
    let dr = resolve_dialect("postgres");
    assert_eq!(dr, Some(DialectRef::from(Dialect::Postgres)));
}

#[test]
fn test_resolve_builtin_case_insensitive() {
    let dr = resolve_dialect("MYSQL");
    assert_eq!(dr, Some(DialectRef::from(Dialect::Mysql)));
}

#[test]
fn test_resolve_custom_dialect() {
    register_dialect(FuncDialect);
    let dr = resolve_dialect("funcdialect");
    assert_eq!(dr, Some(DialectRef::custom("funcdialect")));
}

#[test]
fn test_resolve_nonexistent() {
    let dr = resolve_dialect("no_such_dialect_123");
    assert!(dr.is_none());
}

// ═════════════════════════════════════════════════════════════════════════════
// register_dialect convenience function
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_register_dialect_convenience() {
    struct ConvDialect;
    impl DialectPlugin for ConvDialect {
        fn name(&self) -> &str {
            "convdialect"
        }
    }

    register_dialect(ConvDialect);
    assert!(DialectRegistry::global().get("convdialect").is_some());
}

// ═════════════════════════════════════════════════════════════════════════════
// DialectRef method tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dialect_ref_map_function_builtin() {
    let dr = DialectRef::from(Dialect::Tsql);
    assert_eq!(dr.map_function_name("NOW"), "GETDATE");
}

#[test]
fn test_dialect_ref_map_function_custom() {
    register_dialect(FuncDialect);
    let dr = DialectRef::custom("funcdialect");
    assert_eq!(dr.map_function_name("NOW"), "GET_CURRENT_TS");
    // Unmapped function returns original
    assert_eq!(dr.map_function_name("UPPER"), "UPPER");
}

#[test]
fn test_dialect_ref_map_data_type_builtin() {
    let dr = DialectRef::from(Dialect::BigQuery);
    assert_eq!(dr.map_data_type(&DataType::Text), DataType::String);
}

#[test]
fn test_dialect_ref_map_data_type_custom() {
    register_dialect(TypeDialect);
    let dr = DialectRef::custom("typedialect");
    assert_eq!(
        dr.map_data_type(&DataType::Text),
        DataType::Varchar(Some(65535))
    );
    // Unmapped type returns original
    assert_eq!(dr.map_data_type(&DataType::Double), DataType::Double);
}

#[test]
fn test_dialect_ref_supports_ilike_builtin() {
    // Postgres supports ILIKE
    assert!(DialectRef::from(Dialect::Postgres).supports_ilike());
    // MySQL does not
    assert!(!DialectRef::from(Dialect::Mysql).supports_ilike());
}

#[test]
fn test_dialect_ref_quote_style_builtin() {
    assert_eq!(
        DialectRef::from(Dialect::Mysql).quote_style(),
        QuoteStyle::Backtick
    );
    assert_eq!(
        DialectRef::from(Dialect::Postgres).quote_style(),
        QuoteStyle::DoubleQuote
    );
    assert_eq!(
        DialectRef::from(Dialect::Tsql).quote_style(),
        QuoteStyle::Bracket
    );
}

#[test]
fn test_dialect_ref_custom_nonexistent_defaults() {
    // Custom dialect that isn't registered falls back to defaults
    let dr = DialectRef::custom("does_not_exist_abc");
    assert_eq!(dr.quote_style(), QuoteStyle::DoubleQuote);
    assert!(!dr.supports_ilike());
    assert_eq!(dr.map_function_name("FOO"), "FOO");
    assert_eq!(dr.map_data_type(&DataType::Text), DataType::Text);
}
