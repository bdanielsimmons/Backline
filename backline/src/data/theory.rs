//! A little music theory for the CHORDS tab and the band's instrument view: the notes of a chord, chord names for
//! a set of notes (the analyzer), playable guitar shapes for a chord, and the scales that fit it.

/// Note names, sharp and flat spellings.
const SHARPS: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const FLATS: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];

/// A pitch class's name (flats unless `sharp`).
pub fn note_name(pc: i32, sharp: bool) -> &'static str {
    let i = pc.rem_euclid(12) as usize;
    if sharp {
        SHARPS[i]
    } else {
        FLATS[i]
    }
}

/// What a chord is made of.
#[derive(Clone, Debug, PartialEq)]
pub struct ChordNotes {
    pub root: i32,
    pub bass: i32,
    /// the pitch classes in it (bass included), lowest-first from the root
    pub pcs: Vec<i32>,
    /// each note's name and its interval name from the root, e.g. ("F", "3")
    pub named: Vec<(String, String)>,
}

/// The notes of a chord written like "Dm7", "G7#9", "Fmaj7/A" or iReal's "D-7". None when it cannot be read.
pub fn chord_notes(name: &str) -> Option<ChordNotes> {
    let c = crate::band::parse_chord(name)?;
    let sharp = name.contains('#') || name.contains('\u{266F}');
    // the root and bass as written: the notes are spelt from them (Ab13b9: Ab C Eb Gb A F, not G# B# ...)
    let mut parts = name.splitn(2, '/');
    let root_w = written_note(parts.next().unwrap_or(""));
    let bass_w = parts.next().map(written_note).filter(|b| crate::chart::note_index(b) == Some(c.bass.rem_euclid(12)));
    let mut pcs: Vec<i32> = Vec::new();
    let mut named = Vec::new();
    for (s, iv) in &c.tones {
        let pc = (c.root + s).rem_euclid(12);
        if !pcs.contains(&pc) {
            pcs.push(pc);
            // (a #9 is written with the minor 3rd's letter, as players and the chord books write it: A7#9's is C,
            // not B#)
            let degree: usize = if iv.trim() == "#9" { 3 } else { iv.trim_start_matches(['b', '#']).parse().unwrap_or(1) };
            let n = spell_degree(&root_w, pc, degree).unwrap_or_else(|| note_name(pc, sharp).to_string());
            named.push((n, iv.to_string()));
        }
    }
    if !pcs.contains(&c.bass.rem_euclid(12)) {
        pcs.insert(0, c.bass.rem_euclid(12));
        let n = bass_w.unwrap_or_else(|| note_name(c.bass, sharp).to_string());
        named.insert(0, (n, "bass".to_string()));
    }
    Some(ChordNotes { root: c.root.rem_euclid(12), bass: c.bass.rem_euclid(12), pcs, named })
}

/// The letter names, and their natural notes.
const LETTERS: [char; 7] = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];
const NATURALS: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

/// The name of the note `pc` as the `degree`-th letter up from a root written `root` ("Ab", "F#"): the 3rd of Ab
/// is C, its b7 Gb, the #9 of D7#9 E#. A double sharp or flat (the b9 of Ab7b9, the bb7 of Cdim7) is written as
/// its plain neighbour: A, not Bbb. None when `root` is not a note name.
pub fn spell_degree(root: &str, pc: i32, degree: usize) -> Option<String> {
    let l = LETTERS.iter().position(|c| root.starts_with(*c))?;
    let mut letter = (l + degree.max(1) - 1) % 7;
    let acc = |letter: usize| (pc - NATURALS[letter] + 6).rem_euclid(12) - 6;
    let mut a = acc(letter);
    if a >= 2 {
        letter = (letter + 1) % 7;
        a = acc(letter);
    } else if a <= -2 {
        letter = (letter + 6) % 7;
        a = acc(letter);
    }
    let sign = match a {
        0 => "",
        1 => "#",
        -1 => "b",
        _ => return None,
    };
    Some(format!("{}{}", LETTERS[letter], sign))
}

/// A note name as written at the start of a chord or a bass ("Bb", "F#", "C"), sharps and flats typed or pasted.
fn written_note(s: &str) -> String {
    let s = s.replace('\u{266F}', "#").replace('\u{266D}', "b");
    crate::chart::split_root(&s).0.to_string()
}

/// Chord types the analyzer knows: the name after the root, and the notes from the root.
const TYPES: &[(&str, &[i32])] = &[
    ("", &[0, 4, 7]),
    ("m", &[0, 3, 7]),
    ("dim", &[0, 3, 6]),
    ("aug", &[0, 4, 8]),
    ("sus2", &[0, 2, 7]),
    ("sus4", &[0, 5, 7]),
    ("5", &[0, 7]),
    ("6", &[0, 4, 7, 9]),
    ("m6", &[0, 3, 7, 9]),
    ("7", &[0, 4, 7, 10]),
    ("maj7", &[0, 4, 7, 11]),
    ("m7", &[0, 3, 7, 10]),
    ("mMaj7", &[0, 3, 7, 11]),
    ("m7b5", &[0, 3, 6, 10]),
    ("dim7", &[0, 3, 6, 9]),
    ("7sus4", &[0, 5, 7, 10]),
    ("7#5", &[0, 4, 8, 10]),
    ("maj7#5", &[0, 4, 8, 11]),
    ("7b5", &[0, 4, 6, 10]),
    ("add9", &[0, 2, 4, 7]),
    ("madd9", &[0, 2, 3, 7]),
    ("69", &[0, 2, 4, 7, 9]),
    ("m69", &[0, 2, 3, 7, 9]),
    ("9", &[0, 2, 4, 7, 10]),
    ("maj9", &[0, 2, 4, 7, 11]),
    ("m9", &[0, 2, 3, 7, 10]),
    ("9sus4", &[0, 2, 5, 7, 10]),
    ("7b9", &[0, 1, 4, 7, 10]),
    ("7#9", &[0, 3, 4, 7, 10]),
    ("7#11", &[0, 4, 6, 7, 10]),
    ("maj7#11", &[0, 4, 6, 7, 11]),
    ("7b13", &[0, 4, 7, 8, 10]),
    ("11", &[0, 2, 5, 7, 10]),
    ("m11", &[0, 2, 3, 5, 7, 10]),
    ("13", &[0, 2, 4, 7, 9, 10]),
    ("m13", &[0, 2, 3, 7, 9, 10]),
    ("maj13", &[0, 2, 4, 7, 9, 11]),
    ("7alt", &[0, 1, 3, 4, 6, 8, 10]),
];

/// Names for a set of notes, best first: exact matches, then ones with the fifth left out, then near misses.
/// `bass` is the lowest note (a different one from the root gives a slash chord).

/// Every way to play these pitch classes one to a string on `strings` (low to high) with the notes climbing:
/// `ordered` keeps them in the order given (a stack), else any order (inversions). Fretted notes within `reach`
/// frets of each other; open strings are free. Lowest on the neck first.
pub fn grips_on(pcs: &[i32], strings: &[usize], ordered: bool, reach: u8) -> Vec<[Option<u8>; 6]> {
    fn search(
        k: usize,
        pcs: &[i32],
        strings: &[usize],
        ordered: bool,
        reach: u8,
        cur: &mut Vec<u8>,
        out: &mut Vec<[Option<u8>; 6]>,
    ) {
        if k == strings.len() {
            let fretted: Vec<u8> = cur.iter().copied().filter(|f| *f > 0).collect();
            let span = fretted.iter().max().unwrap_or(&0) - fretted.iter().min().unwrap_or(&0);
            if span <= reach {
                let mut g = [None; 6];
                for (j, s) in strings.iter().enumerate() {
                    g[*s] = Some(cur[j]);
                }
                out.push(g);
            }
            return;
        }
        let s = strings[k];
        for f in 0..=15u8 {
            let m = STANDARD[s] + f as i32;
            let pc = m.rem_euclid(12);
            let ok = if ordered {
                pc == pcs[k].rem_euclid(12)
            } else {
                pcs.iter().any(|p| p.rem_euclid(12) == pc)
                    && !cur.iter().enumerate().any(|(j, cf)| (STANDARD[strings[j]] + *cf as i32).rem_euclid(12) == pc)
            };
            let climbs = k == 0 || m > STANDARD[strings[k - 1]] + cur[k - 1] as i32;
            if ok && climbs {
                cur.push(f);
                search(k + 1, pcs, strings, ordered, reach, cur, out);
                cur.pop();
            }
        }
    }
    let mut out = Vec::new();
    if pcs.len() == strings.len() {
        search(0, pcs, strings, ordered, reach, &mut Vec::new(), &mut out);
    }
    out.sort_by(|a, b| centre(a).partial_cmp(&centre(b)).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// Where a grip sits on the neck (the middle of its fretted notes; open strings count as fret 0).
pub fn centre(g: &[Option<u8>; 6]) -> f32 {
    let f: Vec<f32> = g.iter().flatten().map(|f| *f as f32).collect();
    if f.is_empty() {
        0.0
    } else {
        f.iter().sum::<f32>() / f.len() as f32
    }
}

pub fn name_chords(notes: &[i32], bass: Option<i32>) -> Vec<String> {
    let set: Vec<i32> = {
        let mut v: Vec<i32> = notes.iter().map(|n| n.rem_euclid(12)).collect();
        v.sort();
        v.dedup();
        v
    };
    if set.is_empty() {
        return Vec::new();
    }
    let sharp = false;
    let mut found: Vec<(i32, String)> = Vec::new();
    for &root in &set {
        let mut rel: Vec<i32> = set.iter().map(|p| (p - root).rem_euclid(12)).collect();
        rel.sort();
        for (suffix, iv) in TYPES {
            let mut f: Vec<i32> = iv.iter().map(|x| x % 12).collect();
            f.sort();
            f.dedup();
            let score = if rel == f {
                100
            } else if f.contains(&7) && rel == f.iter().copied().filter(|x| *x != 7).collect::<Vec<_>>() && f.len() > 3 {
                // the fifth left out, as players often do
                85
            } else if rel.iter().all(|x| f.contains(x)) && f.len() == rel.len() + 1 {
                // one note of the chord missing
                55
            } else {
                continue;
            };
            let slash = match bass.map(|b| b.rem_euclid(12)) {
                Some(b) if b != root => format!("/{}", note_name(b, sharp)),
                _ => String::new(),
            };
            // a root in the bass reads more naturally; simple names beat long ones
            let s = score + if slash.is_empty() { 8 } else { 0 } - suffix.len() as i32;
            found.push((s, format!("{}{}{}", note_name(root, sharp), suffix, slash)));
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.len().cmp(&b.1.len())));
    let mut out: Vec<String> = Vec::new();
    for (_, n) in found {
        if !out.contains(&n) {
            out.push(n);
        }
    }
    out.truncate(8);
    out
}

/// One chord the notes could be: its root, its kind (an entry of TYPES), the bass when it is not the root, whether
/// the notes make the whole chord, the chord's notes not played, and why it might not work.
#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion {
    pub root: i32,
    pub suffix: String,
    pub bass: Option<i32>,
    pub exact: bool,
    pub pcs: Vec<i32>,
    pub missing: Vec<i32>,
    pub warnings: Vec<&'static str>,
}

/// How chord names are written.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spelling {
    /// symbols: - for minor, a triangle for major 7, a circle for diminished, + for augmented
    pub symbols: bool,
    /// sharps for the roots (else flats)
    pub sharps: bool,
    /// the notes left out are named, e.g. C7(no5)
    pub detailed: bool,
    /// a bass that is not the root is written as a slash chord
    pub slash: bool,
}

impl Suggestion {
    /// Its name, written the way `sp` asks.
    pub fn name(&self, sp: Spelling) -> String {
        let mut suffix = self.suffix.to_string();
        if sp.symbols {
            suffix = match self.suffix.as_str() {
                "m" => "-".to_string(),
                "mMaj7" => "-\u{0394}7".to_string(),
                "m7b5" => "\u{00f8}7".to_string(),
                "dim" => "\u{00b0}".to_string(),
                "dim7" => "\u{00b0}7".to_string(),
                "aug" => "+".to_string(),
                s if s.starts_with("maj") => s.replacen("maj", "\u{0394}", 1),
                s if s.starts_with('m') => s.replacen('m', "-", 1),
                s => s.replace("#5", "+5"),
            };
        }
        let mut out = format!("{}{}", note_name(self.root, sp.sharps), suffix);
        if sp.detailed {
            let rel: Vec<i32> = self.missing.iter().map(|p| (p - self.root).rem_euclid(12)).collect();
            let mut no: Vec<&str> = Vec::new();
            if rel.contains(&0) {
                no.push("no root");
            }
            if rel.contains(&3) || rel.contains(&4) {
                no.push("no3");
            }
            if rel.contains(&7) {
                no.push("no5");
            }
            if !no.is_empty() {
                out += &format!("({})", no.join(","));
            }
        }
        if let (true, Some(b)) = (sp.slash, self.bass) {
            out += &format!("/{}", note_name(b, sp.sharps));
        }
        out
    }

    /// Its name for looking it up again (plain spelling, flats, slash): what the rest of Backline reads.
    pub fn key(&self) -> String {
        self.name(Spelling { slash: true, ..Spelling::default() })
    }
}

/// Every chord these notes could belong to, best first: the chords made of exactly these notes (or all but the
/// fifth), then chords that contain them and add more - so two or three notes already suggest many chords. The
/// root need not be among the notes. Each one says why it might not work.
pub fn suggestions(notes: &[i32], bass: Option<i32>) -> Vec<Suggestion> {
    let mut set: Vec<i32> = notes.iter().map(|n| n.rem_euclid(12)).collect();
    set.sort();
    set.dedup();
    if set.len() < 2 {
        return Vec::new();
    }
    let bass = bass.map(|b| b.rem_euclid(12));
    let mut found: Vec<(i32, Suggestion)> = Vec::new();
    for root in 0..12 {
        for (suffix, iv) in TYPES {
            let mut c: Vec<i32> = iv.iter().map(|x| (root + x).rem_euclid(12)).collect();
            c.sort();
            c.dedup();
            if !set.iter().all(|n| c.contains(n)) {
                continue;
            }
            let fifth = (root + 7).rem_euclid(12);
            let missing: Vec<i32> = c.iter().copied().filter(|x| !set.contains(x)).collect();
            let exact = missing.is_empty() || (c.len() > 3 && missing == vec![fifth]);
            let mut score = if missing.is_empty() {
                100
            } else if exact {
                88
            } else {
                60 - missing.len() as i32 * 7
            };
            if set.contains(&root) {
                score += 10;
            }
            match bass {
                Some(b) if b != root => score -= 4,
                Some(_) => score += 6,
                None => {}
            }
            score -= suffix.len() as i32;
            let suffix = suffix.to_string();
            // why it might not work
            let rel = |p: i32| (p - root).rem_euclid(12);
            let played: Vec<i32> = set.iter().map(|p| rel(*p)).collect();
            let has3 = iv.contains(&3) || iv.contains(&4);
            let mut warnings: Vec<&'static str> = Vec::new();
            if !set.contains(&root) {
                warnings.push("The root is not played: the bass player (or the band) has to supply it.");
            }
            if has3 && !played.contains(&3) && !played.contains(&4) {
                warnings.push("No third is played, so it could be major or minor.");
            }
            if played.contains(&4) && played.contains(&5) && !suffix.contains("sus") && !suffix.contains("11") {
                warnings.push("The 11th sits a half step above the major third and clashes with it.");
            }
            if played.contains(&3) && played.contains(&1) {
                warnings.push("A flat 9 on a minor chord is a harsh rub.");
            }
            if missing.len() >= 3 {
                warnings.push("Only a small part of this chord is played.");
            }
            let b = bass.filter(|b| *b != root);
            found.push((score, Suggestion { root, suffix, bass: b, exact, pcs: c, missing, warnings }));
        }
    }
    // chords the table doesn't have (G13b9, C7#9b13, Cm9b5...): named from the notes over each note played
    for &root in &set {
        let rel: Vec<i32> = set.iter().map(|p| (p - root).rem_euclid(12)).collect();
        let Some(suffix) = built_name(&rel) else { continue };
        if found.iter().any(|(_, s)| s.root == root && s.exact) {
            continue;
        }
        // the table's version of the same name with notes missing gives way to this one, made of exactly these notes
        found.retain(|(_, s)| !(s.root == root && s.suffix == suffix));
        // a little under the table's exact chords, so an inversion reads as one (G C E over G is C/G, not G6sus4)
        let mut score = 94 + 10 - suffix.len() as i32;
        match bass {
            Some(b) if b != root => score -= 4,
            Some(_) => score += 6,
            None => {}
        }
        let mut warnings: Vec<&'static str> = Vec::new();
        if rel.contains(&3) && rel.contains(&1) && !rel.contains(&4) {
            warnings.push("A flat 9 on a minor chord is a harsh rub.");
        }
        let b = bass.filter(|b| *b != root);
        found.push((score, Suggestion { root, suffix, bass: b, exact: true, pcs: set.clone(), missing: Vec::new(), warnings }));
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.key().len().cmp(&b.1.key().len())));
    let mut out: Vec<Suggestion> = Vec::new();
    for (_, s) in found {
        if !out.iter().any(|o| o.key() == s.key()) {
            out.push(s);
        }
        if out.len() == 30 {
            break;
        }
    }
    out
}

