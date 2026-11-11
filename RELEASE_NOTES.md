# ETracker 0.1.0 — Windows x64 preview

This first usable preview provides local accounts, transaction history and audit trails, statement import and review, monthly finance planning, analytics, forecasts, theme preferences, and encrypted backup and restore.

## Installers

- `ETracker_0.1.0_x64_en-US.msi` — Windows Installer package.
- `ETracker_0.1.0_x64-setup.exe` — NSIS setup package.

Both packages are unsigned. Keep encrypted backups outside the application data folder. See [`docs/first-use-guide.md`](docs/first-use-guide.md) before using real financial data.

## Current artifact checksums

Both unsigned x64 installers were rebuilt on 12 November 2026 from the current source, including transaction attachments.

- MSI (11.58 MiB): `1E669E548DF2DF7D7C74AD48B6F960C5EEF01931D1185E75148815F41D6DEFEC`
- NSIS (10.50 MiB): `3BACC487A0AA22FFA49F46E03786EDD50E5BD9C31739CC686F74F51E5C65C074`

Build artifacts are in `src-tauri/target/release/bundle/` and are intentionally excluded from source control.

## Known limits

Provider-specific statement parsing, database-at-rest encryption, signed installers, automated updates and verified install/upgrade behavior are not included. Scanned PDFs use bundled offline English OCR, and every extracted line still requires review. Windows support beyond the tested Windows x64 build environment and native backup file dialogs still need verification. Current release verification details are tracked in [`docs/development/known-issues.md`](docs/development/known-issues.md).
