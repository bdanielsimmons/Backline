//! TRIADS and MOVES (under CHORDS). TRIADS: a triad's inversions mapped on the neck on any set of strings; voice
//! leading through changes (each chord the closest inversion to the last, on the same strings); and a triad over a
//! bass note, named (upper structures). MOVES (experimental): common gospel moves built from triads, with the
//! hammer-ons, slides and whammy dips marked; and McCoy Tyner style stacks of 4ths (and of 2nds - clusters) planed
//! through a mode. All of it is worked out from the theory, written on a staff with tab under it, and played.

use super::*;
use crate::chords::{degree, grip_notes, CL_BASS, CL_TONE};
use crate::clusters::chord_heads;
use crate::fretboard::fret_map;
use crate::theory;
use crate::views::field;

const KEYS: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];
const QUALITIES: [(&str, [i32; 3]); 6] = [
    ("MAJOR", [0, 4, 7]),
    ("MINOR", [0, 3, 7]),
    ("DIM", [0, 3, 6]),
    ("AUG", [0, 4, 8]),
    ("SUS2", [0, 2, 7]),
    ("SUS4", [0, 5, 7]),
];
/// string sets, low string first (0 is the low E); named the guitarist's way (1 is the high E)
const SETS: [(&str, &[usize]); 7] = [
    ("STRINGS 1 2 3", &[3, 4, 5]),
    ("STRINGS 2 3 4", &[2, 3, 4]),
    ("STRINGS 3 4 5", &[1, 2, 3]),
    ("STRINGS 4 5 6", &[0, 1, 2]),
    ("1 2 4 (SPREAD)", &[2, 4, 5]),
    ("2 3 5 (SPREAD)", &[1, 3, 4]),
    ("3 4 6 (SPREAD)", &[0, 2, 3]),
];
const SETS4: [(&str, &[usize]); 3] =
    [("STRINGS 1-4", &[2, 3, 4, 5]), ("STRINGS 2-5", &[1, 2, 3, 4]), ("STRINGS 3-6", &[0, 1, 2, 3])];
const POSITIONS: [(&str, f32); 3] = [("LOW ON THE NECK", 2.0), ("MIDDLE", 7.0), ("HIGH", 12.0)];

/// How a step of a move is played.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Art {
    Strum,
    /// the notes that change are hammered on, the rest ring on
    Hammer,
    /// slid into from the chord before
    Slide,
    /// a whammy-bar dip on it
    Dip,
}

/// One chord of a move: its grip, its name, how it is played and how long it lasts (beats).
#[derive(Clone)]
pub(crate) struct Step {
    pub grip: [Option<u8>; 6],
    pub name: String,
    pub art: Art,
    pub beats: f32,
}

pub(crate) use crate::theory::{centre, grips_on};

