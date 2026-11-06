# ETracker — Database Design

**Document ID:** DOC-004  
**Version:** 0.2  
**Status:** Revised Draft  
**Last Updated:** 4 November 2026

---

## 1. Introduction

### 1.1 Purpose

This document defines the conceptual database architecture for ETracker.

It describes the proposed data entities, relationships, integrity rules, persistence strategy and financial data handling requirements.

The database must provide a dependable foundation for financial management while preserving historical information, supporting controlled corrections and preventing inaccurate financial calculations.

### 1.2 Database Objectives

The database must:

1. Store financial information accurately and consistently.
2. Support multiple financial accounts.
3. Preserve financial history and source information.
4. Support income, expenses, transfers, refunds and fees.
5. Separate financial events from account movements and category allocations.
6. Support split-category transactions.
7. Support historical balance calculations.
8. Preserve audit trails for financial corrections.
9. Support staged statement imports and atomic finalization.
10. Prevent duplicate financial records.
11. Support reusable budget templates and independent monthly budgets.
12. Track intended savings and actual savings separately.
13. Support subscriptions, forecasts, analytics and notifications.
14. Support reliable backup and recovery.
15. Provide a foundation for future cross-device synchronization.

### 1.3 Database Technology

**Provisional database:** SQLite.

The database will be stored locally on the user's device.

The Rust backend will manage authoritative database operations.

The exact SQLite library, migration framework and supporting dependencies remain subject to technology evaluation.

---

## 2. Database Design Principles

### 2.1 Financial Accuracy

Financial records must use precise monetary representations.

Binary floating-point arithmetic must not be used for authoritative monetary calculations.

The proposed approach is integer storage in the currency's smallest supported unit, with currency metadata determining the applicable scale.

For KES, amounts will normally be represented in cents where required.

### 2.2 Separation of Concerns

Different financial concepts must have distinct representations.

In particular:

- Financial transactions represent events.
- Account entries represent balance movements.
- Category allocations represent classifications.
- Budgets represent plans.
- Actual transactions represent real financial activity.
- Subscriptions represent recurring obligations.
- Forecasts represent predictions.

These concepts must not be conflated.

### 2.3 Historical Preservation

Financial records must not disappear through casual deletion or modification.

Corrections must preserve sufficient information to understand what originally happened and how it was corrected.

### 2.4 Referential Integrity

Relationships between records must be protected through appropriate database constraints.

Records with financial dependencies must not be deleted in ways that leave balances, allocations, audit history or import history inconsistent.

### 2.5 Stable Identification

Entities should use stable unique identifiers.

Identifiers must not depend on mutable descriptions, account names or transaction dates.

This will also support future synchronization.

### 2.6 Offline Availability

All essential financial data must remain available locally.

The database must not require a remote service for ordinary financial operations.

---

## 3. Core Financial Data Model

### 3.1 Conceptual Overview

The financial ledger uses three separate concepts:

1. Financial transaction headers.
2. Account entries.
3. Category allocations.

```text
+---------------------------+
|    Financial Transaction |
|---------------------------|
| Transaction ID            |
| Transaction Type          |
| Effective Date            |
| Description               |
| Source                    |
| Reference                 |
| Status                    |
+-------------+-------------+
              |
              | 1
              |
              | Many
              v
+---------------------------+
|       Account Entries     |
|---------------------------|
| Entry ID                  |
| Transaction ID            |
| Account ID                |
| Signed Amount             |
| Effective Date            |
+-------------+-------------+
              |
              | 1
              |
              | Many
              v
+---------------------------+
|     Category Allocations  |
|---------------------------|
| Allocation ID             |
| Entry ID                  |
| Category ID               |
| Allocated Amount          |
+---------------------------+
```

This separation supports split-category purchases, transfers, opening balances and other financial events without introducing duplicate balance movements.

### 3.2 Financial Transactions

**Proposed table:** `financial_transactions`

