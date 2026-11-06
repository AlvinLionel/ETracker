CREATE TABLE imported_record_allocations (
    id TEXT PRIMARY KEY,
    imported_record_id TEXT NOT NULL REFERENCES imported_records(id) ON DELETE RESTRICT,
    category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(imported_record_id, category_id)
);
CREATE INDEX idx_imported_record_allocations_record ON imported_record_allocations(imported_record_id);
