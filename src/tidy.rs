//! Clean-up after a successful run: moving the extra files of a set to the
//! destination, and sending folders left empty to the recycle bin.
//!
//! Both operations are conservative. Extra files are only moved when the set is
//! alone in its own sub-folder of the watched folder, so files belonging to other
//! downloads are never touched. Folders are only removed when they are completely
//! empty, never above or at the watched folder, and always to the recycle bin.

use crate::extract::free_path;
use crate::scan::{self, FileSet};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Prefix of the files written by Rar N Rush itself; they are never moved.
const OWN_FILES_PREFIX: &str = "rar-n-rush";

/// Errors raised while tidying up.
#[derive(Debug, thiserror::Error)]
pub enum TidyError {
    /// The folder of the set could not be listed.
    #[error("cannot read {path}: {source}")]
    ReadDir {
        /// Folder concerned.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
    /// A file could not be moved.
    #[error("cannot move {from} to {to}: {source}")]
    Move {
        /// File being moved.
        from: PathBuf,
        /// Target path.
        to: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
    /// A folder could not be sent to the recycle bin.
    #[error("cannot send {path} to the Recycle Bin: {message}")]
    Trash {
        /// Folder concerned.
        path: PathBuf,
        /// Message from the system.
        message: String,
    },
}

/// Why extra files were not moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotMoved {
    /// The set lies directly in the watched folder, next to other downloads.
    InWatchedFolder,
    /// The folder of the set also holds other sets.
    SharedFolder,
    /// The destination is the folder of the set itself.
    SameFolder,
}

/// Source files of a set: PAR2, SFV, RAR volumes and PAR2 backups (`name.1`…).
pub fn source_files(set: &FileSet) -> Vec<PathBuf> {
    let mut sources: Vec<PathBuf> = set.par2_files.clone();
    sources.extend(set.sfv.clone());
    for group in &set.rar_groups {
        sources.extend(scan::rar_volumes_on_disk(&set.dir, group));
    }
    for target in &set.targets {
        for n in 1..10 {
            sources.push(set.dir.join(format!("{target}.{n}")));
        }
    }
    sources.retain(|p| p.is_file());
    sources.sort();
    sources.dedup();
    sources
}

/// Lists the extra files of `set`: regular files directly in its folder that are
/// not source files. Returns why nothing may be moved when the set does not have
/// a folder of its own.
pub fn extra_files(
    set: &FileSet,
    root: &Path,
    dest: &Path,
) -> Result<Result<Vec<PathBuf>, NotMoved>, TidyError> {
    if same_path(&set.dir, root) || !set.dir.starts_with(root) {
        return Ok(Err(NotMoved::InWatchedFolder));
    }
    if same_path(&set.dir, dest) {
        return Ok(Err(NotMoved::SameFolder));
    }
    let sets = scan::scan(&set.dir, false, None).map_err(|e| TidyError::ReadDir {
        path: set.dir.clone(),
        source: io::Error::other(e.to_string()),
    })?;
    if sets.len() > 1 {
        return Ok(Err(NotMoved::SharedFolder));
    }
    let sources: HashSet<PathBuf> = source_files(set).into_iter().collect();
    let entries = fs::read_dir(&set.dir).map_err(|source| TidyError::ReadDir {
        path: set.dir.clone(),
        source,
    })?;
    let mut extras: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .filter(|p| !sources.contains(p))
        .filter(|p| !scan::file_name(p).to_lowercase().starts_with(OWN_FILES_PREFIX))
        .collect();
    extras.sort();
    Ok(Ok(extras))
}

/// Moves `file` into `dest_dir`. An existing file is kept (a free name such as
/// `name (1).ext` is used) unless `overwrite` is set. Returns the new path.
pub fn move_file(file: &Path, dest_dir: &Path, overwrite: bool) -> Result<PathBuf, TidyError> {
    let move_error = |to: &Path, source: io::Error| TidyError::Move {
        from: file.to_path_buf(),
        to: to.to_path_buf(),
        source,
    };
    fs::create_dir_all(dest_dir).map_err(|e| move_error(dest_dir, e))?;
    let target = dest_dir.join(file.file_name().unwrap_or_default());
    let target = if overwrite { target } else { free_path(&target) };
    if fs::rename(file, &target).is_ok() {
        return Ok(target);
    }
    // Different drive: copy, then remove the original. If the original cannot be
    // removed, the copy is deleted so the file never ends up in two places.
    fs::copy(file, &target).map_err(|e| move_error(&target, e))?;
    if let Err(e) = fs::remove_file(file) {
        let _ = fs::remove_file(&target);
        return Err(move_error(&target, e));
    }
    Ok(target)
}

/// Sends `start` to the recycle bin if it is empty, then each empty parent,
/// stopping below `root`. Returns the folders removed.
pub fn remove_empty_dirs(start: &Path, root: &Path) -> Result<Vec<PathBuf>, TidyError> {
    remove_empty_dirs_with(start, root, |dir| trash::delete(dir).map_err(|e| e.to_string()))
}

/// [`remove_empty_dirs`] with the removal step injected (tests avoid the recycle bin).
fn remove_empty_dirs_with(
    start: &Path,
    root: &Path,
    mut remove: impl FnMut(&Path) -> Result<(), String>,
) -> Result<Vec<PathBuf>, TidyError> {
    let mut removed = Vec::new();
    let mut dir = start.to_path_buf();
    while dir.starts_with(root) && !same_path(&dir, root) && is_empty_dir(&dir) {
        remove(&dir).map_err(|message| TidyError::Trash {
            path: dir.clone(),
            message,
        })?;
        removed.push(dir.clone());
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => break,
        }
    }
    Ok(removed)
}

