# ETracker

**A local-first personal financial management application.**

ETracker is a Windows desktop application designed to help individuals understand, manage and plan their finances. It aims to bring financial accounts, transactions, budgeting, savings, subscriptions and financial insights together in one application.

Built with a local-first approach, ETracker is designed to keep core financial functionality available offline while prioritizing financial accuracy, data privacy and user control.

## Project Status

ETracker is a local-first personal finance workspace. It supports accounts, transaction entry and audit history, transfers, statement imports with review and reconciliation, editable monthly budget plans, savings and subscriptions, analytics, forecasts, encrypted backup and recovery, and light/dark themes. CSV, Excel and selectable-text PDF statements can be staged locally; password-protected PDFs can be unlocked in the import workspace. Imported rows remain under review until corrected and confirmed. Scanned-PDF OCR and institution-specific statement parsers are not available yet.

## Technology Stack

| Component | Technology |
|---|---|
| Desktop framework | Tauri 2 |
| Frontend | React, TypeScript |
| Backend | Rust |
| Styling | Tailwind CSS |
| Database | SQLite |
| Build tooling | Vite, Cargo |

## Getting Started

### Prerequisites

- Node.js and npm
- Rust and Cargo
- Microsoft C++ Build Tools
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
.\node_modules\.bin\tauri.cmd build --bundles msi,nsis
```

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
