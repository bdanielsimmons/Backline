//! The improvising tabs under LINES. PENTATONICS: the five boxes of a pentatonic scale in any key (minor or major),
//! one at a time or across the whole neck, with the blue note and the in-between notes to add, the degrees, notes
//! or fingers on the dots, and exercises written out in notation and tab and played - alone, over a drone, a chord
//! or the band. Everything is worked out from the scale; nothing is a stored diagram.

use super::*;
use crate::lines;
use crate::theory;

/// A place on the neck: (string, 0 = low E ... 5 = high e; fret).
pub(crate) type Spot = (usize, u8);

pub(crate) const PENT_MINOR: [i32; 5] = [0, 3, 5, 7, 10];
pub(crate) const PENT_MAJOR: [i32; 5] = [0, 2, 4, 7, 9];
/// The scales the pentatonics sit in, for the in-between notes: natural minor, major.
const PARENT_MINOR: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];
const PARENT_MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
/// The highest fret drawn.
const TOP_FRET: u8 = 17;

/// A scale's positions with `nps` notes on every string: position k starts on the scale's k-th note on the low E
/// string (position 1 on the root) and climbs the scale, `nps` notes to a string. With 2 to a string a pentatonic
/// gives the five boxes. Positions that would run off the neck are taken an octave down.
pub(crate) fn positions(root: i32, steps: &[i32], nps: usize) -> Vec<Vec<Spot>> {
    let open = theory::STANDARD;
    let r0 = (root - open[0]).rem_euclid(12);
    (0..steps.len())
        .map(|k| {
            // the scale climbing from its k-th note on the low E string
            let start = open[0] + r0 + steps[k];
            let mut pitches: Vec<i32> = Vec::new();
            let mut oct = 0;
            let mut i = k;
            while pitches.len() < nps * 6 {
                pitches.push(open[0] + r0 + steps[i] + 12 * oct);
                i += 1;
                if i == steps.len() {
                    i = 0;
                    oct += 1;
                }
            }
            debug_assert_eq!(pitches[0], start);
            let mut spots: Vec<(usize, i32)> = pitches.iter().enumerate().map(|(j, p)| (j / nps, p - open[j / nps])).collect();
            // off the top of the neck: an octave down, when that still fits
            let hi = spots.iter().map(|s| s.1).max().unwrap_or(0);
            let lo = spots.iter().map(|s| s.1).min().unwrap_or(0);
            if hi > TOP_FRET as i32 && lo >= 12 {
                for s in spots.iter_mut() {
                    s.1 -= 12;
                }
            }
            spots.into_iter().filter(|s| s.1 >= 0).map(|(s, f)| (s, f as u8)).collect()
        })
        .collect()
}

/// The finger for each spot of a position: its lowest fret is the first finger, a fret a finger from there (the
/// fourth finger stretches when a position spans five frets).
pub(crate) fn fingers(pos: &[Spot]) -> Vec<u8> {
    let f0 = pos.iter().map(|s| s.1).min().unwrap_or(0);
    pos.iter().map(|s| [1u8, 2, 3, 4, 4][((s.1 - f0) as usize).min(4)]).collect()
}

/// Where a box's lowest root is: "ROOT ON 6TH STRING, 1ST FINGER".
pub(crate) fn root_place(pos: &[Spot], root: i32) -> String {
    const STRINGS: [&str; 6] = ["6TH", "5TH", "4TH", "3RD", "2ND", "1ST"];
    const FINGERS: [&str; 5] = ["", "1ST", "2ND", "3RD", "4TH"];
    let fing = fingers(pos);
    match pos.iter().enumerate().find(|(_, s)| (theory::STANDARD[s.0] + s.1 as i32 - root).rem_euclid(12) == 0) {
        Some((k, s)) => format!("ROOT ON {} STRING, {} FINGER", STRINGS[s.0], FINGERS[fing[k] as usize]),
        None => "NO ROOT IN IT".to_string(),
    }
}

/// NUMBERING: which position is number 1 - the one you picked, kept by where its root sits (so it means the same in
/// any key, scale or family, and each tab keeps its own), else your usual one (marked in the list: the first of
/// `usual` found). Returns its index in `about`, and whether it was just changed.
pub(crate) fn numbering_line(ui: &mut egui::Ui, id: &str, about: &[String], saved: &mut String, usual: &[&str]) -> (usize, bool) {
    let def = usual.iter().find_map(|u| about.iter().position(|a| a.starts_with(u))).unwrap_or(0);
    let cur = about.iter().position(|a| !saved.is_empty() && a == saved).unwrap_or(def);
    let labels: Vec<String> =
        about.iter().enumerate().map(|(i, a)| format!("1 = {}{}", a, if i == def { "  (USUAL)" } else { "" })).collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    match crate::choice_line(ui, id, "NUMBERING", &labels, cur, 360.0) {
        Some(i) => {
            // the usual one picked: follow it (nothing kept)
            *saved = if i == def { String::new() } else { about[i].clone() };
            (i, true)
        }
        None => (cur, false),
    }
}

/// A degree's name in a scale (4, not 11; b6, not b13).
fn degree_name(pc: i32, root: i32) -> &'static str {
    const N: [&str; 12] = ["R", "\u{266D}2", "2", "\u{266D}3", "3", "4", "\u{266D}5", "5", "\u{266D}6", "6", "\u{266D}7", "7"];
    N[(pc - root).rem_euclid(12) as usize]
}

/// The exercises: (id, name, what it does); some only for some scales.
const EXERCISES: [(u8, &str, &str); 15] = [
    (0, "UP & DOWN", "The box up and down: say each note's degree as you play it"),
    (1, "4-NOTE SEQUENCE", "Four up, back three, four up again - then the same coming down"),
    (2, "3-NOTE SEQUENCE", "Three up, back two, three up again - then the same coming down"),
    (3, "SKIP ONE", "Skip a note, come back one: wider intervals, an open sound"),
    (4, "SKIP TWO", "Skip two notes: wider still"),
    (5, "ONE STRING", "The scale along one string, up the neck and back - play it with one finger, then with slides"),
    (6, "TWO STRINGS", "The scale on two strings up the neck, position by position - try double stops"),
    (7, "CONNECT TWO POSITIONS", "Up this position, down the next one along: the change of position on each string"),
    (8, "ALTERNATE THE TRIADS", "One triad, then the other, through their inversions up the position and back down"),
    (9, "COLTRANE CELLS", "Four-note cells (1 2 3 5, or 1 b3 4 5 over minor) from the root, then from the 5th, up and back"),
    (10, "COLTRANE FROM THE 5TH", "The same cells starting on the 5th and the 9th: a brighter colour over the chord"),
    (11, "APPROACH FROM BELOW", "Each note of the position reached from a half step below - then from above coming down"),
    (12, "ENCLOSURES", "Each target note enclosed: the scale note above, the half step below, then the target"),
    (13, "CHROMATIC PASSING", "Every whole step and wider filled in chromatically: a line that never stops moving"),
    (14, "SKIP ONE (DOWN FIRST)", "The skips coming down first, then back up"),
];

/// A scale the tab offers: its name, steps from the key, the chord it is played over, what it suits, and for a
/// triad pair the two triads (their notes from the key).
pub(crate) struct SmallScale {
    pub name: String,
    pub steps: Vec<i32>,
    pub chord: Vec<i32>,
    pub about: String,
    pub triads: Option<([i32; 3], [i32; 3])>,
}

