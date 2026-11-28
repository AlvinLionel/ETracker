# ETracker — Future Enhancements and Ideas

**Document status:** Living backlog

**Purpose:** Capture potential capabilities and improvements identified during development that may be valuable in future versions of ETracker.

**Scope:** This document is for planning and traceability. An entry here does not automatically become an approved feature, a release requirement, or authorization to implement it.

This backlog supplements the project requirements and design documents; it does not replace or redefine them. FE-001 proposes receipt recognition as an extension of the transaction-attachment requirements FR-043 and FR-205–208. Offline English OCR for scanned statement PDFs (FR-058) is available in the preview; transaction attachments and receipt extraction are not. FE-001 remains outside preview scope and creates no delivery commitment. See the [software requirements specification](02-software-requirements-specification.md).

---

## 1. How to use this document

Record ideas here when they emerge during design, implementation, testing, or general product discussions.

The backlog exists to ensure useful ideas are not lost while protecting the current release from uncontrolled scope expansion.

## 2. Future feature candidates

### FE-001 — Smart Receipt Capture

**Status:** Proposed

**Priority:** Long-term candidate; not included in the 0.1.0 preview

**Initial platform:** Desktop

**Future platform:** Mobile

#### Problem or opportunity

Some financial transactions are evidenced by physical or digital receipts but may not be easy to record manually. A receipt can contain useful transaction details, including the merchant, date, total amount, individual products or services, and payment references.

Users should eventually be able to provide a receipt image and have ETracker extract relevant information, suggest a transaction classification, and prepare a transaction for review. This could also help capture purchases that do not appear in an imported bank or M-Pesa statement.

#### Concept

Introduce a receipt-processing workflow that accepts an uploaded image, identifies the document type, extracts relevant information, suggests categories, checks for possible duplicates, and prepares a transaction for user review.

The first implementation should support **desktop file uploads only**. Direct camera capture should be considered when a mobile version of ETracker is developed.

#### Proposed workflow

1. **Upload**
   - The user selects a receipt image from the desktop.
   - Initially support common image formats such as JPEG and PNG.
   - Validate file type, size, and image readability.

2. **Document recognition**
   - Determine whether the image appears to be a receipt or another supported financial document.
   - Do not assume every uploaded image is a valid receipt.
   - Unsupported or ambiguous documents should be reported clearly.

3. **Information extraction**
   - Attempt to extract the merchant or service provider, transaction date, total amount, currency, line items, taxes, payment details, and available transaction references.
   - Retain the original image as supporting evidence, subject to a separately defined receipt-retention policy.
   - Distinguish extracted values from values inferred or suggested by the system.

4. **Product and transaction categorization**
   - Suggest categories using merchant identity, item descriptions, and available context.
   - Potential examples include:
     - Food and groceries
     - Entertainment
     - Bills and utilities
     - Medical or hospital expenses
     - Transport
   - Transport-related extraction may retain available context such as transport mode (plane, train, bus, or vehicle) and identifiers (for example, a flight number or vehicle registration) where relevant and appropriate.
   - Hospital-related purchases may be identified as medical expenses. An emergency classification or flag must not be inferred solely because a merchant is a hospital; the system should distinguish medical classification from emergency context.
   - Suggestions must use the user's available categories and must not silently create new categories or change existing category definitions.

5. **Confidence and review**
   - Indicate when extracted information is uncertain or incomplete.
   - Allow the user to correct the merchant, date, amount, line items, and category suggestions.
   - Require explicit user confirmation before a receipt-derived transaction is posted to the ledger.
   - Never silently post a transaction based solely on OCR output.

6. **Transaction creation**
   - Submit confirmed data through ETracker's existing transaction application and domain rules.
   - Do not create a separate receipt-only financial ledger or bypass existing validation, audit, allocation, and account rules.
   - Retain a relationship between the resulting transaction and its receipt evidence.

7. **Statement reconciliation and duplicate prevention**
   - When a bank or M-Pesa statement is subsequently imported, compare its staged records against existing receipt-derived transactions.
   - Use multiple signals where available, including amount, date/time, merchant, payment reference, and account.
   - A strong match should allow the statement record to be linked to the existing transaction rather than creating a second ledger transaction.
   - An uncertain match should be presented for user review.
   - A non-match should continue through the normal statement-import workflow.
   - Preserve the statement's source record, receipt evidence, and audit history. Matching must not silently overwrite user corrections or source data.

