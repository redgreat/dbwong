ALTER TABLE connections ADD COLUMN IF NOT EXISTS deleted_at timestamptz;
ALTER TABLE connections DROP CONSTRAINT IF EXISTS connections_name_key;
CREATE UNIQUE INDEX IF NOT EXISTS connections_active_name ON connections(name) WHERE deleted_at IS NULL;
