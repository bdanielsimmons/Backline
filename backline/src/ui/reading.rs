//! READING (under PRACTICE): sight reading, two ways.
//!
//! THE BOOK: your own copy of a guitar method, worked through lesson by lesson in the book's order - its exercises,
//! solos and duets bar for bar, one at a time, each marked done when you have it, so you pick up where you left off.
//! The book is read from a file kept on this computer only (reading_book.txt in Backline's folder; it is never part
//! of Backline itself). In a duet you read your part while the other guitar plays.
//!
//! MADE-UP LINES: for everyone, with or without a book - lines made up fresh in a key and a position, the rhythms
//! getting harder step by step, the way the classic reading courses work: read in one position, don't look at the
//! neck, keep going.
//!
//! Either way the music is written out properly (key and time signatures, rests, beams, dots, ties, triplets,
//! chords, sharps, flats and naturals), the tab hidden until you want to check, and it plays with a count-in and a
//! cursor - or just clicks while you play it.

use super::*;
use crate::theory;

/// Keys in the order the courses bring them in: (name, tonic, sharps (+) or flats (-) in the signature).
const KEYS: [(&str, i32, i32); 10] = [
    ("C MAJOR", 0, 0),
    ("G MAJOR", 7, 1),
    ("F MAJOR", 5, -1),
    ("D MAJOR", 2, 2),
    ("Bb MAJOR", 10, -2),
    ("A MAJOR", 9, 3),
    ("Eb MAJOR", 3, -3),
    ("E MAJOR", 4, 4),
    ("Ab MAJOR", 8, -4),
    ("Db MAJOR", 1, -5),
];
/// Positions: where the first finger sits (0 = the 1st / open position, open strings included).
const POSITIONS: [(&str, u8); 9] = [
    ("1ST POSITION", 0),
    ("2ND POSITION", 2),
    ("3RD POSITION", 3),
    ("4TH POSITION", 4),
    ("5TH POSITION", 5),
    ("7TH POSITION", 7),
    ("8TH POSITION", 8),
    ("9TH POSITION", 9),
    ("12TH POSITION", 12),
];
const LEVELS: [&str; 6] = [
    "1 - WHOLES, HALVES, QUARTERS",
    "2 - + EIGHTHS",
    "3 - + DOTTED NOTES",
    "4 - + SYNCOPATION",
    "5 - + TRIPLETS",
    "6 - + SIXTEENTHS",
];
const BARS: [usize; 3] = [4, 8, 16];
/// a beat is 12 units, so eighths (6), triplets (4), quarter triplets (8) and sixteenths (3) are whole numbers
const BEAT: u16 = 12;

/// One note head: the pitch it sounds, where it sits on the staff (guitar is written an octave up; the bottom line
/// E is 30) and the sharp (1), flat (-1) or natural (0) it is written with.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Head {
    pub midi: i32,
    pub step: i32,
    pub acc: i32,
}

/// A note, a chord (several heads) or a rest (none), its length in twelfths of a beat, and whether it is tied on.
#[derive(Clone, Debug)]
pub(crate) struct RNote {
    pub heads: Vec<Head>,
    pub dur: u16,
    pub tie: bool,
    pub marks: Marks,
}

/// What is printed with a note besides the note: pick stroke, fingers, string, accents, slurs, and words over it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Marks {
    /// 1 a down-stroke, -1 an up-stroke
    pub pick: i8,
    /// a finger for each head, lowest first ('0' open, 'x' none)
    pub fingers: String,
    /// a circled string number
    pub string: u8,
    pub accent: bool,
    pub stac: bool,
    pub tenuto: bool,
    pub fermata: bool,
    pub slur_start: bool,
    pub slur_end: bool,
    /// words written over it (Ritard, a tempo, mf, pos V ...)
    pub text: Vec<String>,
}

/// The marks in braces after a note: {d,f21,s2,>,ferm,stac,(,)}.
fn parse_marks(s: &str) -> Option<Marks> {
    let mut m = Marks::default();
    for it in s.split(',').map(str::trim).filter(|x| !x.is_empty()) {
        match it {
            "d" => m.pick = 1,
            "u" => m.pick = -1,
            ">" => m.accent = true,
            "stac" => m.stac = true,
            "ten" => m.tenuto = true,
            "ferm" => m.fermata = true,
            "(" => m.slur_start = true,
            ")" => m.slur_end = true,
            _ if it.starts_with('f') && it.len() > 1 && it[1..].chars().all(|c| matches!(c, '0'..='4' | 'x')) => {
                m.fingers = it[1..].to_string()
            }
            _ if it.starts_with('s') => m.string = it[1..].parse().ok().filter(|n| (1..=6).contains(n))?,
            _ => return None,
        }
    }
    Some(m)
}

/// One bar, and the repeat signs at its start and end.
#[derive(Clone, Debug, Default)]
pub(crate) struct Bar {
    pub notes: Vec<RNote>,
    pub rep_start: bool,
    pub rep_end: bool,
}

/// A line of music to read, and how it is written and fingered.
#[derive(Clone, Debug)]
pub(crate) struct Reading {
    pub bars: Vec<Bar>,
    pub beats: u16,
    /// the note that gets a beat (4 a quarter, 2 a half in cut time, 8 an eighth)
    pub den: u16,
    /// sharps (+) or flats (-) in the key signature
    pub sig: i32,
    /// the position it is read in (for the tab): where the first finger sits, 0 the 1st position
    pub pos: u8,
}

impl Reading {
    /// How long a beat is, in note units (a quarter is BEAT).
    pub(crate) fn beat_len(&self) -> u16 {
        BEAT * 4 / self.den.max(1)
    }
}

/// An exercise, solo or duet: its name and its parts (one, or two for a duet).
#[derive(Clone, Debug)]
pub(crate) struct Piece {
    pub name: String,
    pub parts: Vec<(String, Reading)>,
}

/// A lesson of the book: its page, its title, a few words on what it teaches and how, and its pieces (none for a
/// page that is worked from the book itself, like chord forms).
#[derive(Clone, Debug, Default)]
pub(crate) struct Lesson {
    pub page: u32,
    pub title: String,
    pub notes: Vec<String>,
    pub pieces: Vec<Piece>,
    pub tempo: u32,
    /// the chapter it belongs to (a `@chapter` line before it), empty when the book has none
    pub chapter: String,
}

// ------------------------------------------------------------------ writing notes down

fn head_of(m: i32, sharps: bool) -> Head {
    let (letter, acc, oct) = crate::lines::spell(m + 12, sharps);
    Head { midi: m, step: oct * 7 + letter, acc }
}

/// A written pitch like C4, F#4, Bb3, En5 (as it sits on the treble staff): its head.
fn parse_pitch(s: &str) -> Option<Head> {
    let mut ch = s.chars();
    let letter = "CDEFGAB".find(ch.next()?.to_ascii_uppercase())? as i32;
    let rest: String = ch.collect();
    let (acc, oct) = match rest.chars().next()? {
        _ if rest.starts_with("bb") => (-2, &rest[2..]),
        _ if rest.starts_with("##") => (2, &rest[2..]),
        'x' => (2, &rest[1..]),
        '#' => (1, &rest[1..]),
        'b' => (-1, &rest[1..]),
        'n' => (0, &rest[1..]),
        _ => (0, &rest[..]),
    };
    let oct: i32 = oct.parse().ok()?;
    const PC: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
    let written = 12 * (oct + 1) + PC[letter as usize] + acc;
    Some(Head { midi: written - 12, step: oct * 7 + letter, acc })
}

/// One token of written music: C4q, F#4e., [C4E4G4]h, rq, D4h~ (tied on), E4t (a triplet eighth), G4T (a quarter-note
/// triplet), C4s (sixteenth). w h q e s = whole, half, quarter, eighth, sixteenth; a dot after it adds half.
fn parse_note(tok: &str) -> Option<RNote> {
    let (tok, marks) = match tok.strip_suffix('}').and_then(|t| t.split_once('{')) {
        Some((t, m)) => (t, parse_marks(m)?),
        None => (tok, Marks::default()),
    };
    let tie = tok.ends_with('~');
    let tok = tok.trim_end_matches('~');
    let dotted = tok.ends_with('.');
    let tok = tok.trim_end_matches('.');
    let (body, d) = tok.split_at(tok.len().checked_sub(1)?);
    let mut dur: u16 = match d {
        "w" => 48,
        "h" => 24,
        "q" => 12,
        "e" => 6,
        "s" => 3,
        "t" => 4,
        "T" => 8,
        _ => return None,
    };
    if dotted {
        dur = dur * 3 / 2;
    }
    let heads = if body == "r" {
        Vec::new()
    } else if let Some(inner) = body.strip_prefix('[').and_then(|b| b.strip_suffix(']')) {
        // a chord: pitches one after another, each starting with its letter
        let mut out = Vec::new();
        let mut cur = String::new();
        for c in inner.chars() {
            if "ABCDEFG".contains(c) && !cur.is_empty() {
                out.push(parse_pitch(&cur)?);
                cur.clear();
            }
            cur.push(c);
        }
        if !cur.is_empty() {
            out.push(parse_pitch(&cur)?);
        }
        out.sort_by_key(|h| h.step);
        out
    } else {
        vec![parse_pitch(body)?]
    };
    Some(RNote { heads, dur, tie, marks })
}

/// Bars of written music: tokens split by | (a bar line), |: and :| (repeat signs).
fn parse_music(text: &str) -> Result<Vec<Bar>, String> {
    let mut bars: Vec<Bar> = vec![Bar::default()];
    // words (!Ritard, !mf) wait for the next note
    let mut words: Vec<String> = Vec::new();
    for tok in text.split_whitespace() {
        if let Some(w) = tok.strip_prefix('!').filter(|w| !w.is_empty()) {
            words.push(w.replace('_', " "));
            continue;
        }
        match tok {
            "|" | "||" => bars.push(Bar::default()),
            "|:" => {
                if !bars.last().is_some_and(|b| b.notes.is_empty()) {
                    bars.push(Bar::default());
                }
                if let Some(b) = bars.last_mut() {
                    b.rep_start = true;
                }
            }
            ":|" => {
                if let Some(b) = bars.last_mut() {
                    b.rep_end = true;
                }
                bars.push(Bar::default());
            }
            t => {
                let mut n = parse_note(t).ok_or_else(|| format!("can't read '{}'", t))?;
                n.marks.text.append(&mut words);
                bars.last_mut().unwrap().notes.push(n);
            }
        }
    }
    bars.retain(|b| !b.notes.is_empty());
    Ok(bars)
}

/// Sharps (+) or flats (-) for a key name: C, G, F, Bb, Am, Em, Dm ...
fn sig_of(key: &str) -> i32 {
    match key.trim() {
        "G" | "Em" => 1,
        "D" | "Bm" => 2,
        "A" | "F#m" => 3,
        "E" | "C#m" => 4,
        "B" | "G#m" => 5,
        "F" | "Dm" => -1,
        "Bb" | "Gm" => -2,
        "Eb" | "Cm" => -3,
        "Ab" | "Fm" => -4,
        "Db" | "Bbm" => -5,
        "Gb" | "Ebm" => -6,
        _ => 0,
    }
}