/// A chord name built from the notes over a root (`rel`: each note's distance above it, the root as 0): the third
/// (or sus), the seventh, the highest natural extension, then the altered ones - b9, #9, #11, b13, b5, #5. None
/// when the notes don't make a sensible chord over this root (every note has to be accounted for).
pub fn built_name(rel: &[i32]) -> Option<String> {
    let has = |x: i32| rel.contains(&x);
    let maj3 = has(4);
    let min3 = has(3) && !maj3;
    let sharp9 = has(3) && maj3;
    let sus4 = !maj3 && !min3 && has(5);
    let sus2 = !maj3 && !min3 && !sus4 && has(2);
    let (b7, maj7) = (has(10), has(11));
    let fifth = has(7);
    let dimish = min3 && has(6) && !fifth;
    // a suspended 4th over a flat 5th and no 5th (B E F A) is no chord anyone names that way
    if sus4 && has(6) && !fifth {
        return None;
    }
    let mut used: Vec<i32> = vec![0];
    if maj3 {
        used.push(4);
    }
    if min3 {
        used.push(3);
    }
    if sus4 {
        used.push(5);
    }
    if sus2 {
        used.push(2);
    }
    if fifth {
        used.push(7);
    }
    let sus_txt = if sus4 {
        "sus4"
    } else if sus2 {
        "sus2"
    } else {
        ""
    };
    let no3 = !maj3 && !min3 && !sus4 && !sus2;
    let name = if b7 || maj7 {
        if b7 && maj7 {
            return None;
        }
        used.push(if b7 { 10 } else { 11 });
        // the highest natural extension (the ones below it are implied, played or not)
        let num = if has(9) && !(dimish && !b7) {
            used.push(9);
            if has(2) {
                used.push(2);
            }
            if has(5) && (min3 || sus4) {
                used.push(5);
            }
            "13"
        } else if has(5) && !sus4 && !maj3 {
            used.push(5);
            if has(2) {
                used.push(2);
            }
            "11"
        } else if has(2) && !sus2 {
            used.push(2);
            "9"
        } else {
            "7"
        };
        let mut alt = String::new();
        if has(1) {
            used.push(1);
            alt += "b9";
        }
        if sharp9 {
            used.push(3);
            alt += "#9";
        }
        if has(5) && !used.contains(&5) {
            used.push(5);
            alt += "add11";
        }
        let mut b5 = false;
        if has(6) && !dimish {
            used.push(6);
            if fifth || num != "7" {
                alt += "#11";
            } else {
                b5 = true;
            }
        }
        if has(8) {
            used.push(8);
            alt += if fifth { "b13" } else { "#5" };
        }
        if b5 {
            alt = format!("b5{}", alt);
        }
        if dimish {
            used.push(6);
            if b7 {
                format!("m{}b5{}", num, alt)
            } else {
                format!("mMaj{}b5{}", num, alt)
            }
        } else if min3 {
            format!("{}{}{}", if maj7 { "mMaj" } else { "m" }, num, alt)
        } else if maj7 {
            format!("maj{}{}{}{}", num, alt, sus_txt, if no3 { "(no3)" } else { "" })
        } else {
            format!("{}{}{}{}", num, alt, sus_txt, if no3 { "(no3)" } else { "" })
        }
    } else {
        // no seventh: triads, sixths, added notes
        if dimish {
            used.push(6);
            if has(9) {
                used.push(9);
                return used_all(rel, &used).then(|| "dim7".to_string());
            }
        }
        let mut s = String::new();
        if min3 && !dimish {
            s.push('m');
        }
        if dimish {
            s.push_str("dim");
        }
        if has(9) {
            used.push(9);
            if has(2) && !sus2 {
                used.push(2);
                s.push_str("69");
            } else {
                s.push('6');
            }
        } else if has(2) && !sus2 {
            used.push(2);
            s.push_str("add9");
        }
        if has(5) && !sus4 && (maj3 || min3) {
            used.push(5);
            s.push_str("add11");
        }
        if maj3 && has(8) && !fifth {
            used.push(8);
            s = if s.is_empty() { "aug".to_string() } else { format!("{}#5", s) };
        }
        if maj3 && has(6) && !fifth {
            used.push(6);
            s.push_str("b5");
        } else if has(6) && fifth && (maj3 || min3) {
            used.push(6);
            s.push_str("add#11");
        }
        if no3 && fifth && s.is_empty() {
            s.push('5');
        }
        s.push_str(sus_txt);
        s
    };
    used_all(rel, &used).then_some(name)
}

/// Every note of `rel` is among `used`.
fn used_all(rel: &[i32], used: &[i32]) -> bool {
    rel.iter().all(|r| used.contains(r))
}

/// The names only, with whether each is exact (plain spelling).
#[cfg(test)]
pub fn chords_containing(notes: &[i32], bass: Option<i32>) -> Vec<(String, bool)> {
    suggestions(notes, bass).into_iter().map(|s| (s.key(), s.exact)).collect()
}

/// The chord finder's funnel: kinds of chord, each with its chords (label, what follows the root).
pub const FINDER_FAMILIES: [(&str, &[(&str, &str)]); 6] = [
    (
        "MAJOR",
        &[
            ("MAJOR", ""),
            ("6", "6"),
            ("6/9", "69"),
            ("ADD9", "add9"),
            ("MAJ7", "maj7"),
            ("MAJ9", "maj9"),
            ("MAJ7#11", "maj7#11"),
            ("MAJ13", "maj13"),
        ],
    ),
    (
        "MINOR",
        &[
            ("MINOR", "m"),
            ("MIN6", "m6"),
            ("MIN ADD9", "madd9"),
            ("MIN7", "m7"),
            ("MIN9", "m9"),
            ("MIN11", "m11"),
            ("MIN(MAJ7)", "m(maj7)"),
        ],
    ),
    (
        "DOMINANT",
        &[
            ("7", "7"),
            ("9", "9"),
            ("13", "13"),
            ("7b9", "7b9"),
            ("7#9", "7#9"),
            ("7#11", "7#11"),
            ("7b5", "7b5"),
            ("7#5", "7#5"),
            ("7b13", "7b13"),
            ("13b9", "13b9"),
            ("13#11", "13#11"),
            ("9#11", "9#11"),
            ("7b9b13", "7b9b13"),
            ("7#9b13", "7#9b13"),
            ("7#9#5", "7#9#5"),
            ("7b5b9", "7b5b9"),
            ("7b9#11", "7b9#11"),
        ],
    ),
    ("SUSPENDED", &[("SUS2", "sus2"), ("SUS4", "sus4"), ("7SUS4", "7sus4"), ("9SUS4", "9sus4")]),
    ("DIMINISHED", &[("DIM", "dim"), ("DIM7", "dim7"), ("HALF-DIM (MIN7b5)", "m7b5")]),
    ("AUGMENTED", &[("AUG", "aug"), ("AUG7", "aug7"), ("MAJ7#5", "maj7#5")]),
];

/// Standard guitar tuning, low string first (MIDI notes).
pub const STANDARD: [i32; 6] = [40, 45, 50, 55, 59, 64];

/// Playable guitar shapes for a chord, best first: per string a fret, or None for a string not played. Every
/// note of the chord sounds (or all but the fifth), the chord's bass is the lowest note, within a four-fret reach.
// ----------------------------------------------------------------- the grips jazz guitarists use
// Each is written as the chord degree on each string, low E to high E (semitones above the root: R 0, b9 1, 9 2,
// #9/b3 3, 3 4, 11 5, #11/b5 6, 5 7, #5/b13 8, 6/13 9, b7 10, 7 11; X a string not played), with the root the lowest
// note. The frets come from the degrees, so a grip is right in every key by construction (and a test checks it).
const X: i8 = -1;
pub(crate) const GRIPS: &[(&str, [i8; 6])] = &[
    // root on the 6th string
    ("maj7", [0, X, 11, 4, 7, X]),
    ("6", [0, X, 9, 4, 7, X]),
    ("7", [0, X, 10, 4, 7, X]),
    ("m7", [0, X, 10, 3, 7, X]),
    ("m7b5", [0, X, 10, 3, 6, X]),
    ("dim7", [0, X, 9, 3, 6, X]),
    ("m6", [0, X, 9, 3, 7, X]),
    ("mmaj7", [0, X, 11, 3, 7, X]),
    ("9", [0, X, 10, 4, X, 2]),
    ("maj9", [0, X, 11, 4, X, 2]),
    ("m9", [0, X, 10, 3, X, 2]),
    ("m11", [0, X, 10, 3, 5, X]),
    ("13", [0, X, 10, 4, 9, X]),
    ("13 9", [0, X, 10, 4, 9, 2]),
    ("13b9", [0, X, 10, 4, 9, 1]),
    ("7b9", [0, X, 10, 4, 7, 1]),
    ("7#9", [0, X, 10, 4, X, 3]),
    ("7#5", [0, X, 10, 4, 8, X]),
    ("7#11", [0, X, 10, 4, 6, X]),
    ("7sus4", [0, X, 10, 5, 7, X]),
    ("maj7#11", [0, X, 11, 4, 6, X]),
    ("69", [0, 4, 9, 2, 7, 0]),
    // root on the 5th string
    ("maj7", [X, 0, 7, 11, 4, X]),
    ("6", [X, 0, 7, 9, 4, X]),
    ("69", [X, 0, 4, 9, 2, 7]),
    ("7", [X, 0, 7, 10, 4, X]),
    ("m7", [X, 0, 7, 10, 3, X]),
    ("m7b5", [X, 0, 6, 10, 3, X]),
    ("dim7", [X, 0, 6, 9, 3, X]),
    ("m6", [X, 0, 7, 9, 3, X]),
    ("mmaj7", [X, 0, 7, 11, 3, X]),
    ("9", [X, 0, 4, 10, 2, 7]),
    ("9 no5", [X, 0, 4, 10, 2, X]),
    ("maj9", [X, 0, 4, 11, 2, X]),
    ("m9", [X, 0, 3, 10, 2, X]),
    ("m11", [X, 0, X, 10, 3, 5]),
    ("13", [X, 0, X, 10, 2, 9]),
    ("13 3", [X, 0, 4, 10, 2, 9]),
    ("13b9", [X, 0, 4, 10, 1, 9]),
    ("7b9", [X, 0, 4, 10, 1, X]),
    ("7#9", [X, 0, 4, 10, 3, X]),
    ("7#5", [X, 0, X, 10, 4, 8]),
    ("7#11", [X, 0, 4, 10, X, 6]),
    ("maj7#11", [X, 0, 6, 11, 4, X]),
    ("7b9b13", [X, 0, 4, 10, 1, 8]),
    ("7#9b13", [X, 0, 4, 10, 3, 8]),
    ("7sus4", [X, 0, 7, 10, 5, X]),
    ("9sus4", [X, 0, 5, 10, 2, X]),
    // root on the 4th string
    ("maj7", [X, X, 0, 7, 11, 4]),
    ("6", [X, X, 0, 7, 9, 4]),
    ("7", [X, X, 0, 7, 10, 4]),
    ("m7", [X, X, 0, 7, 10, 3]),
    ("m7b5", [X, X, 0, 6, 10, 3]),
    ("dim7", [X, X, 0, 6, 9, 3]),
    ("m6", [X, X, 0, 7, 9, 3]),
    ("9", [X, X, 0, 4, 10, 2]),
    ("maj9", [X, X, 0, 4, 11, 2]),
    ("m9", [X, X, 0, 3, 10, 2]),
    ("7b9", [X, X, 0, 4, 10, 1]),
    ("7#9", [X, X, 0, 4, 10, 3]),
    // ---- the rest of the chord books' standard grips, so every chord the finder offers has its root on the 6th
    // string and on the 5th (Ted Greene, Joe Pass, the common barre and drop shapes)
    // triads: the E, A and D barre shapes
    ("", [0, 7, 0, 4, 7, 0]),
    ("", [X, 0, 7, 0, 4, 7]),
    ("", [X, X, 0, 7, 0, 4]),
    ("m", [0, 7, 0, 3, 7, 0]),
    ("m", [X, 0, 7, 0, 3, 7]),
    ("m", [X, X, 0, 7, 0, 3]),
    ("dim", [0, 6, 0, 3, X, X]),
    ("dim", [X, 0, 6, 0, 3, X]),
    ("aug", [0, X, 0, 4, 8, X]),
    ("aug", [X, 0, 4, 8, 0, X]),
    ("sus2", [0, 7, 0, 2, 7, 0]),
    ("sus2", [X, 0, 7, 0, 2, 7]),
    ("sus4", [0, 7, 0, 5, 7, 0]),
    ("sus4", [X, 0, 7, 0, 5, 7]),
    // sixths and adds
    ("6", [X, X, 0, 7, 9, 4]),
    ("add9", [0, 4, 7, 2, X, X]),
    ("add9", [X, 0, 4, X, 2, 7]),
    ("madd9", [0, 3, 7, 2, X, X]),
    ("madd9", [X, 0, 3, X, 2, 7]),
    ("69", [X, 0, 4, 9, 2, 7]),
    // major sevenths and on
    ("maj7", [0, 7, 11, 4, X, X]),
    ("maj13", [0, X, 11, 4, 9, X]),
    ("maj13", [X, 0, X, 11, 4, 9]),
    ("maj7#5", [0, X, 11, 4, 8, X]),
    ("maj7#5", [X, 0, 8, 11, 4, X]),
    // dominants: the barre shapes, then the altered ones
    ("7", [0, 7, 10, 4, 7, 0]),
    ("7", [X, 0, 7, 10, 4, 7]),
    ("m7", [0, 7, 10, 3, 7, 0]),
    ("m7", [X, 0, 7, 10, 3, 7]),
    ("7b5", [X, 0, 6, 10, 4, X]),
    ("7b5", [0, X, 10, 4, 6, X]),
    ("13#11", [0, 6, 10, 4, 9, X]),
    ("13#11", [X, 0, 6, 10, 4, 9]),
    // the C and G shapes (CAGED): open C and G at the nut, and up the neck
    ("", [X, 0, 4, 7, 0, 4]),
    ("", [0, 4, 7, 0, 4, 0]),
    ("m", [X, 0, 3, 7, 0, X]),
    ("9#11", [X, 0, 4, 10, 2, 6]),
    ("7b9b13", [0, X, 10, 4, 8, 1]),
    ("7#9b13", [0, X, 10, 4, 8, 3]),
    ("7b5b9", [X, 0, 4, 10, 1, 6]),
    ("9sus4", [0, X, 10, 2, 5, X]),
    ("9sus4", [0, X, 10, 5, X, 2]),
    ("9sus4", [X, 0, X, 10, 2, 5]),
];

