//! TRIAD PAIRS (under LINES > OVER CHORDS): two triads with six different notes between them, played in turn.
//! Over a chord - the pairs that fit it, best first, or any two you pick - or through changes (the open tune, or a
//! set progression in any key), a pair for each chord, with the band. On the whole neck or on one string set, the
//! exercises written out in notation and tab. The working out is in tp_core.rs.

use super::*;
use crate::improv::{chord_rel, look_piano, neck_view, NeckDot, Spot};
use crate::lines;
use crate::si_core::SI_SCALES;
use crate::theory;
use crate::tp_core::{self, Triad, Voicing, KINDS};
use std::collections::HashMap;

/// Where the voicings' lowest notes may be, anywhere on the neck (E on the 4th string's 2nd fret to the A above).
const LO: i32 = 52;
const HI: i32 = 69;
/// The highest fret used on a string set.
const TOP: u8 = 15;
/// The two triads' colours (as in PENTATONICS & MORE).
const COL_A: Color32 = Color32::from_rgb(95, 168, 211);
const COL_B: Color32 = Color32::from_rgb(217, 164, 65);

/// The set changes to play through: name, then each bar's chords as (root from the key, suffix).
type Prog = (&'static str, &'static [&'static [(i32, &'static str)]]);
const PROGS: [Prog; 6] = [
    ("II-V-I IN MAJOR", &[&[(2, "m7")], &[(7, "7")], &[(0, "maj7")], &[(0, "maj7")]]),
    ("II-V-I IN MINOR", &[&[(2, "m7b5")], &[(7, "7b9")], &[(0, "m6")], &[(0, "m6")]]),
    ("I-VI-II-V", &[&[(0, "maj7")], &[(9, "m7")], &[(2, "m7")], &[(7, "7")]]),
    ("III-VI-II-V-I", &[&[(4, "m7")], &[(9, "7")], &[(2, "m7")], &[(7, "7")], &[(0, "maj7")], &[(0, "maj7")]]),
    (
        "JAZZ BLUES",
        &[
            &[(0, "7")],
            &[(5, "7")],
            &[(0, "7")],
            &[(7, "m7"), (0, "7")],
            &[(5, "7")],
            &[(6, "dim7")],
            &[(0, "7")],
            &[(9, "7")],
            &[(2, "m7")],
            &[(7, "7")],
            &[(0, "7"), (9, "7")],
            &[(2, "m7"), (7, "7")],
        ],
    ),
    ("COLTRANE CYCLE", &[&[(0, "maj7"), (3, "7")], &[(8, "maj7"), (11, "7")], &[(4, "maj7"), (7, "7")], &[(0, "maj7")]]),
];

/// The string sets: name, the lowest of the three (0 = low E); 0 is anywhere.
const STRINGS: [(&str, usize); 5] =
    [("ANYWHERE", 0), ("1ST 2ND 3RD", 3), ("2ND 3RD 4TH", 2), ("3RD 4TH 5TH", 1), ("4TH 5TH 6TH", 0)];

/// A triad's name, spelt in its own key: D, EM, F#+, BDIM.
fn triad_name(t: Triad) -> String {
    format!("{}{}", theory::note_name(t.root, lines::key_sharps(t.root)), KINDS[t.kind as usize].1)
}

/// A scale's name without what is in brackets: LYDIAN (#11) is LYDIAN.
fn scale_short(i: usize) -> &'static str {
    SI_SCALES[i].0.split(" (").next().unwrap_or(SI_SCALES[i].0)
}

impl App {
    /// The voicings of a triad where the page plays them: anywhere (by pitch), or on the string set picked.
    fn tp_voicings(&self, t: Triad, second: bool) -> Vec<Voicing> {
        let s = (self.tp_strings as usize).min(STRINGS.len() - 1);
        if s == 0 {
            tp_core::by_pitch(t, LO, HI, second)
        } else {
            tp_core::on_strings(t, STRINGS[s].1, &theory::STANDARD, TOP, second)
        }
    }

    /// TRIAD PAIRS.
    pub(crate) fn triad_pairs(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Two triads that share no notes, played one after the other. Six notes that sound like the two chords, not like a scale. Pick a chord to see the pairs that fit it, or run a pair through each chord of the changes.", pal().ink2);
        if let Some(i) =
            crate::choice_line(ui, "tp_mode", "OVER", &["A CHORD", "THE CHANGES"], (self.tp_mode as usize).min(1), 200.0)
        {
            self.tp_mode = i as u8;
        }
        if let Some(i) = crate::choice_line(
            ui,
            "tp_strings",
            "STRINGS",
            &STRINGS.map(|s| s.0),
            (self.tp_strings as usize).min(STRINGS.len() - 1),
            200.0,
        ) {
            self.tp_strings = i as u8;
        }
        if self.tp_mode == 1 {
            self.tp_changes(ui, acts);
        } else {
            self.tp_chord(ui, acts);
        }
    }

