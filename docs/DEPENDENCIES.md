# Dependencies

**English** · [Français](DEPENDENCIES.fr.md)

Every direct dependency is listed below with the reason it is needed. The complete list of the 215 crates compiled into the release binary, with their licences, is generated in [THIRD-PARTY-LICENSES.md](THIRD-PARTY-LICENSES.md) (`python scripts/third-party.py`).

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

## Development dependencies

Used by the tests only; not shipped in the binary.

| Crate | Version | Licence | Why |
|---|---|---|---|
| [`tempfile`](https://crates.io/crates/tempfile) | 3 | MIT OR Apache-2.0 | Temporary folders for tests, removed automatically. |
| [`egui_kittest`](https://crates.io/crates/egui_kittest) | 0.37 | MIT OR Apache-2.0 | Headless UI tests through the accessibility tree, and off-screen rendering of the documentation screenshots. |

## External programs

| Program | Licence | Why |
|---|---|---|
| `par2.exe` ([par2cmdline-turbo](https://github.com/animetosho/par2cmdline-turbo) or par2cmdline) | GPL-2.0 | PAR2 verification and repair. Not bundled: it runs as a separate process, found next to the executable, in `PATH`, or at the path set in the settings. |

## Development tools

| Tool | Why |
|---|---|
| `rustfmt`, `clippy` | Formatting and lints (`clippy::pedantic`, `unwrap_used`, `expect_used`, `panic` denied). |
| [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) | Licences, advisories, banned sources. |
| WinRAR `Rar.exe` | Only to regenerate the test archives (`tests/fixtures/make-fixtures.ps1`). |
