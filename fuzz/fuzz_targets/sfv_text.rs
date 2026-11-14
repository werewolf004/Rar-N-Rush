//! SFV parser on arbitrary bytes (decoded like a real file): must never panic.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let _ = rar_n_rush::scan::parse_sfv_text(&text);
});
