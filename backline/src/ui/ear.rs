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
/// The store's record of the "what's on top?" answers.
const TOP_KEY: &str = "top";
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

// ------------------------------------------------------------------ CHORD QUALITIES: the chords

/// How a chord's notes are laid out: built from its tones the way the VOICING option says, or always its own way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Shape {
    Built,
    /// one of these (semitones over the root, low to high)
    Fixed(&'static [&'static [i32]]),
    /// a major triad this many semitones over the root, on top of the dominant's 3rd and 7th (an upper structure)
    Upper(i32),
    /// a major triad this many semitones over the root, over the root in the bass (a slash chord)
    Slash(i32),
}

/// A chord to name: the answer shown, its level (an index of LEVELS), its symbol over C for Backline's chord reader
/// ("?": none fits), its tones (semitones over the root; tensions written above the octave: 14 is the 9, 2 a sus2 or
/// added 2nd), the tones that always sound (empty: all), and how it is laid out.
pub(crate) struct Chord {
    pub label: &'static str,
    pub level: usize,
    pub name: &'static str,
    pub tones: &'static [i32],
    pub need: &'static [i32],
    pub shape: Shape,
}

const fn ch(label: &'static str, level: usize, name: &'static str, tones: &'static [i32], need: &'static [i32]) -> Chord {
    Chord { label, level, name, tones, need, shape: Shape::Built }
}
const fn sh(label: &'static str, name: &'static str, tones: &'static [i32], shape: Shape) -> Chord {
    Chord { label, level: 4, name, tones, need: &[], shape }
}
/// (a sound without its root: the root is the bass player's, there when BASS says)
const fn rl(label: &'static str, name: &'static str, tones: &'static [i32], need: &'static [i32], shape: Shape) -> Chord {
    Chord { label, level: 4, name, tones, need, shape }
}

pub(crate) const LEVELS: [&str; 5] = ["TRIADS", "7THS & 6THS", "EXTENSIONS", "ALTERED", "BANDSTAND"];

/// Every chord, level by level, in the order they come into play.
pub(crate) const CHORDS: [Chord; 51] = [
    // triads
    ch("MAJOR", 0, "", &[0, 4, 7], &[]),
    ch("MINOR", 0, "m", &[0, 3, 7], &[]),
    ch("DIM", 0, "dim", &[0, 3, 6], &[]),
    ch("AUG", 0, "aug", &[0, 4, 8], &[]),
    ch("SUS4", 0, "sus4", &[0, 5, 7], &[]),
    ch("SUS2", 0, "sus2", &[0, 2, 7], &[]),
    // sevenths and sixths: all four notes always sound (without its 5th a maj7 is a shell, a level up)
    ch("\u{0394}7", 1, "maj7", &[0, 4, 7, 11], &[]),
    ch("-7", 1, "m7", &[0, 3, 7, 10], &[]),
    ch("7", 1, "7", &[0, 4, 7, 10], &[]),
    ch("-7\u{266D}5", 1, "m7b5", &[0, 3, 6, 10], &[]),
    ch("\u{00b0}7", 1, "dim7", &[0, 3, 6, 9], &[]),
    ch("6", 1, "6", &[0, 4, 7, 9], &[]),
    ch("-6", 1, "m6", &[0, 3, 7, 9], &[]),
    ch("-\u{0394}7", 1, "m(maj7)", &[0, 3, 7, 11], &[]),
    ch("7SUS4", 1, "7sus4", &[0, 5, 7, 10], &[]),
    ch("\u{0394}7#5", 1, "maj7#5", &[0, 4, 8, 11], &[]),
    // extensions: the 3rd, the 7th (or 6th) and the colour always; the root and 5th as the voicing allows
    ch("\u{0394}9", 2, "maj9", &[0, 4, 7, 11, 14], &[4, 11, 14]),
    ch("-9", 2, "m9", &[0, 3, 7, 10, 14], &[3, 10, 14]),
    ch("9", 2, "9", &[0, 4, 7, 10, 14], &[4, 10, 14]),
    ch("13", 2, "13", &[0, 4, 7, 10, 14, 21], &[4, 10, 21]),
    ch("6/9", 2, "69", &[0, 4, 7, 9, 14], &[4, 9, 14]),
    ch("-11", 2, "m11", &[0, 3, 7, 10, 14, 17], &[3, 10, 17]),
    // (its 5th always: without it, it is the F/G slash sound)
    ch("9SUS4", 2, "9sus4", &[0, 5, 7, 10, 14], &[5, 7, 10, 14]),
    ch("\u{0394}7#11", 2, "maj7#11", &[0, 4, 7, 11, 18], &[4, 11, 18]),
    ch("-6/9", 2, "m69", &[0, 3, 7, 9, 14], &[3, 9, 14]),
    // altered dominants
    ch("7\u{266D}9", 3, "7b9", &[0, 4, 7, 10, 13], &[4, 10, 13]),
    ch("7#9", 3, "7#9", &[0, 4, 7, 10, 15], &[4, 10, 15]),
    // (the natural 5th always, or it is a 7b5)
    ch("7#11", 3, "7#11", &[0, 4, 7, 10, 18], &[4, 7, 10, 18]),
    // (the natural 5th always, or it is a 7#5)
    ch("7\u{266D}13", 3, "7b13", &[0, 4, 7, 10, 20], &[4, 7, 10, 20]),
    ch("13\u{266D}9", 3, "13b9", &[0, 4, 7, 10, 13, 21], &[4, 10, 13, 21]),
    ch("13#11", 3, "13#11", &[0, 4, 7, 10, 14, 18, 21], &[4, 10, 18, 21]),
    ch("7#5", 3, "7#5", &[0, 4, 8, 10], &[]),
    ch("7\u{266D}9\u{266D}13", 3, "7b9b13", &[0, 4, 10, 13, 20], &[4, 10, 13, 20]),
    // the altered voicing guitarists play: R 3 b7 #9 b13 (the reader spells it 7#5#9)
    ch("7ALT", 3, "7#5#9", &[0, 4, 10, 15, 20], &[4, 10, 15, 20]),
    ch("7\u{266D}5", 3, "7b5", &[0, 4, 6, 10], &[]),
    // bandstand sounds: shells
    sh("\u{0394}7 SHELL", "maj7", &[0, 4, 11], Shape::Fixed(&[&[0, 4, 11], &[0, 11, 16]])),
    sh("-7 SHELL", "m7", &[0, 3, 10], Shape::Fixed(&[&[0, 3, 10], &[0, 10, 15]])),
    sh("7 SHELL", "7", &[0, 4, 10], Shape::Fixed(&[&[0, 4, 10], &[0, 10, 16]])),
    // fourths with a major 3rd on top (E A D G B over E): a minor 11th
    sh("SO WHAT", "m11", &[0, 3, 7, 10, 17], Shape::Fixed(&[&[0, 5, 10, 15, 19]])),
    // rootless A (3rd at the bottom) and B (7th at the bottom) voicings, named for the chord they imply
    rl("9 ROOTLESS", "9", &[0, 4, 7, 10, 14], &[4, 7, 10, 14], Shape::Fixed(&[&[4, 7, 10, 14], &[10, 14, 16, 19]])),
    rl("-9 ROOTLESS", "m9", &[0, 3, 7, 10, 14], &[3, 7, 10, 14], Shape::Fixed(&[&[3, 7, 10, 14], &[10, 14, 15, 19]])),
    rl("\u{0394}9 ROOTLESS", "maj9", &[0, 4, 7, 11, 14], &[4, 7, 11, 14], Shape::Fixed(&[&[4, 7, 11, 14], &[11, 14, 16, 19]])),
    rl("13 ROOTLESS", "13", &[0, 4, 9, 10, 14], &[4, 9, 10, 14], Shape::Fixed(&[&[4, 9, 10, 14], &[10, 14, 16, 21]])),
    // upper-structure triads over a dominant: D/C7 = C13#11, Eb/C7 = C7#9, Ab/C7 = C7#5#9 (alt), F#/C7 = C7b9#11
    rl("II/7", "13#11", &[0, 4, 10, 14, 18, 21], &[4, 10, 14, 18, 21], Shape::Upper(2)),
    rl("\u{266D}III/7", "7#9", &[0, 4, 7, 10, 15], &[4, 7, 10, 15], Shape::Upper(3)),
    rl("\u{266D}VI/7", "7#5#9", &[0, 4, 10, 15, 20], &[4, 10, 15, 20], Shape::Upper(8)),
    rl("#IV/7", "7b9#11", &[0, 4, 10, 13, 18], &[4, 10, 13, 18], Shape::Upper(6)),
    // slash chords: F/G = G9sus (no 5th), D/C = C lydian (no 3rd)
    sh("\u{266D}VII/1", "9sus4", &[0, 10, 14, 17], Shape::Slash(10)),
    sh("II/1", "?", &[0, 14, 18, 21], Shape::Slash(2)),
    // clusters
    sh("1 2 3 5", "add9", &[0, 2, 4, 7], Shape::Fixed(&[&[0, 2, 4, 7]])),
    sh("1 2 \u{266D}3 5", "madd9", &[0, 2, 3, 7], Shape::Fixed(&[&[0, 2, 3, 7]])),
];

/// The chords of a level, in the order they come into play.
pub(crate) fn level_chords(level: usize) -> Vec<usize> {
    (0..CHORDS.len()).filter(|i| CHORDS[*i].level == level).collect()
}

/// Look-alikes: chords that are easy to take for one another, drilled against each other.
pub(crate) const ALIKES: [&[&str]; 9] = [
    &["\u{0394}7", "6", "\u{0394}9", "6/9"],
    &["-7", "-6", "-\u{0394}7", "-9"],
    &["7\u{266D}9", "7#9", "13\u{266D}9", "7ALT"],
    &["7#11", "7\u{266D}5", "13#11"],
    &["-7\u{266D}5", "\u{00b0}7", "-6"],
    &["AUG", "\u{0394}7#5", "7#5"],
    &["7SUS4", "9SUS4", "\u{266D}VII/1"],
    &["9", "13", "7"],
    &["-9", "-11", "-7"],
];
/// What the menu calls YOUR MIX-UPS (the set after the last of ALIKES).
const MIXUPS_SET: &str = "YOUR MIX-UPS";

pub(crate) const MODES: [&str; 3] = ["LEVELS", "LOOK-ALIKES", "MY SET"];
pub(crate) const VOICINGS: [&str; 6] = ["CLOSE", "SPREAD", "DROP 2", "DROP 3", "ROOTLESS", "MIXED"];
const ROOTS: [&str; 2] = ["ANY ROOT", "SAME ROOT"];
const BASSES: [&str; 2] = ["ROOT IN BASS", "NO BASS"];

/// A chord by its answer label.
pub(crate) fn chord_by_label(label: &str) -> Option<usize> {
    CHORDS.iter().position(|c| c.label == label)
}

