//! Packs: the files that switch things on (My Plan, the reading book, the pattern book, atonal lines, chord
//! melodies) and your loops and playlists, in ONE file to hand a friend. Dropped on Backline (or opened from
//! PREFERENCES), each piece goes where it belongs in Backline's folder. Your plan's log, keepers and diary never go
//! in a pack.

use std::io::{Read, Write};
use std::path::Path;

/// The pieces a pack can hold: key, name, and the file in Backline's folder (the chord melodies are a folder).
pub const PARTS: [(&str, &str, &str); 5] = [
    ("plan", "MY PLAN", "plan.json"),
    ("reading", "READING BOOK", "reading_book.txt"),
    ("pattern", "PATTERN BOOK", "pattern_book.txt"),
    ("atonal", "ATONAL LINES", "atonal_lines.json"),
    ("melodies", "CHORD MELODIES", "transcriptions/"),
];

/// Where loops and playlists sit inside a pack.
const SHARE_FILE: &str = "loops_and_playlists.json";
/// What a pack says it is.
const MARK_FILE: &str = "backline_pack.txt";

/// The chord melodies on this computer: (name in a pack, path).
fn melody_files() -> Vec<(String, std::path::PathBuf)> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(crate::transcription::dir()) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("json")) {
                if let Some(n) = p.file_name().and_then(|n| n.to_str()) {
                    out.push((format!("transcriptions/{}", n), p.clone()));
                }
            }
        }
    }
    out.sort();
    out
}

/// What this computer could put in a pack: each piece there is, with how many (chord melodies) where it counts.
pub fn available() -> Vec<(&'static str, String)> {
    let dir = crate::api::config_dir();
    let mut out = Vec::new();
    for (key, name, file) in PARTS {
        if key == "melodies" {
            let n = melody_files().len();
            if n > 0 {
                out.push((key, format!("{} ({})", name, n)));
            }
        } else if dir.join(file).exists() {
            out.push((key, name.to_string()));
        }
    }
    out
}

/// Write a pack with these pieces (and the loops and playlists, already as text, when given).
pub fn make(path: &Path, keys: &[&str], share: Option<String>) -> Result<(), String> {
    let dir = crate::api::config_dir();
    let mut files: Vec<(String, Vec<u8>)> = vec![(MARK_FILE.to_string(), b"A Backline pack. Drop it on Backline to open it.\n".to_vec())];
    for (key, _, file) in PARTS {
        if !keys.contains(&key) {
            continue;
        }
        if key == "melodies" {
            for (name, p) in melody_files() {
                files.push((name, std::fs::read(&p).map_err(|e| e.to_string())?));
            }
        } else {
            files.push((file.to_string(), std::fs::read(dir.join(file)).map_err(|e| format!("{}: {}", file, e))?));
        }
    }
    if let Some(s) = share {
        files.push((SHARE_FILE.to_string(), s.into_bytes()));
    }
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipWriter::new(f);
    for (name, data) in &files {
        z.start_file(name.as_str(), zip::write::SimpleFileOptions::default()).map_err(|e| e.to_string())?;
        z.write_all(data).map_err(|e| e.to_string())?;
    }
    z.finish().map_err(|e| e.to_string())?;
    Ok(())
}

/// Is this a pack: a zip (not a loops-and-playlists file), or one unzipped (a folder with the pack's mark)?
pub fn is_pack(path: &Path) -> bool {
    if path.is_dir() {
        return find_mark(path).is_some();
    }
    let mut head = [0u8; 2];
    std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut head)).is_ok() && &head == b"PK"
}

/// One piece of a pack being opened: what it is, its files, whether it would replace one of yours, and whether
/// it is ticked (a piece that replaces yours starts unticked).
#[derive(Clone)]
pub struct InPart {
    pub key: &'static str,
    pub label: String,
    pub files: Vec<(String, Vec<u8>)>,
    pub replaces: bool,
    pub on: bool,
}

/// A pack opened: its pieces, and its loops and playlists.
#[derive(Clone)]
pub struct Opened {
    pub parts: Vec<InPart>,
    pub share: Option<crate::store::Share>,
}

/// The folder holding a pack's mark: this one, or one inside it (as a Mac leaves a zip it opened).
fn find_mark(dir: &Path) -> Option<std::path::PathBuf> {
    if dir.join(MARK_FILE).exists() {
        return Some(dir.to_path_buf());
    }
    std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| p.is_dir() && p.join(MARK_FILE).exists())
}

