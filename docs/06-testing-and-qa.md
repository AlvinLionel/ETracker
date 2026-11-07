# ETracker — Testing and Quality Assurance

**Document ID:** DOC-006  
**Version:** 0.2  
**Status:** Revised Draft — Pre-Implementation  
**Last Updated:** 4 November 2026

---

## 1. Introduction

### 1.1 Purpose

This document defines the testing and quality assurance strategy for ETracker.

It establishes how the application will be verified throughout development, including the tools, testing levels, test scenarios, financial correctness checks, defect management and release acceptance criteria.

The purpose is to ensure that ETracker is dependable, accurate, secure and usable before it is considered ready for regular financial use.

### 1.2 Quality Objectives

Testing must establish that:

1. Financial calculations are correct.
2. Account balances remain consistent.
3. Transactions and corrections preserve financial history.
4. Imports do not introduce duplicate or incomplete financial records.
5. Reconciliation behaves according to approved rules.
6. Budgets remain separate from actual financial activity.
7. Savings are tracked without double-counting.
8. Forecasts remain distinguishable from actual results.
9. Core functionality works offline.
10. Data survives application closure, updates and recovery operations.
11. Sensitive financial information is protected.
12. The interface communicates financial information accurately.

### 1.3 Current Testing Status

This is a pre-implementation testing strategy.

No application functionality has been implemented or tested as part of this document.

All test scenarios below are planned tests. Test execution results will be recorded separately when implementation begins.

---

## 2. Quality Assurance Principles

### 2.1 Financial Correctness First

Financial accuracy takes priority over visual polish and convenience.

A feature must not be accepted simply because its interface works if its underlying calculations or records are incorrect.

### 2.2 Test at Multiple Levels

Testing must cover individual functions, integrated modules, complete workflows and user acceptance.

### 2.3 Prevent Regression

Previously verified functionality must be retested when changes affect related components.

Financial calculations and import operations require particularly strong regression coverage.

### 2.4 Reproducible Testing

Tests should use controlled inputs and documented expected results.

Failures must be reproducible wherever practical.

### 2.5 Data Integrity

Testing must verify not only that correct results are produced, but also that incorrect or incomplete operations do not corrupt existing records.

### 2.6 Traceability

Important requirements must map to test cases and, eventually, recorded test results.

### 2.7 Honest Test Reporting

The project must distinguish:

- Planned tests.
- Implemented automated tests.
- Manually executed tests.
- Passed tests.
- Failed tests.
- Blocked tests.
- Tests not yet executed.

A test must never be reported as passed merely because its implementation exists.

---

## 3. Testing Levels

### 3.1 Unit Testing

Tests individual functions and domain operations in isolation.

Examples:
- Monetary calculations.
- Balance calculations.
- Date handling.
- Category allocation validation.
- Duplicate matching.
- Budget variance calculations.
- Forecast calculations.

Unit tests should be fast and deterministic.

### 3.2 Component Testing

Tests individual application components and their expected behaviour.

Examples:
- Transaction forms.
- Account balance displays.
- Import record tables.
- Budget allocation components.
- Notification components.
- Chart data presentation.

### 3.3 Integration Testing

Tests interactions between application modules and infrastructure.

Examples:
- Rust backend and SQLite.
- Transaction service and account-entry persistence.
- Import pipeline and ledger finalization.
- Budget calculations and transaction queries.
- Attachment storage and database references.

### 3.4 System Testing

Tests the complete application as an integrated desktop product.

Examples:
- First launch and database initialization.
- Creating accounts and transactions.
- Importing statements.
- Reviewing budgets.
- Viewing analytics.
- Creating backups.
- Restoring application data.

### 3.5 Acceptance Testing

Verifies that the application satisfies its agreed requirements and intended workflows.

Acceptance testing should use realistic scenarios based on the user's financial management needs.

---

## 4. Testing Categories

### 4.1 Functional Testing

Verifies that features behave according to requirements.