/// A chord's pitch classes over its root, sorted.
fn pcs_of(c: &Chord) -> Vec<i32> {
    let mut v: Vec<i32> = c.tones.iter().map(|t| t.rem_euclid(12)).collect();
    v.sort();
    v.dedup();
    v
}

/// The tones that always sound (as pitch classes).
fn needs_of(c: &Chord) -> Vec<i32> {
    let n = if c.need.is_empty() { c.tones } else { c.need };
    n.iter().map(|t| t.rem_euclid(12)).collect()
}

/// These notes (pitch classes over the root) spell this chord too: nothing it doesn't have, everything it needs.
/// (So a played 13#11 is also right called II/7, its upper structure, which has the same notes.)
pub(crate) fn fits(c: &Chord, played: &[i32]) -> bool {
    let all = pcs_of(c);
    played.iter().all(|p| all.contains(&p.rem_euclid(12))) && needs_of(c).iter().all(|n| played.contains(n))
}

/// A tone's name in the chord: R 3 5 b7 9 #11 13 ... (a diminished 7th's 6th is its bb7).
pub(crate) fn tone_label(c: &Chord, t: i32) -> &'static str {
    match t {
        0 | 12 => "R",
        1 | 13 => "\u{266D}9",
        2 => "2",
        14 => "9",
        3 => "\u{266D}3",
        15 => "#9",
        4 | 16 => "3",
        5 => "4",
        17 => "11",
        6 => "\u{266D}5",
        18 => "#11",
        7 | 19 => "5",
        8 => "#5",
        20 => "\u{266D}13",
        9 if c.name == "dim7" => "\u{266D}\u{266D}7",
        9 => "6",
        21 => "13",
        10 | 22 => "\u{266D}7",
        11 | 23 => "7",
        _ => "?",
    }
}

/// The chord tone with this pitch class (over the root), as the chord writes it.
fn tone_of(c: &Chord, pc: i32) -> Option<i32> {
    c.tones.iter().copied().find(|t| t.rem_euclid(12) == pc.rem_euclid(12))
}

/// How the CHORD QUALITIES question is played: the voicing (an index of VOICINGS), no bass note under it, a ii-V
/// before it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ChordOpts {
    pub voicing: u8,
    pub no_bass: bool,
    pub context: bool,
}

/// The four (or three) notes a chord is voiced with, as pitch classes over the root: a triad or a 7th chord as it
/// is; a bigger one keeps the notes it needs, the root if `rooted` and there is room, then its other colours, then
/// the 5th.
fn form_of(c: &Chord, rooted: bool) -> Vec<i32> {
    if c.tones.len() <= 4 {
        return pcs_of(c);
    }
    let mut out = needs_of(c);
    if rooted && out.len() < 4 {
        out.push(0);
    }
    let mut opt: Vec<i32> = c.tones.iter().copied().filter(|t| *t != 0 && !out.contains(&t.rem_euclid(12))).collect();
    opt.sort_by_key(|t| {
        (
            if *t > 12 {
                0
            } else if *t == 7 {
                1
            } else {
                2
            },
            *t,
        )
    });
    for t in opt {
        if out.len() >= 4 {
            break;
        }
        out.push(t.rem_euclid(12));
    }
    out.sort();
    out.dedup();
    out
}

/// Spread the way a guitarist plays it: the root low, then the 5th (when it stays), the 7th or 6th, the 3rd (or
/// sus note), the colours on top going up from the 3rd. Big chords drop the 5th, then the 9, to five notes.
fn spread_of(c: &Chord) -> Vec<i32> {
    let need = needs_of(c);
    let mut keep: Vec<i32> = c.tones.to_vec();
    if keep.len() >= 5 && !need.contains(&7) {
        keep.retain(|t| *t != 7);
    }
    for t in [14, 2] {
        if keep.len() > 5 && !need.contains(&(t % 12)) {
            keep.retain(|x| *x != t);
        }
    }
    let pc: Vec<i32> = keep.iter().map(|t| t.rem_euclid(12)).collect();
    let mut order: Vec<i32> = vec![0];
    let mut left: Vec<i32> = pc.iter().copied().filter(|p| *p != 0).collect();
    let take = |opts: &[i32], order: &mut Vec<i32>, left: &mut Vec<i32>| {
        if let Some(x) = opts.iter().copied().find(|x| left.contains(x)) {
            order.push(x);
            left.retain(|p| *p != x);
            Some(x)
        } else {
            None
        }
    };
    take(&[7, 6, 8], &mut order, &mut left);
    take(&[10, 11, 9], &mut order, &mut left);
    let third = take(&[4, 3, 5, 2], &mut order, &mut left).unwrap_or(0);
    left.sort_by_key(|p| (p - third).rem_euclid(12));
    order.extend(left);
    // each next note the first of its name above the one before
    let mut out: Vec<i32> = Vec::new();
    for p in order {
        let m = match out.last() {
            None => p,
            Some(&l) => l + 1 + (p - l - 1).rem_euclid(12),
        };
        out.push(m);
    }
    out
}

/// Notes closer than a 2nd below C3, or a 3rd below A2, are mud.
fn muddy(v: &[i32]) -> bool {
    let mut s = v.to_vec();
    s.sort();
    s.windows(2).any(|w| {
        let gap = w[1] - w[0];
        (gap <= 2 && w[0] < 48) || (gap <= 4 && w[0] < 45)
    })
}

/// A voicing (semitones over the root) placed on this root somewhere between E2 and E5 (picked by `rng` from the
/// places it fits), the root put under it in the bass (E2 to E\u{266D}3, a 3rd or more below) unless `no_bass` or it
/// is there already, and not too far under it (an octave and a 5th; more only when nothing else fits).
/// The first of `choices` that sits well (up to E5, the bass close enough); if none does, the first one as best it
/// can.
fn place(choices: &[Vec<i32>], root: i32, no_bass: bool, rng: &mut Rng) -> Vec<i32> {
    for rel in choices {
        let fits = places(rel, root, no_bass, &[(76, 19)]);
        if !fits.is_empty() {
            return fits[rng.below(fits.len())].clone();
        }
    }
    let rel = choices.first().cloned().unwrap_or_else(|| vec![0]);
    let fits = places(&rel, root, no_bass, &[(79, 24), (84, 36)]);
    if fits.is_empty() {
        let mut v: Vec<i32> = rel.iter().map(|x| root.rem_euclid(12) + 48 + x).collect();
        v.sort();
        return v;
    }
    fits[rng.below(fits.len())].clone()
}

/// Every octave `rel` fits in, from the first of `limits` (highest note, how far the bass may be under it) that
/// has any.
fn places(rel: &[i32], root: i32, no_bass: bool, limits: &[(i32, i32)]) -> Vec<Vec<i32>> {
    let root = root.rem_euclid(12);
    let mut fits: Vec<Vec<i32>> = Vec::new();
    for &(hi, reach) in limits {
        for k in 1..8 {
            let base = root + 12 * k;
            let mut v: Vec<i32> = rel.iter().map(|x| base + x).collect();
            v.sort();
            if !no_bass && v[0].rem_euclid(12) != root {
                let b = at_or_above(root, 40);
                if b > v[0] - 3 || v[0] - b > reach {
                    continue;
                }
                v.insert(0, b);
            }
            if v[0] >= 40 && *v.last().unwrap() <= hi && !muddy(&v) {
                fits.push(v);
            }
        }
        if !fits.is_empty() {
            break;
        }
    }
    fits
}

/// A major triad `t` semitones over the root, in inversion `inv`, its lowest note the first of its name above `over`.
fn triad_above(t: i32, inv: usize, over: i32) -> Vec<i32> {
    let tri = [t, t + 4, t + 7];
    let mut out: Vec<i32> = Vec::new();
    for k in 0..3 {
        let p = tri[(inv + k) % 3];
        let lo = out.last().copied().unwrap_or(over);
        out.push(lo + 1 + (p - lo - 1).rem_euclid(12));
    }
    out
}

/// `list` turned to start at a random place (the first is the one wanted, the rest stand-ins when it won't fit).
fn turned<T: Clone>(list: Vec<T>, rng: &mut Rng) -> Vec<T> {
    let n = list.len().max(1);
    let k = rng.below(n);
    (0..list.len()).map(|i| list[(k + i) % n].clone()).collect()
}

/// The chord's voicings as semitones over the root, the one picked first and the others of its kind after it (in
/// case it won't sit in the guitar's range on this root), and the voicing used (an index of VOICINGS; None: its own
/// shape).
fn voicing_of(c: &Chord, o: ChordOpts, rng: &mut Rng) -> (Vec<Vec<i32>>, Option<usize>) {
    match c.shape {
        Shape::Fixed(list) => return (turned(list.iter().map(|s| s.to_vec()).collect(), rng), None),
        Shape::Upper(t) => {
            let mut all: Vec<Vec<i32>> = Vec::new();
            for shell in [[4, 10], [10, 16]] {
                for inv in 0..3 {
                    let mut v = shell.to_vec();
                    v.extend(triad_above(t, inv, shell[1]));
                    all.push(v);
                }
            }
            return (turned(all, rng), None);
        }
        Shape::Slash(t) => {
            let all: Vec<Vec<i32>> = (0..3)
                .map(|inv| {
                    let mut v = vec![0];
                    v.extend(triad_above(t, inv, 7));
                    v
                })
                .collect();
            return (turned(all, rng), None);
        }
        Shape::Built => {}
    }
    let big = c.tones.len() >= 5;
    let mut kind = o.voicing as usize;
    if kind >= 5 {
        kind = rng.below(if big { 5 } else { 4 });
    }
    if kind == 4 && !big {
        // (rootless only makes sense with colours to play instead of the root)
        kind = 2;
    }
    let form = form_of(c, o.no_bass);
    let invs = |drop: &[usize]| (0..form.len()).map(|i| theory::voiced(&form, i, drop)).collect::<Vec<_>>();
    let all = match kind {
        0 => invs(&[]),
        1 => vec![spread_of(c)],
        2 | 3 => invs(if kind == 3 && form.len() >= 4 { &[3] } else { &[2] }),
        _ => {
            // A: the 3rd at the bottom; B: the 7th (or 6th)
            let f = form_of(c, false);
            [[4, 3, 5, 2], [10, 11, 9, 9]]
                .iter()
                .map(|from| {
                    let low = from.iter().copied().find(|x| f.contains(x)).unwrap_or(f[0]);
                    theory::voiced(&f, f.iter().position(|x| *x == low).unwrap_or(0), &[])
                })
                .collect()
        }
    };
    (turned(all, rng), Some(kind))
}

/// The letter a chord tone is written with: the 3rd's letter for a #9 (as players write A7#9's C), the 4th's for
/// a #11, the 6th's for a b13, the 7th's for a diminished 7th.
fn tone_degree(t: i32) -> usize {
    match t {
        0 | 12 => 1,
        1 | 2 | 13 | 14 => 2,
        3 | 4 | 15 | 16 => 3,
        5 | 17 | 18 => 4,
        6 | 7 | 8 | 19 => 5,
        20 | 21 => 6,
        9 => 6,
        _ => 7,
    }
}

