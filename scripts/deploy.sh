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

# Build the site in place. nginx serves dist/ directly
# (root /home/marshall/dev/blog/dist), so the build IS the deploy — no copy
# step. The incremental cache invalidates itself when the binary, templates,
# config.yaml, or manifest.json change, and prunes output for deleted posts.
"$BLOG_BIN" build --incremental

echo "✅ Site built — served directly from dist/"