/// The book file: lessons in order.
///
/// ```text
/// @lesson 4 Notes in the 1st Position
/// key C | pos 1 | time 4/4 | tempo 72
/// note One beat to each quarter note; say the count out loud.
/// @ex Exercise 1
/// C4q D4q E4q F4q | G4h [C4E4G4]h | ...
/// @duet Sea to Sea
/// part 1st Guitar: ...
/// part 2nd Guitar: ...
/// ```
/// A setting line (key, pos, time) holds until the next one. Lines starting with # are comments.
pub(crate) fn parse_book(text: &str) -> (Vec<Lesson>, Vec<String>) {
    let mut lessons: Vec<Lesson> = Vec::new();
    let mut errs = Vec::new();
    let (mut sig, mut pos, mut beats, mut den) = (0i32, 0u8, 4u16, 4u16);
    let mut chapter = String::new();
    // the piece being read: (name, parts as (part name, music text))
    let mut piece: Option<(String, Vec<(String, String)>)> = None;
    let finish = |piece: &mut Option<(String, Vec<(String, String)>)>,
                  lessons: &mut Vec<Lesson>,
                  errs: &mut Vec<String>,
                  sig,
                  pos,
                  (beats, den)| {
        if let Some((name, parts)) = piece.take() {
            let mut out = Vec::new();
            for (pn, music) in parts {
                match parse_music(&music) {
                    Ok(bars) => out.push((pn, Reading { bars, beats, den, sig, pos })),
                    Err(e) => errs.push(format!("{}: {}", name, e)),
                }
            }
            if let Some(l) = lessons.last_mut() {
                if !out.is_empty() {
                    l.pieces.push(Piece { name, parts: out });
                }
            }
        }
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("@chapter") {
            chapter = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("@lesson") {
            finish(&mut piece, &mut lessons, &mut errs, sig, pos, (beats, den));
            let rest = rest.trim();
            let (page, title) = rest.split_once(' ').unwrap_or((rest, ""));
            lessons.push(Lesson {
                page: page.parse().unwrap_or(0),
                title: title.trim().to_string(),
                tempo: 72,
                chapter: chapter.clone(),
                ..Default::default()
            });
        } else if let Some(rest) = line.strip_prefix("@ex").or_else(|| line.strip_prefix("@duet")) {
            finish(&mut piece, &mut lessons, &mut errs, sig, pos, (beats, den));
            piece = Some((rest.trim().to_string(), vec![("".to_string(), String::new())]));
        } else if let Some(rest) = line.strip_prefix("part ") {
            let (pn, music) = rest.split_once(':').unwrap_or((rest, ""));
            if let Some((_, parts)) = piece.as_mut() {
                if parts.len() == 1 && parts[0].0.is_empty() && parts[0].1.trim().is_empty() {
                    parts.clear();
                }
                parts.push((pn.trim().to_string(), music.to_string()));
            }
        } else if let Some(rest) = line.strip_prefix("note ") {
            if let Some(l) = lessons.last_mut() {
                l.notes.push(rest.trim().to_string());
            }
        } else if line.starts_with("key ") || line.starts_with("pos ") || line.starts_with("time ") || line.starts_with("tempo ")
        {
            for set in line.split('|') {
                let set = set.trim();
                if let Some(k) = set.strip_prefix("key ") {
                    sig = sig_of(k);
                } else if let Some(p) = set.strip_prefix("pos ") {
                    let n: u8 = p.trim().parse().unwrap_or(1);
                    pos = if n <= 1 { 0 } else { n };
                } else if let Some(t) = set.strip_prefix("time ") {
                    let (b, d) = t.trim().split_once('/').unwrap_or((t.trim(), "4"));
                    beats = b.trim().parse().unwrap_or(4);
                    den = match d.trim() {
                        "2" => 2,
                        "8" => 8,
                        _ => 4,
                    };
                } else if let Some(t) = set.strip_prefix("tempo ") {
                    if let Some(l) = lessons.last_mut() {
                        l.tempo = t.trim().parse().unwrap_or(72);
                    }
                }
            }
        } else if let Some((_, parts)) = piece.as_mut() {
            // more music for the part being written
            if let Some(p) = parts.last_mut() {
                p.1.push(' ');
                p.1.push_str(line);
            }
        }
    }
    finish(&mut piece, &mut lessons, &mut errs, sig, pos, (beats, den));
    (lessons, errs)
}

/// Where the book file lives (this computer only).
pub(crate) fn book_path() -> std::path::PathBuf {
    crate::api::config_dir().join("reading_book.txt")
}

// ------------------------------------------------------------------ made-up lines

/// A small random number maker.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn pick_weighted(&mut self, w: &[u32]) -> usize {
        let total: u32 = w.iter().sum();
        let mut r = (self.next() % total.max(1) as u64) as u32;
        for (i, x) in w.iter().enumerate() {
            if r < *x {
                return i;
            }
            r -= x;
        }
        0
    }
}

/// The frets a position covers (a stretch either side allowed).
fn window(pos: u8) -> (u8, u8) {
    if pos == 0 {
        (0, 4)
    } else {
        (pos - 1, pos + 4)
    }
}

/// The notes of a key that can be played in a position, low to high.
fn position_notes(key: usize, pos: u8) -> Vec<i32> {
    let scale = [0, 2, 4, 5, 7, 9, 11];
    let tonic = KEYS[key].1;
    let (lo, hi) = window(pos);
    let mut out: Vec<i32> = Vec::new();
    for s in 0..6 {
        for f in lo..=hi {
            let m = theory::STANDARD[s] + f as i32;
            if scale.contains(&(m - tonic).rem_euclid(12)) && !out.contains(&m) {
                out.push(m);
            }
        }
    }
    out.sort();
    out
}

/// Where the notes of a chord (or a single note) are played in a position: a (string, fret) each, every note on its
/// own string, the first finger's frets first.
fn finger_all(midis: &[i32], pos: u8) -> Vec<Option<(usize, u8)>> {
    let (lo, hi) = window(pos);
    let core = |f: u8| if pos == 0 { f <= 3 } else { f >= pos && f <= pos + 3 };
    let mut used: Vec<usize> = Vec::new();
    let mut out = vec![None; midis.len()];
    // the highest note first, on the highest string it fits
    let mut order: Vec<usize> = (0..midis.len()).collect();
    order.sort_by_key(|i| std::cmp::Reverse(midis[*i]));
    for i in order {
        let mut best: Option<(usize, u8, i32)> = None;
        for s in (0..6).filter(|s| !used.contains(s)) {
            let f = midis[i] - theory::STANDARD[s];
            if f < lo as i32 || f > hi as i32 {
                continue;
            }
            let f = f as u8;
            let cost = if core(f) { 0 } else { 10 } + (f as i32 - pos as i32 - 1).abs();
            if best.is_none_or(|b| cost < b.2) {
                best = Some((s, f, cost));
            }
        }
        if let Some((s, f, _)) = best {
            used.push(s);
            out[i] = Some((s, f));
        }
    }
    out
}

/// The rhythm cells a level allows: groups of (length, rest?) that fill one to four beats, with how often each comes.
fn cells(level: usize) -> Vec<(Vec<(u16, bool)>, u32)> {
    let n = |d: u16| (d, false);
    let r = |d: u16| (d, true);
    let mut c: Vec<(Vec<(u16, bool)>, u32)> = vec![(vec![n(12)], 8), (vec![n(24)], 4), (vec![n(48)], 1), (vec![r(12)], 1)];
    if level >= 1 {
        c.push((vec![n(6), n(6)], 7));
    }
    if level >= 2 {
        c.push((vec![n(18), n(6)], 4));
        c.push((vec![n(36)], 1));
        c.push((vec![r(24)], 1));
    }
    if level >= 3 {
        c.push((vec![n(6), n(12), n(6)], 4));
        c.push((vec![r(6), n(6)], 2));
        c.push((vec![n(6), r(6)], 1));
    }
    if level >= 4 {
        c.push((vec![n(4), n(4), n(4)], 4));
    }
    if level >= 5 {
        c.push((vec![n(3), n(3), n(3), n(3)], 4));
        c.push((vec![n(6), n(3), n(3)], 3));
        c.push((vec![n(3), n(3), n(6)], 3));
        c.push((vec![n(9), n(3)], 3));
    }
    c
}

/// A fresh line: the rhythm a bar at a time from the level's cells, then the pitches - mostly steps through the
/// key, a leap now and then, starting and ending on the key note.
pub(crate) fn make_line(seed: u64, key: usize, pos: u8, level: usize, bars: usize, beats: u16) -> Reading {
    let mut rng = Rng(seed | 1);
    let bar_len = beats * BEAT;
    let all = cells(level);
    let mut rhythm: Vec<Vec<(u16, bool)>> = Vec::new();
    for b in 0..bars {
        let mut bar: Vec<(u16, bool)> = Vec::new();
        let mut at = 0u16;
        let last = b + 1 == bars;
        while at < bar_len {
            let left = bar_len - at;
            if last && left <= 2 * BEAT && at > 0 {
                bar.push((left, false));
                break;
            }
            let fit: Vec<usize> = (0..all.len())
                .filter(|i| {
                    let len: u16 = all[*i].0.iter().map(|x| x.0).sum();
                    len <= left && (len < 2 * BEAT || at % (2 * BEAT) == 0) && (len != 4 * BEAT || at == 0)
                })
                .collect();
            let w: Vec<u32> = fit.iter().map(|i| all[*i].1).collect();
            let cell = &all[fit[rng.pick_weighted(&w)]].0;
            if b == 0 && bar.is_empty() && cell[0].1 {
                continue;
            }
            bar.extend(cell.iter().copied());
            at += cell.iter().map(|x| x.0).sum::<u16>();
        }
        rhythm.push(bar);
    }
    let avail = position_notes(key, pos);
    let tonic = KEYS[key].1;
    let sharps = KEYS[key].2 > 0;
    let tonics: Vec<usize> = (0..avail.len()).filter(|i| (avail[*i] - tonic).rem_euclid(12) == 0).collect();
    let mid = avail.len() / 2;
    let mut i = tonics.iter().copied().min_by_key(|t| (*t as i32 - mid as i32).abs()).unwrap_or(mid);
    let n_notes = rhythm.iter().flatten().filter(|x| !x.1).count();
    let mut k = 0;
    let mut out_bars = Vec::new();
    for bar in rhythm {
        let mut notes = Vec::new();
        for (dur, rest) in bar {
            if rest {
                notes.push(RNote { heads: Vec::new(), dur, tie: false, marks: Marks::default() });
                continue;
            }
            if k > 0 {
                if k + 1 == n_notes {
                    i = tonics.iter().copied().min_by_key(|t| (*t as i32 - i as i32).abs()).unwrap_or(i);
                } else {
                    const MOVES: [i32; 9] = [0, 1, -1, 2, -2, 3, -3, 4, -4];
                    let w: [u32; 9] = [4, 30, 30, 11, 11, 4, 4, 2 + level as u32, 2 + level as u32];
                    let mut step = MOVES[rng.pick_weighted(&w)];
                    if (i as i32 + step) < 0 || (i as i32 + step) >= avail.len() as i32 {
                        step = -step;
                    }
                    i = (i as i32 + step).clamp(0, avail.len() as i32 - 1) as usize;
                }
            }
            notes.push(RNote {
                heads: avail.get(i).map(|m| vec![head_of(*m, sharps)]).unwrap_or_default(),
                dur,
                tie: false,
                marks: Marks::default(),
            });
            k += 1;
        }
        out_bars.push(Bar { notes, ..Default::default() });
    }
    Reading { bars: out_bars, beats, den: 4, sig: KEYS[key].2, pos }
}

