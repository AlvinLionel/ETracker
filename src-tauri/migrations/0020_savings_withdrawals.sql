CREATE TABLE savings_withdrawals (
    id TEXT PRIMARY KEY,
    goal_id TEXT NOT NULL REFERENCES savings_goals(id) ON DELETE RESTRICT,
    transaction_id TEXT NOT NULL UNIQUE REFERENCES financial_transactions(id) ON DELETE RESTRICT,
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    withdrawal_date TEXT NOT NULL,
    description TEXT
);

CREATE INDEX idx_savings_withdrawals_goal_date
    ON savings_withdrawals(goal_id, withdrawal_date);
