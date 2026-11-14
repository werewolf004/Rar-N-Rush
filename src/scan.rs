//! Discovery of file sets in the watched folder.
//!
//! A set groups what belongs together: PAR2 files (index + volumes), the files
//! they protect, an optional SFV and the RAR volumes to extract.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Maximum folder depth explored below the watched folder.
pub const MAX_DEPTH: usize = 8;
/// Backup suffixes `.1` … `.9` created by `par2` when it repairs a file.
const PAR2_BACKUP_SUFFIXES: std::ops::Range<u32> = 1..10;
/// Upper bound for a PAR2 `FileDesc` packet body, to avoid huge allocations on bad input.
const MAX_FILEDESC_BODY: u64 = 1 << 20;

/// Errors raised while scanning.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    /// The watched folder cannot be listed.
    #[error("cannot read folder {path}: {source}")]
    ReadRoot {
        /// Folder concerned.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
}

/// Fingerprint of a set, used to detect files that are still arriving.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Signature {
    /// Number of files.
    pub count: usize,
    /// Sum of file sizes in bytes.
    pub total_size: u64,
    /// Most recent modification time.
    pub newest: Option<SystemTime>,
}

/// Kind of a set, derived from the files it contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetKind {
    /// PAR2 files protecting RAR volumes.
    Par2Rar,
    /// PAR2 files protecting other files.
    Par2,
    /// SFV file covering RAR volumes.
    SfvRar,
    /// SFV file covering other files.
    Sfv,
    /// RAR archive without verification data.
    Rar,
}

impl SetKind {
    /// Short label (identical in every language).
    pub fn label(self) -> &'static str {
        match self {
            Self::Par2Rar => "PAR2 + RAR",
            Self::Par2 => "PAR2",
            Self::SfvRar => "SFV + RAR",
            Self::Sfv => "SFV",
            Self::Rar => "RAR",
        }
    }
}

/// A set of related files found by [`scan`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSet {
    /// Stable identifier: folder + name, lower-cased.
    pub key: String,
    /// Display name (original case).
    pub name: String,
    /// Folder containing the set.
    pub dir: PathBuf,
    /// Smallest `.par2` file of the set (normally the index).
    pub par2_index: Option<PathBuf>,
    /// All `.par2` files of the set.
    pub par2_files: Vec<PathBuf>,
    /// SFV file covering the set.
    pub sfv: Option<PathBuf>,
    /// Names of the files protected by the PAR2 or SFV (present or not).
    pub targets: Vec<String>,
    /// Lower-cased base names of the RAR groups attached to this set.
    pub rar_groups: Vec<String>,
    /// Every file on disk that belongs to the set.
    pub files: Vec<PathBuf>,
    /// Fingerprint of [`FileSet::files`].
    pub signature: Signature,
}

impl FileSet {
    /// Kind of the set.
    pub fn kind(&self) -> SetKind {
        match (
            self.par2_index.is_some(),
            self.sfv.is_some(),
            !self.rar_groups.is_empty(),
        ) {
            (true, _, true) => SetKind::Par2Rar,
            (true, _, false) => SetKind::Par2,
            (false, true, true) => SetKind::SfvRar,
            (false, true, false) => SetKind::Sfv,
            _ => SetKind::Rar,
        }
    }
}

/// Parses a RAR volume name into (lower-cased base name, volume rank).
///
/// Rank 0 is an old-style `.rar` first volume (followed by `.r00`, `.r01`, …).
pub fn rar_part_info(file_name: &str) -> Option<(String, u32)> {
    let lower = file_name.to_lowercase();
    if let Some(stem) = lower.strip_suffix(".rar") {
        if let Some(pos) = stem.rfind(".part") {
            let num = stem.get(pos + 5..).unwrap_or_default();
            if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
                let base = stem.get(..pos).unwrap_or_default().to_owned();
                return Some((base, num.parse().unwrap_or(0)));
            }
        }
        return Some((stem.to_owned(), 0));
    }
    // .r00 … .r99, then .s00 … .s99
    let (stem, ext) = lower.rsplit_once('.')?;
    let mut chars = ext.chars();
    let (letter, d1, d2) = (chars.next()?, chars.next()?, chars.next()?);
    if chars.next().is_some() || !(letter == 'r' || letter == 's') {
        return None;
    }
    let n = d1.to_digit(10)? * 10 + d2.to_digit(10)?;
    let offset = if letter == 's' { 101 } else { 1 };
    Some((stem.to_owned(), n + offset))
}

