//! The pure parts of the ATONAL lines, shared by the desktop and the phone: the generated lines (alternate triads,
//! interval cycles) and where a line sits under the hand (every string, every finger).

use crate::theory;

/// The lowest and highest notes a generated line uses, as written (G below middle C to the C two octaves up).
pub(crate) const LOW: i32 = 55;
pub(crate) const HIGH: i32 = 84;

/// The root's lowest note at or above LOW.
pub(crate) fn base_of(root: i32) -> i32 {
    LOW + (root - LOW).rem_euclid(12)
}

/// ALTERNATE TRIADS: the scale's triads - each note with the notes two and four places above it in the scale - on
/// every other degree (1 3 5 7, then 2 4 6), the first going up, the next coming down, and so on.
pub(crate) fn alternate_triads(root: i32, steps: &[i32]) -> Vec<i32> {
    let n = steps.len();
    if n < 3 {
        return Vec::new();
    }
    let base = base_of(root);
    let deg = |i: usize| base + steps[i % n] + 12 * (i / n) as i32;
    let order: Vec<usize> = (0..n).step_by(2).chain((1..n).step_by(2)).collect();
    let mut out = Vec::new();
    for (k, d) in order.into_iter().enumerate() {
        let mut t = vec![deg(d), deg(d + 2), deg(d + 4)];
        if k % 2 == 1 {
            t.reverse();
        }
        out.extend(t);
    }
    out
}

/// INTERVAL CYCLE: round a cycle of one interval (`semis`) until it comes home, then again from a half step higher,
/// the next time coming down, and so on to about 24 notes; kept inside the range by an octave when it runs out.
pub(crate) fn interval_cycle(root: i32, semis: i32) -> Vec<i32> {
    let semis = semis.clamp(1, 11);
    let len = (12 / gcd(12, semis)) as usize;
    let rounds = 24_usize.div_ceil(len);
    let mut out = Vec::new();
    for k in 0..rounds {
        let up = k % 2 == 0;
        let start = 60 + (root + k as i32).rem_euclid(12) + if up { 0 } else { 12 };
        let mut m = start.clamp(LOW, HIGH);
        for _ in 0..len {
            out.push(m);
            m = if up { m + semis } else { m - semis };
            if m > HIGH {
                m -= 12;
            }
            if m < LOW {
                m += 12;
            }
        }
    }
    out
}

pub(crate) fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

// ------------------------------------------------------------------ what a line is made of

/// The strings by name, low E first (as the tuning lists them).
pub(crate) const STRING_NAMES: [&str; 6] = ["LOW E", "A", "D", "G", "B", "HIGH E"];

/// Every place on the neck (frets 1 to 20) where a line starting on `first` (sounding) can start: that note in any
/// octave, low E string first.
pub(crate) fn starts(first: i32) -> Vec<(usize, u8)> {
    let mut out = Vec::new();
    for s in 0..6 {
        for f in 1..=20u8 {
            if (theory::STANDARD[s] + f as i32 - first).rem_euclid(12) == 0 {
                out.push((s, f));
            }
        }
    }
    out
}

/// The hand position (the fret under the first finger) when the line's first note at `start` is played with
/// `finger` (1 to 4); none when that would be off the end of the neck.
pub(crate) fn position_of(start: (usize, u8), finger: u8) -> Option<u8> {
    let p = start.1 as i32 - (finger as i32 - 1);
    (p >= 1).then_some(p as u8)
}

/// The line (sounding) played from `start` in position `pos`: moved by octaves so it starts there (a note that
/// would fall off the neck moved an octave back on), every note fingered as close to the position as it goes.
/// The notes as played, where each is, and which were moved an octave.
pub(crate) fn finger_from(notes: &[i32], start: (usize, u8), pos: u8) -> (Vec<i32>, Vec<(usize, u8)>, Vec<bool>) {
    let tuning = &theory::STANDARD;
    let (lo, hi) = (tuning[0], tuning[5] + 22);
    let shift = tuning[start.0] + start.1 as i32 - notes[0];
    let mut played = Vec::with_capacity(notes.len());
    let mut moved = Vec::with_capacity(notes.len());
    for m in notes {
        let mut x = m + shift;
        let was = x;
        while x < lo {
            x += 12;
        }
        while x > hi {
            x -= 12;
        }
        played.push(x);
        moved.push(x != was);
    }
    // the cost of a spot: frets out of the four-fret box cost more the further out (one fret's stretch is cheap)
    let boxed = |sp: (usize, u8)| -> f32 {
        let f = sp.1 as i32;
        let (a, b) = (pos as i32, pos as i32 + 3);
        let out = if f == 0 { (a - 1).max(0) } else if f < a { a - f } else if f > b { f - b } else { 0 };
        match out {
            0 => 0.0,
            1 => 1.0,
            n => 1.0 + (n - 1) as f32 * 4.0,
        }
    };
    let step = |a: (usize, u8), b: (usize, u8)| (a.0 as f32 - b.0 as f32).abs() * 0.3 + (a.1 as f32 - b.1 as f32).abs() * 0.1;
    let opts: Vec<Vec<(usize, u8)>> = played
        .iter()
        .enumerate()
        .map(|(i, m)| {
            if i == 0 {
                return vec![start];
            }
            (0..6).filter_map(|s| {
                let f = m - tuning[s];
                (0..=24).contains(&f).then_some((s, f as u8))
            })
            .collect()
        })
        .collect();
    // best cost to reach each choice of each note, and where it came from
    let mut best: Vec<Vec<(f32, usize)>> = vec![vec![(0.0, 0)]];
    for i in 1..opts.len() {
        let row = opts[i]
            .iter()
            .map(|sp| {
                opts[i - 1]
                    .iter()
                    .enumerate()
                    .map(|(k, q)| (best[i - 1][k].0 + step(*q, *sp) + boxed(*sp), k))
                    .min_by(|a, b| a.0.total_cmp(&b.0))
                    .unwrap_or((0.0, 0))
            })
            .collect();
        best.push(row);
    }
    let mut spots = vec![start; opts.len()];
    let mut k = (0..best[opts.len() - 1].len()).min_by(|a, b| best[opts.len() - 1][*a].0.total_cmp(&best[opts.len() - 1][*b].0)).unwrap_or(0);
    for i in (0..opts.len()).rev() {
        spots[i] = opts[i][k];
        k = best[i][k].1;
    }
    (played, spots, moved)
}

/// The finger for a fret in position `pos`: one finger a fret, a stretch taken by the first or fourth; 0 open.
pub(crate) fn finger_for(fret: u8, pos: u8) -> u8 {
    if fret == 0 {
        0
    } else {
        (fret as i32 - pos as i32 + 1).clamp(1, 4) as u8
    }
}
