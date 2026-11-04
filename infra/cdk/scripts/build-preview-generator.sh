#!/bin/bash
# Build the Rust preview-generator Lambda for AWS Lambda (x86_64)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RUST_SRC="${SCRIPT_DIR}/src/lambda/preview-generator-rust"
DIST_DIR="${SCRIPT_DIR}/dist/lambda/preview-generator-rust"

echo "🔨 Building preview-generator Lambda..."
echo "   Source: ${RUST_SRC}"
echo "   Target: ${DIST_DIR}"

# Create output directory
mkdir -p "${DIST_DIR}"

# Cross-compile for x86_64-unknown-linux-gnu (Lambda runtime)
cd "${RUST_SRC}"
cargo build --release --target x86_64-unknown-linux-gnu 2>&1

# Copy the binary
cp "target/x86_64-unknown-linux-gnu/release/bootstrap" "${DIST_DIR}/bootstrap"

# Make it executable
chmod +x "${DIST_DIR}/bootstrap"

# Show binary size
BINARY_SIZE=$(stat -f%z "${DIST_DIR}/bootstrap" 2>/dev/null || stat -c%s "${DIST_DIR}/bootstrap" 2>/dev/null || echo "?")
echo ""
echo "✅ Build complete!"
echo "   Binary: ${DIST_DIR}/bootstrap (${BINARY_SIZE} bytes)"

if [ "$BINARY_SIZE" -gt 250000000 ]; then
    echo "   ⚠️  Binary exceeds 250MB Lambda limit - use container image instead"
fi
