#!/bin/bash
# PostgreSQL daily backup (local + optional S3)
# Triggered by systemd timer: crab-backup.timer

set -euo pipefail

BACKUP_DIR="/opt/crab/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="crab-${TIMESTAMP}.sql.gz"
S3_BUCKET="crab-backups"

mkdir -p "${BACKUP_DIR}"

# Find postgres container name dynamically
PG_CONTAINER=$(docker ps --filter "name=postgres" --format "{{.Names}}" | head -1)
if [ -z "${PG_CONTAINER}" ]; then
    echo "[$(date)] ERROR: PostgreSQL container not found"
    exit 1
fi

# Dump + compress
docker exec "${PG_CONTAINER}" pg_dump -U crab crab | gzip > "${BACKUP_DIR}/${BACKUP_FILE}"

# Upload to S3 (skip if no permission or bucket doesn't exist)
aws s3 cp "${BACKUP_DIR}/${BACKUP_FILE}" "s3://${S3_BUCKET}/pg/${BACKUP_FILE}" 2>/dev/null || true

# Clean local backups older than 7 days
find "${BACKUP_DIR}" -name "*.sql.gz" -mtime +7 -delete

echo "[$(date)] Backup completed: ${BACKUP_FILE} ($(du -h "${BACKUP_DIR}/${BACKUP_FILE}" | cut -f1))"
