//! LINES: the desktop's ATONAL lines on a phone. A generated line (alternate triads from any of the app's scales,
//! two tetrachords joined, or an interval cycle) on the long neck, its notes numbered in the order you play them,
//! from the start and finger you pick (every string, every finger); the player (tempo, feel, loop) and the pickers
//! pull up from the bottom, with the line's analysis (intervals, cells, the scales that hold it, its syllables).

use crate::*;

const KINDS: [&str; 3] = ["Alternate triads", "Tetrachords joined", "Interval cycle"];
const CYCLES: [(&str, i32); 4] = [("Minor 3rds", 3), ("Major 3rds", 4), ("4ths", 5), ("Tritones", 6)];
/// notes to a beat, as the desktop's FEEL
const FEELS: [(&str, u32); 7] = [("1/4", 1), ("1/8", 2), ("Triplets", 3), ("1/16", 4), ("Quintuplets", 5), ("Sextuplets", 6), ("Septuplets", 7)];

/// A scale's name the phone's way: capitalised, not shouted.
pub(crate) fn nice(name: &str) -> String {
    let low = name.to_lowercase();
    let mut c = low.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

impl App {
    /// The line as written (treble clef; the guitar sounds it an octave lower) and its name.
    fn ln_line(&self) -> (Vec<i32>, String) {
        let root = theory::note_name(self.ln_root, false);
        match self.ln_kind {
            0 => {
                let list = atonal_core::scale_list();
                let (name, steps) = &list[self.ln_scale % list.len()];
                (atonal_core::alternate_triads(self.ln_root, steps), format!("Alternate triads, {} {}", root, nice(name).to_lowercase()))
            }
            1 => {
                let t = &music_data::TETRACHORDS;
                let (lo, up) = (self.ln_lower % t.len(), self.ln_upper % t.len());
                (atonal_core::tetrachord_line(self.ln_root, lo, up), format!("{} + {} on {}", nice(t[lo].0), nice(t[up].0).to_lowercase(), root))
            }
            _ => {
                let (name, semis) = CYCLES[self.ln_cycle % CYCLES.len()];
                (atonal_core::interval_cycle(self.ln_root, semis), format!("Cycle of {}, from {}", name.to_lowercase(), root))
            }
        }
    }

    /// Play the line from the top (it replaces whatever was sounding).
    fn ln_play(&mut self, played: &[i32]) {
        let gap = 60.0 / self.ln_bpm.max(20) as f32 / self.ln_sub.max(1) as f32;
        let notes: Vec<Vec<i32>> = played.iter().map(|m| vec![*m]).collect();
        self.player.send(Cmd::StopOnce);
        self.player.send(Cmd::Once(band::line_sound(&notes, gap), 0.9));
        self.ln_end = Some(Instant::now() + Duration::from_secs_f32(notes.len() as f32 * gap + 0.4));
    }

    pub(crate) fn lines_ui(&mut self, ui: &mut egui::Ui) {
        let (tr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
        if let Some(i) = seg_tabs(ui, tr, &["Atonal lines", "Superimpose"], self.ln_page as usize, "ln_page") {
            self.ln_page = i as u8;
            self.player.send(Cmd::StopOnce);
            self.ln_end = None;
        }
        ui.add_space(4.0);
        if self.ln_page == 1 {
            return self.superimpose_ui(ui);
        }
        let (written, name) = self.ln_line();
        let sounding: Vec<i32> = written.iter().map(|m| m - 12).collect();
        let starts = atonal_core::starts(sounding.first().copied().unwrap_or(40));
        if starts.is_empty() {
            return;
        }
        let start = starts[self.ln_start % starts.len()];
        let finger = self.ln_finger.clamp(1, 4);
        let pos = atonal_core::position_of(start, finger).unwrap_or(1);
        let (played, spots, _) = atonal_core::finger_from(&sounding, start, pos);

        // the player: Stop while sounding, round again when it loops
        let now = Instant::now();
        let mut playing = self.ln_end.is_some_and(|e| now < e);
        if !playing && self.ln_end.is_some() {
            if self.ln_loop {
                self.ln_play(&played);
                playing = true;
            } else {
                self.ln_end = None;
            }
        }
        if playing {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        // two rows of choices, always showing: the line (kind, root, scale), then where and how (start, finger,
        // tempo, loop, what it's made of)
        let mut restart = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
            if let Some(i) = self.pick_chip(ui, "ln_kind", &KINDS, self.ln_kind as usize) {
                self.ln_kind = i as u8;
                self.ln_start = 0;
                restart = true;
            }
            let roots: Vec<&str> = (0..12).map(|k| theory::note_name(k, false)).collect();
            if let Some(i) = self.pick_chip(ui, "ln_root", &roots, self.ln_root as usize) {
                self.ln_root = i as i32;
                self.ln_start = 0;
                restart = true;
            }
            match self.ln_kind {
                0 => {
                    let names: Vec<String> = atonal_core::scale_list().iter().map(|s| nice(s.0)).collect();
                    let names: Vec<&str> = names.iter().map(String::as_str).collect();
                    if let Some(i) = self.pick_chip(ui, "ln_scale", &names, self.ln_scale.min(names.len() - 1)) {
                        self.ln_scale = i;
                        self.ln_start = 0;
                        restart = true;
                    }
                }
                1 => {
                    let names: Vec<String> = music_data::TETRACHORDS.iter().map(|t| nice(t.0)).collect();
                    let names: Vec<&str> = names.iter().map(String::as_str).collect();
                    if let Some(i) = self.pick_chip(ui, "ln_lower", &names, self.ln_lower.min(names.len() - 1)) {
                        self.ln_lower = i;
                        self.ln_start = 0;
                        restart = true;
                    }
                    if let Some(i) = self.pick_chip(ui, "ln_upper", &names, self.ln_upper.min(names.len() - 1)) {
                        self.ln_upper = i;
                        self.ln_start = 0;
                        restart = true;
                    }
                }
                _ => {
                    let names: Vec<&str> = CYCLES.iter().map(|c| c.0).collect();
                    if let Some(i) = self.pick_chip(ui, "ln_cycle", &names, self.ln_cycle.min(names.len() - 1)) {
                        self.ln_cycle = i;
                        self.ln_start = 0;
                        restart = true;
                    }
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
            let snames: Vec<String> = starts.iter().map(|(s, f)| format!("{} string, fret {}", ["Low E", "A", "D", "G", "B", "High E"][*s], f)).collect();
            let snames: Vec<&str> = snames.iter().map(String::as_str).collect();
            if let Some(i) = self.pick_chip(ui, "ln_start", &snames, self.ln_start % starts.len()) {
                self.ln_start = i;
            }
            let fingers = ["Finger 1", "Finger 2", "Finger 3", "Finger 4"];
            if let Some(i) = self.pick_chip(ui, "ln_finger", &fingers, (finger - 1) as usize) {
                self.ln_finger = i as u8 + 1;
            }
            if pill_w(ui, "-", 30.0, false).clicked() {
                self.ln_bpm = self.ln_bpm.saturating_sub(5).max(20);
            }
            let bt = format!("{} bpm", self.ln_bpm);
            let (br, _) = ui.allocate_exact_size(Vec2::new(text_w(&bt, 1.2) + 10.0, 32.0), Sense::hover());
            ptext(ui.painter(), br.center(), Align::Center, &bt, 1.2, pal().ink);
            if pill_w(ui, "+", 30.0, false).clicked() {
                self.ln_bpm = (self.ln_bpm + 5).min(400);
            }
            let feel_names: Vec<&str> = FEELS.iter().map(|f| f.0).collect();
            let cur = FEELS.iter().position(|f| f.1 == self.ln_sub).unwrap_or(1);
            if let Some(i) = self.pick_chip(ui, "ln_feel", &feel_names, cur) {
                self.ln_sub = FEELS[i].1;
            }
            if pill(ui, "Loop", self.ln_loop).clicked() {
                self.ln_loop = !self.ln_loop;
            }
            if pill(ui, "What it's made of", self.ln_sheet).clicked() {
                self.ln_sheet = !self.ln_sheet;
            }
        });
        if restart && playing {
            self.player.send(Cmd::StopOnce);
            self.ln_end = None;
            playing = false;
        }

        let avail = ui.available_rect_before_wrap();
        let foot = 60.0;
        if self.ln_sheet {
            // what it's made of: the line on staves and its analysis, over the neck
            let body = Rect::from_min_max(avail.min, Pos2::new(avail.max.x, avail.max.y - foot));
            ui.painter().rect_filled(body, 10.0, pal().beige_lt);
            let mut sub = ui.new_child(egui::UiBuilder::new().max_rect(body.shrink(8.0)).layout(egui::Layout::top_down(Align::Min)));
            egui::ScrollArea::vertical().id_salt("ln_made").auto_shrink([false, false]).show(&mut sub, |ui| {
                let lit = self.ln_end.filter(|e| Instant::now() < *e).map(|e| {
                    let gap = 60.0 / self.ln_bpm.max(20) as f32 / self.ln_sub.max(1) as f32;
                    let left = (e - Instant::now()).as_secs_f32() - 0.4;
                    (written.len() as f32 - left / gap).floor().max(0.0) as usize
                });
                line_staff(ui, &written, lit);
                ui.add_space(8.0);
                analysis(ui, &written);
            });
        } else {
            let neck_r = Rect::from_min_max(avail.min, Pos2::new(avail.max.x, avail.max.y - foot));
            line_neck(ui.painter(), neck_r, &spots);
        }
        // the foot: the line's name and Play
        let fr = Rect::from_min_max(Pos2::new(avail.min.x, avail.max.y - foot + 6.0), avail.max);
        let p = ui.painter().clone();
        ptext_fit(&p, Pos2::new(fr.min.x + 4.0, fr.center().y), Align::Min, &name, 1.3, fr.width() - 70.0, pal().ink);
        let pc = Pos2::new(fr.max.x - 26.0, fr.center().y);
        p.circle_filled(pc, 24.0, pal().red);
        play_glyph(&p, pc, playing, pal().bar_txt);
        if ui.interact(Rect::from_center_size(pc, Vec2::splat(48.0)), ui.id().with("ln_play"), Sense::click()).clicked() {
            if playing {
                self.player.send(Cmd::StopOnce);
                self.ln_end = None;
            } else {
                self.ln_play(&played);
            }
        }
    }
}

impl App {
    /// SUPERIMPOSE: a shape from another root (a pentatonic, a triad, an arpeggio) over a chord, every note shown on
    /// the long neck by what it is over the chord (a chord tone, a tension, or one to avoid landing on); the chord's
    /// own notes not in the shape ringed. The pickers and the why pull up from the bottom; PLAY plays the shape up
    /// and back over the chord.
    fn superimpose_ui(&mut self, ui: &mut egui::Ui) {
        use si_core::*;
        let fams = theory::FINDER_FAMILIES;
        let fam = self.si_fam.min(fams.len() - 1);
        let list = fams[fam].1;
        let root = self.si_root.rem_euclid(12);
        let ch = self.si_chord.min(list.len() - 1);
        let suffix = list[ch].1;
        let chord_name = format!("{}{}", theory::note_name(root, false), suffix);
        let chord: Vec<i32> = match theory::chord_notes(&chord_name) {
            Some(n) => {
                let mut v: Vec<i32> = n.pcs.iter().map(|p| (p - n.root).rem_euclid(12)).collect();
                v.sort();
                v.dedup();
                v
            }
            None => vec![0, 4, 7],
        };
        let chord_abs: Vec<i32> = chord.iter().map(|x| (root + x).rem_euclid(12)).collect();
        let fits = si_scales_for(&chord);
        let sc = self.si_scale.min(fits.len().saturating_sub(1));
        let scale: Vec<i32> = fits.get(sc).map_or(chord.clone(), |i| SI_SCALES[*i].1[..SI_SCALES[*i].2].to_vec());
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
        let mut sug: Vec<(usize, i32, &'static str)> = curated.map(si_suggestions).unwrap_or_default();
        for (k, o) in si_auto(&chord, &scale) {
            if !sug.iter().any(|s| s.0 == k && s.1 == o) {
                sug.push((k, o, ""));
            }
        }
        if sug.is_empty() {
            sug.push((1, 0, ""));
        }
        let pick = self.si_pick.min(sug.len() - 1);
        let (kind, off, why): (usize, i32, String) = if self.si_any {
            (self.si_kind.min(SI_SHAPES.len() - 1), self.si_kroot, String::new())
        } else {
            let (k, o, w) = sug[pick];
            let why = if !w.is_empty() {
                nice(w)
            } else {
                let pcs: Vec<i32> = SI_SHAPES[k].1.iter().map(|x| (root + o + x).rem_euclid(12)).collect();
                let adds: Vec<&str> = pcs.iter().filter(|p| !chord.contains(&((*p - root).rem_euclid(12)))).map(|p| degree_in(*p, root, &chord_abs).0).collect();
                if adds.is_empty() { "The chord itself".to_string() } else { format!("Adds {}", adds.join(" ")) }
            };
            (k, o, why)
        };
        let shape_root = (root + off).rem_euclid(12);
        let shape: Vec<i32> = SI_SHAPES[kind].1.iter().map(|x| (shape_root + x).rem_euclid(12)).collect();
        let class = |pc: i32| -> Color32 {
            let r = (pc - root).rem_euclid(12);
            if chord.contains(&r) {
                degree(pc, root).1
            } else if chord.contains(&((r - 1).rem_euclid(12))) && !(chord.contains(&10) && chord.contains(&4) && (r == 1 || r == 3 || r == 8)) {
                Color32::from_rgb(220, 70, 60)
            } else {
                Color32::from_rgb(95, 168, 211)
            }
        };
        // the neck
        let mut dots: Vec<((usize, u8), String, Color32, bool)> = Vec::new();
        for s in 0..6 {
            for f in 0..=15u8 {
                let pc = (theory::STANDARD[s] + f as i32).rem_euclid(12);
                if shape.contains(&pc) {
                    dots.push(((s, f), degree_in(pc, root, &chord_abs).0.to_string(), class(pc), false));
                } else if chord.contains(&((pc - root).rem_euclid(12))) {
                    dots.push(((s, f), degree_in(pc, root, &chord_abs).0.to_string(), pal().dim, true));
                }
            }
        }
        let title = format!("{} {} over {}", theory::note_name(shape_root, false), nice(SI_SHAPES[kind].0).to_lowercase(), chord_name);
        // the line: the shape up (to 12 notes) and back, over the chord
        let mut up: Vec<i32> = (52..=76i32).filter(|m| shape.contains(&m.rem_euclid(12))).collect();
        up.truncate(12);
        let mut notes = up.clone();
        notes.extend(up.iter().rev().skip(1));
        let voiced: Vec<i32> = chord.iter().take(4).map(|x| 48 + root + x).collect();
        let playing = self.ln_end.is_some_and(|e| Instant::now() < e);
        if playing {
            ui.ctx().request_repaint_after(Duration::from_millis(150));
        }

        let avail = ui.available_rect_before_wrap();
        let sheet_h = if self.ln_sheet { (avail.height() * 0.66).max(320.0) } else { 64.0 };
        let neck_r = Rect::from_min_max(avail.min, Pos2::new(avail.max.x, avail.max.y - sheet_h - 8.0));
        dot_neck(ui.painter(), neck_r, &dots);
        let sr = Rect::from_min_max(Pos2::new(avail.min.x - 4.0, avail.max.y - sheet_h), Pos2::new(avail.max.x + 4.0, avail.max.y + 6.0));
        ui.painter().rect_filled(sr, egui::Rounding { nw: 16.0, ne: 16.0, sw: 0.0, se: 0.0 }, pal().beige_lt);
        ui.painter().rect_filled(Rect::from_center_size(Pos2::new(sr.center().x, sr.min.y + 8.0), Vec2::new(40.0, 4.0)), 2.0, pal().beige_dk);
        let head = Rect::from_min_size(sr.min + Vec2::new(12.0, 14.0), Vec2::new(sr.width() - 24.0, 44.0));
        let p = ui.painter().clone();
        ptext_fit(&p, Pos2::new(head.min.x, head.min.y + 12.0), Align::Min, &title, 1.3, head.width() - 60.0, pal().ink);
        ptext_fit(&p, Pos2::new(head.min.x, head.min.y + 30.0), Align::Min, &why, 1.0, head.width() - 60.0, pal().dim);
        let pc = Pos2::new(head.max.x - 22.0, head.center().y);
        p.circle_filled(pc, 22.0, pal().red);
        play_glyph(&p, pc, playing, pal().bar_txt);
        if ui.interact(Rect::from_center_size(pc, Vec2::splat(44.0)), ui.id().with("si_play"), Sense::click()).clicked() {
            self.player.send(Cmd::StopOnce);
            if playing {
                self.ln_end = None;
            } else {
                let gap = 60.0 / self.ln_bpm.max(20) as f32 / self.ln_sub.max(1) as f32;
                let line: Vec<Vec<i32>> = notes.iter().map(|m| vec![*m]).collect();
                self.player.send(Cmd::Once(band::chord_sound(&voiced), 0.6));
                self.player.send(Cmd::Once(band::line_sound(&line, gap), 0.9));
                self.ln_end = Some(Instant::now() + Duration::from_secs_f32(line.len() as f32 * gap + 0.4));
            }
        }
        let toggle = Rect::from_min_max(sr.min, Pos2::new(head.max.x - 48.0, head.max.y));
        if ui.interact(toggle, ui.id().with("si_sheet"), Sense::click()).clicked() {
            self.ln_sheet = !self.ln_sheet;
        }
        if !self.ln_sheet {
            return;
        }
        let body = Rect::from_min_max(Pos2::new(sr.min.x + 12.0, head.max.y + 6.0), sr.max - Vec2::new(12.0, 10.0));
        let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(Align::Min)));
        egui::ScrollArea::vertical().id_salt("si_pick").auto_shrink([false, false]).show(&mut ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
            dim_line(ui, "Chord", 1.0, pal().ink2);
            ui.horizontal_wrapped(|ui| {
                for k in 0..12 {
                    if pill_w(ui, theory::note_name(k, false), 40.0, root == k).clicked() {
                        self.si_root = k;
                    }
                }
            });
            let fam_names: Vec<String> = fams.iter().map(|f| nice(f.0)).collect();
            let fam_names: Vec<&str> = fam_names.iter().map(String::as_str).collect();
            let labels: Vec<String> = list.iter().map(|(l, _)| format!("{}{}", theory::note_name(root, false), l)).collect();
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            ui.horizontal_wrapped(|ui| {
                if let Some(i) = self.pick_chip(ui, "si_fam", &fam_names, fam) {
                    self.si_fam = i;
                    self.si_chord = 0;
                    self.si_scale = 0;
                    self.si_pick = 0;
                }
                if let Some(i) = self.pick_chip(ui, "si_chord", &labels, ch) {
                    self.si_chord = i;
                    self.si_scale = 0;
                    self.si_pick = 0;
                }
            });
            if !fits.is_empty() {
                let names: Vec<String> = fits.iter().map(|i| nice(SI_SCALES[*i].0)).collect();
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                ui.horizontal(|ui| {
                    label(ui, "Scale", 56.0);
                    if let Some(i) = self.pick_chip(ui, "si_scale", &names, sc) {
                        self.si_scale = i;
                        self.si_pick = 0;
                    }
                });
            }
            ui.horizontal_wrapped(|ui| {
                if pill(ui, "The usual choices", !self.si_any).clicked() {
                    self.si_any = false;
                }
                if pill(ui, "Any shape, anywhere", self.si_any).clicked() {
                    self.si_any = true;
                }
            });
            if self.si_any {
                let sn: Vec<String> = SI_SHAPES.iter().map(|s| nice(s.0)).collect();
                let sn: Vec<&str> = sn.iter().map(String::as_str).collect();
                let roots: Vec<String> = (0..12).map(|k| format!("{} ({})", theory::note_name(root + k, false), degree(root + k, root).0)).collect();
                let roots: Vec<&str> = roots.iter().map(String::as_str).collect();
                ui.horizontal_wrapped(|ui| {
                    if let Some(i) = self.pick_chip(ui, "si_kind", &sn, self.si_kind.min(SI_SHAPES.len() - 1)) {
                        self.si_kind = i;
                    }
                    if let Some(i) = self.pick_chip(ui, "si_kroot", &roots, (self.si_kroot as usize).min(11)) {
                        self.si_kroot = i as i32;
                    }
                });
            } else {
                let names: Vec<String> = sug.iter().map(|(k, o, _)| format!("{} {}", theory::note_name(root + o, false), nice(SI_SHAPES[*k].0).to_lowercase())).collect();
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                ui.horizontal(|ui| {
                    label(ui, "Play", 56.0);
                    if let Some(i) = self.pick_chip(ui, "si_pick", &names, pick) {
                        self.si_pick = i;
                    }
                });
            }
            // the notes, each named over the chord
            let mut sorted = shape.clone();
            sorted.sort_by_key(|p| (p - shape_root).rem_euclid(12));
            let line: Vec<String> = sorted.iter().map(|pc| format!("{} = {}", theory::note_name(*pc, false), degree_in(*pc, root, &chord_abs).0)).collect();
            for l in wrap(&line.join("    "), 1.2, ui.available_width() - 8.0) {
                dim_line(ui, &l, 1.2, pal().ink);
            }
            ui.horizontal_wrapped(|ui| {
                for (name, col) in [("Chord tone", pal().ink), ("Tension", Color32::from_rgb(95, 168, 211)), ("Avoid", Color32::from_rgb(220, 70, 60))] {
                    let (r, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 6.0, col);
                    label(ui, name, 0.0);
                    ui.add_space(8.0);
                }
            });
            dim_line(ui, "Rings: the chord's own notes not in the shape", 1.0, pal().dim);
        });
    }
}

/// The long neck with named, coloured dots (frets 0 to 15 down the screen); a ringed dot is outlined only.
pub(crate) fn dot_neck(p: &egui::Painter, r: Rect, dots: &[((usize, u8), String, Color32, bool)]) {
    let frets = 15;
    let x0 = r.min.x + 26.0;
    let sw = (r.width() - 40.0) / 5.0;
    let y0 = r.min.y + 18.0;
    let fh = (r.height() - 24.0) / frets as f32;
    p.rect_filled(r, 10.0, pal().beige_lt);
    let line = pal().beige_dk;
    for f in [3, 5, 7, 9, 15] {
        p.circle_filled(Pos2::new(x0 + 2.5 * sw, y0 + (f as f32 - 0.5) * fh), 3.0, line);
    }
    for f in 0..=frets {
        let y = y0 + f as f32 * fh;
        p.line_segment([Pos2::new(x0, y), Pos2::new(x0 + 5.0 * sw, y)], Stroke::new(if f == 0 { 4.0 } else { 1.2 }, line));
        if f > 0 {
            ptext(p, Pos2::new(r.min.x + 12.0, y - fh / 2.0), Align::Center, &f.to_string(), 0.9, pal().dim);
        }
    }
    for s in 0..6 {
        let x = x0 + s as f32 * sw;
        p.line_segment([Pos2::new(x, y0), Pos2::new(x, y0 + frets as f32 * fh)], Stroke::new(1.8 - s as f32 * 0.2, pal().ink2));
    }
    let rad = (sw * 0.34).min(fh * 0.42).min(11.0);
    for ((s, f), text, col, ring) in dots {
        let c = Pos2::new(x0 + *s as f32 * sw, if *f == 0 { y0 - 9.0 } else { y0 + (*f as f32 - 0.5) * fh });
        if *ring {
            p.circle_stroke(c, rad * 0.8, Stroke::new(1.4, *col));
        } else {
            p.circle_filled(c, rad, *col);
            ptext(p, c, Align::Center, text, 0.85, Color32::from_gray(15));
        }
    }
}

/// The line's analysis, as the desktop shows it: the interval between each pair of notes, its three- and four-note
/// cells, the scales that hold every note, and its syllables (moveable Do from the first note, and fixed Do).
fn analysis(ui: &mut egui::Ui, line: &[i32]) {
    // a heading, then the text wrapped small under it
    let fact = |ui: &mut egui::Ui, name: &str, text: &str| {
        dim_line(ui, name, 1.0, pal().ink2);
        for line in wrap(text, 1.2, ui.available_width() - 8.0) {
            let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
            ptext(ui.painter(), Pos2::new(r.min.x + 2.0, r.center().y), Align::Min, &line, 1.2, pal().ink);
        }
        ui.add_space(4.0);
    };
    let ints: Vec<String> = line.windows(2).map(|w| atonal_core::interval_name(w[1] - w[0])).collect();
    fact(ui, "Intervals", &ints.join("  "));
    let tri: Vec<String> = atonal_core::trichords(line)
        .iter()
        .map(|(p, c)| {
            let set: String = p.iter().map(|x| ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'E'][*x as usize]).collect();
            if *c > 1 {
                format!("({}) x{}", set, c)
            } else {
                format!("({})", set)
            }
        })
        .collect();
    fact(ui, "Trichords", &if tri.is_empty() { "None".to_string() } else { tri.join(",  ") });
    let tet: Vec<String> = atonal_core::tetrachords(line)
        .iter()
        .map(|(t, c)| {
            let n = nice(music_data::TETRACHORDS[*t].0);
            if *c > 1 {
                format!("{} x{}", n, c)
            } else {
                n
            }
        })
        .collect();
    fact(ui, "Tetrachords", &if tet.is_empty() { "None named".to_string() } else { tet.join(",  ") });
    let list = atonal_core::scale_list();
    let holding = atonal_core::scales_holding(line);
    let held: Vec<String> = holding.iter().take(12).map(|(k, i)| format!("{} {}", theory::note_name(*k, false), nice(list[*i].0).to_lowercase())).collect();
    let more = holding.len().saturating_sub(12);
    let held_text = if held.is_empty() {
        "None of the app's scales holds every note.".to_string()
    } else if more > 0 {
        format!("{},  and {} more", held.join(",  "), more)
    } else {
        held.join(",  ")
    };
    fact(ui, "Scales that hold it", &held_text);
    let (moveable, fixed) = atonal_core::syllables(line);
    fact(ui, "Moveable Do", &moveable.join(" "));
    fact(ui, "Fixed Do", &fixed.join(" "));
}

