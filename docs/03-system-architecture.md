# ETracker — System Architecture

**Document ID:** DOC-003  
**Version:** 0.2  
**Status:** Revised Draft  
**Last Updated:** 4 November 2026

---

## 1. Introduction

### 1.1 Purpose

This document defines the high-level architecture of ETracker.

It describes the system's major components, responsibilities, interactions, data flow, technology direction, architectural constraints and design principles.

It provides the technical foundation for database design, implementation, testing, deployment and future system evolution.

### 1.2 Architectural Objectives

The architecture must:

1. Support dependable personal financial management.
2. Preserve accurate financial records and calculations.
3. Operate fully offline for core functionality.
4. Maintain local ownership of financial information.
5. Support reliable financial statement imports and reconciliation.
6. Separate financial events, account movements and category classifications.
7. Support controlled financial corrections and auditability.
8. Provide maintainable and testable modular components.
9. Support reliable backup and recovery.
10. Avoid unnecessary platform dependencies.
11. Accommodate future Linux compatibility.
12. Leave a clear architectural path toward optional cloud synchronization.

### 1.3 Architectural Style

ETracker will initially use a **local-first modular monolithic architecture**.

The application will be delivered as one desktop application with internally separated modules and responsibilities.

It will not initially use a distributed microservices architecture.

This approach provides clear separation of concerns without introducing the operational complexity of multiple independently deployed services.

---

## 2. System Context

### 2.1 System Overview

ETracker consists of a desktop application that manages local financial data and optionally communicates with external services when explicitly required.

The initial system context is:

```text
+--------------------------------------+
|             ETracker User            |
+-------------------+------------------+
                    |
                    v
+--------------------------------------+
|          ETracker Desktop App        |
|                                      |
|  +--------------------------------+  |
|  |       Presentation Layer       |  |
|  +--------------------------------+  |
|                  |                   |
|  +--------------------------------+  |
|  |       Application Layer        |  |
|  +--------------------------------+  |
|                  |                   |
|  +--------------------------------+  |
|  |          Domain Layer          |  |
|  +--------------------------------+  |
|                  |                   |
|  +--------------------------------+  |
|  |       Infrastructure Layer     |  |
|  +--------------------------------+  |
|                  |                   |
|  +--------------------------------+  |
|  |       Local Data Storage       |  |
|  | SQLite + Managed Files         |  |
|  +--------------------------------+  |
+--------------------------------------+
                    |
          Optional External Services
                    |
          +---------+----------+
          |                    |
          v                    v
    External AI          Future Cloud
    Integration          Synchronization
```

Core financial operations must not depend on optional external services.

### 2.2 External Interactions

The initial system may interact with:

- Local PDF, CSV and Excel files.
- Local attachment files.
- Optional external AI services.
- Operating-system facilities such as notifications and file selection.

Future releases may introduce secure cloud synchronization.

Direct integration with M-Pesa or bank APIs is outside the initial release scope.

---

## 3. High-Level Architectural Structure

The application will be divided into the following logical layers.

### 3.1 Presentation Layer

**Proposed technology:** React and TypeScript.

Responsibilities:

- Display application screens.
- Collect user input.
- Present financial summaries and visualizations.
- Display transactions and account information.
- Provide import review interfaces.
- Present validation errors and processing progress.
- Manage user interaction and navigation.
- Apply the application's visual design system.

The presentation layer must not independently implement authoritative financial calculations.

Financial calculations and validation rules belong in the domain layer.

### 3.2 Application Layer

**Proposed technology:** Rust.

Responsibilities:

- Coordinate application workflows.
- Execute use cases.
- Validate application-level requests.
- Coordinate domain services.
- Manage transaction boundaries.
- Coordinate database operations.
- Orchestrate imports and finalization.
- Manage backup and recovery workflows.
- Coordinate notifications.
- Communicate with optional external integrations.

The application layer connects presentation requests to domain operations and infrastructure.

### 3.3 Domain Layer

**Proposed technology:** Rust.

This layer contains the authoritative business and financial rules.