    /// Over a chord: the pair (suggested, or any two), what it is over the chord, the neck, the exercise, the line.
    fn tp_chord(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let (root, suffix, chord_name, chord) = self.si_chord_pick(ui);
        let chord_abs: Vec<i32> = chord.iter().map(|x| (root + x).rem_euclid(12)).collect();
        if let Some(i) = crate::choice_line(
            ui,
            "tp_any",
            "CHOOSE FROM",
            &["PAIRS THAT FIT", "ANY TWO TRIADS"],
            self.tp_any as usize,
            260.0,
        ) {
            self.tp_any = i == 1;
        }
        let deg = |pc: i32| crate::fretboard::degree_in(pc, root, &chord_abs).0;
        let degs = |t: Triad| t.pcs().iter().map(|p| deg(*p)).collect::<Vec<_>>().join(" ");
        let (a, b, about): (Triad, Triad, Vec<String>) = if self.tp_any {
            let names: Vec<&str> = (0..12).map(|pc| theory::note_name(pc, lines::key_sharps(pc))).collect();
            let kinds: Vec<&str> = KINDS.iter().map(|k| k.0).collect();
            for (label, id, which) in [("FIRST TRIAD", "tp_a", 0), ("SECOND TRIAD", "tp_b", 1)] {
                let cur = if which == 0 { self.tp_a } else { self.tp_b };
                let mut next = cur;
                ui.horizontal(|ui| {
                    crate::views::form_label(ui, label);
                    if let Some(i) = crate::dropdown_fixed(ui, &format!("{id}_root"), &names, cur.0.rem_euclid(12) as usize, 70.0) {
                        next.0 = i as i32;
                    }
                    if let Some(i) = crate::dropdown_fixed(ui, &format!("{id}_kind"), &kinds, (cur.1 as usize).min(3), 150.0) {
                        next.1 = i as u8;
                    }
                });
                if which == 0 {
                    self.tp_a = next;
                } else {
                    self.tp_b = next;
                }
            }
            let (a, b) = (Triad::new(self.tp_a.0, self.tp_a.1), Triad::new(self.tp_b.0, self.tp_b.1));
            let mut about = vec![format!("{} = {}     {} = {}", triad_name(a), degs(a), triad_name(b), degs(b))];
            // what to look out for: shared notes, notes that rub
            let shared: Vec<&str> = a.pcs().iter().filter(|p| b.pcs().contains(p)).map(|p| theory::note_name(*p, false)).collect();
            if !shared.is_empty() {
                about.push(format!("THEY SHARE {}: NOT A TRUE PAIR", shared.join(" AND ")));
            }
            let mut rub: Vec<i32> = a.pcs().iter().chain(b.pcs().iter()).copied().filter(|p| tp_core::avoid(&chord, p - root)).collect();
            rub.dedup();
            if !rub.is_empty() {
                let n: Vec<&str> = rub.iter().map(|p| theory::note_name(*p, lines::key_sharps(root))).collect();
                about.push(format!("{} RUBS OVER {}: DON'T LAND ON IT", n.join(" AND "), chord_name));
            }
            (a, b, about)
        } else {
            let fits = tp_core::pairs_for(&chord);
            let (a, b) = tp_core::best_pair(&chord);
            if fits.is_empty() {
                (a.moved(root), b.moved(root), vec!["NO PAIR FITS: ITS OWN TRIAD AND THE ONE A STEP UP".to_string()])
            } else {
                let labels: Vec<String> = fits
                    .iter()
                    .map(|f| {
                        format!(
                            "{} + {}  \u{00B7}  {} {}",
                            triad_name(f.a.moved(root)),
                            triad_name(f.b.moved(root)),
                            theory::note_name(root, lines::key_sharps(root)),
                            scale_short(f.scale)
                        )
                    })
                    .collect();
                let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
                let pick = (self.tp_pick as usize).min(fits.len() - 1);
                if let Some(i) = crate::choice_line(ui, "tp_pick", "PAIR", &labels, pick, 320.0) {
                    self.tp_pick = i as u8;
                }
                let f = fits[(self.tp_pick as usize).min(fits.len() - 1)];
                let (a, b) = (f.a.moved(root), f.b.moved(root));
                // what it adds: the notes not in the chord
                let mut adds: Vec<i32> = a.pcs().iter().chain(b.pcs().iter()).copied().filter(|p| !chord_abs.contains(p)).collect();
                adds.sort_by_key(|p| (p - root).rem_euclid(12));
                let adds: Vec<&str> = adds.iter().map(|p| deg(*p)).collect();
                let mut about = vec![format!("{} = {}     {} = {}", triad_name(a), degs(a), triad_name(b), degs(b))];
                about.push(format!(
                    "BOTH FROM {} {}.{}",
                    theory::note_name(root, lines::key_sharps(root)),
                    SI_SCALES[f.scale].0,
                    if adds.is_empty() { String::new() } else { format!(" THEY ADD {}.", adds.join(" ")) }
                ));
                (a, b, about)
            }
        };
        section_header(ui, &format!("{} + {} OVER {}", triad_name(a), triad_name(b), chord_name));
        for line in &about {
            crate::views::dim_line(ui, line, 1.0, pal().ink2);
        }

        // the exercise
        let set = self.tp_strings > 0;
        let offered: Vec<&(u8, &str, &str, bool)> = tp_core::EXERCISES.iter().filter(|e| e.3 || !set).collect();
        if !offered.iter().any(|e| e.0 == self.tp_ex) {
            self.tp_ex = offered[0].0;
        }
        let names: Vec<&str> = offered.iter().map(|e| e.1).collect();
        let cur = offered.iter().position(|e| e.0 == self.tp_ex).unwrap_or(0);
        if let Some(i) = crate::choice_line(ui, "tp_ex", "PLAY", &names, cur, 220.0) {
            self.tp_ex = offered[i].0;
        }
        let ex = offered.iter().find(|e| e.0 == self.tp_ex).map_or("", |e| e.2);
        crate::views::dim_line(ui, &crate::font::as_label(ex), 1.0, pal().ink2);
        let (va, vb) = (self.tp_voicings(a, false), self.tp_voicings(b, true));
        let pcs: Vec<i32> = a.pcs().iter().chain(b.pcs().iter()).copied().collect();
        let line = tp_core::exercise(self.tp_ex, &va, &vb, &pcs, LO);
        if line.is_empty() {
            crate::views::dim_line(ui, "THESE TWO DON'T FIT ON THESE STRINGS. TRY ANOTHER SET.", 1.0, pal().ink2);
            return;
        }
        let notes: Vec<i32> = line.iter().map(|n| n.0).collect();
        let tab: Vec<Option<Spot>> = if set { line.iter().map(|n| n.1).collect() } else { lines::finger_line(&notes, &theory::STANDARD) };
        let lit = self.seq_now("line").and_then(|i| tab.get(i).copied().flatten());
        self.tp_neck(ui, a, b, root, &chord_abs, lit);
        let keys: Vec<i32> = match self.seq_now("line").and_then(|i| notes.get(i)) {
            Some(m) => vec![*m],
            None => pcs.iter().map(|p| 60 + (p - root).rem_euclid(12) + root.rem_euclid(12)).collect(),
        };
        look_piano(ui, "tp_piano", &keys, Some(root));

        // played over the chord
        ui.add_space(6.0);
        let voiced: Vec<i32> = chord.iter().take(4).map(|x| 48 + root + x).collect();
        self.ln_backing = Some((self.ln_over, voiced));
        let key = format!("tp:{}:{}:{}:{}:{}:{}", root, suffix, triad_name(a), triad_name(b), self.tp_ex, self.tp_strings);
        self.line_player(ui, acts, &notes, &key);
        let lit_i = self.seq_now("line");
        crate::lines_ui::line_score(ui, acts, &notes, self.ln_sub as usize, lines::key_sharps(root), lit_i, Some(tab));
    }

