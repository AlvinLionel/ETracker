CREATE TABLE forecast_snapshots (
    id TEXT PRIMARY KEY,
    generated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    currency_code TEXT NOT NULL CHECK (length(currency_code) = 3),
    period_months INTEGER NOT NULL CHECK (period_months BETWEEN 1 AND 12),
    history_months INTEGER NOT NULL CHECK (history_months >= 1),
    method TEXT NOT NULL,
    confidence TEXT NOT NULL CHECK (confidence IN ('limited','moderate','higher')),
    inputs_json TEXT NOT NULL,
    limitations_json TEXT NOT NULL
);

CREATE TABLE forecast_periods (
    id TEXT PRIMARY KEY,
    snapshot_id TEXT NOT NULL REFERENCES forecast_snapshots(id) ON DELETE RESTRICT,
    month_start TEXT NOT NULL,
    predicted_income_minor INTEGER NOT NULL,
    predicted_expense_minor INTEGER NOT NULL,
    predicted_subscription_minor INTEGER NOT NULL,
    predicted_savings_minor INTEGER NOT NULL,
    predicted_ending_balance_minor INTEGER NOT NULL,
    planned_income_minor INTEGER,
    planned_expense_minor INTEGER,
    planned_savings_minor INTEGER,
    UNIQUE(snapshot_id, month_start)
);

CREATE TABLE forecast_evaluations (
    id TEXT PRIMARY KEY,
    snapshot_id TEXT NOT NULL REFERENCES forecast_snapshots(id) ON DELETE RESTRICT,
    month_start TEXT NOT NULL,
    evaluated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    actual_income_minor INTEGER NOT NULL,
    actual_expense_minor INTEGER NOT NULL,
    actual_savings_minor INTEGER NOT NULL,
    income_variance_minor INTEGER NOT NULL,
    expense_variance_minor INTEGER NOT NULL,
    savings_variance_minor INTEGER NOT NULL,
    limitations_json TEXT NOT NULL,
    UNIQUE(snapshot_id, month_start)
);

CREATE INDEX idx_forecast_periods_snapshot ON forecast_periods(snapshot_id, month_start);
CREATE INDEX idx_forecast_evaluations_snapshot ON forecast_evaluations(snapshot_id, month_start);
