# ETracker — Project Decision Log

**Document ID:** ADR-LOG-001  
**Version:** 0.1  
**Date:** 4 November 2026  
**Status:** Active  
**Project:** ETracker

---

## 1. Purpose

This document records important architectural, financial, database, product and user-experience decisions made during ETracker's requirements and design phases.

It serves as the central record of why particular directions were chosen and which decisions future implementation work must respect.

The decision log is maintained alongside the project documentation. When a decision changes, the change must be recorded rather than silently replacing the previous direction.

### 1.1 Decision statuses

| Status | Meaning |
|---|---|
| Approved | Explicitly accepted project decision or direction. |
| Provisional | Current preferred direction, subject to technical validation or further review. |
| Pending | Requires further investigation, clarification or approval. |
| Superseded | Replaced by a later approved decision. |

A decision being documented does not automatically make it approved.

---

ADR- for architectural and product-level decisions, DB- for the existing database decisions, and UX- for the existing UX decision IDs

# 2. Approved Project and Product Decisions

## ADR-001 — Personal-first financial management application

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-001, DOC-002

### Context
ETracker originated from the need to understand and manage personal finances more accurately, particularly where money moves through multiple accounts and payment channels.

### Decision
ETracker will be developed as a personal-first financial management application.

The initial product will prioritize dependable personal financial tracking and management rather than a commercial multi-user or SaaS model.

### Rationale and implications
- Prioritize the user's own financial workflows.
- Avoid premature multi-user infrastructure.
- Keep the design adaptable to potential future use by other people.
- Focus on financial accuracy, usability and data ownership.

---

## ADR-002 — Local-first application

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-001, DOC-002, DOC-003, DOC-007

### Context
Personal financial information is sensitive, and the application must remain useful without an internet connection.

### Decision
ETracker will follow a local-first design. Core financial operations and access to locally stored financial data must work offline.

### Rationale and implications
- Financial records remain available without internet access.
- Core financial calculations do not depend on external services.
- External integrations must not become dependencies for ordinary application use.
- Local data ownership, backup and recovery are important architectural concerns.

---

## ADR-003 — Windows desktop as the initial platform

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-001, DOC-002, DOC-003, DOC-007

### Context
The initial development and intended deployment target is a desktop environment.

### Decision
Windows desktop will be the initial supported platform.

### Rationale and implications
- Prioritize Windows development and testing.
- Avoid delaying the first release by targeting several operating systems simultaneously.
- Preserve architectural portability where reasonably practical.

Linux support remains a future compatibility objective, not an initial release commitment.

---

## ADR-004 — Initial financial currency: KES

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-001, DOC-002, DOC-004

### Context
The initial financial environment is based on Kenyan financial accounts and transactions.

### Decision
Kenyan Shillings (KES) will be the initial currency used by ETracker.

### Rationale and implications
- Initial financial displays and calculations use KES.
- Monetary values must be stored and calculated precisely.
- The architecture should avoid unnecessarily preventing future multi-currency support.

---

## ADR-005 — No live bank or M-Pesa API integration in v1

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-001, DOC-002, DOC-003

### Context
Statement-based data acquisition is the primary method of importing historical and ongoing financial activity.

### Decision
The initial release will not depend on live M-Pesa or bank API integrations.

Statement imports and manual entry will provide the initial transaction acquisition workflows.

### Rationale and implications
- Avoid dependency on financial institution APIs.
- Keep the application functional without network connectivity.
- Concentrate on reliable statement processing and transaction management.
- Future integrations may be considered separately.

---

## ADR-006 — Manual and imported transactions coexist

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-003, DOC-004

### Context
Not every financial transaction will appear in an imported statement. Cash spending, additional income and backdated transactions may require manual entry.

### Decision
ETracker will support both manual and imported transactions as part of the same financial record system.

Manual transactions must be supported at any point, including backdated entries.

### Rationale and implications
- Users can record financial activity not captured by statements.
- Manual and imported records must participate in the same account and financial calculations.
- Source and provenance information must distinguish how a record entered the system.
- Import processing must account for existing manual records during duplicate detection and revised-statement handling.

---

## ADR-007 — Future Linux compatibility

**Status:** Approved direction  
**Date:** 4 November 2026  
**Related documents:** DOC-003, DOC-007

### Context
Although Windows is the initial target, the architecture should not unnecessarily lock ETracker to Windows.

### Decision
The application should be designed with future Linux compatibility in mind.

