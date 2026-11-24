//! THE CHORD BOOK: the owner's notes on Scott McGill's "The Chord Book" videos, voicing by voicing, string set by
//! string set. It is read from a file kept on this computer only (chord_book.txt in Backline's folder; it is never
//! part of Backline, and it syncs to a linked computer like the other books). Each voicing is written as its notes,
//! low to high; the tab, the degrees over the family's root, a chord symbol and the other names it goes by are
//! worked out here, with the CHORDS analyzer.
//!
//! The file:
//!   @set 1 2 3 4 | D G B E                    a string set (string numbers, 1 = high E), then a label
//!   @family Major 7th | G A B C# D E F#       a chord family: its name, then its scale (the first note is the root)
//!   @video g2RKfgRpMaA                        the YouTube video it comes from
//!   @about ...                                a paragraph about the family
//!   @group From root position                 a heading
//!   G D F# B | Root position: ...             a voicing (notes low to high) and a note about it
//! A note that starts with ? is one to check. Lines starting with # are comments.

use crate::theory;

/// The file's name in Backline's folder.
pub const FILE: &str = "chord_book.txt";

pub fn path() -> std::path::PathBuf {
    crate::api::config_dir().join(FILE)
}

/// The voicings you starred (your keepers), one key each (see `star_key`), in Backline's folder. SYNC carries it.
pub const STARS_FILE: &str = "chord_book_stars.json";

/// A voicing's key in the stars file: its string set, its family and its notes ("1 2 3 4 | Dominant 7th | G D F B").
/// The same notes on the same strings are the same grip, wherever the family plays them.
pub fn star_key(set: &Set, fam: &Family, v: &Voicing) -> String {
    format!("{} | {} | {}", set.name, fam.name, v.spelled.join(" "))
}

