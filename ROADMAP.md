# Roadmap

**English** · [Français](ROADMAP.fr.md)

Each block ends with a tagged release and its changelog entry. Order and content may change; done items move to [CHANGELOG.md](CHANGELOG.md).

## ✅ v0.1.0 — Foundations

Complete processing chain (scan, PAR2, SFV, RAR), typed errors, no `unsafe`/`unwrap`, unit, integration and headless UI tests, CI/CD, bilingual documentation, licence.

## v0.2.0 — UI/UX and accessibility

- ✅ Settings with inline validation (folder exists, destination writable, par2 found) and an "unsaved changes" guard.
- ✅ Shortcuts F5 (scan), Ctrl+, (settings), Esc. Still to do: full keyboard navigation of the list, Del to remove.
- ✅ Screen-reader labels on every control (AccessKit), checked by the headless UI tests.
- ✅ Contrast checked against WCAG 2.2 AA in both themes by an automated test.
- Confirmation before destructive actions; non-blocking toasts for results.
- ✅ Window size and position remembered. Still to do: DPI scaling tested at 100–200 %.
- Snapshot tests of the main screens to catch visual regressions.

## v0.3.0 — Data and security

- Archive passwords stored in the Windows Credential Manager instead of the settings file.
- Free-space check before extraction; clean-up of partial files on failure.
- Fuzzing of the PAR2 and SFV parsers (`cargo fuzz`), property tests for name grouping.
- Log export and rotation; no secret ever written to the log.
- Signed release checksums (Sigstore / GitHub artifact attestations).

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
