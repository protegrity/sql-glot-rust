//! C FFI bindings for sqlglot-rust.
//!
//! This module exposes a C-compatible API so the library can be consumed
//! from C, C++, or any language that supports the C ABI.
//!
//! # Memory management
//!
//! Every `*mut c_char` returned by a function in this module **must** be freed
//! by calling [`sqlglot_free`]. Failing to do so will leak memory.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use crate::dialects::Dialect;

// ── helpers ──────────────────────────────────────────────────────────────

/// Convert a nullable C string to an `Option<&str>`.
/// Returns `None` when the pointer is null or the bytes are not valid UTF-8.
unsafe fn cstr_to_option(p: *const c_char) -> Option<&'static str> {
    if p.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(p) }.to_str().ok()
}

/// Resolve a C dialect string to a `Dialect` enum, falling back to `Ansi`.
fn resolve_dialect(name: Option<&str>) -> Dialect {
    name.and_then(Dialect::from_str).unwrap_or(Dialect::Ansi)
}

/// Return a C-owned string, or null on encoding failure.
fn to_c_string(s: String) -> *mut c_char {
    CString::new(s)
        .map(CString::into_raw)
        .unwrap_or(ptr::null_mut())
}

// ── public C API ─────────────────────────────────────────────────────────

/// Parse a SQL string and return its AST serialised as JSON.
///
/// * `sql`     – null-terminated SQL string (required).
/// * `dialect` – null-terminated dialect name, e.g. `"postgres"`. Pass `NULL`
///               for ANSI SQL.
///
/// Returns a heap-allocated JSON string on success, or `NULL` on failure.
/// The caller **must** free a non-null return value with [`sqlglot_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sqlglot_parse(sql: *const c_char, dialect: *const c_char) -> *mut c_char {
    let sql_str = match unsafe { cstr_to_option(sql) } {
        Some(s) => s,
        None => return ptr::null_mut(),
    };
    let dialect_enum = resolve_dialect(unsafe { cstr_to_option(dialect) });

    match crate::parse(sql_str, dialect_enum) {
        Ok(ast) => match serde_json::to_string(&ast) {
            Ok(json) => to_c_string(json),
            Err(_) => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// Transpile a single SQL statement from one dialect to another.
///
/// * `sql`          – null-terminated SQL string (required).
/// * `from_dialect` – source dialect name, or `NULL` for ANSI.
/// * `to_dialect`   – target dialect name, or `NULL` for ANSI.
///
/// Returns a heap-allocated SQL string on success, or `NULL` on failure.
/// The caller **must** free a non-null return value with [`sqlglot_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sqlglot_transpile(
    sql: *const c_char,
    from_dialect: *const c_char,
    to_dialect: *const c_char,
) -> *mut c_char {
    let sql_str = match unsafe { cstr_to_option(sql) } {
        Some(s) => s,
        None => return ptr::null_mut(),
    };
    let from = resolve_dialect(unsafe { cstr_to_option(from_dialect) });
    let to = resolve_dialect(unsafe { cstr_to_option(to_dialect) });

    match crate::transpile(sql_str, from, to) {
        Ok(result) => to_c_string(result),
        Err(_) => ptr::null_mut(),
    }
}

/// Generate SQL from a JSON-serialised AST for the given dialect.
///
/// * `ast_json` – null-terminated JSON string of a serialised `Statement`.
/// * `dialect`  – target dialect name, or `NULL` for ANSI.
///
/// Returns a heap-allocated SQL string on success, or `NULL` on failure.
/// The caller **must** free a non-null return value with [`sqlglot_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sqlglot_generate(
    ast_json: *const c_char,
    dialect: *const c_char,
) -> *mut c_char {
    let json_str = match unsafe { cstr_to_option(ast_json) } {
        Some(s) => s,
        None => return ptr::null_mut(),
    };
    let dialect_enum = resolve_dialect(unsafe { cstr_to_option(dialect) });

    match serde_json::from_str::<crate::ast::Statement>(json_str) {
        Ok(ast) => to_c_string(crate::generate(&ast, dialect_enum)),
        Err(_) => ptr::null_mut(),
    }
}

/// Generate pretty-printed SQL from a JSON-serialised AST for the given dialect.
///
/// * `ast_json` – null-terminated JSON string of a serialised `Statement`.
/// * `dialect`  – target dialect name, or `NULL` for ANSI.
///
/// Returns a heap-allocated SQL string on success, or `NULL` on failure.
/// The caller **must** free a non-null return value with [`sqlglot_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sqlglot_generate_pretty(
    ast_json: *const c_char,
    dialect: *const c_char,
) -> *mut c_char {
    let json_str = match unsafe { cstr_to_option(ast_json) } {
        Some(s) => s,
        None => return ptr::null_mut(),
    };
    let dialect_enum = resolve_dialect(unsafe { cstr_to_option(dialect) });

    match serde_json::from_str::<crate::ast::Statement>(json_str) {
        Ok(ast) => to_c_string(crate::generate_pretty(&ast, dialect_enum)),
        Err(_) => ptr::null_mut(),
    }
}

/// Return the library version as a static null-terminated string.
///
/// The returned pointer **must not** be freed — it points to static memory.
#[unsafe(no_mangle)]
pub extern "C" fn sqlglot_version() -> *const c_char {
    // The trailing \0 makes this a valid C string.
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

/// Free a string previously returned by any `sqlglot_*` function.
///
/// Passing `NULL` is safe and results in a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sqlglot_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}