// ------------------------------------------------------------------ playing

/// Every note in the order it is played (repeats taken), tied notes joined: (note number as written, start and
/// length in beats, the notes sounding). The note number counts through the bars as written.
fn play_order(r: &Reading) -> Vec<(usize, f32, f32, Vec<i32>)> {
    let first: Vec<usize> = {
        let mut n = 0;
        r.bars
            .iter()
            .map(|b| {
                let s = n;
                n += b.notes.len();
                s
            })
            .collect()
    };
    let mut order: Vec<usize> = Vec::new();
    let mut from = 0;
    let mut repeated: Vec<usize> = Vec::new();
    let mut b = 0;
    while b < r.bars.len() {
        if r.bars[b].rep_start {
            from = b;
        }
        order.push(b);
        if r.bars[b].rep_end && !repeated.contains(&b) {
            repeated.push(b);
            b = from;
            continue;
        }
        b += 1;
    }
    let mut out: Vec<(usize, f32, f32, Vec<i32>)> = Vec::new();
    let mut at = 0.0f32;
    let mut tied = false;
    for b in order {
        for (j, n) in r.bars[b].notes.iter().enumerate() {
            let d = n.dur as f32 / r.beat_len() as f32;
            if tied {
                if let Some(last) = out.last_mut() {
                    last.2 += d;
                }
            } else {
                out.push((first[b] + j, at, d, n.heads.iter().map(|h| h.midi).collect()));
            }
            tied = n.tie && !n.heads.is_empty();
            at += d;
        }
    }
    out
}

// ------------------------------------------------------------------ drawing

/// The sharp or flat the key signature gives a letter (0 C .. 6 B).
fn key_acc(sig: i32, letter: i32) -> i32 {
    const SHARPS: [i32; 7] = [3, 0, 4, 1, 5, 2, 6];
    const FLATS: [i32; 7] = [6, 2, 5, 1, 4, 0, 3];
    if sig > 0 && SHARPS[..sig as usize].contains(&letter) {
        1
    } else if sig < 0 && FLATS[..(-sig) as usize].contains(&letter) {
        -1
    } else {
        0
    }
}

/// A natural sign at `c`.
fn natural(p: &egui::Painter, c: Pos2, g: f32, col: Color32) {
    let s = Stroke::new(1.1_f32, col);
    p.line_segment([Pos2::new(c.x - 0.35 * g, c.y - 1.3 * g), Pos2::new(c.x - 0.35 * g, c.y + 0.5 * g)], s);
    p.line_segment([Pos2::new(c.x + 0.35 * g, c.y - 0.5 * g), Pos2::new(c.x + 0.35 * g, c.y + 1.3 * g)], s);
    p.line_segment(
        [Pos2::new(c.x - 0.35 * g, c.y - 0.1 * g), Pos2::new(c.x + 0.35 * g, c.y - 0.4 * g)],
        Stroke::new(1.8_f32, col),
    );
    p.line_segment(
        [Pos2::new(c.x - 0.35 * g, c.y + 0.5 * g), Pos2::new(c.x + 0.35 * g, c.y + 0.2 * g)],
        Stroke::new(1.8_f32, col),
    );
}

/// A rest of `dur` twelfths of a beat, centred at `x` on the staff whose bottom line is at `bottom`.
fn draw_rest(p: &egui::Painter, x: f32, bottom: f32, g: f32, dur: u16, col: Color32) {
    let mid = bottom - 2.0 * g;
    match dur {
        48 => {
            let y = bottom - 3.0 * g;
            p.rect_filled(Rect::from_min_max(Pos2::new(x - g * 0.7, y), Pos2::new(x + g * 0.7, y + g * 0.5)), 0.0, col);
        }
        24 | 36 => {
            p.rect_filled(Rect::from_min_max(Pos2::new(x - g * 0.7, mid - g * 0.5), Pos2::new(x + g * 0.7, mid)), 0.0, col);
        }
        12 | 18 | 8 => {
            let k = g / 7.0 * 1.2;
            let pts: Vec<Pos2> = [(-2.0, -10.0), (3.0, -5.0), (-2.0, 0.0), (3.0, 5.0), (-1.0, 5.0), (1.0, 10.0)]
                .iter()
                .map(|(dx, dy)| Pos2::new(x + dx * k, mid + dy * k))
                .collect();
            p.add(egui::Shape::line(pts, Stroke::new(2.0_f32, col)));
        }
        _ => {
            let hooks = if dur <= 3 { 2 } else { 1 };
            p.line_segment([Pos2::new(x + 3.0, mid - g * 0.8), Pos2::new(x - 1.0, mid + g * 1.4)], Stroke::new(1.6_f32, col));
            for h in 0..hooks {
                p.circle_filled(Pos2::new(x - 2.0 - h as f32, mid - g * 0.6 + h as f32 * g), 2.0, col);
            }
        }
    }
}

/// Draw a line of music on staves, as many bars to a row as fit. `lit` is the note sounding now (counted through
/// the bars as written). With `tab` the tab is under each staff; with `names` the note names are.
fn draw_reading(ui: &mut egui::Ui, r: &Reading, lit: Option<usize>, tab: bool, names: bool, zoom: f32) {
    draw_score(ui, &[(r, lit, "")], tab, names, zoom);
}

/// The score magnified by `zoom`: laid out at the width it will have once scaled, drawn on its own layer that is
/// scaled up (so every line, head and mark grows together) and placed where it belongs on the page.
fn draw_score(ui: &mut egui::Ui, parts: &[(&Reading, Option<usize>, &str)], tab: bool, names: bool, zoom: f32) {
    if (zoom - 1.0).abs() < 0.01 {
        draw_score_at(ui, parts, tab, names);
        return;
    }
    let origin = ui.cursor().min;
    let w = ui.available_width().max(300.0);
    let layer = egui::LayerId::new(ui.layer_id().order, ui.id().with("score_zoom"));
    let tf = egui::emath::TSTransform::new(origin.to_vec2() * (1.0 - zoom), zoom);
    ui.ctx().set_transform_layer(layer, tf);
    ui.ctx().set_sublayer(ui.layer_id(), layer);
    let clip = tf.inverse().mul_rect(ui.clip_rect());
    let h = ui
        .with_layer_id(layer, |ui| {
            let mut c = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(origin, Vec2::new(w / zoom, 1.0e6))));
            c.set_clip_rect(clip);
            draw_score_at(&mut c, parts, tab, names);
            c.min_rect().height()
        })
        .inner;
    ui.allocate_exact_size(Vec2::new(w, h * zoom), Sense::hover());
}

