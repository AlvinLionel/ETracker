# ETracker

**A local-first personal financial management application.**

ETracker is a Windows desktop application designed to help individuals understand, manage and plan their finances. It aims to bring financial accounts, transactions, budgeting, savings, subscriptions and financial insights together in one application.

Built with a local-first approach, ETracker is designed to keep core financial functionality available offline while prioritizing financial accuracy, data privacy and user control.

## Project Status

Foundation initialized

The initial desktop application scaffold has been created and successfully tested.

Core financial features, database integration and transaction processing are still under development.

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
git clone <https://github.com/AlvinLionel/ETracker.git>
cd ETracker
```

Install frontend dependencies:

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

## Project Structure

```text
ETracker/
├── docs/           # Project and engineering documentation
├── public/         # Static frontend assets
├── src/            # React and TypeScript frontend
├── src-tauri/      # Tauri and Rust backend
├── tests/          # Cross-component tests (as development progresses)
├── package.json
├── vite.config.ts
└── README.md
```

The frontend follows a feature-oriented organization, while the Rust backend follows a layered architecture with domain boundaries.

## Documentation

Detailed project documentation is maintained in [`docs/`](docs/)

## Development Philosophy

ETracker is being developed with an emphasis on:

- Financial accuracy and data integrity.
- Local-first operation and offline availability.
- Privacy, security and user control.
- Maintainable architecture and clear separation of concerns.
- Thorough testing and reliable data handling.

## License

No open-source license has been assigned yet.