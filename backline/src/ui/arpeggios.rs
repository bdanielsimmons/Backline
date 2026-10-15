//! ARPEGGIOS (under LINES): the bridge between the scales and the chords - a chord's notes picked out of its scale,
//! in the same five positions as SCALES. The chords of a scale (each degree's 7th chord) or any chord on its own,
//! seen on the neck inside the scale around it, gripped, on the staff, and played: up and down, from the 3rd, 5th
//! or 7th, the 3-5-7-9 on top, every chord of the scale in turn (zig-zag), every position in turn. And through a
//! tune: the guide tones (3rds and 7ths, one a chord), the arpeggios one after another, 3-5-7-9 up and 5-3-1-7 down
//! into the next chord - each starting near where the last ended.

use super::*;
use crate::improv::{caged, neck_view, NeckDot, Spot};
use crate::lines;
use crate::theory;

/// The scales whose chords are offered: name, steps.
const AR_SCALES: [(&str, [i32; 7]); 3] =
    [("MAJOR", [0, 2, 4, 5, 7, 9, 11]), ("MELODIC MINOR", [0, 2, 3, 5, 7, 9, 11]), ("HARMONIC MINOR", [0, 2, 3, 5, 7, 8, 11])];

/// The chords offered on their own: suffix (as Backline reads chord names), notes from the root, the scale around it.
const AR_CHORDS: [(&str, [i32; 4], &[i32]); 11] = [
    ("maj7", [0, 4, 7, 11], &[0, 2, 4, 5, 7, 9, 11]),
    ("7", [0, 4, 7, 10], &[0, 2, 4, 5, 7, 9, 10]),
    ("m7", [0, 3, 7, 10], &[0, 2, 3, 5, 7, 9, 10]),
    ("m7b5", [0, 3, 6, 10], &[0, 2, 3, 5, 6, 8, 10]),
    ("dim7", [0, 3, 6, 9], &[0, 2, 3, 5, 6, 8, 9, 11]),
    ("6", [0, 4, 7, 9], &[0, 2, 4, 5, 7, 9, 11]),
    ("m6", [0, 3, 7, 9], &[0, 2, 3, 5, 7, 9, 11]),
    ("7#5", [0, 4, 8, 10], &[0, 2, 4, 6, 8, 10]),
    ("mMaj7", [0, 3, 7, 11], &[0, 2, 3, 5, 7, 9, 11]),
    ("maj7#5", [0, 4, 8, 11], &[0, 2, 4, 6, 8, 9, 11]),
    ("7sus4", [0, 5, 7, 10], &[0, 2, 4, 5, 7, 9, 10]),
];

const ROMAN: [&str; 7] = ["I", "II", "III", "IV", "V", "VI", "VII"];

/// What to play in a position.
const AR_EXERCISES: [(&str, &str); 9] = [
    ("UP AND DOWN", "The arpeggio up through the position and back down: learn one shape before the next"),
    ("DOWN AND UP", "From the top down, then back up"),
    ("FROM THE 3RD", "Up from the lowest 3rd and back to it: the chord heard from its 3rd"),
    ("FROM THE 5TH", "Up from the lowest 5th and back"),
    ("FROM THE 7TH", "Up from the lowest 7th and back"),
    ("3-5-7-9", "The arpeggio a 3rd up - 3 5 7 9: over Dm7 it is Fmaj7 - the richer sound with the 9th on top"),
    (
        "EVERY CHORD OF THE SCALE, ZIG-ZAG",
        "Each chord of the scale in turn in this position: the first up, the next down, and so on",
    ),
    ("EVERY CHORD, ZIG-ZAG STARTING DOWN", "The same, the first one coming down"),
    ("SHIFT THROUGH ALL POSITIONS", "Up and down in every position in turn, up the neck"),
];

/// A chord to arpeggiate: its root (pitch class), name, notes from the root, its 9th, and the scale it sits in
/// (from that scale's root).
#[derive(Clone)]
struct Arp {
    root: i32,
    name: String,
    tones: [i32; 4],
    ninth: i32,
    scale_root: i32,
    scale: Vec<i32>,
}