Coverage includes:
- Account management.
- Transaction management.
- Categories.
- Statement imports.
- Budgeting.
- Savings.
- Subscriptions.
- Analytics.
- Forecasting.
- Notifications.
- Settings.

### 4.2 Financial Accuracy Testing

Verifies:
- Account balances.
- Consolidated balances.
- Income and expense totals.
- Transfers.
- Refunds.
- Fees.
- Split-category allocations.
- Historical balances.
- Budget performance.
- Savings progress.

Financial calculations require deterministic expected results.

### 4.3 Data Integrity Testing

Verifies:
- Database constraints.
- Referential integrity.
- Transaction atomicity.
- Audit history.
- Controlled corrections.
- Migration safety.
- Duplicate prevention.
- Recovery consistency.

### 4.4 Import Testing

Verifies:
- PDF extraction.
- Scanned PDF and OCR processing.
- Excel workbook inspection.
- CSV parsing.
- Field mapping.
- Formula interpretation.
- Duplicate detection.
- Reconciliation.
- Revised statement handling.
- Staging recovery.
- Atomic finalization.

### 4.5 Offline Testing

Verifies that core financial operations remain functional without an internet connection.

### 4.6 Security and Privacy Testing

Verifies:
- Local data protection.
- Safe credential handling.
- Sensitive information handling.
- External AI consent.
- File validation.
- Logging practices.
- Application update integrity.

### 4.7 Performance Testing

Verifies acceptable performance when working with:
- Large transaction histories.
- Multiple accounts.
- Large statement imports.
- Extensive category allocations.
- Historical analytics.
- Multiple budget periods.

### 4.8 Usability and Visual Testing

Verifies:
- Navigation clarity.
- Form behaviour.
- Table readability.
- Chart interpretation.
- Theme consistency.
- Error messages.
- Keyboard accessibility.
- Responsive desktop layouts.

### 4.9 Regression Testing

Verifies that changes do not break previously accepted functionality.

Regression tests should prioritize shared financial services and database operations.

---

## 5. Financial Accuracy Test Scenarios

These scenarios establish foundational expected financial behaviour.

### QA-FIN-001 — Basic Account Balance

**Objective:** Verify that account balances are calculated from valid account entries.

Initial balance: KSh 5,000.

| Activity | Amount |
|---|---:|
| Opening balance | +5,000 |
| Income | +2,000 |
| Expense | -500 |
| Expected balance | **KSh 6,500** |

**Expected result:** The account balance is KSh 6,500.

### QA-FIN-002 — Internal Transfer

**Objective:** Verify that transfers update individual accounts without changing consolidated financial position.

Initial balances:

| Account | Balance |
|---|---:|
| M-Pesa | 5,000 |
| Bank | 2,000 |
| Consolidated | 7,000 |

Transfer KSh 1,000 from M-Pesa to bank.

Expected balances:

| Account | Balance |
|---|---:|
| M-Pesa | 4,000 |
| Bank | 3,000 |
| Consolidated | **7,000** |

**Expected result:** Individual account balances change, but consolidated balance remains KSh 7,000. Transfer principal is not counted as income or expense.

### QA-FIN-003 — Transfer with Fee

**Objective:** Verify that a transfer fee is recorded separately as an expense.

Transfer: KSh 1,000.

Fee: KSh 10.

Expected balances:

| Account | Balance |
|---|---:|
| M-Pesa | 3,990 |
| Bank | 3,000 |
| Consolidated | **6,990** |

**Expected result:** The transfer principal is excluded from income and expenses. The KSh 10 fee is recorded as an expense.

### QA-FIN-004 — Refund Handling

**Objective:** Verify that refunds are distinguished from ordinary income.

Scenario:
- Original expense: KSh 1,500.
- Subsequent refund: KSh 500.

**Expected result:**
- The refund is recorded as a separate financial event.
- It is not classified as ordinary income.
- Reporting keeps the refund separate from ordinary income and gross expenses.
- Account balance reflects the actual refund movement.

