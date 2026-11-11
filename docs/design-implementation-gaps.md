# Design Implementation: Backend Gaps

This document records the behavior implied by the nine design screens that does not currently have a matching persisted backend operation. Existing screens should continue to use their current Tauri commands and real database results. Numbers, institutions, transaction counts, audit hashes, yields, alerts, and customer names shown in the PNG designs are presentation examples, not seed data.

## Already Backed By The Application

- **Dashboard:** `get_dashboard_summary` and `get_analytics_summary` provide balances, posted activity, accounts, budgets, savings progress, subscriptions, and historical chart data.
- **Accounts:** account create, update, archive, reactivate, balance lookup, and account-list import are implemented.
- **Transaction journal:** transaction create, edit, transfer, void, reversal, allocation, attachment, audit history, account filtering, and date filtering are implemented.
- **Imports and reconciliation:** CSV/XLSX/PDF staging, review, duplicate/revision handling, allocation, reconciliation, finalization, and audit logs are implemented.
- **Budgeting:** reusable templates, monthly periods, planned items, actual totals, and CSV/XLSX plan import are implemented.
- **Savings:** goals, manual contributions, eligible incoming-transfer linking, and goal status changes are implemented.
- **Analytics:** filtered summaries, trends, category breakdown, account comparisons, budget performance, savings progress, recurring expenses, and unusual expenses are implemented.
- **Forecasting:** local forecast snapshots, schedule configuration, and month evaluation are implemented. The current model is documented by its returned assumptions and limitations.
- **Settings:** theme and subscription reminder preferences, encrypted backup/restore, database integrity checks, and privacy-safe diagnostics are implemented.
- **Exports:** filtered journal, analytics report, monthly budget worksheet, savings contribution ledger, and active account portfolio CSV exports are implemented. PDF/print output and saved report definitions remain open.

## Backend Work Needed

### Shared Workspace

- Global search covers posted transactions, accounts, active categories, and budget templates with a bounded local result set. Rules remain out of scope because ETracker has no rule model.
- Add persisted active-ledger selection before shell period controls can switch datasets rather than merely changing labels. Current data remains in the one local ledger.

### Dashboard

- Add reconciliation summary data and a workflow entry point if the dashboard is to expose the design's global reconcile action. Import-batch reconciliation exists, but it is not the same as reconciling all posted account activity.
- Account-level yield, sync uptime, liquidity trajectory, and monthly burn pacing require sources and calculation definitions; they must not be inferred from the design mock's sample values.

### Accounts

- Institution connectivity, live sync status, yield/accrual calculations, statements, and cross-institution balance reconciliation need provider contracts, persistence, and validation rules. Current account records are user-maintained local records.
- Account portfolio CSV export is implemented with a fresh balance lookup for every active account and per-currency amounts. Portfolio comparison history and PDF output remain open.

### Transaction Journal

- Transaction search now supports free text/reference, status and flow filters, account/date filters, and bounded server-side pagination. Selectable batch reconciliation and dedicated split-edit workflows remain unimplemented.
- Filtered transaction CSV export is implemented with a 10,000-row safety cap. An audit-integrity/hash snapshot service remains unimplemented.

### Budgeting

- Add envelope policy fields (caps, rollover, group/category policy), reallocation transfers, rule evaluation, breach decisions/overrides, and audit events for those decisions.
- Define and persist monthly pacing inputs before showing projected month-end deltas, burn-velocity forecasts, or automated surplus vault behavior.
- Selected monthly plan CSV export is implemented from the persisted worksheet values. Multi-period reporting and print/PDF output remain open.

### Savings

- Add allocation rules and execution history for automatic skims, payroll sweeps, and transfer-triggered allocation. Existing transfer linking only records an already-posted transfer.
- Define supported account yield/rate, compounding, tax, and maturity calculations before displaying yield estimators or projected goal dates beyond the currently stored target date.
- Savings contribution ledger and CSV export are implemented for manual contributions and linked transfers.

### Analytics

- Analytics CSV export and an equal-length previous-period comparison are implemented with the active date/account/category/currency filters. Saved report definitions and print/PDF output remain open.

### Forecasting

- Add scenario definitions and persistence for editable income/expense/yield/inflation drivers, named scenarios, stress events, and simulation-run parameters.
- Add a validated stochastic simulation engine and percentile outputs before presenting Monte Carlo confidence curves or risk probabilities. Current forecasts are deterministic local estimates, not Monte Carlo simulations.
- Add scenario promotion into budget templates, plus forecast audit export, only after approval and provenance rules are defined.

### Settings

- Persisted preferences now include theme, grid opacity and row density. Primary/reporting currencies, fiscal-year start, accounting method, backup schedule, vault sync, and background notification delivery remain open; notifications are currently shown only when the app is open.
- A local backup command exists, but automated scheduled backups need a lifecycle-safe scheduler and retention policy. The design's hardware keystore, SQLCipher state, and sync status must be sourced from actual platform capabilities before display.

## Implementation Rule

Until the corresponding backend operation exists, a design affordance should be either omitted, visibly disabled with an explanation, or implemented as a clearly local-only preview. It must not report success, fabricate ledger data, or imply that an external bank/API connection is active.
