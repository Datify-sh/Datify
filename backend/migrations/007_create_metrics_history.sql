-- Metrics snapshots table for storing historical metrics data
CREATE TABLE IF NOT EXISTS metrics_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    database_id TEXT NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    -- Query metrics
    total_queries INTEGER NOT NULL DEFAULT 0,
    queries_per_sec REAL NOT NULL DEFAULT 0.0,
    avg_latency_ms REAL NOT NULL DEFAULT 0.0,
    -- Row metrics
    rows_read INTEGER NOT NULL DEFAULT 0,
    rows_written INTEGER NOT NULL DEFAULT 0,
    -- Resource metrics
    cpu_percent REAL NOT NULL DEFAULT 0.0,
    memory_percent REAL NOT NULL DEFAULT 0.0,
    memory_used_bytes INTEGER NOT NULL DEFAULT 0,
    -- Connection metrics
    active_connections INTEGER NOT NULL DEFAULT 0,
    -- Storage metrics
    storage_used_bytes INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (database_id) REFERENCES databases(id) ON DELETE CASCADE
);

-- Index for efficient time-range queries per database
CREATE INDEX IF NOT EXISTS idx_metrics_snapshots_database_timestamp
    ON metrics_snapshots(database_id, timestamp DESC);

-- Index for cleanup queries (old data)
CREATE INDEX IF NOT EXISTS idx_metrics_snapshots_timestamp
    ON metrics_snapshots(timestamp);

-- Trigger to auto-cleanup metrics older than 24 hours
-- This runs on every insert to keep the table size manageable
CREATE TRIGGER IF NOT EXISTS metrics_snapshots_cleanup
    AFTER INSERT ON metrics_snapshots
    FOR EACH ROW
BEGIN
    DELETE FROM metrics_snapshots
    WHERE timestamp < datetime('now', '-24 hours');
END;
