//! End-to-end tests: the engine watches a folder filled with real archives.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use rar_n_rush::engine::{Command, Engine, Status};
use rar_n_rush::settings::{Settings, Store};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(60);

fn settings(watch: &Path, out: &Path) -> Settings {
    Settings {
        watch_dir: watch.display().to_string(),
        extract_dir: out.display().to_string(),
        stable_secs: 0,
        scan_interval_secs: 2,
        passwords: vec!["wrong".into(), "abc".into()],
        ..Settings::default()
    }
}

/// Polls the engine until `done` returns true or the timeout expires.
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

fn status_of(engine: &Engine, name: &str) -> Option<Status> {
    engine.with_state(|s| s.jobs.iter().find(|j| j.set.name == name).map(|j| j.status))
}

#[test]
fn processes_a_folder_of_real_archives() {
    let root = tempfile::tempdir().unwrap();
    let (watch, out, state) = (
        root.path().join("watch"),
        root.path().join("out"),
        root.path().join("state"),
    );
    for d in [&watch, &out, &state] {
        fs::create_dir_all(d).unwrap();
    }
    common::copy_fixtures(
        &watch,
        &[
            "multi.part1.rar",
            "multi.part2.rar",
            "multi.part3.rar",
            "password.rar",
            "headers.rar",
            "nested.rar",
            "corrupt.rar",
        ],
    );
    common::write_sfv(
        &watch,
        "multi.sfv",
        &["multi.part1.rar", "multi.part2.rar", "multi.part3.rar"],
    );
    fs::write(watch.join("bad.bin"), b"abc").unwrap();
    fs::write(watch.join("bad.sfv"), "bad.bin 00000000\n").unwrap();

    let engine = Engine::start(settings(&watch, &out), Store::at(&state), || {}).unwrap();
    let names = ["multi", "password", "headers", "nested", "corrupt", "bad"];
    wait_until(&engine, |e| {
        names
            .iter()
            .all(|n| matches!(status_of(e, n), Some(Status::Done | Status::Failed)))
    });

    assert_eq!(status_of(&engine, "multi"), Some(Status::Done));
    assert_eq!(status_of(&engine, "password"), Some(Status::Done));
    assert_eq!(status_of(&engine, "headers"), Some(Status::Done));
    assert_eq!(status_of(&engine, "nested"), Some(Status::Done));
    assert_eq!(status_of(&engine, "corrupt"), Some(Status::Failed));
    assert_eq!(status_of(&engine, "bad"), Some(Status::Failed));

    assert_eq!(
        fs::read(out.join("multi").join("payload.bin")).unwrap(),
        common::payload()
    );
    assert_eq!(
        fs::read_to_string(out.join("password").join("secret.txt")).unwrap(),
        "secret"
    );
    assert_eq!(
        fs::read_to_string(out.join("nested").join("sub").join("inner.txt")).unwrap(),
        "inner"
    );
    assert!(
        !out.join("nested").join("inner.rar").exists(),
        "nested archive must be removed after extraction"
    );

    // Processed sets are remembered and not processed again by a new engine.
    let processed = fs::read_to_string(state.join("rar-n-rush-processed.txt")).unwrap();
    assert_eq!(processed.lines().count(), 4, "{processed}");
    drop(engine);
    let engine = Engine::start(settings(&watch, &out), Store::at(&state), || {}).unwrap();
    wait_until(&engine, |e| e.with_state(|s| s.last_scan.is_some()));
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(status_of(&engine, "multi"), None);
    assert!(
        status_of(&engine, "corrupt").is_some(),
        "failed sets are retried after a restart"
    );
}

