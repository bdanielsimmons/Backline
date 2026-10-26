//! Pure music data shared by the desktop and the phone: the scales, intervals and cells the ear drills and the
//! atonal lines use, the exotic scales, and the syllables (moveable and fixed Do) to sing a line on.

use crate::theory;

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

/// FREE INTERVALS: out to a major 10th, the ones easiest to tell apart first: the answer, its name in words,
/// semitones.
pub(crate) const FREE: [(&str, &str, i32); 16] = [
    ("M3", "MAJOR 3RD", 4),
    ("m3", "MINOR 3RD", 3),
    ("P5", "PERFECT 5TH", 7),
    ("P4", "PERFECT 4TH", 5),
    ("P8", "OCTAVE", 12),
    ("M2", "MAJOR 2ND", 2),
    ("m7", "MINOR 7TH", 10),
    ("M6", "MAJOR 6TH", 9),
    ("m2", "MINOR 2ND", 1),
    ("M7", "MAJOR 7TH", 11),
    ("TRITONE", "TRITONE", 6),
    ("m6", "MINOR 6TH", 8),
    ("m9", "MINOR 9TH", 13),
    ("M9", "MAJOR 9TH", 14),
    ("m10", "MINOR 10TH", 15),
    ("M10", "MAJOR 10TH", 16),
];

/// CELLS: the tetrachords, the four-note cells scales are joined from, the ones most unlike each other first: name,
/// semitones over the first note. (The Lydian one is the whole-tone one too: whole, whole, whole.)
pub(crate) const TETRACHORDS: [(&str, [i32; 4]); 8] = [
    ("MAJOR", [0, 2, 4, 5]),
    ("PHRYGIAN", [0, 1, 3, 5]),
    ("LYDIAN", [0, 2, 4, 6]),
    ("HARMONIC", [0, 1, 4, 5]),
    ("MINOR", [0, 2, 3, 5]),
    ("DIMINISHED", [0, 1, 3, 4]),
    ("HUNGARIAN", [0, 2, 3, 6]),
    ("CHROMATIC", [0, 1, 2, 5]),
];

/// CELLS as three-note sets: all twelve, by their prime forms (the most packed way of writing the set, from 0), the
/// ones most unlike each other first.
pub(crate) const TRICHORDS: [(&str, [i32; 3]); 12] = [
    ("0 3 7", [0, 3, 7]),
    ("0 1 2", [0, 1, 2]),
    ("0 4 8", [0, 4, 8]),
    ("0 2 7", [0, 2, 7]),
    ("0 3 6", [0, 3, 6]),
    ("0 2 4", [0, 2, 4]),
    ("0 1 5", [0, 1, 5]),
    ("0 1 6", [0, 1, 6]),
    ("0 1 3", [0, 1, 3]),
    ("0 1 4", [0, 1, 4]),
    ("0 2 5", [0, 2, 5]),
    ("0 2 6", [0, 2, 6]),
];

/// The prime form of a set of pitch classes: its most packed form from 0, the set or turned upside down.
pub(crate) fn prime_form(set: &[i32]) -> Vec<i32> {
    let mut best: Option<Vec<i32>> = None;
    for flip in [false, true] {
        let mut pcs: Vec<i32> = set.iter().map(|x| if flip { (-x).rem_euclid(12) } else { x.rem_euclid(12) }).collect();
        pcs.sort();
        pcs.dedup();
        for k in 0..pcs.len() {
            let mut v: Vec<i32> = pcs.iter().map(|x| (x - pcs[k]).rem_euclid(12)).collect();
            v.sort();
            // packed most to the left: smallest span, then smallest steps from the bottom
            let key = |v: &Vec<i32>| {
                let mut k = vec![*v.last().unwrap()];
                k.extend(v.iter().copied());
                k
            };
            if best.as_ref().map_or(true, |b| key(&v) < key(b)) {
                best = Some(v);
            }
        }
    }
    best.unwrap()
}

/// A scale for the atonal drills: its name, each note as a degree of the major scale (1 to 7) and how far it is moved
/// from it (-1 flat, 1 sharp), and for a scale built from two tetrachords, which (places in TETRACHORDS: the one on
/// the root, the one on the 5th).
pub(crate) struct Exotic {
    pub name: &'static str,
    pub degs: &'static [(usize, i32)],
    pub built: Option<(usize, usize)>,
}

pub(crate) const fn ex(name: &'static str, degs: &'static [(usize, i32)]) -> Exotic {
    Exotic { name, degs, built: None }
}

/// The major scale, the degrees the scales are written against.
pub(crate) const MAJOR_STEPS: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