/// A grip's shape: per string, the fret relative to the root's (None: not played). Worked out once, mid-neck
/// where nothing is near the nut, so the shape is the same in every key - a movable grip, as a guitarist slides it.
/// None when it won't sit within four frets.
pub(crate) fn grip_offsets(degrees: &[i8; 6], tuning: &[i32; 6]) -> Option<[Option<i32>; 6]> {
    let rs = degrees.iter().position(|d| *d == 0)?;
    let at = 7;
    let root = tuning[rs] + at;
    let mut out = [None; 6];
    for (s, d) in degrees.iter().enumerate() {
        if *d < 0 {
            continue;
        }
        let base = (root + *d as i32 - tuning[s]).rem_euclid(12);
        // the fret for that note nearest the root's (a shape leans up the neck from it)
        let f = [base, base + 12].into_iter().min_by_key(|f| ((f - at) * 2 - 1).abs())?;
        out[s] = Some(f - at);
    }
    let offs: Vec<i32> = out.iter().flatten().copied().collect();
    let (lo, hi) = (offs.iter().min().copied()?, offs.iter().max().copied()?);
    (hi - lo <= 3).then_some(out)
}

/// A grip with its root at fret `f0`: the shape moved there whole (None when part of it falls off the neck).
fn place_grip(degrees: &[i8; 6], root: i32, f0: i32, tuning: &[i32; 6]) -> Option<[Option<u8>; 6]> {
    let rs = degrees.iter().position(|d| *d == 0)?;
    if (tuning[rs] + f0 - root).rem_euclid(12) != 0 {
        return None;
    }
    let offs = grip_offsets(degrees, tuning)?;
    let mut out = [None; 6];
    for (s, o) in offs.iter().enumerate() {
        if let Some(o) = o {
            let f = f0 + o;
            if !(0..=17).contains(&f) {
                return None;
            }
            out[s] = Some(f as u8);
        }
    }
    Some(out)
}

/// The library grips that spell this chord: every note a grip plays is in the chord, and the chord's notes it leaves
/// out are only ones a guitarist leaves out (the 5th; the 9th under a 13th). Lowest on the neck first.
pub(crate) fn library_grips(c: &ChordNotes, tuning: &[i32; 6]) -> Vec<[Option<u8>; 6]> {
    library_grips_indexed(c, tuning).into_iter().map(|(_, g)| g).collect()
}

/// The library grips with each one's place in the table (GRIPS lists the grips to know first first).
fn library_grips_indexed(c: &ChordNotes, tuning: &[i32; 6]) -> Vec<(usize, [Option<u8>; 6])> {
    if c.bass != c.root {
        return Vec::new();
    }
    let rel: Vec<i32> = c.pcs.iter().map(|p| (p - c.root).rem_euclid(12)).collect();
    let has = |x: i32| rel.contains(&x);
    let mut out: Vec<(i32, usize, [Option<u8>; 6])> = Vec::new();
    for (gi, (_, degrees)) in GRIPS.iter().enumerate() {
        let set: Vec<i32> = degrees.iter().filter(|d| **d >= 0).map(|d| *d as i32).collect();
        if !set.iter().all(|d| has(*d)) {
            continue;
        }
        let left_out: Vec<i32> = rel.iter().copied().filter(|d| !set.contains(d)).collect();
        // a 7alt is played as 3 and b7 with some of its altered notes, never all four (C7#9b13, C7b9b13)
        let alt = has(4) && has(10) && has(1) && has(3) && has(6) && has(8);
        let alt_ok = alt && set.contains(&4) && set.contains(&10) && set.iter().any(|d| matches!(d, 1 | 3 | 6 | 8));
        // (a guitarist leaves out the 5th, and the 9th under an 11th or a 13th)
        let ok = alt_ok || left_out.iter().all(|d| *d == 7 || (*d == 2 && (has(9) || (has(5) && (has(3) || has(4))))));
        if !ok {
            continue;
        }
        // each grip once, where it sits lowest on the neck with all of it on (every shape shows in every key)
        let rs = degrees.iter().position(|d| *d == 0).unwrap_or(0);
        let f0 = (c.root - tuning[rs]).rem_euclid(12);
        if let Some((f, g)) = [f0, f0 + 12].into_iter().find_map(|f| place_grip(degrees, c.root, f, tuning).map(|g| (f, g))) {
            if !out.iter().any(|(_, _, o)| *o == g) {
                out.push((f, gi, g));
            }
        }
    }
    out.sort_by_key(|(f, _, g)| (*f, g.iter().position(|x| x.is_some()).unwrap_or(0)));
    out.into_iter().map(|(_, i, g)| (i, g)).collect()
}

// ----------------------------------------------------------------- voicing groups (four-note chords, every way)
// After the drop voicings arrangers use (Ted Greene's V-System sorts them the same way): a chord's four notes stacked
// close, then the 2nd voice from the top dropped an octave (drop 2), the 3rd (drop 3), or the 2nd and 4th (drop 2 & 4).
// Each in all four inversions, on the string sets where that spacing falls under the hand.

/// The voicing groups: name, which voices of the close voicing (counted from the top, 1 = top) drop an octave, and
/// the string sets (low string first, 0 = low E) the spacing fits on.
pub const VOICING_GROUPS: &[(&str, &[usize], &[[usize; 4]])] = &[
    ("Drop 2", &[2], &[[0, 1, 2, 3], [1, 2, 3, 4], [2, 3, 4, 5]]),
    ("Drop 3", &[3], &[[0, 2, 3, 4], [1, 3, 4, 5]]),
    ("Drop 2 & 4", &[2, 4], &[[0, 1, 3, 4], [1, 2, 4, 5], [0, 2, 3, 5]]),
    ("Close", &[], &[[1, 2, 3, 4], [2, 3, 4, 5], [0, 1, 2, 3]]),
];

/// The four notes a chord is voiced with (semitones above the root, each once): a four-note chord as it is; a
/// bigger one keeps the 3rd and the 7th (the 6th of a sixth chord) and the tensions that name it - altered ones first,
/// then 13, 11, 9 - dropping the 5th and then the root, as the chord books thin them. `rooted`: keep the root.
pub fn four_note_form(rel: &[i32], rooted: bool) -> Option<Vec<i32>> {
    let mut set: Vec<i32> = rel.to_vec();
    set.sort();
    set.dedup();
    if set.len() < 4 {
        return None;
    }
    if set.len() == 4 {
        return (rooted == set.contains(&0)).then_some(set);
    }
    let has = |x: i32| set.contains(&x);
    let mut out: Vec<i32> = Vec::new();
    // the 3rd (or the 4th of a sus chord), the 7th (or the 6th)
    for g in [4, 3, 5] {
        if has(g) {
            out.push(g);
            break;
        }
    }
    for g in [10, 11, 9] {
        if has(g) {
            out.push(g);
            break;
        }
    }
    if rooted {
        out.push(0);
    }
    for x in [1, 3, 6, 8, 9, 5, 2] {
        if out.len() >= 4 {
            break;
        }
        if has(x) && !out.contains(&x) {
            out.push(x);
        }
    }
    for x in [0, 7] {
        if out.len() < 4 && has(x) && !out.contains(&x) && (x != 0 || rooted) {
            out.push(x);
        }
    }
    out.sort();
    (out.len() == 4).then_some(out)
}

/// One voicing of a group: its frets, which group, the note in the bass (semitones above the root) and the strings.
#[derive(Clone, Debug, PartialEq)]
pub struct Voicing {
    pub frets: [Option<u8>; 6],
    pub group: &'static str,
    pub bass: i32,
    pub strings: [usize; 4],
}

/// The fingers a grip takes: one per fretted note, except that notes on the same fret on neighbouring strings with
/// nothing fretted lower between them take one finger together (a barre). Open strings take none.
pub fn grip_fingers(offs: &[Option<i32>; 6]) -> usize {
    let lowest = offs.iter().flatten().copied().min().unwrap_or(0);
    let mut n = 0;
    let mut frets: Vec<i32> = offs.iter().flatten().copied().collect();
    frets.sort();
    frets.dedup();
    for f in frets {
        let mut run = false;
        for s in 0..6 {
            match offs[s] {
                Some(x) if x == f => {
                    if !run {
                        n += 1;
                        run = true;
                    }
                }
                Some(x) if x > f => {}
                None if f == lowest => {}
                _ => run = false,
            }
        }
    }
    n
}

/// Every voicing of the chord in each group: its four notes (with the root, and without when it is a bigger chord),
/// each inversion on each of the group's string sets, low on the neck, in four frets and four fingers.
pub fn voicing_groups(c: &ChordNotes, tuning: &[i32; 6]) -> Vec<(&'static str, Vec<Voicing>)> {
    let rel: Vec<i32> = c.pcs.iter().map(|p| (p - c.root).rem_euclid(12)).collect();
    let forms: Vec<Vec<i32>> = [true, false].into_iter().filter_map(|r| four_note_form(&rel, r)).collect();
    let mut out = Vec::new();
    for (name, drops, sets) in VOICING_GROUPS {
        let mut list: Vec<Voicing> = Vec::new();
        for form in &forms {
            for inv in 0..4 {
                // the close voicing from this inversion's bass up, within an octave
                let mut close: Vec<i32> = (0..4).map(|k| form[(inv + k) % 4]).collect();
                for k in 1..4 {
                    while close[k] <= close[k - 1] {
                        close[k] += 12;
                    }
                }
                // drop the chosen voices (counted from the top) an octave
                let mut v = close.clone();
                for d in drops.iter() {
                    v[4 - d] -= 12;
                }
                v.sort();
                for set in sets.iter() {
                    // the voicing put on the strings: the root's pitch class plus each voice, the lowest octave that
                    // keeps every fret on the neck
                    let base: Vec<i32> = (0..4).map(|k| c.root + v[k] - tuning[set[k]]).collect();
                    let lift = (0..30).map(|o| o * 12 - 24).find(|l| base.iter().all(|b| b + l >= 0));
                    let Some(lift) = lift else { continue };
                    let frets: Vec<i32> = base.iter().map(|b| b + lift).collect();
                    let fretted: Vec<i32> = frets.iter().copied().filter(|f| *f > 0).collect();
                    let (lo, hi) = (fretted.iter().min().copied().unwrap_or(0), fretted.iter().max().copied().unwrap_or(0));
                    if hi - lo > 3 || hi > 17 {
                        continue;
                    }
                    let mut g = [None; 6];
                    let mut offs = [None; 6];
                    for k in 0..4 {
                        g[set[k]] = Some(frets[k] as u8);
                        offs[set[k]] = Some(frets[k]);
                    }
                    if grip_fingers(&offs) > 4 {
                        continue;
                    }
                    let vc = Voicing { frets: g, group: name, bass: v[0].rem_euclid(12), strings: *set };
                    if !list.iter().any(|x| x.frets == vc.frets) {
                        list.push(vc);
                    }
                }
            }
        }
        // string sets low to high, then up the neck
        list.sort_by_key(|x| (x.strings[0], x.frets.iter().flatten().copied().filter(|f| *f > 0).min().unwrap_or(0)));
        out.push((*name, list));
    }
    out
}

/// A chord's grips the way a chord book lays them out: the ones to know first (the main grip with the root on the 6th
/// string and the main one on the 5th), open-position grips near the nut, and every other movable shape.
#[derive(Clone, Debug, Default)]
pub struct GripSections {
    pub must: Vec<[Option<u8>; 6]>,
    pub open: Vec<[Option<u8>; 6]>,
    pub moveable: Vec<[Option<u8>; 6]>,
}

pub fn grip_sections(c: &ChordNotes, tuning: &[i32; 6]) -> GripSections {
    let lib = library_grips_indexed(c, tuning);
    let root_string = |g: &[Option<u8>; 6]| g.iter().position(|f| f.is_some()).unwrap_or(0);
    let mut s = GripSections::default();
    for rs in [0, 1] {
        if let Some((_, g)) = lib.iter().filter(|(_, g)| root_string(g) == rs).min_by_key(|(i, _)| *i) {
            s.must.push(*g);
        }
    }
    // open position: grips with open strings ringing, near the nut (found by the search, held to the same rules)
    for g in search_shapes(c, tuning) {
        let open = g.iter().any(|f| *f == Some(0));
        let hi = g.iter().flatten().copied().max().unwrap_or(0);
        if open && hi <= 4 && !lib.iter().any(|(_, l)| *l == g) && s.open.len() < 3 {
            s.open.push(g);
        }
    }
    for (_, g) in &lib {
        if !s.must.contains(g) {
            s.moveable.push(*g);
        }
    }
    // a chord the library doesn't know (a slash chord, an add11...): what the search finds is what it has
    if lib.is_empty() {
        for g in search_shapes(c, tuning) {
            if !s.open.contains(&g) && s.moveable.len() < 4 {
                s.moveable.push(g);
            }
        }
    }
    s
}

pub fn guitar_shapes(c: &ChordNotes, tuning: &[i32; 6]) -> Vec<[Option<u8>; 6]> {
    // the grips players use first, then what the search finds for anything they don't cover
    // (the search only for chords the library doesn't cover: it finds odd grips beside real ones)
    let out = library_grips(c, tuning);
    if !out.is_empty() {
        return out;
    }
    let mut out = out;
    for g in search_shapes(c, tuning) {
        let pos = |x: &[Option<u8>; 6]| x.iter().flatten().filter(|f| **f > 0).min().copied().unwrap_or(0);
        if !out.iter().any(|o| *o == g || pos(o).abs_diff(pos(&g)) < 2) {
            out.push(g);
        }
        if out.len() >= 4 {
            break;
        }
    }
    out
}

fn search_shapes(c: &ChordNotes, tuning: &[i32; 6]) -> Vec<[Option<u8>; 6]> {
    let mut best: Vec<(i32, [Option<u8>; 6])> = Vec::new();
    let needed: Vec<i32> = c.pcs.clone();
    for start in 0..=10i32 {
        // each string: not played, or a fret in reach that gives a chord note (open strings count anywhere)
        let options: Vec<Vec<Option<u8>>> = tuning
            .iter()
            .map(|open| {
                let mut v: Vec<Option<u8>> = vec![None];
                for f in std::iter::once(0).chain(start.max(1)..start.max(1) + 4) {
                    if needed.contains(&((open + f).rem_euclid(12))) && !v.contains(&Some(f as u8)) {
                        v.push(Some(f as u8));
                    }
                }
                v
            })
            .collect();
        let mut pick = [None; 6];
        fn walk(s: usize, o: &[Vec<Option<u8>>], pick: &mut [Option<u8>; 6], out: &mut Vec<[Option<u8>; 6]>) {
            if s == 6 {
                out.push(*pick);
                return;
            }
            for f in &o[s] {
                pick[s] = *f;
                walk(s + 1, o, pick, out);
            }
        }
        let mut all = Vec::new();
        walk(0, &options, &mut pick, &mut all);
        for shape in all {
            let played: Vec<(usize, u8)> = shape.iter().enumerate().filter_map(|(i, f)| f.map(|f| (i, f))).collect();
            if played.len() < 3 {
                continue;
            }
            let notes: Vec<i32> = played.iter().map(|(i, f)| (tuning[*i] + *f as i32).rem_euclid(12)).collect();
            // the lowest sounding note (not just the lowest string: an open string can sound under a fretted one)
            // is the chord's bass
            let low = played.iter().map(|(i, f)| tuning[*i] + *f as i32).min().unwrap_or(0);
            if low.rem_euclid(12) != c.bass {
                continue;
            }
            // nothing a half or whole step over the bass (Ab with an open A over it): mud, not a voicing
            let mut sounding: Vec<i32> = played.iter().map(|(i, f)| tuning[*i] + *f as i32).collect();
            sounding.sort();
            if sounding.len() > 1 && sounding[1] - sounding[0] < 3 {
                continue;
            }
            // the 3rd and the 7th say what the chord is: never left out
            if [3, 4, 10, 11].iter().any(|d| {
                let pc = (c.root + d).rem_euclid(12);
                needed.contains(&pc) && !notes.contains(&pc)
            }) {
                continue;
            }
            let missing: Vec<i32> = needed.iter().copied().filter(|n| !notes.contains(n)).collect();
            let fifth = (c.root + 7).rem_euclid(12);
            if !(missing.is_empty() || missing == vec![fifth]) {
                continue;
            }
            let fretted: Vec<u8> = played.iter().map(|p| p.1).filter(|f| *f > 0).collect();
            let (lo, hi) = (fretted.iter().min().copied().unwrap_or(0), fretted.iter().max().copied().unwrap_or(0));
            if hi.saturating_sub(lo) > 3 {
                continue;
            }
            // open strings only in shapes near the nut, where the hand can reach both
            if played.iter().any(|p| p.1 == 0) && lo > 4 {
                continue;
            }
            // strings not played in between the played ones are hard to mute
            let first = played[0].0;
            let last = played[played.len() - 1].0;
            let mut gaps = (first..=last).filter(|i| shape[*i].is_none()).count() as i32;
            // except the string just above the bass: the bass finger leans on it (G13b9 as 3 x 3 4 5 4)
            if gaps == 1 && first + 1 < shape.len() && shape[first + 1].is_none() {
                gaps = 0;
            }
            let mut score = played.len() as i32 * 10 - gaps * 25 - lo as i32 * 2 - if missing.is_empty() { 0 } else { 6 };
            // fingers: more than four fretted notes needs a barre at the lowest fret
            let fingers = fretted.iter().filter(|f| **f > lo).count() + usize::from(lo > 0);
            if fingers > 4 {
                score -= 30;
            }
            best.push((score, shape));
        }
    }
    best.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out: Vec<[Option<u8>; 6]> = Vec::new();
    for (_, s) in best {
        // different enough from the ones already chosen (another position on the neck)
        let pos = |x: &[Option<u8>; 6]| x.iter().flatten().filter(|f| **f > 0).min().copied().unwrap_or(0);
        if !out.iter().any(|o| *o == s || pos(o).abs_diff(pos(&s)) < 2) {
            out.push(s);
        }
        if out.len() == 4 {
            break;
        }
    }
    out
}