/// The scales of a family in this key: 0 pentatonic, 1 hexatonic, 2 diminished.
pub(crate) fn family_scales(fam: u8, root: i32, sharps: bool) -> Vec<SmallScale> {
    let n = |off: i32| theory::note_name(root + off, sharps).to_string();
    let tri = |off: i32, minor: bool| -> [i32; 3] { [off, off + if minor { 3 } else { 4 }, off + 7].map(|x| x.rem_euclid(12)) };
    let pair = |a: (i32, bool), b: (i32, bool), chord: &[i32], about: String| -> SmallScale {
        let (t1, t2) = (tri(a.0, a.1), tri(b.0, b.1));
        let mut steps: Vec<i32> = t1.iter().chain(t2.iter()).copied().collect();
        steps.sort();
        steps.dedup();
        SmallScale {
            name: format!("TRIAD PAIR {}{} + {}{}", n(a.0), if a.1 { "m" } else { "" }, n(b.0), if b.1 { "m" } else { "" }),
            steps,
            chord: chord.to_vec(),
            about,
            triads: Some((t1, t2)),
        }
    };
    let plain = |name: &str, steps: &[i32], chord: &[i32], about: String| SmallScale {
        name: name.to_string(),
        steps: steps.to_vec(),
        chord: chord.to_vec(),
        about,
        triads: None,
    };
    match fam {
        1 => vec![
            pair((0, false), (2, false), &[0, 4, 7, 11, 18], format!("LYDIAN: OVER {}MAJ7#11 - ALSO {}M7 AND {}7SUS", n(0), n(9), n(2))),
            pair((5, false), (7, false), &[0, 7, 14, 17], format!("DIATONIC, NO 3RD TO CLASH: OVER {}MAJ7, {}M7, {}7SUS, {}M7", n(0), n(2), n(7), n(9))),
            pair((2, true), (4, true), &[0, 4, 7, 11, 14], format!("OVER {}MAJ7 AND {}7SUS - THE 9TH AND 13TH UP TOP", n(0), n(7))),
            pair((6, false), (8, false), &[0, 4, 10, 15, 20], format!("ALTERED: OVER {}7ALT - THE {}9 #9 #11 {}13", n(0), "\u{266D}", "\u{266D}")),
            pair((0, false), (6, false), &[0, 4, 7, 10, 13], format!("FROM THE DIMINISHED SCALE: OVER {}7({}9 #11)", n(0), "\u{266D}")),
            pair((0, false), (8, true), &[0, 4, 8, 11], format!("THE AUGMENTED SCALE: OVER {}MAJ7#5 - IT REPEATS EVERY MAJOR 3RD", n(0))),
            plain("MAJOR HEXATONIC  (R 2 3 5 6 7)", &[0, 2, 4, 7, 9, 11], &[0, 4, 7, 11], format!("THE MAJOR SCALE WITHOUT ITS 4TH: OVER {}MAJ7, NOTHING TO AVOID", n(0))),
            plain("MINOR HEXATONIC  (R 2 \u{266D}3 4 5 \u{266D}7)", &[0, 2, 3, 5, 7, 10], &[0, 3, 7, 10], format!("DORIAN WITHOUT ITS 6TH: OVER {}M7 - THE PENTATONIC WITH A 9TH", n(0))),
            plain("WHOLE TONE", &[0, 2, 4, 6, 8, 10], &[0, 4, 8, 10], format!("OVER {}7#5 - SYMMETRIC: EVERY NOTE A WHOLE STEP APART, TWO OF THEM IN ALL", n(0))),
        ],
        2 => vec![
            plain("HALF-WHOLE", &[0, 1, 3, 4, 6, 7, 9, 10], &[0, 4, 7, 10, 13], format!("OVER {}7({}9): HALF STEP, WHOLE STEP, ROUND AND ROUND - IT REPEATS EVERY MINOR 3RD", n(0), "\u{266D}")),
            plain("WHOLE-HALF", &[0, 2, 3, 5, 6, 8, 9, 11], &[0, 3, 6, 9], format!("OVER {}DIM7: WHOLE STEP, HALF STEP - IT REPEATS EVERY MINOR 3RD", n(0))),
        ],
        // a scale with one chromatic note added between two of its own, so a line of eighths that starts on a chord
        // tone on the beat keeps landing chord tones on the beats
        3 => vec![
            plain("DOMINANT  (R 2 3 4 5 6 \u{266D}7 7)", &[0, 2, 4, 5, 7, 9, 10, 11], &[0, 4, 7, 10], format!("OVER {}7: THE MAJOR 7TH PASSES BETWEEN THE \u{266D}7 AND THE ROOT - DOWN FROM A CHORD TONE, CHORD TONES FALL ON THE BEATS", n(0))),
            plain("MAJOR  (R 2 3 4 5 #5 6 7)", &[0, 2, 4, 5, 7, 8, 9, 11], &[0, 4, 7, 11], format!("OVER {}MAJ7 AND {}6: THE #5 PASSES BETWEEN THE 5TH AND 6TH - THE {}6 CHORD AND A DIMINISHED CHORD TAKING TURNS", n(0), n(0), n(0))),
            plain("MINOR  (R 2 \u{266D}3 3 4 5 6 \u{266D}7)", &[0, 2, 3, 4, 5, 7, 9, 10], &[0, 3, 7, 10], format!("OVER {}M7: THE 3RD PASSES BETWEEN THE \u{266D}3 AND THE 4TH - DORIAN WITH ONE NOTE MORE", n(0))),
            plain("MINOR 6  (R 2 \u{266D}3 4 5 #5 6 7)", &[0, 2, 3, 5, 7, 8, 9, 11], &[0, 3, 7, 9], format!("OVER {}M6 AND {}M(MAJ7): MELODIC MINOR WITH THE #5 PASSING - THE {}M6 CHORD AND A DIMINISHED CHORD TAKING TURNS", n(0), n(0), n(0))),
            plain("DOMINANT 7SUS  (R 2 3 4 5 6 \u{266D}7 7)", &[0, 2, 4, 5, 7, 9, 10, 11], &[0, 5, 7, 10], format!("OVER {}7SUS: THE SAME NOTES AS THE DOMINANT ONE, THE 4TH NOW THE CHORD TONE AND THE 3RD PASSING", n(0))),
        ],
        // pentatonics from Japan, Bali, Egypt and the jazz players' own
        4 => vec![
            plain("HIRAJOSHI  (R 2 \u{266D}3 5 \u{266D}6)", &[0, 2, 3, 7, 8], &[0, 3, 7], format!("JAPANESE: DARK AND OPEN - OVER {}M(ADD9) OR {}M6 WITH A FLAT 6TH", n(0), n(0))),
            plain("IN SEN  (R \u{266D}2 4 5 \u{266D}7)", &[0, 1, 5, 7, 10], &[0, 5, 7, 10], format!("JAPANESE: THE KOTO'S SOUND - OVER {}7SUS4 WITH A FLAT 9TH, OR {}PHRYGIAN", n(0), n(0))),
            plain("IWATO  (R \u{266D}2 4 \u{266D}5 \u{266D}7)", &[0, 1, 5, 6, 10], &[0, 3, 6, 10], format!("JAPANESE, THE DARKEST: OVER {}M7(\u{266D}5)", n(0))),
            plain("KUMOI  (R 2 \u{266D}3 5 6)", &[0, 2, 3, 7, 9], &[0, 3, 7, 9], format!("JAPANESE, MELODIC MINOR'S PENTATONIC: OVER {}M6 AND {}M(MAJ7)", n(0), n(0))),
            plain("PELOG  (R \u{266D}2 \u{266D}3 5 \u{266D}6)", &[0, 1, 3, 7, 8], &[0, 3, 7], format!("BALINESE GAMELAN: OVER {}PHRYGIAN - EERIE, SHIMMERING", n(0))),
            plain("EGYPTIAN  (R 2 4 5 \u{266D}7)", &[0, 2, 5, 7, 10], &[0, 5, 7, 10], format!("SUSPENDED: NO 3RD AT ALL - OVER {}7SUS4 AND {}M11", n(0), n(0))),
            plain("DOMINANT  (R 2 3 5 \u{266D}7)", &[0, 2, 4, 7, 10], &[0, 4, 7, 10], format!("THE 9TH CHORD AS A SCALE: OVER {}9 - NO 4TH TO AVOID", n(0))),
            plain("MINOR 6  (R \u{266D}3 4 5 6)", &[0, 3, 5, 7, 9], &[0, 3, 7, 9], format!("DORIAN'S PENTATONIC: OVER {}M6 AND {}M13", n(0), n(0))),
            plain("MAJOR \u{266D}6  (R 2 3 5 \u{266D}6)", &[0, 2, 4, 7, 8], &[0, 4, 7], format!("OVER {}7(\u{266D}13) AND {}(ADD \u{266D}6): MAJOR WITH A DARK EDGE", n(0), n(0))),
            plain("ALTERED  (R \u{266D}2 3 \u{266D}5 \u{266D}7)", &[0, 1, 4, 6, 10], &[0, 4, 6, 10], format!("OVER {}7ALT: THE \u{266D}9 AND \u{266D}5 IN A FIVE-NOTE SHAPE", n(0))),
        ],
        // six-note scales beyond the major and minor hexatonics
        5 => vec![
            plain("BLUES  (R \u{266D}3 4 \u{266D}5 5 \u{266D}7)", &[0, 3, 5, 6, 7, 10], &[0, 3, 7, 10], format!("THE MINOR PENTATONIC WITH THE BLUE NOTE: OVER {}7 AND {}M7", n(0), n(0))),
            plain("MAJOR BLUES  (R 2 \u{266D}3 3 5 6)", &[0, 2, 3, 4, 7, 9], &[0, 4, 7, 9], format!("THE MAJOR PENTATONIC WITH THE BLUE NOTE: OVER {}6 AND {}7 - COUNTRY AND GOSPEL", n(0), n(0))),
            plain("PROMETHEUS  (R 2 3 #4 6 \u{266D}7)", &[0, 2, 4, 6, 9, 10], &[0, 4, 10, 18], format!("SCRIABIN'S MYSTIC CHORD: OVER {}7(#11) AND {}13(#11)", n(0), n(0))),
            plain("TRITONE  (R \u{266D}2 3 \u{266D}5 5 \u{266D}7)", &[0, 1, 4, 6, 7, 10], &[0, 4, 7, 10], format!("TWO MAJOR TRIADS A TRITONE APART: OVER {}7(\u{266D}9 #11) - THE SUB INSIDE", n(0))),
            plain("AUGMENTED  (R #2 3 5 #5 7)", &[0, 3, 4, 7, 8, 11], &[0, 4, 8, 11], format!("TWO AUGMENTED TRIADS: OVER {}MAJ7#5 - SYMMETRIC, EVERY MAJOR 3RD", n(0))),
        ],
        // four notes: the Coltrane cells as scales, sparse and strong
        6 => vec![
            plain("1 2 3 5", &[0, 2, 4, 7], &[0, 4, 7], format!("COLTRANE'S CELL AS A SCALE: OVER {} AND {}MAJ7", n(0), n(0))),
            plain("1 \u{266D}3 4 5", &[0, 3, 5, 7], &[0, 3, 7], format!("THE MINOR CELL: OVER {}M7 - THE PENTATONIC'S CORE", n(0))),
            plain("1 2 4 5", &[0, 2, 5, 7], &[0, 5, 7], format!("SUSPENDED, NO 3RD: OVER {}SUS AND {}7SUS4", n(0), n(0))),
            plain("1 3 5 \u{266D}7", &[0, 4, 7, 10], &[0, 4, 7, 10], format!("THE DOMINANT ARPEGGIO AS A SCALE: OVER {}7", n(0))),
        ],
        _ => vec![
            plain("MINOR  (R \u{266D}3 4 5 \u{266D}7)", &PENT_MINOR, &[0, 3, 7, 10], format!("OVER {}M7 - AND OVER A {}7 OR {} BLUES", n(0), n(0), n(0))),
            plain("MAJOR  (R 2 3 5 6)", &PENT_MAJOR, &[0, 4, 7, 9], format!("OVER {} AND {}6 - THE SAME NOTES AS {} MINOR", n(0), n(0), n(9))),
        ],
    }
}

/// Where a pitch is best played around a position: the string whose fret falls inside (or nearest) its frets.
fn place_near(pitch: i32, pos: &[Spot]) -> Option<Spot> {
    let lo = pos.iter().map(|s| s.1).min().unwrap_or(0) as i32;
    let hi = pos.iter().map(|s| s.1).max().unwrap_or(0) as i32;
    (0..6)
        .filter_map(|s| {
            let f = pitch - theory::STANDARD[s];
            (0..=TOP_FRET as i32).contains(&f).then_some((s, f))
        })
        .min_by_key(|(s, f)| {
            let off = if *f < lo {
                lo - f
            } else if *f > hi {
                f - hi
            } else {
                0
            };
            // inside the position first; then the string the position already uses for that height
            off * 10 + (*s as i32 - 2).abs()
        })
        .map(|(s, f)| (s, f as u8))
}

