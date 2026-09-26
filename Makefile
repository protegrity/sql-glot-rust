.PHONY: build test lint fmt sbom clean all bump-version \
       ffi ffi-header ffi-all ffi-macos-arm64 ffi-macos-amd64 ffi-linux-amd64 ffi-linux-arm64 \
       cli cli-target cli-macos-arm64 cli-macos-amd64 cli-linux-amd64 cli-linux-arm64 cli-all \
       dist dist-all \
       pkg-deb pkg-rpm \
       uniffi-bindings uniffi-test

all: build sbom

build:
	cargo build

test:
	cargo test

lint:
	cargo clippy

fmt:
	cargo fmt

sbom:
	@echo "Generating SPDX SBOM..."
	@mkdir -p target/sbom
	cargo sbom --output-format spdx_json_2_3 > target/sbom/sqlglot-rust.spdx.json
	@echo "SBOM written to target/sbom/sqlglot-rust.spdx.json"

clean:
	cargo clean

# ── C/C++ FFI targets ────────────────────────────────────────────────────

FFI_OUT = target/ffi
HEADER  = $(FFI_OUT)/include/sqlglot.h

## Generate the C header with cbindgen
ffi-header: $(HEADER)
$(HEADER):
	@mkdir -p $(FFI_OUT)/include
	cbindgen --config cbindgen.toml --crate sqlglot-rust --output $(HEADER)
	@echo "Header written to $(HEADER)"

## Build for a single target (set TARGET, e.g. make ffi-target TARGET=aarch64-apple-darwin)
ffi-target: ffi-header
ifndef TARGET
	$(error TARGET is required. E.g. make ffi-target TARGET=aarch64-apple-darwin)
endif
	cargo build --release --target $(TARGET)
	@mkdir -p $(FFI_OUT)/$(TARGET)/lib
	@cp -f target/$(TARGET)/release/libsqlglot_rust.a  $(FFI_OUT)/$(TARGET)/lib/ 2>/dev/null || true
	@cp -f target/$(TARGET)/release/libsqlglot_rust.so $(FFI_OUT)/$(TARGET)/lib/ 2>/dev/null || true
	@cp -f target/$(TARGET)/release/libsqlglot_rust.dylib $(FFI_OUT)/$(TARGET)/lib/ 2>/dev/null || true
	@echo "Built FFI libraries for $(TARGET) → $(FFI_OUT)/$(TARGET)/lib/"

## Convenience per-platform targets
ffi-macos-arm64:
	$(MAKE) ffi-target TARGET=aarch64-apple-darwin

ffi-macos-amd64:
	$(MAKE) ffi-target TARGET=x86_64-apple-darwin

ffi-linux-amd64:
	$(MAKE) ffi-target TARGET=x86_64-unknown-linux-gnu

ffi-linux-arm64:
	$(MAKE) ffi-target TARGET=aarch64-unknown-linux-gnu

## Build for the current host only
ffi: ffi-header
	cargo build --release
	@mkdir -p $(FFI_OUT)/lib
	@cp -f target/release/libsqlglot_rust.a      $(FFI_OUT)/lib/ 2>/dev/null || true
	@cp -f target/release/libsqlglot_rust.so     $(FFI_OUT)/lib/ 2>/dev/null || true
	@cp -f target/release/libsqlglot_rust.dylib  $(FFI_OUT)/lib/ 2>/dev/null || true
	@echo "Built FFI libraries for host → $(FFI_OUT)/lib/"

## Build all four platform/arch combinations
ffi-all: ffi-macos-arm64 ffi-macos-amd64 ffi-linux-amd64 ffi-linux-arm64
	@echo "All FFI targets built → $(FFI_OUT)/"

# ── UniFFI bindings (Swift / Kotlin / Python) ────────────────────────────

UNIFFI_OUT = target/uniffi
UNIFFI_LIB = target/release/libsqlglot_rust.$(if $(filter Darwin,$(shell uname -s)),dylib,so)

