//! The PRACTICE window: the song's waveform (tap to jump; the loop, its A and B, the saved loops and the bookmarks on
//! it), then the desktop's tabs - LOOP & SPEED, TRAINER, PITCH & EAR (with the equalizer) and BOOKMARKS.

use crate::*;

/// A text box drawn as Backline's LCD (the phone's keyboard comes up while it is being typed in).
pub(crate) fn text_box(ui: &mut egui::Ui, text: &mut String, w: f32, hint: &str) -> egui::Response {
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
    inset(ui.painter(), r, pal().lcd);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(Vec2::new(5.0, 3.0))));
    child.visuals_mut().override_text_color = Some(pal().ink);
    child.visuals_mut().extreme_bg_color = pal().lcd;
    child.add(
        egui::TextEdit::singleline(text)
            .frame(false)
            .hint_text(egui::RichText::new(hint).color(pal().dim))
            .desired_width(r.width() - 10.0)
            .font(egui::FontId::proportional(13.0)),
    )
}

/// A number with - and + either side, in an LCD (the desktop's stepper).
pub(crate) fn stepper(ui: &mut egui::Ui, text: &str, w: f32) -> i32 {
    let mut d = 0;
    if retro_btn_w(ui, "-", 30.0, false).clicked() {
        d = -1;
    }
    lcd_box(ui, text, w, pal().ink);
    if retro_btn_w(ui, "+", 30.0, false).clicked() {
        d = 1;
    }
    d
}

/// The bookmark colours (the desktop's).
pub(crate) const MARK_COLS: [[u8; 3]; 8] =
    [[235, 80, 80], [245, 150, 50], [240, 215, 70], [110, 210, 100], [80, 200, 210], [90, 140, 245], [175, 120, 240], [240, 120, 190]];

