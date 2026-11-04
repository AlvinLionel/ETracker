# ETracker — Development Conventions

**Document:** DOC-008  
**Version:** 0.1  
**Status:** Approved working baseline  
**Project:** ETracker

---

## 1. Purpose

This document establishes the engineering conventions governing ETracker's development.

Its purpose is to promote consistency, maintainability, testability, security and collaboration across the frontend, backend and supporting infrastructure.

These conventions apply throughout the project unless superseded by an approved architectural decision or documented change.

## 2. General Engineering Principles

ETracker development follows these principles:

- Separation of concerns: each component should have a clearly defined responsibility.
- Maintainability: prefer understandable, maintainable solutions over unnecessary complexity.
- Financial integrity: financial calculations and transaction operations must preserve defined business rules.
- Security and privacy: sensitive information must be protected throughout processing, storage and logging.
- Testability: business logic should be independently testable.
- Consistency: follow the established conventions of each technology.
- Incremental development: introduce tools and dependencies when they become necessary.
- Explicit error handling: failures must be identifiable and appropriately handled.

## 4. Frontend Organization

The frontend uses a feature-oriented organization.

```text
src/
├── app/
│   ├── App.tsx
│   ├── routes.tsx
│   └── providers/
├── components/
│   ├── ui/
│   └── layout/
├── features/
│   ├── accounts/
│   ├── transactions/
│   ├── imports/
│   ├── budgeting/
│   ├── savings/
│   ├── subscriptions/
│   ├── analytics/
│   ├── forecasting/
│   └── settings/
├── hooks/
├── lib/
├── styles/
└── types/
```

### 4.1 Directory Responsibilities

| Directory | Responsibility |
|---|---|
| `app/` | Application initialization, routing and global providers |
| `components/ui/` | Reusable interface elements |
| `components/layout/` | Shared navigation and layout components |
| `features/` | Feature-specific functionality |
| `hooks/` | Shared React hooks |
| `lib/` | Shared utilities and frontend infrastructure |
| `styles/` | Global styles and design tokens |
| `types/` | Shared TypeScript contracts |

Feature-specific code should remain within its owning feature whenever possible.

A feature may contain its own components, hooks, services, types and public exports.

Code should only be promoted into shared directories when it genuinely serves multiple features.

### 4.2 Frontend and Backend Communication

Frontend components must not scatter direct Tauri command invocations throughout the interface.

Communication with the Rust backend should be centralized behind a dedicated integration layer.

This keeps UI components independent of backend implementation details and improves testability.

## 5. Backend Organization

The Rust backend follows a layered architecture with domain boundaries.

```text
src-tauri/src/
├── commands/
├── application/
├── domain/
├── infrastructure/
├── persistence/
├── lib.rs
└── main.rs
```

### 5.1 Layer Responsibilities

| Layer | Responsibility |
|---|---|
| `commands/` | Tauri commands exposed to the frontend |
| `application/` | Coordinates application use cases |
| `domain/` | Financial entities, rules and business invariants |
| `infrastructure/` | Technical services and external integrations |
| `persistence/` | Database access and repository implementations |

### 5.2 Dependency Direction

Dependencies should point inward towards the domain.

- The domain must not depend on Tauri, SQLite or frontend technologies.
- The application layer coordinates use cases and domain operations.
- Repository contracts should separate application/domain requirements from persistence implementation.
- Infrastructure and persistence implement technical details without defining financial business rules.
- Tauri commands act as entry points and delegate work rather than containing substantial business logic.

Financial calculations must remain independent of the source of the underlying data.

## 6. Error Handling

ETracker uses structured and typed error handling.

### 6.1 Rust Backend

Operations that may fail should return appropriate `Result` types.

```rust
fn create_transaction() -> Result<Transaction, AppError> {
    // Perform operation
}
```

Application-level errors should distinguish meaningful categories, such as:

- Validation errors
- Not-found errors
- Conflict errors
- Database errors
- Import errors
- Internal errors

The final error model will be established during implementation.

Routine production code must not rely on `unwrap()` or `expect()` as substitutes for proper error handling.

### 6.2 Frontend

The frontend should receive predictable error information and present appropriate user-facing messages.

Raw database errors, SQL statements, stack traces and internal implementation details should not be exposed to users.

### 6.3 Error-Handling Rules