### Rationale and implications
- Prefer portable application and domain logic.
- Avoid unnecessary platform-specific dependencies.
- Evaluate platform-specific functionality before adopting it.
- Linux deployment is not part of the initial release commitment.

---

## ADR-008 — Future cloud synchronization readiness

**Status:** Approved direction  
**Date:** 4 November 2026  
**Related documents:** DOC-003, DOC-004, DOC-007

### Context
Future use across multiple devices will require a way to synchronize financial records.

### Decision
The architecture should preserve the possibility of future cloud synchronization.

Cloud synchronization will not be implemented as part of the initial local-first release.

### Rationale and implications
- Use stable record identifiers.
- Maintain clear domain and persistence boundaries.
- Avoid tightly coupling business logic to a single device.
- Consider future synchronization conflicts and data ownership when designing records.
- Do not introduce cloud infrastructure prematurely.

---

## ADR-009 — Optional AI assistance requires explicit consent

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-003, DOC-005

### Context
AI may provide additional forecasting or financial analysis capabilities, but financial information is sensitive and the application must remain useful offline.

### Decision
AI functionality may be offered as an optional external capability. It must require explicit user consent before financial information is sent to an external service.

### Rationale and implications
- Core financial operations remain local.
- AI is not a dependency for the application's essential functionality.
- Users must understand when information leaves the local environment.
- External AI results must not silently modify financial records or initiate financial actions.

---

# 3. Approved Architecture and Financial Data Decisions

## ADR-010 — Modular monolith architecture

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-003

### Context
ETracker requires several interconnected financial modules, but the initial product does not justify distributed services and their operational complexity.

### Decision
ETracker will follow a local-first modular monolith architecture with clear domain boundaries.

### Rationale and implications
- Keep deployment and maintenance manageable.
- Separate financial domains and application responsibilities.
- Avoid premature microservices.
- Maintain boundaries between presentation, application, domain and infrastructure concerns.

The exact technology implementation remains provisional.

---

## DB-003 — Separate financial transactions from account entries

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-003, DOC-004, DOC-006

### Context
A financial event can affect more than one account. Treating every account movement as an independent transaction can distort consolidated financial reporting.

### Decision
A financial transaction will have a transaction header containing shared event information and one or more associated account entries representing actual account balance movements.

### Example
A KSh 1,000 transfer from M-Pesa to a bank account consists of:

- One financial transaction.
- One M-Pesa account entry of -KSh 1,000.
- One bank account entry of +KSh 1,000.

The transfer principal must not be counted as consolidated income or expense.

If the transfer incurs a KSh 10 fee, that fee is recorded separately as an expense.

### Rationale and implications
- Represent one financial event consistently.
- Support transfers between accounts.
- Separate account balances from consolidated income and expense reporting.
- Provide a foundation for historical balances and financial reconciliation.

---

## DB-013 — Opening balances as special ledger transactions

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-004, DOC-006

### Context
Accounts may be added after financial activity has already taken place. ETracker needs a reliable starting balance without falsely recording it as income.

### Decision
Opening balances will be represented as special ledger transactions with an effective date.

Each account will have one controlled opening-balance transaction.

### Rationale and implications
- Opening balances are not ordinary income.
- Transactions before the opening date must not be counted again on top of the opening position.
- Corrections to opening balances must use controlled adjustments rather than casual rewrites.
- Historical account balances must respect the effective opening date.

---

## DB-014 — Split-category transactions

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-004, DOC-006

### Context
One purchase may contain items belonging to several expense categories.

### Decision
ETracker will support transactions whose amounts are allocated across multiple categories.

### Example
A KSh 2,000 purchase may be classified as:
- Food: KSh 1,500.
- Household: KSh 500.

### Rationale and implications
- Improve category-level financial reporting.
- Support more accurate budget comparisons.
- Ensure allocations reconcile to the amount being classified.

---

## DB-015 — Controlled financial corrections

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-004, DOC-006

### Context
Financial records may contain errors or require correction after import or manual entry.

### Decision
Financial corrections will use controlled edits, voids and reversals with an audit trail.

### Rationale and implications
- Preserve the history of financial changes.
- Retain original imported information.
- Exclude voided transactions from active balances.
- Represent reversals as separate financial events linked to the original.
- Perform correction operations atomically.
- Avoid destructive deletion of financial history.

---

## DB-016 — Historical account balances

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-004, DOC-006

### Context
Users need to understand their financial position at a previous date, not only the current balance.

### Decision
ETracker will support historical account balance calculations for any relevant historical date.

