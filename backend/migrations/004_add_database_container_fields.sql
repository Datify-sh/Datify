ALTER TABLE databases ADD COLUMN container_id TEXT;
ALTER TABLE databases ADD COLUMN container_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE databases ADD COLUMN host TEXT;
ALTER TABLE databases ADD COLUMN port INTEGER;
ALTER TABLE databases ADD COLUMN username TEXT NOT NULL DEFAULT 'postgres';
ALTER TABLE databases ADD COLUMN password_encrypted TEXT;
ALTER TABLE databases ADD COLUMN cpu_limit REAL NOT NULL DEFAULT 1.0;
ALTER TABLE databases ADD COLUMN memory_limit_mb INTEGER NOT NULL DEFAULT 512;
ALTER TABLE databases ADD COLUMN storage_limit_mb INTEGER NOT NULL DEFAULT 1024;