1. Never silently swallow errors.
2. Never report a financial operation as successful without confirmation.
3. Preserve useful diagnostic information when propagating errors.
4. Avoid exposing sensitive information in error messages.
5. Use database transactions and rollback where atomicity is required.
6. Distinguish recoverable failures from those requiring user intervention.
7. Present clear and actionable feedback whenever possible.

## 7. Logging and Operational Visibility

ETracker uses a shared structured logging and operational activity system.

The system should provide meaningful visibility into application operations, particularly long-running tasks.

### 7.1 Severity Levels

| Level | Purpose |
|---|---|
| `TRACE` | Highly detailed diagnostic information |
| `DEBUG` | Development and troubleshooting information |
| `INFO` | Normal operational events |
| `WARN` | Unexpected conditions that do not necessarily stop an operation |
| `ERROR` | Failed operations |
| `FATAL` | Critical failures preventing continued operation |

### 7.2 Structured Log Information

Where applicable, events should include:

- Timestamp
- Severity
- Operation or process identifier
- Processing stage
- Event description
- Relevant batch or record identifier
- Diagnostic information
- Corrective guidance where appropriate

### 7.3 Operational Visibility

The application distinguishes three complementary views:

**Activity**
- Human-readable operational progress.
- Current processing stage.
- Important milestones and outcomes.

**Records**
- Individual items being processed.
- Validation and processing statuses.
- Duplicate and reconciliation information.

**Technical Logs**
- Detailed diagnostic events.
- Errors, warnings and relevant technical context.
- Information useful for troubleshooting.

### 7.4 Live Progress Reporting

Long-running backend operations should emit progress events to the frontend as work progresses.

The interface must not depend on waiting for an entire operation to finish before displaying useful information.

### 7.5 Persistence and Privacy

Important operational logs should be retained locally across application restarts.

Logging must avoid:
- Passwords and authentication credentials.
- Encryption keys and access tokens.
- Unnecessary personal or financial information.
- Sensitive statement contents.

Logging should provide meaningful visibility without generating excessive noise.

The import workspace will be the first major consumer of the shared logging system.

## 8. Testing Conventions

ETracker follows a four-level testing strategy.

### 8.1 Unit Testing

Tests individual functions, components and business rules independently.

Examples:
- Financial calculations.
- Transaction validation.
- Budget allocation calculations.
- Date and currency handling.

### 8.2 Integration Testing

Tests interactions between multiple components.

Examples:
- Transaction creation and database persistence.
- Repository operations.
- Import staging and validation.
- Application use cases.

### 8.3 End-to-End Testing

Tests complete application workflows.

Examples:
- Creating an account.
- Recording an expense.
- Importing and reviewing a statement.
- Creating a monthly budget.
- Reviewing financial summaries.

### 8.4 Regression Testing

Ensures previously fixed defects do not reappear.

Where practical, bug fixes should include regression tests.

### 8.5 Testing Responsibilities

| Area | Testing approach |
|---|---|
| Rust backend | Built-in Rust testing framework |
| Frontend | Vitest |
| React components | React Testing Library |
| Complete application | Playwright, with Tauri integration evaluated during implementation |

### 8.6 Testing Requirements

1. Financial calculations and invariants require automated tests.
2. Test valid, invalid and boundary inputs.
3. Test failure paths, including database failures and interrupted operations.
4. Verify transaction integrity, including transfers, reversals, voids and split allocations.
5. Verify that failed operations do not leave unintended partial financial records.
6. Include regression coverage for identified defects where practical.
7. Manual verification alone does not establish that a feature is complete.

Testing tools will be introduced when they become necessary rather than installed prematurely.

## 9. Documentation and Change Management

The `docs/` directory is the authoritative source of project requirements, architecture, design and engineering conventions.

- Keep documentation aligned with actual implementation.
- Update relevant documentation when approved decisions change.
- Record significant architectural decisions in `docs/decisions/decision-log.md`.
- Distinguish approved decisions from provisional choices and pending decisions.
- Do not silently alter accepted requirements or architectural decisions.

Changes affecting scope, financial rules, data integrity, security or architecture must be evaluated and documented before implementation.

## 12. Status and Future Refinement

This document establishes the approved working conventions for ETracker's initial development.

Some implementation details remain intentionally open, including:
- Exact Rust error structures.
- Logging library and storage implementation.
- Frontend/backend event contracts.
- Testing library configuration.
- Formatting and linting tool configuration.
- Exact branch protection and merge policies.

These details will be finalized when the relevant implementation work begins.

The conventions may evolve through controlled changes as the application matures.