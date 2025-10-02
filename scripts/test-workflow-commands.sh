#!/bin/bash

# Test script to validate GitHub workflow commands locally
set -e

echo "🧪 Testing GitHub workflow commands locally..."

echo "📋 Running format check..."
cargo fmt --all -- --check

echo "📋 Running clippy..."
cargo clippy --all --all-targets --no-deps -- --deny warnings

echo "📋 Running cargo-sort check..."
if ! command -v cargo-sort &> /dev/null; then
    echo "Installing cargo-sort..."
    cargo install cargo-sort --locked
fi
cargo-sort --check

echo "📋 Running tests..."
cargo test --verbose

echo "📋 Building release..."
cargo build --release --verbose

echo "✅ All workflow commands completed successfully!"
echo "🚀 The GitHub workflows should now work correctly."
