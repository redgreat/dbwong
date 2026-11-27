ALTER TABLE connections ADD COLUMN IF NOT EXISTS enabled boolean NOT NULL DEFAULT true;
