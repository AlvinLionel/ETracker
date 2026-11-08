ALTER TABLE import_batches ADD COLUMN discarded_at TEXT;

CREATE TRIGGER prevent_staging_records_for_discarded_batches
BEFORE INSERT ON imported_records
WHEN (SELECT discarded_at FROM import_batches WHERE id=NEW.batch_id) IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'Import batch has been discarded'); END;

CREATE TRIGGER prevent_changes_to_discarded_records
BEFORE UPDATE ON imported_records
WHEN (SELECT discarded_at FROM import_batches WHERE id=OLD.batch_id) IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'Import batch has been discarded'); END;

CREATE TRIGGER prevent_deleting_discarded_records
BEFORE DELETE ON imported_records
WHEN (SELECT discarded_at FROM import_batches WHERE id=OLD.batch_id) IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'Import batch has been discarded'); END;

CREATE TRIGGER prevent_allocations_for_discarded_batches
BEFORE INSERT ON imported_record_allocations
WHEN (SELECT b.discarded_at FROM imported_records r JOIN import_batches b ON b.id=r.batch_id WHERE r.id=NEW.imported_record_id) IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'Import batch has been discarded'); END;

CREATE TRIGGER prevent_changes_to_discarded_allocations
BEFORE UPDATE ON imported_record_allocations
WHEN (SELECT b.discarded_at FROM imported_records r JOIN import_batches b ON b.id=r.batch_id WHERE r.id=OLD.imported_record_id) IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'Import batch has been discarded'); END;

CREATE TRIGGER prevent_deleting_discarded_allocations
BEFORE DELETE ON imported_record_allocations
WHEN (SELECT b.discarded_at FROM imported_records r JOIN import_batches b ON b.id=r.batch_id WHERE r.id=OLD.imported_record_id) IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'Import batch has been discarded'); END;

CREATE TRIGGER prevent_resolution_changes_for_discarded_batches
BEFORE UPDATE ON import_revision_changes
WHEN (SELECT discarded_at FROM import_batches WHERE id=OLD.batch_id) IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'Import batch has been discarded'); END;

CREATE TRIGGER prevent_status_changes_for_discarded_batches
BEFORE UPDATE OF status, reconciliation_status ON import_batches
WHEN OLD.discarded_at IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'Import batch has been discarded'); END;

CREATE TRIGGER prevent_reopening_discarded_batches
BEFORE UPDATE OF discarded_at ON import_batches
WHEN OLD.discarded_at IS NOT NULL AND NEW.discarded_at IS NOT OLD.discarded_at
BEGIN SELECT RAISE(ABORT, 'Discarded import batches cannot be reopened'); END;
