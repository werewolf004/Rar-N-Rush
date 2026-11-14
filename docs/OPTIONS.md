# Settings reference

**English** · [Français](OPTIONS.fr.md)

Every setting is available in the **Settings** window and stored in `rar-n-rush.toml`. Changes apply as soon as you click **Save**; the file is written atomically (a crash can never leave it half-written).

## Where the file is

At start-up, Rar N Rush looks for the file in this order and uses the **first one that exists**, so your configuration is found again even if you move the executable:

1. next to `rar-n-rush.exe` (portable mode);
2. in `%APPDATA%\RarNRush`.

On a first start (no file anywhere), it is created next to `rar-n-rush.exe`, or in `%APPDATA%\RarNRush` when the program folder is not writable (for example under `Program Files`).

## If the file has a problem

Your settings are never silently lost:

- a missing key takes its default value (an old or partial file keeps working); unknown keys are ignored;
- each key is read on its own: **valid values are kept**, only invalid ones are reset to their default;
- out-of-range numbers are clamped to the allowed range;
- a byte-order mark added by Notepad is accepted;
- whenever something is reset, the original file is first copied to `rar-n-rush.toml.<date-time>.bak`, and a banner at the top of the window says which keys were reset and where the copy is.

The settings window has three pages: **General** (folders, timing, language, theme), **Processing** (behaviour, clean-up, passwords) and **Window**.

![Settings window, General page](screenshots/settings-en-light.png)

## Folders *(General page)*

| Key | Window label | Type | Default | Effect |
|---|---|---|---|---|
| `watch_dir` | Watched folder | path | *(empty)* | Folder scanned for new sets. Empty means monitoring cannot start. Also editable at any time in the **Watched folder** bar of the main window: type a path and press Enter, pick one under **Recent folders**, or use **Browse…**. A folder that does not exist is refused. |
| `recent_watch_dirs` | Recent folders *(main window)* | list of paths | `[]` | Folders watched recently, most recent first, offered by **Recent folders**. Kept up to date automatically, at most 10 entries, duplicates removed (case-insensitive). |
| `extract_dir` | Destination | path | *(empty)* | Where archives are extracted. Empty means "next to the source files". When it is inside the watched folder, it is excluded from scans. |
| `par2_path` | par2.exe | path | *(empty)* | Path to `par2.exe`. Empty means automatic lookup: next to `rar-n-rush.exe`, then in `PATH`. The window shows the resolved path, or a warning if none is found. |

## Timing *(General page)*

| Key | Window label | Type | Default | Range | Effect |
|---|---|---|---|---|---|
| `scan_interval_secs` | Scan every | seconds | `15` | 2–3600 | Delay between two scans of the watched folder. **Scan** in the toolbar forces a scan. |
| `stable_secs` | Wait for files to settle | seconds | `10` | 0–3600 | A set is processed only after its files (count, size, modification time) have not changed for this long. |

## Behaviour *(Processing page)*

![Processing page](screenshots/settings-processing-en.png)

| Key | Window label | Type | Default | Effect |
|---|---|---|---|---|
| `recursive` | Also scan sub-folders | bool | `true` | Scan sub-folders, up to 8 levels deep. Symbolic links are not followed. |
| `create_subfolder` | Extract into a sub-folder named after the set | bool | `true` | Extracts `Holidays.part1.rar` into `<destination>\Holidays\`. Characters forbidden by Windows are replaced with `_`. |
| `extract_nested` | Extract RAR archives found inside an archive | bool | `true` | Extracts archives found in an extracted archive (up to 3 levels), then deletes those inner archives. |
| `overwrite_existing` | Overwrite existing files | bool | `false` | When `false`, an existing file is kept and the extracted copy is named `name (1).ext`, `name (2).ext`… |
| `delete_after_success` | Move PAR2/SFV/RAR files to the Recycle Bin after success | bool | `false` | After a successful run, sends the PAR2 files, the SFV, the RAR volumes and par2 backups (`file.1`) to the Recycle Bin. Verified non-RAR files are always kept. |
| `autostart` | Start monitoring when the app starts | bool | `true` | Starts monitoring on launch when a watched folder is set. |

## Passwords *(Processing page)*

| Key | Window label | Type | Default | Effect |
|---|---|---|---|---|
| `passwords` | Passwords to try (one per line) | list of text | `[]` | Tried in order on protected archives. A password typed in a row of the main window is tried first. |

> **Warning**: in this version, passwords are stored in plain text in `rar-n-rush.toml`. Moving them to the Windows Credential Manager is planned for v0.3.0 (see [ROADMAP.md](../ROADMAP.md)).

## After a successful run *(Processing page)*

| Key | Window label | Type | Default | Effect |
|---|---|---|---|---|
| `move_extra_files` | Move the other files of the set (.nfo, samples…) to the destination | bool | `false` | After extraction, moves the files of the set's folder that are not PAR2/SFV/RAR sources (`.nfo`, `.jpg`, verified non-RAR files…) to the destination. **Only when the set is alone in its own sub-folder** of the watched folder: a set lying directly in the watched folder, or sharing its folder with other sets, keeps its files (the log says why). Nothing is moved when the destination is the set's own folder. An existing file is kept (`name (1).ext`) unless overwriting is enabled. |
| `delete_empty_dirs` | Move folders left empty to the Recycle Bin | bool | `false` | After a successful run, sends the set's folder to the Recycle Bin if it is completely empty, then its parents while they are empty. The watched folder itself is never removed. Combine with the two options above to leave nothing behind. |

## Window *(Window page)*

![Window page](screenshots/settings-window-en.png)

| Key | Window label | Type | Default | Effect |
|---|---|---|---|---|
| `follow_active` | Keep the set being processed in view | bool | `true` | Scrolls the list to each new set being processed, once, so you can still scroll freely in between. |
| `minimize_to_tray` | Minimise to the notification area | bool | `false` | Minimising hides the window; an icon in the notification area brings it back (click, or **Show Rar N Rush** in its menu). |
| `close_to_tray` | Close to the notification area | bool | `false` | The window's ✕ hides the window instead of quitting; monitoring goes on. Quit with **Quit** in the icon's menu. If the icon cannot be created, the window closes normally and the settings window shows the reason. |

## Appearance *(General page)*

| Key | Window label | Values | Default | Effect |
|---|---|---|---|---|
| `language` | Language | `system`, `english`, `french` | `system` | Interface language. `system` follows the Windows display language (French if it starts with `fr`, English otherwise). |
| `theme` | Theme | `system`, `light`, `dark` | `system` | `system` follows the Windows light/dark setting, live. |

## Example file

```toml
watch_dir = 'D:\Downloads'
recent_watch_dirs = ['D:\Downloads', 'E:\Usenet']
extract_dir = 'D:\Extracted'
create_subfolder = true
recursive = true
par2_path = ''
delete_after_success = false
extract_nested = true
overwrite_existing = false
scan_interval_secs = 15
stable_secs = 10
passwords = []
autostart = true
move_extra_files = false
delete_empty_dirs = false
follow_active = true
minimize_to_tray = false
close_to_tray = false
language = "system"
theme = "system"
```

## Other files

| File | Content |
|---|---|
| `rar-n-rush-processed.txt` | One line per set already processed or skipped, so it is not processed again after a restart. Delete a line (or the file) to process a set again. |
