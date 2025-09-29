#!/bin/bash
# Build script for all architectures with automatic naming
# Usage: ./build-all.sh [--debug]

set -e

# Parse arguments
BUILD_TYPE="release"
BUILD_FLAG="--release"
if [[ "$1" == "--debug" ]]; then
    BUILD_TYPE="debug"
    BUILD_FLAG=""
fi

# Get version info
VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
BUILD_NUMBER=$(cat build_number.txt 2>/dev/null || echo "1")
GIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
FULL_VERSION="$VERSION+$BUILD_NUMBER.$GIT_HASH"

echo "Building udp-over-tcp v$FULL_VERSION"

# Define targets and their output names
declare -a TARGETS=(
    "x86_64-unknown-linux-musl:x86_64-linux:Linux x64 (static)"
    "aarch64-unknown-linux-musl:aarch64-linux:Linux ARM64 (static)"
)

# Create output directory
OUTPUT_DIR="dist"
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

echo "Output directory: $OUTPUT_DIR"

for target_info in "${TARGETS[@]}"; do
    IFS=':' read -r target suffix description <<< "$target_info"
    
    echo ""
    echo "Building for $description..."
    
    # Add target if not already installed
    rustup target add "$target" 2>/dev/null || true
    
    # Build for target
    echo "Running: cargo build --target $target $BUILD_FLAG"
    
    if cargo build --target "$target" $BUILD_FLAG; then
        # Determine source path
        SOURCE_PATH="target/$target/$BUILD_TYPE/udp-over-tcp"
        
        # Create output filename with version
        OUTPUT_NAME="udp-over-tcp-v$VERSION-build$BUILD_NUMBER-$suffix"
        OUTPUT_PATH="$OUTPUT_DIR/$OUTPUT_NAME"
        
        # Copy and rename binary
        if [[ -f "$SOURCE_PATH" ]]; then
            cp "$SOURCE_PATH" "$OUTPUT_PATH"
            SIZE=$(du -h "$OUTPUT_PATH" | cut -f1)
            echo "✓ Built: $OUTPUT_NAME ($SIZE)"
        else
            echo "✗ Failed: Source not found at $SOURCE_PATH"
        fi
    else
        echo "✗ Build failed for $description"
    fi
done

echo ""
echo "Build Summary:"
echo "Version: $FULL_VERSION"
echo "Build Type: $BUILD_TYPE"
echo "Output Directory: $OUTPUT_DIR"

if [[ -d "$OUTPUT_DIR" ]]; then
    echo ""
    echo "Generated Binaries:"
    ls -lh "$OUTPUT_DIR" | tail -n +2 | awk '{print "  " $9 " (" $5 ")"}'
fi

echo ""
echo "Build completed!"
