//! EAR (PRACTICE): ear training. One drill at a time, picked from one menu, most important first: chord
//! qualities, intervals, progressions in numerals, scale degrees in a key, guide-tone lines, scales and modes, the
//! bass note of a chord, and playing a short lick back. A question plays (as often as you like), you pick the
//! answer; 20 questions make a session. Each drill starts with four answers in play and adds more once you get 80%
//! in a session; the answers you miss come up more often. Your record per drill is kept in `store.ear`, and each
//! finished session's minutes go in the practice diary.

use super::*;
use crate::band;
use crate::lines;
use crate::store::EarStats;
use crate::theory;

/// Questions in a session.
pub(crate) const SESSION: u32 = 20;
/// How many answers a drill starts with.
const START: usize = 4;
/// What the practice diary calls this time.
const DIARY_LABEL: &str = "Ear training";

/// A drill: its name, its key in the store, the one-line explanation.
pub(crate) struct Drill {
    pub name: &'static str,
    pub key: &'static str,
    pub about: &'static str,
}

pub(crate) const DRILLS: [Drill; 8] = [
    Drill {
        name: "CHORD QUALITIES",
        key: "quality",
        about: "A chord plays. Name its quality: listen for the 3rd (major or minor) and the 7th (major, minor or diminished), then the colours on top.",
    },
    Drill {
        name: "INTERVALS",
        key: "interval",
        about: "Two notes play. Name the distance between them. Up, down, or both at once.",
    },
    Drill {
        name: "PROGRESSIONS",
        key: "prog",
        about: "A progression plays in a random key. Name it in numerals. Follow the bass, and hear where it lands.",
    },
    Drill {
        name: "SCALE DEGREES",
        key: "degree",
        about: "A cadence sets the key, then one note plays. Which degree of the key is it? Sing down to the root if you need to.",
    },
    Drill {
        name: "GUIDE TONES",
        key: "guide",
        about: "A ii-V-I plays with a line on top, then the line alone. Which chord tones does the line follow?",
    },
    Drill {
        name: "SCALES & MODES",
        key: "scale",
        about: "A scale plays up and down from its root. Name it.",
    },
    Drill {
        name: "BASS NOTE",
        key: "bass",
        about: "A 7th chord plays. Which of its notes is in the bass: the root, the 3rd, the 5th or the 7th?",
    },
    Drill {
        name: "PLAY IT BACK",
        key: "back",
        about: "The key's chord plays, then a short lick. Play it back on your guitar, then click its notes on the neck, in order.",
    },
];

// ------------------------------------------------------------------ the theory each drill asks about

/// A chord quality: the name shown (jazz symbols), the name Backline reads, and two voicings (semitones over the
/// root, low to high): close, and spread the way a guitarist plays it.
pub(crate) struct Quality {
    pub label: &'static str,
    /// (checked against the chord reader in the tests)
    #[allow(dead_code)]
    pub name: &'static str,
    pub close: &'static [i32],
    pub spread: &'static [i32],
}

pub(crate) const QUALITIES: [Quality; 11] = [
    Quality { label: "\u{0394}7", name: "maj7", close: &[0, 4, 7, 11], spread: &[0, 7, 11, 16] },
    Quality { label: "-7", name: "m7", close: &[0, 3, 7, 10], spread: &[0, 7, 10, 15] },
    Quality { label: "7", name: "7", close: &[0, 4, 7, 10], spread: &[0, 7, 10, 16] },
    Quality { label: "-7\u{266D}5", name: "m7b5", close: &[0, 3, 6, 10], spread: &[0, 6, 10, 15] },
    Quality { label: "\u{00b0}7", name: "dim7", close: &[0, 3, 6, 9], spread: &[0, 6, 9, 15] },
    // R 3 #5 b7 #9; spread: R b7 3 #5 #9 (the classic one on the 6th string)
    Quality { label: "7ALT", name: "7#5#9", close: &[0, 4, 8, 10, 15], spread: &[0, 10, 16, 20, 27] },
    // R 3 b7 9 13; spread: R b7 3 13 with the 9 on top
    Quality { label: "13", name: "13", close: &[0, 4, 10, 14, 21], spread: &[0, 10, 16, 21, 26] },
    Quality { label: "-6", name: "m6", close: &[0, 3, 7, 9], spread: &[0, 9, 15, 19] },
    Quality { label: "7SUS4", name: "7sus4", close: &[0, 5, 7, 10], spread: &[0, 7, 10, 17] },
    Quality { label: "-\u{0394}7", name: "m(maj7)", close: &[0, 3, 7, 11], spread: &[0, 7, 11, 15] },
    Quality { label: "6", name: "6", close: &[0, 4, 7, 9], spread: &[0, 9, 16, 19] },
];

/// Intervals, easiest to tell apart first: name, semitones.
pub(crate) const INTERVALS: [(&str, i32); 18] = [
    ("MAJOR 3RD", 4),
    ("MINOR 3RD", 3),
    ("PERFECT 5TH", 7),
    ("PERFECT 4TH", 5),
    ("OCTAVE", 12),
    ("MAJOR 2ND", 2),
    ("MINOR 7TH", 10),
    ("MAJOR 6TH", 9),
    ("MINOR 2ND", 1),
    ("MAJOR 7TH", 11),
    ("TRITONE", 6),
    ("MINOR 6TH", 8),
    ("\u{266D}9", 13),
    ("9", 14),
    ("#9", 15),
    ("#11", 18),
    ("\u{266D}13", 20),
    ("13", 21),
];

pub(crate) const DIRECTIONS: [&str; 4] = ["UP", "DOWN", "TOGETHER", "MIXED"];

const MAJ7: &[i32] = &[0, 4, 7, 11];
const MIN7: &[i32] = &[0, 3, 7, 10];
const DOM7: &[i32] = &[0, 4, 7, 10];
const HALF_DIM: &[i32] = &[0, 3, 6, 10];
/// V7(b9) in minor: R 3 b7 b9
const DOM_B9: &[i32] = &[0, 4, 10, 13];
const MIN6: &[i32] = &[0, 3, 7, 9];
/// a major triad with its root on top too (the cadence chords)
const TRIAD: &[i32] = &[0, 4, 7, 12];

/// A progression: its numerals, and each chord (semitones above the key, notes over its root, the symbol).
pub(crate) struct Prog {
    pub label: &'static str,
    pub chords: &'static [(i32, &'static [i32], &'static str)],
}

pub(crate) const PROGS: [Prog; 6] = [
    Prog { label: "ii-V-I", chords: &[(2, MIN7, "-7"), (7, DOM7, "7"), (0, MAJ7, "\u{0394}7")] },
    Prog { label: "ii-V-i", chords: &[(2, HALF_DIM, "-7\u{266D}5"), (7, DOM_B9, "7\u{266D}9"), (0, MIN6, "-6")] },
    Prog { label: "I-vi-ii-V", chords: &[(0, MAJ7, "\u{0394}7"), (9, MIN7, "-7"), (2, MIN7, "-7"), (7, DOM7, "7")] },
    Prog { label: "iii-VI-ii-V", chords: &[(4, MIN7, "-7"), (9, DOM7, "7"), (2, MIN7, "-7"), (7, DOM7, "7")] },
    Prog { label: "ii-\u{266D}II-I", chords: &[(2, MIN7, "-7"), (1, DOM7, "7"), (0, MAJ7, "\u{0394}7")] },
    Prog { label: "iv-\u{266D}VII-I", chords: &[(5, MIN7, "-7"), (10, DOM7, "7"), (0, MAJ7, "\u{0394}7")] },
];

