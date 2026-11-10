# Milestone Technical Audit

**Audit date:** 2026-11-07 · **Scope:** read-only review at `77512b23d7141fb6cd428ac574a1a9e0b57612e9`

## Executive summary

Core transaction, generic import, planning, analytics and forecast workflows exist and their reported commits are present. The report is generally supported by source and tests, with material exceptions below. Provider-specific M-Pesa/bank parsers, dashboard, reminders as a notification system, settings/themes, backup/recovery, E2E and release verification remain incomplete. No implementation changes were made for this audit.

## Repository state

- Branch `main`; HEAD `77512b23d7141fb6cd428ac574a1a9e0b57612e9`.
- `origin/main` comparison: 20 commits ahead, 0 behind. Latest commits include `75f2a46` refunds and `77512b2` scheduled forecasts. The main milestones in the progress report have commits.
- Working tree is not clean: eight modified paths, including `docs/development/agent-handover.md` and formatter-only account/infrastructure files. None are included in HEAD. Their ownership/intended disposition is unverified; preserve them.

## Implementation inventory

| Area | Verified implementation and files | Classification |
|---|---|---|
| Transactions | `src-tauri/src/persistence/transactions.rs`, `commands/transactions.rs`, `src/features/transactions/`, migrations `0001`, `0011`. Active-category retrieval, split allocation, history, edit/audit, void, linked reversal, inter-account transfer and separate fee expense exist. | Verified; Rust and UI tests cover primary and several failure paths. No desktop/E2E run. |
| Refunds | `refund_transactions` tags an existing ledger income inflow and optionally links an expense. Linked amounts are limited to remaining unreversed expense, same account/currency; original splits are proportionally copied. Unlinked records have no categories. Active refunds prevent editing, voiding or reversing the original; refund itself can be voided/reversed. Audit is on the refund record, not an additional event on the original. | Implemented and unit-tested. **Not a distinct stored `financial_transactions.transaction_type`:** list/analytics infer `refund` through the relation. Audit model should say “tagged income inflow.” |
| Imports | Migrations `0002–0006`; `src-tauri/src/persistence/imports.rs`, `commands/imports.rs`, `src/features/imports/`. Durable staging/logs; SQLite source bytes and SHA-256; generic mapped CSV/XLSX; selectable-text PDF extraction staged uncertain; duplicate candidates; reconciliation gates; atomic finalization; revised batches with row decisions and reversals. | Generic import verified by repository/UI tests. No provider-specific M-Pesa or bank parsers; scanned PDF OCR is absent. |
| Planning | Migration `0007` adds templates/periods/items, goals/contributions, subscriptions/history; `0008` links goals to accounts; `0009` links existing expenses to subscriptions. Persistence/UI in `src-tauri/src/persistence/planning.rs`, `src/features/planning/`. Periods copy templates; actuals use posted ledger; manual contributions do not move balances; incoming transfer matching avoids duplicate ledger entries; subscription projections/reminders are shown separately. | Implemented, with workflow tests. Subscription projection groups currencies without conversion. |
| Analytics | `src-tauri/src/persistence/analytics.rs`, `src/features/analytics/`. Income, gross expense, refunds, trends, categories, accounts, budgets/savings, filters and CSV export; transfers/voided/reversed excluded. What-if is temporary frontend state. | Core reports tested. Historical comparisons are monthly trends, not a dedicated comparison-period feature. |
| Forecasting | Migration `0010`; migration `0012` adds opt-in schedule and unique scheduled-month tag. `src-tauri/src/persistence/forecasting.rs`, `commands/forecasting.rs`, `src/features/forecasting/`. Six completed-month averages, subscription/budget context, stored input/limitations and period snapshots, post-month evaluation. | Local generation/evaluation and schedule idempotence tested. See schedule limitations below. |

## Migration inventory

- `0002_import_staging.sql`: batches, staged records, logs, indexes.
- `0003_import_reconciliation.sql`: one reconciliation record per batch.
- `0004_import_category_allocations.sql`: staged confirmed allocations.
- `0005_import_source_files.sql`: original BLOB/hash/expiry and activity-refresh triggers.
- `0006_import_revisions.sql`: prior-batch link and row-level revision changes.
- `0007_financial_planning.sql`: budgets, savings, subscriptions and term history.
- `0008_savings_account_links.sql`: optional goal/account association.
- `0009_subscription_payments.sql`: link existing ledger expenses to subscriptions.
- `0010_forecasting.sql`: forecast snapshots, periods and evaluations.
- `0011_refund_records.sql`: refund tag and optional original-expense relation.
- `0012_forecast_schedule.sql`: monthly schedule preference, scheduled-month column/index.

