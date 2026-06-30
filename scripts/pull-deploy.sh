#!/bin/bash
set -euo pipefail

# Pull-based deploy: the server reaches out to GitHub (outbound only) instead of
# CI pushing in over SSH. Run on a timer; idempotent via the released commit sha.
#
#   binary + version.txt are published by the `Deploy binary` workflow to a
#   rolling release tag, so the download URLs below are stable.

# Re-exec from a stable copy so the `git reset --hard` below can never rewrite
# this script's own file while bash is still reading it (this script is tracked
# and may itself change between deploys).
REPO_DIR="${REPO_DIR:-$(cd "$(dirname "$0")/.." && pwd)}"
export REPO_DIR
if [ "${PULL_DEPLOY_REEXEC:-}" != "1" ]; then
    self_copy="$(mktemp)"
    cp "$0" "$self_copy"
    chmod +x "$self_copy"
    PULL_DEPLOY_REEXEC=1 exec "$self_copy" "$@"
fi
BIN_TMP=""
trap 'rm -f "$0" "$BIN_TMP"' EXIT

cd "$REPO_DIR"

REPO="${REPO:-marshallku/blog}"
DEPLOY_TAG="${DEPLOY_TAG:-deploy-latest}"
RELEASE_BASE="https://github.com/${REPO}/releases/download/${DEPLOY_TAG}"
STATE_FILE="${STATE_FILE:-.deployed-sha}"
DEPLOY_BACKEND="${DEPLOY_BACKEND:-1}"

deploy_site() {
    local remote_sha local_sha
    remote_sha="$(curl -fsSL "${RELEASE_BASE}/version.txt" | tr -d '[:space:]')"
    local_sha="$(cat "$STATE_FILE" 2>/dev/null || true)"

    if [ -z "$remote_sha" ]; then
        echo "❌ Could not read remote version.txt"
        return 1
    fi

    if [ "$remote_sha" = "$local_sha" ]; then
        echo "✅ Site already at ${remote_sha}, nothing to do"
        return 0
    fi

    echo "⬇️  New site release ${remote_sha} (was ${local_sha:-none})"
    BIN_TMP="$(mktemp)"
    curl -fsSL "${RELEASE_BASE}/blog" -o "$BIN_TMP"
    chmod +x "$BIN_TMP"

    # Sync tracked files (templates, static, manifest.json) to the exact commit
    # the binary was built from, then swap the binary and rebuild the site.
    git fetch origin master
    git reset --hard "$remote_sha"

    BINARY_STAGING="$BIN_TMP" ./scripts/update-binary.sh
    ./scripts/deploy.sh

    echo "$remote_sha" > "$STATE_FILE"
    echo "✅ Site deployed ${remote_sha}"
}

deploy_backend() {
    if [ "$DEPLOY_BACKEND" != "1" ]; then
        return 0
    fi
    echo "🐳 Refreshing backend image..."
    ./scripts/deploy-backend.sh
}

pull_repos() {
    git pull origin master
    cd content
    git pull origin master
    cd ..
}

pull_repos
deploy_site
deploy_backend
