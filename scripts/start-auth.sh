#!/bin/bash
# crab-auth 本地开发启动脚本
# 依赖: Docker, cargo-lambda
#
# 用法:
#   ./scripts/start-auth.sh          # 启动 PG + LocalStack + cargo lambda watch
#   ./scripts/start-auth.sh db       # 只启动 PG + LocalStack
#   ./scripts/start-auth.sh reset    # 重置所有容器
#   ./scripts/start-auth.sh stop     # 停止所有容器

set -euo pipefail

PG_CONTAINER="crab-pg"
LS_CONTAINER="crab-localstack"
PG_USER="crab"
PG_PASS="crab"
PG_DB="crab_auth"
PG_PORT="5432"
LS_PORT="4566"
TEST_PASSWORD="test123"

DATABASE_URL="postgres://${PG_USER}:${PG_PASS}@localhost:${PG_PORT}/${PG_DB}"

# ============================================================================
# 颜色输出
# ============================================================================
info()  { echo -e "\033[1;34m[INFO]\033[0m  $*"; }
ok()    { echo -e "\033[1;32m[OK]\033[0m    $*"; }
warn()  { echo -e "\033[1;33m[WARN]\033[0m  $*"; }
err()   { echo -e "\033[1;31m[ERROR]\033[0m $*"; }

# ============================================================================
# PostgreSQL
# ============================================================================
start_pg() {
    if docker ps -q -f name="${PG_CONTAINER}" | grep -q .; then
        ok "PostgreSQL already running"
        return
    fi

    if docker ps -aq -f name="${PG_CONTAINER}" | grep -q .; then
        info "Starting existing PG container..."
        docker start "${PG_CONTAINER}" > /dev/null
    else
        info "Creating PostgreSQL container..."
        docker run -d \
            --name "${PG_CONTAINER}" \
            -e POSTGRES_USER="${PG_USER}" \
            -e POSTGRES_PASSWORD="${PG_PASS}" \
            -e POSTGRES_DB="${PG_DB}" \
            -p "${PG_PORT}:5432" \
            postgres:16 > /dev/null
    fi

    info "Waiting for PostgreSQL..."
    for i in $(seq 1 30); do
        if docker exec "${PG_CONTAINER}" pg_isready -U "${PG_USER}" -d "${PG_DB}" > /dev/null 2>&1; then
            ok "PostgreSQL ready (port ${PG_PORT})"
            return
        fi
        sleep 0.5
    done
    err "PostgreSQL failed to start within 15s"
    exit 1
}

# ============================================================================
# LocalStack (Secrets Manager 模拟)
# ============================================================================
start_localstack() {
    if docker ps -q -f name="${LS_CONTAINER}" | grep -q .; then
        ok "LocalStack already running"
        return
    fi

    if docker ps -aq -f name="${LS_CONTAINER}" | grep -q .; then
        info "Starting existing LocalStack container..."
        docker start "${LS_CONTAINER}" > /dev/null
    else
        info "Creating LocalStack container..."
        docker run -d \
            --name "${LS_CONTAINER}" \
            -e SERVICES=secretsmanager,s3 \
            -p "${LS_PORT}:4566" \
            localstack/localstack > /dev/null
    fi

    info "Waiting for LocalStack..."
    for i in $(seq 1 30); do
        if curl -s "http://localhost:${LS_PORT}/_localstack/health" | grep -q '"secretsmanager": "available"' 2>/dev/null; then
            ok "LocalStack ready (port ${LS_PORT})"
            return
        fi
        sleep 1
    done
    err "LocalStack failed to start within 30s"
    exit 1
}

# ============================================================================
# 数据库种子
# ============================================================================
seed_db() {
    info "Creating external tables and seed data..."

    local hash
    hash=$(cargo run --example gen_hash -p crab-auth --quiet -- "${TEST_PASSWORD}" 2>/dev/null)

    docker exec -i "${PG_CONTAINER}" psql -U "${PG_USER}" -d "${PG_DB}" <<SQL
-- 外部表: tenants (SaaS 管理平台维护)
CREATE TABLE IF NOT EXISTS tenants (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    hashed_password TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active'
);

-- 外部表: subscriptions (Stripe webhook 维护)
CREATE TABLE IF NOT EXISTS subscriptions (
    id                TEXT PRIMARY KEY,
    tenant_id         TEXT NOT NULL REFERENCES tenants(id),
    status            TEXT NOT NULL DEFAULT 'active',
    plan              TEXT NOT NULL DEFAULT 'pro',
    max_edge_servers  INT NOT NULL DEFAULT 3,
    max_clients       INT NOT NULL DEFAULT 10,
    features          TEXT[] NOT NULL DEFAULT '{}',
    current_period_end BIGINT,
    created_at        BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000)::BIGINT
);

