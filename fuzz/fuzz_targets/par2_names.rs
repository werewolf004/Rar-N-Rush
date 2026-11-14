//! PAR2 `FileDesc` parser on arbitrary bytes: must never panic, hang or
//! allocate without bound.
#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    let _ = rar_n_rush::scan::par2_target_names_from(Cursor::new(data));
});
