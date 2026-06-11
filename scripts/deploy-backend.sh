#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

REGISTRY="${REGISTRY:-ghcr.io}"
REGISTRY_USER="${REGISTRY_USER:-marshallku}"

if [ -n "${GH_PACKAGE_TOKEN:-}" ]; then
    echo "🔑 Logging in to $REGISTRY..."
    echo "$GH_PACKAGE_TOKEN" | docker login "$REGISTRY" -u "$REGISTRY_USER" --password-stdin
fi

echo "🐳 Pulling latest backend image..."
docker compose pull api

echo "🚀 Restarting backend..."
docker compose up -d --wait --no-deps api

docker image prune -f >/dev/null

echo "✅ Backend deploy complete"
docker compose ps api
