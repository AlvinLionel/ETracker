# ETracker — Software Requirements Specification

**Document ID:** DOC-002  
**Version:** 0.2  
**Status:** Revised Working Draft  
**Last Updated:** 4 November 2026  

---

## 1. Introduction

### 1.1 Purpose

This document defines the functional and non-functional requirements of ETracker, a personal-first, local-first financial management application.

It establishes the expected system behaviour, constraints, business rules, workflows and acceptance conditions that will guide architecture, implementation, testing and future maintenance.

The SRS serves as the primary reference for determining what the application must do and how its behaviour should be validated.

### 1.2 Scope

ETracker is initially a Windows desktop application designed to manage personal financial information across multiple accounts.

The application must support:
- Financial account management.
- Income and expense tracking.
- Transfers and refunds.
- Financial statement and spreadsheet imports.
- Transaction classification and categorization.
- Budgeting and financial planning.
- Savings tracking.
- Subscription management.
- Financial analytics and reporting.
- Forecasting and forecast evaluation.
- Notifications and reminders.
- Local document attachments.
- Data backup and recovery.

Core financial functionality must operate offline.

Future Linux support and cloud synchronization are architectural directions, not requirements for the initial release.

### 1.3 Intended Audience

This document is intended for:
- Project stakeholders.
- Developers.
- Database designers.
- UI/UX designers.
- Software testers.
- Future maintainers.

### 1.4 Definitions

| Term | Definition |
|---|---|
| Account | A financial container such as M-Pesa, a bank account or cash holdings |
| Financial transaction | A recorded financial event |
| Account entry | A signed movement affecting an account balance |
| Category allocation | Classification of an account entry into one or more financial categories |
| Opening balance | The initial financial position of an account at a defined effective date |
| Imported record | A transaction extracted from an external statement or file |
| Staging | Temporary persistent storage used to review and validate an import before ledger finalization |
| Reconciliation | Comparison of calculated financial activity against authoritative statement balances |
| Duplicate candidate | An imported record that may represent an existing financial transaction |
| Budget template | A reusable starting point for creating a monthly budget |
| Budget instance | An independent budget associated with a particular financial period |
| Intended savings | Money planned to be saved |
| Actual savings | Money genuinely set aside and recorded as saved |
| Unallocated surplus | Remaining funds not assigned to another budget purpose |
| Forecast | A projection of possible future financial outcomes |
| Forecast snapshot | A preserved forecast with its original inputs and assumptions |
| Subscription | A recurring financial obligation |
| Atomic operation | An operation that either completes fully or has no partial effect on the authoritative data |

---

## 2. General System Description

### 2.1 Product Perspective

ETracker is a standalone local-first desktop application.

The initial architecture is a modular application with:
- A desktop presentation layer.
- Application and domain logic.
- Financial processing services.
- A local relational database.
- Local document and attachment storage.
- Optional external integrations.

The provisional technology direction is Tauri 2, React, TypeScript, Rust and SQLite. Technology selection remains subject to technical verification.

### 2.2 User Characteristics

The initial application is intended for an individual managing personal finances.

The user may:
- Maintain multiple financial accounts.
- Import statements.
- Enter transactions manually.
- Review and correct extracted information.
- Create budgets and savings goals.
- Analyse financial activity.
- Configure reminders and forecasts.

The application should provide understandable financial information without requiring users to understand database or accounting implementation details.

### 2.3 Operating Environment

Initial target:
- Windows desktop.
- Local SQLite database.
- Local attachment storage.
- Internet connection optional for core functionality.

The architecture should avoid unnecessary operating-system dependencies and consider future Linux compatibility.

### 2.4 System Constraints

- Core financial functionality must work offline.
- Initial currency is KES.
- No live M-Pesa or bank API integration in the initial release.
- Financial calculations must use precise monetary representations.
- Imported records must undergo validation before entering the authoritative ledger.
- Financial history must remain auditable.
- Optional AI services require explicit consent.
- Cloud synchronization is excluded from the initial release.

---

# 3. Functional Requirements

Priority definitions:

- **P0:** Essential for the initial release.
- **P1:** Important functionality that may follow core implementation.
- **P2:** Lower-priority functionality or enhancement.

The priorities below are working classifications and may be refined through implementation planning.

## 3.1 Dashboard and Financial Overview

| ID | Requirement | Priority |
|---|---|---|
| FR-001 | The system shall provide a consolidated financial dashboard. | P0 |
| FR-002 | The system shall display individual account balances. | P0 |
| FR-003 | The system shall calculate and display consolidated financial balances. | P0 |
| FR-004 | The system shall summarize income and expenses for a selected period. | P0 |
| FR-005 | The system shall display savings information and progress. | P0 |
| FR-006 | The system shall display budget performance and variance. | P0 |
| FR-007 | The system shall provide historical financial comparisons. | P1 |
| FR-008 | The system shall visualize spending trends and category breakdowns. | P1 |
| FR-009 | The system shall display recent financial activity. | P0 |
| FR-010 | The system shall display import and reconciliation status. | P0 |
| FR-011 | The system shall allow financial summaries to be filtered by period and relevant account or category. | P1 |
| FR-012 | The system shall disclose when displayed financial information is incomplete, outdated or unreconciled. | P0 |

