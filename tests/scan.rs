//! Integration tests for file-set discovery.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use rar_n_rush::scan::{self, SetKind};
use std::fs;

fn touch(dir: &std::path::Path, names: &[&str]) {
    for n in names {
        fs::write(dir.join(n), b"x").unwrap();
    }
}

#[test]
fn groups_par2_sfv_and_rar_sets() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    // PAR2 set protecting a multi-volume RAR (one volume still missing).
    touch(d, &["Movie.part1.rar", "Movie.part2.rar", "Movie.vol00+01.par2"]);
    common::write_par2(
        &d.join("Movie.par2"),
        &["Movie.part1.rar", "Movie.part2.rar", "Movie.part3.rar"],
    );
    // SFV set covering an old-style RAR.
    touch(d, &["Album.rar", "Album.r00"]);
    common::write_sfv(d, "Album.sfv", &["Album.rar", "Album.r00"]);
    // Plain RAR and an unrelated file.
    touch(d, &["Photos.rar", "notes.txt"]);

    let sets = scan::scan(d, false, None).unwrap();
    let find = |name: &str| {
        sets.iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("{name} missing: {sets:#?}"))
    };
    assert_eq!(sets.len(), 3, "{sets:#?}");

    let movie = find("Movie");
    assert_eq!(movie.kind(), SetKind::Par2Rar);
    assert_eq!(movie.par2_files.len(), 2);
    assert_eq!(movie.targets.len(), 3);
    assert_eq!(movie.rar_groups, vec!["movie".to_owned()]);
    assert_eq!(movie.files.len(), 4); // 2 PAR2 + 2 volumes present

    let album = find("Album");
    assert_eq!(album.kind(), SetKind::SfvRar);
    assert_eq!(album.files.len(), 3);

    assert_eq!(find("Photos").kind(), SetKind::Rar);
}

#[test]
fn par2_claims_its_sfv() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    touch(d, &["a.rar"]);
    common::write_par2(&d.join("a.par2"), &["a.rar"]);
    common::write_sfv(d, "a.sfv", &["a.rar"]);
    let sets = scan::scan(d, false, None).unwrap();
    assert_eq!(sets.len(), 1, "{sets:#?}");
    assert!(sets[0].sfv.is_some());
    assert_eq!(sets[0].kind(), SetKind::Par2Rar);
}

#[test]
fn rar_without_first_volume_is_not_a_set_yet() {
    let dir = tempfile::tempdir().unwrap();
    touch(dir.path(), &["x.r00", "x.r01"]);
    assert_eq!(scan::scan(dir.path(), false, None).unwrap(), Vec::new());
}

#[test]
fn recursion_and_skip_dir() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    fs::create_dir_all(d.join("sub").join("deeper")).unwrap();
    fs::create_dir_all(d.join("out")).unwrap();
    touch(&d.join("sub").join("deeper"), &["deep.rar"]);
    touch(&d.join("out"), &["extracted.rar"]);
    touch(d, &["top.rar"]);

    let flat = scan::scan(d, false, None).unwrap();
    assert_eq!(
        flat.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        vec!["top"]
    );

    let deep = scan::scan(d, true, Some(&d.join("out"))).unwrap();
    let mut names: Vec<&str> = deep.iter().map(|s| s.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, vec!["deep", "top"]);
}

#[test]
fn signature_changes_when_files_grow() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    touch(d, &["g.rar"]);
    let before = scan::scan(d, false, None).unwrap()[0].signature;
    fs::write(d.join("g.rar"), b"bigger content").unwrap();
    let after = scan::scan(d, false, None).unwrap()[0].signature;
    assert_ne!(before, after);
    assert_eq!(after.total_size, 14);
}

#[test]
fn missing_root_is_a_typed_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(scan::scan(&dir.path().join("nope"), true, None).is_err());
}
