#!/usr/bin/env bash
# Initialize Secrets Manager secrets with actual values
#
# Run ONCE after CloudFormation stack creation.
# CloudFormation creates empty secrets; this script sets the values.
#
# Usage:
#   ./deploy/setup-secrets.sh
#
# You will be prompted for each secret value interactively.

set -euo pipefail

AWS_REGION="${AWS_REGION:-eu-south-2}"
ENVIRONMENT="${ENVIRONMENT:-production}"

echo "═══════════════════════════════════════════"
echo "  Crab SaaS — Secret Initialization"
echo "  Environment: $ENVIRONMENT"
echo "  Region: $AWS_REGION"
echo "═══════════════════════════════════════════"
echo ""

# Function to set a secret value
set_secret() {
    local name="$1"
    local description="$2"

    echo "── $description ──"
    echo "Secret: crab/$ENVIRONMENT/$name"
    echo -n "Value: "
    read -rs value
    echo ""

    if [ -z "$value" ]; then
        echo "  ⚠ Skipped (empty value)"
        return
    fi

    aws secretsmanager put-secret-value \
        --secret-id "crab/$ENVIRONMENT/$name" \
        --secret-string "$value" \
        --region "$AWS_REGION" \
        > /dev/null

    echo "  ✓ Set successfully"
    echo ""
}

set_secret "database-url" "PostgreSQL connection string (postgres://user:pass@host:5432/crab)"
set_secret "stripe-secret-key" "Stripe API secret key (sk_live_... or sk_test_...)"
set_secret "stripe-webhook-secret" "Stripe webhook signing secret (whsec_...)"
set_secret "jwt-secret" "JWT signing secret for tenant auth (min 32 chars, e.g. openssl rand -hex 32)"

echo ""
echo "═══════════════════════════════════════════"
echo "  ✓ All secrets configured"
echo ""
echo "  Verify with:"
echo "    aws secretsmanager list-secrets --region $AWS_REGION \\"
echo "      --filter Key=name,Values=crab/$ENVIRONMENT"
echo "═══════════════════════════════════════════"