## 3.2 Account Management

| ID | Requirement | Priority |
|---|---|---|
| FR-013 | The system shall support multiple financial accounts. | P0 |
| FR-014 | The system shall support account types including mobile money, bank and cash. | P0 |
| FR-015 | The system shall allow users to create and manage accounts. | P0 |
| FR-016 | The system shall allow an opening balance to be assigned to an account with an effective date. | P0 |
| FR-017 | Opening balances shall be represented as special ledger transactions rather than ordinary income. | P0 |
| FR-018 | Each account shall have one controlled opening-balance transaction. | P0 |
| FR-019 | The system shall calculate account balances from the opening position and valid account entries. | P0 |
| FR-020 | The system shall support individual and consolidated account views. | P0 |
| FR-021 | The system shall calculate historical account balances for a selected date. | P1 |
| FR-022 | The system shall support account archiving without destroying associated financial history. | P1 |
| FR-023 | The system shall prevent transactions from being silently detached from archived or otherwise inactive accounts. | P0 |

## 3.3 Transaction Management

| ID | Requirement | Priority |
|---|---|---|
| FR-024 | The system shall support manual income recording. | P0 |
| FR-025 | The system shall support manual expense recording. | P0 |
| FR-026 | The system shall support internal transfers between accounts. | P0 |
| FR-027 | The system shall support refund records. | P0 |
| FR-028 | The system shall support transaction fees as expenses. | P0 |
| FR-029 | The system shall support backdated financial transactions. | P0 |
| FR-030 | The system shall allow authorized editing of financial transactions. | P0 |
| FR-031 | The system shall support controlled transaction voiding. | P0 |
| FR-032 | The system shall support reversal transactions linked to their original transactions. | P0 |
| FR-033 | The system shall maintain an audit trail of financially significant changes. | P0 |
| FR-034 | The system shall distinguish transaction effective dates from record creation timestamps. | P0 |
| FR-035 | The system shall distinguish imported records from manually created records. | P0 |
| FR-036 | The system shall preserve original imported descriptions and references. | P0 |
| FR-037 | The system shall support transaction descriptions and notes. | P0 |
| FR-038 | The system shall support categories and subcategories. | P0 |
| FR-039 | The system shall support multiple category allocations for one account entry. | P0 |
| FR-040 | Category allocation totals shall reconcile to the amount being classified. | P0 |
| FR-041 | The system shall support searching and filtering transactions. | P0 |
| FR-042 | The system shall support filtering by date, account, amount, category, type and source where applicable. | P1 |
| FR-043 | The system shall support optional receipt and document attachments. | P1 |
| FR-044 | The system shall preserve financial history when transactions are voided or reversed. | P0 |
| FR-045 | The system shall prevent voided transactions from affecting account balances. | P0 |

## 3.4 Ledger and Financial Processing

| ID | Requirement | Priority |
|---|---|---|
| FR-046 | The system shall represent each financial event using a transaction header and one or more account entries. | P0 |
| FR-047 | Each account entry shall identify the account affected and the signed amount of its movement. | P0 |
| FR-048 | Internal transfers shall create corresponding movements in the affected accounts. | P0 |
| FR-049 | Internal transfer principal shall not be counted as consolidated income or expense. | P0 |
| FR-050 | Transfer fees shall be represented separately as expenses. | P0 |
| FR-051 | Refunds shall remain distinguishable from ordinary income. | P0 |
| FR-052 | Opening-balance transactions shall not be included in ordinary income and expense totals. | P0 |
| FR-053 | Category allocations shall be separate from account balance movements. | P0 |
| FR-054 | The system shall prevent invalid or unbalanced multi-account financial operations. | P0 |
| FR-055 | Financial corrections involving multiple related records shall be atomic. | P0 |
| FR-056 | Historical balance calculations shall exclude voided transactions. | P0 |

## 3.5 Statement and File Imports

### 3.5.1 Import Sources

| ID | Requirement | Priority |
|---|---|---|
| FR-057 | The system shall support PDF statement imports. | P0 |
| FR-058 | The system shall support scanned PDF documents through OCR processing. | P1 |
| FR-059 | The system shall support CSV imports. | P0 |
| FR-060 | The system shall support XLSX workbook imports. | P0 |
| FR-061 | The system shall support XLS workbook imports subject to verified library compatibility. | P1 |
| FR-062 | The system shall support format-specific extraction and processing adapters. | P0 |
| FR-063 | The system shall preserve original source files and relevant source information. | P0 |

### 3.5.2 Extraction and Interpretation

