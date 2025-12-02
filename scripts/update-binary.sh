#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

BINARY_STAGING="${BINARY_STAGING:-/tmp/blog-binary}"
BINARY_TARGET="${BINARY_TARGET:-./blog}"

echo "🔄 Updating binary..."

if [ ! -f "$BINARY_STAGING" ]; then
    echo "❌ Binary not found at $BINARY_STAGING"
    echo "   Upload binary first: rsync blog user@vps:$BINARY_STAGING"
    exit 1
fi

# Backup current binary
if [ -f "$BINARY_TARGET" ]; then
    cp "$BINARY_TARGET" "${BINARY_TARGET}.bak"
fi

# Atomic move
mv "$BINARY_STAGING" "$BINARY_TARGET"
chmod +x "$BINARY_TARGET"

echo "✅ Binary updated"
"$BINARY_TARGET" --version 2>/dev/null || echo "   (no --version support)"
