CREATE TABLE import_category_rule_terms (
    id TEXT PRIMARY KEY,
    category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
    category_type TEXT NOT NULL CHECK (category_type IN ('income', 'expense')),
    match_text TEXT NOT NULL CHECK (length(trim(match_text)) > 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX idx_import_category_rule_terms_unique
    ON import_category_rule_terms(category_type, lower(match_text));

CREATE INDEX idx_import_category_rule_terms_category
    ON import_category_rule_terms(category_id);
