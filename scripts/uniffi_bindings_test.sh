#!/usr/bin/env bash
# Build the library, generate UniFFI bindings, and run the per-language tests.
#
# Usage: scripts/uniffi_bindings_test.sh [python] [swift] [kotlin]
#   (defaults to all three; every requested language must run or the script fails)
# Env:   KOTLINC (default: kotlinc), JNA_JAR (required for kotlin)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

LANGUAGES=("$@")
[[ ${#LANGUAGES[@]} -eq 0 ]] && LANGUAGES=(python swift kotlin)

OUT="$ROOT/target/uniffi"
LIB_DIR="$ROOT/target/debug"
case "$(uname -s)" in
  Darwin) LIB="$LIB_DIR/libsqlglot_rust.dylib" ;;
  *)      LIB="$LIB_DIR/libsqlglot_rust.so" ;;
esac
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
export SQLGLOT_EXPECTED_VERSION="$VERSION"

echo "==> building library and uniffi-bindgen"
cargo build --features uniffi-bindgen --lib --bin uniffi-bindgen
rm -rf "$OUT"

generate() {
  "$LIB_DIR/uniffi-bindgen" generate --no-format --library "$LIB" --language "$1" --out-dir "$OUT/$1"
}

run_python() {
  generate python
  cp "$LIB" "$OUT/python/"
  PYTHONPATH="$OUT/python" python3 "$ROOT/tests/uniffi/test_bindings.py"
}

run_swift() {
  generate swift
  local dir="$OUT/swift"
  cp "$ROOT/tests/uniffi/test_bindings.swift" "$dir/main.swift"
  swiftc -module-name SqlglotBindingsTest \
    -Xcc -fmodule-map-file="$dir/sqlglot_rustFFI.modulemap" -I "$dir" \
    -L "$LIB_DIR" -lsqlglot_rust -Xlinker -rpath -Xlinker "$LIB_DIR" \
    "$dir/sqlglot_rust.swift" "$dir/main.swift" -o "$dir/test_bindings"
  LD_LIBRARY_PATH="$LIB_DIR" "$dir/test_bindings"
}

run_kotlin() {
  local kotlinc="${KOTLINC:-kotlinc}"
  command -v "$kotlinc" >/dev/null || { echo "kotlinc not found (set KOTLINC)" >&2; return 1; }
  [[ -f "${JNA_JAR:-}" ]] || { echo "JNA_JAR must point to a JNA jar" >&2; return 1; }
  generate kotlin
  local dir="$OUT/kotlin"
  "$kotlinc" -nowarn -classpath "$JNA_JAR" \
    "$dir/uniffi/sqlglot_rust/sqlglot_rust.kt" "$ROOT/tests/uniffi/TestBindings.kt" \
    -include-runtime -d "$dir/test_bindings.jar"
  java --enable-native-access=ALL-UNNAMED -Djna.library.path="$LIB_DIR" \
    -cp "$dir/test_bindings.jar:$JNA_JAR" TestBindingsKt
}

for language in "${LANGUAGES[@]}"; do
  echo "==> $language bindings"
  case "$language" in
    python) run_python ;;
    swift)  run_swift ;;
    kotlin) run_kotlin ;;
    *) echo "unknown language: $language" >&2; exit 2 ;;
  esac
done
echo "==> uniffi bindings OK: ${LANGUAGES[*]}"
