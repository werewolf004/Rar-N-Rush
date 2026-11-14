//! The real program, started while another instance holds the lock, must show
//! that instance and exit at once, without opening a window of its own.
#![cfg(windows)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rar_n_rush::instance::{self, Instance, SHOW_FILE};
use rar_n_rush::settings::SETTINGS_FILE;
use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn second_start_asks_the_first_to_show_itself_and_exits() {
    // A portable copy: a settings file next to the executable makes the program
    // use this folder, never the user's own settings.
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("rar-n-rush.exe");
    fs::copy(env!("CARGO_BIN_EXE_rar-n-rush"), &exe).unwrap();
    fs::write(dir.path().join(SETTINGS_FILE), "").unwrap();

    // This test plays the running instance.
    let Instance::First(guard) = instance::acquire(dir.path()).unwrap() else {
        panic!("the temporary folder cannot be locked by anyone else");
    };

    let mut child = Command::new(&exe).current_dir(dir.path()).spawn().unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(20) {
            let _ = child.kill();
            panic!("the second instance did not exit: it would have opened a window");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(status.success(), "{status}");
    assert!(dir.path().join(SHOW_FILE).exists(), "show request written");
    drop(guard);
}