/// Scale degrees, the ones that anchor a key first: name, semitones above the key.
pub(crate) const DEGREES: [(&str, i32); 12] = [
    ("1", 0),
    ("5", 7),
    ("3", 4),
    ("4", 5),
    ("2", 2),
    ("6", 9),
    ("7", 11),
    ("\u{266D}7", 10),
    ("\u{266D}3", 3),
    ("#4", 6),
    ("\u{266D}6", 8),
    ("\u{266D}2", 1),
];

/// The degree's name for any number of semitones above the key.
fn degree_label(semis: i32) -> &'static str {
    let s = semis.rem_euclid(12);
    DEGREES.iter().find(|d| d.1 == s).map_or("?", |d| d.0)
}

/// A guide-tone line through a ii-V-I: its name, minor or major, the line's note above each chord's root.
pub(crate) const GUIDES: [(&str, bool, [i32; 3]); 6] = [
    ("MAJOR 7-3-7", false, [10, 4, 11]),
    ("MAJOR 3-7-3", false, [3, 10, 4]),
    ("MINOR 7-3-7", true, [10, 4, 10]),
    ("MINOR 3-7-3", true, [3, 10, 3]),
    ("MAJOR 9-13-9", false, [2, 9, 2]),
    ("MAJOR 5-9-5", false, [7, 2, 7]),
];
/// The ii-V-I under a guide-tone line: (semitones above the key, notes over the root, symbol), major then minor.
const GUIDE_MAJOR: [(i32, &[i32], &str); 3] = [(2, MIN7, "-7"), (7, DOM7, "7"), (0, MAJ7, "\u{0394}7")];
const GUIDE_MINOR: [(i32, &[i32], &str); 3] = [(2, HALF_DIM, "-7\u{266D}5"), (7, DOM_B9, "7\u{266D}9"), (0, MIN7, "-7")];

/// Scales and modes: name, steps from the root.
pub(crate) const SCALES: [(&str, &[i32]); 16] = [
    ("IONIAN", &[0, 2, 4, 5, 7, 9, 11]),
    ("AEOLIAN", &[0, 2, 3, 5, 7, 8, 10]),
    ("DORIAN", &[0, 2, 3, 5, 7, 9, 10]),
    ("MIXOLYDIAN", &[0, 2, 4, 5, 7, 9, 10]),
    ("LYDIAN", &[0, 2, 4, 6, 7, 9, 11]),
    ("PHRYGIAN", &[0, 1, 3, 5, 7, 8, 10]),
    ("LOCRIAN", &[0, 1, 3, 5, 6, 8, 10]),
    ("MELODIC MINOR", &[0, 2, 3, 5, 7, 9, 11]),
    ("LYDIAN DOMINANT", &[0, 2, 4, 6, 7, 9, 10]),
    ("ALTERED", &[0, 1, 3, 4, 6, 8, 10]),
    ("WHOLE TONE", &[0, 2, 4, 6, 8, 10]),
    ("HALF-WHOLE", &[0, 1, 3, 4, 6, 7, 9, 10]),
    ("DORIAN \u{266D}2", &[0, 1, 3, 5, 7, 9, 10]),
    ("LYDIAN AUGMENTED", &[0, 2, 4, 6, 8, 9, 11]),
    ("MIXOLYDIAN \u{266D}6", &[0, 2, 4, 5, 7, 8, 10]),
    ("LOCRIAN \u{266E}9", &[0, 2, 3, 5, 6, 8, 10]),
];

pub(crate) const BASS: [&str; 4] = ["ROOT", "3RD", "5TH", "7TH"];
/// The chords the bass-note drill uses, with their symbols.
const BASS_CHORDS: [(&[i32], &str); 4] = [(MAJ7, "\u{0394}7"), (MIN7, "-7"), (DOM7, "7"), (HALF_DIM, "-7\u{266D}5")];

/// The answers a drill has, in the order they come into play.
pub(crate) fn answers(drill: usize) -> Vec<&'static str> {
    match drill {
        0 => QUALITIES.iter().map(|q| q.label).collect(),
        1 => INTERVALS.iter().map(|i| i.0).collect(),
        2 => PROGS.iter().map(|p| p.label).collect(),
        3 | 7 => DEGREES.iter().map(|d| d.0).collect(),
        4 => GUIDES.iter().map(|g| g.0).collect(),
        5 => SCALES.iter().map(|s| s.0).collect(),
        _ => BASS.to_vec(),
    }
}

// ------------------------------------------------------------------ questions

/// A small random number maker (xorshift): the same seed gives the same question, so an answer can be heard
/// "on the same root" and the tests can go through every case.
#[derive(Clone)]
pub(crate) struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng((seed ^ 0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn clock_seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7)
}

/// Something to play: from `at` seconds, these steps (each a chord or a note) `gap` seconds apart; clean ones are
/// short single notes (a line), the others ring.
#[derive(Clone, Debug)]
pub(crate) struct Part {
    pub at: f32,
    pub steps: Vec<Vec<i32>>,
    pub gap: f32,
    pub clean: bool,
}

/// A question: the drill, the right answer, the root or key it is built on and the seed for the rest (so another
/// answer can be built the same way), what plays, the arpeggio version (empty: none), and the lick to play back.
#[derive(Clone, Debug)]
pub(crate) struct Question {
    pub answer: usize,
    pub root: i32,
    pub seed: u64,
    pub opt: u8,
    pub parts: Vec<Part>,
    pub arp: Vec<Part>,
    /// PLAY IT BACK: the notes, and each one's place in DEGREES
    pub lick: Vec<i32>,
    pub lick_deg: Vec<usize>,
    /// what the answer says, spelt out once you have answered
    pub reveal: String,
}

/// A note name for jazz eyes (flat sign, not "b").
fn nm(pc: i32, sharps: bool) -> String {
    theory::note_name(pc, sharps).replace('b', "\u{266D}")
}

/// `pc` as the lowest note at or above `low`.
fn at_or_above(pc: i32, low: i32) -> i32 {
    low + (pc - low).rem_euclid(12)
}

/// Chords through a progression, as a guitarist would comp them: the root low in the bass (E2 to E\u{266D}3), the
/// other notes close together from about E3 up, each chord's inversion the one nearest the chord before.
pub(crate) fn voice_prog(key: i32, chords: &[(i32, &[i32])]) -> Vec<Vec<i32>> {
    let mut out: Vec<Vec<i32>> = Vec::new();
    let mut prev: Option<Vec<i32>> = None;
    for (deg, f) in chords {
        let root = (key + deg).rem_euclid(12);
        let bass = at_or_above(root, 40);
        let upper: Vec<i32> = f[1..].iter().map(|x| (root + x).rem_euclid(12)).collect();
        let n = upper.len();
        let mut best: Option<(i32, Vec<i32>)> = None;
        for start in 0..n {
            let mut v: Vec<i32> = Vec::new();
            let mut at = 51;
            for k in 0..n {
                let m = at_or_above(upper[(start + k) % n], at + 1);
                v.push(m);
                at = m;
            }
            let score = match &prev {
                Some(p) => v.iter().map(|m| p.iter().map(|q| (m - q).abs()).min().unwrap_or(0)).sum::<i32>(),
                None => (v[n - 1] - 67).abs(),
            };
            if best.as_ref().map_or(true, |b| score < b.0) {
                best = Some((score, v));
            }
        }
        let upper = best.map(|b| b.1).unwrap_or_default();
        prev = Some(upper.clone());
        let mut c = vec![bass];
        c.extend(upper);
        out.push(c);
    }
    out
}

