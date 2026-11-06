CREATE TABLE budget_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    currency_code TEXT NOT NULL CHECK (length(currency_code) = 3),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE budget_template_items (
    id TEXT PRIMARY KEY,
    template_id TEXT NOT NULL REFERENCES budget_templates(id) ON DELETE CASCADE,
    item_type TEXT NOT NULL CHECK (item_type IN ('income','expense','savings')),
    category_id TEXT REFERENCES categories(id) ON DELETE RESTRICT,
    label TEXT NOT NULL CHECK (length(trim(label)) > 0),
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    UNIQUE(template_id, item_type, category_id)
);

CREATE TABLE budget_periods (
    id TEXT PRIMARY KEY,
    template_id TEXT REFERENCES budget_templates(id) ON DELETE SET NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    currency_code TEXT NOT NULL CHECK (length(currency_code) = 3),
    starts_on TEXT NOT NULL,
    ends_on TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (starts_on <= ends_on),
    UNIQUE(starts_on, ends_on)
);

CREATE TABLE budget_period_items (
    id TEXT PRIMARY KEY,
    budget_id TEXT NOT NULL REFERENCES budget_periods(id) ON DELETE CASCADE,
    item_type TEXT NOT NULL CHECK (item_type IN ('income','expense','savings')),
    category_id TEXT REFERENCES categories(id) ON DELETE RESTRICT,
    label TEXT NOT NULL CHECK (length(trim(label)) > 0),
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    UNIQUE(budget_id, item_type, category_id)
);

CREATE INDEX idx_budget_period_items_budget ON budget_period_items(budget_id);

CREATE TABLE savings_goals (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    description TEXT,
    target_amount_minor INTEGER NOT NULL CHECK (target_amount_minor > 0),
    currency_code TEXT NOT NULL CHECK (length(currency_code) = 3),
    target_date TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','completed','archived')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE savings_contributions (
    id TEXT PRIMARY KEY,
    goal_id TEXT NOT NULL REFERENCES savings_goals(id) ON DELETE RESTRICT,
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    contribution_date TEXT NOT NULL,
    description TEXT,
    transaction_id TEXT REFERENCES financial_transactions(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE subscriptions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    description TEXT,
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    currency_code TEXT NOT NULL CHECK (length(currency_code) = 3),
    frequency TEXT NOT NULL CHECK (frequency IN ('weekly','monthly','quarterly','yearly')),
    next_due_date TEXT NOT NULL,
    account_id TEXT REFERENCES accounts(id) ON DELETE RESTRICT,
    category_id TEXT REFERENCES categories(id) ON DELETE RESTRICT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','paused','cancelled')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE subscription_history (
    id TEXT PRIMARY KEY,
    subscription_id TEXT NOT NULL REFERENCES subscriptions(id) ON DELETE RESTRICT,
    previous_values TEXT NOT NULL,
    new_values TEXT NOT NULL,
    changed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
