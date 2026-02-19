#!/usr/bin/env bash
# Build and push crab-cloud Docker image to ECR
#
# Usage:
#   ./deploy/build-cloud.sh                    # Build only
#   ./deploy/build-cloud.sh push               # Build + push to ECR
#
# Environment variables:
#   AWS_ACCOUNT_ID   - AWS account ID (required for push)
#   AWS_REGION       - AWS region (default: eu-south-2)
#   IMAGE_TAG        - Image tag (default: latest)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

AWS_REGION="${AWS_REGION:-eu-south-2}"
IMAGE_TAG="${IMAGE_TAG:-latest}"
REPO_NAME="crab-cloud"

cd "$PROJECT_ROOT"

GIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

echo "==> Building crab-cloud Docker image (git: $GIT_HASH)"
docker build -f crab-cloud/Dockerfile --build-arg GIT_HASH="$GIT_HASH" -t "$REPO_NAME:$IMAGE_TAG" .

if [ "${1:-}" = "push" ]; then
    if [ -z "${AWS_ACCOUNT_ID:-}" ]; then
        AWS_ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
    fi

    ECR_URI="$AWS_ACCOUNT_ID.dkr.ecr.$AWS_REGION.amazonaws.com"

    echo "==> Logging in to ECR ($ECR_URI)"
    aws ecr get-login-password --region "$AWS_REGION" | \
        docker login --username AWS --password-stdin "$ECR_URI"

    echo "==> Tagging and pushing"
    docker tag "$REPO_NAME:$IMAGE_TAG" "$ECR_URI/$REPO_NAME:$IMAGE_TAG"
    docker push "$ECR_URI/$REPO_NAME:$IMAGE_TAG"

    echo "==> Pushed: $ECR_URI/$REPO_NAME:$IMAGE_TAG"
else
    echo "==> Image built: $REPO_NAME:$IMAGE_TAG"
    echo "    Run with 'push' argument to push to ECR"
fi
