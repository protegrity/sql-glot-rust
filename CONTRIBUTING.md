# Contributing to sqlglot-rust

Thank you for your interest in contributing to sqlglot-rust! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/<your-username>/sql-glot-rust.git
   cd sql-glot-rust
   ```
3. Create a branch for your changes:
   ```bash
   git checkout -b my-feature
   ```

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- Cargo (included with Rust)

## Building

```bash
cargo build
```

## Running Tests

```bash
cargo test
```

## Code Quality

Before submitting a pull request, ensure your code passes all checks:

```bash
cargo fmt        # Format code
cargo clippy     # Run linter
cargo test       # Run all tests
```

## Running Benchmarks

```bash
cargo bench
```

## Project Structure

- `src/ast/` — AST node definitions
- `src/parser/` — SQL tokenizer and parser
- `src/generator/` — SQL code generation from AST
- `src/dialects/` — Dialect-specific parsing and generation rules
- `src/optimizer/` — Query optimization passes
- `src/tokens/` — Token definitions and tokenizer
- `src/errors/` — Error types
- `tests/` — Integration tests
- `benches/` — Benchmarks

## Conventions

- Use `thiserror` for error types
- Use `serde` for AST serialization
- Use `#[must_use]` on pure functions returning values
- Write unit tests alongside modules
- Follow [Rust API guidelines](https://rust-lang.github.io/api-guidelines/)

## Submitting Changes

1. Commit your changes with a clear, descriptive commit message
2. Push your branch to your fork
3. Open a pull request against the `master` branch
4. Describe your changes and the problem they solve in the PR description

## Reporting Issues

If you find a bug or have a feature request, please open an issue on GitHub with:
- A clear description of the problem or feature
- Steps to reproduce (for bugs)
- Expected vs actual behavior (for bugs)
- SQL examples if applicable

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
