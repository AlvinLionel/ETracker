# ETracker — Deployment and Maintenance

**Document ID:** DOC-007  
**Version:** 0.3
**Status:** Implemented baseline — preview release verification in progress
**Last Updated:** 13 November 2026

---

## 1. Introduction

### 1.1 Purpose

This document defines the deployment, installation, release, maintenance and operational strategy for ETracker.

It establishes how the application will move from development into a stable desktop release and how it will be maintained throughout its lifecycle.

### 1.2 Objectives

The deployment and maintenance strategy must ensure that:

1. ETracker can be installed and used as a local desktop application.
2. Core financial functionality works without an internet connection.
3. User financial data remains stored locally.
4. Application updates do not silently damage existing data.
5. Database migrations preserve financial history.
6. Backups can be created and restored reliably.
7. Releases are tested and traceable.
8. Application failures can be diagnosed without exposing unnecessary financial information.
9. Dependencies and security issues are maintained responsibly.
10. The architecture remains suitable for future Linux support and cloud synchronization.

### 1.3 Scope

This document covers:
- Development and release environments.
- Application packaging.
- Installation.
- Initial database setup.
- Database migrations.
- Updates.
- Backup and recovery.
- Logging and diagnostics.
- Security maintenance.
- Release management.
- Continuous integration and build verification.
- Defect handling.
- Long-term maintenance.

It does not define a cloud-hosted deployment for the initial release.

---

## 2. Deployment Architecture

### 2.1 Initial Deployment Model

ETracker will initially be deployed as a local desktop application.

The implemented technology direction is:

| Component | Proposed technology |
|---|---|
| Desktop shell | Tauri 2 |
| Frontend | React |
| Frontend language | TypeScript |
| Backend | Rust |
| Database | SQLCipher (SQLite-compatible) |
| Styling | Tailwind CSS |

The frontend is React with TypeScript and the backend is Rust. The database is SQLCipher with a key protected by current-user Windows DPAPI. Release packaging and clean-profile installation still require final verification.

### 2.2 Local-First Operation

The initial application must not require:
- A hosted backend server.
- A remote database.
- A continuously available internet connection.
- External authentication services for ordinary local use.

Core functionality must operate locally.

Optional online services, including AI-assisted functionality, must remain separate from essential financial operations.

### 2.3 Data Storage

The application must maintain a clear separation between:

- Application installation files.
- User financial database.
- Managed attachments.
- Backups.
- Application logs.
- Configuration and preferences.

Financial data must not be stored in temporary installation directories or locations that may be deleted during an update.

### 2.4 Platform Direction

Windows desktop is the initial deployment target.

The architecture should avoid unnecessary Windows-specific assumptions so that Linux support can be evaluated later.

Linux support is a future compatibility objective, not a commitment to release on Linux in the initial version.

---

## 3. Development, Testing and Production Environments

### 3.1 Development Environment

The development environment is used for:
- Feature implementation.
- Debugging.
- Local database experimentation.
- Automated testing.
- Interface development.
- Integration testing.

Development data should use synthetic or disposable financial records.

### 3.2 Test Environment

The test environment is used for controlled verification.

It should include:
- Representative database records.
- Synthetic statement files.
- Known expected financial results.
- Import edge cases.
- Backup and recovery test data.
- Release candidate builds where appropriate.

Real personal financial information should not be used unnecessarily in automated tests or shared test artifacts.

### 3.3 Production Environment

The production environment is the installed application used for actual financial management.

It must use the user's persistent local data storage.

Production financial records must not be accidentally replaced by development fixtures or test databases.

### 3.4 Environment Separation

The application must clearly distinguish development, test and production configurations.

Build configuration must prevent accidental use of test settings in production releases.

---

## 4. Application Packaging and Installation

### 4.1 Packaging

Tauri builds unsigned Windows x64 MSI and NSIS setup packages. For the 0.1.x preview, MSI is the primary managed-install option and NSIS is an alternative. A tag or manual workflow run builds both and retains the artifacts for 30 days; the workflow does not publish public releases (`DB-036`).

### 4.2 Installation Requirements

The installation process should:

- Install application files in an appropriate location.
- Create required application directories.
- Preserve existing user data.
- Initialize the database on first launch.
- Apply required schema migrations.
- Provide a clear completion or failure message.

Building the Windows release requires the Rust and Tauri toolchains, Node.js, the Windows native build prerequisites, and Perl for vendored OpenSSL used by SQLCipher. CI and release workflows install Strawberry Perl.

