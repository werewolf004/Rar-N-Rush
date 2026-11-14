//! End-to-end tests with a real `par2` executable (par2cmdline or
//! par2cmdline-turbo): recovery files are created, data is damaged or
//! deleted, then Rar N Rush verifies and repairs it.
//!
//! The executable comes from the `RAR_N_RUSH_PAR2` environment variable.
//! Without it the tests are skipped locally, but they fail in CI (`CI=true`)
//! so that a broken install step cannot silently turn them off.
#![cfg(windows)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stderr
)]

mod common;

use rar_n_rush::engine::{Engine, Notice, Status};
use rar_n_rush::par2::{self, Outcome};
use rar_n_rush::settings::{Settings, Store};
use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

const VOLUMES: [&str; 3] = ["multi.part1.rar", "multi.part2.rar", "multi.part3.rar"];
const TIMEOUT: Duration = Duration::from_secs(120);

/// The real `par2`, or `None` (test skipped) outside CI.
fn par2_exe() -> Option<PathBuf> {
    match std::env::var_os("RAR_N_RUSH_PAR2") {
        Some(p) if !p.is_empty() => {
            let p = PathBuf::from(p);
            assert!(
                p.is_file(),
                "RAR_N_RUSH_PAR2 points to a missing file: {}",
                p.display()
            );
            Some(p)
        }
        _ => {
            assert!(
                std::env::var("CI").as_deref() != Ok("true"),
                "RAR_N_RUSH_PAR2 must be set in CI"
            );
            eprintln!("skipped: set RAR_N_RUSH_PAR2 to a par2 executable to run this test");
            None
        }
    }
}

/// Runs `par2 create` in `dir`: `-b` source blocks, `-c` recovery blocks.
fn create(exe: &Path, dir: &Path, index: &str, files: &[&str], blocks: u32, recovery: u32) {
    let out = Command::new(exe)
        .current_dir(dir)
        .args(["c", "-q", "-q"])
        .arg(format!("-b{blocks}"))
        .arg(format!("-c{recovery}"))
        .arg("--")
        .arg(index)
        .args(files)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "par2 create failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Overwrites `len` bytes at `offset` with a pattern that differs from the data.
fn damage(path: &Path, offset: u64, len: usize) {
    let original = fs::read(path).unwrap();
    let mut f = fs::OpenOptions::new().write(true).open(path).unwrap();
    f.seek(SeekFrom::Start(offset)).unwrap();
    let start = usize::try_from(offset).unwrap();
    let bytes: Vec<u8> = original[start..start + len].iter().map(|b| !b).collect();
    f.write_all(&bytes).unwrap();
}

fn run(exe: &Path, index: &Path) -> Outcome {
    par2::verify_and_repair(exe, index, &AtomicBool::new(false), |_, _| {}, |_| {}).unwrap()
}

/// Folder holding the three RAR volumes.
fn volumes() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    common::copy_fixtures(dir.path(), &VOLUMES);
    dir
}

#[test]
fn intact_set_is_all_correct() {
    let Some(exe) = par2_exe() else { return };
    let dir = volumes();
    create(&exe, dir.path(), "multi.par2", &VOLUMES, 100, 10);
    assert_eq!(run(&exe, &dir.path().join("multi.par2")), Outcome::AllCorrect);
}

#[test]
fn damaged_volume_is_repaired() {
    let Some(exe) = par2_exe() else { return };
    let dir = volumes();
    create(&exe, dir.path(), "multi.par2", &VOLUMES, 100, 10);
    let part2 = dir.path().join("multi.part2.rar");
    let before = fs::read(&part2).unwrap();
    damage(&part2, 1000, 1500);
    assert_ne!(fs::read(&part2).unwrap(), before);

    assert_eq!(run(&exe, &dir.path().join("multi.par2")), Outcome::Repaired);
    assert_eq!(fs::read(&part2).unwrap(), before, "volume restored byte for byte");
}

#[test]
fn missing_volume_is_rebuilt() {
    let Some(exe) = par2_exe() else { return };
    let dir = volumes();
    create(&exe, dir.path(), "multi.par2", &VOLUMES, 100, 60);
    let part3 = dir.path().join("multi.part3.rar");
    let before = fs::read(&part3).unwrap();
    fs::remove_file(&part3).unwrap();

    assert_eq!(run(&exe, &dir.path().join("multi.par2")), Outcome::Repaired);
    assert_eq!(fs::read(&part3).unwrap(), before);
}