/// The stars, as saved (none when there's no file yet).
pub fn load_stars() -> Vec<String> {
    std::fs::read_to_string(crate::api::config_dir().join(STARS_FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save_stars(stars: &[String]) {
    if let Ok(t) = serde_json::to_string_pretty(stars) {
        let _ = std::fs::write(crate::api::config_dir().join(STARS_FILE), t);
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Book {
    pub sets: Vec<Set>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Set {
    /// as written ("1 2 3 4")
    pub name: String,
    /// the strings' names ("D G B E")
    pub label: String,
    /// the strings played, low to high (0 = low E)
    pub strings: Vec<usize>,
    pub families: Vec<Family>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Family {
    pub name: String,
    /// its root and scale, as pitch classes (the root first)
    pub root: i32,
    pub scale: Vec<i32>,
    /// the scale as written
    pub scale_text: String,
    /// sharps for the names (the scale is written with them)
    pub sharps: bool,
    pub video: String,
    pub about: String,
    pub groups: Vec<Group>,
    /// each chord is named on its own root, not the family's (a progression in a key, like his book II videos):
    /// written `| own roots` after the scale
    pub own_roots: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Group {
    pub title: String,
    pub voicings: Vec<Voicing>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Voicing {
    /// the notes as written, low to high
    pub spelled: Vec<String>,
    /// and as pitch classes
    pub pcs: Vec<i32>,
    pub note: String,
    /// his own names for it, written {like this} in the note (the braces are taken out of `note`)
    pub names: Vec<String>,
    /// marked ? to check
    pub unsure: bool,
    /// its line in the file (from 1)
    pub line: usize,
}

/// A note name ("F#", "Bb", "Db") as a pitch class.
fn pc_of(s: &str) -> Option<i32> {
    let s = s.trim().replace('\u{266F}', "#").replace('\u{266D}', "b");
    let mut ch = s.chars();
    let base = match ch.next()?.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let mut pc = base;
    for c in ch {
        match c {
            '#' => pc += 1,
            'b' => pc -= 1,
            _ => return None,
        }
    }
    Some((pc as i32).rem_euclid(12))
}

/// Which station a group of his is built from, by its title.
#[derive(Debug, PartialEq)]
enum Anchor {
    Station(usize),
    /// it names a bass that's no station's ("D in the bass (the 9)")
    Elsewhere,
    Unsaid,
}

/// The station a group's title points at: the bass it names ("B in the bass", "C, D and E in the bass"), the chord
/// it opens with ("Ab: ...", "Db (F and Ab stay)"), or the inversion it names ("From the 1st inversion"; `tones` are
/// the first station's chord tones up from its bass). `bass` is each station's bass.
fn station_anchor(title: &str, bass: &[i32], tones: &[i32]) -> Anchor {
    let note = |w: &str| -> Option<i32> {
        let w = w.trim_matches(|c| matches!(c, '(' | ')' | ',' | ':'));
        let w = w.strip_suffix("dim").or_else(|| w.strip_suffix('m')).unwrap_or(w);
        let mut ch = w.chars();
        let first = ch.next()?;
        (('A'..='G').contains(&first) && ch.all(|c| c == '#' || c == 'b')).then(|| pc_of(w)).flatten()
    };
    let words: Vec<&str> = title.split_whitespace().collect();
    let mut named: Vec<i32> = Vec::new();
    if let Some(at) = title.find(" in the bass") {
        named = title[..at].split_whitespace().filter_map(note).collect();
    } else if let Some(w0) = words.first().filter(|w| w.ends_with(':') || words.get(1).is_some_and(|n| n.starts_with('('))) {
        named.extend(note(w0));
    }
    if let Some(k) = named.iter().find_map(|p| bass.iter().position(|b| b == p)) {
        return Anchor::Station(k);
    }
    let low = title.to_lowercase();
    for (k, w) in ["root position", "1st inversion", "2nd inversion", "3rd inversion"].iter().enumerate() {
        if let Some(s) = tones.get(k).filter(|_| low.contains(w)).and_then(|t| bass.iter().position(|b| b == t)) {
            return Anchor::Station(s);
        }
    }
    if named.is_empty() { Anchor::Unsaid } else { Anchor::Elsewhere }
}

/// The book in `text`. Lines it can't read are skipped and reported as (line, why).
pub fn parse(text: &str) -> (Book, Vec<(usize, String)>) {
    let mut book = Book::default();
    let mut errs = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (head, rest) = match line.split_once(char::is_whitespace) {
            Some((h, r)) => (h, r.trim()),
            None => (line, ""),
        };
        match head {
            "@set" => {
                let (nums, label) = rest.split_once('|').map(|(a, b)| (a.trim(), b.trim())).unwrap_or((rest, ""));
                let mut strings: Vec<usize> = Vec::new();
                for t in nums.split_whitespace() {
                    match t.parse::<usize>() {
                        Ok(s) if (1..=6).contains(&s) => strings.push(6 - s),
                        _ => errs.push((n, format!("not a string number: {t}"))),
                    }
                }
                strings.sort();
                book.sets.push(Set { name: nums.to_string(), label: label.to_string(), strings, families: Vec::new() });
            }
            "@family" => {
                let Some(set) = book.sets.last_mut() else {
                    errs.push((n, "a family before any @set".into()));
                    continue;
                };
                let (name, scale_text) = rest.split_once('|').map(|(a, b)| (a.trim(), b.trim())).unwrap_or((rest, ""));
                let (scale_text, flags) =
                    scale_text.split_once('|').map(|(a, b)| (a.trim(), b.trim())).unwrap_or((scale_text, ""));
                let scale: Vec<i32> = scale_text.split_whitespace().filter_map(pc_of).collect();
                if scale.is_empty() {
                    errs.push((n, "a family needs its scale after |".into()));
                }
                set.families.push(Family {
                    name: name.to_string(),
                    root: scale.first().copied().unwrap_or(7),
                    sharps: scale_text.contains('#'),
                    scale_text: scale_text.to_string(),
                    scale,
                    own_roots: flags.contains("own roots"),
                    ..Default::default()
                });
            }
            "@video" | "@about" | "@group" => {
                let Some(fam) = book.sets.last_mut().and_then(|s| s.families.last_mut()) else {
                    errs.push((n, format!("{head} before any @family")));
                    continue;
                };
                match head {
                    "@video" => fam.video = rest.to_string(),
                    "@about" => {
                        if !fam.about.is_empty() {
                            fam.about.push(' ');
                        }
                        fam.about.push_str(rest)
                    }
                    _ => fam.groups.push(Group { title: rest.to_string(), voicings: Vec::new() }),
                }
            }
            h if h.starts_with('@') => errs.push((n, format!("unknown {h}"))),
            _ => {
                let Some(fam) = book.sets.last_mut().and_then(|s| s.families.last_mut()) else {
                    errs.push((n, "a voicing before any @family".into()));
                    continue;
                };
                let (notes, note) = line.split_once('|').map(|(a, b)| (a.trim(), b.trim())).unwrap_or((line, ""));
                let spelled: Vec<String> = notes.split_whitespace().map(str::to_string).collect();
                let pcs: Vec<Option<i32>> = spelled.iter().map(|s| pc_of(s)).collect();
                if pcs.iter().any(Option::is_none) || spelled.is_empty() {
                    errs.push((n, format!("can't read the notes: {notes}")));
                    continue;
                }
                if fam.groups.is_empty() {
                    fam.groups.push(Group::default());
                }
                let unsure = note.starts_with('?');
                let note = note.trim_start_matches('?').trim();
                let names: Vec<String> = note
                    .split('{')
                    .skip(1)
                    .filter_map(|p| p.split_once('}').map(|(n, _)| n.trim().to_string()))
                    .filter(|n| !n.is_empty())
                    .collect();
                fam.groups.last_mut().unwrap().voicings.push(Voicing {
                    spelled,
                    pcs: pcs.into_iter().flatten().collect(),
                    // (the names show on their own, so the note reads without them)
                    note: note
                        .split('{')
                        .enumerate()
                        .map(|(k, p)| if k == 0 { p } else { p.split_once('}').map(|x| x.1).unwrap_or(p) })
                        .collect::<String>()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                    names,
                    unsure,
                    line: n,
                });
            }
        }
    }
    (book, errs)
}

impl Voicing {
    /// Where to play it on `strings`: the notes in order, one to a string, climbing, the lowest on the neck within
    /// a stretch of five frets (six if it must). None when it doesn't fit those strings.
    pub fn grip(&self, strings: &[usize]) -> Option<[Option<u8>; 6]> {
        if self.pcs.len() != strings.len() {
            return None;
        }
        for reach in [5u8, 6] {
            if let Some(g) = theory::grips_on(&self.pcs, strings, true, reach).into_iter().next() {
                return Some(g);
            }
        }
        // (he plays some up past the 15th fret)
        self.high_grip(strings)
    }

    /// The notes in order, climbing, one to a string, fretted within six frets, somewhere up to the 19th fret.
    fn high_grip(&self, strings: &[usize]) -> Option<[Option<u8>; 6]> {
        fn go(k: usize, pcs: &[i32], strings: &[usize], cur: &mut Vec<u8>) -> bool {
            if k == strings.len() {
                let (lo, hi) = (cur.iter().min().copied().unwrap_or(0), cur.iter().max().copied().unwrap_or(0));
                return hi - lo <= 6 && lo > 0;
            }
            for f in 1..=19u8 {
                let m = theory::STANDARD[strings[k]] + f as i32;
                let climbs = k == 0 || m > theory::STANDARD[strings[k - 1]] + cur[k - 1] as i32;
                if m.rem_euclid(12) == pcs[k].rem_euclid(12) && climbs {
                    cur.push(f);
                    if go(k + 1, pcs, strings, cur) {
                        return true;
                    }
                    cur.pop();
                }
            }
            false
        }
        let mut cur = Vec::new();
        if !go(0, &self.pcs, strings, &mut cur) {
            return None;
        }
        let mut g = [None; 6];
        for (j, s) in strings.iter().enumerate() {
            g[*s] = Some(cur[j]);
        }
        Some(g)
    }

    /// Each note's degree over the family's root (R, b9, 9, #9, 3, 11, #11, 5, #5, 13, b7, 7).
    pub fn degrees(&self, fam: &Family) -> Vec<String> {
        // over its own chord's root in an own-roots family (a minor chord's 3rd is its b3, a natural 5th assumed)
        let own = fam.own_roots.then(|| self.own_chord()).flatten();
        let root = own.as_ref().map_or(fam.root, |s| s.root);
        let rel: Vec<i32> = self.pcs.iter().map(|p| (p - root).rem_euclid(12)).collect();
        let has7 = rel.contains(&10) || rel.contains(&11);
        let minor = match &own {
            Some(s) => s.suffix.starts_with('m') && !s.suffix.starts_with("maj"),
            None => fam.kind() == Kind::Minor,
        };
        let fifth = own.is_some() || fam.has(7);
        rel.iter()
            .map(|r| {
                match r {
                    0 => "R",
                    1 => "b9",
                    2 => "9",
                    3 if minor => "b3",
                    3 => "#9",
                    4 => "3",
                    5 => "11",
                    6 if fifth => "#11",
                    6 => "b5",
                    7 => "5",
                    8 if fifth => "b13",
                    8 => "#5",
                    9 if has7 => "13",
                    9 => "6",
                    10 => "b7",
                    _ => "7",
                }
                .to_string()
            })
            .collect()
    }

    /// Its name as a chord on the family's root: the analyzer's name for these notes over the root (the bass player's
    /// note when the root isn't played), "/B" when the root is played but isn't the lowest, "(no root)" when it isn't
    /// played at all.
    pub fn symbol(&self, fam: &Family) -> String {
        if fam.own_roots {
            let sp = theory::Spelling { sharps: fam.sharps, slash: true, ..Default::default() };
            return self.own_chord().map(|s| s.name(sp)).unwrap_or_else(|| self.spelled.join(" "));
        }
        let mut rel: Vec<i32> = self.pcs.iter().map(|p| (p - fam.root).rem_euclid(12)).collect();
        let rooted = rel.contains(&0);
        if !rooted {
            rel.push(0);
        }
        // the family's 3rd when the voicing leaves it out (and isn't a sus4): it's still that kind of chord, without
        // its 3rd. A dominant's minor 3rd is its #9.
        let third = if fam.kind() == Kind::Minor { 3 } else { 4 };
        let sus4 = rel.contains(&5) && !rel.contains(&3) && !rel.contains(&4);
        let other_third = rel.contains(&(7 - third)) && fam.kind() != Kind::Dominant;
        let no3 = !rel.contains(&third) && !sus4 && !other_third;
        if no3 {
            rel.push(third);
        }
        // a raised 4th in a family with a natural 5th is its #11 (the 5th left out, as usual), not a flat 5th
        if fam.has(7) && rel.contains(&6) && !rel.contains(&7) {
            rel.push(7);
        }
        rel.sort();
        rel.dedup();
        let root = theory::note_name(fam.root, fam.sharps);
        let mut suffix = theory::built_name(&rel).unwrap_or_else(|| fam.fallback_suffix(&rel));
        if no3 {
            suffix += "(no3)";
        }
        let mut s = format!("{root}{suffix}");
        let bass = self.pcs.first().copied().unwrap_or(fam.root);
        if rooted && bass != fam.root {
            s += &format!("/{}", self.spelled.first().cloned().unwrap_or_default());
        }
        if !rooted {
            s += " (no root)";
        }
        s
    }

    /// The chord these notes make on their own: the analyzer's best name, with the lowest note as the bass.
    fn own_chord(&self) -> Option<theory::Suggestion> {
        let all = theory::suggestions(&self.pcs, self.pcs.first().copied());
        let i = all.iter().position(|s| s.exact && s.missing.is_empty()).unwrap_or(0);
        all.into_iter().nth(i)
    }

    /// The other chords these notes make (their own roots, not the family's): what the voicing is when you hear it
    /// another way, best first.
    pub fn also(&self, fam: &Family, max: usize) -> Vec<String> {
        let sp = theory::Spelling { sharps: fam.sharps, ..Default::default() };
        // in an own-roots family the symbol already is the chord's own name
        let skip = if fam.own_roots { self.own_chord().map_or(fam.root, |s| s.root) } else { fam.root };
        let mut out: Vec<String> = Vec::new();
        for s in theory::suggestions(&self.pcs, self.pcs.first().copied()) {
            if s.root == skip || (!s.missing.is_empty() && !s.exact) {
                continue;
            }
            // names players use (not "D6add11" or "Baugb5")
            let odd = ["add", "sus", "no", "aug", "6b", "6#", "b5#5", "dim"];
            if odd.iter().any(|o| s.suffix.contains(o)) && s.suffix != "dim7" {
                continue;
            }
            let n = s.name(sp);
            if !out.contains(&n) {
                out.push(n);
            }
            if out.len() == max {
                break;
            }
        }
        out
    }

    /// Notes that aren't in the family's scale (a slip to check).
    pub fn outside(&self, fam: &Family) -> Vec<String> {
        self.pcs.iter().zip(&self.spelled).filter(|(p, _)| !fam.scale.contains(p)).map(|(_, s)| s.clone()).collect()
    }
}

/// What kind of chord a family is, by its scale: what its 3rd and 7th are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Major,
    Minor,
    Dominant,
}

impl Family {
    fn has(&self, rel: i32) -> bool {
        self.scale.iter().any(|p| (p - self.root).rem_euclid(12) == rel)
    }

    pub fn kind(&self) -> Kind {
        if self.has(4) && self.has(11) && !self.has(10) {
            Kind::Major
        } else if self.has(3) && !self.has(4) {
            Kind::Minor
        } else {
            Kind::Dominant
        }
    }

    /// A plain name when the analyzer has none: the family's chord with its colors in brackets.
    fn fallback_suffix(&self, rel: &[i32]) -> String {
        let has = |x: i32| rel.contains(&x);
        let base = match self.kind() {
            Kind::Major => "maj7",
            Kind::Minor => "m7",
            Kind::Dominant => "7",
        };
        let fifth = self.has(7);
        let mut colors: Vec<&str> = Vec::new();
        for (x, name) in [(1, "b9"), (2, "9"), (3, "#9"), (5, "11"), (6, if fifth { "#11" } else { "b5" }), (8, if fifth { "b13" } else { "#5" }), (9, "13")] {
            if has(x) && !(x == 3 && self.kind() == Kind::Minor) {
                colors.push(name);
            }
        }
        if colors.is_empty() {
            base.to_string()
        } else {
            format!("{base}({})", colors.join(","))
        }
    }
}

/// A family's voicings as a walk (in his order, or by station: see `Order`): where each sits on the neck (near the
/// one it came from, so the walk stays in one place the way he plays it), which earlier voicing it came from, and the
/// move in words.
#[derive(Clone, Debug, Default)]
pub struct Walk {
    pub steps: Vec<Step>,
    /// where each station's button goes (by its number in the stations group): where the walk turns to it; when the
    /// family doesn't start from stations, where each of his groups starts
    pub stations: Vec<usize>,
    /// the family starts from its stations (his four inversions, or close and spread ones), not from group starts
    pub inversions: bool,
    /// the walk turns to its stations one by one: the chords after a station are built from it
    pub paths: bool,
    /// the stops along the line, as ranges of steps: a station (several side by side on one bass, close and spread,
    /// are one stop), or where a part starts
    pub stops: Vec<std::ops::Range<usize>>,
    pub order: Order,
}

impl Walk {
    /// The station step `k` is (its number in the stations group), when it's one.
    pub fn station_at(&self, k: usize) -> Option<usize> {
        self.steps.get(k).filter(|s| self.inversions && s.at.0 == 0).map(|s| s.at.1)
    }

    /// Step `k` is a stop on the line: a station, or where a part starts.
    pub fn is_stop(&self, k: usize) -> bool {
        self.stops.iter().any(|r| r.contains(&k))
    }
}

/// The order WALK goes through a family that starts from its stations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Order {
    /// his order: the stations as he plays them first, and each again where the walk turns to it
    #[default]
    Lesson,
    /// each station once, with the chords built from it right after it
    ByStation,
}

/// Where a family's walk turns to its stations (see `Family::station_turns`).
struct Turns {
    /// each station's bass
    bass: Vec<i32>,
    /// for each group, the station it turns to
    turn: Vec<Option<usize>>,
}

#[derive(Clone, Debug, Default)]
pub struct Step {
    /// where it is in the family: (group, voicing in the group)
    pub at: (usize, usize),
    pub grip: Option<[Option<u8>; 6]>,
    /// the voicing it came from (an earlier step) and the move
    pub from: Option<usize>,
    pub how: String,
    /// the voices that moved (0 = the lowest)
    pub moved: Vec<usize>,
    /// on a station's path: a chord off the path (an earlier station's) that fewer notes change from, and that move
    pub also: Option<usize>,
    pub also_how: String,
}

/// The voices of a four-note voicing, low to high.
pub fn voice_name(i: usize, n: usize) -> &'static str {
    match (n, i) {
        (_, 0) => "Bass",
        (_, i) if i + 1 == n => "Top voice",
        (3, 1) => "Middle voice",
        (4, 1) | (5, 1) | (6, 2) => "Tenor",
        (4, 2) | (5, 3) | (6, 4) => "Alto",
        (5, 2) | (6, 3) => "Middle voice",
        (6, 1) => "Baritone",
        _ => "Inner voice",
    }
}

/// "up a half step", "down a minor 3rd"...
fn step_words(d: i32) -> String {
    let w = match d.abs() {
        0 => return "an octave away".into(),
        1 => "a half step",
        2 => "a whole step",
        3 => "a minor 3rd",
        4 => "a major 3rd",
        5 => "a 4th",
        6 => "a tritone",
        7 => "a 5th",
        8 => "a minor 6th",
        9 => "a major 6th",
        10 => "a minor 7th",
        11 => "a major 7th",
        _ => "an octave or more",
    };
    format!("{} {w}", if d > 0 { "up" } else { "down" })
}

/// Every way to play the notes in order on `strings`, climbing, fretted within six frets, up to the 19th fret (open
/// strings only in a shape down by the nut).
fn all_grips(pcs: &[i32], strings: &[usize]) -> Vec<[Option<u8>; 6]> {
    fn go(k: usize, pcs: &[i32], strings: &[usize], cur: &mut Vec<u8>, out: &mut Vec<[Option<u8>; 6]>) {
        if k == strings.len() {
            let fretted: Vec<u8> = cur.iter().copied().filter(|f| *f > 0).collect();
            let (lo, hi) = (fretted.iter().min().copied().unwrap_or(0), fretted.iter().max().copied().unwrap_or(0));
            if hi - lo <= 6 && (!cur.contains(&0) || hi <= 6) {
                let mut g = [None; 6];
                for (j, s) in strings.iter().enumerate() {
                    g[*s] = Some(cur[j]);
                }
                out.push(g);
            }
            return;
        }
        for f in 0..=19u8 {
            let m = theory::STANDARD[strings[k]] + f as i32;
            let climbs = k == 0 || m > theory::STANDARD[strings[k - 1]] + cur[k - 1] as i32;
            if m.rem_euclid(12) == pcs[k].rem_euclid(12) && climbs {
                cur.push(f);
                go(k + 1, pcs, strings, cur, out);
                cur.pop();
            }
        }
    }
    let mut out = Vec::new();
    if pcs.len() == strings.len() {
        go(0, pcs, strings, &mut Vec::new(), &mut out);
    }
    out
}

/// How far a hand moves from one grip to another (frets, string by string).
fn distance(a: &[Option<u8>; 6], b: &[Option<u8>; 6]) -> i32 {
    a.iter().zip(b).map(|(x, y)| match (x, y) {
        (Some(x), Some(y)) => (*x as i32 - *y as i32).abs(),
        _ => 0,
    }).sum()
}

impl Family {
    /// Its voicings in order: (group, index in the group).
    pub fn order(&self) -> Vec<(usize, usize)> {
        self.groups.iter().enumerate().flat_map(|(g, gr)| (0..gr.voicings.len()).map(move |v| (g, v))).collect()
    }

    pub fn voicing(&self, at: (usize, usize)) -> &Voicing {
        &self.groups[at.0].voicings[at.1]
    }

    /// The voices that differ between two voicings (by note, voice by voice).
    pub fn changed(a: &Voicing, b: &Voicing) -> Vec<usize> {
        if a.pcs.len() != b.pcs.len() {
            return (0..b.pcs.len()).collect();
        }
        (0..a.pcs.len()).filter(|&i| a.pcs[i] != b.pcs[i]).collect()
    }

    /// Two voices that traded notes (a voice exchange), if that's the move.
    fn exchange(a: &Voicing, b: &Voicing, changed: &[usize]) -> Option<(usize, usize)> {
        match changed {
            [i, j] if a.pcs[*i] == b.pcs[*j] && a.pcs[*j] == b.pcs[*i] => Some((*i, *j)),
            _ => None,
        }
    }

    /// The move from `a` (played as `ga`) to `b` (as `gb`), in words.
    fn move_words(a: &Voicing, b: &Voicing, ga: Option<&[Option<u8>; 6]>, gb: Option<&[Option<u8>; 6]>, strings: &[usize]) -> String {
        let changed = Self::changed(a, b);
        let n = b.pcs.len();
        if changed.is_empty() {
            return "The same notes.".into();
        }
        let pitch = |g: Option<&[Option<u8>; 6]>, i: usize| -> Option<i32> {
            let s = *strings.get(i)?;
            g.and_then(|g| g[s]).map(|f| theory::STANDARD[s] + f as i32)
        };
        let delta = |i: usize| -> Option<i32> { Some(pitch(gb, i)? - pitch(ga, i)?) };
        if let Some((i, j)) = Self::exchange(a, b, &changed) {
            return format!(
                "Voice exchange: the {} and {} trade notes ({} and {}).",
                voice_name(i, n).to_lowercase(),
                voice_name(j, n).to_lowercase(),
                a.spelled[i],
                a.spelled[j]
            );
        }
        let parts: Vec<String> = changed
            .iter()
            .map(|&i| {
                let how = delta(i).map(|d| format!(" ({})", step_words(d))).unwrap_or_default();
                format!("{} {} to {}{}", voice_name(i, n), a.spelled[i], b.spelled[i], how)
            })
            .collect();
        let ds: Vec<i32> = changed.iter().filter_map(|&i| delta(i)).collect();
        if changed.len() == n && ds.len() == n && (ds.iter().all(|d| *d > 0) || ds.iter().all(|d| *d < 0)) {
            let dir = if ds[0] > 0 { "up" } else { "down" };
            return format!("Every voice moves {dir} to the next note of the chord: {}.", parts.join("; "));
        }
        format!("{}.", parts.join("; "))
    }

    /// Where the walk turns to each station: for each group, the station its title points at (or, when the title
    /// doesn't say, the one whose bass most of its chords sit on, when that's the next station along). None when the
    /// family doesn't start from stations, or nothing is built from any station but the first (and the stations on
    /// its bass).
    fn station_turns(&self) -> Option<Turns> {
        let st = self.groups.first().filter(|g| g.title.to_lowercase().contains("station") && !g.voicings.is_empty())?;
        let ns = st.voicings.len();
        let bass: Vec<i32> = st.voicings.iter().map(|v| v.pcs.first().copied().unwrap_or(-1)).collect();
        // (the chord tones of the first station, up from its bass: the inversions' bass notes)
        let mut tones = st.voicings[0].pcs.clone();
        tones.sort_by_key(|p| (p - bass[0]).rem_euclid(12));
        tones.dedup();
        let mut placed: Vec<bool> = bass.iter().map(|b| *b == bass[0]).collect();
        let mut turn = vec![None];
        for g in self.groups.iter().skip(1) {
            let k = match station_anchor(&g.title, &bass, &tones) {
                Anchor::Station(k) => Some(k),
                Anchor::Elsewhere => None,
                Anchor::Unsaid => {
                    let next = (0..ns).find(|&j| !placed[j]);
                    (0..ns)
                        .find(|&j| g.voicings.iter().filter(|v| v.pcs.first() == Some(&bass[j])).count() * 2 > g.voicings.len())
                        .filter(|&j| next.is_some_and(|nx| bass[nx] == bass[j]))
                }
            };
            if let Some(k) = k {
                for j in 0..ns {
                    placed[j] |= bass[j] == bass[k];
                }
            }
            turn.push(k);
        }
        // (nothing built from any station but the first)
        if (0..ns).all(|j| placed[j] == (bass[j] == bass[0])) {
            return None;
        }
        Some(Turns { bass, turn })
    }

    /// Whether the two orders differ: the walk turns to its stations one by one (see `Order`).
    pub fn has_station_paths(&self) -> bool {
        self.station_turns().is_some()
    }

    /// The order WALK goes through the family (see `Order`): his order, with each station again where the walk
    /// turns to it; or each station once, just before the chords built from it, so NEXT from a station walks its own
    /// path. Stations on the same bass (close and spread) go together. In the second, a station with nothing built
    /// from it waits after the path of the station before it. Both are his order as it is when nothing is built from
    /// any station but the first.
    pub fn walk_order(&self, by: Order) -> Vec<(usize, usize)> {
        let Some(t) = self.station_turns() else {
            return self.order();
        };
        let ns = t.bass.len();
        // a station and the stations on its bass
        let bass = &t.bass;
        let run = |k: usize| (0..ns).filter(move |&j| bass[j] == bass[k]).map(|j| (0usize, j));
        let mut out: Vec<(usize, usize)> = Vec::new();
        match by {
            Order::Lesson => {
                // the stations as the lesson plays them first, then each again where the walk turns to it (the first
                // station right after them, unless the walk turns elsewhere at once)
                out.extend((0..ns).map(|j| (0, j)));
                let mut cur: Option<i32> = None;
                for (gi, g) in self.groups.iter().enumerate().skip(1) {
                    let to = t.turn[gi].or((gi == 1).then_some(0));
                    if let Some(k) = to.filter(|&k| cur != Some(t.bass[k])) {
                        out.extend(run(k));
                        cur = Some(t.bass[k]);
                    }
                    out.extend((0..g.voicings.len()).map(|v| (gi, v)));
                }
            }
            Order::ByStation => {
                let mut placed = vec![false; ns];
                for (gi, g) in self.groups.iter().enumerate().skip(1) {
                    let to = if gi == 1 { Some(0) } else { None };
                    for k in to.into_iter().chain(t.turn[gi]) {
                        if !placed[k] {
                            for (_, j) in run(k) {
                                placed[j] = true;
                                out.push((0, j));
                            }
                        }
                    }
                    out.extend((0..g.voicings.len()).map(|v| (gi, v)));
                }
                for j in 1..ns {
                    if placed[j] {
                        continue;
                    }
                    // after the station before it (and its partners on the same bass) and its path
                    let mut q = out.iter().position(|&a| a == (0, j - 1)).map_or(out.len(), |p| p + 1);
                    while q < out.len() && out[q].0 == 0 && t.bass[out[q].1] == t.bass[j - 1] {
                        q += 1;
                    }
                    while q < out.len() && out[q].0 != 0 {
                        q += 1;
                    }
                    out.insert(q, (0, j));
                    placed[j] = true;
                }
            }
        }
        out
    }

    /// The family as a walk on `strings` (see Walk), in the order `by`.
    pub fn walk(&self, strings: &[usize], by: Order) -> Walk {
        let order = self.walk_order(by);
        let inversions = self.groups.first().is_some_and(|g| g.title.to_lowercase().contains("station"));
        let station = |k: usize| inversions && order[k].0 == 0;
        let paths = inversions && order != self.order();
        let mut steps: Vec<Step> = Vec::with_capacity(order.len());
        for (k, &at) in order.iter().enumerate() {
            let v = self.voicing(at);
            // a station played again: the same grip, and the same move as the first time (from the latest time the
            // chord it came from was played)
            if let Some(first) = (0..k).find(|&j| order[j] == at) {
                let s = &steps[first];
                let from = s.from.and_then(|f| (0..k).rev().find(|&j| order[j] == order[f]));
                steps.push(Step { at, grip: s.grip, from, how: s.how.clone(), moved: s.moved.clone(), ..Default::default() });
                continue;
            }
            // from: the earlier voicing it differs from least (the latest of those); a station comes from a station.
            // On a station's path, from that path (the station, the stations on its bass, the chords after it) when
            // anything there shares a note with it: the lesson puts it with that station. A chord off the path that
            // fewer notes change from is kept as `also`.
            let nearest = |from_j: usize| {
                let mut best: Option<(usize, usize)> = None;
                for j in (from_j..k).rev() {
                    if station(k) && !station(j) {
                        continue;
                    }
                    let c = Self::changed(self.voicing(order[j]), v).len();
                    if c > 0 && best.is_none_or(|(_, b)| c < b) {
                        best = Some((j, c));
                    }
                }
                best
            };
            let anywhere = nearest(0);
            let path_start = (paths && !station(k)).then(|| (0..k).rev().find(|&j| station(j))).flatten().map(|mut s| {
                let bass = |j: usize| self.voicing(order[j]).pcs.first().copied();
                while s > 0 && station(s - 1) && bass(s - 1) == bass(s) {
                    s -= 1;
                }
                s
            });
            let on_path = path_start.and_then(nearest).filter(|(_, c)| *c < v.pcs.len());
            let from = on_path.or(anywhere);
            let also = match (on_path, anywhere, path_start) {
                (Some((_, c)), Some((j, ca)), Some(s)) if ca < c && j < s => Some(j),
                _ => None,
            };
            let cands = all_grips(&v.pcs, strings);
            let near = from.and_then(|(j, _)| steps[j].grip);
            let grip = match near {
                Some(n) => cands.iter().min_by_key(|g| distance(g, &n)).copied(),
                // (a walk starts where it's lowest on the neck)
                None => v.grip(strings).or_else(|| cands.first().copied()),
            };
            let (how, moved) = match from {
                Some((j, _)) => {
                    let a = self.voicing(order[j]);
                    (Self::move_words(a, v, steps[j].grip.as_ref(), grip.as_ref(), strings), Self::changed(a, v))
                }
                None => (String::new(), Vec::new()),
            };
            let also_how = also.map_or(String::new(), |j| Self::move_words(self.voicing(order[j]), v, steps[j].grip.as_ref(), grip.as_ref(), strings));
            steps.push(Step { at, grip, from: from.map(|f| f.0), how, moved, also, also_how });
        }
        // the stops: each station (side by side on one bass, one stop), or where each part starts
        let mut stops: Vec<std::ops::Range<usize>> = Vec::new();
        for k in 0..order.len() {
            if !(if inversions { order[k].0 == 0 } else { order[k].1 == 0 }) {
                continue;
            }
            let bass = |k: usize| self.voicing(order[k]).pcs.first().copied();
            match stops.last_mut() {
                Some(r) if inversions && r.end == k && bass(r.end - 1) == bass(k) => r.end = k + 1,
                _ => stops.push(k..k + 1),
            }
        }
        // where each station's button goes: where the walk turns to it (past the opening stations), else where it
        // is; or where each part starts
        let stations = if inversions {
            let opening = if paths && by == Order::Lesson { self.groups[0].voicings.len() } else { 0 };
            (0..self.groups[0].voicings.len())
                .map(|m| (opening..order.len()).find(|&k| order[k] == (0, m)).or_else(|| order.iter().position(|&a| a == (0, m))).unwrap_or(0))
                .collect()
        } else {
            stops.iter().map(|r| r.start).collect()
        };
        Walk { steps, stations, inversions, paths, stops, order: by }
    }

    /// The voicings one move from step `i` of `walk`: one voice moved, or two voices trading notes. (step, the move)
    pub fn neighbours(&self, walk: &Walk, i: usize, strings: &[usize]) -> Vec<(usize, String)> {
        let Some(here) = walk.steps.get(i) else { return Vec::new() };
        let a = self.voicing(here.at);
        let mut out: Vec<(usize, String, usize)> = Vec::new();
        for (j, st) in walk.steps.iter().enumerate() {
            if j == i {
                continue;
            }
            let b = self.voicing(st.at);
            let c = Self::changed(a, b);
            let swap = Self::exchange(a, b, &c).is_some();
            if c.len() == 1 || swap {
                // (sorted by the voice that moves, top first, then exchanges)
                let key = if swap { 9 } else { 4 - c[0].min(4) };
                out.push((j, Self::move_words(a, b, here.grip.as_ref(), st.grip.as_ref(), strings), key));
            }
        }
        out.sort_by_key(|x| (x.2, x.0));
        // (a voicing written twice in his order shows once)
        let mut seen: Vec<Vec<i32>> = Vec::new();
        out.into_iter()
            .filter(|(j, _, _)| {
                let p = self.voicing(walk.steps[*j].at).pcs.clone();
                if seen.contains(&p) {
                    false
                } else {
                    seen.push(p);
                    true
                }
            })
            .map(|(j, t, _)| (j, t))
            .collect()
    }

    /// A note's name the family's way: as the scale spells it when it's in the scale.
    fn spell(&self, pc: i32) -> String {
        let pc = pc.rem_euclid(12);
        self.scale_text
            .split_whitespace()
            .find(|t| pc_of(t) == Some(pc))
            .map(str::to_string)
            .unwrap_or_else(|| theory::note_name(pc, self.sharps).to_string())
    }

    /// What the rules make of step `i`, the ones from his book and videos (only the ones that fit the strings):
    /// - one note moved, where it makes a chord with a name. His: lower the 5th of a m7 for m7b5, lower the 3rd of
    ///   a maj7 or raise the 7th of a m7 for m(maj7), raise the 5th of a maj7 for maj7#5, lower the 9th of a 9sus4;
    ///   the steps of his chain (m7b5, m(maj7), maj7#5, 7#9, 7b9: one note each, all one melodic minor); in the
    ///   diminished scale lower the 13th or raise the 5th and it's altered; a dim7 with one note down is a dominant
    /// - voice exchange: two voices a 2nd apart trade notes on the same strings
    /// - the next inversion: every voice to the next note of the chord
    /// - a tritone away (a #11 grip is altered there, and back), the rule of the fifth (and the fourth)
    /// - string transference: the note on one E string moved to the other (another string set)
    /// - planing, and his other grips with these notes
    pub fn rules(&self, walk: &Walk, i: usize, strings: &[usize]) -> Vec<Rule> {
        let Some(here) = walk.steps.get(i) else { return Vec::new() };
        let a = self.voicing(here.at);
        let n = a.pcs.len();
        let deg = a.degrees(self);
        let rel = |p: i32| (p - self.root).rem_euclid(12);
        let his = |pcs: &[i32], on: &[usize]| {
            if on == strings {
                walk.steps.iter().position(|s| self.voicing(s.at).pcs == pcs)
            } else {
                None
            }
        };
        let near = |pcs: &[i32], on: &[usize]| -> Option<[Option<u8>; 6]> {
            let c = all_grips(pcs, on);
            match here.grip {
                Some(g) => c.into_iter().min_by_key(|x| distance(x, &g)),
                None => c.into_iter().next(),
            }
        };
        // the same shape moved `by` frets along the neck (None when it runs off)
        let shifted = |by: i32| -> Option<[Option<u8>; 6]> {
            let g = here.grip?;
            let mut out = [None; 6];
            for s in 0..6 {
                if let Some(f) = g[s] {
                    let nf = f as i32 + by;
                    if !(0..=19).contains(&nf) {
                        return None;
                    }
                    out[s] = Some(nf as u8);
                }
            }
            Some(out)
        };
        let own = |pcs: &[i32]| Voicing { pcs: pcs.to_vec(), ..Default::default() }.own_chord();
        // (`fix`: a moved voice's spelling and degree, when the plain ones would read wrong)
        let make = |kind: &'static str,
                    how: String,
                    pcs: Vec<i32>,
                    on: &[usize],
                    grip: Option<[Option<u8>; 6]>,
                    moved: Vec<usize>,
                    fix: Option<(usize, String, String)>|
         -> Option<Rule> {
            let grip = grip?;
            let mut spelled: Vec<String> = pcs.iter().map(|&p| self.spell(p)).collect();
            if let Some((vi, s, _)) = &fix {
                spelled[*vi] = s.clone();
            }
            let v = Voicing { spelled: spelled.clone(), pcs: pcs.clone(), ..Default::default() };
            let sp = theory::Spelling { sharps: self.sharps, slash: true, ..Default::default() };
            let name = v.own_chord().map(|s| s.name(sp).replace("mMaj7", "m(maj7)")).unwrap_or_else(|| spelled.join(" "));
            Some(Rule {
                kind,
                how,
                in_scale: pcs.iter().all(|p| self.scale.contains(p)),
                over: v.symbol(self).replace("mMaj7", "m(maj7)"),
                degrees: {
                    let mut d = v.degrees(self);
                    if let Some((vi, _, g)) = &fix {
                        d[*vi] = g.clone();
                    }
                    d
                },
                his: his(&pcs, on),
                strings: on.to_vec(),
                spelled,
                pcs,
                grip: Some(grip),
                name,
                moved,
                stated: false,
                from_book: false,
                given: false,
            })
        };
        let mut out: Vec<Rule> = Vec::new();
        let before = own(&a.pcs);
        let fam_name = self.name.to_lowercase();
        // 1. one note moved: a half step (any change with a name), or a whole step (his chain and rules only)
        for vi in (0..n).rev() {
            for d in [-1, 1, -2, 2] {
                let mut pcs = a.pcs.clone();
                pcs[vi] = (pcs[vi] + d).rem_euclid(12);
                // (onto another voice's note: that's a doubling, not a new chord)
                if a.pcs.iter().enumerate().any(|(k, p)| k != vi && *p == pcs[vi]) {
                    continue;
                }
                let after = own(&pcs);
                let (from, to) = (rel(a.pcs[vi]), rel(pcs[vi]));
                let types = (before.as_ref().map(|s| (s.root, s.suffix.as_str())), after.as_ref().map(|s| (s.root, s.suffix.as_str())));
                let has = |r: i32| a.pcs.iter().any(|p| rel(*p) == r);
                // his named changes: by the chords' own types, or by the degrees in the family
                let book = match types {
                    (Some((r1, "m7")), Some((r2, "m7b5"))) if r1 == r2 => Some(("Core rule", "lower the 5th of a minor 7th: m7b5")),
                    (Some((r1, "maj7")), Some((r2, "mMaj7"))) if r1 == r2 => Some(("Core rule", "lower the 3rd of a major 7th: m(maj7)")),
                    (Some((r1, "m7")), Some((r2, "mMaj7"))) if r1 == r2 => Some(("Core rule", "raise the 7th of a minor 7th: m(maj7)")),
                    (Some((r1, "maj7")), Some((r2, "maj7#5"))) if r1 == r2 => Some(("Core rule", "raise the 5th of a major 7th: maj7#5")),
                    (Some((_, "m7b5")), Some((_, "mMaj7"))) if d == 2 => Some(("The chain", "step 1, raise the root of the m7b5: m(maj7)")),
                    (Some((_, "mMaj7")), Some((_, "maj7#5"))) if d == 2 => Some(("The chain", "step 2, raise the root of the m(maj7): maj7#5")),
                    (Some((_, "maj7#5")), Some((_, "7#9"))) if d == 2 => Some(("The chain", "step 3, raise the 3rd of the maj7#5: 7#9")),
                    (Some((_, "7#9")), Some((_, "7b9"))) if d == -2 => Some(("The chain", "step 4, lower the #9: 7b9")),
                    (Some((_, "dim7")), Some((_, "7"))) if d == -1 => Some(("Diminished to dominant", "a dim7 with any note down a half step is a dominant 7th")),
                    _ => match (from, to) {
                        (9, 8) if fam_name.contains("diminished") => Some(("Diminished to altered", "lower the 13th: the diminished sound becomes altered")),
                        (7, 8) if fam_name.contains("diminished") => Some(("Diminished to altered", "raise the 5th: the diminished sound becomes altered")),
                        (8, 9) if fam_name.contains("altered") => Some(("Altered to diminished", "raise the b13 to the 13th: the altered sound becomes diminished")),
                        (2, 1) if has(5) && !has(4) && d == -1 => Some(("Core rule", "lower the 9th of a 9sus4: 7b9sus4")),
                        _ => None,
                    },
                };
                // anything else: a half step that makes a chord players name
                const NAMED: [&str; 16] =
                    ["6", "m6", "7", "maj7", "m7", "mMaj7", "m7b5", "dim7", "7sus4", "7#5", "maj7#5", "7b5", "7b9", "7#9", "9", "maj9"];
                let named = after.as_ref().is_some_and(|s| s.exact && NAMED.contains(&s.suffix.as_str()));
                if book.is_none() && (d.abs() == 2 || !named) {
                    continue;
                }
                let tag = match (from, to) {
                    (4, 3) => "major to minor",
                    (3, 4) => "minor to major",
                    (11, 10) => "major 7th to b7",
                    (10, 11) => "b7 to major 7th",
                    (7, 6) => "flat the 5th",
                    (7, 8) => "sharp the 5th",
                    (10, 9) => "b7 down to the 6th",
                    (9, 10) => "6th up to the b7",
                    (4, 5) => "3rd up to the 4th: sus",
                    (5, 4) => "sus down to the 3rd",
                    (2, 1) => "9 to b9",
                    (2, 3) => "9 to #9",
                    _ => "",
                };
                let dir = if d > 0 { "up" } else { "down" };
                let step = if d.abs() == 1 { "a half step" } else { "a whole step" };
                let mut new_deg = Voicing { pcs: pcs.clone(), ..Default::default() }.degrees(self)[vi].clone();
                let mut old_deg = deg[vi].clone();
                // (a minor 3rd here is the chord's b3, not a #9; a raised 5th its #5)
                match (from, to) {
                    (4, 3) => new_deg = "b3".into(),
                    (3, 4) => old_deg = "b3".into(),
                    (7, 8) => new_deg = "#5".into(),
                    _ => {}
                }
                let mut how = format!("{} {} {dir} {step} ({old_deg} to {new_deg})", voice_name(vi, n), a.spelled[vi]);
                let kind = match book {
                    Some((k, words)) => {
                        how += &format!(": {words}");
                        k
                    }
                    None => {
                        if !tag.is_empty() {
                            how += &format!(": {tag}");
                        }
                        "One note"
                    }
                };
                // a note out of the scale spelled the way it moved (B down to Bb, not A#)
                let name = if self.scale.contains(&pcs[vi]) { self.spell(pcs[vi]) } else { theory::note_name(pcs[vi], d > 0).to_string() };
                if let Some(r) = make(kind, how + ".", pcs.clone(), strings, near(&pcs, strings), vec![vi], Some((vi, name, new_deg.clone()))) {
                    out.push(r);
                }
            }
        }
        // 2. voice exchange: neighbouring voices a 2nd apart trade notes, each staying on its string
        if let Some(g) = here.grip {
            let pitch = |v: usize| g[strings[v]].map(|f| theory::STANDARD[strings[v]] + f as i32);
            for v in 0..n.saturating_sub(1) {
                let (Some(p), Some(q)) = (pitch(v), pitch(v + 1)) else { continue };
                if !(1..=2).contains(&(q - p)) {
                    continue;
                }
                let mut pcs = a.pcs.clone();
                pcs.swap(v, v + 1);
                let how = format!(
                    "The {} and {} trade notes ({} and {}, a {} apart): the same strings, a new voicing.",
                    voice_name(v, n).to_lowercase(),
                    voice_name(v + 1, n).to_lowercase(),
                    a.spelled[v],
                    a.spelled[v + 1],
                    if q - p == 1 { "half step" } else { "whole step" }
                );
                if let Some(r) = make("Voice exchange", how, pcs.clone(), strings, near(&pcs, strings), vec![v, v + 1], None) {
                    out.push(r);
                }
            }
        }
        // 3. the next inversion: every voice to the next note of the chord (lined up as a scale), up and down
        let mut tones = a.pcs.clone();
        tones.sort();
        tones.dedup();
        if tones.len() == n && n > 2 {
            for (d, words) in [(1i32, "up"), (-1, "down")] {
                let pcs: Vec<i32> = a
                    .pcs
                    .iter()
                    .map(|p| {
                        let k = tones.iter().position(|x| x == p).unwrap_or(0) as i32;
                        tones[(k + d).rem_euclid(n as i32) as usize]
                    })
                    .collect();
                let how = format!("Every voice {words} to its next chord tone, on the same strings.");
                if let Some(r) = make("Next inversion", how, pcs.clone(), strings, near(&pcs, strings), (0..n).collect(), None) {
                    out.push(r);
                }
            }
        }
        // 4. a tritone away (dominant families): a #11 grip there is altered, an altered one #11
        if self.kind() == Kind::Dominant {
            let pcs: Vec<i32> = a.pcs.iter().map(|p| (p + 6).rem_euclid(12)).collect();
            let how = "The same shape a tritone away: over the same bass, #11 turns altered and altered turns #11.".to_string();
            if let Some(r) = make("Tritone", how, pcs, strings, shifted(-6).or_else(|| shifted(6)), (0..n).collect(), None) {
                out.push(r);
            }
        }
        // 5. his rule of the fifth (and the fourth): the same shape moved, on the same strings
        for (semis, kind, words) in [(7, "Rule of the fifth", "up a 5th (or down a 4th)"), (5, "Rule of the fourth", "up a 4th (or down a 5th)")] {
            let pcs: Vec<i32> = a.pcs.iter().map(|p| (p + semis).rem_euclid(12)).collect();
            let how = format!("The same shape {words}: the sound holds.");
            if let Some(r) = make(kind, how, pcs, strings, shifted(semis - 12).or_else(|| shifted(semis)), (0..n).collect(), None) {
                out.push(r);
            }
        }
        // 6. string transference: the note on one E string moved to the other (when the set has just one of them)
        let (lo_e, hi_e) = (strings.contains(&0), strings.contains(&5));
        if lo_e != hi_e && n > 1 {
            let (on, pcs, moved, words) = if hi_e {
                let mut on: Vec<usize> = strings.iter().copied().filter(|s| *s != 5).collect();
                on.insert(0, 0);
                let mut pcs = vec![a.pcs[n - 1]];
                pcs.extend_from_slice(&a.pcs[..n - 1]);
                (on, pcs, 0, format!("The {} on the high E moves to the low E", a.spelled[n - 1]))
            } else {
                let mut on: Vec<usize> = strings.iter().copied().filter(|s| *s != 0).collect();
                on.push(5);
                let mut pcs = a.pcs[1..].to_vec();
                pcs.push(a.pcs[0]);
                (on, pcs, n - 1, format!("The {} on the low E moves to the high E", a.spelled[0]))
            };
            let mut nums: Vec<usize> = on.iter().map(|s| 6 - s).collect();
            nums.sort();
            let set: Vec<String> = nums.iter().map(|x| x.to_string()).collect();
            let how = format!("{words}: the same notes on the {} strings.", set.join(" "));
            if let Some(r) = make("String transference", how, pcs.clone(), &on, near(&pcs, &on), vec![moved], None) {
                out.push(r);
            }
        }
        // the string set a list of strings makes, as written ("1 2 4 5")
        let set_name = |on: &[usize]| {
            let mut nums: Vec<usize> = on.iter().map(|s| 6 - s).collect();
            nums.sort();
            nums.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ")
        };
        // 6b. octave transference: an inner voice an octave up onto the next string above the set (the new top),
        // or an octave down onto the next string below it (the new bass); the others stay where they are
        if n >= 3 {
            let above = (strings[n - 1] + 1..6).find(|s| !strings.contains(s));
            let below = (0..strings[0]).rev().find(|s| !strings.contains(s));
            for vi in 1..n - 1 {
                for (to, up) in [(above, true), (below, false)] {
                    let Some(to) = to else { continue };
                    let mut on: Vec<usize> = strings.iter().copied().filter(|s| *s != strings[vi]).collect();
                    let mut pcs: Vec<i32> = a.pcs.iter().enumerate().filter(|(k, _)| *k != vi).map(|(_, p)| *p).collect();
                    let moved = if up {
                        on.push(to);
                        pcs.push(a.pcs[vi]);
                        n - 1
                    } else {
                        on.insert(0, to);
                        pcs.insert(0, a.pcs[vi]);
                        0
                    };
                    let how = format!(
                        "The {} {} moves an octave {} to become the {}: the same notes on the {} strings.",
                        voice_name(vi, n).to_lowercase(),
                        a.spelled[vi],
                        if up { "up" } else { "down" },
                        if up { "top" } else { "bass" },
                        set_name(&on)
                    );
                    if let Some(r) = make("Octave transference", how, pcs.clone(), &on, near(&pcs, &on), vec![moved], None) {
                        out.push(r);
                    }
                }
            }
        }
        // 6c. a fifth (and sixth) note: a bass on a string below the set, or a top note on a string above it, that
        // makes a chord with a name (the family's root as the bass always)
        const BIG: [&str; 18] =
            ["9", "maj9", "m9", "7b9", "7#9", "7#11", "maj7#11", "13", "maj13", "m13", "11", "m11", "69", "m69", "9sus4", "7b13", "7alt", "m7b5"];
        let lowest = (0..strings[0]).find(|s| !strings.contains(s));
        let highest = (strings[n - 1] + 1..6).rev().find(|s| !strings.contains(s));
        for (to, bass) in [(lowest, true), (highest, false)] {
            let Some(to) = to else { continue };
            let mut found = 0;
            // (the root first as a bass, then the scale in order)
            let mut cands: Vec<i32> = self.scale.clone();
            if bass {
                cands.sort_by_key(|p| *p != self.root);
            }
            for p in cands {
                if a.pcs.contains(&p) || found == 2 {
                    continue;
                }
                let mut on = strings.to_vec();
                let mut pcs = a.pcs.clone();
                if bass {
                    on.insert(0, to);
                    pcs.insert(0, p);
                } else {
                    on.push(to);
                    pcs.push(p);
                }
                // (any name the analyzer gives these exact notes, not just its first: E G D F# B is Em9 as well as Gmaj7/E)
                let sp = theory::Spelling { sharps: self.sharps, slash: true, ..Default::default() };
                let named = theory::suggestions(&pcs, pcs.first().copied())
                    .into_iter()
                    // (a new bass is the chord's root: Em9, not Gmaj9/A)
                    .find(|s| s.exact && s.missing.is_empty() && BIG.contains(&s.suffix.as_str()) && (!bass || s.root == p))
                    .map(|s| s.name(sp));
                if !(named.is_some() || (bass && p == self.root)) {
                    continue;
                }
                let how = if bass {
                    format!("{} in the bass on the {} string: five notes, a fuller chord.", self.spell(p), 6 - to)
                } else {
                    format!("{} on top on the {} string: five notes, a new top line.", self.spell(p), 6 - to)
                };
                let moved = if bass { 0 } else { n };
                if let Some(mut r) = make(if bass { "Add a bass" } else { "Add a top note" }, how, pcs.clone(), &on, near(&pcs, &on), vec![moved], None) {
                    if let Some(nm) = named {
                        r.name = nm;
                    }
                    out.push(r);
                    found += 1;
                }
            }
        }
        // 7. planing: every voice to the next note of the scale, up and down
        let sc = {
            let mut s = self.scale.clone();
            s.sort();
            s.dedup();
            s
        };
        if a.pcs.iter().all(|p| sc.contains(p)) && sc.len() > 1 {
            for (d, words) in [(1i32, "up"), (-1, "down")] {
                let pcs: Vec<i32> = a
                    .pcs
                    .iter()
                    .map(|p| {
                        let k = sc.iter().position(|x| x == p).unwrap_or(0) as i32;
                        sc[(k + d).rem_euclid(sc.len() as i32) as usize]
                    })
                    .collect();
                let how = format!("Every voice {words} to the next scale note: the shape slides, the sound stays.");
                if let Some(r) = make("Planing", how, pcs.clone(), strings, near(&pcs, strings), (0..n).collect(), None) {
                    out.push(r);
                }
            }
        }
        // 8. his other grips with these notes: the same chord, maybe an easier one
        let mut key = a.pcs.clone();
        key.sort();
        for (k, st) in walk.steps.iter().enumerate() {
            let b = self.voicing(st.at);
            let mut bk = b.pcs.clone();
            bk.sort();
            if k == i || bk != key || b.pcs == a.pcs || out.iter().any(|r| r.pcs == b.pcs && r.strings == strings) {
                continue;
            }
            let span = |g: &[Option<u8>; 6]| {
                let f: Vec<u8> = g.iter().flatten().copied().filter(|f| *f > 0).collect();
                f.iter().max().copied().unwrap_or(0) as i32 - f.iter().min().copied().unwrap_or(0) as i32
            };
            let easier = match (st.grip, here.grip) {
                (Some(x), Some(y)) if span(&x) < span(&y) => " (a smaller stretch)",
                _ => "",
            };
            let how = format!("The same notes in another order{easier}.");
            let moved: Vec<usize> = (0..n).filter(|&v| b.pcs.get(v) != a.pcs.get(v)).collect();
            if let Some(r) = make("Same notes", how, b.pcs.clone(), strings, st.grip, moved, None) {
                out.push(r);
            }
        }
        // his rules first, then the moves, then the rest
        let rank = |k: &str| match k {
            "Core rule" | "The chain" => 0,
            "Diminished to altered" | "Altered to diminished" | "Diminished to dominant" => 1,
            "Voice exchange" | "Next inversion" => 2,
            "Tritone" | "Rule of the fifth" | "Rule of the fourth" | "String transference" | "Octave transference" => 3,
            "Same notes" | "Add a bass" | "Add a top note" => 4,
            "Planing" => 5,
            _ => 6,
        };
        out.sort_by_key(|r| rank(r.kind));
        out
    }
}

/// A rule applied to a voicing (see Family::rules).
#[derive(Clone, Debug)]
pub struct Rule {
    /// which rule: "Core rule", "The chain", "Diminished to altered", "Altered to diminished", "Diminished to
    /// dominant", "One note", "Voice exchange", "Next inversion", "Tritone", "Rule of the fifth", "Rule of the
    /// fourth", "String transference", "Octave transference", "Add a bass", "Add a top note", "Planing", "Same notes";
    /// "Resolution" (where it goes) and "Passing chord" (a chord before it that leads into it)
    pub kind: &'static str,
    /// the change in words
    pub how: String,
    pub spelled: Vec<String>,
    pub pcs: Vec<i32>,
    /// where it's played: the strings (low to high, 0 = low E; another set after a string transference) and the grip
    pub strings: Vec<usize>,
    pub grip: Option<[Option<u8>; 6]>,
    /// its name on its own root, and as a chord on the family's root
    pub name: String,
    pub over: String,
    pub degrees: Vec<String>,
    /// all its notes are in the family's scale
    pub in_scale: bool,
    /// the step of his walk with these notes in this order, when he plays it
    pub his: Option<usize>,
    /// the voices that move
    pub moved: Vec<usize>,
    /// for a resolution: where the notes say it goes, and a grip from the set's own family of that chord
    pub stated: bool,
    pub from_book: bool,
    /// a resolution whose landing chord is written out in the note (from the lesson or the printed book), as written
    pub given: bool,
}

/// A chord a dominant lands on: its root, and minor or major.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub root: i32,
    pub minor: bool,
}

/// A word that names a chord to land on ("F", "Fm", "Bbm6", "Abmaj7", "Gbm(maj9)"): its root, and Some(minor) for a
/// written quality (None for a bare note). Dominant chords ("Db7", "F7#5") and other words are None.
fn landing_chord(w: &str) -> Option<(i32, Option<bool>)> {
    let w = w.trim_matches(|c: char| "(),".contains(c));
    let mut ch = w.chars();
    let c0 = ch.next()?;
    if !('A'..='G').contains(&c0) {
        return None;
    }
    let cut = if matches!(ch.next(), Some('#') | Some('b')) { 2 } else { 1 };
    let root = pc_of(w.get(..cut)?)?;
    let rest = &w[cut..];
    let q = if rest.is_empty() {
        None
    } else if rest.starts_with("maj") || rest.starts_with('6') || rest.starts_with("add") {
        Some(false)
    } else if rest.starts_with('m') && rest[1..].chars().next().is_none_or(|c| c.is_ascii_digit() || c == '(') {
        Some(true)
    } else {
        return None;
    };
    Some((root, q))
}

/// One place a note names, remembered as the one a written-out landing that follows belongs to.
fn add_target(t: Target, out: &mut Vec<Target>, last: &mut Option<Target>) {
    if !out.contains(&t) {
        out.push(t);
    }
    *last = Some(t);
}

/// The end of a run of bare note names: one or two are places ("G or Db"), more are a landing written out note by
/// note ("C (E G B D)"), kept when it has `n` notes, with the place named just before it.
#[allow(clippy::too_many_arguments)]
fn close_run(
    run: &mut Vec<(i32, usize)>,
    words: &[&str],
    both: bool,
    n: usize,
    out: &mut Vec<Target>,
    landings: &mut Vec<(Option<Target>, Vec<i32>)>,
    last: &mut Option<Target>,
) {
    if run.len() >= 3 {
        if run.len() == n {
            landings.push((*last, run.iter().map(|r| r.0).collect()));
        }
    } else {
        for &(root, j) in run.iter() {
            let next = |d: usize| words.get(j + d).map(|w| w.trim_matches(|c: char| "(),".contains(c))).unwrap_or("");
            let (maj, min) = match next(1) {
                "minor" => (false, true),
                "major" if next(2) == "or" && next(3) == "minor" => (true, true),
                "major" => (true, false),
                _ => (true, both),
            };
            if maj {
                add_target(Target { root, minor: false }, out, last);
            }
            if min {
                add_target(Target { root, minor: true }, out, last);
            }
        }
    }
    run.clear();
}

/// Where a note says a chord goes: the chords named after "resolve" up to the end of that sentence ("Resolves to F or
/// Fm", "to D major or minor", "to G, Bb, Db or E"), and landings written out note by note, low to high, in brackets
/// after their place ("Resolves to F (E A C F) or to B (D# A B F#)") or bare ("resolves to C F# G E"), when they have
/// `n` notes. A dominant named there ("the Db7 sound to C minor") isn't a place it lands.
pub fn stated_targets(text: &str, n: usize) -> (Vec<Target>, Vec<(Option<Target>, Vec<i32>)>) {
    let mut out: Vec<Target> = Vec::new();
    let mut landings: Vec<(Option<Target>, Vec<i32>)> = Vec::new();
    let mut from = 0;
    while let Some(k) = text.get(from..).and_then(|t| t.find("esolv")) {
        let start = from + k;
        let end = text[start..].find(['.', ';', ':']).map_or(text.len(), |e| start + e);
        let part = &text[start..end];
        let both = part.contains("major or minor") || part.contains("any kind");
        let words: Vec<&str> = part.split_whitespace().collect();
        let mut run: Vec<(i32, usize)> = Vec::new();
        let mut last: Option<Target> = None;
        for (j, w) in words.iter().enumerate() {
            match landing_chord(w) {
                Some((root, None)) => {
                    // (a bracket opens a landing: the place before it is done)
                    if w.starts_with('(') {
                        close_run(&mut run, &words, both, n, &mut out, &mut landings, &mut last);
                    }
                    run.push((root, j));
                    if w.ends_with([',', ')']) {
                        close_run(&mut run, &words, both, n, &mut out, &mut landings, &mut last);
                    }
                }
                Some((root, Some(minor))) => {
                    close_run(&mut run, &words, both, n, &mut out, &mut landings, &mut last);
                    add_target(Target { root, minor }, &mut out, &mut last);
                }
                None => close_run(&mut run, &words, both, n, &mut out, &mut landings, &mut last),
            }
        }
        close_run(&mut run, &words, both, n, &mut out, &mut landings, &mut last);
        from = end.max(start + 5);
    }
    (out, landings)
}

/// How far a resolution moves the root, in words ("Down a 5th", "Down a half step").
fn root_move(d: i32) -> &'static str {
    match d.rem_euclid(12) {
        0 => "To its own root",
        1 => "Up a half step",
        2 => "Up a whole step",
        3 => "Up a minor 3rd",
        4 => "Up a major 3rd",
        5 => "Down a 5th",
        6 => "A tritone away",
        7 => "Down a 4th",
        8 => "Down a major 3rd",
        9 => "Down a minor 3rd",
        10 => "Down a whole step",
        _ => "Down a half step",
    }
}

impl Family {
    /// Where step `i` resolves, each as a Rule of kind "Resolution": the places its note (or the family's text)
    /// names first, then for a dominant down a 5th or (as a tritone sub) down a half step, major or minor; a
    /// diminished grip works on four dominants a minor 3rd apart, so it lands on all four of their homes. Each is
    /// voice-led from the grip it's played with, on the same strings: every voice to a note of the new chord a few
    /// half steps away at most, the fewest and smallest moves (the 7th falls to the 3rd, the 3rd holds as the 7th),
    /// or one of the set's own grips of that chord when it's as smooth.
    pub fn resolutions(&self, set: &Set, walk: &Walk, i: usize) -> Vec<Rule> {
        let strings = &set.strings[..];
        let Some(here) = walk.steps.get(i) else { return Vec::new() };
        let Some(g) = here.grip else { return Vec::new() };
        let a = self.voicing(here.at);
        let n = a.pcs.len();
        if n < 3 || n != strings.len() {
            return Vec::new();
        }
        let Some(src) = strings.iter().map(|&s| g[s].map(|f| theory::STANDARD[s] + f as i32)).collect::<Option<Vec<i32>>>() else {
            return Vec::new();
        };
        let root = if self.own_roots { a.own_chord().map_or(self.root, |s| s.root) } else { self.root };
        let (said, landings) = stated_targets(&a.note, n);
        let mut targets: Vec<(Target, bool)> = said.into_iter().map(|t| (t, true)).collect();
        if targets.is_empty() && landings.is_empty() {
            targets.extend(stated_targets(&self.about, 0).0.into_iter().map(|t| (t, true)));
        }
        let fam_name = self.name.to_lowercase();
        if self.kind() == Kind::Dominant && !self.own_roots {
            let mut moves = vec![5, 11];
            if fam_name.contains("diminished") {
                moves.extend([8, 2]);
            }
            for d in moves {
                for minor in [false, true] {
                    targets.push((Target { root: (root + d).rem_euclid(12), minor }, false));
                }
            }
        }
        let mut seen: Vec<Target> = Vec::new();
        targets.retain(|(t, _)| {
            let t = Target { root: t.root.rem_euclid(12), ..*t };
            if seen.contains(&t) {
                false
            } else {
                seen.push(t);
                true
            }
        });
        targets.truncate(8);
        // each home with its minor beside it, in the order they came up
        let roots: Vec<i32> = targets.iter().map(|(t, _)| t.root.rem_euclid(12)).fold(Vec::new(), |mut v, r| {
            if !v.contains(&r) {
                v.push(r);
            }
            v
        });
        targets.sort_by_key(|(t, _)| (roots.iter().position(|r| *r == t.root.rem_euclid(12)), t.minor));
        let src_deg = a.degrees(self);
        let mut out: Vec<Rule> = Vec::new();
        // the landings written out (from the lesson or the book) first, as written
        let mut given: Vec<Target> = Vec::new();
        for (t, pcs) in landings {
            let t = t.or_else(|| {
                let s = Voicing { pcs: pcs.clone(), ..Default::default() }.own_chord()?;
                Some(Target { root: s.root, minor: s.suffix.starts_with('m') && !s.suffix.starts_with("maj") })
            });
            let Some(t) = t else { continue };
            if let Some(mut r) = self.land(set, &src, &src_deg, root, t, Some(&pcs), true) {
                r.given = true;
                given.push(t);
                out.push(r);
            }
        }
        for (t, stated) in targets {
            if given.contains(&t) {
                continue;
            }
            if let Some(r) = self.land(set, &src, &src_deg, root, t, None, stated) {
                if !out.iter().any(|o| o.pcs == r.pcs && o.name == r.name) {
                    out.push(r);
                }
            }
        }
        out
    }

    /// One resolution of the voicing sounding `src` (pitches on the set's strings, its degrees `src_deg`, the chord's
    /// root `root`) to `t`: these exact notes when `only` says so, else the smoothest voice-leading.
    #[allow(clippy::too_many_arguments)]
    fn land(&self, set: &Set, src: &[i32], src_deg: &[String], root: i32, t: Target, only: Option<&[i32]>, stated: bool) -> Option<Rule> {
        let strings = &set.strings[..];
        let n = src.len();
        let troot = t.root.rem_euclid(12);
        let third = if t.minor { 3 } else { 4 };
        let tones: Vec<i32> = if t.minor { vec![0, 2, 3, 7, 9, 10] } else { vec![0, 2, 4, 7, 9, 11] };
        // how much a landing costs: each voice's move (a leap in the top voice double), a bass that blurs the chord
        let cost = |q: &[i32]| -> i32 {
            let mut c = 0;
            for (k, (x, y)) in src.iter().zip(q).enumerate() {
                let d = (x - y).abs();
                c += d * 2;
                if k + 1 == n && d > 2 {
                    c += d * 2;
                }
            }
            let rel = |p: i32| (p - troot).rem_euclid(12);
            c += match (rel(q[0]), t.minor) {
                (2, _) => 2,
                (11, false) | (9, true) => 4,
                (9, false) => 3,
                _ => 0,
            };
            let has = |r: i32| q.iter().any(|p| rel(*p) == r);
            if !t.minor && has(9) && has(11) {
                c += 3;
            }
            c
        };
        // the chord, not a fragment: its 3rd, its 7th or 6th (four notes or more), no note doubled in a small voicing
        let full = |q: &[i32]| -> bool {
            let mut rels: Vec<i32> = q.iter().map(|p| (p - troot).rem_euclid(12)).collect();
            rels.sort();
            rels.dedup();
            // (a 7th, a 6th or an added 9th: Cmaj7, C6, C6/9, Cadd9)
            let colors = if t.minor { [10, 9, 2] } else { [11, 9, 2] };
            // (an 11th or #11 only with a 7th or 6th: Fm11 and Cmaj7#11, not "Fm add9 add11" or "C add9 add#11")
            let eleven_ok = !(rels.contains(&5) || rels.contains(&6)) || rels.contains(&colors[0]) || rels.contains(&9);
            rels.contains(&third) && (n < 4 || colors.iter().any(|s| rels.contains(s))) && rels.len() >= n.min(5) && eleven_ok
        };
        // (a landing the lesson spells may stretch further than one worked out here)
        let reach: u8 = if only.is_some() { 7 } else { 5 };
        let fits = |q: &[i32]| fit_grip(q, strings, reach);
        // (pitches, grip, cost, from the book)
        let mut best: Option<(Vec<i32>, [Option<u8>; 6], i32, bool)> = None;
        let consider = |q: Vec<i32>, book: bool, best: &mut Option<(Vec<i32>, [Option<u8>; 6], i32, bool)>| {
            if q.windows(2).any(|w| w[1] <= w[0]) {
                return;
            }
            let Some(g) = fits(&q) else { return };
            let c = cost(&q) - if book { 2 } else { 0 };
            if best.as_ref().is_none_or(|b| c < b.2) {
                *best = Some((q, g, c, book));
            }
        };
        let near = |pcs: &[i32], reach: i32| -> Vec<Vec<i32>> {
            // the notes in order on these strings, each voice within `reach` half steps of where it was
            let mut all: Vec<Vec<i32>> = vec![Vec::new()];
            for k in 0..n {
                let mut next = Vec::new();
                for cur in &all {
                    for d in -reach..=reach {
                        let q = src[k] + d;
                        if q.rem_euclid(12) == pcs[k].rem_euclid(12) && cur.last().is_none_or(|l| q > *l) {
                            let mut c = cur.clone();
                            c.push(q);
                            next.push(c);
                        }
                    }
                }
                all = next;
            }
            all
        };
        match only {
            Some(pcs) => {
                // (a written landing as written, however far its voices go; anywhere on the neck if it must)
                for q in near(pcs, 7) {
                    consider(q, false, &mut best);
                }
                if best.is_none() {
                    for g in all_grips(pcs, strings) {
                        let q: Vec<i32> = strings.iter().filter_map(|&s| g[s].map(|f| theory::STANDARD[s] + f as i32)).collect();
                        if q.len() == n {
                            consider(q, false, &mut best);
                        }
                    }
                }
            }
            None => {
                // every voice to a note of the chord a few half steps away
                let mut all: Vec<Vec<i32>> = vec![Vec::new()];
                for k in 0..n {
                    let mut next = Vec::new();
                    for cur in &all {
                        for d in -3..=3 {
                            let q = src[k] + d;
                            if tones.contains(&(q - troot).rem_euclid(12)) && cur.last().is_none_or(|l| q > *l) {
                                let mut c = cur.clone();
                                c.push(q);
                                next.push(c);
                            }
                        }
                    }
                    all = next;
                }
                for q in all {
                    if full(&q) {
                        consider(q, false, &mut best);
                    }
                }
                // the set's own grips of that chord, moved to its root
                if let Some(fam) = set.family_for(if t.minor { "Minor 7th" } else { "Major 7th" }, if t.minor { Kind::Minor } else { Kind::Major }) {
                    let by = (troot - fam.root).rem_euclid(12);
                    for at in fam.order() {
                        let pcs: Vec<i32> = fam.voicing(at).pcs.iter().map(|p| (p + by).rem_euclid(12)).collect();
                        if pcs.len() == n {
                            for q in near(&pcs, 4).into_iter().filter(|q| full(q)) {
                                consider(q, true, &mut best);
                            }
                        }
                    }
                }
            }
        }
        let (q, grip, _, from_book) = best?;
        let pcs: Vec<i32> = q.iter().map(|p| p.rem_euclid(12)).collect();
        // the chord it lands on, spelled in its key (flats for F, Bb, Eb... and their relative minors); a minor on a
        // black key named the way its major is (Gb and Gbm, as the book pairs them)
        let major_key = if t.minor && ![1, 3, 6, 8, 10].contains(&troot) { troot + 3 } else { troot };
        let sharps = !matches!(major_key.rem_euclid(12), 5 | 10 | 3 | 8 | 1 | 6);
        let scale: Vec<i32> =
            (if t.minor { [0, 2, 3, 5, 7, 9, 10] } else { [0, 2, 4, 5, 7, 9, 11] }).iter().map(|x| (troot + x).rem_euclid(12)).collect();
        let tf = Family {
            root: troot,
            scale_text: scale.iter().map(|&p| theory::note_name(p, sharps).to_string()).collect::<Vec<_>>().join(" "),
            scale,
            sharps,
            ..Default::default()
        };
        let spelled: Vec<String> = pcs.iter().map(|&p| tf.spell(p)).collect();
        let v = Voicing { pcs: pcs.clone(), spelled: spelled.clone(), ..Default::default() };
        let name = v.symbol(&tf).replace("mMaj7", "m(maj7)");
        let degrees = v.degrees(&tf);
        let from_sp: Vec<String> = src.iter().map(|p| self.spell(*p)).collect();
        // each voice: where it goes, and what it was and becomes
        let mut parts: Vec<String> = Vec::new();
        let mut holds: Vec<String> = Vec::new();
        for k in 0..n {
            let d = q[k] - src[k];
            let deg = |s: &str| if s == "R" { "root".to_string() } else { s.to_string() };
            if d == 0 {
                holds.push(format!("{} {} stays, {} becomes {}", voice_name(k, n), from_sp[k], deg(&src_deg[k]), deg(&degrees[k])));
            } else {
                parts.push(format!("{} {} to {} ({}, {} to {})", voice_name(k, n), from_sp[k], spelled[k], step_words(d), deg(&src_deg[k]), deg(&degrees[k])));
            }
        }
        parts.extend(holds);
        let mv = root_move(troot - root);
        let mv = if (troot - root).rem_euclid(12) == 11 { format!("{mv}, as a tritone sub") } else { mv.to_string() };
        let how = format!("{mv}{}. {}.", if t.minor { ", to minor" } else { "" }, parts.join("; "));
        Some(Rule {
            kind: "Resolution",
            how,
            spelled,
            in_scale: true,
            over: format!("{}{}", root_move(troot - root), if t.minor { ", Minor" } else { "" }),
            degrees,
            his: None,
            strings: strings.to_vec(),
            moved: (0..n).filter(|&k| q[k] != src[k]).collect(),
            grip: Some(grip),
            name,
            pcs,
            stated,
            from_book,
            given: false,
        })
    }
}

/// Pitches on `strings` (low to high) as a grip, when they sit within `reach` frets of each other (open strings only
/// in a shape down by the nut), up to the 19th fret.
fn fit_grip(q: &[i32], strings: &[usize], reach: u8) -> Option<[Option<u8>; 6]> {
    let mut g = [None; 6];
    for (k, &s) in strings.iter().enumerate() {
        let f = q[k] - theory::STANDARD[s];
        if !(0..=19).contains(&f) {
            return None;
        }
        g[s] = Some(f as u8);
    }
    let fretted: Vec<u8> = g.iter().flatten().copied().filter(|f| *f > 0).collect();
    let (lo, hi) = (fretted.iter().min().copied().unwrap_or(0), fretted.iter().max().copied().unwrap_or(0));
    (hi - lo <= reach && (!g.iter().flatten().any(|f| *f == 0) || hi <= reach)).then_some(g)
}

/// A note spelled as a degree of a chord on `root` (a note name): the letter `letter` steps up from the root's (1 =
/// the root's own), with the sharp or flat that makes it `pc`; plainly when that would take a double sharp or flat.
fn spell_as(root: &str, letter: usize, pc: i32, sharps: bool) -> String {
    const LETTERS: [char; 7] = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];
    const NATURAL: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
    let Some(l0) = LETTERS.iter().position(|c| root.starts_with(*c)) else {
        return theory::note_name(pc, sharps).to_string();
    };
    let l = (l0 + letter.max(1) - 1) % 7;
    let diff = ((pc - NATURAL[l]).rem_euclid(12) + 6) % 12 - 6;
    match diff {
        -1 => format!("{}b", LETTERS[l]),
        0 => LETTERS[l].to_string(),
        1 => format!("{}#", LETTERS[l]),
        _ => theory::note_name(pc, sharps).to_string(),
    }
}

/// A kind of passing chord worked out by voice-leading into the chord it comes before.
struct Lead {
    /// what it is, short (a button's label when names are hidden)
    label: &'static str,
    /// its root, over the chord it leads into
    by: i32,
    /// its notes over its own root, each with the letter it's spelled on (1 = the root's)
    tones: &'static [(i32, usize)],
    /// notes it must have (its 3rd and 7th)
    needs: &'static [i32],
    /// and one of these, when there are any (an altered note)
    one_of: &'static [i32],
    /// its root spelled with sharps (Some), with flats (Some(false)), or the family's way (None)
    sharps: Option<bool>,
    /// a diminished 7th (named by hand: the analyzer names it on any of its four notes)
    dim: bool,
}

/// The passing chords worked out by voice-leading (the half steps above and below are the grip itself, moved).
const LEADS: [Lead; 5] = [
    Lead {
        label: "V7",
        by: 7,
        tones: &[(0, 1), (2, 2), (4, 3), (7, 5), (9, 6), (10, 7)],
        needs: &[4, 10],
        one_of: &[],
        sharps: None,
        dim: false,
    },
    Lead {
        label: "V7 altered",
        by: 7,
        tones: &[(0, 1), (1, 2), (3, 3), (4, 3), (8, 6), (10, 7)],
        needs: &[4, 10],
        one_of: &[1, 3, 8],
        sharps: None,
        dim: false,
    },
    Lead {
        label: "Tritone sub",
        by: 1,
        tones: &[(0, 1), (2, 2), (4, 3), (6, 4), (7, 5), (9, 6), (10, 7)],
        needs: &[4, 10],
        one_of: &[],
        sharps: Some(false),
        dim: false,
    },
    Lead {
        label: "Dim below",
        by: -1,
        tones: &[(0, 1), (3, 3), (6, 5), (9, 6)],
        needs: &[],
        one_of: &[],
        sharps: Some(true),
        dim: true,
    },
    Lead {
        label: "Dim on the root",
        by: 0,
        tones: &[(0, 1), (3, 3), (6, 5), (9, 6)],
        needs: &[],
        one_of: &[],
        sharps: None,
        dim: true,
    },
];

impl Family {
    /// Passing chords into step `i`: chords played just before it that lead into it, on the same strings, each a Rule
    /// of kind "Passing chord" (its grip is the passing chord's; the move is from it into step `i`):
    /// - the same grip a half step above and a half step below: every voice slides in by a half step
    /// - its V7, plain and altered, and the tritone sub (the dominant a half step above): the dominant that
    ///   resolves to it
    /// - a diminished 7th a half step below its root (C#dim7 into Dm7), and one on its root (Cdim7 into Cmaj7)
    ///
    /// The worked-out ones come in by a whole step at most in each voice (a minor 3rd when there's no other way),
    /// the fewest and smallest moves, no note doubled.
    pub fn passing(&self, set: &Set, walk: &Walk, i: usize) -> Vec<Rule> {
        let strings = &set.strings[..];
        let Some(here) = walk.steps.get(i) else { return Vec::new() };
        let Some(g) = here.grip else { return Vec::new() };
        let a = self.voicing(here.at);
        let n = a.pcs.len();
        if n < 3 || n != strings.len() {
            return Vec::new();
        }
        let Some(dst) = strings.iter().map(|&s| g[s].map(|f| theory::STANDARD[s] + f as i32)).collect::<Option<Vec<i32>>>() else {
            return Vec::new();
        };
        let root = if self.own_roots { a.own_chord().map_or(self.root, |s| s.root) } else { self.root };
        let dst_deg = a.degrees(self);
        let dst_sp: Vec<String> = dst.iter().map(|p| self.spell(*p)).collect();
        // a key's way with its accidentals: flats for F, Bb, Eb, Ab, Db, Gb
        let key_sharps = |pc: i32| !matches!(pc.rem_euclid(12), 5 | 10 | 3 | 8 | 1 | 6);
        // each voice into this chord, in words
        let moves = |q: &[i32], sp: &[String], deg: &[String]| -> String {
            let d_ = |s: &str| if s == "R" { "root".to_string() } else { s.to_string() };
            let mut parts: Vec<String> = Vec::new();
            let mut holds: Vec<String> = Vec::new();
            for k in 0..n {
                let d = dst[k] - q[k];
                if d == 0 && deg[k] == dst_deg[k] {
                    holds.push(format!("{} {} stays ({})", voice_name(k, n), sp[k], d_(&deg[k])));
                } else if d == 0 {
                    holds.push(format!("{} {} stays, {} becomes {}", voice_name(k, n), sp[k], d_(&deg[k]), d_(&dst_deg[k])));
                } else {
                    parts.push(format!("{} {} {} to {} ({} to {})", voice_name(k, n), sp[k], step_words(d), dst_sp[k], d_(&deg[k]), d_(&dst_deg[k])));
                }
            }
            parts.extend(holds);
            parts.join("; ")
        };
        let in_scale = |pcs: &[i32]| pcs.iter().all(|p| self.scale.contains(p));
        let mut out: Vec<Rule> = Vec::new();
        // the same grip a half step above and below
        for d in [1, -1] {
            let q: Vec<i32> = dst.iter().map(|p| p + d).collect();
            let Some(grip) = fit_grip(&q, strings, 7) else { continue };
            let pcs: Vec<i32> = q.iter().map(|p| p.rem_euclid(12)).collect();
            let sharps = key_sharps(root + d);
            let scale: Vec<i32> = self.scale.iter().map(|p| (p + d).rem_euclid(12)).collect();
            let tf = Family {
                root: (self.root + d).rem_euclid(12),
                scale_text: scale.iter().map(|&p| theory::note_name(p, sharps).to_string()).collect::<Vec<_>>().join(" "),
                scale,
                sharps,
                own_roots: self.own_roots,
                ..Default::default()
            };
            let spelled: Vec<String> = pcs.iter().map(|&p| tf.spell(p)).collect();
            let v = Voicing { pcs: pcs.clone(), spelled: spelled.clone(), ..Default::default() };
            let degrees = v.degrees(&tf);
            let (label, way) = if d > 0 { ("Half step above", "down") } else { ("Half step below", "up") };
            out.push(Rule {
                kind: "Passing chord",
                how: format!("{label}: the same grip, every voice slides {way} a half step into this chord."),
                name: v.symbol(&tf).replace("mMaj7", "m(maj7)"),
                spelled,
                in_scale: in_scale(&pcs),
                over: label.to_string(),
                degrees,
                his: None,
                strings: strings.to_vec(),
                moved: (0..n).collect(),
                grip: Some(grip),
                pcs,
                stated: false,
                from_book: false,
                given: false,
            });
        }
        // the ones worked out by voice-leading
        for lead in &LEADS {
            let proot = (root + lead.by).rem_euclid(12);
            let rel = |p: i32| (p - proot).rem_euclid(12);
            let tones: Vec<i32> = lead.tones.iter().map(|t| t.0).collect();
            let full = |q: &[i32]| -> bool {
                let mut rels: Vec<i32> = q.iter().map(|p| rel(*p)).collect();
                rels.sort();
                rels.dedup();
                rels.len() == n.min(tones.len())
                    && lead.needs.iter().all(|x| rels.contains(x))
                    && (lead.one_of.is_empty() || lead.one_of.iter().any(|x| rels.contains(x)))
            };
            let cost = |q: &[i32]| -> i32 {
                let mut c = 0;
                for (k, (x, y)) in dst.iter().zip(q).enumerate() {
                    let d = (x - y).abs();
                    c += d * 2;
                    if k + 1 == n && d > 2 {
                        c += d * 2;
                    }
                }
                c
            };
            // every way in that fits the hand, smoothest first (a whole step at most in each voice; a minor 3rd when
            // there's no other way)
            let mut ways: Vec<(i32, Vec<i32>, [Option<u8>; 6])> = Vec::new();
            for reach in [2, 3] {
                let mut all: Vec<Vec<i32>> = vec![Vec::new()];
                for k in 0..n {
                    let mut next = Vec::new();
                    for cur in &all {
                        for d in -reach..=reach {
                            let q = dst[k] + d;
                            if tones.contains(&rel(q)) && cur.last().is_none_or(|l| q > *l) {
                                let mut c = cur.clone();
                                c.push(q);
                                next.push(c);
                            }
                        }
                    }
                    all = next;
                }
                for q in all {
                    if q == dst || !full(&q) {
                        continue;
                    }
                    if let Some(grip) = fit_grip(&q, strings, 5) {
                        ways.push((cost(&q), q, grip));
                    }
                }
                if !ways.is_empty() {
                    break;
                }
            }
            ways.sort_by_key(|w| w.0);
            let sharps = lead.sharps.unwrap_or(self.sharps);
            let root_name = if lead.by == 0 { self.spell(proot) } else { theory::note_name(proot, sharps).to_string() };
            let letter = |p: i32| lead.tones.iter().find(|t| t.0 == rel(p)).map_or(1, |t| t.1);
            let what = match lead.label {
                "V7" => "Its V7, the dominant a 5th above",
                "V7 altered" => "Its V7 altered",
                "Tritone sub" => "The tritone sub, the dominant a half step above",
                "Dim below" => "A diminished 7th a half step below its root",
                _ => "A diminished 7th on its root, the root held",
            };
            let make = |q: &[i32], grip: [Option<u8>; 6]| -> Rule {
                let pcs: Vec<i32> = q.iter().map(|p| p.rem_euclid(12)).collect();
                let spelled: Vec<String> = pcs.iter().map(|&p| spell_as(&root_name, letter(p), p, sharps)).collect();
                let (name, degrees) = if lead.dim {
                    let deg = |p: i32| match rel(p) {
                        0 => "R",
                        3 => "b3",
                        6 => "b5",
                        _ => "bb7",
                    };
                    (format!("{root_name}dim7"), pcs.iter().map(|&p| deg(p).to_string()).collect::<Vec<_>>())
                } else {
                    let scale: Vec<i32> = lead.tones.iter().map(|t| (proot + t.0).rem_euclid(12)).collect();
                    let tf = Family {
                        root: proot,
                        scale_text: lead.tones.iter().map(|t| spell_as(&root_name, t.1, proot + t.0, sharps)).collect::<Vec<_>>().join(" "),
                        scale,
                        sharps,
                        ..Default::default()
                    };
                    let v = Voicing { pcs: pcs.clone(), spelled: spelled.clone(), ..Default::default() };
                    (v.symbol(&tf), v.degrees(&tf))
                };
                Rule {
                    kind: "Passing chord",
                    how: format!("{what}. Into this chord: {}.", moves(q, &spelled, &degrees)),
                    name,
                    in_scale: in_scale(&pcs),
                    over: lead.label.to_string(),
                    degrees,
                    spelled,
                    his: None,
                    strings: strings.to_vec(),
                    moved: (0..n).filter(|&k| q[k] != dst[k]).collect(),
                    grip: Some(grip),
                    pcs,
                    stated: false,
                    from_book: false,
                    given: false,
                }
            };
            let pcs_of = |q: &[i32]| q.iter().map(|p| p.rem_euclid(12)).collect::<Vec<i32>>();
            let Some(first) = ways.first() else { continue };
            // (the smoothest way in can be a chord already here by another name: an altered V7 is its tritone sub's
            // notes. Said there, and the next smoothest that's new is this one.)
            if let Some(j) = out.iter().position(|o| o.pcs == pcs_of(&first.1)) {
                let mut lower = what.to_string();
                lower[..1].make_ascii_lowercase();
                out[j].how = format!("{} The same notes are {lower}.", out[j].how);
                if let Some(w) = ways.iter().find(|w| !out.iter().any(|o| o.pcs == pcs_of(&w.1))) {
                    out.push(make(&w.1, w.2));
                }
            } else {
                out.push(make(&first.1, first.2));
            }
        }
        out
    }
}

/// A ii–V–I (or ii–V–i) made of a set's own grips, each chord the one nearest the chord before it: the way a
/// pianist voice-leads, the fewest notes moving the least.
#[derive(Clone, Debug, Default)]
pub struct Cadence {
    pub steps: Vec<CadenceStep>,
}

#[derive(Clone, Debug, Default)]
pub struct CadenceStep {
    /// its name (Am9, D13...) and the family it comes from
    pub name: String,
    pub family: String,
    pub spelled: Vec<String>,
    pub degrees: Vec<String>,
    pub grip: Option<[Option<u8>; 6]>,
    /// the voices that move from the chord before, and the move in words
    pub moved: Vec<usize>,
    pub how: String,
    /// the other choices here, smoothest first (this one first): their names
    pub choices: Vec<String>,
}

/// The pitches a grip sounds on `strings`, low to high (None for a string it doesn't use).
fn grip_pitches_on(g: &[Option<u8>; 6], strings: &[usize]) -> Vec<Option<i32>> {
    strings.iter().map(|&s| g[s].map(|f| theory::STANDARD[s] + f as i32)).collect()
}

/// How much moving from one grip to the next costs the ear and the hand: every voice's distance in half steps, a
/// leap in the top voice counted double (the melody should move by step), and a little for the hand's travel.
fn lead_cost(a: &[Option<u8>; 6], b: &[Option<u8>; 6], strings: &[usize]) -> i32 {
    let (pa, pb) = (grip_pitches_on(a, strings), grip_pitches_on(b, strings));
    let mut cost = 0;
    for (k, (x, y)) in pa.iter().zip(&pb).enumerate() {
        if let (Some(x), Some(y)) = (x, y) {
            let d = (x - y).abs();
            cost += d * 2;
            if k + 1 == strings.len() && d > 2 {
                cost += d * 2;
            }
        }
    }
    cost + distance(a, b) / 2
}

impl Set {
    /// The family for one chord of a cadence: the first whose name starts with `name` ("Major 7th", "Minor 7th",
    /// "Dominant 7th"), else the first of that kind.
    fn family_for(&self, name: &str, kind: Kind) -> Option<&Family> {
        self.families
            .iter()
            .find(|f| f.name == name)
            .or_else(|| self.families.iter().find(|f| f.name.starts_with(name)))
            .or_else(|| self.families.iter().find(|f| f.kind() == kind && !f.own_roots))
    }

    /// A ii–V–I in `key` (a pitch class; `minor` for ii–V–i) on this set's strings, from the `start`-th grip of the
    /// minor family (his stations first), each next chord the nearest of its family's grips moved to its root; `pick`
    /// chooses another than the nearest for the V and the I (0 = the nearest). None when the set lacks a family.
    pub fn cadence(&self, key: i32, minor: bool, start: usize, pick: [usize; 2]) -> Option<Cadence> {
        let ii = self.family_for("Minor 7th", Kind::Minor)?;
        let v = self.family_for("Dominant 7th", Kind::Dominant)?;
        let one = if minor { self.family_for("Minor 7th", Kind::Minor)? } else { self.family_for("Major 7th", Kind::Major)? };
        let flats = matches!(key.rem_euclid(12), 5 | 10 | 3 | 8 | 1 | 6);
        // a family moved to another root: its grips and names there
        let moved_to = |f: &Family, root: i32| -> Family {
            let by = (root - f.root).rem_euclid(12);
            let scale: Vec<i32> = f.scale.iter().map(|p| (p + by).rem_euclid(12)).collect();
            Family {
                name: f.name.clone(),
                root: root.rem_euclid(12),
                scale_text: scale.iter().map(|&p| theory::note_name(p, !flats).to_string()).collect::<Vec<_>>().join(" "),
                scale,
                sharps: !flats,
                ..Default::default()
            }
        };
        let voicings = |f: &Family, root: i32| -> Vec<Voicing> {
            let by = (root - f.root).rem_euclid(12);
            f.order()
                .iter()
                .map(|&at| {
                    let pcs: Vec<i32> = f.voicing(at).pcs.iter().map(|p| (p + by).rem_euclid(12)).collect();
                    let spelled = pcs.iter().map(|&p| theory::note_name(p, !flats).to_string()).collect();
                    Voicing { pcs, spelled, ..Default::default() }
                })
                .collect()
        };
        let roots = [key + 2, key + 7, key];
        let fams = [moved_to(ii, roots[0]), moved_to(v, roots[1]), moved_to(one, roots[2])];
        let pools = [voicings(ii, roots[0]), voicings(v, roots[1]), voicings(one, roots[2])];
        // the ii: the start-th grip, fretted (no open strings, so the whole cadence can move) around the 7th fret
        let first = pools[0].get(start.min(pools[0].len().saturating_sub(1)))?.clone();
        let home: [Option<u8>; 6] = std::array::from_fn(|s| self.strings.contains(&s).then_some(7));
        let all = all_grips(&first.pcs, &self.strings);
        let fretted: Vec<_> = all.iter().filter(|g| g.iter().flatten().all(|f| *f > 0)).copied().collect();
        let g0 = if fretted.is_empty() { all } else { fretted }.into_iter().min_by_key(|g| distance(g, &home))?;
        let fam_names = [ii.name.clone(), v.name.clone(), one.name.clone()];
        let mut steps = vec![CadenceStep {
            name: first.symbol(&fams[0]).replace("mMaj7", "m(maj7)"),
            family: fam_names[0].clone(),
            degrees: first.degrees(&fams[0]),
            spelled: first.spelled.clone(),
            grip: Some(g0),
            ..Default::default()
        }];
        let (mut prev, mut prev_g) = (first, g0);
        for k in 1..3 {
            // every grip of the family there, placed nearest the chord before, smoothest first
            let mut cands: Vec<(i32, Voicing, [Option<u8>; 6])> = Vec::new();
            for vc in &pools[k] {
                if cands.iter().any(|c| c.1.pcs == vc.pcs) {
                    continue;
                }
                if let Some(g) = all_grips(&vc.pcs, &self.strings).into_iter().min_by_key(|g| lead_cost(&prev_g, g, &self.strings)) {
                    cands.push((lead_cost(&prev_g, &g, &self.strings), vc.clone(), g));
                }
            }
            cands.sort_by_key(|c| c.0);
            let choice = pick[k - 1].min(cands.len().saturating_sub(1));
            let (_, vc, g) = cands.get(choice)?.clone();
            let mut choices: Vec<String> = cands.iter().take(4).map(|c| c.1.symbol(&fams[k]).replace("mMaj7", "m(maj7)")).collect();
            choices.dedup();
            steps.push(CadenceStep {
                name: vc.symbol(&fams[k]).replace("mMaj7", "m(maj7)"),
                family: fam_names[k].clone(),
                degrees: vc.degrees(&fams[k]),
                spelled: vc.spelled.clone(),
                grip: Some(g),
                moved: (0..vc.pcs.len()).filter(|&i| grip_pitches_on(&prev_g, &self.strings)[i] != grip_pitches_on(&g, &self.strings)[i]).collect(),
                how: Family::move_words(&prev, &vc, Some(&prev_g), Some(&g), &self.strings),
                choices,
            });
            prev = vc;
            prev_g = g;
        }
        Some(Cadence { steps })
    }
}

/// The book from Backline's folder: None when there's no file (THE CHORD BOOK is then not shown).
pub fn load() -> Option<(Book, Vec<(usize, String)>)> {
    let text = std::fs::read_to_string(path()).ok()?;
    Some(parse(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "@set 1 2 3 4 | D G B E\n@family Major 7th | G A B C# D E F#\n@video abc\n@about One.\n@about Two.\n@group Stations\nG D F# B | Root position.\nB F# G D | ? 1st inversion.\nX Y | bad\n@family Altered | G Ab Bb B Db Eb F\nF B Eb Ab | Fm7b5.\n";

    #[test]
    fn a_book_reads() {
        let (b, errs) = parse(SAMPLE);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!(b.sets[0].strings, vec![2, 3, 4, 5]);
        let f = &b.sets[0].families[0];
        assert_eq!((f.root, f.sharps, f.about.as_str(), f.video.as_str()), (7, true, "One. Two.", "abc"));
        let v = &f.groups[0].voicings;
        assert_eq!(v.len(), 2);
        assert!(v[1].unsure && v[1].note == "1st inversion.");
        assert_eq!(f.kind(), Kind::Major);
        assert_eq!(b.sets[0].families[1].kind(), Kind::Dominant);
    }

    #[test]
    fn a_voicing_gets_its_tab_degrees_and_names() {
        let (b, _) = parse(SAMPLE);
        let set = &b.sets[0];
        let f = &set.families[0];
        let root_pos = &f.groups[0].voicings[0];
        // G D F# B on the top four strings: D string 5th fret, G 7, B 7, E 7
        assert_eq!(root_pos.grip(&set.strings), Some([None, None, Some(5), Some(7), Some(7), Some(7)]));
        assert_eq!(root_pos.degrees(f), vec!["R", "5", "7", "3"]);
        assert_eq!(root_pos.symbol(f), "Gmaj7");
        let first = &f.groups[0].voicings[1];
        assert_eq!(first.symbol(f), "Gmaj7/B");
        let alt = &set.families[1];
        let fm = &alt.groups[0].voicings[0];
        assert_eq!(fm.degrees(alt), vec!["b7", "3", "#5", "b9"]);
        assert!(fm.symbol(alt).ends_with("(no root)"), "{}", fm.symbol(alt));
        assert!(fm.also(alt, 3).iter().any(|n| n.starts_with("Fm7b5")), "{:?}", fm.also(alt, 3));
        assert!(fm.outside(alt).is_empty());
    }

    #[test]
    fn own_roots_name_each_chord_on_its_own_root() {
        let text = "@set 1 2 3 4 | D G B E\n@family Triads in C | C D E F G A B | own roots\nE G C E | {C/E} doubled 3rd.\nD F A D | Dm.\n@family Major 7th | G A B C# D E F#\nB F# G D | Gmaj7/B.\n";
        let (b, errs) = parse(text);
        assert!(errs.is_empty(), "{errs:?}");
        let set = &b.sets[0];
        let (tri, maj) = (&set.families[0], &set.families[1]);
        assert!(tri.own_roots && !maj.own_roots);
        assert_eq!(tri.scale.len(), 7, "the flag isn't part of the scale");
        let v = |fam: &Family, i: usize| fam.groups.iter().flat_map(|g| g.voicings.iter()).nth(i).cloned().unwrap();
        let c_e = v(tri, 0);
        assert_eq!(c_e.symbol(tri), "C/E");
        assert_eq!(c_e.degrees(tri), vec!["3", "5", "R", "3"]);
        assert!(c_e.grip(&set.strings).is_some(), "a doubled note still fits");
        let dm = v(tri, 1);
        assert_eq!(dm.symbol(tri), "Dm");
        assert_eq!(dm.degrees(tri), vec!["R", "b3", "5", "R"]);
        // an ordinary family still names over its root
        assert_eq!(v(maj, 0).symbol(maj), "Gmaj7/B");
        // no stations group: the jumps are where his groups start
        let w = tri.walk(&set.strings, Order::Lesson);
        assert!(!w.inversions);
        assert_eq!(w.stations, vec![0]);
    }

    #[test]
    fn the_rules_turn_a_voicing_into_its_relatives() {
        let text = "@set 1 2 3 4 | D G B E\n@family Major 7th | G A B C# D E F#\n@group The four stations\nG D F# B | Root.\nB F# G D | 1st.\n";
        let (b, _) = parse(text);
        let set = &b.sets[0];
        let fam = &set.families[0];
        let w = fam.walk(&set.strings, Order::Lesson);
        let r = fam.rules(&w, 0, &set.strings);
        let find = |name: &str| r.iter().find(|x| x.name == name).unwrap_or_else(|| panic!("{name} in {:?}", r.iter().map(|x| &x.name).collect::<Vec<_>>()));
        // the 3rd down: major to minor; the 7th down: dominant
        let m = find("Gm(maj7)");
        assert!(m.kind == "Core rule" && m.how.contains("lower the 3rd") && m.moved == vec![3] && !m.in_scale, "{m:?}");
        // (spelled and named the way it moved: B down to Bb, the b3)
        assert_eq!((m.spelled[3].as_str(), m.degrees[3].as_str(), m.over.as_str()), ("Bb", "b3", "Gm(maj7)"));
        assert!(find("G7").how.contains("major 7th to b7"));
        // his rule of the fifth: the same shape on D, still in G Lydian
        let d = find("Dmaj7");
        assert_eq!(d.kind, "Rule of the fifth");
        assert!(d.in_scale && d.grip.is_some());
        // planing up the scale: A B G# ... every voice to the next scale note
        assert!(r.iter().any(|x| x.kind == "Planing"));
        // the next inversion is his next station (B F# G D), and he plays it
        let inv = r.iter().find(|x| x.kind == "Next inversion" && x.pcs == vec![11, 6, 7, 2]).expect("next inversion");
        assert_eq!(inv.his, Some(1));
        // string transference: the top B moves to the low E string, onto the 2 3 4 6 strings
        let t = r.iter().find(|x| x.kind == "String transference").expect("transference");
        assert!(t.how.contains("2 3 4 6") && t.strings == vec![0, 2, 3, 4] && t.pcs[0] == 11 && t.grip.is_some(), "{t:?}");
        // octave transference: the tenor D down an octave to the A string, the bass: the 1 2 4 5 strings
        let o = r.iter().find(|x| x.kind == "Octave transference" && x.pcs == vec![2, 7, 6, 11]).expect("octave transference");
        assert!(o.how.contains("1 2 4 5") && o.strings == vec![1, 2, 4, 5] && o.grip.is_some(), "{o:?}");
        // every new bass names a chord on itself (or is the family's root) and fits five strings
        for b in r.iter().filter(|x| x.kind == "Add a bass") {
            assert!(b.strings.len() == 5 && b.grip.is_some(), "{b:?}");
        }
    }

    #[test]
    fn a_two_five_one_moves_the_fewest_voices() {
        let text = "@set 1 2 3 4 | D G B E
@family Major 7th | G A B C# D E F#
@group The four stations
G D F# B | 1.
B F# G D | 2.
D G B F# | 3.
F# B D G | 4.
@family Minor 7th | G A Bb C D E F
@group The four stations
G D F Bb | 1.
Bb F G D | 2.
D G Bb F | 3.
F Bb D G | 4.
@family Dominant 7th | G A B C D E F
@group The four stations
G D F B | 1.
B F G D | 2.
D G B F | 3.
F B D G | 4.
";
        let (b, errs) = parse(text);
        assert!(errs.is_empty(), "{errs:?}");
        let c = b.sets[0].cadence(0, false, 0, [0, 0]).expect("a ii-V-I in C");
        let names: Vec<&str> = c.steps.iter().map(|s| s.name.as_str()).collect();
        assert!(names[0].starts_with("Dm7") && names[1].starts_with("G7") && names[2].starts_with("Cmaj7"), "{names:?}");
        // the smoothest V and I keep common tones: not every voice moves
        assert!(c.steps[1].moved.len() < 4 && c.steps[2].moved.len() < 4, "{:?}", c.steps.iter().map(|s| (&s.name, &s.moved, &s.how)).collect::<Vec<_>>());
        assert!(c.steps.iter().all(|s| s.grip.is_some()));
        // ii-V-i: the minor family again for the i
        let m = b.sets[0].cadence(0, true, 0, [0, 0]).unwrap();
        assert!(m.steps[2].name.starts_with("Cm"), "{}", m.steps[2].name);
    }

    #[test]
    fn a_note_says_where_a_chord_resolves() {
        let t = |root: i32, minor: bool| Target { root, minor };
        assert_eq!(stated_targets("Resolves to F or Fm, to B.", 4).0, vec![t(5, false), t(5, true), t(11, false)]);
        assert_eq!(stated_targets("It resolves to D major or minor.", 4).0, vec![t(2, false), t(2, true)]);
        assert_eq!(stated_targets("Resolves to Eb, Gb, A or C, major or minor.", 4).0.len(), 8);
        // a dominant named on the way isn't a landing; "C minor" is
        assert_eq!(stated_targets("Resolve by a half step (the Db7 sound to C minor) or up a 4th (F7#5 to Bb).", 4).0, vec![t(0, true), t(10, false)]);
        assert_eq!(stated_targets("Resolves to Bbm or F#m9.", 4).0, vec![t(10, true), t(6, true)]);
        // a voicing written out, bare or in brackets after its place
        let (ts, exact) = stated_targets("Top voice to D: resolves to C F# G E.", 4);
        assert!(ts.is_empty());
        assert_eq!(exact, vec![(None, vec![0, 6, 7, 4])]);
        let (ts, given) = stated_targets("Resolves to F (E A C F) or to B (D# A B F#).", 4);
        assert_eq!(ts, vec![t(5, false), t(11, false)]);
        assert_eq!(given, vec![(Some(t(5, false)), vec![4, 9, 0, 5]), (Some(t(11, false)), vec![3, 9, 11, 6])]);
        let (ts, given) = stated_targets("Resolves to Cm (Eb G Bb D).", 4);
        assert_eq!((ts, given), (vec![t(0, true)], vec![(Some(t(0, true)), vec![3, 7, 10, 2])]));
        assert!(stated_targets("Resolves nicely.", 4).0.is_empty());
    }

    #[test]
    fn passing_chords_lead_into_a_grip() {
        let text = "@set 1 2 3 4 | D G B E
@family Minor 7th | G A Bb C D E F
@group The four stations
G D F Bb | 1.
";
        let (b, errs) = parse(text);
        assert!(errs.is_empty(), "{errs:?}");
        let set = &b.sets[0];
        let fam = &set.families[0];
        let walk = fam.walk(&set.strings, Order::Lesson);
        let ps = fam.passing(set, &walk, 0);
        let names: Vec<(&str, &str)> = ps.iter().map(|r| (r.over.as_str(), r.name.as_str())).collect();
        let dst = grip_pitches_on(&walk.steps[0].grip.unwrap(), &set.strings);
        // the same grip a half step above and below
        assert_eq!(ps[0].over, "Half step above", "{names:?}");
        assert!(ps[0].name.starts_with("Abm"), "{names:?}");
        let pu = grip_pitches_on(&ps[0].grip.unwrap(), &set.strings);
        assert!(pu.iter().zip(&dst).all(|(x, y)| x.unwrap() - y.unwrap() == 1));
        assert_eq!(ps[1].over, "Half step below", "{names:?}");
        assert!(ps[1].name.starts_with("Gbm") || ps[1].name.starts_with("F#m"), "{names:?}");
        // its V7 (D7): the 3rd and 7th of D (F# and C), each voice a few half steps at most from where it goes
        let v = ps.iter().find(|r| r.over == "V7").expect("a V7");
        assert!(v.name.starts_with('D') && v.pcs.contains(&6) && v.pcs.contains(&0), "{names:?} {:?}", v.spelled);
        let pv = grip_pitches_on(&v.grip.unwrap(), &set.strings);
        assert!(pv.iter().zip(&dst).all(|(x, y)| (x.unwrap() - y.unwrap()).abs() <= 3), "{pv:?} {dst:?}");
        assert!(v.how.contains("Into this chord"), "{}", v.how);
        // the altered one has a b9, #9 or b13 (Eb, F or Bb), spelled as the book spells them
        let alt = ps.iter().find(|r| r.over == "V7 altered").expect("an altered V7");
        assert!(alt.pcs.iter().any(|p| [3, 5, 10].contains(p)), "{:?}", alt.spelled);
        assert!(!alt.spelled.iter().any(|s| s == "D#" || s == "A#"), "{:?}", alt.spelled);
        // (its notes are the tritone sub's too: said there, and the tritone sub button is another voicing)
        assert!(alt.how.contains("the tritone sub"), "{}", alt.how);
        // the tritone sub, Ab7
        let tt = ps.iter().find(|r| r.over == "Tritone sub").expect("a tritone sub");
        assert!(tt.name.starts_with("Ab"), "{}", tt.name);
        // F#dim7, all four of its notes
        let dim = ps.iter().find(|r| r.over == "Dim below").expect("a diminished below");
        let mut d = dim.pcs.clone();
        d.sort();
        assert_eq!(d, vec![0, 3, 6, 9]);
        assert_eq!(dim.name, "F#dim7");
        let on = ps.iter().find(|r| r.over == "Dim on the root").expect("a diminished on the root");
        assert_eq!(on.name, "Gdim7");
        assert!(on.how.contains("G stays"), "{}", on.how);
    }

    #[test]
    fn a_dominant_resolves_by_voice_leading() {
        let text = "@set 1 2 3 4 | D G B E
@family Major 7th | C D E F# G A B
@group The four stations
C G B E | 1.
@family Dominant 7th | G A B C D E F
@group The four stations
G D F B | 1.
B F G D | 2. Resolves to C (C E G D).
@family Diminished (Dominant) | G Ab Bb B Db D E F
E Bb Db G | A diminished grip.
";
        let (b, errs) = parse(text);
        assert!(errs.is_empty(), "{errs:?}");
        let set = &b.sets[0];
        let fam = &set.families[1];
        let walk = fam.walk(&set.strings, Order::Lesson);
        let res = fam.resolutions(set, &walk, 0);
        let names: Vec<&str> = res.iter().map(|r| r.name.as_str()).collect();
        // down a 5th (C, Cm) and down a half step (Gb, Gbm)
        assert_eq!(res.len(), 4, "{names:?}");
        assert!(names[0].starts_with("Cmaj"), "{names:?}");
        assert!(names[1].starts_with("Cm") && !names[1].starts_with("Cmaj") && names[2].starts_with("Gb") && names[3].starts_with("Gbm"), "{names:?}");
        // G D F B to C: the 7th (F) falls to the 3rd (E), the 3rd (B) holds as the 7th; small moves only
        let c = &res[0];
        assert!(c.spelled.contains(&"E".to_string()), "{:?} {}", c.spelled, c.how);
        assert!(c.how.contains("F to E"), "{}", c.how);
        assert!(c.how.starts_with("Down a 5th."), "{}", c.how);
        // a landing the lesson writes out comes first, as written, and its place isn't worked out again
        let res2 = fam.resolutions(set, &walk, 1);
        assert!(res2[0].given && res2[0].stated, "{:?}", res2.iter().map(|r| (&r.name, r.given)).collect::<Vec<_>>());
        assert_eq!(res2[0].spelled, vec!["C", "E", "G", "D"]);
        assert_eq!(res2.iter().filter(|r| r.name.starts_with('C') && !r.name.starts_with("Cm")).count(), 1);
        let (pa, pb) = (grip_pitches_on(&walk.steps[0].grip.unwrap(), &set.strings), grip_pitches_on(&c.grip.unwrap(), &set.strings));
        assert!(pa.iter().zip(&pb).all(|(x, y)| (x.unwrap() - y.unwrap()).abs() <= 3), "{pa:?} {pb:?}");
        assert!(res[2].how.contains("tritone sub"), "{}", res[2].how);
        // a diminished grip lands on four homes a minor 3rd apart, major and minor
        let dim = &set.families[2];
        let dw = dim.walk(&set.strings, Order::Lesson);
        let roots: Vec<i32> = dim.resolutions(set, &dw, 0).iter().filter_map(|r| {
            let v = Voicing { pcs: r.pcs.clone(), ..Default::default() };
            v.own_chord().map(|s| s.root)
        }).collect();
        assert_eq!(dim.resolutions(set, &dw, 0).len(), 8, "{roots:?}");
        // a major family's chord goes nowhere unless its note says so
        let maj = &set.families[0];
        assert!(maj.resolutions(set, &maj.walk(&set.strings, Order::Lesson), 0).is_empty());
    }

    #[test]
    fn a_fifth_note_under_an_upper_structure() {
        // Dmaj7 (D F# A C#) in G Lydian is Gmaj9#11 without its root: put G in the bass on the low E string
        let text = "@set 1 2 3 4 | D G B E\n@family Major 7th | G A B C# D E F#\nD F# A C# | Dmaj7 over G.\n";
        let (b, _) = parse(text);
        let set = &b.sets[0];
        let fam = &set.families[0];
        let w = fam.walk(&set.strings, Order::Lesson);
        let r = fam.rules(&w, 0, &set.strings);
        let g = r.iter().find(|x| x.kind == "Add a bass" && x.pcs[0] == 7).unwrap_or_else(|| panic!("{:?}", r.iter().map(|x| (x.kind, &x.name)).collect::<Vec<_>>()));
        assert!(g.strings == vec![0, 2, 3, 4, 5] && g.grip.is_some() && g.how.contains("G in the bass"), "{g:?}");
    }

    #[test]
    fn his_chain_walks_through_melodic_minor() {
        // C7#11 is G melodic minor: Em7b5, raise its root, and it's Gm(maj7)
        let text = "@set 1 2 3 4 | D G B E\n@family Dominant 7th #11 | C D E F# G A Bb\nE G Bb D | Em7b5.\n";
        let (b, _) = parse(text);
        let set = &b.sets[0];
        let fam = &set.families[0];
        let w = fam.walk(&set.strings, Order::Lesson);
        let r = fam.rules(&w, 0, &set.strings);
        let step = r.iter().find(|x| x.kind == "The chain").unwrap_or_else(|| panic!("{:?}", r.iter().map(|x| (x.kind, &x.name)).collect::<Vec<_>>()));
        assert!(step.how.contains("raise the root") && step.name.starts_with("Gm(maj7)"), "{step:?}");
        // and a #11 grip a tritone away
        assert!(r.iter().any(|x| x.kind == "Tritone"));
    }

    #[test]
    fn a_walk_says_which_voice_moved() {
        let text = "@set 1 2 3 4 | D G B E
@family Major 7th | G A B C# D E F#
@group The four stations
G D F# B | Root.
B F# G D | 1st.
D G B F# | 2nd.
F# B D G | 3rd.
@group Moves
G D F# C# | {Gmaj7#11} Top voice up.
F# D G B | Exchange.
";
        let (b, _) = parse(text);
        let set = &b.sets[0];
        let f = &set.families[0];
        let w = f.walk(&set.strings, Order::Lesson);
        assert_eq!(w.stations, vec![0, 1, 2, 3]);
        let up = &w.steps[4];
        assert_eq!(up.from, Some(0));
        assert_eq!(up.moved, vec![3]);
        assert_eq!(up.how, "Top voice B to C# (up a whole step).");
        // near where it came from: the same fret area as root position
        assert_eq!(up.grip, Some([None, None, Some(5), Some(7), Some(7), Some(9)]));
        assert_eq!(f.voicing(up.at).names, vec!["Gmaj7#11".to_string()]);
        assert_eq!(f.voicing(up.at).note, "Top voice up.");
        let ex = &w.steps[5];
        assert!(ex.how.starts_with("Voice exchange: the bass and alto trade notes (G and F#)"), "{}", ex.how);
        let n = f.neighbours(&w, 0, &set.strings);
        assert!(n.iter().any(|(j, _)| *j == 4) && n.iter().any(|(j, _)| *j == 5), "{n:?}");
        // (nothing here is built from a station but the first: his order as it is, either way)
        assert!(!w.paths && !f.has_station_paths());
        assert_eq!(f.walk_order(Order::ByStation), f.order());
    }

    #[test]
    fn each_station_heads_the_chords_built_from_it() {
        let text = "@set 1 2 3 4 | D G B E
@family Major 7th | G A B C# D E F#
@group The four stations
G D F# B | Root.
B F# G D | 1st.
D G B F# | 2nd.
F# B D G | 3rd.
@group From root position
G D F# C# | Top up.
@group A detour
G E F# B | Tenor down.
@group From the 1st inversion (B in the bass)
B F# G E | Top down.
B F# A D | Alto down.
@group D in the bass
D G B E | Top up.
@group A in the bass (the 9)
A D F# B | New bass.
";
        let (b, _) = parse(text);
        let set = &b.sets[0];
        let f = &set.families[0];
        assert!(f.has_station_paths());
        let w = f.walk(&set.strings, Order::ByStation);
        assert!(w.paths && w.inversions);
        // station 1, its two; station 2 (by its inversion and its bass), its two; station 3 (its bass), its one and
        // the group on a bass no station has; station 4 has nothing built from it: after station 3's path
        let order: Vec<(usize, usize)> = w.steps.iter().map(|s| s.at).collect();
        assert_eq!(order, vec![(0, 0), (1, 0), (2, 0), (0, 1), (3, 0), (3, 1), (0, 2), (4, 0), (5, 0), (0, 3)]);
        assert_eq!(w.stations, vec![0, 3, 6, 9]);
        assert_eq!(w.stops, vec![0..1, 3..4, 6..7, 9..10]);
        assert_eq!(w.station_at(3), Some(1));
        // a station's move is from the station before it; a chord's from its own station's path
        assert_eq!(w.steps[3].from, Some(0));
        assert!(w.steps[3].how.starts_with("Every voice moves up"), "{}", w.steps[3].how);
        assert_eq!(w.steps[4].from, Some(3));
        // his order: the four stations first, then each again where the walk turns to it (station 4 never: its
        // button stays on the opening one)
        let l = f.walk(&set.strings, Order::Lesson);
        assert!(l.paths);
        let order: Vec<(usize, usize)> = l.steps.iter().map(|s| s.at).collect();
        assert_eq!(
            order,
            vec![(0, 0), (0, 1), (0, 2), (0, 3), (0, 0), (1, 0), (2, 0), (0, 1), (3, 0), (3, 1), (0, 2), (4, 0), (5, 0)]
        );
        assert_eq!(l.stations, vec![4, 7, 10, 3]);
        assert_eq!(l.stops, vec![0..1, 1..2, 2..3, 3..4, 4..5, 7..8, 10..11]);
        // played again: the same grip and the same move, from the latest time the chord it came from was played
        assert_eq!(l.steps[7].grip, l.steps[1].grip);
        assert_eq!(l.steps[7].from, Some(4));
        assert_eq!(l.steps[7].how, l.steps[1].how);
        assert_eq!(l.steps[4].from, None);
        assert_eq!(l.steps[5].from, Some(4));
        assert_eq!(l.steps[8].from, Some(7));
        // on station 2's path, from that path, even when a chord on station 1's path is closer (kept as `also`)
        let text2 = text.replace("B F# A D | Alto down.", "B F# A D | Alto down.
B E F# C# | Closer to a chord on station 1's path than to its own.");
        let (b2, _) = parse(&text2);
        let f2 = &b2.sets[0].families[0];
        let w2 = f2.walk(&b2.sets[0].strings, Order::ByStation);
        let k = w2.steps.iter().position(|s| s.at == (3, 2)).unwrap();
        let from = w2.steps[k].from.unwrap();
        assert!(from >= 3 && from < k, "from {from}: {:?}", w2.steps.iter().map(|s| s.at).collect::<Vec<_>>());
        let also = w2.steps[k].also.expect("a closer chord off the path");
        assert!(also < 3, "{also}");
        assert!(!w2.steps[k].also_how.is_empty());
        assert_eq!(super::station_anchor("Ab: the octave", &[8, 10], &[8]), super::Anchor::Station(0));
        assert_eq!(super::station_anchor("Bbm: down", &[8, 10], &[8]), super::Anchor::Station(1));
        assert_eq!(super::station_anchor("Em7b5 and Gm7", &[4], &[4]), super::Anchor::Unsaid);
        assert_eq!(super::station_anchor("C in the bass (the #11)", &[7, 11], &[7, 11]), super::Anchor::Elsewhere);
    }

    /// Writes what THE CHORD BOOK shows for each voicing in BACKLINE_CHORDBOOK (a chord_book.txt) to
    /// BACKLINE_CHORDBOOK_OUT as JSON, for checking a write-up before it goes into Backline's folder.
    #[test]
    #[ignore]
    fn chord_book_dump() {
        let src = std::env::var("BACKLINE_CHORDBOOK").expect("BACKLINE_CHORDBOOK");
        let out = std::env::var("BACKLINE_CHORDBOOK_OUT").expect("BACKLINE_CHORDBOOK_OUT");
        let (b, errs) = parse(&std::fs::read_to_string(src).unwrap());
        let mut sets = Vec::new();
        for s in &b.sets {
            let mut fams = Vec::new();
            for f in &s.families {
                let walk = f.walk(&s.strings, Order::Lesson);
                let mut groups = Vec::new();
                for (gi, g) in f.groups.iter().enumerate() {
                    let vs: Vec<serde_json::Value> = g
                        .voicings
                        .iter()
                        .enumerate()
                        .map(|(vi, v)| {
                            let k = walk.steps.iter().position(|st| st.at == (gi, vi));
                            let st = k.map(|k| &walk.steps[k]);
                            // where it resolves (the lesson's first, its written landings marked given)
                            let res: Vec<serde_json::Value> = k
                                .map(|k| f.resolutions(s, &walk, k))
                                .unwrap_or_default()
                                .iter()
                                .map(|r| serde_json::json!({"name": r.name, "stated": r.stated, "given": r.given, "book": r.from_book, "notes": r.spelled, "how": r.how}))
                                .collect();
                            // the passing chords into it
                            let pass: Vec<serde_json::Value> = k
                                .map(|k| f.passing(s, &walk, k))
                                .unwrap_or_default()
                                .iter()
                                .map(|r| serde_json::json!({"over": r.over, "name": r.name, "notes": r.spelled, "degrees": r.degrees, "how": r.how}))
                                .collect();
                            serde_json::json!({
                                "pass": pass,
                                "notes": v.spelled, "note": v.note, "names": v.names, "unsure": v.unsure, "line": v.line,
                                "grip": st.and_then(|st| st.grip), "degrees": v.degrees(f), "symbol": v.symbol(f),
                                "also": v.also(f, 3), "outside": v.outside(f), "how": st.map(|st| st.how.clone()),
                                "res": res,
                            })
                        })
                        .collect();
                    groups.push(serde_json::json!({"title": g.title, "voicings": vs}));
                }
                fams.push(serde_json::json!({"name": f.name, "scale": f.scale_text, "video": f.video, "about": f.about, "groups": groups}));
            }
            sets.push(serde_json::json!({"name": s.name, "label": s.label, "strings": s.strings, "families": fams}));
        }
        let errs: Vec<String> = errs.iter().map(|(n, e)| format!("line {n}: {e}")).collect();
        std::fs::write(out, serde_json::to_string_pretty(&serde_json::json!({"sets": sets, "errors": errs})).unwrap()).unwrap();
    }
}