impl App {
    /// PENTATONICS & MORE: the positions of a small scale in any key - pentatonics, hexatonics (triad pairs among
    /// them), the diminished scales - with labels, the spices, and exercises to play over the scale's chord.
    pub(crate) fn pentatonics(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Small scales and how they lie on the neck, in any key. Learn one position at a time - where its roots are, which finger plays them - then see how they join up the whole neck. Pick what to play it over and hear what each one does.", pal().ink2);
        let sharps = lines::key_sharps(self.pn_root);
        if let Some(pc) = Self::note_row(ui, "KEY", self.pn_root, sharps) {
            self.pn_root = pc;
        }
        if let Some(i) = crate::choice_line(
            ui,
            "pn_fam",
            "FAMILY",
            &[
                "PENTATONIC  (5 NOTES)",
                "HEXATONIC  (6 NOTES)",
                "DIMINISHED  (8 NOTES)",
                "CHROMATIC OPTIONS  (8 NOTES)",
                "EXOTIC PENTATONICS  (5 NOTES)",
                "MORE HEXATONICS  (6 NOTES)",
                "FOUR-NOTE SCALES",
            ],
            self.pn_fam as usize,
            260.0,
        ) {
            if i as u8 != self.pn_fam {
                self.pn_fam = i as u8;
                self.pn_scale = 0;
                self.pn_box = 0;
                // triad pairs and the diminished scales lie best three to a string
                self.pn_nps = if i == 0 { 2 } else { 3 };
            }
        }
        let root = self.pn_root;
        let scales = family_scales(self.pn_fam, root, sharps);
        let sc_i = (self.pn_scale as usize).min(scales.len() - 1);
        let names: Vec<String> = scales.iter().map(|s| s.name.replace("\u{266D}", "\u{266D}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        if let Some(i) = crate::choice_line(ui, "pn_scale_i", "SCALE", &names, sc_i, 300.0) {
            self.pn_scale = i as u8;
            self.pn_box = 0;
        }
        let sc_i = (self.pn_scale as usize).min(scales.len() - 1);
        let sc = &scales[sc_i];
        crate::views::dim_line(ui, &sc.about.replace("\u{266D}", "\u{266D}"), 1.0, pal().ink2);
        if let Some(i) = crate::choice_line(
            ui,
            "pn_nps",
            "NOTES A STRING",
            &["2 TO A STRING", "3 TO A STRING"],
            (self.pn_nps as usize).clamp(2, 3) - 2,
            220.0,
        ) {
            self.pn_nps = i as u8 + 2;
            self.pn_box = 0;
        }
        let pent = self.pn_fam == 0;
        let major_pent = pent && sc_i == 1;
        let steps: Vec<i32> = sc.steps.clone();
        let parent: &[i32] = if major_pent { &PARENT_MAJOR } else { &PARENT_MINOR };
        let nps = (self.pn_nps as usize).clamp(2, 3);
        let boxes = positions(root, &steps, nps);
        let nb = boxes.len();
        let word = if pent && nps == 2 { "BOX" } else { "POSITION" };
        // each position by where its lowest root is; number 1 is the one you choose, the rest count on up the neck
        let about: Vec<String> = boxes.iter().map(|b| root_place(b, root)).collect();
        let (first, changed) =
            numbering_line(ui, "pn_num", &about, &mut self.pn_num, &["ROOT ON 6TH STRING, 1ST", "ROOT ON 6TH"]);
        if changed {
            self.dirty = true;
        }
        let shown: Vec<String> = (0..nb)
            .map(|k| format!("{} {}  -  {}", word, k + 1, about[(first + k) % nb]))
            .chain(std::iter::once(format!("WHOLE NECK  -  ALL {} TOGETHER", nb)))
            .collect();
        let shown: Vec<&str> = shown.iter().map(String::as_str).collect();
        let pick = (self.pn_box as usize).min(nb);
        if let Some(i) = crate::choice_line(ui, "pn_box", "SHOW", &shown, pick, 440.0) {
            self.pn_box = i as u8;
        }
        let pick = (self.pn_box as usize).min(nb);
        if let Some(i) =
            crate::choice_line(ui, "pn_label", "LABELS", &["DEGREES", "NOTES", "FINGERS"], self.pn_label as usize, 260.0)
        {
            self.pn_label = i as u8;
        }
        if pent {
            ui.horizontal_wrapped(|ui| {
                crate::views::label(ui, "ADD", 100.0);
                let blue = if major_pent { "BLUE NOTE (\u{266D}3)" } else { "BLUE NOTE (\u{266D}5)" };
                if crate::check_box(ui, blue, self.pn_blue).tip("The blues scale: the pentatonic with the blue note added").clicked() {
                    self.pn_blue = !self.pn_blue;
                }
                if crate::check_box(ui, "IN-BETWEEN NOTES", self.pn_between)
                    .tip("A note between the two of the box on each string - from the scale when it fits, else chromatic. Passing tones: resolve them")
                    .clicked()
                {
                    self.pn_between = !self.pn_between;
                }
            });
        }
        // the position shown, as the scale lists them (nb: the whole neck)
        let sel = if pick >= nb { nb } else { (first + pick) % nb };
        // a triad pair: each triad in its own colour
        let triad_col = |pc: i32| -> Option<Color32> {
            let (a, b) = sc.triads?;
            let r = (pc - root).rem_euclid(12);
            if a.contains(&r) {
                Some(Color32::from_rgb(95, 168, 211))
            } else if b.contains(&r) {
                Some(Color32::from_rgb(217, 164, 65))
            } else {
                None
            }
        };

        // the exercise, and the note sounding in it
        ui.add_space(6.0);
        let (spots, ex_about) = self.small_exercise(&boxes, sel.min(nb - 1), root, sc);
        let notes: Vec<i32> = spots.iter().map(|(s, f)| theory::STANDARD[*s] + *f as i32).collect();
        let lit = self.seq_now("line").and_then(|i| spots.get(i).copied());

        // ---- the neck
        let mut dots: Vec<NeckDot> = Vec::new();
        let label_of = |s: Spot, finger: Option<u8>| -> String {
            let pc = (theory::STANDARD[s.0] + s.1 as i32).rem_euclid(12);
            match self.pn_label {
                1 => theory::note_name(pc, sharps).to_string(),
                2 => finger.map_or(String::new(), |f| f.to_string()),
                _ => degree_name(pc, root).to_string(),
            }
        };
        for (b, pos) in boxes.iter().enumerate() {
            let on = sel == nb || sel == b;
            let fing = fingers(pos);
            for (k, s) in pos.iter().enumerate() {
                let pc = (theory::STANDARD[s.0] + s.1 as i32).rem_euclid(12);
                // a spot two positions share: drawn once, bright if either is shown
                if let Some(d) = dots.iter_mut().find(|d| d.spot == *s) {
                    if on {
                        d.strength = 1.0;
                        d.text = label_of(*s, Some(fing[k]));
                    }
                    continue;
                }
                let col = triad_col(pc).unwrap_or_else(|| crate::fretboard::degree(pc, root).1);
                dots.push(NeckDot {
                    spot: *s,
                    text: if on { label_of(*s, Some(fing[k])) } else { String::new() },
                    col,
                    strength: if on { 1.0 } else { 0.22 },
                    ring: false,
                    root: pc == root.rem_euclid(12),
                });
            }
        }
        // the spices, around the box shown (or everywhere on the whole neck)
        if pent {
            let span = |s: usize| -> (u8, u8) {
                if sel == nb {
                    return (0, TOP_FRET);
                }
                let on: Vec<u8> = boxes[sel].iter().filter(|x| x.0 == s).map(|x| x.1).collect();
                (on.iter().copied().min().unwrap_or(0), on.iter().copied().max().unwrap_or(0))
            };
            let blue_pc = (root + if major_pent { 3 } else { 6 }).rem_euclid(12);
            for s in 0..6 {
                let (lo, hi) = span(s);
                for f in lo..=hi {
                    let m = theory::STANDARD[s] + f as i32;
                    let pc = m.rem_euclid(12);
                    let between = self.pn_between && sel < nb && {
                        // one note between the box's two on this string: from the scale if one fits, else the middle
                        let fits: Vec<u8> = (lo + 1..hi)
                            .filter(|g| parent.contains(&(theory::STANDARD[s] + *g as i32 - root).rem_euclid(12)))
                            .collect();
                        match fits.first() {
                            Some(g) => *g == f,
                            None => hi > lo + 1 && f == (lo + hi) / 2,
                        }
                    };
                    let blue = self.pn_blue && pc == blue_pc;
                    if (blue || between) && !dots.iter().any(|d| d.spot == (s, f)) {
                        let col = if blue { Color32::from_rgb(90, 150, 255) } else { pal().dim };
                        let text = match self.pn_label {
                            1 => theory::note_name(pc, sharps).to_string(),
                            2 => String::new(),
                            _ => degree_name(pc, root).to_string(),
                        };
                        dots.push(NeckDot { spot: (s, f), text, col, strength: 1.0, ring: true, root: false });
                    }
                }
            }
        }
        neck_view(ui, &dots, lit);
        let keys: Vec<i32> = match lit {
            Some(s) => vec![theory::STANDARD[s.0] + s.1 as i32],
            None => steps.iter().map(|x| 48 + root.rem_euclid(12) + x).collect(),
        };
        look_piano(ui, "pn_piano", &keys, Some(root));
        if let Some((a, b)) = sc.triads {
            let names = |t: [i32; 3]| t.iter().map(|x| theory::note_name(root + x, sharps)).collect::<Vec<_>>().join(" ");
            crate::views::dim_line(ui, &format!("BLUE: {}     ORANGE: {}", names(a), names(b)), 1.0, pal().ink2);
        }
        // where the position starts: the lowest root, its string and finger
        if sel < nb {
            crate::views::dim_line(ui, &format!("{} {}: {}", word, pick + 1, about[sel]), 1.0, pal().ink2);
        }

        // ---- the exercise
        ui.add_space(8.0);
        section_header(ui, "EXERCISE");
        let offered: Vec<&(u8, &str, &str)> =
            EXERCISES.iter().filter(|e| (e.0 != 8 || sc.triads.is_some()) && e.0 != 9 && e.0 != 10).collect();
        if !offered.iter().any(|e| e.0 == self.pn_ex) {
            self.pn_ex = offered[0].0;
        }
        let names: Vec<&str> = offered.iter().map(|e| e.1).collect();
        let cur = offered.iter().position(|e| e.0 == self.pn_ex).unwrap_or(0);
        if let Some(i) = crate::choice_line(ui, "pent_ex", "PLAY", &names, cur, 260.0) {
            self.pn_ex = offered[i].0;
        }
        // which string(s) for the one- and two-string exercises
        if self.pn_ex == 5 {
            const ONE: [&str; 6] = ["6TH (LOW E)", "5TH (A)", "4TH (D)", "3RD (G)", "2ND (B)", "1ST (HIGH E)"];
            if let Some(i) = crate::choice_line(ui, "pent_one", "STRING", &ONE, (self.pn_string as usize).min(5), 260.0) {
                self.pn_string = i as u8;
            }
        }
        if self.pn_ex == 6 {
            const TWO: [&str; 5] = ["6TH + 5TH", "5TH + 4TH", "4TH + 3RD", "3RD + 2ND", "2ND + 1ST"];
            if let Some(i) = crate::choice_line(ui, "pent_two", "STRINGS", &TWO, (self.pn_string as usize).min(4), 260.0) {
                self.pn_string = i as u8;
            }
        }
        crate::views::dim_line(ui, &ex_about.to_uppercase(), 1.0, pal().ink2);
        // what the line is played over: the scale's chord
        let chord: Vec<i32> = sc.chord.iter().map(|x| 48 + root.rem_euclid(12) + x).collect();
        self.ln_backing = Some((self.ln_over, chord));
        let key = format!("small:{}:{}:{}:{}:{}:{}", self.pn_fam, sc_i, nps, self.pn_box, self.pn_ex, self.pn_string);
        self.line_player(ui, acts, &notes, &key);
        let lit_i = self.seq_now("line");
        let tab: Vec<Option<(usize, u8)>> = spots.iter().map(|s| Some(*s)).collect();
        crate::lines_ui::line_score(ui, acts, &notes, self.ln_sub as usize, sharps, lit_i, Some(tab));
    }

    /// The chosen exercise in position `sel` as spots on the neck, and what it is.
    fn small_exercise(&self, boxes: &[Vec<Spot>], sel: usize, root: i32, sc: &SmallScale) -> (Vec<Spot>, &'static str) {
        let steps = &sc.steps;
        let pitch = |s: &Spot| theory::STANDARD[s.0] + s.1 as i32;
        let mut boxn: Vec<Spot> = boxes[sel].clone();
        boxn.sort_by_key(pitch);
        boxn.dedup_by_key(|s| pitch(s));
        // up and back down (the top note once)
        let there_and_back = |up: Vec<Spot>| -> Vec<Spot> {
            let mut v = up.clone();
            v.extend(up.iter().rev().skip(1));
            v
        };
        // a pattern of groups up the line, then the same groups down it
        let groups = |line: &[Spot], step: &dyn Fn(usize) -> Vec<usize>| -> Vec<Spot> {
            let mut out: Vec<Spot> = Vec::new();
            for dir in [false, true] {
                let l: Vec<Spot> = if dir { line.iter().rev().copied().collect() } else { line.to_vec() };
                for i in 0..l.len() {
                    let idx = step(i);
                    if idx.iter().any(|j| *j >= l.len()) {
                        break;
                    }
                    out.extend(idx.iter().map(|j| l[*j]));
                }
            }
            out
        };
        // pitches placed in this position (the chromatic ones too)
        let placed = |ps: &[i32]| -> Vec<Spot> { ps.iter().filter_map(|p| place_near(*p, &boxes[sel])).collect() };
        let ups: Vec<i32> = boxn.iter().map(pitch).collect();
        let ex = EXERCISES.iter().find(|e| e.0 == self.pn_ex).copied().unwrap_or(EXERCISES[0]);
        let spots = match ex.0 {
            1 => groups(&boxn, &|i| vec![i, i + 1, i + 2, i + 3]),
            2 => groups(&boxn, &|i| vec![i, i + 1, i + 2]),
            3 => groups(&boxn, &|i| vec![i, i + 2]),
            4 => groups(&boxn, &|i| vec![i, i + 3]),
            14 => {
                let down: Vec<Spot> = boxn.iter().rev().copied().collect();
                groups(&down, &|i| vec![i, i + 2])
            }
            5 => {
                // along one string, up to the 15th fret and back
                let s = (self.pn_string as usize).min(5);
                let up: Vec<Spot> = (0..=15u8)
                    .filter(|f| steps.contains(&(theory::STANDARD[s] + *f as i32 - root).rem_euclid(12)))
                    .map(|f| (s, f))
                    .collect();
                there_and_back(up)
            }
            6 => {
                // two neighbouring strings, position by position up the neck
                let lo = (self.pn_string as usize).min(4);
                let mut order: Vec<&Vec<Spot>> = boxes.iter().collect();
                order.sort_by_key(|b| b.iter().map(|s| s.1).min().unwrap_or(0));
                let mut up: Vec<Spot> = Vec::new();
                for b in order {
                    let mut two: Vec<Spot> = b.iter().filter(|s| s.0 == lo || s.0 == lo + 1).copied().collect();
                    two.sort_by_key(pitch);
                    for s in two {
                        if !up.iter().any(|u| pitch(u) >= pitch(&s)) {
                            up.push(s);
                        }
                    }
                }
                there_and_back(up)
            }
            7 => {
                // up this position, down the next one up the neck
                let mut by_fret: Vec<usize> = (0..boxes.len()).collect();
                by_fret.sort_by_key(|b| boxes[*b].iter().map(|s| s.1).min().unwrap_or(0));
                let at = by_fret.iter().position(|b| *b == sel).unwrap_or(0);
                let next = by_fret[(at + 1).min(by_fret.len() - 1)];
                let mut nb = boxes[next].clone();
                nb.sort_by_key(pitch);
                let mut v = boxn.clone();
                v.extend(nb.iter().rev().skip(1));
                v
            }
            8 => {
                // triad A, triad B, each through its inversions: the classic triad-pair line
                let Some((a, b)) = sc.triads else { return (there_and_back(boxn), ex.2) };
                let of = |t: [i32; 3]| -> Vec<Spot> {
                    boxn.iter().filter(|s| t.contains(&(pitch(s) - root).rem_euclid(12))).copied().collect()
                };
                let (ta, tb) = (of(a), of(b));
                let mut up: Vec<Spot> = Vec::new();
                let mut k = 0;
                while k + 3 <= ta.len() && k + 3 <= tb.len() {
                    up.extend(&ta[k..k + 3]);
                    up.extend(&tb[k..k + 3]);
                    k += 1;
                }
                let mut v = up.clone();
                v.extend(up.iter().rev().skip(1));
                v
            }
            9 | 10 => {
                // four-note cells over the scale's chord: 1 2 3 5 (1 b3 4 5 over minor), from the root and the 5th -
                // or from the 5th and the 9th
                let minor = sc.chord.contains(&3);
                let cell: [i32; 4] = if minor { [0, 3, 5, 7] } else { [0, 2, 4, 7] };
                let (lo, hi) = (ups.first().copied().unwrap_or(48), ups.last().copied().unwrap_or(72));
                let starts: [i32; 2] = if ex.0 == 9 { [0, 7] } else { [7, 14] };
                let mut base = lo - (lo - root).rem_euclid(12);
                let mut up: Vec<i32> = Vec::new();
                while base <= hi {
                    for st in starts {
                        let c: Vec<i32> = cell.iter().map(|x| base + st + x).collect();
                        if c[0] >= lo - 2 && c[3] <= hi + 2 {
                            up.extend(c);
                        }
                    }
                    base += 12;
                }
                let mut v = up.clone();
                let mut down_cells: Vec<Vec<i32>> = up.chunks(4).map(|c| c.iter().rev().copied().collect()).collect();
                down_cells.reverse();
                v.extend(down_cells.into_iter().flatten());
                placed(&v)
            }
            11 => {
                // half step below each note going up, half step above coming down
                let mut v: Vec<i32> = ups.iter().flat_map(|t| [t - 1, *t]).collect();
                v.extend(ups.iter().rev().flat_map(|t| [t + 1, *t]));
                placed(&v)
            }
            12 => {
                // the scale note above, the half step below, the target
                let mut v: Vec<i32> = Vec::new();
                for i in 0..ups.len().saturating_sub(1) {
                    v.extend([ups[i + 1], ups[i] - 1, ups[i]]);
                }
                placed(&v)
            }
            13 => {
                // every gap of a whole step or more filled in
                let mut up: Vec<i32> = Vec::new();
                for w in ups.windows(2) {
                    up.extend(w[0]..w[1]);
                }
                if let Some(l) = ups.last() {
                    up.push(*l);
                }
                let mut v = up.clone();
                v.extend(up.iter().rev().skip(1));
                placed(&v)
            }
            _ => there_and_back(boxn),
        };
        (spots, ex.2)
    }
}

// ------------------------------------------------------------------ SUPERIMPOSE

/// The shapes that can go over a chord: name, notes from its own root.
pub(crate) const SI_SHAPES: [(&str, &[i32]); 13] = [
    ("MINOR PENTATONIC", &[0, 3, 5, 7, 10]),
    ("MAJOR PENTATONIC", &[0, 2, 4, 7, 9]),
    ("MAJOR TRIAD", &[0, 4, 7]),
    ("MINOR TRIAD", &[0, 3, 7]),
    ("AUGMENTED TRIAD", &[0, 4, 8]),
    ("DIMINISHED TRIAD", &[0, 3, 6]),
    ("MAJ7", &[0, 4, 7, 11]),
    ("7", &[0, 4, 7, 10]),
    ("M7", &[0, 3, 7, 10]),
    ("M(MAJ7)", &[0, 3, 7, 11]),
    ("M7\u{266D}5", &[0, 3, 6, 10]),
    ("DIM7", &[0, 3, 6, 9]),
    ("MINOR 6TH PENTATONIC", &[0, 3, 5, 7, 9]),
];

/// The scales a chord can be heard in, the usual first: name, steps.
const SI_SCALES: [(&str, [i32; 8], usize); 20] = [
    ("IONIAN (MAJOR)", [0, 2, 4, 5, 7, 9, 11, 0], 7),
    ("LYDIAN (#11)", [0, 2, 4, 6, 7, 9, 11, 0], 7),
    ("MIXOLYDIAN", [0, 2, 4, 5, 7, 9, 10, 0], 7),
    ("LYDIAN DOMINANT (#11)", [0, 2, 4, 6, 7, 9, 10, 0], 7),
    ("ALTERED (\u{266D}9 #9 #11 \u{266D}13)", [0, 1, 3, 4, 6, 8, 10, 0], 7),
    ("HALF-WHOLE DIMINISHED (\u{266D}9 #9 #11 13)", [0, 1, 3, 4, 6, 7, 9, 10], 8),
    ("WHOLE TONE (9 #11 #5)", [0, 2, 4, 6, 8, 10, 0, 0], 6),
    ("PHRYGIAN DOMINANT (\u{266D}9 \u{266D}13)", [0, 1, 4, 5, 7, 8, 10, 0], 7),
    ("MIXOLYDIAN \u{266D}6 (\u{266D}13)", [0, 2, 4, 5, 7, 8, 10, 0], 7),
    ("DORIAN", [0, 2, 3, 5, 7, 9, 10, 0], 7),
    ("AEOLIAN (NATURAL MINOR)", [0, 2, 3, 5, 7, 8, 10, 0], 7),
    ("PHRYGIAN (\u{266D}9)", [0, 1, 3, 5, 7, 8, 10, 0], 7),
    ("LOCRIAN NATURAL 9", [0, 2, 3, 5, 6, 8, 10, 0], 7),
    ("LOCRIAN", [0, 1, 3, 5, 6, 8, 10, 0], 7),
    ("MELODIC MINOR", [0, 2, 3, 5, 7, 9, 11, 0], 7),
    ("HARMONIC MINOR", [0, 2, 3, 5, 7, 8, 11, 0], 7),
    ("WHOLE-HALF DIMINISHED", [0, 2, 3, 5, 6, 8, 9, 11], 8),
    ("LYDIAN AUGMENTED (#11 #5)", [0, 2, 4, 6, 8, 9, 11, 0], 7),
    ("HARMONIC MAJOR (\u{266D}13)", [0, 2, 4, 5, 7, 8, 11, 0], 7),
    ("AUGMENTED (SYMMETRIC)", [0, 3, 4, 7, 8, 11, 0, 0], 6),
];

/// The scales that fit a chord (its notes from the root): every note of the chord in the scale - except a dominant's
/// 5th, which the altered, whole-tone and diminished sounds change. Indexes into SI_SCALES, the usual first.
fn si_scales_for(chord: &[i32]) -> Vec<usize> {
    let dominant = chord.contains(&4) && chord.contains(&10);
    let mut fits: Vec<usize> = (0..SI_SCALES.len())
        .filter(|i| {
            let (_, steps, n) = SI_SCALES[*i];
            let s = &steps[..n];
            chord.iter().all(|c| s.contains(&c.rem_euclid(12)) || (dominant && c.rem_euclid(12) == 7))
        })
        .collect();
    // the standard choice first: whole-half for a diminished 7th, whole tone for an augmented triad or a 7#5,
    // Lydian augmented for a maj7#5, half-whole for a 7b9 (with no b13)
    let has = |x: i32| chord.contains(&x);
    let first: Option<&str> = if has(3) && has(6) && has(9) && !has(4) {
        Some("WHOLE-HALF")
    } else if has(4) && has(8) && has(11) {
        Some("LYDIAN AUGMENTED")
    } else if has(4) && has(8) && !has(7) {
        Some("WHOLE TONE")
    } else if dominant && has(1) && !has(8) {
        Some("HALF-WHOLE")
    } else {
        None
    };
    // and a scale with both 3rds (b3 and 3) is never the usual one over a minor chord (nor one with a b3 over a
    // major chord that isn't a dominant): those go after the rest
    let minor = has(3) && !has(4);
    let major = has(4) && !has(3) && !has(10);
    let clash = |i: usize| {
        let s = &SI_SCALES[i].1[..SI_SCALES[i].2];
        (minor && s.contains(&4)) || (major && s.contains(&3))
    };
    fits.sort_by_key(|i| (first.map_or(true, |f| !SI_SCALES[*i].0.starts_with(f)), clash(*i)));
    fits
}

/// Every shape on every note that fits inside the scale, best first: the ones that add the chord's colours
/// (tensions) while still touching its 3rd or 7th lead; one that only restates the chord comes last. At most 14.
/// (shape, its root from the chord's root).
pub(crate) fn si_auto(chord: &[i32], scale: &[i32]) -> Vec<(usize, i32)> {
    let mut found: Vec<(i32, usize, i32)> = Vec::new();
    for (k, (_, notes)) in SI_SHAPES.iter().enumerate() {
        for off in 0..12 {
            let pcs: Vec<i32> = notes.iter().map(|x| (x + off).rem_euclid(12)).collect();
            if !pcs.iter().all(|p| scale.contains(p)) {
                continue;
            }
            let tensions = pcs.iter().filter(|p| !chord.contains(p)).count() as i32;
            let guide = pcs.iter().any(|p| chord.iter().any(|c| (*c == 3 || *c == 4 || *c == 10 || *c == 11) && c == p)) as i32;
            let score = tensions * 2 + guide * 3 + if notes.len() == 5 { 1 } else { 0 } - if tensions == 0 { 6 } else { 0 };
            found.push((score, k, off));
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    found.into_iter().take(14).map(|(_, k, o)| (k, o)).collect()
}

/// The usual choices over each chord: (shape, its root from the chord's root, why).
fn si_suggestions(quality: usize) -> Vec<(usize, i32, &'static str)> {
    match quality {
        // maj7
        0 => vec![
            (1, 0, "THE MAJOR PENTATONIC ON THE ROOT: SAFE, NO 4TH TO CLASH"),
            (1, 7, "MAJOR PENTATONIC FROM THE 5TH: ADDS THE 7TH AND THE 9TH"),
            (1, 2, "MAJOR PENTATONIC A STEP UP: THE LYDIAN SOUND (#11)"),
            (0, 4, "MINOR PENTATONIC FROM THE 3RD: THE SAME NOTES AS THE 5TH'S MAJOR, IN MINOR SHAPES"),
            (0, 11, "MINOR PENTATONIC FROM THE 7TH: LYDIAN AGAIN (#11), BRIGHT"),
            (8, 4, "THE M7 FROM THE 3RD: 3 5 7 9 - THE CHORD WITHOUT ITS ROOT, PLUS THE 9TH"),
            (2, 2, "A MAJOR TRIAD A STEP UP: 9 #11 13, THE LYDIAN UPPER STRUCTURE"),
        ],
        // 7
        1 => vec![
            (0, 7, "MINOR PENTATONIC FROM THE 5TH: 5 \u{266D}7 R 9 11 - A SUS FLAVOUR"),
            (1, 0, "MAJOR PENTATONIC ON THE ROOT: R 9 3 5 13, SWEET"),
            (0, 9, "MINOR PENTATONIC FROM THE 6TH: 13 R 9 3 5 - THE SAME NOTES, LAID OUT AGAIN"),
            (9, 7, "M(MAJ7) FROM THE 5TH: 5 \u{266D}7 9 #11 - THE MELODIC MINOR, LYDIAN DOMINANT"),
            (2, 2, "MAJOR TRIAD A STEP UP: 9 #11 13 - LYDIAN DOMINANT UPPER STRUCTURE"),
            (1, 6, "MAJOR PENTATONIC A TRITONE AWAY: ALL THE ALTERATIONS (#11 \u{266D}13 \u{266D}7 \u{266D}9 #9)"),
            (2, 9, "MAJOR TRIAD ON THE 6TH: 13 \u{266D}9 3 - THE DIMINISHED SOUND"),
        ],
        // 7alt
        2 => vec![
            (1, 6, "MAJOR PENTATONIC A TRITONE AWAY: #11 \u{266D}13 \u{266D}7 \u{266D}9 #9 - THE ALTERED SOUND"),
            (0, 3, "MINOR PENTATONIC FROM THE \u{266D}3: THE SAME NOTES, IN MINOR SHAPES"),
            (2, 8, "MAJOR TRIAD ON THE \u{266D}6: \u{266D}13 R #9"),
            (2, 6, "MAJOR TRIAD ON THE \u{266D}5: #11 \u{266D}7 \u{266D}9"),
            (9, 1, "M(MAJ7) A HALF STEP UP: THE ALTERED SCALE IS ITS MELODIC MINOR"),
            (4, 0, "AUGMENTED TRIAD ON THE ROOT: R 3 #5 - THE WHOLE-TONE COLOUR"),
        ],
        // 7#11
        3 => vec![
            (9, 7, "M(MAJ7) FROM THE 5TH: 5 \u{266D}7 9 #11"),
            (2, 2, "MAJOR TRIAD A STEP UP: 9 #11 13"),
            (1, 0, "MAJOR PENTATONIC ON THE ROOT: R 9 3 5 13 - ADD THE #11 AS A PASSING NOTE"),
            (0, 7, "MINOR PENTATONIC FROM THE 5TH: 5 \u{266D}7 R 9 11 - THE 11 RUBS: LAND ON #11 INSTEAD"),
        ],
        // m7
        4 => vec![
            (0, 0, "THE MINOR PENTATONIC ON THE ROOT: HOME BASE"),
            (0, 2, "MINOR PENTATONIC A STEP UP: 9 11 5 13 R - THE DORIAN SOUND"),
            (0, 7, "MINOR PENTATONIC FROM THE 5TH: 5 \u{266D}7 R 9 11 - OPEN, MODERN"),
            (6, 3, "MAJ7 FROM THE \u{266D}3: \u{266D}3 5 \u{266D}7 9 - THE CHORD WITH ITS 9TH"),
            (2, 10, "MAJOR TRIAD ON THE \u{266D}7: \u{266D}7 9 11"),
            (2, 5, "MAJOR TRIAD ON THE 4TH: 11 13 R - DORIAN"),
        ],
        // m7b5
        5 => vec![
            (12, 3, "MINOR 6TH PENTATONIC FROM THE \u{266D}3: \u{266D}3 \u{266D}5 \u{266D}6 \u{266D}7 R"),
            (2, 10, "MAJOR TRIAD ON THE \u{266D}7: \u{266D}7 9 11 - LOCRIAN #2"),
            (8, 3, "M7 FROM THE \u{266D}3: \u{266D}3 \u{266D}5 \u{266D}7 \u{266D}9"),
        ],
        // m(maj7)
        6 => vec![
            (2, 7, "MAJOR TRIAD FROM THE 5TH: 5 7 9"),
            (3, 2, "MINOR TRIAD A STEP UP: 9 11 13 - MELODIC MINOR"),
            (4, 3, "AUGMENTED TRIAD FROM THE \u{266D}3: \u{266D}3 5 7"),
            (0, 0, "MINOR PENTATONIC ON THE ROOT, LANDING ON THE MAJOR 7TH"),
        ],
        // maj7#11
        _ => vec![
            (1, 2, "MAJOR PENTATONIC A STEP UP: 9 3 #11 13 7"),
            (2, 2, "MAJOR TRIAD A STEP UP: 9 #11 13"),
            (0, 11, "MINOR PENTATONIC FROM THE 7TH: 7 9 3 #11 13"),
            (1, 7, "MAJOR PENTATONIC FROM THE 5TH: 5 13 7 9 3"),
        ],
    }
}

impl App {
    /// SUPERIMPOSE: a shape over a chord - the usual choices for it, or any shape you pick - with each note marked
    /// a chord tone, a tension or a note to avoid, on the neck and the piano, and played over the chord.
    pub(crate) fn superimpose(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Play one thing over another: a pentatonic, a triad or an arpeggio from somewhere other than the chord's root. Each note shows what it is over the chord - a chord tone, a tension (colour), or a note to avoid landing on.", pal().ink2);
        let sharps = lines::key_sharps(self.si_root);
        if let Some(pc) = Self::note_row(ui, "CHORD ROOT", self.si_root, sharps) {
            self.si_root = pc;
        }
        // the chord, as in FIND A CHORD: its kind, then which
        let fams: Vec<&str> = theory::FINDER_FAMILIES.iter().map(|f| f.0).collect();
        let fam = (self.si_fam as usize).min(fams.len() - 1);
        if let Some(i) = crate::choice_line(ui, "si_fam", "KIND", &fams, fam, 200.0) {
            self.si_fam = i as u8;
            self.si_chord = 0;
            self.si_scale = 0;
            self.si_pick = 0;
        }
        let fam = (self.si_fam as usize).min(fams.len() - 1);
        let list = theory::FINDER_FAMILIES[fam].1;
        let root = self.si_root.rem_euclid(12);
        let labels: Vec<String> = list.iter().map(|(l, _)| format!("{}{}", theory::note_name(root, sharps), l)).collect();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let ch = (self.si_chord as usize).min(list.len() - 1);
        if let Some(i) = crate::choice_line(ui, "si_chord", "CHORD", &labels, ch, 200.0) {
            self.si_chord = i as u8;
            self.si_scale = 0;
            self.si_pick = 0;
        }
        let ch = (self.si_chord as usize).min(list.len() - 1);
        let suffix = list[ch].1;
        let chord_name = format!("{}{}", theory::note_name(root, sharps), suffix);
        let chord: Vec<i32> = match theory::chord_notes(&chord_name) {
            Some(n) => {
                let mut v: Vec<i32> = n.pcs.iter().map(|p| (p - n.root).rem_euclid(12)).collect();
                v.sort();
                v.dedup();
                v
            }
            None => vec![0, 4, 7],
        };
        // the chord's notes as they are (for naming each note over it: #11, \u{266D}13...)
        let chord_abs: Vec<i32> = chord.iter().map(|x| (root + x).rem_euclid(12)).collect();
        // the scale it is heard in: every one that fits, the usual first
        let fits = si_scales_for(&chord);
        let scale_names: Vec<&str> = fits.iter().map(|i| SI_SCALES[*i].0).collect();
        let sc = (self.si_scale as usize).min(fits.len().saturating_sub(1));
        if !fits.is_empty() {
            if let Some(i) = crate::choice_line(ui, "si_scale", "SCALE", &scale_names, sc, 300.0) {
                self.si_scale = i as u8;
                self.si_pick = 0;
            }
        }
        let sc = (self.si_scale as usize).min(fits.len().saturating_sub(1));
        let scale: Vec<i32> = fits.get(sc).map_or(chord.clone(), |i| SI_SCALES[*i].1[..SI_SCALES[*i].2].to_vec());
        // the hand-picked tips (with why) for the chords that have them, in their usual scale; the 7's altered too
        let curated: Option<usize> = match (suffix, sc) {
            ("maj7", 0) => Some(0),
            ("7", 0) => Some(1),
            ("7", _) if fits.get(sc).map(|i| SI_SCALES[*i].0.starts_with("ALTERED")).unwrap_or(false) => Some(2),
            ("7#11", 0) => Some(3),
            ("m7", 0) => Some(4),
            ("m7b5", 0) => Some(5),
            ("m(maj7)", 0) => Some(6),
            ("maj7#11", 0) => Some(7),
            _ => None,
        };
        let q = curated.unwrap_or(0);
        if let Some(i) = crate::choice_line(
            ui,
            "si_any",
            "CHOOSE FROM",
            &["THE USUAL CHOICES", "ANY SHAPE, ANYWHERE"],
            self.si_any as usize,
            260.0,
        ) {
            self.si_any = i == 1;
        }
        // the shape over it: (shape, its root from the chord's root, why)
        let (kind, off, why): (usize, i32, String) = if self.si_any {
            let sn: Vec<&str> = SI_SHAPES.iter().map(|s| s.0).collect();
            if let Some(i) =
                crate::choice_line(ui, "si_kind", "SHAPE", &sn, (self.si_kind as usize).min(SI_SHAPES.len() - 1), 260.0)
            {
                self.si_kind = i as u8;
            }
            let roots: Vec<String> =
                (0..12).map(|k| format!("{}  ({})", theory::note_name(root + k, sharps), degree_name(root + k, root))).collect();
            let roots: Vec<&str> = roots.iter().map(String::as_str).collect();
            if let Some(i) = crate::choice_line(ui, "si_kroot", "ON", &roots, (self.si_kroot as usize).min(11), 160.0) {
                self.si_kroot = i as u8;
            }
            ((self.si_kind as usize).min(SI_SHAPES.len() - 1), self.si_kroot as i32, String::new())
        } else {
            let mut sug: Vec<(usize, i32, &'static str)> = match curated {
                Some(c) => si_suggestions(c),
                None => Vec::new(),
            };
            for (k, o) in si_auto(&chord, &scale) {
                if !sug.iter().any(|s| s.0 == k && s.1 == o) {
                    sug.push((k, o, ""));
                }
            }
            if sug.is_empty() {
                sug.push((1, 0, ""));
            }
            let names: Vec<String> =
                sug.iter().map(|(k, o, _)| format!("{} {}", theory::note_name(root + o, sharps), SI_SHAPES[*k].0)).collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let pick = (self.si_pick as usize).min(sug.len() - 1);
            if let Some(i) = crate::choice_line(ui, "si_pick", "PLAY", &names, pick, 300.0) {
                self.si_pick = i as u8;
            }
            let pick = (self.si_pick as usize).min(sug.len() - 1);
            // the tip, or what the shape adds over the chord
            let why = if !sug[pick].2.is_empty() {
                sug[pick].2.to_string()
            } else {
                let (k, o) = (sug[pick].0, sug[pick].1);
                let pcs: Vec<i32> = SI_SHAPES[k].1.iter().map(|x| (root + o + x).rem_euclid(12)).collect();
                let names: Vec<&str> = pcs.iter().map(|p| crate::fretboard::degree_in(*p, root, &chord_abs).0).collect();
                let adds: Vec<&str> = pcs
                    .iter()
                    .filter(|p| !chord.contains(&((*p - root).rem_euclid(12))))
                    .map(|p| crate::fretboard::degree_in(*p, root, &chord_abs).0)
                    .collect();
                format!(
                    "{} ON THE {}: {}{}",
                    SI_SHAPES[k].0,
                    crate::fretboard::degree_in((root + o).rem_euclid(12), root, &chord_abs).0,
                    names.join(" "),
                    if adds.is_empty() { " - THE CHORD ITSELF".to_string() } else { format!(" - ADDS {}", adds.join(" ")) }
                )
            };
            let _ = q;
            (sug[pick].0, sug[pick].1, why)
        };
        let shape_root = (root + off).rem_euclid(12);
        // the shape's notes spelled in its own key (a D triad is D F# A, whatever the chord)
        let sharps = lines::key_sharps(shape_root);
        let shape: Vec<i32> = SI_SHAPES[kind].1.iter().map(|x| (shape_root + x).rem_euclid(12)).collect();
        // what each note is over the chord
        let class = |pc: i32| -> (&'static str, Color32) {
            let r = (pc - root).rem_euclid(12);
            if chord.contains(&r) {
                ("CHORD TONE", crate::fretboard::degree(pc, root).1)
            } else if chord.contains(&((r - 1).rem_euclid(12)))
                && !(chord.contains(&10) && chord.contains(&4) && (r == 1 || r == 3))
            {
                // a half step above a chord tone (a dominant's \u{266D}9 and #9 are tensions)
                ("AVOID", Color32::from_rgb(220, 70, 60))
            } else {
                ("TENSION", Color32::from_rgb(95, 168, 211))
            }
        };
        let deg = |pc: i32| -> String { crate::fretboard::degree_in(pc, root, &chord_abs).0.to_string() };
        section_header(ui, &format!("{} {} OVER {}", theory::note_name(shape_root, sharps), SI_SHAPES[kind].0, chord_name));
        if !why.is_empty() {
            crate::views::dim_line(ui, &why, 1.0, pal().ink2);
        }
        // the notes, each named over the chord
        let mut sorted = shape.clone();
        sorted.sort_by_key(|p| (p - shape_root).rem_euclid(12));
        let line: Vec<String> = sorted
            .iter()
            .map(|pc| {
                format!(
                    "{} = {}{}",
                    theory::note_name(*pc, sharps),
                    deg(*pc),
                    if class(*pc).0 == "AVOID" { " (AVOID)" } else { "" }
                )
            })
            .collect();
        crate::views::dim_line(ui, &line.join("    "), 1.0, pal().ink);
        ui.horizontal_wrapped(|ui| {
            for (name, col) in [
                ("CHORD TONE", pal().ink),
                ("TENSION", Color32::from_rgb(95, 168, 211)),
                ("AVOID", Color32::from_rgb(220, 70, 60)),
            ] {
                let (r, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), Sense::hover());
                ui.painter().rect_filled(r, 1.0, col);
                crate::views::label(ui, name, 0.0);
                ui.add_space(10.0);
            }
            crate::views::label(ui, "  RINGS: THE CHORD'S OWN NOTES NOT IN THE SHAPE", 0.0);
        });

        // the line: the shape up two octaves and back, over the chord
        let mut up: Vec<i32> = (52..=76i32).filter(|m| shape.contains(&m.rem_euclid(12))).collect();
        if up.len() > 12 {
            up.truncate(12);
        }
        let mut notes = up.clone();
        notes.extend(up.iter().rev().skip(1));
        let tab = lines::finger_line(&notes, &theory::STANDARD);
        let lit = self.seq_now("line").and_then(|i| tab.get(i).copied().flatten());

        // ---- the neck: the shape everywhere, coloured by what it is over the chord; the chord's other notes ringed
        let mut dots: Vec<NeckDot> = Vec::new();
        for s in 0..6 {
            for f in 0..=15u8 {
                let pc = (theory::STANDARD[s] + f as i32).rem_euclid(12);
                if shape.contains(&pc) {
                    let (_, col) = class(pc);
                    dots.push(NeckDot { spot: (s, f), text: deg(pc), col, strength: 1.0, ring: false, root: pc == root });
                } else if chord.contains(&((pc - root).rem_euclid(12))) {
                    dots.push(NeckDot { spot: (s, f), text: deg(pc), col: pal().dim, strength: 0.6, ring: true, root: false });
                }
            }
        }
        neck_view(ui, &dots, lit);
        // ---- the piano: the chord low, the shape above it
        let mut keys: Vec<i32> = chord.iter().map(|r| 48 + root + r).collect();
        keys.extend(sorted.iter().map(|pc| 60 + (pc - 0).rem_euclid(12) + if *pc < root { 12 } else { 0 }));
        let pw = ui.available_width().min(720.0);
        let (pr, _) = ui.allocate_exact_size(Vec2::new(pw, 70.0), Sense::hover());
        let rel: Vec<i32> = chord.clone();
        crate::fretboard::over_chord(&rel, || crate::fretboard::piano_span(ui, pr, &keys, false, "si_piano", Some(root), 36, 4));

        // ---- play it over the chord
        ui.add_space(6.0);
        let voiced: Vec<i32> = chord.iter().take(4).map(|x| 48 + root + x).collect();
        self.ln_backing = Some((self.ln_over, voiced));
        let key = format!("si:{}:{}:{}:{}:{}", root, suffix, kind, off, self.si_any);
        self.line_player(ui, acts, &notes, &key);
        let lit_i = self.seq_now("line");
        crate::lines_ui::line_score(ui, acts, &notes, self.ln_sub as usize, sharps, lit_i, Some(tab));
    }
}

// ------------------------------------------------------------------ SCALE FINGERINGS

/// Every scale the fingerings offer: (family, name, steps). The LINES scales, then the pentatonics and the blues.
fn all_scales() -> Vec<(&'static str, &'static str, Vec<i32>)> {
    let mut v: Vec<(&'static str, &'static str, Vec<i32>)> = Vec::new();
    for (fam, list) in lines::SCALE_FAMILIES.iter() {
        for (name, steps, _) in list.iter() {
            v.push((fam, name, steps.to_vec()));
        }
    }
    v.push(("PENTATONIC", "Minor pentatonic", PENT_MINOR.to_vec()));
    v.push(("PENTATONIC", "Major pentatonic", PENT_MAJOR.to_vec()));
    v.push(("PENTATONIC", "Blues", vec![0, 3, 5, 6, 7, 10]));
    v
}

/// The five CAGED positions of a scale: five windows of five frets, set by where the root sits on the low E
/// (the C, A, G, E and D shapes), each holding the scale's notes on every string in it (a note the 3rd string and
/// the 2nd both reach goes to the 2nd). In order up the neck.
pub(crate) fn caged(root: i32, steps: &[i32]) -> Vec<Vec<Spot>> {
    let r = (root - theory::STANDARD[0]).rem_euclid(12);
    let mut starts: Vec<i32> = [-8, -6, -4, -1, 1].iter().map(|o| (r + o).rem_euclid(12)).collect();
    starts.sort();
    starts
        .iter()
        .map(|w| {
            let mut spots: Vec<Spot> = Vec::new();
            for s in 0..6 {
                for f in *w..=*w + 4 {
                    let m = theory::STANDARD[s] + f;
                    if steps.contains(&(m - root).rem_euclid(12)) {
                        // the same note already on the string below (only the G string's top reaches the B's bottom)
                        if s == 4 {
                            spots.retain(|x| !(x.0 == 3 && theory::STANDARD[3] + x.1 as i32 == m));
                        }
                        spots.push((s, f as u8));
                    }
                }
            }
            spots
        })
        .collect()
}

/// The colour each position is outlined in on the whole neck.
const POS_COLS: [Color32; 8] = [
    Color32::from_rgb(217, 164, 65),
    Color32::from_rgb(95, 168, 211),
    Color32::from_rgb(214, 104, 83),
    Color32::from_rgb(111, 191, 115),
    Color32::from_rgb(167, 125, 194),
    Color32::from_rgb(80, 190, 180),
    Color32::from_rgb(200, 200, 120),
    Color32::from_rgb(230, 130, 180),
];

impl App {
    /// SCALE FINGERINGS: any scale's positions across the neck - five CAGED positions, or 2, 3 or 4 notes a string -
    /// one at a time or all at once, played, or shifted through one after another.
    pub(crate) fn fingerings(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Every position of a scale, in any key: one at a time with its fingering, or the whole neck with each position in its own colour. Then play it - the position up and down, every position in turn, or straight, in intervals and in groups from every note, until you can go anywhere in it.", pal().ink2);
        let sharps = lines::key_sharps(self.pf_root);
        if let Some(pc) = Self::note_row(ui, "KEY", self.pf_root, sharps) {
            self.pf_root = pc;
        }
        // first the family (your starred scales as one of them), then the scale in it
        let scales = all_scales();
        const MINE: &str = "MY SCALES (STARRED)";
        let mut fams: Vec<&str> = Vec::new();
        if !self.pf_stars.is_empty() {
            fams.push(MINE);
        }
        for s in &scales {
            if !fams.contains(&s.0) {
                fams.push(s.0);
            }
        }
        let own_fam = scales.iter().find(|s| s.1 == self.pf_scale).map_or(scales[0].0, |s| s.0);
        let fam_now = fams.iter().copied().find(|f| *f == self.pf_fam).unwrap_or(own_fam);
        let in_fam = |f: &str| -> Vec<usize> {
            (0..scales.len())
                .filter(|i| if f == MINE { self.pf_stars.iter().any(|s| s == scales[*i].1) } else { scales[*i].0 == f })
                .collect()
        };
        if let Some(i) =
            crate::choice_line(ui, "pf_fam", "FAMILY", &fams, fams.iter().position(|f| *f == fam_now).unwrap_or(0), 260.0)
        {
            self.pf_fam = fams[i].to_string();
            // a scale of that family (the one you were on, if it is in it)
            let list = in_fam(fams[i]);
            if !list.iter().any(|j| scales[*j].1 == self.pf_scale) {
                if let Some(j) = list.first() {
                    self.pf_scale = scales[*j].1.to_string();
                    self.pf_pos = 0;
                }
            }
        }
        let fam_now = fams.iter().copied().find(|f| *f == self.pf_fam).unwrap_or(own_fam);
        let order = in_fam(fam_now);
        let order = if order.is_empty() { (0..scales.len()).collect() } else { order };
        let names: Vec<String> = order.iter().map(|i| scales[*i].1.to_uppercase()).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let cur = order.iter().position(|i| scales[*i].1 == self.pf_scale).unwrap_or(0);
        ui.horizontal(|ui| {
            crate::views::label(ui, "SCALE", 100.0);
            let widest = names.iter().map(|s| crate::font::text_w(s, 2.0)).fold(0.0, f32::max) + 56.0;
            if let Some(i) =
                crate::dropdown_fixed(ui, "pf_scale", &names, cur, widest.min(ui.available_width() - 60.0).max(200.0))
            {
                self.pf_scale = scales[order[i]].1.to_string();
                self.pf_pos = 0;
            }
            // the star: one you use a lot, kept in MY SCALES
            let starred = self.pf_stars.iter().any(|s| *s == self.pf_scale);
            if crate::ibtn(ui, &crate::IC_STAR, "", starred)
                .tip(if starred { "In MY SCALES - click to take it out" } else { "Star it: kept in MY SCALES, the first family" })
                .clicked()
            {
                if starred {
                    let n = self.pf_scale.clone();
                    self.pf_stars.retain(|s| *s != n);
                } else {
                    self.pf_stars.push(self.pf_scale.clone());
                }
                self.dirty = true;
            }
        });
        if !order.iter().any(|i| scales[*i].1 == self.pf_scale) {
            if let Some(j) = order.first() {
                self.pf_scale = scales[*j].1.to_string();
            }
        }
        let si = order[order.iter().position(|i| scales[*i].1 == self.pf_scale).unwrap_or(0)];
        let steps = scales[si].2.clone();
        let root = self.pf_root;
        const SYSTEMS: [&str; 4] = ["5 POSITIONS (CAGED)", "2 NOTES A STRING", "3 NOTES A STRING", "4 NOTES A STRING"];
        if let Some(i) = crate::choice_line(ui, "pf_sys", "SYSTEM", &SYSTEMS, (self.pf_sys as usize).min(3), 260.0) {
            self.pf_sys = i as u8;
            self.pf_pos = 0;
        }
        let positions: Vec<Vec<Spot>> = match self.pf_sys {
            0 => caged(root, &steps),
            n => positions(root, &steps, n as usize + 1),
        };
        let np = positions.len();
        // in order up the neck, numbered from the one you call position 1
        let mut by_neck: Vec<usize> = (0..np).collect();
        by_neck.sort_by_key(|p| positions[*p].iter().map(|s| s.1).min().unwrap_or(0));
        let about: Vec<String> = by_neck.iter().map(|p| root_place(&positions[*p], root)).collect();
        let (first, changed) =
            numbering_line(ui, "pf_num", &about, &mut self.pf_num, &["ROOT ON 5TH STRING, 2ND", "ROOT ON 5TH"]);
        if changed {
            self.dirty = true;
        }
        // position k as numbered (0-based) -> the index into `positions`
        let nth = |k: usize| by_neck[(first + k) % np];
        let shown: Vec<String> = (0..np)
            .map(|k| format!("POSITION {}  -  {}", k + 1, about[(first + k) % np]))
            .chain(std::iter::once(format!("WHOLE NECK  -  ALL {} TOGETHER", np)))
            .collect();
        let shown: Vec<&str> = shown.iter().map(String::as_str).collect();
        ui.horizontal(|ui| {
            crate::views::label(ui, "SHOW", 100.0);
            let pick = (self.pf_pos as usize).min(np);
            if retro_btn_w(ui, "<", 34.0, false).tip("The position below").clicked() {
                self.pf_pos = ((pick + np) % (np + 1)) as u8;
            }
            let widest = shown.iter().map(|s| crate::font::text_w(s, 2.0)).fold(0.0, f32::max) + 56.0;
            if let Some(i) = crate::dropdown_fixed(ui, "pf_pos", &shown, pick, widest.min(ui.available_width() - 60.0).max(200.0))
            {
                self.pf_pos = i as u8;
            }
            if retro_btn_w(ui, ">", 34.0, false).tip("The next position up").clicked() {
                self.pf_pos = ((pick + 1) % (np + 1)) as u8;
            }
        });
        if let Some(i) =
            crate::choice_line(ui, "pf_label", "LABELS", &["DEGREES", "NOTES", "FINGERS"], self.pf_label as usize, 260.0)
        {
            self.pf_label = i as u8;
        }

        // the line: this position up and down - or every position in turn (SHIFT THROUGH ALL)
        let pitch = |s: &Spot| theory::STANDARD[s.0] + s.1 as i32;
        // (the whole position, or one or two octaves of it from its lowest root)
        let range = self.pf_range;
        let up_down = |p: &Vec<Spot>| -> Vec<Spot> {
            let mut u = p.clone();
            u.sort_by_key(|s| pitch(s));
            u.dedup_by_key(|s| pitch(s));
            if range > 0 {
                if let Some(st) = u.iter().position(|s| (pitch(s) - root).rem_euclid(12) == 0) {
                    let top = pitch(&u[st]) + 12 * range as i32;
                    u = u[st..].iter().copied().filter(|s| pitch(s) <= top).collect();
                }
            }
            let mut v = u.clone();
            v.extend(u.iter().rev().skip(1));
            v
        };
        let pick = (self.pf_pos as usize).min(np);
        let mut owner: Vec<usize> = Vec::new();
        let shifting = self.pf_play == 1 || (pick == np && self.pf_play == 0);
        let spots: Vec<Spot> = if self.pf_play >= 2 {
            // a pattern from the lowest root of the position shown (position 1 on the whole neck), played in it
            let at = &positions[nth(if pick == np { 0 } else { pick })];
            let low = at.iter().map(pitch).filter(|m| (m - root).rem_euclid(12) == 0).min().unwrap_or(48 + root.rem_euclid(12));
            let pat = (self.pf_play as usize - 2).min(lines::SCALE_PATTERNS.len() - 1);
            let ns = lines::scale_line(low, &steps, pat, self.sc_oct.max(1), self.sc_dir);
            ns.iter().filter_map(|m| place_near(*m, at)).collect()
        } else if shifting {
            let mut all = Vec::new();
            for k in 0..np {
                let seg = up_down(&positions[nth(k)]);
                owner.extend(std::iter::repeat(k).take(seg.len()));
                all.extend(seg);
            }
            all
        } else {
            up_down(&positions[nth(pick)])
        };
        let notes: Vec<i32> = spots.iter().map(pitch).collect();
        let playing = self.seq_now("line");
        let lit = playing.and_then(|i| spots.get(i).copied());
        // while shifting, the position sounding now is the one shown bright
        let bright: Option<usize> = match (playing, shifting) {
            (Some(i), true) => owner.get(i).copied(),
            // one position shown: that one bright; the whole neck: all of them, each in its colour
            (_, false) => (pick < np).then_some(pick),
            _ => None,
        };

        // ---- the neck
        let mut dots: Vec<NeckDot> = Vec::new();
        for k in 0..np {
            let pos = &positions[nth(k)];
            let fing = fingers(pos);
            let on = bright.map_or(pick == np, |b| b == k);
            for (j, s) in pos.iter().enumerate() {
                let pc = pitch(s).rem_euclid(12);
                let text = match self.pf_label {
                    1 => theory::note_name(pc, sharps).to_string(),
                    2 => fing[j].to_string(),
                    _ => degree_name(pc, root).to_string(),
                };
                // the whole neck: each position in its own colour; one shown: degree colours, the rest dim
                let col = if pick == np && bright.is_none() {
                    POS_COLS[k % POS_COLS.len()]
                } else {
                    crate::fretboard::degree(pc, root).1
                };
                if let Some(d) = dots.iter_mut().find(|d| d.spot == *s) {
                    if on {
                        d.strength = 1.0;
                        d.text = text;
                        d.col = col;
                    }
                    continue;
                }
                dots.push(NeckDot {
                    spot: *s,
                    text: if on || pick == np { text } else { String::new() },
                    col,
                    strength: if on || (pick == np && bright.is_none()) { 1.0 } else { 0.22 },
                    ring: false,
                    root: pc == root.rem_euclid(12),
                });
            }
        }
        neck_view(ui, &dots, lit);
        if let Some(b) = bright.filter(|b| *b < np) {
            crate::views::dim_line(ui, &format!("POSITION {}: {}", b + 1, about[(first + b) % np]), 1.0, pal().ink2);
        }
        // ---- the piano, to look at: the scale, or the note sounding
        let keys: Vec<i32> = match lit {
            Some(s) => vec![pitch(&s)],
            None => steps.iter().map(|x| 48 + root.rem_euclid(12) + x).collect(),
        };
        let pw = ui.available_width().min(720.0);
        let (pr, _) = ui.allocate_exact_size(Vec2::new(pw, 70.0), Sense::hover());
        crate::fretboard::piano_span(ui, pr, &keys, false, "pf_piano", Some(root.rem_euclid(12)), 36, 4);

        // ---- play
        ui.add_space(6.0);
        let mut plays: Vec<String> = vec!["THIS POSITION, UP AND DOWN".to_string(), "SHIFT THROUGH ALL POSITIONS".to_string()];
        plays.extend(lines::SCALE_PATTERNS.iter().map(|p| p.0.to_string()));
        let plays: Vec<&str> = plays.iter().map(String::as_str).collect();
        if let Some(i) = crate::choice_line(ui, "pf_play", "PLAY", &plays, (self.pf_play as usize).min(plays.len() - 1), 260.0) {
            self.pf_play = i as u8;
        }
        if self.pf_play < 2 {
            const RANGES: [&str; 3] = ["THE WHOLE POSITION", "1 OCTAVE, FROM THE ROOT", "2 OCTAVES, FROM THE ROOT"];
            if let Some(i) = crate::choice_line(ui, "pf_range", "RANGE", &RANGES, (self.pf_range as usize).min(2), 260.0) {
                self.pf_range = i as u8;
            }
        }
        if self.pf_play >= 2 {
            let pat = (self.pf_play as usize - 2).min(lines::SCALE_PATTERNS.len() - 1);
            crate::views::dim_line(ui, &lines::SCALE_PATTERNS[pat].1.to_uppercase(), 1.0, pal().ink2);
            if let Some(i) = crate::choice_line(ui, "sc_dir", "DIRECTION", &lines::DIRECTIONS, self.sc_dir, 220.0) {
                self.sc_dir = i;
            }
            if let Some(i) =
                crate::choice_line(ui, "sc_oct", "RANGE", &["1 OCTAVE", "2 OCTAVES"], self.sc_oct.saturating_sub(1), 220.0)
            {
                self.sc_oct = i + 1;
            }
        }
        let key =
            format!("pf:{}:{}:{}:{}:{}:{}:{}", self.pf_scale, root, self.pf_sys, pick, self.pf_play, self.sc_dir, self.sc_oct);
        self.line_player(ui, acts, &notes, &key);
        let tab: Vec<Option<(usize, u8)>> = spots.iter().map(|s| Some(*s)).collect();
        crate::lines_ui::line_score(ui, acts, &notes, self.ln_sub as usize, sharps, playing, Some(tab));
        ui.add_space(10.0);
        self.coltrane_tune(ui, acts);
    }

    /// COLTRANE CELLS THROUGH A TUNE: one four-note cell a chord (two beats, in eighths), each from that chord's own
    /// scale and starting near where the last one ended - the way he ran them through Giant Steps and Countdown.
    fn coltrane_tune(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        section_header(ui, "COLTRANE CELLS THROUGH A TUNE");
        para(ui, "One cell a chord, two beats each, from the chord's own scale - each starting near where the last one ended. Hear it, then play it over the changes yourself.", pal().ink2);
        let (title, changes) = self.practice_changes(ui);
        if changes.is_empty() {
            return;
        }
        if let Some(i) = crate::choice_line(ui, "ct_cell", "CELL", &CT_CELLS.map(|c| c.0), self.ct_cell as usize, 300.0) {
            self.ct_cell = i as u8;
        }
        crate::views::dim_line(ui, CT_CELLS[(self.ct_cell as usize).min(2)].1, 1.0, pal().ink2);
        let cells = coltrane_cells(&changes, self.ct_cell as usize);
        if cells.is_empty() {
            dim_line(ui, "NO CHORDS BACKLINE CAN READ IN THESE CHANGES", 1.0, pal().dim);
            return;
        }
        let sharps = changes.first().and_then(|c| theory::chord_notes(c)).map_or(false, |c| lines::key_sharps(c.root));
        let gap = 60.0 / self.band_bpm() / 2.0;
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, &title.to_uppercase(), 240.0);
            // each chord's first eighth carries the chord, low, so you hear the changes go by
            self.seq_button(
                ui,
                acts,
                "PLAY IT",
                "Eighth notes at the band's tempo, the chord under each cell",
                "coltrane",
                || {
                    cells
                        .iter()
                        .flat_map(|(_, root, chord, cell)| {
                            cell.iter().enumerate().map(move |(k, m)| {
                                let mut v = vec![*m];
                                if k == 0 {
                                    let bass = 40 + (root - 40).rem_euclid(12);
                                    v.extend(chord.iter().map(|x| bass + x));
                                }
                                v
                            })
                        })
                        .collect()
                },
                gap,
            );
        });
        let playing = self.seq_now("coltrane");
        // each chord and its cell, the one sounding lit
        ui.horizontal_wrapped(|ui| {
            for (i, (name, _, _, cell)) in cells.iter().enumerate() {
                let on = playing.is_some_and(|p| p / 4 == i);
                let txt = format!(
                    "{}: {}",
                    crate::chart::pretty_chord(name),
                    cell.iter().map(|m| theory::note_name(*m, sharps)).collect::<Vec<_>>().join(" ")
                );
                let w = crate::font::text_w(&txt, 1.0) + 14.0;
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::hover());
                if on {
                    ui.painter().rect_filled(r, 2.0, pal().sel);
                }
                ptext(ui.painter(), Pos2::new(r.min.x + 6.0, r.center().y), Align::Min, &txt, 1.0, pal().ink);
            }
        });
        let notes: Vec<i32> = cells.iter().flat_map(|c| c.3.clone()).collect();
        crate::lines_ui::line_score(ui, acts, &notes, 2, sharps, playing, None);
    }
}

