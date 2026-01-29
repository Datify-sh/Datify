CREATE TABLE IF NOT EXISTS databases (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    postgres_version TEXT NOT NULL DEFAULT '16',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE(project_id, name)
);

CREATE INDEX IF NOT EXISTS idx_databases_project_id ON databases(project_id);

CREATE TRIGGER IF NOT EXISTS databases_updated_at
    AFTER UPDATE ON databases
    FOR EACH ROW
BEGIN
    UPDATE databases SET updated_at = datetime('now') WHERE id = OLD.id;
END;