#[test]
fn password_typed_by_the_user() {
    let root = tempfile::tempdir().unwrap();
    let (watch, out) = (root.path().join("watch"), root.path().join("out"));
    fs::create_dir_all(&watch).unwrap();
    common::copy_fixtures(&watch, &["password.rar"]);
    let mut s = settings(&watch, &out);
    s.passwords.clear();
    let engine = Engine::start(s, Store::at(root.path()), || {}).unwrap();

    wait_until(&engine, |e| {
        status_of(e, "password") == Some(Status::NeedPassword)
    });
    let id = engine.with_state(|s| s.jobs[0].id);
    engine.send(Command::Password(id, "Wr0ng-P4ss!".into()));
    wait_until(&engine, |e| {
        e.with_state(|s| matches!(s.jobs[0].detail, rar_n_rush::engine::Detail::WrongPassword))
    });
    engine.send(Command::Password(id, "abc".into()));
    wait_until(&engine, |e| status_of(e, "password") == Some(Status::Done));
    assert_eq!(
        fs::read_to_string(out.join("password").join("secret.txt")).unwrap(),
        "secret"
    );
    // The log file tells the story, but never shows a password.
    let log = fs::read_to_string(root.path().join("rar-n-rush.log")).unwrap();
    assert!(log.contains("password required"), "{log}");
    assert!(!log.contains("Wr0ng-P4ss!"), "{log}");
}

#[cfg(windows)]
#[test]
fn par2_set_is_verified_then_extracted() {
    let root = tempfile::tempdir().unwrap();
    let (watch, out) = (root.path().join("watch"), root.path().join("out"));
    fs::create_dir_all(&watch).unwrap();
    let volumes = ["multi.part1.rar", "multi.part2.rar", "multi.part3.rar"];
    common::copy_fixtures(&watch, &volumes);
    common::write_par2(&watch.join("multi.par2"), &volumes);
    let fake = common::fake_par2(
        root.path(),
        "par2.cmd",
        &["All files are correct, repair is not required."],
        None,
        0,
    );

    let mut s = settings(&watch, &out);
    s.par2_path = fake.display().to_string();
    let engine = Engine::start(s, Store::at(root.path()), || {}).unwrap();
    wait_until(&engine, |e| {
        matches!(status_of(e, "multi"), Some(Status::Done | Status::Failed))
    });
    let job = engine.with_state(|s| s.jobs[0].clone());
    assert_eq!(job.status, Status::Done, "{:?}", job.detail);
    assert_eq!(job.set.kind(), rar_n_rush::scan::SetKind::Par2Rar);
    assert_eq!(
        fs::read(out.join("multi").join("payload.bin")).unwrap(),
        common::payload()
    );
}

#[cfg(windows)]
#[test]
fn repair_impossible_waits_for_more_files() {
    let root = tempfile::tempdir().unwrap();
    let watch = root.path().join("watch");
    fs::create_dir_all(&watch).unwrap();
    common::copy_fixtures(&watch, &["plain.rar"]);
    common::write_par2(&watch.join("plain.par2"), &["plain.rar"]);
    let fake = common::fake_par2(
        root.path(),
        "par2.cmd",
        &["You need 1 more recovery blocks.", "Repair is not possible."],
        None,
        2,
    );

    let mut s = settings(&watch, &root.path().join("out"));
    s.par2_path = fake.display().to_string();
    let engine = Engine::start(s, Store::at(root.path()), || {}).unwrap();
    wait_until(&engine, |e| {
        status_of(e, "plain") == Some(Status::RepairImpossible)
    });
    // New PAR2 volume arrives -> the set goes back to waiting and is tried again.
    fs::write(watch.join("plain.vol00+01.par2"), b"more blocks").unwrap();
    // Retry and new failure happen within one worker loop, so count log entries
    // instead of trying to observe the short-lived intermediate states.
    let attempts = |e: &Engine| {
        e.with_state(|s| {
            s.log
                .iter()
                .filter(|l| matches!(l.notice, rar_n_rush::engine::Notice::RepairImpossible { .. }))
                .count()
        })
    };
    assert_eq!(attempts(&engine), 1);
    wait_until(&engine, |e| attempts(e) >= 2);
    assert_eq!(status_of(&engine, "plain"), Some(Status::RepairImpossible));
    // Without further changes, the set is not retried in a loop.
    std::thread::sleep(Duration::from_secs(3));
    assert_eq!(attempts(&engine), 2);
}

