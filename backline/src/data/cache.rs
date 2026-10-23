//! Local copy of every track that gets played, so it can be replayed (and looped) without
//! downloading again. Files live in `<config dir>/cache/<track id>.<ext>`.

use crate::api::Track;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

fn set() -> &'static Mutex<HashSet<i64>> {
    static S: OnceLock<Mutex<HashSet<i64>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashSet::new()))
}

pub fn dir() -> PathBuf {
    crate::api::config_dir().join("cache")
}

/// Is this track stored on this computer?
pub fn has(id: i64) -> bool {
    set().lock().map(|s| s.contains(&id)).unwrap_or(false)
}

/// Read the folder once at startup.
pub fn scan() {
    let mut found = HashSet::new();
    if let Ok(rd) = std::fs::read_dir(dir()) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(stem) = name.split('.').next() {
                if let Ok(id) = stem.parse::<i64>() {
                    if !name.ends_with(".part") {
                        found.insert(id);
                    }
                }
            }
        }
    }
    if let Ok(mut s) = set().lock() {
        *s = found;
    }
}

pub fn path_for(id: i64) -> Option<PathBuf> {
    for ext in ["m4a", "flac", "mp3"] {
        let p = dir().join(format!("{}.{}", id, ext));
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// The stored file of a track, if there is one (to show it in the file manager).
pub fn file_of(id: i64) -> Option<PathBuf> {
    path_for(id).filter(|p| p.exists())
}

pub fn get(id: i64) -> Option<Vec<u8>> {
    let p = path_for(id)?;
    match std::fs::read(&p) {
        Ok(b) if b.len() > 1024 => Some(b),
        _ => None,
    }
}

/// Save a downloaded track. Written under a temporary name first, so a crash never
/// leaves a half-written file that looks complete.
pub fn put(id: i64, bytes: &[u8]) -> Option<PathBuf> {
    let ext = if bytes.starts_with(b"fLaC") {
        "flac"
    } else if bytes.starts_with(b"ID3") || (bytes.len() > 2 && bytes[0] == 0xFF && bytes[1] & 0xE0 == 0xE0) {
        "mp3"
    } else {
        "m4a"
    };
    let d = dir();
    std::fs::create_dir_all(&d).ok()?;
    let tmp = d.join(format!("{}.{}.part", id, ext));
    let fin = d.join(format!("{}.{}", id, ext));
    std::fs::write(&tmp, bytes).ok()?;
    std::fs::rename(&tmp, &fin).ok()?;
    if let Ok(mut s) = set().lock() {
        s.insert(id);
    }
    to_tag(id);
    Some(fin)
}

// ------------------------------------------------------------------ the songs' details, in the files themselves
// A stored copy is named by its number only, so its title, artist, album and cover are written into its own tags:
// it shows its name in Explorer and other players, and offline Backline lists it from those tags.

/// The details known for each song (from the lists Backline has loaded and the songs played).
fn known() -> &'static Mutex<HashMap<i64, Track>> {
    static K: OnceLock<Mutex<HashMap<i64, Track>>> = OnceLock::new();
    K.get_or_init(|| Mutex::new(HashMap::new()))
}

/// What each stored song's tags say (filled in the background at start and as songs are tagged).
fn listed() -> &'static Mutex<HashMap<i64, Track>> {
    static L: OnceLock<Mutex<HashMap<i64, Track>>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(HashMap::new()))
}

static TAGGER: Mutex<Option<Sender<i64>>> = Mutex::new(None);

fn to_tag(id: i64) {
    if let Some(tx) = TAGGER.lock().ok().and_then(|t| t.clone()) {
        let _ = tx.send(id);
    }
}

/// Songs whose details are known: a stored copy of any of them gets them in its tags.
pub fn know(tracks: &[Track]) {
    let mut fresh = Vec::new();
    if let Ok(mut k) = known().lock() {
        for t in tracks.iter().filter(|t| t.id != 0 && !t.title.is_empty()) {
            let same =
                k.get(&t.id).is_some_and(|o| o.title == t.title && o.artist_line() == t.artist_line() && o.album == t.album);
            if !same {
                k.insert(t.id, Track { qid: 0, ..t.clone() });
                fresh.push(t.id);
            }
        }
    }
    for id in fresh.into_iter().filter(|id| has(*id)) {
        to_tag(id);
    }
}