Represents a financial event.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique transaction identifier |
| transaction_type | Income, expense, transfer, refund, opening balance, adjustment, etc. |
| effective_date | Date the event affects financial history |
| description | Human-readable description |
| original_description | Original source description where applicable |
| source_type | Manual, imported or system-generated |
| source_reference | External transaction reference where available |
| status | Active, voided or other controlled state |
| created_at | Record creation timestamp |
| updated_at | Last modification timestamp |

Additional metadata may be required depending on transaction type.

Transaction headers should not store duplicated account balance values.

### 3.3 Account Entries

**Proposed table:** `transaction_entries`

Represents actual movements in individual financial accounts.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique entry identifier |
| transaction_id | Parent financial transaction |
| account_id | Affected account |
| amount_minor | Signed monetary amount in the smallest supported unit |
| currency_code | Currency identifier where needed |
| effective_date | Effective date for historical balance calculations |
| status | Controlled entry status |
| created_at | Creation timestamp |

Rules:

- Negative values reduce account balances.
- Positive values increase account balances.
- Entries must belong to valid transactions and accounts.
- Account balance calculations must use valid, non-voided entries.
- Entries must not be silently deleted when corrections are needed.

### 3.4 Category Allocations

**Proposed table:** `transaction_allocations`

Represents how a financial entry is classified.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique allocation identifier |
| entry_id | Account entry being classified |
| category_id | Assigned category |
| amount_minor | Allocated amount |
| created_at | Creation timestamp |
| updated_at | Last modification timestamp |

Rules:

- One entry may have multiple allocations.
- Allocation amounts must reconcile to the amount being classified.
- Allocations must not create account balance movements.
- Transfers and opening balances do not require ordinary income or expense allocations.
- Category changes must preserve appropriate audit history.

For an expense of KSh 2,000:

| Category | Allocation |
|---|---:|
| Food | KSh 1,500 |
| Household | KSh 500 |
| **Total** | **KSh 2,000** |

### 3.5 Financial Transaction Types

The database must support at least:

- Income.
- Expense.
- Transfer.
- Refund.
- Opening balance.
- Adjustment.
- Reversal.

The exact internal enumeration and additional types will be finalized during schema implementation.

A transaction's type determines how its entries and allocations participate in financial calculations.

---

## 4. Accounts

### 4.1 Accounts Table

**Proposed table:** `accounts`

Stores financial account information.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique account identifier |
| name | Account display name |
| account_type | M-Pesa, bank, cash, savings or other supported type |
| institution | Bank, provider or institution where applicable |
| currency_code | Account currency |
| status | Active or archived |
| created_at | Creation timestamp |
| updated_at | Last modification timestamp |

The design should allow additional account types without requiring a complete database redesign.

### 4.2 Account Opening Balances

Opening balances are represented as special financial transactions.

An opening balance:
- Establishes the account's initial position.
- Has an effective date.
- Is not ordinary income.
- Must be distinguishable from subsequent financial activity.

The system must enforce one controlled opening-balance transaction per account.

If the opening balance is incorrect, it must be corrected through an appropriate controlled adjustment.

### 4.3 Account Balance Calculation

The conceptual calculation is:

\[
\text{Balance}(A,t)
=
\text{Opening Position}
+
\sum \text{Valid Entries}(A,\text{effective date}\leq t)
\]

The calculation must account for:
- Opening balances.
- Income.
- Expenses.
- Transfers.
- Fees.
- Refunds.
- Adjustments.
- Reversals.
- Voided records.

The implementation must avoid counting an opening balance and pre-opening transactions twice.

### 4.4 Historical Balances

The database must support balance queries for a specified historical date.

Historical calculations must use effective dates rather than record creation or import timestamps.

A correction must affect the appropriate historical periods according to its effective date and correction rules.

---

## 5. Financial Corrections and Audit History

### 5.1 Controlled Editing

Financial transactions may be edited through controlled operations.

The system must preserve:
- Original values.
- Modified values.
- Relevant timestamps.
- Correction reason where appropriate.
- User or system action information.

### 5.2 Voiding

