#!/bin/bash
set -euo pipefail

# The backend (blog-api + blog-database) does NOT run from this repo's
# docker-compose.yml. It runs from a separate manifest directory whose compose
# owns the real container_name `blog-api`, the external app_network, and the
# /mnt/hdd mongo volumes. Deploying against the repo's own compose would spawn a
# conflicting project, so always target the manifest dir.
BACKEND_COMPOSE_DIR="${BACKEND_COMPOSE_DIR:-$HOME/dev/manifest/docker-compose/blog-backend}"

REGISTRY="${REGISTRY:-ghcr.io}"
REGISTRY_USER="${REGISTRY_USER:-marshallku}"

if [ ! -f "$BACKEND_COMPOSE_DIR/docker-compose.yml" ]; then
    echo "❌ Backend compose not found: $BACKEND_COMPOSE_DIR/docker-compose.yml"
    echo "   Set BACKEND_COMPOSE_DIR to the directory holding the backend compose."
    exit 1
fi

cd "$BACKEND_COMPOSE_DIR"

if [ -n "${GH_PACKAGE_TOKEN:-}" ]; then
    echo "🔑 Logging in to $REGISTRY..."
    echo "$GH_PACKAGE_TOKEN" | docker login "$REGISTRY" -u "$REGISTRY_USER" --password-stdin
fi

echo "🐳 Pulling latest backend image..."
docker compose pull api

echo "🚀 Restarting backend..."
docker compose up -d --no-deps api

docker image prune -f >/dev/null

echo "✅ Backend deploy complete"
docker compose ps api