| ID | Requirement | Priority |
|---|---|---|
| FR-064 | The system shall extract transaction dates where available. | P0 |
| FR-065 | The system shall extract descriptions and transaction references where available. | P0 |
| FR-066 | The system shall extract transaction amounts and identify their direction. | P0 |
| FR-067 | The system shall identify transaction types where sufficient evidence exists. | P0 |
| FR-068 | The system shall identify account information where available. | P0 |
| FR-069 | The system shall identify transaction times where available. | P1 |
| FR-070 | The system shall distinguish transaction records from statement summaries and calculated totals. | P0 |
| FR-071 | The system shall support user correction of uncertain extracted values. | P0 |
| FR-072 | The system shall identify extraction failures and incomplete records. | P0 |

### 3.5.3 Excel and Spreadsheet Interpretation

| ID | Requirement | Priority |
|---|---|---|
| FR-073 | The system shall inspect workbook worksheets and their relevant data ranges. | P0 |
| FR-074 | The system shall inspect worksheet headers, columns, tables and cell types. | P0 |
| FR-075 | The system shall interpret cell formatting where relevant to transaction extraction. | P1 |
| FR-076 | The system shall inspect formulas and available calculated results. | P0 |
| FR-077 | The system shall distinguish transaction rows from calculated summaries and totals. | P0 |
| FR-078 | The system shall avoid importing summary totals as individual financial transactions. | P0 |
| FR-079 | The system shall identify unavailable, unsupported or potentially stale formula results. | P1 |
| FR-080 | The system shall suggest editable mappings between source fields and financial transaction fields. | P0 |
| FR-081 | The system shall validate mapped fields before import finalization. | P0 |
| FR-082 | The system shall provide a preview of interpreted financial records. | P0 |
| FR-083 | The system shall preserve relevant source worksheet and row references. | P1 |

### 3.5.4 Import Staging and Validation

| ID | Requirement | Priority |
|---|---|---|
| FR-084 | The system shall create a persistent staging batch for each import. | P0 |
| FR-085 | Imported records shall remain staged until the entire batch is ready for finalization. | P0 |
| FR-086 | Staged records shall not affect authoritative account balances. | P0 |
| FR-087 | The system shall identify successfully extracted, uncertain and invalid records. | P0 |
| FR-088 | The system shall allow users to review and correct uncertain records. | P0 |
| FR-089 | The system shall automatically identify duplicate candidates. | P0 |
| FR-090 | The system shall provide automatic duplicate skipping and manual review options. | P0 |
| FR-091 | The system shall use multiple matching factors for duplicate detection. | P0 |
| FR-092 | The system shall distinguish high-confidence, probable and unmatched duplicate candidates. | P0 |
| FR-093 | The system shall not silently discard ambiguous duplicate candidates. | P0 |
| FR-094 | The system shall allow users to inspect import processing activity and records. | P0 |
| FR-095 | The system shall provide timestamped technical processing logs. | P0 |
| FR-096 | The system shall preserve unfinished import batches when the application closes. | P0 |
| FR-097 | The system shall prevent unfinished staging batches from affecting financial calculations. | P0 |

### 3.5.5 Reconciliation

| ID | Requirement | Priority |
|---|---|---|
| FR-098 | The system shall support opening and closing balance reconciliation where sufficient statement information exists. | P0 |
| FR-099 | The system shall calculate expected statement balances from validated transactions and relevant opening balances. | P0 |
| FR-100 | The system shall distinguish reconciled, discrepancy-detected and reconciliation-unavailable outcomes. | P0 |
| FR-101 | An unresolved authoritative reconciliation discrepancy shall block finalization. | P0 |
| FR-102 | Reconciliation-unavailable status shall not automatically be treated as a failed reconciliation. | P0 |
| FR-103 | The system shall record reconciliation outcomes and relevant limitations. | P0 |
| FR-104 | The system shall not invent financial adjustments to force reconciliation agreement. | P0 |
| FR-105 | The system shall account for relevant statement conventions, fees, pending transactions and transaction ordering. | P1 |

### 3.5.6 Atomic Import Finalization

| ID | Requirement | Priority |
|---|---|---|
| FR-106 | The system shall revalidate the complete staged batch before finalization. | P0 |
| FR-107 | The system shall commit an approved import as one atomic database operation. | P0 |
| FR-108 | Financial records and import completion status shall be committed together. | P0 |
| FR-109 | Failed finalization shall roll back the entire authoritative ledger commit. | P0 |
| FR-110 | The system shall prevent the same batch from being finalized more than once. | P0 |
| FR-111 | The system shall maintain import history after successful finalization. | P0 |

### 3.5.7 Revised Statements

| ID | Requirement | Priority |
|---|---|---|
| FR-112 | The system shall identify when a previously imported statement is re-imported. | P0 |
| FR-113 | The system shall compare revised statements with the earlier imported version. | P0 |
| FR-114 | The system shall identify additions, removals and modifications. | P0 |
| FR-115 | The system shall offer keep-original, replace, merge and individual-review options. | P0 |
| FR-116 | The system shall preserve historical statement versions and import records. | P0 |
| FR-117 | Conflicting financial values shall require user resolution. | P0 |
| FR-118 | Previously applied manual corrections shall not be silently overwritten. | P0 |
| FR-119 | Revised statement changes shall be staged and revalidated before finalization. | P0 |
| FR-120 | Revised imports shall use controlled financial corrections rather than destructive historical replacement. | P0 |