/// The 7th chord on degree `d` (0-based) of a 7-note scale, stacked in 3rds from the scale.
fn diatonic(key: i32, steps: &[i32; 7], d: usize, sharps: bool) -> Arp {
    let root = (key + steps[d]).rem_euclid(12);
    let rel = |k: usize| (steps[(d + k) % 7] - steps[d]).rem_euclid(12);
    let tones = [0, rel(2), rel(4), rel(6)];
    let suffix = AR_CHORDS.iter().find(|c| c.1 == tones).map_or("", |c| c.0);
    Arp {
        root,
        name: format!("{}{}", theory::note_name(root, sharps), suffix),
        tones,
        ninth: rel(1),
        scale_root: key.rem_euclid(12),
        scale: steps.to_vec(),
    }
}

/// The pitch with this pitch class nearest `to`, kept on the guitar's middle (open D to the 15th fret of the e).
fn near(pc: i32, to: i32) -> i32 {
    (50..=81).filter(|m| (m - pc).rem_euclid(12) == 0).min_by_key(|m| (m - to).abs()).unwrap_or(60)
}

/// A tune's changes as a line: (chord, root, its 3rd and 7th an octave up (to hear the chord under the line),
/// the notes). `kind`: 0 the guide tones - the 3rd or 7th nearest the last note, one a chord; 1 each arpeggio up
/// 1 3 5 7 from the root nearest the last note; 2 3 5 7 9 up, then 5 3 1 7 down on the next chord, and so on.
fn tune_line(changes: &[String], kind: usize) -> Vec<(String, i32, Vec<i32>, Vec<i32>)> {
    let mut out = Vec::new();
    let mut last = 62;
    let mut n_done = 0;
    for c in changes {
        let Some(n) = theory::chord_notes(c) else { continue };
        let r = n.root.rem_euclid(12);
        let has = |iv: i32| n.pcs.iter().any(|p| (p - r).rem_euclid(12) == iv);
        let third = if has(4) {
            4
        } else if has(3) {
            3
        } else if has(5) {
            5
        } else {
            4
        };
        let fifth = if has(7) {
            7
        } else if has(6) {
            6
        } else if has(8) {
            8
        } else {
            7
        };
        let seventh = if has(10) {
            10
        } else if has(11) {
            11
        } else if has(9) {
            9
        } else {
            12
        };
        let ninth = if has(1) && !has(2) { 1 } else { 2 };
        let mut notes: Vec<i32> = match kind {
            0 => {
                let a = near(r + third, last);
                let b = near(r + seventh, last);
                vec![if (a - last).abs() <= (b - last).abs() { a } else { b }]
            }
            1 => {
                let s = near(r, last);
                vec![s, s + third, s + fifth, s + seventh]
            }
            _ if n_done % 2 == 0 => {
                let s = near(r + third, last);
                vec![s, s + fifth - third, s + seventh - third, s + ninth + 12 - third]
            }
            _ => {
                let s = near(r + fifth, last);
                vec![s, s - (fifth - third), s - fifth, s - fifth - (12 - seventh)]
            }
        };
        // kept on the neck
        if notes.iter().any(|m| *m > 84) {
            notes.iter_mut().for_each(|m| *m -= 12);
        }
        if notes.iter().any(|m| *m < 47) {
            notes.iter_mut().for_each(|m| *m += 12);
        }
        last = *notes.last().unwrap_or(&last);
        out.push((c.clone(), r, vec![third + 12, seventh.min(11) + 12], notes));
        n_done += 1;
    }
    out
}