    /// The neck: the two triads in their colours, each note named over the chord; the chord's other notes ringed;
    /// off the string set picked, dimmed.
    fn tp_neck(&self, ui: &mut egui::Ui, a: Triad, b: Triad, root: i32, chord_abs: &[i32], lit: Option<Spot>) {
        let s_i = (self.tp_strings as usize).min(STRINGS.len() - 1);
        let on_set = |s: usize| s_i == 0 || (STRINGS[s_i].1..STRINGS[s_i].1 + 3).contains(&s);
        let mut dots: Vec<NeckDot> = Vec::new();
        for s in 0..6 {
            for f in 0..=TOP {
                let pc = (theory::STANDARD[s] + f as i32).rem_euclid(12);
                let text = crate::fretboard::degree_in(pc, root, chord_abs).0.to_string();
                let col = if a.pcs().contains(&pc) {
                    COL_A
                } else if b.pcs().contains(&pc) {
                    COL_B
                } else {
                    if chord_abs.contains(&pc) {
                        dots.push(NeckDot { spot: (s, f), text, col: pal().dim, strength: 0.6, ring: true, root: false });
                    }
                    continue;
                };
                let strength = if on_set(s) { 1.0 } else { 0.22 };
                dots.push(NeckDot { spot: (s, f), text, col, strength, ring: false, root: pc == root.rem_euclid(12) });
            }
        }
        neck_view(ui, &dots, lit);
        crate::views::dim_line(ui, &format!("BLUE: {}     ORANGE: {}", triad_name(a), triad_name(b)), 1.0, pal().ink2);
    }