### Rationale and implications
- Effective transaction dates must be distinct from creation and import timestamps.
- Historical calculations must exclude voided entries.
- Corrections must affect the appropriate historical periods.
- Account balances must be reproducible from the ledger.

---

## DB-017 — Separate account entries from category allocations

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-003, DOC-004, DOC-006

### Context
Actual account movements and financial classification represent different concepts.

### Decision
Account entries will represent actual account balance movements. Category allocations will represent how an entry is classified.

One account entry may have multiple category allocations.

### Rationale and implications
- Prevent classification from being confused with actual money movement.
- Support split-category transactions.
- Allow transfers and opening balances without ordinary income or expense allocations.
- Require applicable allocations to reconcile to the amount being classified.

---

# 4. Approved Import and Reconciliation Decisions

## DB-018 — Duplicate detection during import validation

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-004, DOC-006

### Context
Statements may overlap or contain transactions already recorded manually or through previous imports.

### Decision
Duplicate detection will occur during import validation.

The system will identify likely duplicates and offer automatic handling for sufficiently confident matches or manual review where necessary.

### Rationale and implications
- Avoid double-counting financial activity.
- Compare imported records against existing financial records.
- Preserve the user's ability to review uncertain matches.
- Duplicate detection must not rely on a single field alone.

---

## DB-019 — Uncertain and invalid records remain in staging

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-004, DOC-006

### Context
Statement extraction and normalization may produce incomplete, ambiguous or invalid records.

### Decision
Uncertain and invalid records will be held for review in the import staging area.

No subset of an import may be committed prematurely when the batch has unresolved records that prevent valid finalization.

### Rationale and implications
- Users can correct extraction or mapping errors before finalization.
- Uncertain financial data must not silently enter the ledger.
- Import validation must cover the entire batch.
- The authoritative ledger remains protected from incomplete imports.

---

## DB-020 — Reconciliation discrepancies block finalization

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-004, DOC-006

### Context
Statement balances provide an important verification mechanism for imported account activity.

### Decision
When reconciliation is available and a discrepancy is detected, the import cannot be finalized until the discrepancy is resolved.

### Rationale and implications
- Avoid accepting unexplained differences in account balances.
- Provide a clear path for investigating missing, duplicated or incorrectly extracted records.
- Never invent adjustments merely to force a reconciliation result.

---

## UX-011 — Live import activity and technical logs

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-004, DOC-005, DOC-006

### Context
Statement imports can involve several processing stages and may take time, especially when OCR or complex document extraction is required.

### Decision
The import workspace will provide live processing activity and technical logs.

The workspace will include three tabs:
1. Activity.
2. Records.
3. Logs.

### Rationale and implications
Logs should support troubleshooting and transparency. Relevant log information includes:
- Timestamp.
- Processing stage.
- Severity: INFO, WARNING or ERROR.
- Description.
- Related record identifier where applicable.
- Error details and corrective actions where appropriate.

Logs must avoid unnecessary exposure of sensitive financial information. Completed import logs should remain available for review.

---

## UX-012 — Statement-first finance workspace and restrained visualizations

**Status:** Approved  
**Date:** 7 November 2026  
**Related documents:** DOC-002, DOC-005

### Context
Historical spending should be captured primarily from financial statements, while users also need a practical way to plan budgets and understand spending patterns.

### Decision
Statement imports are the primary route for adding past spending. Manual transaction entry remains available for cash spending and other gaps. The finance workspace will include an editable, spreadsheet-style monthly plan, with actuals alongside planned amounts. Analytics charts may use accessible native SVG components; a separate charting or UI-generation tool is not required. Large page backgrounds use the user's subtle, low-contrast accounting grid direction in both light and dark themes, while content surfaces remain clear.

### Rationale and implications
- Prioritize reviewable statement data and reconciliation for historical records.
- Keep manual entry for activity that does not appear on statements.
- Present plans and actuals together in a finance-focused worksheet.
- Preserve exact chart values for keyboard and screen-reader users.
- Keep decorative background treatment away from chart and control surfaces.

---

## DB-021 — Separate persistent import staging area

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-004, DOC-006

### Context
Import processing may require multiple rounds of review and correction.

### Decision
Imported records will be stored in a separate staging area before entering the authoritative financial ledger.

Unfinished imports must survive application closure and reopening.

### Rationale and implications
- Unfinished batches do not affect account balances.
- Users can resume interrupted imports.
- Staging records remain separate from finalized financial records.
- Discard, cancellation and retention rules require further specification.

---

## DB-022 — Atomic import finalization

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-003, DOC-004, DOC-006

