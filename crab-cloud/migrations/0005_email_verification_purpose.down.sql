ALTER TABLE email_verifications DROP CONSTRAINT email_verifications_pkey;
ALTER TABLE email_verifications ADD PRIMARY KEY (email);
ALTER TABLE email_verifications DROP COLUMN purpose;