### QA-FIN-005 — Historical Transaction Correction

**Objective:** Verify that historical balances respond correctly to controlled changes.

Scenario:
1. Record an expense of KSh 500 on 10 January.
2. Record income of KSh 2,000 on 15 January.
3. Query the balance on 12 January.
4. Correct the January expense through the controlled correction workflow.
5. Query the historical balance again.

**Expected result:** The historical balance reflects the corrected effective financial event. Audit history preserves the original value and correction.

### QA-FIN-006 — Monetary Precision

**Objective:** Verify that monetary operations avoid binary floating-point inaccuracies.

Test:
- Repeated addition and subtraction of fractional currency values.
- Multiple category allocations.
- Aggregation across accounts.
- Refund and fee calculations.

**Expected result:** Stored and calculated monetary values remain exact within the supported currency precision.

### QA-FIN-007 — Voided Transaction

**Objective:** Verify that voided transactions no longer affect active financial calculations.

**Expected result:**
- The original record remains available for audit.
- Its account entries are excluded from active balances.
- Relevant reports exclude the voided event.
- The application does not silently delete the original record.

### QA-FIN-008 — Split-Category Expense

**Objective:** Verify that one financial entry can be classified across multiple categories.

Scenario:
- One expense: KSh 2,000.
- Food allocation: KSh 1,500.
- Household allocation: KSh 500.

**Expected result:**
- One account movement of KSh 2,000.
- Two category allocations.
- Total allocations equal KSh 2,000.
- No duplicate balance movement.

### QA-FIN-009 — Allocation Mismatch

**Objective:** Verify that invalid category allocations cannot be finalized.

Scenario:
- Expense: KSh 2,000.
- Allocations: KSh 1,500 and KSh 400.

**Expected result:** The system identifies the KSh 100 discrepancy and prevents completion until resolved.

### QA-FIN-010 — Opening Balance

**Objective:** Verify that an opening balance is not treated as ordinary income.

**Expected result:**
- Opening balance affects account balance.
- It does not increase income totals.
- It is represented as a controlled special financial event.
- Duplicate opening balance creation is prevented.

### QA-FIN-011 — Reversal

**Objective:** Verify that reversals are separate linked financial events.

**Expected result:**
- Original transaction remains available.
- Reversal references the original.
- Both events remain traceable.
- Net balance impact follows the correction rules.

---

## 6. Import Testing

### 6.1 PDF Import Tests

**QA-IMP-001 — Standard PDF extraction**

Verify extraction of dates, descriptions, references, amounts and transaction types from supported statement formats.

The generic text-based PDF path currently extracts selectable page text into uncertain page/line records for manual correction. It does not claim automatic bank or M-Pesa field extraction.

**QA-IMP-002 — Scanned PDF extraction**

Verify OCR processing and identification of uncertain extracted values.

Scanned-PDF OCR remains unsupported; the application must report a recoverable extraction failure and preserve the saved workspace/source file.

**QA-IMP-003 — Extraction errors**

Verify that uncertain records are held for review rather than silently accepted.

**QA-IMP-004 — Source traceability**

Verify that extracted records retain references to their original statement locations where supported.

### 6.2 Excel and CSV Tests

**QA-IMP-005 — Workbook inspection**

Verify that workbook inspection identifies:
- Worksheets.
- Tables.
- Headers.
- Data ranges.
- Relevant columns.
- Cell data types.
- Formatting.

The current generic XLSX path verifies worksheet selection, header-row mapping, typed cell extraction, source-row references, and safe staging of formula-derived mapped date or amount cells. Table and style inspection remain future provider-parser work.

**QA-IMP-006 — Formula interpretation**

Verify that formulas and their available calculated results are inspected appropriately.

Stale, missing or unsupported formula results must be flagged.

**QA-IMP-007 — Summary exclusion**

Verify that calculated totals and summary rows are not imported as ordinary transactions.