### Context
A failed import must not leave the financial ledger partially updated.

### Decision
Finalizing an import will be performed as one atomic database commit.

The system must revalidate the batch and commit the applicable financial records and completion status within a single database transaction.

### Rationale and implications
- If finalization fails, the ledger commit must roll back.
- Staging information may persist incrementally before finalization.
- A batch must not be finalized twice.
- The finalization process must preserve financial integrity.

---

## DB-023 — Multi-factor duplicate detection

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-004, DOC-006

### Context
Statement formats differ, and individual fields may be missing or inconsistent.

### Decision
Duplicate detection will use a hierarchy of multiple identifying factors, including:

- Account identity.
- Transaction reference.
- Amount and currency.
- Transaction date.
- Transaction time where available.
- Description or merchant.
- Transaction type.

### Rationale and implications
Matches will be classified into:
- High-confidence duplicate.
- Probable match requiring manual review.
- No match / new candidate.

Missing transaction time must not automatically be treated as a mismatch.

Institution-specific normalization and duplicate confidence thresholds remain pending implementation decisions.

---

## DB-024 — Three reconciliation outcomes

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-004, DOC-006

### Context
Not all financial statements contain sufficient information to verify their opening and closing balances.

### Decision
Reconciliation will have three distinct outcomes:

1. **Reconciled:** Required balance checks agree. The batch may proceed if other validations pass.
2. **Discrepancy detected:** A balance difference exists. Finalization is blocked until resolved.
3. **Reconciliation unavailable:** The statement does not provide sufficient information to perform the required balance check. Other validations still apply, and the limitation must be recorded.

### Rationale and implications
- Missing reconciliation information is not equivalent to failed reconciliation.
- The system must account for statement conventions, pending transactions, fees and transaction ordering.
- Reconciliation must not fabricate missing financial information.
- An unavailable reconciliation result must be visible to the user.

---

## DB-025 — Revised statement handling

**Status:** Approved  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-004, DOC-006

### Context
A financial institution may issue a revised statement after an earlier version has already been imported.

### Decision
When a revised statement is imported, ETracker will compare it with the earlier version and ask the user how to handle the differences.

The user will be able to choose:

1. Keep the original.
2. Replace with the new version.
3. Merge compatible records.
4. Review differences individually.

### Rationale and implications
- Identify additions, removals and modifications.
- Retain unchanged records without duplication.
- Merge complementary information where compatible.
- Require user confirmation for conflicting financial values.
- Preserve original statements and import history.
- Never silently overwrite previous manual corrections.
- Stage and revalidate changes before atomic finalization.
- Replacement and merging must not destructively erase financial history; controlled adjustments or reversals must be used where necessary.

---

## DB-026 — Local source-file retention

**Status:** Approved
**Date:** 6 November 2026
**Related documents:** DOC-002, DOC-003, DOC-004, DOC-007

**Owner confirmation:** Confirmed by the project owner on 7 November 2026: retain original imported statement files for 90 days from latest relevant batch activity and clean up expired source rows at startup. This is the policy implemented below; it does not claim secure physical erasure.

### Context
Original statement files are needed for traceability and review, while long-term retention of sensitive source files should be bounded.

### Decision
Store the original uploaded bytes locally with the import batch and calculate a SHA-256 content hash. Retain and make the source downloadable for 90 days after the latest import-batch activity. Remove expired source bytes during application startup; staged and finalized financial records and import history remain intact.

### Rationale and implications
- Import processing remains local and available offline.
- Activity refreshes the retention deadline so active workspaces keep their source file.
- Source-file expiration does not delete ledger entries, staged rows or audit history.
- Cleanup is performed when the application starts, so physical deletion may occur after the exact expiry time.
- Rules for discarding or cancelling entire import batches remain pending.

---

## DB-027 — Controlled revised statement resolutions

**Status:** Approved
**Date:** 6 November 2026
**Related documents:** DOC-002, DOC-004, DOC-005, DOC-006

### Context
Revised imports can conflict with posted imported transactions and with later manual edits to those transactions. A user must be able to apply a corrected statement while preserving audit history and ledger traceability.

### Decision
Compare a newly staged statement with the finalized same-account batch selected by the user. Require row-level resolution for changed and ambiguous candidates. Permit keep-original, use-revised and compatible merge. A use-revised decision may reverse a posted imported transaction, including one the user manually edited, and apply the revised staged transaction in the same atomic finalization. The reversal links to the original and records the revision batch in its audit reason. Do not create a second reversal for an already voided or reversed original. Compatible merges may fill missing descriptions only when date, amount, type and confirmed allocations already agree. Unchanged matches remain linked to the earlier imported record and do not post twice.