## Generate Swift, Kotlin, and Python bindings from a release build
uniffi-bindings:
	cargo build --release --features uniffi-bindgen --lib --bin uniffi-bindgen
	@for lang in swift kotlin python; do \
		target/release/uniffi-bindgen generate --no-format --library $(UNIFFI_LIB) \
			--language $$lang --out-dir $(UNIFFI_OUT)/$$lang || exit 1; \
	done
	@echo "Bindings written to $(UNIFFI_OUT)/{swift,kotlin,python}; native library: $(UNIFFI_LIB)"

## Run the generated-binding tests (LANGS="python swift kotlin"; kotlin needs JNA_JAR)
uniffi-test:
	cargo test --features uniffi --lib uniffi_api
	cargo test --features uniffi --test test_uniffi
	scripts/uniffi_bindings_test.sh $(LANGS)

# ── CLI cross-compilation targets ─────────────────────────────────────────

CLI_OUT = target/cli

## Build CLI for a single target (set TARGET)
cli-target:
ifndef TARGET
	$(error TARGET is required. E.g. make cli-target TARGET=aarch64-apple-darwin)
endif
	cargo build --release --features cli --target $(TARGET)
	@mkdir -p $(CLI_OUT)/$(TARGET)/bin
	@cp -f target/$(TARGET)/release/sqlglot $(CLI_OUT)/$(TARGET)/bin/ 2>/dev/null || true
	@echo "Built CLI for $(TARGET) → $(CLI_OUT)/$(TARGET)/bin/sqlglot"

## Convenience per-platform CLI targets
cli-macos-arm64:
	$(MAKE) cli-target TARGET=aarch64-apple-darwin

cli-macos-amd64:
	$(MAKE) cli-target TARGET=x86_64-apple-darwin

cli-linux-amd64:
	$(MAKE) cli-target TARGET=x86_64-unknown-linux-gnu

cli-linux-arm64:
	$(MAKE) cli-target TARGET=aarch64-unknown-linux-gnu

## Build CLI for the current host
cli:
	cargo build --release --features cli
	@mkdir -p $(CLI_OUT)/bin
	@cp -f target/release/sqlglot $(CLI_OUT)/bin/ 2>/dev/null || true
	@echo "Built CLI for host → $(CLI_OUT)/bin/sqlglot"

## Build CLI for all four targets
cli-all: cli-macos-arm64 cli-macos-amd64 cli-linux-amd64 cli-linux-arm64
	@echo "All CLI targets built → $(CLI_OUT)/"

# ── Combined distribution targets ────────────────────────────────────────

## Build FFI + CLI for the current host
dist: ffi cli
	@echo "Distribution built for host → target/ffi/ + target/cli/"

## Build FFI + CLI for all four targets
dist-all: ffi-all cli-all
	@echo "Full distribution built for all targets"

# ── Linux packaging targets ──────────────────────────────────────────────

## Build a Debian package (requires cargo-deb; run on Linux after `make ffi cli`)
pkg-deb: ffi cli
	cargo deb --no-build --no-strip
	@echo "Debian package built → target/debian/"

## Build an RPM package (requires cargo-generate-rpm; run on Linux after `make ffi cli`)
pkg-rpm: ffi cli
	cargo generate-rpm
	@echo "RPM package built → target/generate-rpm/"

# ── Version management ───────────────────────────────────────────────────
bump-version:
ifndef VERSION
	$(error VERSION is required. Usage: make bump-version VERSION=1.2.3)
endif
	@echo "$(VERSION)" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$$' || \
		{ echo "Error: VERSION must be a full semantic version (MAJOR.MINOR.PATCH), e.g. 1.2.3"; exit 1; }
	@echo "Bumping version to $(VERSION)..."
	@# Update Cargo.toml
	sed -i '' 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml
	@# Update version in documentation
	sed -i '' 's/sqlglot-rust = "[^"]*"/sqlglot-rust = "$(VERSION)"/' README.md
	sed -i '' 's/sqlglot-rust = "[^"]*"/sqlglot-rust = "$(VERSION)"/' docs/installation.md
	@# Sync Cargo.lock
	cargo generate-lockfile
	@echo "Version updated to $(VERSION)"
