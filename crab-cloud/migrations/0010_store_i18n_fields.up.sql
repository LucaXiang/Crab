-- Add internationalization fields to stores table
ALTER TABLE stores ADD COLUMN currency_code TEXT;
ALTER TABLE stores ADD COLUMN currency_symbol TEXT;
ALTER TABLE stores ADD COLUMN currency_decimal_places INTEGER;
ALTER TABLE stores ADD COLUMN timezone TEXT;
ALTER TABLE stores ADD COLUMN receipt_header TEXT;
ALTER TABLE stores ADD COLUMN receipt_footer TEXT;