/// The Coltrane cells offered through a tune: name and what it is.
const CT_CELLS: [(&str, &str); 3] = [
    (
        "1 2 3 5 (1 B3 4 5 OVER MINOR)",
        "UP FROM THE ROOT: 1 2 3 5 OVER MAJOR AND DOMINANT, 1 B3 4 5 OVER MINOR, 1 B3 4 B5 OVER HALF-DIMINISHED",
    ),
    ("5 3 2 1 (5 4 B3 1 OVER MINOR)", "THE SAME CELLS COMING DOWN FROM THE 5TH"),
    ("1 2 3 5 EVERYWHERE (1 2 B3 5 OVER MINOR)", "ONE SHAPE FOR EVERY CHORD - ONLY THE 3RD CHANGES"),
];

/// A tune's changes as Coltrane cells: (chord, root, the chord's 3rd and 7th from its root (to hear it under the cell),
/// the cell's four notes), each cell placed near where the one before ended (within the guitar's middle range).
fn coltrane_cells(changes: &[String], kind: usize) -> Vec<(String, i32, Vec<i32>, Vec<i32>)> {
    let mut out = Vec::new();
    let mut last = 64;
    for c in changes {
        let Some(n) = theory::chord_notes(c) else { continue };
        let root = n.root.rem_euclid(12);
        let has = |iv: i32| n.pcs.iter().any(|p| (p - root).rem_euclid(12) == iv);
        let minor = has(3) && !has(4);
        let half_dim = minor && has(6) && !has(7);
        let dim = half_dim && has(9) && !has(10);
        let fifth = if has(8) && !has(7) {
            8
        } else if half_dim {
            6
        } else {
            7
        };
        let cell: [i32; 4] = match kind {
            2 => [0, 2, if minor { 3 } else { 4 }, fifth],
            _ if dim => [0, 3, 6, 9],
            _ if minor => [0, 3, 5, fifth],
            _ => [0, 2, 4, fifth],
        };
        let cell: Vec<i32> = if kind == 1 { cell.iter().rev().copied().collect() } else { cell.to_vec() };
        // the octave whose first note is nearest the last note played, kept between the low E's G and the 15th fret
        let first =
            (52..=76).filter(|m| (m - (root + cell[0])).rem_euclid(12) == 0).min_by_key(|m| (m - last).abs()).unwrap_or(60);
        let base = first - cell[0];
        let notes: Vec<i32> = cell.iter().map(|x| base + x).collect();
        last = *notes.last().unwrap_or(&last);
        let third = if minor { 3 } else { 4 };
        let seventh = if dim {
            9
        } else if has(10) {
            10
        } else if has(11) {
            11
        } else {
            7
        };
        out.push((c.clone(), root, vec![third + 12, seventh + 12], notes));
    }
    out
}