    /// The changes to run through: the title, each chord in turn with its beats, and the chart the band plays (its
    /// text and its bars in play order). The open tune's, or a set progression in the key picked.
    fn tp_progression(&mut self, ui: &mut egui::Ui) -> (String, Vec<(String, f32)>, String, Vec<usize>) {
        let tune: Option<(String, String)> = self.chart_text().map(|text| {
            let name = self.chart_tune().map(|i| self.store.tunes[i].name.clone()).unwrap_or_else(|| "THIS SONG".to_string());
            (name, text)
        });
        // tp_prog: 0 the open tune, then the set ones
        let mut items: Vec<String> = Vec::new();
        if let Some((name, _)) = &tune {
            items.push(format!("THE OPEN TUNE: {}", name.to_uppercase()));
        }
        items.extend(PROGS.iter().map(|p| p.0.to_string()));
        let off = tune.is_none() as usize;
        let shown = (self.tp_prog as usize).max(off) - off;
        let shown = shown.min(items.len() - 1);
        let labels: Vec<&str> = items.iter().map(String::as_str).collect();
        if let Some(i) = crate::choice_line(ui, "tp_prog", "CHANGES", &labels, shown, 300.0) {
            self.tp_prog = (i + off) as u8;
        }
        let prog = (self.tp_prog as usize).max(off);
        if prog == 0 {
            if let Some((name, text)) = tune {
                let chart = crate::chart::parse_friendly(&text);
                let order = chart.play_order();
                let mut chords: Vec<(String, f32)> = Vec::new();
                let mut last: Vec<String> = Vec::new();
                for b in &order {
                    let bar = &chart.bars[*b];
                    let beats = if bar.len > 0.0 { bar.len } else { chart.beats.max(1) as f32 };
                    // (a bar with no chords of its own: the last ones go on)
                    if !bar.chords.is_empty() {
                        last = bar.chords.clone();
                    }
                    let n = last.len().max(1) as f32;
                    if last.is_empty() {
                        chords.push(("N.C.".to_string(), beats));
                    }
                    for c in &last {
                        chords.push((c.clone(), beats / n));
                    }
                }
                return (name, chords, text, order);
            }
        }
        let p = PROGS[(prog - 1).min(PROGS.len() - 1)];
        let sharps = lines::key_sharps(self.tp_key);
        if let Some(pc) = Self::note_row(ui, "KEY", self.tp_key, sharps) {
            self.tp_key = pc;
        }
        let key = self.tp_key;
        let name = |r: i32, s: &str| format!("{}{}", theory::note_name(key + r, sharps), s);
        let name = &name;
        let chords: Vec<(String, f32)> =
            p.1.iter().flat_map(|bar| bar.iter().map(move |(r, s)| (name(*r, s), 4.0 / bar.len() as f32))).collect();
        let text = format!(
            "T44 {}|",
            p.1.iter().map(|bar| format!("| {} ", bar.iter().map(|(r, s)| name(*r, s)).collect::<Vec<_>>().join(" "))).collect::<String>()
        );
        let order: Vec<usize> = (0..p.1.len()).collect();
        (p.0.to_string(), chords, text, order)
    }