**QA-IMP-008 — Field mapping**

Verify that the user can review and correct suggested mappings before import.

**QA-IMP-009 — CSV parsing**

Verify supported CSV structures, date formats, amount formats and invalid rows.

### 6.3 Duplicate Detection Tests

**QA-IMP-010 — Exact duplicate**

Verify that a high-confidence matching record is identified.

**QA-IMP-011 — Probable duplicate**

Verify that a likely match with incomplete or conflicting evidence is presented for manual review.

**QA-IMP-012 — Missing transaction time**

Verify that missing time information does not automatically prevent a valid duplicate candidate from being identified.

**QA-IMP-013 — Different account**

Verify that otherwise similar transactions in different accounts are not automatically treated as duplicates.

**QA-IMP-014 — Different transaction reference**

Verify that transaction references are considered as strong matching evidence.

**QA-IMP-015 — Different transaction type**

Verify that transactions with incompatible types are not automatically treated as identical.

### 6.4 Reconciliation Tests

**QA-IMP-016 — Successful reconciliation**

Verify that matching statement and calculated balances produce a reconciled outcome.

**QA-IMP-017 — Discrepancy detected**

Verify that a discrepancy blocks finalization.

**QA-IMP-018 — Reconciliation unavailable**

Verify that a statement without sufficient balance information receives an unavailable status.

**Expected result:** Other mandatory validation checks still apply, and finalization may proceed with the limitation explicitly recorded.

**QA-IMP-019 — Pending transactions**

Verify that statement-specific pending transaction conventions do not produce incorrect reconciliation results.

**QA-IMP-020 — Fees and balance conventions**

Verify that relevant statement conventions and fees are accounted for correctly.

### 6.5 Staging and Finalization Tests

**QA-IMP-021 — Persistent staging**

Verify that an unfinished import survives application closure and reopening.

**QA-IMP-022 — No premature ledger changes**

Verify that staged records do not affect account balances before finalization.

**QA-IMP-023 — Unresolved record blocking**

Verify that unresolved mandatory records prevent finalization.

**QA-IMP-024 — Atomic finalization success**

Verify that all approved records are committed together.

**QA-IMP-025 — Atomic finalization failure**

Simulate a database failure during finalization.

**Expected result:** No partial ledger commit remains.

**QA-IMP-026 — Double finalization prevention**

Verify that an already finalized batch cannot be committed again.

### 6.6 Revised Statement Tests

**QA-IMP-027 — Unchanged records**

Verify that unchanged transactions are not duplicated.

**QA-IMP-028 — Added records**

Verify that newly detected records can be added through controlled finalization.

**QA-IMP-029 — Removed records**

Verify that removed records are presented for review and are not silently deleted.

**QA-IMP-030 — Modified records**

Verify that changed amounts, descriptions or dates are identified.

**QA-IMP-031 — Conflicting manual corrections**

Verify that revised statement information does not silently overwrite a manually corrected ledger record.

**QA-IMP-032 — Merge and replacement**

Verify that the selected action is applied through staging and controlled financial changes while preserving historical import information.

The current regression suite compares generic mapped imports, classifies unique unchanged/modified/add/remove cases, marks competing candidates for review, requires explicit resolution before finalization, preserves unchanged ledger entries, and creates a linked reversal when the user explicitly chooses revised values for a modified posted imported transaction.

---

## 7. Budgeting and Savings Testing

The implemented planning slice verifies independent monthly snapshots, active category validation, uniqueness of category plan lines, month bounds, actual totals by budget currency and period, and exclusion of voided/reversed transactions. Savings tests verify manual contributions do not create ledger movements, qualifying incoming transfers link once, and reversed linked transfers stop counting toward progress. Subscription tests verify changed terms/status are historized, projections do not create transactions, existing posted expenses link only once, and wrong-currency, voided or reversed payments do not count. Frontend coverage verifies budget performance, template/month editing, savings progress/contribution/transfer linking, and subscription payment linking.