/// A chord's notes one at a time, low to high (to hear it as an arpeggio).
fn arpeggio(chord: &[i32]) -> Vec<Part> {
    let mut v = chord.to_vec();
    v.sort();
    vec![Part { at: 0.0, steps: v.into_iter().map(|m| vec![m]).collect(), gap: 0.45, clean: false }]
}

/// The question for drill `drill` with this answer, on this root (a pitch class, or for intervals the low note),
/// the rest chosen by `seed`. `opt` is the intervals' direction; `level` the answers in play (PLAY IT BACK picks
/// its notes from them).
pub(crate) fn build(drill: usize, answer: usize, root: i32, seed: u64, opt: u8, level: usize) -> Question {
    let mut rng = Rng::new(seed);
    let mut q = Question {
        answer,
        root,
        seed,
        opt,
        parts: Vec::new(),
        arp: Vec::new(),
        lick: Vec::new(),
        lick_deg: Vec::new(),
        reveal: String::new(),
    };
    let one = |steps: Vec<Vec<i32>>, gap: f32, clean: bool| vec![Part { at: 0.0, steps, gap, clean }];
    match drill {
        // a chord, close or spread, its root somewhere comfortable
        0 => {
            let ql = &QUALITIES[answer.min(QUALITIES.len() - 1)];
            let spread = rng.below(2) == 1;
            let chord: Vec<i32> = if spread {
                let base = at_or_above(root, 40);
                ql.spread.iter().map(|x| base + x).collect()
            } else {
                let base = at_or_above(root, 48);
                ql.close.iter().map(|x| base + x).collect()
            };
            q.reveal = format!("{}{}  {}", nm(root, lines::key_sharps(root)), ql.label, if spread { "SPREAD" } else { "CLOSE" });
            q.arp = arpeggio(&chord);
            q.parts = one(vec![chord], 1.0, false);
        }
        // two notes, up, down or together
        1 => {
            let semis = INTERVALS[answer.min(INTERVALS.len() - 1)].1;
            let lo = root;
            let hi = lo + semis;
            let dir = if opt >= 3 { rng.below(3) as u8 } else { opt };
            let up = one(vec![vec![lo], vec![hi]], 1.0, false);
            let (a, b) = (nm(lo, false), nm(hi, false));
            match dir {
                0 => {
                    q.parts = up;
                    q.reveal = format!("{} UP TO {}", a, b);
                }
                1 => {
                    q.parts = one(vec![vec![hi], vec![lo]], 1.0, false);
                    q.reveal = format!("{} DOWN TO {}", b, a);
                }
                _ => {
                    q.parts = one(vec![vec![lo, hi]], 1.0, false);
                    q.arp = up;
                    q.reveal = format!("{} AND {}", a, b);
                }
            }
        }
        // a progression in a key
        2 => {
            let p = &PROGS[answer.min(PROGS.len() - 1)];
            let minor = p.label.ends_with('i') && !p.label.ends_with("I");
            let sharps = lines::key_sharps(if minor { root + 3 } else { root });
            let shape: Vec<(i32, &[i32])> = p.chords.iter().map(|c| (c.0, c.1)).collect();
            q.parts = one(voice_prog(root, &shape), 1.4, false);
            let names: Vec<String> = p.chords.iter().map(|c| format!("{}{}", nm(root + c.0, sharps), c.2)).collect();
            q.reveal = format!("IN {}{}:  {}", nm(root, sharps), if minor { " MINOR" } else { "" }, names.join("   "));
        }
        // a cadence, then one note
        3 => {
            let semis = DEGREES[answer.min(DEGREES.len() - 1)].1;
            let cad = voice_prog(root, &[(0, TRIAD), (5, TRIAD), (7, DOM7), (0, TRIAD)]);
            let note = at_or_above(root + semis, 60);
            let gap = 0.8;
            q.parts = vec![
                Part { at: 0.0, steps: cad, gap, clean: false },
                Part { at: 4.0 * gap + 0.7, steps: vec![vec![note]], gap: 1.6, clean: false },
            ];
            let sharps = lines::key_sharps(root);
            q.reveal = format!("KEY OF {}: THE NOTE WAS {}", nm(root, sharps), nm(note, sharps));
        }
        // a ii-V-I with a guide-tone line on top, then the line alone
        4 => {
            let (_, minor, line_deg) = GUIDES[answer.min(GUIDES.len() - 1)];
            let chords = if minor { GUIDE_MINOR } else { GUIDE_MAJOR };
            let shells: Vec<(i32, &[i32])> = chords.iter().map(|c| (c.0, c.1)).collect();
            let mut voiced = voice_prog(root, &shells);
            // the line: its first note from G4 up, then each note the nearest one (it steps or stays)
            let mut line: Vec<i32> = Vec::new();
            for (k, c) in chords.iter().enumerate() {
                let pc = (root + c.0 + line_deg[k]).rem_euclid(12);
                let m = match line.last() {
                    None => at_or_above(pc, 67),
                    Some(&p) => at_or_above(pc, p - 6),
                };
                line.push(m);
            }
            for (c, m) in voiced.iter_mut().zip(line.iter()) {
                c.push(*m);
            }
            let gap = 1.3;
            q.parts = vec![
                Part { at: 0.0, steps: voiced, gap, clean: false },
                Part { at: 3.0 * gap + 0.8, steps: line.iter().map(|m| vec![*m]).collect(), gap: 0.9, clean: false },
            ];
            let sharps = lines::key_sharps(if minor { root + 3 } else { root });
            let names: Vec<String> = chords.iter().map(|c| format!("{}{}", nm(root + c.0, sharps), c.2)).collect();
            let notes: Vec<String> = line.iter().map(|m| nm(*m, sharps)).collect();
            q.reveal = format!("{}   THE LINE: {}", names.join("  "), notes.join(" "));
        }
        // a scale up and down
        5 => {
            let (name, steps) = SCALES[answer.min(SCALES.len() - 1)];
            let base = at_or_above(root, 48);
            let mut notes: Vec<i32> = steps.iter().map(|s| base + s).collect();
            notes.push(base + 12);
            let down: Vec<i32> = notes.iter().rev().skip(1).copied().collect();
            notes.extend(down);
            q.parts = one(notes.into_iter().map(|m| vec![m]).collect(), 0.3, true);
            q.reveal = format!("{} {}", nm(root, lines::key_sharps(root)), name);
        }
        // a 7th chord with one of its notes in the bass
        6 => {
            let (f, sym) = BASS_CHORDS[rng.below(BASS_CHORDS.len())];
            let inv = answer.min(3);
            let bass = at_or_above(root + f[inv], 40);
            let mut chord = vec![bass];
            let mut at = (bass + 2).max(51);
            for k in 1..4 {
                let m = at_or_above(root + f[(inv + k) % 4], at + 1);
                chord.push(m);
                at = m;
            }
            let sharps = lines::key_sharps(root);
            q.reveal = if inv == 0 {
                format!("{}{}", nm(root, sharps), sym)
            } else {
                format!("{}{}/{}", nm(root, sharps), sym, nm(bass, sharps))
            };
            q.arp = arpeggio(&chord);
            q.parts = one(vec![chord], 1.0, false);
        }
        // the key's chord, then a short lick from the degrees in play
        _ => {
            let pool = level.clamp(START, DEGREES.len());
            let len = if pool <= 5 {
                3
            } else if pool <= 8 {
                4
            } else {
                5
            };
            let mut prev: Option<(usize, i32)> = None;
            for _ in 0..len {
                // a new degree each time (no note twice in a row)
                let mut d = rng.below(pool);
                if prev.is_some_and(|p| p.0 == d) {
                    d = (d + 1 + rng.below(pool - 1)) % pool;
                }
                let pc = root + DEGREES[d].1;
                let mut m = match prev {
                    None => at_or_above(pc, 60),
                    Some((_, p)) => at_or_above(pc, p - 6),
                };
                if m > 79 {
                    m -= 12;
                } else if m < 52 {
                    m += 12;
                }
                q.lick.push(m);
                q.lick_deg.push(d);
                prev = Some((d, m));
            }
            let tonic = voice_prog(root, &[(0, TRIAD)]);
            q.parts = vec![
                Part { at: 0.0, steps: tonic, gap: 1.6, clean: false },
                Part { at: 1.9, steps: q.lick.iter().map(|m| vec![*m]).collect(), gap: 0.6, clean: true },
            ];
            let sharps = lines::key_sharps(root);
            let notes: Vec<String> = q.lick.iter().map(|m| nm(*m, sharps)).collect();
            let degs: Vec<&str> = q.lick_deg.iter().map(|d| DEGREES[*d].0).collect();
            q.reveal = format!("KEY OF {}:  {}   ({})", nm(root, sharps), notes.join(" "), degs.join(" "));
        }
    }
    q
}

