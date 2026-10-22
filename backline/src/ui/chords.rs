//! The CHORDS tab (under PRACTICE), in the spirit of Oolimo: FIND a chord by name (its notes, intervals, piano
//! keys, guitar shapes and the scales that fit), or ANALYZE notes you click on a guitar neck or a piano (it names
//! the chord). The same piano and guitar pictures follow the band as it plays a chart (the instrument view).

use super::*;
pub(crate) use crate::fretboard::*;
use crate::theory;
use crate::views::field;

impl App {
    /// The CHORDS tab.
    pub(crate) fn chords_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        crate::tour::mark("CHORDSVIEW", ui.clip_rect());
        // a sequence playing: keep redrawing so the chord sounding now lights up
        self.seq_tick(ui.ctx());
        ui.add_space(6.0);
        // three groups on top, the pages of the open one under them (each page keeps its own number)
        const GROUPS: [(&str, &[(u8, &str)]); 3] = [
            ("FIND & NAME", &[(0, "FIND A CHORD"), (1, "CHORD ANALYZER")]),
            ("SHAPES", &[(2, "TRIADS"), (3, "VOICINGS"), (4, "CLUSTERS")]),
            ("HARMONY", &[(6, "SCALE HARMONY"), (5, "PASSING CHORDS"), (7, "MOVES (EXPERIMENTAL)")]),
        ];
        let group = GROUPS.iter().position(|g| g.1.iter().any(|p| p.0 == self.chords_tab)).unwrap_or(0);
        let names: Vec<&str> = GROUPS.iter().map(|g| g.0).collect();
        if let Some(g) = tab_row(ui, &names, group) {
            if g != group {
                self.chords_tab = GROUPS[g].1[0].0;
            }
        }
        ui.horizontal_wrapped(|ui| {
            for (t, name) in GROUPS[group].1.iter() {
                if retro_btn(ui, name, *t == self.chords_tab).clicked() {
                    self.chords_tab = *t;
                }
            }
        });
        ui.add_space(6.0);
        match self.chords_tab {
            0 => self.chord_find(ui, acts),
            1 => self.chord_analyze(ui, acts),
            2 => self.triads(ui, acts),
            3 => self.voicings(ui, acts),
            4 => self.clusters(ui, acts),
            5 => self.passing(ui, acts),
            6 => self.scale_harmony(ui, acts),
            _ => self.moves(ui, acts),
        }
    }

    fn chord_find(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        // a funnel: the root, then the kind of chord, then that kind's chords - each step opens under the one before
        let cur = theory::chord_notes(self.chord_q.trim());
        let suffix_now = crate::chart::split_root(self.chord_q.trim()).1.to_string();
        // a chord typed or picked elsewhere sets the funnel too
        if let Some(c) = &cur {
            self.cf_root = Some(c.root);
            if let Some(f) = theory::FINDER_FAMILIES.iter().position(|(_, l)| l.iter().any(|(_, s)| *s == suffix_now)) {
                self.cf_fam = Some(f);
            }
        }
        let rule = |ui: &mut egui::Ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 9.0), Sense::hover());
            ui.painter().line_segment(
                [Pos2::new(r.min.x + 4.0, r.center().y), Pos2::new(r.max.x - 4.0, r.center().y)],
                Stroke::new(1.0_f32, pal().edge),
            );
        };
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, "1  ROOT", 90.0);
            for pc in 0..12 {
                let name = theory::note_name(pc, false);
                if retro_btn_w(ui, name, 40.0, self.cf_root == Some(pc)).clicked() {
                    self.cf_root = Some(pc);
                    // the same kind of chord on the new root, when one was picked
                    if cur.is_some() && self.cf_fam.is_some() {
                        self.chord_q = format!("{}{}", name, suffix_now);
                    }
                }
            }
        });
        if let Some(root) = self.cf_root {
            rule(ui);
            // (a kind picked from a list, the first line asking until one is)
            let mut kinds: Vec<&str> = Vec::new();
            if self.cf_fam.is_none() {
                kinds.push("PICK ONE");
            }
            kinds.extend(theory::FINDER_FAMILIES.iter().map(|(n, _)| *n));
            let off = usize::from(self.cf_fam.is_none());
            if let Some(i) = crate::choice_line(ui, "cf_fam", "2  KIND", &kinds, self.cf_fam.map_or(0, |f| f + off), 300.0) {
                if i >= off && self.cf_fam != Some(i - off) {
                    self.cf_fam = Some(i - off);
                    self.chord_q.clear();
                }
            }
            if let Some(f) = self.cf_fam {
                rule(ui);
                ui.horizontal_wrapped(|ui| {
                    crate::views::label(ui, "3  CHORD", 90.0);
                    let rn = theory::note_name(root, false);
                    for (_, suffix) in theory::FINDER_FAMILIES[f.min(theory::FINDER_FAMILIES.len() - 1)].1 {
                        let name = format!("{}{}", rn, suffix);
                        if retro_btn(ui, &name, cur.is_some() && suffix_now == *suffix).clicked() {
                            self.chord_q = name.clone();
                            if let Some(c) = theory::chord_notes(&name) {
                                acts.push(Action::PlayNotes(piano_voicing(&c)));
                            }
                        }
                    }
                });
            }
            rule(ui);
        }
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 90.0).max(120.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let _ = field(
                ui,
                &mut self.ed,
                crate::tools::F_CHORD,
                &mut self.chord_q,
                rect,
                "or type any chord, like Dm7, G7#9 or Fmaj7/A",
                false,
            );
            if retro_btn_w(ui, "STRUM", 80.0, false).tip("Hear it").clicked() {
                if let Some(c) = theory::chord_notes(&self.chord_q) {
                    acts.push(Action::PlayNotes(piano_voicing(&c)));
                }
            }
        });
        let Some(c) = theory::chord_notes(self.chord_q.trim()) else {
            ui.add_space(8.0);
            para(ui, "Pick a root, a kind and a chord, or type one. You see it on the neck and the piano, with its shapes and the scales that fit.", pal().ink2);
            return;
        };
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(8.0);
            title_line(ui, self.chord_q.trim(), 4.0, pal().ink);
            let notes: Vec<String> = c.named.iter().map(|(n, i)| format!("{} ({})", n, i)).collect();
            title_line(ui, &notes.join("   "), 2.0, pal().ink2);
            ui.add_space(8.0);
            section_header(ui, "ON THE NECK");
            let w = ui.available_width();
            let (nr, _) = ui.allocate_exact_size(Vec2::new(w, 132.0 * self.neck_zoom), Sense::hover());
            let nr = nr.shrink2(Vec2::new(4.0, 6.0));
            let seventh = c.pcs.iter().any(|p| matches!((p - c.root).rem_euclid(12), 10 | 11));
            SEVENTH.with(|s| s.set(seventh.then_some(c.root.rem_euclid(12))));
            neck(
                ui,
                Rect::from_min_max(nr.min, Pos2::new(nr.max.x, nr.max.y - 12.0)),
                &[None; 6],
                15,
                false,
                "find_neck",
                Some(c.root),
                &c.pcs,
                true,
            );
            // the chord on a piano, to look at (not to click)
            section_header(ui, "ON THE PIANO AND THE STAFF");
            let voiced = piano_voicing(&c);
            ui.horizontal(|ui| {
                let pw = (ui.available_width() - 270.0).clamp(240.0, 560.0);
                let (pr, _) = ui.allocate_exact_size(Vec2::new(pw, 96.0), Sense::hover());
                // (over a seventh the piano names the sixth 13, as the staff and the grips do)
                let seventh = c.pcs.iter().any(|p| matches!((p - c.root).rem_euclid(12), 10 | 11));
                SEVENTH.with(|s| s.set(seventh.then_some(c.root.rem_euclid(12))));
                piano_span(ui, pr.shrink2(Vec2::new(4.0, 4.0)), &voiced, false, "find_piano", Some(c.root), 48, 2);
                SEVENTH.with(|s| s.set(None));
                let (sr, _) = ui.allocate_exact_size(Vec2::new(260.0, 96.0), Sense::hover());
                let pcs = c.pcs.clone();
                let notes: Vec<(i32, Option<(usize, u8)>)> = voiced.iter().map(|m| (*m, None)).collect();
                crate::lines_ui::chord_staff(ui, sr, &notes, lines::key_sharps(c.root), &|m| {
                    let (d, col) = degree_in(m.rem_euclid(12), c.root, &pcs);
                    (d.to_string(), col)
                });
            });
            ui.add_space(6.0);
            section_header(ui, "GUITAR");
            let q = self.chord_q.trim().to_string();
            let shapes = self.shapes_for(&q, &c);
            if shapes.is_empty() && !self.store.custom_grips.contains_key(&grip_key(&q)) {
                para(ui, "No comfortable shape in reach for this one.", pal().ink2);
            }
            self.chord_grips(ui, acts, &grip_key(&q), &c, &shapes, false);
            if retro_btn(ui, "+ ADD MY OWN GRIP", false).tip("Lay it on the neck in CHORD ANALYZER, then save it here.").clicked()
            {
                self.grip_for = Some((grip_key(&q), q.clone()));
                self.an_frets = [None; 6];
                self.chords_tab = 1;
            }
            ui.add_space(6.0);
            section_header(ui, "SCALES THAT FIT");
            // spelt from the chord's root as written (F# Dorian: F# G# A B C# D# E)
            let rw = theory::written_root(&q);
            for (name, steps) in theory::scales_for(&c) {
                let names = theory::scale_names(&rw, c.root, steps);
                ui.horizontal(|ui| {
                    crate::views::label(ui, &format!("{} {}", names[0], name), 260.0);
                    crate::views::label(ui, &names.join(" "), 300.0);
                });
            }
            ui.add_space(20.0);
        });
    }

    fn chord_analyze(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Click notes on the neck and the chord is named as you go. Use one note per string; left of the nut is the open string. Click a note again to lift it. Each note sounds as you place it.", pal().ink2);
        // laying down a grip for a chord from the finder
        if let Some((key, name)) = self.grip_for.clone() {
            ui.horizontal_wrapped(|ui| {
                crate::views::label(ui, &format!("YOUR GRIP FOR {}:", name.to_uppercase()), 200.0);
                let ready = self.an_frets.iter().flatten().count() >= 2;
                if retro_btn(ui, "SAVE IT", false).tip("Save the grip on the neck under this chord.").clicked() && ready {
                    self.store.custom_grips.entry(key.clone()).or_default().push(self.an_frets);
                    self.store_dirty = true;
                    self.grip_for = None;
                    self.chords_tab = 0;
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.grip_for = None;
                    self.chords_tab = 0;
                }
            });
        }
        // the notes so far, lowest first, and what they could be
        let notes: Vec<i32> =
            self.an_frets.iter().enumerate().filter_map(|(i, f)| f.map(|f| theory::STANDARD[i] + f as i32)).collect();
        let pcs: Vec<i32> = notes.iter().map(|m| m.rem_euclid(12)).collect();
        let bass = notes.first().map(|m| m.rem_euclid(12));
        let found = theory::suggestions(&pcs, bass);
        // the reading on show: the one hovered, else the one picked, else the best
        let pick = |k: &Option<String>| k.as_ref().and_then(|k| found.iter().find(|s| &s.key() == k)).cloned();
        let shown = pick(&self.an_hover).or_else(|| pick(&self.an_pick)).or_else(|| found.first().cloned());
        let root = shown.as_ref().map(|s| s.root).or(bass);
        // for this frame's labels: a seventh over the root makes the sixth a 13 (set again every frame, cleared below)
        SEVENTH.with(|c| c.set(root.filter(|rt| pcs.iter().any(|p| matches!((p - rt).rem_euclid(12), 10 | 11)))));
        struct Clear;
        impl Drop for Clear {
            fn drop(&mut self) {
                SEVENTH.with(|c| c.set(None));
            }
        }
        let _clear = Clear;
        let ghost: Vec<i32> = shown.as_ref().map(|s| s.missing.clone()).unwrap_or_default();
        // the neck, as big as you like
        let zoom = self.neck_zoom;
        ui.horizontal(|ui| {
            crate::views::label(ui, "NECK SIZE", 100.0);
            if retro_btn_w(ui, "-", 28.0, false).clicked() {
                self.neck_zoom = (self.neck_zoom - 0.15).max(0.8);
                self.dirty = true;
            }
            if retro_btn_w(ui, "+", 28.0, false).clicked() {
                self.neck_zoom = (self.neck_zoom + 0.15).min(2.2);
                self.dirty = true;
            }
        });
        let w = ui.available_width();
        let (nr, _) = ui.allocate_exact_size(Vec2::new(w, 132.0 * zoom), Sense::hover());
        let nr = nr.shrink2(Vec2::new(4.0, 6.0));
        let shape = self.an_frets;
        let neck_rect = Rect::from_min_max(nr.min, Pos2::new(nr.max.x, nr.max.y - 12.0));
        let alpha = self.ghost_alpha;
        if let Some((s, f)) = neck_alpha(ui, neck_rect, &shape, 15, true, "an_neck", root, &ghost, false, alpha) {
            self.an_frets[s] = f;
            if let Some(f) = f {
                acts.push(Action::PlayNotes(vec![theory::STANDARD[s] + f as i32]));
            }
        }
        ui.add_space(8.0);
        // the same notes on a piano, as a picture (four octaves cover the guitar)
        let shape_now = self.an_frets;
        ui.horizontal(|ui| {
            let pw = (ui.available_width() - 270.0).clamp(240.0, 720.0);
            let (pr, _) = ui.allocate_exact_size(Vec2::new(pw, 96.0), Sense::hover());
            piano_span(ui, pr.shrink2(Vec2::new(0.0, 13.0)), &notes, false, "an_piano", root, 36, 4);
            let (sr, _) = ui.allocate_exact_size(Vec2::new(260.0, 96.0), Sense::hover());
            let staffed: Vec<(i32, Option<(usize, u8)>)> =
                (0..6).filter_map(|s| shape_now[s].map(|f| (theory::STANDARD[s] + f as i32, Some((s, f))))).collect();
            let pcs: Vec<i32> = staffed.iter().map(|n| n.0.rem_euclid(12)).collect();
            let r = root.unwrap_or_else(|| staffed.first().map_or(0, |n| n.0.rem_euclid(12)));
            crate::lines_ui::chord_staff(ui, sr, &staffed, lines::key_sharps(r), &|m| {
                let (d, col) = degree_in(m.rem_euclid(12), r, &pcs);
                (d.to_string(), col)
            });
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if retro_btn(ui, "STRUM", false).tip("Hear the chord, low string to high").clicked() && !notes.is_empty() {
                acts.push(Action::PlayNotes(notes.clone()));
            }
            if retro_btn(ui, "CLEAR", false).clicked() {
                self.an_frets = [None; 6];
                self.an_pick = None;
            }
            // the whole shape a fret down or up the neck (open strings move too, as with a capo), heard as it lands
            let frets: Vec<u8> = self.an_frets.iter().flatten().copied().collect();
            let lo = frets.iter().min().copied();
            let hi = frets.iter().max().copied();
            for (label, d, tip) in
                [("< MOVE", -1i32, "Move the whole shape down a fret"), ("MOVE >", 1, "Move the whole shape up a fret")]
            {
                let can = match (lo, hi) {
                    (Some(l), Some(h)) => (d < 0 && l > 0) || (d > 0 && h < 15),
                    _ => false,
                };
                if retro_btn(ui, label, false).tip(tip).clicked() && can {
                    for f in self.an_frets.iter_mut().flatten() {
                        *f = (*f as i32 + d) as u8;
                    }
                    let moved: Vec<i32> =
                        (0..6).filter_map(|s| self.an_frets[s].map(|f| theory::STANDARD[s] + f as i32)).collect();
                    acts.push(Action::PlayNotes(moved));
                }
            }
            if retro_btn(ui, "SAVE GRIP AS", self.gs_open).tip("Save this grip under a chord name you type or pick.").clicked() {
                self.gs_open = !self.gs_open;
                // start from what the analyzer thinks it is
                if self.gs_open {
                    self.gs_name = shown.as_ref().map(|s| s.key()).unwrap_or_default();
                }
            }
            let mut v: Vec<i32> = pcs.clone();
            v.sort();
            v.dedup();
            let names: Vec<String> = v
                .iter()
                .map(|p| match root {
                    Some(rt) => format!("{} ({})", theory::note_name(*p, self.chord_sp.sharps), degree(*p, rt).0),
                    None => theory::note_name(*p, self.chord_sp.sharps).to_string(),
                })
                .collect();
            crate::views::label(
                ui,
                &format!("NOTES: {}", if names.is_empty() { "-".to_string() } else { names.join("  ") }),
                420.0,
            );
        });
        if self.gs_open {
            self.save_grip_panel(ui);
        }
        section_header(ui, "POSSIBLE CHORDS");
        // how the names are written, and the details
        ui.horizontal_wrapped(|ui| {
            let sp = &mut self.chord_sp;
            let toggle = |ui: &mut egui::Ui, on: &mut bool, label: &str, tip: &str| {
                if retro_btn(ui, label, *on).tip(tip).clicked() {
                    *on = !*on;
                }
            };
            toggle(
                ui,
                &mut sp.symbols,
                "- \u{0394} \u{00b0} +",
                "Jazz symbols: - minor, \u{0394} major 7, \u{00b0} diminished, + augmented. Off: m, maj, dim, aug.",
            );
            toggle(ui, &mut sp.sharps, "#", "Write roots with sharps. Off: flats.");
            toggle(ui, &mut sp.slash, "SLASH", "Write a bass that isn't the root as a slash chord, like C/E.");
            toggle(ui, &mut sp.detailed, "DETAILED", "Name the notes left out, like C7(no5).");
            toggle(
                ui,
                &mut self.chord_details,
                "DETAILS",
                "Break down each chord: its notes, which ones you play, and why some carry a warning.",
            );
        });
        let sp = self.chord_sp;
        if let Some(s) = &shown {
            crate::views::dim_line(
                ui,
                &format!("SHOWING {}.  HOVER ONE TO SEE IT, CLICK TO KEEP IT", s.name(sp)),
                1.0,
                pal().dim,
            );
        }
        if found.is_empty() {
            para(ui, if pcs.len() < 2 { "Lay down two or more notes." } else { "No chord has all of these notes." }, pal().ink2);
        }
        let mut hovered: Option<String> = None;
        ui.horizontal_wrapped(|ui| {
            for s in &found {
                let key = s.key();
                // a little warning triangle before a chord that might not work
                if !s.warnings.is_empty() {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(16.0, bh()), Sense::hover());
                    warn_triangle(ui.painter(), r);
                    resp.tip(s.warnings.join("\n"));
                }
                let picked = self.an_pick.as_deref() == Some(key.as_str());
                let r = retro_btn(ui, &s.name(sp), s.exact || picked).tip(if s.exact {
                    "These notes make this chord"
                } else {
                    "These notes are part of this chord. The faint rings show the rest."
                });
                if r.hovered() {
                    hovered = Some(key.clone());
                }
                if r.clicked() {
                    self.an_pick = if picked { None } else { Some(key) };
                }
                ui.add_space(4.0);
            }
        });
        self.an_hover = hovered;
        // a chord you clicked: grips that play it, anywhere on the neck (click one to hear it)
        if let Some(s) = pick(&self.an_pick) {
            let mut pcs = s.pcs.clone();
            let bass = s.bass.unwrap_or(s.root);
            if !pcs.contains(&bass) {
                pcs.insert(0, bass);
            }
            let c = theory::ChordNotes { root: s.root, bass, pcs, named: Vec::new() };
            let shapes = self.shapes_for(&s.key(), &c);
            ui.add_space(6.0);
            section_header(ui, &format!("GRIPS FOR {}", s.name(sp)));
            let key = grip_key(&s.key());
            if shapes.is_empty() && !self.store.custom_grips.contains_key(&key) {
                para(ui, "No comfortable grip in reach for this one.", pal().ink2);
            }
            self.chord_grips(ui, acts, &key, &c, &shapes, true);
            let mine = self.store.custom_grips.get(&key).is_some_and(|v| v.contains(&self.an_frets));
            if self.an_frets.iter().flatten().count() >= 2
                && !mine
                && self.grip_for.is_none()
                && retro_btn(ui, &format!("+ ADD AS MY GRIP FOR {}", s.name(sp)), false)
                    .tip("Save the grip on the neck under this chord. It shows as MINE; right-click it to delete.")
                    .clicked()
            {
                self.store.custom_grips.entry(key).or_default().push(self.an_frets);
                self.store_dirty = true;
            }
        }
        // the details: each chord broken down
        if self.chord_details && !found.is_empty() {
            ui.add_space(6.0);
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
                ui.horizontal_wrapped(|ui| {
                    crate::views::label(ui, &s.name(sp), 120.0);
                    crate::views::label(ui, &notes.join("  "), 360.0);
                    if s.exact {
                        crate::views::label(ui, "EXACT", 70.0);
                    }
                });
                for w in &s.warnings {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
                        warn_triangle(ui.painter(), r);
                        crate::views::dim_line(ui, w, 1.0, pal().ink2);
                    });
                }
            }
            crate::views::dim_line(ui, "+ YOU PLAY IT   - IT IS NOT PLAYED", 1.0, pal().dim);
        }
        // the colour key
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            for (name, pc) in [("ROOT", 0), ("3RD", 4), ("5TH", 7), ("6TH / 7TH", 10), ("9 / 11", 2)] {
                let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 6.0, degree(pc, 0).1);
                crate::views::label(ui, name, 90.0);
            }
        });
    }

    /// A row of the twelve notes to pick from.
    pub(crate) fn note_row(ui: &mut egui::Ui, label: &str, cur: i32, sharps: bool) -> Option<i32> {
        let names: Vec<&str> = (0..12).map(|pc| theory::note_name(pc, sharps)).collect();
        crate::choice_line(ui, &format!("note_{}", label), label, &names, cur.rem_euclid(12) as usize, 90.0).map(|i| i as i32)
    }

    /// The changes to practise over: the open tune's chart, else the ones typed here (Night & Day's to start).
    pub(crate) fn practice_changes(&mut self, ui: &mut egui::Ui) -> (String, Vec<String>) {
        let from_chart: Option<(String, Vec<String>)> = self.chart_text().map(|text| {
            let chart = crate::chart::parse_friendly(&text);
            let order = chart.play_order();
            let name = self.chart_tune().map(|i| self.store.tunes[i].name.clone()).unwrap_or_else(|| "THIS SONG".to_string());
            (name, order.iter().flat_map(|b| chart.bars[*b].chords.clone()).collect())
        });
        match from_chart.filter(|c| !c.1.is_empty()) {
            Some(c) => c,
            None => {
                para(ui, "Open a tune with a chart in TUNES, or type some changes here.", pal().ink2);
                let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width() - 6.0, bh()), Sense::hover());
                let _ = field(ui, &mut self.ed, crate::tools::F_CL_CHANGES, &mut self.cl_changes, r, "Dm7b5 G7 Cmaj7", false);
                ("TYPED CHANGES".to_string(), self.cl_changes.split_whitespace().map(|s| s.to_string()).collect())
            }
        }
    }

    /// Which chord of a played sequence is sounding now (for the one started with this tag).
    pub(crate) fn seq_now(&self, tag: &str) -> Option<usize> {
        let (t0, gap, tg, len) = self.seq_play.as_ref()?;
        if tg != tag {
            return None;
        }
        let i = (t0.elapsed().as_secs_f32() / gap.max(0.05)) as usize;
        (i < *len).then_some(i)
    }

    /// Whether the sequence started with this tag is playing - the whole time, the pause between rounds of a loop
    /// too (it is let go when it ends or is stopped).
    pub(crate) fn seq_on(&self, tag: &str) -> bool {
        self.seq_play.as_ref().is_some_and(|p| p.2 == tag)
    }

    /// A play button for a sequence that turns into STOP while it plays.
    pub(crate) fn seq_button(
        &self,
        ui: &mut egui::Ui,
        acts: &mut Vec<Action>,
        label: &str,
        tip: &str,
        tag: &str,
        seq: impl FnOnce() -> Vec<Vec<i32>>,
        gap: f32,
    ) {
        if self.seq_on(tag) {
            if retro_btn(ui, "STOP", true).clicked() {
                acts.push(Action::StopSequence);
            }
        } else if retro_btn(ui, label, false).tip(tip).clicked() {
            let s = seq();
            if !s.is_empty() {
                acts.push(Action::PlaySequenceAt(s, gap, tag.to_string()));
            }
        }
    }

    /// The voicing family tabs and string sets (shared by VOICINGS and PASSING CHORDS): (family, strings, notes
    /// dropped, reach).
    fn voicing_picker(&mut self, ui: &mut egui::Ui, about: bool) -> (usize, &'static [usize], &'static [usize], u8) {
        self.voicing_picker_from(ui, about, 0)
    }

    /// The voicing picker without its first `first` families (VOICINGS leaves the triads to TRIADS).
    fn voicing_picker_from(
        &mut self,
        ui: &mut egui::Ui,
        about: bool,
        first: usize,
    ) -> (usize, &'static [usize], &'static [usize], u8) {
        const FAMILIES: [&str; 8] = ["TRIADS", "SPREAD TRIADS", "SHELLS", "CLOSE", "DROP 2", "DROP 3", "DROP 2&4", "3RDS & 7THS"];
        const ABOUT: [&str; 8] = [
            "Three notes on three neighbouring strings. Learn all three inversions (root, 3rd or 5th in the bass) on every string set. Then you can find any major or minor chord anywhere on the neck.",
            "A triad with its middle note dropped an octave, so it skips a string. Wide and open, like a pianist's two hands.",
            "Root, 3rd and 7th: the notes that say what a chord is. The bread and butter of comping, with the root on the 6th or 5th string.",
            "All four notes as close as they go. Lovely on piano, a stretch on guitar. That is why drop voicings exist.",
            "A close voicing with its second highest note dropped an octave. Four neighbouring strings, easy to grip. These are the most used jazz chord shapes there are.",
            "A close voicing with its third highest note dropped an octave. The bass sits on the 6th or 5th string, with a string skipped. A big, full sound for chord melody and comping.",
            "The second and fourth highest notes dropped an octave. Wide and open across two string groups.",
            "Just the 3rd and 7th: the notes that say major, minor or dominant. Through a ii-V-I, the 7th falls a half step to the next 3rd and the 3rd stays put as the next 7th. The hand hardly moves. Use it for light comping; it is the bones of every line.",
        ];
        if (self.vc_family as usize) < first {
            self.vc_family = first as u8;
        }
        if let Some(v) = tab_row(ui, &FAMILIES[first..], self.vc_family as usize - first) {
            self.vc_family = (v + first) as u8;
            self.vc_set = 0;
        }
        let fam = (self.vc_family as usize).min(7);
        if about {
            para(ui, ABOUT[fam], pal().ink2);
        }
        let sets: &'static [&'static [usize]] = match fam {
            0 => &[&[3, 4, 5], &[2, 3, 4], &[1, 2, 3], &[0, 1, 2]],
            1 => &[&[2, 4, 5], &[1, 3, 4], &[0, 2, 3]],
            2 => &[&[0, 1, 2], &[1, 2, 3]],
            3 | 4 => &[&[2, 3, 4, 5], &[1, 2, 3, 4], &[0, 1, 2, 3]],
            5 => &[&[0, 2, 3, 4], &[1, 3, 4, 5]],
            7 => &[&[2, 3], &[3, 4], &[1, 2]],
            _ => &[&[0, 1, 3, 4], &[1, 2, 4, 5]],
        };
        let drop: &'static [usize] = match fam {
            1 | 4 => &[2],
            5 => &[3],
            6 => &[2, 4],
            _ => &[],
        };
        self.vc_set = self.vc_set.min(sets.len() - 1);
        // numbered the guitarist's way: 6 is the low E
        let labels: Vec<String> =
            sets.iter().map(|s| s.iter().map(|x| (6 - x).to_string()).collect::<Vec<_>>().join(" ")).collect();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        ui.horizontal_wrapped(|ui| {
            if let Some(i) = dropdown(ui, "vc_set", "STRINGS", &labels, self.vc_set, 0.0) {
                self.vc_set = i;
            }
        });
        (fam, sets[self.vc_set], drop, if fam == 3 { 5 } else { 4 })
    }

    /// Chords one after another in a voicing family on a string set, each the nearest grip to the one before (the
    /// way you would comp them): (name, grip, root, notes). Chords that cannot be read or reached are left out.
    pub(crate) fn led_grips(
        names: &[String],
        fam: usize,
        strings: &[usize],
        drop: &[usize],
        reach: u8,
    ) -> Vec<(String, [Option<u8>; 6], i32, Vec<i32>)> {
        let mut out = Vec::new();
        let mut at = 5.0_f32;
        for name in names {
            let Some(cn) = theory::chord_notes(name) else { continue };
            let bones = theory::chord_skeleton(&cn, if fam <= 1 { 3 } else { 4 });
            let options = Self::voicing_grips(fam, &bones, cn.root, strings, drop, reach);
            let best = options.iter().filter_map(|o| o.1).min_by(|a, b| {
                (neck_centre(a) - at).abs().partial_cmp(&(neck_centre(b) - at).abs()).unwrap_or(std::cmp::Ordering::Equal)
            });
            if let Some(g) = best {
                at = neck_centre(&g);
                out.push((name.clone(), g, cn.root, cn.pcs.clone()));
            }
        }
        out
    }

    /// A row of named grips, the one sounding now (`lit`) lit up; click one to hear it.
    fn grip_row(
        ui: &mut egui::Ui,
        acts: &mut Vec<Action>,
        grips: &[(String, [Option<u8>; 6], i32, Vec<i32>)],
        lit: Option<usize>,
    ) {
        // as many to a row as fit in what can be seen (the panel can be wider than its visible part)
        let seen = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x).max(120.0);
        let per = ((seen / 112.0).floor() as usize).max(1);
        for (row, chunk) in grips.chunks(per).enumerate() {
            ui.horizontal(|ui| {
                for (j, (name, g, root, pcs)) in chunk.iter().enumerate() {
                    let i = row * per + j;
                    ui.vertical(|ui| {
                        crate::views::label(ui, name, 100.0);
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(96.0, 116.0), Sense::click());
                        if lit == Some(i) {
                            ui.painter().rect_filled(r.expand(3.0), 4.0, pal().bar_txt.gamma_multiply(0.25));
                            ui.painter().rect_stroke(r.expand(3.0), 4.0, Stroke::new(2.5_f32, pal().bar_txt));
                        }
                        chord_box_labelled(ui.painter(), r, g, &|m| degree_in(m, *root, pcs).0.to_string());
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            acts.push(Action::PlayNotes(grip_notes(g)));
                        }
                    });
                }
            });
            ui.add_space(4.0);
        }
    }

    pub(crate) fn band_bpm(&self) -> f32 {
        (if self.bpm > 0 { self.bpm } else { self.mt_bpm }).max(40) as f32
    }

    /// VOICINGS: the voicings a jazz guitarist keeps under the fingers - triads, spread triads, shells, close and
    /// drop 2 / drop 3 / drop 2 & 4 seventh chords - every inversion on a string set, then walked through the changes
    /// of a tune with the smallest moves.
    fn voicings(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if let Some(v) = tab_row(ui, &["SHAPES", "WALK THE CHANGES"], self.vc_page as usize) {
            self.vc_page = v as u8;
        }
        ui.add_space(6.0);
        let (fam, strings, drop, reach) = self.voicing_picker_from(ui, true, 2);
        if self.vc_page == 1 {
            self.walk_changes(ui, acts, fam, strings, drop, reach);
            return;
        }
        let triads = fam <= 1;
        let sharps = self.chord_sp.sharps;
        const KEYS: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];
        let _ = sharps;
        let types: Vec<(&str, Vec<i32>)> = if triads {
            theory::TRIAD_TYPES.iter().map(|(n, v)| (*n, v.to_vec())).collect()
        } else {
            theory::VOICING_TYPES.iter().map(|(n, v)| (*n, v.to_vec())).collect()
        };
        self.vc_type = self.vc_type.min(types.len() - 1);
        let labels: Vec<String> =
            types.iter().map(|(n, _)| format!("{}{}", theory::note_name(self.vc_root, sharps), n)).collect();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        ui.horizontal_wrapped(|ui| {
            if let Some(k) = dropdown(ui, "vc_root", "ROOT", &KEYS, self.vc_root as usize, 0.0) {
                self.vc_root = k as i32;
            }
            if let Some(i) = dropdown(ui, "vc_type", "CHORD", &labels, self.vc_type, 0.0) {
                self.vc_type = i;
            }
        });
        let root = self.vc_root;
        let chord = types[self.vc_type].1.clone();
        let chord_pcs: Vec<i32> = chord.iter().map(|x| (root + x).rem_euclid(12)).collect();
        // every inversion (or for shells, both shapes) on these strings
        let grips = Self::voicing_grips(fam, &chord, root, strings, drop, reach);
        section_header(ui, &format!("{}{}", theory::note_name(root, sharps), types[self.vc_type].0));
        let mut up: Vec<[Option<u8>; 6]> = grips.iter().filter_map(|g| g.1).collect();
        up.sort_by_key(neck_spot);
        let lit_up = self.seq_now("inv").and_then(|i| up.get(i).copied());
        ui.horizontal(|ui| {
            self.seq_button(
                ui,
                acts,
                "PLAY THEM IN TURN",
                "Each inversion, up the neck",
                "inv",
                || up.iter().map(grip_notes).collect(),
                0.9,
            );
            crate::views::dim_line(ui, "CLICK A GRIP TO HEAR IT.  NUMBERS ARE CHORD DEGREES", 1.0, pal().dim);
        });
        ui.horizontal_wrapped(|ui| {
            for (gi, (label, g)) in grips.iter().enumerate() {
                // a thin line between grips
                if gi > 0 {
                    let (d, _) = ui.allocate_exact_size(Vec2::new(9.0, 270.0), Sense::hover());
                    ui.painter().line_segment(
                        [Pos2::new(d.center().x, d.min.y + 4.0), Pos2::new(d.center().x, d.max.y - 4.0)],
                        Stroke::new(1.0_f32, pal().dim),
                    );
                }
                ui.vertical(|ui| {
                    // what is on the bottom and on top, one above the other with a small rule between
                    let (lr, _) = ui.allocate_exact_size(Vec2::new(112.0, 32.0), Sense::hover());
                    let (bass, top) = label.split_once("  ").unwrap_or((label.as_str(), ""));
                    let p = ui.painter();
                    ptext(p, Pos2::new(lr.center().x, lr.min.y + 8.0), Align::Center, top, 2.0, pal().ink);
                    p.line_segment(
                        [Pos2::new(lr.center().x - 22.0, lr.min.y + 16.0), Pos2::new(lr.center().x + 22.0, lr.min.y + 16.0)],
                        Stroke::new(1.0_f32, pal().dim),
                    );
                    ptext(p, Pos2::new(lr.center().x, lr.min.y + 24.0), Align::Center, bass, 2.0, pal().ink2);
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(112.0, 132.0), Sense::click());
                    match g {
                        Some(g) => {
                            if lit_up == Some(*g) {
                                ui.painter().rect_filled(r.expand(3.0), 4.0, pal().bar_txt.gamma_multiply(0.25));
                                ui.painter().rect_stroke(r.expand(3.0), 4.0, Stroke::new(2.5_f32, pal().bar_txt));
                            }
                            chord_box_labelled(ui.painter(), r, g, &|m| degree_in(m, root, &chord_pcs).0.to_string());
                            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                acts.push(Action::PlayNotes(grip_notes(g)));
                            }
                            // the same grip on the staff, each note's degree beside it
                            let (sr, _) = ui.allocate_exact_size(Vec2::new(112.0, 96.0), Sense::hover());
                            let notes: Vec<(i32, Option<(usize, u8)>)> = grip_notes(g).into_iter().map(|m| (m, None)).collect();
                            crate::lines_ui::chord_staff(ui, sr, &notes, sharps, &|m| {
                                let (d, col) = degree_in(m.rem_euclid(12), root, &chord_pcs);
                                (d.to_string(), col)
                            });
                        }
                        None => {
                            ptext(ui.painter(), r.center(), Align::Center, "OUT OF REACH", 1.0, pal().dim);
                        }
                    }
                });
            }
        });

        // ---- the whole neck: each inversion in its own colour, wherever it fits (an octave up or down too); the one
        // sounding stands out
        mini_header(ui, "ACROSS THE NECK");
        const INV_COLS: [Color32; 4] = [
            Color32::from_rgb(217, 164, 65),
            Color32::from_rgb(95, 168, 211),
            Color32::from_rgb(214, 104, 83),
            Color32::from_rgb(111, 191, 115),
        ];
        let mut dots: Vec<crate::improv::NeckDot> = Vec::new();
        for (gi, (_, g)) in grips.iter().enumerate() {
            let Some(g) = g else { continue };
            let sounding = lit_up.is_some_and(|l| l == *g);
            let strength = if lit_up.is_none() || sounding { 1.0 } else { 0.4 };
            for shift in [-12i32, 0, 12] {
                let frets: Vec<(usize, i32)> = (0..6).filter_map(|s| g[s].map(|f| (s, f as i32 + shift))).collect();
                if frets.iter().any(|(_, f)| *f < 0 || *f > 17) {
                    continue;
                }
                for (s, f) in frets {
                    let pc = (theory::STANDARD[s] + f).rem_euclid(12);
                    dots.push(crate::improv::NeckDot {
                        spot: (s, f as u8),
                        text: degree_in(pc, root, &chord_pcs).0.to_string(),
                        col: INV_COLS[gi % 4],
                        strength,
                        ring: false,
                        root: pc == root.rem_euclid(12),
                    });
                }
            }
        }
        crate::improv::neck_view(ui, &dots, None);
        // which colour is which: click one to hear it
        ui.horizontal_wrapped(|ui| {
            for (gi, (label, g)) in grips.iter().enumerate() {
                let Some(g) = g else { continue };
                let (r, resp) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), Sense::click());
                ui.painter().rect_filled(r, 1.0, INV_COLS[gi % 4]);
                let (bass, top) = label.split_once("  ").unwrap_or((label.as_str(), ""));
                if retro_btn(ui, &format!("{}, {}", bass, top), false).tip("Hear it").clicked() || resp.clicked() {
                    acts.push(Action::PlayNotes(grip_notes(g)));
                }
                ui.add_space(10.0);
            }
        });
    }

    /// WALK THE CHANGES: these voicings through a tune, each chord the closest grip to the one before.
    fn walk_changes(
        &mut self,
        ui: &mut egui::Ui,
        acts: &mut Vec<Action>,
        fam: usize,
        strings: &[usize],
        drop: &[usize],
        reach: u8,
    ) {
        para(ui, "Every chord of the tune in this voicing, on these strings. Each grip is the nearest one to the last, the way you would comp it. Play along, then try it without looking.", pal().ink2);
        let (title, changes) = self.practice_changes(ui);
        let walk = Self::led_grips(&changes, fam, strings, drop, reach);
        if walk.is_empty() {
            return;
        }
        let bpm = self.band_bpm();
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, &title.to_uppercase(), 240.0);
            self.seq_button(
                ui,
                acts,
                "COMP IT",
                "Two beats a chord, at the band's tempo",
                "walk",
                || walk.iter().map(|w| grip_notes(&w.1)).collect(),
                120.0 / bpm,
            );
            crate::views::dim_line(ui, &format!("AT {} BPM", bpm), 1.0, pal().dim);
        });
        // the neck: the chord sounding now in its degree colours, where the hand goes next ringed (before it
        // plays: the first chord and the second)
        // or one chord at a time: step through, each one played as you land on it
        self.vc_step = self.vc_step.min(walk.len() - 1);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "< BACK", false).tip("The chord before").clicked() && self.vc_step > 0 {
                self.vc_step -= 1;
                acts.push(Action::PlayNotes(grip_notes(&walk[self.vc_step].1)));
            }
            if retro_btn(ui, "NEXT >", false).tip("The next chord").clicked() {
                self.vc_step = (self.vc_step + 1) % walk.len();
                acts.push(Action::PlayNotes(grip_notes(&walk[self.vc_step].1)));
            }
            crate::views::label(ui, &format!("CHORD {} OF {}", self.vc_step + 1, walk.len()), 0.0);
        });
        let now = self.seq_now("walk").unwrap_or(self.vc_step).min(walk.len() - 1);
        let next = (now + 1 < walk.len()).then(|| now + 1);
        let mut dots: Vec<crate::improv::NeckDot> = Vec::new();
        let mut put = |i: usize, ring: bool| {
            let (_, g, root, pcs) = &walk[i];
            for s in 0..6 {
                if let Some(f) = g[s] {
                    let pc = (theory::STANDARD[s] + f as i32).rem_euclid(12);
                    let (d, col) = degree_in(pc, *root, pcs);
                    dots.push(crate::improv::NeckDot {
                        spot: (s, f),
                        text: d.to_string(),
                        col,
                        strength: if ring { 0.7 } else { 1.0 },
                        ring,
                        root: !ring && pc == root.rem_euclid(12),
                    });
                }
            }
        };
        if let Some(n) = next {
            put(n, true);
        }
        put(now, false);
        crate::improv::neck_view(ui, &dots, None);
        crate::views::dim_line(
            ui,
            &match next {
                Some(n) => format!("NOW: {}      NEXT (RINGED): {}", walk[now].0, walk[n].0),
                None => format!("NOW: {}", walk[now].0),
            },
            1.0,
            pal().ink,
        );
        ui.add_space(4.0);
        Self::grip_row(ui, acts, &walk, Some(now));
    }

    /// PASSING CHORDS: the ways into each chord of a tune - its V7, its ii-V, the tritone sub, a diminished chord, a
    /// half step either side, the backdoor - each played from the chord before, in the voicing and strings you pick.
    fn passing(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "A passing chord leads into the next chord of the tune. Pick two chords in a row, then try each way in. Use the voicing and strings you want to work on.", pal().ink2);
        let (fam, strings, drop, reach) = self.voicing_picker(ui, false);
        let (title, changes) = self.practice_changes(ui);
        let mut pairs: Vec<(String, String)> = Vec::new();
        for w in changes.windows(2) {
            let p = (w[0].clone(), w[1].clone());
            if w[0] != w[1] && !pairs.contains(&p) {
                pairs.push(p);
            }
        }
        if pairs.is_empty() {
            return;
        }
        title_line(ui, &title.to_uppercase(), 2.5, pal().ink);
        let labels: Vec<String> = pairs.iter().map(|(a, b)| format!("{} > {}", a, b)).collect();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        if let Some(i) = crate::choice_line(ui, "pass_pair", "CHANGE", &labels, self.pass_pair.min(pairs.len() - 1), 200.0) {
            self.pass_pair = i;
        }
        let (from, to) = pairs[self.pass_pair.min(pairs.len() - 1)].clone();
        let gap = 60.0 / self.band_bpm() * 2.0;
        for (k, (kind, chords, why)) in theory::passing_options(&to).into_iter().enumerate() {
            let mut names = vec![from.clone()];
            names.extend(chords);
            names.push(to.clone());
            names.dedup();
            section_header(ui, &format!("{}:   {}", kind, names.join("  >  ")));
            para(ui, &why, pal().ink2);
            let grips = Self::led_grips(&names, fam, strings, drop, reach);
            let tag = format!("pass{}", k);
            ui.horizontal(|ui| {
                self.seq_button(
                    ui,
                    acts,
                    "HEAR IT",
                    "From the chord before, through the passing chord, into the next",
                    &tag,
                    || grips.iter().map(|g| grip_notes(&g.1)).collect(),
                    gap,
                );
            });
            Self::grip_row(ui, acts, &grips, self.seq_now(&tag));
        }
    }

    /// The grips of a voicing family for one chord on one string set, each with a label: every inversion (named by
    /// its bass and top notes), or for shells root-3-7 and root-7-3.
    fn voicing_grips(
        fam: usize,
        chord: &[i32],
        root: i32,
        strings: &[usize],
        drop: &[usize],
        reach: u8,
    ) -> Vec<(String, Option<[Option<u8>; 6]>)> {
        const DEG: [&str; 12] = ["R", "b9", "9", "b3", "3", "11", "b5", "5", "#5", "6", "b7", "7"];
        let deg = |x: i32| DEG[x.rem_euclid(12) as usize];
        if fam == 7 {
            let third = chord.get(1).copied().unwrap_or(4);
            let seventh = chord.get(3).copied().filter(|s| *s < 12).unwrap_or(chord.get(2).copied().unwrap_or(7));
            return vec![
                ("3 BASS  7 TOP".to_string(), theory::fit_on(&[third, seventh], root, strings, &theory::STANDARD, reach)),
                ("7 BASS  3 TOP".to_string(), theory::fit_on(&[seventh, third + 12], root, strings, &theory::STANDARD, reach)),
            ];
        }
        if fam == 2 {
            let third = chord.get(1).copied().unwrap_or(4);
            let seventh = chord.get(3).copied().filter(|s| *s < 12).unwrap_or(chord.get(2).copied().unwrap_or(7));
            return vec![
                ("R BASS  7 TOP".to_string(), theory::fit_on(&[0, third, seventh], root, strings, &theory::STANDARD, reach)),
                ("R BASS  3 TOP".to_string(), theory::fit_on(&[0, seventh, third + 12], root, strings, &theory::STANDARD, reach)),
            ];
        }
        let n = strings.len().min(chord.len());
        let chord = &chord[..n];
        (0..n)
            .map(|inv| {
                let v = theory::voiced(chord, inv, drop);
                let label = format!("{} BASS  {} TOP", deg(v[0]), deg(v[v.len() - 1]));
                (label, theory::fit_on(&v, root, strings, &theory::STANDARD, reach))
            })
            .collect()
    }

    /// A chord's grips in a row: yours first (outlined, marked MINE; right-click to delete, asked first), then the
    /// ones worked out. Click one to hear it; with `to_neck` a double-click lays it on the analyzer's neck.
    fn chord_grips(
        &mut self,
        ui: &mut egui::Ui,
        acts: &mut Vec<Action>,
        key: &str,
        c: &theory::ChordNotes,
        shapes: &[[Option<u8>; 6]],
        to_neck: bool,
    ) {
        let mine: Vec<[Option<u8>; 6]> = self.store.custom_grips.get(key).cloned().unwrap_or_default();
        if let Some((k, i)) = self.grip_ask.clone().filter(|(k, _)| k == key) {
            ui.horizontal_wrapped(|ui| {
                crate::views::label(ui, "DELETE YOUR GRIP?", 170.0);
                if retro_btn(ui, "YES, DELETE IT", false).clicked() {
                    if let Some(v) = self.store.custom_grips.get_mut(&k) {
                        if i < v.len() {
                            v.remove(i);
                        }
                        if v.is_empty() {
                            self.store.custom_grips.remove(&k);
                        }
                    }
                    self.store_dirty = true;
                    self.grip_ask = None;
                }
                if retro_btn(ui, "KEEP IT", false).clicked() {
                    self.grip_ask = None;
                }
            });
        }
        let (root, pcs) = (c.root, c.pcs.clone());
        let all: Vec<(Option<usize>, [Option<u8>; 6])> = mine
            .iter()
            .enumerate()
            .map(|(i, g)| (Some(i), *g))
            .chain(shapes.iter().filter(|g| !mine.contains(g)).map(|g| (None, *g)))
            .collect();
        ui.horizontal_wrapped(|ui| {
            for (own, g) in &all {
                let (r, resp) = ui.allocate_exact_size(Vec2::new(120.0, 140.0), Sense::click());
                // (each dot in its degree's colour, as on the piano and the staff)
                crate::fretboard::chord_box_coloured(ui.painter(), r, g, &|m| {
                    let (d, col) = degree_in(m, root, &pcs);
                    (d.to_string(), col)
                });
                let tip = if own.is_some() {
                    "Your grip. Click to hear it, right-click to delete it."
                } else if to_neck {
                    "Click to hear it. Double-click to lay it on the neck."
                } else {
                    "Hear this shape"
                };
                if let Some(i) = own {
                    // your own: a gold outline and a tag
                    let gold = Color32::from_rgb(235, 185, 60);
                    ui.painter().rect_stroke(r.shrink(1.0), 3.0, Stroke::new(2.0_f32, gold));
                    ptext(ui.painter(), Pos2::new(r.max.x - 6.0, r.min.y + 8.0), Align::Max, "MINE", 1.0, gold);
                    let i = *i;
                    resp.context_menu(|ui| {
                        if crate::menu_item(ui, "Delete this grip") {
                            self.grip_ask = Some((key.to_string(), i));
                            ui.close_menu();
                        }
                    });
                }
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
                if resp.clicked() {
                    let notes: Vec<i32> =
                        g.iter().enumerate().filter_map(|(i, f)| f.map(|f| theory::STANDARD[i] + f as i32)).collect();
                    acts.push(Action::PlayNotes(notes));
                }
                if to_neck && resp.double_clicked() {
                    self.an_frets = *g;
                }
                ui.add_space(10.0);
            }
        });
    }

    /// SAVE GRIP AS: name the grip on the neck yourself (typed, or root / kind / type from the funnel) and keep it
    /// with that chord, marked MINE wherever the chord's grips show.
    fn save_grip_panel(&mut self, ui: &mut egui::Ui) {
        const ROOTS: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, "NAME", 50.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(200.0, crate::bh()), Sense::hover());
            crate::views::field(ui, &mut self.ed, crate::tools::F_GRIP_NAME, &mut self.gs_name, rect, "G13b9, Cmaj9/E", false);
            ui.add_space(8.0);
            crate::views::label(ui, "OR PICK", 70.0);
            let (mut r, mut f, mut k) = self.gs_pick;
            let fams: Vec<&str> = theory::FINDER_FAMILIES.iter().map(|(n, _)| *n).collect();
            f = f.min(fams.len() - 1);
            let types: Vec<&str> = theory::FINDER_FAMILIES[f].1.iter().map(|(l, _)| *l).collect();
            k = k.min(types.len().saturating_sub(1));
            let mut changed = false;
            if let Some(i) = crate::dropdown(ui, "gs_root", "", &ROOTS, r, 60.0) {
                r = i;
                changed = true;
            }
            if let Some(i) = crate::dropdown(ui, "gs_fam", "", &fams, f, 130.0) {
                f = i;
                k = 0;
                changed = true;
            }
            let types: Vec<&str> = theory::FINDER_FAMILIES[f].1.iter().map(|(l, _)| *l).collect();
            if let Some(i) = crate::dropdown(ui, "gs_type", "", &types, k, 130.0) {
                k = i;
                changed = true;
            }
            self.gs_pick = (r, f, k);
            if changed {
                let suffix = theory::FINDER_FAMILIES[f].1.get(k).map_or("", |(_, s)| *s);
                self.gs_name = format!("{}{}", ROOTS[r], suffix);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let name = self.gs_name.trim().to_string();
            let ready = !name.is_empty() && self.an_frets.iter().flatten().count() >= 2;
            let known = theory::chord_notes(&name).is_some();
            if retro_btn(ui, "SAVE", false).tip("Save the grip under this chord. It shows as MINE.").clicked() && ready {
                let key = grip_key(&name);
                let list = self.store.custom_grips.entry(key).or_default();
                if !list.contains(&self.an_frets) {
                    list.push(self.an_frets);
                }
                self.store_dirty = true;
                self.gs_open = false;
                self.ed.id = 0;
                self.set_note(&format!("SAVED AS A GRIP FOR {}", name.to_uppercase()));
            }
            if retro_btn(ui, "CANCEL", false).clicked() {
                self.gs_open = false;
                self.ed.id = 0;
            }
            if !name.is_empty() && !known {
                crate::views::dim_line(
                    ui,
                    "BACKLINE CAN'T READ THIS CHORD NAME. THE GRIP SHOWS ONLY UNDER THIS EXACT NAME.",
                    1.0,
                    pal().dim,
                );
            } else if !ready {
                crate::views::dim_line(ui, "LAY THE GRIP ON THE NECK FIRST", 1.0, pal().dim);
            }
        });
        ui.add_space(4.0);
    }

    /// Guitar shapes for a chord, worked out once and kept.
    pub(crate) fn shapes_for(&mut self, name: &str, c: &theory::ChordNotes) -> Vec<[Option<u8>; 6]> {
        if let Some(s) = self.shape_cache.get(name) {
            return s.clone();
        }
        let s = theory::guitar_shapes(c, &theory::STANDARD);
        if self.shape_cache.len() > 400 {
            self.shape_cache.clear();
        }
        self.shape_cache.insert(name.to_string(), s.clone());
        s
    }

    /// While the band plays: the chord it is on, on a piano and a guitar box (like iReal Pro's instrument view).
    pub(crate) fn instrument_view(&mut self, ui: &mut egui::Ui, chart: &crate::chart::Chart) {
        if !self.band_on || self.band_order.is_empty() {
            return;
        }
        let Some((at, beat)) = self.band_where() else { return };
        let Some(bar_ix) = self.band_order.get(at).copied() else { return };
        let Some(bar) = chart.bars.get(bar_ix) else { return };
        let n = bar.chords.len().max(1);
        let k = ((beat / chart.bar_len(bar_ix) * n as f32) as usize).min(n - 1);
        let Some(name) = bar.chords.get(k).cloned() else { return };
        let name = crate::chart::transpose(&name, self.chart_tr);
        let Some(c) = theory::chord_notes(&name) else { return };
        ui.horizontal(|ui| {
            let (cr, _) = ui.allocate_exact_size(Vec2::new(110.0, 96.0), Sense::hover());
            ptext(ui.painter(), Pos2::new(cr.center().x, cr.min.y + 14.0), Align::Center, &name, 3.0, pal().ink);
            ptext(ui.painter(), Pos2::new(cr.center().x, cr.min.y + 40.0), Align::Center, "NOW", 1.0, pal().dim);
            let (pr, _) =
                ui.allocate_exact_size(Vec2::new((ui.available_width() - 130.0).clamp(200.0, 420.0), 80.0), Sense::hover());
            piano(ui, pr, &piano_voicing(&c), false, "band_piano", Some(c.root));
            let shapes = self.shapes_for(&name, &c);
            if let Some(s) = shapes.first() {
                let (gr, _) = ui.allocate_exact_size(Vec2::new(100.0, 110.0), Sense::hover());
                let (root, pcs) = (c.root, c.pcs.clone());
                chord_box_labelled(ui.painter(), gr, s, &|m| degree_in(m, root, &pcs).0.to_string());
            }
        });
    }
}

