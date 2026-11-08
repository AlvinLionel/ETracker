ALTER TABLE transaction_audit_log RENAME TO transaction_audit_log_previous;

CREATE TABLE transaction_audit_log (
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL
        REFERENCES financial_transactions(id) ON DELETE RESTRICT,
    action TEXT NOT NULL CHECK (
        action IN (
            'created',
            'updated',
            'voided',
            'reversed',
            'refund_received',
            'refund_voided',
            'refund_reversed'
        )
    ),
    previous_values TEXT,
    new_values TEXT,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO transaction_audit_log
    (id, transaction_id, action, previous_values, new_values, reason, created_at)
SELECT id, transaction_id, action, previous_values, new_values, reason, created_at
FROM transaction_audit_log_previous;

DROP TABLE transaction_audit_log_previous;

CREATE INDEX idx_transaction_audit_transaction ON transaction_audit_log(transaction_id);
CREATE INDEX idx_transaction_audit_created ON transaction_audit_log(created_at);
