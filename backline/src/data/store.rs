//! Everything you build up while practicing, saved in `%APPDATA%\backline\library.json`:
//! your tune list (with the versions worth studying), named sections, the practice diary,
//! and the files / YouTube clips you added.

use crate::api::Track;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// The share file's format. Raise it when what a share file holds changes: an older Backline still reads what it
/// knows from a newer file (unknown parts are skipped) and says so; a newer Backline reads every older file.
pub const SHARE_VERSION: u32 = 1;

/// Your loops and playlists as one file, to take to another computer (no account: send the file however you like).
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct Share {
    /// "backline-share", so a stray file isn't read as one
    pub kind: String,
    /// the format it was written in (`SHARE_VERSION` then; 0 = before versions were kept, read as 1)
    pub version: u32,
    pub loops: HashMap<i64, Vec<Section>>,
    /// the songs the loops are on (title, artist, where it comes from), so they can be played there
    pub songs: HashMap<i64, Version>,
    pub playlists: Vec<Playlist>,
}

/// What an import added (loops, new playlists, songs put into playlists you already had), and what it found about
/// the songs: files matched to your own copies, files not on this computer, Tidal songs (they need a Tidal login).
#[derive(Debug, PartialEq, Default)]
pub struct Took {
    pub loops: usize,
    pub playlists: usize,
    pub items: usize,
    pub relinked: usize,
    pub missing: usize,
    pub tidal: usize,
}

/// The loop groups everyone starts with (rename or delete any of them).
pub const DEFAULT_GROUPS: [&str; 5] = ["Head", "Solo licks", "Comping", "Intro", "Ending"];

impl Store {
    /// Your loop groups: the ones you made, or the starting ones.
    pub fn groups(&self) -> Vec<String> {
        self.loop_groups.clone().unwrap_or_else(|| DEFAULT_GROUPS.iter().map(|g| g.to_string()).collect())
    }

    /// Rename a group, everywhere a loop is in it ("" removes it: its loops stay, in no group).
    pub fn rename_group(&mut self, old: &str, new: &str) {
        let new = new.trim();
        let mut list = self.groups();
        list.retain(|g| g != old);
        if !new.is_empty() && !list.iter().any(|g| g == new) {
            list.push(new.to_string());
        }
        self.loop_groups = Some(list);
        for s in self.sections.values_mut().flatten() {
            if s.group == old {
                s.group = new.to_string();
            }
        }
    }

    /// Rename a tune, everywhere it is named: the tune itself, the practice time kept under its name each day, the
    /// diary, your takes, the ROUTINE (tune of the week, skipped tunes, your set).
    pub fn rename_tune(&mut self, old: &str, new: &str) {
        let new = new.trim();
        if new.is_empty() || old == new {
            return;
        }
        for t in self.tunes.iter_mut().filter(|t| t.name == old) {
            t.name = new.to_string();
        }
        for d in &mut self.days {
            if let Some(s) = d.secs.remove(old) {
                *d.secs.entry(new.to_string()).or_insert(0) += s;
            }
        }
        for e in self.entries.iter_mut().filter(|e| e.tune == old) {
            e.tune = new.to_string();
        }
        for t in self.takes.iter_mut().filter(|t| t.tune == old) {
            t.tune = new.to_string();
        }
        if self.routine_tune == old {
            self.routine_tune = new.to_string();
        }
        for s in self.skipped_tunes.iter_mut().filter(|s| *s == old) {
            *s = new.to_string();
        }
        for v in self.routine_set.values_mut().filter(|v| v.eq_ignore_ascii_case(old)) {
            *v = new.to_string();
        }
    }

    /// Everything to share: every loop (with its song) and every playlist.
    pub fn share(&self) -> Share {
        let songs = self.sections.keys().filter_map(|id| self.songs.get(id).map(|v| (*id, v.clone()))).collect();
        Share {
            kind: "backline-share".to_string(),
            version: SHARE_VERSION,
            loops: self.sections.clone(),
            songs,
            playlists: self.playlists.clone(),
        }
    }

    /// What bringing in `s` would add, without changing anything (a trial run on copies of the parts it touches).
    pub fn preview_share(&self, s: &Share) -> Took {
        let mut trial = Store {
            sections: self.sections.clone(),
            songs: self.songs.clone(),
            playlists: self.playlists.clone(),
            files: self.files.clone(),
            yt: self.yt.clone(),
            sc: self.sc.clone(),
            ..Default::default()
        };
        trial.take_share(s.clone())
    }

