//! Backline's own transcription files: a tune as a small readable JSON file (one per tune, `Artist - Title.json`,
//! with the recording beside it as `.mp3` when there is one) in the `transcriptions` folder of Backline's data.
//! Guitar Pro files are read once and turned into these; after that only these are read, so they can be kept,
//! backed up or sent on without the originals.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The version of the file format (older files keep loading as fields are added).
pub const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Tune {
    pub format: u32,
    pub title: String,
    pub artist: String,
    /// where it came from ("Guitar Pro", or by hand)
    pub source: String,
    /// quarter notes a minute, as written
    pub tempo: f32,
    pub bars: Vec<Bar>,
    /// the chord grids the transcription draws: name and fret per string, low E first (-1 not played)
    pub grips: Vec<Grip>,
    /// the recording's file name, beside this one (empty: none)
    pub audio: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Bar {
    /// beats a bar, and the note that gets the beat
    pub time: (u32, u32),
    /// the key signature, when the file gives one: sharps (+) or flats (-)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<i8>,
    pub beats: Vec<Beat>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Beat {
    /// start and length in quarter notes, from the top of the bar
    pub at: f32,
    pub len: f32,
    /// each note as [pitch (MIDI), string (0 = low E), fret, tied (1) or struck (0)]
    pub notes: Vec<[i32; 4]>,
    /// the chord name written over it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chord: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Grip {
    pub name: String,
    pub frets: [i8; 6],
}

/// A move kept from a transcription: its name, its kind (see the MOVES list), the tune and bars it came from,
/// its beats (starts counted from the first), starred or not, and a note.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Move {
    pub name: String,
    pub kind: usize,
    pub tune: String,
    pub bars: (usize, usize),
    /// where it is in the tune: (bar, beat) from and to, counted from 0 - to mark it on the song
    pub spots: Option<((usize, usize), (usize, usize))>,
    pub beats: Vec<Beat>,
    /// the grips its chords are drawn with
    pub grips: Vec<Grip>,
    pub star: bool,
    pub note: String,
}

/// The moves you kept (in `moves.json` beside the transcriptions).
pub fn load_moves() -> Vec<Move> {
    std::fs::read(dir().join("moves.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn save_moves(m: &[Move]) {
    let _ = std::fs::create_dir_all(dir());
    if let Ok(t) = serde_json::to_string(m) {
        let _ = std::fs::write(dir().join("moves.json"), t);
    }
}

/// A spotted move you removed: the tune, its kind, and where it was ((bar, beat) from and to).
pub type Hidden = (String, usize, (usize, usize), (usize, usize));

/// The spotted moves you removed (in Backline's folder, not with the transcriptions you might send).
pub fn load_hidden() -> Vec<Hidden> {
    std::fs::read(crate::api::config_dir().join("hidden_moves.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save_hidden(h: &[Hidden]) {
    if let Ok(t) = serde_json::to_string(h) {
        let _ = std::fs::write(crate::api::config_dir().join("hidden_moves.json"), t);
    }
}

/// The folder the transcriptions live in.
pub fn dir() -> PathBuf {
    crate::api::config_dir().join("transcriptions")
}

/// A name safe for a file.
fn file_name(t: &Tune) -> String {
    let raw = if t.artist.trim().is_empty() { t.title.clone() } else { format!("{} - {}", t.artist.trim(), t.title.trim()) };
    let clean: String = raw.chars().map(|c| if "\\/:*?\"<>|".contains(c) { '-' } else { c }).collect();
    let clean = clean.trim().trim_end_matches('.').to_string();
    if clean.is_empty() {
        "Untitled".to_string()
    } else {
        clean
    }
}

/// A tune from a Guitar Pro file (`fallback` names it when the file doesn't: the file's own name). The recording,
/// if any, comes back alongside.
pub fn from_gp(bytes: &[u8], fallback: &str) -> Result<(Tune, Option<Vec<u8>>), String> {
    let s = crate::gp::read(bytes)?;
    // "Artist - Title" from the file name when the score leaves them out
    let (fa, ft) = fallback.split_once(" - ").map_or(("", fallback), |(a, b)| (a, b));
    let tune = Tune {
        format: FORMAT,
        title: if s.title.trim().is_empty() { ft.trim().to_string() } else { s.title.trim().to_string() },
        artist: if s.artist.trim().is_empty() { fa.trim().to_string() } else { s.artist.trim().to_string() },
        source: "Guitar Pro".to_string(),
        tempo: s.tempo,
        bars: s
            .bars
            .iter()
            .map(|b| Bar {
                time: b.time,
                key: Some(b.key.clamp(-7, 7) as i8),
                beats: b
                    .beats
                    .iter()
                    .map(|x| Beat {
                        at: x.start,
                        len: x.len,
                        notes: x.notes.iter().map(|n| [n.midi, n.string as i32, n.fret as i32, n.tied as i32]).collect(),
                        chord: x.chord.clone(),
                    })
                    .collect(),
            })
            .collect(),
        grips: s
            .chords
            .iter()
            .filter(|c| c.frets.iter().any(|f| f.is_some()))
            .map(|c| Grip { name: c.name.clone(), frets: c.frets.map(|f| f.map_or(-1, |v| v as i8)) })
            .collect(),
        audio: String::new(),
    };
    Ok((tune, s.audio))
}

/// Keep a tune (and its recording) in the folder, in `folder` (the transcriptions folder when None). Returns
/// the file written.
pub fn save_in(folder: Option<&std::path::Path>, mut t: Tune, audio: Option<&[u8]>) -> Result<PathBuf, String> {
    let d = folder.map(|p| p.to_path_buf()).unwrap_or_else(dir);
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    // a different tune by the same name (another transcription of it): "... (2)"; the same one again replaces it
    let base = file_name(&t);
    let mut name = base.clone();
    let mut k = 2;
    while let Ok(old) = load(&d.join(format!("{}.json", name))) {
        if old.bars.len() == t.bars.len() && old.source == t.source && old.audio.is_empty() == audio.is_none() {
            break;
        }
        name = format!("{} ({})", base, k);
        k += 1;
    }
    if let Some(a) = audio {
        let an = format!("{}.mp3", name);
        std::fs::write(d.join(&an), a).map_err(|e| e.to_string())?;
        t.audio = an;
    }
    t.format = FORMAT;
    let path = d.join(format!("{}.json", name));
    let text = serde_json::to_string(&t).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Every tune in the folder: (file, title as listed).
pub fn list() -> Vec<(PathBuf, String)> {
    let mut out: Vec<(PathBuf, String)> = std::fs::read_dir(dir())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("json")))
        .filter(|p| p.file_name().is_some_and(|n| n != "moves.json"))
        .map(|p| {
            let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            (p, name)
        })
        .collect();
    out.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
    out
}

/// A tune from its file.
pub fn load(path: &std::path::Path) -> Result<Tune, String> {
    let b = std::fs::read(path).map_err(|e| e.to_string())?;
    let t: Tune = serde_json::from_slice(&b).map_err(|e| format!("not a Backline transcription: {}", e))?;
    if t.bars.is_empty() {
        return Err("this transcription has no bars".to_string());
    }
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tune_round_trips_through_its_file() {
        let t = Tune {
            format: FORMAT,
            title: "Blues".into(),
            artist: "Me".into(),
            tempo: 100.0,
            bars: vec![Bar {
                key: None,
                time: (4, 4),
                beats: vec![Beat { at: 0.0, len: 1.0, notes: vec![[48, 1, 3, 0]], chord: Some("C7".into()) }],
            }],
            grips: vec![Grip { name: "C7".into(), frets: [-1, 3, 2, 3, 1, -1] }],
            ..Default::default()
        };
        let d = std::env::temp_dir().join(format!("backline_tr_test_{}", std::process::id()));
        let p = save_in(Some(&d), t.clone(), None).unwrap();
        assert!(p.ends_with("Me - Blues.json"));
        let back = load(&p).unwrap();
        assert_eq!(back.bars[0].beats[0].notes, vec![[48, 1, 3, 0]]);
        assert_eq!(back.grips[0].frets, [-1, 3, 2, 3, 1, -1]);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// The owner's Guitar Pro files turned into transcriptions in a scratch folder (not the normal test run).
    #[test]
    #[ignore]
    fn converts_the_collections() {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        // into the project's share folder (kept on this computer only, never in git)
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("share").join("transcriptions");
        let mut todo: Vec<std::path::PathBuf> = [
            "Joe-Pass-Collection-GP-Files",
            "Fingerstyle-Jazz-Guitar-Collection-GP-Files-5iydej",
            "Best-of-Fingerstyle-Jazz-Guitar",
        ]
        .iter()
        .map(|d| std::path::PathBuf::from(format!("{}\\Downloads\\{}", home, d)))
        .collect();
        let mut files: Vec<std::path::PathBuf> = Vec::new();
        while let Some(d) = todo.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    todo.push(p);
                } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("gp")) {
                    files.push(p);
                }
            }
        }
        for f in files {
            let stem = f.file_stem().unwrap().to_string_lossy().to_string();
            let (t, a) = from_gp(&std::fs::read(&f).unwrap(), &stem).unwrap();
            let p = save_in(Some(&out), t, a.as_deref()).unwrap();
            let back = load(&p).unwrap();
            eprintln!(
                "{:60} {:7} KB  {} bars",
                p.file_name().unwrap().to_string_lossy(),
                std::fs::metadata(&p).unwrap().len() / 1024,
                back.bars.len()
            );
        }
    }
}