#[test]
fn not_enough_blocks_is_reported() {
    let Some(exe) = par2_exe() else { return };
    let dir = volumes();
    create(&exe, dir.path(), "multi.par2", &VOLUMES, 100, 2);
    damage(&dir.path().join("multi.part1.rar"), 2000, 10_000);

    match run(&exe, &dir.path().join("multi.par2")) {
        Outcome::NotPossible { details } => {
            assert!(details.to_lowercase().contains("you need"), "{details}");
        }
        other => panic!("unexpected outcome {other:?}"),
    }
}

fn status_of(engine: &Engine, name: &str) -> Option<Status> {
    engine.with_state(|s| s.jobs.iter().find(|j| j.set.name == name).map(|j| j.status))
}

fn wait_until(engine: &Engine, mut done: impl FnMut(&Engine) -> bool) {
    let start = Instant::now();
    while !done(engine) {
        assert!(
            start.elapsed() < TIMEOUT,
            "timeout; jobs: {:#?}",
            engine.with_state(|s| s.jobs.clone())
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn engine_settings(exe: &Path, watch: &Path, out: &Path) -> Settings {
    Settings {
        watch_dir: watch.display().to_string(),
        extract_dir: out.display().to_string(),
        par2_path: exe.display().to_string(),
        stable_secs: 0,
        scan_interval_secs: 2,
        ..Settings::default()
    }
}

/// The whole chain: a damaged download is repaired, then extracted.
#[test]
fn engine_repairs_then_extracts() {
    let Some(exe) = par2_exe() else { return };
    let root = tempfile::tempdir().unwrap();
    let (watch, out) = (root.path().join("watch"), root.path().join("out"));
    fs::create_dir_all(&watch).unwrap();
    common::copy_fixtures(&watch, &VOLUMES);
    create(&exe, &watch, "multi.par2", &VOLUMES, 100, 10);
    damage(&watch.join("multi.part2.rar"), 500, 2000);

    let engine = Engine::start(engine_settings(&exe, &watch, &out), Store::at(root.path()), || {}).unwrap();
    wait_until(&engine, |e| {
        matches!(status_of(e, "multi"), Some(Status::Done | Status::Failed))
    });
    let job = engine.with_state(|s| s.jobs[0].clone());
    assert_eq!(job.status, Status::Done, "{:?}", job.detail);
    assert!(
        engine.with_state(|s| s.log.iter().any(|l| matches!(l.notice, Notice::Repaired { .. }))),
        "a repair was logged"
    );
    assert_eq!(
        fs::read(out.join("multi").join("payload.bin")).unwrap(),
        common::payload()
    );
}

/// Not enough blocks: the set waits; once the recovery volumes arrive it is
/// repaired and extracted without any user action.
#[test]
fn engine_waits_for_recovery_volumes() {
    let Some(exe) = par2_exe() else { return };
    let root = tempfile::tempdir().unwrap();
    let (watch, out, staging) = (
        root.path().join("watch"),
        root.path().join("out"),
        root.path().join("staging"),
    );
    fs::create_dir_all(&watch).unwrap();
    fs::create_dir_all(&staging).unwrap();
    common::copy_fixtures(&staging, &VOLUMES);
    create(&exe, &staging, "multi.par2", &VOLUMES, 100, 20);
    damage(&staging.join("multi.part2.rar"), 500, 2000);
    // First only the volumes and the index (no recovery blocks)...
    let mut recovery = Vec::new();
    for entry in fs::read_dir(&staging).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.contains(".vol") {
            recovery.push(path);
        } else {
            fs::copy(&path, watch.join(&name)).unwrap();
        }
    }
    assert!(!recovery.is_empty(), "par2 created recovery volumes");

    let engine = Engine::start(engine_settings(&exe, &watch, &out), Store::at(root.path()), || {}).unwrap();
    wait_until(&engine, |e| {
        status_of(e, "multi") == Some(Status::RepairImpossible)
    });

    // ...then the recovery volumes arrive.
    for path in &recovery {
        fs::copy(path, watch.join(path.file_name().unwrap())).unwrap();
    }
    wait_until(&engine, |e| {
        matches!(status_of(e, "multi"), Some(Status::Done | Status::Failed))
    });
    let job = engine.with_state(|s| s.jobs[0].clone());
    assert_eq!(job.status, Status::Done, "{:?}", job.detail);
    assert_eq!(
        fs::read(out.join("multi").join("payload.bin")).unwrap(),
        common::payload()
    );
}
