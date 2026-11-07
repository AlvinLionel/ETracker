# ETracker — Known Issues and Open Questions

**Last reviewed:** 2026-11-07 · **Baseline:** `77512b23d7141fb6cd428ac574a1a9e0b57612e9`

This list reflects repository inspection at the baseline above. “Confirmed” means the limitation or defect is visible in current code. “Needs verification” means available evidence is insufficient to close it. Product and architecture choices remain subject to the decision log; recording a question here does not approve a change.

## Confirmed defects and risks

| ID | Area | Finding and impact | Evidence / next action | Status |
|---|---|---|---|---|
| KI-002 | Forecast scheduling | Scheduled generation runs when enabled settings are saved and during app startup. It uses a unique UTC `scheduled_month`, so retries that month are idempotent. If the app misses a month, next startup generates only for the current month; it does not backfill missed months. | `src-tauri/src/persistence/forecasting.rs`, `generate_scheduled_if_due`; behavior is explicitly approved. | Documented limitation |
| KI-003 | Refund data/audit | Refunds are income ledger rows tagged through `refund_transactions`; transaction history derives the displayed `refund` label. Creating a refund audits the refund transaction but does not add a corresponding audit event to the original expense. | `src-tauri/migrations/0011_refund_records.sql`; `persistence/transactions.rs`, `create_refund`, `list`; confirm this provenance/audit model is sufficient. | Open — design confirmation |
| KI-004 | Source-file privacy | Expiry deletes the source-file row/BLOB from SQLite, but this is not a secure physical overwrite guarantee (for example, WAL/filesystem remnants may persist). No source-size limit was confirmed during this review. | `infrastructure/database/connection.rs`; `migrations/0005_import_source_files.sql`; `persistence/imports.rs`; assess storage protection, size limits and deletion expectations before release. | Open — security/privacy verification |
| KI-005 | Diagnostics | Some Tauri commands translate persistence failures to generic user errors; an end-user diagnostic export/structured operational logging and retention policy are not implemented/verified. | `src-tauri/src/commands/`; deployment decision `PEND-028`; define privacy-safe diagnostics. | Open — incomplete |

## Incomplete implementation

| ID | Area | Remaining work | Status / dependency |
|---|---|---|---|
| OW-001 | Statement imports | Institution-specific M-Pesa and bank parsers are absent; current CSV/XLSX mapping and selectable-text PDF extraction are generic. | Pending verified provider samples and normalized formats (`PEND-002`). Do not claim provider parser support. |
| OW-002 | PDF imports | Scanned-PDF OCR is unsupported; text-based PDFs are staged as uncertain rows for review. | Pending local OCR library decision (`PEND-008`). |
| OW-003 | Dashboard | Dashboard navigation is present but nonfunctional; no financial dashboard view exists. | Pending implementation and final wireframes (`PEND-011`). |
| OW-004 | Notifications/reminders | Subscription due dates are shown in the subscription screen and forecast evaluation is user-triggered. A notification/reminder center, scheduling and preference management are absent. | Pending notification UX (`PEND-015`) and reminder implementation. Import reminder cadence also needs product definition. |
| OW-005 | Settings/themes | Settings navigation is a placeholder. Light/dark/system theme selection and persistent settings are absent. | Pending UI design/accessibility choices (`PEND-009`, `010`, `016`). |
| OW-006 | Backup/recovery | No user backup, verification or restoration workflow is implemented. | Backup format, retention, encryption and migration/recovery policy remain pending (`PEND-025–027`). |
| OW-007 | Release | Windows installer/package, supported OS versions, signing and update path are not verified. | Pending release decisions (`PEND-021–024`, `PEND-029–030`). No manual install verification evidenced. |
| OW-008 | Verification | No executed Tauri/Playwright E2E suite or manual desktop workflow verification is evidenced. Offline behavior and production CSP/local-data protection are not comprehensively verified. | Tooling/release gates pending (`PEND-017–019`); CSP/security review outstanding. |
| OW-009 | Project setup/docs | README repository URL placeholder/setup instructions and license decision remain open; current progress/known-issues documents have had stale entries. | Verify README and align status docs after resolving this list. Licensing/distribution decision is not recorded in the open decision table. |

## Scope, behavior and decision questions

| ID | Question or uncertainty | Evidence and required confirmation |
|---|---|---|
| Q-001 | Legacy non-KES display behavior | KES-only new-write enforcement is implemented for planning and forecasts. Existing non-KES rows are retained and subscription projections show separate currency totals without conversion. Verify future screens preserve this separation; never sum these values. |
| Q-002 | Original-expense audit visibility for refunds | `DB-029` approves tagged-income refunds, separate gross/refund/net analytics and gross budget actuals. A linked refund has its own audit record, but the original expense’s audit history does not list that refund. Confirm whether this cross-link is required; do not change refund representation. |
| Q-003 | Forecast scheduling cadence | Approved behavior is opt-in checks on setting save and app startup, UTC calendar month, one scheduled snapshot/month, no backfill. Operational limitation: a month is skipped if the application is never started during it. |
| Q-004 | Is duplicate detection’s conservative rule sufficient? | Current matching flags exact account/date/type/amount candidates as probable; provider reference matching and confidence thresholds are pending (`PEND-003`). |
| Q-005 | What should happen to pending transactions during reconciliation? | Still pending (`PEND-004`). Current staged format assumes reviewed posted rows. |
| Q-006 | What are the full savings goal/contribution semantics? | Implemented manual and linked-transfer contributions; exact relationship remains listed pending (`PEND-007`). Verify withdrawals, corrections and goal/account balance expectations before extending. |
| Q-007 | What is the policy for discarding/cancelling import batches? | 90-day source-byte retention is approved in `DB-026`; it is not configurable. Whole-batch discard/cancellation remains pending (`PEND-006`). |
| Q-008 | Refund transaction screen’s audit linkage | Verify the UI identifies the selected original expense and that users can inspect the refund’s own audit history. It does not add a record to the original expense’s audit history. |

Other open decision-log items remain applicable: `PEND-001` final physical schema; `PEND-009–016` visual design, navigation, charting, interaction and accessibility; `PEND-017–020` QA tooling, CI, release gates and acceptance criteria; and `PEND-021–031` deployment, backup, logs, versioning, cadence and AI credential handling. `PEND-005` is absent from the current table and its disposition needs checking. `DB-026` records the owner-approved source retention rule; `DB-029` records the refund reporting and KES-only planning rules.

## Resolved items removed from the old outstanding list

The earlier entries for planning, analytics and forecasting were stale: budgets, savings, subscriptions, analytics, local forecasts and forecast evaluation now exist and have tests. The unused `formatKES` TypeScript compiler error was fixed. CSV formula-prefix protection was added for untrusted text columns while preserving negative numeric amounts and quote escaping. These are not open issues; broader release verification remains.

## Configuration review still required

Review Tauri CSP and app identity, default icons, `.gitignore`, dev/production configuration, dependency/lockfile maintenance, sensitive-data logging, import size limits, backup confidentiality, and local database protection before release. These are review items, not confirmed failures except where explicitly identified above.