/// A piano to look at (not to play) under a neck: these keys lit, named from `root`.
pub(crate) fn look_piano(ui: &mut egui::Ui, id: &str, keys: &[i32], root: Option<i32>) {
    let pw = ui.available_width().min(720.0);
    let (pr, _) = ui.allocate_exact_size(Vec2::new(pw, 70.0), Sense::hover());
    crate::fretboard::piano_span(ui, pr, keys, false, id, root.map(|r| r.rem_euclid(12)), 36, 4);
}

/// One dot on the neck picture.
pub(crate) struct NeckDot {
    pub spot: Spot,
    pub text: String,
    pub col: Color32,
    /// 1 bright, lower is dimmer (the boxes not shown)
    pub strength: f32,
    /// a ring rather than a filled dot (the added notes)
    pub ring: bool,
    /// the root: a square
    pub root: bool,
}

/// The neck lying down (high string on top), frets 0 to 17, with these dots; `lit` is ringed brightly (the note
/// sounding).
pub(crate) fn neck_view(ui: &mut egui::Ui, dots: &[NeckDot], lit: Option<Spot>) {
    let w = ui.available_width().min(1100.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 190.0), Sense::hover());
    let neck = Rect::from_min_max(rect.min + Vec2::new(34.0, 10.0), rect.max - Vec2::new(8.0, 26.0));
    let p = ui.painter();
    p.rect_filled(neck, 2.0, pal().lcd);
    // the neck runs to the 17th fret, or further when a position climbs higher (up to the 24th)
    let top = dots.iter().map(|d| d.spot.1).max().unwrap_or(0).clamp(TOP_FRET, 24);
    let frets = top as f32;
    let fw = neck.width() / frets;
    let sh = neck.height() / 6.0;
    let x_of = |f: u8| if f == 0 { neck.min.x - 16.0 } else { neck.min.x + (f as f32 - 0.5) * fw };
    let y_of = |s: usize| neck.min.y + (5 - s) as f32 * sh + sh / 2.0;
    for f in 0..=top {
        let x = neck.min.x + f as f32 * fw;
        p.line_segment(
            [Pos2::new(x, neck.min.y), Pos2::new(x, neck.max.y)],
            Stroke::new(if f == 0 { 4.0_f32 } else { 1.2 }, pal().ink2),
        );
    }
    for f in [3u8, 5, 7, 9, 12, 15, 17, 19, 21, 24].into_iter().filter(|f| *f <= top) {
        ptext(p, Pos2::new(x_of(f), neck.max.y + 12.0), Align::Center, &f.to_string(), 1.0, pal().dim);
    }
    for s in 0..6 {
        p.line_segment(
            [Pos2::new(neck.min.x, y_of(s)), Pos2::new(neck.max.x, y_of(s))],
            Stroke::new(1.0_f32, pal().ink.gamma_multiply(0.5)),
        );
    }
    let r = (sh * 0.42).min(fw * 0.42);
    // dim first, bright on top
    let mut order: Vec<&NeckDot> = dots.iter().collect();
    order.sort_by(|a, b| a.strength.total_cmp(&b.strength));
    for d in order {
        let c = Pos2::new(x_of(d.spot.1), y_of(d.spot.0));
        let col = d.col.gamma_multiply(d.strength);
        if d.ring {
            p.circle_filled(c, r * 0.8, pal().lcd);
            p.circle_stroke(c, r * 0.8, Stroke::new(1.5_f32, col));
            ptext(p, c, Align::Center, &d.text, 1.0, col);
        } else if d.root {
            let sq = Rect::from_center_size(c, Vec2::splat(r * 1.9));
            p.rect_filled(sq, 2.0, col);
            ptext(p, c, Align::Center, &d.text, 1.0, Color32::from_gray(12));
        } else {
            p.circle_filled(c, if d.strength < 0.5 { r * 0.6 } else { r }, col);
            ptext(p, c, Align::Center, &d.text, 1.0, Color32::from_gray(12));
        }
    }
    if let Some(s) = lit {
        let c = Pos2::new(x_of(s.1), y_of(s.0));
        p.circle_stroke(c, r + 3.0, Stroke::new(2.5_f32, pal().ink));
        ui.ctx().request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_and_shapes_over_any_chord() {
        let names = |c: &[i32]| si_scales_for(c).iter().map(|i| SI_SCALES[*i].0).collect::<Vec<_>>();
        let dom = names(&[0, 4, 7, 10]);
        assert!(dom[0].starts_with("MIXOLYDIAN"), "{:?}", dom);
        assert!(dom.iter().any(|n| n.starts_with("ALTERED")) && dom.iter().any(|n| n.starts_with("WHOLE TONE")), "{:?}", dom);
        assert!(names(&[0, 4, 7, 11]).iter().any(|n| n.starts_with("LYDIAN AUGMENTED")) == false);
        assert!(names(&[0, 4, 8, 11]).iter().any(|n| n.starts_with("LYDIAN AUGMENTED")));
        assert!(names(&[0, 3, 6, 9])[0].starts_with("WHOLE-HALF"));
        assert!(names(&[0, 4, 8])[0].starts_with("WHOLE TONE"));
        assert!(names(&[0, 4, 8, 10])[0].starts_with("WHOLE TONE"));
        assert!(names(&[0, 4, 8, 11])[0].starts_with("LYDIAN AUGMENTED"));
        assert!(names(&[0, 1, 4, 7, 10])[0].starts_with("HALF-WHOLE"));
        assert!(names(&[0, 3, 6, 10])[0].starts_with("LOCRIAN NATURAL 9"));
        assert!(names(&[0, 3, 7, 10])[0].starts_with("DORIAN"));
        // every shape suggested sits inside its scale, and something is always suggested
        for chord in [vec![0, 4, 7, 11], vec![0, 4, 7, 10], vec![0, 3, 6, 9], vec![0, 4, 8, 11], vec![0, 3, 7, 9]] {
            for i in si_scales_for(&chord) {
                let scale = &SI_SCALES[i].1[..SI_SCALES[i].2];
                let sug = si_auto(&chord, scale);
                assert!(!sug.is_empty(), "{:?} {}", chord, SI_SCALES[i].0);
                for (k, o) in sug {
                    assert!(SI_SHAPES[k].1.iter().all(|x| scale.contains(&(x + o).rem_euclid(12))));
                }
            }
        }
    }

    #[test]
    fn every_family_lays_out_on_the_neck() {
        for fam in 0..=6u8 {
            let list = family_scales(fam, 9, false);
            assert!(!list.is_empty(), "family {}", fam);
            for sc in &list {
                assert!(sc.steps.iter().all(|s| (0..12).contains(s)), "{}", sc.name);
                for nps in [2, 3] {
                    assert!(!positions(9, &sc.steps, nps).is_empty(), "{} {}", sc.name, nps);
                }
            }
        }
    }

    #[test]
    fn coltrane_cells_follow_the_changes() {
        let ch: Vec<String> = ["Dm7", "G7", "Cmaj7"].iter().map(|s| s.to_string()).collect();
        let c = coltrane_cells(&ch, 0);
        let pcs = |v: &[i32]| v.iter().map(|m| m.rem_euclid(12)).collect::<Vec<_>>();
        assert_eq!(pcs(&c[0].3), vec![2, 5, 7, 9]); // D F G A
        assert_eq!(pcs(&c[1].3), vec![7, 9, 11, 2]); // G A B D
        assert_eq!(pcs(&c[2].3), vec![0, 2, 4, 7]); // C D E G
                                                    // each cell starts near where the last ended
        for w in c.windows(2) {
            assert!((w[1].3[0] - w[0].3[3]).abs() <= 6);
        }
    }

    #[test]
    fn a_minor_box_one_is_the_fifth_fret_box() {
        let boxes = positions(9, &PENT_MINOR, 2);
        assert_eq!(boxes.len(), 5);
        // root A on the low E at the 5th fret, two notes a string, the B string a fret up
        assert_eq!(
            boxes[0],
            vec![(0, 5), (0, 8), (1, 5), (1, 7), (2, 5), (2, 7), (3, 5), (3, 7), (4, 5), (4, 8), (5, 5), (5, 8)]
        );
        assert_eq!(fingers(&boxes[0])[..2], [1, 4]);
        // box 2 starts on the C, a 3rd up
        assert_eq!(boxes[1][0], (0, 8));
    }

    #[test]
    fn every_scale_is_whole() {
        for fam in 0..3u8 {
            for sc in family_scales(fam, 0, false) {
                let n = match fam {
                    0 => 5,
                    1 => 6,
                    _ => 8,
                };
                assert_eq!(sc.steps.len(), n, "{}", sc.name);
                // the positions climb the scale on every string, in every key
                for root in 0..12 {
                    for nps in [2, 3] {
                        let p = positions(root, &sc.steps, nps);
                        assert_eq!(p.len(), n);
                        assert!(p.iter().all(|b| b.len() == nps * 6 || b.len() < nps * 6), "{}", sc.name);
                    }
                }
            }
        }
        // C + D is the Lydian pair: C D E F# G A
        let hex = family_scales(1, 0, false);
        assert_eq!(hex[0].steps, vec![0, 2, 4, 6, 7, 9]);
    }

    #[test]
    fn c_major_has_five_caged_positions() {
        let p = caged(0, &[0, 2, 4, 5, 7, 9, 11]);
        assert_eq!(p.len(), 5);
        // the open position (frets 0-4): every note of the scale there, no note twice
        let open = &p[0];
        let mut pitches: Vec<i32> = open.iter().map(|s| theory::STANDARD[s.0] + s.1 as i32).collect();
        let n = pitches.len();
        pitches.dedup();
        assert_eq!(pitches.len(), n);
        assert!(open.iter().all(|s| s.1 <= 4));
        // the A shape: the root on the 5th string at the 3rd fret
        assert!(p.iter().any(|q| q.contains(&(1, 3)) && q.iter().all(|s| (2..=6).contains(&s.1))));
    }

    #[test]
    fn boxes_stay_on_the_neck() {
        for root in 0..12 {
            for b in positions(root, &PENT_MINOR, 2) {
                assert_eq!(b.len(), 12);
                assert!(b.iter().all(|s| s.1 <= TOP_FRET + 3));
            }
        }
    }
}
