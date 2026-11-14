# Roadmap

**English** · [Français](ROADMAP.fr.md)

Each block ends with a tagged release and its changelog entry. Order and content may change; done items move to [CHANGELOG.md](CHANGELOG.md).

## ✅ v0.1.0 — Foundations

Complete processing chain (scan, PAR2, SFV, RAR), typed errors, no `unsafe`/`unwrap`, unit, integration and headless UI tests, CI/CD, bilingual documentation, licence.

## ✅ v0.2.0 — UI/UX and accessibility

Watched folder editable in the main window with recent folders; Par-N-Rar options (move extra files, delete empty folders, follow the active set, notification area); settings in three pages with validation and an unsaved-changes guard; shortcuts F5, Ctrl+, and Esc; window size and position remembered; WCAG 2.2 AA contrast test; AccessKit labels checked by the UI tests.

## ✅ v0.3.0 — Data and security

Archive passwords in the Windows Credential Manager (migrated from the settings file, never lost); free-space check before extraction and clean-up of failed extractions; log file with rotation and export, with no secret ever written; signed build provenance for releases; property tests and `cargo fuzz` targets for every parser of untrusted input; keyboard navigation of the list, non-blocking notices and confirmation before overwriting.

## ✅ v0.4.0 — Complete PAR2

par2cmdline-turbo shipped in the release zip (pinned version and SHA-256, licence and source included; the application stays offline); end-to-end tests with the real par2 in CI (damaged and missing volumes, not enough blocks, waiting for recovery volumes); pixel snapshot tests of the main screens without a GPU; accessible names for every settings field.

## v0.5.0 — Windows integration

- ~~Icon in the notification area, minimise/close to tray.~~ Brought forward to v0.2.0.
- ✅ Windows notifications when a set finishes, fails or needs a password (only while the window is not in front).
- ✅ Start with Windows (optional), single running instance.

## v1.0.0 — Distribution

- Installer (MSI) and winget package.
- Code signing.
- User guide with screenshots for every screen.
