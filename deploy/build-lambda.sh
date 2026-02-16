#!/usr/bin/env bash
# Build crab-auth Lambda deployment package
#
# Usage:
#   ./deploy/build-lambda.sh          # Docker-based cross-compilation (recommended)
#   ./deploy/build-lambda.sh native   # Native build (requires aarch64 target)
#
# Output: deploy/crab-auth-lambda.zip

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_ZIP="$SCRIPT_DIR/crab-auth-lambda.zip"

cd "$PROJECT_ROOT"

if [ "${1:-docker}" = "native" ]; then
    echo "==> Native build (aarch64-unknown-linux-gnu)"
    rustup target add aarch64-unknown-linux-gnu 2>/dev/null || true
    cargo build --release --target aarch64-unknown-linux-gnu -p crab-auth
    cp target/aarch64-unknown-linux-gnu/release/crab-auth "$SCRIPT_DIR/bootstrap"
else
    echo "==> Docker-based cross-compilation"
    docker build -f crab-auth/Dockerfile.lambda -t crab-auth-lambda .
    # Extract bootstrap binary from container
    CONTAINER_ID=$(docker create crab-auth-lambda)
    docker cp "$CONTAINER_ID:/var/runtime/bootstrap" "$SCRIPT_DIR/bootstrap"
    docker rm "$CONTAINER_ID"
fi

# Package as Lambda zip
cd "$SCRIPT_DIR"
chmod 755 bootstrap
zip -j "$OUTPUT_ZIP" bootstrap
rm bootstrap

echo "==> Lambda package ready: $OUTPUT_ZIP"
echo "    Size: $(du -h "$OUTPUT_ZIP" | cut -f1)"
echo ""
echo "Deploy with:"
echo "  aws lambda update-function-code \\"
echo "    --function-name crab-auth \\"
echo "    --zip-file fileb://$OUTPUT_ZIP \\"
echo "    --architectures arm64"