## Decisions, scope and risks

- **Source retention — verified/approved:** `DB-026` in `docs/decisions/decision-log.md` explicitly approves 90 days after latest batch activity, startup cleanup, and preservation of staged/ledger/history. Triggers refresh `expires_at`; `Database::initialize` deletes expired rows from `import_source_files`. The BLOB row is logically deleted (not a secure overwrite guarantee); policy is not configurable. The import UI documents the 90-day availability. Batch discard/cancellation remains pending (`PEND-006`). This no longer conflicts with the current decision log, though older database prose still calls retention an implementation detail.
- **Multi-currency — implemented beyond KES-first scope:** account validation in `src-tauri/src/domain/accounts/account.rs` accepts only KES, while subscriptions and projections store/sum multiple currency codes and the UI groups totals by currency. No FX rates or conversion are used. This is separate-currency bookkeeping, not converted consolidated value. `ADR-004` names KES as initial currency; the broader projection UI is not recorded as an approved scope decision.
- **Refund consistency — potential risk:** Analytics separates refunds from ordinary income and gross expenses. Budget actual-income query and forecast historical/evaluated income exclude tagged refunds. Account balances retain the positive inflow. Transaction history displays the relation as `refund`; underlying rows remain income. Proportional allocations and over-refund limits are tested. Directly linked refunds require same account and currency; unlinked refunds are allowed. The original transaction does not receive a linked-refund audit entry.
- **Forecast schedule — limitation:** schedule is opt-in, checked during Tauri startup and when saved. A unique `scheduled_month` allows one scheduled snapshot per UTC calendar month. If app is not opened during a month, next startup generates only for the current month; missed months are not backfilled. It is not a continuously running calendar scheduler. Manual snapshots remain separate.
- **CSV formula injection — potential security risk:** `AnalyticsPage.tsx` quotes cells and doubles quotes, but does not neutralize leading `=`, `+`, `-`, `@`, tab or carriage-return formula triggers. Spreadsheet applications may evaluate attacker-controlled category/account/description text on open. No mitigation/test found.
- **Other scope/doc gaps:** no backup/restore, dashboard, settings/theme implementation or notification center; navigation items are placeholders in `src/App.tsx`. The implementation progress says Phase A–C complete and Phase D underway, which matches delivered feature work, but known-issues and progress status are stale (known issues still lists planning/analytics/forecasting as outstanding). The decision log has no refund-specific decision or schedule policy; multi-currency projection extension is also unrecorded. `docs/04-database-design.md` is not fully synchronized with migrations `0011–0012` beyond recent added prose.

## Quality, privacy and coverage

- Last recorded full checks: `cargo test --offline` — 58 passed; `npm run test:run` — 40 passed in 9 files; `npm run build` passed; `cargo fmt` completed. Existing Rust dead-code and Windows incremental hard-link warnings remain. The audit did not rerun checks.
- Tests include SQLite rollback injection for transaction/import writes, refund limits and schedule idempotence. Frontend tests cover core screens. No Playwright/Tauri E2E configuration or executed E2E, no manual desktop verification evidenced.
- Import logs are persisted; no evidence of raw source BLOB contents being logged. File bytes are retained in the local SQLite database until expiry; deletion removes the table row, not a cryptographic erase. No upload-size limit was confirmed in this audit. Backups and encryption policy are pending (`PEND-025/026`).
- Rust command errors are often mapped to generic messages; diagnostics/structured logging and CSP review remain technical debt. No runtime release/installer verification found.

## Outstanding work and continuation order

1. Resolve audit findings: protect CSV exports from formula injection; reconcile refund semantics/audit linkage and document the data model; decide whether separate-currency projections belong in KES-first v1; define scheduled catch-up behavior.
2. Complete Phase 6 from the assignment: dashboard, notification/reminder preferences, settings/themes, backup/restore, offline/data-integrity checks, logging/error review and Windows packaging.
3. Resolve provider sample/format and scanned-PDF OCR decisions before adding provider parsers.
4. Refresh stale progress/known-issues and decision records after the technical lead accepts findings; then proceed to the approved UI redesign phase.
