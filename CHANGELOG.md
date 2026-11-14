# Changelog

**English** · [Français](CHANGELOG.fr.md)

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-11-14

Data and security: passwords in the Windows Credential Manager, free-space check and clean-up of failed extractions, a rotated log file, signed release provenance, property tests and fuzzing, plus keyboard navigation of the list and non-blocking notices.

### Added

- The free space of the destination drive is checked before extracting: when the archive content plus a 64 MB reserve does not fit, the set fails with a clear message (needed and available sizes) instead of a half-written extraction.
- **Archive passwords are kept in the Windows Credential Manager**, no longer in plain text in `rar-n-rush.toml`. Passwords from an older settings file are moved there at start-up. If the Credential Manager is unavailable, they stay in the file so none is lost, and the window says so.
- **Log file**: the log is also written to `rar-n-rush.log` (`log_to_file`, on by default), rotated at 1 MB with three old files kept. **Export…** in the log panel saves the lines shown to a text file. A test checks that no password ever reaches the log.
- Release archives and executables carry a signed build provenance (GitHub artifact attestation); `gh attestation verify` checks it.
- Keyboard navigation of the list: **↑/↓** select a set (a click works too), **Del** removes it from the list, **Enter** opens its folder.
- Short, non-blocking notices in the bottom-right corner when a set finishes, fails or needs a password (`show_toasts`, on by default).
- Turning on **Overwrite existing files** asks for confirmation first, because an overwritten file cannot be recovered.
- Property tests (`proptest`) and fuzz targets (`cargo fuzz`, weekly **Fuzz** workflow) for the parsers of untrusted input: PAR2, SFV, RAR names, archive entry paths, folder names.

### Fixed

- At the smallest window size, the Processing page of the settings pushed **Save** out of the window; the page now scrolls and the buttons stay visible.
- A selected row could make status colours too faint (below 4.5:1); the selection colour is now softer and covered by the contrast test.
- A set named like a reserved Windows device followed by an extension (`CON.txt`, `nul.tar`) produced an extraction folder Windows cannot create; it is now prefixed with `_`. Found by the new property tests.
- A failed or cancelled extraction no longer leaves files behind: everything it created is removed (files that existed before are kept), so **Retry** starts clean instead of producing `name (1).ext` duplicates.

## [0.2.0] - 2026-11-14

User experience: the watched folder in the main window, the Par-N-Rar options (tidy-up, notification area), keyboard shortcuts and safer settings.

### Added

- **Watched folder** bar in the main window: the watched folder can be changed at any time, even during processing, without opening the settings. Type a path and press Enter (Esc cancels), pick one of the last 10 folders under **Recent folders**, or use **Browse…**. A folder that does not exist is refused with a message, and the current settings are kept.
- New setting `recent_watch_dirs` (list of recently watched folders).
- **Move extra files** (`move_extra_files`): after extraction, the other files of the set (`.nfo`, samples…) go to the destination. Only for a set alone in its own sub-folder, so other downloads are never touched.
- **Delete empty folders** (`delete_empty_dirs`): a set folder left empty after a successful run goes to the Recycle Bin, then its empty parents; never the watched folder itself.
- **Keep the set being processed in view** (`follow_active`, on by default): the list scrolls to each new active set.
- **Notification area**: minimise (`minimize_to_tray`) and/or close (`close_to_tray`) to an icon; monitoring goes on, a click brings the window back, **Quit** in its menu exits.
- Application icon for the window, the taskbar and the notification area.
- Keyboard shortcuts: **F5** scans now, **Ctrl+,** opens the settings, **Esc** closes them.
- The settings window checks the values before saving: **Save** is disabled, with the reason shown, while the watched folder does not exist, the destination is a file or read-only, or `par2.exe` is not at the given path.
- Closing the settings window with unsaved changes asks **Discard your changes?** first.
- The window size and position are remembered (`window` table in the settings file).
- Automated test: every status colour reaches the WCAG 2.2 AA contrast ratio (4.5:1) on every background, in both themes.

### Changed

- The monitoring state (Watching / Paused, time of the last scan) moved to the toolbar.
- The settings window is split into three pages (General, Processing, Window) so Save and Cancel always stay visible.

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

[Unreleased]: https://github.com/werewolf004/Rar-N-Rush/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/werewolf004/Rar-N-Rush/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/werewolf004/Rar-N-Rush/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/werewolf004/Rar-N-Rush/releases/tag/v0.1.0
