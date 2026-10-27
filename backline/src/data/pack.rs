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

/// Is this file a pack (a zip, not a loops-and-playlists file)?
pub fn is_pack(path: &Path) -> bool {
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

/// Read a pack. Only the files a pack can hold are taken; anything else in it is left alone.
pub fn open(path: &Path) -> Result<Opened, String> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|_| "that isn't a Backline pack".to_string())?;
    let mut got: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..z.len() {
        let mut e = z.by_index(i).map_err(|e| e.to_string())?;
        if e.is_dir() || e.size() > 50_000_000 {
            continue;
        }
        let name = e.name().replace('\\', "/");
        let mut data = Vec::new();
        e.read_to_end(&mut data).map_err(|e| e.to_string())?;
        got.push((name, data));
    }
    if !got.iter().any(|(n, _)| n == MARK_FILE) {
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
    /// `BACKLINE_HOME=<empty dir> cargo test --release pack_round_trip -- --ignored`.
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