// ----------------------------------------------------------------- two-note clusters, after Robbie Barnby's
// "Cluster Voicings for Advanced Guitar": two notes a semitone or a tone apart, a bass under them, and the modes
// of the four parent scales that hold them.

/// The four parent scales and their seven modes, by name.
pub const PARENT_MODES: [(&str, [i32; 7], [&str; 7]); 4] = [
    ("Major", [0, 2, 4, 5, 7, 9, 11], ["Ionian", "Dorian", "Phrygian", "Lydian", "Mixolydian", "Aeolian", "Locrian"]),
    (
        "Melodic Minor",
        [0, 2, 3, 5, 7, 9, 11],
        ["Melodic Minor", "Phrygian Nat 6", "Lydian Augmented", "Lydian Dominant", "Mixolydian b6", "Locrian Nat 9", "Altered"],
    ),
    (
        "Harmonic Major",
        [0, 2, 4, 5, 7, 8, 11],
        [
            "Harmonic Major",
            "Locrian Nat 9 Nat 13",
            "Altered Nat 5",
            "Melodic Minor #4",
            "Mixolydian b2",
            "Lydian Augmented #2",
            "Locrian Diminished",
        ],
    ),
    (
        "Harmonic Minor",
        [0, 2, 3, 5, 7, 8, 11],
        [
            "Harmonic Minor",
            "Locrian Nat 13",
            "Ionian Augmented",
            "Dorian #11",
            "Phrygian Dominant",
            "Lydian #9",
            "Altered Diminished",
        ],
    ),
];

/// Symmetrical scales that also hold many clusters.
pub const OTHER_SCALES: [(&str, &[i32]); 3] = [
    ("Whole Half Diminished", &[0, 2, 3, 5, 6, 8, 9, 11]),
    ("Half Whole Diminished", &[0, 1, 3, 4, 6, 7, 9, 10]),
    ("Augmented", &[0, 3, 4, 7, 8, 11]),
];

/// The notes of mode `m` of a parent scale, from its own root (0).
pub fn mode_steps(scale: &[i32; 7], m: usize) -> Vec<i32> {
    (0..7).map(|k| (scale[(m + k) % 7] - scale[m]).rem_euclid(12)).collect()
}

/// A note's degree over a bass, the way the lessons name it ("7th", "#4th (b5th)").
pub fn degree_word(pc: i32, bass: i32) -> &'static str {
    const W: [&str; 12] =
        ["Root", "b2nd", "2nd", "b3rd (#2nd)", "3rd", "4th", "#4th (b5th)", "5th", "b6th (#5th)", "6th", "b7th", "7th"];
    W[(pc - bass).rem_euclid(12) as usize]
}

/// The modes rooted on `bass` that hold every one of `notes`: per parent scale its mode names (an empty list is
/// an "X"), then the symmetrical scales that do.
pub fn modes_holding(bass: i32, notes: &[i32]) -> (Vec<(&'static str, Vec<&'static str>)>, Vec<&'static str>) {
    let rel: Vec<i32> = notes.iter().map(|n| (n - bass).rem_euclid(12)).collect();
    let parents = PARENT_MODES
        .iter()
        .map(|(parent, scale, names)| {
            let fit: Vec<&str> =
                (0..7).filter(|m| rel.iter().all(|r| mode_steps(scale, *m).contains(r))).map(|m| names[m]).collect();
            (*parent, fit)
        })
        .collect();
    let others = OTHER_SCALES.iter().filter(|(_, s)| rel.iter().all(|r| s.contains(r))).map(|(n, _)| *n).collect();
    (parents, others)
}

/// The semitone (step 1) or tone (step 2) pairs inside a parent scale: the degrees (0 = its first note) where a pair
/// starts.
pub fn pairs_in(scale: &[i32; 7], step: i32) -> Vec<usize> {
    (0..7).filter(|d| (scale[(d + 1) % 7] - scale[*d]).rem_euclid(12) == step).collect()
}

/// Where a chord can come from: every mode rooted on the chord's root that holds all its notes, as (parent index,
/// mode index, the parent scale's own root note).
pub fn chord_sources(c: &ChordNotes) -> Vec<(usize, usize, i32)> {
    let rel: Vec<i32> = c.pcs.iter().map(|p| (p - c.root).rem_euclid(12)).collect();
    let mut out = Vec::new();
    for (pi, (_, scale, _)) in PARENT_MODES.iter().enumerate() {
        for m in 0..7 {
            if rel.iter().all(|r| mode_steps(scale, m).contains(r)) {
                out.push((pi, m, (c.root - scale[m]).rem_euclid(12)));
            }
        }
    }
    out
}

/// The clusters a mode offers over its root: its neighbouring notes `step` apart (1 semitone, 2 tone), as pitch
/// classes.
pub fn mode_clusters(scale: &[i32; 7], m: usize, root: i32, step: i32) -> Vec<[i32; 2]> {
    let steps = mode_steps(scale, m);
    (0..7)
        .filter(|k| (steps[(k + 1) % 7] - steps[*k]).rem_euclid(12) == step)
        .map(|k| [(root + steps[k]).rem_euclid(12), (root + steps[(k + 1) % 7]).rem_euclid(12)])
        .collect()
}

/// Grips for a two-note cluster over a bass: the bass on the 6th or 5th string (the lowest note), the two cluster
/// notes on two neighbouring higher strings (either note on either string - the higher string may hold the lower
/// note, which is what makes them reachable), and maybe one more chord tone from `extras` (best first). Open strings
/// are welcome, the reach is at most five frets; different spots on the neck, best first.
pub fn cluster_voicings(bass: i32, pair: [i32; 2], extras: &[i32], tuning: &[i32; 6]) -> Vec<[Option<u8>; 6]> {
    let frets_for = |s: usize, pc: i32| -> Vec<u8> {
        (0..=15u8).filter(|f| (tuning[s] + *f as i32).rem_euclid(12) == pc.rem_euclid(12)).collect()
    };
    let mut found: Vec<(i32, [Option<u8>; 6])> = Vec::new();
    for b in 0..2usize {
        for fb in frets_for(b, bass) {
            for s in (b + 1)..5usize {
                for (lo, hi) in [(pair[0], pair[1]), (pair[1], pair[0])] {
                    for f1 in frets_for(s, lo) {
                        for f2 in frets_for(s + 1, hi) {
                            // with nothing more, then with each extra on each other string above the bass
                            let mut tries: Vec<(Option<(usize, u8)>, i32)> = vec![(None, 0)];
                            for (k, e) in extras.iter().enumerate() {
                                for t in (b + 1)..6usize {
                                    if t == s || t == s + 1 {
                                        continue;
                                    }
                                    for fe in frets_for(t, *e) {
                                        tries.push((Some((t, fe)), 4 - k as i32));
                                    }
                                }
                            }
                            for (extra, bonus) in tries {
                                let mut shape = [None; 6];
                                shape[b] = Some(fb);
                                shape[s] = Some(f1);
                                shape[s + 1] = Some(f2);
                                if let Some((t, fe)) = extra {
                                    shape[t] = Some(fe);
                                }
                                let fretted: Vec<u8> = shape.iter().flatten().copied().filter(|f| *f > 0).collect();
                                let (mn, mx) =
                                    (fretted.iter().min().copied().unwrap_or(0), fretted.iter().max().copied().unwrap_or(0));
                                if mx.saturating_sub(mn) > 4 {
                                    continue;
                                }
                                // the bass is the lowest note
                                let low = tuning[b] + fb as i32;
                                if shape
                                    .iter()
                                    .enumerate()
                                    .any(|(i, f)| i != b && f.map_or(false, |f| tuning[i] + f as i32 <= low))
                                {
                                    continue;
                                }
                                let opens = shape.iter().flatten().filter(|f| **f == 0).count() as i32;
                                let score = bonus * 3 + opens * 2 - mx.saturating_sub(mn) as i32 - mn as i32 / 3;
                                found.push((score, shape));
                            }
                        }
                    }
                }
            }
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    let pos = |x: &[Option<u8>; 6]| x.iter().flatten().filter(|f| **f > 0).min().copied().unwrap_or(0);
    let mut out: Vec<[Option<u8>; 6]> = Vec::new();
    for (_, s) in found {
        if !out.iter().any(|o| *o == s || pos(o).abs_diff(pos(&s)) < 2) {
            out.push(s);
        }
        if out.len() == 4 {
            break;
        }
    }
    out
}

/// The chord tones worth adding to a cluster over a bass, best first: the third, seventh and fifth of the first
/// mode that holds it (none when no mode does).
pub fn cluster_extras(bass: i32, pair: [i32; 2]) -> Vec<i32> {
    let rel = [(pair[0] - bass).rem_euclid(12), (pair[1] - bass).rem_euclid(12)];
    for (_, scale, _) in PARENT_MODES.iter() {
        for m in 0..7 {
            let mode = mode_steps(scale, m);
            if rel.iter().all(|r| mode.contains(r)) {
                return [mode[2], mode[6], mode[4]]
                    .into_iter()
                    .filter(|x| !rel.contains(x))
                    .map(|x| (bass + x).rem_euclid(12))
                    .collect();
            }
        }
    }
    Vec::new()
}

// ----------------------------------------------------------------- voicings: triads, shells, close and drop
// voicings, every inversion on a set of strings.

/// Seventh chords to voice: (suffix, notes from the root).
pub const VOICING_TYPES: [(&str, [i32; 4]); 9] = [
    ("maj7", [0, 4, 7, 11]),
    ("7", [0, 4, 7, 10]),
    ("m7", [0, 3, 7, 10]),
    ("m7b5", [0, 3, 6, 10]),
    ("dim7", [0, 3, 6, 9]),
    ("mMaj7", [0, 3, 7, 11]),
    ("6", [0, 4, 7, 9]),
    ("m6", [0, 3, 7, 9]),
    ("7sus4", [0, 5, 7, 10]),
];

/// Triads to voice: (suffix, notes from the root).
pub const TRIAD_TYPES: [(&str, [i32; 3]); 5] =
    [("", [0, 4, 7]), ("m", [0, 3, 7]), ("dim", [0, 3, 6]), ("aug", [0, 4, 8]), ("sus4", [0, 5, 7])];

/// The bones of a chord, from the root: root, third (or the suspended note), fifth and - when `n` is 4 - the
/// seventh (a sixth when it has one instead, else the root again an octave up). Extensions are left out.
pub fn chord_skeleton(c: &ChordNotes, n: usize) -> Vec<i32> {
    let rel: Vec<i32> = c.pcs.iter().map(|p| (p - c.root).rem_euclid(12)).collect();
    let first = |opts: &[i32]| opts.iter().copied().find(|x| rel.contains(x));
    let third = first(&[4, 3, 5, 2]).unwrap_or(4);
    let fifth = first(&[7, 6, 8]).unwrap_or(7);
    if n == 3 {
        return vec![0, third, fifth];
    }
    vec![0, third, fifth, first(&[11, 10, 9]).unwrap_or(12)]
}

/// A voicing as pitches from the root (low to high): the chord in close position, turned `inversion` times (the
/// lowest note moved up an octave each time), then the notes counted from the top in `drop` (2 = the second
/// highest) moved down an octave - drop 2, drop 3, drop 2 & 4.
pub fn voiced(chord: &[i32], inversion: usize, drop: &[usize]) -> Vec<i32> {
    let mut v: Vec<i32> = chord.to_vec();
    v.sort();
    for _ in 0..inversion {
        let low = v.remove(0);
        v.push(low + 12);
    }
    let n = v.len();
    for d in drop {
        if *d >= 1 && *d <= n {
            v[n - d] -= 12;
        }
    }
    v.sort();
    v
}

/// A voicing (pitches from a root, low to high) on a set of strings (low to high, one note each): the lowest spot
/// on the neck it fits with a reach of `reach` frets, preferring a movable shape (no open strings).
pub fn fit_on(pitches: &[i32], root: i32, strings: &[usize], tuning: &[i32; 6], reach: u8) -> Option<[Option<u8>; 6]> {
    if pitches.len() != strings.len() || pitches.is_empty() {
        return None;
    }
    let mut fits: Vec<[Option<u8>; 6]> = Vec::new();
    for f0 in 0..=14i32 {
        let low = tuning[strings[0]] + f0;
        if (low - root - pitches[0]).rem_euclid(12) != 0 {
            continue;
        }
        let mut shape = [None; 6];
        let mut ok = true;
        for (p, s) in pitches.iter().zip(strings) {
            let f = low + (p - pitches[0]) - tuning[*s];
            if !(0..=17).contains(&f) {
                ok = false;
                break;
            }
            shape[*s] = Some(f as u8);
        }
        let fretted: Vec<u8> = shape.iter().flatten().copied().filter(|f| *f > 0).collect();
        let span = fretted.iter().max().unwrap_or(&0) - fretted.iter().min().unwrap_or(&0);
        if ok && span <= reach {
            fits.push(shape);
        }
    }
    let movable = |s: &[Option<u8>; 6]| s.iter().flatten().all(|f| *f > 0);
    fits.iter().find(|s| movable(s)).or(fits.first()).copied()
}

// ----------------------------------------------------------------- passing chords: what can go in front of a chord
// to lead into it.

/// The ways into a chord `to` (a chord name): (kind, the chords to put before it, why it works - with this chord's
/// own notes).
pub fn passing_options(to: &str) -> Vec<(&'static str, Vec<String>, String)> {
    let Some(c) = chord_notes(to) else { return Vec::new() };
    let t = c.root;
    let rel: Vec<i32> = c.pcs.iter().map(|p| (p - t).rem_euclid(12)).collect();
    let minor = rel.contains(&3) && !rel.contains(&4);
    let major = rel.contains(&4) && !rel.contains(&10);
    // flats for most roots; sharps for the ones a half step below (F# into G, not Gb)
    let n = |x: i32| note_name(t + x, false);
    let ns = |x: i32| note_name(t + x, true);
    let quality = crate::chart::split_root(to).1.split('/').next().unwrap_or("").to_string();
    let target = n(0);
    let v7 = format!("{}7", n(7));
    let mut out = vec![
        (
            "V7",
            vec![v7.clone()],
            format!(
                "{} is the V7 of {}. Its 3rd, {}, sits a half step under {} and wants to rise to it - the strongest pull in tonal music.",
                v7,
                to,
                ns(11),
                target
            ),
        ),
        (
            "ii - V",
            vec![format!("{}{}", n(2), if minor { "m7b5" } else { "m7" }), v7.clone()],
            format!(
                "Put the ii in front of the V and you have a two-five into {}: {} then {}. Most standard tunes are chains of these.",
                to,
                format!("{}{}", n(2), if minor { "m7b5" } else { "m7" }),
                v7
            ),
        ),
        (
            "TRITONE SUB",
            vec![format!("{}7", n(1))],
            format!(
                "{}7 is a tritone away from {} and shares its 3rd and 7th ({} and {}, the other way round), so it does the same job - while the bass slides down a half step into {}.",
                n(1),
                v7,
                ns(11),
                n(5),
                target
            ),
        ),
        (
            "DIMINISHED",
            vec![format!("{}dim7", ns(11))],
            format!(
                "{}dim7 is {}b9 without its root ({} {} {} {}). The bass climbs a half step up into {}.",
                ns(11),
                v7,
                ns(11),
                n(2),
                n(5),
                n(8),
                target
            ),
        ),
        (
            "HALF STEP ABOVE",
            vec![format!("{}{}", n(1), quality)],
            format!(
                "The same chord a half step higher, slid down into {}: keep the shape and move every finger one fret. Pure chromatic voice leading.",
                to
            ),
        ),
        (
            "HALF STEP BELOW",
            vec![format!("{}{}", ns(11), quality)],
            format!("The same chord a half step lower, slid up into {}: same shape, one fret up.", to),
        ),
    ];
    if major {
        out.push((
            "BACKDOOR",
            vec![format!("{}m7", n(5)), format!("{}7", n(10))],
            format!(
                "{}m7 then {}7, borrowed from {} minor. The bVII7 rises a whole step home - a softer way in than {}.",
                n(5),
                n(10),
                target,
                v7
            ),
        ));
    }
    out
}

/// Scales that fit a chord: (name, notes from the root), the usual first choice first. Every scale offered holds
/// every note of the chord (the natural 5th aside, which players leave out of an altered chord), except the
/// pentatonics and the blues scale, which are offered where players use them.
pub fn scales_for(c: &ChordNotes) -> Vec<(&'static str, &'static [i32])> {
    const IONIAN: (&str, &[i32]) = ("Ionian (major)", &[0, 2, 4, 5, 7, 9, 11]);
    const LYDIAN: (&str, &[i32]) = ("Lydian", &[0, 2, 4, 6, 7, 9, 11]);
    const MIXO: (&str, &[i32]) = ("Mixolydian", &[0, 2, 4, 5, 7, 9, 10]);
    const LYD_DOM: (&str, &[i32]) = ("Lydian dominant", &[0, 2, 4, 6, 7, 9, 10]);
    const BEBOP: (&str, &[i32]) = ("Dominant bebop", &[0, 2, 4, 5, 7, 9, 10, 11]);
    const BLUES: (&str, &[i32]) = ("Blues", &[0, 3, 5, 6, 7, 10]);
    const ALTERED: (&str, &[i32]) = ("Altered", &[0, 1, 3, 4, 6, 8, 10]);
    const HW: (&str, &[i32]) = ("Half-whole diminished", &[0, 1, 3, 4, 6, 7, 9, 10]);
    const WH: (&str, &[i32]) = ("Whole-half diminished", &[0, 2, 3, 5, 6, 8, 9, 11]);
    const PHRYG_DOM: (&str, &[i32]) = ("Phrygian dominant", &[0, 1, 4, 5, 7, 8, 10]);
    const MIXO_B6: (&str, &[i32]) = ("Mixolydian b6", &[0, 2, 4, 5, 7, 8, 10]);
    const WHOLE: (&str, &[i32]) = ("Whole tone", &[0, 2, 4, 6, 8, 10]);
    const LYD_AUG: (&str, &[i32]) = ("Lydian augmented", &[0, 2, 4, 6, 8, 9, 11]);
    const LOCRIAN: (&str, &[i32]) = ("Locrian", &[0, 1, 3, 5, 6, 8, 10]);
    const LOCRIAN9: (&str, &[i32]) = ("Locrian nat 9", &[0, 2, 3, 5, 6, 8, 10]);
    const DORIAN: (&str, &[i32]) = ("Dorian", &[0, 2, 3, 5, 7, 9, 10]);
    const AEOLIAN: (&str, &[i32]) = ("Aeolian (natural minor)", &[0, 2, 3, 5, 7, 8, 10]);
    const PHRYGIAN: (&str, &[i32]) = ("Phrygian", &[0, 1, 3, 5, 7, 8, 10]);
    const MEL_MIN: (&str, &[i32]) = ("Melodic minor", &[0, 2, 3, 5, 7, 9, 11]);
    const HARM_MIN: (&str, &[i32]) = ("Harmonic minor", &[0, 2, 3, 5, 7, 8, 11]);
    const MIN_PENT: (&str, &[i32]) = ("Minor pentatonic", &[0, 3, 5, 7, 10]);
    const MAJ_PENT: (&str, &[i32]) = ("Major pentatonic", &[0, 2, 4, 7, 9]);
    let rel: Vec<i32> = c.pcs.iter().map(|p| (p - c.root).rem_euclid(12)).collect();
    let has = |x: i32| rel.contains(&x);
    let (maj3, min3) = (has(4), has(3) && !has(4));
    let (b7, maj7) = (has(10), has(11));
    let dom = maj3 && b7;
    let sus = !has(3) && !maj3 && has(5);
    let sharp9 = maj3 && has(3);
    let list: Vec<(&'static str, &'static [i32])> = if min3 && has(6) && has(9) && !b7 {
        vec![WH]
    } else if min3 && has(6) && b7 {
        // half-diminished: Locrian, or Locrian nat 9 when the 9 is written (Cm9b5)
        if has(2) {
            vec![LOCRIAN9, LOCRIAN]
        } else {
            vec![LOCRIAN, LOCRIAN9]
        }
    } else if dom {
        let b9 = has(1);
        let b13 = has(8) && has(7);
        let sharp5 = has(8) && !has(7);
        if sharp9 || (b9 && has(8)) || (sharp5 && (b9 || sharp9)) || (has(1) && has(3) && has(6) && has(8)) {
            // the altered sounds: 7alt, 7#9, 7b9b13, 7#9#5 ...
            vec![ALTERED, HW, PHRYG_DOM]
        } else if b9 {
            // 7b9 and 13b9 (a natural 13): half-whole diminished; Phrygian dominant over a 7b9 going to minor
            vec![HW, PHRYG_DOM, ALTERED]
        } else if sharp5 {
            vec![WHOLE, ALTERED]
        } else if b13 {
            vec![MIXO_B6, PHRYG_DOM, ALTERED]
        } else if has(6) {
            vec![LYD_DOM, WHOLE, HW]
        } else {
            vec![MIXO, LYD_DOM, BEBOP, BLUES]
        }
    } else if sus && b7 {
        // 7sus4, 9sus4, 13sus4, 11; Phrygian for a 7b9sus4
        vec![MIXO, BEBOP, PHRYGIAN]
    } else if maj3 && maj7 {
        if has(8) && !has(7) {
            vec![LYD_AUG]
        } else if has(6) {
            vec![LYDIAN, IONIAN]
        } else {
            vec![IONIAN, LYDIAN]
        }
    } else if min3 && maj7 {
        vec![MEL_MIN, HARM_MIN]
    } else if min3 && has(9) && !b7 {
        // m6, m69: the natural 6th - Dorian or melodic minor, never Aeolian
        vec![DORIAN, MEL_MIN]
    } else if min3 && b7 {
        vec![DORIAN, AEOLIAN, MIN_PENT, PHRYGIAN]
    } else if min3 && has(6) {
        // a diminished triad
        vec![LOCRIAN, WH]
    } else if min3 {
        vec![DORIAN, AEOLIAN, MIN_PENT, MEL_MIN, PHRYGIAN]
    } else if maj3 && has(8) && !has(7) {
        vec![WHOLE, LYD_AUG]
    } else if maj3 && (has(9) || has(2)) {
        // 6, 6/9, add9: the major scale and its pentatonic
        vec![IONIAN, MAJ_PENT, LYDIAN]
    } else if sus || has(2) && !maj3 && !has(3) {
        vec![MIXO, IONIAN]
    } else {
        vec![IONIAN, MAJ_PENT, LYDIAN, MIXO]
    };
    let fits = |steps: &[i32]| rel.iter().all(|r| *r == 7 || steps.contains(r));
    list.into_iter()
        .filter(|(name, steps)| matches!(*name, "Minor pentatonic" | "Major pentatonic" | "Blues") || fits(steps))
        .collect()
}

/// The note names of a scale on a root written `root` ("F#", "Bb"): each degree its own letter for the
/// seven-note scales (F# Dorian: F# G# A B C# D# E), the altered scale as its chord's notes (C Db D# E F# Ab Bb),
/// and the usual names for the others (C D E F# G# Bb, C Db Eb E F# G A Bb). `root_pc` is used when `root` is not
/// a note name.
pub fn scale_names(root: &str, root_pc: i32, steps: &[i32]) -> Vec<String> {
    let has = |x: i32| steps.contains(&x);
    let altered = steps == [0, 1, 3, 4, 6, 8, 10];
    steps
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let degree = if altered {
                // spelt as the altered chord's notes: 1 b9 #9 3 #11 b13 b7
                match s {
                    0 => 1,
                    1 => 2,
                    // (the #9 with the minor 3rd's letter: A Bb C C# D# F G)
                    3 => 3,
                    4 => 3,
                    6 => 4,
                    8 => 6,
                    _ => 7,
                }
            } else if steps.len() == 7 {
                i + 1
            } else {
                match s {
                    0 => 1,
                    1 | 2 => 2,
                    3 | 4 => 3,
                    5 => 4,
                    // #4 without a 4th (whole tone, half-whole), else b5 (blues, whole-half)
                    6 if !has(5) => 4,
                    6 | 7 => 5,
                    // #5 in the whole-tone scale, else b6
                    8 if has(4) && !has(7) => 5,
                    8 | 9 => 6,
                    _ => 7,
                }
            };
            let pc = (root_pc + s).rem_euclid(12);
            spell_degree(root, pc, degree).unwrap_or_else(|| note_name(pc, false).to_string())
        })
        .collect()
}