/// A new question: an answer from the ones in play (weighted by `weights`), a random root.
pub(crate) fn ask(drill: usize, level: usize, weights: &[f32], opt: u8, seed: u64) -> Question {
    let mut rng = Rng::new(seed);
    let total: f32 = weights.iter().sum();
    let mut x = rng.unit() * total;
    let mut answer = 0;
    for (i, w) in weights.iter().enumerate() {
        answer = i;
        if x < *w {
            break;
        }
        x -= w;
    }
    // a root anywhere in the octave; an interval's low note from E3 to C#4
    let root = if drill == 1 { 52 + rng.below(10) as i32 } else { rng.below(12) as i32 };
    build(drill, answer, root, rng.next(), opt, level)
}

/// How long a question takes to play, in seconds.
fn length(parts: &[Part]) -> f32 {
    parts.iter().map(|p| p.at + p.steps.len() as f32 * p.gap).fold(0.0, f32::max)
}

/// The parts mixed into one sound.
fn render(parts: &[Part]) -> Vec<i16> {
    let mut out: Vec<i32> = Vec::new();
    for p in parts {
        let pcm = if p.clean { band::line_sound(&p.steps, p.gap) } else { band::sequence_sound(&p.steps, p.gap) };
        let off = (p.at * band::RATE) as usize;
        if out.len() < off + pcm.len() {
            out.resize(off + pcm.len(), 0);
        }
        for (i, s) in pcm.iter().enumerate() {
            out[off + i] += *s as i32;
        }
    }
    out.into_iter().map(|s| s.clamp(i16::MIN as i32, i16::MAX as i32) as i16).collect()
}

/// How likely each answer in play is to come up: the ones you miss more often, the one just asked less.
fn weights(st: &EarStats, names: &[&str], last: Option<usize>) -> Vec<f32> {
    names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let asked = st.asked.get(*n).copied().unwrap_or(0) as f32;
            let right = st.right.get(*n).copied().unwrap_or(0) as f32;
            let miss = (asked - right).max(0.0) / (asked + 3.0);
            let w = 1.0 + 3.0 * miss;
            if last == Some(i) {
                w * 0.35
            } else {
                w
            }
        })
        .collect()
}

/// The answers in play for a drill with this record.
fn level_of(st: &EarStats, drill: usize) -> usize {
    let all = answers(drill).len();
    if st.level == 0 {
        START.min(all)
    } else {
        st.level.clamp(START.min(all), all)
    }
}

/// Right over asked in these sessions (from day `from`), as a share.
fn share_since(st: &EarStats, from: i64) -> Option<(u32, u32)> {
    let (r, n) = st.sessions.iter().filter(|s| s.0 >= from).fold((0, 0), |a, s| (a.0 + s.1, a.1 + s.2));
    (n > 0).then_some((r, n))
}

// ------------------------------------------------------------------ the page

/// Where you are on the EAR page.
#[derive(Default)]
pub(crate) struct Ear {
    pub drill: usize,
    /// intervals: up, down, together, mixed
    pub dir: u8,
    pub q: Option<Question>,
    /// the answer you picked for the question (None: not yet)
    pub given: Option<usize>,
    /// PLAY IT BACK: the notes you clicked, and whether they have been checked
    pub played: Vec<i32>,
    pub checked: bool,
    /// this session: asked, right, seconds spent (gaps over two minutes not counted), the last thing you did
    pub n: u32,
    pub right: u32,
    pub secs: f32,
    pub last: Option<Instant>,
    /// the session just finished: right, asked, minutes
    pub done: Option<(u32, u32, u32)>,
}

impl App {
    /// A test copy opened on a drill (BACKLINE_EAR=drill number) with a question asked and answered wrong.
    pub(crate) fn ear_demo(&mut self, drill: usize) {
        self.ear = Ear { drill: drill.min(DRILLS.len() - 1), ..Default::default() };
        let level = level_of(&self.ear_stats().clone(), self.ear.drill);
        let q = ask(self.ear.drill, level, &vec![1.0; level], 0, 11);
        if self.ear.drill == 7 {
            self.ear.played = q.lick.iter().map(|m| m + 2).collect();
            self.ear.checked = true;
        } else {
            self.ear.given = Some((q.answer + 1) % level);
        }
        self.ear.n = 6;
        self.ear.right = 4;
        self.ear.q = Some(q);
    }

    fn ear_stats(&mut self) -> &mut EarStats {
        let key = DRILLS[self.ear.drill.min(DRILLS.len() - 1)].key;
        self.store.ear.entry(key.to_string()).or_default()
    }

    /// Time on the page: what passed since you last did something, unless you walked away.
    fn ear_mark(&mut self) {
        let now = Instant::now();
        if let Some(l) = self.ear.last {
            let gap = now.duration_since(l).as_secs_f32();
            if gap <= 120.0 {
                self.ear.secs += gap;
            }
        }
        self.ear.last = Some(now);
    }

    /// Play these parts (a question, its arpeggio, an answer to compare).
    fn ear_play(&mut self, parts: &[Part]) {
        if parts.is_empty() {
            return;
        }
        self.player.send(Cmd::StopOnce);
        self.player.send(Cmd::Once(render(parts), 0.9));
        self.seq_play = Some((Instant::now(), length(parts), "ear".to_string(), 1));
    }

    /// The next question, played straight away.
    fn ear_next(&mut self) {
        if self.ear.done.is_some() {
            self.ear.done = None;
            self.ear.n = 0;
            self.ear.right = 0;
            self.ear.secs = 0.0;
        }
        let drill = self.ear.drill;
        let names = answers(drill);
        let last = self.ear.q.as_ref().map(|q| q.answer);
        let st = self.ear_stats().clone();
        let level = level_of(&st, drill);
        let w = if drill == 7 { vec![1.0] } else { weights(&st, &names[..level], last) };
        let q = ask(drill, level, &w, self.ear.dir, clock_seed());
        self.ear_play(&q.parts.clone());
        self.ear.q = Some(q);
        self.ear.given = None;
        self.ear.played.clear();
        self.ear.checked = false;
        self.ear_mark();
    }

    /// Note one answer in the drill's record (asked, given).
    fn ear_note(&mut self, asked: &str, given: &str) {
        let st = self.ear_stats();
        *st.asked.entry(asked.to_string()).or_insert(0) += 1;
        if asked == given {
            *st.right.entry(asked.to_string()).or_insert(0) += 1;
        } else {
            *st.mixups.entry(format!("{}>{}", asked, given)).or_insert(0) += 1;
        }
    }