/// Every file of a pack, by its name in the pack: from a zip, or from a folder (a pack unzipped, or the pieces
/// put in a folder by hand). A zip packed again on a Mac has everything one folder down and a __MACOSX folder
/// beside it: that folder comes off the names, the __MACOSX stays out.
fn read_files(path: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut got: Vec<(String, Vec<u8>)> = Vec::new();
    if path.is_dir() {
        let root = find_mark(path).unwrap_or_else(|| path.to_path_buf());
        let mut names: Vec<String> = PARTS.iter().filter(|x| x.0 != "melodies").map(|x| x.2.to_string()).collect();
        names.push(MARK_FILE.to_string());
        names.push(SHARE_FILE.to_string());
        if let Ok(rd) = std::fs::read_dir(root.join("transcriptions")) {
            names.extend(rd.flatten().filter_map(|e| e.file_name().to_str().map(|n| format!("transcriptions/{}", n))));
        }
        for n in names {
            if let Ok(d) = std::fs::read(root.join(&n)) {
                got.push((n, d));
            }
        }
        return Ok(got);
    }
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|_| "that isn't a Backline pack".to_string())?;
    for i in 0..z.len() {
        let mut e = z.by_index(i).map_err(|e| e.to_string())?;
        if e.is_dir() || e.size() > 50_000_000 {
            continue;
        }
        let name = e.name().replace('\\', "/");
        if name.starts_with("__MACOSX/") {
            continue;
        }
        let mut data = Vec::new();
        e.read_to_end(&mut data).map_err(|e| e.to_string())?;
        got.push((name, data));
    }
    if !got.iter().any(|(n, _)| n == MARK_FILE) {
        let top = got
            .iter()
            .find_map(|(n, _)| n.strip_suffix(MARK_FILE).filter(|t| t.ends_with('/') && t.matches('/').count() == 1))
            .map(String::from);
        if let Some(top) = top {
            got = got.into_iter().filter_map(|(n, d)| n.strip_prefix(&top).map(|s| (s.to_string(), d))).collect();
        }
    }
    Ok(got)
}

/// Read a pack (a zip or a folder). Only the files a pack can hold are taken; anything else in it is left alone.
/// A folder picked by hand needs no mark: any of the pieces in it will do.
pub fn open(path: &Path) -> Result<Opened, String> {
    let got = read_files(path)?;
    let known = |n: &str| PARTS.iter().any(|x| x.2 == n) || n.starts_with("transcriptions/");
    if !got.iter().any(|(n, _)| n == MARK_FILE) && !(path.is_dir() && got.iter().any(|(n, _)| known(n))) {
        return Err("that isn't a Backline pack".to_string());
    }
    let dir = crate::api::config_dir();
    let mut parts = Vec::new();
    for (key, name, file) in PARTS {
        if key == "melodies" {
            // a melody you have already (same file name) stays yours; only new ones come in
            let mut new: Vec<(String, Vec<u8>)> = Vec::new();
            for (n, data) in &got {
                let Some(base) = n.strip_prefix("transcriptions/") else { continue };
                let safe = !base.is_empty() && !base.contains('/') && !base.contains("..") && base.to_lowercase().ends_with(".json");
                if safe && !dir.join("transcriptions").join(base).exists() {
                    new.push((n.clone(), data.clone()));
                }
            }
            if !new.is_empty() {
                let label = format!("{} ({} NEW)", name, new.len());
                parts.push(InPart { key, label, files: new, replaces: false, on: true });
            }
        } else if let Some((_, data)) = got.iter().find(|(n, _)| n == file) {
            if key == "plan" {
                if let Err(e) = crate::plan::parse(&String::from_utf8_lossy(data)) {
                    return Err(format!("its plan doesn't read ({})", e));
                }
            }
            let mine = std::fs::read(dir.join(file)).ok();
            if mine.as_deref() == Some(data.as_slice()) {
                continue; // the same as yours already
            }
            let replaces = mine.is_some();
            parts.push(InPart { key, label: name.to_string(), files: vec![(file.to_string(), data.clone())], replaces, on: !replaces });
        }
    }
    let share = got
        .iter()
        .find(|(n, _)| n == SHARE_FILE)
        .and_then(|(_, d)| serde_json::from_slice::<crate::store::Share>(d).ok())
        .filter(|s| s.kind == "backline-share");
    Ok(Opened { parts, share })
}

