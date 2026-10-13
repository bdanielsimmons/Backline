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
        if let Some(t) = tab_row(ui, &["FIND A CHORD", "ANALYZER", "TRIADS"], self.ch_tab as usize) {
            self.ch_tab = t as u8;
        }
        ui.add_space(4.0);
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
        let w = ((ui.available_width() - 10.0) / 2.0).min(150.0);
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
        let rule = |ui: &mut egui::Ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 9.0), Sense::hover());
            ui.painter().line_segment([Pos2::new(r.min.x + 4.0, r.center().y), Pos2::new(r.max.x - 4.0, r.center().y)], Stroke::new(1.0, pal().edge));
        };
        dim_line(ui, "1  ROOT", 1.0, pal().ink2);
        ui.horizontal_wrapped(|ui| {
            for pc in 0..12 {
                let name = theory::note_name(pc, false);
                if retro_btn_w(ui, name, 40.0, self.cf_root == Some(pc)).clicked() {
                    self.cf_root = Some(pc);
                    if cur.is_some() && self.cf_fam.is_some() {
                        self.chord_q = format!("{}{}", name, suffix_now);
                    }
                }
            }
        });
        if let Some(root) = self.cf_root {
            rule(ui);
            dim_line(ui, "2  KIND", 1.0, pal().ink2);
            ui.horizontal_wrapped(|ui| {
                for (i, (name, _)) in theory::FINDER_FAMILIES.iter().enumerate() {
                    if retro_btn(ui, name, self.cf_fam == Some(i)).clicked() && self.cf_fam != Some(i) {
                        self.cf_fam = Some(i);
                        self.chord_q.clear();
                    }
                }
            });
            if let Some(f) = self.cf_fam {
                rule(ui);
                dim_line(ui, "3  CHORD", 1.0, pal().ink2);
                ui.horizontal_wrapped(|ui| {
                    let rn = theory::note_name(root, false);
                    for (_, suffix) in theory::FINDER_FAMILIES[f.min(theory::FINDER_FAMILIES.len() - 1)].1 {
                        let name = format!("{}{}", rn, suffix);
                        if retro_btn(ui, &name, cur.is_some() && suffix_now == *suffix).clicked() {
                            self.chord_q = name.clone();
                            if let Some(c) = theory::chord_notes(&name) {
                                self.hear(&piano_voicing(&c));
                            }
                        }
                    }
                });
            }
            rule(ui);
        }
        ui.horizontal(|ui| {
            let w = ui.available_width() - 82.0;
            text_box(ui, &mut self.chord_q, w, "or type any chord: Dm7, G7#9, Fmaj7/A...");
            if retro_btn_w(ui, "STRUM", 74.0, false).clicked() {
                if let Some(c) = theory::chord_notes(&self.chord_q) {
                    self.hear(&piano_voicing(&c));
                }
            }
        });
        let Some(c) = theory::chord_notes(self.chord_q.trim()) else {
            ui.add_space(6.0);
            para(ui, "Pick a root, then a kind of chord, then a chord (or type one) to see it on the neck and the piano, its shapes and the scales that fit it.", pal().ink2);
            return;
        };
        ui.add_space(6.0);
        title_line(ui, self.chord_q.trim(), 4.0, pal().ink);
        let notes: Vec<String> = c.named.iter().map(|(n, i)| format!("{} ({})", n, i)).collect();
        para(ui, &notes.join("   "), pal().ink2);
        section_header(ui, "ON THE NECK");
        let w = ui.available_width();
        let (nr, _) = ui.allocate_exact_size(Vec2::new(w, 120.0), Sense::hover());
        neck(ui, Rect::from_min_max(nr.min, Pos2::new(nr.max.x, nr.max.y - 12.0)), &[None; 6], 12, false, "find_neck", Some(c.root), &c.pcs, true);
        section_header(ui, "ON THE PIANO");
        let (pr, _) = ui.allocate_exact_size(Vec2::new(w, 80.0), Sense::hover());
        piano_span(ui, pr.shrink2(Vec2::new(2.0, 4.0)), &piano_voicing(&c), false, "find_piano", Some(c.root), 48, 2);
        section_header(ui, "GUITAR");
        let q = self.chord_q.trim().to_string();
        let shapes = self.shapes_for(&q, &c);
        if shapes.is_empty() {
            para(ui, "No comfortable shape in reach for this one.", pal().ink2);
        }
        self.grip_cards(ui, &c, &shapes, false);
        section_header(ui, "SCALES THAT FIT");
        for (name, steps) in theory::scales_for(&c) {
            let names: Vec<&str> = steps.iter().map(|s| theory::note_name(c.root + s, false)).collect();
            dim_line(ui, &format!("{} {}", theory::note_name(c.root, false), name.to_uppercase()), 2.0, pal().ink);
            dim_line(ui, &names.join(" "), 1.0, pal().ink2);
        }
    }

    fn chord_analyze(&mut self, ui: &mut egui::Ui) {
        para(ui, "Lay notes down on the neck: one per string, left of the nut is the open string, tap a note again to lift it. Each one sounds as you place it, and the chord is worked out as you go.", pal().ink2);
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
        let (nr, _) = ui.allocate_exact_size(Vec2::new(w, 140.0), Sense::hover());
        let neck_rect = Rect::from_min_max(nr.min, Pos2::new(nr.max.x, nr.max.y - 12.0));
        let shape = self.an_frets;
        if let Some((s, f)) = neck_alpha(ui, neck_rect, &shape, 12, true, "an_neck", root, &ghost, false, 1.0) {
            self.an_frets[s] = f;
            if let Some(f) = f {
                self.hear(&[theory::STANDARD[s] + f as i32]);
            }
        }
        let (pr, _) = ui.allocate_exact_size(Vec2::new(w, 64.0), Sense::hover());
        piano_span(ui, pr, &notes, false, "an_piano", root, 36, 4);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "STRUM", false).clicked() {
                self.hear(&notes);
            }
            if retro_btn(ui, "CLEAR", false).clicked() {
                self.an_frets = [None; 6];
                self.an_pick = None;
            }
            // the whole shape a fret down or up the neck
            let frets: Vec<u8> = self.an_frets.iter().flatten().copied().collect();
            let (lo, hi) = (frets.iter().min().copied(), frets.iter().max().copied());
            for (label, d) in [("< MOVE", -1i32), ("MOVE >", 1)] {
                let can = matches!((lo, hi), (Some(l), Some(h)) if (d < 0 && l > 0) || (d > 0 && h < 15));
                if retro_btn(ui, label, false).clicked() && can {
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
        dim_line(ui, &format!("NOTES: {}", if names.is_empty() { "-".to_string() } else { names.join("  ") }), 1.0, pal().ink2);
        section_header(ui, "POSSIBLE CHORDS");
        // how the names are written, and the details
        ui.horizontal_wrapped(|ui| {
            let sp = &mut self.chord_sp;
            if retro_btn(ui, "- \u{0394} \u{00b0} +", sp.symbols).clicked() {
                sp.symbols = !sp.symbols;
            }
            if retro_btn(ui, "#", sp.sharps).clicked() {
                sp.sharps = !sp.sharps;
            }
            if retro_btn(ui, "SLASH", sp.slash).clicked() {
                sp.slash = !sp.slash;
            }
            if retro_btn(ui, "DETAILED", sp.detailed).clicked() {
                sp.detailed = !sp.detailed;
            }
            if retro_btn(ui, "DETAILS", self.chord_details).clicked() {
                self.chord_details = !self.chord_details;
                self.save_settings();
            }
        });
        let sp = self.chord_sp;
        if let Some(s) = &shown {
            dim_line(ui, &format!("SHOWING {}  -  TAP ONE TO KEEP IT", s.name(sp)), 1.0, pal().dim);
        }
        if found.is_empty() {
            para(ui, if pcs.len() < 2 { "Lay down two or more notes." } else { "No chord has all of these notes." }, pal().ink2);
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
                if retro_btn(ui, &s.name(sp), s.exact || picked).clicked() {
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
            section_header(ui, &format!("GRIPS FOR {}", s.name(sp)));
            if shapes.is_empty() {
                para(ui, "No comfortable grip in reach for this one.", pal().ink2);
            }
            self.grip_cards(ui, &c, &shapes, true);
        }
        // the details: each chord broken down, with its warnings
        if self.chord_details && !found.is_empty() {
            section_header(ui, "DETAILS");
            for s in found.iter().take(12) {
                let notes: Vec<String> = s
                    .pcs
                    .iter()
                    .map(|p| {
                        let mark = if s.missing.contains(p) { "-" } else { "+" };
                        format!("{}{} ({})", mark, theory::note_name(*p, sp.sharps), degree(*p, s.root).0)
                    })
                    .collect();
                dim_line(ui, &format!("{}{}", s.name(sp), if s.exact { "   EXACT" } else { "" }), 2.0, pal().ink);
                para(ui, &notes.join("  "), pal().ink2);
                for w in &s.warnings {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
                        warn_triangle(ui.painter(), r);
                        ui.vertical(|ui| para(ui, w, pal().ink2));
                    });
                }
            }
            dim_line(ui, "+ YOU PLAY IT   - IT IS NOT PLAYED", 1.0, pal().dim);
        }
        // the colour key
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for (name, pc) in [("ROOT", 0), ("3RD", 4), ("5TH", 7), ("6TH / 7TH", 10), ("9 / 11", 2)] {
                let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 6.0, degree(pc, 0).1);
                label(ui, name, 70.0);
            }
        });
        SEVENTH.with(|c| c.set(None));
    }

    /// TRIADS: one triad, every inversion up the neck on the strings you pick, the whole neck lit under them.
    fn triads(&mut self, ui: &mut egui::Ui) {
        para(ui, "One triad, every inversion up the neck on the strings you pick. Root position, then the 3rd on the bottom, then the 5th, and round again an octave up. Tap a dot or a box to pick a shape.", pal().ink2);
        ui.horizontal_wrapped(|ui| {
            if let Some(k) = dropdown(ui, "tr_key", "KEY", &KEYS, self.tr_root as usize, 90.0) {
                self.tr_root = k as i32;
            }
            let sets: Vec<&str> = SETS.iter().map(|s| s.0).collect();
            if let Some(i) = dropdown(ui, "tr_set", "", &sets, self.tr_set, 150.0) {
                self.tr_set = i;
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (i, (name, _)) in QUALITIES.iter().enumerate() {
                if retro_btn(ui, name, self.tr_q == i).clicked() {
                    self.tr_q = i;
                }
            }
        });
        let root = self.tr_root;
        let q = QUALITIES[self.tr_q.min(5)].1;
        let pcs: Vec<i32> = q.iter().map(|x| (root + x).rem_euclid(12)).collect();
        let grips = theory::grips_on(&pcs, SETS[self.tr_set.min(SETS.len() - 1)].1, false, 4);
        if grips.is_empty() {
            para(ui, "No shape for this on these strings in one hand-span.", pal().ink2);
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
            if retro_btn(ui, "HEAR THEM UP THE NECK", false).clicked() {
                let mut all = Vec::new();
                for g in &grips {
                    all.extend(grip_notes(g));
                }
                // one after another, a beat apart
                let seq: Vec<Vec<i32>> = grips.iter().map(grip_notes).collect();
                let mut pcm: Vec<i16> = Vec::new();
                for n in &seq {
                    let mut c = band::chord_sound(n);
                    c.resize((band::RATE * 0.7) as usize, 0);
                    pcm.extend(c);
                }
                self.player.send(Cmd::Once(pcm, 0.9));
            }
            label(ui, "R ROOT   3 THIRD   5 FIFTH", 200.0);
        });
        // the shapes as boxes, two to a row
        let w = ((ui.available_width() - 10.0) / 2.0).min(150.0);
        let mut pick = None;
        for (row, chunk) in grips.chunks(2).enumerate() {
            ui.horizontal(|ui| {
                for (k, g) in chunk.iter().enumerate() {
                    let i = row * 2 + k;
                    let bass = grip_notes(g).iter().min().map(|m| (m - root).rem_euclid(12)).unwrap_or(0);
                    let inv = if bass == 0 {
                        "ROOT POSITION"
                    } else if bass == q[1] {
                        "3RD ON THE BOTTOM"
                    } else {
                        "5TH ON THE BOTTOM"
                    };
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, w * 1.3), Sense::click());
                    if i == self.tr_sel {
                        outline(ui.painter(), r, 2.0, pal().red);
                    }
                    ptext_fit(ui.painter(), Pos2::new(r.center().x, r.min.y + 8.0), Align::Center, inv, 1.0, r.width() - 8.0, pal().ink);
                    let boxr = Rect::from_min_max(r.min + Vec2::new(0.0, 14.0), r.max);
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
}