/// The grip of these notes closest to `prev` (the fewest frets moved), or to fret `near` when there is none before.
fn closest(options: &[[Option<u8>; 6]], prev: Option<&[Option<u8>; 6]>, near: f32) -> Option<[Option<u8>; 6]> {
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
fn shifted(g: &[Option<u8>; 6], d: i32) -> [Option<u8>; 6] {
    let mut out = *g;
    for f in out.iter_mut().flatten() {
        *f = (*f as i32 + d).clamp(0, 17) as u8;
    }
    out
}

/// A bass note added under a grip on the 6th or 5th string (whichever is free and closest to where the grip is).
fn with_bass(g: &[Option<u8>; 6], bass: i32) -> [Option<u8>; 6] {
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
fn grip_name(g: &[Option<u8>; 6], sp: theory::Spelling) -> String {
    let notes = grip_notes(g);
    let pcs: Vec<i32> = notes.iter().map(|m| m.rem_euclid(12)).collect();
    let bass = notes.iter().min().map(|m| m.rem_euclid(12));
    bass.and_then(|b| theory::suggestions(&pcs, Some(b)).first().map(|s| s.name(sp))).unwrap_or_default()
}

/// A move written out: a staff with each chord stacked in its own column, the tab under it, the chord names over
/// it and the hammer-ons (H), slides (SL) and whammy dips (DIP) marked. Notes that move from the chord before are
/// blue; the ones that stay are black. Returns a column clicked.
fn steps_score(ui: &mut egui::Ui, steps: &[Step], sharps: bool, lit: Option<usize>) -> Option<usize> {
    let g = 6.0_f32;
    let col_w = 78.0_f32;
    let left = 44.0_f32;
    let w = ui.available_width().max(240.0);
    let per = (((w - left - 10.0) / col_w).floor() as usize).max(2);
    let mut hit = None;
    for (row, chunk) in steps.chunks(per).enumerate() {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 196.0), Sense::click());
        let p = ui.painter();
        p.rect_filled(rect.shrink2(Vec2::new(0.0, 2.0)), 3.0, pal().beige_lt);
        let top = rect.min.y + 52.0;
        let bottom = top + 4.0 * g;
        let line = Stroke::new(1.0_f32, pal().ink2);
        let x1 = rect.min.x + left + chunk.len() as f32 * col_w;
        for k in 0..5 {
            let y = top + k as f32 * g;
            p.line_segment([Pos2::new(rect.min.x + 6.0, y), Pos2::new(x1, y)], line);
        }
        pixmap(
            p,
            Pos2::new(rect.min.x + 10.0, top - 1.7 * g),
            7.4 * g / crate::lines_ui::CLEF.len() as f32,
            &crate::lines_ui::CLEF,
            pal().ink,
        );
        let tab_top = bottom + 46.0;
        let tg = 8.0_f32;
        let tab_y = |s: usize| tab_top + (5 - s) as f32 * tg;
        for s in 0..6 {
            p.line_segment([Pos2::new(rect.min.x + 6.0, tab_y(s)), Pos2::new(x1, tab_y(s))], line);
        }
        for (k, ch) in ["T", "A", "B"].iter().enumerate() {
            ptext(p, Pos2::new(rect.min.x + 18.0, tab_top + 6.0 + k as f32 * 14.0), Align::Center, ch, 1.0, pal().ink2);
        }
        for (j, st) in chunk.iter().enumerate() {
            let i = row * per + j;
            let x = rect.min.x + left + (j as f32 + 0.5) * col_w;
            if lit == Some(i) {
                p.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(x - col_w / 2.0, rect.min.y + 4.0),
                        Pos2::new(x + col_w / 2.0, rect.max.y - 4.0),
                    ),
                    2.0,
                    pal().red.gamma_multiply(0.15),
                );
            }
            let notes = grip_notes(&st.grip);
            let before: Vec<i32> = if i > 0 { grip_notes(&steps[i - 1].grip) } else { notes.clone() };
            let moved: Vec<i32> = notes.iter().filter(|m| !before.contains(m)).map(|m| m.rem_euclid(12)).collect();
            ptext_fit(p, Pos2::new(x, rect.min.y + 14.0), Align::Center, &st.name, 2.0, col_w - 6.0, pal().ink);
            chord_heads(p, x + 4.0, bottom, g, &notes, &|pc| moved.contains(&pc).then_some(CL_TONE), sharps);
            for s in 0..6 {
                let Some(f) = st.grip[s] else { continue };
                let txt = f.to_string();
                let changed = i > 0 && steps[i - 1].grip[s] != Some(f);
                p.rect_filled(
                    Rect::from_center_size(Pos2::new(x, tab_y(s)), Vec2::new(text_w(&txt, 1.0) + 6.0, tg)),
                    0.0,
                    pal().beige_lt,
                );
                ptext(p, Pos2::new(x, tab_y(s)), Align::Center, &txt, 1.0, if changed { CL_TONE } else { pal().ink });
                // the hammer-on or slide into it, on the strings that change
                if changed && j > 0 {
                    let mid = x - col_w / 2.0;
                    match st.art {
                        Art::Hammer => {
                            ptext(p, Pos2::new(mid, tab_y(s) - 1.0), Align::Center, "H", 1.0, pal().red);
                        }
                        Art::Slide => {
                            p.line_segment(
                                [Pos2::new(mid - 10.0, tab_y(s) + 3.0), Pos2::new(mid + 10.0, tab_y(s) - 3.0)],
                                Stroke::new(1.5_f32, pal().red),
                            );
                        }
                        _ => {}
                    }
                }
            }
            let mark_y = tab_top - 12.0;
            match st.art {
                Art::Dip => {
                    // a dip: the bar pushed down and let back up
                    let pts = vec![
                        Pos2::new(x - 14.0, mark_y - 4.0),
                        Pos2::new(x - 4.0, mark_y + 3.0),
                        Pos2::new(x + 6.0, mark_y - 4.0),
                    ];
                    p.add(egui::Shape::line(pts, Stroke::new(1.5_f32, pal().red)));
                    ptext(p, Pos2::new(x + 10.0, mark_y), Align::Min, "DIP", 1.0, pal().red);
                }
                Art::Hammer => {
                    ptext(p, Pos2::new(x, mark_y), Align::Center, "HAMMER", 1.0, pal().red);
                }
                Art::Slide => {
                    ptext(p, Pos2::new(x, mark_y), Align::Center, "SLIDE", 1.0, pal().red);
                }
                Art::Strum => {}
            }
        }
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let k = ((pos.x - rect.min.x - left) / col_w).floor();
                if k >= 0.0 && (k as usize) < chunk.len() {
                    hit = Some(row * per + k as usize);
                }
            }
        }
        ui.add_space(6.0);
    }
    hit
}

/// The notes of a move as the strings play them: (string, note, start in seconds, how hard). A hammered step only
/// sounds the notes that change (softly, no pick) and lets the rest ring; a string that stops playing is damped.
fn steps_events(steps: &[Step], spb: f32) -> (Vec<(usize, i32, f32, f32)>, Vec<f32>) {
    let mut ev = Vec::new();
    let mut starts = Vec::new();
    let mut t = 0.1_f32;
    for (i, st) in steps.iter().enumerate() {
        starts.push(t);
        let prev = if i > 0 { Some(steps[i - 1].grip) } else { None };
        let mut strum = 0.0;
        for s in 0..6 {
            match st.grip[s] {
                Some(f) => {
                    let same = prev.is_some_and(|p| p[s] == Some(f));
                    let m = theory::STANDARD[s] + f as i32;
                    match st.art {
                        Art::Hammer | Art::Slide if i > 0 => {
                            if !same {
                                ev.push((s, m, t, 0.55));
                            }
                        }
                        _ => {
                            ev.push((s, m, t + strum, 0.9));
                            strum += 0.022;
                        }
                    }
                }
                None => {
                    if prev.is_some_and(|p| p[s].is_some()) {
                        ev.push((s, -1, t, 0.0));
                    }
                }
            }
        }
        t += st.beats * spb;
    }
    starts.push(t);
    (ev, starts)
}

