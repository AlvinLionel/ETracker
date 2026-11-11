# ETracker — Implementation Progress

**Last updated:** 2026-11-11

## 1. Project Status

**Current phase:** Phase 6 — Application completion and first usable Windows preview

The Tauri, React, Rust, SQLite and account-management foundation is implemented. Phases A — Transaction Management, B — generic Statement Imports, C — Financial Planning, and D — Analytics and Forecasting are complete and verified. Phase 6 — Application Completion is underway. Analytics reports ordinary income, gross expenses and refunds separately, plus net spending (gross expenses less refunds); monthly net cash flow includes refunds, while budgets retain gross actual expenses. Reports include date, account, category and currency filters, budget and savings views, CSV export with formula-prefix protection for untrusted text, and a temporary what-if simulator. Forecasts save independent local snapshots, explain assumptions and data coverage, include active KES subscriptions and budget context, support actual evaluation, and can generate one snapshot per UTC calendar month under an opt-in schedule. Linked refund receipts, voids and reversals now appear in both related audit histories.

Phase B generic import workflows are implemented. Migrations 0002–0006 add persistent batches, staged records, logs, statement reconciliation, staged category allocations, original source-file retention and revision tracking. CSV, legacy/modern Excel (.xls/.xlsx), and PDF files can be staged, including password-protected and scanned image-only documents. Spreadsheet headers preselect likely date, amount and description fields, with explicit mappings available; Excel users select a worksheet and header row. Each source filename is preserved as its own workspace title. Budget-plan and account-list headings are detected and blocked from transaction import. Budget-plan CSV/Excel rows load into the reusable-template editor for review and correction before saving. Account-list CSV/Excel rows can be reviewed in Accounts and create new KES accounts with optional dated opening balances; account numbers are ignored, duplicate name/institution pairs are excluded, and unsupported currencies are rejected. Password-protected PDFs are extracted locally using bundled PDF.js; the password is kept in the current workspace only, and page text is sent to the Rust backend without the password. PDF text and local English OCR output remain uncertain page-and-line content for correction; OCR runs offline and never posts a transaction automatically. Original uploaded bytes are limited to 25 MiB, stored in SQLite with a computed SHA-256 hash and downloadable for 90 days after the latest batch activity; expired files are removed on startup. ZIP-based workbook expansion and extracted PDF text are capped at 100 MiB. Invalid or uncertain rows remain staged; users can correct or review records, inspect processing logs, reconcile statement opening and closing balances, assign confirmed split categories, identify possible duplicates, and atomically finalize reviewed batches into the ledger. Revised imports allow selection of a prior finalized batch, row-level comparison, ambiguity review, keep-original, use-revised and compatible-merge actions. User-selected replacement of a modified imported transaction creates an audited linked reversal and replacement atomically. Provider-specific parsing remains open. Phase C includes editable reusable budget templates and independent monthly snapshots, planned income/expense/savings, category actuals and overall actual/unallocated calculations, manual savings contributions and deduplicated incoming transfer links, plus recurring subscription terms/status/history, existing ledger payment links, multi-currency monthly projections and upcoming payment reminders. Phase C coverage includes failure paths and regression checks for budget edits, savings deduplication/reversal and subscription payment matching/void/reversal handling.

## 2. Completed Milestones

### 2.1 Project Documentation

- [x] Project overview established.
- [x] Software requirements specification established.
- [x] System architecture documented.
- [x] Database design documented.
- [x] UI/UX specification documented.
- [x] Testing and QA strategy documented.
- [x] Deployment and maintenance approach documented.
- [x] Architecture and design decision log established.
- [x] Development conventions established.

### 2.2 Repository Foundation

- [x] Single-project Git repository established.
- [x] Approved repository structure established through the application scaffold.
- [x] React and TypeScript frontend initialized.
- [x] Tauri 2 desktop framework initialized.
- [x] Rust backend initialized.
- [x] Frontend dependencies installed.
- [x] Dependency installation completed with zero reported vulnerabilities.
- [x] TypeScript compilation verified.
- [x] Vite production build verified.
- [x] Windows desktop application successfully launched.
- [x] React-to-Rust command communication verified using the scaffold greeting command.
- [x] Application foundation committed and pushed.

