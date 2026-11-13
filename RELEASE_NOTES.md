# ETracker 0.1.0 — Windows x64 preview

This first usable preview provides local accounts, transaction history and audit trails, statement import and review, monthly finance planning, analytics, forecasts, theme preferences, and encrypted backup and restore.

## Installers

- `ETracker_0.1.0_x64_en-US.msi` — Windows Installer package.
- `ETracker_0.1.0_x64-setup.exe` — NSIS setup package.

Both packages are unsigned. Keep encrypted backups outside the application data folder. See [`docs/first-use-guide.md`](docs/first-use-guide.md) before using real financial data.

## Current artifact status

The source now encrypts the live database and attachments at rest. The last local bundle attempt used the machine's dynamic OpenSSL libraries and produced an executable that depends on `libcrypto-3-x64.dll`, which the bundle does not include. Those packages are not suitable for distribution and are not listed with release checksums. The default vendored OpenSSL build needs Perl with `Locale::Maketext::Simple`; the Windows CI and release workflows install Strawberry Perl for that build. Rebuild and record hashes only after the default Windows workflow succeeds.

## Known limits

Provider-specific statement parsing, signed installers, automated updates and verified install/upgrade behavior are not included. Scanned PDFs use bundled offline English OCR, and every extracted line still requires review. Windows support beyond the tested Windows x64 build environment and native backup file dialogs still need verification. Current release verification details are tracked in [`docs/development/known-issues.md`](docs/development/known-issues.md).
