#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "🔨 Building release binary..."

cargo build --release

cp target/release/blog ./blog
chmod +x ./blog

echo "✅ Binary built and copied to ./blog"
./blog --version 2>/dev/null || echo "   $(ls -lh ./blog | awk '{print $5}')"