/// The root of a chord as it is written ("Bb" of Bbmaj7, "F#" of F#m7): for spelling its notes and scales.
pub fn written_root(chord: &str) -> String {
    written_note(chord.split('/').next().unwrap_or(""))
}

#[cfg(test)]
mod tests {
    /// The analyzer names chords the table doesn't list, from the notes: the top name for each set of notes.
    #[test]
    fn built_names() {
        let top = |notes: &[i32], bass: i32| super::suggestions(notes, Some(bass)).first().map(|s| s.key()).unwrap_or_default();
        // the altered dominants spell all their colour notes: Eb13b9 = Eb G Db E C (the 5th left to the bass player)
        let e = chord_notes("Eb13b9").unwrap();
        for pc in [3, 7, 1, 4, 0] {
            assert!(e.pcs.contains(&pc), "Eb13b9 lacks {}", pc);
        }
        assert!(!guitar_shapes(&e, &STANDARD).is_empty(), "no grip for Eb13b9");
        let c = chord_notes("C7#9b13").unwrap();
        for pc in [0, 4, 10, 3, 8] {
            assert!(c.pcs.contains(&pc), "C7#9b13 lacks {}", pc);
        }
        // the jazz grip, with the A string muted under the bass, is among the first grips offered
        let g = chord_notes("G13b9").unwrap();
        let grips = guitar_shapes(&g, &STANDARD);
        let want = [Some(3), None, Some(3), Some(4), Some(5), Some(4)];
        assert!(grips.iter().any(|s| *s == want), "3x3454 not offered: {:?}", &grips[..grips.len().min(3)]);
        // G B F E Ab over G (the 13b9 grip: 3 x 3 4 5 4)
        assert_eq!(top(&[7, 11, 5, 4, 8], 7), "G13b9");
        assert_eq!(top(&[0, 4, 10, 3, 8], 0), "C7#9#5");
        assert_eq!(top(&[0, 4, 7, 10, 3, 8], 0), "C7#9b13");
        assert_eq!(top(&[0, 4, 7, 11, 2, 6], 0), "Cmaj9#11");
        assert_eq!(top(&[0, 3, 6, 10, 2], 0), "Cm9b5");
        assert_eq!(top(&[0, 4, 10, 9, 6], 0), "C13#11");
        assert_eq!(top(&[0, 4, 10, 1, 8], 0), "C7b9#5");
        assert_eq!(top(&[0, 5, 10, 1], 0), "C7b9sus4");
        assert_eq!(top(&[0, 4, 11, 9], 0), "Cmaj13");
        assert_eq!(top(&[0, 3, 11, 2], 0), "CmMaj9");
        // the table's own still win: C7, Cm7b5, Cdim7, C6/9
        assert_eq!(top(&[0, 4, 7, 10], 0), "C7");
        assert_eq!(top(&[0, 3, 6, 10], 0), "Cm7b5");
        assert_eq!(top(&[0, 3, 6, 9], 0), "Cdim7");
        assert_eq!(top(&[0, 4, 7, 9, 2], 0), "C69");
    }

    use super::*;

    #[test]
    fn every_finder_chord_reads_right() {
        // each name in the funnel, on C: its notes as they should be
        let want: &[(&str, &[i32])] = &[
            ("", &[0, 4, 7]),
            ("6", &[0, 4, 7, 9]),
            ("69", &[0, 2, 4, 7, 9]),
            ("add9", &[0, 2, 4, 7]),
            ("maj7", &[0, 4, 7, 11]),
            ("maj9", &[0, 2, 4, 7, 11]),
            ("maj7#11", &[0, 4, 6, 7, 11]),
            ("maj13", &[0, 2, 4, 7, 9, 11]),
            ("m", &[0, 3, 7]),
            ("m6", &[0, 3, 7, 9]),
            ("madd9", &[0, 2, 3, 7]),
            ("m7", &[0, 3, 7, 10]),
            ("m9", &[0, 2, 3, 7, 10]),
            ("m11", &[0, 2, 3, 5, 7, 10]),
            ("m(maj7)", &[0, 3, 7, 11]),
            ("7", &[0, 4, 7, 10]),
            ("9", &[0, 2, 4, 7, 10]),
            ("13", &[0, 2, 4, 7, 9, 10]),
            ("7b9", &[0, 1, 4, 7, 10]),
            ("7#9", &[0, 3, 4, 7, 10]),
            ("7#11", &[0, 4, 6, 7, 10]),
            ("7b5", &[0, 4, 6, 10]),
            ("7#5", &[0, 4, 8, 10]),
            ("7b13", &[0, 4, 7, 8, 10]),
            ("13b9", &[0, 1, 4, 7, 9, 10]),
            ("13#11", &[0, 2, 4, 6, 7, 9, 10]),
            ("9#11", &[0, 2, 4, 6, 7, 10]),
            ("7b9b13", &[0, 1, 4, 7, 8, 10]),
            ("7#9b13", &[0, 3, 4, 7, 8, 10]),
            ("7#9#5", &[0, 3, 4, 8, 10]),
            ("7b5b9", &[0, 1, 4, 6, 10]),
            ("7b9#11", &[0, 1, 4, 6, 7, 10]),
            ("sus2", &[0, 2, 7]),
            ("sus4", &[0, 5, 7]),
            ("7sus4", &[0, 5, 7, 10]),
            ("9sus4", &[0, 2, 5, 7, 10]),
            ("dim", &[0, 3, 6]),
            ("dim7", &[0, 3, 6, 9]),
            ("m7b5", &[0, 3, 6, 10]),
            ("aug", &[0, 4, 8]),
            ("aug7", &[0, 4, 8, 10]),
            ("maj7#5", &[0, 4, 8, 11]),
        ];
        // every chord the funnel offers is checked here
        for (_, list) in FINDER_FAMILIES.iter() {
            for (_, suffix) in list.iter() {
                assert!(want.iter().any(|w| w.0 == *suffix), "no expected notes for C{}", suffix);
            }
        }
        let mut wrong = Vec::new();
        for (suffix, notes) in want {
            let name = format!("C{}", suffix);
            match chord_notes(&name) {
                Some(c) => {
                    let mut got: Vec<i32> = c.pcs.iter().map(|p| p.rem_euclid(12)).collect();
                    got.sort();
                    got.dedup();
                    if got != notes.to_vec() {
                        wrong.push(format!("{}: got {:?}, want {:?}", name, got, notes));
                    }
                }
                None => wrong.push(format!("{}: not read", name)),
            }
        }
        assert!(wrong.is_empty(), "{:#?}", wrong);
    }