/// Several parts (a duet) as systems: their staves braced together, the bars lined up and wrapping as one, each
/// part named on the first row. Each part has its own note sounding now (counted through its bars as written).
fn draw_score_at(ui: &mut egui::Ui, parts: &[(&Reading, Option<usize>, &str)], tab: bool, names: bool) {
    let Some(first) = parts.first() else { return };
    let g = 7.0_f32;
    let sig = first.0.sig;
    let beats = first.0.beats;
    let slot = |d: u16| 20.0 + d as f32 * 1.7;
    let bar_w = |b: &Bar| {
        18.0 + b.notes.iter().map(|n| slot(n.dur) + if n.heads.iter().any(|h| h.acc != 0) { 8.0 } else { 0.0 }).sum::<f32>()
    };
    let n_bars = parts.iter().map(|p| p.0.bars.len()).max().unwrap_or(0);
    // a bar is as wide as its busiest part
    let shared = |bi: usize| parts.iter().filter_map(|p| p.0.bars.get(bi)).map(|b| bar_w(b)).fold(0.0, f32::max);
    let w = ui.available_width().max(300.0);
    let head_w = |first: bool| 40.0 + sig.unsigned_abs() as f32 * 9.0 + if first { 26.0 } else { 0.0 };
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut used = 0.0;
    for bi in 0..n_bars {
        let need = shared(bi);
        if rows.is_empty() || used + need > w - 12.0 {
            rows.push(Vec::new());
            used = head_w(rows.len() == 1);
        }
        if let Some(row) = rows.last_mut() {
            row.push(bi);
        }
        used += need;
    }
    let (above, staff_h, below) = (62.0, 4.0 * g, 52.0);
    let tab_h = if tab { 58.0 } else { 0.0 };
    let names_h = if names { 18.0 } else { 0.0 };
    let block_h = above + staff_h + below + tab_h + names_h;
    let multi = parts.len() > 1;
    for (ri, row) in rows.iter().enumerate() {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, block_h * parts.len() as f32), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect.shrink2(Vec2::new(0.0, 2.0)), 3.0, pal().beige_lt);
        let line = Stroke::new(1.0_f32, pal().ink2);
        let ink = pal().ink;
        let natural_w: f32 = row.iter().map(|b| shared(*b)).sum();
        let x_head = rect.min.x + head_w(ri == 0);
        let room = rect.max.x - 10.0 - x_head;
        let stretch_row = if ri + 1 == rows.len() && natural_w < room * 0.6 { 1.0 } else { room / natural_w };
        let x_end = x_head + natural_w * stretch_row;
        // where each bar of the row starts, and how wide it is
        let mut bar_x: Vec<(f32, f32)> = Vec::new();
        let mut xx = x_head;
        for bi in row {
            let wd = shared(*bi) * stretch_row;
            bar_x.push((xx, wd));
            xx += wd;
        }
        let staff_top = |pi: usize| rect.min.y + pi as f32 * block_h + above;
        if multi {
            // the brace joining the staves, and the bar lines running through them all
            let (y0, y1) = (staff_top(0), staff_top(parts.len() - 1) + staff_h);
            p.line_segment([Pos2::new(rect.min.x + 6.0, y0), Pos2::new(rect.min.x + 6.0, y1)], Stroke::new(3.0_f32, ink));
            for (x0, wd) in &bar_x {
                p.line_segment([Pos2::new(x0 + wd, y0), Pos2::new(x0 + wd, y1)], line);
            }
        }
        for (pi, (r, lit, pname)) in parts.iter().enumerate() {
            let lit = *lit;
            let top = staff_top(pi);
            let bottom = top + staff_h;
            let y_of = |step: i32| bottom - (step - 30) as f32 * g / 2.0;
            let mut first_note = Vec::new();
            let mut n = 0;
            for b in &r.bars {
                first_note.push(n);
                n += b.notes.len();
            }
            if multi && ri == 0 && !pname.is_empty() {
                ptext(p, Pos2::new(rect.min.x + 14.0, top - 3.6 * g), Align::Min, &pname.to_uppercase(), 1.0, pal().ink2);
            }
            for k in 0..5 {
                let y = top + k as f32 * g;
                p.line_segment([Pos2::new(rect.min.x + 6.0, y), Pos2::new(x_end, y)], line);
            }
            pixmap(
                p,
                Pos2::new(rect.min.x + 10.0, top - 1.7 * g),
                7.4 * g / crate::lines_ui::CLEF.len() as f32,
                &crate::lines_ui::CLEF,
                ink,
            );
            const SHARP_STEPS: [i32; 7] = [38, 35, 39, 36, 33, 37, 34];
            const FLAT_STEPS: [i32; 7] = [34, 37, 33, 36, 32, 35, 31];
            for k in 0..(sig.unsigned_abs() as usize).min(7) {
                let st = if sig > 0 { SHARP_STEPS[k] } else { FLAT_STEPS[k] };
                crate::lines_ui::accidental(p, Pos2::new(rect.min.x + 40.0 + k as f32 * 9.0, y_of(st)), sig.signum(), g, ink);
            }
            if ri == 0 {
                let tx = rect.min.x + 40.0 + sig.unsigned_abs() as f32 * 9.0 + 12.0;
                ptext(p, Pos2::new(tx, top + g), Align::Center, &beats.to_string(), 3.0, ink);
                ptext(p, Pos2::new(tx, top + 3.0 * g), Align::Center, &first.0.den.to_string(), 3.0, ink);
            }
            if let (0, Some(b0)) = (pi, row.first()) {
                ptext(p, Pos2::new(rect.min.x + 8.0, top - 3.0 * g), Align::Min, &(b0 + 1).to_string(), 1.0, pal().dim);
            }
            let tab_top = bottom + below;
            let tab_y = |s: usize| tab_top + (5 - s) as f32 * 9.0;
            if tab {
                for s in 0..6 {
                    p.line_segment([Pos2::new(rect.min.x + 6.0, tab_y(s)), Pos2::new(x_end, tab_y(s))], line);
                }
            }
            // a tie waiting for its next note: (x, step, stem up)
            let mut tie_from: Option<(f32, Vec<i32>, bool)> = None;
            // a slur under way: where it started (x, the y over the notes)
            let mut slur_from: Option<(f32, f32)> = None;
            for (j, bi) in row.iter().enumerate() {
                let Some(b) = r.bars.get(*bi) else { continue };
                // this part's notes spread over the bar's shared width
                let (x0, wd) = bar_x[j];
                let stretch = wd / bar_w(b);
                let mut x = x0;
                if b.rep_start {
                    p.line_segment([Pos2::new(x + 2.0, top), Pos2::new(x + 2.0, bottom)], Stroke::new(3.0_f32, ink));
                    p.line_segment([Pos2::new(x + 6.0, top), Pos2::new(x + 6.0, bottom)], line);
                    for dy in [1.5, 2.5] {
                        p.circle_filled(Pos2::new(x + 10.0, top + dy * g), 1.8, ink);
                    }
                }
                x += 9.0 * stretch + if b.rep_start { 8.0 } else { 0.0 };
                // where each note sits
                let mut xs: Vec<f32> = Vec::new();
                let mut starts: Vec<u16> = Vec::new();
                let mut xx = x;
                let mut at = 0u16;
                for n in &b.notes {
                    let accw = if n.heads.iter().any(|h| h.acc != 0) { 8.0 } else { 0.0 };
                    xs.push(xx + accw + slot(n.dur) * stretch * 0.35);
                    starts.push(at);
                    xx += (slot(n.dur) + accw) * stretch;
                    at += n.dur;
                }
                // beam groups: short notes inside one beat
                let mut groups: Vec<Vec<usize>> = Vec::new();
                let mut cur: Vec<usize> = Vec::new();
                for (k, n) in b.notes.iter().enumerate() {
                    let short = n.dur < BEAT && n.dur != 8 && !n.heads.is_empty();
                    let same_beat = cur.last().is_some_and(|l| starts[*l] / BEAT == starts[k] / BEAT);
                    if short && same_beat {
                        cur.push(k);
                    } else {
                        if !cur.is_empty() {
                            groups.push(std::mem::take(&mut cur));
                        }
                        if short {
                            cur.push(k);
                        }
                    }
                }
                if !cur.is_empty() {
                    groups.push(cur);
                }
                let mean_step = |k: usize| {
                    let h = &b.notes[k].heads;
                    h.iter().map(|x| x.step).sum::<i32>() as f32 / h.len().max(1) as f32
                };
                let mut tips: Vec<Option<(f32, bool)>> = vec![None; b.notes.len()];
                for gr in groups.iter().filter(|g| g.len() > 1) {
                    let up = gr.iter().map(|k| mean_step(*k)).sum::<f32>() / gr.len() as f32 <= 34.0;
                    let ys: Vec<f32> = gr.iter().flat_map(|k| b.notes[*k].heads.iter().map(|h| y_of(h.step))).collect();
                    let tip = if up {
                        ys.iter().cloned().fold(f32::INFINITY, f32::min) - 3.4 * g
                    } else {
                        ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max) + 3.4 * g
                    };
                    for k in gr {
                        tips[*k] = Some((tip, up));
                    }
                }
                // sharps, flats and naturals written in this bar so far (by staff step)
                let mut shown: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
                for (k, n) in b.notes.iter().enumerate() {
                    let gi = first_note[*bi] + k;
                    let cx = xs[k];
                    let on = lit == Some(gi);
                    let col = if on { pal().red } else { ink };
                    if on {
                        p.rect_filled(
                            Rect::from_min_max(
                                Pos2::new(cx - 11.0, top - above + 6.0),
                                Pos2::new(cx + 13.0, bottom + below - 4.0),
                            ),
                            2.0,
                            pal().red.gamma_multiply(0.15),
                        );
                    }
                    if n.heads.is_empty() {
                        draw_rest(p, cx, bottom, g, n.dur, col);
                        draw_marks(p, n, cx, top, bottom, top - g, bottom + g, false, g, &mut slur_from, col);
                        tie_from = None;
                        continue;
                    }
                    let rad = g * 0.58;
                    let up = tips[k].map(|t| t.1).unwrap_or(mean_step(k) <= 34.0);
                    // ledger lines
                    let lo = n.heads.iter().map(|h| h.step).min().unwrap_or(30);
                    let hi = n.heads.iter().map(|h| h.step).max().unwrap_or(30);
                    let mut s = 28;
                    while s >= lo {
                        p.line_segment(
                            [
                                Pos2::new(cx - 9.0, y_of(s)),
                                Pos2::new(cx + 9.0 + rad * 2.0 * (n.heads.len() > 1) as i32 as f32, y_of(s)),
                            ],
                            line,
                        );
                        s -= 2;
                    }
                    let mut s = 40;
                    while s <= hi {
                        p.line_segment(
                            [
                                Pos2::new(cx - 9.0, y_of(s)),
                                Pos2::new(cx + 9.0 + rad * 2.0 * (n.heads.len() > 1) as i32 as f32, y_of(s)),
                            ],
                            line,
                        );
                        s += 2;
                    }
                    // heads (the upper of two a step apart beside the other), dots, accidentals
                    let open = n.dur >= 2 * BEAT;
                    let mut side = vec![0.0f32; n.heads.len()];
                    for i in 1..n.heads.len() {
                        if n.heads[i].step - n.heads[i - 1].step <= 1 && side[i - 1] == 0.0 {
                            side[i] = rad * 2.0;
                        }
                    }
                    let mut acc_col = 0;
                    for (i, h) in n.heads.iter().enumerate().rev() {
                        let c = Pos2::new(cx + side[i], y_of(h.step));
                        if open {
                            p.circle_stroke(c, rad - 0.6, Stroke::new(1.8_f32, col));
                        } else {
                            p.circle_filled(c, rad, col);
                        }
                        if matches!(n.dur, 9 | 18 | 36) {
                            let dy = if h.step % 2 == 0 { -g / 2.0 } else { 0.0 };
                            p.circle_filled(
                                Pos2::new(cx + side.iter().cloned().fold(0.0, f32::max) + rad + 4.0, c.y + dy),
                                1.8,
                                col,
                            );
                        }
                        let expected = shown.get(&h.step).copied().unwrap_or(key_acc(sig, h.step.rem_euclid(7)));
                        if h.acc != expected {
                            let ax = cx - g * 2.0 - acc_col as f32 * g * 1.5;
                            if h.acc == 0 {
                                natural(p, Pos2::new(ax, c.y), g, col);
                            } else {
                                if h.acc == 2 {
                                    // a double sharp
                                    ptext(p, Pos2::new(ax, c.y - 5.0), Align::Center, "x", 1.0, col);
                                } else if h.acc == -2 {
                                    crate::lines_ui::accidental(p, Pos2::new(ax - g * 0.9, c.y), -1, g, col);
                                    crate::lines_ui::accidental(p, Pos2::new(ax, c.y), -1, g, col);
                                } else {
                                    crate::lines_ui::accidental(p, Pos2::new(ax, c.y), h.acc, g, col);
                                }
                            }
                            acc_col = (acc_col + 1) % 2;
                            shown.insert(h.step, h.acc);
                        }
                    }
                    // a tie arriving here from the note before
                    if let Some((fx, steps, tup)) = tie_from.take() {
                        for st in steps {
                            let y = y_of(st) + if tup { g * 0.9 } else { -g * 0.9 };
                            let mid = Pos2::new((fx + cx) / 2.0, y + if tup { 5.0 } else { -5.0 });
                            let pts: Vec<Pos2> = (0..=8)
                                .map(|t| {
                                    let t = t as f32 / 8.0;
                                    let a = Pos2::new(fx + 4.0, y);
                                    let z = Pos2::new(cx - 4.0, y);
                                    let u = 1.0 - t;
                                    Pos2::new(
                                        u * u * a.x + 2.0 * u * t * mid.x + t * t * z.x,
                                        u * u * a.y + 2.0 * u * t * mid.y + t * t * z.y,
                                    )
                                })
                                .collect();
                            p.add(egui::Shape::line(pts, Stroke::new(1.3_f32, col)));
                        }
                    }
                    if n.tie {
                        tie_from = Some((cx, n.heads.iter().map(|h| h.step).collect(), up));
                    }
                    if n.dur < 4 * BEAT {
                        let ys: Vec<f32> = n.heads.iter().map(|h| y_of(h.step)).collect();
                        let (ytop, ybot) = (
                            ys.iter().cloned().fold(f32::INFINITY, f32::min),
                            ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
                        );
                        let tip = tips[k].map(|t| t.0).unwrap_or(if up { ytop - 3.4 * g } else { ybot + 3.4 * g });
                        let sx = if up { cx + rad - 0.5 } else { cx - rad + 0.5 };
                        p.line_segment(
                            [Pos2::new(sx, if up { ybot } else { ytop }), Pos2::new(sx, tip)],
                            Stroke::new(1.3_f32, col),
                        );
                        let lone = groups.iter().find(|gr| gr.contains(&k)).is_none_or(|gr| gr.len() < 2);
                        if n.dur < BEAT && n.dur != 8 && lone {
                            let flags = if n.dur <= 3 { 2 } else { 1 };
                            for f in 0..flags {
                                let d = if up { 1.0 } else { -1.0 };
                                let fy = tip + f as f32 * 6.0 * d;
                                let pts = vec![
                                    Pos2::new(sx, fy),
                                    Pos2::new(sx + 6.0, fy + 7.0 * d),
                                    Pos2::new(sx + 5.0, fy + 13.0 * d),
                                ];
                                p.add(egui::Shape::line(pts, Stroke::new(1.6_f32, col)));
                            }
                        }
                        if n.dur == 8 {
                            let ty = if up { tip - 8.0 } else { tip + 8.0 };
                            ptext(p, Pos2::new(cx, ty), Align::Center, "3", 1.0, pal().ink2);
                        }
                    }
                    {
                        // marks: over the note and its stem, or beside a chord's heads
                        let ys: Vec<f32> = n.heads.iter().map(|h| y_of(h.step)).collect();
                        let ytop = ys.iter().cloned().fold(f32::INFINITY, f32::min);
                        let ybot = ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                        let stem_tip = tips[k].map(|t| t.0).unwrap_or(if up { ytop - 3.4 * g } else { ybot + 3.4 * g });
                        let ceil = if up && n.dur < 4 * BEAT { stem_tip.min(ytop) } else { ytop } - 4.0;
                        let floor = if !up && n.dur < 4 * BEAT { stem_tip.max(ybot) } else { ybot } + 4.0;
                        draw_marks(p, n, cx, top, bottom, ceil, floor, up, g, &mut slur_from, col);
                        if n.heads.len() > 1 && !n.marks.fingers.is_empty() {
                            let fx = cx + rad * 2.0 + 7.0 + if matches!(n.dur, 9 | 18 | 36) { 6.0 } else { 0.0 };
                            for (h, f) in n.heads.iter().zip(n.marks.fingers.chars()) {
                                if f != 'x' {
                                    ptext(p, Pos2::new(fx, y_of(h.step) - 4.0), Align::Center, &f.to_string(), 0.8, pal().ink2);
                                }
                            }
                        }
                    }
                    if tab {
                        let midis: Vec<i32> = n.heads.iter().map(|h| h.midi).collect();
                        for sf in finger_all(&midis, r.pos).into_iter().flatten() {
                            let txt = sf.1.to_string();
                            p.rect_filled(
                                Rect::from_center_size(Pos2::new(cx, tab_y(sf.0)), Vec2::new(text_w(&txt, 1.0) + 6.0, 9.0)),
                                0.0,
                                pal().beige_lt,
                            );
                            ptext(p, Pos2::new(cx, tab_y(sf.0)), Align::Center, &txt, 1.0, col);
                        }
                    }
                    if names {
                        let nm: Vec<String> = n
                            .heads
                            .iter()
                            .rev()
                            .map(|h| {
                                format!(
                                    "{}{}",
                                    ["C", "D", "E", "F", "G", "A", "B"][h.step.rem_euclid(7) as usize],
                                    match h.acc {
                                        1 => "#",
                                        -1 => "b",
                                        _ => "",
                                    }
                                )
                            })
                            .collect();
                        ptext(
                            p,
                            Pos2::new(cx, top - above + block_h - 10.0),
                            Align::Center,
                            &nm.join(""),
                            1.0,
                            if on { pal().red } else { pal().dim },
                        );
                    }
                }
                // beams: one for eighths, a second for sixteenths, a 3 over triplets
                for gr in groups.iter().filter(|g| g.len() > 1) {
                    let Some((tip, up)) = tips[gr[0]] else { continue };
                    let sx = |k: usize| xs[k] + if up { g * 0.58 - 0.5 } else { -g * 0.58 + 0.5 };
                    let (a, z) = (sx(gr[0]), sx(gr[gr.len() - 1]));
                    let thick = 3.0;
                    let y1 = if up { tip } else { tip - thick };
                    p.rect_filled(Rect::from_min_max(Pos2::new(a, y1), Pos2::new(z, y1 + thick)), 0.0, ink);
                    let y2 = if up { tip + 5.0 } else { tip - 5.0 - thick };
                    for (j, k) in gr.iter().enumerate() {
                        if b.notes[*k].dur > 3 {
                            continue;
                        }
                        let next16 = gr.get(j + 1).is_some_and(|kk| b.notes[*kk].dur <= 3);
                        let x0 = sx(*k);
                        let x1 = if next16 {
                            sx(gr[j + 1])
                        } else if j + 1 < gr.len() {
                            x0 + 8.0
                        } else {
                            x0 - 8.0
                        };
                        p.rect_filled(Rect::from_min_max(Pos2::new(x0.min(x1), y2), Pos2::new(x0.max(x1), y2 + thick)), 0.0, ink);
                    }
                    if b.notes[gr[0]].dur == 4 {
                        let ty = if up { tip - 9.0 } else { tip + 9.0 };
                        ptext(p, Pos2::new((a + z) / 2.0, ty), Align::Center, "3", 1.0, pal().ink2);
                    }
                }
                x += (bar_w(b) - 9.0) * stretch;
                let end = *bi + 1 == n_bars;
                if b.rep_end {
                    for dy in [1.5, 2.5] {
                        p.circle_filled(Pos2::new(x - 10.0, top + dy * g), 1.8, ink);
                    }
                }
                if end || b.rep_end {
                    p.line_segment([Pos2::new(x - 4.0, top), Pos2::new(x - 4.0, bottom)], line);
                    p.line_segment([Pos2::new(x, top), Pos2::new(x, bottom)], Stroke::new(3.0_f32, ink));
                } else {
                    p.line_segment([Pos2::new(x, top), Pos2::new(x, bottom)], line);
                }
            }
            // a tie running over the end of the row
            if let Some((fx, steps, tup)) = tie_from {
                for st in steps {
                    let y = y_of(st) + if tup { g * 0.9 } else { -g * 0.9 };
                    p.line_segment(
                        [Pos2::new(fx + 4.0, y), Pos2::new(x_end, y + if tup { 3.0 } else { -3.0 })],
                        Stroke::new(1.3_f32, pal().ink),
                    );
                }
            }
        }
        ui.add_space(6.0);
    }
}

