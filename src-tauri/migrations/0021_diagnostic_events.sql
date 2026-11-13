CREATE TABLE diagnostic_events (
    id TEXT PRIMARY KEY,
    event_code TEXT NOT NULL CHECK (
        event_code IN (
            'app_started',
            'attachment_storage_ready',
            'backup_created',
            'backup_failed',
            'backup_restored',
            'restore_failed'
        )
    ),
    occurred_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_diagnostic_events_occurred_at
    ON diagnostic_events(occurred_at);