## 3.6 Income Management

| ID | Requirement | Priority |
|---|---|---|
| FR-121 | The system shall support multiple income sources. | P0 |
| FR-122 | The system shall support regular and irregular income. | P0 |
| FR-123 | The system shall maintain historical income records. | P0 |
| FR-124 | The system shall support expected future income information. | P1 |
| FR-125 | Changes to expected income shall not rewrite historical income records. | P0 |
| FR-126 | The system shall support configurable income categories. | P1 |
| FR-127 | The system shall distinguish actual income from expected income. | P0 |

## 3.7 Budgeting and Financial Planning

| ID | Requirement | Priority |
|---|---|---|
| FR-128 | The system shall support reusable budget templates. | P0 |
| FR-129 | The system shall generate independent monthly budget instances from templates. | P0 |
| FR-130 | Changes to a monthly budget shall not modify another monthly budget instance. | P0 |
| FR-131 | Changes to a template shall not automatically modify existing or future budget instances. | P0 |
| FR-132 | The system shall support planned income allocations. | P0 |
| FR-133 | The system shall support planned category-based expenses. | P0 |
| FR-134 | The system shall support planned subscription costs. | P0 |
| FR-135 | The system shall support intended savings allocations. | P0 |
| FR-136 | The system shall calculate planned remaining funds. | P0 |
| FR-137 | The system shall compare planned spending against actual spending. | P0 |
| FR-138 | Actual spending shall be derived from valid financial ledger activity. | P0 |
| FR-139 | Budget changes shall not create or modify actual financial transactions. | P0 |
| FR-140 | Financial transactions shall not silently modify budget allocations. | P0 |
| FR-141 | The system shall calculate category-level and overall budget variance. | P0 |
| FR-142 | The system shall distinguish planned, actual and unallocated amounts. | P0 |
| FR-143 | The system shall support independent budget periods and historical review. | P0 |

## 3.8 Savings Management

| ID | Requirement | Priority |
|---|---|---|
| FR-144 | The system shall support savings goals. | P0 |
| FR-145 | The system shall support intended savings allocations in budgets. | P0 |
| FR-146 | The system shall support actual savings contributions. | P0 |
| FR-147 | The system shall allow actual savings to be recorded without requiring an account transfer. | P0 |
| FR-148 | The system shall recognize qualifying transfers into designated savings accounts where appropriate. | P0 |
| FR-149 | The system shall distinguish intended savings from actual savings. | P0 |
| FR-150 | The system shall distinguish actual savings from unallocated surplus. | P0 |
| FR-151 | The system shall prevent savings contributions from being counted as artificial income. | P0 |
| FR-152 | The system shall prevent double-counting of savings already represented in account balances. | P0 |
| FR-153 | The system shall display savings progress against goals. | P1 |
| FR-154 | The system shall retain historical savings information. | P1 |

## 3.9 Subscription Management

| ID | Requirement | Priority |
|---|---|---|
| FR-155 | The system shall support recurring financial obligations. | P0 |
| FR-156 | The system shall store subscription names and descriptions. | P0 |
| FR-157 | The system shall store subscription amounts and billing frequencies. | P0 |
| FR-158 | The system shall store next payment dates. | P0 |
| FR-159 | The system shall associate subscriptions with relevant payment accounts or methods. | P0 |
| FR-160 | The system shall support active, paused and cancelled subscription statuses. | P0 |
| FR-161 | The system shall include projected subscription costs in planning and forecasts. | P0 |
| FR-162 | Projected subscription payments shall not create actual transactions or reduce account balances. | P0 |
| FR-163 | Actual subscription payments shall be represented by confirmed financial transactions. | P0 |
| FR-164 | The system shall preserve relevant subscription history when terms change. | P1 |
| FR-165 | The system shall support upcoming subscription reminders. | P0 |

## 3.10 Analytics and Reporting

| ID | Requirement | Priority |
|---|---|---|
| FR-166 | The system shall provide income and expense trend analysis. | P1 |
| FR-167 | The system shall provide category-based financial breakdowns. | P1 |
| FR-168 | The system shall support account comparisons. | P1 |
| FR-169 | The system shall support comparisons across historical periods. | P1 |
| FR-170 | The system shall identify recurring expense patterns. | P1 |
| FR-171 | The system shall identify potentially unusual spending patterns. | P1 |
| FR-172 | The system shall provide budget performance reports. | P1 |
| FR-173 | The system shall provide savings progress reports. | P1 |
| FR-174 | The system shall support financial report exports. | P1 |
| FR-175 | The system shall allow relevant reports to be filtered by date, account and category. | P1 |
| FR-176 | The system shall support what-if financial simulations. | P2 |
| FR-177 | What-if simulations shall remain separate from authoritative financial records. | P0 |

## 3.11 Forecasting

