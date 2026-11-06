# ETracker — Known Issues and Outstanding Work

**Last updated:** 2026-11-06

## 1. Confirmed Defects

No open confirmed defects are currently recorded. A TypeScript build defect (unused `formatKES` helper in `AccountsPage.tsx`, TS6133) was found during transaction-slice verification and fixed in this milestone.

## 2. Outstanding Implementation Work

The following items represent functionality and engineering work that has not yet been implemented. They are not necessarily defects.

| ID | Area | Outstanding work | Status |
|---|---|---|---|
| OW-003 | Imports | Preserve uploaded source files and calculate stable content hashes | Pending |
| OW-005 | Imports | Add confirmed category allocation review and suggestion controls to imports | Pending |
| OW-006 | Imports | Verify provider statement formats and add sanitized fixtures before claiming provider parser support | Pending |
| OW-011 | Imports | Compare revised statements and provide controlled keep, replace, merge and individual-review choices | Pending |
| OW-007 | Planning | Implement budgets, savings goals and subscriptions | Pending |
| OW-008 | System | Implement analytics, forecasting, reminders and backup/recovery | Pending |
| OW-009 | Security | Review Content Security Policy and local data protection before release | Pending |
| OW-010 | Documentation | Replace the README repository URL placeholder and finish setup instructions | Pending |
| OW-012 | Release | Decide licensing and installer/distribution policies | Pending |

## 3. Configuration Review Items

The following generated scaffold configuration requires review as the project develops:

- Root `.gitignore` completeness.
- Tauri application identity and display configuration.
- Tauri Content Security Policy.
- Default application icons.
- Development and production configuration consistency.
- Dependency and lockfile maintenance.

These are review items, not confirmed configuration failures.

## 4. Reporting and Resolution Guidelines

When a defect is identified:

1. Assign a unique issue identifier.
2. Describe the observed behavior.
3. Record expected behavior.
4. Record reproduction steps where applicable.
5. Identify severity and impact.
6. Document the investigation and resolution.
7. Add regression coverage where practical.

Outstanding implementation work should be removed from this document once implemented and verified, with progress recorded in `implementation-progress.md`.
