CREATE TABLE import_reconciliations (
    id TEXT PRIMARY KEY,
    batch_id TEXT NOT NULL UNIQUE REFERENCES import_batches(id) ON DELETE RESTRICT,
    opening_balance_minor INTEGER,
    closing_balance_minor INTEGER,
    expected_closing_balance_minor INTEGER,
    difference_minor INTEGER,
    outcome TEXT NOT NULL CHECK (outcome IN ('reconciled','discrepancy','unavailable')),
    explanation TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
