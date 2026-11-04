# ETracker — Project Overview

**Document ID:** DOC-001  
**Version:** 0.2  
**Status:** Revised Draft  
**Last Updated:** 4 November 2026

---

## 1. Introduction

ETracker is a personal-first, local-first financial management application intended to help individuals understand, organize, monitor and manage their finances.

The initial release will be a desktop application, targeting Windows first. Its core financial functionality must operate fully offline, with financial records stored locally and controlled by the user.

Although the initial implementation focuses on desktop use, the architecture should accommodate future expansion to other desktop platforms, particularly Linux, and eventual cloud synchronization across multiple devices.

ETracker is intended to be more than an expense tracker. It will provide an integrated environment for managing accounts, recording financial activity, importing statements, planning budgets, tracking savings, monitoring recurring obligations, analysing financial patterns and evaluating financial forecasts.

## 2. Problem Statement

Individuals may receive, transfer and spend substantial amounts of money across different financial accounts without maintaining a clear understanding of their overall financial position.

Financial information is frequently fragmented across:
- Mobile money accounts.
- Bank accounts.
- Cash holdings.
- Financial statements.
- Spreadsheets and personal records.
- Manually maintained budgets and savings plans.

Relying on fragmented records makes it difficult to establish accurate balances, identify spending patterns, understand financial commitments, evaluate savings progress and make informed financial decisions.

Existing financial applications may also depend on internet connectivity, external financial integrations or hosted services that introduce privacy, availability and data-ownership concerns.

ETracker aims to address these problems through a consolidated, locally controlled financial management environment.

## 3. Project Vision

To develop a dependable, comprehensive and extensible personal financial management application that gives users a clear understanding of their financial position and supports informed financial decisions.

ETracker should provide reliable financial records, meaningful analysis and practical planning tools while maintaining local ownership, offline availability and financial data integrity.

## 4. Project Objectives

The primary objectives are to:

1. Provide a consolidated view of financial accounts and overall financial position.
2. Support accurate recording and management of income, expenses, transfers, refunds and other financial events.
3. Simplify the acquisition of financial data through statement and spreadsheet imports.
4. Maintain accurate account balances and historical financial records.
5. Support reusable budget templates and independent monthly financial plans.
6. Distinguish planned spending, actual spending, intended savings, actual savings and unallocated funds.
7. Track recurring financial obligations and subscription payments.
8. Provide useful financial reports, visualizations and historical comparisons.
9. Generate financial forecasts and evaluate their performance against actual results.
10. Provide configurable reminders and notifications.
11. Protect financial information through local data ownership, controlled access, backup and recovery mechanisms.
12. Establish an architecture that can support future cross-platform availability and optional cloud synchronization.

## 5. Target Users

### 5.1 Initial User Group

The initial target is an individual managing personal finances across multiple accounts and payment channels.

The application will initially be developed and evaluated through personal use.

### 5.2 Future User Group

ETracker may eventually be adapted for other individuals with different financial habits, account structures, currencies and planning requirements.

However, the initial release will not implement a SaaS platform, shared household accounts, organizational finance management or multi-user collaboration.

Future expansion must be evaluated separately rather than assumed to be part of the initial scope.

## 6. Scope

### 6.1 Initial Release Scope

The initial release will include the following functional areas.

#### 6.1.1 Dashboard and Financial Overview

- Consolidated financial position.
- Individual account balances.
- Income and expense summaries.
- Savings information.
- Budget performance.
- Historical comparisons.
- Spending trends and category breakdowns.
- Recent financial activity.
- Import and reconciliation status.
- Clear indications when financial information is incomplete or outdated.

#### 6.1.2 Account Management

- Multiple financial accounts.
- Account types such as mobile money, bank and cash.
- Opening balances with effective dates.
- Individual and consolidated account views.
- Historical account balance calculations.
- Account status and lifecycle management.

#### 6.1.3 Transaction Management

- Manual income and expense recording.
- Internal transfers.
- Refunds and transaction fees.
- Backdated transactions.
- Editing, voiding and reversing financial records through controlled operations.
- Category and subcategory classification.
- Split-category transactions.
- Transaction notes, descriptions and supporting attachments.
- Search, filtering and historical review.
- Preservation of original imported information and source references.

#### 6.1.4 Financial Statement and Spreadsheet Imports

- PDF statement imports, including scanned documents requiring OCR.
- CSV imports.
- Excel workbook imports.
- Format-specific extraction and processing.
- Field identification and mapping.
- Transaction preview and correction.
- Duplicate detection.
- Import staging and review.
- Opening and closing balance reconciliation where sufficient information exists.
- Revised statement comparison and controlled replacement or merging.
- Import history and technical processing logs.

Imported information must not enter the authoritative financial ledger until the entire import satisfies mandatory validation and finalization requirements.

#### 6.1.5 Income Management

- Multiple income sources.
- Regular and irregular income.
- Historical income records.
- Expected future income.
- Income categories.
- Historical versus expected income comparisons.

#### 6.1.6 Budgeting and Financial Planning

- Reusable budget templates.
- Independent monthly budget instances.
- Planned income and expenses.
- Category-based allocations.
- Planned savings.
- Actual spending comparisons.
- Budget variance analysis.
- Unallocated remaining funds.

A monthly budget generated from a template must be independent of other monthly instances. Changes to one period must not silently modify another.

#### 6.1.7 Savings Management

- Savings goals.
- Intended savings allocations.
- Actual savings contributions.
- Savings progress.
- Savings variance and historical comparisons.
- Manual recording of money deliberately set aside, including amounts not transferred between accounts.

Intended savings, actual savings and unallocated surplus must remain distinct concepts.

#### 6.1.8 Subscription Management

- Recurring financial obligations.
- Subscription amounts and billing frequencies.
- Next payment dates.
- Payment account or method.
- Active, paused and cancelled statuses.
- Upcoming payment reminders.
- Projected subscription costs.

Projected payments must not be treated as actual expenses or account deductions.

#### 6.1.9 Analytics and Reporting

- Income and expense trends.
- Category breakdowns.
- Account comparisons.
- Historical financial summaries.
- Recurring expense analysis.
- Unusual spending patterns.
- Budget performance.
- Savings progress.
- Reports and data exports.
- What-if financial simulations.

#### 6.1.10 Financial Forecasting

- Local financial forecasting.
- Predicted income, expenses, balances and savings.
- Forecast assumptions and confidence information.
- Scheduled and on-demand forecast generation.
- Immutable forecast snapshots.
- Evaluation against actual financial activity.
- Optional external AI-assisted analysis with explicit user consent.

Forecasting must not initiate financial transactions or automatically modify authoritative financial records.

#### 6.1.11 Notifications and Reminders

The application will support:
- Subscription reminders.
- Statement import reminders.
- Forecast evaluation reminders.

Reminder schedules and notification preferences must be configurable.

#### 6.1.12 Attachments and Supporting Documents

- Local attachment storage.
- Receipts and supporting financial documents.
- Transaction-to-attachment associations.
- Attachment metadata and file references.
- Coordinated backup and recovery of attachments and database references.

#### 6.1.13 Data Management and Application Settings

- Application preferences.
- Theme selection.
- Currency configuration.
- Notification preferences.
- Import settings.
- Forecast preferences and AI consent.
- Backup settings.
- Data integrity checks.
- Backup and recovery functionality.
- Financial audit history.

### 6.2 Out of Scope for the Initial Release

The following are excluded from the initial implementation:

- Live M-Pesa API integration.
- Live bank API integration.
- Automatic synchronization with financial institutions.
- Cloud-hosted financial database.
- Cross-device cloud synchronization.
- SaaS subscriptions and hosted multi-user services.
- Shared household or organizational accounts.
- Automatic execution of payments or transfers.
- Autonomous financial actions initiated by AI.

These capabilities may be evaluated in future project phases.

### 6.3 Future Architectural Direction

Although the initial application is desktop-first and local-first, its design should avoid unnecessary restrictions that would prevent future development.

Future capabilities may include:

- Linux desktop support.
- Additional desktop platforms.
- Secure cloud synchronization between devices.
- Device identity and authentication.
- Change tracking and conflict resolution.
- Secure synchronization of locally created and modified records.
- Optional online services.

Future synchronization must not depend on blindly copying entire SQLite database files between devices. It will require a deliberate synchronization architecture.

Cloud functionality must not become a prerequisite for the core offline financial management capabilities.

## 7. Financial Data Sources

ETracker will support multiple sources of financial information.

### 7.1 Imported Information

Financial statements and spreadsheets are the primary sources for bulk historical and ongoing transaction acquisition.

### 7.2 Manual Information

Users must be able to enter:
- Transactions missing from statements.
- Cash spending.
- Backdated transactions.
- Additional income.
- Corrections and other legitimate financial events.
- Actual savings contributions.

Imported and manually recorded information must coexist without creating duplicate financial activity.

### 7.3 Initial Currency

Kenyan Shillings (KES) will be the initial currency.

The financial data model should avoid unnecessary assumptions that permanently restrict the application to one currency. Any future multi-currency support will require explicit exchange-rate and conversion rules.

## 8. Core Financial Principles

Financial correctness is central to ETracker.

The application must follow these principles:

1. Account balances must reflect opening balances and valid account entries.
2. Internal transfers must affect individual account balances without inflating consolidated income or expenses.
3. Transfer fees must be treated as expenses.
4. Refunds must remain distinguishable from ordinary income.
5. Planned amounts must never be confused with actual financial activity.
6. Projected payments must never be confused with completed payments.
7. Intended savings must never automatically become actual savings.
8. Actual savings must not double-count money already represented in account balances.
9. Historical financial records must remain traceable.
10. Imported source information must be preserved.
11. Duplicate imports must not double-count financial activity.
12. Financial operations requiring multiple changes must be atomic.
13. Monetary calculations must use precise decimal or integer minor-unit representations rather than binary floating-point arithmetic.
14. Effective transaction dates must be distinguishable from record creation and import timestamps.
15. Missing or incomplete financial information must be clearly disclosed rather than silently interpreted as complete.

## 9. Offline and Online Operation

### 9.1 Offline Operation

The core application must operate without an internet connection.

Offline functionality includes:
- Account management.
- Manual transaction entry.
- Financial calculations.
- Budgeting and planning.
- Savings management.
- Subscription management.
- Local analytics.
- Local forecasting.
- Local data storage.
- Backup and recovery.

### 9.2 Optional Online Functionality

Internet connectivity may be used for explicitly optional services, such as external AI-assisted analysis and future cloud synchronization.

External services must not silently receive financial information.

The user must provide explicit consent before financial data is shared with an external AI service.

## 10. Privacy and Security Principles

ETracker will prioritize user ownership and protection of financial information.

The application should:

- Store authoritative financial information locally in the initial release.
- Avoid unnecessary transmission of financial records.
- Protect locally stored data.
- Minimize sensitive information in application logs.
- Preserve audit history for financially significant operations.
- Provide dependable backup and recovery mechanisms.
- Evaluate database and attachment encryption.
- Require explicit consent for external AI processing.
- Keep external integrations isolated from core financial processing.

Detailed security mechanisms will be finalized during technical design.

## 11. Proposed Technology Direction

The current architectural direction is a local-first modular desktop application.

The provisional technology stack is:

| Component | Proposed technology |
|---|---|
| Desktop framework | Tauri 2 |
| Frontend | React |
| Frontend language | TypeScript |
| Backend | Rust |
| Local database | SQLite |
| Styling | Tailwind CSS |
| Database management | Rust-managed database layer |

This stack remains provisional pending dependency, library, platform compatibility and implementation suitability verification.

Technology decisions should prioritize:
- Financial correctness.
- Offline reliability.
- Maintainability.
- Security.
- Cross-platform potential.
- Performance.
- Suitable ecosystem support.
- Long-term extensibility.

Specialist libraries for PDF extraction, OCR and spreadsheet processing will be evaluated against representative source documents before selection.

## 12. Development Approach

ETracker will follow a structured software engineering lifecycle.

The development approach includes:

1. Requirements engineering.
2. System architecture and technical design.
3. Database design.
4. UI/UX design.
5. Development environment and repository setup.
6. Incremental implementation.
7. Testing and quality assurance.
8. Deployment and release.
9. Maintenance and continuous improvement.

Requirements, architecture and design documents will be maintained throughout the lifecycle.

Implementation must proceed incrementally, with meaningful testing and version-control checkpoints.

## 13. Engineering and Project Governance

The project will maintain formal documentation and controlled decision-making.

The primary documentation set includes:

- DOC-001: Project Overview.
- DOC-002: Software Requirements Specification.
- DOC-003: System Architecture.
- DOC-004: Database Design.
- DOC-005: UI/UX Specification.
- DOC-006: Testing and Quality Assurance.
- DOC-007: Deployment and Maintenance.

A separate decision log will document approved architectural and design decisions.

Requirements and design changes must be assessed for their impact on:
- Existing requirements.
- Architecture.
- Database structure.
- UI/UX.
- Testing.
- Security.
- Deployment and maintenance.

Documents must distinguish proposed, provisional, approved and implemented work.

## 14. Success Criteria

ETracker will be considered successful when the initial release can:

1. Maintain accurate records across multiple financial accounts.
2. Calculate individual and consolidated balances correctly.
3. Record and manage income, expenses, transfers, refunds and fees.
4. Import supported financial statements without silently duplicating or losing records.
5. Reconcile statement balances where sufficient information is available.
6. Support manual and imported financial records together.
7. Maintain independent monthly budgets and compare them against actual activity.
8. Distinguish intended savings from actual savings and unallocated surplus.
9. Track subscriptions without confusing projected and actual payments.
10. Provide meaningful historical financial analysis.
11. Generate explainable local forecasts and evaluate them against actual results.
12. Operate its core financial functions fully offline.
13. Protect financial information through dependable local storage, auditability, backup and recovery.
14. Provide a clear, usable desktop experience.
15. Maintain an architecture that does not unnecessarily prevent future Linux support and cloud synchronization.

## 15. Assumptions and Constraints

The current project is based on the following assumptions:

- The first release targets Windows desktop.
- Kenyan Shillings are the initial currency.
- M-Pesa is an important initial financial data source.
- Bank statements are an additional financial data source.
- Cash transactions are less frequent but must be supported.
- Users may import overlapping or revised statements.
- Financial statements may vary in structure and quality.
- Scanned documents may require OCR.
- Users must be able to correct uncertain extracted information.
- Core financial management must remain available offline.
- Optional AI services require explicit user consent.
- Future cloud synchronization is a direction, not an initial release requirement.

## 16. Current Project Status

As of 4 November 2026:

| Area | Status |
|---|---|
| Initial requirements engineering | Completed |
| SRS | Accepted working baseline v0.1 |
| System architecture | Draft v0.1; technology stack provisional |
| Database design | Draft v0.1; reviewed decisions approved for incorporation |
| UI/UX specification | Revised draft v0.2 |
| Testing and QA | Pre-implementation draft v0.1 |
| Deployment and maintenance | Pre-implementation draft v0.1 |
| Repository foundation | Pending |
| Implementation | Not started |
| Testing execution | Not started |
| Deployment | Not started |

The project is currently in its design and documentation refinement phase.

---

**End of Document — DOC-001**