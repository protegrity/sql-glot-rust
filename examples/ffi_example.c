/**
 * sqlglot-rust C FFI example — parse, transpile, and generate SQL.
 *
 * Build (macOS):
 *   cargo build --release
 *   cbindgen --config cbindgen.toml --crate sqlglot-rust --output target/ffi/include/sqlglot.h
 *   gcc examples/ffi_example.c -Itarget/ffi/include -Ltarget/release -lsqlglot_rust -o ffi_example
 *   ./ffi_example                           # macOS: dylib is found via -L
 *
 * Build (Linux):
 *   cargo build --release
 *   cbindgen --config cbindgen.toml --crate sqlglot-rust --output target/ffi/include/sqlglot.h
 *   gcc examples/ffi_example.c -Itarget/ffi/include -Ltarget/release -lsqlglot_rust -lpthread -ldl -lm -o ffi_example
 *   LD_LIBRARY_PATH=target/release ./ffi_example
 */

#include <stdio.h>
#include <stdlib.h>
#include "sqlglot.h"

int main(void) {
    /* ── Library version ──────────────────────────────────────────── */
    printf("sqlglot-rust version: %s\n\n", sqlglot_version());

    /* ── Transpile ────────────────────────────────────────────────── */
    const char *sql = "SELECT NOW(), IFNULL(a, b) FROM t LIMIT 10";
    printf("Input (MySQL):    %s\n", sql);

    char *pg = sqlglot_transpile(sql, "mysql", "postgres");
    if (pg) {
        printf("Output (Postgres): %s\n", pg);
        sqlglot_free(pg);
    }

    char *tsql = sqlglot_transpile(sql, "mysql", "tsql");
    if (tsql) {
        printf("Output (T-SQL):    %s\n\n", tsql);
        sqlglot_free(tsql);
    }

    /* ── Parse to JSON AST ────────────────────────────────────────── */
    const char *simple = "SELECT a, b FROM users WHERE active = true";
    char *json = sqlglot_parse(simple, "ansi");
    if (json) {
        printf("AST JSON (first 200 chars):\n  %.200s...\n\n", json);
    }

    /* ── Generate SQL back from the JSON AST ──────────────────────── */
    if (json) {
        char *regenerated = sqlglot_generate(json, "postgres");
        if (regenerated) {
            printf("Regenerated (Postgres): %s\n", regenerated);
            sqlglot_free(regenerated);
        }

        char *pretty = sqlglot_generate_pretty(json, "postgres");
        if (pretty) {
            printf("Pretty (Postgres):\n%s\n", pretty);
            sqlglot_free(pretty);
        }

        sqlglot_free(json);
    }

    /* ── Error handling: NULL on invalid SQL ──────────────────────── */
    char *bad = sqlglot_parse("NOT VALID SQL ???", "ansi");
    if (bad == NULL) {
        printf("\nGraceful NULL on parse error — as expected.\n");
    } else {
        sqlglot_free(bad);
    }

    return 0;
}
