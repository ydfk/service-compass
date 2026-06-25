ALTER TABLE monitors ADD COLUMN request_body_type TEXT NOT NULL DEFAULT 'json';
ALTER TABLE monitors ADD COLUMN request_body_secret TEXT;
ALTER TABLE monitors ADD COLUMN request_headers_secret TEXT;