### 2.3 Development Environment

- [x] Node.js and npm verified.
- [x] Rust and Cargo verified.
- [x] Windows native compilation environment verified.
- [x] WebView2 availability verified.
- [x] Initial native Rust compilation verified.

### 2.4 Engineering Conventions

- [x] Naming and code style conventions established.
- [x] Frontend organization approved.
- [x] Backend layered architecture approved.
- [x] Structured error-handling approach approved.
- [x] Shared operational logging approach approved.
- [x] Four-level testing strategy established.
- [x] Git and collaboration workflow established.
- [x] Dependency and documentation management principles established.

### 2.5 Account Management

- [x] Create accounts with optional opening balances.
- [x] Edit accounts and view active and archived accounts.
- [x] Archive with stale-balance detection and reactivate.
- [x] Derive balances from posted account entries.
- [x] Rust repository and UI tests verified in the account milestone.

### 2.6 Transaction Management — Initial Slice

- [x] Validate positive integer minor-unit amounts, account, currency, RFC 3339 effective timestamp and reconciled allocations.
- [x] Create manual income and expenses with signed ledger entries.
- [x] Reject missing or archived accounts and account currency mismatches.
- [x] Support zero allocations for uncategorized records and confirmed split allocations in the backend.
- [x] Validate active category type and allocation amount before persistence.
- [x] Persist transaction, entry, allocations and creation audit row atomically.
- [x] Expose Tauri commands and a frontend entry/history view with account and date filters.
- [x] Add Rust domain and SQLite repository tests.
- [x] Retrieve active categories filtered by income/expense type; exclude archived categories.
- [x] Enter and edit optional split allocations in the UI; uncategorized remains supported.
- [x] Edit posted manual transactions atomically, preserving before/after values in audit history.
- [x] Display transaction audit history in the UI.
- [x] Void manual transactions with a required reason; posted-balance queries exclude them.
- [x] Create linked reversal entries atomically and prevent duplicate reversal through the schema constraint.
- [x] Transfer funds atomically between active same-currency accounts, with an optional separate expense transaction for the fee.
- [x] Preserve existing allocations to archived categories during edits only when their category and amount remain unchanged; prevent reassignment to archived categories.
- [x] Verify transaction balance, audit, rollback, reversal, void and transfer paths with Rust repository tests.
- [x] Add transaction UI regression coverage for uncategorized and split-allocation workflows.
- [x] Complete Phase A verification: `cargo test` passed 36 tests; `npm run test:run` passed 21 tests; `npm run build` passed.

## 3. Current Implementation State

The application has an operational Tauri/React/Rust/SQLite foundation and account management. Account balances derive from posted ledger entries. Transaction workflows support manual income and expense entry, category splits, history and filters, controlled edits with audit history, voids, reversals, and transfers with separate fee expenses. The import workflow persists workspaces, original file bytes and staged CSV/XLSX/PDF text records; it supports correction, explicit review and revision decisions, exposes batch logs, reconciles statements and atomically posts finalized records. Financial planning has editable reusable budget templates and period snapshots, category-level and overall actual comparisons, planned and actual unallocated amounts, manual and transfer-linked savings contributions, subscription terms with history, projections and reminders, and links actual payments to existing ledger expenses without duplicating them. Analytics includes transaction and planning reports with export and temporary scenario analysis. Forecasts are generated locally on demand, saved with their input snapshot, kept separate from actuals and evaluated against posted results when the month ends. The functional dashboard summarizes KES account balances and monthly income, gross expenses, refunds and net spending, with recent activity and planning summaries; legacy foreign-currency account balances remain individually displayed and excluded from the KES total. Settings persist subscription reminders and theme choice, show upcoming payments, check local database integrity, export encrypted backups and restore supported databases, and download a redacted diagnostic report. Persistent application logs and operating-system notifications are not implemented.

## 4. Verification on 2026-11-07