    /// Bring in a shared file: loops you don't have yet (same song, same start and end) are added, songs you don't
    /// know are learned, a playlist with a new name is added whole, and one you already have gets the songs it
    /// lacks. Nothing of yours is changed or removed, so importing the same file twice adds nothing.
    pub fn take_share(&mut self, mut s: Share) -> Took {
        let mut took = Took::default();
        // every song in the file, once
        let mut all: HashMap<i64, Version> = s.songs.clone();
        for v in s.playlists.iter().flat_map(|p| &p.items) {
            all.entry(v.id).or_insert_with(|| v.clone());
        }
        // a file is known by where it is, which differs from computer to computer: matched to your own copy (same
        // file name, or same title and artist) it takes that copy's place everywhere in the file
        let mut moved: HashMap<i64, (i64, String)> = HashMap::new();
        for v in all.values() {
            match v.kind.as_str() {
                "file" if !std::path::Path::new(&v.src).exists() => {
                    // the file's own name, whichever system the path came from (a Mac doesn't split C:\ paths)
                    let name = |p: &str| p.rsplit(['/', '\\']).next().filter(|n| !n.is_empty()).map(|n| n.to_lowercase());
                    let same = |e: &&Ext| {
                        (name(&e.src).is_some() && name(&e.src) == name(&v.src))
                            || (!v.title.is_empty()
                                && e.title.eq_ignore_ascii_case(&v.title)
                                && e.artist.eq_ignore_ascii_case(&v.artist))
                    };
                    match self.files.iter().find(same) {
                        Some(e) => {
                            moved.insert(v.id, (e.id, e.src.clone()));
                            took.relinked += 1;
                        }
                        None => took.missing += 1,
                    }
                }
                // YouTube and SoundCloud: the same everywhere; put in your lists so they show and play
                "yt" | "sc" => {
                    let list = if v.kind == "yt" { &mut self.yt } else { &mut self.sc };
                    if !list.iter().any(|e| e.id == v.id) {
                        list.push(Ext {
                            kind: v.kind.clone(),
                            id: v.id,
                            src: v.src.clone(),
                            title: v.title.clone(),
                            artist: v.artist.clone(),
                            album: String::new(),
                            dur: v.dur,
                            cover: v.cover.clone(),
                        });
                    }
                }
                _ if v.id > 0 => took.tidal += 1,
                _ => {}
            }
        }
        let fix = |v: &mut Version| {
            if let Some((id, src)) = moved.get(&v.id) {
                v.id = *id;
                v.src = src.clone();
            }
        };
        s.loops = s.loops.into_iter().map(|(id, l)| (moved.get(&id).map_or(id, |m| m.0), l)).collect();
        s.songs = s
            .songs
            .into_iter()
            .map(|(id, mut v)| {
                fix(&mut v);
                (moved.get(&id).map_or(id, |m| m.0), v)
            })
            .collect();
        for v in s.playlists.iter_mut().flat_map(|p| p.items.iter_mut()) {
            fix(v);
        }
        for (id, loops) in s.loops {
            let mine = self.sections.entry(id).or_default();
            for l in loops {
                if !mine.iter().any(|m| (m.a - l.a).abs() < 0.05 && (m.b - l.b).abs() < 0.05) {
                    mine.push(l);
                    took.loops += 1;
                }
            }
        }
        for (id, v) in s.songs {
            self.songs.entry(id).or_insert(v);
        }
        let same = |a: &Version, b: &Version| a.id == b.id && a.loop_ab == b.loop_ab;
        for p in s.playlists {
            match self.playlists.iter_mut().find(|m| m.name == p.name) {
                Some(m) => {
                    for v in p.items {
                        if !m.items.iter().any(|x| same(x, &v)) {
                            m.items.push(v);
                            took.items += 1;
                        }
                    }
                }
                None => {
                    self.playlists.push(p);
                    took.playlists += 1;
                }
            }
        }
        took
    }
}

/// A playlist you built inside the player: any mix of Tidal, SoundCloud, YouTube and file tracks.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Playlist {
    pub name: String,
    pub items: Vec<Version>,
}

/// A playlist link kept for later: SoundCloud or YouTube (`kind` = "sc" | "yt").
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct SavedList {
    pub kind: String,
    pub name: String,
    pub url: String,
    /// its songs as last read, so it opens at once (REFRESH reads it again)
    pub items: Vec<Ext>,
}