/// From a list of names, returns `(base, first volume name)` for every RAR group
/// that has a `.rar` file.
pub fn rar_first_volumes<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<(String, String)> {
    let mut best: BTreeMap<String, (u32, String)> = BTreeMap::new();
    for name in names {
        // `.rNN` files are never the first volume.
        if !name.to_lowercase().ends_with(".rar") {
            continue;
        }
        if let Some((base, idx)) = rar_part_info(name) {
            let entry = best.entry(base).or_insert((u32::MAX, String::new()));
            if idx < entry.0 {
                *entry = (idx, name.to_owned());
            }
        }
    }
    best.into_iter().map(|(base, (_, name))| (base, name)).collect()
}

/// Lower-cased base name of a `.par2` file (without `.par2` nor `.volXX+YY`).
fn par2_base(file_name: &str) -> Option<String> {
    let lower = file_name.to_lowercase();
    let stem = lower.strip_suffix(".par2")?;
    Some(strip_vol_suffix(stem).to_owned())
}

/// Removes a trailing `.volNN+MM` / `.volNN-MM` from a PAR2 stem (case-insensitive).
fn strip_vol_suffix(stem: &str) -> &str {
    if let Some(pos) = stem.to_lowercase().rfind(".vol") {
        let tail = stem.get(pos + 4..).unwrap_or_default();
        if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit() || c == '+' || c == '-') {
            return stem.get(..pos).unwrap_or(stem);
        }
    }
    stem
}

fn le_u64(bytes: &[u8]) -> Option<u64> {
    bytes.try_into().ok().map(u64::from_le_bytes)
}

/// Reads the names of the protected files from a PAR2 file (`FileDesc` packets).
///
/// Unknown or damaged data simply ends the parsing: a partial list is returned.
pub fn par2_target_names(path: &Path) -> Vec<String> {
    match File::open(path) {
        Ok(file) => par2_target_names_from(BufReader::new(file)),
        Err(_) => Vec::new(),
    }
}

/// [`par2_target_names`] on any seekable reader (used by tests and fuzzing).
pub fn par2_target_names_from(mut reader: impl Read + Seek) -> Vec<String> {
    const MAGIC: &[u8] = b"PAR2\0PKT";
    const FILEDESC: &[u8] = b"PAR 2.0\0FileDesc";
    /// File id (16) + MD5 (16) + MD5 of first 16 KiB (16) + length (8).
    const FILEDESC_FIXED: usize = 56;

    let mut names = Vec::new();
    let mut seen = HashSet::new();
    let mut header = [0u8; 64];
    while reader.read_exact(&mut header).is_ok() {
        if header.get(0..8) != Some(MAGIC) {
            break;
        }
        let Some(len) = header.get(8..16).and_then(le_u64) else {
            break;
        };
        if len < 64 || len % 4 != 0 {
            break;
        }
        let body_len = len - 64;
        let is_filedesc = header.get(48..64) == Some(FILEDESC);
        if is_filedesc && body_len >= FILEDESC_FIXED as u64 && body_len < MAX_FILEDESC_BODY {
            let Ok(size) = usize::try_from(body_len) else {
                break;
            };
            let mut body = vec![0u8; size];
            if reader.read_exact(&mut body).is_err() {
                break;
            }
            let raw = body.get(FILEDESC_FIXED..).unwrap_or_default();
            let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            let name = String::from_utf8_lossy(raw.get(..end).unwrap_or_default()).into_owned();
            if !name.is_empty() && seen.insert(name.to_lowercase()) {
                names.push(name);
            }
        } else {
            let Ok(skip) = i64::try_from(body_len) else { break };
            if reader.seek(SeekFrom::Current(skip)).is_err() {
                break;
            }
        }
    }
    names
}

