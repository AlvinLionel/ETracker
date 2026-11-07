CREATE TABLE refund_transactions (
    transaction_id TEXT PRIMARY KEY REFERENCES financial_transactions(id) ON DELETE RESTRICT,
    original_expense_transaction_id TEXT REFERENCES financial_transactions(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (original_expense_transaction_id IS NULL OR original_expense_transaction_id <> transaction_id)
);

CREATE INDEX idx_refund_transactions_original_expense
    ON refund_transactions(original_expense_transaction_id);
