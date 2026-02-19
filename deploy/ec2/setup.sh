#!/bin/bash
set -euo pipefail

# ════════════════════════════════════════════
# Crab Cloud — EC2 Setup Script
# Run on a fresh Amazon Linux 2023 instance
# ════════════════════════════════════════════

AWS_REGION="eu-south-2"
AWS_ACCOUNT_ID="364453382269"
ECR_REGISTRY="${AWS_ACCOUNT_ID}.dkr.ecr.${AWS_REGION}.amazonaws.com"

echo "=== Installing Docker ==="
sudo dnf update -y
sudo dnf install -y docker
sudo systemctl enable docker
sudo systemctl start docker
sudo usermod -aG docker ec2-user

echo "=== Installing Docker Compose ==="
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

echo "=== Installing AWS CLI (for ECR login) ==="
# Amazon Linux 2023 has AWS CLI pre-installed

echo "=== Setting up app directory ==="
sudo mkdir -p /opt/crab/{certs,data}
sudo chown -R ec2-user:ec2-user /opt/crab

echo "=== ECR Login ==="
aws ecr get-login-password --region "$AWS_REGION" | docker login --username AWS --password-stdin "$ECR_REGISTRY"

echo "=== Pulling crab-cloud image ==="
docker pull "${ECR_REGISTRY}/crab-cloud:latest"

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo "1. Copy docker-compose.yml and .env to /opt/crab/"
echo "2. Copy mTLS certs to /opt/crab/certs/"
echo "3. cd /opt/crab && docker-compose up -d"
echo ""
