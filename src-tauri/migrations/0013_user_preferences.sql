CREATE TABLE user_preferences (
    preference_key TEXT PRIMARY KEY,
    preference_value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO user_preferences(preference_key, preference_value)
VALUES ('subscription_reminders_enabled', 'true'),
       ('subscription_reminder_days', '7');