impl App {
    /// ARPEGGIOS: a chord's notes inside its scale, in the scale's positions, seen and played; and through a tune.
    pub(crate) fn arpeggios(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "The bridge between the scales and the chords: a chord's own notes, picked out of the scale around it, in the same positions as SCALES. See the shape inside the scale, grip the chord, play the arpeggio every way round - then run them through a tune.", pal().ink2);
        if let Some(i) =
            crate::choice_line(ui, "ar_src", "FROM", &["THE CHORDS OF A SCALE", "ANY CHORD"], self.ar_src as usize, 300.0)
        {
            self.ar_src = i as u8;
            self.ar_pos = 0;
        }
        let sharps = lines::key_sharps(self.ar_root);
        if let Some(pc) = Self::note_row(ui, if self.ar_src == 0 { "KEY" } else { "ROOT" }, self.ar_root, sharps) {
            self.ar_root = pc;
        }
        let root_key = self.ar_root.rem_euclid(12);
        // the chord: one of the scale's (with the rest of them, for playing them in turn) - or any on its own
        let (arp, family): (Arp, Vec<Arp>) = if self.ar_src == 0 {
            let names: Vec<&str> = AR_SCALES.iter().map(|s| s.0).collect();
            if let Some(i) = crate::choice_line(ui, "ar_scale", "SCALE", &names, (self.ar_scale as usize).min(2), 260.0) {
                self.ar_scale = i as u8;
            }
            let steps = AR_SCALES[(self.ar_scale as usize).min(2)].1;
            let all: Vec<Arp> = (0..7).map(|d| diatonic(root_key, &steps, d, sharps)).collect();
            let labels: Vec<String> =
                all.iter().enumerate().map(|(d, a)| format!("{}   {}", ROMAN[d], crate::chart::pretty_chord(&a.name))).collect();
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            if let Some(i) = crate::choice_line(ui, "ar_deg", "CHORD", &labels, (self.ar_deg as usize).min(6), 260.0) {
                self.ar_deg = i as u8;
            }
            (all[(self.ar_deg as usize).min(6)].clone(), all)
        } else {
            let labels: Vec<String> = AR_CHORDS
                .iter()
                .map(|c| crate::chart::pretty_chord(&format!("{}{}", theory::note_name(root_key, sharps), c.0)))
                .collect();
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            if let Some(i) =
                crate::choice_line(ui, "ar_q", "CHORD", &labels, (self.ar_q as usize).min(AR_CHORDS.len() - 1), 260.0)
            {
                self.ar_q = i as u8;
            }
            let c = AR_CHORDS[(self.ar_q as usize).min(AR_CHORDS.len() - 1)];
            let a = Arp {
                root: root_key,
                name: format!("{}{}", theory::note_name(root_key, sharps), c.0),
                tones: c.1,
                ninth: 2,
                scale_root: root_key,
                scale: c.2.to_vec(),
            };
            (a, Vec::new())
        };

        // ---- the positions: the scale's five, numbered as SCALES numbers them (1 = the root on the 5th string)
        let positions: Vec<Vec<Spot>> = caged(arp.scale_root, &arp.scale);
        let np = positions.len();
        let pitch = |s: &Spot| theory::STANDARD[s.0] + s.1 as i32;
        let mut by_neck: Vec<usize> = (0..np).collect();
        by_neck.sort_by_key(|p| positions[*p].iter().map(|s| s.1).min().unwrap_or(0));
        let about: Vec<String> = by_neck.iter().map(|p| crate::improv::root_place(&positions[*p], arp.root)).collect();
        let first = about.iter().position(|a| a.starts_with("ROOT ON 5TH")).unwrap_or(0);
        let nth = |k: usize| by_neck[(first + k) % np];
        let shown: Vec<String> = (0..np)
            .map(|k| format!("POSITION {}  -  {}", k + 1, about[(first + k) % np]))
            .chain(std::iter::once("WHOLE NECK".to_string()))
            .collect();
        let shown: Vec<&str> = shown.iter().map(String::as_str).collect();
        ui.horizontal(|ui| {
            crate::views::label(ui, "SHOW", 100.0);
            let pick = (self.ar_pos as usize).min(np);
            if retro_btn_w(ui, "<", 34.0, false).tip("The position below").clicked() {
                self.ar_pos = ((pick + np) % (np + 1)) as u8;
            }
            let widest = shown.iter().map(|s| crate::font::text_w(s, 2.0)).fold(0.0, f32::max) + 56.0;
            if let Some(i) = crate::dropdown_fixed(ui, "ar_pos", &shown, pick, widest.min(ui.available_width() - 60.0).max(200.0))
            {
                self.ar_pos = i as u8;
            }
            if retro_btn_w(ui, ">", 34.0, false).tip("The next position up").clicked() {
                self.ar_pos = ((pick + 1) % (np + 1)) as u8;
            }
        });
        let pick = (self.ar_pos as usize).min(np);

        // ---- what to play
        let offered: Vec<usize> = (0..AR_EXERCISES.len()).filter(|e| self.ar_src == 0 || !(6..=7).contains(e)).collect();
        if !offered.contains(&(self.ar_ex as usize)) {
            self.ar_ex = 0;
        }
        let names: Vec<&str> = offered.iter().map(|e| AR_EXERCISES[*e].0).collect();
        let cur = offered.iter().position(|e| *e == self.ar_ex as usize).unwrap_or(0);
        if let Some(i) = crate::choice_line(ui, "ar_ex", "PLAY", &names, cur, 300.0) {
            self.ar_ex = offered[i] as u8;
        }
        let ex = self.ar_ex as usize;
        crate::views::dim_line(ui, &AR_EXERCISES[ex].1.to_uppercase(), 1.0, pal().ink2);