/// The marks printed with a note: words and the pick stroke, finger and string over it (`ceil` is the top of the
/// note and its stem), accents and staccato by the head away from the stem, a fermata on top, and slurs.
#[allow(clippy::too_many_arguments)]
fn draw_marks(
    p: &egui::Painter,
    n: &RNote,
    cx: f32,
    top: f32,
    bottom: f32,
    ceil: f32,
    floor: f32,
    up: bool,
    g: f32,
    slur_from: &mut Option<(f32, f32)>,
    col: Color32,
) {
    let m = &n.marks;
    let ink = pal().ink;
    // stacked upwards from over the staff (or the note, if higher)
    let mut y = ceil.min(top - g * 0.6);
    if (m.accent || m.stac || m.tenuto) && !n.heads.is_empty() {
        // by the head, on the side away from the stem
        let ay = if up { floor.max(bottom) + g * 0.2 } else { y - g * 0.4 };
        if m.accent {
            p.add(egui::Shape::line(
                vec![Pos2::new(cx - 5.0, ay - 3.0), Pos2::new(cx + 5.0, ay), Pos2::new(cx - 5.0, ay + 3.0)],
                Stroke::new(1.3_f32, col),
            ));
        } else if m.tenuto {
            p.line_segment([Pos2::new(cx - 5.0, ay), Pos2::new(cx + 5.0, ay)], Stroke::new(1.6_f32, col));
        } else {
            p.circle_filled(Pos2::new(cx, ay), 1.6, col);
        }
        if !up {
            y -= g;
        }
    }
    if n.heads.len() <= 1 && !m.fingers.is_empty() && m.fingers != "x" {
        ptext(p, Pos2::new(cx, y - 7.0), Align::Center, &m.fingers, 0.8, pal().ink2);
        y -= 10.0;
    }
    if m.string > 0 {
        let c = Pos2::new(cx, y - 6.0);
        p.circle_stroke(c, 5.0, Stroke::new(1.0_f32, pal().ink2));
        ptext(p, Pos2::new(c.x, c.y - 4.0), Align::Center, &m.string.to_string(), 0.7, pal().ink2);
        y -= 13.0;
    }
    if m.pick != 0 {
        let (l, r, t0, b0) = (cx - 4.0, cx + 4.0, y - 9.0, y - 2.0);
        let s = Stroke::new(1.2_f32, ink);
        if m.pick > 0 {
            // down-stroke: an open box, thick on top
            p.rect_filled(Rect::from_min_max(Pos2::new(l, t0), Pos2::new(r, t0 + 2.2)), 0.0, ink);
            p.line_segment([Pos2::new(l, t0), Pos2::new(l, b0)], s);
            p.line_segment([Pos2::new(r, t0), Pos2::new(r, b0)], s);
        } else {
            p.add(egui::Shape::line(vec![Pos2::new(l, t0), Pos2::new(cx, b0), Pos2::new(r, t0)], s));
        }
        y -= 12.0;
    }
    if m.fermata {
        let c = Pos2::new(cx, y - 3.0);
        let pts: Vec<Pos2> = (0..=10)
            .map(|i| {
                let a = std::f32::consts::PI * i as f32 / 10.0;
                Pos2::new(c.x - 7.0 * a.cos(), c.y - 6.0 * a.sin())
            })
            .collect();
        p.add(egui::Shape::line(pts, Stroke::new(1.4_f32, ink)));
        p.circle_filled(c, 1.5, ink);
        y -= 10.0;
    }
    // words: dynamics under the staff, the rest over everything
    const DYN: [&str; 10] = ["ppp", "pp", "p", "mp", "mf", "f", "ff", "fff", "sfz", "fp"];
    let mut wy = y;
    for w in &m.text {
        if DYN.contains(&w.as_str()) || w.starts_with("cresc") || w.starts_with("dim") {
            ptext(p, Pos2::new(cx - 6.0, floor.max(bottom) + g * 0.6), Align::Min, w, 1.0, ink);
        } else {
            ptext(p, Pos2::new(cx - 6.0, wy - 11.0), Align::Min, w, 1.0, ink);
            wy -= 11.0;
        }
    }
    // slurs: an arc over the notes from where it started
    let arc_y = ceil.min(top) - 2.0;
    if m.slur_end {
        if let Some((sx, sy)) = slur_from.take() {
            let yy = sy.min(arc_y);
            let mid = Pos2::new((sx + cx) / 2.0, yy - 8.0);
            let pts: Vec<Pos2> = (0..=12)
                .map(|t| {
                    let t = t as f32 / 12.0;
                    let u = 1.0 - t;
                    Pos2::new(u * u * sx + 2.0 * u * t * mid.x + t * t * cx, u * u * sy + 2.0 * u * t * mid.y + t * t * arc_y)
                })
                .collect();
            p.add(egui::Shape::line(pts, Stroke::new(1.2_f32, col)));
        }
    }
    if m.slur_start {
        *slur_from = Some((cx, arc_y));
    }
}

