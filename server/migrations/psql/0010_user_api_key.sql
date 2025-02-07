ALTER TABLE ct_user ADD COLUMN api_key UUID NULL;
ALTER TABLE ct_user ADD CONSTRAINT api_key_idx UNIQUE (api_key);
