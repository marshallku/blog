#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "📦 Building frontend assets..."

# Install dependencies if needed
if [ ! -d "node_modules" ] || [ "pnpm-lock.yaml" -nt "node_modules" ]; then
    pnpm install --frozen-lockfile
fi

# Build all packages and copy to static/
pnpm build:assets

echo "✅ Assets built successfully"
echo "   Versions:"
grep '"version"' manifest.json | head -3
