CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE TABLE IF NOT EXISTS accounts (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 name text NOT NULL UNIQUE CHECK(length(name) BETWEEN 1 AND 100),
 password_hash text NOT NULL,
 admin boolean NOT NULL DEFAULT false,
 enabled boolean NOT NULL DEFAULT true
);
CREATE TABLE IF NOT EXISTS sessions (
 digest bytea PRIMARY KEY,
 account_id bigint NOT NULL REFERENCES accounts,
 expires timestamptz NOT NULL
);
CREATE TABLE IF NOT EXISTS connections (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 name text NOT NULL UNIQUE CHECK(length(name) BETWEEN 1 AND 100),
 engine text NOT NULL CHECK(engine IN ('mysql','postgres','mssql','mongo','redis')),
 environment text NOT NULL CHECK(environment IN ('test','production')),
 encrypted_dsn bytea NOT NULL
);
CREATE TABLE IF NOT EXISTS connection_read_grants (
 account_id bigint NOT NULL REFERENCES accounts,
 connection_id bigint NOT NULL REFERENCES connections,
 PRIMARY KEY(account_id,connection_id)
);
CREATE TABLE IF NOT EXISTS admin_events (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 occurred_at timestamptz NOT NULL DEFAULT now(),
 actor bigint NOT NULL REFERENCES accounts,
 action text NOT NULL,
 subject bigint NOT NULL,
 connection_id bigint REFERENCES connections
);