impl App {
    pub(crate) fn practice_ui(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.cur_track().cloned() else {
            para(ui, "Play a song, then loop it, slow it down, change its pitch, train it up to tempo and bookmark it here.", pal().ink2);
            return;
        };
        self.waveform(ui, &t);
        ui.add_space(6.0);
        if let Some(i) = tab_row(ui, &["LOOP & SPEED", "TRAINER", "PITCH & EAR", "BOOKMARKS"], self.mtab as usize) {
            self.mtab = i as u8;
        }
        ui.add_space(4.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            match self.mtab {
                0 => self.loop_tab(ui, &t),
                1 => self.trainer_tab(ui),
                2 => self.ear_tab(ui),
                _ => self.marks_tab(ui, &t),
            }
            ui.add_space(12.0);
        });
    }

    /// The desktop's waveform: the song's shape, the loop, A and B, the saved loops along the bottom, the bookmarks,
    /// where the song is and the time marks. Tap to jump.
    fn waveform(&mut self, ui: &mut egui::Ui, t: &Track) {
        let (bar, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 74.0), Sense::click_and_drag());
        let p = ui.painter().clone();
        inset(&p, bar, pal().lcd);
        let inn = bar.shrink2(Vec2::new(5.0, 5.0));
        let dur = t.duration.max(0.001);
        let at = |x: f32| inn.min.x + inn.width() * (x / dur).clamp(0.0, 1.0);
        let shown = self.seek_drag.unwrap_or_else(|| self.pos());
        if !self.wave.is_empty() {
            let cols = (inn.width() / 2.0).max(1.0) as usize;
            let mid = inn.center().y;
            let n = self.wave.len();
            for c in 0..cols {
                let a = c * n / cols;
                let b = ((c + 1) * n / cols).max(a + 1).min(n);
                let m = self.wave[a.min(n - 1)..b].iter().copied().max().unwrap_or(0);
                let h = (m as f32 / 255.0) * (inn.height() * 0.5);
                let x = inn.min.x + c as f32 * 2.0;
                fill_rect(&p, Rect::from_min_max(Pos2::new(x, mid - h - 0.5), Pos2::new(x + 1.5, mid + h + 0.5)), pal().ink2.gamma_multiply(0.55));
            }
        } else {
            ptext(&p, inn.center(), Align::Center, if self.loading { "LOADING..." } else { "" }, 1.0, pal().dim);
        }
        // saved loops as faint bands along the bottom
        for s in self.sections.get(&t.id).into_iter().flatten() {
            fill_rect(&p, Rect::from_min_max(Pos2::new(at(s.a), inn.max.y - 3.0), Pos2::new(at(s.b).max(at(s.a) + 1.0), inn.max.y)), pal().ink.gamma_multiply(0.6));
        }
        if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
            let col = if self.loop_on { loop_col().gamma_multiply(0.35) } else { pal().ink.gamma_multiply(0.2) };
            fill_rect(&p, Rect::from_min_max(Pos2::new(at(a), inn.min.y), Pos2::new(at(b), inn.max.y)), col);
        }
        for (v, lab) in [(self.loop_a, "A"), (self.loop_b, "B")] {
            if let Some(x) = v {
                fill_rect(&p, Rect::from_min_max(Pos2::new(at(x) - 1.0, bar.min.y + 2.0), Pos2::new(at(x) + 1.0, bar.max.y - 2.0)), loop_col());
                ptext(&p, Pos2::new(at(x) + 4.0, inn.min.y + 5.0), Align::Min, lab, 1.0, loop_col());
            }
        }
        // bookmarks: a line in their colour and a numbered flag
        for (k, (bt, name)) in self.marks.get(&t.id).into_iter().flatten().enumerate() {
            let c = MARK_COLS[k % MARK_COLS.len()];
            let col = Color32::from_rgb(c[0], c[1], c[2]);
            let x = at(*bt);
            fill_rect(&p, Rect::from_min_max(Pos2::new(x - 1.0, bar.min.y + 12.0), Pos2::new(x + 1.0, bar.max.y - 2.0)), col);
            let lab = if name.is_empty() { (k + 1).to_string() } else { fit(name, 1.0, 50.0) };
            let fw = text_w(&lab, 1.0) + 8.0;
            let flag = Rect::from_min_size(Pos2::new(x, bar.max.y - 22.0), Vec2::new(fw, 13.0));
            p.rect_filled(flag, 1.0, col);
            ptext(&p, flag.center(), Align::Center, &lab, 1.0, Color32::from_gray(12));
        }
        fill_rect(&p, Rect::from_min_max(Pos2::new(at(shown) - 1.0, bar.min.y + 2.0), Pos2::new(at(shown) + 1.0, bar.max.y - 2.0)), pal().ink);
        let readout = format!("{} / {}", fmt_t(shown), fmt_t(t.duration));
        ptext(&p, Pos2::new(bar.max.x - 8.0, bar.min.y + 8.0), Align::Max, &readout, 1.0, pal().ink);
        // time marks along the top
        let step = [5.0f32, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0].into_iter().find(|s| dur / s <= 6.0).unwrap_or(600.0);
        let limit = bar.max.x - 16.0 - text_w(&readout, 1.0);
        let mut x0 = step;
        while x0 < dur {
            let x = at(x0);
            fill_rect(&p, Rect::from_min_max(Pos2::new(x, bar.min.y + 2.0), Pos2::new(x + 1.0, bar.min.y + 7.0)), pal().ink);
            if x + 30.0 < limit {
                let lab = format!("{}:{:02}", (x0 / 60.0) as u32, (x0 % 60.0) as u32);
                ptext(&p, Pos2::new(x + 3.0, bar.min.y + 7.0), Align::Min, &lab, 0.7, pal().ink2);
            }
            x0 += step;
        }
        // tap or drag: jump there
        if let Some(pp) = resp.interact_pointer_pos() {
            if resp.dragged() || resp.clicked() {
                self.seek_drag = Some(((pp.x - inn.min.x) / inn.width()).clamp(0.0, 1.0) * dur);
            }
        }
        if resp.drag_stopped() || resp.clicked() {
            if let Some(s) = self.seek_drag.take() {
                self.player.send(Cmd::Seek(s));
            }
        }
    }

    /// LOOP & SPEED: A and B (nudge them), the loop on / off, back 2 seconds, back to the start, the speed, and the
    /// song's saved loops.
    fn loop_tab(&mut self, ui: &mut egui::Ui, t: &Track) {
        let pos = self.pos();
        let dur = t.duration;
        for (which, name) in [(0, "A"), (1, "B")] {
            ui.horizontal(|ui| {
                let v = if which == 0 { self.loop_a } else { self.loop_b };
                if retro_btn_w(ui, name, 40.0, false).clicked() {
                    if which == 0 {
                        self.loop_a = Some(pos);
                    } else {
                        self.loop_b = Some(pos);
                        if self.loop_a.is_some_and(|a| pos > a) {
                            self.loop_on = true;
                        }
                    }
                    self.sync_player();
                }
                let txt = v.map(fmt_t).unwrap_or_else(|| "-:--.-".to_string());
                let d = stepper(ui, &txt, 86.0);
                if d != 0 {
                    let base = v.unwrap_or(pos);
                    let nv = (base + d as f32 * 0.1).clamp(0.0, dur.max(0.1));
                    if which == 0 {
                        self.loop_a = Some(nv);
                    } else {
                        self.loop_b = Some(nv);
                    }
                    self.sync_player();
                }
                if which == 0 {
                    ui.add_space(4.0);
                    let both = self.loop_a.is_some() && self.loop_b.is_some();
                    if icon_btn_w(ui, &IC_REP, self.loop_on && both, pal().ink, 40.0).clicked() && both {
                        self.loop_on = !self.loop_on;
                        self.sync_player();
                    }
                    if icon_btn_w(ui, &IC_X, false, pal().ink, 40.0).clicked() {
                        self.loop_a = None;
                        self.loop_b = None;
                        self.loop_on = false;
                        self.sync_player();
                    }
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            if ibtn(ui, &IC_REW, "2 SEC", false).clicked() {
                self.player.send(Cmd::Seek((pos - 2.0).max(0.0)));
            }
            if ibtn(ui, &IC_PREV, "RESTART", false).clicked() {
                self.player.send(Cmd::Seek(self.loop_a.unwrap_or(0.0)));
            }
        });
        ui.horizontal(|ui| {
            label(ui, "SPEED", 56.0);
            let names: Vec<String> = [25u32, 40, 50, 60, 70, 75, 80, 85, 90, 95, 100, 110, 125, 150].iter().map(|p| format!("{}%", p)).collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let speeds = [25u32, 40, 50, 60, 70, 75, 80, 85, 90, 95, 100, 110, 125, 150];
            let cur = speeds.iter().position(|s| *s >= self.speed).unwrap_or(10);
            if let Some(i) = dropdown(ui, "speed_dd", "", &names, cur, 34.0) {
                self.speed = speeds[i];
                self.sync_player();
            }
            let d = stepper(ui, &format!("{}%", self.speed), 64.0);
            if d != 0 {
                self.speed = (self.speed as i32 + d).clamp(25, 250) as u32;
                self.sync_player();
            }
        });
        ui.add_space(4.0);
        // saved loops of this song
        ui.horizontal(|ui| {
            let w = ui.available_width() - 120.0;
            let r = text_box(ui, &mut self.sec_name, w.max(100.0), "Name this loop (bridge, head, lick...)");
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ibtn(ui, &IC_BOOK, "SAVE LOOP", false).clicked() || enter {
                match (self.loop_a, self.loop_b) {
                    (Some(a), Some(b)) if b > a => {
                        let list = self.sections.entry(t.id).or_default();
                        let name = if self.sec_name.trim().is_empty() { format!("LOOP {}", list.len() + 1) } else { self.sec_name.trim().to_string() };
                        list.push(Section { name, a, b });
                        save_json("loops.json", &self.sections);
                        self.sec_name.clear();
                        self.set_note("LOOP SAVED");
                    }
                    _ => self.set_note("SET LOOP A AND B FIRST"),
                }
            }
        });
        let secs = self.sections.get(&t.id).cloned().unwrap_or_default();
        if secs.is_empty() {
            dim_line(ui, "SAVED LOOPS OF THIS SONG SHOW UP HERE", 1.0, pal().dim);
        }
        let mut drop = None;
        for (k, s) in secs.iter().enumerate() {
            ui.horizontal(|ui| {
                let on = self.loop_on && self.loop_a == Some(s.a) && self.loop_b == Some(s.b);
                let w = ui.available_width() - 48.0;
                if retro_btn_w(ui, &format!("{}   {} - {}", s.name.to_uppercase(), fmt_t(s.a), fmt_t(s.b)), w, on).clicked() {
                    self.loop_a = Some(s.a);
                    self.loop_b = Some(s.b);
                    self.loop_on = true;
                    self.sync_player();
                    self.player.send(Cmd::Seek(s.a));
                }
                if icon_btn_w(ui, &IC_X, false, pal().ink, 40.0).clicked() {
                    drop = Some(k);
                }
            });
        }
        if let Some(k) = drop {
            if let Some(l) = self.sections.get_mut(&t.id) {
                l.remove(k);
            }
            save_json("loops.json", &self.sections);
        }
    }

    /// TRAINER: the count-off before each pass, and the speed trainer (so many passes, then a little faster).
    fn trainer_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            label(ui, "COUNT-OFF", 90.0);
            let txt = if self.count_in == 0 { "OFF".to_string() } else { format!("{} BEATS", self.count_in) };
            if retro_btn_w(ui, &txt, 90.0, self.count_in > 0).clicked() {
                self.count_in = match self.count_in {
                    0 => 2,
                    2 => 4,
                    _ => 0,
                };
                self.sync_player();
            }
            lcd_box(ui, &format!("{:.0} BPM", self.song_bpm), 70.0, pal().ink);
            if retro_btn_w(ui, "TAP", 44.0, false).clicked() {
                if let Some(b) = self.tap() {
                    self.song_bpm = b;
                    self.sync_player();
                }
            }
        });
        dim_line(ui, "CLICKS BEFORE EVERY PASS OF THE LOOP, AT THE SONG'S TEMPO", 1.0, pal().dim);
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            label(ui, "TRAIN", 90.0);
            if retro_btn_w(ui, if self.trainer { "ON" } else { "OFF" }, 56.0, self.trainer).clicked() {
                self.trainer = !self.trainer;
                self.passes = 0;
            }
            if self.trainer {
                lcd_box(ui, &format!("PASS {}/{}", self.passes, self.trainer_n), 90.0, pal().ink);
            }
        });
        ui.horizontal_wrapped(|ui| {
            label(ui, "EVERY", 56.0);
            let d = stepper(ui, &self.trainer_n.to_string(), 36.0);
            self.trainer_n = (self.trainer_n as i32 + d).clamp(1, 20) as u32;
            label(ui, "LOOPS", 50.0);
            let d = stepper(ui, &format!("+{}%", self.trainer_step), 50.0);
            self.trainer_step = (self.trainer_step as i32 + d).clamp(1, 20) as u32;
        });
        dim_line(ui, "STARTS AT THE SPEED SET ON LOOP & SPEED AND STOPS AT 100%", 1.0, pal().dim);
    }

    /// PITCH & EAR: the pitch (semitones and cents), the ear mode, and the equalizer.
    fn ear_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            label(ui, "PITCH", 56.0);
            for (s, d) in [("-", -100), ("-10C", -10)] {
                if retro_btn_w(ui, s, if s.len() > 1 { 48.0 } else { 30.0 }, false).clicked() {
                    self.cents = (self.cents + d).clamp(-1200, 1200);
                    self.sync_player();
                }
            }
            lcd_box(ui, &format!("{:+.2}", self.cents as f32 / 100.0), 60.0, pal().ink);
            for (s, d) in [("+10C", 10), ("+", 100)] {
                if retro_btn_w(ui, s, if s.len() > 1 { 48.0 } else { 30.0 }, false).clicked() {
                    self.cents = (self.cents + d).clamp(-1200, 1200);
                    self.sync_player();
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            label(ui, "EAR", 56.0);
            if retro_btn_w(ui, CHAN_NAMES[self.chan as usize % 6], 150.0, self.chan != 0).clicked() {
                self.chan = (self.chan + 1) % 6;
                self.sync_player();
            }
        });
        dim_line(ui, "LEFT / RIGHT: ONE SIDE IN BOTH EARS. NO CENTER: VOCALS OUT. BASS ONLY: THE LOW END.", 1.0, pal().dim);
        ui.add_space(8.0);
        section_header(ui, "EQUALIZER");
        ui.horizontal_wrapped(|ui| {
            if check_box(ui, "ON", self.eq_on).clicked() {
                self.eq_on = !self.eq_on;
                self.player.eq.set(self.eq, 0.0, self.eq_on);
                self.save_settings();
            }
            for (name, g) in EQ_PRESETS {
                if retro_btn(ui, name, self.eq == g).clicked() {
                    self.eq = g;
                    self.eq_on = true;
                    self.player.eq.set(self.eq, 0.0, self.eq_on);
                    self.save_settings();
                }
            }
        });
        // ten sliders, the desktop's way: a groove and a raised thumb (drag up and down, +-12 dB)
        let bands = ["31", "62", "125", "250", "500", "1K", "2K", "4K", "8K", "16K"];
        let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 150.0), Sense::click_and_drag());
        let p = ui.painter();
        let area = Rect::from_min_max(r.min + Vec2::new(0.0, 6.0), Pos2::new(r.max.x, r.max.y - 18.0));
        let col_w = area.width() / 10.0;
        let mid = area.center().y;
        if let Some(pp) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let k = (((pp.x - area.min.x) / col_w).floor() as usize).min(9);
            self.eq[k] = (((mid - pp.y) / (area.height() / 2.0)) * 12.0).clamp(-12.0, 12.0).round();
            self.eq_on = true;
            self.player.eq.set(self.eq, 0.0, true);
        }
        if resp.drag_stopped() || resp.clicked() {
            self.save_settings();
        }
        fill_rect(p, Rect::from_min_max(Pos2::new(area.min.x, mid), Pos2::new(area.max.x, mid + 1.0)), pal().groove);
        for (k, b) in bands.iter().enumerate() {
            let cx = area.min.x + col_w * (k as f32 + 0.5);
            let groove = Rect::from_center_size(Pos2::new(cx, mid), Vec2::new(7.0, area.height()));
            inset(p, groove, pal().groove);
            let y = mid - self.eq[k] / 12.0 * (area.height() / 2.0 - 6.0);
            raised(p, Rect::from_center_size(Pos2::new(cx, y), Vec2::new((col_w - 8.0).clamp(14.0, 24.0), 11.0)), false);
            ptext(p, Pos2::new(cx, r.max.y - 8.0), Align::Center, b, 1.0, pal().ink2);
        }
    }

    /// BOOKMARKS: this song's fixed points - add one where the song is, jump to one, name it, take it away.
    fn marks_tab(&mut self, ui: &mut egui::Ui, t: &Track) {
        let now = self.pos();
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, &format!("+ BOOKMARK AT {}", fmt_t(now)), false).clicked() {
                let l = self.marks.entry(t.id).or_default();
                l.push((now, String::new()));
                l.sort_by(|a, b| a.0.total_cmp(&b.0));
                save_json("bookmarks.json", &self.marks);
            }
            if self.marks.get(&t.id).is_some_and(|l| !l.is_empty()) && retro_btn(ui, "REMOVE ALL", false).clicked() {
                self.marks.remove(&t.id);
                save_json("bookmarks.json", &self.marks);
            }
        });
        let n = self.marks.get(&t.id).map_or(0, |l| l.len());
        if n == 0 {
            para(ui, "No bookmarks on this song yet. They are fixed points to find your way: the solo, the key change, the bar you keep missing.", pal().ink2);
            return;
        }
        let mut drop = None;
        let mut changed = false;
        for k in 0..n {
            let (at, _) = self.marks[&t.id][k].clone();
            ui.horizontal(|ui| {
                let c = MARK_COLS[k % MARK_COLS.len()];
                let (sw, _) = ui.allocate_exact_size(Vec2::new(22.0, bh()), Sense::hover());
                ui.painter().rect_filled(sw.shrink2(Vec2::new(2.0, 4.0)), 2.0, Color32::from_rgb(c[0], c[1], c[2]));
                ptext(ui.painter(), sw.center(), Align::Center, &(k + 1).to_string(), 1.0, Color32::from_gray(12));
                if retro_btn_w(ui, &fmt_t(at), 70.0, false).clicked() {
                    self.player.send(Cmd::Seek(at));
                }
                let w = ui.available_width() - 48.0;
                if let Some(l) = self.marks.get_mut(&t.id) {
                    if text_box(ui, &mut l[k].1, w.max(60.0), "Name it (solo, bridge...)").changed() {
                        changed = true;
                    }
                }
                if icon_btn_w(ui, &IC_X, false, pal().ink, 40.0).clicked() {
                    drop = Some(k);
                }
            });
        }
        if let Some(k) = drop {
            if let Some(l) = self.marks.get_mut(&t.id) {
                l.remove(k);
            }
            changed = true;
        }
        if changed {
            save_json("bookmarks.json", &self.marks);
        }
    }

    /// Tap the tempo in: the beats tapped in the last few seconds give it (None until two taps).
    pub(crate) fn tap(&mut self) -> Option<f32> {
        let now = Instant::now();
        self.taps.retain(|x| now.duration_since(*x).as_secs_f32() < 3.0);
        self.taps.push(now);
        (self.taps.len() >= 2).then(|| {
            let span = now.duration_since(self.taps[0]).as_secs_f32();
            (60.0 * (self.taps.len() - 1) as f32 / span).clamp(30.0, 300.0).round()
        })
    }
}