Voiding marks a financial record as invalid for active financial calculations.

Voided records must:
- Remain available for audit.
- Be excluded from account balances.
- Be excluded from relevant financial reports.
- Retain their original information.

### 5.3 Reversals

A reversal is a separate financial event linked to the original transaction.

It must not silently overwrite the original event.

The relationship between original and reversal records must be preserved.

### 5.4 Atomic Corrections

Related financial changes must succeed or fail together.

For example, correcting a transfer must not update one account while leaving the other account unchanged.

---

## 6. Categories

### 6.1 Categories Table

**Proposed table:** `categories`

Stores financial classification information.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique category identifier |
| name | Category name |
| parent_category_id | Optional parent category |
| category_type | Income, expense or other supported classification |
| status | Active or archived |
| created_at | Creation timestamp |
| updated_at | Last modification timestamp |

### 6.2 Category Hierarchy

The system should support categories and subcategories.

Examples:

- Food
  - Groceries
  - Restaurants
  - Snacks
- Transport
  - Public Transport
  - Fuel
  - Ride Services

### 6.3 Category Preservation

Categories associated with historical transactions must not be destructively removed.

Archiving should prevent ordinary future selection without compromising historical reports.

Category reassignment must preserve appropriate audit information.

---

## 7. Import Data Architecture

### 7.1 Import Design

Imports must use a staging model.

Extracted records are stored separately from the authoritative financial ledger until the complete batch is approved for finalization.

### 7.2 Import Batches

**Proposed table:** `import_batches`

Represents an individual import operation.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique batch identifier |
| account_id | Target financial account |
| source_file_name | Original file name |
| source_file_hash | Source identity/integrity reference |
| source_format | PDF, scanned PDF, CSV, XLSX or other supported format |
| statement_period_start | Statement start date where available |
| statement_period_end | Statement end date where available |
| status | Current processing state |
| reconciliation_status | Reconciliation outcome |
| created_at | Batch creation timestamp |
| finalized_at | Successful finalization timestamp |

A batch must retain enough information to resume or review an unfinished import.

### 7.3 Imported Records

**Proposed table:** `imported_records`

Stores individual extracted records before finalization.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique staged record identifier |
| batch_id | Parent import batch |
| source_row_reference | Original row, page or location |
| original_date | Original extracted date |
| normalized_date | Interpreted transaction date |
| original_description | Original source description |
| normalized_description | Normalized description |
| original_amount | Original monetary representation |
| normalized_amount_minor | Interpreted monetary value |
| transaction_reference | Source transaction reference |
| transaction_type | Interpreted transaction type |
| duplicate_status | Duplicate assessment |
| validation_status | Validation outcome |
| review_status | User review outcome |
| linked_transaction_id | Resulting ledger transaction where finalized |

The exact extraction metadata may be expanded to support different file formats.

### 7.4 Import Logs

**Proposed table:** `import_logs`

Stores import processing activity.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique log identifier |
| batch_id | Associated import batch |
| record_id | Optional related staged record |
| timestamp | Event timestamp |
| stage | Processing stage |
| severity | INFO, WARNING or ERROR |
| message | Description of activity |
| corrective_action | Relevant action or guidance |

Logs must not expose unnecessary sensitive financial information.

### 7.5 Physical schema status

Migrations `0002_import_staging.sql` through `0004_import_category_allocations.sql` implement persistent batches, staged records, import logs, reconciliation and staged category allocations. The batch workspace stores account, source name/format/hash metadata, status and reconciliation outcome. Staged rows remain separate from ledger entries. Generic mapped CSV and XLSX paths use user-selected date, amount and optional description columns, a selected positive-amount direction, and ISO-date or RFC 3339 date formats. The XLSX path allows worksheet and header-row selection and retains worksheet row references. Formula-derived mapped date or amount cells are held invalid for manual correction because cached formula results may be stale. Invalid and uncertain records remain staged, corrections and explicit duplicate-review decisions are logged, and reviewed imports are atomically posted with linked records, category allocations and transaction audit entries. Current duplicate detection conservatively flags exact account/date/type/amount matches as probable candidates; it does not auto-skip. Users may enter statement opening and closing balances; the system compares the closing balance with the opening balance plus staged signed movements, records the outcome and blocks finalization on discrepancies. Source-file bytes and hashes are not yet retained, provider-specific parsing and revised-statement comparison remain outstanding. Source-format metadata does not imply provider parser support.