### Rationale and implications
- Revised imports remain staged and must pass existing validation and reconciliation gates.
- A user explicitly choosing revised values authorizes the reversal and replacement operation.
- Ambiguous matching candidates remain individually visible and unresolved until the user acts.
- Old statement rows, financial transactions, reversals and audit history remain queryable.
- Generic matching is conservative and provider-specific identifiers will improve confidence once verified formats are available.

---

## DB-028 — Local text extraction for selectable PDF statements

**Status:** Approved
**Date:** 6 November 2026
**Related documents:** DOC-002, DOC-003, DOC-004, DOC-005, DOC-006

### Context
The requirements support text-based PDF statements while keeping import processing local. Generic PDFs do not provide a consistent row layout, so inferred dates and amounts cannot safely be posted without format-specific parsing.

### Decision
Use the Rust `pdf-extract` crate for page-level selectable text extraction. Stage each non-empty text line as an uncertain record with its page and line reference. Do not infer or post date, amount, direction, category or transaction reference from generic PDF text. Users must correct each transaction line or skip non-transaction content. If a PDF contains no extractable text, return a recoverable failure and retain the workspace/source file. Scanned-PDF OCR and institution-specific PDF parsers remain separate work.

### Rationale and implications
- PDF statements remain usable for manual correction without claiming a provider parser.
- Headings, balances and summaries cannot silently become ledger entries.
- Selectable line text is retained in the staged description for traceability.
- OCR library selection is pending until the supported Windows deployment and privacy requirements can be validated.

---

## DB-029 — Refund reporting and KES-only v1 planning

**Status:** Approved
**Date:** 7 November 2026
**Related documents:** DOC-002, DOC-004, DOC-005, DOC-006

### Context
Refunds are represented as tagged positive ledger inflows, and the initial release is KES-only. Reports must communicate their effect without misclassifying them or changing the approved ledger representation.

### Decision
- Preserve refunds as tagged income inflows with optional original-expense linkage; do not add a separate financial transaction type.
- Analytics presents gross expenses and refunds separately and also shows net spending calculated as gross expenses minus refunds. Monthly net cash flow includes refund inflows.
- Budget actual expense and category comparisons remain gross; refunds are not silently reassigned as income or netted against category budgets.
- New planning and forecasting records in v1 use KES. Existing records with other currency codes are preserved and shown only in their own currency context; no exchange-rate conversion or cross-currency summation is performed.
- Forecast income baselines and evaluations exclude tagged refunds from ordinary income; future refunds are not projected.

### Rationale and implications
- Keeps the ledger and audit trail intact while making refund effects visible.
- Leaves linked refunds’ original category attribution available for separate refund analysis without changing gross budget actuals.
- Maintains compatibility with existing currency codes without expanding KES-only v1 or rewriting stored data.

---

## DB-030 — Encrypted local backup and restore

**Status:** Approved
**Date:** 7 November 2026
**Related documents:** DOC-002, DOC-004, DOC-005, DOC-006, DOC-007

### Context
The application stores financial records and retained statement source files locally. Users need a portable recovery copy without exposing that data in an unprotected backup file.

### Decision
- Users explicitly create and download a complete SQLite backup; ETracker does not keep a separate backup archive or impose a retention period on downloaded files.
- Encrypt backups with AES-256-GCM and derive the encryption key using PBKDF2-HMAC-SHA256 with 600,000 iterations, a random per-backup salt and nonce. Require a passphrase of at least 12 characters. Never save or transmit the passphrase; a forgotten passphrase cannot be recovered.
- Restore only after authenticating and validating the database. Apply supported older migrations to an isolated in-memory copy, reject unsupported future schemas, run integrity and foreign-key checks, then replace the current database.
- Backup files include all database contents, including statement source files still within their retention period. Users are responsible for storing and deleting downloaded backups safely.

### Rationale and implications
- Authenticated encryption protects downloaded copies against disclosure and tampering.
- Isolated validation and migration ensure invalid files and unsupported schemas are rejected before replacement begins.
- Backups and restores currently buffer the database in memory and pass base64 data over local IPC; large database behavior requires desktop validation and may need a streaming/size-limit design.
- Local database encryption at rest and a guaranteed rollback after a low-level failure during SQLite page replacement are not provided by this workflow and require release review.

---

## DB-031 — Privacy-safe diagnostic export

