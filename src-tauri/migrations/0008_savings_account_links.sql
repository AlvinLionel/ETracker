ALTER TABLE savings_goals ADD COLUMN account_id TEXT REFERENCES accounts(id) ON DELETE RESTRICT;

CREATE INDEX idx_savings_goals_account ON savings_goals(account_id);