    #[test]
    fn chords_have_the_right_notes() {
        let c = chord_notes("Dm7").unwrap();
        assert_eq!((c.root, c.bass), (2, 2));
        let names: Vec<&str> = c.named.iter().map(|n| n.0.as_str()).collect();
        assert_eq!(names, ["D", "F", "A", "C"]);
        let s = chord_notes("Fmaj7/A").unwrap();
        assert_eq!(s.bass, 9);
        assert!(chord_notes("hello").is_none());
    }

    #[test]
    fn notes_are_named() {
        assert_eq!(name_chords(&[2, 5, 9, 0], Some(2))[0], "Dm7");
        assert_eq!(name_chords(&[7, 11, 2, 5], Some(7))[0], "G7");
        // C E G with E in the bass
        assert_eq!(name_chords(&[0, 4, 7], Some(4))[0], "C/E");
        // G7 with the fifth left out still reads as G7
        assert!(name_chords(&[7, 11, 5], Some(7)).contains(&"G7".to_string()));
        // F A C E: Fmaj7 or Am/F; Fmaj7 first
        assert_eq!(name_chords(&[5, 9, 0, 4], Some(5))[0], "Fmaj7");
    }

    #[test]
    fn two_notes_suggest_many_chords() {
        // C and E: C first, and many more that contain them (Am, Cmaj7, A7...)
        let s = chords_containing(&[0, 4], Some(0));
        assert!(s.len() >= 15, "{}", s.len());
        assert_eq!(s[0].0, "C5".replace("5", "")); // plain C (the fifth left out)
        assert!(s.iter().any(|x| x.0 == "Am/C") && s.iter().any(|x| x.0 == "Cmaj7"));
        // a full Dm7 is exact
        let d = chords_containing(&[2, 5, 9, 0], Some(2));
        assert_eq!(d[0], ("Dm7".to_string(), true));
        assert!(chords_containing(&[0], None).is_empty());
    }

    #[test]
    fn spellings_and_warnings() {
        let s = suggestions(&[2, 5, 9, 0], Some(2));
        let d = &s[0];
        let plain = Spelling { slash: true, ..Spelling::default() };
        assert_eq!(d.name(plain), "Dm7");
        assert_eq!(d.name(Spelling { symbols: true, ..plain }), "D-7");
        assert!(d.warnings.is_empty());
        // C and G alone: no third, so C (and Cm) carry a warning
        let cg = suggestions(&[0, 7], Some(0));
        assert!(cg.iter().find(|x| x.key() == "C").unwrap().warnings.iter().any(|w| w.contains("third")));
        // detailed names say what is left out
        let c7 = suggestions(&[0, 4, 10], Some(0)).into_iter().find(|x| x.key() == "C7").unwrap();
        assert_eq!(c7.name(Spelling { detailed: true, ..plain }), "C7(no5)");
        // sharps
        assert_eq!(suggestions(&[1, 5, 8], Some(1))[0].name(Spelling { sharps: true, ..plain }), "C#");
    }

    #[test]
    fn guitar_shapes_are_playable() {
        for name in ["C", "G7", "Dm7", "Fmaj7", "Bb7#9", "Em7b5", "A7sus4", "C6"] {
            let c = chord_notes(name).unwrap();
            let shapes = guitar_shapes(&c, &STANDARD);
            assert!(!shapes.is_empty(), "no shape for {}", name);
            for s in &shapes {
                let low = s.iter().enumerate().find_map(|(i, f)| f.map(|f| (STANDARD[i] + f as i32).rem_euclid(12))).unwrap();
                assert_eq!(low, c.bass, "{} {:?}", name, s);
            }
        }
        // open C is found: x32010
        let c = chord_notes("C").unwrap();
        assert!(guitar_shapes(&c, &STANDARD).contains(&[None, Some(3), Some(2), Some(0), Some(1), Some(0)]));
    }

    #[test]
    fn two_note_clusters_follow_the_lessons() {
        // B and C over C: 7th + Root; held by C Ionian and C Lydian, among others
        assert_eq!((degree_word(11, 0), degree_word(0, 0)), ("7th", "Root"));
        let (parents, others) = modes_holding(0, &[11, 0]);
        assert_eq!(parents[0], ("Major", vec!["Ionian", "Lydian"]));
        assert_eq!(parents[1].1, vec!["Melodic Minor", "Lydian Augmented"]);
        assert!(others.contains(&"Augmented"));
        // over Db the same notes are b7th + 7th: no mode holds both
        let (p2, _) = modes_holding(1, &[11, 0]);
        assert!(p2.iter().all(|(_, m)| m.is_empty()));
        // B C over D: 6th + b7th - D Dorian and D Mixolydian
        assert_eq!(modes_holding(2, &[11, 0]).0[0].1, vec!["Dorian", "Mixolydian"]);
        // the major scale's semitones sit between its 3rd and 4th and its 7th and root
        assert_eq!(pairs_in(&PARENT_MODES[0].1, 1), vec![2, 6]);
        // grips: the bass lowest, both cluster notes there
        let g = cluster_voicings(0, [11, 0], &cluster_extras(0, [11, 0]), &STANDARD);
        assert!(!g.is_empty());
        for s in &g {
            let notes: Vec<i32> =
                s.iter().enumerate().filter_map(|(i, f)| f.map(|f| (STANDARD[i] + f as i32).rem_euclid(12))).collect();
            assert_eq!(notes[0], 0);
            assert!(notes.contains(&11) && notes.contains(&0));
        }
        // Night & Day's first chord, D half-diminished, comes from D Locrian (Eb major) and D Locrian Nat 9 (F melodic minor)
        let d = chord_notes("Dm7b5").unwrap();
        let src = chord_sources(&d);
        assert!(src.contains(&(0, 6, 3)) && src.contains(&(1, 5, 5)), "{:?}", src);
        // and D Locrian Nat 9 offers the semitone Eb-D? no: its semitones are C-Db? check it has some
        assert!(!mode_clusters(&PARENT_MODES[1].1, 5, 2, 1).is_empty());
    }

    #[test]
    fn grips_have_their_bass_lowest() {
        for name in ["Dm7", "Cmaj7", "G7", "Am7", "Bbmaj7", "F#m7b5"] {
            let c = chord_notes(name).unwrap();
            for s in guitar_shapes(&c, &STANDARD) {
                let played: Vec<(usize, u8)> = s.iter().enumerate().filter_map(|(i, f)| f.map(|f| (i, f))).collect();
                let low = played.iter().map(|(i, f)| STANDARD[*i] + *f as i32).min().unwrap();
                assert_eq!(low.rem_euclid(12), c.bass, "{} {:?}", name, s);
                let lo = played.iter().map(|p| p.1).filter(|f| *f > 0).min().unwrap_or(0);
                assert!(!(played.iter().any(|p| p.1 == 0) && lo > 4), "{} {:?}", name, s);
            }
        }
    }

    #[test]
    fn every_finder_chord_reads() {
        // each chord in the funnel is read as its kind (thirds and sevenths where they belong)
        for (fam, chords) in FINDER_FAMILIES {
            for (label, suffix) in chords {
                let c = chord_notes(&format!("C{}", suffix)).unwrap_or_else(|| panic!("{} {}", fam, label));
                let has = |x: i32| c.pcs.contains(&x);
                match fam {
                    "MAJOR" => assert!(has(4) && !has(3) && !has(10), "{}", label),
                    "MINOR" => assert!(has(3) && !has(4), "{}", label),
                    "DOMINANT" => assert!(has(4) && has(10), "{}", label),
                    "SUSPENDED" => assert!(!has(3) && !has(4), "{}", label),
                    "DIMINISHED" => assert!(has(3) && has(6), "{}", label),
                    _ => assert!(has(4) && has(8), "{}", label),
                }
            }
        }
    }

    #[test]
    fn drop_voicings_land_where_guitarists_play_them() {
        let cmaj7 = [0, 4, 7, 11];
        // drop 2 from close position: G C E B, all at the 5th fret on the top four strings (B on the 7th)
        assert_eq!(voiced(&cmaj7, 0, &[2]), vec![-5, 0, 4, 11]);
        assert_eq!(
            fit_on(&voiced(&cmaj7, 0, &[2]), 0, &[2, 3, 4, 5], &STANDARD, 4),
            Some([None, None, Some(5), Some(5), Some(5), Some(7)])
        );
        // drop 3 with the root in the bass: C on the 6th string's 8th fret, B E G above
        assert_eq!(voiced(&cmaj7, 3, &[3]), vec![0, 11, 16, 19]);
        assert_eq!(
            fit_on(&voiced(&cmaj7, 3, &[3]), 0, &[0, 2, 3, 4], &STANDARD, 4),
            Some([Some(8), None, Some(9), Some(9), Some(8), None])
        );
        // drop 2 & 4 takes two notes down
        assert_eq!(voiced(&cmaj7, 0, &[2, 4]), vec![-12, -5, 4, 11]);
        // a chord's bones: G7(b9) is still G B D F
        let g = chord_notes("G7b9").unwrap();
        assert_eq!(chord_skeleton(&g, 4), vec![0, 4, 7, 10]);
        assert_eq!(chord_skeleton(&chord_notes("C").unwrap(), 4), vec![0, 4, 7, 12]);
    }

    #[test]
    fn passing_chords_lead_into_the_target() {
        let o = passing_options("Cmaj7");
        let get = |k: &str| o.iter().find(|x| x.0 == k).map(|x| x.1.clone()).unwrap();
        assert_eq!(get("V7"), vec!["G7"]);
        assert_eq!(get("ii - V"), vec!["Dm7", "G7"]);
        assert_eq!(get("TRITONE SUB"), vec!["Db7"]);
        assert_eq!(get("DIMINISHED"), vec!["Bdim7"]);
        assert_eq!(get("HALF STEP ABOVE"), vec!["Dbmaj7"]);
        assert_eq!(get("BACKDOOR"), vec!["Fm7", "Bb7"]);
        // into a minor chord the two is half-diminished; no backdoor into a dominant
        let m = passing_options("Am7");
        assert_eq!(m.iter().find(|x| x.0 == "ii - V").unwrap().1, vec!["Bm7b5", "E7"]);
        assert!(!passing_options("G7").iter().any(|x| x.0 == "BACKDOOR"));
        assert_eq!(passing_options("G7").iter().find(|x| x.0 == "DIMINISHED").unwrap().1, vec!["F#dim7"]);
    }

    #[test]
    fn scales_fit() {
        assert_eq!(scales_for(&chord_notes("Dm7").unwrap())[0].0, "Dorian");
        assert_eq!(scales_for(&chord_notes("G7").unwrap())[0].0, "Mixolydian");
        assert_eq!(scales_for(&chord_notes("G7#9").unwrap())[0].0, "Altered");
        assert_eq!(scales_for(&chord_notes("Bm7b5").unwrap())[0].0, "Locrian");
    }
}

#[cfg(test)]
mod chord_spellings {
    use crate::theory::{chord_notes, guitar_shapes, place_grip, GRIPS, STANDARD};

    #[test]
    #[ignore]
    fn print_grips() {
        for name in
            ["A13b9", "Ab13b9", "E13b9", "B13b9", "Eb13b9", "C7", "Dm7", "G7#9", "Fmaj7", "Bbm7b5", "C6", "Cm6", "Caug", "Csus4"]
        {
            let c = chord_notes(name).unwrap();
            let s: Vec<String> = guitar_shapes(&c, &STANDARD)
                .iter()
                .map(|g| g.iter().map(|f| f.map_or("x".to_string(), |f| f.to_string())).collect::<Vec<_>>().join("-"))
                .collect();
            println!("{:8} {}", name, s.join("   "));
        }
    }

    #[test]
    fn library_grips_are_right_in_every_key() {
        for (name, degrees) in GRIPS {
            let want: Vec<i32> = degrees.iter().filter(|d| **d >= 0).map(|d| *d as i32).collect();
            let rs = degrees.iter().position(|d| *d == 0).unwrap();
            for root in 0..12i32 {
                let f0 = (root - STANDARD[rs]).rem_euclid(12);
                for f in [f0, f0 + 12] {
                    if f > 15 {
                        continue;
                    }
                    let Some(g) = place_grip(degrees, root, f, &STANDARD) else { continue };
                    // every string sounds the degree written for it
                    for (s, d) in degrees.iter().enumerate() {
                        match (*d, g[s]) {
                            (-1, None) => {}
                            (d, Some(fr)) if d >= 0 => {
                                assert_eq!(
                                    (STANDARD[s] + fr as i32 - root).rem_euclid(12),
                                    d as i32,
                                    "{} root {} string {}",
                                    name,
                                    root,
                                    s
                                )
                            }
                            _ => panic!("{} root {}: string {} wrong", name, root, s),
                        }
                    }
                    // the root is the lowest note
                    let low = g.iter().enumerate().filter_map(|(s, f)| f.map(|f| STANDARD[s] + f as i32)).min().unwrap();
                    assert_eq!(low.rem_euclid(12), root, "{} root {}: bass", name, root);
                    let _ = &want;
                }
            }
        }
    }

    #[test]
    fn the_grips_players_know_are_offered() {
        let has = |name: &str, g: [Option<u8>; 6]| {
            let c = chord_notes(name).unwrap();
            let shapes = guitar_shapes(&c, &STANDARD);
            assert!(shapes.contains(&g), "{}: {:?} not in {:?}", name, g, shapes);
        };
        let n = None;
        // the 13b9 with the root on the 6th string: R x b7 3 13 b9
        has("A13b9", [Some(5), n, Some(5), Some(6), Some(7), Some(6)]);
        has("Eb13b9", [Some(11), n, Some(11), Some(12), Some(13), Some(12)]);
        has("G13b9", [Some(3), n, Some(3), Some(4), Some(5), Some(4)]);
        // the Hendrix chord
        has("E7#9", [n, Some(7), Some(6), Some(7), Some(8), n]);
        has("C7#9", [n, Some(3), Some(2), Some(3), Some(4), n]);
        // shells and drop 3s
        has("Cmaj7", [Some(8), n, Some(9), Some(9), Some(8), n]);
        has("Dm7", [n, Some(5), Some(7), Some(5), Some(6), n]);
        has("Bm7b5", [n, Some(2), Some(3), Some(2), Some(3), n]);
        has("G13", [Some(3), n, Some(3), Some(4), Some(5), n]);
        // the altered dominant as 3 b7 #9 b13 with the root on the A string; maj7#11 and 6/9 as played
        has("C7alt", [n, Some(3), Some(2), Some(3), Some(4), Some(4)]);
        has("Cmaj7#11", [n, Some(3), Some(4), Some(4), Some(5), n]);
        has("G69", [Some(3), Some(2), Some(2), Some(2), Some(3), Some(3)]);
    }

    #[test]
    fn every_offered_grip_spells_its_chord() {
        for root in ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"] {
            for kind in [
                "", "m", "7", "maj7", "m7", "m7b5", "dim7", "6", "m6", "9", "m9", "maj9", "13", "7b9", "7#9", "13b9", "7#5",
                "7#11", "7sus4", "7alt", "maj7#11", "69", "11", "7b13", "9sus4", "m11", "7b9b13", "7#9b13",
            ] {
                let name = format!("{}{}", root, kind);
                let Some(c) = chord_notes(&name) else { continue };
                for g in guitar_shapes(&c, &STANDARD) {
                    let notes: Vec<i32> = g.iter().enumerate().filter_map(|(s, f)| f.map(|f| STANDARD[s] + f as i32)).collect();
                    for m in &notes {
                        assert!(c.pcs.contains(&m.rem_euclid(12)), "{}: {:?} plays a note not in the chord", name, g);
                    }
                    let low = notes.iter().min().unwrap();
                    assert_eq!(low.rem_euclid(12), c.bass, "{}: {:?} has the wrong bass", name, g);
                    // the 3rd and the 7th, the notes that say what a chord is, are there
                    for d in [3, 4, 10, 11] {
                        let pc = (c.root + d).rem_euclid(12);
                        // (over a major 3rd a minor one is the #9, a tension)
                        if c.pcs.contains(&pc) && !(d == 3 && c.pcs.contains(&((c.root + 4).rem_euclid(12)))) {
                            assert!(notes.iter().any(|m| m.rem_euclid(12) == pc), "{}: {:?} leaves out a guide tone", name, g);
                        }
                    }
                    // nothing a half step or whole step over the bass, down low (a mud no one voices)
                    let mut s = notes.clone();
                    s.sort();
                    if s.len() > 1 {
                        assert!(s[1] - s[0] >= 3, "{}: {:?} crowds the bass", name, g);
                    }
                }
            }
        }
    }

    #[test]
    fn added_notes_and_suspensions() {
        // an add9 has no seventh; sus2 has the 2nd; every chord of a real chart reads
        assert_eq!(super::chord_notes("Gadd9/A").unwrap().pcs, vec![7, 11, 2, 9]);
        assert_eq!(super::chord_notes("Dsus2").unwrap().pcs, vec![2, 4, 9]);
        assert_eq!(super::chord_notes("CmMaj7").unwrap().pcs, vec![0, 3, 7, 11]);
        for c in "Em11 Fm7/Bb Em7/A Fadd9/G Asus4 A7#5 Bb/C Dm9 F/Bb Dm/A C7sus4 G6/A".split(' ') {
            assert!(super::chord_notes(c).is_some(), "{}", c);
        }
    }
}

