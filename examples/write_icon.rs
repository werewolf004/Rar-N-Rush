//! Writes the application icon as a `.ico` file, for the installer package:
//! `cargo run --release --example write_icon -- target/msi/rar-n-rush.ico`.

// A build tool: its errors go to the console of the build script.
#![allow(clippy::print_stderr)]

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: write_icon <file.ico>");
        return ExitCode::FAILURE;
    };
    match std::fs::write(&path, rar_n_rush::icon::ico(64)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            ExitCode::FAILURE
        }
    }
}