**Status:** Approved
**Date:** 7 November 2026
**Related documents:** DOC-005, DOC-006, DOC-007

### Decision
Settings may create a user-requested local JSON report containing application/platform versions, SQLite/schema versions, and redacted database integrity and foreign-key status. The report excludes transaction, account, category, statement and personal data; it is downloaded locally and never transmitted by ETracker. Do not persist diagnostic logs until a separate logging and retention policy is approved.

### Rationale and implications
This offers useful first-line support details while limiting sensitive data exposure. It does not replace detailed local integrity results, operational logs, or desktop workflow verification.

---

# 5. Approved Budgeting, Savings and Subscription Decisions

## ADR-011 — Monthly budgets are independent planning instances

**Status:** Approved direction  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-003, DOC-004, DOC-005, DOC-006

### Context
Users need reusable monthly budgets but must also be able to adapt each month to changing financial circumstances.

### Decision
ETracker will support reusable budget templates that generate independent monthly budget instances.

### Rationale and implications
- A reusable template can be used to create a new monthly plan.
- Each generated month is an independent copy.
- Editing one month's budget must not alter previous or future budget instances.
- Actual transactions remain separate from budget plans.
- Actual financial activity is compared against the relevant budget period.

---

## ADR-012 — Separate planned, actual and unallocated financial amounts

**Status:** Approved direction  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-004, DOC-005, DOC-006

### Context
Planned spending and savings should not be confused with actual financial activity.

### Decision
ETracker will distinguish:
- Planned income.
- Actual income.
- Planned expenses.
- Actual expenses.
- Intended savings.
- Actual savings.
- Unallocated surplus.
- Variances between planned and actual amounts.

### Rationale and implications
- A budget is a plan, not a financial transaction.
- Planned savings must not automatically be reported as money already saved.
- Unallocated surplus must remain distinguishable from actual savings.
- Budget performance must be based on actual recorded financial activity.

---

## ADR-013 — Savings are a distinct financial planning concept

**Status:** Approved direction  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-003, DOC-004, DOC-005, DOC-006

### Context
Savings are important to the user's overall financial position and should be tracked alongside income, expenses and budgets.

### Decision
ETracker will support savings goals, intended savings and actual savings.

Actual savings must represent genuine money set aside or explicitly recorded as savings.

### Rationale and implications
- Users can define savings goals and track progress.
- Savings contributions can be associated with goals.
- Intended savings and actual savings remain separate.
- A transfer to a separate savings account must not be counted as an ordinary expense in consolidated reporting.
- Designating money within an existing account as savings must not create an artificial account movement.
- Savings representation must not double-count account balance movements.

The exact relationship between savings goals, contributions and ledger entries remains subject to database design validation.

---

## ADR-014 — Subscription obligations are separate from actual payments

**Status:** Approved direction  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-003, DOC-004, DOC-005

### Context
Recurring financial obligations need to be planned and monitored without creating transactions before money actually moves.

### Decision
Subscription records will represent projected or expected obligations. Actual subscription payments will be represented by separate financial transactions.

### Rationale and implications
- Subscription projections must not be counted as actual expenses.
- Actual payments must be recorded in the financial ledger.
- Subscription records can support budgeting and reminders.
- Subscription status and payment history must remain distinguishable.

---

## ADR-015 — Three primary reminder categories

**Status:** Approved direction  
**Date:** 4 November 2026  
**Related documents:** DOC-002, DOC-005, DOC-007

### Context
The application needs reminders that support consistent financial management without overwhelming the user.

### Decision
The initial reminder categories will cover:

1. Subscription reminders.
2. Statement import reminders.
3. Forecast evaluation reminders.

### Rationale and implications
- Users can be reminded about recurring obligations.
- Statement imports can be prompted according to a configurable cadence.
- Forecasts can be evaluated against actual financial activity.
- Import reminders should consider the last successfully reconciled statement period.

The user's preferred weekly, fortnightly or monthly import reminder cadence remains configurable and has not yet been selected.

---

# 6. Approved UI/UX Direction

## ADR-016 — Light, dark and system theme support

**Status:** Approved direction  
**Date:** 4 November 2026  
**Related documents:** DOC-005

### Context
The application should provide a comfortable and consistent visual experience in different environments.

### Decision
ETracker will support:
- Light theme.
- Dark theme.
- System theme preference.

The initial default will follow the operating system's appearance preference, with a user override available.

### Rationale and implications
- Support different working environments.
- Maintain consistent information hierarchy across themes.
- Validate contrast and readability in both themes.

Exact production colours remain subject to visual and accessibility testing.

