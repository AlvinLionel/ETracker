ALTER TABLE forecast_snapshots ADD COLUMN scheduled_month TEXT;
CREATE UNIQUE INDEX idx_forecast_snapshots_scheduled_month
    ON forecast_snapshots(scheduled_month)
    WHERE scheduled_month IS NOT NULL;

CREATE TABLE forecast_schedule (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0,1)),
    currency_code TEXT NOT NULL CHECK (length(currency_code) = 3),
    period_months INTEGER NOT NULL CHECK (period_months BETWEEN 1 AND 12),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO forecast_schedule(id,enabled,currency_code,period_months)
VALUES (1,0,'KES',3);
