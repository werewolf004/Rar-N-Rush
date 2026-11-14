# Dependencies

**English** · [Français](DEPENDENCIES.fr.md)

Every direct dependency is listed below with the reason it is needed. The complete list of the 229 crates compiled into the release binary, with their licences, is generated in [THIRD-PARTY-LICENSES.md](THIRD-PARTY-LICENSES.md) (`python scripts/third-party.py`).

Rules applied to dependencies:

- every licence must be in the allow-list of [`deny.toml`](../deny.toml) (permissive licences only);
- no known vulnerability (RustSec advisory database), checked by `cargo deny` in CI;
- crates.io only: no git or unknown registry;
- Dependabot proposes updates every week.

## Runtime dependencies

| Crate | Version | Licence | Why |
|---|---|---|---|
| [`eframe`](https://crates.io/crates/eframe) | 0.37 | MIT OR Apache-2.0 | Native window, rendering and event loop for egui. |
| [`egui`](https://crates.io/crates/egui) | 0.37 | MIT OR Apache-2.0 | Immediate-mode GUI. Enabled feature: `monochrome_emoji_fonts`, so the toolbar icons render identically on every machine. |
| [`egui_extras`](https://crates.io/crates/egui_extras) | 0.37 | MIT OR Apache-2.0 | Resizable table of file sets. |
| [`rfd`](https://crates.io/crates/rfd) | 0.18 | MIT | Native Windows folder and file pickers. |
| [`unrar`](https://crates.io/crates/unrar) | 0.5.8 (patched) | MIT OR Apache-2.0; embeds UnRAR (UnRAR licence) | RAR extraction with the official UnRAR library. Vendored in `vendor/unrar` with a memory-safety fix, see [vendor/unrar/PATCHES.md](../vendor/unrar/PATCHES.md). |
| [`crc32fast`](https://crates.io/crates/crc32fast) | 1 | MIT OR Apache-2.0 | CRC32 for SFV verification (SIMD-accelerated). |
| [`trash`](https://crates.io/crates/trash) | 5 | MIT | Sends source files to the Recycle Bin instead of deleting them. |
| [`serde`](https://crates.io/crates/serde) | 1 | MIT OR Apache-2.0 | Settings (de)serialisation. |
| [`toml`](https://crates.io/crates/toml) | 1 | MIT OR Apache-2.0 | Human-readable settings file. |
| [`thiserror`](https://crates.io/crates/thiserror) | 2 | MIT OR Apache-2.0 | Typed error enums. |
| [`chrono`](https://crates.io/crates/chrono) | 0.4 | MIT OR Apache-2.0 | Local time of log lines. Only the `clock` feature is enabled. |
| [`sys-locale`](https://crates.io/crates/sys-locale) | 0.3 | MIT OR Apache-2.0 | Reads the Windows display language to pick English or French. |
| [`tray-icon`](https://crates.io/crates/tray-icon) | 0.26 | MIT OR Apache-2.0 | Icon and menu in the notification area (only created when a tray option is enabled). Default features off: on Windows it only pulls `muda` (menu), `windows-sys` and `crossbeam-channel`. |
| [`fs4`](https://crates.io/crates/fs4) | 0.13 | MIT OR Apache-2.0 | Free space of the destination drive, checked before each extraction. Only the `sync` feature is enabled. |
| [`keyring-core`](https://crates.io/crates/keyring-core) and [`windows-native-keyring-store`](https://crates.io/crates/windows-native-keyring-store) | 1 | MIT OR Apache-2.0 | Archive passwords in the Windows Credential Manager. Default features of the store are off (no `regex`). |
| [`windows-registry`](https://crates.io/crates/windows-registry) | 0.6 | MIT OR Apache-2.0 | Safe access to the current user's registry: start with Windows (`Run` key) and the identity of the Windows notifications. Shares the `windows-*` 0.62 family already pulled by other dependencies. |
| [`tauri-winrt-notification`](https://crates.io/crates/tauri-winrt-notification) | 0.8 | MIT OR Apache-2.0 | Windows notifications (WinRT toasts). Uses the `windows` 0.62 crate already in the tree; adds only `windows-version`. |

## Development dependencies

Used by the tests only; not shipped in the binary.

| Crate | Version | Licence | Why |
|---|---|---|---|
| [`tempfile`](https://crates.io/crates/tempfile) | 3 | MIT OR Apache-2.0 | Temporary folders for tests, removed automatically. |
| [`egui_kittest`](https://crates.io/crates/egui_kittest) | 0.37 | MIT OR Apache-2.0 | Headless UI tests through the accessibility tree, and off-screen rendering of the documentation screenshots. |

## External programs

| Program | Licence | Why |
|---|---|---|
| `par2.exe` ([par2cmdline-turbo](https://github.com/animetosho/par2cmdline-turbo) or par2cmdline) | GPL-2.0 | PAR2 verification and repair. par2cmdline-turbo 1.5.0 ships unmodified in the release zip (`par2.exe`, with its licence in `par2-COPYING.txt`); its exact source archive is attached to each release. It runs as a separate process, never linked: found at the path set in the settings, next to the executable, or in `PATH`. Version and SHA-256 are pinned in `scripts/fetch-par2.ps1`. |

## Development tools

| Tool | Why |
|---|---|
| `rustfmt`, `clippy` | Formatting and lints (`clippy::pedantic`, `unwrap_used`, `expect_used`, `panic` denied). |
| [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) | Licences, advisories, banned sources. |
| WinRAR `Rar.exe` | Only to regenerate the test archives (`tests/fixtures/make-fixtures.ps1`). |
| [WiX Toolset](https://github.com/wixtoolset/wix) 5.0.2 (MS-RL) | Builds the installer (`.msi`); not shipped. Version and SHA-256 are pinned in `scripts/fetch-wix.ps1`; it runs on an installed .NET runtime. WiX 5 is kept on purpose: WiX 6 and later ask projects that earn money for a maintenance fee. |
