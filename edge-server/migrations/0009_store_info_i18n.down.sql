-- SQLite doesn't support DROP COLUMN before 3.35.0, but we use recent SQLite
ALTER TABLE store_info DROP COLUMN currency_code;
ALTER TABLE store_info DROP COLUMN currency_symbol;
ALTER TABLE store_info DROP COLUMN currency_decimal_places;
ALTER TABLE store_info DROP COLUMN timezone;
ALTER TABLE store_info DROP COLUMN receipt_header;
ALTER TABLE store_info DROP COLUMN receipt_footer;
