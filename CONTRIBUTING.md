# Contributing

**English** · [Français](CONTRIBUTING.fr.md)

Thank you for your interest in Rar N Rush. This guide explains how to build, test and submit changes.

## Ground rules

- Code, identifiers, comments and commit messages are in **English**. Every documentation file exists in English and French (`FILE.md` and `FILE.fr.md`): update both.
- Every user-facing text goes through [`src/i18n.rs`](src/i18n.rs), in both languages. No hard-coded label in the UI.
- No `unsafe` (forbidden by the compiler). No `unwrap`, `expect` or `panic` outside tests: return a typed error (`thiserror`) instead.
- Every change comes with tests: unit tests next to the code, integration tests in `tests/`, UI tests in `tests/ui.rs`.
- A user-visible change adds a line under `## [Unreleased]` in `CHANGELOG.md` **and** `CHANGELOG.fr.md`.
- A new or changed setting is documented in `docs/OPTIONS.md` and `docs/OPTIONS.fr.md`.
- A new dependency is justified in `docs/DEPENDENCIES.md` / `.fr.md`, must pass `cargo deny`, and `docs/THIRD-PARTY-LICENSES.md` must be regenerated.

## Setup

- Windows 10/11, Rust ≥ 1.95 (`rustup` installs the right toolchain from `rust-toolchain.toml`).
- Visual Studio C++ build tools (MSVC), needed to compile UnRAR.
- Optional: [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) (`cargo install cargo-deny`), Python 3 for `scripts/third-party.py`, WinRAR to regenerate test archives.

## Before every commit

These are the checks run by CI; run them locally first:

```bash
cargo fmt --all
```

```bash
cargo clippy --all-targets --locked -- -D warnings
```

```bash
cargo test --all-targets --locked
```

```bash
cargo deny check
```

## Project layout

| Path | Content |
|---|---|
| `src/scan.rs` | Groups files into sets (PAR2, SFV, RAR volumes) |
| `src/par2.rs` | Runs `par2.exe`, parses its progress |
| `src/sfv.rs` | CRC32 verification |
| `src/extract.rs` | RAR extraction, passwords, overwrite policy, path safety |
| `src/engine.rs` | Background worker: scan loop, queue, processing |
| `src/settings.rs` | Settings and persistent state (atomic writes) |
| `src/i18n.rs` | Every user-facing text, English and French |
| `src/ui.rs` | egui interface |
| `src/secrets.rs` | Archive passwords in the Windows Credential Manager |
| `src/tidy.rs` | Moving extra files, removing empty folders |
| `src/tray.rs`, `src/icon.rs` | Notification-area icon, application icon |
| `src/instance.rs`, `src/autostart.rs`, `src/notify.rs` | One instance at a time, start with Windows, Windows notifications |
| `src/uninstall.rs` | Registry clean-up run by the installer on removal |
| `installer/rar-n-rush.wxs`, `scripts/*-msi.ps1`, `scripts/fetch-wix.ps1` | Windows Installer package (WiX 5): source, build, end-to-end test |
| `packaging/winget/`, `scripts/winget-manifest.ps1` | winget manifests of each release, and their generator |
| `fuzz/` | Fuzz targets (`cargo fuzz`), outside the main build |
| `tests/` | Integration and UI tests; `tests/fixtures/` holds real RAR archives |
| `vendor/unrar/` | Patched `unrar` crate, see `PATCHES.md` |

## Tests

- `tests/fixtures/*.rar` are real archives made with WinRAR by `tests/fixtures/make-fixtures.ps1`. They are committed, so CI does not need WinRAR.
- PAR2 behaviour is tested with small batch scripts that mimic `par2` output and exit codes (`tests/common/mod.rs`), and end to end with the real par2cmdline-turbo (`tests/par2_real.rs`): damaged volume, missing volume, not enough blocks, and a set that waits for its recovery volumes. CI always runs them; locally:

  ```powershell
  pwsh scripts/fetch-par2.ps1   # pinned version, SHA-256 checked -> target/par2/par2.exe
  $env:RAR_N_RUSH_PAR2 = (Resolve-Path target/par2/par2.exe)
  cargo test --test par2_real
  ```

  Without `RAR_N_RUSH_PAR2` these tests are skipped locally, and fail in CI.
