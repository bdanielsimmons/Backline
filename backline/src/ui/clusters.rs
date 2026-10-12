//! CLUSTERS (under CHORDS), after Robbie Barnby's "Cluster Voicings for Advanced Guitar": two notes a step apart -
//! a semitone (red) or a tone (blue) - sounded together over a bass. Ways in: the clusters each parent scale's
//! modes give you; one cluster held while the bass walks under it; one bass held while the cluster walks over it;
//! a tune's changes voiced with clusters; and your own, laid down on the neck and kept in MY CHORDS.
//! Every voicing is a card: the guitar chord box, the notes on a staff with the tab under them, a click to hear it.

use super::*;
use crate::chords::{chord_box_coloured, degree_in, grip_notes, neck_alpha, octave_keys, CL_BASS, CL_SEMI, CL_TONE};
use crate::theory;
use crate::views::field;

/// What a card's buttons asked for.
#[derive(PartialEq)]
pub(crate) enum CardAct {
    None,
    Edit,
    Remove,
}

/// How a card colours a note (by pitch class): a cluster note red or blue, the rest plain.
type Tint<'a> = &'a dyn Fn(i32) -> Option<Color32>;

/// The notes of a voicing that sit a step from a neighbour as they sound: red for a semitone, blue for a tone.
pub(crate) fn step_tint(notes: &[i32]) -> Vec<(i32, Color32)> {
    let mut sorted = notes.to_vec();
    sorted.sort();
    sorted.dedup();
    let mut out: Vec<(i32, Color32)> = Vec::new();
    for w in sorted.windows(2) {
        let col = match w[1] - w[0] {
            1 => CL_SEMI,
            2 => CL_TONE,
            _ => continue,
        };
        for m in [w[0], w[1]] {
            match out.iter_mut().find(|o| o.0 == m) {
                // a note in both kinds keeps the semitone's red
                Some(o) if col == CL_SEMI => o.1 = col,
                Some(_) => {}
                None => out.push((m, col)),
            }
        }
    }
    out
}

/// A chord written out: a treble staff (guitar sounds an octave below), the notes stacked (the upper of two a step
/// apart beside the other, as engravers do), sharps and flats in columns, and the tab under it when there is a grip.
pub(crate) fn chord_staff(
    p: &egui::Painter,
    rect: Rect,
    notes: &[i32],
    grip: Option<&[Option<u8>; 6]>,
    tint: &dyn Fn(i32) -> Option<Color32>,
    sharps: bool,
) {
    let paper = pal().beige_lt;
    p.rect_filled(rect, 3.0, paper);
    let g = 6.0_f32;
    let line = Stroke::new(1.0_f32, pal().ink2);
    let (left, right) = (rect.min.x + 6.0, rect.max.x - 6.0);
    let top = rect.min.y + 30.0;
    let bottom = top + 4.0 * g;
    for k in 0..5 {
        let y = top + k as f32 * g;
        p.line_segment([Pos2::new(left, y), Pos2::new(right, y)], line);
    }
    pixmap(
        p,
        Pos2::new(left + 3.0, top - 1.7 * g),
        7.4 * g / crate::lines_ui::CLEF.len() as f32,
        &crate::lines_ui::CLEF,
        pal().ink,
    );
    let x = rect.center().x + 16.0;
    chord_heads(p, x, bottom, g, notes, tint, sharps);
    let Some(grip) = grip else { return };
    let tg = 8.0_f32;
    let tab_top = bottom + 34.0;
    for s in 0..6 {
        let y = tab_top + s as f32 * tg;
        p.line_segment([Pos2::new(left, y), Pos2::new(right, y)], line);
    }
    for (k, ch) in ["T", "A", "B"].iter().enumerate() {
        ptext(p, Pos2::new(left + 8.0, tab_top + 6.0 + k as f32 * 14.0), Align::Center, ch, 1.0, pal().ink2);
    }
    for (s, f) in grip.iter().enumerate() {
        let Some(f) = f else { continue };
        let y = tab_top + (5 - s) as f32 * tg;
        let txt = f.to_string();
        let m = theory::STANDARD[s] + *f as i32;
        let col = tint(m.rem_euclid(12)).unwrap_or(pal().ink);
        p.rect_filled(Rect::from_center_size(Pos2::new(x, y), Vec2::new(text_w(&txt, 1.0) + 6.0, tg)), 0.0, paper);
        ptext(p, Pos2::new(x, y), Align::Center, &txt, 1.0, col);
    }
}

/// The notes of a chord stacked on a staff whose bottom line is at `bottom` (`g` between lines), at `x`: the upper
/// of two notes a step apart set beside the other, ledger lines, sharps and flats in columns.
pub(crate) fn chord_heads(
    p: &egui::Painter,
    x: f32,
    bottom: f32,
    g: f32,
    notes: &[i32],
    tint: &dyn Fn(i32) -> Option<Color32>,
    sharps: bool,
) {
    let line = Stroke::new(1.0_f32, pal().ink2);
    let mut sorted = notes.to_vec();
    sorted.sort();
    sorted.dedup();
    // (staff step, sharp or flat, the note)
    let spelled: Vec<(i32, i32, i32)> = sorted
        .iter()
        .map(|m| {
            let (letter, acc, oct) = crate::lines::spell(m + 12, sharps);
            (oct * 7 + letter, acc, *m)
        })
        .collect();
    let y_of = |step: i32| bottom - (step - 30) as f32 * g / 2.0;
    let mut side = vec![0.0_f32; spelled.len()];
    for i in 1..spelled.len() {
        if spelled[i].0 - spelled[i - 1].0 <= 1 && side[i - 1] == 0.0 {
            side[i] = g * 1.2;
        }
    }
    // ledger lines, wide enough for a note set beside another
    if let (Some(lo), Some(hi)) = (spelled.first(), spelled.last()) {
        let right_edge = x + g * 1.1 + if side.iter().any(|s| *s > 0.0) { g * 1.2 } else { 0.0 };
        let mut s = 28;
        while s >= lo.0 {
            p.line_segment([Pos2::new(x - g * 1.1, y_of(s)), Pos2::new(right_edge, y_of(s))], line);
            s -= 2;
        }
        let mut s = 40;
        while s <= hi.0 {
            p.line_segment([Pos2::new(x - g * 1.1, y_of(s)), Pos2::new(right_edge, y_of(s))], line);
            s += 2;
        }
    }
    let mut acc_cols: Vec<(i32, usize)> = Vec::new();
    for i in (0..spelled.len()).rev() {
        let (step, acc, m) = spelled[i];
        let col = tint(m.rem_euclid(12)).unwrap_or(pal().ink);
        let c = Pos2::new(x + side[i], y_of(step));
        p.circle_filled(c, g * 0.58, col);
        if col != pal().ink {
            p.circle_stroke(c, g * 0.58, Stroke::new(1.0_f32, pal().ink.gamma_multiply(0.6)));
        }
        if acc != 0 {
            // the next column left when another sign is too close above
            let mut k = 0;
            while acc_cols.iter().any(|(s, kk)| *kk == k && (s - step).abs() < 6) {
                k += 1;
            }
            acc_cols.push((step, k));
            crate::lines_ui::accidental(p, Pos2::new(x - g * 2.0 - k as f32 * g * 1.6, c.y), acc, g, col);
        }
    }
}

