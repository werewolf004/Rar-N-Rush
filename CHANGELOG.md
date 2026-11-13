# Changelog

**English** · [Français](CHANGELOG.fr.md)

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-13

First public version: the complete processing chain, a quality foundation and the documentation.

### Added

- Monitoring of a download folder (optionally recursive) at a configurable interval. A set is processed only once its files have stopped changing.
- Grouping of files into sets: PAR2 index and volumes (protected names read from the `FileDesc` packets), SFV files and RAR volumes (`.partN.rar`, `.rar` + `.rNN`/`.sNN`).
- PAR2 verification and repair through an external `par2.exe`, with live progress and cancellation. When repair is impossible, the set waits and is retried when new files arrive.
- Native SFV (CRC32) verification.
- RAR extraction with the official UnRAR library: multi-volume archives, password list, password typed in the window, encrypted headers, nested archives (up to 3 levels).
- Data protection: existing files are kept (`name (1).ext`) unless overwriting is enabled; entries with absolute paths or `..` are refused; source files go to the Recycle Bin, never permanently deleted; processed sets are recorded before their status is shown.
- egui interface: list of sets with status, progress and detail, Skip, Retry, Open folder and Remove actions, filterable log, settings window.
- English and French interface, following the Windows language by default.
- Light, dark or automatic theme (follows Windows live).
- Portable settings next to the executable with atomic writes, falling back to `%APPDATA%\RarNRush`. An existing settings file is always found again (next to the executable first, then `%APPDATA%`), even after the executable is moved.
- Settings are never silently lost: each key is loaded on its own, invalid values are reset individually after a backup copy (`rar-n-rush.toml.<date-time>.bak`), and a banner tells the user. A UTF-8 byte-order mark (Notepad) is accepted.
- No network access at all: no update check, no telemetry. Networking crates are banned by `cargo deny`.
- Documentation in English and French: README, settings reference, dependencies, contributing guide, security policy, roadmap, screenshots generated off-screen.
- CI (GitHub Actions): formatting, `clippy::pedantic` with warnings as errors, unit, integration and headless UI tests, MSRV check, `cargo deny`. Release workflow that publishes the binary, its SHA-256 and the matching changelog section.

### Fixed

- Random `STATUS_ACCESS_VIOLATION` crashes on multi-volume archives, caused by an out-of-bounds read in the `unrar` crate 0.5.8 (patched copy in `vendor/unrar`).

### Security

- `#![forbid(unsafe_code)]` on the whole crate; `unwrap`, `expect` and `panic` are denied outside tests.

[Unreleased]: https://github.com/werewolf004/Rar-N-Rush/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/werewolf004/Rar-N-Rush/releases/tag/v0.1.0
