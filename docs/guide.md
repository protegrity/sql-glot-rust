# sqlglot-rust Documentation

A SQL parser, optimizer, and transpiler library written in Rust, inspired by Python's [sqlglot](https://github.com/tobymao/sqlglot).

---

## Documents

| Guide | Description |
| --- | --- |
| **[Installation](installation.md)** | Add the crate, verify your setup, and understand imports |
| **[Developer Guide](developer-guide.md)** | Parsing, generating, transpiling, AST traversal, optimization, serialization — with full input/output examples |
| **[API Reference](reference.md)** | Complete type catalog, function signatures, dialect tables, operator enums, data types, and error variants |
| **[SQL Execution Engine](reference.md#sql-execution-engine)** | In-memory query execution against Rust data structures for testing and validation |
| **[Custom Dialect Plugins](developer-guide.md#custom-dialect-plugins)** | Register custom dialects at runtime with the `DialectPlugin` trait and `DialectRegistry` |
| **[C/C++ FFI Bindings](developer-guide.md#cc-ffi-bindings)** | Use sqlglot-rust from C, C++, or any language with C ABI support |
| **[Swift, Kotlin, and Python](uniffi.md)** | Integrate the UniFFI bindings on macOS, iOS, JVM, Android, and Python |
| **CLI** | Command-line interface for transpiling, parsing, and formatting SQL (see [README](../README.md#cli)) |