        // a chord's notes in a position, low to high (`extra`: more degrees, the 9th for 3-5-7-9)
        let in_pos = |pos: &[Spot], a: &Arp, degs: &[i32]| -> Vec<Spot> {
            let mut v: Vec<Spot> = pos.iter().filter(|s| degs.contains(&((pitch(s) - a.root).rem_euclid(12)))).copied().collect();
            v.sort_by_key(|s| pitch(s));
            v.dedup_by_key(|s| pitch(s));
            v
        };
        let up_down = |u: Vec<Spot>| -> Vec<Spot> {
            let mut v = u.clone();
            v.extend(u.iter().rev().skip(1));
            v
        };
        let from_deg = |u: Vec<Spot>, a: &Arp, deg: i32| -> Vec<Spot> {
            let start = u.iter().position(|s| (pitch(s) - a.root).rem_euclid(12) == deg).unwrap_or(0);
            up_down(u[start..].to_vec())
        };
        let at: &Vec<Spot> = &positions[nth(if pick == np { 0 } else { pick })];
        // the notes, and which chord (or position) each belongs to
        let mut owner: Vec<usize> = Vec::new();
        let spots: Vec<Spot> = match ex {
            1 => {
                let mut v = up_down(in_pos(at, &arp, &arp.tones));
                v.reverse();
                v
            }
            2..=4 => from_deg(in_pos(at, &arp, &arp.tones), &arp, arp.tones[ex - 1]),
            5 => {
                let degs = [arp.tones[1], arp.tones[2], arp.tones[3], arp.ninth];
                from_deg(in_pos(at, &arp, &degs), &arp, arp.tones[1])
            }
            6 | 7 => {
                let mut all = Vec::new();
                for (d, a) in family.iter().enumerate() {
                    let mut u = in_pos(at, a, &a.tones);
                    if (d % 2 == 1) != (ex == 7) {
                        u.reverse();
                    }
                    owner.extend(std::iter::repeat(d).take(u.len()));
                    all.extend(u);
                }
                all
            }
            8 => {
                let mut all = Vec::new();
                for k in 0..np {
                    let seg = up_down(in_pos(&positions[nth(k)], &arp, &arp.tones));
                    owner.extend(std::iter::repeat(k).take(seg.len()));
                    all.extend(seg);
                }
                all
            }
            _ => up_down(in_pos(at, &arp, &arp.tones)),
        };
        let notes: Vec<i32> = spots.iter().map(pitch).collect();
        let playing = self.seq_now("line");
        let lit = playing.and_then(|i| spots.get(i).copied());
        // the chord sounding now (every chord in turn), and the position shown bright (shifting)
        let now_arp: Arp = match (ex, playing.and_then(|i| owner.get(i))) {
            (6 | 7, Some(d)) => family[*d].clone(),
            _ => arp.clone(),
        };
        let bright: usize = match (ex, playing.and_then(|i| owner.get(i))) {
            (8, Some(k)) => *k,
            _ => pick,
        };