/// How tall chord_staff wants to be.
pub(crate) const STAFF_H: f32 = 136.0;

/// A small piano, two octaves from C3, with notes marked in their own colours and named.
pub(crate) fn piano_marked(ui: &mut egui::Ui, w: f32, marks: &[(i32, Color32)], sharps: bool) {
    let h = 64.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let p = ui.painter();
    const WHITE: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
    let ww = w / 14.0;
    let mark = |m: i32| marks.iter().find(|k| 48 + (k.0 - 48).rem_euclid(24) == m).map(|k| k.1);
    for k in 0..14 {
        let m = 48 + (k / 7) * 12 + WHITE[(k % 7) as usize];
        let r = Rect::from_min_size(Pos2::new(rect.min.x + k as f32 * ww, rect.min.y), Vec2::new(ww - 1.0, h));
        p.rect_filled(r, 2.0, mark(m).unwrap_or(Color32::from_gray(236)));
        p.rect_stroke(r, 2.0, Stroke::new(1.0_f32, Color32::from_gray(90)));
        if mark(m).is_some() {
            ptext(
                p,
                Pos2::new(r.center().x, r.max.y - 9.0),
                Align::Center,
                theory::note_name(m, sharps),
                1.0,
                Color32::from_gray(10),
            );
        }
    }
    for k in 0..14 {
        let pc = WHITE[(k % 7) as usize];
        if matches!(pc, 4 | 11) {
            continue;
        }
        let m = 48 + (k / 7) * 12 + pc + 1;
        let r = Rect::from_min_size(
            Pos2::new(rect.min.x + (k as f32 + 1.0) * ww - ww * 0.32, rect.min.y),
            Vec2::new(ww * 0.64, h * 0.58),
        );
        p.rect_filled(r, 2.0, mark(m).unwrap_or(Color32::from_gray(28)));
        if mark(m).is_some() {
            ptext(
                p,
                Pos2::new(r.center().x, r.max.y - 8.0),
                Align::Center,
                theory::note_name(m, sharps),
                1.0,
                Color32::from_gray(10),
            );
        }
    }
}

impl App {
    fn cl_gap(&self) -> i32 {
        if self.cl_semi {
            1
        } else {
            2
        }
    }

    fn cl_col(&self) -> Color32 {
        if self.cl_semi {
            CL_SEMI
        } else {
            CL_TONE
        }
    }

