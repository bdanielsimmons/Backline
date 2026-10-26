//! The pure parts of TRIADS, shared by the desktop and the phone: the nearest grip to the last (voice leading),
//! a bass note added under a grip, a grip's name, and a token (a chord, a numeral, 1-7) read as a triad.

use crate::fretboard::grip_notes;
use crate::theory::{self, centre};

/// The grip of these notes closest to `prev` (the fewest frets moved), or to fret `near` when there is none before.
pub(crate) fn closest(options: &[[Option<u8>; 6]], prev: Option<&[Option<u8>; 6]>, near: f32) -> Option<[Option<u8>; 6]> {
    let cost = |g: &[Option<u8>; 6]| -> f32 {
        match prev {
            Some(p) => (0..6)
                .map(|s| match (g[s], p[s]) {
                    (Some(a), Some(b)) => (a as f32 - b as f32).abs(),
                    (Some(_), None) | (None, Some(_)) => 3.0,
                    _ => 0.0,
                })
                .sum::<f32>(),
            None => (centre(g) - near).abs(),
        }
    };
    options.iter().min_by(|a, b| cost(a).partial_cmp(&cost(b)).unwrap_or(std::cmp::Ordering::Equal)).copied()
}

/// A grip moved along the neck by `d` frets (open strings become fretted).
pub(crate) fn shifted(g: &[Option<u8>; 6], d: i32) -> [Option<u8>; 6] {
    let mut out = *g;
    for f in out.iter_mut().flatten() {
        *f = (*f as i32 + d).clamp(0, 17) as u8;
    }
    out
}

/// A bass note added under a grip on the 6th or 5th string (whichever is free and closest to where the grip is).
pub(crate) fn with_bass(g: &[Option<u8>; 6], bass: i32) -> [Option<u8>; 6] {
    let at = centre(g);
    let mut best: Option<(usize, u8, f32)> = None;
    for s in [0usize, 1] {
        if g[s].is_some() || g[..s].iter().any(|x| x.is_some()) {
            continue;
        }
        for f in 0..=15u8 {
            if (theory::STANDARD[s] + f as i32).rem_euclid(12) != bass.rem_euclid(12) {
                continue;
            }
            // an open string is always in reach
            let cost = if f == 0 { 1.0 } else { (f as f32 - at).abs() };
            if best.is_none_or(|b| cost < b.2) {
                best = Some((s, f, cost));
            }
        }
    }
    let mut out = *g;
    if let Some((s, f, _)) = best {
        out[s] = Some(f);
    }
    out
}

/// The name of a grip, read from its notes over its lowest one.
pub(crate) fn grip_name(g: &[Option<u8>; 6], sp: theory::Spelling) -> String {
    let notes = grip_notes(g);
    let pcs: Vec<i32> = notes.iter().map(|m| m.rem_euclid(12)).collect();
    let bass = notes.iter().min().map(|m| m.rem_euclid(12));
    bass.and_then(|b| theory::suggestions(&pcs, Some(b)).first().map(|s| s.name(sp))).unwrap_or_default()
}

/// The triad (root, third or sus note, fifth) in a chord name, or a numeral in the key: 1-7 for the key's own triads
/// (1 major, 2 minor ... 7 diminished), or I, ii, bVII and the rest.
pub(crate) fn triad_of(tok: &str, key: i32) -> Option<(i32, [i32; 3])> {
    if let Ok(n) = tok.parse::<usize>() {
        if (1..=7).contains(&n) {
            const MAJ: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
            let root = (key + MAJ[n - 1]).rem_euclid(12);
            let q = match n {
                2 | 3 | 6 => [0, 3, 7],
                7 => [0, 3, 6],
                _ => [0, 4, 7],
            };
            return Some((root, q));
        }
    }
    let name = theory::chord_notes(tok).map(|_| tok.to_string()).or_else(|| crate::lines::roman_chord(tok, key, false, false))?;
    let cn = theory::chord_notes(&name)?;
    let bones = theory::chord_skeleton(&cn, 3);
    Some((cn.root, [bones[0].rem_euclid(12), bones[1].rem_euclid(12), bones[2].rem_euclid(12)]))
}
