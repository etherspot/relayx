#!/bin/bash

# Setup development environment for RelayX
set -e

echo "🔧 Setting up RelayX development environment..."

# Install required Rust components
echo "📦 Installing Rust components..."
rustup component add rustfmt
rustup component add clippy

# Install nightly components for udeps
echo "📦 Installing nightly components..."
rustup component add clippy --toolchain nightly || echo "⚠️  Nightly clippy already installed or unavailable"
rustup component add rustfmt --toolchain nightly || echo "⚠️  Nightly rustfmt already installed or unavailable"

# Install useful development tools
echo "📦 Installing development tools..."
cargo install cargo-sort || echo "⚠️  cargo-sort already installed"
cargo install cargo-udeps --locked || echo "⚠️  cargo-udeps already installed"

# Install system dependencies (platform-specific)
echo "📦 Installing system dependencies..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    if command -v brew &> /dev/null; then
        brew install rocksdb || echo "⚠️  RocksDB already installed"
    else
        echo "⚠️  Homebrew not found. Please install RocksDB manually: brew install rocksdb"
    fi
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux
    if command -v apt-get &> /dev/null; then
        sudo apt-get update
        sudo apt-get install -y librocksdb-dev || echo "⚠️  RocksDB already installed"
    elif command -v yum &> /dev/null; then
        sudo yum install -y rocksdb-devel || echo "⚠️  RocksDB already installed"
    else
        echo "⚠️  Package manager not found. Please install RocksDB development libraries manually"
    fi
else
    echo "⚠️  Unsupported OS. Please install RocksDB development libraries manually"
fi

echo "✅ Development environment setup complete!"
echo ""
echo "🚀 You can now run:"
echo "  make lint     - Run clippy, fmt, sort, and udeps"
echo "  make test     - Build and test the service"
echo "  make run      - Run the service"
echo "  cargo clippy --all --all-targets --no-deps -- --deny warnings"