### 7.1 Budget Template Tests

**QA-BUD-001 — Template creation**

Verify that reusable templates can be created and saved.

**QA-BUD-002 — Independent monthly instance**

Verify that creating a new budget period from a template produces an independent copy.

**QA-BUD-003 — Historical independence**

Verify that changing a new monthly budget does not change previous periods.

**QA-BUD-004 — Template modification**

Verify that changing a template does not silently modify existing budget instances.

### 7.2 Planned Versus Actual Tests

**QA-BUD-005 — Planned expense separation**

Verify that a planned expense does not create a ledger transaction.

**QA-BUD-006 — Actual expense comparison**

Verify that qualifying ledger transactions are reflected in the correct budget period.

**QA-BUD-007 — Historical period calculation**

Verify that actuals use effective transaction dates.

**QA-BUD-008 — Refund handling**

Verify that refunds are represented consistently in budget performance calculations.

### 7.3 Savings Tests

**QA-SAV-001 — Intended savings**

Verify that planned savings remain separate from actual contributions.

**QA-SAV-002 — Actual contribution**

Verify that actual savings progress is based on qualifying recorded contributions.

**QA-SAV-003 — Unallocated surplus**

Verify that unallocated funds are not automatically counted as actual savings.

**QA-SAV-004 — Savings account transfer**

Verify that a transfer into a designated savings account does not create an artificial expense or duplicate balance movement.

**QA-SAV-005 — Same-account savings designation**

Verify that designating funds as saved without moving money between accounts does not create an artificial account entry.

**QA-SAV-006 — Goal progress**

Verify that contributions update the appropriate savings goal without double-counting.

---

## 8. Subscription Testing

**QA-SUB-001 — Subscription creation**

Verify that subscription records can be created with valid payment information.

**QA-SUB-002 — Recurrence calculation**

Verify expected future payment dates for supported frequencies.

**QA-SUB-003 — Subscription status**

Verify active, paused and cancelled behaviour.

**QA-SUB-004 — Actual payment separation**

Verify that projected subscription costs are not counted as actual expenses.

**QA-SUB-005 — Payment history**

Verify that actual subscription payments remain separate financial transactions.

**QA-SUB-006 — Subscription reminders**

Verify reminder scheduling and notification behaviour.

---

## 9. Analytics and Forecast Testing

### 9.1 Analytics

**QA-ANA-001 — Income and expense summaries**

Verify that totals match authoritative financial records.

**QA-ANA-002 — Category breakdown**

Verify that split-category allocations are aggregated correctly.

**QA-ANA-003 — Transfer exclusion**

Verify that internal transfer principal is excluded from consolidated income and expense totals.

**QA-ANA-004 — Historical filtering**

Verify that date and account filters return correct results.

**QA-ANA-005 — Refund reporting**

Verify refunds appear separately from ordinary income and expenses, and linked split refunds follow the original category proportions.

Analytics tests verify posted ordinary income, gross expenses and separately reported refunds against the ledger, exclude internal transfers and voided or reversed activity, respect date/currency/account/category filters, report allocated values for split transactions, surface repeated descriptions and transactions at least twice the category's six-month average after three prior examples, compare stored budget plans to actuals, show savings-goal progress and export report data as CSV. Linked refunds inherit their original expense's category distribution; refund values are not netted against gross expenses or mixed into ordinary income. The what-if scenario is temporary client state and does not write to the ledger or budget records.

Forecasting tests verify local generation, six-completed-month averaging, normalized active subscription costs, future budget context, independent snapshot preservation, assumptions and limitations, opt-in monthly scheduling with once-per-month idempotence, evaluation after a forecast month ends, duplicate-evaluation rejection and actual variance persistence. Forecast generation and evaluation do not create or modify ledger activity.

### 9.2 Forecasting

**QA-FOR-001 — Local forecast availability**

Verify that core forecasting functionality works without an internet connection.

