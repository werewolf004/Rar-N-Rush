# Security policy

**English** · [Français](SECURITY.fr.md)

## Supported versions

Only the latest release receives security fixes.

## Reporting a vulnerability

Please do **not** open a public issue. Use GitHub's private reporting instead: **Security → Report a vulnerability** on the repository page. You will get an answer within 7 days. Include the version, the steps to reproduce and, if possible, a sample archive.

## Security model

Rar N Rush processes files downloaded from the Internet, which must be treated as hostile. The measures below are in place.

### Code

- `#![forbid(unsafe_code)]`: the project's own code contains no `unsafe`.
- `unwrap`, `expect` and `panic` are denied outside tests (`clippy` lints, enforced in CI). Every failure is a typed error (`thiserror`) shown to the user.
- A poisoned lock does not crash the application: the shared state holds plain data and is recovered.

### Archives and files

- Archive entries with an absolute path, a drive letter or a `..` component are **skipped**: nothing can be written outside the destination folder.
- Existing files are **never overwritten** unless the user enables it; a free name (`name (1).ext`) is used instead.
- Files written with a wrong password are removed before the next password is tried.
- The scan does not follow symbolic links, so a link cannot pull files from outside the watched tree.
- Folder names derived from archive names are sanitised (characters forbidden by Windows, reserved names such as `CON` or `NUL`).
- The log file (`rar-n-rush.log`) never contains a password: log entries are typed events that carry no secret, and a test checks it.
- The free space of the destination drive is checked before extracting (archive size plus a 64 MB reserve).
- A failed or cancelled extraction removes the files it created; files that existed before are never removed.
- Source files are sent to the **Recycle Bin**, never deleted permanently, and only after a fully successful run.
- `par2.exe` is started without a shell, with arguments passed separately (no command injection), without a console window, and only from the path set by the user, the program folder or `PATH`. The `par2.exe` shipped in the release zip is the official par2cmdline-turbo build, checked against a pinned SHA-256 before it is tested and packaged, and covered by the release provenance attestation.

### Network access: only the optional update check

- No telemetry, no crash reporting. By default Rar N Rush makes **no network request at all**.
- The only exception is the update check, **off by default** (Settings → General → Updates). When it is on, about a minute after start-up and then at most once a day, week or month, the program asks `https://api.github.com/repos/werewolf004/Rar-N-Rush/releases/latest` for the number of the latest release. The request carries nothing but a `rar-n-rush/<version>` user agent: no identifier, no statistics. Nothing is downloaded or installed; the window shows a link to the release page and you decide.
- The request goes through `%SystemRoot%\System32\curl.exe`, shipped with Windows 10 and 11 (never a copy found through `PATH`), HTTPS only, redirects included, with a 30-second time limit and a 1 MB size limit. Only a strict `vX.Y.Z` release number is read from the answer; the link shown is built from that number, never taken from the answer.
- The program itself still carries no network code, enforced by `cargo deny`: HTTP, TLS and async networking crates (`reqwest`, `hyper`, `rustls`, `tokio`, `self_update`…) are banned, so CI fails if one ever enters the dependency graph. Tests never go online (a fake answers instead).

### Persistent data

- Settings are never silently lost: a settings file that cannot be read is never replaced, a file saved in "ANSI" is read as Windows-1252, a Credential Manager that could not be read at start-up is never written to during that session; each key is read on its own, invalid values are reset individually, and the original file is backed up first (`rar-n-rush.toml.<date-time>.bak`). The user is told in a banner.
- An existing settings file is always found again, next to the executable or in `%APPDATA%\RarNRush`, even after the executable is moved.
- Settings and the processed-sets list are written atomically (temporary file then rename): a crash cannot leave a truncated file.
- A set is recorded as processed **before** its status is shown, so a crash cannot cause it to be processed twice.
- Registry writes are limited to the current user (`HKEY_CURRENT_USER`), need no administrator rights and are undone when the option is turned off: the `Rar N Rush` value under `...\CurrentVersion\Run` (start with Windows) and the `RarNRush.App` key under `Software\Classes\AppUserModelId` (name and icon of the Windows notifications).
- The installer is per-user: it writes only under the user's profile and `HKEY_CURRENT_USER` (no administrator rights, no UAC prompt, no service, nothing for other accounts). On removal (not on upgrade) it starts `rar-n-rush.exe --uninstall`, which removes the autostart entry only if it starts that installed copy, and the notification identity. Settings and stored passwords are kept.
- One instance per settings folder: a lock file held by the running program (released by Windows when it ends, even after a crash) prevents two copies from writing the same settings and lists at the same time.
- Archive passwords are kept in the Windows Credential Manager (protected by your Windows account), not in `rar-n-rush.toml`. Writes are crash-safe: a new copy is written, then switched to, then the old one is deleted. If the Credential Manager is unavailable, the passwords stay in the settings file in plain text so none is lost, and the window says so.
- **Known limitation**: backup copies of the settings file made by older versions (`rar-n-rush.toml.<date-time>.bak`) may still contain plain-text passwords; delete them if needed.

### Third-party code

- The only `unsafe` code comes from dependencies. The most sensitive one is the RAR extraction: the `unrar` crate binds the official UnRAR C++ library through FFI.
- `unrar` 0.5.8 had an out-of-bounds read on volume change that crashed the process at random. The copy in `vendor/unrar` fixes it; see [vendor/unrar/PATCHES.md](vendor/unrar/PATCHES.md).
- `cargo deny` checks the RustSec advisory database, licences and sources on every CI run. Dependabot proposes updates weekly.
- GitHub Actions are pinned to full commit SHAs, and workflows run with read-only permissions by default.

### Releases

- The program and the installer carry no Authenticode signature (code-signing certificates are not free), so Windows SmartScreen may warn on first use. Their origin is proven instead by the SHA-256 checksum and the signed build provenance below.
- Each release archive and installer is published with its SHA-256 checksum. Verify it before use (see the README).
- The installer is built with WiX Toolset 5.0.2, pinned by SHA-256 (`scripts/fetch-wix.ps1`); CI installs, upgrades and removes it silently on every push and checks what each step leaves behind.
- From v0.3.0, the archive and the executable carry a signed SLSA build provenance (GitHub artifact attestation, Sigstore): From v1.0.0 the installer is covered too. `gh attestation verify` proves they were built by the release workflow of this repository from the tagged commit.