/// The name your own grips are kept under: the root as a pitch (so Ab and G# are one chord) and the rest as typed.
pub(crate) fn grip_key(name: &str) -> String {
    let (r, rest) = crate::chart::split_root(name.trim());
    match crate::chart::note_index(r) {
        Some(pc) => format!("{}:{}", pc.rem_euclid(12), rest.trim()),
        None => name.trim().to_lowercase(),
    }
}

/// The notes a grip sounds (MIDI), low string first.
/// Where a grip sits on the neck: its lowest fretted fret.
fn neck_spot(g: &[Option<u8>; 6]) -> u8 {
    g.iter().flatten().copied().filter(|f| *f > 0).min().unwrap_or(0)
}

/// The middle of a grip on the neck (open strings count as fret 0).
fn neck_centre(g: &[Option<u8>; 6]) -> f32 {
    let f: Vec<f32> = g.iter().flatten().map(|f| *f as f32).collect();
    f.iter().sum::<f32>() / f.len().max(1) as f32
}

/// The colours clusters are drawn in: a semitone pair red, a tone pair blue, the bass orange.
pub(crate) const CL_SEMI: Color32 = Color32::from_rgb(240, 96, 96);
pub(crate) const CL_TONE: Color32 = Color32::from_rgb(70, 150, 255);
pub(crate) const CL_BASS: Color32 = Color32::from_rgb(255, 170, 60);