### 7.6 Reconciliation Records

**Proposed table:** `reconciliations`

Stores statement reconciliation information.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique reconciliation identifier |
| batch_id | Associated import batch |
| opening_balance | Statement opening balance where available |
| closing_balance | Statement closing balance where available |
| calculated_balance | Calculated result |
| difference | Identified discrepancy |
| status | Reconciled, discrepancy detected or unavailable |
| explanation | Relevant reconciliation details |
| created_at | Creation timestamp |

Reconciliation must account for statement-specific conventions, pending transactions, fees and ordering.

### 7.6 Duplicate Detection

Duplicate detection must consider multiple factors:

- Account identity.
- Transaction reference.
- Amount and currency.
- Transaction date.
- Transaction time where available.
- Description or merchant.
- Transaction type.

The system should distinguish:

1. High-confidence duplicate.
2. Probable duplicate.
3. No identified duplicate.

Automatic skipping should only occur when the evidence meets the configured high-confidence criteria.

Ambiguous matches must be available for manual review.

Missing time information must not automatically disqualify a possible match.

### 7.7 Revised Statements

The database must preserve previous statement versions and their import history.

Revised statements must support:
- Comparison.
- Added and removed records.
- Modified records.
- Compatible merges.
- Conflicting value review.
- User-approved replacement.

A replacement must not destructively erase previous imports or silently overwrite manually corrected financial records.

Changes must be staged and validated before finalization.

### 7.8 Staging Persistence

Unfinished imports must survive application closure and reopening.

Staged records must remain separate from authoritative financial records.

The system must prevent a batch from being finalized more than once.

The exact retention and cancellation policy for abandoned imports remains an implementation detail to finalize.

### 7.9 Atomic Finalization

Each import must be finalized as one atomic database operation.

The finalization transaction must include all required ledger records and the relevant completion state.

If any part fails, the entire ledger commit must roll back.

Staging information may be persisted incrementally before finalization, but no successful subset of the batch may enter the authoritative ledger prematurely.

---

## 8. Budgeting and Planning Data Architecture

### 8.1 Budget Templates

**Proposed table:** `budget_templates`

Stores reusable budget structures.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique template identifier |
| name | Template name |
| description | Optional description |
| status | Active or archived |
| created_at | Creation timestamp |
| updated_at | Last modification timestamp |

### 8.2 Budget Template Items

**Proposed table:** `budget_template_items`

Stores reusable planned allocations.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique item identifier |
| template_id | Parent template |
| category_id | Associated category |
| planned_amount_minor | Planned amount |
| allocation_type | Income, expense or savings |
| description | Optional item description |

### 8.3 Budget Periods

**Proposed table:** `budget_periods`

Represents an independent budget instance for a specific period.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique budget period identifier |
| template_id | Source template where applicable |
| period_start | Period start date |
| period_end | Period end date |
| name | Optional period name |
| status | Draft, active or completed |
| created_at | Creation timestamp |
| updated_at | Last modification timestamp |

### 8.4 Budget Period Items

**Proposed table:** `budget_period_items`

Stores the actual planned allocations for a specific period.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique item identifier |
| budget_period_id | Parent budget period |
| category_id | Associated category |
| planned_amount_minor | Planned amount |
| allocation_type | Income, expense or savings |
| notes | Optional notes |

### 8.5 Template and Instance Rules

A reusable template creates a new independent budget instance.

The instance copies the relevant template items.

Subsequent changes to the template must not automatically modify existing monthly instances.

Changes to one monthly instance must not modify previous or future instances.