/// A note `semis` over the root, spelt as degree `degree` of it (jazz eyes: a flat sign).
fn spelt_as(root: i32, semis: i32, degree: usize) -> String {
    let r = theory::note_name(root, lines::key_sharps(root));
    theory::spell_degree(r, (root + semis).rem_euclid(12), degree)
        .unwrap_or_else(|| theory::note_name(root + semis, lines::key_sharps(root)).to_string())
        .replace('b', "\u{266D}")
}

/// A chord tone's note name on this root (a dim7's 6th written as its bb7, so as its 7th's letter).
fn tone_name(c: &Chord, root: i32, t: i32) -> String {
    let deg = if t == 9 && c.name == "dim7" { 7 } else { tone_degree(t) };
    spelt_as(root, t, deg)
}

/// Chord names for jazz eyes: C MAJOR, C-7, C SO WHAT, D/C7 (C13#11), F/G.
fn chord_title(c: &Chord, root: i32) -> String {
    let r = nm(root, lines::key_sharps(root));
    // the triad's name as the degree it is: Bb/G7, not A#/G7
    let tri = |t: i32| {
        spelt_as(
            root,
            t,
            match t {
                2 => 2,
                3 => 3,
                6 => 4,
                8 => 6,
                _ => 7,
            },
        )
    };
    match c.shape {
        Shape::Upper(t) => format!("{}/{}7  ({}{})", tri(t), r, r, c.name.replace('b', "\u{266D}")),
        Shape::Slash(t) => format!("{}/{}", tri(t), r),
        _ if c.level == 0 || c.label == "SO WHAT" || c.label.starts_with("1 ") => {
            format!("{} {}", r, c.label)
        }
        _ => format!("{}{}", r, c.label),
    }
}

/// The CHORD QUALITIES question for chord `chord` (an index of CHORDS) on this root, the rest picked by `seed`.
pub(crate) fn build_chord(chord: usize, root: i32, seed: u64, o: ChordOpts) -> Question {
    let mut rng = Rng::new(seed);
    let c = &CHORDS[chord.min(CHORDS.len() - 1)];
    let root = root.rem_euclid(12);
    let (rel, kind) = voicing_of(c, o, &mut rng);
    let notes = place(&rel, root, o.no_bass, &mut rng);
    let mut pcs: Vec<i32> = notes.iter().map(|m| (m - root).rem_euclid(12)).collect();
    pcs.sort();
    pcs.dedup();
    // the top note, and the chord's tones that sound (the choices for "what's on top?")
    let top_t = tone_of(c, notes.last().copied().unwrap_or(root) - root).unwrap_or(0);
    let mut sounding: Vec<i32> = pcs.iter().filter_map(|p| tone_of(c, *p)).collect();
    sounding.sort_by_key(|t| if *t == 0 { -1 } else { *t });
    let tops: Vec<&'static str> = sounding.iter().map(|t| tone_label(c, *t)).collect();
    let top = tops.iter().position(|l| *l == tone_label(c, top_t)).unwrap_or(0);
    let sharps = lines::key_sharps(root);
    let how = kind.map_or(String::new(), |k| format!("   {}", VOICINGS[k]));
    let spelt: Vec<String> =
        notes.iter().map(|m| tone_of(c, m - root).map_or_else(|| nm(*m, sharps), |t| tone_name(c, root, t))).collect();
    let degs: Vec<&str> = notes.iter().map(|m| tone_of(c, m - root).map_or("?", |t| tone_label(c, t))).collect();
    let mut q = Question {
        answer: chord,
        root,
        seed,
        opt: 0,
        parts: Vec::new(),
        arp: arpeggio(&notes),
        lick: Vec::new(),
        lick_deg: Vec::new(),
        reveal: format!("{}{}", chord_title(c, root), how),
        chord,
        copts: o,
        pcs,
        tops,
        top,
        spelt: format!("{}   ({})", spelt.join(" "), degs.join(" ")),
        named: notes
            .iter()
            .zip(spelt.iter())
            .map(|(m, n)| (n.clone(), tone_of(c, m - root).map_or("?", |t| tone_label(c, t)), tone_of(c, m - root).unwrap_or(0)))
            .collect(),
    };
    if o.context {
        // a ii-V into it; a dominant is heard as the V of its own ii-V
        let p = pcs_of(c);
        // (a sus chord with a b7 too; a minor 11th is no dominant)
        let dominant = p.contains(&10) && (p.contains(&4) || (p.contains(&5) && !p.contains(&3))) && !p.contains(&11);
        let minor = p.contains(&3) && !p.contains(&4);
        let lead: Vec<(i32, &[i32])> = if dominant {
            vec![(7, MIN7)]
        } else if minor {
            vec![(2, HALF_DIM), (7, DOM_B9)]
        } else {
            vec![(2, MIN7), (7, DOM7)]
        };
        let steps = voice_prog(root, &lead);
        let gap = 1.1;
        let at = steps.len() as f32 * gap + 0.1;
        q.parts = vec![Part { at: 0.0, steps, gap, clean: false }, Part { at, steps: vec![notes], gap: 1.6, clean: false }];
    } else {
        q.parts = vec![Part { at: 0.0, steps: vec![notes], gap: 1.0, clean: false }];
    }
    q
}

/// A new CHORD QUALITIES question: a chord from the pool (weighted), on `root` or a random one. Its answer is its
/// place in the pool.
pub(crate) fn ask_chord(pool: &[usize], weights: &[f32], o: ChordOpts, root: Option<i32>, seed: u64) -> Question {
    let mut rng = Rng::new(seed);
    let i = pick_weighted(weights, &mut rng).min(pool.len().saturating_sub(1));
    let root = root.unwrap_or_else(|| rng.below(12) as i32);
    let mut q = build_chord(pool.get(i).copied().unwrap_or(0), root, rng.next(), o);
    q.answer = i;
    q
}

/// The chords you take for one another most: the ones in your most-made mix-ups, up to six.
pub(crate) fn mixup_set(st: &EarStats) -> Vec<usize> {
    let mut mix: Vec<(&String, &u32)> = st.mixups.iter().filter(|m| *m.1 > 0).collect();
    mix.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let mut out: Vec<usize> = Vec::new();
    for (k, _) in mix {
        let Some((a, b)) = k.split_once('>') else { continue };
        let (Some(a), Some(b)) = (chord_by_label(a), chord_by_label(b)) else { continue };
        let new: Vec<usize> = [a, b].into_iter().filter(|x| !out.contains(x)).collect();
        if out.len() + new.len() > 6 {
            continue;
        }
        out.extend(new);
    }
    out
}

/// The chords in play: the levels ticked (each its first four, more as you get them), a look-alike set (or your
/// own mix-ups), or the chords you picked. Never fewer than two.
pub(crate) fn chord_pool(o: &crate::store::EarChords, st: &EarStats) -> Vec<usize> {
    let mut out: Vec<usize> = match o.mode {
        1 if o.alike < ALIKES.len() => ALIKES[o.alike].iter().filter_map(|l| chord_by_label(l)).collect(),
        1 => mixup_set(st),
        2 => (0..CHORDS.len()).filter(|i| o.mine.iter().any(|m| m == CHORDS[*i].label)).collect(),
        _ => {
            let mut lv: Vec<usize> = o.levels.iter().copied().filter(|l| *l < LEVELS.len()).collect();
            lv.sort();
            lv.dedup();
            if lv.is_empty() {
                lv.push(1);
            }
            lv.iter().flat_map(|l| level_chords(*l).into_iter().take(level_in_play(o, *l))).collect()
        }
    };
    out.dedup();
    if out.len() < 2 {
        out = match o.mode {
            1 => ALIKES[0].iter().filter_map(|l| chord_by_label(l)).collect(),
            _ => level_chords(1).into_iter().take(START).collect(),
        };
    }
    out
}

/// How many of a level's chords are in play.
pub(crate) fn level_in_play(o: &crate::store::EarChords, level: usize) -> usize {
    let all = level_chords(level).len();
    o.grown.get(level).copied().unwrap_or(0).clamp(START.min(all), all)
}

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
        0 => CHORDS.iter().map(|c| c.label).collect(),
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
    /// CHORD QUALITIES: the chord (an index of CHORDS), how it was played, the notes that sound (pitch classes over
    /// the root), the choices for "what's on top?" and the right one, and the notes spelt out
    pub chord: usize,
    pub copts: ChordOpts,
    pub pcs: Vec<i32>,
    pub tops: Vec<&'static str>,
    pub top: usize,
    #[cfg_attr(not(test), allow(dead_code))]
    pub spelt: String,
    /// each note low to high: its name, its degree, its tone (semitones over the root)
    pub named: Vec<(String, &'static str, i32)>,
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
    if drill == 0 {
        // (here `answer` is the chord's place in CHORDS)
        return build_chord(answer, root, seed, ChordOpts::default());
    }
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
        chord: 0,
        copts: ChordOpts::default(),
        pcs: Vec::new(),
        tops: Vec::new(),
        top: 0,
        spelt: String::new(),
        named: Vec::new(),
    };
    let one = |steps: Vec<Vec<i32>>, gap: f32, clean: bool| vec![Part { at: 0.0, steps, gap, clean }];
    match drill {
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
    let answer = pick_weighted(weights, &mut rng);
    // a root anywhere in the octave; an interval's low note from E3 to C#4
    let root = if drill == 1 { 52 + rng.below(10) as i32 } else { rng.below(12) as i32 };
    build(drill, answer, root, rng.next(), opt, level)
}