- Earlier foundation checks: `cargo test` passed 20 library tests, `npm run test:run` passed 13 tests across 2 files, and `npm run build` passed after fixing the unused `formatKES` helper in `AccountsPage.tsx`.
- Audit-remediation checkpoint: `cargo test --offline` passed 60 library tests; `npm run test:run` passed 48 tests across 10 files; `npm run build`, `cargo check --offline --manifest-path src-tauri/Cargo.toml`, and `cargo fmt -- --check` passed. This checkpoint is committed as `fa422d4`.
- Dashboard milestone: added a Rust read model and command, enabled the dashboard as the default screen, and added summary, accounts, activity, budgets, goals and upcoming subscriptions. Rust and React component tests passed. Committed as `a80b5fb`; desktop workflow and usability review remain.
- Reminder/settings milestone: migration-backed preferences control subscription reminder visibility and a 1–30 day lookahead; the Settings screen lists active due subscriptions. Committed as `9ed4627`. Background and operating-system notifications remain out of scope for this local reminder view.
- Data-integrity check: Settings runs SQLite `integrity_check` and `foreign_key_check`, presenting clean results or violating record references without modifying data. Healthy and failure paths are tested; the changes are committed in `0c6714f` and `26991a1`.
- Windows release checkpoint: `npx tauri build` produced x64 MSI (`ETracker_0.1.0_x64_en-US.msi`) and NSIS (`ETracker_0.1.0_x64-setup.exe`) installers. Both were rebuilt after the encrypted backup feature and verified by the Tauri bundler. Manual install/upgrade, supported Windows versions, signing and update strategy remain unverified.
- Theme preferences: persisted system/light/dark selection; system mode responds to operating-system theme changes and app views receive dark palette overrides. Committed in `f4f0827`; frontend build and Rust/UI checks passed.
- Encrypted backup and restore: Settings exports a versioned SQLite snapshot encrypted with AES-256-GCM and PBKDF2-HMAC-SHA256 (600,000 iterations) using a user passphrase. Restore authenticates and validates the database in memory, applies supported forward migrations to the isolated copy, checks integrity and foreign keys, then replaces current data. The app never stores the passphrase; downloaded backups remain user-managed. Rust tests pass for authentication/tampering, successful restore, and rejection of an invalid backup or wrong passphrase without replacing existing data. Desktop workflow, interrupted replacement behavior, and large-database memory/IPC limits remain to verify. Decision recorded as `DB-030`.
- Release and diagnostics checkpoint: rebuilt the x64 MSI and NSIS installers with backup and diagnostics support. Verification passed: 68 Rust tests, 57 frontend tests, production frontend build, Rust format check and both Tauri bundles. Settings downloads a redacted JSON report with app/platform/SQLite/schema versions and database health status; failure-path tests verify that database identifiers are excluded. The report excludes financial records and is not sent by ETracker. Persistent log retention remains pending (`PEND-028`).
- Desktop E2E tooling: added an isolated Tauri debug configuration, Windows WebDriver setup script and Selenium workflow for account creation, income, transfers with separate fees, transaction editing/audit history, voiding, reversal, CSV statement staging, correction, opening/closing balance reconciliation, batch finalization, refusal to finalize an invalid staged row, explicit revised-statement replacement with linked reversal, persisted dark/light theme switching, encrypted backup creation, wrong-passphrase rejection and successful restore. Backup bytes are captured in the test before OS file-download handling, then supplied back to the restore UI. Ports 4444/4445 were unavailable on this host, so the harness uses ports 54231/54232. It clears only its dedicated generated E2E data directory before each run. The full workflow passes against the real Edge desktop app. Native browser file-download behavior and release-install workflows still need manual verification. The E2E file is excluded from Vitest discovery. Current verification: 68 Rust tests, 60 frontend tests, production frontend build, Rust formatting check and desktop E2E pass.
- UI redesign foundation: applied the approved warm neutral/purple workspace palette with matching charcoal/navy theme tokens to the app shell, dashboard and common legacy utility classes across feature screens. Compact horizontal navigation supports narrow windows; the dashboard uses consistent financial cards, accessible budget progress indicators and clear links to related records. React tests verify navigation, active-page state, dashboard figures and progress clamping. Verification passed: 58 frontend tests and production frontend build. Feature layouts still need screen-by-screen refinement and desktop visual review.
- Accounts screen redesign: aligned the account list and archive states with the shared visual language, improved heading hierarchy, added selected state to the active/archive toggle and keyboard focus visibility, and applied row hover/reflow treatments. A React test covers active/archive switching. Verification passed: 59 frontend tests and production frontend build.
- Transaction screen redesign: grouped entry, transfer and history into consistent surfaces; clarified the entry heading, made monetary values use tabular figures, and preserved a usable horizontally scrolling history table at narrow widths. Existing transaction form/regression tests still pass. Verification passed: 59 frontend tests and production frontend build.
- Statement-import screen redesign: grouped import setup and saved workspaces into shared surfaces, clarified the task heading and added hover/reflow affordances to batch rows while retaining the staged review flow. Existing import workflow tests remain green. Verification passed: 59 frontend tests and production frontend build.
- Cross-screen visual consistency: routed existing rounded white feature panels and neutral/status utility colors through shared light/dark tokens, adding restrained surface depth while preserving each screen’s business behavior. This brings budgeting, savings, subscriptions, analytics, forecasting and settings into the same palette without replacing their existing components. Verification passed: all 59 frontend tests, production frontend build, Rust formatting check and diff checks.
- Planning and insights heading pass: aligned Budgeting, Savings, Subscriptions, Analytics, Forecasting and Settings page headings with the same accent label, title scale and muted supporting text used by the refined core screens. Verification passed: all 59 frontend tests and production frontend build.
- Navigation accessibility pass: gave the desktop sidebar, compact navigation and import-workspace view switcher distinct accessible landmark names, with regression checks for the application and import navigation. Broader keyboard and screen-reader review remains open.
- Import correction display fix: staged rows now expose the normalized description separately from the original source description, display the normalized value after correction, and prefill it when correcting again. Added Rust repository and React UI regression assertions. Verification passed: 68 Rust tests, 60 frontend tests, production build and the existing desktop E2E workflow.
- Finance-first visual workflow: moved Statement imports ahead of Transactions and clarified that imported statements are the main route for historical spending, while manual entries fill gaps. Redesigned Budgeting’s monthly plan table as an editable finance worksheet with row numbers and accessible caption. Added responsive SVG monthly income/expense/refund bars, an expense-category donut with labeled values and a screen-reader accessible data table, and the restrained accounting grid texture across light/dark app backgrounds. Verification passed: all 60 frontend tests, production build and desktop E2E on real Edge.
- Product direction recorded as `UX-012`: statement imports lead historical spending, budgeting uses a finance-focused worksheet, charts use accessible native SVG, and no separate UI generation tool is needed for these components.
- Dashboard entry point: added a primary “Import a statement” action so statement-based history is the easiest path into the application. The action is covered by a Dashboard regression test. The desktop workflow now also verifies statement correction and balance reconciliation before finalization.
- Category management: added an active income/expense category page, category creation with trimmed names and type validation, and reversible archival that preserves historical assignments and protects system categories. The desktop import flow now creates categories and exercises split allocations. Verified with 70 Rust tests, 61 frontend tests, production build, and the real Edge desktop E2E flow, including encrypted backup and both failed and successful restore paths.
- Import resource bound: original statement uploads above 25 MiB are rejected in the UI before file reading and again by the application service before batch persistence. The policy is recorded as `DB-032`; workbook expansion and extraction limits still need review.
- Current verification after the resource bound: 71 Rust tests, 62 frontend tests, and production frontend build pass. The import screen test confirms an oversized file is rejected before it is read or sent to the backend.
- Import recognition and protected PDF step: CSV/XLS/XLSX headers now preselect likely fields; budget-plan and account-list spreadsheets are detected and blocked from transaction posting. PDF imports request a password inline, extract locally with PDF.js and retain the same workspace for retry. A standards-valid encrypted fixture verified correct extraction and wrong-password recovery in the Edge desktop E2E workflow. At this checkpoint, scanned-PDF OCR was not yet available.
- Import hardening checkpoint (8 November 2026): XLSX ZIP entries and extracted PDF text are checked against 100 MiB processing limits; originals remain capped at 25 MiB. Budget-plan CSV/Excel rows load into the reusable-template editor for review before saving. Standards-valid encrypted PDF extraction, wrong-password retry and workspace retention pass in the desktop workflow; OCR had not yet been implemented at this checkpoint. Account-list CSV/Excel rows now load in Accounts for review and creation with duplicate/currency safeguards; account numbers are ignored, opening balance dates are normalized from ISO or DD/MM/YYYY, and statement classification handles dated balance sheets separately from transaction rows. The earlier `PEND-005` is resolved by approved `DB-027`.
- Account-list import milestone (8 November 2026): React coverage verifies CSV/Excel review, duplicate and unsupported-currency exclusion, safe opening-balance conversion, date normalization, account-number omission, oversize rejection and partial creation failure handling. Spreadsheet classification covers dated budgets and account lists while keeping account-name statement headers in statement flow. Verification: 75 Rust tests, 74 frontend tests, production build and full Edge desktop E2E pass; Rust formatting passes.
- CI milestone (8 November 2026): `.github/workflows/ci.yml` runs on Windows for pushes and pull requests, checking rustfmt, frontend tests/build and Rust tests. PEND-017/018 are resolved; first hosted run remains unverified. Local equivalents pass.
- Refund audit milestone (8 November 2026): migration 0015 preserves existing audit rows and extends allowed events; linked refunds now add receipt, void and reversal events to the original expense atomically, including an identifying description on receipt. Rust migration/lifecycle tests and the transaction-history UI test pass; the Edge workflow covers creating and reversing a linked refund while reviewing the original's audit history.
- Retained-file cleanup hardening (8 November 2026): expired source BLOBs are removed at startup with SQLite secure-delete enabled, followed by a WAL truncate checkpoint. A file-backed regression test verifies expired rows are gone, secure-delete is active and the WAL is empty. Filesystem or SSD-level physical erasure and database encryption at rest remain open. Current coverage before the discard milestone: 76 Rust and 74 frontend tests; after it, 77 Rust and 75 frontend tests.
- Release package checkpoint: rebuilt the optimized Windows application and verified fresh unsigned x64 MSI (4.16 MiB) and NSIS (3.01 MiB) installers with the category management commands. Signing, installing/upgrading the packages, supported Windows versions and release identity remain unverified.
- Import discard milestone (8 November 2026): unfinished statement workspaces can be explicitly discarded, become read-only, and cannot be finalized or modified through the database; staged history and source bytes are retained, with source cleanup still following the approved 90-day period. Finalized statements are not discardable. Migration 0016, backend enforcement, UI action and regression coverage added; `PEND-006` resolved by `DB-034`.
- Statement date-format support: CSV imports now offer explicit ISO, RFC 3339, DD/MM/YYYY, MM/DD/YYYY and DD-MM-YYYY formats. Parsing is strict and never guesses the meaning of ambiguous slash dates. Added backend format and invalid-date tests plus UI mapping coverage. Verification: 78 Rust tests, 75 frontend tests, production build and formatting pass; provider-specific normalization still depends on sample files.
- CSV dialect support: first-row delimiter detection now handles comma, semicolon, tab and pipe separators while ignoring quoted punctuation; header preview, row preview and staging share the same delimiter selection. A repository test verifies semicolon staging with strict DD/MM/YYYY dates and quoted commas. Verification: 80 Rust tests pass.
- First usable product checkpoint (8 November 2026): built ETracker 0.1.0 unsigned x64 MSI and NSIS installers (4.24 MiB and 3.21 MiB), recorded SHA-256 checksums in `RELEASE_NOTES.md`, added a first-use guide and configured tag/manual Windows artifact builds. The complete real Edge E2E flow passes after import-discard changes. The local profile already contains data under the production app identifier, so installing over it was avoided; install/upgrade and native backup file prompts remain open.
- Backup and confirmation hardening: production backup export now uses a native Windows save dialog and writes the encrypted bytes to the selected file; cancellation leaves no output file. Explicit confirmation prompts now await Tauri's asynchronous native dialog result, preventing actions from being treated as accepted merely because a Promise is truthy. Automated tests verify save-path writes, confirmation decline paths, and the full isolated Edge workflow. Native file dialog interaction itself still needs manual verification.
- Scanned-PDF OCR milestone (8 November 2026): added Tesseract.js 7 with bundled English `best_int` data and local WebAssembly worker assets generated from locked dependencies during dev, production, and E2E builds. PDF.js renders image-only pages; OCR text is staged as uncertain review records, with page progress and 200-page/100-MiB limits. No network service is used. Three extraction tests and the real Edge E2E flow with a generated scanned statement verify OCR and uncertain staging. `PEND-008` is resolved by `DB-037`; recognition accuracy on varied real statements, non-English scans and disconnected-machine startup remain manual checks. The rebuilt unsigned MSI and NSIS packages include the OCR assets (10.96 MiB and 9.88 MiB); their hashes are in `RELEASE_NOTES.md`. Final local verification: 80 Rust tests, 82 frontend tests, production build, desktop Edge E2E and both installer bundles passed. Vitest is capped at two workers after unconstrained worker startup timeouts on this host.
- Transaction attachment milestone (8 November 2026): added managed PDF/JPEG/PNG files for transactions, SQLite metadata and hashes, file-integrity and missing-file reporting, user removal, and coordinated inclusion in encrypted backups with legacy database-only backup compatibility. Automated storage, integrity, missing-file, backup/restore and UI-flow coverage passes. Verification: 83 Rust tests, 83 frontend tests, production build, formatting and real Edge E2E pass. Native file picking/opening still needs manual confirmation; files are not encrypted at rest. The OCR-era installers predate this change and must be rebuilt before distribution.
- Design fidelity follow-up (11 November 2026): mapped the shared mono utility to bundled JetBrains Mono; standardized currency output to ISO codes; kept foreign-currency account balances out of the KES portfolio total; corrected dark-mode violet accent contrast; and labeled the shell quarter as calendar-based while identifying its reconciliation shortcut as statement-import reconciliation. Unsupported fiscal-period switching, global ledger search and account-wide reconciliation remain documented backend gaps. Verification: all 84 frontend tests and the production build pass. Browser preview cannot load Tauri-backed financial records, so populated desktop-screen comparison remains pending.