#### Important financial rules

- The receipt's total is the authoritative candidate for the transaction amount, subject to user review.
- Line-item categorization may create split allocations only when the extracted amounts reconcile to the receipt total.
- If line items are missing, ambiguous, or fail reconciliation, request review rather than inventing allocations.
- Tax, discounts, tips, fees, and rounding must be handled explicitly in a future specification.
- A receipt is evidence of a purchase, not necessarily proof that a particular ETracker account paid for it. Account and payment-source information may require user confirmation.
- Receipt processing must respect the existing ledger model, integer minor-unit amounts, audit history, and transaction validation rules.
- Duplicate matching should not rely on amount alone.

#### Potential technical components

- Image ingestion and validation
- OCR engine
- Document-type classifier
- Receipt information extractor
- Merchant normalization
- Category suggestion engine
- Confidence and exception handling
- Duplicate and reconciliation matcher
- Receipt review interface
- Receipt evidence storage and transaction association

A staged approach is preferable. Begin with reliable OCR and structured extraction, then introduce more advanced classification or AI-assisted interpretation if it provides measurable value.

#### Privacy and security considerations

- Prefer local processing where practical, especially for financial documents.
- Do not transmit receipt images or extracted financial data to external AI or OCR services without explicit user consent and a clearly explained processing policy.
- Define supported file limits and protect against malformed or malicious uploads.
- Avoid exposing receipt contents in logs or diagnostics unnecessarily.
- Define encryption, backup, restore, retention, and deletion behaviour for receipt evidence.
- Do not automatically apply the existing 90-day statement-source retention policy to receipts. Receipt retention requires its own product decision.
- Ensure backups and restores preserve receipt-to-transaction relationships and evidence consistently.

#### Possible implementation phases

| Phase | Potential scope |
|---|---|
| 1 — Desktop upload | Image upload, validation, OCR, basic receipt recognition, extraction, and review |
| 2 — Transaction integration | Category suggestions, confirmed transaction creation, and receipt evidence association |
| 3 — Reconciliation | Matching receipt-derived transactions against staged statement records and preventing duplicate ledger entries |
| 4 — Improved intelligence | Better merchant recognition, line-item classification, confidence handling, and learning from user corrections |
| 5 — Mobile support | Camera capture, cropping, perspective correction, and mobile receipt submission |

These phases are indicative only. They do not establish delivery dates or authorize implementation.

#### Open questions for a future specification

- Which image formats, maximum file sizes, and image resolutions should be supported?
- Is the existing offline OCR suitable for receipt layout and field extraction, or is additional technology needed?
- How should managed receipt files be organized, protected and backed up together with their database references?
- What receipt-retention and deletion controls should users have?
- How should the system handle receipts paid partly by cash and partly by another account?
- How should discounts, tips, taxes, deposits, refunds, and multiple currencies be represented?
- Which matching thresholds and review rules should determine strong, possible, and rejected duplicate matches?
- Should line-item classification be optional, with a simpler merchant-and-total workflow as the default?
- What, if anything, should be learned from user corrections, and how should that learning remain local and controllable?

---

### FE-002- Human like dashboard
Remove the 'local mode' icon on the dashboard, it's giving ai made, the dashboard should give an overall mood of humanly/homey/finance/banker/teller/paper. Also remove the 'Local workspace Your financial data stays on this device.' from the bottom of the menu and make it so that the menu is scrollable when it extends beyond the screen height, the dashboard tab should remain fixed on the screen unless it's content also advances beyond the screen, then it becomes scrollable.

Fix the chosen file, no file chosen,account list file display styles on the accounts page, same for the budgeting page and the budget plan files and the following button and input
Fix the layout of the history table in the transactions tab
In the budgetting tab, is the template's feature complete? if so which templates is the user picking from.
Did you implement the bar and line graphs and the pie chart in the analytics page? If not do this.
Add a hover effect over all the buttons and clickable/selectable features.
Fix the choose file input displays all over the platform
Fix the layout of the records/logs section of the imports tab, right now it's tranparent, add a background(make sure it fit's the aesthetic of the selected theme but also that it blends in).
Fix the display of all the buttons/links, right now they appear like normal text, add an idle/hover/click effect for each