/// One octave of keys, C to B, to pick a note on: `marks` colours keys, `faded` keys are dimmed with an X.
/// A click gives that key's pitch class.
pub(crate) fn octave_keys(ui: &mut egui::Ui, id: &str, marks: &[(i32, Color32)], faded: &[i32]) -> Option<i32> {
    let (w, h) = (196.0_f32, 46.0_f32);
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let p = ui.painter();
    let ww = w / 7.0;
    const WHITE: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
    const BLACK: [(i32, f32); 5] = [(1, 1.0), (3, 2.0), (6, 4.0), (8, 5.0), (10, 6.0)];
    let mark = |pc: i32| marks.iter().find(|m| m.0.rem_euclid(12) == pc).map(|m| m.1);
    let black_rect =
        |at: f32| Rect::from_min_size(Pos2::new(r.min.x + at * ww - ww * 0.32, r.min.y), Vec2::new(ww * 0.64, h * 0.6));
    for (i, pc) in WHITE.iter().enumerate() {
        let k = Rect::from_min_size(Pos2::new(r.min.x + i as f32 * ww, r.min.y), Vec2::new(ww - 1.0, h));
        p.rect_filled(k, 2.0, mark(*pc).unwrap_or(Color32::from_gray(236)));
        if faded.contains(pc) {
            p.rect_filled(k, 2.0, Color32::from_black_alpha(90));
            ptext(p, Pos2::new(k.center().x, k.max.y - 9.0), Align::Center, "x", 1.0, Color32::from_gray(40));
        }
    }
    for (pc, at) in BLACK {
        let k = black_rect(at);
        p.rect_filled(k, 2.0, mark(pc).unwrap_or(Color32::from_gray(28)));
        if faded.contains(&pc) {
            p.rect_filled(k, 2.0, Color32::from_black_alpha(90));
            ptext(p, Pos2::new(k.center().x, k.max.y - 7.0), Align::Center, "x", 1.0, Color32::from_gray(200));
        }
    }
    let pos = resp.interact_pointer_pos().filter(|_| resp.clicked())?;
    let _ = id;
    if let Some((pc, _)) = BLACK.iter().find(|(_, at)| black_rect(*at).contains(pos)) {
        return Some(*pc);
    }
    let i = (((pos.x - r.min.x) / ww).floor() as usize).min(6);
    Some(WHITE[i])
}