/// Reads `(file name, CRC32)` pairs from an SFV file. Comment lines start with `;`.
pub fn parse_sfv(path: &Path) -> io::Result<Vec<(String, u32)>> {
    let bytes = fs::read(path)?;
    Ok(parse_sfv_text(&String::from_utf8_lossy(&bytes)))
}

/// Parses SFV content (see [`parse_sfv`]).
pub fn parse_sfv_text(text: &str) -> Vec<(String, u32)> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with(';'))
        .filter_map(|line| {
            let (name, crc) = line.rsplit_once(char::is_whitespace)?;
            let crc = u32::from_str_radix(crc.trim(), 16).ok()?;
            let name = name.trim();
            (!name.is_empty()).then(|| (name.to_owned(), crc))
        })
        .collect()
}

struct Entry {
    path: PathBuf,
    name: String,
    size: u64,
    modified: Option<SystemTime>,
}

/// Scans `root` (and its sub-folders when `recursive`) and returns the sets found.
///
/// `skip_dir` is not explored (typically the extraction folder when it lives
/// inside the watched folder). Unreadable sub-folders are silently skipped.
pub fn scan(root: &Path, recursive: bool, skip_dir: Option<&Path>) -> Result<Vec<FileSet>, ScanError> {
    fs::read_dir(root).map_err(|source| ScanError::ReadRoot {
        path: root.to_path_buf(),
        source,
    })?;
    let skip = skip_dir.and_then(|p| p.canonicalize().ok());
    let mut sets = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(read_dir) = fs::read_dir(&dir) else {
            continue;
        };
        let mut files = Vec::new();
        for entry in read_dir.flatten() {
            // `symlink_metadata`: never follow links out of the watched tree.
            let Ok(meta) = entry.path().symlink_metadata() else {
                continue;
            };
            let path = entry.path();
            if meta.is_dir() {
                let skipped = skip
                    .as_ref()
                    .is_some_and(|s| path.canonicalize().is_ok_and(|p| &p == s));
                if recursive && depth < MAX_DEPTH && !skipped {
                    stack.push((path, depth + 1));
                }
            } else if meta.is_file() {
                files.push(Entry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path,
                    size: meta.len(),
                    modified: meta.modified().ok(),
                });
            }
        }
        sets.extend(scan_dir(&dir, &files));
    }
    sets.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(sets)
}

fn scan_dir(dir: &Path, files: &[Entry]) -> Vec<FileSet> {
    let mut rar_groups: BTreeMap<String, Vec<&Entry>> = BTreeMap::new();
    let mut par2_groups: BTreeMap<String, Vec<&Entry>> = BTreeMap::new();
    for f in files {
        if let Some((base, _)) = rar_part_info(&f.name) {
            rar_groups.entry(base).or_default().push(f);
        }
        if let Some(base) = par2_base(&f.name) {
            par2_groups.entry(base).or_default().push(f);
        }
    }
    let mut grouper = DirGrouper {
        dir,
        by_name: files.iter().map(|f| (f.name.to_lowercase(), f)).collect(),
        sfv_files: files
            .iter()
            .filter(|f| f.name.to_lowercase().ends_with(".sfv"))
            .map(|f| (f, parse_sfv(&f.path).unwrap_or_default()))
            .collect(),
        rar_groups,
        used_rar: HashSet::new(),
        used_sfv: HashSet::new(),
        sets: Vec::new(),
    };
    for pars in par2_groups.into_values() {
        grouper.add_par2_set(pars);
    }
    grouper.add_sfv_sets();
    grouper.add_rar_sets();
    grouper.sets
}

