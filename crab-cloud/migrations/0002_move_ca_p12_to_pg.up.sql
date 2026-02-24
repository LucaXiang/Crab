-- Tenant CA: store cert+key directly in tenants table
ALTER TABLE tenants ADD COLUMN ca_cert_pem TEXT;
ALTER TABLE tenants ADD COLUMN ca_key_pem TEXT;

-- P12: store binary data + password directly, remove SM reference
ALTER TABLE p12_certificates ADD COLUMN p12_data TEXT;
ALTER TABLE p12_certificates ADD COLUMN p12_password TEXT;
ALTER TABLE p12_certificates DROP COLUMN secret_name;
