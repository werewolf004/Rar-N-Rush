//! Integration tests for the par2 runner, using fake `par2` batch scripts
//! that reproduce the real program's output and exit codes.
#![cfg(windows)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use rar_n_rush::par2::{self, Outcome, Par2Error, Phase};
use std::fs;
use std::sync::atomic::AtomicBool;

/// Result of a fake run: outcome, progress events and logged lines.
type RunResult = (Result<Outcome, Par2Error>, Vec<(Phase, f32)>, Vec<String>);

fn run(lines: &[&str], stderr: Option<&str>, code: i32) -> RunResult {
    let dir = tempfile::tempdir().unwrap();
    let exe = common::fake_par2(dir.path(), "par2.cmd", lines, stderr, code);
    let index = dir.path().join("set.par2");
    fs::write(&index, b"").unwrap();
    let mut progress = Vec::new();
    let mut logs = Vec::new();
    let result = par2::verify_and_repair(
        &exe,
        &index,
        &AtomicBool::new(false),
        |phase, p| progress.push((phase, p)),
        |line| logs.push(line),
    );
    (result, progress, logs)
}

#[test]
fn all_files_correct() {
    let (result, progress, _) = run(
        &[
            "Loading: 100.0%%",
            "All files are correct, repair is not required.",
        ],
        None,
        0,
    );
    assert_eq!(result.unwrap(), Outcome::AllCorrect);
    assert_eq!(progress, vec![(Phase::Verify, 1.0)]);
}

#[test]
fn repaired_set() {
    let (result, progress, logs) = run(
        &[
            "Target: \"a.rar\" - damaged. Found 9 of 10 data blocks.",
            "Repair is required.",
            "Repairing: 50.0%%",
            "Repair complete.",
        ],
        None,
        0,
    );
    assert_eq!(result.unwrap(), Outcome::Repaired);
    assert_eq!(progress, vec![(Phase::Repair, 0.5)]);
    assert_eq!(logs.len(), 1, "damaged target line is logged: {logs:?}");
}

#[test]
fn repair_not_possible() {
    let (result, _, _) = run(
        &[
            "You need 3 more recovery blocks to be able to repair.",
            "Repair is not possible.",
        ],
        None,
        2,
    );
    match result.unwrap() {
        Outcome::NotPossible { details } => assert!(details.contains("You need 3 more"), "{details}"),
        other => panic!("unexpected outcome {other:?}"),
    }
}

#[test]
fn unexpected_exit_code_is_a_typed_error() {
    let (result, _, _) = run(&[], Some("Main packet not found."), 3);
    match result.unwrap_err() {
        Par2Error::Failed { code, message } => {
            assert_eq!(code, Some(3));
            assert_eq!(message, "Main packet not found.");
        }
        other => panic!("unexpected error {other:?}"),
    }
}

#[test]
fn missing_executable_is_a_spawn_error() {
    let dir = tempfile::tempdir().unwrap();
    let result = par2::verify_and_repair(
        &dir.path().join("does-not-exist.exe"),
        &dir.path().join("x.par2"),
        &AtomicBool::new(false),
        |_, _| {},
        |_| {},
    );
    assert!(matches!(result, Err(Par2Error::Spawn { .. })));
}

#[test]
fn cancel_kills_the_process() {
    let dir = tempfile::tempdir().unwrap();
    let exe = common::fake_par2(dir.path(), "slow.cmd", &["Loading: 1.0%%"], None, 0);
    // Make the script wait ~5 s so cancellation has something to interrupt.
    let mut script = fs::read_to_string(&exe).unwrap();
    script = script.replace("exit /b 0", "ping -n 6 127.0.0.1 >nul\r\nexit /b 0");
    fs::write(&exe, script).unwrap();
    let index = dir.path().join("set.par2");
    fs::write(&index, b"").unwrap();
    let started = std::time::Instant::now();
    let result = par2::verify_and_repair(&exe, &index, &AtomicBool::new(true), |_, _| {}, |_| {});
    assert_eq!(result.unwrap(), Outcome::Cancelled);
    assert!(
        started.elapsed().as_secs() < 4,
        "cancel took {:?}",
        started.elapsed()
    );
}