/// One of the answers, each as likely as its weight.
fn pick_weighted(weights: &[f32], rng: &mut Rng) -> usize {
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
    answer
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

/// The answer chime: right, a soft 5th up high (E5 B5); wrong, one quiet low note (A2); after `wait` seconds.
fn chime_sound(ok: bool, wait: f32) -> Vec<i16> {
    let (notes, gap, soft) = if ok { (vec![vec![76], vec![83]], 0.13, 0.3) } else { (vec![vec![45]], 0.35, 0.22) };
    let mut out = vec![0i16; (wait * band::RATE) as usize];
    out.extend(band::line_sound(&notes, gap).iter().map(|s| (*s as f32 * soft) as i16));
    out
}

/// One short line on what sets the answer you picked apart from the right one: the 3rd, the 5th, the 7th (or 6th),
/// the tensions, at most two of them; when those match, both chords' notes.
pub(crate) fn chord_hint(picked: &Chord, asked: &Chord) -> String {
    let feats = |c: &Chord| -> [String; 4] {
        let has = |t: i32| c.tones.contains(&t);
        let third = if has(4) {
            "a major 3rd"
        } else if has(3) {
            "a minor 3rd"
        } else if has(5) {
            "a sus 4"
        } else if has(2) && !has(4) && !has(3) {
            "a sus 2"
        } else {
            "no 3rd"
        };
        // (a 5th that is only left in or out by the voicing says nothing)
        let fifth = if has(6) {
            "a flat 5"
        } else if has(8) {
            "a sharp 5"
        } else if has(7) && needs_of(c).contains(&7) {
            "a natural 5"
        } else if has(7) {
            ""
        } else {
            "no 5th"
        };
        let seventh = if has(10) {
            "a \u{266D}7"
        } else if has(11) {
            "a major 7th"
        } else if has(9) && c.name == "dim7" {
            "a \u{266D}\u{266D}7"
        } else if has(9) {
            "a 6th"
        } else {
            "no 7th"
        };
        let tens: Vec<String> = c.tones.iter().filter(|t| **t > 12).map(|t| format!("a {}", tone_label(c, *t))).collect();
        let tens = if tens.is_empty() { "no tensions".to_string() } else { tens.join(" and ") };
        [third.to_string(), fifth.to_string(), seventh.to_string(), tens]
    };
    let (p, a) = (feats(picked), feats(asked));
    let diff: Vec<usize> = (0..4).filter(|k| !p[*k].is_empty() && !a[*k].is_empty() && p[*k] != a[*k]).take(2).collect();
    if diff.is_empty() {
        let degs = |c: &Chord| {
            let mut t = c.tones.to_vec();
            t.sort();
            t.iter().map(|x| tone_label(c, *x)).collect::<Vec<_>>().join(" ")
        };
        return format!("You picked {}: {}\n{}: {}", picked.label, degs(picked), asked.label, degs(asked));
    }
    let say = |f: &[String; 4]| diff.iter().map(|k| f[*k].clone()).collect::<Vec<_>>().join(" and ");
    format!("You picked {}: {}\n{}: {}", picked.label, say(&p), asked.label, say(&a))
}

/// A hint for a wrong answer, where the drill has one.
fn wrong_hint(drill: usize, q: &Question, pool: &[usize], given: usize) -> Option<String> {
    match drill {
        0 => {
            let p = &CHORDS[*pool.get(given)?];
            Some(chord_hint(p, &CHORDS[q.chord.min(CHORDS.len() - 1)]))
        }
        1 => {
            let (pn, ps) = INTERVALS.get(given)?;
            let (an, as_) = INTERVALS.get(q.answer)?;
            Some(format!("You picked {}: {} half steps\n{}: {} half steps", pn, ps, an, as_))
        }
        _ => None,
    }
}

/// A milestone for a run of right answers.
fn streak_word(n: u32) -> &'static str {
    match n {
        5 => "ON A ROLL",
        10 => "EARS OPEN",
        n if n >= 20 && n % 10 == 0 => "BANDSTAND READY",
        _ => "",
    }
}

/// A skin with light text on a dark ground.
fn dark_skin() -> bool {
    let i = pal().ink;
    (i.r() as u32 + i.g() as u32 + i.b() as u32) > 384
}

/// The green a right answer is shown in, readable on light and dark skins.
fn ok_col() -> Color32 {
    if dark_skin() {
        Color32::from_rgb(120, 214, 130)
    } else {
        Color32::from_rgb(28, 128, 58)
    }
}

/// The red a wrong answer is shown in (the skin's own red, unless the skin greys it out).
fn bad_col() -> Color32 {
    let r = pal().red;
    if r.r() as i32 - (r.g() as i32).max(r.b() as i32) > 60 {
        r
    } else if dark_skin() {
        Color32::from_rgb(240, 96, 96)
    } else {
        Color32::from_rgb(196, 36, 60)
    }
}

/// The accent for a run of right answers.
fn acc_col() -> Color32 {
    if dark_skin() {
        Color32::from_rgb(255, 176, 64)
    } else {
        Color32::from_rgb(190, 104, 16)
    }
}

/// A chord tone's colour, as the necks and the triad map colour them: the root red, the 3rd (or the note in its
/// place) blue, the 5th green, the 6th and 7th orange, the tensions purple.
fn tone_col(t: i32) -> Color32 {
    match t {
        0 | 12 => Color32::from_rgb(235, 64, 64),
        2..=5 | 16 => Color32::from_rgb(64, 140, 255),
        6..=8 | 19 => Color32::from_rgb(56, 196, 90),
        9..=11 | 22 | 23 => Color32::from_rgb(255, 150, 40),
        _ => Color32::from_rgb(178, 96, 255),
    }
}

/// Each drill's colour, a dot beside the drill menu.
fn drill_col(d: usize) -> Color32 {
    tone_col([0, 4, 7, 10, 14, 3, 19, 11][d % 8])
}

/// Answer `i` is right: the one asked, or (CHORD QUALITIES) a chord the notes heard spell just as well.
fn answer_right(drill: usize, q: &Question, pool: &[usize], i: usize) -> bool {
    i == q.answer || (drill == 0 && pool.get(i).is_some_and(|c| fits(&CHORDS[*c], &q.pcs)))
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
    /// CHORD QUALITIES: the root kept for SAME ROOT, your answer to "what's on top?", this session's top notes
    /// (asked, right), MY SET's chooser open
    pub same_root: Option<i32>,
    pub top_given: Option<usize>,
    pub top_n: (u32, u32),
    pub choose: bool,
    /// answers right in a row (this drill, since you opened it)
    pub streak: u32,
    /// this session, each answer right or not (the bar under the question count)
    pub marks: Vec<bool>,
}

impl App {
    /// A test copy opened on a drill (BACKLINE_EAR=drill number) with a question asked and answered wrong.
    pub(crate) fn ear_demo(&mut self, drill: usize) {
        self.ear = Ear { drill: drill.min(DRILLS.len() - 1), ..Default::default() };
        let (_, level, pool) = self.ear_in_play();
        let q = if self.ear.drill == 0 {
            ask_chord(&pool, &vec![1.0; level], self.ear_copts(), None, 11)
        } else {
            ask(self.ear.drill, level, &vec![1.0; level], 0, 11)
        };
        if self.ear.drill == 7 {
            self.ear.played = q.lick.iter().map(|m| m + 2).collect();
            self.ear.checked = true;
        } else if std::env::var("BACKLINE_EAR_OK").is_ok() {
            // (BACKLINE_EAR_OK: answered right, five in a row, waiting on "what's on top?")
            self.ear.given = Some(q.answer);
            self.ear.streak = 5;
            self.store.ear_chords.top = true;
        } else {
            self.ear.given = Some((q.answer + 1) % level);
        }
        self.ear.n = 6;
        self.ear.right = 4;
        self.ear.marks = vec![true, true, false, true, true, false];
        self.ear.q = Some(q);
    }

    fn ear_stats(&mut self) -> &mut EarStats {
        let key = DRILLS[self.ear.drill.min(DRILLS.len() - 1)].key;
        self.store.ear.entry(key.to_string()).or_default()
    }

    /// The answers in play: their names, how many, and for CHORD QUALITIES the chords (places in CHORDS).
    fn ear_in_play(&mut self) -> (Vec<&'static str>, usize, Vec<usize>) {
        let drill = self.ear.drill.min(DRILLS.len() - 1);
        let st = self.ear_stats().clone();
        if drill == 0 {
            let pool = chord_pool(&self.store.ear_chords, &st);
            let names: Vec<&'static str> = pool.iter().map(|i| CHORDS[*i].label).collect();
            (names, pool.len(), pool)
        } else {
            (answers(drill), level_of(&st, drill), Vec::new())
        }
    }

