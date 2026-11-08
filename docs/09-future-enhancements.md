# ETracker — Future Enhancements and Ideas

**Document status:** Living backlog

**Purpose:** Capture potential capabilities and improvements identified during development that may be valuable in future versions of ETracker.

**Scope:** This document is for planning and traceability. An entry here does not automatically become an approved feature, a release requirement, or authorization to implement it.

This backlog supplements the project requirements and design documents; it does not replace or redefine them. FE-001 proposes a receipt-image workflow related to FR-043 (optional attachments) and extends OCR from FR-058 (scanned statement PDFs) to receipt recognition. English OCR for image-only statement PDF pages is already available in the preview; receipt capture and evidence attachments are not. The receipt proposal remains outside preview scope, creates no delivery commitment, and does not change the [software requirements specification](02-software-requirements-specification.md).

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
- Which OCR engine provides the best balance of offline capability, accuracy, maintenance, and licensing?
- Should receipt evidence be stored inside the application database, in managed application storage, or through another approach?
- What receipt-retention and deletion controls should users have?
- How should the system handle receipts paid partly by cash and partly by another account?
- How should discounts, tips, taxes, deposits, refunds, and multiple currencies be represented?
- Which matching thresholds and review rules should determine strong, possible, and rejected duplicate matches?
- Should line-item classification be optional, with a simpler merchant-and-total workflow as the default?
- What, if anything, should be learned from user corrections, and how should that learning remain local and controllable?

---
