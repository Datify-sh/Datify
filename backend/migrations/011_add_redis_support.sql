ALTER TABLE databases ADD COLUMN redis_version TEXT;

CREATE INDEX IF NOT EXISTS idx_databases_redis_version ON databases(redis_version);
