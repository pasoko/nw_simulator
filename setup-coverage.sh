#!/bin/bash
# Setup script for Rust code coverage tools

echo "Setting up Rust code coverage tools..."

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed. Please install Rust first."
    exit 1
fi

# Install cargo-tarpaulin for code coverage
echo "Installing cargo-tarpaulin..."
cargo install cargo-tarpaulin

# Create coverage directory
mkdir -p target/coverage

echo "Setup complete!"
echo ""
echo "To generate coverage report, run:"
echo "  cargo tarpaulin --out Html --output-dir target/coverage"
echo "  # Or use the Makefile:"
echo "  make coverage-rust"
echo ""
echo "The HTML coverage report will be available at: target/coverage/index.html"