/// The notes of the key in a position, on a little neck (fret numbers along the bottom).
fn position_map(ui: &mut egui::Ui, tonic: i32, pos: u8, sharps: bool) {
    let (lo, hi) = window(pos);
    let n = (hi - lo + 1) as f32;
    let w = (n * 56.0 + 40.0).min(ui.available_width());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 130.0), Sense::hover());
    let neck = Rect::from_min_max(rect.min + Vec2::new(30.0, 8.0), rect.max - Vec2::new(8.0, 22.0));
    let p = ui.painter();
    p.rect_filled(neck, 2.0, pal().lcd);
    let fw = neck.width() / n;
    let sh = neck.height() / 6.0;
    for f in 0..=(hi - lo + 1) {
        let x = neck.min.x + f as f32 * fw;
        p.line_segment(
            [Pos2::new(x, neck.min.y), Pos2::new(x, neck.max.y)],
            Stroke::new(if lo == 0 && f == 1 { 3.0_f32 } else { 1.0 }, pal().ink2),
        );
    }
    for f in lo..=hi {
        ptext(
            p,
            Pos2::new(neck.min.x + (f - lo) as f32 * fw + fw / 2.0, neck.max.y + 11.0),
            Align::Center,
            &f.to_string(),
            1.0,
            pal().dim,
        );
    }
    for s in 0..6 {
        let y = neck.min.y + (5 - s) as f32 * sh + sh / 2.0;
        p.line_segment([Pos2::new(neck.min.x, y), Pos2::new(neck.max.x, y)], Stroke::new(1.0_f32, pal().ink.gamma_multiply(0.5)));
        for f in lo..=hi {
            let m = theory::STANDARD[s] + f as i32;
            if ![0, 2, 4, 5, 7, 9, 11].contains(&(m - tonic).rem_euclid(12)) {
                continue;
            }
            let c = Pos2::new(neck.min.x + (f - lo) as f32 * fw + fw / 2.0, y);
            let home = (m - tonic).rem_euclid(12) == 0;
            p.circle_filled(c, sh * 0.4, if home { pal().red } else { Color32::from_gray(235) });
            ptext(p, c, Align::Center, theory::note_name(m, sharps), 1.0, Color32::from_gray(10));
        }
    }
}

/// The key note for a signature (major), for the position map.
fn tonic_of(sig: i32) -> i32 {
    (sig * 7).rem_euclid(12)
}