#[cfg(test)]
mod chord_audit {

    /// The whole dictionary as data (for the printed ruleset): every finder chord in C, its grips by section and its
    /// voicing groups, each string's fret and degree. Run with --ignored, writes $BACKLINE_DICT.
    #[test]
    #[ignore]
    fn dump_dictionary() {
        use super::*;
        let path = std::env::var("BACKLINE_DICT").unwrap_or_else(|_| "dictionary.json".into());
        let grip = |c: &ChordNotes, g: &[Option<u8>; 6]| {
            let strings: Vec<serde_json::Value> = g
                .iter()
                .enumerate()
                .map(|(s, f)| match f {
                    Some(f) => {
                        let m = STANDARD[s] + *f as i32;
                        serde_json::json!({"fret": f, "degree": crate::fretboard::degree_in(m.rem_euclid(12), c.root, &c.pcs).0})
                    }
                    None => serde_json::Value::Null,
                })
                .collect();
            serde_json::json!(strings)
        };
        let mut fams = Vec::new();
        for (fam, chords) in FINDER_FAMILIES.iter() {
            let mut list = Vec::new();
            for (label, suffix) in chords.iter() {
                let name = format!("C{}", suffix);
                let c = chord_notes(&name).unwrap();
                let s = grip_sections(&c, &STANDARD);
                let groups: Vec<serde_json::Value> = voicing_groups(&c, &STANDARD)
                    .iter()
                    .map(|(g, l)| serde_json::json!({"group": g, "voicings": l.iter().map(|v| grip(&c, &v.frets)).collect::<Vec<_>>()}))
                    .collect();
                list.push(serde_json::json!({
                    "label": label, "name": name,
                    "notes": c.named.iter().map(|(n, i)| format!("{} ({})", n, i)).collect::<Vec<_>>(),
                    "must": s.must.iter().map(|g| grip(&c, g)).collect::<Vec<_>>(),
                    "open": s.open.iter().map(|g| grip(&c, g)).collect::<Vec<_>>(),
                    "moveable": s.moveable.iter().map(|g| grip(&c, g)).collect::<Vec<_>>(),
                    "groups": groups,
                }));
            }
            fams.push(serde_json::json!({"family": fam, "chords": list}));
        }
        std::fs::write(&path, serde_json::to_string_pretty(&fams).unwrap()).unwrap();
    }

