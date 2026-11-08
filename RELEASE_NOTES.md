# ETracker 0.1.0 — Windows x64 preview

This first usable preview provides local accounts, transaction history and audit trails, statement import and review, monthly finance planning, analytics, forecasts, theme preferences, and encrypted backup and restore.

## Installers

- `ETracker_0.1.0_x64_en-US.msi` — Windows Installer package.
- `ETracker_0.1.0_x64-setup.exe` — NSIS setup package.

Both packages are unsigned. Keep encrypted backups outside the application data folder. See [`docs/first-use-guide.md`](docs/first-use-guide.md) before using real financial data.

## Locally built artifact checksums

SHA-256 (verified on 8 November 2026):

- MSI: `2725F9E4C1A044B0196181409D90EE9D4BB37CBAA6A670323D969792D6456F66`
- NSIS: `677EBC29DA887DD69C10E64C04346123C3284BF447473DA219CF5C4358459E81`

Build artifacts are in `src-tauri/target/release/bundle/` and are intentionally excluded from source control.

## Known limits

Provider-specific statement parsing, scanned-PDF OCR, database-at-rest encryption, signed installers, automated updates and verified install/upgrade behavior are not included. Windows support beyond the tested Windows x64 build environment and native backup file dialogs still need verification. Current release verification details are tracked in [`docs/development/known-issues.md`](docs/development/known-issues.md).