#[test]
fn missing_par2_executable_fails_cleanly() {
    let root = tempfile::tempdir().unwrap();
    let watch = root.path().join("watch");
    fs::create_dir_all(&watch).unwrap();
    common::copy_fixtures(&watch, &["plain.rar"]);
    common::write_par2(&watch.join("plain.par2"), &["plain.rar"]);
    let mut s = settings(&watch, &root.path().join("out"));
    s.par2_path = root.path().join("absent.exe").display().to_string();
    let engine = Engine::start(s, Store::at(root.path()), || {}).unwrap();
    wait_until(&engine, |e| status_of(e, "plain") == Some(Status::Failed));
    let detail = engine.with_state(|s| s.jobs[0].detail.clone());
    assert!(matches!(
        detail,
        rar_n_rush::engine::Detail::Error(rar_n_rush::engine::JobError::Par2NotFound)
    ));
}

#[test]
fn skip_marks_the_set_and_remembers_it() {
    let root = tempfile::tempdir().unwrap();
    let watch = root.path().join("watch");
    fs::create_dir_all(&watch).unwrap();
    // Written now (not copied) so its modification time is fresh: with a one-hour
    // stability delay the set deterministically stays "waiting".
    fs::write(
        watch.join("plain.rar"),
        fs::read(common::fixtures().join("plain.rar")).unwrap(),
    )
    .unwrap();
    let out = root.path().join("out");
    let mut s = settings(&watch, &out);
    s.stable_secs = 3600;
    let engine = Engine::start(s, Store::at(root.path()), || {}).unwrap();
    wait_until(&engine, |e| status_of(e, "plain") == Some(Status::Waiting));

    let id = engine.with_state(|s| s.jobs[0].id);
    engine.send(Command::Skip(id));
    wait_until(&engine, |e| status_of(e, "plain") == Some(Status::Skipped));
    let processed = fs::read_to_string(root.path().join("rar-n-rush-processed.txt")).unwrap();
    assert_eq!(processed.lines().count(), 1);
    assert!(!out.exists(), "a skipped set must not be extracted");

    // "Retry" processes it right away, without waiting for the stability delay.
    engine.send(Command::Retry(id));
    wait_until(&engine, |e| status_of(e, "plain") == Some(Status::Done));
    assert_eq!(
        fs::read_to_string(out.join("plain").join("hello.txt")).unwrap(),
        "hello"
    );
}

#[test]
fn extra_files_follow_the_extracted_set() {
    let root = tempfile::tempdir().unwrap();
    let (watch, out, state) = (
        root.path().join("watch"),
        root.path().join("out"),
        root.path().join("state"),
    );
    let release = watch.join("Release");
    for d in [&release, &out, &state] {
        fs::create_dir_all(d).unwrap();
    }
    common::copy_fixtures(&release, &["plain.rar"]);
    fs::write(release.join("info.nfo"), b"release notes").unwrap();
    // A loose file next to other downloads must never be moved.
    fs::write(watch.join("unrelated.txt"), b"keep me").unwrap();

    let s = Settings {
        move_extra_files: true,
        delete_empty_dirs: true, // the folder still holds the archive: it must stay
        ..settings(&watch, &out)
    };
    let engine = Engine::start(s, Store::at(&state), || {}).unwrap();
    wait_until(&engine, |e| status_of(e, "plain") == Some(Status::Done));

    assert_eq!(
        fs::read(out.join("plain").join("info.nfo")).unwrap(),
        b"release notes"
    );
    assert!(!release.join("info.nfo").exists());
    assert!(
        release.join("plain.rar").is_file(),
        "sources stay unless deletion is enabled"
    );
    assert!(watch.join("unrelated.txt").is_file());
}