| ID | Requirement | Priority |
|---|---|---|
| FR-178 | The system shall support local financial forecasting. | P1 |
| FR-179 | The forecasting engine shall operate without requiring internet connectivity. | P1 |
| FR-180 | The system shall generate forecasts using relevant historical financial activity and current financial information. | P1 |
| FR-181 | The system shall consider relevant recurring obligations and planning information. | P1 |
| FR-182 | The system shall distinguish forecasts from budgets and actual financial activity. | P0 |
| FR-183 | The system shall preserve forecast inputs and assumptions. | P1 |
| FR-184 | Each forecast shall be stored as an independent snapshot. | P1 |
| FR-185 | Generating a new forecast shall not overwrite a previous forecast. | P0 |
| FR-186 | The system shall support scheduled forecast generation. | P1 |
| FR-187 | The system shall support on-demand forecast generation. | P1 |
| FR-188 | Forecasts shall distinguish predicted income, expenses, balances and savings. | P1 |
| FR-189 | The system shall support evaluation of forecasts against actual financial activity. | P1 |
| FR-190 | The system shall record forecast evaluation results and relevant limitations. | P1 |
| FR-191 | The system shall communicate relevant forecast assumptions and confidence information. | P1 |
| FR-192 | Forecasting shall not automatically initiate financial transactions or modify authoritative financial records. | P0 |
| FR-193 | Optional external AI-assisted analysis shall require explicit user consent. | P1 |
| FR-194 | The application shall remain useful when external AI services are unavailable or disabled. | P0 |

## 3.12 Notifications and Reminders

| ID | Requirement | Priority |
|---|---|---|
| FR-195 | The system shall support subscription reminders. | P0 |
| FR-196 | The system shall support statement import reminders. | P0 |
| FR-197 | The system shall support forecast evaluation reminders. | P1 |
| FR-198 | The system shall allow notification preferences to be configured. | P0 |
| FR-199 | The system shall support enabling and disabling notification categories. | P0 |
| FR-200 | The system shall maintain appropriate notification history. | P1 |
| FR-201 | The system shall distinguish unread and read notifications. | P1 |
| FR-202 | Import reminders shall be based on the last successfully reconciled statement period where applicable. | P0 |
| FR-203 | The system shall distinguish retrospective alerts from ordinary scheduled reminders. | P1 |
| FR-204 | Unusual spending notifications shall not automatically classify a transaction as fraudulent or unnecessary. | P0 |

## 3.13 Attachments and Supporting Documents

| ID | Requirement | Priority |
|---|---|---|
| FR-205 | The system shall support local attachment files associated with financial transactions. | P1 |
| FR-206 | The system shall store attachment metadata and file references in the database. | P1 |
| FR-207 | The system shall retain attachments when associated transactions are edited or voided. | P0 |
| FR-208 | The system shall report missing or inaccessible attachment files. | P1 |

## 3.14 Settings, Audit and Data Management

| ID | Requirement | Priority |
|---|---|---|
| FR-209 | The system shall provide centralized application settings. | P0 |
| FR-210 | The system shall support system, light and dark theme preferences. | P1 |
| FR-211 | The system shall support configurable reminder and notification preferences. | P0 |
| FR-212 | The system shall maintain an audit trail for financially significant operations. | P0 |

Additional supporting capabilities required by the approved architecture:

- Manual and scheduled backups.
- User-selected backup destinations.
- Multiple backup generations.
- Database and attachment backup consistency.
- Backup integrity verification.
- Controlled restoration.
- Database integrity checks.
- Versioned database migrations.
- Recovery safeguards.

These capabilities must be covered by the detailed database, architecture and deployment specifications and traced into implementation tests.

---

# 4. Business Rules

| ID | Business Rule |
|---|---|
| BR-001 | Account balances must be calculated from the opening balance and valid account entries. |
| BR-002 | Internal transfer principal must not be included in consolidated income or expenses. |
| BR-003 | Transfer fees must be classified as expenses. |
| BR-004 | Refunds must remain distinguishable from ordinary income. |
| BR-005 | Opening balances must not be treated as ordinary income. |
| BR-006 | A financial transaction may contain multiple account entries where required. |
| BR-007 | Category allocations must reconcile to the amount being classified. |
| BR-008 | Voided transactions must not affect account balances or active financial totals. |
| BR-009 | Reversals must be represented as separate financial events linked to their originals. |
| BR-010 | Planned amounts must remain separate from actual financial activity. |
| BR-011 | Projected payments must not be treated as completed transactions. |
| BR-012 | Intended savings must not automatically become actual savings. |
| BR-013 | Actual savings and unallocated surplus must remain distinguishable. |
| BR-014 | Duplicate imported financial activity must not be counted more than once. |
| BR-015 | Imported records must not affect authoritative balances before successful finalization. |
| BR-016 | An unresolved authoritative reconciliation discrepancy blocks import finalization. |
| BR-017 | Reconciliation unavailable is not equivalent to reconciliation failure. |
| BR-018 | Each finalized import must be atomic. |
| BR-019 | Revised statements must not silently destroy previous financial history. |
| BR-020 | Historical account balances must respect transaction effective dates and void status. |
| BR-021 | Financial calculations must use precise monetary representations. |
| BR-022 | A forecast is a projection, not a guaranteed future financial outcome. |
| BR-023 | External AI processing of financial information requires explicit consent. |
| BR-024 | Core financial functionality must not depend on internet connectivity. |
| BR-025 | Account, transaction and category archival must preserve financial history. |
| BR-026 | Database and attachment backups must remain consistent. |

