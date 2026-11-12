CREATE TABLE categories_new (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    category_type TEXT NOT NULL CHECK (
        category_type IN ('income', 'expense', 'emergency')
    ),
    parent_id TEXT REFERENCES categories_new(id) ON DELETE RESTRICT,
    is_system INTEGER NOT NULL DEFAULT 0
        CHECK (is_system IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TEXT
);

INSERT INTO categories_new (
    id, name, category_type, parent_id, is_system, created_at, updated_at, archived_at
)
SELECT
    id, name, category_type, parent_id, is_system, created_at, updated_at, archived_at
FROM categories;

DROP TABLE categories;
ALTER TABLE categories_new RENAME TO categories;

CREATE UNIQUE INDEX idx_categories_unique_active_name
    ON categories(category_type, lower(name))
    WHERE archived_at IS NULL;
CREATE INDEX idx_categories_parent ON categories(parent_id);

CREATE TABLE subscriptions_new (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    description TEXT,
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    currency_code TEXT NOT NULL CHECK (length(currency_code) = 3),
    frequency TEXT NOT NULL CHECK (
        frequency IN ('weekly', 'monthly', 'quarterly', 'yearly')
        OR (
            frequency GLOB 'custom:[1-9]*:days'
            AND substr(frequency, 8, length(frequency) - 12) NOT GLOB '*[^0-9]*'
            AND CAST(substr(frequency, 8, length(frequency) - 12) AS INTEGER) BETWEEN 1 AND 365
        )
        OR (
            frequency GLOB 'custom:[1-9]*:weeks'
            AND substr(frequency, 8, length(frequency) - 13) NOT GLOB '*[^0-9]*'
            AND CAST(substr(frequency, 8, length(frequency) - 13) AS INTEGER) BETWEEN 1 AND 365
        )
        OR (
            frequency GLOB 'custom:[1-9]*:months'
            AND substr(frequency, 8, length(frequency) - 14) NOT GLOB '*[^0-9]*'
            AND CAST(substr(frequency, 8, length(frequency) - 14) AS INTEGER) BETWEEN 1 AND 365
        )
        OR (
            frequency GLOB 'custom:[1-9]*:years'
            AND substr(frequency, 8, length(frequency) - 13) NOT GLOB '*[^0-9]*'
            AND CAST(substr(frequency, 8, length(frequency) - 13) AS INTEGER) BETWEEN 1 AND 365
        )
    ),
    next_due_date TEXT NOT NULL,
    account_id TEXT REFERENCES accounts(id) ON DELETE RESTRICT,
    category_id TEXT REFERENCES categories(id) ON DELETE RESTRICT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'cancelled')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO subscriptions_new (
    id, name, description, amount_minor, currency_code, frequency, next_due_date,
    account_id, category_id, status, created_at, updated_at
)
SELECT
    id, name, description, amount_minor, currency_code, frequency, next_due_date,
    account_id, category_id, status, created_at, updated_at
FROM subscriptions;

DROP TABLE subscriptions;
ALTER TABLE subscriptions_new RENAME TO subscriptions;
