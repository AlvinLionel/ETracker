ALTER TABLE import_batches
ADD COLUMN revision_of_batch_id TEXT REFERENCES import_batches(id) ON DELETE RESTRICT;

CREATE TABLE import_revision_changes (
    id TEXT PRIMARY KEY,
    batch_id TEXT NOT NULL REFERENCES import_batches(id) ON DELETE RESTRICT,
    original_record_id TEXT REFERENCES imported_records(id) ON DELETE RESTRICT,
    revised_record_id TEXT REFERENCES imported_records(id) ON DELETE RESTRICT,
    change_type TEXT NOT NULL CHECK (change_type IN ('unchanged','added','removed','modified','ambiguous')),
    resolution TEXT CHECK (resolution IN ('keep_original','use_revised','merge')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (original_record_id IS NOT NULL OR revised_record_id IS NOT NULL),
    UNIQUE(batch_id, original_record_id),
    UNIQUE(batch_id, revised_record_id)
);

CREATE INDEX idx_import_revision_changes_batch ON import_revision_changes(batch_id, change_type);