---

# 5. Non-Functional Requirements

## 5.1 Financial Accuracy and Data Integrity

| ID | Requirement |
|---|---|
| NFR-001 | The system shall perform financial calculations using precise decimal or integer minor-unit representations. |
| NFR-002 | The system shall prevent invalid account balance movements. |
| NFR-003 | Financial operations requiring multiple related changes shall be atomic. |
| NFR-004 | The system shall preserve financial transaction history and relevant audit information. |
| NFR-005 | Database constraints shall prevent invalid relationships and references. |
| NFR-006 | Database migrations shall preserve existing valid financial data. |
| NFR-007 | Historical calculations shall apply correct transaction effective dates and statuses. |
| NFR-008 | Import finalization shall prevent partial authoritative ledger commits. |

## 5.2 Offline Availability

| ID | Requirement |
|---|---|
| NFR-009 | Core financial management shall work without internet connectivity. |
| NFR-010 | Local financial records shall remain accessible when optional external services are unavailable. |
| NFR-011 | Local analytics and forecasting shall not require remote processing. |
| NFR-012 | Offline operation shall not silently disable essential financial functionality. |

## 5.3 Performance

| ID | Requirement |
|---|---|
| NFR-013 | Common account and transaction operations should complete without noticeable delay under expected personal-use workloads. |
| NFR-014 | Dashboard calculations should remain responsive as financial history grows. |
| NFR-015 | Large imports should provide visible progress and must not make the application appear unresponsive. |
| NFR-016 | Long-running extraction and processing operations should support progress reporting and recoverable failure handling. |

Exact performance thresholds will be established through representative workload testing.

## 5.4 Security and Privacy

| ID | Requirement |
|---|---|
| NFR-017 | Financial data shall be protected against unauthorized access through appropriate local security mechanisms. |
| NFR-018 | The system shall minimize exposure of sensitive financial information in logs. |
| NFR-019 | External services shall not receive financial information without appropriate user authorization. |
| NFR-020 | Optional AI processing shall require explicit user consent. |
| NFR-021 | Backup and recovery mechanisms shall protect financial data from accidental loss. |
| NFR-022 | Database and attachment encryption shall be evaluated and incorporated according to the finalized security design. |
| NFR-023 | Sensitive credentials and external service configuration shall be handled securely. |

## 5.5 Reliability and Recovery

| ID | Requirement |
|---|---|
| NFR-024 | The application shall recover safely from interrupted operations. |
| NFR-025 | Unfinished imports shall remain recoverable after application closure. |
| NFR-026 | Failed financial database commits shall not leave partially applied ledger changes. |
| NFR-027 | Backup integrity shall be verifiable before restoration. |
| NFR-028 | Restoration shall protect existing data from accidental irreversible replacement. |
| NFR-029 | Database and attachment references shall be checked for consistency. |

## 5.6 Usability and Accessibility

| ID | Requirement |
|---|---|
| NFR-030 | The application shall present financial information clearly and consistently. |
| NFR-031 | Important financial distinctions shall not rely on colour alone. |
| NFR-032 | Errors shall explain the problem and provide actionable guidance where possible. |
| NFR-033 | Import workflows shall clearly communicate processing progress, warnings and unresolved records. |
| NFR-034 | The interface shall support light and dark themes. |
| NFR-035 | The interface shall provide understandable empty, loading, success and error states. |

## 5.7 Maintainability and Extensibility

| ID | Requirement |
|---|---|
| NFR-036 | The application shall use modular and clearly separated responsibilities. |
| NFR-037 | Financial domain logic shall remain separate from presentation concerns. |
| NFR-038 | Platform-specific functionality shall be isolated where practical. |
| NFR-039 | The architecture should accommodate future Linux compatibility. |
| NFR-040 | The architecture should accommodate future secure cloud synchronization without requiring it in the initial release. |
| NFR-041 | The system shall support controlled database schema evolution through migrations. |

---

# 6. Major User Workflows

## 6.1 Account Creation

1. User opens account management.
2. User creates an account.
3. User supplies account name, type and relevant details.
4. User specifies the opening balance and its effective date.
5. The system creates a controlled opening-balance transaction.
6. The account becomes available for financial activity.

The opening balance must not appear as ordinary income.

## 6.2 Manual Transaction Entry

1. User selects transaction entry.
2. User chooses transaction type.
3. User selects the relevant account or accounts.
4. User enters the amount, effective date and description.
5. User selects category allocations where applicable.
6. The system validates the information.
7. The system records the financial event and account entries.
8. Account balances and relevant reports update.

