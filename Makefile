.PHONY: build clean test run help

# Default target
all: build

# Build the service
build:
	@echo "🔨 Building RelayX service..."
	cargo build --release
	@echo "✅ Build complete!"

# Build in debug mode
build-debug:
	@echo "🔨 Building RelayX service (debug)..."
	cargo build
	@echo "✅ Debug build complete!"

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	rm -rf relayx_db
	@echo "✅ Clean complete!"

# Run the service
run:
	@echo "🚀 Starting RelayX service..."
	./target/release/relayx

# Run with custom config
run-custom:
	@echo "🚀 Starting RelayX service with custom config..."
	./target/release/relayx --rpc-host 0.0.0.0 --rpc-port 8545 --db-path ./relayx_db

# Run in debug mode
run-debug:
	@echo "🚀 Starting RelayX service (debug mode)..."
	./target/debug/relayx

# Test the service
test: build
	@echo "🧪 Testing RelayX service..."
	./scripts/test_service.sh

# Check code quality
check:
	@echo "🔍 Checking code quality..."
	cargo check
	cargo clippy
	@echo "✅ Code quality check complete!"

# Format code
fmt:
	@echo "✨ Formatting code..."
	cargo fmt
	@echo "✅ Code formatting complete!"

# Install development dependencies
install-dev:
	@echo "📦 Installing development dependencies..."
	cargo install cargo-watch
	@echo "✅ Development dependencies installed!"

# Watch and rebuild on changes
watch:
	@echo "👀 Watching for changes..."
	cargo watch -x check -x test -x run

.PHONY: lint
lint: # Run `clippy` and `rustfmt`.
	cargo +nightly fmt --all
	cargo clippy --all --all-targets --features "$(FEATURES)" --no-deps -- --deny warnings

	# cargo sort
	cargo sort --grouped 

	# udeps
	cargo +nightly udeps --all-targets

clean-deps:
	cargo +nightly udeps --all-targets --release

# Show help
help:
	@echo "RelayX Service - Available Commands"
	@echo "=================================="
	@echo "build        - Build the service (release)"
	@echo "build-debug  - Build the service (debug)"
	@echo "clean        - Clean build artifacts"
	@echo "run          - Run the service (release)"
	@echo "run-custom   - Run with custom configuration"
	@echo "run-debug    - Run the service (debug)"
	@echo "test         - Build and test the service"
	@echo "check        - Check code quality"
	@echo "fmt          - Format code"
	@echo "lint         - Run clippy, rustfmt, and udeps"
	@echo "install-dev  - Install development dependencies"
	@echo "watch        - Watch for changes and rebuild"
	@echo "help         - Show this help message"