---

## ADR-017 — Professional, restrained financial interface

**Status:** Approved direction  
**Date:** 4 November 2026  
**Related documents:** DOC-005

### Context
ETracker is intended to be a serious financial management tool. Its interface should prioritize clarity and trust rather than decorative complexity.

### Decision
The visual direction will be professional, clear and financially oriented.

The interface should avoid excessive card-based layouts and generic template aesthetics.

A subtle spreadsheet-inspired grid may be used as a background or navigation detail, provided it does not compete with financial information.

### Rationale and implications
- Financial figures remain easy to scan.
- Navigation and data hierarchy take priority over decoration.
- Visual elements should support comprehension.
- Background effects must remain restrained.

---

## ADR-018 — Consistent financial colour semantics

**Status:** Approved direction  
**Date:** 4 November 2026  
**Related documents:** DOC-005

### Context
Financial information benefits from consistent visual cues, particularly when users compare income, expenses, savings and warnings.

### Decision
The interface will use consistent colour semantics:

- Green: income, positive financial values and savings progress.
- Purple: navigation, planning and selected highlights.
- Muted red or coral: expenses and negative financial values.
- Amber: warnings and attention states.
- Neutral colours: general content and supporting information.

### Rationale and implications
- Users can recognize financial information more easily.
- Colour must never be the only way to communicate meaning.
- Text, labels, icons and other indicators must also convey important states.

---

# 7. Provisional Technology and Implementation Directions

The following are the current preferred technical choices. They are **not yet final approved technology decisions** because the required environment, dependency, library and implementation evaluations have not been completed.

## PROV-001 — Desktop application framework

**Status:** Provisional

**Current direction:** Tauri 2.

**Reason:** A lightweight desktop application framework that can support a web-based frontend and native backend capabilities.

**Pending validation:**
- Windows development and packaging.
- Linux portability.
- Plugin and dependency compatibility.
- Security and permission model.
- Long-term maintenance considerations.

---

## PROV-002 — Frontend framework

**Status:** Provisional

**Current direction:** React with TypeScript.

**Reason:** A structured component-based interface with strong typing and a mature ecosystem.

**Pending validation:**
- Integration with Tauri.
- Frontend testing approach.
- State management needs.
- UI component and charting library choices.

---

## PROV-003 — Backend language

**Status:** Provisional

**Current direction:** Rust.

**Reason:** Native application capabilities, strong type safety and suitability for local financial processing.

**Pending validation:**
- Application architecture.
- Database access libraries.
- Document processing integrations.
- Error handling and testing conventions.

---

## PROV-004 — Local database

**Status:** Provisional

**Current direction:** SQLite.

**Reason:** A mature embedded relational database suitable for a local-first desktop application.

**Pending validation:**
- Database access approach.
- Migration tooling.
- Transaction and concurrency requirements.
- Backup and recovery strategy.
- Future synchronization implications.

---

## PROV-005 — Styling framework

**Status:** Provisional

**Current direction:** Tailwind CSS.

**Reason:** Utility-based styling and consistent design implementation.

**Pending validation:**
- Compatibility with the chosen frontend setup.
- Theme implementation.
- Design token conventions.
- Accessibility and maintainability.

---

## PROV-006 — Rust-managed database access

**Status:** Provisional

**Current direction:** Database operations will be managed through the Rust backend rather than allowing unrestricted direct database access from the frontend.

**Reason:** Keep persistence responsibilities controlled and maintain a clear application boundary.

**Pending validation:**
- Exact command and service boundaries.
- Query and migration libraries.
- Error propagation.
- Transaction handling.

---

# 8. Pending Decisions and Design Investigations

The following matters require additional research, technical validation or explicit approval before they become binding implementation decisions.

## 8.1 Financial database and import processing

| ID | Pending decision | Priority | Related document |
|---|---|---|---|
| PEND-001 | Final physical SQLite schema and relationships | High | DOC-004 |
| PEND-002 | Institution-specific statement normalization | High | DOC-004 |
| PEND-003 | Duplicate confidence thresholds and matching rules | High | DOC-004 |
| PEND-004 | Treatment of pending transactions in reconciliation | High | DOC-004 |
| PEND-006 | Import batch cancellation and discard behavior (source-file retention resolved in DB-026) | Medium | DOC-004 |
| PEND-007 | Exact savings-goal, contribution and ledger relationship | Medium | DOC-004 |
| PEND-008 | Local OCR library for scanned statement files (text-based PDF extraction resolved in DB-028) | High | DOC-003, DOC-004 |

