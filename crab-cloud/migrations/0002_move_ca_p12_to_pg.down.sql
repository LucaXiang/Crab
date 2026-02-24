ALTER TABLE tenants DROP COLUMN ca_cert_pem;
ALTER TABLE tenants DROP COLUMN ca_key_pem;

ALTER TABLE p12_certificates DROP COLUMN p12_data;
ALTER TABLE p12_certificates DROP COLUMN p12_password;
ALTER TABLE p12_certificates ADD COLUMN secret_name TEXT NOT NULL DEFAULT '';
