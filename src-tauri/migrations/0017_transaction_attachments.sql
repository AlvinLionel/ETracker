CREATE TABLE transaction_attachments (
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL REFERENCES financial_transactions(id) ON DELETE CASCADE,
    stored_name TEXT NOT NULL UNIQUE,
    original_name TEXT NOT NULL CHECK (length(trim(original_name)) > 0),
    media_type TEXT NOT NULL CHECK (media_type IN ('application/pdf', 'image/jpeg', 'image/png')),
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0 AND size_bytes <= 10485760),
    sha256 TEXT NOT NULL CHECK (length(sha256) = 64),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_transaction_attachments_transaction
    ON transaction_attachments(transaction_id, created_at, id);