/// The long neck for a line: the strings down the screen, a window of frets around the line, and each note a dot
/// numbered in the order it is played; a spot played again shows both numbers (3/9).
fn line_neck(p: &egui::Painter, r: Rect, spots: &[(usize, u8)]) {
    let lo_f = spots.iter().map(|s| s.1).min().unwrap_or(1).saturating_sub(1).max(1);
    let hi_f = (spots.iter().map(|s| s.1).max().unwrap_or(12) + 1).max(lo_f + 7);
    let n = (hi_f - lo_f + 1) as f32;
    let x0 = r.min.x + 30.0;
    let sw = (r.width() - 46.0) / 5.0;
    let y0 = r.min.y + 12.0;
    let fh = (r.height() - 18.0) / n;
    p.rect_filled(r, 10.0, pal().beige_lt);
    let line = pal().beige_dk;
    for k in 0..=(hi_f - lo_f + 1) {
        let y = y0 + k as f32 * fh;
        p.line_segment([Pos2::new(x0, y), Pos2::new(x0 + 5.0 * sw, y)], Stroke::new(1.2, line));
    }
    for f in lo_f..=hi_f {
        let y = y0 + (f - lo_f) as f32 * fh + fh / 2.0;
        if [3, 5, 7, 9, 15, 17, 19].contains(&f) {
            p.circle_filled(Pos2::new(x0 + 2.5 * sw, y), 3.5, line);
        }
        if f == 12 {
            p.circle_filled(Pos2::new(x0 + 1.5 * sw, y), 3.5, line);
            p.circle_filled(Pos2::new(x0 + 3.5 * sw, y), 3.5, line);
        }
        ptext(p, Pos2::new(r.min.x + 14.0, y), Align::Center, &f.to_string(), 0.9, pal().dim);
    }
    for s in 0..6 {
        let x = x0 + s as f32 * sw;
        p.line_segment([Pos2::new(x, y0), Pos2::new(x, y0 + n * fh)], Stroke::new(1.8 - s as f32 * 0.2, pal().ink2));
    }
    let rad = (sw * 0.4).min(fh * 0.45).min(14.0);
    // every number played at each spot, in order
    let mut at: Vec<((usize, u8), Vec<usize>)> = Vec::new();
    for (i, sp) in spots.iter().enumerate() {
        match at.iter_mut().find(|a| a.0 == *sp) {
            Some(a) => a.1.push(i + 1),
            None => at.push((*sp, vec![i + 1])),
        }
    }
    for (sp, nums) in at {
        if sp.1 < lo_f || sp.1 > hi_f {
            continue;
        }
        let c = Pos2::new(x0 + sp.0 as f32 * sw, y0 + (sp.1 - lo_f) as f32 * fh + fh / 2.0);
        let first = nums[0] == 1;
        let txt = nums.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("/");
        let w = (text_w(&txt, 0.9) + 10.0).max(rad * 2.0);
        p.rect_filled(Rect::from_center_size(c, Vec2::new(w, rad * 2.0)), rad, if first { pal().red } else { pal().ink });
        ptext(p, c, Align::Center, &txt, 0.9, if first { pal().bar_txt } else { pal().beige_lt });
    }
}

