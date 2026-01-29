ALTER TABLE databases ADD COLUMN database_type TEXT NOT NULL DEFAULT 'postgres';
ALTER TABLE databases ADD COLUMN valkey_version TEXT;
CREATE INDEX idx_databases_type ON databases(database_type);
