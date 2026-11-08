# ETracker 0.1.0 — Windows x64 preview

This first usable preview provides local accounts, transaction history and audit trails, statement import and review, monthly finance planning, analytics, forecasts, theme preferences, and encrypted backup and restore.

## Installers

- `ETracker_0.1.0_x64_en-US.msi` — Windows Installer package.
- `ETracker_0.1.0_x64-setup.exe` — NSIS setup package.

Both packages are unsigned. Keep encrypted backups outside the application data folder. See [`docs/first-use-guide.md`](docs/first-use-guide.md) before using real financial data.

## Locally built artifact checksums

SHA-256 (verified on 8 November 2026):

- MSI: `5763F52D59A59980716FEDBFA205B8C1FF91E119799051998F6AC05AC88FD455`
- NSIS: `CB19FC909BABB161ED378F97067B2FAF6C76423664722854B51C4F88B2E63B30`

Build artifacts are in `src-tauri/target/release/bundle/` and are intentionally excluded from source control.

## Known limits

Provider-specific statement parsing, scanned-PDF OCR, database-at-rest encryption, signed installers, automated updates and verified install/upgrade behavior are not included. Windows support beyond the tested Windows x64 build environment and native backup file dialogs still need verification. Current release verification details are tracked in [`docs/development/known-issues.md`](docs/development/known-issues.md).