/// A line written on staves (as guitar music is, notes as written), eight notes to a staff, the clef at the start of
/// each; an accidental on every note that has one. `lit`: the note sounding, in the accent.
fn line_staff(ui: &mut egui::Ui, line: &[i32], lit: Option<usize>) {
    let g = 6.0f32;
    let per = 8usize;
    let w = ui.available_width();
    for (row, chunk) in line.chunks(per).enumerate() {
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 13.0 * g), Sense::hover());
        let p = ui.painter();
        let top = r.min.y + 4.5 * g;
        let bottom = top + 4.0 * g;
        let stroke = Stroke::new(1.0, pal().ink2);
        for k in 0..5 {
            let y = top + k as f32 * g;
            p.line_segment([Pos2::new(r.min.x + 4.0, y), Pos2::new(r.max.x - 4.0, y)], stroke);
        }
        pixmap(p, Pos2::new(r.min.x + 8.0, top - 1.7 * g), 7.4 * g / staff::CLEF.len() as f32, &staff::CLEF, pal().ink);
        let x0 = r.min.x + 5.0 * g;
        let step_w = (r.width() - 6.0 * g) / per as f32;
        for (k, m) in chunk.iter().enumerate() {
            let i = row * per + k;
            let (letter, acc, oct) = lines::spell(*m, true);
            let step = oct * 7 + letter;
            let y = bottom - (step - 30) as f32 * g / 2.0;
            let x = x0 + (k as f32 + 0.5) * step_w;
            // ledger lines above and below the staff
            let mut s = 28;
            while s >= step {
                let ly = bottom - (s - 30) as f32 * g / 2.0;
                p.line_segment([Pos2::new(x - 7.0, ly), Pos2::new(x + 7.0, ly)], stroke);
                s -= 2;
            }
            let mut s = 40;
            while s <= step {
                let ly = bottom - (s - 30) as f32 * g / 2.0;
                p.line_segment([Pos2::new(x - 7.0, ly), Pos2::new(x + 7.0, ly)], stroke);
                s += 2;
            }
            let col = if lit == Some(i) { pal().red } else { pal().ink };
            p.circle_filled(Pos2::new(x, y), g * 0.6, col);
            if acc != 0 {
                staff::accidental(p, Pos2::new(x - g * 1.7, y), acc, g, col);
            }
        }
    }
}
