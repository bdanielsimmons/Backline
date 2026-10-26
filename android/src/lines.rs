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
fn nice(name: &str) -> String {
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

        let avail = ui.available_rect_before_wrap();
        let sheet_h = if self.ln_sheet { (avail.height() * 0.62).max(300.0) } else { 64.0 };
        let neck_r = Rect::from_min_max(avail.min, Pos2::new(avail.max.x, avail.max.y - sheet_h - 8.0));
        line_neck(ui.painter(), neck_r, &spots);

        // the sheet: what the line is, Play, and (pulled up) the player, the pickers and the analysis
        let sr = Rect::from_min_max(Pos2::new(avail.min.x - 4.0, avail.max.y - sheet_h), Pos2::new(avail.max.x + 4.0, avail.max.y + 6.0));
        ui.painter().rect_filled(sr, egui::Rounding { nw: 16.0, ne: 16.0, sw: 0.0, se: 0.0 }, pal().beige_lt);
        ui.painter().rect_filled(Rect::from_center_size(Pos2::new(sr.center().x, sr.min.y + 8.0), Vec2::new(40.0, 4.0)), 2.0, pal().beige_dk);
        let head = Rect::from_min_size(sr.min + Vec2::new(12.0, 14.0), Vec2::new(sr.width() - 24.0, 44.0));
        let p = ui.painter().clone();
        ptext_fit(&p, Pos2::new(head.min.x, head.min.y + 12.0), Align::Min, &name, 1.3, head.width() - 60.0, pal().ink);
        let str_name = ["low E", "A", "D", "G", "B", "high E"][start.0];
        ptext_fit(&p, Pos2::new(head.min.x, head.min.y + 30.0), Align::Min, &format!("{} string, fret {}, finger {}", str_name, start.1, finger), 1.1, head.width() - 60.0, pal().dim);
        let pc = Pos2::new(head.max.x - 22.0, head.center().y);
        p.circle_filled(pc, 22.0, pal().red);
        play_glyph(&p, pc, playing, pal().bar_txt);
        if ui.interact(Rect::from_center_size(pc, Vec2::splat(44.0)), ui.id().with("ln_play"), Sense::click()).clicked() {
            if playing {
                self.player.send(Cmd::StopOnce);
                self.ln_end = None;
            } else {
                self.ln_play(&played);
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
        let mut restart = false;
        egui::ScrollArea::vertical().id_salt("ln_pick").auto_shrink([false, false]).show(&mut ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
            // the player
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "Loop", self.ln_loop).clicked() {
                    self.ln_loop = !self.ln_loop;
                }
                if retro_btn_w(ui, "-", 36.0, false).clicked() {
                    self.ln_bpm = self.ln_bpm.saturating_sub(5).max(20);
                }
                label(ui, &format!("{} bpm", self.ln_bpm), 70.0);
                if retro_btn_w(ui, "+", 36.0, false).clicked() {
                    self.ln_bpm = (self.ln_bpm + 5).min(400);
                }
            });
            let feel_names: Vec<&str> = FEELS.iter().map(|f| f.0).collect();
            let cur = FEELS.iter().position(|f| f.1 == self.ln_sub).unwrap_or(1);
            ui.horizontal(|ui| {
                label(ui, "Feel", 56.0);
                if let Some(i) = dropdown(ui, "ln_feel", "", &feel_names, cur, 0.0) {
                    self.ln_sub = FEELS[i].1;
                }
            });
            ui.add_space(4.0);
            // the line
            ui.horizontal_wrapped(|ui| {
                for (k, n) in KINDS.iter().enumerate() {
                    if retro_btn(ui, n, self.ln_kind as usize == k).clicked() {
                        self.ln_kind = k as u8;
                        self.ln_start = 0;
                        restart = true;
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                for pc in 0..12 {
                    if retro_btn_w(ui, theory::note_name(pc, false), 40.0, self.ln_root == pc).clicked() {
                        self.ln_root = pc;
                        self.ln_start = 0;
                        restart = true;
                    }
                }
            });
            match self.ln_kind {
                0 => {
                    let names: Vec<String> = atonal_core::scale_list().iter().map(|s| nice(s.0)).collect();
                    let names: Vec<&str> = names.iter().map(String::as_str).collect();
                    ui.horizontal(|ui| {
                        label(ui, "Scale", 56.0);
                        if let Some(i) = dropdown(ui, "ln_scale", "", &names, self.ln_scale.min(names.len() - 1), 0.0) {
                            self.ln_scale = i;
                            self.ln_start = 0;
                            restart = true;
                        }
                    });
                }
                1 => {
                    let names: Vec<String> = music_data::TETRACHORDS.iter().map(|t| nice(t.0)).collect();
                    let names: Vec<&str> = names.iter().map(String::as_str).collect();
                    for (k, title) in ["Lower", "Upper"].iter().enumerate() {
                        ui.horizontal(|ui| {
                            label(ui, title, 56.0);
                            let cur = if k == 0 { self.ln_lower } else { self.ln_upper };
                            if let Some(i) = dropdown(ui, if k == 0 { "ln_lower" } else { "ln_upper" }, "", &names, cur.min(names.len() - 1), 0.0) {
                                if k == 0 {
                                    self.ln_lower = i;
                                } else {
                                    self.ln_upper = i;
                                }
                                self.ln_start = 0;
                                restart = true;
                            }
                        });
                    }
                }
                _ => {
                    ui.horizontal_wrapped(|ui| {
                        for (k, (n, _)) in CYCLES.iter().enumerate() {
                            if retro_btn(ui, n, self.ln_cycle == k).clicked() {
                                self.ln_cycle = k;
                                self.ln_start = 0;
                                restart = true;
                            }
                        }
                    });
                }
            }
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
            // what the line is made of
            ui.add_space(8.0);
            analysis(ui, &written);
        });
        if restart && playing {
            self.player.send(Cmd::StopOnce);
            self.ln_end = None;
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
