#!/usr/bin/env bash
# Deploy dev environment ONLY. Uses docker-compose.dev.yml — physically isolated from prod.
#
# Usage:
#   ./deploy/deploy-dev.sh cloud           # deploy dev-cloud only
#   ./deploy/deploy-dev.sh console         # deploy dev-console only
#   ./deploy/deploy-dev.sh all             # deploy both
#   ./deploy/deploy-dev.sh reset-db        # reset dev database only
#
set -euo pipefail

EC2_KEY="deploy/ec2/crab-ec2.pem"
EC2_HOST="ec2-user@51.92.72.162"
ECR="364453382269.dkr.ecr.eu-south-2.amazonaws.com"
SSH="ssh -i $EC2_KEY $EC2_HOST"
SCP="scp -i $EC2_KEY"

# Dev uses its own compose file — cannot touch prod
DEV_COMPOSE="cd /opt/crab && docker-compose -f docker-compose.dev.yml"

die()  { echo "ERROR: $*" >&2; exit 1; }
info() { echo "==> $*"; }

deploy_cloud() {
  info "Building and pushing crab-cloud image..."
  ./deploy/build-cloud.sh push

  info "Pulling image and restarting dev-cloud on EC2..."
  $SSH "
    aws ecr get-login-password --region eu-south-2 | \
      docker login --username AWS --password-stdin $ECR && \
    docker pull $ECR/crab-cloud:latest && \
    $DEV_COMPOSE up -d dev-cloud
  "

  info "Waiting for dev-cloud to start..."
  sleep 5
  local health
  health=$($SSH "curl -sf http://localhost:8081/health" 2>/dev/null || echo "FAIL")
  if echo "$health" | grep -q '"status":"ok"'; then
    info "dev-cloud healthy: $health"
  else
    echo "WARNING: dev-cloud health check failed. Check logs:"
    echo "  ssh -i $EC2_KEY $EC2_HOST '$DEV_COMPOSE logs --tail=20 dev-cloud'"
  fi
}

deploy_console() {
  info "Building dev-console (mode=development)..."
  cd crab-console
  npx vite build --mode development
  cp build/index.html build/200.html
  cd ..

  info "Uploading to EC2..."
  $SCP -r crab-console/build/* $EC2_HOST:/opt/crab/dev-console/

  info "dev-console deployed."
}

reset_db() {
  info "Resetting dev database (dev_pgdata only)..."
  $SSH "
    $DEV_COMPOSE stop dev-cloud dev-postgres && \
    $DEV_COMPOSE rm -f dev-cloud dev-postgres && \
    docker volume rm crab_dev_pgdata && \
    $DEV_COMPOSE up -d dev-postgres && \
    sleep 5 && \
    $DEV_COMPOSE up -d dev-cloud
  "
  info "Dev database reset complete."
}

case "${1:-}" in
  cloud)   deploy_cloud ;;
  console) deploy_console ;;
  all)     deploy_cloud; deploy_console ;;
  reset-db) reset_db ;;
  *)       die "Usage: $0 {cloud|console|all|reset-db}" ;;
esac
