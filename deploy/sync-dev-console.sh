#!/usr/bin/env bash
# Build & sync crab-console (dev) to EC2
#
# Usage:
#   ./deploy/sync-dev-console.sh              # Build + sync + reload Caddy
#   ./deploy/sync-dev-console.sh --dry-run    # Build + preview what would be synced
#   ./deploy/sync-dev-console.sh --skip-build # Sync existing build without rebuilding
#
# Prerequisites:
#   - SSH key at deploy/ec2/crab-ec2.pem
#   - EC2_HOST env var or default from .env

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CONSOLE_DIR="$PROJECT_ROOT/crab-console"
BUILD_DIR="$CONSOLE_DIR/build"
PEM_FILE="$SCRIPT_DIR/ec2/crab-ec2.pem"
SSH_USER="ec2-user"
REMOTE_DIR="/opt/crab/dev-console"

# Load EC2_HOST from .env if not set
if [ -z "${EC2_HOST:-}" ] && [ -f "$SCRIPT_DIR/ec2/.env" ]; then
    EC2_HOST=$(grep -E '^EC2_HOST=' "$SCRIPT_DIR/ec2/.env" | cut -d'=' -f2 || true)
fi

if [ -z "${EC2_HOST:-}" ]; then
    echo "ERROR: EC2_HOST not set. Export it or add to deploy/ec2/.env"
    exit 1
fi

if [ ! -f "$PEM_FILE" ]; then
    echo "ERROR: SSH key not found at $PEM_FILE"
    exit 1
fi

SSH_OPTS="-i $PEM_FILE -o StrictHostKeyChecking=no"
DRY_RUN=""
SKIP_BUILD=""

for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN="--dry-run"; echo "==> DRY RUN mode" ;;
        --skip-build) SKIP_BUILD="1" ;;
    esac
done

# Build with dev env vars
if [ -z "$SKIP_BUILD" ]; then
    echo "==> Building crab-console (dev)"
    cd "$CONSOLE_DIR"
    npm ci --silent
    VITE_API_BASE=https://dev-cloud.redcoral.app \
    VITE_WS_BASE=wss://dev-cloud.redcoral.app \
    npm run build
    cd "$PROJECT_ROOT"
fi

if [ ! -d "$BUILD_DIR" ]; then
    echo "ERROR: Build directory not found at $BUILD_DIR. Run 'npm run build' first."
    exit 1
fi

echo "==> Syncing dev-console build to $EC2_HOST:$REMOTE_DIR"
rsync -avz --delete $DRY_RUN \
    -e "ssh $SSH_OPTS" \
    "$BUILD_DIR/" \
    "$SSH_USER@$EC2_HOST:$REMOTE_DIR/"

if [ -z "$DRY_RUN" ]; then
    echo "==> Reloading Caddy config"
    ssh $SSH_OPTS "$SSH_USER@$EC2_HOST" \
        "cd /opt/crab && docker-compose exec -T caddy caddy reload --config /etc/caddy/Caddyfile"
    echo "==> Done! Dev console live at https://dev-console.redcoral.app"
fi
