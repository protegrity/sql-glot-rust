/**
 * sqlglot-rust C++ FFI example — RAII wrapper around the C API.
 *
 * Build (macOS):
 *   cargo build --release
 *   make ffi-header
 *   g++ -std=c++17 examples/ffi_example.cpp -Itarget/ffi/include -Ltarget/release -lsqlglot_rust -o ffi_example_cpp
 *   ./ffi_example_cpp
 *
 * Build (Linux):
 *   cargo build --release
 *   make ffi-header
 *   g++ -std=c++17 examples/ffi_example.cpp -Itarget/ffi/include -Ltarget/release -lsqlglot_rust -lpthread -ldl -lm -o ffi_example_cpp
 *   LD_LIBRARY_PATH=target/release ./ffi_example_cpp
 */

#include <cstdio>
#include <memory>
#include <optional>
#include <string>
#include <stdexcept>

extern "C" {
#include "sqlglot.h"
}

// ── RAII helper ──────────────────────────────────────────────────────────
// unique_ptr with a custom deleter so every sqlglot string is freed
// automatically when the SqlglotString goes out of scope.

struct SqlglotDeleter {
    void operator()(char *p) const noexcept { sqlglot_free(p); }
};

using SqlglotString = std::unique_ptr<char, SqlglotDeleter>;

// ── Convenience wrappers ─────────────────────────────────────────────────

/// Parse SQL and return the JSON AST, or std::nullopt on failure.
std::optional<std::string> parse(const char *sql, const char *dialect = nullptr) {
    SqlglotString result(sqlglot_parse(sql, dialect));
    if (!result) return std::nullopt;
    return std::string(result.get());
}

/// Transpile SQL between dialects, or std::nullopt on failure.
std::optional<std::string> transpile(const char *sql,
                                     const char *from_dialect,
                                     const char *to_dialect) {
    SqlglotString result(sqlglot_transpile(sql, from_dialect, to_dialect));
    if (!result) return std::nullopt;
    return std::string(result.get());
}

/// Generate SQL from a JSON AST, or std::nullopt on failure.
std::optional<std::string> generate(const char *ast_json, const char *dialect = nullptr) {
    SqlglotString result(sqlglot_generate(ast_json, dialect));
    if (!result) return std::nullopt;
    return std::string(result.get());
}

/// Generate formatted SQL from a JSON AST, or std::nullopt on failure.
std::optional<std::string> generate_pretty(const char *ast_json, const char *dialect = nullptr) {
    SqlglotString result(sqlglot_generate_pretty(ast_json, dialect));
    if (!result) return std::nullopt;
    return std::string(result.get());
}

// ── Main ────────────────────────────────────────────────────────────────

int main() {
    std::printf("sqlglot-rust version: %s\n\n", sqlglot_version());

    // --- Transpile examples ---
    struct Example {
        const char *sql;
        const char *from;
        const char *to;
        const char *label;
    };

    Example examples[] = {
        {"SELECT NOW()",                 "postgres",  "tsql",     "NOW → GETDATE"},
        {"SELECT IFNULL(a, b) FROM t",  "mysql",     "postgres", "IFNULL → COALESCE"},
        {"SELECT * FROM t LIMIT 5",     "mysql",     "tsql",     "LIMIT → TOP"},
    };

    for (const auto &ex : examples) {
        auto result = transpile(ex.sql, ex.from, ex.to);
        std::printf("[%s]\n  %s → %s\n  IN:  %s\n  OUT: %s\n\n",
                    ex.label, ex.from, ex.to, ex.sql,
                    result ? result->c_str() : "(error)");
    }

    // --- Parse → re-generate round-trip ---
    const char *input = "SELECT a, b FROM users WHERE active = true";
    auto json = parse(input, "ansi");
    if (json) {
        auto sql = generate(json->c_str(), "snowflake");
        auto pretty = generate_pretty(json->c_str(), "snowflake");
        std::printf("Round-trip through JSON AST:\n"
                    "  Original:  %s\n"
                    "  Generated: %s\n"
                    "  Pretty:\n%s\n",
                    input, sql ? sql->c_str() : "(error)",
                    pretty ? pretty->c_str() : "(error)");
    }

    return 0;
}