    /// The CLUSTERS page.
    pub(crate) fn clusters(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if let Some(v) =
            tab_row(ui, &["SCALES", "FIXED CLUSTER", "FIXED BASS", "IN A TUNE", "BUILD", "MY CHORDS"], self.cl_view as usize)
        {
            self.cl_view = v as u8;
        }
        ui.add_space(6.0);
        // the kind of cluster, for the lessons
        if self.cl_view < 4 {
            ui.horizontal_wrapped(|ui| {
                crate::views::label(ui, "CLUSTER", 90.0);
                if retro_btn(ui, "SEMITONE", self.cl_semi).tip("Two notes a half step apart: they rub (red)").clicked() {
                    self.cl_semi = true;
                }
                if retro_btn(ui, "TONE", !self.cl_semi).tip("Two notes a whole step apart: they shimmer (blue)").clicked() {
                    self.cl_semi = false;
                }
                ui.add_space(14.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, bh()), Sense::hover());
                ui.painter().circle_filled(r.center(), 6.0, CL_SEMI);
                crate::views::label(ui, "SEMITONE", 80.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, bh()), Sense::hover());
                ui.painter().circle_filled(r.center(), 6.0, CL_TONE);
                crate::views::label(ui, "TONE", 50.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, bh()), Sense::hover());
                ui.painter().circle_filled(r.center(), 6.0, CL_BASS);
                crate::views::label(ui, "BASS", 50.0);
            });
            ui.add_space(6.0);
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            match self.cl_view {
                0 => self.cl_scales(ui, acts),
                1 => self.cl_fixed_cluster(ui, acts),
                2 => self.cl_fixed_bass(ui, acts),
                3 => self.cl_tune(ui, acts),
                4 => self.cl_build(ui, acts),
                _ => self.cl_mine(ui, acts),
            }
            ui.add_space(20.0);
        });
    }

    /// One voicing as a card: its name, the chord box, the staff and tab, and HEAR / SAVE (and EDIT / X when asked).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn voicing_card(
        &mut self,
        ui: &mut egui::Ui,
        acts: &mut Vec<Action>,
        grip: &[Option<u8>; 6],
        bass: i32,
        name: &str,
        tint: Tint,
        edit: bool,
        remove: bool,
    ) -> CardAct {
        let notes = grip_notes(grip);
        let pcs: Vec<i32> = notes.iter().map(|m| m.rem_euclid(12)).collect();
        let sharps = self.chord_sp.sharps;
        let saved = self.store.my_chords.iter().position(|c| c.grip == *grip);
        let cw = 180.0 * font::ui_scale();
        let mut out = CardAct::None;
        let bg = ui.painter().add(egui::Shape::Noop);
        let resp = ui.allocate_ui_with_layout(Vec2::new(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(cw);
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
            let (nr, _) = ui.allocate_exact_size(Vec2::new(cw, 24.0), Sense::hover());
            ptext(ui.painter(), nr.center(), Align::Center, &font::fit(name, 2.0, cw - 12.0), 2.0, pal().ink);
            let (br, b) = ui.allocate_exact_size(Vec2::new(cw, 144.0), Sense::click());
            let box_r = Rect::from_center_size(br.center(), Vec2::new(122.0_f32.min(cw - 16.0), 140.0));
            chord_box_coloured(ui.painter(), box_r, grip, &|m| {
                let pc = m.rem_euclid(12);
                let col = tint(pc).unwrap_or(if pc == bass { CL_BASS } else { Color32::from_gray(245) });
                (degree_in(m, bass, &pcs).0.to_string(), col)
            });
            let (sr, s) = ui.allocate_exact_size(Vec2::new(cw, STAFF_H), Sense::click());
            chord_staff(ui.painter(), sr.shrink2(Vec2::new(6.0, 0.0)), &notes, Some(grip), tint, sharps);
            if b.union(s).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Hear it").clicked() {
                acts.push(Action::PlayNotes(notes.clone()));
            }
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                // in MY CHORDS a card has X instead of SAVE
                let n = 1 + edit as usize + 1;
                let bw = ((cw - 8.0 - 4.0 * n as f32) / n as f32).max(36.0);
                if retro_btn_w(ui, "HEAR", bw, false).clicked() {
                    acts.push(Action::PlayNotes(notes.clone()));
                }
                let tip = if saved.is_some() { "In MY CHORDS - click to take it out" } else { "Keep it in MY CHORDS" };
                if !remove
                    && retro_btn_w(ui, if saved.is_some() { "SAVED" } else { "SAVE" }, bw, saved.is_some()).tip(tip).clicked()
                {
                    match saved {
                        Some(i) => {
                            self.store.my_chords.remove(i);
                            self.set_note("TAKEN OUT OF MY CHORDS");
                        }
                        None => {
                            self.store.my_chords.push(crate::store::MyChord { name: name.to_string(), grip: *grip });
                            self.set_note("SAVED TO MY CHORDS");
                        }
                    }
                    self.store_dirty = true;
                }
                if edit && retro_btn_w(ui, "EDIT", bw, false).tip("Put it on the neck in BUILD to change it").clicked() {
                    out = CardAct::Edit;
                }
                if remove && retro_btn_w(ui, "X", bw, false).tip("Take it out of MY CHORDS").clicked() {
                    out = CardAct::Remove;
                }
            });
            ui.add_space(4.0);
        });
        let r = resp.response.rect.expand(4.0);
        ui.painter().set(bg, egui::Shape::rect_stroke(r, 4.0, Stroke::new(1.0_f32, pal().edge)));
        out
    }

    /// A wrapping grid of cards with room between them.
    pub(crate) fn card_grid(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(22.0, 22.0);
            ui.add_space(6.0);
            add(ui);
        });
        ui.add_space(10.0);
    }

    /// Up to `n` cards for one cluster over one bass, each named, the cluster in its colour.
    fn pair_cards(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, bass: i32, pair: [i32; 2], extras: &[i32], n: usize) {
        let grips = theory::cluster_voicings(bass, pair, extras, &theory::STANDARD);
        if grips.is_empty() {
            crate::views::dim_line(ui, "NO GRIP IN ONE HAND-SPAN FOR THIS ONE", 1.0, pal().dim);
            return;
        }
        let col = if (pair[1] - pair[0]).rem_euclid(12) == 1 { CL_SEMI } else { CL_TONE };
        let sp = self.chord_sp;
        Self::card_grid(ui, |ui| {
            for g in grips.iter().take(n) {
                let pcs: Vec<i32> = grip_notes(g).iter().map(|m| m.rem_euclid(12)).collect();
                let name = theory::suggestions(&pcs, Some(bass)).first().map(|s| s.name(sp)).unwrap_or_default();
                let tint = |pc: i32| pair.contains(&pc).then_some(col);
                self.voicing_card(ui, acts, g, bass, &name, &tint, false, false);
            }
        });
    }

    /// The scales that hold these notes over this bass, one line per parent scale (X when none of its modes do).
    fn scale_options(&self, ui: &mut egui::Ui, bass: i32, pcs: &[i32]) {
        let sharps = self.chord_sp.sharps;
        let n = theory::note_name(bass, sharps);
        let (parents, others) = theory::modes_holding(bass, pcs);
        for (parent, modes) in &parents {
            let list = if modes.is_empty() {
                "X".to_string()
            } else {
                modes.iter().map(|m| format!("{} {}", n, m)).collect::<Vec<_>>().join(", ")
            };
            ui.horizontal_wrapped(|ui| {
                crate::views::label(ui, &parent.to_uppercase(), 170.0);
                crate::views::label(ui, &list.to_uppercase(), 300.0);
            });
        }
        if !others.is_empty() {
            ui.horizontal_wrapped(|ui| {
                crate::views::label(ui, "OTHER", 170.0);
                crate::views::label(
                    ui,
                    &others.iter().map(|m| format!("{} {}", n, m)).collect::<Vec<_>>().join(", ").to_uppercase(),
                    300.0,
                );
            });
        }
    }

    /// A note picker: the label, a step down, an octave of keys, a step up, and the note's name.
    fn note_picker(
        &self,
        ui: &mut egui::Ui,
        id: &str,
        label: &str,
        cur: i32,
        marks: &[(i32, Color32)],
        faded: &[i32],
    ) -> Option<i32> {
        let mut out = None;
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, label, 150.0);
            if retro_btn_w(ui, "<", 30.0, false).tip("A half step down").clicked() {
                out = Some((cur + 11) % 12);
            }
            if let Some(pc) = octave_keys(ui, id, marks, faded) {
                out = Some(pc);
            }
            if retro_btn_w(ui, ">", 30.0, false).tip("A half step up").clicked() {
                out = Some((cur + 1) % 12);
            }
            ui.add_space(8.0);
            let names: Vec<String> = marks.iter().map(|m| theory::note_name(m.0, self.chord_sp.sharps).to_string()).collect();
            crate::views::label(ui, &names.join(" + "), 80.0);
        });
        ui.add_space(6.0);
        out
    }

    /// The twelve steps of a walk as a table: what the notes are over each, how many modes hold them, the chord.
    /// A click picks that row.
    fn walk_table(ui: &mut egui::Ui, head: [&str; 4], rows: &[(String, String, usize, String)], sel: usize) -> Option<usize> {
        let w = ui.available_width().min(640.0);
        let cols = [10.0, w * 0.16, w * 0.52, w * 0.68];
        let (hr, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::hover());
        for (k, h) in head.iter().enumerate() {
            ptext(ui.painter(), Pos2::new(hr.min.x + cols[k], hr.center().y), Align::Min, h, 1.0, pal().dim);
        }
        let mut hit = None;
        for (i, (a, b, fits, d)) in rows.iter().enumerate() {
            let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 24.0), Sense::click());
            let p = ui.painter();
            if i == sel {
                p.rect_filled(r, 2.0, pal().sel);
            } else if resp.hovered() {
                p.rect_filled(r, 2.0, pal().row_sel.gamma_multiply(0.5));
            }
            let col = if *fits == 0 { pal().dim } else { pal().ink };
            ptext(p, Pos2::new(r.min.x + cols[0], r.center().y), Align::Min, a, 2.0, col);
            ptext(
                p,
                Pos2::new(r.min.x + cols[1], r.center().y),
                Align::Min,
                &font::fit(b, 2.0, cols[2] - cols[1] - 8.0),
                2.0,
                col,
            );
            let fit_txt = if *fits == 0 { "X".to_string() } else { format!("{} MODES", fits) };
            ptext(p, Pos2::new(r.min.x + cols[2], r.center().y), Align::Min, &fit_txt, 2.0, col);
            ptext(p, Pos2::new(r.min.x + cols[3], r.center().y), Align::Min, &font::fit(d, 2.0, w - cols[3] - 8.0), 2.0, col);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                hit = Some(i);
            }
        }
        ui.add_space(8.0);
        hit
    }

    /// How many modes (all parent scales, and the others) hold these notes over this bass.
    fn fits(bass: i32, pcs: &[i32]) -> usize {
        let (parents, others) = theory::modes_holding(bass, pcs);
        parents.iter().map(|p| p.1.len()).sum::<usize>() + others.len()
    }

    /// The chord a cluster makes over a bass, with the notes its likeliest mode adds.
    fn pair_chord(&self, bass: i32, pair: [i32; 2]) -> String {
        let mut pcs = vec![bass, pair[0], pair[1]];
        pcs.extend(theory::cluster_extras(bass, pair));
        theory::suggestions(&pcs, Some(bass)).first().map(|s| s.name(self.chord_sp)).unwrap_or_default()
    }

    /// The cluster over the bass, explained: what the two notes are, on the piano, which scales hold them.
    fn pair_summary(&self, ui: &mut egui::Ui, bass: i32, pair: [i32; 2]) {
        let sharps = self.chord_sp.sharps;
        let n = |p: i32| theory::note_name(p, sharps);
        let word = |p: i32| theory::degree_word(p, bass).split(' ').next().unwrap_or("").to_uppercase();
        section_header(
            ui,
            &format!("{} + {} OVER {}  =  {} + {}", n(pair[0]), n(pair[1]), n(bass), word(pair[0]), word(pair[1])),
        );
        ui.add_space(6.0);
        let col = self.cl_col();
        // each in its own place: the bass in the low octave, the cluster in the high one, so walking one leaves the
        // other where it is
        let mut lo = 60 + pair[0].rem_euclid(12);
        let span = (pair[1] - pair[0]).rem_euclid(12);
        if lo + span > 71 {
            lo -= 12;
        }
        let marks: Vec<(i32, Color32)> = vec![(48 + bass.rem_euclid(12), CL_BASS), (lo, col), (lo + span, col)];
        piano_marked(ui, ui.available_width().min(420.0), &marks, sharps);
        ui.add_space(8.0);
        if Self::fits(bass, &pair) == 0 {
            para(
                ui,
                &format!(
                    "No scale holds {} and {} over {}: as the {} and {} of a chord on {} they rub against it rather than colour it. It can still be played as a passing sound, but no chord name fits it.",
                    n(pair[0]),
                    n(pair[1]),
                    n(bass),
                    word(pair[0]).to_lowercase(),
                    word(pair[1]).to_lowercase(),
                    n(bass)
                ),
                pal().ink2,
            );
            return;
        }
        crate::views::dim_line(ui, "SCALES THAT HOLD IT", 1.0, pal().dim);
        self.scale_options(ui, bass, &pair);
    }

    /// SCALES: a parent scale written out with its clusters named, then one mode at a time with its grips.
    fn cl_scales(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Every scale has notes a step apart. Those pairs are its clusters. All seven modes of a scale share the same pairs, so the same two notes colour seven different chords, one for each mode's root. Pick a scale, then a mode.", pal().ink2);
        let sharps = self.chord_sp.sharps;
        const KEYS: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];
        let parents: Vec<String> = theory::PARENT_MODES.iter().map(|p| p.0.to_uppercase()).collect();
        let parents: Vec<&str> = parents.iter().map(|s| s.as_str()).collect();
        ui.horizontal_wrapped(|ui| {
            if let Some(k) = dropdown(ui, "cl_key", "KEY", &KEYS, self.cl_key as usize, 110.0) {
                self.cl_key = k as i32;
            }
            ui.add_space(10.0);
            if let Some(s) = dropdown(ui, "cl_parent", "SCALE", &parents, self.cl_scale.min(3), 220.0) {
                self.cl_scale = s;
            }
        });
        ui.add_space(6.0);
        let (pname, scale, modes) = theory::PARENT_MODES[self.cl_scale.min(3)];
        let key = self.cl_key;
        let step = self.cl_gap();
        let col = self.cl_col();
        // the scale on the staff, with its tab
        let notes: Vec<i32> = (0..8).map(|d| 48 + key + scale[d % 7] + if d == 7 { 12 } else { 0 }).collect();
        crate::lines_ui::line_score(ui, acts, &notes, 2, sharps, None, None);
        let pairs: Vec<String> = theory::pairs_in(&scale, step)
            .iter()
            .map(|d| {
                format!("{}-{}", theory::note_name(key + scale[*d], sharps), theory::note_name(key + scale[(*d + 1) % 7], sharps))
            })
            .collect();
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, if self.cl_semi { "ITS SEMITONES" } else { "ITS TONES" }, 150.0);
            for pr in &pairs {
                let w = text_w(pr, 2.0) + 18.0;
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                ui.painter().rect_filled(r.shrink(1.0), 3.0, col.gamma_multiply(0.85));
                ptext(ui.painter(), r.center(), Align::Center, pr, 2.0, Color32::from_gray(10));
                ui.add_space(4.0);
            }
        });
        ui.add_space(8.0);
        // the modes: pick one
        let root_of = |m: usize| (key + scale[m]).rem_euclid(12);
        let names: Vec<String> =
            (0..7).map(|m| format!("{} {}", theory::note_name(root_of(m), sharps), modes[m]).to_uppercase()).collect();
        let names: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let m = self.cl_mode.min(6);
        ui.horizontal_wrapped(|ui| {
            if let Some(i) = dropdown(ui, "cl_mode", "MODE", &names, m, 260.0) {
                self.cl_mode = i;
            }
            ui.add_space(10.0);
            // the first grip of every mode, in turn
            let mut seq: Vec<Vec<i32>> = Vec::new();
            for k in 0..7 {
                let root = root_of(k);
                let steps = theory::mode_steps(&scale, k);
                let ex: Vec<i32> = [2usize, 6, 4].iter().map(|j| (root + steps[*j]).rem_euclid(12)).collect();
                if let Some(pair) = theory::mode_clusters(&scale, k, root, step).first() {
                    let ex: Vec<i32> = ex.iter().copied().filter(|e| !pair.contains(e)).collect();
                    if let Some(g) = theory::cluster_voicings(root, *pair, &ex, &theory::STANDARD).first() {
                        seq.push(grip_notes(g));
                    }
                }
            }
            if retro_btn(ui, "PLAY THROUGH THE MODES", false).tip("One grip from each mode in turn").clicked() && !seq.is_empty()
            {
                acts.push(Action::PlaySequence(seq));
            }
        });
        let root = root_of(m);
        let steps = theory::mode_steps(&scale, m);
        let tetrad: Vec<i32> = [0, 2, 4, 6].iter().map(|k| (root + steps[*k]).rem_euclid(12)).collect();
        let chord = theory::suggestions(&tetrad, Some(root)).first().map(|s| s.name(self.chord_sp)).unwrap_or_default();
        section_header(
            ui,
            &format!(
                "{} {}  ({} {})   -   {}",
                theory::note_name(root, sharps),
                modes[m].to_uppercase(),
                theory::note_name(key, sharps),
                pname.to_uppercase(),
                chord
            ),
        );
        let extras: Vec<i32> = [2usize, 6, 4].iter().map(|k| (root + steps[*k]).rem_euclid(12)).collect();
        let pairs = theory::mode_clusters(&scale, m, root, step);
        if pairs.is_empty() {
            para(ui, "This mode has no cluster of this kind. Try the other kind.", pal().ink2);
        }
        for pair in pairs.into_iter().take(2) {
            let word = |p: i32| theory::degree_word(p, root).split(' ').next().unwrap_or("").to_uppercase();
            ui.add_space(4.0);
            crate::views::dim_line(
                ui,
                &format!(
                    "{} + {}  ({} + {})",
                    theory::note_name(pair[0], sharps),
                    theory::note_name(pair[1], sharps),
                    word(pair[0]),
                    word(pair[1])
                ),
                2.0,
                pal().ink,
            );
            let ex: Vec<i32> = extras.iter().copied().filter(|e| !pair.contains(e)).collect();
            self.pair_cards(ui, acts, root, pair, &ex, 3);
        }
    }

    /// FIXED CLUSTER: one cluster held while the bass walks under it.
    fn cl_fixed_cluster(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Hold two notes and walk the bass under them, a half step at a time. Each bass makes the same two notes different degrees, so each one is a different chord. X: no scale holds them over that bass.", pal().ink2);
        let sharps = self.chord_sp.sharps;
        let col = self.cl_col();
        let pair = [self.cl_low, (self.cl_low + self.cl_gap()).rem_euclid(12)];
        if let Some(pc) = self.note_picker(ui, "cl_top", "YOUR CLUSTER", pair[0], &[(pair[0], col), (pair[1], col)], &[]) {
            self.cl_low = pc;
        }
        let pair = [self.cl_low, (self.cl_low + self.cl_gap()).rem_euclid(12)];
        // the bass, on its own keyboard: faded keys are basses no scale holds the cluster over
        let no_fit: Vec<i32> = (0..12).filter(|b| Self::fits(*b, &pair) == 0).collect();
        if let Some(pc) = self.note_picker(ui, "cl_fc_bass", "THE BASS", self.fc_bass, &[(self.fc_bass, CL_BASS)], &no_fit) {
            self.fc_bass = pc;
        }
        crate::views::dim_line(ui, "FADED KEYS: NO SCALE HOLDS THE CLUSTER OVER THAT BASS", 1.0, pal().dim);
        ui.add_space(4.0);
        // the walk: every bass under it
        let word = |p: i32, b: i32| theory::degree_word(p, b).split(' ').next().unwrap_or("").to_uppercase();
        let rows: Vec<(String, String, usize, String)> = (0..12)
            .map(|k| {
                let b = (pair[0] + 1 - k).rem_euclid(12);
                let fits = Self::fits(b, &pair);
                let chord = if fits == 0 { String::new() } else { self.pair_chord(b, pair) };
                (theory::note_name(b, sharps).to_string(), format!("{} + {}", word(pair[0], b), word(pair[1], b)), fits, chord)
            })
            .collect();
        let sel = (0..12).find(|k| (pair[0] + 1 - k).rem_euclid(12) == self.fc_bass).unwrap_or(0) as usize;
        if let Some(k) = Self::walk_table(ui, ["BASS", "THE CLUSTER IS", "FITS", "MAKES"], &rows, sel) {
            self.fc_bass = (pair[0] + 1 - k as i32).rem_euclid(12);
        }
        let bass = self.fc_bass;
        self.pair_summary(ui, bass, pair);
        if Self::fits(bass, &pair) > 0 {
            section_header(ui, "VOICINGS");
            let extras = theory::cluster_extras(bass, pair);
            self.pair_cards(ui, acts, bass, pair, &extras, 6);
        }
    }

    /// FIXED BASS: one bass held while the cluster walks over it.
    fn cl_fixed_bass(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Hold the bass and walk the two notes up over it, a half step at a time. Each spot is a different pair of degrees over the same root, so a different chord type.", pal().ink2);
        let sharps = self.chord_sp.sharps;
        let gap = self.cl_gap();
        if let Some(pc) = self.note_picker(ui, "cl_bass2", "THE BASS", self.cl_bass, &[(self.cl_bass, CL_BASS)], &[]) {
            self.cl_bass = pc;
        }
        let bass = self.cl_bass;
        let col = self.cl_col();
        let fb = [self.fb_low, (self.fb_low + gap).rem_euclid(12)];
        let no_fit: Vec<i32> = (0..12).filter(|lo| Self::fits(bass, &[*lo, (lo + gap).rem_euclid(12)]) == 0).collect();
        if let Some(pc) = self.note_picker(ui, "cl_fb_top", "THE CLUSTER", fb[0], &[(fb[0], col), (fb[1], col)], &no_fit) {
            self.fb_low = pc;
        }
        crate::views::dim_line(ui, "FADED KEYS: NO SCALE HOLDS A CLUSTER FROM THERE OVER THIS BASS", 1.0, pal().dim);
        ui.add_space(4.0);
        let word = |p: i32| theory::degree_word(p, bass).split(' ').next().unwrap_or("").to_uppercase();
        let rows: Vec<(String, String, usize, String)> = (0..12)
            .map(|k| {
                let lo = (bass - 1 + k).rem_euclid(12);
                let pair = [lo, (lo + gap).rem_euclid(12)];
                let fits = Self::fits(bass, &pair);
                let chord = if fits == 0 { String::new() } else { self.pair_chord(bass, pair) };
                (
                    format!("{}+{}", theory::note_name(pair[0], sharps), theory::note_name(pair[1], sharps)),
                    format!("{} + {}", word(pair[0]), word(pair[1])),
                    fits,
                    chord,
                )
            })
            .collect();
        let sel = (0..12).find(|k| (bass - 1 + k).rem_euclid(12) == self.fb_low).unwrap_or(0) as usize;
        if let Some(k) = Self::walk_table(ui, ["CLUSTER", "OVER THE BASS IT IS", "FITS", "MAKES"], &rows, sel) {
            self.fb_low = (bass - 1 + k as i32).rem_euclid(12);
        }
        let pair = [self.fb_low, (self.fb_low + gap).rem_euclid(12)];
        self.pair_summary(ui, bass, pair);
        if Self::fits(bass, &pair) > 0 {
            section_header(ui, "VOICINGS");
            let extras = theory::cluster_extras(bass, pair);
            self.pair_cards(ui, acts, bass, pair, &extras, 6);
        }
    }

    /// IN A TUNE: the changes of the open tune (or typed ones), each chord with the modes it can come from and the
    /// clusters each gives - and played through in time.
    fn cl_tune(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Clusters in a real tune. Pick a chord: it shows the modes it can come from and the clusters each one gives. A chord can come from more than one scale (Dm7b5 is D Locrian from Eb major, or D Locrian Nat 9 from F melodic minor), and each gives different clusters.", pal().ink2);
        let sharps = self.chord_sp.sharps;
        let (title, changes) = self.practice_changes(ui);
        if changes.is_empty() {
            return;
        }
        let mut unique: Vec<String> = Vec::new();
        for c in &changes {
            if !unique.contains(c) {
                unique.push(c.clone());
            }
        }
        let bpm = self.band_bpm();
        let step = self.cl_gap();
        let first_grip = |c: &str| -> Option<Vec<i32>> {
            let cn = theory::chord_notes(c)?;
            let (pi, m, proot) = *theory::chord_sources(&cn).first()?;
            let scale = theory::PARENT_MODES[pi].1;
            let pair = *theory::mode_clusters(&scale, m, (proot + scale[m]).rem_euclid(12), step).first()?;
            let ex: Vec<i32> = cn.pcs.iter().copied().filter(|p| !pair.contains(p) && *p != cn.root).collect();
            theory::cluster_voicings(cn.bass, pair, &ex, &theory::STANDARD).first().map(grip_notes)
        };
        let comp: Vec<(String, Vec<i32>)> = changes.iter().filter_map(|c| first_grip(c).map(|g| (c.clone(), g))).collect();
        let sounding = self.seq_now("cl").and_then(|i| comp.get(i)).map(|c| c.0.clone());
        title_line(ui, &title.to_uppercase(), 2.5, pal().ink);
        ui.horizontal_wrapped(|ui| {
            self.seq_button(
                ui,
                acts,
                "COMP THROUGH THE CHANGES",
                "Every chord in turn with a cluster voicing, two beats each at the band's tempo",
                "cl",
                || comp.iter().map(|c| c.1.clone()).collect(),
                120.0 / bpm,
            );
            ui.add_space(8.0);
            match &sounding {
                Some(c) => crate::lcd_box(ui, c, 120.0, pal().ink),
                None => crate::views::label(ui, &format!("AT {} BPM", bpm), 90.0),
            }
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            for (i, c) in unique.iter().enumerate() {
                if retro_btn(ui, c, self.cl_chord == i || sounding.as_deref() == Some(c.as_str())).clicked() {
                    self.cl_chord = i;
                }
            }
        });
        let Some(chord) = unique.get(self.cl_chord.min(unique.len() - 1)).cloned() else { return };
        let Some(cn) = theory::chord_notes(&chord) else {
            para(ui, "That chord could not be read.", pal().ink2);
            return;
        };
        let sources = theory::chord_sources(&cn);
        if sources.is_empty() {
            para(ui, "No mode of these scales holds every note of this chord.", pal().ink2);
        }
        for (pi, m, proot) in sources.into_iter().take(3) {
            let (pname, scale, modes) = theory::PARENT_MODES[pi];
            let root = (proot + scale[m]).rem_euclid(12);
            section_header(
                ui,
                &format!(
                    "{}  -  {} {}  ({} {})",
                    chord,
                    theory::note_name(root, sharps),
                    modes[m].to_uppercase(),
                    theory::note_name(proot, sharps),
                    pname.to_uppercase()
                ),
            );
            let pairs = theory::mode_clusters(&scale, m, root, step);
            if pairs.is_empty() {
                crate::views::dim_line(ui, "THIS MODE HAS NO CLUSTER OF THIS KIND - TRY THE OTHER", 1.0, pal().dim);
            }
            for pair in pairs.into_iter().take(2) {
                let ex: Vec<i32> = cn.pcs.iter().copied().filter(|p| !pair.contains(p) && *p != cn.root).collect();
                self.pair_cards(ui, acts, cn.bass, pair, &ex, 3);
            }
        }
    }

    /// BUILD: lay a voicing down on the neck. It is named, its clusters coloured, written out, and can be kept.
    /// Below, a start: pick a cluster shape (semitone and tone steps) and a bass, and take one of its grips to the neck.
    fn cl_build(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Make your own. Click frets on the neck (left of the nut is the open string, click again to lift it). Notes a step apart light up as clusters: red a semitone, blue a tone. Save the ones you like to MY CHORDS.", pal().ink2);
        let sharps = self.chord_sp.sharps;
        let notes = grip_notes(&self.cb_frets);
        let pcs: Vec<i32> = notes.iter().map(|m| m.rem_euclid(12)).collect();
        let bass = notes.iter().min().map(|m| m.rem_euclid(12));
        let name =
            bass.and_then(|b| theory::suggestions(&pcs, Some(b)).first().map(|s| s.name(self.chord_sp))).unwrap_or_default();
        let w = ui.available_width();
        let zoom = self.neck_zoom;
        let (nr, _) = ui.allocate_exact_size(Vec2::new(w, 132.0 * zoom), Sense::hover());
        let neck_rect = Rect::from_min_max(nr.min + Vec2::new(4.0, 6.0), Pos2::new(nr.max.x - 4.0, nr.max.y - 18.0));
        let shape = self.cb_frets;
        let alpha = self.ghost_alpha;
        if let Some((s, f)) = neck_alpha(ui, neck_rect, &shape, 15, true, "cb_neck", bass, &[], false, alpha) {
            self.cb_frets[s] = f;
            if let Some(f) = f {
                acts.push(Action::PlayNotes(vec![theory::STANDARD[s] + f as i32]));
            }
        }
        ui.add_space(6.0);
        let tint_list = step_tint(&notes);
        let tint_pcs: Vec<(i32, Color32)> = tint_list.iter().map(|(m, c)| (m.rem_euclid(12), *c)).collect();
        let tint = |pc: i32| tint_pcs.iter().find(|t| t.0 == pc).map(|t| t.1);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "HEAR", false).clicked() && !notes.is_empty() {
                acts.push(Action::PlayNotes(notes.clone()));
            }
            if retro_btn(ui, "CLEAR", false).clicked() {
                self.cb_frets = [None; 6];
                self.cb_name.clear();
            }
            ui.add_space(12.0);
            crate::views::label(ui, "NAME", 50.0);
            let (r, _) = ui.allocate_exact_size(Vec2::new(200.0, bh()), Sense::hover());
            let hint = if name.is_empty() { "your name for it" } else { name.as_str() };
            let _ = field(ui, &mut self.ed, crate::tools::F_CB_NAME, &mut self.cb_name, r, hint, false);
            let saved = self.store.my_chords.iter().any(|c| c.grip == self.cb_frets);
            if retro_btn(ui, if saved { "SAVED" } else { "SAVE TO MY CHORDS" }, saved).clicked() && notes.len() >= 2 && !saved {
                let n = if self.cb_name.trim().is_empty() { name.clone() } else { self.cb_name.trim().to_string() };
                self.store.my_chords.push(crate::store::MyChord { name: n, grip: self.cb_frets });
                self.store_dirty = true;
                self.set_note("SAVED TO MY CHORDS");
            }
        });
        if notes.len() >= 2 {
            let b = bass.unwrap_or(0);
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                // the written-out chord
                let (sr, _) = ui.allocate_exact_size(Vec2::new(190.0, STAFF_H), Sense::hover());
                chord_staff(ui.painter(), sr, &notes, Some(&self.cb_frets), &tint, sharps);
                ui.add_space(18.0);
                ui.vertical(|ui| {
                    title_line(ui, if name.is_empty() { "?" } else { &name }, 3.0, pal().ink);
                    let mut sorted = notes.clone();
                    sorted.sort();
                    let pairs: Vec<String> = sorted
                        .windows(2)
                        .filter(|w| matches!(w[1] - w[0], 1 | 2))
                        .map(|w| {
                            format!(
                                "{}-{} {}",
                                theory::note_name(w[0], sharps),
                                theory::note_name(w[1], sharps),
                                if w[1] - w[0] == 1 { "SEMITONE" } else { "TONE" }
                            )
                        })
                        .collect();
                    crate::views::label(
                        ui,
                        &if pairs.is_empty() {
                            "NO CLUSTER YET: PUT TWO NOTES A STEP APART ON NEIGHBOURING STRINGS".to_string()
                        } else {
                            format!("CLUSTERS: {}", pairs.join("   "))
                        },
                        300.0,
                    );
                    ui.add_space(6.0);
                    let marks: Vec<(i32, Color32)> = notes
                        .iter()
                        .map(|m| {
                            (
                                *m,
                                tint(m.rem_euclid(12)).unwrap_or(if m.rem_euclid(12) == b {
                                    CL_BASS
                                } else {
                                    Color32::from_gray(150)
                                }),
                            )
                        })
                        .collect();
                    piano_marked(ui, ui.available_width().min(380.0), &marks, sharps);
                });
            });
            ui.add_space(8.0);
            crate::views::dim_line(ui, "SCALES THAT HOLD IT", 1.0, pal().dim);
            let mut set = pcs.clone();
            set.sort();
            set.dedup();
            set.retain(|p| *p != b);
            self.scale_options(ui, b, &set);
        }

        // ---- a start: a cluster shape over a bass, and its grips
        section_header(ui, "NEED A START? PICK A SHAPE");
        para(ui, "A cluster is notes stacked a step apart: H a semitone (bite), W a tone (air). Pick how many notes and the steps, where it starts and the bass. Then take a grip to the neck with EDIT.", pal().ink2);
        let size = self.cb_size.clamp(2, 5) as usize;
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, "NOTES", 150.0);
            for n in 2u8..=5 {
                if retro_btn_w(ui, &n.to_string(), 36.0, size == n as usize).clicked() {
                    self.cb_size = n;
                }
            }
            ui.add_space(16.0);
            crate::views::label(ui, "STEPS", 60.0);
            for k in 0..size - 1 {
                let semi = self.cb_steps[k];
                if retro_btn_w(ui, if semi { "H" } else { "W" }, 36.0, semi)
                    .tip("Click to switch: H a semitone, W a tone")
                    .clicked()
                {
                    self.cb_steps[k] = !semi;
                }
            }
        });
        ui.add_space(6.0);
        let mut shape_notes = vec![self.cb_lo];
        for k in 0..size - 1 {
            let last = *shape_notes.last().unwrap_or(&0);
            shape_notes.push(last + if self.cb_steps[k] { 1 } else { 2 });
        }
        let spcs: Vec<i32> = shape_notes.iter().map(|n| n.rem_euclid(12)).collect();
        let marks: Vec<(i32, Color32)> = shape_notes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                (
                    n.rem_euclid(12),
                    if i > 0 && self.cb_steps[i - 1] || i + 1 < size && self.cb_steps[i] { CL_SEMI } else { CL_TONE },
                )
            })
            .collect();
        if let Some(pc) = self.note_picker(ui, "cb_top", "STARTS ON", self.cb_lo, &marks, &[]) {
            self.cb_lo = pc;
        }
        let x: Vec<i32> = (0..12).filter(|b| Self::fits(*b, &spcs) == 0).collect();
        if let Some(pc) = self.note_picker(ui, "cb_bass", "THE BASS", self.cl_bass, &[(self.cl_bass, CL_BASS)], &x) {
            self.cl_bass = pc;
        }
        let sb = self.cl_bass;
        if Self::fits(sb, &spcs) == 0 {
            para(ui, "X: no scale holds all of these over this bass. Move the bass (the keys with x won't work either) or switch a step. Three semitones in a row fit none of them.", pal().red);
        }
        let mut all = vec![sb];
        for p in &spcs {
            if !all.contains(p) {
                all.push(*p);
            }
        }
        let cn = theory::ChordNotes { root: sb, bass: sb, pcs: all, named: Vec::new() };
        let grips = theory::guitar_shapes(&cn, &theory::STANDARD);
        if grips.is_empty() {
            para(ui, "No grip in one hand-span for this one. Spread it out: bass on the 6th or 5th string, the cluster on higher strings, open strings taking some of its notes.", pal().ink2);
        } else {
            let sp = self.chord_sp;
            let shape_col = if self.cb_steps[..size - 1].iter().any(|s| *s) { CL_SEMI } else { CL_TONE };
            let mut load = None;
            Self::card_grid(ui, |ui| {
                for g in grips.iter().take(6) {
                    let gp: Vec<i32> = grip_notes(g).iter().map(|m| m.rem_euclid(12)).collect();
                    let nm = theory::suggestions(&gp, Some(sb)).first().map(|s| s.name(sp)).unwrap_or_default();
                    let tint = |pc: i32| spcs.contains(&pc).then_some(shape_col);
                    if self.voicing_card(ui, acts, g, sb, &nm, &tint, true, false) == CardAct::Edit {
                        load = Some(*g);
                    }
                }
            });
            if let Some(g) = load {
                self.cb_frets = g;
                self.cb_name.clear();
                self.set_note("ON THE NECK ABOVE");
            }
        }
    }

    /// MY CHORDS: the voicings you kept.
    fn cl_mine(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if self.store.my_chords.is_empty() {
            para(
                ui,
                "Nothing here yet. Press SAVE under any voicing, or build one on the neck in BUILD and save it.",
                pal().ink2,
            );
            return;
        }
        let seq: Vec<Vec<i32>> = self.store.my_chords.iter().map(|c| grip_notes(&c.grip)).collect();
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "PLAY THEM ALL", false).tip("Each one in turn").clicked() {
                acts.push(Action::PlaySequence(seq.clone()));
            }
            ui.add_space(10.0);
            crate::views::label(ui, &format!("{} SAVED", self.store.my_chords.len()), 100.0);
        });
        let chords = self.store.my_chords.clone();
        let (mut edit, mut remove) = (None, None);
        Self::card_grid(ui, |ui| {
            for (i, c) in chords.iter().enumerate() {
                let notes = grip_notes(&c.grip);
                let bass = notes.iter().min().map(|m| m.rem_euclid(12)).unwrap_or(0);
                let tl: Vec<(i32, Color32)> = step_tint(&notes).iter().map(|(m, col)| (m.rem_euclid(12), *col)).collect();
                let tint = |pc: i32| tl.iter().find(|t| t.0 == pc).map(|t| t.1);
                match self.voicing_card(ui, acts, &c.grip, bass, &c.name, &tint, true, true) {
                    CardAct::Edit => edit = Some(i),
                    CardAct::Remove => remove = Some(i),
                    CardAct::None => {}
                }
            }
        });
        if let Some(i) = edit {
            self.cb_frets = chords[i].grip;
            self.cb_name = chords[i].name.clone();
            self.cl_view = 4;
        }
        if let Some(i) = remove {
            if i < self.store.my_chords.len() {
                self.store.my_chords.remove(i);
                self.store_dirty = true;
            }
        }
    }
}
