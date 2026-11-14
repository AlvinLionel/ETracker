# ETracker 0.1.0 — First-use guide

ETracker is a Windows x64 desktop finance workspace. Accounts, statements and plans stay on this computer; core workflows do not need an online account or service.

## Start with your history

1. Create or import an account in **Accounts**.
2. Open **Statement imports** and choose the account and statement file.
3. CSV files support comma, semicolon, tab and pipe delimiters. Excel files support `.xls` and `.xlsx`. PDF text is extracted locally, including password-protected files after you enter a password. Image-only pages use the bundled English OCR model and require careful review.
4. Optionally add automatic category rules using category names and phrases found in descriptions. Check the detected date and money-in/money-out fields, then choose **Import** once to post all eligible transactions. Income/expense direction comes from separate direction columns or signed amounts. An unsigned generic amount has no reliable direction and stays excluded. Invalid rows, unclear-direction rows and possible duplicates remain unposted; you can correct rows if needed. Categories are optional, and statement balance checks are not required.
5. In **Suggested categories**, review recurring phrases found in at least three uncategorized transactions from finalized statement imports. Income and expense suggestions are kept separate. Approving a suggestion creates or reuses its category and rule, then categorizes matching existing imports; future imports use that rule automatically.
6. Use **Budgeting** to build an editable monthly plan. Use Analytics to inspect income, gross expenses, refunds and net spending.

## Keep your data recoverable

Create an encrypted backup from **Settings**, choose a save location separate from this computer, and keep the file and passphrase separately. Use a unique passphrase of at least 12 characters; ETracker cannot recover a forgotten passphrase. Restoring a backup replaces the current database, so export a fresh backup first if the current data may be needed.

To erase the local profile, use **Settings → Reset system** and confirm the warning. This permanently removes saved records, settings, imported statements, and attached documents from this device. It does not delete encrypted backup files saved elsewhere; make a backup first if you may want to recover your data.

On Windows, ETracker stores its database under the current user's roaming application-data directory for `com.etracker.app`. Keep this folder intact when updating or uninstalling if you want to preserve the local profile.

The working database is encrypted with SQLCipher. Its random encryption key is protected for the current Windows user with DPAPI. Attachments are encrypted separately. Keep the Windows account and profile available; if the protected key is lost, restore from an encrypted backup. Opening an attachment in another application creates a temporary decrypted copy, which ETracker removes at next startup or when you use the cleanup control in Settings.

Savings withdrawals from an active goal are recorded as expenses from its linked account and reduce the goal's saved balance. Voiding or reversing the expense restores the amount to the goal balance.

## Current limits

- Statement mappings are generic; institution-specific bank and mobile-money parsers have not been verified against real statement samples.
- Generic PDF text and OCR output are staged as uncertain lines and need correction before they can be imported. The supported M-PESA table layout identifies clear completed transactions for the one-step import; ambiguous rows and possible duplicates remain unposted. OCR currently supports English printed text; inspect its results and correct errors before importing.
- The database key is tied to the current Windows user; an encrypted backup is required to recover data if that key or Windows profile is lost. Temporary copies opened in external applications are not guaranteed to be physically erased from SSDs or filesystem snapshots.
- Installers are currently unsigned. Windows may show an unknown-publisher prompt.
- Only Windows x64 packages are produced. The supported OS floor, installation/upgrade behavior, native backup file prompts, offline desktop behavior and accessibility review have not all been manually verified.
- Budget and forecast entries created in this version use KES; legacy records in other currencies are kept separate without conversion.