### 8.6 Actual Versus Planned Data

Budget records represent intended financial activity.

Actual transactions remain stored in the financial ledger.

The budgeting module compares actual activity against the corresponding period and category.

The database must preserve the distinction between:

- Planned income.
- Actual income.
- Planned expenses.
- Actual expenses.
- Planned savings.
- Actual savings.
- Unallocated surplus.

Actual spending must not be stored as a duplicate of the ledger.

### 8.7 Budget Period Calculation

Budget performance is calculated using the budget instance and qualifying actual transactions for its period.

The implementation must account for:
- Effective transaction dates.
- Category allocations.
- Transfers.
- Refunds.
- Voided transactions.
- Split-category transactions.

---

## 9. Savings Data Architecture

### 9.1 Savings Goals

**Proposed table:** `savings_goals`

Stores individual savings objectives.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique goal identifier |
| name | Goal name |
| description | Optional description |
| target_amount_minor | Target savings amount |
| target_date | Optional target date |
| status | Active, completed or archived |
| created_at | Creation timestamp |
| updated_at | Last modification timestamp |

### 9.2 Savings Contributions

**Proposed table:** `savings_contributions`

Stores records of actual savings contributions or qualifying savings activity.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique contribution identifier |
| goal_id | Associated savings goal where applicable |
| amount_minor | Amount saved |
| contribution_date | Effective date |
| source_type | Ledger-linked or manually recorded |
| transaction_id | Related financial event where applicable |
| entry_id | Related account entry where applicable |
| description | Optional description |
| created_at | Creation timestamp |

The final representation of manual savings records requires careful implementation so that manually recording a contribution does not create an artificial account movement.

### 9.3 Three Separate Savings Measures

The system must track these independently:

**Intended savings**

The amount allocated toward savings in a budget.

**Actual savings**

The amount genuinely set aside and recorded as saved.

**Unallocated remaining funds**

Money not assigned to a planned expense or recorded savings contribution.

These values must not be treated as interchangeable.

### 9.4 Savings Integrity

Savings contributions must not:
- Create artificial income.
- Duplicate an existing account transfer.
- Double-count an account balance.
- Automatically convert unallocated surplus into actual savings.

Savings progress must be traceable to its underlying records.

---

## 10. Subscription Data Architecture

### 10.1 Subscriptions

**Proposed table:** `subscriptions`

Stores recurring financial obligations.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique subscription identifier |
| name | Subscription name |
| description | Optional description |
| amount_minor | Expected payment amount |
| currency_code | Currency |
| frequency | Weekly, monthly, yearly or other supported frequency |
| next_due_date | Next expected payment date |
| account_id | Associated payment account where applicable |
| status | Active, paused or cancelled |
| created_at | Creation timestamp |
| updated_at | Last modification timestamp |

### 10.2 Subscription History

Subscription changes should preserve relevant historical information.

The system should retain information about changes in:
- Payment amount.
- Billing frequency.
- Due dates.
- Account or payment method.
- Status.

### 10.3 Actual Subscription Payments

Subscription records represent obligations, not completed payments.

Actual payments must be recorded independently as financial transactions.

This prevents projected subscription costs from being counted as actual expenses.

---

## 11. Forecasting Data Architecture

### 11.1 Forecasts

**Proposed table:** `forecasts`

Stores individual forecast snapshots.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique forecast identifier |
| forecast_type | Forecast category |
| period_start | Forecast start |
| period_end | Forecast end |
| generated_at | Generation timestamp |
| generation_method | Local or optional external AI-assisted |
| assumptions | Stored assumptions or reference |
| confidence_level | Confidence indicator where supported |
| status | Generated, evaluated or other controlled state |

### 11.2 Forecast Items

**Proposed table:** `forecast_items`

Stores individual forecast results.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique item identifier |
| forecast_id | Parent forecast |
| category_id | Optional associated category |
| account_id | Optional associated account |
| predicted_amount_minor | Predicted amount |
| item_type | Income, expense, balance or other forecast measure |
| explanation | Relevant interpretation |

