# ETracker

**A local-first personal financial management application.**

ETracker is a Windows desktop application designed to help individuals understand, manage and plan their finances. It aims to bring financial accounts, transactions, budgeting, savings, subscriptions and financial insights together in one application.

Built with a local-first approach, ETracker is designed to keep core financial functionality available offline while prioritizing financial accuracy, data privacy and user control.

## Project Status

ETracker is a local-first personal finance workspace. It supports accounts, transaction entry and audit history, receipt/document attachments, transfers, statement imports with review and reconciliation, editable monthly budget plans, savings and subscriptions, analytics, forecasts, encrypted backup and recovery, and light/dark themes. CSV, Excel and PDF statements can be staged locally; password-protected PDFs can be unlocked in the import workspace, and image-only pages use bundled offline English OCR. Imported PDF rows remain uncertain until reviewed. Institution-specific statement parsers are not available yet.

Planning includes custom subscription intervals (1–365 days, weeks, months or years) and emergency expense categories. Transfers are created from the transaction form, with transfer-only account and fee fields. Forecasts can be saved as budget templates or exported as CSV. Savings allocation suggestions are previews only: they do not move money or create ledger entries. The month selector sets the month for a new budget; it is not a global date filter.

Savings withdrawals are recorded as expense transactions against a goal's linked account and reduce the goal balance. Voiding or reversing a withdrawal restores the tracked goal balance. The local database is encrypted with SQLCipher using a random key protected by the current Windows user's DPAPI; attachment files use AES-GCM. Encrypted, passphrase-protected backups are the recovery path if the Windows profile or local key is lost. Opening an attachment in an external app creates a temporary decrypted copy, removed at the next app start or through Settings. Physical erasure of old file-system blocks cannot be guaranteed.

The app also keeps a local, allowlisted diagnostics event history for up to 30 days. A support report can include recent event labels and timestamps, but not financial records or free-form event details; ETracker does not send the report.

## Technology Stack

| Component | Technology |
|---|---|
| Desktop framework | Tauri 2 |
| Frontend | React, TypeScript |
| Backend | Rust |
| Styling | Tailwind CSS |
| Database | SQLite with SQLCipher encryption at rest |
| Build tooling | Vite, Cargo |

## Getting Started

### Prerequisites

- Node.js and npm
- Rust and Cargo
- Microsoft C++ Build Tools
- Strawberry Perl (for the vendored SQLCipher/OpenSSL build)
- Microsoft Edge WebView2 Runtime

### Installation

Clone the repository:

```bash
git clone https://github.com/AlvinLionel/ETracker.git
cd ETracker
```

Install frontend dependencies (including the locally bundled PDF reader):

```bash
npm install
```

Start the desktop application in development mode:

```bash
npm run tauri dev
```

Build the frontend:

```bash
npm run build
```

Run the Rust and frontend test suites:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:run
```

The Windows desktop end-to-end workflow uses Edge WebDriver. Install the matching driver with `npm run setup:e2e`, then run `npm run test:e2e`.

### Build Windows installers

On a Windows build machine with the Rust and Node prerequisites above, run:

```powershell
winget install --id StrawberryPerl.StrawberryPerl --exact
```

Open a new PowerShell window and confirm Strawberry Perl and the required module are available:

```powershell
perl -MLocale::Maketext::Simple -e "print 'Perl build dependency ready'"
```

Then build the installers:

```powershell
.\node_modules\.bin\tauri.cmd build --bundles msi,nsis
```

The release build uses vendored OpenSSL for SQLCipher and requires Strawberry Perl, including the `Locale::Maketext::Simple` module. Perl bundled with Git for Windows or MSYS2 is not a substitute. If `winget` requires elevation, approve the installer prompt or use the configured Windows CI/release workflow, which installs Strawberry Perl automatically. Do not set `OPENSSL_NO_VENDOR` for distributable builds.

The x64 MSI and NSIS installers are written under `src-tauri/target/release/bundle/`. They are currently unsigned. Read the [first-use guide](docs/first-use-guide.md) before importing personal financial data or restoring a backup.

## Project Structure

```text
ETracker/
├── docs/           # Project and engineering documentation
├── public/         # Static frontend assets
├── src/            # React and TypeScript frontend
├── src-tauri/      # Tauri and Rust backend
├── e2e/            # Windows desktop end-to-end workflow
├── package.json
├── vite.config.ts
└── README.md
```

The frontend follows a feature-oriented organization, while the Rust backend follows a layered architecture with domain boundaries.

## Documentation

Detailed project documentation is maintained in [`docs/`](docs/). Start with the [first-use guide](docs/first-use-guide.md) for the current desktop product.

Proposed ideas that are outside the preview scope are tracked in [Future Enhancements](docs/09-future-enhancements.md).

The current preview build details and package checksums are in [`RELEASE_NOTES.md`](RELEASE_NOTES.md).

## Development Philosophy

ETracker is being developed with an emphasis on:

- Financial accuracy and data integrity.
- Local-first operation and offline availability.
- Privacy, security and user control.
- Maintainable architecture and clear separation of concerns.
- Thorough testing and reliable data handling.

## License

No open-source license has been assigned yet.