    /// A question answered: right or not; the 20th ends the session.
    fn ear_scored(&mut self, ok: bool) {
        self.ear_mark();
        self.ear.n += 1;
        self.ear.right += ok as u32;
        if self.ear.n >= SESSION {
            let (right, n) = (self.ear.right, self.ear.n);
            let secs = self.ear.secs.round() as u32;
            let today = crate::store::today_days();
            let st = self.ear_stats();
            st.sessions.push((today, right, n));
            if st.sessions.len() > 400 {
                st.sessions.remove(0);
            }
            if secs > 0 {
                let date = crate::store::today_str();
                *self.store.day_mut(&date).secs.entry(DIARY_LABEL.to_string()).or_insert(0) += secs;
            }
            self.ear.done = Some((right, n, (secs + 30) / 60));
        }
        self.store_dirty = true;
    }

    /// EAR: the drill menu, the question (A), your scores (B).
    pub(crate) fn ear_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        self.seq_tick(ui.ctx());
        ui.add_space(6.0);
        title_line(ui, "EAR TRAINING", 3.0, pal().ink);
        ui.add_space(4.0);
        let drill = self.ear.drill.min(DRILLS.len() - 1);
        ui.horizontal_wrapped(|ui| {
            let names: Vec<&str> = DRILLS.iter().map(|d| d.name).collect();
            if let Some(i) = crate::dropdown(ui, "ear_drill", "DRILL", &names, drill, 0.0) {
                if i != drill {
                    self.ear = Ear { drill: i, dir: self.ear.dir, ..Default::default() };
                    acts.push(Action::StopSequence);
                }
            }
            if drill == 1 {
                if let Some(i) = crate::dropdown(ui, "ear_dir", "PLAYED", &DIRECTIONS, self.ear.dir as usize, 0.0) {
                    self.ear.dir = i as u8;
                }
            }
        });
        para(ui, DRILLS[drill].about, pal().ink2);
        ui.add_space(18.0);
        self.ear_question(ui, acts);
        ui.add_space(22.0);
        self.ear_scores(ui);
        ui.add_space(16.0);
    }

    /// Layout A: the big PLAY, where you are in the session, the answers, then hear again / arpeggio / next.
    fn ear_question(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let drill = self.ear.drill.min(DRILLS.len() - 1);
        let names = answers(drill);
        let level = level_of(&self.ear_stats().clone(), drill);
        let playing = self.seq_on("ear");
        let answered = self.ear.given.is_some() || self.ear.checked;
        let mut play = false;
        let mut next = false;
        let mut arp = false;
        let mut pick: Option<usize> = None;
        ui.vertical_centered(|ui| {
            // the big round PLAY (STOP while it sounds)
            let d = 96.0;
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(d), Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            let p = ui.painter();
            let c = r.center();
            let face = if resp.hovered() { pal().ink } else { pal().ink.gamma_multiply(0.88) };
            p.circle_filled(c, d / 2.0 - 3.0, face);
            p.circle_stroke(c, d / 2.0 - 1.0, Stroke::new(2.0_f32, pal().bar_txt));
            if playing {
                p.rect_filled(Rect::from_center_size(c, Vec2::splat(d * 0.26)), 3.0, pal().lcd);
            } else {
                let s = d * 0.2;
                let pts =
                    vec![Pos2::new(c.x - s * 0.7, c.y - s), Pos2::new(c.x + s * 1.1, c.y), Pos2::new(c.x - s * 0.7, c.y + s)];
                p.add(egui::Shape::convex_polygon(pts, pal().lcd, Stroke::NONE));
            }
            if resp.clicked() {
                if playing {
                    acts.push(Action::StopSequence);
                } else {
                    play = true;
                }
            }
            ui.add_space(10.0);
            let line = |ui: &mut egui::Ui, text: &str, px: f32, col: Color32| {
                let h = 7.0 * spx(px) + 8.0;
                let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
                ptext_fit(ui.painter(), r.center(), Align::Center, text, px, r.width() - 12.0, col);
            };
            line(ui, DRILLS[drill].name, 2.2, pal().ink);
            let shown = (self.ear.n + (!answered && self.ear.q.is_some() && self.ear.done.is_none()) as u32).clamp(1, SESSION);
            let at = if self.ear.q.is_none() {
                "PRESS PLAY TO START".to_string()
            } else {
                format!("QUESTION {} OF {}", shown, SESSION)
            };
            line(ui, &at, 1.0, pal().ink2);
            ui.add_space(14.0);

            if let Some(q) = self.ear.q.clone() {
                if drill == 7 {
                    self.ear_neck(ui, acts, &q);
                } else {
                    // the answers in play: after answering, the right one lit, a wrong pick marked; a click on any of
                    // them then plays it on the same root, to compare
                    let labels: Vec<&str> = names[..level].to_vec();
                    let given = self.ear.given;
                    if let Some(i) = centred_pills(ui, "ear_ans", &labels, |i| {
                        (answered && i == q.answer, answered && given == Some(i) && i != q.answer)
                    }) {
                        pick = Some(i);
                    }
                }
                ui.add_space(12.0);
                // what it was
                if answered {
                    let ok = if drill == 7 {
                        self.ear.played.len() == q.lick.len()
                            && self.ear.played.iter().zip(q.lick.iter()).all(|(a, b)| (a - b).rem_euclid(12) == 0)
                    } else {
                        self.ear.given == Some(q.answer)
                    };
                    let head = if ok {
                        "RIGHT".to_string()
                    } else if drill == 7 {
                        "NOT QUITE".to_string()
                    } else {
                        // (numerals keep their case, so the words around them are written in normal case too)
                        // (always written in normal case: "It was -7b5", never title-cased as a label)
                        format!("It was {}", names[q.answer])
                    };
                    line(ui, &head, 2.0, if ok { pal().ink } else { pal().red });
                    line(ui, &q.reveal, 1.0, pal().ink2);
                    if drill != 7 {
                        line(ui, "CLICK ANY ANSWER TO HEAR IT ON THE SAME ROOT", 1.0, pal().dim);
                    }
                    ui.add_space(8.0);
                }
                // hear again, as an arpeggio, next
                let mut row: Vec<&str> = vec!["HEAR AGAIN"];
                if !q.arp.is_empty() {
                    row.push("HEAR AS ARPEGGIO");
                }
                if answered {
                    row.push(if self.ear.done.is_some() { "NEW SESSION" } else { "NEXT" });
                }
                if let Some(i) = centred_pills(ui, "ear_row", &row, |_| (false, false)) {
                    match row[i] {
                        "HEAR AGAIN" => play = true,
                        "HEAR AS ARPEGGIO" => arp = true,
                        _ => next = true,
                    }
                }
                if let Some((right, n, mins)) = self.ear.done {
                    ui.add_space(10.0);
                    let pct = right * 100 / n.max(1);
                    line(ui, &format!("SESSION DONE: {} OF {} RIGHT ({}%)", right, n, pct), 2.0, pal().ink);
                    let more = if pct >= 80 && level < names.len() { "  -  READY FOR MORE ANSWERS" } else { "" };
                    line(ui, &format!("{} MIN IN YOUR PRACTICE DIARY{}", mins.max(1), more), 1.0, pal().ink2);
                }
            }
        });
        if play {
            match self.ear.q.clone() {
                Some(q) => {
                    self.ear_play(&q.parts);
                    self.ear_mark();
                }
                None => self.ear_next(),
            }
        }
        if arp {
            if let Some(q) = self.ear.q.clone() {
                self.ear_play(&q.arp);
                self.ear_mark();
            }
        }
        if next {
            self.ear_next();
        }
        if let (Some(i), Some(q)) = (pick, self.ear.q.clone()) {
            if self.ear.given.is_none() {
                self.ear.given = Some(i);
                self.ear_note(names[q.answer], names[i]);
                self.ear_scored(i == q.answer);
            } else {
                // the answer picked, built the same way on the same root
                let other = build(drill, i, q.root, q.seed, q.opt, level);
                self.ear_play(&other.parts);
            }
        }
    }

    /// PLAY IT BACK: click the notes you played on the neck, in order; they are checked once there are as many as
    /// in the lick.
    fn ear_neck(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, q: &Question) {
        let sharps = lines::key_sharps(q.root);
        let w = ui.available_width().min(760.0);
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 132.0), Sense::hover());
        let rect = Rect::from_min_max(r.min + Vec2::new(4.0, 4.0), r.max - Vec2::new(4.0, 18.0));
        let mut shape: [Option<u8>; 6] = [None; 6];
        if let Some(&m) = self.ear.played.last() {
            // the last note clicked, where it is
            if let Some(s) = (0..6).rev().find(|&s| m >= theory::STANDARD[s] && m - theory::STANDARD[s] <= 15) {
                shape[s] = Some((m - theory::STANDARD[s]) as u8);
            }
        }
        if let Some((s, f)) = crate::fretboard::neck(ui, rect, &shape, 15, !self.ear.checked, "ear_neck", None, &[], true) {
            let fret = f.or(shape[s]).unwrap_or(0) as i32;
            let m = theory::STANDARD[s] + fret;
            acts.push(Action::PlayNotes(vec![m]));
            if self.ear.played.len() < q.lick.len() {
                self.ear.played.push(m);
            }
            if self.ear.played.len() == q.lick.len() {
                self.ear_check(q);
            }
        }
        ui.add_space(6.0);
        let mut said: Vec<String> = self.ear.played.iter().map(|m| nm(*m, sharps)).collect();
        while said.len() < q.lick.len() {
            said.push("_".to_string());
        }
        let text = format!("KEY OF {}     YOUR NOTES:  {}", nm(q.root, sharps), said.join("  "));
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 7.0 * spx(1.5) + 8.0), Sense::hover());
        ptext_fit(ui.painter(), r.center(), Align::Center, &text, 1.5, r.width() - 12.0, pal().ink);
        if !self.ear.checked && !self.ear.played.is_empty() {
            ui.add_space(6.0);
            if let Some(i) = centred_pills(ui, "ear_undo", &["UNDO", "CLEAR"], |_| (false, false)) {
                if i == 0 {
                    self.ear.played.pop();
                } else {
                    self.ear.played.clear();
                }
            }
        }
    }

    /// PLAY IT BACK: each note you clicked against the lick's (any octave counts).
    fn ear_check(&mut self, q: &Question) {
        self.ear.checked = true;
        let mut all = true;
        for (k, d) in q.lick_deg.iter().enumerate() {
            let asked = DEGREES[*d].0;
            let given = self.ear.played.get(k).map_or("?", |p| degree_label(p - q.root));
            all &= asked == given;
            self.ear_note(asked, given);
        }
        self.ear_scored(all);
    }

    /// Layout B: this session and this week, the answers in play (and MORE ANSWERS), what you mix up most.
    fn ear_scores(&mut self, ui: &mut egui::Ui) {
        let drill = self.ear.drill.min(DRILLS.len() - 1);
        let names = answers(drill);
        let st = self.ear_stats().clone();
        let level = level_of(&st, drill);
        section_header(ui, "YOUR SCORE");
        ui.add_space(6.0);
        let today = crate::store::today_days();
        let pct = |r: u32, n: u32| if n == 0 { "-".to_string() } else { format!("{}%", r * 100 / n) };
        let session = if self.ear.done.is_some() { (0, 0) } else { (self.ear.right, self.ear.n) };
        let week = share_since(&st, today - 6).unwrap_or((0, 0));
        let week = (week.0 + session.0, week.1 + session.1);
        let in_play = if drill == 7 { "NOTES IN PLAY" } else { "ANSWERS IN PLAY" };
        let (sr, sn) = match self.ear.done {
            Some((r, n, _)) => (r, n),
            None => session,
        };
        let tiles = [
            (pct(sr, sn), "THIS SESSION".to_string()),
            (pct(week.0, week.1), "THIS WEEK".to_string()),
            (format!("{} OF {}", level, names.len()), in_play.to_string()),
        ];
        let tw = ((ui.available_width() - 20.0) / 3.0).clamp(96.0, 170.0);
        ui.horizontal_wrapped(|ui| {
            for (big, small) in tiles.iter() {
                let (r, _) = ui.allocate_exact_size(Vec2::new(tw, 62.0), Sense::hover());
                let p = ui.painter();
                p.rect_stroke(r.shrink(2.0), 6.0, Stroke::new(1.0_f32, pal().beige_dk));
                ptext_fit(p, Pos2::new(r.center().x, r.min.y + 24.0), Align::Center, big, 3.0, r.width() - 12.0, pal().ink);
                ptext_fit(p, Pos2::new(r.center().x, r.max.y - 13.0), Align::Center, small, 1.0, r.width() - 12.0, pal().dim);
            }
        });
        ui.add_space(6.0);
        // more answers once a session goes 80% or better (this one, ten questions in, or the last one finished)
        if level < names.len() {
            let now_ok = sn >= 10 && sr * 100 >= sn * 80;
            let last_ok = st.sessions.last().is_some_and(|s| s.2 > 0 && s.1 * 100 >= s.2 * 80);
            ui.horizontal_wrapped(|ui| {
                if now_ok || last_ok {
                    let next: Vec<&str> = names[level..(level + 2).min(names.len())].to_vec();
                    if retro_btn(ui, "MORE ANSWERS", false).tip(&format!("Adds {}", next.join(" and "))).clicked() {
                        let all = names.len();
                        let st = self.ear_stats();
                        st.level = (level + 2).min(all);
                        self.store_dirty = true;
                    }
                } else {
                    crate::views::dim_line(ui, "GET 80% IN A SESSION AND MORE ANSWERS COME INTO PLAY", 1.0, pal().dim);
                }
            });
        }
        if level > START.min(names.len()) {
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "FEWER ANSWERS", false).tip("Back to fewer answers").clicked() {
                    let st = self.ear_stats();
                    st.level = level.saturating_sub(2).max(START);
                    self.store_dirty = true;
                }
            });
        }
        // what you take for what, most often first
        ui.add_space(10.0);
        section_header(ui, "MIXED UP MOST");
        let mut mix: Vec<(&String, &u32)> = st.mixups.iter().filter(|m| *m.1 > 0).collect();
        mix.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        if mix.is_empty() {
            crate::views::dim_line(ui, "NOTHING YET. THE ANSWERS YOU MISS COME UP MORE OFTEN.", 1.0, pal().dim);
        } else {
            ui.horizontal_wrapped(|ui| {
                for (k, n) in mix.iter().take(4) {
                    let (a, b) = k.split_once('>').unwrap_or((k.as_str(), ""));
                    let text = format!("{} HEARD AS {}  x{}", a, b, n);
                    let w = text_w(&text, 1.5) + 24.0;
                    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::hover());
                    let p = ui.painter();
                    p.rect_filled(r, 14.0, pal().beige_dk.gamma_multiply(0.35));
                    p.rect_stroke(r, 14.0, Stroke::new(1.0_f32, pal().beige_dk));
                    ptext(p, r.center(), Align::Center, &text, 1.5, pal().ink2);
                }
            });
            crate::views::dim_line(ui, "THESE COME UP MORE OFTEN UNTIL YOU GET THEM.", 1.0, pal().dim);
        }
    }
}