    /// The grips to know first are the chord books' first ones (and Oolimo's MUST KNOW).
    #[test]
    fn must_know_grips() {
        use super::*;
        let must = |name: &str| grip_sections(&chord_notes(name).unwrap(), &STANDARD).must;
        let n = None;
        assert!(must("A9sus4").contains(&[Some(5), n, Some(5), Some(4), Some(3), n]), "{:?}", must("A9sus4"));
        assert!(must("A13b9").contains(&[Some(5), n, Some(5), Some(6), Some(7), Some(6)]), "{:?}", must("A13b9"));
        assert!(must("Cmaj7").contains(&[Some(8), n, Some(9), Some(9), Some(8), n]), "{:?}", must("Cmaj7"));
        assert!(must("Dm7").contains(&[n, Some(5), Some(7), Some(5), Some(6), n]), "{:?}", must("Dm7"));
        assert!(must("G7").contains(&[Some(3), n, Some(3), Some(4), Some(3), n]), "{:?}", must("G7"));
        // every finder chord has something to know first, in every key
        for (_, chords) in FINDER_FAMILIES.iter() {
            for (_, suffix) in chords.iter() {
                for root in ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"] {
                    let name = format!("{}{}", root, suffix);
                    assert!(!must(&name).is_empty(), "{}: nothing to know first", name);
                }
            }
        }
    }

    /// Every voicing in the groups is right by its rule: four notes of the chord, the root's pitch class respected,
    /// the spacing its group's (drop 2: the 2nd voice from the top an octave down), on adjacent string sets, playable.
    #[test]
    fn voicing_groups_are_right() {
        use super::*;
        for root in ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"] {
            for kind in ["maj7", "7", "m7", "m7b5", "dim7", "6", "m6", "9", "13", "7b9", "7#9", "13b9", "7b9b13", "m11", "maj9"] {
                let name = format!("{}{}", root, kind);
                let c = chord_notes(&name).unwrap();
                let groups = voicing_groups(&c, &STANDARD);
                let drop2 = &groups.iter().find(|g| g.0 == "Drop 2").unwrap().1;
                assert!(drop2.len() >= 4, "{}: only {} drop 2 voicings", name, drop2.len());
                for (_, list) in &groups {
                    for v in list {
                        let notes: Vec<i32> =
                            v.frets.iter().enumerate().filter_map(|(s, f)| f.map(|f| STANDARD[s] + f as i32)).collect();
                        assert_eq!(notes.len(), 4, "{} {:?}", name, v);
                        for m in &notes {
                            assert!(c.pcs.contains(&m.rem_euclid(12)), "{} {:?} plays a note not in it", name, v);
                        }
                        // notes rise string by string, and the 3rd and the 7th are in it
                        assert!(notes.windows(2).all(|w| w[1] > w[0]), "{} {:?} crosses", name, v);
                        for d in [3, 4, 10, 11] {
                            let pc = (c.root + d).rem_euclid(12);
                            if c.pcs.contains(&pc) && ![3, 4].contains(&d)
                                || c.pcs.contains(&pc) && !c.pcs.contains(&((c.root + 7 - d).rem_euclid(12)))
                            {
                            }
                        }
                        let offs: [Option<i32>; 6] = v.frets.map(|f| f.map(|f| f as i32));
                        assert!(grip_fingers(&offs) <= 4, "{} {:?} fingers", name, v);
                        // drop 2: from the top, voices 1, 3, 4 lie within an octave and voice 2 sits an octave lower
                        if v.group == "Drop 2" {
                            let mut close = notes.clone();
                            close[0] += 12;
                            close.sort();
                            assert!(close[3] - close[0] < 12, "{} {:?} isn't a drop 2", name, v);
                        }
                    }
                }
            }
        }
        // the textbook ones: Cmaj7 drop 2 on the top four, root position (x x 10 12 12 12)
        let c = chord_notes("Cmaj7").unwrap();
        let g = voicing_groups(&c, &STANDARD);
        let d2 = &g.iter().find(|x| x.0 == "Drop 2").unwrap().1;
        assert!(d2.iter().any(|v| v.frets == [None, None, Some(10), Some(12), Some(12), Some(12)]), "{:?}", d2);
        // G7 drop 3, root on the 6th: 3 x 3 4 3 x
        let c = chord_notes("G7").unwrap();
        let g = voicing_groups(&c, &STANDARD);
        let d3 = &g.iter().find(|x| x.0 == "Drop 3").unwrap().1;
        assert!(d3.iter().any(|v| v.frets == [Some(3), None, Some(3), Some(4), Some(3), None]), "{:?}", d3);
    }

    /// Every grip in the library is a movable shape that sits in four frets and four fingers (a barre counts as one
    /// finger), and it shows for every root, the same shape in every key.
    #[test]
    fn every_grip_is_movable_and_playable() {
        use super::*;
        for (name, degrees) in GRIPS.iter() {
            let offs =
                grip_offsets(degrees, &STANDARD).unwrap_or_else(|| panic!("{} {:?} won't sit in four frets", name, degrees));
            // (no thumb over the neck: every grip is four fingers, as the chord books and Oolimo give them)
            assert!(grip_fingers(&offs) <= 4, "{} {:?} needs {} fingers", name, degrees, grip_fingers(&offs));
            let rs = degrees.iter().position(|d| *d == 0).unwrap();
            for root in 0..12i32 {
                let f0 = (root - STANDARD[rs]).rem_euclid(12);
                let g = [f0, f0 + 12].into_iter().find_map(|f| place_grip(degrees, root, f, &STANDARD));
                let g = g.unwrap_or_else(|| panic!("{} {:?} missing with root {}", name, degrees, root));
                // the same shape: each string the same distance from the root's fret
                let rf = g[rs].unwrap() as i32;
                for s in 0..6 {
                    assert_eq!(g[s].map(|f| f as i32 - rf), offs[s], "{} root {} string {}", name, root, s);
                }
            }
        }
    }

    /// Every chord the finder offers, in every key, has library grips with the root on the 6th string and on the
    /// 5th (the shapes the chord books give), each one spelling the chord.
    #[test]
    fn every_finder_chord_has_book_grips() {
        use super::*;
        let mut short = Vec::new();
        for (_, chords) in FINDER_FAMILIES.iter() {
            for (_, suffix) in chords.iter() {
                for root in ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"] {
                    let name = format!("{}{}", root, suffix);
                    let c = chord_notes(&name).unwrap_or_else(|| panic!("{} can't be read", name));
                    let grips = library_grips(&c, &STANDARD);
                    let on = |s: usize| grips.iter().any(|g| g.iter().position(|f| f.is_some()) == Some(s));
                    if !(on(0) || on(1)) {
                        short.push(format!("{} (6th: {}, 5th: {})", name, on(0), on(1)));
                    }
                }
            }
        }
        assert!(short.is_empty(), "no grip with the root on the 6th or 5th string: {:?}", short);
    }

    /// Each grip offered, played into the analyzer, is named as the chord it was offered for (its root at least, and
    /// the same notes).
    #[test]
    fn grips_round_trip_through_the_analyzer() {
        use super::*;
        let mut bad = Vec::new();
        for root in ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"] {
            for kind in [
                "", "m", "7", "maj7", "m7", "m7b5", "dim7", "6", "m6", "69", "9", "m9", "maj9", "m11", "13", "7b9", "7#9",
                "13b9", "7#5", "7#11", "maj7#11", "7alt", "7sus4", "9sus4", "m(maj7)", "sus4", "aug",
            ] {
                let name = format!("{}{}", root, kind);
                let Some(c) = chord_notes(&name) else {
                    bad.push(format!("{}: can't read", name));
                    continue;
                };
                for g in guitar_shapes(&c, &STANDARD) {
                    let notes: Vec<i32> = g.iter().enumerate().filter_map(|(s, f)| f.map(|f| STANDARD[s] + f as i32)).collect();
                    let low = notes.iter().min().map(|m| m.rem_euclid(12));
                    let s = suggestions(&notes, low);
                    match s.first() {
                        Some(top) if top.root == c.root => {}
                        Some(top) => bad.push(format!("{} {:?}: analyzer says {}", name, g, top.key())),
                        None => bad.push(format!("{} {:?}: no name", name, g)),
                    }
                }
            }
        }
        for b in &bad {
            println!("{}", b);
        }
        assert!(bad.is_empty(), "{} grips named wrong", bad.len());
    }
    use super::*;

    /// The interval names of a chord, in order.
    fn ivs(name: &str) -> String {
        let c = chord_notes(name).unwrap_or_else(|| panic!("{} not read", name));
        c.named.iter().map(|n| n.1.as_str()).collect::<Vec<_>>().join(" ")
    }

    fn notes(name: &str) -> String {
        chord_notes(name).unwrap().named.iter().map(|n| n.0.as_str()).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn every_common_spelling_reads_right() {
        let want: &[(&str, &[&str])] = &[
            ("1 3 5", &["C", "CM", "Cmaj", "CMaj"]),
            ("1 b3 5", &["Cm", "C-", "Cmi", "Cmin", "CMI"]),
            ("1 3 5 7", &["Cmaj7", "C^7", "C^", "C\u{0394}7", "C\u{0394}", "CM7", "Cma7", "CMA7", "CMaj7", "CMAJ7"]),
            ("1 b3 5 b7", &["Cm7", "C-7", "Cmi7", "Cmin7", "CMI7", "CMIN7"]),
            ("1 b3 b5 b7", &["Cm7b5", "C\u{00f8}", "C\u{00f8}7", "Ch7", "Ch", "C-7b5", "Cmin7(b5)", "Cmi7b5", "Cm7-5"]),
            ("1 b3 b5 bb7", &["Cdim7", "Co7", "C\u{00b0}7"]),
            ("1 b3 b5", &["Cdim", "Co", "C\u{00b0}"]),
            ("1 3 #5", &["Caug", "C+", "C+5", "C(#5)"]),
            ("1 3 #5 b7", &["C7#5", "C7+", "C+7", "Caug7", "C7+5", "C7(#5)", "C7aug"]),
            ("1 3 b5 b7", &["C7b5", "C7(b5)", "C7-5"]),
            ("1 3 5 b7 b9", &["C7b9", "C7(b9)", "C7-9"]),
            ("1 3 5 b7 #9", &["C7#9", "C7(#9)", "C7+9"]),
            ("1 3 5 b7 #11", &["C7#11", "C7(#11)", "C7+11"]),
            ("1 3 5 b7 b13", &["C7b13", "C7(b13)"]),
            ("1 3 b7 b9 #9 #11 b13", &["C7alt", "Calt"]),
            ("1 3 5 b7 9", &["C9"]),
            ("1 11 5 b7 9", &["C11"]),
            ("1 3 5 b7 9 13", &["C13"]),
            ("1 3 5 b7 b9 13", &["C13b9", "C13(b9)"]),
            ("1 3 5 b7 9 #11 13", &["C13#11"]),
            ("1 3 5 b7 #9 13", &["C13#9"]),
            ("1 b3 5 b7 9", &["Cm9", "C-9", "Cmi9"]),
            ("1 b3 5 b7 9 11", &["Cm11", "C-11"]),
            ("1 b3 5 b7 9 13", &["Cm13", "C-13"]),
            ("1 3 5 7 9", &["Cmaj9", "C^9", "CM9", "C\u{0394}9"]),
            ("1 3 5 7 #11", &["Cmaj7#11", "C^7#11", "CM7#11", "Cmaj7(#11)"]),
            ("1 3 5 7 9 #11", &["Cmaj9#11"]),
            ("1 3 5 7 9 13", &["Cmaj13", "C^13"]),
            ("1 3 #5 7", &["Cmaj7#5", "C+maj7", "Cmaj7+5", "C^7#5", "Caugmaj7"]),
            ("1 3 5 6", &["C6", "Cmaj6"]),
            ("1 b3 5 6", &["Cm6", "C-6", "Cmi6"]),
            ("1 3 5 6 9", &["C69", "C6/9", "C6add9"]),
            ("1 b3 5 6 9", &["Cm69", "Cm6/9", "C-69"]),
            ("1 4 5", &["Csus", "Csus4"]),
            ("1 2 5", &["Csus2"]),
            ("1 4 5 b7", &["C7sus4", "C7sus"]),
            ("1 4 5 b7 9", &["C9sus4", "C9sus"]),
            ("1 4 5 b7 9 13", &["C13sus4", "C13sus"]),
            ("1 4 5 b7 b9", &["C7sus4b9", "C7susb9", "C7b9sus4"]),
            ("1 3 5 9", &["Cadd9", "C(add9)"]),
            ("1 b3 5 9", &["Cmadd9", "Cm(add9)", "C-add9"]),
            (
                "1 b3 5 7",
                &["Cm(maj7)", "CmMaj7", "C-^7", "CmM7", "Cm^7", "Cmi(ma7)", "Cm/maj7", "CminMaj7", "C-maj7", "C-\u{0394}7"],
            ),
            ("1 b3 5 7 9", &["C-^9", "CmMaj9", "Cm(maj9)"]),
            ("1 b3 b5 b7 9", &["Cm9b5", "C\u{00f8}9", "Ch9"]),
            ("1 3 5 b7 b9 b13", &["C7b9b13", "C7(b9,b13)"]),
            ("1 3 5 b7 #9 b13", &["C7#9b13"]),
            ("1 3 #5 b7 #9", &["C7#9#5", "C7#5#9"]),
            ("1 3 5 b7 b9 #11", &["C7b9#11"]),
            ("1 3 b5 b7 b9", &["C7b5b9", "C7b9b5"]),
            ("1 3 5 b7 9 #11", &["C9#11"]),
            ("1 5", &["C5"]),
        ];
        let mut wrong = Vec::new();
        for (iv, names) in want {
            for n in *names {
                match chord_notes(n) {
                    Some(_) if ivs(n) == *iv => {}
                    Some(_) => wrong.push(format!("{}: {} (want {})", n, ivs(n), iv)),
                    None => wrong.push(format!("{}: not read", n)),
                }
            }
        }
        assert!(wrong.is_empty(), "{:#?}", wrong);
        // the notes are the intervals they are named for
        let semis = |iv: &str| match iv {
            "1" => 0,
            "b9" => 1,
            "2" | "9" => 2,
            "b3" | "#9" => 3,
            "3" => 4,
            "4" | "11" => 5,
            "b5" | "#11" => 6,
            "5" => 7,
            "#5" | "b13" => 8,
            "6" | "13" | "bb7" => 9,
            "b7" => 10,
            "7" => 11,
            x => panic!("{}", x),
        };
        for (iv, names) in want {
            let c = chord_notes(names[0]).unwrap();
            let mut got = c.pcs.clone();
            got.sort();
            let mut w: Vec<i32> = iv.split(' ').map(semis).collect();
            w.sort();
            assert_eq!(got, w, "{}", names[0]);
        }
    }

    #[test]
    fn roots_and_slashes() {
        for (n, root, bass) in [
            ("Gb7", 6, 6),
            ("F#7", 6, 6),
            ("Cb", 11, 11),
            ("E#m", 5, 5),
            ("B#7", 0, 0),
            ("Fb", 4, 4),
            ("C/E", 0, 4),
            ("D7/F#", 2, 6),
            ("Fmaj7/A", 5, 9),
            ("C7/Bb", 0, 10),
            ("C/Bb", 0, 10),
            ("B\u{266d}7", 10, 10),
            ("F\u{266f}m7", 6, 6),
        ] {
            let c = chord_notes(n).unwrap_or_else(|| panic!("{}", n));
            assert_eq!((c.root, c.bass), (root, bass), "{}", n);
        }
        // a bass from outside the chord comes first, named as written
        let c = chord_notes("C/Bb").unwrap();
        assert_eq!(c.named[0], ("Bb".to_string(), "bass".to_string()));
        assert_eq!(notes("D7/F#"), "D F# A C");
    }

    #[test]
    fn notes_are_spelt_from_the_root() {
        assert_eq!(notes("Ab13b9"), "Ab C Eb Gb A F");
        assert_eq!(notes("Ab7"), "Ab C Eb Gb");
        assert_eq!(notes("Db7"), "Db F Ab Cb");
        assert_eq!(notes("F#7"), "F# A# C# E");
        assert_eq!(notes("Bb7#11"), "Bb D F Ab E");
        // (the #9 with the minor 3rd's letter, as the chord books write it)
        assert_eq!(notes("Eb7#9"), "Eb G Bb Db Gb");
        assert_eq!(notes("E7#9"), "E G# B D G");
        assert_eq!(notes("Cdim7"), "C Eb Gb A");
        assert_eq!(notes("Bm7b5"), "B D F A");
        assert_eq!(notes("C#m7"), "C# E G# B");
        assert_eq!(notes("Ebmaj7#11"), "Eb G Bb D A");
        assert_eq!(notes("G7alt"), "G B F Ab Bb C# Eb");
        assert_eq!(notes("Gbmaj7"), "Gb Bb Db F");
    }

    #[test]
    fn the_analyzer_names_known_grips() {
        // (strings low to high, frets; None not played) -> the first name
        let name_of = |g: [Option<u8>; 6]| {
            let ms: Vec<i32> = g.iter().enumerate().filter_map(|(s, f)| f.map(|f| STANDARD[s] + f as i32)).collect();
            let bass = *ms.iter().min().unwrap();
            suggestions(&ms, Some(bass)).first().map(|s| s.key()).unwrap_or_default()
        };
        let n = None;
        let mut wrong = Vec::new();
        for (g, want) in [
            ([n, Some(3), Some(2), Some(0), Some(1), Some(0)], "C"),
            ([n, Some(0), Some(2), Some(2), Some(1), Some(0)], "Am"),
            ([Some(3), Some(2), Some(0), Some(0), Some(0), Some(3)], "G"),
            ([n, n, Some(0), Some(2), Some(3), Some(2)], "D"),
            ([Some(0), Some(2), Some(2), Some(1), Some(0), Some(0)], "E"),
            ([Some(3), n, Some(3), Some(4), Some(3), n], "G7"),
            ([n, Some(3), Some(2), Some(3), Some(1), n], "C7"),
            ([n, Some(5), Some(7), Some(5), Some(6), n], "Dm7"),
            ([Some(8), n, Some(9), Some(9), Some(8), n], "Cmaj7"),
            ([n, Some(2), Some(3), Some(2), Some(3), n], "Bm7b5"),
            ([n, Some(7), Some(6), Some(7), Some(8), n], "E7#9"),
            ([n, Some(3), Some(2), Some(2), Some(3), Some(3)], "C69"),
            ([n, Some(0), Some(2), Some(0), Some(2), Some(0)], "A7"),
            ([n, n, Some(0), Some(2), Some(1), Some(2)], "D7"),
            ([Some(0), Some(2), Some(0), Some(0), Some(0), Some(0)], "Em7"),
            ([n, Some(3), Some(5), Some(3), Some(4), Some(3)], "Cm7"),
            ([n, Some(3), Some(4), Some(2), Some(4), n], "Cdim7"),
            ([n, n, Some(0), Some(2), Some(3), Some(3)], "Dsus4"),
            ([n, Some(0), Some(2), Some(2), Some(0), Some(0)], "Asus2"),
            ([n, Some(3), Some(2), Some(0), Some(0), Some(0)], "Cmaj7"),
            ([Some(0), Some(3), Some(2), Some(0), Some(1), Some(0)], "C/E"),
        ] {
            if name_of(g) != want {
                wrong.push(format!("{:?}: {} (want {})", g, name_of(g), want));
            }
        }
        assert!(wrong.is_empty(), "{:#?}", wrong);
        // an inversion reads as the chord over its bass, not as an odd chord on the bass note
        let top = |n: &[i32], b: i32| suggestions(n, Some(b)).first().map(|s| s.key()).unwrap_or_default();
        assert_eq!(top(&[7, 0, 4], 7), "C/G");
        assert_eq!(top(&[4, 7, 0], 4), "C/E");
        // rootless voicings: E Bb D G is Em7b5 (a C9 without its root), F C E A is Fmaj7 (a rootless Dm9)
        assert_eq!(top(&[4, 10, 2, 7], 4), "Em7b5");
        assert_eq!(top(&[5, 0, 4, 9], 5), "Fmaj7");
        // the G13 without its root (B F A E) offers G13 and never a "7b5sus4"
        let g = suggestions(&[11, 5, 9, 4], Some(11));
        assert!(g.iter().take(4).any(|s| s.key() == "G13/B"), "{:?}", g.iter().take(4).map(|s| s.key()).collect::<Vec<_>>());
        assert!(!g.iter().any(|s| s.key().contains("b5sus")));
        // Bb/C is C11 (C Bb D F, a 9sus4), and D/C is D7/C
        assert_eq!(top(&[0, 10, 2, 5], 0), "C11");
        assert_eq!(top(&[0, 2, 6, 9], 0), "D7/C");
    }

    #[test]
    fn the_scales_offered_fit() {
        let first = |n: &str| scales_for(&chord_notes(n).unwrap()).first().map(|s| s.0).unwrap_or("");
        let all = |n: &str| scales_for(&chord_notes(n).unwrap()).iter().map(|s| s.0).collect::<Vec<_>>();
        assert_eq!(first("C7b9"), "Half-whole diminished");
        assert!(all("C7b9").contains(&"Phrygian dominant"));
        assert_eq!(first("C13b9"), "Half-whole diminished");
        assert_eq!(first("C7alt"), "Altered");
        assert_eq!(first("C7#9b13"), "Altered");
        assert_eq!(first("C7b9b13"), "Altered");
        assert!(all("C7b9b13").contains(&"Phrygian dominant"));
        assert_eq!(first("Cm7b5"), "Locrian");
        assert!(all("Cm7b5").contains(&"Locrian nat 9"));
        assert_eq!(first("Cm9b5"), "Locrian nat 9");
        assert_eq!(first("Cmaj7#11"), "Lydian");
        assert_eq!(first("C7#11"), "Lydian dominant");
        assert_eq!(first("C13#11"), "Lydian dominant");
        assert_eq!(first("C7b13"), "Mixolydian b6");
        assert_eq!(first("C7#5"), "Whole tone");
        assert_eq!(first("C7sus4"), "Mixolydian");
        assert_eq!(first("C7sus4b9"), "Phrygian");
        assert_eq!(first("Cm6"), "Dorian");
        assert!(!all("Cm6").contains(&"Aeolian (natural minor)"));
        assert_eq!(first("CmMaj7"), "Melodic minor");
        assert_eq!(first("Cdim7"), "Whole-half diminished");
        assert_eq!(first("Caug"), "Whole tone");
        assert_eq!(first("Cmaj7#5"), "Lydian augmented");
        assert!(!all("C6").contains(&"Mixolydian"));
        assert!(!all("Csus4").contains(&"Lydian"));
        // every scale offered holds the chord's notes (the 5th aside; the pentatonics and blues are extras)
        for kind in [
            "", "m", "7", "maj7", "m7", "m7b5", "dim7", "6", "m6", "69", "9", "m9", "maj9", "11", "m11", "13", "m13", "7b9",
            "7#9", "13b9", "7#5", "7b5", "7#11", "7b13", "7alt", "7sus4", "9sus4", "maj7#11", "mMaj7", "aug", "dim", "sus4",
            "sus2", "add9", "maj7#5",
        ] {
            let c = chord_notes(&format!("C{}", kind)).unwrap();
            let s = scales_for(&c);
            assert!(!s.is_empty(), "no scale for C{}", kind);
            for (name, steps) in s {
                if matches!(name, "Minor pentatonic" | "Major pentatonic" | "Blues") {
                    continue;
                }
                for p in &c.pcs {
                    assert!(*p == 7 || steps.contains(p), "C{}: {} lacks {}", kind, name, p);
                }
            }
        }
        // the scale steps are the scales
        let steps =
            |n: &str, s: &str| scales_for(&chord_notes(n).unwrap()).into_iter().find(|x| x.0 == s).map(|x| x.1.to_vec()).unwrap();
        assert_eq!(steps("C7alt", "Altered"), vec![0, 1, 3, 4, 6, 8, 10]);
        assert_eq!(steps("C7b9", "Half-whole diminished"), vec![0, 1, 3, 4, 6, 7, 9, 10]);
        assert_eq!(steps("C7b9", "Phrygian dominant"), vec![0, 1, 4, 5, 7, 8, 10]);
        assert_eq!(steps("Cdim7", "Whole-half diminished"), vec![0, 2, 3, 5, 6, 8, 9, 11]);
        assert_eq!(steps("C7#11", "Lydian dominant"), vec![0, 2, 4, 6, 7, 9, 10]);
        assert_eq!(steps("Cm7b5", "Locrian nat 9"), vec![0, 2, 3, 5, 6, 8, 10]);
    }

    #[test]
    fn scales_are_spelt_from_the_root() {
        let j = |v: Vec<String>| v.join(" ");
        assert_eq!(j(scale_names("F#", 6, &[0, 2, 3, 5, 7, 9, 10])), "F# G# A B C# D# E");
        assert_eq!(j(scale_names("Gb", 6, &[0, 2, 4, 5, 7, 9, 11])), "Gb Ab Bb Cb Db Eb F");
        assert_eq!(j(scale_names("C", 0, &[0, 1, 3, 4, 6, 8, 10])), "C Db Eb E F# Ab Bb");
        assert_eq!(j(scale_names("C", 0, &[0, 1, 3, 4, 6, 7, 9, 10])), "C Db Eb E F# G A Bb");
        assert_eq!(j(scale_names("C", 0, &[0, 2, 3, 5, 6, 8, 9, 11])), "C D Eb F Gb Ab A B");
        assert_eq!(j(scale_names("C", 0, &[0, 2, 4, 6, 8, 10])), "C D E F# G# Bb");
        assert_eq!(j(scale_names("C", 0, &[0, 3, 5, 6, 7, 10])), "C Eb F Gb G Bb");
        assert_eq!(j(scale_names("Bb", 10, &[0, 2, 4, 6, 7, 9, 10])), "Bb C D E F G Ab");
        assert_eq!(written_root("Bbmaj7/D"), "Bb");
        // the parent scales on their own letters: A melodic minor's F# G#, D harmonic major's Bb, G harmonic minor's F#
        let parent = |key: &str, pc: i32, steps: [i32; 7]| {
            (0..7).map(|k| spell_degree(key, pc + steps[k], k + 1).unwrap()).collect::<Vec<_>>().join(" ")
        };
        assert_eq!(parent("A", 9, [0, 2, 3, 5, 7, 9, 11]), "A B C D E F# G#");
        assert_eq!(parent("D", 2, [0, 2, 4, 5, 7, 8, 11]), "D E F# G A Bb C#");
        assert_eq!(parent("G", 7, [0, 2, 3, 5, 7, 8, 11]), "G A Bb C D Eb F#");
        assert_eq!(parent("Eb", 3, [0, 2, 3, 5, 7, 8, 11]), "Eb F Gb Ab Bb Cb D");
    }

    #[test]
    fn the_band_voices_the_right_notes() {
        for kind in [
            "", "m", "7", "maj7", "m7", "m7b5", "dim7", "6", "m6", "69", "9", "m9", "maj9", "11", "m11", "13", "m13", "7b9",
            "7#9", "13b9", "13#11", "7#5", "7b5", "7#11", "7b13", "7alt", "7sus4", "9sus4", "maj7#11", "mMaj7", "aug", "dim",
            "sus4", "add9", "7b9b13",
        ] {
            for root in ["C", "Eb", "F#", "Ab", "B"] {
                let name = format!("{}{}", root, kind);
                let c = chord_notes(&name).unwrap();
                let rel = |p: i32| (p - c.root).rem_euclid(12);
                let v: Vec<i32> = crate::band::voicing_of(&name).unwrap().iter().map(|m| rel(*m)).collect();
                let chord: Vec<i32> = c.pcs.iter().map(|p| rel(*p)).collect();
                // nothing outside the chord
                for p in &v {
                    assert!(chord.contains(p), "{}: plays {} not in the chord", name, p);
                }
                // the 3rd (or what stands for it) and the 7th (or the 6th) always
                for d in [3, 4, 10, 11] {
                    if chord.contains(&d) && !(d == 3 && chord.contains(&4)) {
                        assert!(v.contains(&d), "{}: {:?} leaves out {}", name, v, d);
                    }
                }
                if kind.contains("sus") || kind == "11" {
                    assert!(v.contains(&5), "{}: no 4th", name);
                }
                if kind.starts_with('6') || kind.starts_with("m6") || kind.starts_with("69") || kind == "dim7" {
                    assert!(v.contains(&9), "{}: no 6th", name);
                }
                // the colour notes written
                for (t, d) in [("b9", 1), ("#9", 3), ("#11", 6), ("b13", 8), ("13", 9), ("alt", 3), ("alt", 8)] {
                    if kind.contains(t) && !(t == "13" && kind.contains("b13")) {
                        assert!(v.contains(&d), "{}: {:?} leaves out the {}", name, v, t);
                    }
                }
            }
        }
    }
}