/// Every stored song, with the details in its tags (the ones with none left out).
pub fn tagged() -> Vec<Track> {
    listed().lock().map(|l| l.values().filter(|t| has(t.id)).cloned().collect()).unwrap_or_default()
}

/// Start the tagger: every stored song's tags are read (for the offline list), and the ones missing their details,
/// or with old ones, are written - one at a time, in the background. `cover` finds a song's cover picture.
pub fn start_tagging(cover: impl Fn(&Track) -> Option<Vec<u8>> + Send + 'static) {
    let (tx, rx) = channel::<i64>();
    if let Ok(mut t) = TAGGER.lock() {
        *t = Some(tx.clone());
    }
    let all: Vec<i64> = set().lock().map(|s| s.iter().copied().collect()).unwrap_or_default();
    for id in all {
        let _ = tx.send(id);
    }
    std::thread::spawn(move || {
        while let Ok(id) = rx.recv() {
            let Some(path) = path_for(id) else { continue };
            let want = known().lock().ok().and_then(|k| k.get(&id).cloned());
            let has_now = read_tags(&path, id);
            // (the picture looked for only when it would be written: a song with no cover to hand isn't written
            // again every start)
            let mut pic = None;
            let up_to_date = match (&want, &has_now) {
                (Some(w), Some(h)) if h.0.title == w.title && h.0.artist == w.artist_line() && h.0.album == w.album => {
                    h.1 || {
                        pic = cover(w);
                        pic.is_none()
                    }
                }
                (None, _) => true,
                (Some(w), _) => {
                    pic = cover(w);
                    false
                }
            };
            let got = if up_to_date {
                has_now.map(|h| h.0)
            } else {
                let w = want.unwrap_or_default();
                match write_tags(&path, &w, pic) {
                    Ok(()) => read_tags(&path, id).map(|h| h.0),
                    Err(e) => {
                        crate::api::log(&format!("track {}: could not write its details: {}", id, e));
                        has_now.map(|h| h.0)
                    }
                }
            };
            if let (Some(t), Ok(mut l)) = (got, listed().lock()) {
                l.insert(id, t);
            }
        }
    });
}

/// A stored file's details as a song (the cover: the picture inside it), and whether it has a cover picture.
fn read_tags(path: &std::path::Path, id: i64) -> Option<(Track, bool)> {
    use lofty::prelude::*;
    let tagged = lofty::read_from_path(path).ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    let title = tag.title()?.to_string();
    if title.is_empty() {
        return None;
    }
    let pic = !tag.pictures().is_empty();
    let t = Track {
        id,
        title,
        artist: tag.artist().map(|s| s.to_string()).unwrap_or_default(),
        album: tag.album().map(|s| s.to_string()).unwrap_or_default(),
        cover: if pic { format!("file:{}", path.display()) } else { String::new() },
        duration: tagged.properties().duration().as_secs_f32(),
        ..Track::default()
    };
    Some((t, pic))
}