    /// How CHORD QUALITIES plays its chords, from your choices.
    fn ear_copts(&self) -> ChordOpts {
        let o = &self.store.ear_chords;
        ChordOpts { voicing: o.voicing.min(VOICINGS.len() as u8 - 1), no_bass: o.no_bass, context: o.context }
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

    /// A soft two notes up for a right answer, one low quiet note for a wrong one, after what is still ringing.
    fn ear_chime(&mut self, ok: bool) {
        if self.store.ear_quiet {
            return;
        }
        let left = match &self.seq_play {
            Some((at, len, tag, _)) if tag == "ear" => (len - at.elapsed().as_secs_f32()).max(0.0),
            _ => 0.0,
        };
        let pcm = chime_sound(ok, (left + 0.15).min(3.0));
        self.player.send(Cmd::Once(pcm, 0.5));
    }

    /// The next question, played straight away.
    fn ear_next(&mut self) {
        if self.ear.done.is_some() {
            self.ear.done = None;
            self.ear.n = 0;
            self.ear.right = 0;
            self.ear.secs = 0.0;
            self.ear.same_root = None;
            self.ear.top_n = (0, 0);
            self.ear.marks.clear();
        }
        let drill = self.ear.drill;
        let (names, level, pool) = self.ear_in_play();
        let last = self.ear.q.as_ref().map(|q| q.answer);
        let st = self.ear_stats().clone();
        let w = if drill == 7 { vec![1.0] } else { weights(&st, &names[..level], last) };
        let q = if drill == 0 {
            let root = if self.store.ear_chords.same_root {
                Some(*self.ear.same_root.get_or_insert(Rng::new(clock_seed()).below(12) as i32))
            } else {
                None
            };
            ask_chord(&pool, &w, self.ear_copts(), root, clock_seed())
        } else {
            ask(drill, level, &w, self.ear.dir, clock_seed())
        };
        self.ear_play(&q.parts.clone());
        self.ear.q = Some(q);
        self.ear.given = None;
        self.ear.top_given = None;
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
        self.ear.streak = if ok { self.ear.streak + 1 } else { 0 };
        self.ear.marks.push(ok);
        let streak = self.ear.streak;
        let st = self.ear_stats();
        st.best = st.best.max(streak);
        self.ear_chime(ok);
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
        crate::tour::mark("EARVIEW", ui.clip_rect());
        self.seq_tick(ui.ctx());
        ui.add_space(6.0);
        title_line(ui, "EAR TRAINING", 3.0, pal().ink);
        ui.add_space(4.0);
        let drill = self.ear.drill.min(DRILLS.len() - 1);
        ui.horizontal_wrapped(|ui| {
            let names: Vec<&str> = DRILLS.iter().map(|d| d.name).collect();
            // the drill's colour
            let (r, _) = ui.allocate_exact_size(Vec2::new(12.0, bh()), Sense::hover());
            ui.painter().circle_filled(r.center(), 5.0, drill_col(drill));
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
            if check_box(ui, "CHIME", !self.store.ear_quiet).tip("A soft sound after each answer").clicked() {
                self.store.ear_quiet = !self.store.ear_quiet;
                self.store_dirty = true;
            }
        });
        para(ui, DRILLS[drill].about, pal().ink2);
        if drill == 0 {
            ui.add_space(6.0);
            self.ear_chord_controls(ui, acts);
        }
        ui.add_space(18.0);
        self.ear_question(ui, acts);
        ui.add_space(22.0);
        self.ear_scores(ui);
        ui.add_space(16.0);
    }

    /// CHORD QUALITIES: what's in play (levels, look-alikes or your own set, its choices beside it), then how the
    /// chords are played, one row each. A change starts a fresh question.
    fn ear_chord_controls(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let mut o = self.store.ear_chords.clone();
        ui.horizontal_wrapped(|ui| {
            if let Some(i) = crate::dropdown(ui, "ear_mode", "IN PLAY", &MODES, (o.mode as usize).min(2), 0.0) {
                o.mode = i as u8;
            }
            ui.add_space(6.0);
            match o.mode {
                1 => {
                    let mut sets: Vec<String> = ALIKES.iter().map(|s| s.join("  ")).collect();
                    sets.push(MIXUPS_SET.to_string());
                    let refs: Vec<&str> = sets.iter().map(|s| s.as_str()).collect();
                    if let Some(i) = crate::dropdown(ui, "ear_alike", "", &refs, o.alike.min(ALIKES.len()), 0.0) {
                        o.alike = i;
                    }
                }
                2 => {
                    let text = format!("CHOOSE CHORDS ({})", o.mine.len());
                    if retro_btn(ui, &text, self.ear.choose).tip("Tick any chords, from any level").clicked() {
                        self.ear.choose = !self.ear.choose;
                    }
                }
                _ => {
                    let lv: Vec<usize> = o.levels.iter().copied().filter(|l| *l < LEVELS.len()).collect();
                    let text = match lv.as_slice() {
                        [l] => LEVELS[*l].to_string(),
                        [] => LEVELS[1].to_string(),
                        _ => format!("{} LEVELS", lv.len()),
                    };
                    if retro_btn(ui, &text, self.ear.choose).tip("Pick one or more levels").clicked() {
                        self.ear.choose = !self.ear.choose;
                    }
                }
            }
        });
        if o.mode == 0 && self.ear.choose {
            ui.horizontal_wrapped(|ui| {
                for (l, name) in LEVELS.iter().enumerate() {
                    let on = o.levels.contains(&l);
                    if check_box(ui, name, on).clicked() {
                        if on {
                            if o.levels.len() > 1 {
                                o.levels.retain(|x| *x != l);
                            }
                        } else {
                            o.levels.push(l);
                            o.levels.sort();
                        }
                    }
                }
            });
        }
        if o.mode != self.store.ear_chords.mode {
            self.ear.choose = false;
        }
        if o.mode == 1 && o.alike >= ALIKES.len() && mixup_set(&self.ear_stats().clone()).len() < 2 {
            crate::views::dim_line(ui, "NO MIX-UPS YET: THE FIRST SET PLAYS UNTIL YOU HAVE SOME", 1.0, pal().dim);
        }
        if o.mode == 2 && self.ear.choose {
            // every chord, a row of ticks per level
            for (l, name) in LEVELS.iter().enumerate() {
                ui.add_space(4.0);
                mini_header(ui, name);
                ui.horizontal_wrapped(|ui| {
                    for i in level_chords(l) {
                        let label = CHORDS[i].label;
                        let on = o.mine.iter().any(|m| m == label);
                        if check_box(ui, label, on).clicked() {
                            if on {
                                o.mine.retain(|m| m != label);
                            } else {
                                o.mine.push(label.to_string());
                            }
                        }
                    }
                });
            }
            if o.mine.len() < 2 {
                crate::views::dim_line(ui, "TICK TWO OR MORE", 1.0, pal().dim);
            }
        }
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            if let Some(i) = crate::dropdown(ui, "ear_voicing", "VOICING", &VOICINGS, (o.voicing as usize).min(5), 0.0) {
                o.voicing = i as u8;
            }
            if let Some(i) = crate::dropdown(ui, "ear_root", "", &ROOTS, o.same_root as usize, 0.0) {
                o.same_root = i == 1;
            }
            if let Some(i) = crate::dropdown(ui, "ear_bass", "", &BASSES, o.no_bass as usize, 0.0) {
                o.no_bass = i == 1;
            }
            if check_box(ui, "TOP NOTE", o.top).on_hover_text("After the chord: which of its notes is on top?").clicked() {
                o.top = !o.top;
            }
            if check_box(ui, "IN CONTEXT", o.context).on_hover_text("Hear it after a ii-V (a dominant as the V)").clicked() {
                o.context = !o.context;
            }
        });
        if o != self.store.ear_chords {
            let played = (o.voicing, o.no_bass, o.context) != {
                let s = &self.store.ear_chords;
                (s.voicing, s.no_bass, s.context)
            };
            let st = self.ear_stats().clone();
            let pool_moved = chord_pool(&o, &st) != chord_pool(&self.store.ear_chords, &st);
            if o.same_root != self.store.ear_chords.same_root {
                self.ear.same_root = None;
            }
            self.store.ear_chords = o;
            self.store_dirty = true;
            // a new set of answers (or a new sound) wants a new question
            if pool_moved || played {
                self.ear.q = None;
                self.ear.given = None;
                self.ear.top_given = None;
                acts.push(Action::StopSequence);
            }
        }
    }

    /// Layout A: the big PLAY, where you are in the session, the answers, then hear again / arpeggio / next.
    fn ear_question(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let drill = self.ear.drill.min(DRILLS.len() - 1);
        let (names, level, pool) = self.ear_in_play();
        let top_on = drill == 0 && self.store.ear_chords.top;
        let mut top_pick: Option<usize> = None;
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
            // a centred line, wrapped to the room there is (a "\n" starts a new one)
            let line = |ui: &mut egui::Ui, text: &str, px: f32, col: Color32| {
                let room = (ui.available_width() - 24.0).max(60.0);
                for row in wrap(text, px, room) {
                    let h = 7.0 * spx(px) + 8.0;
                    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
                    ptext_fit(ui.painter(), r.center(), Align::Center, &row, px, r.width() - 12.0, col);
                }
            };
            line(ui, DRILLS[drill].name, 2.2, pal().ink);
            let shown = (self.ear.n + (!answered && self.ear.q.is_some() && self.ear.done.is_none()) as u32).clamp(1, SESSION);
            let at = if self.ear.q.is_none() {
                "PRESS PLAY TO START".to_string()
            } else {
                format!("QUESTION {} OF {}", shown, SESSION)
            };
            line(ui, &at, 1.0, pal().ink2);
            if self.ear.q.is_some() {
                // the session so far: a block per question, green right, red wrong
                ui.add_space(4.0);
                let w = ui.available_width().min(280.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), Sense::hover());
                let x0 = r.center().x - w / 2.0;
                let seg = w / SESSION as f32;
                for k in 0..SESSION as usize {
                    let cell = Rect::from_min_size(Pos2::new(x0 + k as f32 * seg + 1.0, r.min.y), Vec2::new(seg - 2.0, 6.0));
                    let col = match self.ear.marks.get(k) {
                        Some(true) => ok_col(),
                        Some(false) => bad_col(),
                        None => pal().beige_dk.gamma_multiply(0.6),
                    };
                    ui.painter().rect_filled(cell, 2.0, col);
                }
            }
            ui.add_space(14.0);

            if let Some(q) = self.ear.q.clone() {
                if drill == 7 {
                    self.ear_neck(ui, acts, &q);
                } else {
                    // the answers in play: after answering, the right one lit, a wrong pick marked; a click on any of
                    // them then plays it on the same root, to compare
                    let labels: Vec<&str> = names[..level].to_vec();
                    let given = self.ear.given;
                    let right = |i: usize| answer_right(drill, &q, &pool, i);
                    if let Some(i) = centred_pills(ui, "ear_ans", &labels, |i| {
                        let lit = answered && (i == q.answer || (given == Some(i) && right(i)));
                        (lit, answered && given == Some(i) && !right(i))
                    }) {
                        pick = Some(i);
                    }
                }
                ui.add_space(12.0);
                // what it was
                if answered {
                    // (what was asked: the chord itself, for CHORD QUALITIES)
                    let asked = if drill == 0 { CHORDS[q.chord.min(CHORDS.len() - 1)].label } else { names[q.answer] };
                    let ok = if drill == 7 {
                        self.ear.played.len() == q.lick.len()
                            && self.ear.played.iter().zip(q.lick.iter()).all(|(a, b)| (a - b).rem_euclid(12) == 0)
                    } else {
                        self.ear.given.is_some_and(|g| answer_right(drill, &q, &pool, g))
                    };
                    let head = if ok && self.ear.given.is_some_and(|g| g != q.answer) {
                        format!("Correct: the same notes as {}", asked)
                    } else if ok {
                        "CORRECT!".to_string()
                    } else if drill == 7 {
                        "NOT QUITE".to_string()
                    } else {
                        // (numerals keep their case, so the words after the colon are written in normal case:
                        // "NOT QUITE: it was -7b5", never title-cased as a label)
                        format!("NOT QUITE: it was {}", asked)
                    };
                    line(ui, &head, if ok { 2.6 } else { 2.0 }, if ok { ok_col() } else { bad_col() });
                    // what sets the one you picked apart
                    if !ok {
                        if let Some(h) = self.ear.given.and_then(|g| wrong_hint(drill, &q, &pool, g)) {
                            line(ui, &h, 1.0, pal().ink);
                        }
                    }
                    // the run of right answers
                    let s = self.ear.streak;
                    if s >= 2 {
                        line(ui, &format!("{} IN A ROW", s), 1.5, acc_col());
                        let word = streak_word(s);
                        if !word.is_empty() {
                            let hi = if dark_skin() { Color32::from_rgb(255, 206, 120) } else { Color32::from_rgb(160, 80, 0) };
                            line(ui, word, 1.8, hi);
                        }
                    }
                    line(ui, &q.reveal, 1.0, pal().ink2);
                    if drill == 0 && top_on && !q.tops.is_empty() {
                        // the second question: the note on top
                        ui.add_space(8.0);
                        let tg = self.ear.top_given;
                        let head = match tg {
                            None => "WHAT'S ON TOP?".to_string(),
                            Some(t) if t == q.top => format!("ON TOP: {}. CORRECT", q.tops[q.top]),
                            Some(_) => format!("ON TOP: {}", q.tops[q.top]),
                        };
                        let col = match tg {
                            Some(t) if t == q.top => ok_col(),
                            Some(_) => bad_col(),
                            None => pal().ink,
                        };
                        line(ui, &head, 1.5, col);
                        if let Some(i) =
                            centred_pills(ui, "ear_top", &q.tops, |i| (tg.is_some() && i == q.top, tg == Some(i) && i != q.top))
                        {
                            top_pick = Some(i);
                        }
                        ui.add_space(4.0);
                    }
                    if drill == 0 && (!top_on || self.ear.top_given.is_some()) {
                        named_row(ui, &q.named);
                    }
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
                let ok = answer_right(drill, &q, &pool, i);
                self.ear_note(names[q.answer], if ok { names[q.answer] } else { names[i] });
                self.ear_scored(ok);
            } else {
                // the answer picked, built the same way on the same root
                let other = if drill == 0 {
                    build_chord(pool.get(i).copied().unwrap_or(0), q.root, q.seed, q.copts)
                } else {
                    build(drill, i, q.root, q.seed, q.opt, level)
                };
                self.ear_play(&other.parts);
            }
        }
        if let (Some(i), Some(q)) = (top_pick, self.ear.q.clone()) {
            if self.ear.top_given.is_none() {
                self.ear.top_given = Some(i);
                self.ear.top_n.0 += 1;
                self.ear.top_n.1 += (i == q.top) as u32;
                let (asked, given) = (q.tops[q.top], q.tops[i]);
                let st = self.store.ear.entry(TOP_KEY.to_string()).or_default();
                *st.asked.entry(asked.to_string()).or_insert(0) += 1;
                if i == q.top {
                    *st.right.entry(asked.to_string()).or_insert(0) += 1;
                } else {
                    *st.mixups.entry(format!("{}>{}", asked, given)).or_insert(0) += 1;
                }
                self.store_dirty = true;
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
        let (_, level, pool) = self.ear_in_play();
        let names = answers(drill);
        let st = self.ear_stats().clone();
        section_header(ui, "YOUR SCORE");
        ui.add_space(6.0);
        let today = crate::store::today_days();
        let pct = |r: u32, n: u32| if n == 0 { "-".to_string() } else { format!("{}%", r * 100 / n) };
        let session = if self.ear.done.is_some() { (0, 0) } else { (self.ear.right, self.ear.n) };
        let week = share_since(&st, today - 6).unwrap_or((0, 0));
        let week = (week.0 + session.0, week.1 + session.1);
        let (sr, sn) = match self.ear.done {
            Some((r, n, _)) => (r, n),
            None => session,
        };
        let play_tile = match drill {
            0 => (level.to_string(), "CHORDS IN PLAY"),
            7 => (format!("{} OF {}", level, names.len()), "NOTES IN PLAY"),
            _ => (format!("{} OF {}", level, names.len()), "ANSWERS IN PLAY"),
        };
        let tiles = [
            (pct(sr, sn), "THIS SESSION".to_string()),
            (pct(week.0, week.1), "THIS WEEK".to_string()),
            (play_tile.0, play_tile.1.to_string()),
            (st.best.max(self.ear.streak).to_string(), "BEST RUN".to_string()),
        ];
        let tw = ((ui.available_width() - 28.0) / 4.0).clamp(80.0, 150.0);
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
        let now_ok = sn >= 10 && sr * 100 >= sn * 80;
        let last_ok = st.sessions.last().is_some_and(|s| s.2 > 0 && s.1 * 100 >= s.2 * 80);
        if drill == 0 {
            self.ear_chord_growth(ui, now_ok || last_ok);
        } else {
            if level < names.len() {
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
        }
        if drill == 0 {
            // each chord in play, the weakest first
            let mut how: Vec<(u32, String)> = pool
                .iter()
                .filter_map(|c| {
                    let l = CHORDS[*c].label;
                    let n = st.asked.get(l).copied().unwrap_or(0);
                    let r = st.right.get(l).copied().unwrap_or(0).min(n);
                    (n > 0).then(|| (r * 100 / n, format!("{}  {}%", l, r * 100 / n)))
                })
                .collect();
            how.sort_by_key(|h| h.0);
            if !how.is_empty() {
                ui.add_space(10.0);
                section_header(ui, "CHORD BY CHORD");
                chips(ui, &how.into_iter().take(10).map(|h| h.1).collect::<Vec<_>>());
            }
            if self.store.ear_chords.top && self.ear.top_n.0 > 0 {
                let (n, r) = self.ear.top_n;
                crate::views::dim_line(ui, &format!("WHAT'S ON TOP: {} OF {} RIGHT THIS SESSION", r, n), 1.0, pal().ink2);
            }
        }
        // what you take for what, most often first
        ui.add_space(10.0);
        section_header(ui, "MIXED UP MOST");
        let mut mix: Vec<(&String, &u32)> = st.mixups.iter().filter(|m| *m.1 > 0).collect();
        mix.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        if mix.is_empty() {
            crate::views::dim_line(ui, "NOTHING YET. THE ANSWERS YOU MISS COME UP MORE OFTEN.", 1.0, pal().dim);
        } else {
            let texts: Vec<String> = mix
                .iter()
                .take(4)
                .map(|(k, n)| {
                    let (a, b) = k.split_once('>').unwrap_or((k.as_str(), ""));
                    format!("{} HEARD AS {}  x{}", a, b, n)
                })
                .collect();
            chips(ui, &texts);
            let drilled = drill == 0 && self.store.ear_chords.mode == 1 && self.store.ear_chords.alike >= ALIKES.len();
            let hint = if drill == 0 && !drilled {
                "THESE COME UP MORE OFTEN. DRILL THEM HEAD TO HEAD: IN PLAY - LOOK-ALIKES - YOUR MIX-UPS."
            } else {
                "THESE COME UP MORE OFTEN UNTIL YOU GET THEM."
            };
            crate::views::dim_line(ui, hint, 1.0, pal().dim);
        }
    }

    /// CHORD QUALITIES, by levels: two more chords of the first level ticked that has more, once you get 80%; and
    /// back to fewer.
    fn ear_chord_growth(&mut self, ui: &mut egui::Ui, ready: bool) {
        let o = self.store.ear_chords.clone();
        if o.mode != 0 {
            return;
        }
        let mut lv: Vec<usize> = o.levels.iter().copied().filter(|l| *l < LEVELS.len()).collect();
        lv.sort();
        lv.dedup();
        let grow = lv.iter().copied().find(|l| level_in_play(&o, *l) < level_chords(*l).len());
        let shrink = lv.iter().rev().copied().find(|l| level_in_play(&o, *l) > START.min(level_chords(*l).len()));
        let mut set: Option<(usize, usize)> = None;
        ui.horizontal_wrapped(|ui| {
            if let Some(l) = grow {
                let have = level_in_play(&o, l);
                let all = level_chords(l);
                if ready {
                    let next: Vec<&str> = all[have..(have + 2).min(all.len())].iter().map(|c| CHORDS[*c].label).collect();
                    if retro_btn(ui, "MORE CHORDS", false).tip(&format!("Adds {}", next.join(" and "))).clicked() {
                        set = Some((l, (have + 2).min(all.len())));
                    }
                } else {
                    crate::views::dim_line(ui, "GET 80% IN A SESSION AND MORE CHORDS COME INTO PLAY", 1.0, pal().dim);
                }
            }
            if let Some(l) = shrink {
                if retro_btn(ui, "FEWER CHORDS", false).tip(&format!("Back to fewer {}", LEVELS[l].to_lowercase())).clicked() {
                    set = Some((l, level_in_play(&o, l).saturating_sub(2).max(START)));
                }
            }
        });
        if let Some((l, n)) = set {
            let g = &mut self.store.ear_chords.grown;
            if g.len() <= l {
                g.resize(l + 1, 0);
            }
            g[l] = n;
            self.store_dirty = true;
        }
    }
}

/// A chord's notes low to high, each over its degree, both in the degree's colour, centred (more rows if narrow).
fn named_row(ui: &mut egui::Ui, named: &[(String, &'static str, i32)]) {
    if named.is_empty() {
        return;
    }
    let gap = 12.0;
    let cols: Vec<f32> = named.iter().map(|(n, d, _)| text_w(n, 1.5).max(text_w(d, 1.0)) + gap).collect();
    let room = (ui.available_width() - 24.0).max(60.0);
    let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
    let mut w = 0.0;
    for (i, cw) in cols.iter().enumerate() {
        if w + cw > room && !rows.last().unwrap().is_empty() {
            rows.push(Vec::new());
            w = 0.0;
        }
        rows.last_mut().unwrap().push(i);
        w += cw;
    }
    let (hn, hd) = (7.0 * spx(1.5) + 4.0, 7.0 * spx(1.0) + 6.0);
    for row in rows {
        let total: f32 = row.iter().map(|i| cols[*i]).sum();
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), hn + hd), Sense::hover());
        let mut x = r.center().x - total / 2.0;
        let p = ui.painter();
        for i in row {
            let (n, d, t) = &named[i];
            let cx = x + cols[i] / 2.0;
            let col = tone_col(*t);
            ptext(p, Pos2::new(cx, r.min.y + hn / 2.0), Align::Center, n, 1.5, col);
            ptext(p, Pos2::new(cx, r.min.y + hn + hd / 2.0 - 2.0), Align::Center, d, 1.0, col);
            x += cols[i];
        }
    }
}

