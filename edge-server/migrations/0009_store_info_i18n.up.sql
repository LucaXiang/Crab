-- Add internationalization fields to store_info
ALTER TABLE store_info ADD COLUMN currency_code TEXT;
ALTER TABLE store_info ADD COLUMN currency_symbol TEXT;
ALTER TABLE store_info ADD COLUMN currency_decimal_places INTEGER;
ALTER TABLE store_info ADD COLUMN timezone TEXT;
ALTER TABLE store_info ADD COLUMN receipt_header TEXT;
ALTER TABLE store_info ADD COLUMN receipt_footer TEXT;