**QA-FOR-002 — Forecast snapshot preservation**

Verify that generating a new forecast does not overwrite previous snapshots.

**QA-FOR-003 — Forecast evaluation**

Verify that predicted values can be compared against actual results.

**QA-FOR-004 — Data limitations**

Verify that incomplete financial information is communicated in forecast results.

**QA-FOR-005 — External AI consent**

Verify that external AI processing cannot occur without explicit user consent.

**QA-FOR-006 — No automatic financial actions**

Verify that forecasts do not automatically create, modify or execute financial transactions.

---

## 10. Offline, Security and Privacy Testing

### 10.1 Offline Tests

**QA-OFF-001 — Offline launch**

Verify that the application starts without internet connectivity.

**QA-OFF-002 — Offline financial operations**

Verify that core account and transaction functionality works offline.

**QA-OFF-003 — Offline import processing**

Verify supported local statement processing without network access.

**QA-OFF-004 — Optional service unavailability**

Verify that failure of optional external services does not disable core functionality.

### 10.2 Security Tests

**QA-SEC-001 — Sensitive logging**

Verify that logs do not expose unnecessary transaction details, credentials or sensitive file contents.

**QA-SEC-002 — Credential handling**

Verify that sensitive credentials are not stored in ordinary plaintext application settings.

**QA-SEC-003 — File validation**

Verify that unsupported or malformed files are handled safely.

**QA-SEC-004 — Database integrity**

Verify that invalid operations cannot create broken financial relationships.

**QA-SEC-005 — External AI privacy**

Verify that data transmission follows the user's explicit consent and the application's stated privacy behaviour.

**QA-SEC-006 — Dependency review**

Verify that dependencies are checked for known security issues as part of the maintenance process.

---

## 11. Performance Testing

Performance tests should use representative datasets rather than only small demonstration records.

Areas to evaluate:

- Application startup.
- Database initialization.
- Transaction searches.
- Historical balance queries.
- Dashboard calculations.
- Large statement extraction.
- Duplicate matching.
- Reconciliation.
- Import finalization.
- Analytics generation.
- Backup creation and restoration.

Performance targets must be established after representative implementation benchmarks.

A performance optimization must not compromise financial correctness.

---

## 12. Database and Recovery Testing

### 12.1 Database Tests

Verify:
- Foreign key enforcement.
- Unique constraints.
- Monetary constraints.
- Valid transaction relationships.
- Allocation reconciliation.
- Atomic multi-record operations.
- Migration behaviour.
- Query correctness.

### 12.2 Migration Tests

Verify that migrations:
- Apply in the expected order.
- Preserve existing financial records.
- Handle interrupted execution safely.
- Reject incompatible database states appropriately.
- Can be tested against representative earlier schema versions.

### 12.3 Backup Tests

Verify:
- Backup creation.
- Backup integrity.
- Inclusion of required attachments.
- Backup generation consistency.
- Recovery from a known-good backup.

### 12.4 Recovery Tests

Simulate:
- Interrupted import.
- Application closure during processing.
- Database write failure.
- Failed migration.
- Missing attachment.
- Invalid backup.
- Interrupted restoration.

Expected behaviour must prioritize preservation of existing financial information.

---

## 13. UI/UX Testing

Testing should cover both visual correctness and interaction quality.

Areas include:

- Light and dark themes.
- System theme behaviour.
- Navigation.
- Forms and validation.
- Transaction tables.
- Import workspace tabs.
- Progress and status indicators.
- Charts and financial summaries.
- Notifications.
- Error messages.
- Empty and loading states.
- Keyboard navigation.
- Focus visibility.
- Colour contrast.
- Desktop window resizing.

Visual correctness must include verifying that the numbers shown by the interface match the underlying financial calculations.

---

## 14. Test Data and Fixtures

Testing must use controlled datasets.

### 14.1 Financial Fixtures