/// `true` for a real folder (not a link) without any entry.
fn is_empty_dir(dir: &Path) -> bool {
    fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir())
        && fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_none())
}

/// Compares paths the way Windows does: case-insensitive, trailing separator ignored.
fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| p.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase();
    norm(a) == norm(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rm(dir: &Path) -> Result<(), String> {
        fs::remove_dir(dir).map_err(|e| e.to_string())
    }

    fn set_in(dir: &Path) -> FileSet {
        let sets = scan::scan(dir, false, None).unwrap();
        assert_eq!(sets.len(), 1, "expected one set in {}", dir.display());
        sets.into_iter().next().unwrap()
    }

    #[test]
    fn same_path_ignores_case_and_trailing_separator() {
        assert!(same_path(Path::new("C:\\Dl\\"), Path::new("c:\\dl")));
        assert!(!same_path(Path::new("C:\\Dl\\a"), Path::new("C:\\Dl")));
    }

    #[test]
    fn move_file_keeps_existing_files() {
        let tmp = tempfile::tempdir().unwrap();
        let (src, dest) = (tmp.path().join("src"), tmp.path().join("dest"));
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dest).unwrap();
        fs::write(src.join("info.nfo"), b"new").unwrap();
        fs::write(dest.join("info.nfo"), b"old").unwrap();

        let moved = move_file(&src.join("info.nfo"), &dest, false).unwrap();
        assert_eq!(moved, dest.join("info (1).nfo"));
        assert_eq!(fs::read(dest.join("info.nfo")).unwrap(), b"old");
        assert_eq!(fs::read(&moved).unwrap(), b"new");
        assert!(!src.join("info.nfo").exists());
    }

    #[test]
    fn move_file_overwrites_when_asked() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.txt"), b"new").unwrap();
        let dest = tmp.path().join("out");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("a.txt"), b"old").unwrap();
        let moved = move_file(&tmp.path().join("a.txt"), &dest, true).unwrap();
        assert_eq!(moved, dest.join("a.txt"));
        assert_eq!(fs::read(&moved).unwrap(), b"new");
    }

    #[test]
    fn extra_files_skip_sources_and_own_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Release");
        fs::create_dir_all(&dir).unwrap();
        for name in [
            "movie.part1.rar",
            "movie.part2.rar",
            "movie.nfo",
            "rar-n-rush.toml",
        ] {
            fs::write(dir.join(name), b"x").unwrap();
        }
        fs::create_dir_all(dir.join("Sample")).unwrap();
        let set = set_in(&dir);
        let extras = extra_files(&set, tmp.path(), &tmp.path().join("out")).unwrap();
        assert_eq!(extras, Ok(vec![dir.join("movie.nfo")]));
    }

    #[test]
    fn extra_files_are_refused_outside_a_folder_of_their_own() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.rar"), b"x").unwrap();
        fs::write(tmp.path().join("other.nfo"), b"x").unwrap();
        let set = set_in(tmp.path());
        let out = tmp.path().join("out");
        assert_eq!(
            extra_files(&set, tmp.path(), &out).unwrap(),
            Err(NotMoved::InWatchedFolder)
        );

        let shared = tmp.path().join("shared");
        fs::create_dir_all(&shared).unwrap();
        fs::write(shared.join("a.rar"), b"x").unwrap();
        fs::write(shared.join("b.rar"), b"x").unwrap();
        let sets = scan::scan(&shared, false, None).unwrap();
        assert_eq!(
            extra_files(&sets[0], tmp.path(), &out).unwrap(),
            Err(NotMoved::SharedFolder)
        );
        let own = tmp.path().join("own");
        fs::create_dir_all(&own).unwrap();
        fs::write(own.join("a.rar"), b"x").unwrap();
        assert_eq!(
            extra_files(&set_in(&own), tmp.path(), &own).unwrap(),
            Err(NotMoved::SameFolder)
        );
    }

    #[test]
    fn empty_folders_are_removed_up_to_the_watched_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("watch");
        let leaf = root.join("a").join("b");
        fs::create_dir_all(&leaf).unwrap();
        fs::create_dir_all(root.join("keep")).unwrap();
        fs::write(root.join("keep").join("file.txt"), b"x").unwrap();

        let removed = remove_empty_dirs_with(&leaf, &root, rm).unwrap();
        assert_eq!(removed, vec![leaf.clone(), root.join("a")]);
        assert!(root.is_dir(), "the watched folder is never removed");
        assert!(root.join("keep").join("file.txt").is_file());
    }

    #[test]
    fn non_empty_folders_and_the_root_are_kept() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("watch");
        fs::create_dir_all(root.join("a")).unwrap();
        fs::write(root.join("a").join("x"), b"x").unwrap();
        assert_eq!(
            remove_empty_dirs_with(&root.join("a"), &root, rm).unwrap(),
            Vec::<PathBuf>::new()
        );
        let empty_root = tmp.path().join("empty");
        fs::create_dir_all(&empty_root).unwrap();
        assert_eq!(
            remove_empty_dirs_with(&empty_root, &empty_root, rm).unwrap(),
            Vec::<PathBuf>::new()
        );
        assert!(empty_root.is_dir());
    }
}
