#!/usr/bin/env bash
# Full deployment script for Crab SaaS
#
# Usage:
#   ./deploy/deploy.sh setup     # First-time: create CloudFormation stack
#   ./deploy/deploy.sh auth      # Update crab-auth Lambda code
#   ./deploy/deploy.sh cloud     # Build + push + deploy crab-cloud
#   ./deploy/deploy.sh all       # Deploy everything
#
# Required environment variables:
#   AWS_REGION           (default: eu-south-2)
#   DATABASE_URL         PostgreSQL connection string
#   STRIPE_SECRET_KEY    Stripe API key
#   STRIPE_WEBHOOK_SECRET  Stripe webhook secret
#   ACM_CERTIFICATE_ARN  ACM certificate ARN

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

AWS_REGION="${AWS_REGION:-eu-south-2}"
STACK_NAME="${STACK_NAME:-crab-production}"
LAMBDA_S3_BUCKET="${LAMBDA_S3_BUCKET:-crab-deploy-artifacts}"

cd "$PROJECT_ROOT"

deploy_auth() {
    echo "══════════════════════════════════════"
    echo "  Deploying crab-auth (Lambda)"
    echo "══════════════════════════════════════"

    # Build Lambda package
    "$SCRIPT_DIR/build-lambda.sh"

    # Upload to S3
    aws s3 cp "$SCRIPT_DIR/crab-auth-lambda.zip" \
        "s3://$LAMBDA_S3_BUCKET/crab-auth-lambda.zip" \
        --region "$AWS_REGION"

    # Update Lambda function code
    aws lambda update-function-code \
        --function-name crab-auth \
        --s3-bucket "$LAMBDA_S3_BUCKET" \
        --s3-key crab-auth-lambda.zip \
        --architectures arm64 \
        --region "$AWS_REGION"

    echo "==> crab-auth Lambda updated"
}

deploy_cloud() {
    echo "══════════════════════════════════════"
    echo "  Deploying crab-cloud (ECS Fargate)"
    echo "══════════════════════════════════════"

    # Build and push Docker image
    "$SCRIPT_DIR/build-cloud.sh" push

    # Force new deployment
    aws ecs update-service \
        --cluster crab-production \
        --service crab-cloud \
        --force-new-deployment \
        --region "$AWS_REGION"

    echo "==> crab-cloud ECS deployment triggered"
    echo "    Monitor: aws ecs describe-services --cluster crab-production --services crab-cloud"
}

setup_stack() {
    echo "══════════════════════════════════════"
    echo "  Creating CloudFormation stack"
    echo "══════════════════════════════════════"

    # Create S3 bucket for Lambda artifacts (if not exists)
    aws s3 mb "s3://$LAMBDA_S3_BUCKET" --region "$AWS_REGION" 2>/dev/null || true

    # Build and upload crab-auth first
    deploy_auth

    # Build and push crab-cloud image
    "$SCRIPT_DIR/build-cloud.sh" push

    AWS_ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
    ECR_URI="$AWS_ACCOUNT_ID.dkr.ecr.$AWS_REGION.amazonaws.com/crab-cloud:latest"

    aws cloudformation deploy \
        --template-file "$SCRIPT_DIR/cloudformation.yml" \
        --stack-name "$STACK_NAME" \
        --capabilities CAPABILITY_IAM \
        --region "$AWS_REGION" \
        --parameter-overrides \
            "DatabaseUrl=$DATABASE_URL" \
            "StripeSecretKey=$STRIPE_SECRET_KEY" \
            "StripeWebhookSecret=$STRIPE_WEBHOOK_SECRET" \
            "CrabCloudImageUri=$ECR_URI" \
            "CrabAuthCodeS3Bucket=$LAMBDA_S3_BUCKET" \
            "CertificateArn=$ACM_CERTIFICATE_ARN"

    echo ""
    echo "==> Stack outputs:"
    aws cloudformation describe-stacks \
        --stack-name "$STACK_NAME" \
        --query 'Stacks[0].Outputs' \
        --output table \
        --region "$AWS_REGION"
}

case "${1:-help}" in
    setup) setup_stack ;;
    auth)  deploy_auth ;;
    cloud) deploy_cloud ;;
    all)   deploy_auth && deploy_cloud ;;
    *)
        echo "Usage: $0 {setup|auth|cloud|all}"
        echo ""
        echo "  setup  - First-time CloudFormation stack creation"
        echo "  auth   - Update crab-auth Lambda code"
        echo "  cloud  - Build + push + deploy crab-cloud"
        echo "  all    - Deploy both services"
        exit 1
        ;;
esac
