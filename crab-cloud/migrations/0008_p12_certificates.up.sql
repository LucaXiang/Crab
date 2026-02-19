-- p12 certificates table (Verifactu electronic signature)
-- The actual .p12 file is stored in S3 with SSE-KMS encryption
-- The .p12 password is stored in AWS Secrets Manager (NOT in PG)

CREATE TABLE IF NOT EXISTS p12_certificates (
    tenant_id         TEXT PRIMARY KEY,
    s3_key            TEXT NOT NULL,
    fingerprint       TEXT,
    common_name       TEXT,
    serial_number     TEXT,
    organization_id   TEXT,
    organization      TEXT,
    issuer            TEXT,
    country           TEXT,
    expires_at        BIGINT,
    not_before        BIGINT,
    uploaded_at       BIGINT NOT NULL,
    updated_at        BIGINT NOT NULL
);