/// The scales sung in intervals (and the 7-note ones stacked in VERTICALS), in the order they are sung.
pub(crate) const EXOTICS: [Exotic; 12] = [
    ex("DOUBLE HARMONIC", &[(1, 0), (2, -1), (3, 0), (4, 0), (5, 0), (6, -1), (7, 0)]),
    ex("HUNGARIAN MAJOR", &[(1, 0), (2, 1), (3, 0), (4, 1), (5, 0), (6, 0), (7, -1)]),
    // C D# E G Ab B: a minor 3rd and a half step, three times over
    ex("SIX-TONE SYMMETRICAL", &[(1, 0), (2, 1), (3, 0), (5, 0), (6, -1), (7, 0)]),
    ex("ENIGMATIC", &[(1, 0), (2, -1), (3, 0), (4, 1), (5, 1), (6, 1), (7, 0)]),
    ex("LYDIAN DOMINANT", &[(1, 0), (2, 0), (3, 0), (4, 1), (5, 0), (6, 0), (7, -1)]),
    ex("HUNGARIAN MINOR", &[(1, 0), (2, 0), (3, -1), (4, 1), (5, 0), (6, -1), (7, 0)]),
    ex("NEAPOLITAN MAJOR", &[(1, 0), (2, -1), (3, -1), (4, 0), (5, 0), (6, 0), (7, 0)]),
    ex("NEAPOLITAN MINOR", &[(1, 0), (2, -1), (3, -1), (4, 0), (5, 0), (6, -1), (7, 0)]),
    ex("PERSIAN", &[(1, 0), (2, -1), (3, 0), (4, 0), (5, -1), (6, -1), (7, 0)]),
    // built: a tetrachord on the root, another on the 5th (Lydian on G ends on C#, sung as the scale's Db)
    Exotic {
        name: "LYDIAN + LYDIAN",
        degs: &[(1, 0), (2, -1), (2, 0), (3, 0), (4, 1), (5, 0), (6, 0), (7, 0)],
        built: Some((2, 2)),
    },
    Exotic { name: "LYDIAN + HARMONIC", degs: &[(1, 0), (2, 0), (3, 0), (4, 1), (5, 0), (6, -1), (7, 0)], built: Some((2, 3)) },
    Exotic {
        name: "DIMINISHED + DIMINISHED",
        degs: &[(1, 0), (2, -1), (3, -1), (3, 0), (5, 0), (6, -1), (7, -1), (7, 0)],
        built: Some((5, 5)),
    },
];

/// A scale's notes as semitones over its root, low to high.
pub(crate) fn exotic_steps(e: &Exotic) -> Vec<i32> {
    e.degs.iter().map(|(d, a)| MAJOR_STEPS[(d - 1) % 7] + a).collect()
}

/// Chromatic syllables over Do: going up (raised), coming down (lowered).
pub(crate) const RISING: [&str; 12] = ["Do", "Di", "Re", "Ri", "Mi", "Fa", "Fi", "Sol", "Si", "La", "Li", "Ti"];

pub(crate) const FALLING: [&str; 12] = ["Do", "Ra", "Re", "Me", "Mi", "Fa", "Se", "Sol", "Le", "La", "Te", "Ti"];

/// The moveable-Do syllable of a degree (1 to 7) moved by `alt`: Do Re Mi Fa Sol La Ti, raised Di Ri Fi Si Li, lowered
/// Ra Me Se Le Te. None for the ones no singer uses (a sharp 3rd or 7th, a flat root or 4th).
pub(crate) fn degree_syllable(deg: usize, alt: i32) -> Option<&'static str> {
    const PLAIN: [&str; 7] = ["Do", "Re", "Mi", "Fa", "Sol", "La", "Ti"];
    const UP: [&str; 7] = ["Di", "Ri", "", "Fi", "Si", "Li", ""];
    const DOWN: [&str; 7] = ["", "Ra", "Me", "", "Se", "Le", "Te"];
    let row = match alt {
        0 => PLAIN,
        1 => UP,
        -1 => DOWN,
        _ => return None,
    };
    row.get(deg.wrapping_sub(1)).copied().filter(|s| !s.is_empty())
}

/// The fixed-Do syllable of a note written `name` ("C", "F#", "B\u{266D}"): its letter's syllable, raised or lowered
/// as written. A note there is no syllable for (E#, B#, F\u{266D}, C\u{266D}) is sung as the note it sounds.
pub(crate) fn fixed_syllable(name: &str) -> &'static str {
    let name = name.replace('\u{266D}', "b");
    let letter = "CDEFGAB".find(name.chars().next().unwrap_or('C')).unwrap_or(0);
    let alt = if name.ends_with('#') {
        1
    } else if name.len() > 1 && name.ends_with('b') {
        -1
    } else {
        0
    };
    degree_syllable(letter + 1, alt).unwrap_or_else(|| {
        let pc = (MAJOR_STEPS[letter] + alt).rem_euclid(12) as usize;
        if alt > 0 {
            RISING[pc]
        } else {
            FALLING[pc]
        }
    })
}

/// A note name for jazz eyes (flat sign, not "b").
pub(crate) fn nm(pc: i32, sharps: bool) -> String {
    theory::note_name(pc, sharps).replace('b', "\u{266D}")
}

/// Each note of a line spelt the way it moves (sharps going up, flats coming down) with the syllable to sing it on:
/// moveable Do from the first note (Di Ri Fi Si Li going up, Ra Me Se Le Te coming down), fixed Do by its name, or La.
pub(crate) fn line_words(line: &[i32], syl: u8) -> Vec<(String, &'static str)> {
    line.iter()
        .enumerate()
        .map(|(k, m)| {
            let up = k > 0 && *m > line[k - 1];
            let name = nm(*m, up);
            let s = match syl {
                0 => (if up { RISING } else { FALLING })[(m - line[0]).rem_euclid(12) as usize],
                1 => fixed_syllable(&name),
                _ => "La",
            };
            (name, s)
        })
        .collect()
}