/// Put the ticked pieces in place. The names of the pieces put in.
pub fn unlock(parts: &[InPart]) -> Result<Vec<String>, String> {
    let dir = crate::api::config_dir();
    let mut done = Vec::new();
    for p in parts.iter().filter(|p| p.on) {
        for (name, data) in &p.files {
            let dest = dir.join(name);
            if let Some(d) = dest.parent() {
                std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            std::fs::write(&dest, data).map_err(|e| format!("{}: {}", name, e))?;
        }
        done.push(p.label.clone());
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pack made, then opened into an empty folder: every piece lands, nothing else does, and a piece you
    /// have already is marked as replacing yours. Run alone with BACKLINE_HOME set to an empty folder:
    /// `BACKLINE_HOME=<empty dir> cargo test --release pack_ -- --ignored --test-threads=1` (one at a time: they share the folder).
    #[test]
    #[ignore]
    fn pack_round_trip() {
        let Some(home) = std::env::var_os("BACKLINE_HOME").map(std::path::PathBuf::from) else { return };
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("transcriptions")).unwrap();
        let plan = r#"{"title":"T","start":"2026-11-02","days":[{"name":"Monday"},{"name":"Tuesday"},{"name":"Wednesday"},{"name":"Thursday"},{"name":"Friday"},{"name":"Saturday"},{"name":"Sunday"}],"weeks":[{"phase":"One"}]}"#;
        std::fs::write(home.join("plan.json"), plan).unwrap();
        std::fs::write(home.join("reading_book.txt"), "@lesson 1\n").unwrap();
        std::fs::write(home.join("transcriptions").join("A - Tune.json"), "{}").unwrap();
        std::fs::write(home.join("plan_log.json"), "private").unwrap();
        let keys: Vec<&str> = available().into_iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["plan", "reading", "melodies"]);
        let out = home.join("out.backline");
        make(&out, &keys, None).unwrap();
        assert!(is_pack(&out));

        // a fresh folder (the friend's): everything new, nothing replaces
        for f in ["plan.json", "reading_book.txt", "plan_log.json"] {
            std::fs::remove_file(home.join(f)).unwrap();
        }
        std::fs::remove_dir_all(home.join("transcriptions")).unwrap();
        let o = open(&out).unwrap();
        assert_eq!(o.parts.len(), 3);
        assert!(o.parts.iter().all(|p| p.on && !p.replaces));
        unlock(&o.parts).unwrap();
        assert_eq!(std::fs::read_to_string(home.join("plan.json")).unwrap(), plan);
        assert!(home.join("transcriptions").join("A - Tune.json").exists());
        assert!(!home.join("plan_log.json").exists(), "progress never travels");

        // opened again: the same files are skipped; a changed one would replace, so it starts unticked
        assert!(open(&out).unwrap().parts.is_empty());
        std::fs::write(home.join("reading_book.txt"), "@lesson 2\n").unwrap();
        let o = open(&out).unwrap();
        assert_eq!(o.parts.len(), 1);
        assert!(o.parts[0].replaces && !o.parts[0].on);
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[cfg(test)]
mod mac_tests {
    use super::*;

    /// A pack unzipped into a folder, and one zipped again on a Mac (everything one folder down, a __MACOSX
    /// beside it), open like the pack itself. Run with BACKLINE_HOME set to an empty folder, like pack_round_trip.
    #[test]
    #[ignore]
    fn pack_from_a_folder_and_a_mac_zip() {
        let Some(home) = std::env::var_os("BACKLINE_HOME").map(std::path::PathBuf::from) else { return };
        let _ = std::fs::remove_dir_all(&home);
        let unz = home.join("dl").join("Backline pack");
        std::fs::create_dir_all(unz.join("transcriptions")).unwrap();
        std::fs::write(unz.join(MARK_FILE), "x").unwrap();
        std::fs::write(unz.join("reading_book.txt"), "@lesson 1\n").unwrap();
        std::fs::write(unz.join("transcriptions").join("T.json"), "{}").unwrap();
        // the folder it was unzipped into, and the folder itself
        assert!(is_pack(&home.join("dl")) && is_pack(&unz));
        let o = open(&home.join("dl")).unwrap();
        assert_eq!(o.parts.iter().map(|p| p.key).collect::<Vec<_>>(), vec!["reading", "melodies"]);
        // zipped again on a Mac
        let z = home.join("again.zip");
        {
            let mut w = zip::ZipWriter::new(std::fs::File::create(&z).unwrap());
            for (n, d) in [("Backline pack/backline_pack.txt", "x"), ("Backline pack/reading_book.txt", "@lesson 1\n"), ("__MACOSX/Backline pack/._reading_book.txt", "junk")] {
                w.start_file(n, zip::write::SimpleFileOptions::default()).unwrap();
                w.write_all(d.as_bytes()).unwrap();
            }
            w.finish().unwrap();
        }
        let o = open(&z).unwrap();
        assert_eq!(o.parts.len(), 1);
        assert_eq!(o.parts[0].files[0].0, "reading_book.txt");
        // a music folder is not a pack
        std::fs::create_dir_all(home.join("music")).unwrap();
        assert!(!is_pack(&home.join("music")));
        let _ = std::fs::remove_dir_all(&home);
    }
}