-- 测试租户 (用户名: demo, 密码: ${TEST_PASSWORD})
INSERT INTO tenants (id, name, hashed_password, status)
VALUES ('demo', 'Demo Restaurant', '${hash}', 'active')
ON CONFLICT (id) DO UPDATE SET hashed_password = EXCLUDED.hashed_password;

-- 测试订阅 (Pro plan, 有效期 1 年)
INSERT INTO subscriptions (id, tenant_id, status, plan, max_edge_servers, max_clients, features, current_period_end)
VALUES (
    'sub_demo', 'demo', 'active', 'pro', 3, 10,
    ARRAY['audit_log', 'advanced_reporting', 'api_access'],
    (EXTRACT(EPOCH FROM NOW() + INTERVAL '365 days') * 1000)::BIGINT
)
ON CONFLICT (id) DO UPDATE SET
    status = EXCLUDED.status,
    current_period_end = EXCLUDED.current_period_end;
SQL

    ok "Seeded: tenant=demo, password=${TEST_PASSWORD}"
}

# ============================================================================
# 子命令
# ============================================================================
cmd_stop() {
    for c in "${PG_CONTAINER}" "${LS_CONTAINER}"; do
        if docker ps -q -f name="${c}" | grep -q .; then
            info "Stopping ${c}..."
            docker stop "${c}" > /dev/null
            ok "${c} stopped"
        else
            warn "${c} is not running"
        fi
    done
}

cmd_reset() {
    for c in "${PG_CONTAINER}" "${LS_CONTAINER}"; do
        if docker ps -a -q -f name="${c}" | grep -q .; then
            info "Removing ${c}..."
            docker rm -f "${c}" > /dev/null
        fi
    done
    start_pg
    start_localstack
    seed_db
    ok "All containers reset"
}

cmd_db() {
    start_pg
    start_localstack
    seed_db
    echo ""
    ok "Infrastructure ready"
    info "DATABASE_URL=${DATABASE_URL}"
    info "AWS_ENDPOINT_URL=http://localhost:${LS_PORT}"
    echo ""
    info "Run crab-auth manually with:"
    echo "  DATABASE_URL=${DATABASE_URL} \\"
    echo "  AWS_ENDPOINT_URL=http://localhost:${LS_PORT} \\"
    echo "  AWS_ACCESS_KEY_ID=test \\"
    echo "  AWS_SECRET_ACCESS_KEY=test \\"
    echo "  AWS_REGION=us-east-1 \\"
    echo "  cargo lambda watch -p crab-auth"
}

cmd_start() {
    start_pg
    start_localstack
    seed_db

    echo ""
    info "Starting crab-auth (cargo lambda watch)..."
    info "HTTP endpoint: http://localhost:3001"
    echo ""

    DATABASE_URL="${DATABASE_URL}" \
    AWS_ENDPOINT_URL="http://localhost:${LS_PORT}" \
    AWS_ACCESS_KEY_ID="test" \
    AWS_SECRET_ACCESS_KEY="test" \
    AWS_REGION="us-east-1" \
    cargo lambda watch --package crab-auth --invoke-address 127.0.0.1 --invoke-port 3001
}

# ============================================================================
# 入口
# ============================================================================
case "${1:-start}" in
    start) cmd_start ;;
    db)    cmd_db ;;
    reset) cmd_reset ;;
    stop)  cmd_stop ;;
    *)
        echo "Usage: $0 {start|db|reset|stop}"
        echo "  start  - Start PG + LocalStack + cargo lambda watch (default)"
        echo "  db     - Start PG + LocalStack only, print env vars"
        echo "  reset  - Reset all containers and reseed"
        echo "  stop   - Stop all containers"
        exit 1
        ;;
esac