        // ---- the chord: a grip near this position, and its notes on the staff
        let name = now_arp.name.clone();
        if let Some(cn) = theory::chord_notes(&name) {
            let shapes = self.shapes_for(&name, &cn);
            let centre = at.iter().map(|s| s.1 as f32).sum::<f32>() / at.len().max(1) as f32;
            let grip = shapes.iter().min_by(|a, b| {
                (theory::centre(a) - centre)
                    .abs()
                    .partial_cmp(&(theory::centre(b) - centre).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            // one area: the grip on the left (click it to hear it), the chord on the staff beside it
            crate::views::dim_line(ui, &crate::chart::pretty_chord(&name), 1.5, pal().ink);
            let (area, resp) = ui.allocate_exact_size(Vec2::new(320.0, 124.0), Sense::click());
            let rt = now_arp.root;
            let gr = Rect::from_min_size(area.min, Vec2::new(100.0, 124.0));
            if let Some(g) = grip {
                crate::fretboard::chord_box_labelled(ui.painter(), gr, g, &|m| {
                    crate::fretboard::degree(m.rem_euclid(12), rt).0.to_string()
                });
                if resp.clicked() && resp.interact_pointer_pos().is_some_and(|p| gr.contains(p)) {
                    acts.push(Action::PlayNotes(crate::fretboard::grip_notes(g)));
                }
            }
            let sr = Rect::from_min_max(Pos2::new(area.min.x + 112.0, area.min.y), area.max);
            let staffed: Vec<(i32, Option<(usize, u8)>)> = now_arp.tones.iter().map(|x| (60 + rt + x, None)).collect();
            let pcs: Vec<i32> = now_arp.tones.iter().map(|x| (rt + x).rem_euclid(12)).collect();
            crate::lines_ui::chord_staff(ui, sr, &staffed, sharps, &|m| {
                let (d, col) = crate::fretboard::degree_in(m.rem_euclid(12), rt, &pcs);
                (d.to_string(), col)
            });
        }

        // ---- the neck: the arpeggio bright (by degree), the rest of the scale around it faint
        let mut dots: Vec<NeckDot> = Vec::new();
        let show_pos: Vec<usize> = if pick == np && ex != 8 { (0..np).collect() } else { vec![bright.min(np - 1)] };
        let extra: Vec<i32> = if ex == 5 { vec![now_arp.ninth] } else { Vec::new() };
        for k in show_pos {
            for s in &positions[nth(k)] {
                if dots.iter().any(|d| d.spot == *s) {
                    continue;
                }
                let iv = (pitch(s) - now_arp.root).rem_euclid(12);
                let pc = pitch(s).rem_euclid(12);
                let (txt, col) = crate::fretboard::degree(pc, now_arp.root);
                let tone = now_arp.tones.contains(&iv) || extra.contains(&iv);
                dots.push(NeckDot {
                    spot: *s,
                    text: txt.to_string(),
                    col: if tone { col } else { pal().dim },
                    strength: if tone { 1.0 } else { 0.4 },
                    ring: !tone,
                    root: iv == 0,
                });
            }
        }
        neck_view(ui, &dots, lit);
        crate::views::dim_line(ui, "BRIGHT: THE ARPEGGIO (BY DEGREE)   RINGS: THE REST OF THE SCALE AROUND IT", 1.0, pal().dim);

        // ---- play it, over the chord
        ui.add_space(6.0);
        const OVER: [&str; 4] =
            ["NOTHING - JUST THE LINE", "DRONE - THE ROOT HELD", "CHORD - STRUMMED EVERY BAR", "BAND - DRUMS, BASS, COMPING"];
        if let Some(i) = crate::choice_line(ui, "ar_over", "OVER", &OVER, self.ar_back as usize, 300.0) {
            self.ar_back = i as u8;
        }
        // (every chord in turn changes chord: under it, just the key's root)
        self.ln_backing = Some(if ex == 6 || ex == 7 {
            (self.ar_back.min(1), vec![48 + arp.scale_root])
        } else {
            (self.ar_back, arp.tones.iter().map(|x| 48 + arp.root + x).collect())
        });
        let key = format!("ar:{}:{}:{}:{}:{}:{}:{}", self.ar_src, root_key, self.ar_scale, self.ar_deg, self.ar_q, pick, ex);
        self.line_player(ui, acts, &notes, &key);
        let tab: Vec<Option<(usize, u8)>> = spots.iter().map(|s| Some(*s)).collect();
        crate::lines_ui::line_score(ui, acts, &notes, self.ln_sub as usize, sharps, playing, Some(tab));

        ui.add_space(10.0);
        self.arpeggio_tune(ui, acts);
    }

    /// ARPEGGIOS THROUGH A TUNE: the guide tones, the arpeggios, or 3-5-7-9 up and down - one chord after another,
    /// each starting near where the last ended.
    fn arpeggio_tune(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        section_header(ui, "THROUGH A TUNE");
        para(ui, "The guide tones first: the 3rd or 7th of each chord, whichever is nearest - through a ii-V-I the 7th falls a half step to the next 3rd and the 3rd stays as the next 7th. Then the arpeggios one after another, each starting near the last. Hear it, then play it over the changes yourself.", pal().ink2);
        let (title, changes) = self.practice_changes(ui);
        if changes.is_empty() {
            return;
        }
        const KINDS: [&str; 3] = [
            "GUIDE TONES - THE 3RD OR 7TH, ONE A CHORD",
            "ARPEGGIOS - 1 3 5 7 UP, EACH NEAR THE LAST",
            "3 5 7 9 UP, THEN 5 3 1 7 DOWN INTO THE NEXT",
        ];
        if let Some(i) = crate::choice_line(ui, "ar_tune", "LINE", &KINDS, (self.ar_tune as usize).min(2), 320.0) {
            self.ar_tune = i as u8;
        }
        let kind = (self.ar_tune as usize).min(2);
        let line = tune_line(&changes, kind);
        if line.is_empty() {
            dim_line(ui, "NO CHORDS BACKLINE CAN READ IN THESE CHANGES", 1.0, pal().dim);
            return;
        }
        let sharps = theory::chord_notes(&line[0].0).map_or(false, |c| lines::key_sharps(c.root));
        // the guide tones a half note each (two beats a chord), the arpeggios in eighths
        let gap = if kind == 0 { 60.0 / self.band_bpm() * 2.0 } else { 60.0 / self.band_bpm() / 2.0 };
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, &title.to_uppercase(), 240.0);
            self.seq_button(
                ui,
                acts,
                "PLAY IT",
                "At the band's tempo, the chord under each one's first note",
                "arptune",
                || {
                    line.iter()
                        .flat_map(|(_, root, shell, notes)| {
                            notes.iter().enumerate().map(move |(k, m)| {
                                let mut v = vec![*m];
                                if k == 0 {
                                    let bass = 40 + (root - 40).rem_euclid(12);
                                    v.push(bass);
                                    v.extend(shell.iter().map(|x| bass + x));
                                }
                                v
                            })
                        })
                        .collect()
                },
                gap,
            );
        });
        // each chord and its notes, the one sounding lit
        let playing = self.seq_now("arptune");
        let mut at = 0;
        ui.horizontal_wrapped(|ui| {
            for (name, root, _, notes) in &line {
                let on = playing.is_some_and(|p| p >= at && p < at + notes.len());
                at += notes.len();
                let degs: Vec<String> = notes
                    .iter()
                    .map(|m| format!("{} {}", theory::note_name(*m, sharps), crate::fretboard::degree(m.rem_euclid(12), *root).0))
                    .collect();
                let txt = format!("{}: {}", crate::chart::pretty_chord(name), degs.join("  "));
                let w = crate::font::text_w(&txt, 1.0) + 14.0;
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::hover());
                if on {
                    ui.painter().rect_filled(r, 2.0, pal().sel);
                }
                ptext(ui.painter(), Pos2::new(r.min.x + 6.0, r.center().y), Align::Min, &txt, 1.0, pal().ink);
            }
        });
        let notes: Vec<i32> = line.iter().flat_map(|l| l.3.clone()).collect();
        crate::lines_ui::line_score(ui, acts, &notes, if kind == 0 { 1 } else { 2 }, sharps, playing, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chords_of_a_scale() {
        let c = AR_SCALES[0].1;
        let ii = diatonic(0, &c, 1, false);
        assert_eq!(ii.root, 2);
        assert_eq!(ii.tones, [0, 3, 7, 10]);
        assert_eq!(ii.name, "Dm7");
        assert_eq!(diatonic(0, &c, 4, false).name, "G7");
        assert_eq!(diatonic(0, &c, 6, false).name, "Bm7b5");
        // the 9th of the III chord is a b9 (phrygian)
        assert_eq!(diatonic(0, &c, 2, false).ninth, 1);
    }

    #[test]
    fn guide_tones_move_by_a_step_or_not_at_all() {
        let ch: Vec<String> = ["Dm7", "G7", "Cmaj7"].iter().map(|s| s.to_string()).collect();
        let l = tune_line(&ch, 0);
        let ns: Vec<i32> = l.iter().map(|x| x.3[0]).collect();
        for w in ns.windows(2) {
            assert!((w[1] - w[0]).abs() <= 2, "{:?}", ns);
        }
        // each is the chord's 3rd or 7th
        for (name, root, _, notes) in &l {
            let iv = (notes[0] - root).rem_euclid(12);
            assert!([3, 4, 10, 11].contains(&iv), "{} {}", name, iv);
        }
    }
}
