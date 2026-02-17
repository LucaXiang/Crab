#!/usr/bin/env bash
# Full deployment script for Crab SaaS
#
# Usage:
#   ./deploy/deploy.sh setup     # First-time: create CloudFormation stack
#   ./deploy/deploy.sh secrets   # Initialize Secrets Manager values
#   ./deploy/deploy.sh auth      # Update crab-auth Lambda code
#   ./deploy/deploy.sh cloud     # Build + push + deploy crab-cloud
#   ./deploy/deploy.sh all       # Deploy both services
#   ./deploy/deploy.sh status    # Check deployment health
#
# Required environment variables:
#   AWS_REGION           (default: eu-south-2)
#   ACM_CERTIFICATE_ARN  ACM certificate ARN (for setup)
#   ALERT_EMAIL          Alarm notification email (for setup)
#
# Sensitive values are stored in Secrets Manager, NOT environment variables.

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

    "$SCRIPT_DIR/build-lambda.sh"

    aws s3 cp "$SCRIPT_DIR/crab-auth-lambda.zip" \
        "s3://$LAMBDA_S3_BUCKET/crab-auth-lambda.zip" \
        --region "$AWS_REGION"

    aws lambda update-function-code \
        --function-name crab-auth \
        --s3-bucket "$LAMBDA_S3_BUCKET" \
        --s3-key crab-auth-lambda.zip \
        --architectures arm64 \
        --region "$AWS_REGION"

    aws lambda wait function-updated \
        --function-name crab-auth \
        --region "$AWS_REGION"

    echo "==> crab-auth Lambda updated"
}

deploy_cloud() {
    echo "══════════════════════════════════════"
    echo "  Deploying crab-cloud (ECS Fargate)"
    echo "══════════════════════════════════════"

    "$SCRIPT_DIR/build-cloud.sh" push

    aws ecs update-service \
        --cluster "$STACK_NAME" \
        --service crab-cloud \
        --force-new-deployment \
        --region "$AWS_REGION" \
        > /dev/null

    echo "==> Waiting for ECS deployment to stabilize..."
    aws ecs wait services-stable \
        --cluster "$STACK_NAME" \
        --services crab-cloud \
        --region "$AWS_REGION"

    echo "==> crab-cloud deployed successfully"
}

setup_stack() {
    echo "══════════════════════════════════════"
    echo "  Creating CloudFormation stack"
    echo "══════════════════════════════════════"

    # Validate required env vars
    : "${ACM_CERTIFICATE_ARN:?ACM_CERTIFICATE_ARN is required}"

    # Create S3 bucket for Lambda artifacts
    aws s3 mb "s3://$LAMBDA_S3_BUCKET" --region "$AWS_REGION" 2>/dev/null || true

    # Build and upload crab-auth
    deploy_auth

    # Build and push crab-cloud image
    "$SCRIPT_DIR/build-cloud.sh" push

    AWS_ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
    ECR_URI="$AWS_ACCOUNT_ID.dkr.ecr.$AWS_REGION.amazonaws.com/crab-cloud:latest"

    # No sensitive params — all secrets managed via Secrets Manager
    aws cloudformation deploy \
        --template-file "$SCRIPT_DIR/cloudformation.yml" \
        --stack-name "$STACK_NAME" \
        --capabilities CAPABILITY_IAM \
        --region "$AWS_REGION" \
        --tags Project=crab Environment=production \
        --parameter-overrides \
            "CrabCloudImageUri=$ECR_URI" \
            "CrabAuthCodeS3Bucket=$LAMBDA_S3_BUCKET" \
            "CertificateArn=$ACM_CERTIFICATE_ARN" \
            "AlertEmail=${ALERT_EMAIL:-ops@crab.es}"

    echo ""
    echo "==> Stack outputs:"
    aws cloudformation describe-stacks \
        --stack-name "$STACK_NAME" \
        --query 'Stacks[0].Outputs' \
        --output table \
        --region "$AWS_REGION"

    echo ""
    echo "==> NEXT STEP: Run './deploy/deploy.sh secrets' to set secret values"
}

check_status() {
    echo "══════════════════════════════════════"
    echo "  Deployment Health Check"
    echo "══════════════════════════════════════"
    echo ""

    # ECS
    echo "── crab-cloud (ECS) ──"
    aws ecs describe-services \
        --cluster "$STACK_NAME" \
        --services crab-cloud \
        --query 'services[0].{Status:status,Running:runningCount,Desired:desiredCount,Health:healthCheckGracePeriodSeconds}' \
        --output table \
        --region "$AWS_REGION" 2>/dev/null || echo "  Not deployed yet"

    echo ""

    # Lambda
    echo "── crab-auth (Lambda) ──"
    aws lambda get-function \
        --function-name crab-auth \
        --query 'Configuration.{State:State,LastModified:LastModified,Memory:MemorySize,Timeout:Timeout}' \
        --output table \
        --region "$AWS_REGION" 2>/dev/null || echo "  Not deployed yet"

    echo ""

    # RDS
    echo "── RDS PostgreSQL ──"
    aws rds describe-db-instances \
        --db-instance-identifier "crab-production" \
        --query 'DBInstances[0].{Status:DBInstanceStatus,Engine:Engine,Class:DBInstanceClass,Storage:AllocatedStorage}' \
        --output table \
        --region "$AWS_REGION" 2>/dev/null || echo "  Not created yet"

    echo ""

    # Alarms
    echo "── Active Alarms ──"
    aws cloudwatch describe-alarms \
        --alarm-name-prefix "crab-" \
        --state-value ALARM \
        --query 'MetricAlarms[].{Name:AlarmName,State:StateValue}' \
        --output table \
        --region "$AWS_REGION" 2>/dev/null || echo "  None"
}

case "${1:-help}" in
    setup)   setup_stack ;;
    secrets) "$SCRIPT_DIR/setup-secrets.sh" ;;
    auth)    deploy_auth ;;
    cloud)   deploy_cloud ;;
    all)     deploy_auth && deploy_cloud ;;
    status)  check_status ;;
    *)
        echo "Usage: $0 {setup|secrets|auth|cloud|all|status}"
        echo ""
        echo "  setup    - First-time CloudFormation stack creation"
        echo "  secrets  - Initialize Secrets Manager values (run after setup)"
        echo "  auth     - Update crab-auth Lambda code"
        echo "  cloud    - Build + push + deploy crab-cloud"
        echo "  all      - Deploy both services"
        echo "  status   - Check deployment health"
        exit 1
        ;;
esac