- The installer is tested end to end by `scripts/test-msi.ps1` (CI runs it on every push): silent install, upgrade, removal, checking the files, the Start menu shortcut and the registry after each step. It needs a .NET runtime (6 or later) for WiX, which `scripts/fetch-wix.ps1` downloads (pinned, SHA-256 checked). It installs for the current user, so it refuses to run if Rar N Rush is already installed or registered on that account, and removes everything it created at the end:

  ```powershell
  pwsh scripts/fetch-par2.ps1
  cargo build --release --locked
  pwsh scripts/test-msi.ps1     # or: pwsh scripts/build-msi.ps1 -> target/msi/*.msi
  ```
- UI tests drive the real application through its accessibility tree with `egui_kittest`. No window is opened.
- Tests never show a Windows notification and never touch the real autostart entry: notifications go through a fake `Notifier`, and the registry tests use throw-away keys under `HKEY_CURRENT_USER\Software\RarNRush-test-<pid>-…`, removed at the end. `tests/single_instance.rs` starts the real program while the lock is held and checks that it exits at once, without a window.
- Pixel snapshot tests (`snapshots` in `tests/ui.rs`) compare the main screens with `tests/snapshots/*.png`: main window (English light and dark, French), the three settings pages, the welcome screen. They are rendered by Windows' software rasterizer (WARP), so they need no GPU and run in CI. Times are frozen so every run gives the same image; the demo lives in `C:\Users\Public\RarNRush-snapshots`. They also check that every widget has an accessible name. After an intended visual change, look at the `*.diff.png` files (CI uploads them as the `snapshot-diffs` artifact), then update the images and commit them:

  ```powershell
  $env:UPDATE_SNAPSHOTS = 1; cargo test --test ui snapshots; Remove-Item Env:UPDATE_SNAPSHOTS
  ```

- `tests/properties.rs` checks properties of the parsers of untrusted input (PAR2, SFV, RAR names, entry paths, folder names) on thousands of generated inputs with `proptest`.
- One test writes to the real Windows Credential Manager, under a unique `test-<pid>-` prefix, and removes what it wrote.

### Fuzzing

The same parsers are fuzzed with `cargo fuzz` (nightly). On Windows, libFuzzer needs AddressSanitizer, whose DLL ships with the MSVC build tools:

```powershell
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --locked
$env:PATH = "<VS>\VC\Tools\MSVC\<version>\bin\Hostx64\x64;$env:PATH"   # clang_rt.asan_dynamic-x86_64.dll
cargo +nightly fuzz run par2_names -- -max_total_time=60
```

Targets: `par2_names`, `sfv_text`, `names`. The **Fuzz** workflow runs each of them every week and on demand.

### Screenshots

Documentation screenshots are rendered off-screen (no window appears):

```bash
UPDATE_SCREENSHOTS=1 cargo test --test ui screenshots -- --test-threads=1
```

They are written to `docs/screenshots/`. The demo data lives under `C:\Users\Public\RarNRush-demo` so no personal path ever appears in an image.

## Commits and pull requests

- One topic per pull request; describe what changes for the user and how it was tested.
- Commit messages: imperative mood, short first line (`Add free-space check before extraction`).
- CI must be green before merge.

## Releases

1. Move the `[Unreleased]` entries to a new `## [X.Y.Z] - YYYY-MM-DD` section in both changelogs and update the links at the bottom.
2. Set `version = "X.Y.Z"` in `Cargo.toml`, run `cargo build` to update `Cargo.lock`.
3. Commit, then tag: `git tag -a vX.Y.Z -m "vX.Y.Z"` and push the tag.
4. The **Release** workflow checks that the tag matches `Cargo.toml`, runs the tests, builds the binary, tests and builds the installer, and publishes the zip, the installer, their SHA-256 files and the provenance attestation, with the changelog section as notes.
5. winget: download the published installer, write the manifests, check them, and open a pull request to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) with the new folder (same path under `manifests/`):

   ```powershell
   gh release download vX.Y.Z -p "*.msi" -D target/winget-dl
   powershell -File scripts/winget-manifest.ps1 -Version X.Y.Z -Msi target/winget-dl/rar-n-rush-vX.Y.Z-windows-x64.msi
   winget validate --manifest packaging/winget/manifests/w/werewolf004/RarNRush/X.Y.Z
   ```

## Code of conduct

This project follows the [Code of Conduct](CODE_OF_CONDUCT.md).
