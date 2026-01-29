-- Add branch support fields to databases table
ALTER TABLE databases ADD COLUMN parent_branch_id TEXT REFERENCES databases(id) ON DELETE SET NULL;
ALTER TABLE databases ADD COLUMN branch_name TEXT NOT NULL DEFAULT 'main';
ALTER TABLE databases ADD COLUMN is_default_branch INTEGER NOT NULL DEFAULT 1;
ALTER TABLE databases ADD COLUMN forked_at TEXT;

CREATE INDEX IF NOT EXISTS idx_databases_parent_branch ON databases(parent_branch_id);
CREATE INDEX IF NOT EXISTS idx_databases_branch_name ON databases(project_id, branch_name);
