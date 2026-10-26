//! The CHORDS window, the desktop's own: FIND A CHORD (root, then kind, then the chord - its notes, the neck, the
//! piano, the shapes and the scales that fit), the ANALYZER (lay notes on the neck: every chord they could be, with
//! warnings and details) and TRIADS (every inversion up the neck on the strings you pick).

use crate::practice::text_box;
use crate::*;

const QUALITIES: [(&str, [i32; 3]); 6] =
    [("MAJOR", [0, 4, 7]), ("MINOR", [0, 3, 7]), ("DIM", [0, 3, 6]), ("AUG", [0, 4, 8]), ("SUS2", [0, 2, 7]), ("SUS4", [0, 5, 7])];
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
const KEYS: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];

impl App {
    pub(crate) fn chords_ui(&mut self, ui: &mut egui::Ui) {
        let (tr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
        if let Some(t) = seg_tabs(ui, tr, &["Find", "Analyzer", "Triads"], self.ch_tab as usize, "ch_tabs") {
            self.ch_tab = t as u8;
        }
        ui.add_space(4.0);
        // a chord picked: Oolimo's way, the long neck down the left and its voicings down the right
        if self.ch_tab == 0 && self.cf_neck {
            if let Some(c) = theory::chord_notes(self.chord_q.trim()) {
                self.chord_neck_view(ui, &c);
                return;
            }
            self.cf_neck = false;
        }
        egui::ScrollArea::vertical().id_salt("chords").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
            match self.ch_tab {
                0 => self.chord_find(ui),
                1 => self.chord_analyze(ui),
                _ => self.triads(ui),
            }
            ui.add_space(16.0);
        });
    }

    /// The chord on the long neck (frets 0 to 15 down the screen, the lit voicing's notes named by their role and
    /// the chord's other notes faint), its voicings down the right (tap one to light it and hear it), up and down to
    /// step through them, and CHANGE back to the picker.
    fn chord_neck_view(&mut self, ui: &mut egui::Ui, c: &theory::ChordNotes) {
        let q = self.chord_q.trim().to_string();
        // as a chord book lays them out (the desktop's way): the grips to know first, open position, the movable
        // ones by the string the root is on, then with a capo. Each with the heading it starts (if any).
        let mut heads: Vec<Option<String>> = Vec::new();
        let mut shapes: Vec<[Option<u8>; 6]> = Vec::new();
        let sec = theory::grip_sections(c, &theory::STANDARD);
        let mine = self.my_grips.get(&grip_key(&q)).cloned().unwrap_or_default();
        for (k, g) in mine.iter().enumerate() {
            heads.push((k == 0).then(|| "Mine".to_string()));
            shapes.push(*g);
        }
        for (title, list) in [("Must know", &sec.must), ("Open position", &sec.open)] {
            for (k, g) in list.iter().enumerate() {
                heads.push((k == 0).then(|| title.to_string()));
                shapes.push(*g);
            }
        }
        for (rs, list) in sec.moveable_by_string() {
            let name = ["6th", "5th", "4th", "3rd", "2nd", "1st"][rs.min(5)];
            for (k, g) in list.iter().enumerate() {
                heads.push((k == 0).then(|| format!("Root on the {}", name)));
                shapes.push(*g);
            }
        }
        for (k, s) in theory::capo_shapes(c, &theory::STANDARD).iter().enumerate() {
            heads.push(Some(if k == 0 { format!("With a capo: {} shape, capo {}", s.shape, s.capo) } else { format!("{} shape, capo {}", s.shape, s.capo) }));
            shapes.push(s.frets.map(|f| f.map(|f| f + s.capo)));
        }
        if shapes.is_empty() {
            shapes = self.shapes_for(&q, c);
            heads = vec![None; shapes.len()];
        }
        if !shapes.is_empty() {
            self.cf_voice = self.cf_voice.min(shapes.len() - 1);
        }
        let (head, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
        ptext(ui.painter(), Pos2::new(head.min.x + 2.0, head.center().y), Align::Min, &q, 3.2, pal().ink);
        let cr = Rect::from_min_size(Pos2::new(head.max.x - 164.0, head.min.y + 4.0), Vec2::new(164.0, 32.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(cr).layout(egui::Layout::left_to_right(Align::Center)));
        if pill_w(&mut child, "Staff", 72.0, self.cf_staff).clicked() {
            self.cf_staff = !self.cf_staff;
        }
        if pill_w(&mut child, "Change", 84.0, false).clicked() {
            self.cf_neck = false;
        }
        let notes: Vec<String> = c.named.iter().map(|(n, i)| format!("{} {}", n, i)).collect();
        let (nl, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
        ptext_fit(ui.painter(), Pos2::new(nl.min.x + 2.0, nl.center().y), Align::Min, &notes.join("   "), 1.1, nl.width(), pal().dim);
        ui.add_space(4.0);
        // the chord on a staff (as guitar music writes it) and a piano, when asked for
        if self.cf_staff {
            let voiced = piano_voicing(c);
            let w = ui.available_width();
            let (sr, _) = ui.allocate_exact_size(Vec2::new(w, 104.0), Sense::hover());
            let notes: Vec<(i32, Option<(usize, u8)>)> = voiced.iter().map(|m| (*m, None)).collect();
            let pcs = c.pcs.clone();
            let root = c.root;
            staff::chord_staff(ui, sr, &notes, lines::key_sharps(c.root), &|m| {
                let (d, col) = degree_in(m.rem_euclid(12), root, &pcs);
                (d.to_string(), col)
            });
            let (pr, _) = ui.allocate_exact_size(Vec2::new(w, 64.0), Sense::hover());
            piano_span(ui, pr.shrink2(Vec2::new(2.0, 4.0)), &voiced, false, "cv_piano", Some(c.root), 48, 2);
            ui.add_space(4.0);
        }
        let avail = ui.available_rect_before_wrap();
        let body = Rect::from_min_max(avail.min, Pos2::new(avail.max.x, avail.max.y - 64.0));
        let col_w = (body.width() * 0.36).min(130.0);
        let neck_r = Rect::from_min_max(body.min, Pos2::new(body.max.x - col_w - 8.0, body.max.y));
        let lit = shapes.get(self.cf_voice).copied();
        long_neck(ui.painter(), neck_r, c, lit.as_ref());
        // the voicings, a column of chord boxes
        let col_r = Rect::from_min_max(Pos2::new(body.max.x - col_w, body.min.y), body.max);
        let mut col = ui.new_child(egui::UiBuilder::new().max_rect(col_r).layout(egui::Layout::top_down(Align::Min)));
        let mut pick = None;
        egui::ScrollArea::vertical().id_salt("voicings").auto_shrink([false, false]).show(&mut col, |ui| {
            let (root, pcs) = (c.root, c.pcs.clone());
            for (k, g) in shapes.iter().enumerate() {
                if let Some(h) = &heads[k] {
                    for line in wrap(h, 0.9, col_w - 8.0) {
                        let (lr, _) = ui.allocate_exact_size(Vec2::new(col_w - 6.0, 14.0), Sense::hover());
                        ptext(ui.painter(), Pos2::new(lr.min.x + 2.0, lr.center().y), Align::Min, &line, 0.9, pal().ink2);
                    }
                }
                let (r, resp) = ui.allocate_exact_size(Vec2::new(col_w - 6.0, (col_w - 6.0) * 1.1), Sense::click());
                ui.painter().rect_filled(r, 8.0, pal().beige_lt);
                if k == self.cf_voice {
                    ui.painter().rect_stroke(r, 8.0, Stroke::new(2.0, pal().red));
                }
                chord_box_labelled(ui.painter(), r.shrink(4.0), g, &|m| degree_in(m, root, &pcs).0.to_string());
                if resp.clicked() {
                    pick = Some(k);
                }
                ui.add_space(4.0);
            }
            if shapes.is_empty() {
                note_text(ui, "No comfortable shape in reach.", pal().dim);
            }
        });
        if let Some(k) = pick {
            self.cf_voice = k;
            self.hear(&grip_notes(&shapes[k]));
        }
        // up, play, down
        let tr = Rect::from_min_max(Pos2::new(avail.min.x, avail.max.y - 60.0), avail.max);
        let mid = tr.center();
        let p = ui.painter().clone();
        p.circle_filled(mid, 24.0, pal().red);
        play_glyph(&p, mid, false, pal().bar_txt);
        if ui.interact(Rect::from_center_size(mid, Vec2::splat(48.0)), ui.id().with("cv_play"), Sense::click()).clicked() {
            match shapes.get(self.cf_voice) {
                Some(g) => self.hear(&grip_notes(g)),
                None => self.hear(&piano_voicing(c)),
            }
        }
        for (dx, d) in [(-80.0, -1i32), (80.0, 1)] {
            let cc = mid + Vec2::new(dx, 0.0);
            let s = Stroke::new(2.4, pal().ink);
            let y = if d < 0 { 4.0 } else { -4.0 };
            p.line_segment([cc + Vec2::new(-9.0, y), cc + Vec2::new(0.0, -y)], s);
            p.line_segment([cc + Vec2::new(0.0, -y), cc + Vec2::new(9.0, y)], s);
            if ui.interact(Rect::from_center_size(cc, Vec2::splat(48.0)), ui.id().with(("cv_step", d)), Sense::click()).clicked() && !shapes.is_empty() {
                let n = shapes.len() as i32;
                self.cf_voice = ((self.cf_voice as i32 + d).rem_euclid(n)) as usize;
                self.hear(&grip_notes(&shapes[self.cf_voice]));
            }
        }
    }

    /// Guitar shapes for a chord, worked out once and kept.
    fn shapes_for(&mut self, name: &str, c: &theory::ChordNotes) -> Vec<[Option<u8>; 6]> {
        if let Some(s) = self.shape_cache.get(name) {
            return s.clone();
        }
        let s = theory::guitar_shapes(c, &theory::STANDARD);
        self.shape_cache.insert(name.to_string(), s.clone());
        s
    }

    /// Chord boxes two to a row, the notes named by their role; tap one to hear it (or, `to_neck`, to put it on
    /// the analyzer's neck too).
    fn grip_cards(&mut self, ui: &mut egui::Ui, c: &theory::ChordNotes, shapes: &[[Option<u8>; 6]], to_neck: bool) {
        let (root, pcs) = (c.root, c.pcs.clone());
        let w = ((ui.available_width() - 10.0) / 2.0).min(220.0);
        let mut hear = None;
        for row in shapes.chunks(2) {
            ui.horizontal(|ui| {
                for g in row {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, w * 1.15), Sense::click());
                    chord_box_labelled(ui.painter(), r, g, &|m| degree_in(m, root, &pcs).0.to_string());
                    if resp.clicked() {
                        hear = Some(*g);
                    }
                }
            });
        }
        if let Some(g) = hear {
            self.hear(&grip_notes(&g));
            if to_neck {
                self.an_frets = g;
            }
        }
    }

    fn chord_find(&mut self, ui: &mut egui::Ui) {
        // a funnel: the root, then the kind of chord, then that kind's chords - each step opens under the one before
        let cur = theory::chord_notes(self.chord_q.trim());
        let suffix_now = chart::split_root(self.chord_q.trim()).1.to_string();
        if let Some(c) = &cur {
            self.cf_root = Some(c.root);
            if let Some(f) = theory::FINDER_FAMILIES.iter().position(|(_, l)| l.iter().any(|(_, s)| *s == suffix_now)) {
                self.cf_fam = Some(f);
            }
        }
        // the picker you sent: the twelve roots on top, a row to narrow the kind, then every chord of that root in a
        // grid; tap one and its neck opens
        let root = self.cf_root.unwrap_or(0);
        let w = ui.available_width();
        let cell = (w - 5.0 * 4.0) / 6.0;
        let (rr, _) = ui.allocate_exact_size(Vec2::new(w, 2.0 * 40.0 + 4.0), Sense::hover());
        for pc in 0..12 {
            let r = Rect::from_min_size(rr.min + Vec2::new((pc % 6) as f32 * (cell + 4.0), (pc / 6) as f32 * 44.0), Vec2::new(cell, 40.0));
            let resp = ui.interact(r, ui.id().with(("cf_root", pc)), Sense::click());
            let on = self.cf_root == Some(pc);
            ui.painter().rect_filled(r, 8.0, if on { pal().ink } else if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige_lt });
            ptext(ui.painter(), r.center(), Align::Center, theory::note_name(pc, false), 1.5, if on { pal().beige } else { pal().ink });
            if resp.clicked() {
                self.cf_root = Some(pc);
            }
        }
        ui.add_space(6.0);
        let fams: Vec<&str> = std::iter::once("All").chain(theory::FINDER_FAMILIES.iter().map(|(n, _)| *n)).collect();
        let cur_f = self.cf_fam.map_or(0, |f| f + 1);
        // the kinds as small chips, wrapped
        {
            let mut x = 0.0f32;
            let mut y = 0.0f32;
            let start = ui.cursor().min;
            let mut chips = Vec::new();
            for n in fams.iter() {
                let label = crate::lines_ui::nice(n);
                let cw = text_w(&label, 1.2) + 22.0;
                if x + cw > w {
                    x = 0.0;
                    y += 36.0;
                }
                chips.push((Rect::from_min_size(start + Vec2::new(x, y), Vec2::new(cw, 32.0)), label));
                x += cw + 6.0;
            }
            ui.allocate_exact_size(Vec2::new(w, y + 34.0), Sense::hover());
            for (k, (r, label)) in chips.into_iter().enumerate() {
                let resp = ui.interact(r, ui.id().with(("cf_fam", k)), Sense::click());
                let on = k == cur_f;
                ui.painter().rect_filled(r, 16.0, if on { pal().ink } else if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige_lt });
                ptext(ui.painter(), r.center(), Align::Center, &label, 1.2, if on { pal().beige } else { pal().ink });
                if resp.clicked() {
                    self.cf_fam = if k == 0 { None } else { Some(k - 1) };
                }
            }
        }
        ui.add_space(4.0);
        let rn = theory::note_name(root, false);
        let names: Vec<String> = theory::FINDER_FAMILIES
            .iter()
            .enumerate()
            .filter(|(i, _)| self.cf_fam.is_none_or(|f| f == *i))
            .flat_map(|(_, (_, l))| l.iter().map(|(_, s)| format!("{}{}", rn, s)))
            .collect();
        let cols = 3;
        let gw = (w - (cols - 1) as f32 * 4.0) / cols as f32;
        let rows = names.len().div_ceil(cols);
        let (gr, _) = ui.allocate_exact_size(Vec2::new(w, rows as f32 * 44.0), Sense::hover());
        for (k, name) in names.iter().enumerate() {
            let r = Rect::from_min_size(gr.min + Vec2::new((k % cols) as f32 * (gw + 4.0), (k / cols) as f32 * 44.0), Vec2::new(gw, 40.0));
            let resp = ui.interact(r, ui.id().with(("cf_chord", k)), Sense::click());
            let on = self.chord_q.trim() == name;
            ui.painter().rect_filled(r, 8.0, if on { pal().ink } else if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige_lt });
            ptext_fit(ui.painter(), r.center(), Align::Center, name, 1.4, gw - 8.0, if on { pal().beige } else { pal().ink });
            if resp.clicked() {
                self.chord_q = name.clone();
                if let Some(c) = theory::chord_notes(name) {
                    self.hear(&piano_voicing(&c));
                    self.cf_neck = true;
                    self.cf_voice = 0;
                }
            }
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let w = ui.available_width() - 82.0;
            text_box(ui, &mut self.chord_q, w, "Or type one: Dm7, G7#9, Fmaj7/A...");
            if pill_w(ui, "Show", 74.0, false).clicked() && theory::chord_notes(&self.chord_q).is_some() {
                self.cf_neck = true;
                self.cf_voice = 0;
            }
        });
        let Some(c) = theory::chord_notes(self.chord_q.trim()) else {
            ui.add_space(6.0);
            note_text(ui, "Pick a root, then a kind of chord, then a chord (or type one) to see it on the neck and the piano, its shapes and the scales that fit it.", pal().ink2);
            return;
        };
        ui.add_space(6.0);
        title_line(ui, self.chord_q.trim(), 4.0, pal().ink);
        let notes: Vec<String> = c.named.iter().map(|(n, i)| format!("{} ({})", n, i)).collect();
        note_text(ui, &notes.join("   "), pal().ink2);
        dim(ui, "On the neck", 1.2, pal().ink2);
        let w = ui.available_width();
        let (nr, _) = ui.allocate_exact_size(Vec2::new(w, 120.0), Sense::hover());
        neck(ui, Rect::from_min_max(nr.min, Pos2::new(nr.max.x, nr.max.y - 12.0)), &[None; 6], 12, false, "find_neck", Some(c.root), &c.pcs, true);
        dim(ui, "On the piano", 1.2, pal().ink2);
        let (pr, _) = ui.allocate_exact_size(Vec2::new(w, 80.0), Sense::hover());
        piano_span(ui, pr.shrink2(Vec2::new(2.0, 4.0)), &piano_voicing(&c), false, "find_piano", Some(c.root), 48, 2);
        dim(ui, "Guitar", 1.2, pal().ink2);
        let q = self.chord_q.trim().to_string();
        let shapes = self.shapes_for(&q, &c);
        if shapes.is_empty() {
            note_text(ui, "No comfortable shape in reach for this one.", pal().ink2);
        }
        self.grip_cards(ui, &c, &shapes, false);
        dim(ui, "Scales that fit", 1.2, pal().ink2);
        for (name, steps) in theory::scales_for(&c) {
            let names: Vec<&str> = steps.iter().map(|s| theory::note_name(c.root + s, false)).collect();
            dim(ui, &format!("{} {}", theory::note_name(c.root, false), name.to_uppercase()), 2.0, pal().ink);
            dim(ui, &names.join(" "), 1.0, pal().ink2);
        }
    }

    fn chord_analyze(&mut self, ui: &mut egui::Ui) {
        dim(ui, "Tap notes on the neck, one per string; tap again to lift one", 1.0, pal().ink2);
        let notes: Vec<i32> = self.an_frets.iter().enumerate().filter_map(|(i, f)| f.map(|f| theory::STANDARD[i] + f as i32)).collect();
        let pcs: Vec<i32> = notes.iter().map(|m| m.rem_euclid(12)).collect();
        let bass = notes.first().map(|m| m.rem_euclid(12));
        let found = theory::suggestions(&pcs, bass);
        let pick = |k: &Option<String>| k.as_ref().and_then(|k| found.iter().find(|s| &s.key() == k)).cloned();
        let shown = pick(&self.an_pick).or_else(|| found.first().cloned());
        let root = shown.as_ref().map(|s| s.root).or(bass);
        // a seventh over the root makes the sixth a 13 (for this frame's labels)
        SEVENTH.with(|c| c.set(root.filter(|rt| pcs.iter().any(|p| matches!((p - rt).rem_euclid(12), 10 | 11)))));
        let ghost: Vec<i32> = shown.as_ref().map(|s| s.missing.clone()).unwrap_or_default();
        let w = ui.available_width();
        // the long neck, as in Find: tap a note to lay it down (one per string), tap it again to lift it; the
        // answer beside it
        let h = if self.an_big { 520.0 } else { 420.0 };
        let (row, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
        let side = 118.0;
        let nr = Rect::from_min_max(row.min, Pos2::new(row.max.x - side - 8.0, row.max.y));
        let mut dots: Vec<((usize, u8), String, Color32, bool)> = Vec::new();
        for (s, f) in self.an_frets.iter().enumerate() {
            if let Some(f) = f {
                let pc = (theory::STANDARD[s] + *f as i32).rem_euclid(12);
                let (d, col) = match root {
                    Some(rt) => degree_in(pc, rt, &pcs),
                    None => ("", pal().ink),
                };
                dots.push(((s, *f), d.to_string(), col, false));
            }
        }
        // the chord's missing notes, faint rings
        for (s, open) in theory::STANDARD.iter().enumerate() {
            for f in 0..=15u8 {
                let pc = (open + f as i32).rem_euclid(12);
                if ghost.iter().any(|g| g.rem_euclid(12) == pc) && self.an_frets[s] != Some(f) {
                    dots.push(((s, f), String::new(), pal().dim, true));
                }
            }
        }
        crate::lines_ui::dot_neck(ui.painter(), nr, &dots);
        let resp = ui.interact(nr, ui.id().with("an_long"), Sense::click());
        if let Some(pp) = resp.interact_pointer_pos().filter(|_| resp.clicked()) {
            let x0 = nr.min.x + 26.0;
            let sw = (nr.width() - 40.0) / 5.0;
            let y0 = nr.min.y + 18.0;
            let fh = (nr.height() - 24.0) / 15.0;
            let s = (((pp.x - x0) / sw).round() as i32).clamp(0, 5) as usize;
            let f = if pp.y < y0 { 0u8 } else { (((pp.y - y0) / fh).floor() as i32 + 1).clamp(1, 15) as u8 };
            if self.an_frets[s] == Some(f) {
                self.an_frets[s] = None;
            } else {
                self.an_frets[s] = Some(f);
                self.hear(&[theory::STANDARD[s] + f as i32]);
            }
            self.an_pick = None;
        }
        // beside it: what it is, what else it could be, and the buttons
        let sr = Rect::from_min_max(Pos2::new(nr.max.x + 8.0, row.min.y), row.max);
        let mut col = ui.new_child(egui::UiBuilder::new().max_rect(sr).layout(egui::Layout::top_down(Align::Min)));
        col.spacing_mut().item_spacing = Vec2::new(4.0, 6.0);
        match shown.as_ref() {
            Some(s) => {
                let name = s.name(self.chord_sp);
                for line in wrap(&name, 2.2, side - 4.0) {
                    let (lr, _) = col.allocate_exact_size(Vec2::new(side, 26.0), Sense::hover());
                    ptext(col.painter(), Pos2::new(lr.min.x, lr.center().y), Align::Min, &line, 2.2, pal().ink);
                }
                let others: Vec<String> = found.iter().filter(|o| o.key() != s.key()).take(3).map(|o| o.name(self.chord_sp)).collect();
                if !others.is_empty() {
                    for line in wrap(&format!("or {}", others.join(", ")), 1.0, side - 4.0) {
                        dim(&mut col, &line, 1.0, pal().dim);
                    }
                }
            }
            None => dim(&mut col, if notes.is_empty() { "Tap notes on the neck" } else { "Add a note or two" }, 1.0, pal().dim),
        }
        col.add_space(6.0);
        if pill_w(&mut col, "Strum", side - 4.0, false).clicked() {
            self.hear(&notes);
        }
        if pill_w(&mut col, "Clear", side - 4.0, false).clicked() {
            self.an_frets = [None; 6];
            self.an_pick = None;
        }
        ui.add_space(8.0);
        let (pr, _) = ui.allocate_exact_size(Vec2::new(w, 64.0), Sense::hover());
        piano_span(ui, pr, &notes, false, "an_piano", root, 36, 4);
        ui.horizontal_wrapped(|ui| {
            if pill(ui, if self.an_big { "Smaller neck" } else { "Bigger neck" }, self.an_big).clicked() {
                self.an_big = !self.an_big;
            }
            // keep this grip as one of yours for the chord it makes (it shows first in Find)
            if let Some(s) = shown.as_ref() {
                let name = s.name(self.chord_sp);
                if notes.len() >= 2 && pill(ui, &format!("Save as my {}", name), false).clicked() {
                    let key = grip_key(&name);
                    let list = self.my_grips.entry(key).or_default();
                    if !list.contains(&self.an_frets) {
                        list.push(self.an_frets);
                    }
                    save_json("my_grips.json", &self.my_grips);
                    self.set_note(&format!("Saved: it shows first under {} in Find", name));
                }
            }
            // the whole shape a fret down or up the neck
            let frets: Vec<u8> = self.an_frets.iter().flatten().copied().collect();
            let (lo, hi) = (frets.iter().min().copied(), frets.iter().max().copied());
            for (label, d) in [("< MOVE", -1i32), ("MOVE >", 1)] {
                let can = matches!((lo, hi), (Some(l), Some(h)) if (d < 0 && l > 0) || (d > 0 && h < 15));
                if pill(ui, label, false).clicked() && can {
                    for f in self.an_frets.iter_mut().flatten() {
                        *f = (*f as i32 + d) as u8;
                    }
                    self.hear(&grip_notes(&self.an_frets));
                }
            }
        });
        let mut v = pcs.clone();
        v.sort();
        v.dedup();
        let names: Vec<String> = v
            .iter()
            .map(|p| match root {
                Some(rt) => format!("{} ({})", theory::note_name(*p, self.chord_sp.sharps), degree(*p, rt).0),
                None => theory::note_name(*p, self.chord_sp.sharps).to_string(),
            })
            .collect();
        dim(ui, &format!("NOTES: {}", if names.is_empty() { "-".to_string() } else { names.join("  ") }), 1.0, pal().ink2);
        dim(ui, "Possible chords", 1.2, pal().ink2);
        // how the names are written, and the details
        ui.horizontal_wrapped(|ui| {
            let sp = &mut self.chord_sp;
            if pill(ui, "- \u{0394} \u{00b0} +", sp.symbols).clicked() {
                sp.symbols = !sp.symbols;
            }
            if pill(ui, "#", sp.sharps).clicked() {
                sp.sharps = !sp.sharps;
            }
            if pill(ui, "Slash", sp.slash).clicked() {
                sp.slash = !sp.slash;
            }
            if pill(ui, "Detailed", sp.detailed).clicked() {
                sp.detailed = !sp.detailed;
            }
            if pill(ui, "Details", self.chord_details).clicked() {
                self.chord_details = !self.chord_details;
                self.save_settings();
            }
        });
        let sp = self.chord_sp;
        if let Some(s) = &shown {
            dim(ui, &format!("SHOWING {}  -  TAP ONE TO KEEP IT", s.name(sp)), 1.0, pal().dim);
        }
        if found.is_empty() {
            note_text(ui, if pcs.len() < 2 { "Lay down two or more notes." } else { "No chord has all of these notes." }, pal().ink2);
        }
        ui.horizontal_wrapped(|ui| {
            for s in &found {
                let key = s.key();
                // a warning triangle before a chord that might not work (DETAILS says why)
                if !s.warnings.is_empty() {
                    let (r, _) = ui.allocate_exact_size(Vec2::new(16.0, bh()), Sense::hover());
                    warn_triangle(ui.painter(), r);
                }
                let picked = self.an_pick.as_deref() == Some(key.as_str());
                if pill(ui, &s.name(sp), s.exact || picked).clicked() {
                    self.an_pick = if picked { None } else { Some(key) };
                }
            }
        });
        // a chord picked: grips that play it, anywhere on the neck (tap one to hear it and lay it on the neck)
        if let Some(s) = pick(&self.an_pick) {
            let mut pcs = s.pcs.clone();
            let bass = s.bass.unwrap_or(s.root);
            if !pcs.contains(&bass) {
                pcs.insert(0, bass);
            }
            let c = theory::ChordNotes { root: s.root, bass, pcs, named: Vec::new() };
            let shapes = self.shapes_for(&s.key(), &c);
            dim(ui, &format!("Grips for {}", s.name(sp)), 1.2, pal().ink2);
            if shapes.is_empty() {
                note_text(ui, "No comfortable grip in reach for this one.", pal().ink2);
            }
            self.grip_cards(ui, &c, &shapes, true);
        }
        // the details: each chord broken down, with its warnings
        if self.chord_details && !found.is_empty() {
            dim(ui, "Details", 1.2, pal().ink2);
            for s in found.iter().take(12) {
                let notes: Vec<String> = s
                    .pcs
                    .iter()
                    .map(|p| {
                        let mark = if s.missing.contains(p) { "-" } else { "+" };
                        format!("{}{} ({})", mark, theory::note_name(*p, sp.sharps), degree(*p, s.root).0)
                    })
                    .collect();
                dim(ui, &format!("{}{}", s.name(sp), if s.exact { "   EXACT" } else { "" }), 2.0, pal().ink);
                note_text(ui, &notes.join("  "), pal().ink2);
                for w in &s.warnings {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
                        warn_triangle(ui.painter(), r);
                        ui.vertical(|ui| note_text(ui, w, pal().ink2));
                    });
                }
            }
            dim(ui, "+ You play it   - it is not played", 1.0, pal().dim);
        }
        // the colour key
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for (name, pc) in [("ROOT", 0), ("3RD", 4), ("5TH", 7), ("6TH / 7TH", 10), ("9 / 11", 2)] {
                let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 6.0, degree(pc, 0).1);
                lbl(ui, name, 70.0);
            }
        });
        SEVENTH.with(|c| c.set(None));
    }

    /// TRIADS: one triad, every inversion up the neck on the strings you pick, the whole neck lit under them.
    fn triads(&mut self, ui: &mut egui::Ui) {
        // the desktop's three: the map up the neck, voice leading through changes, a triad over each bass note
        ui.horizontal_wrapped(|ui| {
            for (k, n) in ["Map", "Voice leading", "Over a bass"].iter().enumerate() {
                if pill(ui, n, self.tr_page as usize == k).clicked() {
                    self.tr_page = k as u8;
                }
            }
        });
        match self.tr_page {
            1 => return self.tr_lead(ui),
            2 => return self.tr_bass(ui),
            _ => {}
        }
        note_text(ui, "One triad, every inversion up the neck on the strings you pick. Root position, then the 3rd on the bottom, then the 5th, and round again an octave up. Tap a dot or a box to pick a shape.", pal().ink2);
        ui.horizontal_wrapped(|ui| {
            if let Some(k) = self.pick_chip(ui, "tr_key", &KEYS, self.tr_root as usize) {
                self.tr_root = k as i32;
            }
            let sets: Vec<&str> = SETS.iter().map(|s| s.0).collect();
            if let Some(i) = self.pick_chip(ui, "tr_set", &sets, self.tr_set) {
                self.tr_set = i;
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (i, (name, _)) in QUALITIES.iter().enumerate() {
                if pill(ui, name, self.tr_q == i).clicked() {
                    self.tr_q = i;
                }
            }
        });
        let root = self.tr_root;
        let q = QUALITIES[self.tr_q.min(5)].1;
        let pcs: Vec<i32> = q.iter().map(|x| (root + x).rem_euclid(12)).collect();
        let grips = theory::grips_on(&pcs, SETS[self.tr_set.min(SETS.len() - 1)].1, false, 4);
        if grips.is_empty() {
            note_text(ui, "No shape for this on these strings in one hand-span.", pal().ink2);
            return;
        }
        self.tr_sel = self.tr_sel.min(grips.len() - 1);
        let role = |m: i32| {
            let (d, c) = degree(m, root);
            (d.to_string(), c)
        };
        if let Some(i) = fret_map(ui, &grips, self.tr_sel, &role, Some(pcs.as_slice())) {
            self.tr_sel = i;
            self.hear(&grip_notes(&grips[i]));
        }
        ui.horizontal_wrapped(|ui| {
            // every shape in turn (the desktop's own sequence sound); STOP while it plays
            let playing = self.seq_end.is_some_and(|e| Instant::now() < e);
            if playing {
                ui.ctx().request_repaint_after(Duration::from_millis(200));
                if pill(ui, "Stop", true).clicked() {
                    self.player.send(Cmd::StopOnce);
                    self.seq_end = None;
                }
            } else if pill(ui, "Hear them up the neck", false).clicked() {
                let seq: Vec<Vec<i32>> = grips.iter().map(grip_notes).collect();
                self.player.send(Cmd::StopOnce);
                self.player.send(Cmd::Once(band::sequence_sound(&seq, 0.9), 0.9));
                self.seq_end = Some(Instant::now() + Duration::from_secs_f32(seq.len() as f32 * 0.9 + 1.5));
            }
            dim(ui, "R root   3 third   5 fifth", 1.0, pal().ink2);
        });
        // the shapes as boxes, two to a row
        let w = ((ui.available_width() - 10.0) / 2.0).min(220.0);
        let mut pick = None;
        for (row, chunk) in grips.chunks(2).enumerate() {
            ui.horizontal(|ui| {
                for (k, g) in chunk.iter().enumerate() {
                    let i = row * 2 + k;
                    let bass = grip_notes(g).iter().min().map(|m| (m - root).rem_euclid(12)).unwrap_or(0);
                    let inv = if bass == 0 {
                        "Root position"
                    } else if bass == q[1] {
                        "3rd on the bottom"
                    } else {
                        "5th on the bottom"
                    };
                    // a card: the shape's name on top, its box under it, the one picked outlined
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, w * 1.38), Sense::click());
                    ui.painter().rect_filled(r, 10.0, pal().beige_lt);
                    if i == self.tr_sel {
                        ui.painter().rect_stroke(r, 10.0, Stroke::new(2.0, pal().red));
                    }
                    ptext_fit(ui.painter(), Pos2::new(r.center().x, r.min.y + 14.0), Align::Center, inv, 1.1, r.width() - 12.0, pal().ink);
                    let boxr = Rect::from_min_max(r.min + Vec2::new(8.0, 30.0), r.max - Vec2::new(8.0, 8.0));
                    chord_box_coloured(ui.painter(), boxr, g, &|m| {
                        let (d, c) = degree(m, root);
                        (d.to_string(), c)
                    });
                    if resp.clicked() {
                        pick = Some(i);
                    }
                }
            });
        }
        if let Some(i) = pick {
            self.tr_sel = i;
            self.hear(&grip_notes(&grips[i]));
        }
    }

    /// The key and the strings, shared by the triad pages.
    fn tr_key_set(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if let Some(k) = self.pick_chip(ui, "tr_key", &KEYS, self.tr_root as usize) {
                self.tr_root = k as i32;
            }
            let sets: Vec<&str> = SETS.iter().map(|s| s.0).collect();
            if let Some(i) = self.pick_chip(ui, "tr_set", &sets, self.tr_set) {
                self.tr_set = i;
            }
        });
    }

    /// VOICE LEADING: type the changes; each becomes the triad on the same strings nearest the last, so the hand
    /// barely moves. The top notes make a melody of their own.
    fn tr_lead(&mut self, ui: &mut egui::Ui) {
        note_text(ui, "Type the changes and each chord becomes the triad nearest the last, all on the same strings. Chord names, numerals (I IV V, ii) or 1-7 for the key's own triads.", pal().ink2);
        self.tr_key_set(ui);
        ui.horizontal_wrapped(|ui| {
            for (k, n) in ["Low on the neck", "Middle", "High"].iter().enumerate() {
                if pill(ui, n, self.tr_pos as usize == k).clicked() {
                    self.tr_pos = k as u8;
                }
            }
        });
        let w = ui.available_width();
        text_box(ui, &mut self.tr_prog, w, "1 4 5 1   or   C Am Dm G7   or   I vi ii V");
        let set = SETS[self.tr_set.min(SETS.len() - 1)].1;
        let near = [2.0f32, 7.0, 12.0][(self.tr_pos as usize).min(2)];
        let mut steps: Vec<(String, [Option<u8>; 6])> = Vec::new();
        let mut prev: Option<[Option<u8>; 6]> = None;
        let mut bad: Vec<String> = Vec::new();
        for tok in self.tr_prog.split_whitespace() {
            let Some((root, q)) = triad_core::triad_of(tok, self.tr_root) else {
                bad.push(tok.to_string());
                continue;
            };
            let pcs: Vec<i32> = q.iter().map(|x| (root + x).rem_euclid(12)).collect();
            if let Some(g) = triad_core::closest(&theory::grips_on(&pcs, set, false, 4), prev.as_ref(), near) {
                steps.push((tok.to_string(), g));
                prev = Some(g);
            }
        }
        if !bad.is_empty() {
            dim(ui, &format!("Couldn't read: {}", bad.join(" ")), 1.0, pal().red);
        }
        if steps.is_empty() {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            if pill(ui, "Play it", false).clicked() {
                let seq: Vec<Vec<i32>> = steps.iter().map(|s| grip_notes(&s.1)).collect();
                self.player.send(Cmd::StopOnce);
                self.player.send(Cmd::Once(band::sequence_sound(&seq, 1.0), 0.9));
            }
        });
        let tops: Vec<String> = steps
            .iter()
            .map(|s| grip_notes(&s.1).iter().max().map(|m| theory::note_name(*m, self.chord_sp.sharps).to_string()).unwrap_or_default())
            .collect();
        dim(ui, &format!("Top notes: {}", tops.join(" ")), 1.0, pal().ink2);
        // each chord as a box, the notes that moved from the one before in the accent
        let bw = (ui.available_width() - 16.0) / 3.0;
        let mut hear = None;
        ui.horizontal_wrapped(|ui| {
            for (k, (name, g)) in steps.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(Vec2::new(bw, bw * 1.3), Sense::click());
                ui.painter().rect_filled(r, 8.0, pal().beige_lt);
                ptext(ui.painter(), Pos2::new(r.center().x, r.min.y + 12.0), Align::Center, name, 1.3, pal().ink);
                let before = if k > 0 { Some(steps[k - 1].1) } else { None };
                chord_box_labelled(ui.painter(), Rect::from_min_max(r.min + Vec2::new(4.0, 22.0), r.max - Vec2::new(4.0, 4.0)), g, &|m| {
                    let moved = before.is_some_and(|b| !grip_notes(&b).contains(&m));
                    if moved {
                        "*".to_string()
                    } else {
                        theory::note_name(m, false).to_string()
                    }
                });
                if resp.clicked() {
                    hear = Some(grip_notes(g));
                }
            }
        });
        if let Some(n) = hear {
            self.hear(&n);
        }
        dim(ui, "* a note that moved from the chord before", 1.0, pal().dim);
    }

    /// OVER A BASS: a triad over each of the twelve bass notes and the chord it makes; pick one to see the grips.
    fn tr_bass(&mut self, ui: &mut egui::Ui) {
        note_text(ui, "Over a new bass note, a triad becomes a bigger chord: D over C is a C Lydian sound, Bb over C a C9sus, F over G the gospel IV over V.", pal().ink2);
        self.tr_key_set(ui);
        ui.horizontal_wrapped(|ui| {
            for (i, (name, _)) in QUALITIES.iter().enumerate() {
                if pill(ui, &crate::lines_ui::nice(name), self.tr_q == i).clicked() {
                    self.tr_q = i;
                }
            }
        });
        let root = self.tr_root;
        let q = QUALITIES[self.tr_q.min(5)].1;
        let pcs: Vec<i32> = q.iter().map(|x| (root + x).rem_euclid(12)).collect();
        let sp = self.chord_sp;
        let w = ui.available_width();
        for b in 0..12 {
            let mut all = pcs.clone();
            if !all.contains(&b) {
                all.push(b);
            }
            let name = theory::suggestions(&all, Some(b)).first().map(|s| s.name(sp)).unwrap_or_else(|| "-".to_string());
            let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 34.0), Sense::click());
            let p = ui.painter();
            if self.tr_bass == b {
                p.rect_filled(r, 6.0, pal().row_sel);
            }
            let col = if self.tr_bass == b { pal().bar_txt } else { pal().ink };
            ptext(p, Pos2::new(r.min.x + 10.0, r.center().y), Align::Min, &format!("Over {}", theory::note_name(b, sp.sharps)), 1.3, col);
            ptext(p, Pos2::new(r.min.x + w * 0.42, r.center().y), Align::Min, &name, 1.3, col);
            if resp.clicked() {
                self.tr_bass = b;
            }
        }
        let bass = self.tr_bass;
        let mut grips: Vec<[Option<u8>; 6]> = Vec::new();
        for set in [&[3usize, 4, 5][..], &[2, 3, 4], &[1, 2, 3]] {
            for g in theory::grips_on(&pcs, set, false, 4) {
                let gb = triad_core::with_bass(&g, bass);
                let fretted: Vec<u8> = gb.iter().flatten().copied().filter(|f| *f > 0).collect();
                let span = fretted.iter().max().unwrap_or(&0) - fretted.iter().min().unwrap_or(&0);
                if gb != g && span <= 5 && !grips.contains(&gb) {
                    grips.push(gb);
                }
            }
        }
        grips.sort_by(|a, b| theory::centre(a).partial_cmp(&theory::centre(b)).unwrap_or(std::cmp::Ordering::Equal));
        ui.add_space(6.0);
        if grips.is_empty() {
            note_text(ui, "No grip in one hand-span. Play the bass on its own, then the triad.", pal().ink2);
            return;
        }
        let bw = (ui.available_width() - 16.0) / 3.0;
        let mut hear = None;
        ui.horizontal_wrapped(|ui| {
            for g in grips.iter().take(6) {
                let (r, resp) = ui.allocate_exact_size(Vec2::new(bw, bw * 1.3), Sense::click());
                ui.painter().rect_filled(r, 8.0, pal().beige_lt);
                ptext_fit(ui.painter(), Pos2::new(r.center().x, r.min.y + 12.0), Align::Center, &triad_core::grip_name(g, sp), 1.2, bw - 8.0, pal().ink);
                chord_box_labelled(ui.painter(), Rect::from_min_max(r.min + Vec2::new(4.0, 22.0), r.max - Vec2::new(4.0, 4.0)), g, &|m| {
                    if m.rem_euclid(12) == bass.rem_euclid(12) && !pcs.contains(&m.rem_euclid(12)) {
                        "B".to_string()
                    } else {
                        degree(m, root).0.to_string()
                    }
                });
                if resp.clicked() {
                    hear = Some(grip_notes(g));
                }
            }
        });
        if let Some(n) = hear {
            self.hear(&n);
        }
    }
}

/// A chord's name as a key for your grips (spaces and case don't matter).
fn grip_key(name: &str) -> String {
    name.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase()
}
