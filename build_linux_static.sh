#!/bin/bash
# Script to build a static Linux x64 binary of udp-over-tcp
# Run this on a Linux system with Rust installed

set -e

echo "Building static Linux x64 binary for udp-over-tcp..."

# Install Rust if not present
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi

# Add musl target for static linking
echo "Adding musl target..."
rustup target add x86_64-unknown-linux-musl

# Install musl-tools if on Ubuntu/Debian
if command -v apt-get &> /dev/null; then
    echo "Installing musl-tools..."
    sudo apt-get update
    sudo apt-get install -y musl-tools
fi

# Build the static binary
echo "Building static binary..."
cargo build --target x86_64-unknown-linux-musl --release

echo "Static binary built successfully!"
echo "Location: target/x86_64-unknown-linux-musl/release/udp-over-tcp"

# Check if the binary is actually static
echo "Checking if binary is static..."
if command -v ldd &> /dev/null; then
    echo "Running ldd check:"
    ldd target/x86_64-unknown-linux-musl/release/udp-over-tcp || echo "Binary is statically linked (good!)"
fi

# Show file size
echo "Binary size:"
ls -lh target/x86_64-unknown-linux-musl/release/udp-over-tcp

echo "Done! You can now copy this binary to any Linux x64 system."
