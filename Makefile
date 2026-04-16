.PHONY: all build release release-linux release-all install uninstall test run clean fmt clippy help

# Binary name
BINARY := skill-trading

# Installation prefix
PREFIX ?= /usr/local

# Rust targets
TARGET ?= $(shell rustc -vV | sed -n 's/host: //p')

# Build flags
RELEASE_FLAGS := --release
DEV_FLAGS :=

all: build

## build: Build debug binary
build:
	@echo "🔨 Building debug binary..."
	cargo build $(DEV_FLAGS)
	@echo "✅ Done: target/debug/$(BINARY)"

## release: Build optimized host-native binary and ship it with a platform suffix
release:
	@echo "🚀 Building release binary (host: $(TARGET))..."
	cargo build $(RELEASE_FLAGS)
	@strip target/release/$(BINARY) 2>/dev/null || true
	@echo "✅ Done: target/release/$(BINARY)"
	@ls -lh target/release/$(BINARY)
	@mkdir -p .claude/skills/skill-trading/scripts
	@case "$(TARGET)" in \
	  aarch64-apple-darwin)       SUFFIX=darwin-arm64 ;; \
	  x86_64-apple-darwin)        SUFFIX=darwin-x64 ;; \
	  x86_64-unknown-linux-gnu)   SUFFIX=linux-x64 ;; \
	  aarch64-unknown-linux-gnu)  SUFFIX=linux-arm64 ;; \
	  *) echo "❌ unknown host target $(TARGET) — add a suffix mapping"; exit 1 ;; \
	esac; \
	cp target/release/$(BINARY) .claude/skills/skill-trading/scripts/$(BINARY)-$$SUFFIX; \
	chmod +x .claude/skills/skill-trading/scripts/$(BINARY)-$$SUFFIX; \
	echo "✅ .claude/skills/skill-trading/scripts/$(BINARY)-$$SUFFIX updated"

## release-linux: Cross-compile Linux x86_64 binary via `cross` (requires Docker running)
release-linux:
	@command -v cross >/dev/null || { echo "❌ cross not installed. run: cargo install cross"; exit 1; }
	@docker info >/dev/null 2>&1 || { echo "❌ Docker daemon not running. start Docker Desktop first"; exit 1; }
	@echo "🐧 Cross-compiling linux-x64..."
	cross build --release --target x86_64-unknown-linux-gnu
	@mkdir -p .claude/skills/skill-trading/scripts
	@cp target/x86_64-unknown-linux-gnu/release/$(BINARY) .claude/skills/skill-trading/scripts/$(BINARY)-linux-x64
	@chmod +x .claude/skills/skill-trading/scripts/$(BINARY)-linux-x64
	@echo "✅ .claude/skills/skill-trading/scripts/$(BINARY)-linux-x64 updated"

## release-all: Build every shipped binary (host + linux-x64) and stage the launcher
release-all: release release-linux
	@test -x .claude/skills/skill-trading/scripts/$(BINARY) || { echo "❌ launcher .claude/skills/skill-trading/scripts/$(BINARY) missing"; exit 1; }
	@echo "✅ shipped:"; ls -lh .claude/skills/skill-trading/scripts/ | awk 'NR>1 {print "    "$$NF}'

## install: Install binary to $(PREFIX)/bin
install: release
	@echo "📦 Installing to $(PREFIX)/bin/$(BINARY)..."
	@mkdir -p $(PREFIX)/bin
	@cp target/release/$(BINARY) $(PREFIX)/bin/
	@chmod +x $(PREFIX)/bin/$(BINARY)
	@echo "✅ Installed! Run: $(BINARY) --help"

## uninstall: Remove installed binary
uninstall:
	@echo "🗑️  Removing $(PREFIX)/bin/$(BINARY)..."
	@rm -f $(PREFIX)/bin/$(BINARY)
	@echo "✅ Uninstalled"

## test: Run tests
test:
	@echo "🧪 Running tests..."
	cargo test --all-features
	@echo "✅ Tests passed"

## run: Run with arguments (e.g., make run -- --help)
run:
	cargo run -- $(filter-out $@,$(MAKECMDGOALS))

## clean: Remove build artifacts
clean:
	@echo "🧹 Cleaning..."
	cargo clean
	@rm -rf dist/
	@echo "✅ Cleaned"

## fmt: Format code
fmt:
	@echo "✨ Formatting code..."
	cargo fmt
	@echo "✅ Formatted"

## clippy: Run linter
clippy:
	@echo "🔍 Running clippy..."
	cargo clippy --all-targets --all-features -- -D warnings
	@echo "✅ Clippy passed"

## check: Quick check without building
check:
	cargo check

## docs: Generate documentation
docs:
	cargo doc --no-deps --open

## dist: Build distribution binaries for multiple platforms
dist: release
	@echo "📦 Building distribution binaries..."
	@mkdir -p dist
	
	# macOS ARM64
	@echo "  🍎 macOS ARM64..."
	cargo build --release --target aarch64-apple-darwin
	@cp target/aarch64-apple-darwin/release/$(BINARY) dist/$(BINARY)-darwin-arm64
	
	# macOS x64
	@echo "  🍎 macOS x64..."
	cargo build --release --target x86_64-apple-darwin
	@cp target/x86_64-apple-darwin/release/$(BINARY) dist/$(BINARY)-darwin-x64
	
	# Linux x64
	@echo "  🐧 Linux x64..."
	cargo build --release --target x86_64-unknown-linux-gnu
	@cp target/x86_64-unknown-linux-gnu/release/$(BINARY) dist/$(BINARY)-linux-x64
	
	# Windows x64
	@echo "  🪟 Windows x64..."
	cargo build --release --target x86_64-pc-windows-gnu
	@cp target/x86_64-pc-windows-gnu/release/$(BINARY).exe dist/$(BINARY)-windows-x64.exe
	
	@echo "✅ Distribution binaries in dist/"

## version: Show version
version:
	@echo "$(BINARY) $(shell cargo pkgid | cut -d@ -f2)"

## help: Show this help
help:
	@echo "📖 $(BINARY) - TTC Trading CLI"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@sed -n 's/^## //p' $(MAKEFILE_LIST) | column -t -s ':'
	@echo ""
	@echo "Examples:"
	@echo "  make build              # Build debug binary"
	@echo "  make release            # Build optimized binary"
	@echo "  make install            # Install to $(PREFIX)/bin"
	@echo "  make test               # Run tests"
	@echo "  make run -- --help      # Run with arguments"
	@echo ""

# Pass-through for run target
%:
	@:
