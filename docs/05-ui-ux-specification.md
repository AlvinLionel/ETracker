# ETracker — UI/UX Specification

**Document ID:** DOC-005  
**Version:** 0.3  
**Status:** Revised Draft  
**Last Updated:** 4 November 2026

---

## 1. Introduction

### 1.1 Purpose

This document defines the user interface and user experience requirements for ETracker.

It describes the visual direction, navigation structure, primary screens, user interactions, feedback mechanisms, accessibility considerations and presentation of financial information.

The interface must make financial information understandable and actionable without compromising accuracy or overwhelming the user.

### 1.2 UX Objectives

ETracker should provide an experience that is:

- Clear and easy to navigate.
- Professional and suitable for financial management.
- Visually attractive without unnecessary decoration.
- Consistent across application modules.
- Transparent about financial calculations and data limitations.
- Efficient for frequent financial tasks.
- Suitable for large transaction datasets.
- Usable in both light and dark themes.
- Reliable during offline operation.

### 1.3 Design Philosophy

The interface should resemble a modern financial workspace rather than a generic administration dashboard.

The design should combine:
- The clarity of accounting software.
- The accessibility of a modern desktop application.
- The information density of a well-organized spreadsheet.
- The visual quality of a polished financial analytics tool.

Decorative elements must never compete with important financial information.

---

## 2. Visual Design Direction

### 2.1 Overall Aesthetic

The selected direction is a modern financial workspace with restrained visual detail.

Key characteristics:

- Clean layouts.
- Clear visual hierarchy.
- Structured data presentation.
- Consistent spacing.
- Carefully selected typography.
- Meaningful charts and indicators.
- Subtle background details.
- Consistent financial colour meanings.

Avoid excessive rounded cards, unnecessary gradients, decorative icons and generic template-like layouts.

### 2.2 Light Theme

The light theme should use warm neutral surfaces rather than pure white backgrounds.

Initial proposed palette:

| Purpose | Colour | Hex |
|---|---|---|
| Main background | Warm cream | `#F5F0E6` |
| Primary text | Soft black | `#20221F` |
| Positive / income | Financial green | `#25845A` |
| Accent / planning | Muted purple | `#7654A8` |
| Secondary surface | Warm beige | `#E8E0D2` |

### 2.3 Dark Theme

The dark theme should use charcoal and deep navy surfaces, with restrained purple and green accents.

Initial proposed palette:

| Purpose | Colour | Hex |
|---|---|---|
| Main background | Charcoal | `#17191F` |
| Secondary surface | Deep navy | `#202638` |
| Accent / planning | Soft purple | `#A88AE0` |
| Positive / income | Soft green | `#54B887` |

### 2.4 Theme Behaviour

The application must support:
- Light theme.
- Dark theme.
- System theme preference.

System theme should be the default on first launch.

The user may override the system preference through application settings.

Theme changes must apply consistently across all screens and visual components.

### 2.5 Colour Semantics

Colours must communicate consistent meanings.

| Meaning | Visual treatment |
|---|---|
| Income / positive movement | Green |
| Expense / negative movement | Muted red or coral |
| Savings progress | Green |
| Planning / navigation / highlights | Purple |
| Warnings | Amber |
| General information | Neutral |
| Disabled / unavailable | Muted neutral |

Colour must not be the only way information is communicated. Labels, icons, values and contextual descriptions must also distinguish states.

### 2.6 Typography

Typography should prioritize readability and a professional financial-product appearance.

Requirements:

- Use a clean, modern sans-serif typeface.
- Avoid using monospaced fonts for general interface text.
- Use consistent heading and body-text scales.
- Use tabular numerals where appropriate for financial figures and tables.
- Distinguish primary totals from supporting information.
- Maintain readable text sizes across supported desktop resolutions.

The final typeface and typography scale remain open for visual evaluation.

### 2.7 Background Grid Treatment

A subtle spreadsheet-inspired grid may be used to reinforce the accounting workspace concept.

Possible placements:
- Main dashboard background.
- Empty or introductory states.
- Transitions between major navigation areas.
- Selected analytical views.

The grid should be restrained and low contrast.

It must not reduce text readability, interfere with charts or make the interface visually noisy.

---

## 3. Application Navigation

### 3.1 Main Navigation Structure

The proposed primary navigation is:

1. Dashboard
2. Accounts
3. Transactions
4. Imports
5. Budgeting
6. Savings
7. Subscriptions
8. Analytics
9. Forecasting
10. Notifications
11. Settings

### 3.2 Navigation Behaviour

Navigation should remain consistent across the application.

Requirements:

- Clearly indicate the active section.
- Preserve appropriate navigation context when moving between related screens.
- Avoid unnecessary nested navigation.
- Provide recognizable icons with text labels.
- Support direct access to frequently used financial operations.
- Make account and date context visible where relevant.

The final sidebar or navigation layout remains subject to wireframe evaluation.

### 3.3 Global Actions

Common actions should be accessible without requiring the user to navigate through unrelated sections.

Examples:
- Add transaction.
- Import statement.
- Search transactions.
- View notifications.
- Access settings.

Actions that change financial records must provide appropriate confirmation and feedback.

---

## 4. Dashboard

### 4.1 Purpose

The dashboard provides an overview of the user's financial position and recent activity.

It should help answer:

- How much money do I currently have?
- How much came in?
- How much was spent?
- How much was actually saved?
- How am I performing against my budget?
- What financial activity needs attention?
- Is my available information complete and current?

### 4.2 Dashboard Components

The dashboard should include the following information.

**Financial position**
- Individual account balances.
- Consolidated account position.
- Account-level balance changes.
- Selected period.

**Income and expenses**
- Total income.
- Total expenses.
- Net movement.
- Comparison with previous periods.

**Savings**
- Intended savings.
- Actual savings.
- Unallocated surplus.
- Savings goal progress.

**Budget overview**
- Planned versus actual expenses.
- Remaining category budgets.
- Budget performance indicators.

**Recent activity**
- Latest transactions.
- Recent account movements.
- Recent imports.
- Relevant subscription activity.

**Alerts and reminders**
- Upcoming subscriptions.
- Import reminders.
- Forecast evaluation reminders.
- Important financial discrepancies.

### 4.3 Dashboard Filters

The dashboard should support appropriate filtering by:
- Date range.
- Account.
- Category.
- Financial period.

Filters must update the relevant totals, charts and transaction summaries consistently.

### 4.4 Data Completeness

The dashboard must not imply that its figures are complete when relevant information is missing or unreconciled.

Where appropriate, show:
- Last successfully imported statement period.
- Last reconciliation result.
- Accounts with outdated information.
- Incomplete import warnings.
- Data limitations affecting calculations.

---

## 5. Accounts Screen

### 5.1 Purpose

The accounts screen manages individual financial accounts and presents their balances.

### 5.2 Account List

Each account should display:
- Account name.
- Account type.
- Current calculated balance.
- Currency.
- Last relevant activity.
- Account status.

### 5.3 Account Details

The account detail screen should provide:
- Current balance.
- Historical balance information.
- Account transaction history.
- Income and expense summaries.
- Transfer activity.
- Import and reconciliation history.
- Account settings.

### 5.4 Account Management

The user should be able to:
- Add accounts.
- Edit account details.
- Set an opening balance through the controlled opening-balance workflow.
- Archive accounts.
- View historical balances.

Archived accounts must remain available for historical financial reporting.

---

## 6. Transactions Screen

### 6.1 Purpose

The transactions screen provides access to financial records from both manual entry and statement imports.

### 6.2 Transaction List

The table should support:
- Date.
- Description.
- Account.
- Transaction type.
- Category.
- Amount.
- Source.
- Status.

### 6.3 Filtering and Search

Users should be able to filter and search by:
- Date range.
- Account.
- Category.
- Transaction type.
- Amount range.
- Source.
- Status.
- Description or reference.

### 6.4 Manual Transaction Entry

The transaction form currently supports manual income, expense and account transfer entry. Refunds and other controlled financial events remain unimplemented.

Fields should be presented according to the selected transaction type.

For example, a transfer requires a source account and destination account, whereas an ordinary expense requires an affected account and classification.

### 6.5 Split Categories

The transaction form must support multiple category allocations.

The user should be able to:
- Add allocations.
- Select categories.
- Enter allocation amounts.
- See the allocation total.
- Identify any difference between allocations and the amount being classified.

The application must prevent saving an invalid allocation.

Manual income and expense entry now allows confirmed optional category allocations. Uncategorized entry remains valid. Transfers are recorded separately and optional transfer fees are separate expense transactions.

### 6.6 Transaction Corrections

Editing, voiding and reversing transactions must be clearly distinguished.

The interface should communicate:
- What action is being performed.
- Which original record is affected.
- The financial impact.
- Whether an audit record will be created.

Irreversible or high-impact actions require appropriate confirmation.

### 6.7 Historical Transactions

The transaction screen should support backdated entries and historical filtering.

The user must be able to distinguish:
- Transaction effective date.
- Record creation date.
- Import date where applicable.

---

## 7. Imports Workspace

### 7.1 Purpose

The imports workspace manages statement uploads, extraction, validation, duplicate detection, reconciliation and finalization.

The generic file-mapping flow supports CSV and XLSX workbooks. For XLSX, users choose a worksheet and header row before mapping date, amount and optional description columns. Formula-derived values in mapped date or amount cells require correction in staged review.

Each saved workspace provides access to its original uploaded file for 90 days after the latest workspace activity. The application removes expired source bytes at startup while retaining the financial records and import history.

When creating an import, the user may select a finalized statement from the same account as the prior version. The workspace displays row-by-row comparison and requires an explicit action for every changed or uncertain match: keep the original, use the revised row, or merge compatible descriptive details. Choosing revised values for a manually edited imported transaction is an explicit instruction to reverse the old posted entry and apply the revised record; the comparison flags this case before finalization. Identical rows are retained from the prior version and excluded from new postings.

Selectable PDF text appears as uncertain page-and-line records for correction and review. Scanned PDFs receive an extraction error because local OCR is not implemented.

It is a critical financial workflow and must provide transparency throughout processing.

### 7.2 Import Workflow

The proposed user journey is:

1. Select account.
2. Upload statement or supported file.
3. Inspect file and statement information.
4. Extract transaction records.
5. Review extracted records.
6. Resolve validation errors and uncertain records.
7. Review duplicate candidates.
8. Complete reconciliation where available.
9. Review the complete import summary.
10. Confirm finalization.
11. View the completed import record.

### 7.3 Import Workspace Layout

The workspace should provide three primary tabs.

**Activity**

Displays the current import progress and processing stages.

**Records**

Displays extracted records and their review status.

**Logs**

Displays detailed processing activity, warnings and errors.

### 7.4 Live Activity

Activity should communicate:
- Current processing stage.
- Progress where measurable.
- Number of extracted records.
- Number of valid records.
- Number requiring review.
- Number of duplicate candidates.
- Reconciliation status.
- Finalization status.

Progress indicators must represent actual work rather than arbitrary percentages.

### 7.5 Record Review

The records interface should provide:
- Original extracted values.
- Normalized values.
- Validation status.
- Duplicate assessment.
- Source reference such as page or row.
- User correction controls.

Users must be able to inspect uncertain records before committing the batch.

### 7.6 Duplicate Review

When duplicates are identified, the interface must explain the findings and offer:

- Automatic duplicate detection and skipping.
- Manual review.

The interface should distinguish high-confidence duplicates from probable matches.

The user must not be led to believe that a probable match is a confirmed duplicate.

### 7.7 Reconciliation Display

The interface must clearly communicate one of three outcomes:

**Reconciled**

Required balance checks agree.

**Discrepancy detected**

Finalization is blocked until the discrepancy is resolved.

**Reconciliation unavailable**

The statement does not provide sufficient information for the required balance checks. The user must be informed of this limitation before finalization.

### 7.8 Revised Statement Review

When a statement is re-imported with changes, the interface should show:
- Previously imported records.
- Newly detected records.
- Removed records.
- Modified records.
- Potential conflicts.
- Records requiring manual confirmation.

The user must be able to choose:
- Keep original.
- Replace with revised version.
- Merge compatible information.
- Review differences individually.

The interface must make clear that replacement does not silently erase historical financial records.

### 7.9 Finalization Confirmation

Before finalization, show a summary containing:
- Total extracted records.
- Accepted records.
- Skipped duplicates.
- Remaining unresolved records.
- Reconciliation outcome.
- Any limitations or warnings.

The application must prevent finalization while mandatory review requirements remain unresolved.

### 7.10 Import Recovery

An unfinished import must be recoverable after closing and reopening the application.

The workspace should identify unfinished batches and allow the user to resume or follow the defined cancellation process.

---

## 8. Budgeting Screen

The budgeting workspace supports reusable template editing, planned income/expense/savings lines, independent monthly snapshots, monthly instance editing, category actuals, and planned versus actual income, expenses, savings and unallocated funds.

The savings screen supports goals, target dates, manual contribution history, progress, completion/archive status, and optional account links. When a goal is linked to an account, the user can link a posted incoming transfer; the UI explains the transfer is already reflected in the account balance. Manual contributions never create ledger movements.

### 8.1 Purpose

The budgeting screen supports financial planning and comparison between planned and actual activity.

### 8.2 Budget Overview

The screen should display:
- Selected budget period.
- Planned income.
- Actual income.
- Planned expenses.
- Actual expenses.
- Intended savings.
- Actual savings.
- Unallocated surplus.
- Remaining budget amounts.

### 8.3 Budget Templates

Users should be able to:
- Create a reusable template.
- Edit a template.
- Duplicate a template.
- Create a new monthly budget from a template.

The interface must communicate that a monthly budget is an independent instance.

Editing a template must not silently change existing budget periods.

### 8.4 Monthly Budget Editing

Users should be able to edit the selected month's plan independently.

The screen should support:
- Income planning.
- Expense allocations.
- Subscription projections.
- Intended savings.
- Unallocated funds.

### 8.5 Planned Versus Actual

The interface must clearly distinguish planned values from actual ledger activity.

It should communicate:
- Planned amount.
- Actual amount.
- Difference or variance.
- Remaining amount where applicable.

A planned expense must never appear as an actual transaction merely because it exists in the budget.

---

## 9. Savings Screen

### 9.1 Purpose

The savings screen tracks savings goals, actual contributions and progress.

### 9.2 Savings Overview

Display:
- Total intended savings.
- Total actual savings.
- Unallocated surplus.
- Active savings goals.
- Progress toward targets.

### 9.3 Savings Goals

Each goal should display:
- Goal name.
- Target amount.
- Actual saved amount.
- Remaining amount.
- Target date where available.
- Progress indicator.

### 9.4 Contributions

Users should be able to inspect contributions and their underlying financial references.

The interface must distinguish between:
- A transfer to a savings account.
- A savings designation that does not create another account movement.
- A planned savings allocation.
- An actual contribution.

### 9.5 Savings Accuracy

The interface must not double-count transfers or artificially increase savings totals through duplicate records.

Savings progress must be traceable to recorded contributions.

---

## 10. Subscriptions Screen

### 10.1 Purpose

The subscriptions screen manages recurring financial obligations, term changes, status and upcoming due reminders. A monthly projection is shown separately from actual transactions. Users can link an existing posted expense from the ledger as an actual payment; linking never creates a second transaction, and voided or reversed payments no longer count toward subscription totals.

### 10.2 Subscription List

Display:
- Subscription name.
- Amount.
- Frequency.
- Next due date.
- Payment account where applicable.
- Status.

### 10.3 Subscription Details

Users should be able to:
- Add subscriptions.
- Edit subscription information.
- Pause subscriptions.
- Cancel subscriptions.
- Review relevant payment history.

### 10.4 Projected Versus Actual Payments

The interface must distinguish expected payments from completed financial transactions.

A projected subscription payment must not be counted as an actual expense.

---

## 11. Analytics Screen

### 11.1 Purpose

The analytics screen presents historical financial patterns and comparisons. The implemented views show income and expense totals and monthly trends, category breakdowns, account comparisons, repeated expense descriptions, review-only unusual-spending signals, budget performance, savings progress and CSV export. A temporary what-if simulator changes assumed income/expenses for the selected period without writing financial records. Date, currency, account and category filters apply to report data. Internal transfers and voided or reversed activity are excluded.

### 11.2 Analytical Views

Possible visualizations include:

- Income and expense trends.
- Category breakdowns.
- Spending by account.
- Monthly comparisons.
- Recurring expenses.
- Savings progress.
- Budget performance.
- Historical balance trends.

### 11.3 Chart Requirements

Every chart must provide:
- A meaningful title.
- Relevant date range.
- Clearly identified values.
- Understandable labels.
- Appropriate units and currency.
- Useful contextual information.

Charts should allow users to inspect underlying records where appropriate.

### 11.4 Visualization Selection

Use visualization types according to the question being answered.

| Visualization | Intended use |
|---|---|
| Donut / pie | Composition and category breakdown |
| Line chart | Trends over time |
| Bar chart | Period or category comparisons |
| Area chart | Cumulative or changing values |
| Sparklines | Compact trend indicators |
| Progress indicator | Goal and budget progress |

Avoid charts that add decoration without improving understanding.

---

## 12. Forecasting Screen

### 12.1 Purpose

The forecasting screen presents projected financial outcomes and allows comparison with actual results.

### 12.2 Forecast Presentation

Display:
- Forecast period.
- Predicted amounts.
- Assumptions.
- Confidence or uncertainty indicators where supported.
- Data limitations.
- Generation method.

### 12.3 Forecast History

Previously generated forecasts must remain available for comparison and evaluation.

### 12.4 Forecast Evaluation

The interface should compare predicted and actual outcomes.

It should communicate:
- Predicted result.
- Actual result.
- Variance.
- Relevant explanation.

Historical results, plans, forecasts and actual financial records must remain visually distinct.

### 12.5 Optional AI Assistance

AI-assisted features must clearly indicate when external processing is involved.

The interface must obtain explicit consent before sending financial information to an external service.

Core forecasting functionality must remain available without AI.

---

## 13. Notifications and Reminders

### 13.1 Notification Types

The application supports three primary reminder categories:

1. Subscription reminders.
2. Statement import reminders.
3. Forecast evaluation reminders.

### 13.2 Notification Presentation

Notifications should be visible without unnecessarily interrupting ordinary workflows.

The interface should distinguish:
- Informational notifications.
- Warnings.
- Errors.
- Reminders.
- Retrospective financial insights.

### 13.3 Reminder Preferences

Users should be able to configure relevant reminder preferences.

Statement import reminders should support weekly, fortnightly and monthly cadences.

The application should provide enough context for the user to understand why a reminder was generated.

---

## 14. Settings Screen

### 14.1 Purpose

The settings screen manages application preferences and operational configuration.

### 14.2 Settings Categories

Proposed categories:

- Appearance.
- Financial preferences.
- Notifications.
- Imports.
- Forecasting.
- AI and privacy.
- Backup and recovery.
- Application information.

### 14.3 Appearance Settings

Allow users to select:
- System theme.
- Light theme.
- Dark theme.

### 14.4 Privacy and AI Settings

Clearly explain:
- Which functionality operates locally.
- Which functionality may use an external service.
- What information would be sent.
- How consent can be changed.

### 14.5 Backup and Recovery

Provide access to:
- Backup creation.
- Backup history where supported.
- Backup verification.
- Restoration workflows.

Restoration must include appropriate warnings and safeguards.

---

## 15. Forms, Tables and Interaction Patterns

### 15.1 Forms

Forms should:
- Use clear labels.
- Group related fields.
- Validate inputs.
- Explain errors.
- Preserve entered information when recoverable.
- Avoid unnecessary steps.

### 15.2 Tables

Tables should support:
- Clear column labels.
- Sorting where useful.
- Filtering.
- Search.
- Pagination or suitable large-dataset handling.
- Readable financial alignment.
- Clear status indicators.

### 15.3 Financial Amounts

Amounts should:
- Display currency consistently.
- Use appropriate decimal precision.
- Align consistently in tables.
- Distinguish positive and negative values.
- Avoid ambiguous signs or abbreviations.

### 15.4 Confirmation and Feedback

Financially significant operations should provide suitable feedback.

Examples:
- Transaction saved.
- Transaction voided.
- Import completed.
- Reconciliation discrepancy detected.
- Backup completed.
- Restoration failed.

Feedback must describe the actual outcome, not simply that a button was clicked.

---

## 16. Loading, Empty, Error and Recovery States

### 16.1 Loading States

The interface should communicate when data is loading or an operation is in progress.

Long-running imports should provide visible progress and relevant activity information.

### 16.2 Empty States

Empty states should explain what the user is seeing and provide a relevant next action.

Examples:
- No accounts created.
- No transactions available.
- No budget created.
- No savings goals.
- No statement imports.

### 16.3 Error States

Errors must be understandable and actionable.

Where appropriate, provide:
- What failed.
- Whether data was saved.
- Whether the operation can be retried.
- What the user can do next.

### 16.4 Interrupted Operations

The interface must clearly communicate the status of interrupted operations.

For example, an unfinished import must not appear as a completed import.

---

## 17. Accessibility and Usability

The application should follow established accessibility principles.

Requirements include:
- Sufficient colour contrast.
- Keyboard navigation.
- Visible focus indicators.
- Readable typography.
- Clear control labels.
- Appropriate accessible descriptions.
- Status communication beyond colour alone.
- Consistent interaction patterns.

Charts should provide accessible textual summaries or equivalent data views.

The final implementation must undergo accessibility testing.

---

## 18. Desktop Responsiveness

ETracker is initially designed for desktop use.

The interface should adapt to different desktop window sizes and resolutions.

Requirements:
- Avoid horizontal overflow at supported window sizes.
- Keep essential financial information visible.
- Allow data tables to remain usable.
- Adapt chart layouts appropriately.
- Maintain consistent navigation.
- Avoid assuming a single monitor resolution.

Future Linux support should not require a fundamentally different interface.

---

## 19. Design Consistency

The application should maintain a shared design system.

This should define:
- Colours and semantic meanings.
- Typography.
- Spacing.
- Component sizes.
- Buttons and controls.
- Tables.
- Forms.
- Dialogs.
- Notifications.
- Charts.
- Loading and empty states.

Common components should behave consistently across modules.

---

## 20. UI/UX Acceptance Considerations

The interface will be evaluated against the following expectations:

1. Users can navigate all primary modules clearly.
2. Users can identify their individual and consolidated account positions.
3. Users can distinguish income, expenses, transfers and refunds.
4. Users can create and review transactions.
5. Users can classify transactions across multiple categories.
6. Users can review an import before it affects account balances.
7. Users can identify duplicate candidates and resolve uncertain records.
8. Users can understand reconciliation outcomes.
9. Users can distinguish independent monthly budget instances.
10. Users can distinguish intended savings from actual savings and unallocated surplus.
11. Users can identify projected subscription costs versus actual payments.
12. Users can understand forecasts and their limitations.
13. Users can change themes.
14. Users can identify incomplete, stale or unreconciled financial information.
15. Users can recover from interrupted workflows without losing staged work.

---

## 21. Open UI/UX Decisions

| ID | Topic | Status |
|---|---|---|
| UX-001 | Exact production colour values | Pending contrast testing |
| UX-002 | Final typography and font | Pending evaluation |
| UX-003 | Navigation layout | Pending wireframes |
| UX-004 | Dashboard wireframe | Pending |
| UX-005 | Charting library | Pending technology evaluation |
| UX-006 | Grid density and placement | Pending visual evaluation |
| UX-007 | Detailed transaction table behaviour | Pending interaction design |
| UX-008 | Notification presentation | Pending detailed design |
| UX-009 | Accessibility verification approach | Pending implementation planning |
| UX-010 | Responsive desktop breakpoints | Pending implementation planning |
| UX-011 | Live import activity and technical logs | Approved |

---

## 22. Current UI/UX Status

As of 4 November 2026:

| Area | Status |
|---|---|
| Overall visual direction | Approved |
| Light and dark themes | Approved |
| System theme as default | Approved |
| Initial colour palette | Proposed |
| Spreadsheet-inspired visual treatment | Approved direction |
| Main navigation modules | Defined |
| Dashboard information requirements | Defined |
| Transaction workflow | Defined |
| Import workspace and three tabs | Approved direction |
| Duplicate and reconciliation presentation | Defined |
| Revised statement review | Defined |
| Budget and savings presentation | Defined |
| Subscription presentation | Defined |
| Analytics and forecasting presentation | Defined |
| Final wireframes | Not completed |
| Design system implementation | Not started |
| Usability testing | Not performed |
| Accessibility testing | Not performed |

---

**End of Document — DOC-005**
