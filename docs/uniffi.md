# Swift, Kotlin, and Python Bindings (UniFFI)

Use sqlglot-rust from **Swift** (macOS, iOS), **Kotlin** (JVM, Android), and
**Python** through bindings generated with [UniFFI](https://mozilla.github.io/uniffi-rs/).

> **See also:** [Installation](installation.md) · [API Reference — UniFFI API](reference.md#uniffi-api-swift--kotlin--python) · [C/C++ FFI](developer-guide.md#cc-ffi-bindings)

---

## Table of Contents

- [What You Get](#what-you-get)
- [Platform Support](#platform-support)
- [Prerequisites](#prerequisites)
- [Step 1 — Build the Library and Generate Bindings](#step-1--build-the-library-and-generate-bindings)
- [Core Concepts](#core-concepts)
- [Python](#python)
- [Swift](#swift)
  - [macOS Command-Line Build](#macos-command-line-build)
  - [Swift Package with an XCFramework (macOS and iOS)](#swift-package-with-an-xcframework-macos-and-ios)
- [Kotlin](#kotlin)
  - [JVM with Gradle](#jvm-with-gradle)
  - [Android](#android)
- [Working with the AST](#working-with-the-ast)
- [Error Handling](#error-handling)
- [Memory and Threading](#memory-and-threading)
- [Troubleshooting](#troubleshooting)
- [Testing Your Integration](#testing-your-integration)

---

## What You Get

| Capability | Function / method |
| --- | --- |
| Parse one statement | `parse(sql, dialect)` → `SqlStatement` |
| Parse a script | `parseStatements(sql, dialect)` → list of `SqlStatement` |
| Render SQL | `statement.generate(dialect)` / `statement.generatePretty(dialect)` |
| Convert between dialects | `transpile(sql, readDialect, writeDialect)` |
| Read the full AST | `statement.toJson()` |
| Build a statement from an (edited) AST | `SqlStatement.fromJson(json)` |
| Library version | `version()` |

All 30 [dialects](reference.md#dialect-list) are available. Failures raise a
typed error — see [Error Handling](#error-handling).

The bindings are an **optional feature**. Regular Rust builds, crates.io
consumers, and the C API are not affected.

---

## Platform Support

| Platform | Artefact you ship | Status |
| --- | --- | --- |
| Python — macOS, Linux | `sqlglot_rust.py` + `libsqlglot_rust.{dylib,so}` | Tested in CI |
| Swift — macOS | `sqlglot_rust.swift` + static or dynamic library | Tested in CI |
| Swift — iOS | XCFramework + `sqlglot_rust.swift` | XCFramework build verified; not run on a device or simulator by this project |
| Kotlin — JVM (macOS, Linux) | `sqlglot_rust.kt` + JNA + native library | Tested in CI |
| Kotlin — Android | `sqlglot_rust.kt` + JNA AAR + `jniLibs/<abi>/libsqlglot_rust.so` | Not verified by this project |
| Windows (any language) | `sqlglot_rust.dll` | Not verified by this project |

"Not verified" recipes follow the standard UniFFI workflow but are not
exercised by this repository's tests. Validate them in your own pipeline.

---

## Prerequisites

| Tool | Needed for |
| --- | --- |
| Rust 1.85+ and Cargo | Always |
| Python 3.8+ | Python bindings |
| Xcode (or Swift toolchain on macOS) | Swift bindings |
| JDK 17+, Kotlin, [JNA](https://github.com/java-native-access/jna) 5.12+ | Kotlin bindings |
| Android NDK + [`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk) | Android |

The binding generator (`uniffi-bindgen`) is built from this repository; you do
not need to install UniFFI separately.

---

## Step 1 — Build the Library and Generate Bindings

From a checkout of this repository:

```bash
make uniffi-bindings
```

This builds the native library with the `uniffi` feature and generates
bindings for all three languages:

```text
target/release/libsqlglot_rust.dylib       # .so on Linux; the native library
target/uniffi/python/sqlglot_rust.py
target/uniffi/swift/sqlglot_rust.swift
target/uniffi/swift/sqlglot_rustFFI.h
target/uniffi/swift/sqlglot_rustFFI.modulemap
target/uniffi/kotlin/uniffi/sqlglot_rust/sqlglot_rust.kt
```

The equivalent manual commands are:

```bash
cargo build --release --features uniffi-bindgen --lib --bin uniffi-bindgen
target/release/uniffi-bindgen generate --library target/release/libsqlglot_rust.dylib \
    --language python --out-dir target/uniffi/python   # or swift / kotlin
```

> **Important:** always ship the generated bindings together with the native
> library **from the same build**. The bindings verify an API checksum when they
> load and refuse to run against a mismatched library.

---

## Core Concepts

- **`Dialect`** — an enum of the 30 supported dialects.
- **`SqlStatement`** — an immutable handle to a parsed statement. It is produced
  by `parse`, `parseStatements`, or `SqlStatement.fromJson`.
- **`generate` vs. `transpile`** — `generate` renders the AST exactly as parsed,
  only adjusting syntax such as identifier quoting. `transpile` additionally
  applies dialect rewrites (for example `LIMIT` → `TOP` for T-SQL). **Use
  `transpile` to convert SQL between dialects.**

  | Call | Result |
  | --- | --- |
  | `transpile("SELECT * FROM t LIMIT 10", MYSQL, TSQL)` | `SELECT TOP 10 * FROM t` |
  | `parse("SELECT * FROM t LIMIT 10", MYSQL).generate(TSQL)` | `SELECT * FROM t LIMIT 10` |

- **AST JSON** — `toJson()` returns the complete AST using the same JSON schema
  as the Rust `serde` types and the C API's `sqlglot_parse`. See
  [Working with the AST](#working-with-the-ast).

Names follow each language's conventions:

| Concept | Python | Swift | Kotlin |
| --- | --- | --- | --- |
| Parse a script | `parse_statements` | `parseStatements(sql:dialect:)` | `parseStatements` |
| Pretty output | `generate_pretty` | `generatePretty(dialect:)` | `generatePretty` |
| AST to JSON | `to_json()` | `toJson()` | `toJson()` |
| AST from JSON | `SqlStatement.from_json(s)` | `SqlStatement.fromJson(json:)` | `SqlStatement.fromJson(s)` |
| Dialect value | `Dialect.BIG_QUERY` | `.bigQuery` | `Dialect.BIG_QUERY` |
| Error type | `SqlglotError.Parser` | `SqlglotError.Parser` | `SqlglotException.Parser` |

---

## Python

The generated module loads `libsqlglot_rust` from **its own directory**, so
place the two files side by side. To vendor them into your application:

```text
myapp/
├── __init__.py
└── sqlglot/
    ├── __init__.py
    ├── sqlglot_rust.py            # from target/uniffi/python/
    └── libsqlglot_rust.dylib      # from target/release/ (.so on Linux)
```

```python
import json
from myapp.sqlglot import sqlglot_rust as sg

statement = sg.parse("SELECT a, b FROM t WHERE a > 1", sg.Dialect.POSTGRES)
print(statement.generate_pretty(sg.Dialect.POSTGRES))

print(sg.transpile("SELECT * FROM t LIMIT 10", sg.Dialect.MYSQL, sg.Dialect.TSQL))

try:
    sg.parse("SELECT 'abc", sg.Dialect.ANSI)
except sg.SqlglotError.Tokenizer as e:
    print(e.detail, e.position)
```

Output:

```text
SELECT
  a,
  b
FROM
  t
WHERE
  a > 1
SELECT TOP 10 * FROM t
Unterminated string literal 7
```

For a standalone script, putting `sqlglot_rust.py` and the library in the same
directory as the script and using `import sqlglot_rust` also works.

The module has no Python dependencies besides the standard library (`ctypes`).
The native library is platform-specific: build it on (or cross-compile for)
each OS and CPU architecture you distribute to.

---

## Swift

The Swift bindings consist of `sqlglot_rust.swift` plus a C module named
`sqlglot_rustFFI` (header + modulemap) that it imports.

### macOS Command-Line Build

For a quick start, or a command-line tool:

```bash
B=target/uniffi/swift
swiftc -module-name MyTool \
    -Xcc -fmodule-map-file=$B/sqlglot_rustFFI.modulemap -I $B \
    -L target/release -lsqlglot_rust \
    -Xlinker -rpath -Xlinker "$PWD/target/release" \
    $B/sqlglot_rust.swift main.swift -o mytool
```

`main.swift`:

```swift
let statement = try parse(sql: "SELECT a, b FROM t WHERE a > 1", dialect: .postgres)
print(try statement.generatePretty(dialect: .postgres))
print(try transpile(sql: "SELECT * FROM t LIMIT 10", readDialect: .mysql, writeDialect: .tsql))
```

### Swift Package with an XCFramework (macOS and iOS)

For apps, package the Rust code as an XCFramework and consume it from a Swift
Package.

**1. Build a static library per Apple target and assemble the XCFramework:**

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios x86_64-apple-darwin

make uniffi-bindings

for target in aarch64-apple-darwin x86_64-apple-darwin \
              aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
  cargo build --release --features uniffi --lib --target "$target"
done

OUT=target/uniffi/apple
mkdir -p "$OUT/macos" "$OUT/ios-sim" "$OUT/headers"
lipo -create target/{aarch64,x86_64}-apple-darwin/release/libsqlglot_rust.a \
     -output "$OUT/macos/libsqlglot_rust.a"
lipo -create target/{aarch64-apple-ios-sim,x86_64-apple-ios}/release/libsqlglot_rust.a \
     -output "$OUT/ios-sim/libsqlglot_rust.a"

# Inside an XCFramework the modulemap must be named module.modulemap.
cp target/uniffi/swift/sqlglot_rustFFI.h "$OUT/headers/"
cp target/uniffi/swift/sqlglot_rustFFI.modulemap "$OUT/headers/module.modulemap"

xcodebuild -create-xcframework \
  -library "$OUT/macos/libsqlglot_rust.a"                     -headers "$OUT/headers" \
  -library target/aarch64-apple-ios/release/libsqlglot_rust.a -headers "$OUT/headers" \
  -library "$OUT/ios-sim/libsqlglot_rust.a"                   -headers "$OUT/headers" \
  -output "$OUT/sqlglot_rustFFI.xcframework"
```

The result contains three slices: `macos-arm64_x86_64`, `ios-arm64`, and
`ios-arm64_x86_64-simulator`. Drop the iOS targets if you only need macOS.

**2. Create the Swift Package:**

```text
SqlGlot/
├── Package.swift
├── sqlglot_rustFFI.xcframework        # from target/uniffi/apple/
└── Sources/SqlGlot/sqlglot_rust.swift # from target/uniffi/swift/
```

```swift
// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "SqlGlot",
    platforms: [.macOS(.v12), .iOS(.v15)],
    products: [.library(name: "SqlGlot", targets: ["SqlGlot"])],
    targets: [
        .binaryTarget(name: "sqlglot_rustFFI", path: "sqlglot_rustFFI.xcframework"),
        .target(name: "SqlGlot", dependencies: ["sqlglot_rustFFI"]),
    ]
)
```

The binary target **must** be named `sqlglot_rustFFI`, the module the generated
Swift file imports.

**3. Use it** — add the package to your Xcode project or `Package.swift`, then:

```swift
import SqlGlot

let statement = try parse(sql: "SELECT a, b FROM t WHERE a > 1", dialect: .postgres)
print(try statement.generatePretty(dialect: .postgres))
print(try transpile(sql: "SELECT * FROM t LIMIT 10", readDialect: .mysql, writeDialect: .tsql))

do {
    _ = try parse(sql: "SELECT FROM WHERE", dialect: .ansi)
} catch let SqlglotError.Parser(detail) {
    print("parse failed: \(detail)")
}
```

---

## Kotlin

The Kotlin bindings (package `uniffi.sqlglot_rust`) call the native library
through [JNA](https://github.com/java-native-access/jna), which loads a library
named `sqlglot_rust` (`libsqlglot_rust.so` / `.dylib`, `sqlglot_rust.dll`).

### JVM with Gradle

**1. Add the generated source and the native library to your project:**

```text
src/main/kotlin/sqlglot_rust.kt                          # from target/uniffi/kotlin/uniffi/sqlglot_rust/
src/main/resources/darwin-aarch64/libsqlglot_rust.dylib  # one folder per platform you ship
src/main/resources/linux-x86-64/libsqlglot_rust.so
```

JNA extracts the library from the jar using its platform folder names:
`darwin-aarch64`, `darwin-x86-64`, `linux-x86-64`, `linux-aarch64`,
`win32-x86-64`.

**2. `build.gradle.kts`:**

```kotlin
plugins {
    kotlin("jvm") version "2.4.20"
    application
}

repositories { mavenCentral() }

dependencies {
    implementation("net.java.dev.jna:jna:5.17.0")
}

kotlin { jvmToolchain(17) }

application {
    mainClass.set("DemoKt")
    // Silences the JDK 22+ restricted-native-access warning.
    applicationDefaultJvmArgs = listOf("--enable-native-access=ALL-UNNAMED")
}
```

**3. Use it:**

```kotlin
import uniffi.sqlglot_rust.*

fun main() {
    parse("SELECT a, b FROM t WHERE a > 1", Dialect.POSTGRES).use { statement ->
        println(statement.generatePretty(Dialect.POSTGRES))
    }
    println(transpile("SELECT * FROM t LIMIT 10", Dialect.MYSQL, Dialect.TSQL))

    try {
        parse("SELECT FROM WHERE", Dialect.ANSI)
    } catch (e: SqlglotException.Parser) {
        println("parse failed: ${e.detail}")
    }
}
```

Instead of bundling resources, you can point JNA at a directory with
`-Djna.library.path=/path/to/lib`, or at a specific file with
`-Duniffi.component.sqlglot_rust.libraryOverride=/path/to/libsqlglot_rust.so`.

### Android

> Not verified by this project. These are the standard UniFFI/JNA steps.

**1. Cross-compile the library for each ABI:**

```bash
cargo install cargo-ndk
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 \
    -o app/src/main/jniLibs \
    build --release --features uniffi --lib
```

This produces `app/src/main/jniLibs/<abi>/libsqlglot_rust.so`.

**2. Generate Android-flavoured Kotlin bindings.** Create `uniffi-android.toml`
(the crate key must use the underscored name `sqlglot_rust`):

```toml
[crates.sqlglot_rust.bindings.kotlin]
android = true
package_name = "com.example.sqlglot"   # optional; default is uniffi.sqlglot_rust
```

```bash
target/release/uniffi-bindgen generate --config uniffi-android.toml \
    --library target/release/libsqlglot_rust.dylib \
    --language kotlin --out-dir app/src/main/java
```

Bindings can be generated from the host library: the interface is identical on
every platform.

**3. Add JNA as an AAR** in `app/build.gradle.kts`:

```kotlin
dependencies {
    implementation("net.java.dev.jna:jna:5.17.0@aar")
}
```

If you use R8/ProGuard, keep JNA and the generated classes:

```text
-keep class com.sun.jna.** { *; }
-keep class com.example.sqlglot.** { *; }
```

---

## Working with the AST

`toJson()` exposes the full AST. You can read it, modify it, and turn it back
into SQL with `SqlStatement.fromJson`. The JSON mirrors the Rust types
documented in the [API Reference](reference.md#statement-enum): the top-level
key is the statement kind (`Select`, `Insert`, …) and enums use their variant
names.

```python
import json
import sqlglot_rust as sg

statement = sg.parse("SELECT a, b FROM t WHERE a > 1", sg.Dialect.POSTGRES)
ast = json.loads(statement.to_json())

print(len(ast["Select"]["columns"]))                 # 2
print(ast["Select"]["from"]["source"]["Table"])      # {'catalog': None, 'schema': None, 'name': 't', ...}

ast["Select"]["from"]["source"]["Table"]["name"] = "orders"
edited = sg.SqlStatement.from_json(json.dumps(ast))
print(edited.generate(sg.Dialect.POSTGRES))          # SELECT a, b FROM orders WHERE a > 1
```

The same flow works in Swift (`JSONSerialization` or `Codable`) and Kotlin
(`kotlinx.serialization`, Jackson, or `org.json`).

Tips:

- The JSON format tracks the library version. Pin the sqlglot-rust version your
  code was written against.
- `fromJson` rejects JSON that does not describe a valid statement with
  `SqlglotError.InvalidAst`.
- `generate` validates the target dialect. For example, an `ARRAY[...]`
  constructor cannot be generated for T-SQL and raises
  `UnsupportedDialectFeature`.

---

## Error Handling

Every function that can fail raises `SqlglotError` (Kotlin:
`SqlglotException`). Match on the case for specific handling:

| Case | Fields | Raised when |
| --- | --- | --- |
| `Tokenizer` | `detail`, `position` (character offset) | The SQL text cannot be tokenized, e.g. an unterminated string |
| `Parser` | `detail` | The SQL is syntactically invalid |
| `UnsupportedDialectFeature` | `detail` | The statement uses a construct the target dialect cannot express |
| `InvalidAst` | `detail` | `fromJson` received JSON that is not a valid statement |
| `Internal` | `detail` | An unexpected internal failure |

```python
try:
    sg.parse("SELECT FROM WHERE", sg.Dialect.ANSI)
except sg.SqlglotError.Parser as e:
    print(e.detail)
except sg.SqlglotError as e:          # any other case
    print(type(e).__name__, e)
```

```swift
do {
    _ = try parse(sql: "SELECT 'abc", dialect: .ansi)
} catch let SqlglotError.Tokenizer(detail, position) {
    print("\(detail) at \(position)")
} catch {
    print("other error: \(error)")
}
```

```kotlin
try {
    parse("SELECT 'abc", Dialect.ANSI)
} catch (e: SqlglotException.Tokenizer) {
    println("${e.detail} at ${e.position}")
} catch (e: SqlglotException) {
    println("other error: $e")
}
```

The `detail` text is meant for humans and may change between versions. Branch
on the error **case**, not on the message.

---

## Memory and Threading

- `SqlStatement` is **immutable** and **thread-safe**: share it freely across
  threads (in Swift it is `Sendable`).
- Python and Swift free the native memory automatically.
- In Kotlin, `SqlStatement` is `AutoCloseable`. It is released by the garbage
  collector, but calling `close()` or `use { }` frees it deterministically. This
  is worthwhile when parsing many statements.
- All calls are synchronous and CPU-bound. On UI threads (iOS main thread,
  Android main thread), run large parses on a background thread or dispatcher.

---

## Troubleshooting

| Symptom | Cause | Fix |
| --- | --- | --- |
| `UniFFI API checksum mismatch` or `contract version mismatch` | Bindings and native library come from different builds | Regenerate the bindings from the exact library you ship |
| Python `OSError: dlopen(... libsqlglot_rust ...)` | Library not next to `sqlglot_rust.py`, or wrong CPU architecture | Copy the library into the module's directory; check with `file libsqlglot_rust.*` |
| Kotlin `UnsatisfiedLinkError: Unable to load library 'sqlglot_rust'` | JNA cannot find the library | Bundle it under the right resource folder, or set `jna.library.path` |
| JDK warning `restricted method in java.lang.System has been called` | JDK 22+ native-access checks | Run with `--enable-native-access=ALL-UNNAMED` |
| Swift `no such module 'sqlglot_rustFFI'` | Modulemap not found, or XCFramework target misnamed | Pass `-fmodule-map-file`, or name the binary target `sqlglot_rustFFI` and the modulemap `module.modulemap` |
| `uniffi-bindgen` warns `looks like an old-style --config override file` and your options are ignored | Pre-0.29 flat config format | Nest options under `[crates.sqlglot_rust.bindings.<language>]` |
| Converted SQL still contains source-dialect syntax (e.g. `LIMIT` for T-SQL) | Used `parse` + `generate` | Use `transpile` |

---

## Testing Your Integration

This repository tests the generated bindings for every language. You can run
the same suites locally:

```bash
make uniffi-test LANGS="python swift"
JNA_JAR=/path/to/jna-5.17.0.jar make uniffi-test LANGS=kotlin
```

The per-language tests in [`tests/uniffi/`](../tests/uniffi/) are good
starting points for your own integration tests.
