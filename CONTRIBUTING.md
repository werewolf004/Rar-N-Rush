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
| `tests/` | Integration and UI tests; `tests/fixtures/` holds real RAR archives |
| `vendor/unrar/` | Patched `unrar` crate, see `PATCHES.md` |

## Tests

- `tests/fixtures/*.rar` are real archives made with WinRAR by `tests/fixtures/make-fixtures.ps1`. They are committed, so CI does not need WinRAR.
- PAR2 behaviour is tested with small batch scripts that mimic `par2` output and exit codes (`tests/common/mod.rs`).
- UI tests drive the real application through its accessibility tree with `egui_kittest`. No window is opened.

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
4. The **Release** workflow checks that the tag matches `Cargo.toml`, runs the tests, builds the binary and publishes the release with the changelog section as notes.

## Code of conduct

This project follows the [Code of Conduct](CODE_OF_CONDUCT.md).
