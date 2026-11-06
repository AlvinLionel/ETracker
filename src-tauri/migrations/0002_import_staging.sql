CREATE TABLE import_batches (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE RESTRICT,
    source_file_name TEXT NOT NULL,
    source_file_hash TEXT,
    source_format TEXT NOT NULL CHECK (source_format IN ('csv','xlsx','pdf','unknown')),
    status TEXT NOT NULL DEFAULT 'review' CHECK (status IN ('review','ready','finalized','failed')),
    reconciliation_status TEXT NOT NULL DEFAULT 'unavailable' CHECK (reconciliation_status IN ('reconciled','discrepancy','unavailable')),
    statement_period_start TEXT,
    statement_period_end TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finalized_at TEXT
);
CREATE INDEX idx_import_batches_account ON import_batches(account_id,created_at DESC);

CREATE TABLE imported_records (
    id TEXT PRIMARY KEY,
    batch_id TEXT NOT NULL REFERENCES import_batches(id) ON DELETE RESTRICT,
    source_row_reference TEXT,
    original_date TEXT,
    normalized_date TEXT,
    original_description TEXT,
    normalized_description TEXT,
    original_amount TEXT,
    normalized_amount_minor INTEGER,
    currency_code TEXT NOT NULL DEFAULT 'KES' CHECK(length(currency_code)=3),
    transaction_reference TEXT,
    transaction_type TEXT CHECK (transaction_type IN ('income','expense')),
    validation_status TEXT NOT NULL DEFAULT 'uncertain' CHECK(validation_status IN ('valid','uncertain','invalid')),
    duplicate_status TEXT NOT NULL DEFAULT 'unmatched' CHECK(duplicate_status IN ('unmatched','high_confidence','probable','confirmed_new','skip')),
    review_status TEXT NOT NULL DEFAULT 'pending' CHECK(review_status IN ('pending','confirmed','corrected','skipped')),
    linked_transaction_id TEXT REFERENCES financial_transactions(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_imported_records_batch ON imported_records(batch_id,created_at);

CREATE TABLE import_logs (
    id TEXT PRIMARY KEY,
    batch_id TEXT NOT NULL REFERENCES import_batches(id) ON DELETE RESTRICT,
    record_id TEXT REFERENCES imported_records(id) ON DELETE RESTRICT,
    timestamp TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    stage TEXT NOT NULL,
    severity TEXT NOT NULL CHECK(severity IN ('INFO','WARNING','ERROR')),
    message TEXT NOT NULL,
    corrective_action TEXT
);
CREATE INDEX idx_import_logs_batch ON import_logs(batch_id,timestamp);