/// Groups the files of one folder into sets. Order matters: PAR2 sets claim their
/// RAR volumes and SFV first, then SFV sets, then whatever RAR archives remain.
struct DirGrouper<'a> {
    dir: &'a Path,
    by_name: HashMap<String, &'a Entry>,
    sfv_files: Vec<(&'a Entry, Vec<(String, u32)>)>,
    rar_groups: BTreeMap<String, Vec<&'a Entry>>,
    used_rar: HashSet<String>,
    used_sfv: HashSet<String>,
    sets: Vec<FileSet>,
}

impl<'a> DirGrouper<'a> {
    fn rar_members(&self, groups: &[String]) -> Vec<&'a Entry> {
        groups
            .iter()
            .flat_map(|g| self.rar_groups.get(g).into_iter().flatten().copied())
            .collect()
    }

    fn add_par2_set(&mut self, mut pars: Vec<&'a Entry>) {
        pars.sort_by_key(|f| f.size);
        let Some(index) = pars.first().copied() else {
            return;
        };
        // The index may be damaged: fall back to the other PAR2 files.
        let targets = pars
            .iter()
            .map(|p| par2_target_names(&p.path))
            .find(|t| !t.is_empty())
            .unwrap_or_default();
        let target_set: HashSet<String> = targets.iter().map(|t| t.to_lowercase()).collect();

        let mut groups = Vec::new();
        for (base, _) in rar_first_volumes(targets.iter().map(String::as_str)) {
            self.used_rar.insert(base.clone());
            groups.push(base);
        }
        for (base, members) in &self.rar_groups {
            let covered = members
                .iter()
                .any(|m| target_set.contains(&m.name.to_lowercase()));
            if covered && self.used_rar.insert(base.clone()) {
                groups.push(base.clone());
            }
        }
        let sfv = self
            .sfv_files
            .iter()
            .find(|(_, entries)| {
                entries
                    .iter()
                    .any(|(n, _)| target_set.contains(&n.to_lowercase()))
            })
            .map(|(s, _)| *s);
        if let Some(s) = sfv {
            self.used_sfv.insert(s.name.to_lowercase());
        }

        let mut members: Vec<&Entry> = pars.clone();
        for t in &targets {
            let lower = t.to_lowercase();
            members.extend(self.by_name.get(&lower).copied());
            for n in PAR2_BACKUP_SUFFIXES {
                members.extend(self.by_name.get(&format!("{lower}.{n}")).copied());
            }
        }
        members.extend(self.rar_members(&groups));
        members.extend(sfv);

        let stem = index
            .name
            .get(..index.name.len().saturating_sub(".par2".len()))
            .unwrap_or(&index.name);
        let set = SetParts {
            name: strip_vol_suffix(stem).to_owned(),
            par2_index: Some(index.path.clone()),
            par2_files: pars.iter().map(|p| p.path.clone()).collect(),
            sfv: sfv.map(|s| s.path.clone()),
            targets,
            rar_groups: groups,
        };
        self.sets.push(set.build(self.dir, members));
    }

    fn add_sfv_sets(&mut self) {
        let sfv_files = std::mem::take(&mut self.sfv_files);
        for (sfv, entries) in &sfv_files {
            if self.used_sfv.contains(&sfv.name.to_lowercase()) || entries.is_empty() {
                continue;
            }
            let targets: Vec<String> = entries.iter().map(|(n, _)| n.clone()).collect();
            let mut groups = Vec::new();
            for (base, _) in rar_first_volumes(targets.iter().map(String::as_str)) {
                if self.used_rar.insert(base.clone()) {
                    groups.push(base);
                }
            }
            let mut members: Vec<&Entry> = vec![*sfv];
            for t in &targets {
                members.extend(self.by_name.get(&t.to_lowercase()).copied());
            }
            members.extend(self.rar_members(&groups));
            let name = sfv
                .name
                .get(..sfv.name.len().saturating_sub(".sfv".len()))
                .unwrap_or(&sfv.name)
                .to_owned();
            let set = SetParts {
                name,
                par2_index: None,
                par2_files: Vec::new(),
                sfv: Some(sfv.path.clone()),
                targets,
                rar_groups: groups,
            };
            self.sets.push(set.build(self.dir, members));
        }
        self.sfv_files = sfv_files;
    }

    fn add_rar_sets(&mut self) {
        for (base, members) in &self.rar_groups {
            if self.used_rar.contains(base) {
                continue;
            }
            let first = rar_first_volumes(members.iter().map(|m| m.name.as_str()))
                .into_iter()
                .next();
            let Some((_, first)) = first else {
                continue; // no `.rar` first volume yet
            };
            let set = SetParts {
                name: original_base(&first),
                par2_index: None,
                par2_files: Vec::new(),
                sfv: None,
                targets: Vec::new(),
                rar_groups: vec![base.clone()],
            };
            self.sets.push(set.build(self.dir, members.clone()));
        }
    }
}