## 6.3 Financial Statement Import

1. User selects a statement or supported file.
2. The system identifies its format.
3. The system creates a persistent staging batch.
4. The system extracts available financial records.
5. The system interprets fields and relevant source information.
6. The system identifies uncertain or invalid records.
7. The system performs duplicate detection.
8. The user reviews duplicates, corrections and unresolved records.
9. The system performs reconciliation where sufficient statement information exists.
10. The system displays import status and processing logs.
11. The system validates the entire batch again.
12. The user confirms finalization.
13. The system commits the batch atomically.
14. The system records import completion and retains its history.

If a mandatory validation fails, the authoritative ledger remains unchanged.

## 6.4 Revised Statement Import

1. User selects a previously imported statement or supplies a revised version.
2. The system identifies the earlier import.
3. The system compares the two versions.
4. The system identifies additions, removals and modifications.
5. The user chooses to keep the original, replace, merge or review differences individually.
6. The system resolves conflicts and protects previous manual corrections.
7. All resulting changes are staged.
8. The system validates and reconciles the resulting batch.
9. The user confirms finalization.
10. The system applies the approved changes atomically while preserving historical records.

## 6.5 Monthly Budget Creation

1. User creates or selects a reusable budget template.
2. User selects a financial period.
3. The system generates an independent budget instance.
4. The user adjusts planned income, expenses and savings.
5. The system calculates remaining planned funds.
6. Actual financial activity is compared against the budget independently.
7. The system displays actual results and variance.

Editing the budget must not create or alter financial transactions.

## 6.6 Savings Tracking

1. User establishes a savings goal or allocation.
2. The system records intended savings separately from actual savings.
3. User records a genuine savings contribution.
4. The contribution may originate from a qualifying account transfer or a manually recorded amount set aside.
5. The system updates savings progress.
6. The system preserves the distinction between actual savings and unallocated surplus.

The system must avoid double-counting contributions already represented in account balances.

## 6.7 Subscription Management

1. User creates a subscription.
2. User enters amount, frequency, next due date and payment information.
3. The system stores the obligation.
4. Upcoming payments contribute to projections and reminders.
5. A payment is recorded as an actual expense only when confirmed.
6. The subscription may be paused, changed or cancelled.
7. Relevant history remains available.

## 6.8 Forecast Generation and Evaluation

1. User requests a forecast or the scheduled generation process runs.
2. The system gathers relevant historical and current financial information.
3. The system applies its local forecasting logic.
4. The system creates an independent forecast snapshot.
5. The system records relevant assumptions and limitations.
6. The user reviews projected financial outcomes.
7. After the forecast period, the system compares predictions with actual results.
8. The system records evaluation information.

Optional external AI analysis requires explicit consent.

---

# 7. Interface Requirements

## 7.1 User Interface

The application shall provide a desktop-oriented interface with clear navigation across the major financial modules.

The interface shall support:
- Dashboard.
- Accounts.
- Transactions.
- Imports.
- Budgeting.
- Savings.
- Subscriptions.
- Analytics.
- Forecasting.
- Notifications.
- Settings.

## 7.2 Import Interface

The import workspace shall provide distinct views for:

- Activity: import progress and processing stages.
- Records: extracted, uncertain, duplicate and invalid records.
- Logs: timestamped processing events and errors.

The interface shall communicate whether a batch is staged, awaiting review, ready for finalization, completed or failed.

## 7.3 Financial Visualization

Financial charts and summaries shall:
- Identify their reporting period.
- Provide clear titles and labels.
- Display relevant values.
- Distinguish planned from actual data.
- Avoid misleading visual representations.
- Allow inspection of underlying records where appropriate.

## 7.4 Notifications

Notifications shall distinguish subscription reminders, statement import reminders and forecast evaluation reminders.

Warnings and unusual activity alerts shall be distinguishable from routine reminders.

---

# 8. Data Requirements

## 8.1 Financial Records

The system shall preserve:
- Transaction identity.
- Transaction type.
- Effective date.
- Creation timestamp.
- Account entries.
- Category allocations.
- Source information.
- Original descriptions and references.
- Record status.
- Relevant audit information.

## 8.2 Import Records

The system shall preserve:
- Original source file.
- Import batch identity.
- Extraction and validation status.
- Source references.
- Duplicate decisions.
- Reconciliation results.
- User corrections.
- Processing history.
- Finalization status.

## 8.3 Budget and Forecast Records

The system shall distinguish:
- Reusable budget templates.
- Independent monthly budget instances.
- Planned allocations.
- Actual financial activity.
- Intended savings.
- Actual savings.
- Unallocated surplus.
- Forecast snapshots.
- Forecast evaluation results.

## 8.4 Attachment Records

Attachment metadata shall be stored in the database while attachment contents remain in application-managed local file storage.

Database and attachment backups must remain coordinated.

---

# 9. Acceptance Criteria

These criteria describe essential outcomes that must be demonstrated during testing.