    /// Through the changes: a pair for each chord (the best that fits it), one line going on from chord to chord,
    /// played with the band.
    fn tp_changes(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let (title, chords, text, order) = self.tp_progression(ui);
        if chords.is_empty() {
            crate::views::dim_line(ui, "THESE CHANGES HAVE NO CHORDS.", 1.0, pal().ink2);
            return;
        }
        // each chord: its root and notes, and its pair (worked out once a chord name)
        let mut memo: HashMap<String, Option<(i32, Vec<i32>, Triad, Triad)>> = HashMap::new();
        let mut info: Vec<Option<(i32, Vec<i32>, Triad, Triad)>> = Vec::new();
        for (name, _) in &chords {
            let got = memo
                .entry(name.clone())
                .or_insert_with(|| {
                    let n = theory::chord_notes(name)?;
                    let rel = chord_rel(name);
                    let (a, b) = tp_core::best_pair(&rel);
                    Some((n.root, rel, a.moved(n.root), b.moved(n.root)))
                })
                .clone();
            info.push(got);
        }
        let sub = self.ln_sub.max(1) as f32;
        let parts: Vec<(Vec<Voicing>, Vec<Voicing>, usize)> = chords
            .iter()
            .zip(&info)
            .map(|((_, beats), c)| {
                let len = (beats * sub).round().max(1.0) as usize;
                match c {
                    Some((_, _, a, b)) => (self.tp_voicings(*a, false), self.tp_voicings(*b, true), len),
                    None => (Vec::new(), Vec::new(), len),
                }
            })
            .collect();
        let line = tp_core::through(&parts);
        let set = self.tp_strings > 0;
        let notes: Vec<i32> = line.iter().map(|n| n.0).collect();
        let tab: Vec<Option<Spot>> = if set { line.iter().map(|n| n.1).collect() } else { lines::finger_line(&notes, &theory::STANDARD) };

        // the pairs, each chord once
        section_header(ui, &format!("PAIRS THROUGH {}", title.to_uppercase()));
        let mut seen: Vec<&str> = Vec::new();
        let mut list: Vec<String> = Vec::new();
        for ((name, _), c) in chords.iter().zip(&info) {
            if seen.contains(&name.as_str()) {
                continue;
            }
            seen.push(name);
            if let Some((_, _, a, b)) = c {
                // (one chord's pair never broken over two lines)
                list.push(format!("{}: {} + {}", name, triad_name(*a), triad_name(*b)).replace(' ', "\u{00A0}"));
            }
        }
        para(ui, &list.join("     "), pal().ink);
        crate::views::dim_line(ui, "EACH CHORD'S BEST PAIR. THE LINE RUNS ON FROM CHORD TO CHORD.", 1.0, pal().ink2);

        // the neck: the pair of the chord sounding (the first, before it plays)
        let i = self.seq_now("line").unwrap_or(0);
        let mut at = 0;
        let mut now = 0;
        for (k, p) in parts.iter().enumerate() {
            if i < at + p.2 {
                now = k;
                break;
            }
            at += p.2;
        }
        let lit = self.seq_now("line").and_then(|i| tab.get(i).copied().flatten());
        match &info[now] {
            Some((root, rel, a, b)) => {
                let chord_abs: Vec<i32> = rel.iter().map(|x| (root + x).rem_euclid(12)).collect();
                crate::views::dim_line(ui, &format!("NOW: {}  ({} + {})", chords[now].0, triad_name(*a), triad_name(*b)), 1.0, pal().ink);
                self.tp_neck(ui, *a, *b, *root, &chord_abs, lit);
            }
            None => crate::views::dim_line(ui, &format!("NOW: {}", chords[now].0), 1.0, pal().ink),
        }

        // the band on the changes
        ui.add_space(6.0);
        let mut voiced: Vec<(Vec<i32>, f32)> = Vec::new();
        for ((_, beats), c) in chords.iter().zip(&info) {
            let v = match c {
                Some((root, rel, ..)) => rel.iter().take(4).map(|x| 48 + root + x).collect(),
                None => voiced.last().map(|l| l.0.clone()).unwrap_or_else(|| vec![48, 52, 55]),
            };
            voiced.push((v, *beats));
        }
        self.ln_backing = Some((self.ln_over, voiced[0].0.clone()));
        self.ln_prog = Some((text, order, voiced));
        let key = format!("tpc:{}:{}:{}", title, self.tp_key, self.tp_strings);
        self.line_player(ui, acts, &notes, &key);
        let lit_i = self.seq_now("line");
        let sharps = info.iter().flatten().next().map_or(false, |c| lines::key_sharps(c.0));
        crate::lines_ui::line_score(ui, acts, &notes, self.ln_sub as usize, sharps, lit_i, Some(tab));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_set_progression_has_a_pair_for_each_chord() {
        for (name, bars) in PROGS {
            for bar in bars {
                for (r, s) in bar.iter() {
                    let chord = format!("{}{}", theory::note_name(5 + r, false), s);
                    assert!(theory::chord_notes(&chord).is_some(), "{name}: {chord}");
                    assert!(!tp_core::pairs_for(&chord_rel(&chord)).is_empty(), "{name}: {chord}");
                }
            }
        }
    }
}