/// Write a song's details into its stored file: on a copy first, then swapped in, so a song being read never sees
/// a half-written file (if it is in use, it is tried again the next time Backline starts).
fn write_tags(path: &std::path::Path, t: &Track, cover: Option<Vec<u8>>) -> Result<(), String> {
    use lofty::picture::{MimeType, Picture, PictureType};
    use lofty::prelude::*;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("m4a").to_string();
    let tmp = path.with_extension(format!("{}.tag.part", ext));
    std::fs::copy(path, &tmp).map_err(|e| e.to_string())?;
    let res = (|| {
        // (by what is in it: the copy's name doesn't end in the file type)
        let probe = lofty::probe::Probe::open(&tmp).map_err(|e| e.to_string())?;
        let mut tagged = probe.guess_file_type().map_err(|e| e.to_string())?.read().map_err(|e| e.to_string())?;
        if tagged.primary_tag().is_none() {
            let kind = tagged.primary_tag_type();
            tagged.insert_tag(lofty::tag::Tag::new(kind));
        }
        let tag = tagged.primary_tag_mut().ok_or("no tag")?;
        tag.set_title(t.title.clone());
        tag.set_artist(t.artist_line());
        if t.album.is_empty() {
            tag.remove_album();
        } else {
            tag.set_album(t.album.clone());
        }
        if let Some(b) = cover.filter(|b| b.len() > 100) {
            let mime = if b.starts_with(&[0x89, b'P', b'N', b'G']) { MimeType::Png } else { MimeType::Jpeg };
            tag.remove_picture_type(PictureType::CoverFront);
            tag.push_picture(Picture::new_unchecked(PictureType::CoverFront, Some(mime), None, b));
        }
        tag.save_to_path(&tmp, lofty::config::WriteOptions::default()).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

/// (number of files, total bytes)
pub fn stats() -> (usize, u64) {
    let mut n = 0;
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(dir()) {
        for e in rd.flatten() {
            if let Ok(m) = e.metadata() {
                if m.is_file() {
                    n += 1;
                    total += m.len();
                }
            }
        }
    }
    (n, total)
}

pub fn remove(id: i64) {
    if let Some(p) = path_for(id) {
        let _ = std::fs::remove_file(p);
    }
    if let Ok(mut s) = set().lock() {
        s.remove(&id);
    }
}

pub fn clear() {
    if let Ok(rd) = std::fs::read_dir(dir()) {
        for e in rd.flatten() {
            let _ = std::fs::remove_file(e.path());
        }
    }
    if let Ok(mut s) = set().lock() {
        s.clear();
    }
}

pub fn open_folder() {
    let d = dir();
    let _ = std::fs::create_dir_all(&d);
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(&d).spawn();
    }
    #[cfg(not(windows))]
    {
        let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        let _ = std::process::Command::new(opener).arg(&d).spawn();
    }
}

pub fn fmt_size(b: u64) -> String {
    if b >= 1 << 30 {
        format!("{:.1} GB", b as f64 / (1u64 << 30) as f64)
    } else if b >= 1 << 20 {
        format!("{:.0} MB", b as f64 / (1u64 << 20) as f64)
    } else {
        format!("{} KB", b / 1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A song's details written into a copy of a real stored file (BACKLINE_TAG_FILE=path, optional
    /// BACKLINE_TAG_COVER=picture): they read back, and the song still decodes to the very same sound.
    #[test]
    #[ignore]
    fn tags_a_stored_file() {
        let Ok(src) = std::env::var("BACKLINE_TAG_FILE") else { return };
        let dir = std::env::temp_dir().join("backline_tagtest");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(std::path::Path::new(&src).file_name().unwrap());
        std::fs::copy(&src, &path).unwrap();
        let before = crate::sources::decode_span(std::fs::read(&path).unwrap(), 0.0, 0.0).unwrap();
        let cover = std::env::var("BACKLINE_TAG_COVER").ok().and_then(|c| std::fs::read(c).ok());
        let had_cover = cover.is_some();
        let t = Track {
            id: 1,
            title: "Hero".into(),
            artist: "Jelee".into(),
            artists: vec![(1, "Jelee".into()), (2, "ekebabyy".into())],
            album: "Test Album".into(),
            ..Track::default()
        };
        write_tags(&path, &t, cover).unwrap();
        let (back, pic) = read_tags(&path, 1).unwrap();
        assert_eq!(back.title, "Hero");
        assert_eq!(back.artist, "Jelee & ekebabyy");
        assert_eq!(back.album, "Test Album");
        assert_eq!(pic, had_cover);
        let after = crate::sources::decode_span(std::fs::read(&path).unwrap(), 0.0, 0.0).unwrap();
        assert_eq!((before.1, before.2), (after.1, after.2));
        assert_eq!(before.0.len(), after.0.len(), "same length");
        assert!(before.0 == after.0, "same sound");
        // written again with the same details: nothing changes
        println!("ok: {} samples, {} Hz, cover {}", after.0.len(), after.2, pic);
    }

    #[test]
    fn sizes() {
        assert_eq!(fmt_size(0), "0 KB");
        assert_eq!(fmt_size(1536), "1 KB");
        assert_eq!(fmt_size(5 << 20), "5 MB");
        assert_eq!(fmt_size(3 << 29), "1.5 GB");
    }
}
