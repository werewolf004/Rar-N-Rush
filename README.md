# Rar N Rush

**English** · [Français](README.fr.md)

[![CI](https://github.com/werewolf004/Rar-N-Rush/actions/workflows/ci.yml/badge.svg)](https://github.com/werewolf004/Rar-N-Rush/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-lightgrey.svg)
![MSRV 1.95](https://img.shields.io/badge/MSRV-1.95-orange.svg)

Rar N Rush watches your download folder. For each set of files, it verifies the PAR2 or SFV data, repairs damaged files when needed, then extracts the RAR archives automatically.

Written in Rust with an [egui](https://github.com/emilk/egui) interface. The concept is inspired by the Par-N-Rar application, but the code is an independent rewrite.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/main-en-dark.png">
  <img alt="Main window: four download sets with their status, progress and detail, and the log below" src="docs/screenshots/main-en-light.png">
</picture>

## Features

- **Folder monitoring**: the watched folder (and optionally its sub-folders) is scanned at a regular interval. A set is only processed once its files have stopped changing, so downloads in progress are never touched.
- **PAR2**: verification and repair through `par2.exe` ([par2cmdline-turbo](https://github.com/animetosho/par2cmdline-turbo) recommended). When there are not enough recovery blocks, the set waits and is retried as soon as new files arrive.
- **SFV**: native CRC32 verification.
- **RAR**: extraction with the official UnRAR library. Supports multi-volume archives (`.partN.rar`, `.rar` + `.r00`), passwords (a list tried automatically, or typed in the window), encrypted headers and archives nested inside archives.
- **Data safety**: existing files are never overwritten by default (`name (1).ext` is used instead). Paths that would escape the destination folder are refused. Source files can be sent to the Recycle Bin, never deleted permanently.
- **Interface**: English or French (follows Windows by default); light, dark or automatic theme following Windows live; optional icon in the notification area (minimise or close to tray); the list follows the set being processed.
- **Tidy-up** (optional): the other files of a set (`.nfo`, samples…) follow it to the destination, and folders left empty go to the Recycle Bin.
- **Reliable settings**: portable file next to the executable, found again even if you move it; invalid values are reset one by one (never the whole file) after a backup copy, and you are told about it.
- **Offline**: no update check, no telemetry, no network access at all. Updating is your choice: download a new release when you want one.

## Installation

1. Download `rar-n-rush-vX.Y.Z-windows-x64.zip` from the [Releases](https://github.com/werewolf004/Rar-N-Rush/releases) page.
2. Check the archive with the `.sha256` file published next to it:
   ```powershell
   Get-FileHash .\rar-n-rush-vX.Y.Z-windows-x64.zip -Algorithm SHA256
   ```
3. Unzip it anywhere and run `rar-n-rush.exe`.
4. For PAR2 repair, place `par2.exe` next to `rar-n-rush.exe`, add it to your `PATH`, or set its path in the settings.

Requirements: Windows 10 or 11 (64-bit).

## Getting started

On first launch, choose the folder where your downloads arrive:

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/onboarding-en.png">
  <img alt="First launch: a button invites you to choose the folder to watch" src="docs/screenshots/onboarding-en.png">
</picture>

Monitoring starts right away. You can switch to another folder at any time, even while sets are being processed, from the **Watched folder** bar at the top of the window: type a path and press Enter (Esc cancels), pick one under **Recent folders**, or click **Browse…**. **Open** shows the folder in File Explorer.

Each set of files appears in the list with its status:

| Status | Meaning |
|---|---|
| Waiting | Files are still arriving. |
| Ready | Complete, waiting for its turn. |
| Verifying / Repairing / Extracting | Being processed. |
| Password required | Type the password in the row and press Enter. |
| Repair impossible | Not enough PAR2 blocks: the set waits for more files. |
| Done | Extracted successfully. |
| Error | See the Detail column and the log; **Retry** re-runs the set. |
| Skipped | Ignored by you; **Retry** processes it anyway. |

Open **Settings** to change the folders, timings, language, theme and behaviour. Every option is described in [docs/OPTIONS.md](docs/OPTIONS.md).

| Shortcut | Action |
|---|---|
| **F5** | Scan the watched folder now |
| **Ctrl+,** | Open the settings |
| **Esc** | Close the settings (asks first if something changed); in the watched-folder field, cancel the edit |
| **Enter** | In the watched-folder field, watch the folder typed; in a password field, try the password |

The window reopens at the size and position it had when you closed it.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/settings-en-dark.png">
  <img alt="Settings window" src="docs/screenshots/settings-en-light.png">
</picture>

## Documentation

| Document | Content |
|---|---|
| [docs/OPTIONS.md](docs/OPTIONS.md) | Every setting: name, default, range, effect |
| [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md) | Every dependency and why it is used |
| [docs/THIRD-PARTY-LICENSES.md](docs/THIRD-PARTY-LICENSES.md) | All 220 crates in the binary, with licences |
| [CHANGELOG.md](CHANGELOG.md) | Changes per release |
| [ROADMAP.md](ROADMAP.md) | What comes next |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to build, test and contribute |
| [SECURITY.md](SECURITY.md) | Security model and how to report a vulnerability |

## Building from source

Requirements: Rust 1.95 or later and the Visual Studio C++ build tools (MSVC), used to compile UnRAR.

```bash
cargo build --release
```

The executable is written to `target/release/rar-n-rush.exe`. See [CONTRIBUTING.md](CONTRIBUTING.md) for tests and checks.

## License

Rar N Rush is distributed under the terms of either the [MIT license](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE), at your option.

The embedded UnRAR library is distributed under the UnRAR licence: it allows extracting archives but forbids using its code to re-create the RAR compression algorithm. See [docs/THIRD-PARTY-LICENSES.md](docs/THIRD-PARTY-LICENSES.md).
