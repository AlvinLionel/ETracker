CREATE TABLE import_source_files (
    batch_id TEXT PRIMARY KEY REFERENCES import_batches(id) ON DELETE RESTRICT,
    contents BLOB NOT NULL,
    sha256 TEXT NOT NULL CHECK(length(sha256)=64),
    stored_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL DEFAULT (datetime('now', '+90 days'))
);

CREATE TRIGGER imported_records_refresh_source_retention_insert
AFTER INSERT ON imported_records
BEGIN
    UPDATE import_source_files
    SET expires_at=datetime('now', '+90 days')
    WHERE batch_id=NEW.batch_id;
END;

CREATE TRIGGER imported_records_refresh_source_retention_update
AFTER UPDATE ON imported_records
BEGIN
    UPDATE import_source_files
    SET expires_at=datetime('now', '+90 days')
    WHERE batch_id=NEW.batch_id;
END;

CREATE TRIGGER import_logs_refresh_source_retention_insert
AFTER INSERT ON import_logs
BEGIN
    UPDATE import_source_files
    SET expires_at=datetime('now', '+90 days')
    WHERE batch_id=NEW.batch_id;
END;

CREATE TRIGGER import_reconciliations_refresh_source_retention_insert
AFTER INSERT ON import_reconciliations
BEGIN
    UPDATE import_source_files
    SET expires_at=datetime('now', '+90 days')
    WHERE batch_id=NEW.batch_id;
END;

CREATE TRIGGER import_reconciliations_refresh_source_retention_update
AFTER UPDATE ON import_reconciliations
BEGIN
    UPDATE import_source_files
    SET expires_at=datetime('now', '+90 days')
    WHERE batch_id=NEW.batch_id;
END;

CREATE TRIGGER imported_record_allocations_refresh_source_retention_insert
AFTER INSERT ON imported_record_allocations
BEGIN
    UPDATE import_source_files
    SET expires_at=datetime('now', '+90 days')
    WHERE batch_id=(SELECT batch_id FROM imported_records WHERE id=NEW.imported_record_id);
END;

CREATE TRIGGER imported_record_allocations_refresh_source_retention_update
AFTER UPDATE ON imported_record_allocations
BEGIN
    UPDATE import_source_files
    SET expires_at=datetime('now', '+90 days')
    WHERE batch_id=(SELECT batch_id FROM imported_records WHERE id=NEW.imported_record_id);
END;

CREATE TRIGGER imported_record_allocations_refresh_source_retention_delete
AFTER DELETE ON imported_record_allocations
BEGIN
    UPDATE import_source_files
    SET expires_at=datetime('now', '+90 days')
    WHERE batch_id=(SELECT batch_id FROM imported_records WHERE id=OLD.imported_record_id);
END;
