CREATE TABLE subscription_payments (
    id TEXT PRIMARY KEY,
    subscription_id TEXT NOT NULL REFERENCES subscriptions(id) ON DELETE RESTRICT,
    transaction_id TEXT NOT NULL UNIQUE REFERENCES financial_transactions(id) ON DELETE RESTRICT,
    linked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_subscription_payments_subscription ON subscription_payments(subscription_id);
