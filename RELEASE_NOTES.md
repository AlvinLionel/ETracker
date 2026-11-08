# ETracker 0.1.0 — Windows x64 preview

This first usable preview provides local accounts, transaction history and audit trails, statement import and review, monthly finance planning, analytics, forecasts, theme preferences, and encrypted backup and restore.

## Installers

- `ETracker_0.1.0_x64_en-US.msi` — Windows Installer package.
- `ETracker_0.1.0_x64-setup.exe` — NSIS setup package.

Both packages are unsigned. Keep encrypted backups outside the application data folder. See [`docs/first-use-guide.md`](docs/first-use-guide.md) before using real financial data.

## Previously built artifact checksums

These checksums identify the OCR milestone installers built on 8 November 2026. They predate transaction attachments and do not represent the current repository source. Rebuild both packages before distributing this source revision.

- MSI (10.96 MiB): `4AD236B0E168585DB0AC19A43DDAB899DDE3F18C951C7A3239585F2CF2A71658`
- NSIS (9.88 MiB): `124E5D57E3C00991707E3EA0DA73F0006F39531D22FBB27F0E918B912C86F366`

Build artifacts are in `src-tauri/target/release/bundle/` and are intentionally excluded from source control.

## Known limits

Provider-specific statement parsing, database-at-rest encryption, signed installers, automated updates and verified install/upgrade behavior are not included. Scanned PDFs use bundled offline English OCR, and every extracted line still requires review. Windows support beyond the tested Windows x64 build environment and native backup file dialogs still need verification. Current release verification details are tracked in [`docs/development/known-issues.md`](docs/development/known-issues.md).