## 8.2 User interface and experience

| ID | Pending decision | Priority | Related document |
|---|---|---|---|
| PEND-009 | Final colour palette and contrast validation | Medium | DOC-005 |
| PEND-010 | Typography and spacing system | Medium | DOC-005 |
| PEND-011 | Main navigation structure and final wireframes | High | DOC-005 |
| PEND-014 | Detailed table interactions and presentation | Medium | DOC-005 |
| PEND-015 | Notification presentation and interaction | Medium | DOC-005 |
| PEND-016 | Accessibility and responsive behaviour specifications | Medium | DOC-005 |

## 8.3 Testing and quality assurance

| ID | Pending decision | Priority | Related document |
|---|---|---|---|
| PEND-017 | Final testing toolchain | High | DOC-006 |
| PEND-018 | CI checks and automation configuration | Medium | DOC-006 |
| PEND-019 | Test coverage expectations and release gates | Medium | DOC-006 |
| PEND-020 | Acceptance criteria refinement where required | Medium | DOC-002, DOC-006 |

## 8.4 Deployment and maintenance

| ID | Pending decision | Priority | Related document |
|---|---|---|---|
| PEND-021 | Supported Windows versions | High | DOC-007 |
| PEND-022 | Installer format and distribution | Medium | DOC-007 |
| PEND-023 | Code signing and integrity verification | Medium | DOC-007 |
| PEND-024 | Application update distribution strategy | Medium | DOC-007 |
| PEND-028 | Persistent diagnostic logging and retention policy | Medium | DOC-007 |
| PEND-029 | Release branch, tagging and versioning policy | Medium | DOC-007 |
| PEND-030 | Release cadence | Low | DOC-007 |
| PEND-031 | Secure storage and handling of optional AI credentials | High | DOC-003, DOC-007 |

---

# 9. Decision Governance and Change Management

Decisions recorded here should remain consistent with the SRS, architecture, database, UI/UX, testing and deployment documentation.

A change to an approved decision must follow a controlled process:

1. Identify the existing decision and explain the problem or new requirement.
2. Propose the alternative and explain why it is preferable.
3. Assess the impact on requirements, architecture, database, implementation, testing and deployment.
4. Discuss the change and obtain approval.
5. Update this decision log with the new status and rationale.
6. Update affected project documents.
7. Preserve the history of the superseded decision.

### 9.1 Decision ownership

The project owner has final approval over project scope, financial behaviour and product direction.

Technical recommendations may be proposed by the technical lead, but they do not become approved project decisions until accepted.

### 9.2 Documentation references

| Document ID | Document |
|---|---|
| DOC-001 | Project Overview |
| DOC-002 | Software Requirements Specification |
| DOC-003 | System Architecture |
| DOC-004 | Database Design |
| DOC-005 | UI/UX Specification |
| DOC-006 | Testing and Quality Assurance |
| DOC-007 | Deployment and Maintenance |

### 9.3 Maintenance

This document must be updated when:
- A pending decision is approved.
- A provisional direction is confirmed or rejected.
- An approved decision is changed or superseded.
- A new significant architectural, financial or product decision is made.

Routine implementation details do not need separate decision records unless they materially affect project architecture, financial correctness, data integrity, security or scope.

---

# 10. Current Decision Status Summary

| Category | Status |
|---|---|
| Personal-first, local-first product direction | Approved |
| Windows desktop initial target | Approved |
| KES initial currency | Approved |
| Offline core functionality | Approved |
| No live M-Pesa/bank APIs in v1 | Approved |
| Manual and imported transactions | Approved |
| Future Linux compatibility direction | Approved |
| Future cloud synchronization readiness | Approved |
| Optional AI with explicit consent | Approved |
| Modular monolith architecture | Approved |
| Financial ledger and account-entry separation | Approved |
| Import staging and atomic finalization | Approved |
| Reconciliation and revised-statement handling | Approved |
| Budget and savings separation | Approved |
| Subscription projection separation | Approved |
| Three reminder categories | Approved |
| Light, dark and system themes | Approved direction |
| Tauri 2, React, TypeScript, Rust and SQLite | Provisional |
| Exact physical database schema | Pending |
| Import libraries and institution-specific processing | Pending |
| Final UI design and component libraries | Pending |
| Testing toolchain | Pending |
| Installer, backup, encryption and update policies | Partially approved; installer and update details pending |

---

**End of Decision Log**

**Document status:** Active  
**Version:** 0.1  
**Last updated:** 4 November 2026