### 4.3 First Launch

On first launch, ETracker should:

1. Initialize the application configuration.
2. Establish required local storage directories.
3. Create the initial database.
4. Apply the initial schema.
5. Initialize default settings.
6. Present the user with the initial setup workflow.

The application must verify that initialization completed successfully before allowing ordinary financial operations.

### 4.4 Existing Installation

Installing a newer version must not overwrite or delete the user's existing database.

The application should identify existing data and determine whether migration is required.

### 4.5 Installation Failure

If installation or initialization fails:

- The error must be communicated clearly.
- Existing user data must remain protected.
- Partial initialization must not be mistaken for successful setup.
- A safe recovery or retry path should be available.

---

## 5. Database Initialization and Migrations

### 5.1 Initial Database Setup

The first application launch must create a valid database structure.

The initialization process must:
- Create required tables and indexes.
- Establish foreign key enforcement.
- Record the schema version.
- Initialize required application metadata.
- Verify database integrity where practical.

### 5.2 Migration Strategy

Database schema changes must use versioned migrations.

Each migration must have:
- A unique version.
- A clear purpose.
- Defined schema changes.
- Defined data transformations where applicable.
- Validation requirements.

### 5.3 Migration Safety

Before applying a potentially destructive or high-risk migration, the application should create or verify an appropriate recovery point.

Migrations must:
- Preserve financial records.
- Maintain referential integrity.
- Handle interrupted execution safely.
- Avoid partial schema changes.
- Record successful completion.

### 5.4 Migration Failure

If a migration fails:

- The application must not proceed as if the migration succeeded.
- The failure must be recorded.
- Existing data must remain recoverable.
- The application must provide a controlled recovery path.

### 5.5 Database Compatibility

Application versions must identify which database schema versions they support.

An older application version must not silently open a newer incompatible database and damage it.

---

## 6. Application Updates

### 6.1 Update Objectives

Updates should deliver:
- Bug fixes.
- Security improvements.
- Performance improvements.
- New features.
- Compatibility changes.

Updates must preserve user financial data.

### 6.2 Update Process

The proposed update workflow is:

1. Identify the available application version.
2. Review release information.
3. Download or obtain the update through the supported distribution method.
4. Verify the update package where supported.
5. Prepare an appropriate database recovery point when required.
6. Install the new application version.
7. Apply required database migrations.
8. Verify application startup and database compatibility.

### 6.3 Update Failure

If an update fails:

- Existing financial data must remain protected.
- The user must receive a clear failure message.
- Recovery procedures must be documented.
- The application must not silently discard incompatible data.

### 6.4 Rollback Considerations

Application rollback and database rollback are separate concerns.

Returning to an older application binary does not guarantee compatibility with a database already migrated by a newer version.

The update strategy must therefore define compatibility and recovery behaviour before enabling automatic database upgrades.

A verified backup may be required before restoring an earlier database state.

### 6.5 Update Distribution

The final distribution and update mechanism remains undecided.

Options may include:
- Manually downloaded releases.
- A supported desktop update mechanism.
- A managed distribution channel.

The selected approach must be appropriate for a local-first financial application and must not compromise data integrity.

---

## 7. Backup Strategy

### 7.1 Purpose

Backup functionality protects the user's financial history against:
- Accidental data loss.
- Database corruption.
- Device failure.
- Application defects.
- Failed updates.
- Accidental deletion of local files.

### 7.2 Backup Contents

A complete backup should account for:

- SQLite database.
- Managed attachments.
- Required metadata.
- Relevant application configuration where appropriate.

### 7.3 Backup Creation

Settings supports user-initiated encrypted backups and restore. Backup creation opens a native save dialog with the ETracker backup extension and writes encrypted bytes to the selected path. Restore uses a user-selected file. Automatic scheduling and app-managed generations are not implemented. Native Windows dialogs still need manual verification.

The SQLite backup API creates a consistent database snapshot. Current managed statement source files are included because they are stored in the database.

SQLite's journaling and transaction behaviour must be considered when implementing backup operations. Copying a database file while it is actively being modified must not be assumed safe.

### 7.4 Backup Retention

Downloaded backup retention is user-managed; ETracker does not maintain an archive or delete exported files. Users should keep multiple copies as appropriate and remove them when no longer needed.

### 7.5 Backup Integrity

