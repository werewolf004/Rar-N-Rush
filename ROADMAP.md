# Roadmap

**English** · [Français](ROADMAP.fr.md)

Each block ends with a tagged release and its changelog entry. Order and content may change; done items move to [CHANGELOG.md](CHANGELOG.md).

## ✅ v0.1.0 — Foundations

Complete processing chain (scan, PAR2, SFV, RAR), typed errors, no `unsafe`/`unwrap`, unit, integration and headless UI tests, CI/CD, bilingual documentation, licence.

## ✅ v0.2.0 — UI/UX and accessibility

Watched folder editable in the main window with recent folders; Par-N-Rar options (move extra files, delete empty folders, follow the active set, notification area); settings in three pages with validation and an unsaved-changes guard; shortcuts F5, Ctrl+, and Esc; window size and position remembered; WCAG 2.2 AA contrast test; AccessKit labels checked by the UI tests.

## v0.3.0 — Data and security

- Carried over from v0.2.0: full keyboard navigation of the list and Del to remove; confirmation before destructive actions and non-blocking toasts; DPI scaling tested at 100–200 %; snapshot tests of the main screens.
- ✅ Archive passwords stored in the Windows Credential Manager instead of the settings file.
- ✅ Free-space check before extraction; clean-up of partial files on failure.
- Fuzzing of the PAR2 and SFV parsers (`cargo fuzz`), property tests for name grouping.
- ✅ Log export and rotation; no secret ever written to the log.
- ✅ Signed build provenance for releases (Sigstore / GitHub artifact attestations).

## v0.4.0 — Complete PAR2

- End-to-end tests with a real `par2` (pinned version and checksum) in CI: damaged and missing files, repair, not enough blocks.
- Optional download of par2cmdline-turbo from the settings, with checksum verification and user confirmation.

## v0.5.0 — Windows integration

- ~~Icon in the notification area, minimise/close to tray.~~ Brought forward to v0.2.0.
- Windows notifications when a set finishes or needs a password.
- Start with Windows (optional), single running instance.

## v1.0.0 — Distribution

- Installer (MSI) and winget package.
- Code signing.
- User guide with screenshots for every screen.