### FE-003- Sync platform with mobile and bank feeds

**status**Proposed

**Priority** Long-term candidate

### Problem or opportunity

Sometimes it might become a hustle to request for statements from the money service provider and upload it to the platform. Users might also forget or delay to upload a statement, this misght coz them to not come back to the platform for a while meaning the app becomes pointless.

### concept
Introduce a feature to sync with mpesa and bank transactions to record them in real time

### FE-004- Flag non- statement files

**status:** Implemented (12 November 2026)

**priority** Urgent

### Problem

The current system accepts any type of file and documents it in the system.

### concept

Imports now validate supported file types and container signatures, inspect mapped CSV/Excel data for transaction date and amount rows, and classify extracted PDF text before creating an import workspace. Budget plans, account lists, unrelated documents, and unsupported or malformed files are rejected with a reviewable error. Provider-specific formats still require representative samples.

### FE-005 — Statement archive

**Status:** Implemented (28 November 2026)

**Priority** Medium

### Problem

The user cannot review the past statements that were used in the previous imports.

### Concepts

The platform should have a screen/page where users can view the statements they used in the past imports. The statements will be displayed for the period they are maintained in the system before deletion. The platform should allow opening and review of these statements either internally or manually. The system will display them in a list and show how long a statement has before being deleted permanently

The Statement archive lists retained source statements, their account, workspace date, status, size and 90-day expiry. Users can save the original file and open its exact batch in Imports. Expired source files are cleaned up; import and ledger history remain. Source files are retained in the encrypted local database.

### FE-006 — Daily analysis

**Status:** Implemented (28 November 2026)

**Priority** High

### Problem

The current anayltics page 'Monthly income vs expenditure trajectory' only displays bars for the month day when the data was imported and a staright line for the rest of the days in between.

### Concept

After import and extraction, the anayltics sector should breakdown the transactions into daily transactions for display. For the bar graph every day gets it own bar. To prevent crowding the bars can be compressed on just shown as lines then on hover the graph zooms in on the day the cursor is at and the zoom moves with the cursor.
Then make the trajectory card full width to prevent scrolling and move the donut to allow this(space is not an issue on the screen). we're trying to avoid the unnecessary horizontal scroll.

The Analytics page now provides a full-width daily overview with a separate bar per date, net-flow and 7-day average lines, keyboard and pointer day selection, a two-week detail window, and an accessible exact-value table. Date ranges up to 366 days are supported; longer or invalid ranges show an explanatory empty state. The former monthly trajectory card was removed from this page.

### FE-007 — Minor improvements

**Status:** Partially implemented (28 November 2026)

**Priority** Low

### Completed and remaining

Implemented in this pass:
- Transaction entry is collapsed by default and opens for new entries or editing. Journal filters sit below the ledger heading in a separate, shaded row.
- Transaction row actions now use compact styled controls; amount and status columns have more space.
- Categories can be renamed, have identifying statement phrases edited, and be deleted when no ledger history or plan refers to them. Database constraints protect referenced categories and the UI explains why deletion failed.
- New categories require identifying phrases; rules apply to future imports and remain editable.
- The obsolete monthly trajectory title is removed from Analytics.

Still open:
- Subscription reminders remain in-app; background/operating-system notifications and import reminders need separate scope.
- A full interaction audit and visual review of legacy hover states (including archived accounts and portfolio export) remain.
- Responsive/accessibility review of the revised layouts remains manual.
- The account helper copy mentioned above was not found on the current Accounts screen; confirm before removing similar explanatory copy elsewhere.
- Automated export tests exist, but page-by-page export usability still needs manual review.
### FE-008 — Automatic update detection

**Status:** Deferred; distribution and updater prerequisites are not configured

**Priority** High

### Problem

Does an already installed app detect an update release and install the update?

### Concept

Make it so that after an update is done, release the next version and any already installed app detects and installs the update on the next restart

The preview currently uses manual versioned updates; this was approved in decision `DB-036`. An updater needs a chosen hosting and release channel, signed update artifacts and a stable public signing key. Add this only after the distribution policy is revisited and the update flow can be tested without weakening package verification.
