use std::ffi::{CStr, CString};

use sqlglot_rust::ffi::{
    sqlglot_build_scope, sqlglot_free, sqlglot_generate_pretty, sqlglot_lineage, sqlglot_parse,
    sqlglot_qualify_columns,
};

fn read_json(pointer: *mut std::os::raw::c_char) -> serde_json::Value {
    assert!(!pointer.is_null());
    let value = unsafe { CStr::from_ptr(pointer) }.to_str().unwrap();
    let parsed = serde_json::from_str(value).unwrap();
    unsafe { sqlglot_free(pointer) };
    parsed
}

#[test]
fn pretty_generation_is_available_through_the_ffi() {
    let sql = CString::new("SELECT `select`, id FROM `events` WHERE active = true").unwrap();
    let mysql = CString::new("mysql").unwrap();
    let tsql = CString::new("tsql").unwrap();

    let ast_json = unsafe { sqlglot_parse(sql.as_ptr(), mysql.as_ptr()) };
    assert!(!ast_json.is_null());

    let generated = unsafe { sqlglot_generate_pretty(ast_json, tsql.as_ptr()) };
    assert!(!generated.is_null());

    let output = unsafe { CStr::from_ptr(generated) }.to_str().unwrap();
    assert!(output.contains("SELECT\n"));
    assert!(output.contains("[select]"));
    assert!(output.contains("[events]"));

    unsafe {
        sqlglot_free(generated);
        sqlglot_free(ast_json);
    }
}

#[test]
fn semantic_analysis_is_available_through_the_ffi() {
    let sql = CString::new(
        "WITH active_cards AS (SELECT id, project_id FROM cards) \
         SELECT projects.id FROM projects \
         JOIN active_cards ON active_cards.project_id = projects.id",
    )
    .unwrap();
    let postgres = CString::new("postgres").unwrap();
    let schema = CString::new(
        r#"{
          "tables": [
            {
              "path": ["cards"],
              "columns": [
                {"name": "id", "data_type": "BIGINT"},
                {"name": "project_id", "data_type": "DECIMAL(18, 4)"}
              ]
            },
            {
              "path": ["projects"],
              "columns": [
                {"name": "id", "data_type": "BIGINT"}
              ]
            }
          ]
        }"#,
    )
    .unwrap();

    let ast = unsafe { sqlglot_parse(sql.as_ptr(), postgres.as_ptr()) };
    let qualified = unsafe { sqlglot_qualify_columns(ast, schema.as_ptr(), postgres.as_ptr()) };
    let qualified_json = read_json(qualified);
    assert_eq!(
        qualified_json["Select"]["columns"][0]["Expr"]["expr"]["Column"]["table"],
        "projects"
    );

    let qualified = CString::new(serde_json::to_string(&qualified_json).unwrap()).unwrap();
    let scope = read_json(unsafe { sqlglot_build_scope(qualified.as_ptr()) });
    assert_eq!(scope["sources"]["active_cards"]["kind"], "scope");
    assert!(scope["sources"]["active_cards"]["scope"].is_object());
    assert_eq!(scope["cte_scopes"][0]["scope_type"], "cte");

    let column = CString::new("id").unwrap();
    let config = CString::new(r#"{"dialect":"postgres"}"#).unwrap();
    let graph = read_json(unsafe {
        sqlglot_lineage(
            column.as_ptr(),
            qualified.as_ptr(),
            schema.as_ptr(),
            config.as_ptr(),
        )
    });
    assert_eq!(graph["node"]["name"], "id");
    assert_eq!(graph["node"]["downstream"][0]["source_name"], "projects");

    unsafe { sqlglot_free(ast) };
}

#[test]
fn schema_paths_do_not_split_dots_inside_identifiers() {
    let sql = CString::new(r#"SELECT id FROM "events.v2""#).unwrap();
    let postgres = CString::new("postgres").unwrap();
    let schema = CString::new(
        r#"{
          "tables": [
            {
              "path": ["events.v2"],
              "columns": [
                {"name": "id", "data_type": "TIMESTAMP(6) WITH TIME ZONE"}
              ]
            }
          ]
        }"#,
    )
    .unwrap();

    let ast = unsafe { sqlglot_parse(sql.as_ptr(), postgres.as_ptr()) };
    let qualified = unsafe { sqlglot_qualify_columns(ast, schema.as_ptr(), postgres.as_ptr()) };

    assert!(!qualified.is_null());
    let qualified_json = read_json(qualified);
    assert_eq!(
        qualified_json["Select"]["columns"][0]["Expr"]["expr"]["Column"]["table"],
        "events.v2"
    );

    unsafe { sqlglot_free(ast) };
}