Phase A repository tests cover category filtering, archived-category preservation during edits, audited updates, void balance exclusion, reversals, transfer fees and rollback. Refund tests cover distinct display, proportional category allocations, remaining-amount limits, blocking invalid edits/reversals, and linked receipt/void/reversal audit events. Phase B covers persistent statement staging, source retention, CSV/XLSX/PDF handling, correction/review, duplicate detection, reconciliation, revisions and atomic finalization. Phase C covers budgets, savings, subscriptions and linked actual payments. Phase D analytics tests cover transfer/reversal exclusion, filters, split allocations, monthly aggregation, unusual spending, budget/savings reports and distinct refund totals/trends/category breakdowns; frontend tests cover transaction-level refund entry plus report filters and CSV export. Forecasting tests cover six-month aggregation, subscriptions, budgets, independent snapshots, opt-in scheduled generation, closed-month evaluation and schedule controls in the UI.

## 5. Immediate Next Steps

1. Obtain sanitized provider statement samples before implementing provider-specific parsers.
2. Continue responsive/accessibility review; manually verify Windows install/upgrade, native backup saving and OCR accuracy on varied statements. Review offline behavior, local database protection and decompressed-workbook limits. Persistent diagnostic logging/retention remains open.

## 6. Development Tracking

Implementation progress should be updated as meaningful milestones are completed.

Completed work must be supported by actual implementation and verification rather than planned functionality or assumptions.

- Backend UI alignment (11 November 2026): connected global search to a bounded SQLite query; added server-side transaction text/status/flow/date/account filters and pagination; added savings contribution history that excludes voided or reversed linked transfers. Search/journal checkpoint 799219; Rust 84 tests passed, focused frontend tests passed, full frontend 91 tests passed, and production build passed. Savings history is in current verification.

- Savings contribution ledger and full-filter transaction export: added a persisted contribution history view that excludes linked transfers which are voided or reversed; transaction CSV now exports all matching results across pages up to 10,000 rows. Verified with 84 Rust tests, 11 transaction UI tests, 4 savings UI tests and a successful production build.