/// A recording of a tune: from Tidal, a file on this computer, or a YouTube clip.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Version {
    pub kind: String, // "tidal" | "file" | "yt"
    pub id: i64,
    pub src: String, // file path or YouTube video id
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover: String,
    pub dur: f32,
    pub stars: u8,
    pub note: String,
    /// a loop of the song rather than all of it (a playlist can hold loops): where it starts and ends, its name
    pub loop_ab: Option<(f32, f32)>,
    pub loop_name: String,
}

/// A file's cover: the picture inside it (or beside it), found by the image loader from "file:<path>".
fn file_cover(kind: &str, src: &str, cover: &str) -> String {
    if kind == "file" && cover.is_empty() {
        format!("file:{}", src)
    } else {
        cover.to_string()
    }
}

impl Version {
    pub fn to_track(&self) -> Track {
        Track {
            id: self.id,
            title: self.title.clone(),
            artist: self.artist.clone(),
            artists: Vec::new(),
            album: self.album.clone(),
            cover: file_cover(&self.kind, &self.src, &self.cover),
            duration: self.dur,
            qid: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Tune {
    pub name: String,
    /// 0 learning, 1 working on it, 2 ready to play
    pub status: u8,
    pub key: String,
    pub bpm: u32,
    pub notes: String,
    /// lead sheet as plain text, e.g. `T44 *A | Dm7 G7 | Cmaj7 | ...`
    pub chart: String,
    /// older versions kept "X - written by Y" here; moved into `written_by` when loaded
    #[serde(skip_serializing)]
    pub info: String,
    /// who wrote it (looked up for you; yours to fix)
    pub written_by: String,
    /// the year it was written or first recorded
    pub year: String,
    /// the show, film or album it comes from
    pub from: String,
    /// swing, ballad, bossa...
    pub style: String,
    pub versions: Vec<Version>,
}

impl Tune {
    /// The writer out of an old "Night and Day - written by Cole Porter" line.
    pub fn take_old_info(&mut self) {
        let info = std::mem::take(&mut self.info);
        if !self.written_by.is_empty() {
            return;
        }
        let lower = info.to_lowercase();
        if let Some(at) = lower.find("written by ") {
            self.written_by = info[at + 11..].lines().next().unwrap_or("").trim().to_string();
        }
    }

    /// Who wrote it, when and where from, in one line ("Cole Porter, 1932, from Gay Divorce").
    pub fn credit_line(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !self.written_by.is_empty() {
            parts.push(format!("Written by {}", self.written_by));
        }
        if !self.year.is_empty() {
            parts.push(self.year.clone());
        }
        if !self.from.is_empty() {
            parts.push(format!("from {}", self.from));
        }
        parts.join(", ")
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Section {
    pub name: String,
    pub a: f32,
    pub b: f32,
    /// the day it was saved (empty for loops saved before this was kept)
    pub saved: String,
    /// your note on it (what it is, what to work on)
    pub note: String,
    /// the group it's kept in within the song ("solo licks", "head"...), empty for none
    pub group: String,
    /// starred: one that matters to you (a gig coming up, a part not to forget)
    pub fav: bool,
}

/// A bookmark on a song's waveform: where, its colour, and a name if you gave it one.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Bookmark {
    pub t: f32,
    pub col: [u8; 3],
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Day {
    pub date: String,
    pub secs: BTreeMap<String, u32>,
    pub loops: u32,
    pub pomos: u32,
    /// sight-reading lines played along to the end
    pub reads: u32,
    /// practice time on each song that day (by track id), tune or not
    pub songs: BTreeMap<i64, u32>,
    /// each loop worked on that day ("track id:start ms:end ms"): seconds on it and passes through it
    pub loop_time: BTreeMap<String, (u32, u32)>,
}

/// Whether two loops of a song are really the same one nudged a little: they share most of their length.
fn nudged(a: (i64, f32, f32), b: (i64, f32, f32)) -> bool {
    let inter = (a.2.min(b.2) - a.1.max(b.1)).max(0.0);
    let uni = a.2.max(b.2) - a.1.min(b.1);
    a.0 == b.0 && uni > 0.0 && inter / uni >= 0.6
}

impl Day {
    /// The current loop's ends moved: what was kept under its old ends carried over to the new ones.
    pub fn move_loop(&mut self, old: &str, new: &str) {
        if let Some(v) = self.loop_time.remove(old) {
            let e = self.loop_time.entry(new.to_string()).or_insert((0, 0));
            e.0 += v.0;
            e.1 += v.1;
        }
    }

    /// Loops kept as several because they were nudged, made one again: the time and passes added up, under the
    /// ends of the one worked on longest.
    pub fn merge_nudged(&mut self) {
        loop {
            let keys: Vec<(String, (i64, f32, f32), u32)> =
                self.loop_time.iter().filter_map(|(k, v)| parse_loop_key(k).map(|p| (k.clone(), p, v.0))).collect();
            let pair = keys
                .iter()
                .enumerate()
                .find_map(|(i, a)| keys[i + 1..].iter().find(|b| nudged(a.1, b.1)).map(|b| (a.clone(), b.clone())));
            let Some((a, b)) = pair else { break };
            let (keep, gone) = if a.2 >= b.2 { (a.0, b.0) } else { (b.0, a.0) };
            let g = self.loop_time.remove(&gone).unwrap_or_default();
            let e = self.loop_time.entry(keep).or_insert((0, 0));
            e.0 += g.0;
            e.1 += g.1;
        }
    }
}

/// The key a loop's practice is kept under in a day: the song and where the loop starts and ends.
pub fn loop_key(id: i64, a: f32, b: f32) -> String {
    format!("{}:{}:{}", id, (a * 1000.0).round() as i64, (b * 1000.0).round() as i64)
}

/// A loop key back as (song, start, end).
pub fn parse_loop_key(k: &str) -> Option<(i64, f32, f32)> {
    let mut it = k.split(':');
    let id = it.next()?.parse().ok()?;
    let a: i64 = it.next()?.parse().ok()?;
    let b: i64 = it.next()?.parse().ok()?;
    Some((id, a as f32 / 1000.0, b as f32 / 1000.0))
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Entry {
    pub date: String,
    pub tune: String,
    pub mins: u32,
    pub bpm: u32,
    pub note: String,
}

/// A file or YouTube clip in the FILES / YT lists.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Ext {
    pub kind: String, // "file" | "yt"
    pub id: i64,
    pub src: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub dur: f32,
    pub cover: String,
}

/// A take you recorded (RECORD): you over what Backline played, and you alone, as WAVs under takes/.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Take {
    /// the mixed file's id (as a file track) and path; the file of you alone
    pub id: i64,
    pub path: String,
    pub dry: String,
    /// the tune it went with ("" for free play), when ("2026-10-20 1435"), how long, and a tag of yours
    pub tune: String,
    pub stamp: String,
    pub secs: f32,
    pub tag: String,
}

impl Take {
    /// The take as a song to play: the mix, or you alone.
    pub fn version(&self, dry: bool) -> Version {
        let src = if dry { self.dry.clone() } else { self.path.clone() };
        Version {
            kind: "file".to_string(),
            id: if dry { hash_id(&format!("file:{}", src)) } else { self.id },
            src,
            title: format!("TAKE {}{}", self.stamp, if dry { " (YOU ALONE)" } else { "" }),
            artist: "YOU".to_string(),
            album: if self.tune.is_empty() { "FREE PLAY".to_string() } else { self.tune.clone() },
            dur: self.secs,
            ..Default::default()
        }
    }
}

impl Ext {
    pub fn to_track(&self) -> Track {
        Track {
            id: self.id,
            title: self.title.clone(),
            artist: self.artist.clone(),
            artists: Vec::new(),
            album: self.album.clone(),
            cover: file_cover(&self.kind, &self.src, &self.cover),
            duration: self.dur,
            qid: 0,
        }
    }

    pub fn to_version(&self) -> Version {
        Version {
            kind: self.kind.clone(),
            id: self.id,
            src: self.src.clone(),
            title: self.title.clone(),
            artist: self.artist.clone(),
            album: self.album.clone(),
            cover: self.cover.clone(),
            dur: self.dur,
            ..Default::default()
        }
    }
}

/// What was playing when the app was closed ("Continue where I left off").
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Last {
    pub version: Version,
    pub pos: f32,
    pub speed: u32,
    /// the whole queue it was in, and its place there (so the queue comes back after a restart or an update)
    pub queue: Vec<Version>,
    pub index: usize,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Store {
    pub tunes: Vec<Tune>,
    pub sections: HashMap<i64, Vec<Section>>,
    /// bookmarks along each song's waveform (by track id), in time order
    pub marks: HashMap<i64, Vec<Bookmark>>,
    pub days: Vec<Day>,
    pub entries: Vec<Entry>,
    pub folders: Vec<String>,
    pub files: Vec<Ext>,
    pub yt: Vec<Ext>,
    /// SoundCloud tracks you kept (same kind of entry as a YouTube clip, with a link instead of a video id)
    pub sc: Vec<Ext>,
    /// Hearted tracks that are not on Tidal (files, YouTube, SoundCloud); kept on this computer only
    pub hearts: Vec<Ext>,
    /// SoundCloud / YouTube playlists you saved by link
    pub lists: Vec<SavedList>,
    pub playlists: Vec<Playlist>,
    pub bpm: HashMap<i64, u32>,
    /// tempo and key found by listening to the audio (see meta.rs)
    pub meta: HashMap<i64, crate::meta::Info>,
    pub last: Option<Last>,
    /// what you played in Backline, newest first: the song and when it started (seconds since 1970); the last 500
    pub recent: Vec<(Version, u64)>,
    /// LINES: the tempo each line was last played at, and your favorites
    pub lines_tempo: HashMap<String, u32>,
    pub lines_stars: Vec<String>,
    /// practice progressions you typed in (Roman numerals)
    pub my_progs: Vec<String>,
    /// lines you typed in from a book: (label, notes)
    pub my_lines: Vec<(String, String)>,
    /// the songs you saved loops on or practised, so LOOPS and the diary can name and play them
    pub songs: HashMap<i64, Version>,
    /// READING > THE BOOK: the exercises you have done ("page:name") and the day you marked each
    pub reading_done: HashMap<String, String>,
    /// voicings you kept (CLUSTERS > MY CHORDS)
    pub my_chords: Vec<MyChord>,
    /// grips you added to a chord yourself (by `chords::grip_key`): shown first, marked MINE, removable
    pub custom_grips: HashMap<String, Vec<[Option<u8>; 6]>>,
    /// your loop groups (None until you change them: the starting ones)
    pub loop_groups: Option<Vec<String>>,
    /// RECORD: your takes, newest last
    pub takes: Vec<Take>,
    /// ROUTINE: the tune of the week, the day it was picked, how many have been picked (the kinds take turns),
    /// jazz or gospel, tunes skipped, the steps done each day, each step's minutes, steps turned off
    pub routine_tune: String,
    pub routine_since: String,
    pub routine_count: u32,
    pub routine_gospel: bool,
    pub skipped_tunes: Vec<String>,
    pub routine_done: HashMap<String, Vec<u8>>,
    pub routine_mins: Vec<u32>,
    pub routine_off: Vec<u8>,
    /// ROUTINE: your set - the tune you are working on for each kind ("BLUES" -> "Blue Monk")
    pub routine_set: HashMap<String, String>,
    /// LISTENING: close listens per record ("artist - album")
    pub listened: HashMap<String, u32>,
    /// EAR: each drill's record (by the drill's key)
    pub ear: HashMap<String, EarStats>,
    /// CHORD QUALITIES: what's in play and how the chords are played
    pub ear_chords: EarChords,
    /// ear training: no chime after an answer
    pub ear_quiet: bool,
}

/// The CHORD QUALITIES drill's choices: what's in play (0 levels, 1 look-alikes, 2 my set), the levels ticked and
/// how many of each level's chords are in play (0: the first four), the look-alike set (past the last: your
/// mix-ups), the chords of MY SET (by label), and the voicing (an index of `ear::VOICINGS`), the same root each
/// time, no bass note, the top-note question, the ii-V before the chord.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct EarChords {
    pub mode: u8,
    pub levels: Vec<usize>,
    pub grown: Vec<usize>,
    pub alike: usize,
    pub mine: Vec<String>,
    pub voicing: u8,
    pub same_root: bool,
    pub no_bass: bool,
    pub top: bool,
    pub context: bool,
}

impl Default for EarChords {
    fn default() -> Self {
        EarChords {
            mode: 0,
            levels: vec![1],
            grown: Vec::new(),
            alike: 0,
            mine: Vec::new(),
            voicing: 5,
            same_root: false,
            no_bass: false,
            top: false,
            context: false,
        }
    }
}

/// One ear-training drill's record: how many answers are in play, each answer asked and got right, which answer
/// you took for which ("asked>given"), and each finished session (day number, right, asked).
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct EarStats {
    pub level: usize,
    pub asked: HashMap<String, u32>,
    pub right: HashMap<String, u32>,
    pub mixups: HashMap<String, u32>,
    pub sessions: Vec<(i64, u32, u32)>,
    /// the most answers right in a row
    pub best: u32,
}

/// A voicing you kept: its name and its grip (a fret per string, low E first; None = not played).
#[derive(Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(default)]
pub struct MyChord {
    pub name: String,
    pub grip: [Option<u8>; 6],
}

fn path() -> PathBuf {
    crate::api::config_dir().join("library.json")
}

impl Store {
    pub fn load() -> Store {
        let mut s: Store = std::fs::read_to_string(path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        for t in &mut s.tunes {
            t.take_old_info();
        }
        // a loop nudged a little was kept as a new one each time: those made one again
        for d in &mut s.days {
            d.merge_nudged();
        }
        s
    }

    pub fn save(&self) {
        let dir = crate::api::config_dir();
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(t) = serde_json::to_string(self) {
            let tmp = dir.join("library.json.tmp");
            if std::fs::write(&tmp, t).is_ok() {
                let _ = std::fs::rename(&tmp, path());
            }
        }
    }

    pub fn day_mut(&mut self, date: &str) -> &mut Day {
        if let Some(i) = self.days.iter().position(|d| d.date == date) {
            return &mut self.days[i];
        }
        self.days.push(Day { date: date.to_string(), ..Default::default() });
        let n = self.days.len();
        &mut self.days[n - 1]
    }

    pub fn secs_on(&self, date: &str) -> u32 {
        self.days.iter().find(|d| d.date == date).map(|d| d.secs.values().sum()).unwrap_or(0)
    }

    pub fn tune_of(&self, id: i64) -> Option<usize> {
        self.tunes.iter().position(|t| t.versions.iter().any(|v| v.id == id))
    }

    pub fn tune_secs(&self, name: &str) -> u32 {
        self.days.iter().map(|d| d.secs.get(name).copied().unwrap_or(0)).sum()
    }
}

// ------------------------------------------------------------------ dates
/// Local calendar date (year, month, day).
pub fn today() -> (i32, u32, u32) {
    let (y, m, d, ..) = now_local();
    (y, m, d)
}

/// Local date and time: (year, month, day, hour, minute, second).
#[cfg(windows)]
pub fn now_local() -> (i32, u32, u32, u32, u32, u32) {
    #[repr(C)]
    struct St {
        y: u16,
        mo: u16,
        dow: u16,
        d: u16,
        h: u16,
        mi: u16,
        s: u16,
        ms: u16,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetLocalTime(st: *mut St);
    }
    unsafe {
        let mut st: St = std::mem::zeroed();
        GetLocalTime(&mut st);
        (st.y as i32, st.mo as u32, st.d as u32, st.h as u32, st.mi as u32, st.s as u32)
    }
}

/// Mac / Linux: ask the system for the local time (so the diary rolls over at local midnight, not UTC midnight).
#[cfg(not(windows))]
pub fn now_local() -> (i32, u32, u32, u32, u32, u32) {
    #[repr(C)]
    struct Tm {
        sec: i32,
        min: i32,
        hour: i32,
        mday: i32,
        mon: i32,
        year: i32,
        wday: i32,
        yday: i32,
        isdst: i32,
        gmtoff: i64,
        zone: *const u8,
    }
    extern "C" {
        fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
    }
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let mut tm =
        Tm { sec: 0, min: 0, hour: 0, mday: 0, mon: 0, year: 0, wday: 0, yday: 0, isdst: 0, gmtoff: 0, zone: std::ptr::null() };
    let ok = unsafe { !localtime_r(&secs, &mut tm).is_null() };
    if ok && tm.mday > 0 {
        (tm.year + 1900, (tm.mon + 1) as u32, tm.mday as u32, tm.hour as u32, tm.min as u32, tm.sec as u32)
    } else {
        let (y, m, d) = civil_from_days(secs.div_euclid(86400));
        let t = secs.rem_euclid(86400) as u32;
        (y, m, d, t / 3600, t / 60 % 60, t % 60)
    }
}

pub fn today_str() -> String {
    let (y, m, d) = today();
    format!("{:04}-{:02}-{:02}", y, m, d)
}

pub fn today_days() -> i64 {
    let (y, m, d) = today();
    days_from_civil(y, m, d)
}

/// Days since 1970-01-01.
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y as i64 - 1 } else { y as i64 };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m: u32 = if mp < 10 { (mp + 3) as u32 } else { (mp - 9) as u32 };
    let yy: i64 = if m <= 2 { y + 1 } else { y };
    (yy as i32, m, d)
}

#[cfg(test)]
pub fn parse_date(s: &str) -> Option<i64> {
    let mut it = s.split('-');
    let y = it.next()?.parse::<i32>().ok()?;
    let m = it.next()?.parse::<u32>().ok()?;
    let d = it.next()?.parse::<u32>().ok()?;
    Some(days_from_civil(y, m, d))
}

pub fn date_str(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

pub fn weekday(days: i64) -> &'static str {
    ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"][((days + 4).rem_euclid(7)) as usize]
}

/// Stable id for a file path / YouTube clip. Always negative so it can't clash with Tidal ids.
pub fn hash_id(s: &str) -> i64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    -((h & 0x3fff_ffff_ffff_ffff) as i64) - 1
}

#[cfg(test)]
mod share_tests {
    use super::*;

    #[test]
    fn a_nudged_loop_stays_one_loop() {
        let mut d = Day::default();
        d.loop_time.insert(loop_key(7, 238.0, 242.0), (30, 2));
        // the current loop's end moved: its time goes with it
        d.move_loop(&loop_key(7, 238.0, 242.0), &loop_key(7, 238.0, 243.0));
        assert_eq!(d.loop_time.len(), 1);
        assert_eq!(d.loop_time.get(&loop_key(7, 238.0, 243.0)), Some(&(30, 2)));
        // ones kept apart before are made one
        let mut old = Day::default();
        old.loop_time.insert(loop_key(7, 238.0, 242.0), (35, 3));
        old.loop_time.insert(loop_key(7, 238.0, 243.0), (10, 1));
        old.loop_time.insert(loop_key(7, 0.0, 9.0), (2, 0));
        old.merge_nudged();
        assert_eq!(old.loop_time.len(), 2);
        assert_eq!(old.loop_time.get(&loop_key(7, 238.0, 242.0)), Some(&(45, 4)));
    }

    #[test]
    fn sharing_adds_only_what_is_new() {
        let mut a = Store::default();
        a.sections.insert(1, vec![Section { name: "head".into(), a: 10.0, b: 20.0, ..Default::default() }]);
        a.songs.insert(1, Version { id: 1, title: "Song".into(), ..Default::default() });
        a.playlists.push(Playlist { name: "Videos".into(), items: vec![Version { id: 1, ..Default::default() }] });
        let mut b = Store::default();
        b.sections.insert(1, vec![Section { name: "head".into(), a: 10.0, b: 20.0, ..Default::default() }]);
        b.playlists.push(Playlist { name: "Videos".into(), items: Vec::new() });
        let took = b.take_share(a.share());
        assert_eq!(took, Took { loops: 0, playlists: 0, items: 1, tidal: 1, ..Default::default() });
        assert_eq!(b.songs.get(&1).map(|v| v.title.as_str()), Some("Song"));
        // the file passes through JSON, and a second import adds nothing
        let text = serde_json::to_string(&a.share()).unwrap();
        let again: Share = serde_json::from_str(&text).unwrap();
        assert_eq!(again.version, SHARE_VERSION);
        let t = b.take_share(again);
        assert_eq!((t.loops, t.playlists, t.items), (0, 0, 0));
    }

    #[test]
    fn a_tune_renamed_keeps_everything_kept_under_its_name() {
        let mut s = Store::default();
        s.tunes.push(Tune { name: "Incompatibilidade de".into(), ..Default::default() });
        s.day_mut("2026-10-01").secs.insert("Incompatibilidade de".into(), 60);
        s.day_mut("2026-10-01").secs.insert("Incompatibilidade de Gênios".into(), 30);
        s.entries.push(Entry { tune: "Incompatibilidade de".into(), ..Default::default() });
        s.takes.push(Take { tune: "Incompatibilidade de".into(), ..Default::default() });
        s.routine_tune = "Incompatibilidade de".into();
        s.skipped_tunes.push("Incompatibilidade de".into());
        s.routine_set.insert("BOSSA".into(), "Incompatibilidade de".into());
        s.rename_tune("Incompatibilidade de", "Incompatibilidade de Gênios");
        let new = "Incompatibilidade de Gênios";
        assert_eq!(s.tunes[0].name, new);
        assert_eq!(s.tune_secs(new), 90);
        assert_eq!(s.tune_secs("Incompatibilidade de"), 0);
        assert_eq!((s.entries[0].tune.as_str(), s.takes[0].tune.as_str()), (new, new));
        assert_eq!((s.routine_tune.as_str(), s.skipped_tunes[0].as_str()), (new, new));
        assert_eq!(s.routine_set["BOSSA"], new);
    }

    #[test]
    fn groups_rename_and_go_everywhere() {
        let mut s = Store::default();
        assert_eq!(s.groups().len(), DEFAULT_GROUPS.len());
        s.sections.insert(1, vec![Section { name: "a".into(), group: "Head".into(), ..Default::default() }]);
        s.rename_group("Head", "Melody");
        assert!(s.groups().contains(&"Melody".to_string()) && !s.groups().contains(&"Head".to_string()));
        assert_eq!(s.sections[&1][0].group, "Melody");
        s.rename_group("Melody", "");
        assert!(!s.groups().contains(&"Melody".to_string()));
        assert_eq!(s.sections[&1][0].group, "");
    }

    #[test]
    fn a_file_is_matched_to_your_own_copy() {
        let mut a = Store::default();
        let theirs = Version {
            kind: "file".into(),
            id: -5,
            src: r"Z:\nowhere\Funky Riff.mp3".into(),
            title: "Funky Riff".into(),
            ..Default::default()
        };
        a.sections.insert(-5, vec![Section { name: "riff".into(), a: 1.0, b: 2.0, ..Default::default() }]);
        a.songs.insert(-5, theirs.clone());
        let mut b = Store::default();
        b.files.push(Ext { kind: "file".into(), id: -9, src: r"C:\music\funky riff.mp3".into(), ..Default::default() });
        let took = b.take_share(a.share());
        assert_eq!((took.relinked, took.missing, took.loops), (1, 0, 1));
        assert!(b.sections.contains_key(&-9) && !b.sections.contains_key(&-5));
        // a newer file with parts this one doesn't know still reads
        let newer = r#"{"kind":"backline-share","version":99,"loops":{},"playlists":[],"future_thing":[1,2]}"#;
        let s: Share = serde_json::from_str(newer).unwrap();
        assert_eq!(s.version, 99);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_keys_round_trip() {
        let k = loop_key(-123, 12.345, 67.8);
        let (id, a, b) = parse_loop_key(&k).unwrap();
        assert_eq!(id, -123);
        assert!((a - 12.345).abs() < 0.001 && (b - 67.8).abs() < 0.001);
        assert!(parse_loop_key("nonsense").is_none());
    }

    #[test]
    fn old_writer_lines_move_over() {
        let mut t = Tune { info: "Night and Day - written by Cole Porter".into(), ..Default::default() };
        t.take_old_info();
        assert_eq!(t.written_by, "Cole Porter");
        t.year = "1932".into();
        t.from = "Gay Divorce".into();
        assert_eq!(t.credit_line(), "Written by Cole Porter, 1932, from Gay Divorce");
    }

    #[test]
    fn calendar_round_trip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11017);
        assert_eq!(date_str(0), "1970-01-01");
        // every day for ~60 years survives the trip, leap days included
        for z in -3650..20000 {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(date_str(parse_date("2024-02-29").unwrap()), "2024-02-29");
        assert_eq!(parse_date("2024-03-01").unwrap() - parse_date("2024-02-28").unwrap(), 2);
        assert_eq!(parse_date("2023-03-01").unwrap() - parse_date("2023-02-28").unwrap(), 1);
        assert_eq!(parse_date("garbage"), None);
        assert_eq!(parse_date("2024-02"), None);
    }

    #[test]
    fn weekdays() {
        assert_eq!(weekday(0), "THU"); // 1970-01-01
        assert_eq!(weekday(parse_date("2026-10-09").unwrap()), "FRI");
        assert_eq!(weekday(-1), "WED");
    }

    #[test]
    fn ids_for_files_never_clash_with_tidal() {
        for s in ["", "a", "C:/music/song.flac", "yt:dQw4w9WgXcQ"] {
            assert!(hash_id(s) < 0, "{}", s);
            assert_eq!(hash_id(s), hash_id(s), "stable");
        }
        assert_ne!(hash_id("a"), hash_id("b"));
    }

    #[test]
    fn old_library_files_still_load() {
        // fields added later default instead of failing the whole file
        let s: Store = serde_json::from_str(r#"{"folders": ["C:/music"], "bpm": {"5": 120}}"#).unwrap();
        assert_eq!(s.folders, vec!["C:/music".to_string()]);
        assert_eq!(s.bpm.get(&5), Some(&120));
        assert!(s.meta.is_empty() && s.playlists.is_empty());
    }
}