/// Everything that describes a set except its on-disk members.
struct SetParts {
    name: String,
    par2_index: Option<PathBuf>,
    par2_files: Vec<PathBuf>,
    sfv: Option<PathBuf>,
    targets: Vec<String>,
    rar_groups: Vec<String>,
}

impl SetParts {
    fn build(self, dir: &Path, mut members: Vec<&Entry>) -> FileSet {
        members.sort_by(|a, b| a.path.cmp(&b.path));
        members.dedup_by(|a, b| a.path == b.path);
        let signature = Signature {
            count: members.len(),
            total_size: members.iter().map(|m| m.size).sum(),
            newest: members.iter().filter_map(|m| m.modified).max(),
        };
        FileSet {
            key: format!(
                "{}|{}",
                dir.to_string_lossy().to_lowercase(),
                self.name.to_lowercase()
            ),
            name: self.name,
            dir: dir.to_path_buf(),
            par2_index: self.par2_index,
            par2_files: self.par2_files,
            sfv: self.sfv,
            targets: self.targets,
            rar_groups: self.rar_groups,
            files: members.iter().map(|m| m.path.clone()).collect(),
            signature,
        }
    }
}

/// File name of a path as a `String` (lossy).
pub fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Base name of a RAR volume, keeping the original case.
fn original_base(name: &str) -> String {
    let stem = name.rfind('.').and_then(|i| name.get(..i)).unwrap_or(name);
    match stem.to_lowercase().rfind(".part") {
        Some(pos)
            if stem
                .get(pos + 5..)
                .is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit())) =>
        {
            stem.get(..pos).unwrap_or(stem).to_owned()
        }
        _ => stem.to_owned(),
    }
}

/// Every RAR volume of group `base` present in `dir`, in volume order.
pub fn rar_volumes_on_disk(dir: &Path, base: &str) -> Vec<PathBuf> {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut volumes: Vec<(u32, PathBuf)> = read_dir
        .flatten()
        .filter_map(|e| {
            let (b, idx) = rar_part_info(&e.file_name().to_string_lossy())?;
            (b == base).then(|| (idx, e.path()))
        })
        .collect();
    volumes.sort();
    volumes.into_iter().map(|(_, p)| p).collect()
}

