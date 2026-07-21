use std::ffi::{CStr, CString};

use sqlglot_rust::ffi::{sqlglot_free, sqlglot_generate_pretty, sqlglot_parse};

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
