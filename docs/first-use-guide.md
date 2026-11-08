# ETracker 0.1.0 — First-use guide

ETracker is a Windows x64 desktop finance workspace. Accounts, statements and plans stay on this computer; core workflows do not need an online account or service.

## Start with your history

1. Create or import an account in **Accounts**.
2. Open **Statement imports** and choose the account and statement file.
3. CSV files support comma, semicolon, tab and pipe delimiters. Excel files support `.xls` and `.xlsx`. PDF text is extracted locally, including password-protected files after you enter a password. Image-only pages use the bundled English OCR model and require careful review.
4. Choose the correct date format and field mappings. Review every staged row, correct uncertain rows, resolve possible duplicates, optionally assign categories, and compare the statement balances before finalizing.
5. Use **Budgeting** to build an editable monthly plan. Use Analytics to inspect income, gross expenses, refunds and net spending.

## Keep your data recoverable

Create an encrypted backup from **Settings**, choose a save location separate from this computer, and keep the file and passphrase separately. Use a unique passphrase of at least 12 characters; ETracker cannot recover a forgotten passphrase. Restoring a backup replaces the current database, so export a fresh backup first if the current data may be needed.

On Windows, ETracker stores its database under the current user's roaming application-data directory for `com.etracker.app`. Keep this folder intact when updating or uninstalling if you want to preserve the local profile.

## Current limits

- Statement mappings are generic; institution-specific bank and mobile-money parsers have not been verified against real statement samples.
- PDF text, including OCR output, is staged as uncertain lines. OCR currently supports English printed text; inspect every result and correct errors before finalizing.
- ETracker does not encrypt its working database at rest. Use Windows device encryption and protect the Windows account.
- Installers are currently unsigned. Windows may show an unknown-publisher prompt.
- Only Windows x64 packages are produced. The supported OS floor, installation/upgrade behavior, native backup file prompts, offline desktop behavior and accessibility review have not all been manually verified.
- Budget and forecast entries created in this version use KES; legacy records in other currencies are kept separate without conversion.
