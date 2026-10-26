//! The METRONOME window, the desktop's own: start / stop, its tempo (tap it in), its own volume, a swinging pendulum
//! and the beat boxes (tap one: its volume, 3 loud ... 0 silent), the meter and what a beat is, accent groupings for
//! odd meters, subdivisions and volume presets, bars of silence, and a tempo that climbs.

use crate::practice::stepper;
use crate::*;

impl App {
    /// The click as set, from the top of the bar (or silence when it is off).
    pub(crate) fn metro_apply(&mut self) {
        if self.mt.on {
            let tempo = self.mt_bpm as f32;
            let body = band::click_loop(&self.mt, tempo);
            let beats = self.mt.beats.clamp(1, 32) as usize;
            let cycle = beats * if self.mt.gap_play > 0 { (self.mt.gap_play + self.mt.gap_mute.max(1)) as usize } else { 1 };
            self.mt_vis = Some((Instant::now(), 60.0 / self.mt.pulse(tempo), cycle));
            self.player.send(Cmd::Pcm(0, Vec::new(), body, 0.8));
        } else {
            self.player.send(Cmd::Pcm(0, Vec::new(), Vec::new(), 0.0));
            self.mt_vis = None;
        }
    }

    pub(crate) fn metro_ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().id_salt("metro").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 8.0);
            let mut changed = false;
            let on = self.mt.on;
            // the beats as dots (the one sounding lit), the tempo big, steps round START, then chips
            let beats = self.mt.beats.clamp(1, 16) as usize;
            let beat_now = match (on, self.mt_vis) {
                (true, Some((t0, spb, _))) => {
                    ui.ctx().request_repaint_after(Duration::from_millis(30));
                    Some(((Instant::now() - t0).as_secs_f32() / spb.max(0.05)).floor() as usize % beats)
                }
                _ => None,
            };
            let w = ui.available_width();
            let (dr, _) = ui.allocate_exact_size(Vec2::new(w, 30.0), Sense::hover());
            let gap = 26.0f32.min((w - 20.0) / beats as f32);
            for k in 0..beats {
                let c = Pos2::new(dr.center().x + (k as f32 - (beats - 1) as f32 / 2.0) * gap, dr.center().y);
                let lit = beat_now == Some(k);
                ui.painter().circle_filled(c, if k == 0 { 9.0 } else { 7.5 }, if lit { pal().red } else { pal().beige_lt });
            }
            let (br, _) = ui.allocate_exact_size(Vec2::new(w, 120.0), Sense::hover());
            ptext(ui.painter(), Pos2::new(br.center().x, br.center().y - 10.0), Align::Center, &self.mt_bpm.to_string(), 6.0, pal().ink);
            ptext(ui.painter(), Pos2::new(br.center().x, br.max.y - 12.0), Align::Center, "bpm", 1.2, pal().dim);
            let (sr, _) = ui.allocate_exact_size(Vec2::new(w, 72.0), Sense::hover());
            let mid = sr.center();
            ui.painter().circle_filled(mid, 32.0, pal().red);
            play_glyph(ui.painter(), mid, on, pal().bar_txt);
            if ui.interact(Rect::from_center_size(mid, Vec2::splat(64.0)), ui.id().with("mt_go"), Sense::click()).clicked() {
                self.mt.on = !self.mt.on;
                changed = true;
            }
            for (dx, d) in [(-122.0, -5i32), (-66.0, -1), (66.0, 1), (122.0, 5)] {
                let c = mid + Vec2::new(dx, 0.0);
                let r = Rect::from_center_size(c, Vec2::splat(46.0));
                let resp = ui.interact(r, ui.id().with(("mt_step", d)), Sense::click());
                ui.painter().circle_filled(c, 22.0, if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige_lt });
                ptext(ui.painter(), c, Align::Center, &format!("{:+}", d), 1.3, pal().ink);
                if resp.clicked() {
                    self.mt_bpm = (self.mt_bpm as i32 + d).clamp(20, 400) as u32;
                    changed = true;
                }
            }
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "Tap", false).clicked() {
                    if let Some(b) = self.tap() {
                        self.mt_bpm = b as u32;
                        changed = true;
                    }
                }
                if retro_btn(ui, &format!("{}/{}", self.mt.beats, self.mt.unit), self.mt_more).clicked() {
                    self.mt_more = !self.mt_more;
                }
                if retro_btn(ui, "Accents", self.mt_more).clicked() {
                    self.mt_more = !self.mt_more;
                }
                if retro_btn(ui, "More", self.mt_more).clicked() {
                    self.mt_more = !self.mt_more;
                }
            });
            if !self.mt_more {
                if changed {
                    self.metro_apply();
                    self.save_settings();
                }
                return;
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                // its own volume: louder or softer against the song
                label(ui, "VOL", 36.0);
                let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width().min(220.0), bh()), Sense::click_and_drag());
                inset(ui.painter(), r, pal().lcd);
                let inner = r.shrink(4.0);
                ui.painter().rect_filled(Rect::from_min_size(inner.min, Vec2::new(inner.width() * self.mt_vol, inner.height())), 0.0, pal().ink2);
                if let Some(p) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
                    self.mt_vol = ((p.x - inner.min.x) / inner.width()).clamp(0.0, 1.0);
                    self.player.send(Cmd::MetroVolume(self.mt_vol * self.mt_vol));
                }
                if resp.drag_stopped() || resp.clicked() {
                    self.save_settings();
                }
            });
            // the pendulum and the beats
            self.metro_vis(ui, &mut changed);
            ui.horizontal_wrapped(|ui| {
                label(ui, "METER", 44.0);
                let d = stepper(ui, &self.mt.beats.to_string(), 26.0);
                if d != 0 {
                    self.mt.beats = (self.mt.beats as i32 + d).clamp(1, 32) as u32;
                    self.mt_group = 0;
                    changed = true;
                }
                label(ui, "/", 8.0);
                for n in [4u8, 8, 16] {
                    if retro_btn_w(ui, &n.to_string(), 28.0, self.mt.unit == n).clicked() {
                        self.mt.unit = n;
                        changed = true;
                    }
                }
            });
            let gs = band::groupings(self.mt.beats);
            if gs.len() > 1 {
                ui.horizontal_wrapped(|ui| {
                    label(ui, "ACCENT", 56.0);
                    let txt = match self.mt_group {
                        0 => "GROUPS".to_string(),
                        k => gs[k - 1].iter().map(|n| n.to_string()).collect::<Vec<_>>().join("+"),
                    };
                    if retro_btn_w(ui, &txt, 160.0, self.mt_group > 0).clicked() {
                        self.mt_group = (self.mt_group + 1) % (gs.len() + 1);
                        match self.mt_group {
                            0 => self.mt.preset(0),
                            k => self.mt.apply_group(&gs[k - 1]),
                        }
                        changed = true;
                    }
                });
            }
            ui.horizontal_wrapped(|ui| {
                label(ui, "SOUND", 56.0);
                let sub = ["NO SUBDIV", "EIGHTHS", "TRIPLETS", "SIXTEENTHS"][(self.mt.sub as usize).min(3)];
                if retro_btn_w(ui, sub, 120.0, self.mt.sub > 0).clicked() {
                    self.mt.sub = (self.mt.sub + 1) % 4;
                    changed = true;
                }
            });
            ui.horizontal_wrapped(|ui| {
                label(ui, "", 56.0);
                for (i, name) in ["ALL", "2 & 4", "1 ONLY"].iter().enumerate() {
                    if retro_btn_w(ui, name, 66.0, false).clicked() {
                        self.mt_group = 0;
                        self.mt.preset(i as u8);
                        changed = true;
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                label(ui, "GAP", 56.0);
                let p = if self.mt.gap_play == 0 { "OFF".to_string() } else { self.mt.gap_play.to_string() };
                let d = stepper(ui, &p, 40.0);
                if d != 0 {
                    self.mt.gap_play = (self.mt.gap_play as i32 + d).clamp(0, 16) as u32;
                    changed = true;
                }
            });
            ui.horizontal_wrapped(|ui| {
                label(ui, "THEN MUTE", 90.0);
                let d = stepper(ui, &self.mt.gap_mute.to_string(), 30.0);
                if d != 0 {
                    self.mt.gap_mute = (self.mt.gap_mute as i32 + d).clamp(1, 16) as u32;
                    changed = true;
                }
            });
            dim_line(ui, "BARS OF CLICK, THEN BARS OF SILENCE - KEEP TIME ON YOUR OWN", 1.0, pal().dim);
            ui.horizontal_wrapped(|ui| {
                label(ui, "FASTER", 56.0);
                let r = if self.mt.ramp_bars == 0 { "OFF".to_string() } else { self.mt.ramp_bars.to_string() };
                let d = stepper(ui, &r, 40.0);
                if d != 0 {
                    self.mt.ramp_bars = (self.mt.ramp_bars as i32 + d).clamp(0, 64) as u32;
                    changed = true;
                }
            });
            ui.horizontal_wrapped(|ui| {
                label(ui, "BY BPM", 90.0);
                let d = stepper(ui, &format!("+{}", self.mt.ramp_bpm), 36.0);
                if d != 0 {
                    self.mt.ramp_bpm = (self.mt.ramp_bpm as i32 + d).clamp(1, 20) as u32;
                    changed = true;
                }
            });
            dim_line(ui, "EVERY SO MANY BARS THE TEMPO GOES UP", 1.0, pal().dim);
            if changed {
                self.metro_apply();
                self.save_settings();
            }
            ui.add_space(16.0);
        });
    }

    /// The desktop's pendulum (it swings with the beat and its tip flashes on it) and the beat boxes.
    fn metro_vis(&mut self, ui: &mut egui::Ui, changed: &mut bool) {
        let beats = self.mt.beats.clamp(1, 32) as usize;
        let (beat_no, frac, muted) = match (self.mt.on, self.mt_vis) {
            (true, Some((t0, spb, cycle))) => {
                let e = (Instant::now() - t0).as_secs_f32() / spb.max(0.05);
                let n = e.floor() as usize % cycle.max(1);
                let silent = self.mt.gap_play > 0 && n >= self.mt.gap_play as usize * beats;
                (Some(n % beats), e.fract(), silent)
            }
            _ => (None, 0.0, false),
        };
        if self.mt.on {
            ui.ctx().request_repaint();
        }
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 150.0), Sense::hover());
        inset(ui.painter(), rect, pal().lcd);
        let p = ui.painter();
        let pivot = Pos2::new(rect.center().x, rect.max.y - 14.0);
        let e = match (self.mt.on, self.mt_vis) {
            (true, Some((t0, spb, _))) => Some((Instant::now() - t0).as_secs_f32() / spb.max(0.05)),
            _ => None,
        };
        let swing = e.map_or(0.0, |e| (std::f32::consts::PI * e).cos());
        let len = 118.0;
        for a in [-0.62f32, 0.0, 0.62] {
            let q = pivot + Vec2::new(a.sin() * (len + 8.0), -a.cos() * (len + 8.0));
            fill_rect(p, Rect::from_center_size(q, Vec2::new(2.0, 6.0)), pal().groove);
        }
        let rod = |ang: f32, col: Color32, size: f32| {
            let tip = pivot + Vec2::new(ang.sin() * len, -ang.cos() * len);
            let n = (len / 2.0) as i32;
            for k in 0..=n {
                let q = pivot + (tip - pivot) * (k as f32 / n as f32);
                fill_rect(p, Rect::from_center_size(q, Vec2::splat(size)), col);
            }
            tip
        };
        // two faint afterimages so the motion reads smooth
        if let Some(e) = e {
            for (back, col) in [(0.16f32, pal().lcd_ghost), (0.08, pal().groove)] {
                let _ = rod(0.62 * (std::f32::consts::PI * (e - back)).cos(), col, 2.0);
            }
        }
        let tip = rod(swing * 0.62, pal().ink2, 2.0);
        let weight = pivot + (tip - pivot) * 0.7;
        fill_rect(p, Rect::from_center_size(weight, Vec2::new(14.0, 10.0)), pal().ink);
        fill_rect(p, Rect::from_center_size(weight, Vec2::new(8.0, 4.0)), pal().lcd);
        let hit = if self.mt.on && !muted { (1.0 - frac / 0.35).clamp(0.0, 1.0) } else { 0.0 };
        let mix = |a: Color32, b: Color32, k: f32| {
            let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k) as u8;
            Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
        };
        fill_rect(p, Rect::from_center_size(tip, Vec2::splat(12.0)), pal().ink);
        fill_rect(p, Rect::from_center_size(tip, Vec2::splat(8.0)), mix(pal().lcd, pal().red, hit));
        fill_rect(p, Rect::from_center_size(pivot, Vec2::new(30.0, 8.0)), pal().ink);
        ptext(p, Pos2::new(rect.min.x + 6.0, rect.min.y + 8.0), Align::Min, "MUTED", 1.0, if muted { pal().red } else { pal().lcd });
        ptext(p, Pos2::new(rect.max.x - 6.0, rect.min.y + 8.0), Align::Max, &format!("{} BPM", self.mt_bpm), 1.0, pal().ink2);
        // the beat boxes: up to 8 to a row
        let per = beats.min(8);
        let rows = beats.div_ceil(per);
        let w = ((ui.available_width() - 6.0) / per as f32 - 6.0).clamp(18.0, 60.0);
        for row in 0..rows {
            ui.horizontal(|ui| {
                for b in (row * per)..((row + 1) * per).min(beats) {
                    if self.beat_box(ui, b, w, 76.0, beat_no == Some(b) && !muted) {
                        self.mt_group = 0;
                        let l = &mut self.mt.levels[b.min(31)];
                        *l = if *l == 0 { 3 } else { *l - 1 };
                        *changed = true;
                    }
                }
            });
        }
        dim_line(ui, "TAP A BEAT FOR ITS VOLUME: 3 LOUD ... 0 SILENT", 1.0, pal().dim);
    }

    /// One beat: its number and its volume pips (true when tapped).
    fn beat_box(&mut self, ui: &mut egui::Ui, b: usize, w: f32, h: f32, now_beat: bool) -> bool {
        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
        let lvl = self.mt.levels[b].min(3) as usize;
        inset(ui.painter(), r, if now_beat && lvl > 0 { pal().sel } else { pal().lcd });
        let p = ui.painter();
        for k in 0..3usize {
            let pip = Rect::from_min_size(Pos2::new(r.min.x + 5.0, r.max.y - 18.0 - k as f32 * 16.0), Vec2::new(r.width() - 10.0, 11.0));
            let lit = k < lvl;
            fill_rect(p, pip, if lit { if now_beat { pal().red } else { pal().ink2 } } else { pal().lcd_ghost });
        }
        ptext(p, Pos2::new(r.center().x, r.min.y + 9.0), Align::Center, &(b + 1).to_string(), if w >= 24.0 { 2.0 } else { 1.0 }, if now_beat { pal().red } else { pal().ink });
        if now_beat {
            outline(p, r, 2.0, pal().red);
        }
        resp.clicked()
    }
}