| ID | Acceptance criterion |
|---|---|
| AC-001 | An account's calculated balance correctly reflects its opening balance and valid account entries. |
| AC-002 | A transfer updates both affected account balances without increasing consolidated income or expenses. |
| AC-003 | A transfer fee is reflected as an expense independently of the transfer principal. |
| AC-004 | A refund remains distinguishable from ordinary income. |
| AC-005 | A transaction can be split across multiple categories without duplicating its account balance movement. |
| AC-006 | A voided transaction no longer affects balances, while its audit history remains available. |
| AC-007 | A backdated transaction affects the correct historical account balance. |
| AC-008 | Duplicate imported records are detected and handled according to the selected duplicate policy. |
| AC-009 | Uncertain imported records remain staged until resolved. |
| AC-010 | An unresolved authoritative reconciliation discrepancy prevents import finalization. |
| AC-011 | An import with unavailable reconciliation can be finalized when other mandatory checks pass, with its limitation recorded. |
| AC-012 | A failed atomic import commit leaves no partial imported financial activity in the authoritative ledger. |
| AC-013 | Revised statement imports preserve previous import history and apply approved changes without silent destructive replacement. |
| AC-014 | A monthly budget instance remains independent of its template and other budget periods. |
| AC-015 | Actual spending is derived from valid financial transactions rather than planned budget amounts. |
| AC-016 | Intended savings, actual savings and unallocated surplus remain separately identifiable. |
| AC-017 | Projected subscription payments do not reduce account balances or create actual expenses. |
| AC-018 | A forecast remains unchanged as a historical snapshot after new financial activity is recorded. |

Additional acceptance coverage shall be derived for backup and restoration, attachment integrity, notifications, security, offline operation, forecasting evaluation and platform compatibility.

---

# 10. Assumptions, Dependencies and Constraints

## 10.1 Assumptions

- The initial release targets Windows desktop.
- KES is the initial currency.
- M-Pesa statements are an important initial import source.
- Bank statements are additional supported sources.
- Cash transactions must be supported.
- Statement formats may vary.
- Some PDF statements may require OCR.
- Users may import overlapping or revised statements.
- Users may manually correct extracted information.
- The application must remain useful without external services.

## 10.2 Dependencies

Implementation may depend on:
- Desktop framework capabilities.
- Database library suitability.
- PDF extraction libraries.
- OCR capabilities.
- Spreadsheet parsing and formula interpretation.
- Local notification mechanisms.
- Backup and recovery mechanisms.
- Optional external AI service availability.

Dependencies must be evaluated before being adopted.

## 10.3 Constraints

- No live financial institution API integration in the initial release.
- No cloud synchronization in the initial release.
- Core financial functionality must work offline.
- Financial data must remain locally controlled.
- External AI use must be optional and consent-based.

---

# 11. Open Decisions

The following items remain subject to detailed technical design or future approval.

| ID | Open decision |
|---|---|
| OD-001 | Exact initial account types and account metadata |
| OD-002 | Whether multi-currency support should be introduced in a later release and how exchange rates will be managed |
| OD-003 | Final category and subcategory management rules |
| OD-004 | Exact duplicate confidence thresholds and automatic handling boundaries |
| OD-005 | Institution-specific normalization rules for imported statements |
| OD-006 | Treatment of pending transactions during reconciliation |
| OD-007 | Detailed conflict resolution where revised statements disagree with manual corrections |
| OD-008 | Staging retention, archival and cancellation policies |
| OD-009 | Exact savings representation and reconciliation rules for manually recorded set-aside funds |
| OD-010 | Detailed forecast models, evaluation metrics and confidence calculation |
| OD-011 | Exact external AI provider, data handling and integration mechanism |
| OD-012 | Attachment file limits and supported formats |
| OD-013 | Final local data encryption and key-management approach |
| OD-014 | Exact backup retention and scheduling defaults |
| OD-015 | Supported Windows versions and future Linux distribution targets |
| OD-016 | Future synchronization architecture, conflict policy and authentication model |
| OD-017 | Final performance thresholds and representative workload sizes |

These open decisions do not all block the current design phase. Their priority depends on the implementation area they affect.

---

# 12. Requirements Governance and Traceability

Each requirement must have a stable identifier.

Requirements must be traceable to:
- Architecture components.
- Database structures.
- UI/UX behaviour.
- Test cases.
- Acceptance criteria.
- Implementation status.

Changes to accepted requirements must follow controlled change management.

A proposed change must be:
1. Identified.
2. Explained and justified.
3. Assessed for impact.
4. Prioritized.
5. Approved or rejected.
6. Reflected in the relevant documentation.
7. Linked to affected tests where appropriate.

The SRS must distinguish approved requirements from proposed changes and future capabilities.

---

# 13. Document Status

This document is a revised working draft based on the accepted v0.1 requirements baseline and subsequent approved architecture and database decisions.

It is not evidence that any functionality has been implemented or tested.

Before treating v0.2 as the replacement accepted baseline, its reconstructed requirement catalogue must be reconciled against the original v0.1 requirement wording and traceability records.

**End of Document — DOC-002**