CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    account_type TEXT NOT NULL CHECK (
        account_type IN ('mobile_money', 'bank', 'cash', 'other')
    ),
    institution_name TEXT,
    currency_code TEXT NOT NULL DEFAULT 'KES'
        CHECK (length(currency_code) = 3),
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TEXT
);

CREATE INDEX idx_accounts_type ON accounts(account_type);

CREATE INDEX idx_accounts_archived ON accounts(archived_at);

CREATE TABLE categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    category_type TEXT NOT NULL CHECK (
        category_type IN ('income', 'expense')
    ),
    parent_id TEXT REFERENCES categories(id) ON DELETE RESTRICT,
    is_system INTEGER NOT NULL DEFAULT 0
        CHECK (is_system IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TEXT
);

CREATE UNIQUE INDEX idx_categories_unique_active_name
    ON categories(category_type, lower(name))
    WHERE archived_at IS NULL;

CREATE INDEX idx_categories_parent ON categories(parent_id);

CREATE TABLE financial_transactions (
    id TEXT PRIMARY KEY,
    transaction_type TEXT NOT NULL CHECK (
        transaction_type IN ( 'income', 'expense', 'transfer', 'opening_balance', 'reversal')
    ),
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    currency_code TEXT NOT NULL DEFAULT 'KES'
        CHECK (length(currency_code) = 3),
    occurred_at TEXT NOT NULL,
    description TEXT,
    notes TEXT,
    source_type TEXT NOT NULL CHECK (
        source_type IN ('manual', 'import', 'opening_balance', 'reversal')
    ),
    source_reference TEXT,
    status TEXT NOT NULL DEFAULT 'posted' CHECK (
        status IN ('posted', 'voided')
    ),
    voided_at TEXT,
    void_reason TEXT,
    reversal_of_transaction_id TEXT
        REFERENCES financial_transactions(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CHECK (
        (status = 'posted' AND voided_at IS NULL)
        OR
        (status = 'voided' AND voided_at IS NOT NULL)
    ),

    CHECK (
        (transaction_type = 'reversal' AND reversal_of_transaction_id IS NOT NULL)
        OR
        (transaction_type <> 'reversal' AND reversal_of_transaction_id IS NULL)
    )
);

CREATE UNIQUE INDEX idx_transactions_unique_reversal
    ON financial_transactions(reversal_of_transaction_id)
    WHERE reversal_of_transaction_id IS NOT NULL;

CREATE INDEX idx_transactions_occurred_at ON financial_transactions(occurred_at);

CREATE INDEX idx_transactions_type ON financial_transactions(transaction_type);

CREATE INDEX idx_transactions_status ON financial_transactions(status);

CREATE TABLE account_entries (
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL
        REFERENCES financial_transactions(id) ON DELETE RESTRICT,
    account_id TEXT NOT NULL
        REFERENCES accounts(id) ON DELETE RESTRICT,
    amount_minor INTEGER NOT NULL CHECK (amount_minor <> 0),
    entry_type TEXT NOT NULL DEFAULT 'normal' CHECK (
        entry_type IN ('normal', 'opening_balance')
    ),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(transaction_id, account_id)
);

CREATE UNIQUE INDEX idx_account_entries_opening_balance
    ON account_entries(account_id)
    WHERE entry_type = 'opening_balance';

CREATE INDEX idx_account_entries_account ON account_entries(account_id);

CREATE INDEX idx_account_entries_transaction ON account_entries(transaction_id);

CREATE TABLE category_allocations (
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL
        REFERENCES financial_transactions(id) ON DELETE RESTRICT,
    category_id TEXT NOT NULL
        REFERENCES categories(id) ON DELETE RESTRICT,
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(transaction_id, category_id)
);

CREATE INDEX idx_category_allocations_transaction ON category_allocations(transaction_id);

CREATE INDEX idx_category_allocations_category ON category_allocations(category_id);

CREATE TABLE transaction_audit_log (
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL
        REFERENCES financial_transactions(id) ON DELETE RESTRICT,
    action TEXT NOT NULL CHECK (
        action IN ('created', 'updated', 'voided', 'reversed')
    ),
    previous_values TEXT,
    new_values TEXT,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_transaction_audit_transaction ON transaction_audit_log(transaction_id);

CREATE INDEX idx_transaction_audit_created ON transaction_audit_log(created_at);