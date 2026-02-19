#!/bin/bash
# PostgreSQL daily backup → S3
# Usage: crontab -e → 0 2 * * * /opt/crab/backup.sh

set -euo pipefail

BACKUP_DIR="/opt/crab/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="crab-${TIMESTAMP}.sql.gz"
S3_BUCKET="crab-backups"

mkdir -p "${BACKUP_DIR}"

# Dump + compress
docker exec crab-postgres pg_dump -U crab crab | gzip > "${BACKUP_DIR}/${BACKUP_FILE}"

# Upload to S3
aws s3 cp "${BACKUP_DIR}/${BACKUP_FILE}" "s3://${S3_BUCKET}/pg/${BACKUP_FILE}"

# Clean local backups older than 7 days
find "${BACKUP_DIR}" -name "*.sql.gz" -mtime +7 -delete

echo "[$(date)] Backup completed: ${BACKUP_FILE}"