Restore authenticates/decrypts the file, validates SQLite structure and schema compatibility, migrates a supported older schema in an isolated in-memory copy, and checks integrity and foreign keys before replacing current data. Automated tests cover valid restore and rejection of wrong-passphrase, tampered and invalid files. Manual Windows restore testing remains open.

### 7.6 Backup Location

Backup locations should be configurable.

The application should support selecting a location outside the main application data directory so that backups can survive failure of the primary storage location.

### 7.7 Backup Security

Backups contain sensitive financial information.

The application must communicate the importance of protecting backup files.

Backups use AES-256-GCM with a random salt and nonce; PBKDF2-HMAC-SHA256 derives keys with 600,000 iterations. Passphrases must contain at least 12 characters and are never stored. A lost passphrase cannot be recovered. The live SQLCipher database key is protected using current-user Windows DPAPI. Attachments are AES-GCM encrypted at rest; opening one externally creates a temporary decrypted copy that is removed on next startup or by the Settings cleanup action. Physical erasure of old filesystem blocks or SSD snapshots cannot be guaranteed.

---

## 8. Recovery and Restoration

### 8.1 Recovery Objectives

Restoration must return ETracker to a valid and usable state without silently introducing missing or duplicated financial records.

### 8.2 Restoration Workflow

The proposed workflow is:

1. Select an encrypted backup and enter its passphrase.
2. Type `RESTORE` to confirm replacement.
3. Authenticate and validate the backup before replacement.
4. Migrate supported older schemas in memory.
5. Replace current data using SQLite's backup API.
6. Reload the app to show restored data.

### 8.3 Restoration Safety

Restoration may replace the current database state.

The application must clearly explain this before the user confirms.

The restore workflow validates before replacing current data. A guaranteed rollback after an underlying SQLite page-copy failure is not provided; verify a separate backup exists before restoring important data.

### 8.4 Recovery Testing

Recovery procedures must be tested with representative databases and attachments.

The project must not assume that a backup is valid simply because a file exists.

### 8.5 Recovery Documentation

Recovery instructions must be maintained alongside the application documentation.

The instructions should explain:
- Where backups are stored.
- How to identify a valid backup.
- How restoration works.
- What happens to current data.
- What to do if restoration fails.

---

## 9. Logging and Diagnostics

### 9.1 Purpose

Logging supports debugging, maintenance and investigation of application failures.

### 9.2 Logging Categories

The application should distinguish:

- Informational events.
- Warnings.
- Errors.
- Critical failures.

### 9.3 Relevant Events

Logs may cover:
- Application startup and shutdown.
- Database initialization.
- Migration activity.
- Import processing.
- Reconciliation outcomes.
- Backup and restoration.
- Application update activity.
- External service failures.

### 9.4 Sensitive Information

Logs must not unnecessarily expose:
- Full financial statements.
- Complete transaction descriptions.
- Sensitive account identifiers.
- Authentication credentials.
- AI service secrets.
- Private attachment contents.

Diagnostic messages should provide enough information to investigate failures without unnecessarily reproducing sensitive financial information.

### 9.5 Log Retention

The app stores only a bounded, allowlisted diagnostic event history in the encrypted local database; events contain fixed codes rather than free-form or financial data and expire after 30 days. Settings can create a user-requested support report with environment/database health and up to 100 recent events. It excludes financial records, is not transmitted by ETracker, and is retained only where the user saves it.

### 9.6 Import Logs

Import-specific activity must remain accessible through the import workspace.

Import logs should identify processing stages, warnings, errors and relevant staged record references.

Completed import logs should remain available for review according to the data-retention policy.

---

## 10. Security Maintenance

### 10.1 Dependency Maintenance

Dependencies must be reviewed and updated when appropriate.

The project should:
- Monitor relevant dependency security advisories.
- Avoid unnecessary dependencies.
- Remove unused dependencies.
- Review significant version changes.
- Test dependency updates before release.

### 10.2 Credential Management

Credentials for optional external services must not be committed to the repository.

Sensitive credentials must not be stored as ordinary plaintext application settings.

The final credential-storage approach remains subject to security evaluation.

### 10.3 Application Integrity

Release packages should use appropriate integrity verification.

Code signing and installer trust mechanisms must be evaluated before public distribution.

### 10.4 Data Protection

Financial data and backups must be treated as sensitive.

The application should minimize unnecessary data exposure in:
- Logs.
- Temporary files.
- Diagnostic exports.
- Error reports.
- External service requests.

### 10.5 Security Defects

Security issues must be documented, prioritized and addressed according to their impact.