### 11.3 Forecast Evaluations

**Proposed table:** `forecast_evaluations`

Stores forecast performance evaluations.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique evaluation identifier |
| forecast_id | Evaluated forecast |
| evaluation_date | Evaluation timestamp |
| evaluation_period | Relevant period |
| actual_result | Observed actual outcome |
| variance | Difference between prediction and actual |
| explanation | Evaluation details |

Forecast snapshots must be preserved.

Generating a new forecast must not overwrite an earlier forecast.

---

## 12. Notification Data Architecture

### 12.1 Notifications

**Proposed table:** `notifications`

Stores application notifications.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique notification identifier |
| notification_type | Subscription, import, forecast or other type |
| title | Notification title |
| message | Notification content |
| created_at | Creation timestamp |
| scheduled_at | Optional scheduled time |
| read_at | Optional read timestamp |
| status | Pending, delivered, read or dismissed |

### 12.2 Notification Preferences

Notification settings should be stored separately from notification history.

Preferences may include:
- Enabled notification types.
- Reminder timing.
- Import reminder cadence.
- Forecast evaluation reminders.
- Subscription reminder settings.

Import reminders must be based on the last successfully reconciled statement period rather than merely the last file uploaded.

---

## 13. Attachments and File Metadata

### 13.1 Attachments

**Proposed table:** `attachments`

Stores metadata for files associated with transactions and other application records.

Proposed fields:

| Field | Purpose |
|---|---|
| id | Unique attachment identifier |
| file_name | Original file name |
| storage_path | Managed storage location |
| mime_type | File type |
| file_size | File size |
| file_hash | Integrity reference |
| created_at | Creation timestamp |

### 13.2 Attachment Relationships

An attachment may be associated with:
- Financial transactions.
- Imported statements.
- Relevant supporting records.

The final relationship design should support multiple attachments without storing large file contents directly inside ordinary transaction records.

### 13.3 Attachment Integrity

The application must detect missing or inaccessible attachments.

Backup and recovery must account for attachment files and their database references together.

---

## 14. Settings and Application Metadata

### 14.1 Application Settings

**Proposed table:** `application_settings`

Stores application preferences.

Examples:
- Theme preference.
- Default currency.
- Default account.
- Notification preferences.
- Import preferences.
- Forecast preferences.
- AI consent settings.
- Backup configuration.

Sensitive credentials must not be stored as ordinary plaintext settings.

### 14.2 Database Metadata

The database should maintain metadata for:
- Schema version.
- Migration history.
- Database creation.
- Application compatibility.
- Relevant integrity checks.

This supports controlled upgrades and recovery.

---

## 15. Database Integrity and Constraints

### 15.1 Referential Integrity

Foreign key relationships must be enforced.

Financial records must not become orphaned when accounts, categories or other associated records are archived.

### 15.2 Financial Constraints

The database and domain layer must enforce appropriate rules for:

- Valid monetary amounts.
- Valid transaction types.
- Account entry relationships.
- Allocation reconciliation.
- Transfer consistency.
- Opening balance restrictions.
- Valid transaction statuses.
- Historical date handling.
- Reversal relationships.
- Duplicate finalization prevention.

Some constraints require domain-level validation in addition to SQLite constraints.

### 15.3 Atomic Operations

Related financial changes must be performed in database transactions.

This applies to:
- Transfers.
- Financial corrections.
- Reversals.
- Import finalization.
- Other multi-record financial operations.

### 15.4 Deletion Policy

Financial history must not be casually deleted.

The system should use archiving, voiding and controlled corrections where appropriate.

Permanent deletion should be limited to data where it is safe and consistent with the application's retention and privacy rules.

---

## 16. Indexing and Query Considerations

Indexes should be introduced for frequently queried fields.

Likely candidates include:

- Account identifiers.
- Transaction effective dates.
- Transaction types.
- Transaction statuses.
- Category identifiers.
- Import batch identifiers.
- Source transaction references.
- Budget period dates.
- Subscription due dates.
- Forecast periods.

