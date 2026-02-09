#!/bin/bash
# crab-auth 本地开发启动脚本
# 依赖: Docker, PostgreSQL 16 镜像
#
# 用法:
#   ./scripts/start-auth.sh          # 启动 PG + crab-auth
#   ./scripts/start-auth.sh db       # 只启动 PG (不启动 crab-auth)
#   ./scripts/start-auth.sh reset    # 重置数据库
#   ./scripts/start-auth.sh stop     # 停止 PG 容器

set -euo pipefail

CONTAINER_NAME="crab-pg"
PG_USER="crab"
PG_PASS="crab"
PG_DB="crab_auth"
PG_PORT="5432"
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
# 子命令
# ============================================================================
cmd_stop() {
    if docker ps -q -f name="${CONTAINER_NAME}" | grep -q .; then
        info "Stopping ${CONTAINER_NAME}..."
        docker stop "${CONTAINER_NAME}" > /dev/null
        ok "Container stopped"
    else
        warn "Container ${CONTAINER_NAME} is not running"
    fi
}

cmd_reset() {
    if docker ps -a -q -f name="${CONTAINER_NAME}" | grep -q .; then
        info "Removing ${CONTAINER_NAME}..."
        docker rm -f "${CONTAINER_NAME}" > /dev/null
    fi
    start_pg
    seed_db
    ok "Database reset complete"
}

start_pg() {
    # 检查容器是否已在运行
    if docker ps -q -f name="${CONTAINER_NAME}" | grep -q .; then
        ok "PostgreSQL already running"
        return
    fi

    # 检查容器是否存在但已停止
    if docker ps -aq -f name="${CONTAINER_NAME}" | grep -q .; then
        info "Starting existing container..."
        docker start "${CONTAINER_NAME}" > /dev/null
    else
        info "Creating PostgreSQL container..."
        docker run -d \
            --name "${CONTAINER_NAME}" \
            -e POSTGRES_USER="${PG_USER}" \
            -e POSTGRES_PASSWORD="${PG_PASS}" \
            -e POSTGRES_DB="${PG_DB}" \
            -p "${PG_PORT}:5432" \
            postgres:16 > /dev/null
    fi

    # 等待 PG 就绪
    info "Waiting for PostgreSQL..."
    for i in $(seq 1 30); do
        if docker exec "${CONTAINER_NAME}" pg_isready -U "${PG_USER}" -d "${PG_DB}" > /dev/null 2>&1; then
            ok "PostgreSQL ready (port ${PG_PORT})"
            return
        fi
        sleep 0.5
    done
    err "PostgreSQL failed to start within 15s"
    exit 1
}

seed_db() {
    info "Creating external tables and seed data..."

    # 生成密码 hash
    local hash
    hash=$(cargo run --example gen_hash -p crab-auth --quiet -- "${TEST_PASSWORD}" 2>/dev/null)

    docker exec -i "${CONTAINER_NAME}" psql -U "${PG_USER}" -d "${PG_DB}" <<SQL
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
    features          TEXT[] NOT NULL DEFAULT '{}',
    current_period_end BIGINT,
    created_at        BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000)::BIGINT
);

-- 测试租户 (用户名: demo, 密码: ${TEST_PASSWORD})
INSERT INTO tenants (id, name, hashed_password, status)
VALUES ('demo', 'Demo Restaurant', '${hash}', 'active')
ON CONFLICT (id) DO UPDATE SET hashed_password = EXCLUDED.hashed_password;

-- 测试订阅 (Pro plan, 有效期 1 年)
INSERT INTO subscriptions (id, tenant_id, status, plan, max_edge_servers, features, current_period_end)
VALUES (
    'sub_demo', 'demo', 'active', 'pro', 3,
    ARRAY['audit_log', 'advanced_reporting', 'api_access'],
    (EXTRACT(EPOCH FROM NOW() + INTERVAL '365 days') * 1000)::BIGINT
)
ON CONFLICT (id) DO UPDATE SET
    status = EXCLUDED.status,
    current_period_end = EXCLUDED.current_period_end;
SQL

    ok "Seeded: tenant=demo, password=${TEST_PASSWORD}"
}

cmd_db() {
    start_pg
    seed_db
    echo ""
    ok "DATABASE_URL=${DATABASE_URL}"
    info "Run crab-auth manually with:"
    echo "  DATABASE_URL=${DATABASE_URL} cargo run -p crab-auth"
}

cmd_start() {
    start_pg
    seed_db

    echo ""
    info "Starting crab-auth..."
    echo ""

    DATABASE_URL="${DATABASE_URL}" \
    AUTH_STORAGE_PATH="./auth_storage" \
    AWS_ACCESS_KEY_ID="test" \
    AWS_SECRET_ACCESS_KEY="test" \
    AWS_DEFAULT_REGION="us-east-1" \
    cargo run -p crab-auth
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
        echo "  start  - Start PG + crab-auth (default)"
        echo "  db     - Start PG only, print DATABASE_URL"
        echo "  reset  - Reset PG container and reseed"
        echo "  stop   - Stop PG container"
        exit 1
        ;;
esac
