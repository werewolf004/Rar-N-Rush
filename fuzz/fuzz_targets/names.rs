//! Name handling on arbitrary text: RAR volume names, archive entry paths and
//! folder names must never panic, and a sanitised folder name is always usable.
#![no_main]

use libfuzzer_sys::fuzz_target;
use std::path::Path;

fuzz_target!(|text: &str| {
    let _ = rar_n_rush::scan::rar_part_info(text);
    let _ = rar_n_rush::scan::rar_first_volumes([text]);
    let _ = rar_n_rush::extract::is_safe_relative(Path::new(text));
    let folder = rar_n_rush::engine::sanitize_folder_name(text);
    assert!(!folder.is_empty());
    assert!(!folder.chars().any(|c| r#"<>:"/\|?*"#.contains(c) || c.is_control()));
});
