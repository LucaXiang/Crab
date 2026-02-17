ALTER TABLE email_verifications ADD COLUMN purpose TEXT NOT NULL DEFAULT 'registration';
ALTER TABLE email_verifications DROP CONSTRAINT email_verifications_pkey;
ALTER TABLE email_verifications ADD PRIMARY KEY (email, purpose);
