//! Integration tests for RAR extraction against real archives made with `WinRAR`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use rar_n_rush::extract::{self, ArchiveProblem, ExtractError, Options, Outcome};
use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;

fn run(first: &Path, dest: &Path, passwords: &[&str], options: Options) -> Result<Outcome, ExtractError> {
    let passwords: Vec<String> = passwords.iter().map(|p| (*p).to_owned()).collect();
    extract::extract(first, dest, &passwords, options, &AtomicBool::new(false), |_| {})
}

#[test]
fn multi_volume_archive_is_extracted_intact() {
    let dir = tempfile::tempdir().unwrap();
    common::copy_fixtures(
        dir.path(),
        &["multi.part1.rar", "multi.part2.rar", "multi.part3.rar"],
    );
    let dest = dir.path().join("out");
    let outcome = run(
        &dir.path().join("multi.part1.rar"),
        &dest,
        &[],
        Options::default(),
    )
    .unwrap();
    assert_eq!(
        outcome,
        Outcome::Extracted {
            files: vec![dest.join("payload.bin")],
            kept: vec![]
        }
    );
    assert_eq!(fs::read(dest.join("payload.bin")).unwrap(), common::payload());
}

#[test]
fn missing_volume_is_a_typed_error() {
    let dir = tempfile::tempdir().unwrap();
    common::copy_fixtures(dir.path(), &["multi.part1.rar", "multi.part3.rar"]);
    let err = run(
        &dir.path().join("multi.part1.rar"),
        &dir.path().join("out"),
        &[],
        Options::default(),
    )
    .unwrap_err();
    assert!(matches!(err, ExtractError::Archive { .. }), "{err:?}");
}

#[test]
fn passwords_are_tried_in_order() {
    let dir = tempfile::tempdir().unwrap();
    common::copy_fixtures(dir.path(), &["password.rar"]);
    let dest = dir.path().join("out");
    let outcome = run(
        &dir.path().join("password.rar"),
        &dest,
        &["wrong", "", "abc"],
        Options::default(),
    )
    .unwrap();
    assert_eq!(
        outcome,
        Outcome::Extracted {
            files: vec![dest.join("secret.txt")],
            kept: vec![]
        }
    );
    assert_eq!(fs::read_to_string(dest.join("secret.txt")).unwrap(), "secret");
}

#[test]
fn unknown_password_leaves_no_partial_file() {
    let dir = tempfile::tempdir().unwrap();
    common::copy_fixtures(dir.path(), &["password.rar"]);
    let dest = dir.path().join("out");
    assert_eq!(
        run(
            &dir.path().join("password.rar"),
            &dest,
            &["nope"],
            Options::default()
        )
        .unwrap(),
        Outcome::NeedPassword
    );
    assert!(!dest.join("secret.txt").exists());
    assert_eq!(
        run(&dir.path().join("password.rar"), &dest, &[], Options::default()).unwrap(),
        Outcome::NeedPassword
    );
}

#[test]
fn encrypted_headers_need_the_password() {
    let dir = tempfile::tempdir().unwrap();
    common::copy_fixtures(dir.path(), &["headers.rar"]);
    let first = dir.path().join("headers.rar");
    let dest = dir.path().join("out");
    assert_eq!(
        run(&first, &dest, &["bad"], Options::default()).unwrap(),
        Outcome::NeedPassword
    );
    assert_eq!(
        run(&first, &dest, &["bad", "abc"], Options::default()).unwrap(),
        Outcome::Extracted {
            files: vec![dest.join("secret.txt")],
            kept: vec![]
        }
    );
}

#[test]
fn existing_files_are_kept_unless_overwrite_is_enabled() {
    let dir = tempfile::tempdir().unwrap();
    common::copy_fixtures(dir.path(), &["plain.rar"]);
    let first = dir.path().join("plain.rar");
    let dest = dir.path().join("out");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("hello.txt"), "precious").unwrap();

    let outcome = run(
        &first,
        &dest,
        &[],
        Options {
            overwrite_existing: false,
        },
    )
    .unwrap();
    // Kept as it is, and no second copy such as "hello (1).txt".
    assert_eq!(
        outcome,
        Outcome::Extracted {
            files: vec![],
            kept: vec![dest.join("hello.txt")]
        }
    );
    assert_eq!(fs::read_to_string(dest.join("hello.txt")).unwrap(), "precious");
    assert_eq!(fs::read_dir(&dest).unwrap().count(), 1, "nothing else written");

    run(
        &first,
        &dest,
        &[],
        Options {
            overwrite_existing: true,
        },
    )
    .unwrap();
    assert_eq!(fs::read_to_string(dest.join("hello.txt")).unwrap(), "hello");
}

#[test]
fn corrupt_data_is_reported_and_partial_files_removed() {
    let dir = tempfile::tempdir().unwrap();
    common::copy_fixtures(dir.path(), &["corrupt.rar"]);
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("mine.txt"), b"user file").unwrap();
    let err = run(&dir.path().join("corrupt.rar"), &out, &[], Options::default()).unwrap_err();
    match err {
        ExtractError::Archive { problem, .. } => assert_eq!(problem, ArchiveProblem::Corrupt),
        other => panic!("unexpected error: {other:?}"),
    }
    // A retry must start clean: nothing written by the failed run is left behind,
    // and what was already there is untouched.
    let left: Vec<_> = fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, vec![std::ffi::OsString::from("mine.txt")]);
}

#[test]
fn not_an_archive_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("fake.rar");
    fs::write(&fake, b"this is not a rar archive").unwrap();
    assert!(run(&fake, &dir.path().join("out"), &[], Options::default()).is_err());
}

#[test]
fn cancel_flag_stops_extraction() {
    let dir = tempfile::tempdir().unwrap();
    common::copy_fixtures(dir.path(), &["plain.rar"]);
    let cancel = AtomicBool::new(true);
    let outcome = extract::extract(
        &dir.path().join("plain.rar"),
        &dir.path().join("out"),
        &[],
        Options::default(),
        &cancel,
        |_| {},
    )
    .unwrap();
    assert_eq!(outcome, Outcome::Cancelled);
}
