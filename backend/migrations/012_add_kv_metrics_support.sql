ALTER TABLE metrics_snapshots ADD COLUMN database_type TEXT NOT NULL DEFAULT 'postgres';
ALTER TABLE metrics_snapshots ADD COLUMN total_keys INTEGER DEFAULT NULL;
ALTER TABLE metrics_snapshots ADD COLUMN keyspace_hits INTEGER DEFAULT NULL;
ALTER TABLE metrics_snapshots ADD COLUMN keyspace_misses INTEGER DEFAULT NULL;
ALTER TABLE metrics_snapshots ADD COLUMN total_commands INTEGER DEFAULT NULL;
ALTER TABLE metrics_snapshots ADD COLUMN ops_per_sec REAL DEFAULT NULL;
ALTER TABLE metrics_snapshots ADD COLUMN used_memory INTEGER DEFAULT NULL;
ALTER TABLE metrics_snapshots ADD COLUMN connected_clients INTEGER DEFAULT NULL;

CREATE INDEX IF NOT EXISTS idx_metrics_snapshots_database_type
    ON metrics_snapshots(database_type);
