#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

BLOG_BIN="${BLOG_BIN:-./blog}"
WWW_PATH="${WWW_PATH:-/var/www/blog}"

echo "🔨 Building site..."

# Check binary exists
if [ ! -x "$BLOG_BIN" ]; then
    echo "❌ Binary not found: $BLOG_BIN"
    echo "   Run update-binary.sh first or set BLOG_BIN"
    exit 1
fi

# Build site to dist/
"$BLOG_BIN" build

echo "🚀 Deploying to $WWW_PATH..."

# Phase 1: Add new files first (prevents 404 on new assets)
echo "   Adding new files..."
rsync -a --ignore-existing dist/ "$WWW_PATH/"

# Phase 2: Update existing files
echo "   Updating files..."
rsync -a dist/ "$WWW_PATH/"

# Phase 3: Remove deleted files last
echo "   Cleaning old files..."
rsync -a --delete dist/ "$WWW_PATH/"

echo "✅ Deploy complete"
