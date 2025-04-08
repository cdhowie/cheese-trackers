CREATE TYPE authentication_source AS ENUM (
    'session_token',
    'api_key'
);

CREATE CAST (text AS authentication_source) WITH INOUT AS ASSIGNMENT;

ALTER TABLE audit ADD COLUMN auth_source authentication_source NULL;