Composite indexes may be needed for common combinations, such as account and effective date.

Index selection must be validated against actual query patterns.

Avoid unnecessary indexes that increase write overhead without meaningful query benefits.

---

## 17. Database Migrations

Database schema changes must be version-controlled.

The application must:
- Track the current schema version.
- Apply migrations in a controlled sequence.
- Validate migration outcomes.
- Protect existing financial records.
- Handle interrupted migrations safely.
- Provide a recovery path before risky upgrades.

Database migrations must be tested against representative existing database versions.

Migration scripts should not silently discard historical financial information.

---

## 18. Backup and Recovery

The backup strategy must preserve a consistent state of:

- SQLite database.
- Managed attachment files.
- Required application metadata.

Backups should support multiple generations.

The application must verify backup integrity where practical.

Restoration must validate:
- Database integrity.
- Schema compatibility.
- Financial record consistency.
- Attachment availability.
- Relevant references.

A recovery copy of existing data should be protected before restoration.

The backup strategy must account for SQLite's transaction and journaling behaviour so that database files are not copied in an inconsistent state.

---

## 19. Future Synchronization Considerations

Cloud synchronization is outside the initial desktop release but must remain a future architectural consideration.

The database should use:
- Stable record identifiers.
- Explicit creation and modification timestamps.
- Clear record relationships.
- Preserved audit history.
- Controlled correction and deletion semantics.

Future synchronization will require additional concepts, potentially including:

- Device identifiers.
- Synchronization state.
- Change tracking.
- Conflict records.
- Remote identifiers.
- Synchronization timestamps.
- Conflict resolution policies.

The initial database must not assume that copying entire database files between devices will be an acceptable synchronization strategy.

Financial conflict resolution must preserve data integrity and avoid silent overwrites.

---

## 20. Open Database Design Decisions

The following areas require further specification or implementation-level validation.

| ID | Topic | Status |
|---|---|---|
| DB-001 | Final account type definitions | Pending |
| DB-002 | Currency and monetary storage conventions | Pending final technical specification |
| DB-003 | Transaction headers with multiple account entries | Approved |
| DB-004 | Transaction and entry status model | Pending detailed specification |
| DB-005 | Category hierarchy and classification rules | Pending detailed specification |
| DB-006 | Transaction date and timestamp conventions | Pending final specification |
| DB-007 | Audit history implementation | Pending technical design |
| DB-008 | Import source identity and provenance | Pending detailed specification |
| DB-009 | Attachment storage and file relationships | Pending technical design |
| DB-010 | Database migration framework | Pending technology evaluation |
| DB-011 | Indexing and query optimization | Pending implementation analysis |
| DB-012 | Backup format and recovery implementation | Pending technical design |
| DB-013–DB-025 | Approved financial and import decisions | See decision register |

---

## 21. Current Database Design Status

As of 4 November 2026:

| Area | Status |
|---|---|
| SQLite direction | Provisional |
| Financial transaction model | Approved conceptual design |
| Account entries | Approved conceptual design |
| Category allocations | Approved conceptual design |
| Split-category transactions | Approved |
| Opening balance representation | Approved |
| Historical balance support | Approved |
| Controlled corrections and reversals | Approved |
| Import staging | Approved |
| Atomic import finalization | Approved |
| Duplicate detection hierarchy | Approved |
| Reconciliation outcomes | Approved |
| Revised statement handling | Approved |
| Budget templates and independent monthly instances | Approved direction |
| Savings tracking | Approved direction |
| Intended, actual and unallocated savings separation | Approved |
| Subscription separation from actual payments | Approved direction |
| Forecast snapshots and evaluations | Approved direction |
| Local attachment storage | Approved direction |
| Future synchronization readiness | Approved direction |
| Physical database schema | Not finalized |
| Migrations | Not implemented |
| Database tests | Not implemented |

---

**End of Document — DOC-004**
