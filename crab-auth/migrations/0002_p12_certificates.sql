-- p12 certificates table (Verifactu electronic signature)
-- Each tenant uploads their .p12 file for invoice signing
-- The actual .p12 file is stored in S3 with SSE-KMS encryption

CREATE TABLE IF NOT EXISTS p12_certificates (
    tenant_id         TEXT PRIMARY KEY,
    s3_key            TEXT NOT NULL,
    p12_password      TEXT NOT NULL,
    fingerprint       TEXT,
    subject           TEXT,
    expires_at        BIGINT,
    uploaded_at       BIGINT NOT NULL,
    updated_at        BIGINT NOT NULL
);