impl App {
    /// The READING page.
    pub(crate) fn reading_view(&mut self, ui: &mut egui::Ui, _acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "SIGHT READING", 3.0, pal().ink);
        // ctrl + the mouse wheel (or a pinch) over the page magnifies the music
        let zd = ui.input(|i| i.zoom_delta());
        if zd != 1.0 && ui.rect_contains_pointer(ui.max_rect()) {
            self.rd_zoom = (self.rd_zoom * zd).clamp(0.75, 3.0);
        }
        if !self.rd_book_loaded {
            self.rd_reload();
        }
        if let Some(v) = tab_row(ui, &["THE BOOK", "MADE-UP LINES"], self.rd_mode as usize) {
            self.rd_mode = v as u8;
            self.player.send(Cmd::StopOnce);
            self.rd_play = None;
        }
        ui.add_space(6.0);
        if self.rd_mode == 0 {
            self.rd_book_page(ui);
        } else {
            self.rd_made_up(ui);
        }
        ui.add_space(10.0);
        section_header(ui, "HOW TO WORK");
        if let Some(v) = tab_row(
            ui,
            &["THE BASICS", "A DAILY ROUTINE", "THE MARKS", "DUETS", "WHEN IT'S HARD", "BEYOND THE BOOK"],
            self.rd_guide as usize,
        ) {
            self.rd_guide = v as u8;
        }
        ui.add_space(4.0);
        let tips: &[&str] = match self.rd_guide {
            1 => &[
                "Five minutes: warm up with yesterday's exercise at its tempo - it should feel easy now.",
                "Ten minutes: today's new exercise. Look it over, count a bar in, play it through once without stopping, however it goes. Then fix the bars that broke, slowly, and play it through again.",
                "Five minutes: a MADE-UP LINE in the same key and position, read once, cold. That is real sight reading - music you have never seen. The book is for learning the notes; made-up lines test them.",
                "Last: a duet with PLAY ALONG, or mark the exercise done and write down the tempo you reached. Twenty minutes a day moves faster than two hours on Sunday.",
            ],
            2 => &[
                "The open box over a note is a down-stroke, the V an up-stroke. Leavitt prints them where the picking matters - follow them exactly; they are the method.",
                "A small number by a note is the finger (0 open string, 1 index ... 4 little finger). A number in a circle is the string. Together they tell you where on the neck it lives.",
                "A dot over or under a note: short (staccato). A short line: hold it its full length (tenuto). The > is an accent - pick it harder. The arc with a dot (fermata): hold it as long as feels right.",
                "Words over the staff: Ritard (slow down), a tempo (back to speed), fine (the end), D.S. al coda (back to the sign, then jump to the coda - written out here as you play it). p, mf, f under the staff: soft, medium, loud.",
                "POS II, POS V ... marks a move to that position. F.S. is a finger stretch out of the position for one note - go back right after.",
            ],
            3 => &[
                "Learn your part with HEAR IT first if you have to, but then turn it off and read.",
                "PLAY ALONG plays only the other guitar and the clicks - you play your part. It counts in a bar. If you get lost, don't stop: find beat one of the next bar and jump back in. That is the most important reading skill there is.",
                "Then swap parts. Each duet is two lessons: the tune and the accompaniment. The 2nd part is usually the one that teaches rhythm.",
                "BOTH STAVES shows the whole system, the way a real chart looks. Glance at the other part while you play - it trains your eye to read ahead.",
            ],
            4 => &[
                "Too fast? Halve the tempo. Speed comes from playing it right slowly many times, not from forcing it.",
                "One bar keeps breaking: play just that bar and the first note of the next, four times, slowly. Then the whole line again.",
                "Lost in the rhythm: tap your foot and say the counts (1 & 2 & ...) without playing. Then play only the rhythm on one open string. Then add the notes.",
                "Lost on the neck: turn on THE POSITION for a minute and see where the key's notes sit. Turn on SHOW TAB only to check a note you weren't sure of - then turn it off again. If the tab stays on, you are reading tab, not music.",
                "Chord pages: say each chord's name out loud as you place it. Shapes stick when they have names.",
            ],
            5 => &[
                "Read something every day that you don't know: a lead sheet from a fake book, a Real Book head, a horn part. Read it once through at a slow tempo without stopping before you practise it.",
                "Read ahead: while you play one bar, your eyes should already be on the next. Cover the bar you're playing with a card if your eyes keep falling back.",
                "Read in other positions: take a book exercise you know and read it in position V or VII. Same notes, a new map.",
                "Read rhythms on their own: clap a line of eighths, triplets and rests from any page before you play it.",
                "Once this book is done, the same work carries on in Volume 2, in the Real Book, and in the horn and piano books - anything with notes on a staff.",
            ],
            _ => &[
                "Before you play, look it over: the key, the time, the highest and lowest notes, the hardest rhythm.",
                "Count a bar in out loud. Play slower than you think you need to, then raise the tempo 5 at a time.",
                "Eyes on the page, not the neck. Stay in the position: one finger per fret.",
                "Keep going. A wrong note is fine; stopping isn't. The beat matters more than the notes.",
                "A little every day beats a lot once a week. Work at your own pace and mark things done when they are.",
            ],
        };
        for tip in tips {
            para(ui, tip, pal().ink2);
        }
        ui.add_space(20.0);
    }

    /// Read the book file again (after it changed).
    pub(crate) fn rd_reload(&mut self) {
        self.rd_book_loaded = true;
        let (lessons, errs) = std::fs::read_to_string(book_path()).map(|t| parse_book(&t)).unwrap_or_default();
        self.rd_book = lessons;
        self.rd_errs = errs;
    }

    /// The tempo row and the play / helps buttons, shared by both kinds of reading.
    fn rd_controls(&mut self, ui: &mut egui::Ui, mine: &Reading, other: Option<&Reading>) {
        let lit_on = self.rd_play.is_some();
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, "TEMPO", 60.0);
            let before = self.rd_bpm;
            if retro_btn_w(ui, "-", 28.0, false).clicked() {
                self.rd_bpm = (self.rd_bpm - 5).max(30);
            }
            if self.rd_bpm_text.is_empty() || self.ed.id != crate::tools::F_RD_BPM {
                self.rd_bpm_text = self.rd_bpm.to_string();
            }
            let (r, _) = ui.allocate_exact_size(Vec2::new(64.0, bh()), Sense::hover());
            let o = crate::views::field(ui, &mut self.ed, crate::tools::F_RD_BPM, &mut self.rd_bpm_text, r, "BPM", false);
            if o.enter {
                if let Ok(b) = self.rd_bpm_text.trim().parse::<u32>() {
                    self.rd_bpm = b.clamp(30, 300);
                }
                self.ed.id = 0;
            }
            if retro_btn_w(ui, "+", 28.0, false).clicked() {
                self.rd_bpm = (self.rd_bpm + 5).min(300);
            }
            crate::views::label(ui, "BPM", 40.0);
            // a change while it plays: it carries on at the new tempo
            if self.rd_bpm != before {
                self.rd_retempo();
            }
            ui.add_space(10.0);
            if retro_btn(ui, "HEAR IT", false).tip("A bar to count you in, then this part").clicked() {
                self.rd_start(mine, None, false);
            }
            if let Some(o) = other {
                if retro_btn(ui, "HEAR BOTH", false).tip("Both guitars together").clicked() {
                    self.rd_start(mine, Some(o), false);
                }
            }
            let along_tip = if other.is_some() {
                "The other guitar plays (and the clicks), you play your part"
            } else {
                "Just the clicks and the cursor - you play it"
            };
            if retro_btn(ui, "PLAY ALONG", false).tip(along_tip).clicked() {
                self.rd_start(mine, other, true);
            }
            if lit_on && retro_btn(ui, "STOP", false).clicked() {
                self.player.send(Cmd::StopOnce);
                self.rd_play = None;
            }
        });
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "SHOW TAB", self.rd_tab).tip("Check yourself: where each note is played in this position").clicked()
            {
                self.rd_tab = !self.rd_tab;
            }
            if retro_btn(ui, "NOTE NAMES", self.rd_names).clicked() {
                self.rd_names = !self.rd_names;
            }
            // magnify the music (ctrl + the mouse wheel over the page does it too)
            ui.add_space(10.0);
            crate::views::label(ui, "ZOOM", 44.0);
            if retro_btn_w(ui, "-", 28.0, false).clicked() {
                self.rd_zoom = (self.rd_zoom - 0.25).max(0.75);
            }
            if retro_btn_w(ui, &format!("{}%", (self.rd_zoom * 100.0).round()), 56.0, false).tip("Back to 100%").clicked() {
                self.rd_zoom = 1.0;
            }
            if retro_btn_w(ui, "+", 28.0, false).clicked() {
                self.rd_zoom = (self.rd_zoom + 0.25).min(3.0);
            }
            ui.add_space(10.0);
            if retro_btn(ui, "THE POSITION", self.rd_map).tip("The key's notes in this position, on the neck").clicked() {
                self.rd_map = !self.rd_map;
            }
        });
        if self.rd_map {
            ui.add_space(4.0);
            position_map(ui, tonic_of(mine.sig), mine.pos, mine.sig > 0);
        }
        ui.add_space(6.0);
    }

    /// THE BOOK: lesson by lesson.
    fn rd_book_page(&mut self, ui: &mut egui::Ui) {
        if self.rd_book.is_empty() {
            para(ui, "Have William Leavitt's A Modern Method for Guitar, Volume 1? Your own copy can be worked through here lesson by lesson: every exercise, solo and duet written out bar for bar, played, and ticked off as you go. The book is kept on this computer only, in a file called reading_book.txt in Backline's folder; it is never part of Backline. No book? MADE-UP LINES works the same way with fresh lines.", pal().ink2);
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "SHOW THE FOLDER", false).clicked() {
                    crate::sources::reveal(&book_path().to_string_lossy());
                }
                if retro_btn(ui, "LOOK AGAIN", false).tip("Read the book file again").clicked() {
                    self.rd_reload();
                }
            });
            return;
        }
        let total: usize = self.rd_book.iter().map(|l| l.pieces.len()).sum();
        let id = |l: &Lesson, p: &Piece| format!("{}:{}", l.page, p.name);
        let done_n = self
            .rd_book
            .iter()
            .map(|l| l.pieces.iter().filter(|p| self.store.reading_done.contains_key(&id(l, p))).count())
            .sum::<usize>();
        // the lesson list, a mark on the ones finished
        self.rd_lesson = self.rd_lesson.min(self.rd_book.len() - 1);
        let names: Vec<String> = self
            .rd_book
            .iter()
            .map(|l| {
                let all = !l.pieces.is_empty() && l.pieces.iter().all(|p| self.store.reading_done.contains_key(&id(l, p)));
                format!("P.{}  {}{}", l.page, l.title.to_uppercase(), if all { "  - DONE" } else { "" })
            })
            .collect();
        // chapters (in book order), and the lessons of the one this lesson is in
        let mut chapters: Vec<String> = Vec::new();
        for l in &self.rd_book {
            if !chapters.contains(&l.chapter) {
                chapters.push(l.chapter.clone());
            }
        }
        let cur_ch = self.rd_book[self.rd_lesson].chapter.clone();
        let in_ch: Vec<usize> = (0..self.rd_book.len()).filter(|i| self.rd_book[*i].chapter == cur_ch).collect();
        let ch_names: Vec<String> = chapters
            .iter()
            .map(|c| {
                let ls: Vec<&Lesson> = self.rd_book.iter().filter(|l| &l.chapter == c).collect();
                let (a, b) = (ls.first().map_or(0, |l| l.page), ls.last().map_or(0, |l| l.page));
                format!("P.{}-{}  {}", a, b, c.to_uppercase())
            })
            .collect();
        let ch_names: Vec<&str> = ch_names.iter().map(|s| s.as_str()).collect();
        let lesson_names: Vec<&str> = in_ch.iter().map(|i| names[*i].as_str()).collect();
        ui.horizontal_wrapped(|ui| {
            if chapters.len() > 1 {
                let ci = chapters.iter().position(|c| *c == cur_ch).unwrap_or(0);
                if let Some(c) = dropdown(ui, "rd_chapter", "", &ch_names, ci, 300.0) {
                    if let Some(first) = self.rd_book.iter().position(|l| l.chapter == chapters[c]) {
                        self.rd_lesson = first;
                        self.rd_piece = 0;
                        self.rd_part = 0;
                    }
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if retro_btn_w(ui, "<", 30.0, false).tip("The lesson before").clicked() && self.rd_lesson > 0 {
                self.rd_lesson -= 1;
                self.rd_piece = 0;
            }
            let li = in_ch.iter().position(|i| *i == self.rd_lesson).unwrap_or(0);
            if let Some(i) = dropdown(ui, "rd_lesson", "", &lesson_names, li, 380.0) {
                self.rd_lesson = in_ch[i];
                self.rd_piece = 0;
                self.rd_part = 0;
            }
            if retro_btn_w(ui, ">", 30.0, false).tip("The next lesson").clicked() && self.rd_lesson + 1 < self.rd_book.len() {
                self.rd_lesson += 1;
                self.rd_piece = 0;
            }
            ui.add_space(10.0);
            crate::views::label(ui, &format!("{} OF {} DONE", done_n, total), 120.0);
            if retro_btn(ui, "PICK UP WHERE I LEFT OFF", false).tip("The first exercise not done yet").clicked() {
                'find: for (li, l) in self.rd_book.iter().enumerate() {
                    for (pi, p) in l.pieces.iter().enumerate() {
                        if !self.store.reading_done.contains_key(&id(l, p)) {
                            self.rd_lesson = li;
                            self.rd_piece = pi;
                            self.rd_part = 0;
                            break 'find;
                        }
                    }
                }
            }
            if retro_btn(ui, "RELOAD", false).tip("Read the book file again (after pages are added)").clicked() {
                self.rd_reload();
            }
        });
        for e in &self.rd_errs {
            crate::views::dim_line(ui, &format!("BOOK FILE: {}", e), 1.0, pal().red);
        }
        let lesson = self.rd_book[self.rd_lesson].clone();
        ui.add_space(4.0);
        section_header(ui, &format!("PAGE {}  -  {}", lesson.page, lesson.title.to_uppercase()));
        for n in &lesson.notes {
            para(ui, n, pal().ink2);
        }
        if lesson.pieces.is_empty() {
            para(ui, "Work this page from your book.", pal().dim);
            return;
        }
        self.rd_piece = self.rd_piece.min(lesson.pieces.len() - 1);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            for (i, p) in lesson.pieces.iter().enumerate() {
                let done = self.store.reading_done.contains_key(&id(&lesson, p));
                let label = format!("{}{}", p.name.to_uppercase(), if done { " *" } else { "" });
                if retro_btn(ui, &label, self.rd_piece == i).tip(if done { "Done (*)" } else { "Not done yet" }).clicked() {
                    self.rd_piece = i;
                    self.rd_part = 0;
                    self.player.send(Cmd::StopOnce);
                    self.rd_play = None;
                }
            }
        });
        let piece = lesson.pieces[self.rd_piece].clone();
        self.rd_part = self.rd_part.min(piece.parts.len() - 1);
        if piece.parts.len() > 1 {
            ui.horizontal_wrapped(|ui| {
                crate::views::label(ui, "YOUR PART", 100.0);
                for (i, (pn, _)) in piece.parts.iter().enumerate() {
                    if retro_btn(ui, &pn.to_uppercase(), self.rd_part == i).clicked() {
                        self.rd_part = i;
                    }
                }
                ui.add_space(10.0);
                if retro_btn(ui, "BOTH STAVES", self.rd_both).tip("The whole duet as systems (off: just your part)").clicked() {
                    self.rd_both = !self.rd_both;
                }
            });
        }
        if self.rd_tempo_for != (self.rd_lesson, self.rd_piece) {
            self.rd_tempo_for = (self.rd_lesson, self.rd_piece);
            self.rd_bpm = lesson.tempo.max(40);
        }
        let mine = piece.parts[self.rd_part].1.clone();
        let other = (piece.parts.len() > 1).then(|| piece.parts[(self.rd_part + 1) % piece.parts.len()].1.clone());
        let key = id(&lesson, &piece);
        ui.add_space(4.0);
        self.rd_controls(ui, &mine, other.as_ref());
        let lit = self.rd_lit(ui);
        if piece.parts.len() > 1 && self.rd_both {
            // the whole duet as systems, your part marked
            let names: Vec<String> = piece
                .parts
                .iter()
                .enumerate()
                .map(|(i, (pn, _))| if i == self.rd_part { format!("{} (YOU)", pn) } else { pn.clone() })
                .collect();
            let parts: Vec<(&Reading, Option<usize>, &str)> = piece
                .parts
                .iter()
                .enumerate()
                .map(|(i, (_, r))| (r, if i == self.rd_part { lit } else { None }, names[i].as_str()))
                .collect();
            draw_score(ui, &parts, self.rd_tab, self.rd_names, self.rd_zoom);
        } else {
            draw_reading(ui, &mine, lit, self.rd_tab, self.rd_names, self.rd_zoom);
        }
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let done = self.store.reading_done.get(&key).cloned();
            match &done {
                Some(d) => {
                    if retro_btn(ui, &format!("DONE {}", d), true).tip("Click to take the mark off").clicked() {
                        self.store.reading_done.remove(&key);
                        self.store_dirty = true;
                    }
                }
                None => {
                    if retro_btn(ui, "MARK IT DONE", false).tip("You can read it through at tempo without stopping").clicked() {
                        self.store.reading_done.insert(key.clone(), crate::store::today_str());
                        self.store_dirty = true;
                    }
                }
            }
            if retro_btn(ui, "NEXT", false).tip("The next exercise").clicked() {
                self.player.send(Cmd::StopOnce);
                self.rd_play = None;
                if self.rd_piece + 1 < lesson.pieces.len() {
                    self.rd_piece += 1;
                } else if self.rd_lesson + 1 < self.rd_book.len() {
                    self.rd_lesson += 1;
                    self.rd_piece = 0;
                }
                self.rd_part = 0;
            }
        });
    }

    /// MADE-UP LINES: fresh lines in a key, position and rhythm level.
    fn rd_made_up(&mut self, ui: &mut egui::Ui) {
        para(ui, "Read in one position, in one key, without looking at the neck. Pick the position, the key and how hard the rhythms get, then press NEW LINE. Every line is new.", pal().ink2);
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            let keys: Vec<&str> = KEYS.iter().map(|k| k.0).collect();
            if let Some(i) = dropdown(ui, "rd_key", "", &keys, self.rd_key, 150.0) {
                self.rd_key = i;
                changed = true;
            }
            let pos: Vec<&str> = POSITIONS.iter().map(|p| p.0).collect();
            if let Some(i) = dropdown(ui, "rd_pos", "", &pos, self.rd_pos, 170.0) {
                self.rd_pos = i;
                changed = true;
            }
            if let Some(i) = dropdown(ui, "rd_lvl", "RHYTHMS", &LEVELS, self.rd_level, 280.0) {
                self.rd_level = i;
                changed = true;
            }
            let bars: Vec<String> = BARS.iter().map(|b| format!("{} BARS", b)).collect();
            let bars: Vec<&str> = bars.iter().map(|s| s.as_str()).collect();
            if let Some(i) = dropdown(ui, "rd_bars", "", &bars, self.rd_bars, 110.0) {
                self.rd_bars = i;
                changed = true;
            }
            if let Some(i) = dropdown(ui, "rd_time", "TIME", &["4/4", "3/4"], self.rd_time, 120.0) {
                self.rd_time = i;
                changed = true;
            }
        });
        if changed || self.rd_line.is_none() {
            self.rd_new_line();
        }
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "NEW LINE", false).tip("A fresh line with these settings").clicked() {
                self.player.send(Cmd::StopOnce);
                self.rd_new_line();
            }
            let today = crate::store::today_str();
            let read = self.store.days.iter().find(|d| d.date == today).map_or(0, |d| d.reads);
            ui.add_space(10.0);
            crate::views::label(ui, &format!("LINES READ TODAY: {}", read), 150.0);
        });
        let Some(line) = self.rd_line.clone() else { return };
        self.rd_controls(ui, &line, None);
        let lit = self.rd_lit(ui);
        draw_reading(ui, &line, lit, self.rd_tab, self.rd_names, self.rd_zoom);
        crate::views::dim_line(ui, "PLAY ALONG TO THE END TO COUNT A LINE", 1.0, pal().dim);
    }

    fn rd_new_line(&mut self) {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
        let beats = if self.rd_time == 1 { 3 } else { 4 };
        self.rd_line = Some(make_line(seed, self.rd_key, POSITIONS[self.rd_pos].1, self.rd_level, BARS[self.rd_bars], beats));
        self.rd_play = None;
    }

    /// Start a play-through: this part (and the other guitar's, when given), or only the clicks and the other part
    /// when `along`; a bar to count in.
    fn rd_start(&mut self, mine: &Reading, other: Option<&Reading>, along: bool) {
        self.rd_start_at(mine, other, along, 0.0, true);
    }

    /// Start a play-through from beat `from` (in the order played), counted in or not.
    fn rd_start_at(&mut self, mine: &Reading, other: Option<&Reading>, along: bool, from: f32, count_in: bool) {
        let spb = 60.0 / self.rd_bpm.max(30) as f32;
        let order: Vec<(usize, f32, f32, Vec<i32>)> = play_order(mine).into_iter().filter(|n| n.1 >= from - 1e-3).collect();
        let shift = |v: &[(usize, f32, f32, Vec<i32>)]| -> Vec<(Vec<i32>, f32, f32)> {
            v.iter().filter(|n| n.1 >= from - 1e-3).map(|(_, at, d, m)| (m.clone(), (at - from) * spb, d * spb)).collect()
        };
        let mut notes: Vec<(Vec<i32>, f32, f32)> = Vec::new();
        if !along {
            notes.extend(shift(&order));
        }
        if let Some(o) = other {
            notes.extend(shift(&play_order(o)));
        }
        let count = if count_in { mine.beats as u32 } else { 0 };
        let lead = count as f32 * spb;
        let total = lead + order.iter().map(|(_, at, d, _)| (at + d - from) * spb).fold(0.0, f32::max);
        self.player.send(Cmd::StopOnce);
        self.player.send(Cmd::Once(
            crate::band::reading_sound(&notes, mine.beats as u32, count, from.round() as u32, spb, total - lead),
            0.9,
        ));
        self.rd_play =
            Some((Instant::now(), order.iter().map(|(gi, at, _, _)| (lead + (at - from) * spb, *gi)).collect(), along, total));
        self.rd_ctx = Some((mine.clone(), other.cloned(), along, from, lead, spb));
    }

    /// The tempo changed: a play-through carries on from the next beat at the new tempo.
    fn rd_retempo(&mut self) {
        self.rd_bpm_text = self.rd_bpm.to_string();
        let (Some((t0, ..)), Some((mine, other, along, from, lead, spb))) = (self.rd_play.clone(), self.rd_ctx.clone()) else {
            return;
        };
        let el = t0.elapsed().as_secs_f32();
        let at = if el < lead { from } else { (from + (el - lead) / spb).ceil() };
        self.rd_start_at(&mine, other.as_ref(), along, at, el < lead);
    }

    /// The note sounding now in a play-through; a play-along that reaches the end is counted in the diary.
    fn rd_lit(&mut self, ui: &egui::Ui) -> Option<usize> {
        let (t0, starts, along, total) = self.rd_play.clone()?;
        let el = t0.elapsed().as_secs_f32();
        if el > total + 0.3 {
            self.rd_play = None;
            if along {
                let date = crate::store::today_str();
                let d = self.store.day_mut(&date);
                d.reads += 1;
                *d.secs.entry("Sight reading".to_string()).or_insert(0) += total as u32;
                self.store_dirty = true;
            }
            return None;
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
        starts.iter().rev().find(|s| el >= s.0).map(|s| s.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every bar adds up, every note is in the key and playable in the position, and the line ends home.
    #[test]
    fn lines_add_up() {
        for level in 0..6 {
            for (key, pos) in [(0usize, 0u8), (1, 2), (3, 5), (4, 7), (9, 9)] {
                for beats in [3u16, 4] {
                    let r = make_line(level as u64 * 7919 + key as u64 * 31 + pos as u64 + 1, key, pos, level, 8, beats);
                    assert_eq!(r.bars.len(), 8);
                    for b in &r.bars {
                        assert_eq!(
                            b.notes.iter().map(|n| n.dur as u32).sum::<u32>(),
                            beats as u32 * BEAT as u32,
                            "a bar adds up"
                        );
                        for n in &b.notes {
                            assert!([3, 4, 6, 9, 12, 18, 24, 36, 48].contains(&n.dur), "odd length {}", n.dur);
                        }
                    }
                    let tonic = KEYS[key].1;
                    let notes: Vec<i32> =
                        r.bars.iter().flat_map(|b| b.notes.iter().flat_map(|n| n.heads.iter().map(|h| h.midi))).collect();
                    for m in &notes {
                        assert!([0, 2, 4, 5, 7, 9, 11].contains(&(m - tonic).rem_euclid(12)), "out of key");
                        assert!(finger_all(&[*m], pos)[0].is_some(), "not in the position");
                    }
                    assert_eq!((notes.last().unwrap() - tonic).rem_euclid(12), 0, "ends home");
                    assert!(!r.bars[0].notes[0].heads.is_empty(), "starts on a note");
                }
            }
        }
    }

    /// Marks after a note and words before it: pick strokes, fingers, string, accents, slurs, Ritard.
    #[test]
    fn marks_read() {
        let bars = parse_music("!mf !Ritard G4q{d,f3,s3} [C5E5]q.{u,f12,>,(} D5e{)} rh{ferm}").unwrap();
        let n = &bars[0].notes;
        assert_eq!(n.len(), 4);
        assert_eq!((n[0].marks.pick, n[0].marks.fingers.as_str(), n[0].marks.string), (1, "3", 3));
        assert_eq!(n[0].marks.text, vec!["mf".to_string(), "Ritard".to_string()]);
        assert!(n[1].marks.accent && n[1].marks.slur_start && n[1].marks.pick == -1 && n[1].dur == 18);
        assert!(n[2].marks.slur_end && n[3].marks.fermata && n[3].heads.is_empty());
        assert!(parse_music("G4q{zz}").is_err());
        let dbl = parse_music("Bbb4q Fx4q F##4q Ab4q").unwrap();
        let m: Vec<i32> = dbl[0].notes.iter().map(|n| n.heads[0].midi).collect();
        assert_eq!(m, vec![57, 55, 55, 56]);
    }

    /// The book file: lessons, settings that carry on, exercises and duets, chords, rests, ties, repeats.
    #[test]
    fn book_file_reads() {
        let text = "# test\n@lesson 4 Notes in the 1st Position\nkey C | pos 1 | time 4/4 | tempo 80\nnote Say the count.\n@ex Exercise 1\nC4q D4q E4q F4q | G4h [C4E4G4]h\n| rq E4e. F4s G4h~ | G4w\n@duet Sea to Sea\npart 1st Guitar: |: E4h G4h :| C5w\npart 2nd Guitar: C4w | C4w\n@lesson 15 Sharps and Flats\nkey G\n@ex Exercise 1\nF#4q Fn4q Bb4h\n";
        let (lessons, errs) = parse_book(text);
        assert!(errs.is_empty(), "{:?}", errs);
        assert_eq!(lessons.len(), 2);
        let l = &lessons[0];
        assert_eq!((l.page, l.title.as_str(), l.tempo), (4, "Notes in the 1st Position", 80));
        assert_eq!(l.notes, vec!["Say the count."]);
        assert_eq!(l.pieces.len(), 2);
        let ex = &l.pieces[0].parts[0].1;
        assert_eq!(ex.bars.len(), 4);
        // middle C as written sounds an octave down (MIDI 48)
        assert_eq!(ex.bars[0].notes[0].heads[0].midi, 48);
        assert_eq!(ex.bars[1].notes[1].heads.len(), 3);
        assert!(ex.bars[2].notes[0].heads.is_empty());
        assert_eq!(ex.bars[2].notes[1].dur, 9);
        assert!(ex.bars[2].notes[3].tie);
        // the tie joins G4h to G4w: one note, six beats
        let order = play_order(ex);
        assert_eq!(order.last().unwrap().2, 6.0);
        let duet = &l.pieces[1];
        assert_eq!(duet.parts.len(), 2);
        assert_eq!(duet.parts[0].0, "1st Guitar");
        assert!(duet.parts[0].1.bars[0].rep_start && duet.parts[0].1.bars[0].rep_end);
        // the repeat plays the first bar twice
        assert_eq!(play_order(&duet.parts[0].1).len(), 5);
        let sharp = &lessons[1].pieces[0].parts[0].1;
        assert_eq!(sharp.sig, 1);
        let h = &sharp.bars[0].notes;
        assert_eq!((h[0].heads[0].acc, h[1].heads[0].acc, h[2].heads[0].acc), (1, 0, -1));
        assert_eq!(h[0].heads[0].midi, h[1].heads[0].midi + 1);
    }
}

/// Your own book file, when there is one: every line has to read (run with --ignored to check it).
#[cfg(test)]
#[test]
#[ignore]
fn my_book_reads() {
    let text = std::fs::read_to_string(book_path()).expect("no reading_book.txt");
    let (lessons, errs) = parse_book(&text);
    assert!(errs.is_empty(), "{:#?}", errs);
    for l in &lessons {
        for p in &l.pieces {
            for (pn, r) in &p.parts {
                let bar = r.beats as u32 * r.beat_len() as u32;
                for (i, b) in r.bars.iter().enumerate() {
                    let n: u32 = b.notes.iter().map(|x| x.dur as u32).sum();
                    // a pickup bar at the start, or a short last bar, may be under length
                    assert!(
                        n == bar || i == 0 || i + 1 == r.bars.len(),
                        "page {} {} {} bar {}: {} of {}",
                        l.page,
                        p.name,
                        pn,
                        i + 1,
                        n,
                        bar
                    );
                }
            }
        }
        println!("page {} {}: {} pieces", l.page, l.title, l.pieces.len());
    }
}