/// First volume of group `base` present in `dir`.
pub fn rar_first_on_disk(dir: &Path, base: &str) -> Option<PathBuf> {
    let names: Vec<String> = rar_volumes_on_disk(dir, base)
        .iter()
        .map(|p| file_name(p))
        .collect();
    rar_first_volumes(names.iter().map(String::as_str))
        .into_iter()
        .next()
        .map(|(_, n)| dir.join(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rar_volume_names() {
        assert_eq!(rar_part_info("Film.part01.rar"), Some(("film".into(), 1)));
        assert_eq!(rar_part_info("Film.part3.rar"), Some(("film".into(), 3)));
        assert_eq!(rar_part_info("film.rar"), Some(("film".into(), 0)));
        assert_eq!(rar_part_info("film.r00"), Some(("film".into(), 1)));
        assert_eq!(rar_part_info("film.s01"), Some(("film".into(), 102)));
        assert_eq!(rar_part_info("film.r0"), None);
        assert_eq!(rar_part_info("film.rxx"), None);
        assert_eq!(rar_part_info("film.mkv"), None);
        assert_eq!(rar_part_info("x.part.rar"), Some(("x.part".into(), 0)));
    }

    #[test]
    fn first_volumes() {
        let firsts = rar_first_volumes(["a.part02.rar", "a.part01.rar", "b.rar", "b.r00", "c.r00"]);
        assert_eq!(
            firsts,
            vec![("a".into(), "a.part01.rar".into()), ("b".into(), "b.rar".into())]
        );
    }

    #[test]
    fn par2_names() {
        assert_eq!(par2_base("Film.vol00+01.par2"), Some("film".into()));
        assert_eq!(par2_base("Film.par2"), Some("film".into()));
        assert_eq!(par2_base("Film.txt"), None);
        assert_eq!(strip_vol_suffix("Film.vol03+04"), "Film");
        assert_eq!(strip_vol_suffix("Film.volume"), "Film.volume");
        assert_eq!(original_base("Mon.Film.part01.rar"), "Mon.Film");
        assert_eq!(original_base("Mon.Film.rar"), "Mon.Film");
    }

    #[test]
    fn sfv_text() {
        let parsed = parse_sfv_text("; comment\r\nfile one.rar 0A0b0C0d\n\nbad line\nx.r00\tFFFFFFFF\n zz\n");
        assert_eq!(
            parsed,
            vec![
                ("file one.rar".into(), 0x0A0B_0C0D),
                ("x.r00".into(), 0xFFFF_FFFF)
            ]
        );
    }

    /// Builds a minimal PAR2 packet. The hash fields are not checked by our parser.
    pub(crate) fn par2_packet(kind: &[u8; 16], body: &[u8]) -> Vec<u8> {
        let len = 64 + body.len() as u64;
        let mut p = Vec::new();
        p.extend_from_slice(b"PAR2\0PKT");
        p.extend_from_slice(&len.to_le_bytes());
        p.extend_from_slice(&[0u8; 32]); // packet hash + recovery set id
        p.extend_from_slice(kind);
        p.extend_from_slice(body);
        p
    }

    pub(crate) fn filedesc(name: &str) -> Vec<u8> {
        let mut body = vec![0u8; 56];
        body.extend_from_slice(name.as_bytes());
        while !body.len().is_multiple_of(4) {
            body.push(0);
        }
        par2_packet(b"PAR 2.0\0FileDesc", &body)
    }

    #[test]
    fn par2_filedesc_parsing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.par2");
        let mut data = par2_packet(b"PAR 2.0\0Main\0\0\0\0", &[0u8; 12]);
        data.extend(filedesc("a.part1.rar"));
        data.extend(filedesc("a.part2.rar"));
        data.extend(filedesc("A.PART1.RAR")); // duplicate, other case
        data.extend_from_slice(b"garbage after the packets");
        fs::write(&path, data).unwrap();
        assert_eq!(
            par2_target_names(&path),
            vec!["a.part1.rar".to_owned(), "a.part2.rar".to_owned()]
        );
    }

    #[test]
    fn par2_parser_survives_bad_lengths() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.par2");
        let mut data = b"PAR2\0PKT".to_vec();
        data.extend_from_slice(&u64::MAX.to_le_bytes());
        data.extend_from_slice(&[0u8; 48]);
        fs::write(&path, data).unwrap();
        assert_eq!(par2_target_names(&path), Vec::<String>::new());
        assert_eq!(
            par2_target_names(&dir.path().join("missing.par2")),
            Vec::<String>::new()
        );
    }
}