Critical security defects must block release until resolved or formally addressed through an approved risk decision.

---

## 11. Release Management

### 11.1 Release Types

The project should distinguish:

**Development builds**

Used during implementation and debugging.

**Test builds**

Used for controlled verification.

**Release candidates**

Builds intended for final release evaluation.

**Stable releases**

Versions considered suitable for normal use.

### 11.2 Versioning

Semantic Versioning is proposed:

`MAJOR.MINOR.PATCH`

- MAJOR: incompatible changes.
- MINOR: backward-compatible functionality.
- PATCH: backward-compatible fixes.

The exact versioning policy will be adopted when release management is implemented.

### 11.3 Release Notes

Each release should include:
- Version.
- Release date.
- New features.
- Bug fixes.
- Improvements.
- Known issues.
- Database migration information.
- Relevant compatibility notes.

### 11.4 Release Checklist

A release candidate should be evaluated against the following checklist:

- [ ] Required functionality implemented.
- [ ] Automated tests pass.
- [ ] Relevant manual tests completed.
- [ ] Financial correctness tests pass.
- [ ] Import and reconciliation tests pass.
- [ ] No unresolved critical data-integrity defects.
- [ ] Database migrations verified.
- [ ] Backup and restoration verified where affected.
- [ ] Application packaging verified.
- [ ] Application startup verified.
- [ ] Offline functionality verified.
- [ ] Sensitive information handling reviewed.
- [ ] Release notes prepared.
- [ ] Build artifact identified and retained.

### 11.5 Release Approval

A stable release must not be declared solely because the application builds successfully.

Release acceptance must consider test results, known defects, financial integrity and recovery readiness.

---

## 12. Build and Continuous Integration

### 12.1 Build Reproducibility

The project should maintain a consistent build process.

Relevant dependencies and build instructions must be documented.

Builds should be identifiable by:
- Version.
- Source commit.
- Build configuration.
- Build date.
- Relevant dependency versions.

### 12.2 Continuous Integration

A Windows CI workflow at `.github/workflows/ci.yml` automates checks on pushes and pull requests.

Configured checks:
1. Dependency installation.
2. Formatting verification.
3. Static analysis.
4. Frontend tests.
5. Rust tests.
6. Database integration tests.
7. Build verification.
8. Relevant security checks (not yet included in the workflow).

### 12.3 Build Artifacts

Release artifacts should be retained with their version and source commit information.

The project should be able to identify which source revision produced a given release.

### 12.4 CI Limitations

CI must not replace real desktop installation, update, backup and recovery testing.

These workflows require suitable environment-level verification.

---

## 13. Maintenance Strategy

### 13.1 Corrective Maintenance

Addresses defects discovered after implementation or release.

Examples:
- Incorrect financial calculations.
- Failed imports.
- Database errors.
- Broken user workflows.
- Security defects.

### 13.2 Adaptive Maintenance

Keeps the application compatible with changes in its environment.

Examples:
- Operating system updates.
- Dependency changes.
- Statement format changes.
- Desktop framework changes.
- Database library updates.

### 13.3 Preventive Maintenance

Reduces the likelihood of future defects.

Examples:
- Dependency reviews.
- Database integrity checks.
- Backup verification.
- Code quality improvements.
- Automated regression testing.

### 13.4 Perfective Maintenance

Improves existing functionality without necessarily correcting a defect.

Examples:
- Better analytics.
- More efficient imports.
- Improved accessibility.
- Better interface usability.
- Performance improvements.

### 13.5 Maintenance Priorities

Financial correctness, data integrity, security and recoverability take precedence over cosmetic improvements.

---

## 14. Incident and Issue Management

### 14.1 Incident Categories

Relevant incidents include:
- Incorrect financial balance.
- Duplicate financial records.
- Missing financial transactions.
- Import corruption.
- Database corruption.
- Failed restoration.
- Sensitive information exposure.
- Application startup failure.

### 14.2 Incident Handling

The proposed process is:

1. Identify and record the incident.
2. Determine its impact.
3. Protect affected financial data.
4. Reproduce the issue where safe.
5. Investigate the cause.
6. Develop a correction.
7. Verify the correction.
8. Run regression tests.
9. Document the resolution.

### 14.3 Financial Integrity Incidents

If an incident may have affected financial data:

- Avoid further destructive operations.
- Preserve relevant logs and database copies.
- Identify the affected records.
- Establish the correct financial state.
- Apply controlled corrections.
- Verify the resulting balances.
- Record the incident and resolution.