Responsibilities include:

- Account balance calculations.
- Financial transaction validation.
- Account entry processing.
- Category allocation validation.
- Transfers and refunds.
- Opening balance rules.
- Financial correction and reversal rules.
- Budget calculations.
- Savings calculations.
- Subscription lifecycle rules.
- Forecast interpretation and evaluation.
- Reconciliation rules.
- Financial integrity constraints.

The domain layer should remain independent of the desktop UI and external service availability.

### 3.4 Infrastructure Layer

Responsibilities:

- SQLite persistence.
- File storage.
- PDF extraction.
- OCR integration.
- Spreadsheet parsing.
- CSV processing.
- Database migrations.
- Logging.
- Backup and recovery.
- Local notification integration.
- Optional AI communication.
- Operating-system-specific facilities.

Infrastructure implementations should be accessed through defined interfaces where appropriate.

### 3.5 Persistence Layer

The persistence layer provides local storage for authoritative financial records and supporting application data.

Primary technologies:

- SQLite for structured data.
- Application-managed file storage for attachments and source documents.

The Rust backend will manage authoritative database access.

The frontend must not directly manipulate the SQLite database.

---

## 4. Desktop Application Architecture

### 4.1 Desktop Framework

**Provisional technology: Tauri 2.**

Tauri provides the desktop application shell and communication boundary between the web-based frontend and the native backend.

Expected responsibilities:

- Desktop window management.
- Frontend asset delivery.
- Native command invocation.
- File and operating-system integration.
- Application packaging.
- Desktop lifecycle management.

Tauri selection remains provisional pending compatibility and dependency verification.

### 4.2 Frontend

**Provisional technology: React with TypeScript.**

The frontend will provide the application's desktop user interface.

Its responsibilities include:
- Dashboard.
- Account management.
- Transaction management.
- Import workspace.
- Budgeting.
- Savings.
- Subscriptions.
- Analytics.
- Forecasting.
- Notifications.
- Settings.

The frontend should use typed interfaces when communicating with the backend.

### 4.3 Backend

**Provisional technology: Rust.**

Rust will manage authoritative financial processing and local data operations.

The backend will expose application operations through controlled Tauri commands or an equivalent internal interface.

The frontend must not bypass backend validation when modifying authoritative financial information.

### 4.4 Frontend and Backend Communication

The expected communication flow is:

```text
User Interaction
       |
       v
React Component
       |
       v
Typed Application Request
       |
       v
Tauri Command Boundary
       |
       v
Rust Application Service
       |
       v
Domain Validation and Processing
       |
       v
Persistence / Infrastructure
       |
       v
Structured Result
       |
       v
React Interface Update
```

Commands should have clearly defined inputs, outputs and error responses.

Financial operations must be validated by the backend even when frontend validation is also provided.

---

## 5. Internal Application Modules

ETracker will be organized into logical functional modules.

### 5.1 Account Module

Responsibilities:

- Account creation and management.
- Account types.
- Opening balances.
- Account status.
- Account balance queries.
- Historical balance calculations.

The account module must preserve historical financial records when accounts are archived.

### 5.2 Transaction and Ledger Module

Responsibilities:

- Financial transaction creation.
- Account entry management.
- Category allocations.
- Transfers.
- Income and expenses.
- Refunds and fees.
- Historical transaction handling.
- Editing, voiding and reversals.
- Audit history.

This module is the authoritative source for actual financial activity.

### 5.3 Import Module

Responsibilities:

- Source file registration.
- File format identification.
- Extraction orchestration.
- Field interpretation and mapping.
- Duplicate detection.
- Staging management.
- User review and correction.
- Reconciliation.
- Revised statement comparison.
- Atomic finalization.
- Import history and technical logs.

Import processing must remain separate from ordinary ledger operations until finalization.

### 5.4 Budgeting Module

Responsibilities:

- Reusable budget templates.
- Monthly budget instances.
- Planned income and expense allocations.
- Planned savings.
- Budget variance.
- Actual versus planned comparisons.
- Unallocated funds.

The budgeting module reads actual financial information from the ledger but does not create or alter actual transactions through budget operations.

### 5.5 Savings Module

Responsibilities:

- Savings goals.
- Intended savings.
- Actual savings contributions.
- Savings progress.
- Savings history.
- Savings designation and reconciliation.

The module must distinguish a genuine savings contribution from a planned allocation or unallocated surplus.

Manual savings records must not introduce artificial income or double-count account balances.

### 5.6 Subscription Module

Responsibilities:

- Recurring obligation records.
- Billing frequency.
- Due dates.
- Payment account or method.
- Subscription lifecycle.
- Upcoming payment projections.
- Subscription reminders.

Subscription projections must remain separate from confirmed financial transactions.

### 5.7 Analytics Module

Responsibilities:

- Historical financial analysis.
- Income and expense summaries.
- Category breakdowns.
- Account comparisons.
- Spending trends.
- Budget performance.
- Savings analysis.
- Reports and exports.
- What-if simulations.

Analytics must use authoritative financial data and distinguish actual activity from projections.

### 5.8 Forecasting Module

Responsibilities:

- Local forecasting.
- Forecast snapshots.
- Forecast assumptions.
- Scheduled generation.
- On-demand generation.
- Evaluation against actual results.
- Confidence and limitation reporting.
- Optional external AI-assisted analysis.

The forecasting module must never modify authoritative financial records.

### 5.9 Notification Module

Responsibilities:

- Subscription reminders.
- Import reminders.
- Forecast evaluation reminders.
- Notification preferences.
- Read/unread status.
- Notification history.

Notifications must not directly modify financial information.

### 5.10 Attachment Module

Responsibilities:

- Local attachment storage.
- File metadata.
- Transaction associations.
- File integrity checks.
- Attachment retrieval.
- Coordinated backup and recovery.

### 5.11 Settings and Data Management Module

Responsibilities:

- Application preferences.
- Theme settings.
- Currency configuration.
- Notification configuration.
- Import preferences.
- Forecast preferences.
- AI consent.
- Backup settings.
- Database integrity checks.
- Database migrations.
- Backup and restoration.

---

## 6. Financial Data Architecture

### 6.1 Core Financial Model

The financial architecture separates three distinct concepts:

1. Financial transaction headers.
2. Account entries.
3. Category allocations.

These concepts must not be collapsed into one generic transaction record.

### 6.2 Financial Transaction Header

A transaction header represents a financial event.

It contains shared information such as:
- Transaction identity.
- Transaction type.
- Effective date.
- Description.
- Source.
- Reference.
- Record status.
- Creation timestamp.
- Relevant audit information.

One financial event may affect multiple accounts.

### 6.3 Account Entries

Account entries represent actual movements affecting financial account balances.

Each entry identifies:
- The affected account.
- Signed monetary amount.
- Relevant transaction relationship.
- Effective date or inherited event date where appropriate.
- Entry status and audit information.

A negative amount reduces the account balance. A positive amount increases it.

### 6.4 Category Allocations

Category allocations classify account entries for reporting and analysis.

An entry may be divided across multiple categories.

Example:

A supermarket purchase of KSh 2,000 using M-Pesa:

```text
Financial Transaction
       |
       +---- Account Entry
       |       M-Pesa: -2,000
       |
       +---- Category Allocations
               Food:       1,500
               Household:   500
```

The allocations must reconcile to the amount being classified.

Category allocations do not create additional account balance movements.

### 6.5 Internal Transfers

Example: KSh 1,000 transferred from M-Pesa to a bank account.

```text
Transfer Transaction
       |
       +---- M-Pesa Entry: -1,000
       |
       +---- Bank Entry:  +1,000
```

The consolidated financial position remains unchanged by the transfer principal.

If a KSh 10 fee is charged, it is represented separately:

```text
Transfer Fee Transaction
       |
       +---- M-Pesa Entry: -10
       |
       +---- Expense Classification: Transfer Fees
```

### 6.6 Opening Balances

Opening balances are represented as special ledger transactions.

They:
- Establish an account's initial position.
- Have an effective date.
- Are excluded from ordinary income.
- Must not cause pre-opening activity to be double-counted.

Corrections use controlled adjustments rather than casual rewriting.

### 6.7 Historical Financial Calculations

The architecture must distinguish:
- Effective transaction date.
- Record creation timestamp.
- Import timestamp.
- Correction or reversal relationships.

Historical balances must be calculated using the appropriate effective dates and transaction statuses.

Voided entries must be excluded from active balances.

### 6.8 Financial Corrections

Financial corrections must preserve history.

Supported operations include:
- Controlled editing.
- Voiding.
- Reversal through a separate linked financial event.

The system must preserve original imported information and relevant audit records.

Related changes must be atomic.

---

## 7. Import Processing Architecture

### 7.1 Design Principle

Import processing is a controlled pipeline that transforms external financial documents into validated ledger records.

Extracted information is not authoritative until finalization.

### 7.2 Import Pipeline

```text
Source File
    |
    v
File Registration
    |
    v
Format Identification
    |
    v
Extraction
    |
    v
Field Interpretation and Mapping
    |
    v
Normalization
    |
    v
Duplicate Detection
    |
    v
Staging
    |
    v
User Review and Correction
    |
    v
Statement Reconciliation
    |
    v
Final Validation
    |
    v
Atomic Ledger Commit
    |
    v
Import Completion
```

### 7.3 Format-Specific Processing

The system should use specialized processing adapters for:

- PDF statements.
- Scanned PDF statements requiring OCR.
- CSV files.
- XLSX workbooks.
- XLS workbooks where supported.

Each adapter converts source-specific information into a normalized internal representation.

The original source document and relevant extraction metadata must remain available.

### 7.4 Spreadsheet Interpretation

Spreadsheet processing must inspect more than simple column headers.

It should consider:
- Worksheets.
- Tables and data ranges.
- Headers.
- Columns and cell types.
- Number and date formatting.
- Formulas.
- Available formula results.
- Summary rows and calculated totals.

The system must distinguish transaction records from summary information.

Unavailable or potentially stale formula results must be identified rather than silently treated as reliable values.

### 7.5 Normalization

Normalization transforms source-specific representations into consistent internal values.

Examples include:
- Date normalization.
- Monetary amount normalization.
- Description cleanup.
- Transaction reference normalization.
- Transaction type interpretation.
- Institution-specific formatting.

Original source values must be preserved separately.

### 7.6 Duplicate Detection

Duplicate detection uses multiple factors:

- Account identity.
- Transaction reference.
- Amount and currency.
- Transaction date.
- Transaction time where available.
- Description or merchant.
- Transaction type.

The system must classify matching evidence according to confidence.

High-confidence duplicates may be handled according to the selected automatic policy.

Probable or ambiguous matches must be available for manual review.

Missing transaction time must not automatically disqualify a match.

### 7.7 Staging Architecture

Every import receives a persistent staging batch.

The staging area stores:
- Extracted records.
- Source references.
- Validation results.
- Duplicate candidates.
- User corrections.
- Reconciliation results.
- Processing status.
- Logs.
- Finalization state.

Unfinished imports must survive application closure.

Staged information must not affect authoritative account balances.

### 7.8 Reconciliation Architecture

The reconciliation service compares extracted and validated financial activity against authoritative statement balances where available.

It supports three outcomes:

**Reconciled**

Required balance checks agree. The import may proceed if all other mandatory validations pass.

**Discrepancy detected**

An unresolved discrepancy exists. Finalization is blocked.

**Reconciliation unavailable**

The statement does not contain sufficient information for the required balance checks. Other validations remain mandatory, and the batch may be finalized with the limitation explicitly recorded.

The reconciliation service must not invent adjustments to force agreement.

### 7.9 Revised Statement Processing

A revised statement is compared against the earlier imported version.

The comparison identifies:
- Added records.
- Removed records.
- Modified records.
- Unchanged records.
- Conflicting values.

The user may:
- Keep the original.
- Replace with the revised version.
- Merge compatible records.
- Review differences individually.

Previous import history and source files must remain preserved.

A replacement must not destructively erase historical financial events.

### 7.10 Atomic Finalization

Finalization is the boundary between staging and authoritative financial records.

Before finalization:
1. The entire batch is revalidated.
2. All required records must be resolved.
3. Duplicate handling must be complete.
4. Mandatory reconciliation checks must pass.
5. Reconciliation-unavailable limitations must be recorded.
6. The user must confirm finalization.

The database commit must include:
- Approved financial records.
- Related account entries.
- Category allocations.
- Relevant import completion status.

The operation must be atomic.

If any part fails, the authoritative ledger commit must roll back completely.

---

## 8. Budgeting, Savings and Subscription Architecture

### 8.1 Budget Templates and Instances

Reusable templates act as starting points for monthly budget creation.

Each generated monthly budget is an independent instance.

Changes to one instance must not automatically modify:
- Previous months.
- Other existing months.
- Future monthly instances.

Templates are not live references that continuously rewrite monthly budgets.

### 8.2 Separation of Planned and Actual Data

Budget planning records and financial ledger records are independent.

The budgeting module reads actual activity from the ledger and compares it with planned allocations.

Budget changes must not create or alter actual financial transactions.

Recording actual transactions must not silently modify budget allocations.

### 8.3 Savings Architecture

Savings information is divided into:

**Intended savings**

Money allocated toward savings in a budget.

**Actual savings**

Money genuinely set aside and recorded as saved.

Actual savings may be represented through:
- Qualifying transfers into designated savings accounts.
- Manually recorded amounts deliberately set aside.

**Unallocated surplus**

Remaining money that has not been assigned or recorded as savings.

The architecture must prevent artificial income creation and double-counting.

The detailed database representation of manually recorded savings must be defined during database implementation design.

### 8.4 Subscription Architecture

Subscriptions are recurring obligation records.

They are not financial transactions.

Subscription data contributes to projections, budgets and forecasts.

Actual payments are recorded independently through confirmed imported or manually entered transactions.

Subscription history must remain traceable when terms change.

---

## 9. Forecasting Architecture

### 9.1 Local Forecasting

The local forecasting engine operates without internet connectivity.

It may use:
- Historical transactions.
- Account information.
- Recurring obligations.
- Budget information.
- Relevant savings data.

### 9.2 Forecast Snapshots

Every forecast is stored as an independent snapshot of the information and assumptions used to generate it.

A new forecast must not overwrite an earlier forecast.

### 9.3 Forecast Evaluation

The forecasting module compares predictions against actual financial activity after the relevant period.

Evaluation records should preserve:
- Original forecast.
- Relevant actual outcomes.
- Differences.
- Evaluation period.
- Assumptions and limitations.

### 9.4 Optional AI Integration

External AI services may provide supplementary financial analysis.

This integration must:
- Remain optional.
- Require explicit consent.
- Clearly disclose when information is sent externally.
- Avoid automatic financial actions.
- Remain separate from authoritative financial processing.

The core forecasting capability must not depend on external AI.

---

## 10. Local Data Storage Architecture

### 10.1 SQLite Database

SQLite is the provisional primary database.

It stores structured application information, including:
- Accounts.
- Financial transactions.
- Account entries.
- Category allocations.
- Import batches.
- Imported records.
- Reconciliation information.
- Budget templates and instances.
- Savings records.
- Subscriptions.
- Forecasts and evaluations.
- Notifications.
- Attachment metadata.
- Settings.
- Audit information.

The exact schema is specified in DOC-004.

### 10.2 Attachment Storage

Attachments are stored as local files in application-managed storage.

SQLite stores metadata and references to those files.

Attachments may include:
- Receipts.
- Supporting transaction documents.
- Relevant imported source files.

The application must handle missing or inaccessible files gracefully.

### 10.3 Persistence Responsibilities

The Rust backend manages authoritative database access.

The architecture should:
- Enforce database constraints.
- Use atomic transactions.
- Support versioned migrations.
- Preserve financial history.
- Coordinate database and file operations.
- Support reliable backup and recovery.

### 10.4 Database Access Boundaries

Frontend components must not directly modify database records.

Database operations must be mediated through application services and domain validation.

---

## 11. Security and Privacy Architecture

### 11.1 Local Data Protection

Financial records are sensitive and must be protected.

The application should:
- Restrict access through appropriate operating-system and application mechanisms.
- Avoid unnecessary external data transmission.
- Minimize sensitive information in logs.
- Protect backup files.
- Evaluate database and attachment encryption.

### 11.2 External Integrations

Optional external services must be isolated behind defined interfaces.

External AI integrations must receive information only after explicit user consent.

### 11.3 Auditability

Financially significant operations must produce appropriate audit records.

The architecture must preserve traceability for:
- Transaction creation.
- Transaction modification.
- Voiding.
- Reversals.
- Imports.
- Reconciliation.
- Corrections.

---

## 12. Backup and Recovery Architecture

### 12.1 Backup Scope

Backups must include:
- SQLite database.
- Managed attachment files.
- Relevant application data required to restore a consistent financial state.

### 12.2 Backup Destinations

Support:
- Application-managed backup storage.
- User-selected directories.
- External drives and other suitable destinations.

### 12.3 Backup Operations

Support manual and scheduled backups.

Maintain multiple backup generations rather than continually overwriting the only available copy.

Backups must represent a consistent state of the database and associated files.

### 12.4 Recovery

Restoration must:
1. Validate the selected backup.
2. Protect current data through a recovery copy.
3. Restore the database and attachment files consistently.
4. Validate database integrity.
5. Check attachment references.
6. Report restoration results.

### 12.5 Failure Handling

Failed backups must be reported clearly.

Incomplete or corrupted backups must not be presented as successful recovery points.

---

## 13. Logging and Error Handling

### 13.1 Application Logging

The application should provide structured logs for relevant technical operations.

Logs must:
- Include timestamps.
- Identify processing stages.
- Distinguish informational messages, warnings and errors.
- Provide actionable diagnostic information.
- Avoid unnecessary sensitive financial information.

### 13.2 Import Logs

Import logs should record:
- File registration.
- Extraction.
- Field interpretation.
- Duplicate detection.
- Validation.
- Reconciliation.
- User corrections.
- Finalization.
- Failures and recovery actions.

### 13.3 Error Handling

Errors should be classified according to their impact.

Financial integrity errors must prevent unsafe operations.

The application should not silently continue when authoritative financial data could become inconsistent.

---

## 14. Platform and Future Synchronization Architecture

### 14.1 Initial Platform

Windows desktop is the initial supported target.

### 14.2 Cross-Platform Design

The architecture should avoid unnecessary operating-system-specific assumptions.

Core domain and financial logic must remain platform-independent.

Platform-specific functionality should be isolated behind appropriate infrastructure interfaces.

Linux compatibility should be evaluated during dependency selection and development.

### 14.3 Future Cloud Synchronization

Cloud synchronization is not part of the initial release.

However, the architecture should avoid unnecessary obstacles to its future implementation.

Future synchronization will require:

- Device identity.
- Authentication.
- Secure synchronization interfaces.
- Change tracking.
- Conflict detection and resolution.
- Offline change handling.
- Reconnection and retry mechanisms.
- Privacy and encryption controls.

The future synchronization design must not rely on copying entire SQLite database files between devices.

### 14.4 Local-First Guarantee

The introduction of future cloud services must not make internet connectivity a requirement for basic financial management.

Local records must remain usable offline.

---

## 15. Proposed Technology Stack

The current proposed stack is:

| Layer | Technology | Status |
|---|---|---|
| Desktop framework | Tauri 2 | Provisional |
| Frontend | React | Provisional |
| Frontend language | TypeScript | Provisional |
| Backend | Rust | Provisional |
| Database | SQLite | Provisional |
| Styling | Tailwind CSS | Provisional |
| Database access | Rust-managed database layer | Provisional |
| Attachment storage | Local application-managed files | Approved direction |

### 15.1 Technology Selection Criteria

Technology selection must consider:
- Reliability.
- Security.
- Maintainability.
- Offline capabilities.
- Performance.
- Platform compatibility.
- Dependency maturity.
- Documentation quality.
- Community support.
- Long-term extensibility.

### 15.2 Specialist Libraries

PDF.js handles local text extraction and rendering. Tesseract.js and its bundled English trained data support offline OCR in the frontend worker. The import workflow stages text as uncertain page/line records; provider-specific field normalization remains separate and requires verified samples.

Selection should use representative real-world financial documents.

No specialist library should be adopted solely because it is popular or convenient.

---

## 16. Testing Considerations

The architecture must support testing at multiple levels.

### 16.1 Domain Testing

Test:
- Account balances.
- Transfers.
- Fees.
- Refunds.
- Opening balances.
- Historical calculations.
- Category allocations.
- Savings calculations.
- Budget variance.
- Subscription projections.
- Forecast evaluation.

### 16.2 Application and Integration Testing

Test:
- Frontend/backend communication.
- Database operations.
- Import orchestration.
- Atomic finalization.
- Reconciliation.
- File handling.
- Backup and recovery.
- Notification workflows.

### 16.3 System Testing

Test:
- Complete financial workflows.
- Offline operation.
- Import processing.
- Data integrity.
- Error recovery.
- Performance.
- Security and privacy.
- User experience.

The detailed testing strategy is specified in DOC-006.

---

## 17. Architectural Risks and Mitigations

| Risk | Potential impact | Mitigation |
|---|---|---|
| Incorrect financial calculations | Misleading balances and reports | Centralized domain rules and extensive financial tests |
| Duplicate imports | Inflated financial activity | Multi-factor detection and staged review |
| Partial import commits | Inconsistent ledger | Atomic finalization |
| Incorrect statement extraction | Financial inaccuracies | Preview, validation and user correction |
| Revised statement conflicts | Loss of corrections or duplicated activity | Version comparison and controlled corrections |
| Database corruption | Loss of financial history | Backup, integrity checks and recovery |
| Attachment/database inconsistency | Missing supporting documents | Coordinated backup and recovery |
| Overly complex initial architecture | Delayed implementation | Modular monolith and deferred cloud capabilities |
| Platform-specific dependencies | Difficult Linux support | Isolated platform integrations and dependency verification |
| Future synchronization conflicts | Inconsistent device records | Deliberate future synchronization architecture |
| External AI data exposure | Privacy risks | Explicit consent and integration isolation |

---

## 18. Architecture Decision Governance

Architecture decisions must be recorded in the project decision log.

Decisions must distinguish:
- Proposed.
- Provisional.
- Approved.
- Implemented.
- Verified.

An approved architecture decision does not mean its implementation has been completed.

Material architecture changes must be assessed against:
- Functional requirements.
- Database design.
- Financial correctness.
- Security.
- Testing.
- Deployment.
- Future compatibility.

---

## 19. Current Architecture Status

As of 4 November 2026:

| Area | Status |
|---|---|
| Architectural style | Approved direction: local-first modular monolith |
| Desktop-first approach | Approved |
| Financial ledger conceptual model | Approved |
| Import staging and atomic finalization | Approved |
| Reconciliation model | Approved |
| Budget and savings separation | Approved |
| Subscription separation | Approved |
| Forecast snapshots | Approved |
| Local attachment storage | Approved |
| Backup and recovery direction | Approved |
| Future Linux readiness | Approved direction |
| Future cloud synchronization readiness | Approved direction; implementation deferred |
| Tauri, React, TypeScript, Rust and SQLite | Provisional technology stack |
| Exact database schema | Pending detailed design |
| Specialist extraction libraries | Pending evaluation |
| Implementation | Not started |

---

**End of Document — DOC-003**
