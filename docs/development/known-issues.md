# ETracker — Known Issues and Outstanding Work

**Last updated:** 2026-11-04

## 1. Confirmed Defects

No confirmed application defects have been identified at the current development stage.

The initial application scaffold has successfully built and launched.

## 2. Outstanding Implementation Work

The following items represent functionality and engineering work that has not yet been implemented. They are not necessarily defects.

| ID | Area | Outstanding work | Status |
|---|---|---|---|
| OW-001 | Frontend | Replace the default React starter interface with ETracker's application interface | Pending |
| OW-002 | Backend | Replace or remove the scaffold demonstration greeting command as appropriate | Pending |
| OW-003 | Database | Integrate SQLite and establish database initialization | Pending |
| OW-004 | Database | Implement schema migrations and repository infrastructure | Pending |
| OW-005 | Styling | Integrate Tailwind CSS | Pending |
| OW-006 | Architecture | Establish the agreed frontend and Rust directory structures as features are implemented | Pending |
| OW-007 | Testing | Introduce the initial automated testing infrastructure | Pending |
| OW-008 | Logging | Implement shared structured logging and operational event infrastructure | Pending |
| OW-009 | Security | Establish an appropriate Content Security Policy before release | Pending |
| OW-010 | Documentation | Finalize repository setup instructions and replace the README repository URL placeholder | Pending |
| OW-011 | Release | Assign an appropriate software license if distribution requires one | Pending |

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