The application must not silently hide or rewrite evidence of a financial integrity problem.

---

## 15. Documentation Maintenance

Project documentation must remain consistent with the implemented application.

Documentation should be updated when changes affect:
- Requirements.
- Architecture.
- Database design.
- UI/UX.
- Testing.
- Deployment.
- Maintenance.
- Approved project decisions.

The decision log must record approved design changes.

The README should remain a concise entry point, while detailed technical information belongs in the documentation directory.

---

## 16. Future Platform and Synchronization Direction

### 16.1 Linux Compatibility

Linux support is a future objective.

The application should avoid unnecessary dependencies on Windows-only behaviour.

Platform-specific installation, storage and integration concerns must be isolated where practical.

### 16.2 Cloud Synchronization

Cross-device synchronization is a future requirement, but not part of the initial desktop deployment.

Future synchronization will require:
- Secure remote infrastructure.
- Authentication and authorization.
- Device identity.
- Change tracking.
- Conflict resolution.
- Data integrity verification.
- Secure transport.
- Recovery and backup policies.

The initial release must remain fully useful without these capabilities.

### 16.3 Future Migration

The local-first design should make it possible to introduce synchronization without treating the desktop application as dependent on a remote service.

The exact synchronization architecture must be designed and approved separately when that phase is reached.

---

## 17. Open Deployment and Maintenance Decisions

| ID | Topic | Status |
|---|---|---|
| DM-001 | Supported Windows versions | Pending evaluation against a Windows version matrix |
| DM-002 | Installer format | Approved: unsigned x64 MSI (primary) and NSIS (alternative) for 0.1.x preview (`DB-036`) |
| DM-003 | Code signing and installer trust | Pending evaluation |
| DM-004 | Update and distribution mechanism | Approved: manual versioned updates; no automatic or public release publishing for preview (`DB-036`) |
| DM-005 | Backup format and retention | Approved: manual encrypted exports, user-managed retention (`DB-030`) |
| DM-006 | Backup encryption | Approved: passphrase-based authenticated encryption (`DB-030`) |
| DM-007 | Database migration framework | Implemented with versioned SQLite migrations; rollback after page-copy failure remains unguaranteed |
| DM-008 | Log retention and diagnostic export | Allowlisted local events retained for 30 days; user-requested report export (`DB-031`) |
| DM-009 | Release branch and tagging policy | Approved: SemVer, `vMAJOR.MINOR.PATCH`, 0.x preview (`DB-036`) |
| DM-010 | Stable release cadence | No fixed cadence for 0.x preview; release only at verified milestones (`DB-036`) |
| DM-011 | External AI credential handling | Pending security design |

---

## 18. Current Deployment and Maintenance Status

As of 8 November 2026:

| Area | Status |
|---|---|
| Local-first deployment direction | Approved |
| Windows desktop as initial platform | Approved |
| Provisional technology stack | Identified |
| Environment separation | Defined |
| Installation workflow | Defined |
| Database migration strategy | Defined conceptually |
| Update safety requirements | Defined |
| Backup and recovery direction | Approved |
| Logging and diagnostics requirements | Defined |
| Security maintenance requirements | Defined |
| Release management approach | Defined |
| Semantic versioning | Approved for 0.x preview tags (`DB-036`) |
| CI strategy | Defined conceptually |
| Maintenance categories | Defined |
| Linux compatibility direction | Approved for future consideration |
| Cloud synchronization | Future requirement; implementation deferred |
| x64 MSI and NSIS installer packages | Built; manual install/upgrade verification pending |
| Database migrations | Implemented and tested |
| Encrypted backup and restoration | Implemented and automated tests pass; manual desktop verification pending |
| CI pipeline | Implemented for Windows: formatting, frontend tests/build and Rust tests; first hosted run pending |
| First preview packages | OCR-era packages rebuilt locally on 8 November 2026: MSI 10.96 MiB, NSIS 9.88 MiB. They predate transaction attachments and are not current for distribution; hashes are in `RELEASE_NOTES.md`; artifacts are not committed |
| Installer workflow | Configured to retain unsigned MSI and NSIS artifacts for 30 days; first hosted run pending |
| Installation on this development profile | Not attempted because this machine already has data under the production app identifier; avoid replacing the existing local profile |
| Stable release | Not produced; manual install/upgrade, native file prompts, OS matrix, signing and offline smoke checks remain |

---

**End of Document — DOC-007**