/// Pills of one width, centred, as many rows as they need. `state(i)` says (lit, marked wrong). Returns the one
/// clicked.
fn centred_pills(ui: &mut egui::Ui, id: &str, labels: &[&str], state: impl Fn(usize) -> (bool, bool)) -> Option<usize> {
    let sc = crate::font::ui_scale();
    let w = labels.iter().map(|l| text_w(l, 2.0)).fold(0.0, f32::max) + 34.0;
    let gap = 10.0;
    let avail = ui.available_width();
    let per = (((avail + gap) / (w + gap)).floor() as usize).clamp(1, 6);
    let mut hit = None;
    let all: Vec<usize> = (0..labels.len()).collect();
    for (ri, row) in all.chunks(per).enumerate() {
        if ri > 0 {
            ui.add_space(8.0);
        }
        let roww = row.len() as f32 * w + (row.len() as f32 - 1.0) * gap;
        ui.push_id((id, ri), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.add_space(((avail - roww) / 2.0).max(0.0));
                for &i in row {
                    let (lit, wrong) = state(i);
                    let resp = retro_btn_w(ui, labels[i], w / sc, lit);
                    if wrong {
                        let rad = (resp.rect.height() / 2.0).min(14.0);
                        ui.painter().rect_stroke(resp.rect.expand(1.0), rad, Stroke::new(2.0_f32, pal().red));
                    }
                    if resp.clicked() {
                        hit = Some(i);
                    }
                }
            });
        });
    }
    hit
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pitch classes over a root, as a sorted set.
    fn set_over(notes: &[i32], root: i32) -> Vec<i32> {
        let mut v: Vec<i32> = notes.iter().map(|m| (m - root).rem_euclid(12)).collect();
        v.sort();
        v.dedup();
        v
    }

    fn all_notes(q: &Question) -> Vec<i32> {
        q.parts.iter().flat_map(|p| p.steps.iter().flatten().copied()).collect()
    }

    #[test]
    fn chord_qualities_are_what_they_say() {
        for (a, ql) in QUALITIES.iter().enumerate() {
            let want = set_over(ql.close, 0);
            assert_eq!(set_over(ql.spread, 0), want, "{}: close and spread differ", ql.label);
            // Backline's own chord reader agrees on the notes
            let read = theory::chord_notes(&format!("C{}", ql.name)).unwrap_or_else(|| panic!("C{} not read", ql.name));
            // (a voicing may leave the 5th out, as guitarists do on a 13)
            let mut read_set = set_over(&read.pcs, 0);
            if !want.contains(&7) {
                read_set.retain(|x| *x != 7);
            }
            assert_eq!(read_set, want, "{}", ql.label);
            for root in 0..12 {
                for seed in 0..8 {
                    let q = build(0, a, root, seed, 0, 99);
                    assert_eq!(q.parts.len(), 1);
                    let chord = &q.parts[0].steps[0];
                    assert_eq!(set_over(chord, root), want, "{} on {}", ql.label, root);
                    // the root in the bass, in a guitar's range
                    assert_eq!(chord.iter().min().unwrap().rem_euclid(12), root);
                    assert!(chord.iter().all(|m| (40..=84).contains(m)), "{:?}", chord);
                    assert_eq!(set_over(&q.arp[0].steps.concat(), root), want);
                }
            }
        }
        // the core four: 3rd and 7th
        assert_eq!(set_over(QUALITIES[0].close, 0), [0, 4, 7, 11]);
        assert_eq!(set_over(QUALITIES[1].close, 0), [0, 3, 7, 10]);
        assert_eq!(set_over(QUALITIES[2].close, 0), [0, 4, 7, 10]);
        assert_eq!(set_over(QUALITIES[3].close, 0), [0, 3, 6, 10]);
        assert_eq!(set_over(QUALITIES[4].close, 0), [0, 3, 6, 9]);
    }

    #[test]
    fn intervals_are_that_far_apart() {
        for (a, (name, semis)) in INTERVALS.iter().enumerate() {
            for lo in 52..62 {
                for dir in 0..4u8 {
                    for seed in 0..4 {
                        let q = build(1, a, lo, seed, dir, 99);
                        let steps = &q.parts[0].steps;
                        let notes: Vec<i32> = steps.concat();
                        assert_eq!(notes.len(), 2);
                        let (min, max) = (*notes.iter().min().unwrap(), *notes.iter().max().unwrap());
                        assert_eq!(max - min, *semis, "{}", name);
                        match dir {
                            0 => assert!(steps[1][0] - steps[0][0] == *semis),
                            1 => assert!(steps[0][0] - steps[1][0] == *semis),
                            2 => assert_eq!(steps.len(), 1),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// A numeral's semitones above the key and whether it is lower case (a minor chord).
    fn numeral(n: &str) -> (i32, bool) {
        let flat = n.starts_with('\u{266D}');
        let core = n.trim_start_matches('\u{266D}');
        let up = core.to_uppercase();
        let semis = match up.as_str() {
            "I" => 0,
            "II" => 2,
            "III" => 4,
            "IV" => 5,
            "V" => 7,
            "VI" => 9,
            "VII" => 11,
            _ => panic!("{}", n),
        };
        (semis - flat as i32, core.chars().all(|c| c.is_lowercase()))
    }

    #[test]
    fn progressions_play_their_numerals() {
        for (a, p) in PROGS.iter().enumerate() {
            let nums: Vec<&str> = p.label.split('-').collect();
            assert_eq!(nums.len(), p.chords.len(), "{}", p.label);
            for key in 0..12 {
                let q = build(2, a, key, 1, 0, 99);
                let chords = &q.parts[0].steps;
                assert_eq!(chords.len(), p.chords.len());
                for (k, c) in chords.iter().enumerate() {
                    let (semis, lower) = numeral(nums[k]);
                    let (deg, f, _) = p.chords[k];
                    assert_eq!(deg, semis, "{} chord {}", p.label, k);
                    let root = (key + deg).rem_euclid(12);
                    // the root in the bass, the chord's notes all there
                    assert_eq!(c[0].rem_euclid(12), root);
                    assert!(c[1..].iter().all(|m| *m > c[0]));
                    assert_eq!(set_over(c, root), set_over(f, 0), "{} chord {} in {}", p.label, k, key);
                    // lower case: a minor 3rd; upper case: a major 3rd
                    let third = if lower { 3 } else { 4 };
                    assert!(set_over(c, root).contains(&third), "{} chord {}", p.label, k);
                }
            }
        }
        // the dominants really are dominants, the tonics land home
        assert_eq!(PROGS[3].chords[1].1, DOM7);
        assert_eq!(PROGS[4].chords[1], (1, DOM7, "7"));
        assert_eq!(PROGS[5].chords[1], (10, DOM7, "7"));
    }

    #[test]
    fn voice_leading_stays_close() {
        let v = voice_prog(0, &[(2, MIN7), (7, DOM7), (0, MAJ7)]);
        for w in v.windows(2) {
            let moved: i32 = w[0][1..].iter().zip(w[1][1..].iter()).map(|(a, b)| (a - b).abs()).sum();
            assert!(moved <= 6, "{:?}", v);
        }
    }

    #[test]
    fn scale_degrees_follow_a_cadence() {
        for (a, (name, semis)) in DEGREES.iter().enumerate() {
            for key in 0..12 {
                let q = build(3, a, key, 3, 0, 99);
                let cad = &q.parts[0].steps;
                assert_eq!(cad.len(), 4);
                // I IV V7 I
                for (c, (deg, f)) in cad.iter().zip([(0, TRIAD), (5, TRIAD), (7, DOM7), (0, TRIAD)]) {
                    assert_eq!(set_over(c, key + deg), set_over(f, 0));
                    assert_eq!(c[0].rem_euclid(12), (key + deg).rem_euclid(12));
                }
                let note = q.parts[1].steps[0][0];
                assert_eq!((note - key).rem_euclid(12), *semis, "{}", name);
                assert_eq!(degree_label(note - key), *name);
                assert!(q.parts[1].at > q.parts[0].steps.len() as f32 * q.parts[0].gap);
            }
        }
    }

    #[test]
    fn guide_tone_lines_are_those_chord_tones() {
        for (a, (name, minor, degs)) in GUIDES.iter().enumerate() {
            let chords = if *minor { GUIDE_MINOR } else { GUIDE_MAJOR };
            for key in 0..12 {
                let q = build(4, a, key, 5, 0, 99);
                let line: Vec<i32> = q.parts[1].steps.concat();
                assert_eq!(line.len(), 3);
                for k in 0..3 {
                    let root = key + chords[k].0;
                    assert_eq!((line[k] - root).rem_euclid(12), degs[k].rem_euclid(12), "{} in {}", name, key);
                    // the line note is a note of its chord (or its 9th / 13th, named so)
                    let tone = (line[k] - root).rem_euclid(12);
                    if !name.contains('9') {
                        assert!(chords[k].1.iter().any(|x| x.rem_euclid(12) == tone), "{} {}", name, k);
                    }
                    // played on top of the chord too
                    let c = &q.parts[0].steps[k];
                    assert_eq!(*c.last().unwrap(), line[k]);
                    assert_eq!(c[0].rem_euclid(12), root.rem_euclid(12));
                }
                // it steps or stays
                assert!(line.windows(2).all(|w| (w[1] - w[0]).abs() <= 2), "{} {:?}", name, line);
            }
        }
        // 7 to 3 to 7 in C: C, B, B
        let q = build(4, 0, 0, 1, 0, 99);
        assert_eq!(q.parts[1].steps.concat().iter().map(|m| m % 12).collect::<Vec<_>>(), [0, 11, 11]);
        // minor 3 to 7 to 3 in C: F, F, Eb
        let q = build(4, 3, 0, 1, 0, 99);
        assert_eq!(q.parts[1].steps.concat().iter().map(|m| m % 12).collect::<Vec<_>>(), [5, 5, 3]);
    }

    #[test]
    fn scales_are_the_modes_they_say() {
        use crate::harmony::{mode_of, PARENTS};
        let names = |p: usize| PARENTS[p].modes;
        // the seven modes of major, and melodic minor's
        for (p, map) in [(0usize, ["IONIAN", "DORIAN", "PHRYGIAN", "LYDIAN", "MIXOLYDIAN", "AEOLIAN", "LOCRIAN"])] {
            for (d, n) in map.iter().enumerate() {
                let s = SCALES.iter().find(|s| s.0 == *n).unwrap();
                assert_eq!(s.1, mode_of(&PARENTS[p].steps, d).as_slice(), "{}", n);
                assert_eq!(names(p)[d], *n);
            }
        }
        for (d, n) in [
            (0, "MELODIC MINOR"),
            (1, "DORIAN \u{266D}2"),
            (2, "LYDIAN AUGMENTED"),
            (3, "LYDIAN DOMINANT"),
            (4, "MIXOLYDIAN \u{266D}6"),
            (5, "LOCRIAN \u{266E}9"),
            (6, "ALTERED"),
        ] {
            let s = SCALES.iter().find(|s| s.0 == n).unwrap();
            assert_eq!(s.1, mode_of(&PARENTS[1].steps, d).as_slice(), "{}", n);
        }
        let hw = SCALES.iter().find(|s| s.0 == "HALF-WHOLE").unwrap();
        assert_eq!(hw.1, theory::OTHER_SCALES[1].1);
        for (a, (name, steps)) in SCALES.iter().enumerate() {
            for root in 0..12 {
                let q = build(5, a, root, 2, 0, 99);
                let notes = all_notes(&q);
                let n = steps.len();
                assert_eq!(notes.len(), 2 * n + 1, "{}", name);
                assert_eq!(notes[0].rem_euclid(12), root);
                assert_eq!(notes[n] - notes[0], 12);
                assert!(notes[..=n].windows(2).all(|w| w[1] > w[0]));
                assert_eq!(set_over(&notes, root), set_over(steps, 0), "{}", name);
            }
        }
    }

    #[test]
    fn the_bass_note_is_the_one_asked() {
        for a in 0..4 {
            for root in 0..12 {
                for seed in 0..16 {
                    let q = build(6, a, root, seed, 0, 99);
                    let c = &q.parts[0].steps[0];
                    assert_eq!(c.len(), 4);
                    let low = *c.iter().min().unwrap();
                    assert_eq!(low, c[0]);
                    // which chord it is: the one whose notes these are
                    let (f, _) = BASS_CHORDS.iter().find(|b| set_over(c, root) == set_over(b.0, 0)).expect("a 7th chord");
                    assert_eq!((low - root).rem_euclid(12), f[a], "{} on {}", BASS[a], root);
                    assert!(c.iter().all(|m| (40..=76).contains(m)), "{:?}", c);
                }
            }
        }
    }

    #[test]
    fn licks_come_from_the_degrees_in_play() {
        for level in [4, 6, 9, 12] {
            for key in 0..12 {
                for seed in 0..20 {
                    let q = build(7, 0, key, seed, 0, level);
                    let want = if level <= 5 {
                        3
                    } else if level <= 8 {
                        4
                    } else {
                        5
                    };
                    assert_eq!(q.lick.len(), want);
                    assert_eq!(q.lick.len(), q.lick_deg.len());
                    for (m, d) in q.lick.iter().zip(q.lick_deg.iter()) {
                        assert!(*d < level);
                        assert_eq!((m - key).rem_euclid(12), DEGREES[*d].1);
                        assert!((52..=79).contains(m), "{}", m);
                    }
                    assert!(q.lick_deg.windows(2).all(|w| w[0] != w[1]));
                    assert_eq!(q.parts[1].steps.concat(), q.lick);
                }
            }
        }
    }

    #[test]
    fn weights_favour_what_you_miss() {
        let mut st = EarStats::default();
        st.asked.insert("-7".into(), 10);
        st.right.insert("-7".into(), 2);
        st.asked.insert("7".into(), 10);
        st.right.insert("7".into(), 10);
        let w = weights(&st, &["\u{0394}7", "-7", "7", "-7\u{266D}5"], Some(3));
        assert!(w[1] > w[0] && w[0] == w[2] && w[3] < w[0]);
        // every drill asks only answers it has
        for drill in 0..8 {
            let n = answers(drill).len();
            for seed in 0..50 {
                let level = START.min(n);
                let q = ask(drill, level, &vec![1.0; level], 3, seed);
                assert!(q.answer < level.max(1));
                assert!(!q.parts.is_empty() && !all_notes(&q).is_empty());
            }
        }
        assert_eq!(level_of(&EarStats::default(), 6), 4);
        assert_eq!(level_of(&EarStats { level: 99, ..Default::default() }, 0), QUALITIES.len());
    }
}
