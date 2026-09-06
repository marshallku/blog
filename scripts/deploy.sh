#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

BLOG_BIN="${BLOG_BIN:-./blog}"

echo "🔨 Building site..."

# Check binary exists
if [ ! -x "$BLOG_BIN" ]; then
    echo "❌ Binary not found: $BLOG_BIN"
    echo "   Run update-binary.sh first or set BLOG_BIN"
    exit 1
fi

"$BLOG_BIN" build --incremental

echo "✅ Site built into dist/"
