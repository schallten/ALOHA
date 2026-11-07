#!/usr/bin/env bash
# ALOHA redeploy: fetch latest code, stop the old container, rebuild, start fresh.
# Usage (on the VPS, from the repo root):  ./deploy.sh [--no-cache]
set -euo pipefail

cd "$(dirname "$0")"

# Pick the compose command (v2 plugin or legacy binary)
if docker compose version >/dev/null 2>&1; then
    COMPOSE=(docker compose)
elif command -v docker-compose >/dev/null 2>&1; then
    COMPOSE=(docker-compose)
else
    echo "error: docker compose not found" >&2
    exit 1
fi

# Refuse to pull over uncommitted changes to tracked files
if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "error: uncommitted changes in tracked files - commit or stash them first:" >&2
    git status --short >&2
    exit 1
fi

echo "==> Fetching latest code..."
git fetch origin
git pull --ff-only

echo "==> Stopping old container..."
"${COMPOSE[@]}" down

echo "==> Rebuilding image..."
if [[ "${1:-}" == "--no-cache" ]]; then
    "${COMPOSE[@]}" build --no-cache
else
    "${COMPOSE[@]}" build
fi

echo "==> Starting new container..."
"${COMPOSE[@]}" up -d

echo "==> Status:"
"${COMPOSE[@]}" ps
"${COMPOSE[@]}" logs --tail=30 aloha

echo "==> Done. Follow logs with: ${COMPOSE[*]} logs -f aloha"