/// The triad (root, third or sus note, fifth) in a chord name, or a numeral in the key: 1-7 for the key's own triads
/// (1 major, 2 minor ... 7 diminished), or I, ii, bVII and the rest.
fn triad_of(tok: &str, key: i32) -> Option<(i32, [i32; 3])> {
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

/// A gospel move: its name, what it is, and its chords - (root above the key, triad, bass above the key, how it is
/// played, beats).
type MoveDef = (&'static str, &'static str, Vec<(i32, [i32; 3], Option<i32>, Art, f32)>);

fn gospel_moves() -> Vec<MoveDef> {
    const MA: [i32; 3] = [0, 4, 7];
    const MI: [i32; 3] = [0, 3, 7];
    const DI: [i32; 3] = [0, 3, 6];
    const S4: [i32; 3] = [0, 5, 7];
    const S2: [i32; 3] = [0, 2, 7];
    use Art::*;
    vec![
        (
            "CHURCH PLAGAL: I - IV/I - I",
            "The amen move. The IV triad over the I bass and back: the bass holds, the top two notes rise a step and come back.",
            vec![
                (0, MA, None, Strum, 2.0),
                (5, MA, Some(0), Strum, 2.0),
                (0, MA, None, Strum, 2.0),
                (5, MA, Some(0), Strum, 2.0),
                (0, MA, None, Strum, 4.0),
            ],
        ),
        (
            "HAMMER INTO THE 3RD",
            "Grab the sus chord, then hammer the 3rd on: Isus4 to I, IVsus2 to IV. Let the other strings ring.",
            vec![
                (0, S4, None, Strum, 1.0),
                (0, MA, None, Hammer, 1.0),
                (5, S2, None, Strum, 1.0),
                (5, MA, None, Hammer, 1.0),
                (0, S4, None, Strum, 1.0),
                (0, MA, None, Hammer, 3.0),
            ],
        ),
        (
            "SLIDE IN FROM A HALF STEP BELOW",
            "Hit the triad a fret low and slide it up into place. Works into any chord.",
            vec![
                (11, MA, None, Strum, 0.5),
                (0, MA, None, Slide, 1.5),
                (4, MA, None, Strum, 0.5),
                (5, MA, None, Slide, 1.5),
                (11, MA, None, Strum, 0.5),
                (0, MA, None, Slide, 3.5),
            ],
        ),
        (
            "PASSING DIMINISHED: I - #I\u{00b0} - ii - V - I",
            "A diminished triad a half step above the I walks you up into the ii.",
            vec![
                (0, MA, None, Strum, 2.0),
                (1, DI, None, Strum, 2.0),
                (2, MI, None, Strum, 2.0),
                (7, MA, None, Strum, 2.0),
                (0, MA, None, Strum, 4.0),
            ],
        ),
        (
            "IV - #IV\u{00b0} - I/V",
            "From the IV, a diminished a half step up leads into the I over its 5th, then home through the V.",
            vec![
                (5, MA, None, Strum, 2.0),
                (6, DI, None, Strum, 2.0),
                (0, MA, Some(7), Strum, 2.0),
                (7, MA, None, Strum, 2.0),
                (0, MA, None, Strum, 4.0),
            ],
        ),
        (
            "MINOR PLAGAL: IV - iv - I",
            "The IV turns minor before home: one note slips down a half step.",
            vec![(5, MA, None, Strum, 2.0), (5, MI, None, Strum, 2.0), (0, MA, None, Strum, 4.0)],
        ),
        (
            "6-5-4-3-2-1 BASS WALK",
            "The bass walks down the scale with a triad over each step: vi, I/5, IV, I/3, ii, then V and home.",
            vec![
                (9, MI, None, Strum, 2.0),
                (0, MA, Some(7), Strum, 2.0),
                (5, MA, None, Strum, 2.0),
                (0, MA, Some(4), Strum, 2.0),
                (2, MI, None, Strum, 2.0),
                (7, MA, None, Strum, 2.0),
                (0, MA, None, Strum, 4.0),
            ],
        ),
        (
            "3-6-2-5-1 WITH MAJOR TRIADS",
            "Each chord a major triad, so each one is the V of the next (secondary dominants): III, VI, II, V, I.",
            vec![
                (4, MA, None, Strum, 2.0),
                (9, MA, None, Strum, 2.0),
                (2, MA, None, Strum, 2.0),
                (7, MA, None, Strum, 2.0),
                (0, MA, None, Strum, 4.0),
            ],
        ),
        (
            "TRIAD PAIR OVER THE ROOT: IV/I - V/I",
            "Two triads a step apart, back and forth over the same bass. Lots of colour from two easy shapes.",
            vec![
                (5, MA, Some(0), Strum, 1.0),
                (7, MA, Some(0), Strum, 1.0),
                (5, MA, Some(0), Strum, 1.0),
                (7, MA, Some(0), Strum, 1.0),
                (0, MA, None, Strum, 4.0),
            ],
        ),
        (
            "bVII - IV - I",
            "Down from the flat seven through the IV to home: the backdoor, gospel and rock style.",
            vec![(10, MA, None, Strum, 2.0), (5, MA, None, Strum, 2.0), (0, MA, None, Strum, 4.0)],
        ),
        (
            "ii - bII - I",
            "The bII stands in for the V (a tritone away), so the top slides down by half steps into the I.",
            vec![(2, MI, None, Strum, 2.0), (1, MA, None, Strum, 2.0), (0, MA, None, Strum, 4.0)],
        ),
    ]
}

impl App {
    fn mv_lit(&mut self, ui: &egui::Ui) -> Option<usize> {
        let (t0, starts) = self.mv_play.as_ref()?;
        let el = t0.elapsed().as_secs_f32();
        if el > starts.last().copied().unwrap_or(0.0) + 0.5 {
            self.mv_play = None;
            return None;
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
        starts.windows(2).position(|w| el >= w[0] && el < w[1])
    }

    /// Play a move at the band's tempo, string by string (hammer-ons let the other strings ring).
    fn play_steps(&mut self, steps: &[Step]) {
        let spb = 60.0 / self.band_bpm().max(30.0);
        let (ev, starts) = steps_events(steps, spb);
        let total = starts.last().copied().unwrap_or(1.0);
        self.player.send(Cmd::StopOnce);
        self.player.send(Cmd::Once(crate::band::string_events(&ev, total + 1.8), 0.9));
        self.mv_play = Some((Instant::now(), starts));
    }

    fn key_and_set(&mut self, ui: &mut egui::Ui, four: bool) {
        ui.horizontal_wrapped(|ui| {
            if let Some(k) = dropdown(ui, "tr_key", "KEY", &KEYS, self.tr_root as usize, 110.0) {
                self.tr_root = k as i32;
            }
            ui.add_space(10.0);
            if four {
                let names: Vec<&str> = SETS4.iter().map(|s| s.0).collect();
                self.tr_set4 = self.tr_set4.min(SETS4.len() - 1);
                if let Some(i) = dropdown(ui, "tr_set4", "", &names, self.tr_set4, 170.0) {
                    self.tr_set4 = i;
                }
            } else {
                let names: Vec<&str> = SETS.iter().map(|s| s.0).collect();
                if let Some(i) = dropdown(ui, "tr_set", "", &names, self.tr_set.min(SETS.len() - 1), 190.0) {
                    self.tr_set = i;
                    self.tr_sel = 0;
                }
            }
        });
        ui.add_space(6.0);
    }

    /// The TRIADS tab.
    pub(crate) fn triads(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if let Some(v) = tab_row(ui, &["MAP", "VOICE LEADING", "OVER A BASS"], self.tr_page as usize) {
            self.tr_page = v as u8;
        }
        ui.add_space(6.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            match self.tr_page {
                0 => self.tr_map(ui, acts),
                1 => self.tr_lead(ui),
                _ => self.tr_bass(ui, acts),
            }
            ui.add_space(20.0);
        });
    }

    fn quality_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (i, (n, _)) in QUALITIES.iter().enumerate() {
                if retro_btn(ui, n, self.tr_q == i).clicked() {
                    self.tr_q = i;
                    self.tr_sel = 0;
                }
            }
        });
        ui.add_space(6.0);
    }

    /// MAP: every inversion of a triad on a set of strings, up the neck.
    fn tr_map(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "One triad, every inversion up the neck on the strings you pick. Root position, then the 3rd on the bottom, then the 5th, and round again an octave up. Learn them on each set of strings and you can grab any triad anywhere. Click a dot to pick a shape.", pal().ink2);
        self.key_and_set(ui, false);
        self.quality_row(ui);
        let root = self.tr_root;
        let q = QUALITIES[self.tr_q.min(5)].1;
        let pcs: Vec<i32> = q.iter().map(|x| (root + x).rem_euclid(12)).collect();
        let grips = grips_on(&pcs, SETS[self.tr_set.min(SETS.len() - 1)].1, false, 4);
        if grips.is_empty() {
            para(ui, "No shape for this on these strings in one hand-span.", pal().ink2);
            return;
        }
        self.tr_sel = self.tr_sel.min(grips.len() - 1);
        let label = |m: i32| {
            let (d, c) = degree(m, root);
            (d.to_string(), c)
        };
        if let Some(i) = fret_map(ui, &grips, self.tr_sel, &label, self.tr_whole.then_some(pcs.as_slice())) {
            self.tr_sel = i;
            acts.push(Action::PlayNotes(grip_notes(&grips[i])));
        }
        ui.horizontal_wrapped(|ui| {
            // (STOP while it plays)
            self.seq_button(
                ui,
                acts,
                "HEAR THEM UP THE NECK",
                "Every shape in turn, up the neck",
                "triads",
                || grips.iter().map(grip_notes).collect(),
                0.9,
            );
            if retro_btn(ui, "WHOLE NECK", self.tr_whole)
                .tip("Light every root, 3rd and 5th on every string, so you see how the shapes sit in the whole neck")
                .clicked()
            {
                self.tr_whole = !self.tr_whole;
                self.dirty = true;
            }
            ui.add_space(8.0);
            crate::views::label(ui, "R ROOT   3 THIRD   5 FIFTH", 200.0);
        });
        let sp = self.chord_sp;
        let mut pick = None;
        crate::App::card_grid(ui, |ui| {
            for (i, g) in grips.iter().enumerate() {
                let bass = grip_notes(g).iter().min().map(|m| (m - root).rem_euclid(12)).unwrap_or(0);
                let inv = if bass == 0 {
                    "ROOT POSITION"
                } else if bass == q[1] {
                    "3RD ON THE BOTTOM"
                } else {
                    "5TH ON THE BOTTOM"
                };
                let fr = g.iter().flatten().filter(|f| **f > 0).min().copied().unwrap_or(0);
                let name = format!("{} - FRET {}", inv, fr);
                let _ = sp;
                let tint = |pc: i32| Some(degree(pc, root).1);
                if self.voicing_card(ui, acts, g, root, &name, &tint, false, false) != crate::clusters::CardAct::None {
                    pick = Some(i);
                }
            }
        });
        if let Some(i) = pick {
            self.tr_sel = i;
        }
    }

    /// VOICE LEADING: changes played with triads on one set of strings, each the closest inversion to the last.
    fn tr_lead(&mut self, ui: &mut egui::Ui) {
        para(ui, "Type the changes (chord names, numerals like I IV V or ii, or the numbers 1-7 for the key's own triads). Each chord becomes the triad closest to the one before, on the same strings, so your hand barely moves. Blue notes move, black ones stay. The top notes make a melody of their own.", pal().ink2);
        self.key_and_set(ui, false);
        ui.horizontal_wrapped(|ui| {
            let names: Vec<&str> = POSITIONS.iter().map(|p| p.0).collect();
            if let Some(i) = dropdown(ui, "tr_pos", "START", &names, self.tr_pos as usize, 200.0) {
                self.tr_pos = i as u8;
            }
            if let Some(text) = self.chart_text() {
                if retro_btn(ui, "USE THE LEAD SHEET'S CHANGES", false).tip("The chords of the lead sheet, in order").clicked() {
                    let chart = crate::chart::parse_friendly(&text);
                    let order = chart.play_order();
                    let mut all: Vec<String> = order.iter().flat_map(|b| chart.bars[*b].chords.clone()).collect();
                    all.dedup();
                    self.tr_prog = all.join(" ");
                }
            }
        });
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width() - 6.0, bh()), Sense::hover());
        let _ = field(
            ui,
            &mut self.ed,
            crate::tools::F_TR_PROG,
            &mut self.tr_prog,
            r,
            "1 4 5 1   or   C Am Dm G7   or   I vi ii V",
            false,
        );
        ui.add_space(6.0);
        let key = self.tr_root;
        let set = SETS[self.tr_set.min(SETS.len() - 1)].1;
        let near = POSITIONS[(self.tr_pos as usize).min(2)].1;
        let mut steps: Vec<Step> = Vec::new();
        let mut prev: Option<[Option<u8>; 6]> = None;
        let mut bad: Vec<String> = Vec::new();
        for tok in self.tr_prog.split_whitespace() {
            let Some((root, q)) = triad_of(tok, key) else {
                bad.push(tok.to_string());
                continue;
            };
            let pcs: Vec<i32> = q.iter().map(|x| (root + x).rem_euclid(12)).collect();
            if let Some(g) = closest(&grips_on(&pcs, set, false, 4), prev.as_ref(), near) {
                steps.push(Step { grip: g, name: tok.to_string(), art: Art::Strum, beats: 2.0 });
                prev = Some(g);
            }
        }
        if !bad.is_empty() {
            crate::views::dim_line(ui, &format!("COULD NOT READ: {}", bad.join(" ")), 1.0, pal().red);
        }
        if steps.is_empty() {
            return;
        }
        let lit = self.mv_lit(ui);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "PLAY IT", lit.is_some()).tip("Two beats a chord at the band's tempo").clicked() {
                self.play_steps(&steps);
            }
            let tops: Vec<String> = steps
                .iter()
                .map(|s| {
                    grip_notes(&s.grip)
                        .iter()
                        .max()
                        .map(|m| theory::note_name(*m, self.chord_sp.sharps).to_string())
                        .unwrap_or_default()
                })
                .collect();
            ui.add_space(8.0);
            crate::views::label(ui, &format!("TOP NOTES: {}", tops.join(" ")), 200.0);
        });
        ui.add_space(4.0);
        if let Some(i) = steps_score(ui, &steps, self.chord_sp.sharps, lit) {
            self.player.send(Cmd::Once(crate::band::chord_sound(&grip_notes(&steps[i].grip)), 0.9));
        }
    }

    /// OVER A BASS: a triad over each of the twelve bass notes, and what chord it makes.
    fn tr_bass(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Put a triad over a different bass note and its notes turn into the colours of a bigger chord: D over C is a C lydian sound, Bb over C a C9sus, F over G the gospel IV-over-V. Pick a triad, then walk the bass under it.", pal().ink2);
        self.key_and_set(ui, false);
        self.quality_row(ui);
        let root = self.tr_root;
        let q = QUALITIES[self.tr_q.min(5)].1;
        let pcs: Vec<i32> = q.iter().map(|x| (root + x).rem_euclid(12)).collect();
        let sp = self.chord_sp;
        let sharps = sp.sharps;
        let w = ui.available_width().min(560.0);
        let (hr, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::hover());
        ptext(ui.painter(), Pos2::new(hr.min.x + 10.0, hr.center().y), Align::Min, "BASS", 1.0, pal().dim);
        ptext(ui.painter(), Pos2::new(hr.min.x + w * 0.2, hr.center().y), Align::Min, "MAKES", 1.0, pal().dim);
        for b in 0..12 {
            let mut all = pcs.clone();
            if !all.contains(&b) {
                all.push(b);
            }
            let name = theory::suggestions(&all, Some(b)).first().map(|s| s.name(sp)).unwrap_or_else(|| "-".to_string());
            let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 24.0), Sense::click());
            let p = ui.painter();
            if self.tr_bass == b {
                p.rect_filled(r, 2.0, pal().sel);
            } else if resp.hovered() {
                p.rect_filled(r, 2.0, pal().row_sel.gamma_multiply(0.5));
            }
            ptext(p, Pos2::new(r.min.x + 10.0, r.center().y), Align::Min, theory::note_name(b, sharps), 2.0, pal().ink);
            ptext(p, Pos2::new(r.min.x + w * 0.2, r.center().y), Align::Min, &name, 2.0, pal().ink);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.tr_bass = b;
            }
        }
        ui.add_space(8.0);
        let bass = self.tr_bass;
        section_header(
            ui,
            &format!(
                "{} {} OVER {}",
                theory::note_name(root, sharps),
                QUALITIES[self.tr_q.min(5)].0,
                theory::note_name(bass, sharps)
            ),
        );
        // the triad on the top strings, the bass on the 6th or 5th
        let mut grips: Vec<[Option<u8>; 6]> = Vec::new();
        for set in [&[3usize, 4, 5][..], &[2, 3, 4], &[1, 2, 3]] {
            for g in grips_on(&pcs, set, false, 4) {
                let gb = with_bass(&g, bass);
                let fretted: Vec<u8> = gb.iter().flatten().copied().filter(|f| *f > 0).collect();
                let span = fretted.iter().max().unwrap_or(&0) - fretted.iter().min().unwrap_or(&0);
                if gb != g && span <= 5 && !grips.contains(&gb) {
                    grips.push(gb);
                }
            }
        }
        grips.sort_by(|a, b| centre(a).partial_cmp(&centre(b)).unwrap_or(std::cmp::Ordering::Equal));
        if grips.is_empty() {
            para(ui, "No grip in one hand-span: play the bass on its own and the triad after it.", pal().ink2);
            return;
        }
        crate::App::card_grid(ui, |ui| {
            for g in grips.iter().take(6) {
                let name = grip_name(g, sp);
                let tint = |pc: i32| if pcs.contains(&pc) { Some(CL_TONE) } else { Some(CL_BASS) };
                self.voicing_card(ui, acts, g, bass, &name, &tint, false, false);
            }
        });
    }

    /// The MOVES tab (experimental).
    pub(crate) fn moves(&mut self, ui: &mut egui::Ui, _acts: &mut Vec<Action>) {
        if let Some(v) = tab_row(ui, &["GOSPEL", "QUARTAL & CLUSTERS"], self.mv_page as usize) {
            self.mv_page = v as u8;
        }
        ui.add_space(4.0);
        crate::views::dim_line(
            ui,
            "EXPERIMENTAL: HAMMER-ONS, SLIDES AND DIPS ARE MARKED, NOT EXACT - PLAY THEM YOUR WAY",
            1.0,
            pal().dim,
        );
        ui.add_space(4.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            match self.mv_page {
                0 => self.mv_gospel(ui),
                _ => self.mv_quartal(ui),
            }
            ui.add_space(20.0);
        });
    }

    fn mv_show(&mut self, ui: &mut egui::Ui, steps: &[Step]) {
        let lit = self.mv_lit(ui);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "PLAY IT", lit.is_some()).tip("At the band's tempo").clicked() {
                self.play_steps(steps);
            }
            ui.add_space(8.0);
            crate::views::label(ui, &format!("AT {} BPM", self.band_bpm()), 90.0);
        });
        ui.add_space(4.0);
        if let Some(i) = steps_score(ui, steps, self.chord_sp.sharps, lit) {
            self.player.send(Cmd::Once(crate::band::chord_sound(&grip_notes(&steps[i].grip)), 0.9));
        }
    }

    /// GOSPEL: common moves built from triads, in any key, voice-led on the strings picked.
    fn mv_gospel(&mut self, ui: &mut egui::Ui) {
        para(ui, "Common gospel moves, built from triads in any key and voiced close on the strings you pick. Learn each one in a few keys, then drop it into tunes.", pal().ink2);
        self.tr_set = self.tr_set.min(3);
        ui.horizontal_wrapped(|ui| {
            if let Some(k) = dropdown(ui, "mv_key", "KEY", &KEYS, self.tr_root as usize, 110.0) {
                self.tr_root = k as i32;
            }
            ui.add_space(10.0);
            let names: Vec<&str> = SETS[..4].iter().map(|s| s.0).collect();
            if let Some(i) = dropdown(ui, "mv_set", "", &names, self.tr_set, 170.0) {
                self.tr_set = i;
            }
            ui.add_space(10.0);
            let pos: Vec<&str> = POSITIONS.iter().map(|p| p.0).collect();
            if let Some(i) = dropdown(ui, "mv_pos", "START", &pos, self.tr_pos as usize, 200.0) {
                self.tr_pos = i as u8;
            }
            if retro_btn(ui, "WHAMMY DIP AT THE END", self.mv_dip).tip("Dip the bar on the last chord").clicked() {
                self.mv_dip = !self.mv_dip;
            }
        });
        ui.add_space(6.0);
        let moves = gospel_moves();
        let names: Vec<&str> = moves.iter().map(|m| m.0).collect();
        self.mv_move = self.mv_move.min(moves.len() - 1);
        if let Some(i) = dropdown(ui, "mv_move", "MOVE", &names, self.mv_move, 380.0) {
            self.mv_move = i;
        }
        ui.add_space(4.0);
        let (_, about, def) = &moves[self.mv_move];
        para(ui, about, pal().ink2);
        let key = self.tr_root;
        let set = SETS[self.tr_set].1;
        let near = POSITIONS[(self.tr_pos as usize).min(2)].1;
        let sp = self.chord_sp;
        let mut steps: Vec<Step> = Vec::new();
        let mut prev: Option<[Option<u8>; 6]> = None;
        for (k, (off, q, bass, art, beats)) in def.iter().enumerate() {
            let root = (key + off).rem_euclid(12);
            let pcs: Vec<i32> = q.iter().map(|x| (root + x).rem_euclid(12)).collect();
            let options = grips_on(&pcs, set, false, 4);
            // a slide comes from the same shape a fret down, so the slid-into chord sits right above it
            let g = match (art, prev) {
                (Art::Slide, Some(p)) => Some(shifted(&p, 1)).filter(|s| {
                    let mut a: Vec<i32> = grip_notes(s).iter().map(|m| m.rem_euclid(12)).collect();
                    let mut b = pcs.clone();
                    a.sort();
                    a.dedup();
                    b.sort();
                    a == b
                }),
                _ => None,
            }
            .or_else(|| closest(&options, prev.as_ref(), near));
            let Some(mut g) = g else { continue };
            // the chord a slide starts from: the next chord's shape, a fret down
            if let Some((_, nq, _, Art::Slide, _)) = def.get(k + 1) {
                let nroot = (key + def[k + 1].0).rem_euclid(12);
                let npcs: Vec<i32> = nq.iter().map(|x| (nroot + x).rem_euclid(12)).collect();
                if let Some(target) = closest(&grips_on(&npcs, set, false, 4), prev.as_ref(), near) {
                    g = shifted(&target, -1);
                }
            }
            prev = Some(g);
            if let Some(b) = bass {
                g = with_bass(&g, (key + b).rem_euclid(12));
            }
            let last = k + 1 == def.len();
            let art = if last && self.mv_dip { Art::Dip } else { *art };
            steps.push(Step { grip: g, name: grip_name(&g, sp), art, beats: *beats });
        }
        if !steps.is_empty() {
            self.mv_show(ui, &steps);
        }
    }

    /// QUARTAL & CLUSTERS: McCoy Tyner style stacks of 4ths (or of 2nds) built on each note of a mode and planed
    /// through it, side-slipped a half step out and back, over a held bass if you like.
    fn mv_quartal(&mut self, ui: &mut egui::Ui) {
        para(ui, "McCoy Tyner's sound: the root and 5th held low, chords built in 4ths (not 3rds) on top, moved up and down the mode in parallel, and slipped a half step out and back for tension. Stack 2nds instead and you get clusters that move the same way. On guitar the low strings are already tuned in 4ths - the open E A D G B is the famous So What chord.", pal().ink2);
        let parents: Vec<String> = theory::PARENT_MODES.iter().map(|p| p.0.to_uppercase()).collect();
        let parents: Vec<&str> = parents.iter().map(|s| s.as_str()).collect();
        ui.horizontal_wrapped(|ui| {
            if let Some(k) = dropdown(ui, "mq_key", "KEY", &KEYS, self.tr_root as usize, 110.0) {
                self.tr_root = k as i32;
            }
            ui.add_space(10.0);
            if let Some(i) = dropdown(ui, "mq_parent", "SCALE", &parents, self.mv_parent.min(3), 220.0) {
                self.mv_parent = i;
            }
        });
        let (_, scale, modes) = theory::PARENT_MODES[self.mv_parent.min(3)];
        let key = self.tr_root;
        let mode_names: Vec<String> = (0..7)
            .map(|m| format!("{} {}", theory::note_name(key + scale[m], self.chord_sp.sharps), modes[m]).to_uppercase())
            .collect();
        let mode_names: Vec<&str> = mode_names.iter().map(|s| s.as_str()).collect();
        ui.horizontal_wrapped(|ui| {
            if let Some(i) = dropdown(ui, "mq_mode", "MODE", &mode_names, self.mv_mode.min(6), 260.0) {
                self.mv_mode = i;
            }
        });
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, "STACK", 70.0);
            for (i, (n, tip)) in [
                ("4THS", "Three notes a 4th apart"),
                ("4THS + 3RD", "So What: two 4ths and a 3rd on top, four notes"),
                ("2NDS (CLUSTER)", "Three scale notes side by side"),
            ]
            .iter()
            .enumerate()
            {
                if retro_btn(ui, n, self.mv_stack == i as u8).tip(tip).clicked() {
                    self.mv_stack = i as u8;
                }
            }
            ui.add_space(12.0);
            crate::views::label(ui, "MOVE", 60.0);
            for (i, n) in ["PLANE UP AND BACK", "SIDE-SLIP"].iter().enumerate() {
                if retro_btn(ui, n, self.mv_motion == i as u8).clicked() {
                    self.mv_motion = i as u8;
                }
            }
            if retro_btn(ui, "BASS UNDER IT", self.mv_pedal).tip("The mode's root held low, McCoy's left hand").clicked() {
                self.mv_pedal = !self.mv_pedal;
            }
        });
        let four = self.mv_stack == 1;
        let set: &[usize] = if four {
            ui.horizontal_wrapped(|ui| {
                let names: Vec<&str> = SETS4.iter().map(|s| s.0).collect();
                if let Some(i) = dropdown(ui, "mq_set4", "", &names, self.tr_set4.min(2), 170.0) {
                    self.tr_set4 = i;
                }
            });
            SETS4[self.tr_set4.min(2)].1
        } else {
            self.tr_set = self.tr_set.min(3);
            ui.horizontal_wrapped(|ui| {
                let names: Vec<&str> = SETS[..4].iter().map(|s| s.0).collect();
                if let Some(i) = dropdown(ui, "mq_set", "", &names, self.tr_set, 170.0) {
                    self.tr_set = i;
                }
            });
            SETS[self.tr_set].1
        };
        ui.add_space(6.0);
        let m = self.mv_mode.min(6);
        let steps_rel = theory::mode_steps(&scale, m);
        let root = (key + scale[m]).rem_euclid(12);
        let pc_at = |d: usize| (root + steps_rel[d % 7]).rem_euclid(12);
        let stack = |d: usize| -> Vec<i32> {
            match self.mv_stack {
                0 => vec![pc_at(d), pc_at(d + 3), pc_at(d + 6)],
                1 => vec![pc_at(d), pc_at(d + 3), pc_at(d + 6), pc_at(d + 8)],
                _ => vec![pc_at(d), pc_at(d + 1), pc_at(d + 2)],
            }
        };
        let reach = if self.mv_stack == 2 { 5 } else { 4 };
        let sp = self.chord_sp;
        let pedal = self.mv_pedal && !set.contains(&0) && !set.contains(&1);
        let mut grips: Vec<[Option<u8>; 6]> = Vec::new();
        let mut prev: Option<[Option<u8>; 6]> = None;
        let degs: Vec<usize> = if self.mv_motion == 0 { vec![0, 1, 2, 3, 4, 5, 6, 7, 6, 5, 4, 3, 2, 1, 0] } else { vec![0] };
        for d in degs {
            if let Some(g) = closest(&grips_on(&stack(d), set, true, reach), prev.as_ref(), 5.0) {
                grips.push(g);
                prev = Some(g);
            }
        }
        if self.mv_motion == 1 {
            if let Some(g) = grips.first().copied() {
                grips = vec![g, shifted(&g, 1), g, shifted(&g, -1), g];
            }
        }
        if grips.is_empty() {
            para(ui, "No shape for this stack on these strings in one hand-span - try other strings.", pal().ink2);
            return;
        }
        let steps: Vec<Step> = grips
            .iter()
            .map(|g| {
                let g = if pedal { with_bass(g, root) } else { *g };
                Step { grip: g, name: grip_name(&g, sp), art: Art::Strum, beats: if self.mv_motion == 1 { 2.0 } else { 1.0 } }
            })
            .collect();
        self.mv_show(ui, &steps);
        if !pedal && self.mv_pedal {
            crate::views::dim_line(ui, "THE BASS NEEDS THE 6TH OR 5TH STRING FREE - PICK HIGHER STRINGS", 1.0, pal().dim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcs_of(g: &[Option<u8>; 6]) -> Vec<i32> {
        let mut v: Vec<i32> = grip_notes(g).iter().map(|m| m.rem_euclid(12)).collect();
        v.sort();
        v
    }

    /// A C triad on the top three strings: every inversion, each grip exactly C E G with the notes climbing.
    #[test]
    fn triad_inversions_on_top_strings() {
        let grips = grips_on(&[0, 4, 7], &[3, 4, 5], false, 4);
        assert!(grips.len() >= 3, "three inversions at least, got {}", grips.len());
        for g in &grips {
            assert_eq!(pcs_of(g), vec![0, 4, 7]);
            let n = grip_notes(g);
            assert!(n.windows(2).all(|w| w[1] > w[0]), "notes climb: {:?}", n);
        }
        // the open-position C on strings 3-2-1 (G C E: 0 1 0) is one of them
        assert!(grips.iter().any(|g| g[3] == Some(0) && g[4] == Some(1) && g[5] == Some(0)));
    }

    /// Stacks keep their order: D G C (4ths) on strings 4-3-2 comes out low to high in that order.
    #[test]
    fn quartal_stack_in_order() {
        let grips = grips_on(&[2, 7, 0], &[2, 3, 4], true, 4);
        assert!(!grips.is_empty());
        for g in &grips {
            let n = grip_notes(g);
            assert_eq!(n.iter().map(|m| m.rem_euclid(12)).collect::<Vec<_>>(), vec![2, 7, 0]);
        }
    }

    /// Voice leading picks the nearest shape: C to F on the top strings moves two notes and keeps the C.
    #[test]
    fn closest_keeps_common_tone() {
        let c = [None, None, None, Some(5), Some(5), Some(3)]; // C E G? (C on G string 5, E on B 5, G on e 3)
        let f = closest(&grips_on(&[5, 9, 0], &[3, 4, 5], false, 4), Some(&c), 5.0).unwrap();
        assert!(grip_notes(&f).contains(&(theory::STANDARD[3] + 5)), "the C stays: {:?}", f);
    }

    /// Numbers read as the key's own triads.
    #[test]
    fn numbers_are_diatonic() {
        assert_eq!(triad_of("2", 0), Some((2, [0, 3, 7])));
        assert_eq!(triad_of("7", 0), Some((11, [0, 3, 6])));
        assert_eq!(triad_of("4", 7), Some((0, [0, 4, 7])));
    }
}