/// Little rounded labels in a row (scores, mix-ups).
fn chips(ui: &mut egui::Ui, texts: &[String]) {
    ui.horizontal_wrapped(|ui| {
        for text in texts {
            let w = text_w(text, 1.5) + 24.0;
            let (r, _) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::hover());
            let p = ui.painter();
            p.rect_filled(r, 14.0, pal().beige_dk.gamma_multiply(0.35));
            p.rect_stroke(r, 14.0, Stroke::new(1.0_f32, pal().beige_dk));
            ptext(p, r.center(), Align::Center, text, 1.5, pal().ink2);
        }
    });
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
                    let rad = (resp.rect.height() / 2.0).min(14.0);
                    if wrong {
                        ui.painter().rect_stroke(resp.rect.expand(1.0), rad, Stroke::new(2.0_f32, bad_col()));
                    } else if lit {
                        // the right one: a green glow and edge
                        ui.painter().rect_filled(resp.rect.expand(1.0), rad, ok_col().gamma_multiply(0.25));
                        ui.painter().rect_stroke(resp.rect.expand(1.0), rad, Stroke::new(2.0_f32, ok_col()));
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

    /// Every way a chord can be played: each voicing, with and without the bass, with and without the ii-V.
    fn all_opts() -> Vec<ChordOpts> {
        let mut v = Vec::new();
        for voicing in 0..VOICINGS.len() as u8 {
            for no_bass in [false, true] {
                for context in [false, true] {
                    v.push(ChordOpts { voicing, no_bass, context });
                }
            }
        }
        v
    }

    /// The chord a question plays (the last thing it plays).
    fn chord_of(q: &Question) -> Vec<i32> {
        q.parts.last().unwrap().steps[0].clone()
    }

    #[test]
    fn every_chord_is_spelt_right() {
        let mut labels: Vec<&str> = Vec::new();
        for c in CHORDS.iter() {
            assert!(!labels.contains(&c.label), "{} twice", c.label);
            labels.push(c.label);
            assert!(c.level < LEVELS.len());
            // each tone named, no pitch class twice, the needed ones among them (never the root)
            let p = pcs_of(c);
            assert_eq!(p.len(), c.tones.len(), "{}", c.label);
            for t in c.tones {
                assert_ne!(tone_label(c, *t), "?", "{} {}", c.label, t);
            }
            for n in c.need {
                assert!(c.tones.contains(n), "{} needs {}", c.label, n);
                assert_ne!(*n, 0, "{}", c.label);
            }
            let need = needs_of(c);
            if c.shape == Shape::Built {
                assert!(need.len() <= 4, "{}", c.label);
                // the 3rd (or sus note) always; past a triad, the 7th or 6th too
                assert!(need.iter().any(|x| [2, 3, 4, 5].contains(x)), "{}", c.label);
                if c.tones.len() >= 4 {
                    assert!(need.iter().any(|x| [9, 10, 11].contains(x)), "{}", c.label);
                }
            }
            // Backline's chord reader agrees: the same notes, bar the ones a voicing leaves out (the 5th; a rootless
            // or slash sound's root or 9th)
            if c.name != "?" {
                let read = theory::chord_notes(&format!("C{}", c.name)).unwrap_or_else(|| panic!("C{} not read", c.name));
                let rs = set_over(&read.pcs, 0);
                assert!(p.iter().all(|x| rs.contains(x)), "{}: {:?} not all in C{} {:?}", c.label, p, c.name, rs);
                let missing: Vec<i32> = rs.iter().copied().filter(|x| !p.contains(x)).collect();
                let may: &[i32] = if c.shape == Shape::Built { &[7] } else { &[0, 7, 2] };
                assert!(missing.iter().all(|x| may.contains(x)), "{}: lacks {:?}", c.label, missing);
            }
        }
        let sizes: Vec<usize> = (0..LEVELS.len()).map(|l| level_chords(l).len()).collect();
        assert_eq!(sizes, [6, 10, 9, 10, 16]);
        // the ones a jazz player checks first
        let spell = |l: &str| pcs_of(&CHORDS[chord_by_label(l).unwrap()]);
        assert_eq!(spell("-7\u{266D}5"), [0, 3, 6, 10]);
        assert_eq!(spell("\u{00b0}7"), [0, 3, 6, 9]);
        assert_eq!(spell("-\u{0394}7"), [0, 3, 7, 11]);
        assert_eq!(spell("\u{0394}7#5"), [0, 4, 8, 11]);
        assert_eq!(spell("7ALT"), [0, 3, 4, 8, 10]);
        assert_eq!(spell("13\u{266D}9"), [0, 1, 4, 7, 9, 10]);
        assert_eq!(spell("6/9"), [0, 2, 4, 7, 9]);
        assert_eq!(spell("-6/9"), [0, 2, 3, 7, 9]);
        assert_eq!(tone_label(&CHORDS[chord_by_label("\u{00b0}7").unwrap()], 9), "\u{266D}\u{266D}7");
    }

    #[test]
    fn every_voicing_spells_its_chord() {
        for (ci, c) in CHORDS.iter().enumerate() {
            for o in all_opts() {
                for root in 0..12 {
                    for seed in 0..6u64 {
                        let q = build_chord(ci, root, seed * 7919 + 1, o);
                        let chord = chord_of(&q);
                        let have = set_over(&chord, root);
                        let what = format!("{} {:?} on {}: {:?}", c.label, o, root, chord);
                        // only its notes, all the ones it needs
                        assert!(have.iter().all(|x| pcs_of(c).contains(x)), "{}", what);
                        assert!(needs_of(c).iter().all(|x| have.contains(x)), "{}", what);
                        assert!(fits(c, &have), "{}", what);
                        assert_eq!(q.pcs, have);
                        // between E2 and E5 (a little over at most), low to high, no mud, no more than three octaves with the bass
                        assert!(chord.iter().all(|m| (40..=79).contains(m)), "{}", what);
                        assert!(chord.windows(2).all(|w| w[1] > w[0]), "{}", what);
                        assert!(!muddy(&chord), "{}", what);
                        assert!(chord.last().unwrap() - chord[0] <= 36, "{}", what);
                        // the root in the bass when asked for
                        if !o.no_bass {
                            assert_eq!(chord[0].rem_euclid(12), root, "{}", what);
                        }
                        // what's on top: the top note's name in the chord, among the choices
                        let top = *chord.last().unwrap();
                        let t = tone_of(c, top - root).unwrap();
                        assert_eq!(q.tops[q.top], tone_label(c, t), "{}", what);
                        assert_eq!(q.tops.len(), have.len());
                        // the arpeggio is the same notes
                        assert_eq!(q.arp[0].steps.concat(), chord);
                        // in context: a ii-V (or the ii before a dominant) then the chord
                        if o.context {
                            assert_eq!(q.parts.len(), 2);
                            let lead = &q.parts[0].steps;
                            let dominant = have.contains(&10) && !have.contains(&11) && pcs_of(c).contains(&10);
                            if dominant && lead.len() == 1 {
                                assert_eq!(set_over(&lead[0], root + 7), [0, 3, 7, 10], "{}", what);
                            } else {
                                assert_eq!(lead.len(), 2, "{}", what);
                                assert_eq!(lead[0][0].rem_euclid(12), (root + 2).rem_euclid(12));
                                assert_eq!(lead[1][0].rem_euclid(12), (root + 7).rem_euclid(12));
                                assert!(set_over(&lead[1], root + 7).contains(&4) && set_over(&lead[1], root + 7).contains(&10));
                            }
                            assert!(q.parts[1].at >= lead.len() as f32 * q.parts[0].gap);
                        } else {
                            assert_eq!(q.parts.len(), 1);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn drop_voicings_are_real_drop_voicings() {
        for (ci, c) in CHORDS.iter().enumerate() {
            if c.shape != Shape::Built {
                continue;
            }
            for (voicing, from_top) in [(2u8, 2usize), (3, 3)] {
                let o = ChordOpts { voicing, no_bass: true, context: false };
                for root in 0..12 {
                    for seed in 0..8u64 {
                        let v = chord_of(&build_chord(ci, root, seed * 31 + 5, o));
                        let n = v.len();
                        // raise the lowest note an octave: a close voicing, with it the 2nd (3rd) voice from the top
                        let from_top = if n < 4 { 2 } else { from_top };
                        let mut close = v[1..].to_vec();
                        close.push(v[0] + 12);
                        close.sort();
                        assert!(close[n - 1] - close[0] < 12, "{} {:?}", c.label, v);
                        assert_eq!(close[n - from_top], v[0] + 12, "{} drop {} {:?}", c.label, from_top, v);
                    }
                }
            }
        }
        // and they are the shapes the CHORDS tab lists for the 7th chords (some string set, low on the neck)
        for l in level_chords(1) {
            let c = &CHORDS[l];
            let read = theory::chord_notes(&format!("C{}", c.name)).unwrap();
            let groups = theory::voicing_groups(&read, &theory::STANDARD);
            for (voicing, group) in [(2u8, "Drop 2"), (3, "Drop 3")] {
                let shapes: Vec<Vec<i32>> = groups
                    .iter()
                    .find(|g| g.0 == group)
                    .unwrap()
                    .1
                    .iter()
                    .map(|v| {
                        let mut p: Vec<i32> =
                            v.strings.iter().map(|s| theory::STANDARD[*s] + v.frets[*s].unwrap() as i32).collect();
                        p.sort();
                        p.iter().map(|x| x - p[0]).collect()
                    })
                    .collect();
                for seed in 0..12u64 {
                    let v = chord_of(&build_chord(l, 0, seed, ChordOpts { voicing, no_bass: true, context: false }));
                    let shape: Vec<i32> = v.iter().map(|x| x - v[0]).collect();
                    assert!(shapes.contains(&shape), "{} {} {:?}", c.label, group, v);
                }
            }
        }
    }

    #[test]
    fn upper_structures_and_slash_chords_are_what_they_say() {
        // D/C7 = C13#11, Eb/C7 = C7#9, Ab/C7 = C7#5#9 (alt), F#/C7 = C7b9#11; F/G = G9sus, D/C = C lydian
        let cases = [
            ("II/7", "D", "C13#11"),
            ("\u{266D}III/7", "Eb", "C7#9"),
            ("\u{266D}VI/7", "Ab", "C7#5#9"),
            ("#IV/7", "F#", "C7b9#11"),
            ("\u{266D}VII/1", "Bb", "C9sus4"),
            ("II/1", "D", "Cmaj13#11"),
        ];
        for (label, triad, chord) in cases {
            let ci = chord_by_label(label).unwrap();
            let c = &CHORDS[ci];
            let tri = set_over(&theory::chord_notes(triad).unwrap().pcs, 0);
            let (t, under): (i32, Vec<i32>) = match c.shape {
                Shape::Upper(t) => (t, vec![0, 4, 10]),
                Shape::Slash(t) => (t, vec![0]),
                _ => panic!("{}", label),
            };
            assert_eq!(set_over(&[t, t + 4, t + 7], 0), tri, "{}", label);
            let mut want: Vec<i32> = under.iter().chain(tri.iter()).copied().collect();
            want.sort();
            want.dedup();
            assert_eq!(pcs_of(c), want, "{}", label);
            // the chord it makes holds these notes (it may have a 5th, or for D/C a 3rd and 7th, they leave out)
            let read = set_over(&theory::chord_notes(chord).unwrap().pcs, 0);
            assert!(want.iter().all(|x| read.contains(x)), "{} in {}", label, chord);
            for root in 0..12 {
                for no_bass in [false, true] {
                    for seed in 0..10u64 {
                        let q = build_chord(ci, root, seed, ChordOpts { voicing: 5, no_bass, context: false });
                        let v = chord_of(&q);
                        // the triad on top, in close position; under it the 3rd and 7th (and the root), or the root
                        let top3 = &v[v.len() - 3..];
                        assert_eq!(set_over(top3, root + t), [0, 4, 7], "{} {:?}", label, v);
                        assert!(top3[2] - top3[0] < 12);
                        let low = set_over(&v[..v.len() - 3], root);
                        match c.shape {
                            Shape::Upper(_) => {
                                assert!(low.contains(&4) && low.contains(&10), "{} {:?}", label, v);
                                assert!(v[..v.len() - 3].iter().all(|m| *m < top3[0]));
                            }
                            _ => assert_eq!(low, [0]),
                        }
                    }
                }
            }
        }
        // F/G in G, by name: G F A C
        let q = build_chord(chord_by_label("\u{266D}VII/1").unwrap(), 7, 3, ChordOpts::default());
        let v = chord_of(&q);
        assert_eq!(v[0] % 12, 7);
        assert_eq!(set_over(&v[1..], 0), [0, 5, 9]);
        assert!(q.reveal.starts_with("F/G"), "{}", q.reveal);
        let q = build_chord(chord_by_label("II/7").unwrap(), 0, 3, ChordOpts::default());
        assert!(q.reveal.starts_with("D/C7  (C13#11)"), "{}", q.reveal);
        // So What: fourths with a major 3rd on top; on E: E A D G B
        let q = build_chord(chord_by_label("SO WHAT").unwrap(), 4, 1, ChordOpts::default());
        let v = chord_of(&q);
        assert_eq!(v.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>(), [5, 5, 5, 4]);
        assert_eq!(v.iter().map(|m| m % 12).collect::<Vec<_>>(), [4, 9, 2, 7, 11]);
        // rootless voicings: A has the 3rd at the bottom, B the 7th
        let r9 = &CHORDS[chord_by_label("9 ROOTLESS").unwrap()];
        if let Shape::Fixed(list) = r9.shape {
            assert_eq!(list[0][0], 4);
            assert_eq!(list[1][0], 10);
        }
        // shells: R 3 7 and R 7 3
        for (l, want) in [("\u{0394}7 SHELL", [0, 4, 11]), ("-7 SHELL", [0, 3, 10]), ("7 SHELL", [0, 4, 10])] {
            assert_eq!(pcs_of(&CHORDS[chord_by_label(l).unwrap()]), want);
        }
    }

    #[test]
    fn whats_on_top_is_named_right() {
        let o = |voicing| ChordOpts { voicing, no_bass: false, context: false };
        // a 13 spread: R b7 3 13 9, the 9 on top
        let q = build_chord(chord_by_label("13").unwrap(), 0, 1, o(1));
        assert_eq!(chord_of(&q).iter().map(|m| (m - 0).rem_euclid(12)).collect::<Vec<_>>(), [0, 10, 4, 9, 2]);
        assert_eq!(q.tops[q.top], "9");
        assert_eq!(q.tops, ["R", "3", "\u{266D}7", "9", "13"]);
        // a 7#9 spread: R b7 3 #9, the #9 on top (not a b3)
        let q = build_chord(chord_by_label("7#9").unwrap(), 9, 1, o(1));
        assert_eq!(q.tops[q.top], "#9");
        // a dim7 spread: R b5 bb7 b3
        let q = build_chord(chord_by_label("\u{00b0}7").unwrap(), 2, 1, o(1));
        assert_eq!(q.tops[q.top], "\u{266D}3");
        // a minor 7th spread: R 5 b7 b3
        let q = build_chord(chord_by_label("-7").unwrap(), 5, 1, o(1));
        assert_eq!(chord_of(&q).windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>(), [7, 3, 5]);
        assert_eq!(q.tops[q.top], "\u{266D}3");
        // a 7b13 keeps its 5th apart from the b13: R 5 b7 3 b13
        let q = build_chord(chord_by_label("7\u{266D}13").unwrap(), 0, 1, o(1));
        assert_eq!(chord_of(&q).windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>(), [7, 3, 6, 4]);
        assert_eq!(q.tops[q.top], "\u{266D}13");
        // spelt by degree: G7alt's #9 is B flat, its b13 E flat; Eb/G7 is not A#/G7; A7#9's #9 is C
        let q = build_chord(chord_by_label("7ALT").unwrap(), 7, 1, o(1));
        assert!(q.spelt.starts_with("G E\u{266D} F B B\u{266D}"), "{}", q.spelt);
        let q = build_chord(chord_by_label("\u{266D}III/7").unwrap(), 7, 1, o(0));
        assert!(q.reveal.starts_with("B\u{266D}/G7"), "{}", q.reveal);
        let q = build_chord(chord_by_label("7#9").unwrap(), 9, 1, o(1));
        assert!(q.spelt.starts_with("A G C# C"), "{}", q.spelt);
        let q = build_chord(chord_by_label("SO WHAT").unwrap(), 4, 1, o(0));
        assert!(q.reveal.starts_with("E SO WHAT"), "{}", q.reveal);
    }

    #[test]
    fn look_alikes_can_be_told_apart() {
        let mut sets: Vec<Vec<usize>> = ALIKES
            .iter()
            .map(|s| s.iter().map(|l| chord_by_label(l).unwrap_or_else(|| panic!("{} unknown", l))).collect())
            .collect();
        for s in &sets {
            assert!(s.len() >= 3);
            let mut d = s.clone();
            d.sort();
            d.dedup();
            assert_eq!(d.len(), s.len());
        }
        // every level is a fair set too, bar the bandstand sounds that are the same notes on purpose
        for l in 0..4 {
            sets.push(level_chords(l));
        }
        // played any way at all, a chord's notes never also spell another in its set
        for s in &sets {
            for &a in s {
                for o in all_opts() {
                    for root in [0, 5, 10] {
                        for seed in 0..6u64 {
                            let q = build_chord(a, root, seed, o);
                            for &b in s {
                                if b != a {
                                    assert!(
                                        !fits(&CHORDS[b], &q.pcs),
                                        "{} played {:?} also reads as {}",
                                        CHORDS[a].label,
                                        q.pcs,
                                        CHORDS[b].label
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        // ... and what the notes do spell, counts as right: the upper structure D/C7 is a 13#11
        let us = chord_by_label("II/7").unwrap();
        let q = build_chord(us, 0, 1, ChordOpts::default());
        assert!(fits(&CHORDS[chord_by_label("13#11").unwrap()], &q.pcs));
        assert!(answer_right(0, &Question { answer: 0, ..q.clone() }, &[us, chord_by_label("13#11").unwrap()], 1));
    }

    #[test]
    fn the_analyzer_names_them_the_same() {
        // Backline's own chord namer, given each chord's notes, names a chord with exactly those notes
        for c in CHORDS.iter().filter(|c| c.shape == Shape::Built) {
            let p = pcs_of(c);
            let name = theory::built_name(&p).unwrap_or_else(|| panic!("{} {:?} not named", c.label, p));
            let read = theory::chord_notes(&format!("C{}", name)).unwrap_or_else(|| panic!("C{} not read", name));
            assert_eq!(set_over(&read.pcs, 0), p, "{} named C{}", c.label, name);
        }
        // and the analyzer reads a triad or 7th chord played with its root in the bass as that chord
        for l in [0, 1] {
            for ci in level_chords(l) {
                let c = &CHORDS[ci];
                for seed in 0..6u64 {
                    let v = chord_of(&build_chord(ci, 0, seed, ChordOpts { voicing: 5, no_bass: false, context: false }));
                    let names = theory::name_chords(&v, Some(v[0]));
                    assert!(
                        names.iter().any(|n| !n.contains('/')
                            && n.starts_with('C')
                            && theory::chord_notes(n).is_some_and(|r| set_over(&r.pcs, 0) == pcs_of(c))),
                        "{} {:?}: {:?}",
                        c.label,
                        v,
                        names
                    );
                }
            }
        }
    }

    #[test]
    fn a_wrong_answer_says_what_differs() {
        let c = |l: &str| &CHORDS[chord_by_label(l).unwrap()];
        assert_eq!(
            chord_hint(c("7"), c("-7\u{266D}5")),
            "You picked 7: a major 3rd and a natural 5\n-7\u{266D}5: a minor 3rd and a flat 5"
        );
        assert_eq!(chord_hint(c("7\u{266D}9"), c("7#9")), "You picked 7\u{266D}9: a \u{266D}9\n7#9: a #9");
        assert_eq!(chord_hint(c("\u{0394}7"), c("6")), "You picked \u{0394}7: a major 7th\n6: a 6th");
        assert_eq!(
            chord_hint(c("7#11"), c("7\u{266D}5")),
            "You picked 7#11: a natural 5 and a #11\n7\u{266D}5: a flat 5 and no tensions"
        );
        assert_eq!(
            chord_hint(c("\u{0394}7"), c("\u{0394}7 SHELL")),
            "You picked \u{0394}7: a natural 5\n\u{0394}7 SHELL: no 5th"
        );
        // every pair says something: two short lines
        for a in CHORDS.iter() {
            for b in CHORDS.iter() {
                let h = chord_hint(a, b);
                assert!(h.lines().count() == 2 && h.lines().all(|l| l.len() < 80), "{}", h);
            }
        }
        // intervals say how far
        let q = build(1, 0, 52, 1, 0, 99);
        assert_eq!(wrong_hint(1, &q, &[], 1).unwrap(), "You picked MINOR 3RD: 3 half steps\nMAJOR 3RD: 4 half steps");
        // runs of right answers, and the chime after what is still ringing
        assert_eq!(streak_word(5), "ON A ROLL");
        assert_eq!(streak_word(10), "EARS OPEN");
        assert_eq!(streak_word(30), "BANDSTAND READY");
        assert_eq!(streak_word(7), "");
        let quiet = chime_sound(true, 1.0);
        assert!(quiet[..(band::RATE * 0.99) as usize].iter().all(|s| *s == 0));
        assert!(quiet.iter().any(|s| *s != 0));
    }

    #[test]
    fn whats_in_play() {
        let mut st = EarStats::default();
        let mut o = crate::store::EarChords::default();
        // a level starts with four, grows by what you have earned
        assert_eq!(chord_pool(&o, &st), level_chords(1)[..4].to_vec());
        o.levels = vec![3, 0];
        o.grown = vec![6, 0, 0, 6];
        let mut want = level_chords(0);
        want.extend(level_chords(3)[..6].iter());
        assert_eq!(chord_pool(&o, &st), want);
        o.levels.clear();
        assert_eq!(chord_pool(&o, &st), level_chords(1)[..4].to_vec());
        // look-alikes
        o.mode = 1;
        for (k, s) in ALIKES.iter().enumerate() {
            o.alike = k;
            let labels: Vec<&str> = chord_pool(&o, &st).iter().map(|i| CHORDS[*i].label).collect();
            assert_eq!(labels, s.to_vec());
        }
        // your mix-ups: none yet, the first set; then the chords of your top mix-ups
        o.alike = ALIKES.len();
        assert_eq!(chord_pool(&o, &st).len(), ALIKES[0].len());
        st.mixups.insert("7#9>7\u{266D}9".into(), 5);
        st.mixups.insert("-6>-7\u{266D}5".into(), 3);
        st.mixups.insert("13>9".into(), 2);
        st.mixups.insert("13>7".into(), 1);
        st.mixups.insert("NOT A CHORD>7".into(), 9);
        let labels: Vec<&str> = chord_pool(&o, &st).iter().map(|i| CHORDS[*i].label).collect();
        assert_eq!(labels, ["7#9", "7\u{266D}9", "-6", "-7\u{266D}5", "13", "9"]);
        // my set, in the levels' order; fewer than two, a level's first four
        o.mode = 2;
        o.mine = vec!["SO WHAT".into(), "7ALT".into()];
        let labels: Vec<&str> = chord_pool(&o, &st).iter().map(|i| CHORDS[*i].label).collect();
        assert_eq!(labels, ["7ALT", "SO WHAT"]);
        o.mine = vec!["SO WHAT".into()];
        assert_eq!(chord_pool(&o, &st), level_chords(1)[..4].to_vec());
        // a question is one of the pool, its answer its place there
        let pool = level_chords(3);
        for seed in 0..40 {
            let q = ask_chord(&pool, &vec![1.0; pool.len()], ChordOpts::default(), Some(5), seed);
            assert_eq!(pool[q.answer], q.chord);
            assert_eq!(q.root, 5);
        }
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
        assert_eq!(level_of(&EarStats { level: 99, ..Default::default() }, 0), CHORDS.len());
    }
}
