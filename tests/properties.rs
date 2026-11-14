//! Property tests: the parsers of untrusted input (PAR2, SFV, RAR names, archive
//! entry paths, folder names) never panic on arbitrary data and give back what a
//! well-formed input contains.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use proptest::prelude::*;
use rar_n_rush::engine::sanitize_folder_name;
use rar_n_rush::extract::is_safe_relative;
use rar_n_rush::i18n::{Lang, human_size};
use rar_n_rush::scan::{par2_target_names_from, parse_sfv_text, rar_first_volumes, rar_part_info};
use std::collections::HashSet;
use std::fmt::Write as _;
use std::io::Cursor;
use std::path::Path;

/// File names as found in downloads: letters, digits, spaces and punctuation,
/// without path separators or line breaks.
fn file_name() -> impl Strategy<Value = String> {
    "[A-Za-z0-9][A-Za-z0-9 ._()\\[\\]é-]{0,40}[A-Za-z0-9)]"
}

/// A `FileDesc` packet as written by par2 (the hashes are not checked by the parser).
fn filedesc(name: &str) -> Vec<u8> {
    let mut body = vec![0u8; 56];
    body.extend_from_slice(name.as_bytes());
    while !body.len().is_multiple_of(4) {
        body.push(0);
    }
    common::par2_packet(b"PAR 2.0\0FileDesc", &body)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn par2_parser_survives_any_bytes(data in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let _ = par2_target_names_from(Cursor::new(data));
    }

    #[test]
    fn par2_parser_survives_corrupted_packets(
        names in proptest::collection::vec(file_name(), 1..6),
        flips in proptest::collection::vec((any::<usize>(), any::<u8>()), 0..8),
    ) {
        let mut data: Vec<u8> = names.iter().flat_map(|n| filedesc(n)).collect();
        for (pos, byte) in flips {
            let len = data.len();
            if let Some(b) = data.get_mut(pos % len) {
                *b = byte;
            }
        }
        let _ = par2_target_names_from(Cursor::new(data));
    }

    #[test]
    fn par2_names_round_trip(names in proptest::collection::vec(file_name(), 0..8)) {
        let data: Vec<u8> = names.iter().flat_map(|n| filedesc(n)).collect();
        let mut seen = HashSet::new();
        let expected: Vec<String> =
            names.iter().filter(|n| seen.insert(n.to_lowercase())).cloned().collect();
        prop_assert_eq!(par2_target_names_from(Cursor::new(data)), expected);
    }

    #[test]
    fn sfv_parser_survives_any_text(text in any::<String>()) {
        let _ = parse_sfv_text(&text);
    }

    #[test]
    fn sfv_round_trip(entries in proptest::collection::vec((file_name(), any::<u32>()), 0..20)) {
        let mut text = String::from("; comment\r\n");
        for (name, crc) in &entries {
            write!(text, "{name} {crc:08X}\r\n").unwrap();
        }
        let expected: Vec<(String, u32)> =
            entries.iter().map(|(n, c)| (n.trim().to_owned(), *c)).collect();
        prop_assert_eq!(parse_sfv_text(&text), expected);
    }

    #[test]
    fn rar_names_never_panic(name in any::<String>()) {
        let _ = rar_part_info(&name);
        let _ = rar_first_volumes([name.as_str()]);
    }

    #[test]
    fn part_volumes_are_ranked(base in file_name(), part in 1u32..1000, width in 1usize..4) {
        let name = format!("{base}.part{part:0width$}.rar");
        prop_assert_eq!(rar_part_info(&name), Some((base.to_lowercase(), part)));
    }

    #[test]
    fn first_volume_is_the_lowest_part(base in file_name(), count in 1u32..30, rotate in 0usize..30) {
        let mut names: Vec<String> = (1..=count).map(|i| format!("{base}.part{i:02}.rar")).collect();
        let len = names.len();
        names.rotate_left(rotate % len);
        let firsts = rar_first_volumes(names.iter().map(String::as_str));
        prop_assert_eq!(firsts, vec![(base.to_lowercase(), format!("{base}.part01.rar"))]);
    }

    #[test]
    fn unsafe_entry_paths_are_refused(prefix in "[a-z]{0,8}", suffix in "[a-z]{0,8}") {
        for path in [
            format!("{prefix}/../{suffix}"),
            format!("..\\{suffix}"),
            format!("C:\\{prefix}"),
            format!("\\{prefix}"),
            format!("/{prefix}"),
        ] {
            prop_assert!(!is_safe_relative(Path::new(&path)), "accepted {path:?}");
        }
    }

    #[test]
    fn entry_path_check_survives_any_text(path in any::<String>()) {
        let _ = is_safe_relative(Path::new(&path));
    }

    #[test]
    fn folder_names_are_always_valid_on_windows(name in any::<String>()) {
        let folder = sanitize_folder_name(&name);
        prop_assert!(!folder.is_empty());
        prop_assert!(!folder.chars().any(|c| "<>:\"/\\|?*".contains(c) || c.is_control()), "{folder:?}");
        prop_assert!(!folder.ends_with(['.', ' ']), "{folder:?}");
        let stem = folder.split('.').next().unwrap_or_default().trim_end();
        let reserved = ["CON", "PRN", "AUX", "NUL"]
            .into_iter()
            .map(str::to_owned)
            .chain((1..=9).flat_map(|n| [format!("COM{n}"), format!("LPT{n}")]));
        for r in reserved {
            prop_assert!(!stem.eq_ignore_ascii_case(&r), "reserved name {folder:?}");
        }
    }

    /// Windows also reserves these names with an extension (`CON.txt`, `nul.tar`).
    #[test]
    fn reserved_names_are_escaped(
        name in "(?i)(con|prn|aux|nul|com[1-9]|lpt[1-9])(\\.[a-z0-9]{1,4}){0,2} {0,2}"
    ) {
        let folder = sanitize_folder_name(&name);
        let stem = folder.split('.').next().unwrap_or_default().trim_end();
        prop_assert!(stem.starts_with('_'), "{name:?} gave {folder:?}");
    }

    #[test]
    fn sizes_always_format(bytes in any::<u64>()) {
        prop_assert!(!human_size(bytes, Lang::En).is_empty());
        prop_assert!(!human_size(bytes, Lang::Fr).is_empty());
    }
}
