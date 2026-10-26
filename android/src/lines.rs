//! LINES: the desktop's ATONAL lines, cut down for a phone. A generated line (alternate triads from a scale, or an
//! interval cycle) on the long neck, its notes numbered in the order you play them, from the start and finger you
//! pick (every string, every finger); the pickers pull up from the bottom.

use crate::*;

/// The scales the phone makes alternate-triad lines from: name, semitones over the root.
const SCALES: [(&str, &[i32]); 6] = [
    ("Major", &[0, 2, 4, 5, 7, 9, 11]),
    ("Melodic minor", &[0, 2, 3, 5, 7, 9, 11]),
    ("Harmonic minor", &[0, 2, 3, 5, 7, 8, 11]),
    ("Diminished", &[0, 2, 3, 5, 6, 8, 9, 11]),
    ("Whole tone", &[0, 2, 4, 6, 8, 10]),
    ("Altered", &[0, 1, 3, 4, 6, 8, 10]),
];
const CYCLES: [(&str, i32); 4] = [("Minor 3rds", 3), ("Major 3rds", 4), ("4ths", 5), ("Tritones", 6)];

impl App {
    /// The line as written (treble clef; the guitar sounds it an octave lower) and its name.
    fn ln_line(&self) -> (Vec<i32>, String) {
        let root = theory::note_name(self.ln_root, false);
        if self.ln_kind == 0 {
            let (name, steps) = SCALES[self.ln_scale % SCALES.len()];
            (atonal_core::alternate_triads(self.ln_root, steps), format!("Alternate triads, {} {}", root, name.to_lowercase()))
        } else {
            let (name, semis) = CYCLES[self.ln_cycle % CYCLES.len()];
            (atonal_core::interval_cycle(self.ln_root, semis), format!("Cycle of {}, from {}", name.to_lowercase(), root))
        }
    }

    pub(crate) fn lines_ui(&mut self, ui: &mut egui::Ui) {
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

        let avail = ui.available_rect_before_wrap();
        let sheet_h = if self.ln_sheet { 300.0 } else { 64.0 };
        let neck_r = Rect::from_min_max(avail.min, Pos2::new(avail.max.x, avail.max.y - sheet_h - 8.0));
        line_neck(ui.painter(), neck_r, &spots);

        // the sheet: what the line is, Play, and (pulled up) the pickers
        let sr = Rect::from_min_max(Pos2::new(avail.min.x - 4.0, avail.max.y - sheet_h), Pos2::new(avail.max.x + 4.0, avail.max.y + 6.0));
        ui.painter().rect_filled(sr, egui::Rounding { nw: 16.0, ne: 16.0, sw: 0.0, se: 0.0 }, pal().beige_lt);
        ui.painter().rect_filled(Rect::from_center_size(Pos2::new(sr.center().x, sr.min.y + 8.0), Vec2::new(40.0, 4.0)), 2.0, pal().beige_dk);
        let head = Rect::from_min_size(sr.min + Vec2::new(12.0, 14.0), Vec2::new(sr.width() - 24.0, 44.0));
        let p = ui.painter().clone();
        ptext_fit(&p, Pos2::new(head.min.x, head.min.y + 12.0), Align::Min, &name, 1.3, head.width() - 60.0, pal().ink);
        let str_name = ["low E", "A", "D", "G", "B", "high E"][start.0];
        ptext_fit(&p, Pos2::new(head.min.x, head.min.y + 30.0), Align::Min, &format!("{} string, fret {}, finger {}", str_name, start.1, finger), 1.1, head.width() - 60.0, pal().dim);
        let pc = Pos2::new(head.max.x - 22.0, head.center().y);
        // Play, or Stop while the line is sounding
        let playing = self.ln_end.is_some_and(|e| Instant::now() < e);
        if playing {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        p.circle_filled(pc, 22.0, pal().red);
        play_glyph(&p, pc, playing, pal().bar_txt);
        if ui.interact(Rect::from_center_size(pc, Vec2::splat(44.0)), ui.id().with("ln_play"), Sense::click()).clicked() {
            self.player.send(Cmd::StopOnce);
            if playing {
                self.ln_end = None;
            } else {
                let notes: Vec<Vec<i32>> = played.iter().map(|m| vec![*m]).collect();
                self.player.send(Cmd::Once(band::line_sound(&notes, 0.28), 0.9));
                self.ln_end = Some(Instant::now() + Duration::from_secs_f32(notes.len() as f32 * 0.28 + 1.0));
            }
        }
        let toggle = Rect::from_min_max(sr.min, Pos2::new(head.max.x - 48.0, head.max.y));
        if ui.interact(toggle, ui.id().with("ln_sheet"), Sense::click()).clicked() {
            self.ln_sheet = !self.ln_sheet;
        }
        if !self.ln_sheet {
            return;
        }
        let body = Rect::from_min_max(Pos2::new(sr.min.x + 12.0, head.max.y + 6.0), sr.max - Vec2::new(12.0, 10.0));
        let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(Align::Min)));
        egui::ScrollArea::vertical().id_salt("ln_pick").auto_shrink([false, false]).show(&mut ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
            ui.horizontal_wrapped(|ui| {
                for (k, n) in ["Alternate triads", "Interval cycle"].iter().enumerate() {
                    if retro_btn(ui, n, self.ln_kind as usize == k).clicked() {
                        self.ln_kind = k as u8;
                        self.ln_start = 0;
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                for pc in 0..12 {
                    if retro_btn_w(ui, theory::note_name(pc, false), 40.0, self.ln_root == pc).clicked() {
                        self.ln_root = pc;
                        self.ln_start = 0;
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                if self.ln_kind == 0 {
                    for (k, (n, _)) in SCALES.iter().enumerate() {
                        if retro_btn(ui, n, self.ln_scale == k).clicked() {
                            self.ln_scale = k;
                            self.ln_start = 0;
                        }
                    }
                } else {
                    for (k, (n, _)) in CYCLES.iter().enumerate() {
                        if retro_btn(ui, n, self.ln_cycle == k).clicked() {
                            self.ln_cycle = k;
                            self.ln_start = 0;
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                label(ui, "Start", 56.0);
                if retro_btn_w(ui, "<", 40.0, false).clicked() {
                    self.ln_start = (self.ln_start + starts.len() - 1) % starts.len();
                }
                if retro_btn_w(ui, ">", 40.0, false).clicked() {
                    self.ln_start = (self.ln_start + 1) % starts.len();
                }
            });
            ui.horizontal(|ui| {
                label(ui, "Finger", 56.0);
                for f in 1..=4u8 {
                    if retro_btn_w(ui, &f.to_string(), 40.0, finger == f).clicked() {
                        self.ln_finger = f;
                    }
                }
            });
        });
    }
}

/// The long neck for a line: the strings down the screen, a window of frets around the line, and each note a dot
/// numbered in the order it is played (a spot played twice keeps its first number).
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
    let rad = (sw * 0.38).min(fh * 0.44).min(12.0);
    let mut seen: Vec<(usize, u8)> = Vec::new();
    for (i, sp) in spots.iter().enumerate() {
        if seen.contains(sp) || sp.1 < lo_f || sp.1 > hi_f {
            continue;
        }
        seen.push(*sp);
        let c = Pos2::new(x0 + sp.0 as f32 * sw, y0 + (sp.1 - lo_f) as f32 * fh + fh / 2.0);
        let first = i == 0;
        p.circle_filled(c, rad, if first { pal().red } else { pal().ink });
        ptext(p, c, Align::Center, &(i + 1).to_string(), 0.95, if first { pal().bar_txt } else { pal().beige_lt });
    }
}