Prepare reusable test data for:
- Multiple accounts.
- Opening balances.
- Income.
- Expenses.
- Transfers.
- Fees.
- Refunds.
- Split-category transactions.
- Voided records.
- Reversals.
- Historical transactions.

### 14.2 Import Fixtures

Prepare representative files for:
- Standard text-based PDF statements.
- Scanned PDF statements.
- CSV exports.
- XLSX workbooks.
- Workbooks with formulas.
- Workbooks with summary rows.
- Duplicate statements.
- Revised statements.
- Statements with reconciliation discrepancies.
- Statements without sufficient reconciliation data.

Test fixtures must be synthetic or appropriately sanitized.

Real financial statements must not be committed to the public repository.

### 14.3 Budget and Savings Fixtures

Prepare datasets for:
- Reusable budget templates.
- Multiple independent budget periods.
- Planned and actual expenses.
- Intended savings.
- Actual savings contributions.
- Unallocated surplus.
- Multiple savings goals.

### 14.4 Reproducibility

Each test dataset should have documented expected outcomes.

Test data should be resettable so that automated and manual tests can be repeated consistently.

---

## 15. Test Automation Strategy

### 15.1 General Approach

Automate deterministic and frequently repeated tests wherever practical.

Prioritize:
- Financial calculations.
- Domain rules.
- Database operations.
- Import validation.
- Duplicate detection.
- Reconciliation.
- Budget calculations.
- Regression tests.

Manual testing remains important for:
- Usability.
- Visual evaluation.
- Exploratory testing.
- Realistic import review workflows.
- Acceptance testing.

### 15.2 Proposed Testing Tools

The following are candidate tools based on the provisional application stack.

| Area | Candidate tool | Purpose |
|---|---|---|
| Rust unit tests | Rust test framework | Domain and backend logic |
| Rust integration tests | Rust test framework | Database and service integration |
| Frontend unit tests | Vitest | TypeScript and React logic |
| UI component tests | React Testing Library | Component behaviour |
| Desktop end-to-end tests | Playwright with suitable Tauri integration | Complete application workflows |
| Database verification | SQLite tooling and test fixtures | Schema and integrity checks |
| Static analysis | Rust Clippy and TypeScript tooling | Code quality |
| Formatting | rustfmt and Prettier | Consistent source formatting |

These tools are proposed, not yet selected or installed.

Final choices must be evaluated against the actual project structure and framework versions.

### 15.3 Continuous Integration

A CI pipeline should eventually run relevant automated checks when code is pushed or a pull request is opened.

Candidate checks:
1. Dependency installation.
2. Formatting verification.
3. Static analysis.
4. Frontend tests.
5. Rust tests.
6. Database integration tests.
7. Build verification.
8. Relevant regression tests.

CI must not report success when mandatory checks have failed or been skipped without explanation.

---

## 16. Defect Management

### 16.1 Defect Classification

Defects should be classified by severity.

| Severity | Description |
|---|---|
| Critical | Financial corruption, data loss, serious security issue or unusable application |
| High | Major financial calculation or core workflow failure |
| Medium | Significant feature malfunction with a workaround |
| Low | Minor functional or visual issue |

### 16.2 Priority

Severity and priority must remain distinct.

Severity describes the impact of the defect.

Priority describes how urgently the defect should be addressed.

A lower-severity issue may still receive high priority depending on its context.

### 16.3 Defect Record

Each defect should record:

- Defect ID.
- Title.
- Description.
- Environment.
- Reproduction steps.
- Expected result.
- Actual result.
- Severity.
- Priority.
- Evidence or logs where appropriate.
- Status.
- Resolution.
- Regression test reference.

### 16.4 Financial Defects

Any defect that produces an incorrect balance, duplicates a financial transaction, loses financial information or corrupts an import must receive serious attention.

A release must not proceed with unresolved critical financial integrity defects.

---

## 17. Requirements Traceability

Important requirements must map to one or more test cases.

The traceability record should include:

- Requirement ID.
- Requirement description.
- Related test case IDs.
- Test level.
- Execution status.
- Defect references where applicable.

This ensures that accepted requirements are not forgotten during implementation.

A separate traceability document will be maintained as implementation progresses.

---

## 18. Testing Environments

The project should distinguish:

**Development environment**

Used for implementation and frequent developer testing.

**Test environment**

Used for controlled testing with synthetic datasets and representative configurations.

**Release candidate environment**

Used to verify the application build intended for distribution.

The environments should avoid accidental use of real personal financial information in automated tests.

---

## 19. Entry and Exit Criteria

### 19.1 Test Entry Criteria

Testing may begin when:
- The relevant feature is implemented.
- Required dependencies and test fixtures are available.
- The expected behaviour is documented.
- The test environment is usable.
- The feature is sufficiently stable to evaluate.

### 19.2 Test Exit Criteria

A test cycle may be considered complete when:
- Planned tests have been executed or explicitly marked as blocked or deferred.
- Results are recorded.
- Critical and high-impact defects are addressed or formally accepted as release blockers.
- Relevant regression tests pass.
- Requirements traceability is updated.
- Remaining risks are documented.

### 19.3 Release Acceptance

A release candidate must not be accepted with unresolved critical financial integrity or data-loss defects.

Other outstanding defects must be evaluated, documented and accepted according to their severity and release impact.

---

## 20. Testing Workflow

The proposed testing workflow is:

1. Identify the requirement or change.
2. Define expected behaviour.
3. Identify relevant risks.
4. Create or update test cases.
5. Prepare test data.
6. Implement automated tests where appropriate.
7. Execute automated and manual tests.
8. Record results.
9. Investigate failures.
10. Fix defects.
11. Run regression tests.
12. Update traceability and quality records.

Testing should occur throughout implementation rather than being postponed until the end.

---

## 21. Test Documentation and Records

The following testing artifacts are proposed:

| Artifact | Purpose |
|---|---|
| `test-cases.md` | Detailed test scenarios and steps |
| `requirements-traceability.md` | Requirement-to-test mapping |
| `test-execution-log.md` | Recorded test runs and outcomes |
| `defect-register.md` | Known defects and resolution status |
| `test-data-and-fixtures.md` | Test dataset documentation |
| `release-test-report.md` | Final release testing summary |

These documents should be maintained under `docs/testing/` as the project grows.

Automated test code and fixtures should remain alongside the relevant source code in the repository.

---

## 22. Open QA Decisions

| ID | Topic | Status |
|---|---|---|
| QA-001 | Final automated testing framework selection | Pending technology evaluation |
| QA-002 | Desktop end-to-end testing approach | Pending implementation evaluation |
| QA-003 | Performance benchmarks and targets | Pending representative benchmarks |
| QA-004 | Test coverage targets | Pending implementation planning |
| QA-005 | Test dataset structure | Pending implementation planning |
| QA-006 | CI pipeline configuration | Pending repository and tool setup |
| QA-007 | Defect tracking method | Pending workflow selection |
| QA-008 | Accessibility testing tools | Pending evaluation |
| QA-009 | Release acceptance thresholds | Defined conceptually; detailed thresholds pending |
| QA-010 | Regression suite organization | Pending implementation planning |

---

## 23. Current QA Status

As of 4 November 2026:

| Area | Status |
|---|---|
| QA strategy | Defined |
| Testing levels | Defined |
| Financial test scenarios | Defined |
| Import test scenarios | Defined |
| Budget and savings scenarios | Defined |
| Subscription scenarios | Defined |
| Forecast scenarios | Defined |
| Offline and privacy scenarios | Defined |
| Database and recovery scenarios | Defined |
| Proposed testing tools | Identified, not finalized |
| Automated test suite | Not implemented |
| Test fixtures | Not created |
| CI pipeline | Not implemented |
| Test execution records | Not available |
| Acceptance testing | Not performed |

---

**End of Document — DOC-006**
