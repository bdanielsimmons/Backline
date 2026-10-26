//! The PRACTICE window: the song's waveform (tap to jump; the loop, its A and B, the saved loops and the bookmarks on
//! it), then the desktop's tabs - LOOP & SPEED, TRAINER, PITCH & EAR (with the equalizer) and BOOKMARKS.

use crate::*;

/// A text box drawn as Backline's LCD (the phone's keyboard comes up while it is being typed in).
pub(crate) fn text_box(ui: &mut egui::Ui, text: &mut String, w: f32, hint: &str) -> egui::Response {
    // a rounded, flat field in the panel's lighter shade
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 38.0), Sense::hover());
    ui.painter().rect_filled(r.shrink2(Vec2::new(0.0, 1.0)), 10.0, pal().beige_lt);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(Vec2::new(12.0, 8.0))));
    child.visuals_mut().override_text_color = Some(pal().ink);
    child.visuals_mut().extreme_bg_color = pal().beige_lt;
    child.add(
        egui::TextEdit::singleline(text)
            .frame(false)
            .hint_text(egui::RichText::new(hint).color(pal().dim))
            .desired_width(r.width() - 24.0)
            .font(egui::FontId::proportional(14.0)),
    )
}

/// A number with - and + either side, in an LCD (the desktop's stepper).
pub(crate) fn stepper(ui: &mut egui::Ui, text: &str, w: f32) -> i32 {
    let mut d = 0;
    if pill_w(ui, "-", 30.0, false).clicked() {
        d = -1;
    }
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 34.0), Sense::hover());
    ptext_fit(ui.painter(), r.center(), Align::Center, text, 1.3, w - 4.0, pal().ink);
    if pill_w(ui, "+", 30.0, false).clicked() {
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
            note_text(ui, "Play a song, then loop it, slow it down, change its pitch, train it up to tempo and bookmark it here.", pal().ink2);
            return;
        };
        self.waveform(ui, &t);
        ui.add_space(6.0);
        let (tr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
        if let Some(i) = seg_tabs(ui, tr, &["Loop", "Trainer", "Pitch", "Marks"], self.mtab as usize, "prac_tabs") {
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
    /// where the song is and the time marks. Tap to jump; drag to make a loop, drag A or B to move it, drag inside
    /// the loop to slide it. Zoom in with + and back with FIT; zoomed, it follows the song.
    fn waveform(&mut self, ui: &mut egui::Ui, t: &Track) {
        let (bar, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 84.0), Sense::click_and_drag());
        let p = ui.painter().clone();
        inset(&p, bar, pal().lcd);
        let inn = bar.shrink2(Vec2::new(5.0, 5.0));
        let dur = t.duration.max(0.001);
        let pos = self.pos();
        // the part of the song in view
        self.wv_zoom = self.wv_zoom.clamp(1.0, 64.0);
        let vlen = dur / self.wv_zoom;
        if self.wv_zoom > 1.0 && self.wv_drag.is_none() && !self.paused && (pos < self.wv_off || pos > self.wv_off + vlen) {
            self.wv_off = pos - vlen * 0.1;
        }
        self.wv_off = self.wv_off.clamp(0.0, (dur - vlen).max(0.0));
        let off = self.wv_off;
        let at = |x: f32| inn.min.x + inn.width() * ((x - off) / vlen);
        let time_at = |px: f32| (off + (px - inn.min.x) / inn.width() * vlen).clamp(0.0, dur);
        let clip = p.with_clip_rect(inn);
        if !self.wave.is_empty() {
            let cols = (inn.width() / 1.5).max(1.0) as usize;
            let cw = inn.width() / cols as f32;
            let mid = inn.center().y;
            let n = self.wave.len();
            let peak = |c: usize| -> f32 {
                let t0 = off + vlen * c as f32 / cols as f32;
                let t1 = off + vlen * (c + 1) as f32 / cols as f32;
                let a = (((t0 / dur) * n as f32) as usize).min(n - 1);
                let b = (((t1 / dur) * n as f32) as usize).max(a + 1).min(n);
                self.wave[a..b].iter().copied().max().unwrap_or(0) as f32
            };
            let hs: Vec<f32> = (0..cols).map(peak).collect();
            let col = pal().ink2.gamma_multiply(0.6);
            for c in 0..cols {
                let l = hs[c.saturating_sub(1)];
                let r = hs[(c + 1).min(cols - 1)];
                let v = hs[c] * 0.6 + (l + r) * 0.2;
                let h = (v / 255.0) * (inn.height() * 0.5);
                let x = inn.min.x + c as f32 * cw;
                fill_rect(&clip, Rect::from_min_max(Pos2::new(x, mid - h - 0.5), Pos2::new(x + cw + 0.3, mid + h + 0.5)), col);
            }
        } else {
            ptext(&p, inn.center(), Align::Center, if self.loading { "Loading..." } else { "" }, 1.0, pal().dim);
        }
        // saved loops as faint bands along the bottom
        for s in self.sections.get(&t.id).into_iter().flatten() {
            fill_rect(&clip, Rect::from_min_max(Pos2::new(at(s.a), inn.max.y - 3.0), Pos2::new(at(s.b).max(at(s.a) + 1.0), inn.max.y)), pal().ink.gamma_multiply(0.6));
        }
        if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
            let col = if self.loop_on { loop_col().gamma_multiply(0.35) } else { pal().ink.gamma_multiply(0.2) };
            fill_rect(&clip, Rect::from_min_max(Pos2::new(at(a), inn.min.y), Pos2::new(at(b), inn.max.y)), col);
        }
        for (v, lab) in [(self.loop_a, "A"), (self.loop_b, "B")] {
            if let Some(x) = v {
                fill_rect(&clip, Rect::from_min_max(Pos2::new(at(x) - 1.5, bar.min.y + 2.0), Pos2::new(at(x) + 1.5, bar.max.y - 2.0)), loop_col());
                ptext(&clip, Pos2::new(at(x) + 4.0, inn.min.y + 5.0), Align::Min, lab, 1.0, loop_col());
            }
        }
        // bookmarks: a line in their colour and a numbered flag
        for (k, (bt, name)) in self.marks.get(&t.id).into_iter().flatten().enumerate() {
            let c = MARK_COLS[k % MARK_COLS.len()];
            let col = Color32::from_rgb(c[0], c[1], c[2]);
            let x = at(*bt);
            fill_rect(&clip, Rect::from_min_max(Pos2::new(x - 1.0, bar.min.y + 12.0), Pos2::new(x + 1.0, bar.max.y - 2.0)), col);
            let lab = if name.is_empty() { (k + 1).to_string() } else { fit(name, 1.0, 50.0) };
            let fw = text_w(&lab, 1.0) + 8.0;
            let flag = Rect::from_min_size(Pos2::new(x, bar.max.y - 22.0), Vec2::new(fw, 13.0));
            clip.rect_filled(flag, 1.0, col);
            ptext(&clip, flag.center(), Align::Center, &lab, 1.0, Color32::from_gray(12));
        }
        fill_rect(&clip, Rect::from_min_max(Pos2::new(at(pos) - 1.0, bar.min.y + 2.0), Pos2::new(at(pos) + 1.0, bar.max.y - 2.0)), pal().ink);
        let readout = format!("{} / {}", fmt_t(pos), fmt_t(t.duration));
        ptext(&p, Pos2::new(bar.max.x - 8.0, bar.min.y + 8.0), Align::Max, &readout, 1.0, pal().ink);
        // time marks along the top
        let step = [1.0f32, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0].into_iter().find(|s| vlen / s <= 6.0).unwrap_or(600.0);
        let limit = bar.max.x - 16.0 - text_w(&readout, 1.0);
        let mut x0 = (off / step).ceil() * step;
        while x0 < off + vlen {
            let x = at(x0);
            fill_rect(&p, Rect::from_min_max(Pos2::new(x, bar.min.y + 2.0), Pos2::new(x + 1.0, bar.min.y + 7.0)), pal().ink);
            if x + 30.0 < limit {
                let lab = format!("{}:{:02}", (x0 / 60.0) as u32, (x0 % 60.0) as u32);
                ptext(&p, Pos2::new(x + 3.0, bar.min.y + 7.0), Align::Min, &lab, 0.7, pal().ink2);
            }
            x0 += step;
        }
        // tap: jump there. Drag: by A or B moves it, inside the loop slides it, anywhere else makes a new loop
        if resp.drag_started() {
            if let Some(pp) = resp.interact_pointer_pos() {
                let t0 = time_at(pp.x);
                let (a, b) = (self.loop_a.unwrap_or(-1.0), self.loop_b.unwrap_or(-1.0));
                let kind = if self.loop_a.is_some() && (pp.x - at(a)).abs() < 18.0 {
                    1
                } else if self.loop_b.is_some() && (pp.x - at(b)).abs() < 18.0 {
                    2
                } else if a >= 0.0 && b > a && t0 > a && t0 < b {
                    3
                } else {
                    0
                };
                self.wv_drag = Some((kind, t0, a, b));
            }
        }
        if let (Some((kind, t0, a0, b0)), Some(pp)) = (self.wv_drag, resp.interact_pointer_pos()) {
            if resp.dragged() {
                let tn = time_at(pp.x);
                match kind {
                    0 => {
                        self.loop_a = Some(t0.min(tn));
                        self.loop_b = Some(t0.max(tn));
                    }
                    1 => self.loop_a = Some(tn),
                    2 => self.loop_b = Some(tn),
                    _ => {
                        let d = (tn - t0).clamp(-a0, dur - b0);
                        self.loop_a = Some(a0 + d);
                        self.loop_b = Some(b0 + d);
                    }
                }
            }
        }
        if resp.drag_stopped() {
            if let Some((kind, ..)) = self.wv_drag.take() {
                if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
                    if b < a {
                        (self.loop_a, self.loop_b) = (Some(b), Some(a));
                    }
                    if kind == 0 {
                        self.sec_cur = None;
                    }
                    self.loop_on = (b - a).abs() > 0.2;
                    self.practice = true;
                    self.sync_saved(t.id);
                    self.sync_player();
                    if kind == 0 && self.loop_on {
                        self.player.send(Cmd::Seek(self.loop_a.unwrap_or(0.0)));
                    }
                }
            }
        }
        if resp.clicked() {
            if let Some(pp) = resp.interact_pointer_pos() {
                self.player.send(Cmd::Seek(time_at(pp.x)));
            }
        }
        // zoom
        ui.horizontal(|ui| {
            if pill_w(ui, "-", 36.0, false).clicked() {
                self.wv_zoom = (self.wv_zoom / 2.0).max(1.0);
            }
            if pill_w(ui, "+", 36.0, false).clicked() {
                self.wv_zoom = (self.wv_zoom * 2.0).min(64.0);
                self.wv_off = pos - dur / self.wv_zoom * 0.3;
            }
            if pill(ui, "Fit", self.wv_zoom <= 1.0).clicked() {
                self.wv_zoom = 1.0;
                self.wv_off = 0.0;
            }
            dim(ui, "Drag on the wave to make a loop", 1.0, pal().dim);
        });
    }

    /// The saved loop playing follows changes to A and B (nudged, dragged, shifted).
    fn sync_saved(&mut self, id: i64) {
        if let (Some(k), Some(a), Some(b)) = (self.sec_cur, self.loop_a, self.loop_b) {
            if let Some(s) = self.sections.get_mut(&id).and_then(|l| l.get_mut(k)) {
                s.a = a;
                s.b = b;
                save_json("loops.json", &self.sections);
            }
        }
    }

    /// LOOP & SPEED, top to bottom in the order you use it: 1 mark the bit (start, loop, end), 2 slow it down,
    /// then play it, 3 keep it (the saved loops).
    fn loop_tab(&mut self, ui: &mut egui::Ui, t: &Track) {
        let pos = self.pos();
        let dur = t.duration;
        let step = |ui: &mut egui::Ui, text: &str| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
            ptext(ui.painter(), Pos2::new(r.min.x + 2.0, r.center().y), Align::Min, text, 1.1, pal().ink2);
        };
        // ---- 1 mark the bit
        ui.horizontal(|ui| {
            });
        let w = ui.available_width();
        let mid_w = 70.0;
        let box_w = (w - mid_w - 16.0) / 2.0;
        let (row, _) = ui.allocate_exact_size(Vec2::new(w, 124.0), Sense::hover());
        for which in 0..2 {
            let x = if which == 0 { row.min.x } else { row.max.x - box_w };
            let r = Rect::from_min_size(Pos2::new(x, row.min.y), Vec2::new(box_w, 124.0));
            let v = if which == 0 { self.loop_a } else { self.loop_b };
            let p = ui.painter().clone();
            p.rect_filled(r, 10.0, pal().beige_lt);
            ptext(&p, Pos2::new(r.center().x, r.min.y + 14.0), Align::Center, if which == 0 { "Start (A)" } else { "End (B)" }, 1.1, pal().ink2);
            ptext(&p, Pos2::new(r.center().x, r.min.y + 38.0), Align::Center, &v.map(fmt_t).unwrap_or_else(|| "-:--".to_string()), 2.0, pal().ink);
            // SET A / SET B: here, where the song is
            let setr = Rect::from_center_size(Pos2::new(r.center().x, r.min.y + 66.0), Vec2::new((box_w - 20.0).min(110.0), 28.0));
            p.rect_filled(setr, 14.0, pal().ink);
            ptext(&p, setr.center(), Align::Center, if which == 0 { "Set A" } else { "Set B" }, 1.3, pal().beige);
            // the top two thirds sets it where the song is; - and + along the foot nudge it a tenth
            let set = ui.interact(setr, ui.id().with(("ab_set", which)), Sense::click());
            if set.clicked() {
                // setting either end loops it straight away (an end with no start loops from the top)
                if which == 0 {
                    self.loop_a = Some(pos);
                } else {
                    self.loop_b = Some(pos);
                    if self.loop_a.is_none_or(|a| a >= pos) {
                        self.loop_a = Some(0.0);
                    }
                }
                self.loop_on = self.loop_a.zip(self.loop_b).is_some_and(|(a, b)| b > a);
                self.practice = true;
                self.sec_cur = None;
                self.sync_player();
            }
            for (k, d) in [(-1i32, "-"), (1, "+")] {
                let c = Pos2::new(r.center().x + k as f32 * 22.0, r.min.y + 102.0);
                let nr = Rect::from_center_size(c, Vec2::new(36.0, 26.0));
                let resp = ui.interact(nr, ui.id().with(("ab_nudge", which, k)), Sense::click());
                p.rect_filled(nr, 13.0, if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige });
                ptext(&p, c, Align::Center, d, 1.6, pal().ink);
                if resp.clicked() {
                    let nv = (v.unwrap_or(pos) + k as f32 * 0.1).clamp(0.0, dur.max(0.1));
                    if which == 0 {
                        self.loop_a = Some(nv);
                    } else {
                        self.loop_b = Some(nv);
                    }
                    self.sync_saved(t.id);
                    self.sync_player();
                }
            }
        }
        // the loop switch between them (hold it to clear the loop)
        let lr = Rect::from_center_size(row.center(), Vec2::new(mid_w, 124.0));
        let both = self.loop_a.is_some() && self.loop_b.is_some();
        let on = self.loop_on && both;
        let resp = ui.interact(lr, ui.id().with("loop_tog"), Sense::click());
        let p = ui.painter();
        p.rect_filled(lr, 10.0, if on { pal().row_sel } else { pal().beige });
        let col = if on { pal().bar_txt } else { pal().ink };
        loop_glyph(p, Pos2::new(lr.center().x, lr.min.y + 50.0), col);
        ptext(p, Pos2::new(lr.center().x, lr.min.y + 84.0), Align::Center, if on { "Loop on" } else { "Loop off" }, 1.1, col);
        if resp.clicked() && both {
            self.loop_on = !self.loop_on;
            self.sync_player();
        }
        if both {
            ui.horizontal_wrapped(|ui| {
                // the next (or last) stretch of the same length: bar 5-8 after bar 1-4
                for (name, d) in [("< Shift back", -1.0f32), ("Shift on >", 1.0)] {
                    if pill(ui, name, false).clicked() {
                        if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
                            let len = b - a;
                            let na = (a + d * len).clamp(0.0, (dur - len).max(0.0));
                            self.loop_a = Some(na);
                            self.loop_b = Some(na + len);
                            self.sec_cur = None;
                            self.sync_player();
                            self.player.send(Cmd::Seek(na));
                        }
                    }
                }
                if pill(ui, "Clear", false).clicked() {
                    self.loop_a = None;
                    self.loop_b = None;
                    self.loop_on = false;
                    self.sec_cur = None;
                    self.sync_player();
                }
            });
        }
        ui.add_space(6.0);
        // ---- 2 slow it down
        // the speed: one row of chips, and a step either way for the ones between
        let (sr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), Sense::hover());
        let names = ["-", "50%", "60%", "75%", "90%", "100%", "+"];
        let cw = sr.width() / names.len() as f32;
        for (k, n) in names.iter().enumerate() {
            let c = Rect::from_min_size(Pos2::new(sr.min.x + k as f32 * cw, sr.min.y + 3.0), Vec2::new(cw - 4.0, 32.0));
            let resp = ui.interact(c, ui.id().with(("spd", k)), Sense::click());
            let val = [0u32, 50, 60, 75, 90, 100, 0][k];
            let on = val == self.speed;
            ui.painter().rect_filled(c, 16.0, if on { pal().ink } else if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige_lt });
            let lab = if k == 0 || k == 6 { n.to_string() } else { n.to_string() };
            ptext_fit(ui.painter(), c.center(), Align::Center, &lab, 1.2, cw - 8.0, if on { pal().beige } else { pal().ink });
            if resp.clicked() {
                self.speed = match k {
                    0 => self.speed.saturating_sub(5).max(25),
                    6 => (self.speed + 5).min(250),
                    _ => val,
                };
                self.sync_player();
            }
        }
        if ![50, 60, 75, 90, 100].contains(&self.speed) {
            dim(ui, &format!("Speed {}%", self.speed), 1.0, pal().ink2);
        }
        // play it: back 2 seconds, play / pause, back to the loop's start
        let (tr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 64.0), Sense::hover());
        let mid = tr.center();
        let p = ui.painter().clone();
        p.circle_filled(mid, 28.0, pal().red);
        play_glyph(&p, mid, !(self.paused || self.stopped), pal().bar_txt);
        if ui.interact(Rect::from_center_size(mid, Vec2::splat(56.0)), ui.id().with("pr_play"), Sense::click()).clicked() {
            self.play_pause();
        }
        for (dx, k) in [(-76.0, 0), (76.0, 1)] {
            let c = mid + Vec2::new(dx, 0.0);
            ptext(&p, c + Vec2::new(0.0, 0.0), Align::Center, if k == 0 { "-2s" } else { "|<" }, 1.6, pal().ink);
            ptext(&p, c + Vec2::new(0.0, 20.0), Align::Center, if k == 0 { "back" } else { "restart" }, 1.0, pal().dim);
            if ui.interact(Rect::from_center_size(c, Vec2::splat(52.0)), ui.id().with(("pr_t", k)), Sense::click()).clicked() {
                self.player.send(Cmd::Seek(if k == 0 { (pos - 2.0).max(0.0) } else { self.loop_a.unwrap_or(0.0) }));
            }
        }
        ui.add_space(6.0);
        // ---- 3 keep it
        ui.add_space(4.0);
        // saved loops of this song
        ui.horizontal(|ui| {
            // the save button's own width (its icon and text), the name box the rest
            let bw = text_w("Save loop", 1.2) + 26.0 + 8.0;
            let w = ui.available_width() - bw;
            let r = text_box(ui, &mut self.sec_name, w.max(80.0), "Name this loop (bridge, head, lick...)");
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if pill(ui, "Save loop", false).clicked() || enter {
                match (self.loop_a, self.loop_b) {
                    (Some(a), Some(b)) if b > a => {
                        let list = self.sections.entry(t.id).or_default();
                        let name = if self.sec_name.trim().is_empty() { format!("LOOP {}", list.len() + 1) } else { self.sec_name.trim().to_string() };
                        list.push(Section { name, a, b, ..Default::default() });
                        save_json("loops.json", &self.sections);
                        self.sec_name.clear();
                        self.set_note("Loop saved");
                    }
                    _ => self.set_note("Set loop A and B first"),
                }
            }
        });
        let secs = self.sections.get(&t.id).cloned().unwrap_or_default();
        if secs.is_empty() {
            dim(ui, "Saved loops of this song show up here", 1.0, pal().dim);
        }
        let mut groups: Vec<String> = Vec::new();
        for s in &secs {
            if !groups.contains(&s.group) {
                groups.push(s.group.clone());
            }
        }
        groups.sort_by_key(|g| (!g.is_empty(), g.to_lowercase()));
        let mut drop = None;
        let mut changed = false;
        for g in &groups {
            if !g.is_empty() {
                dim(ui, g, 1.0, pal().ink2);
            }
            for (k, s) in secs.iter().enumerate().filter(|(_, s)| &s.group == g) {
                ui.horizontal(|ui| {
                    let on = self.loop_on && self.loop_a == Some(s.a) && self.loop_b == Some(s.b);
                    let w = ui.available_width() - 100.0;
                    if pill_w(ui, &format!("{}   {} - {}", s.name, fmt_t(s.a), fmt_t(s.b)), w, on).clicked() {
                        self.loop_a = Some(s.a);
                        self.loop_b = Some(s.b);
                        self.loop_on = true;
                        self.sec_cur = Some(k);
                        self.practice = true;
                        self.sync_player();
                        self.player.send(Cmd::Seek(s.a));
                    }
                    if pill_w(ui, "Edit", 50.0, self.sec_edit == Some(k)).clicked() {
                        self.sec_edit = if self.sec_edit == Some(k) { None } else { Some(k) };
                    }
                    let asked = self.sec_del == Some(k);
                    if pill_w(ui, if asked { "Sure?" } else { "X" }, 42.0, asked).clicked() {
                        if asked {
                            drop = Some(k);
                            self.sec_del = None;
                        } else {
                            self.sec_del = Some(k);
                        }
                    }
                });
                if !s.note.is_empty() && self.sec_edit != Some(k) {
                    dim(ui, &s.note, 1.0, pal().dim);
                }
                if self.sec_edit == Some(k) {
                    let w = ui.available_width();
                    if let Some(e) = self.sections.get_mut(&t.id).and_then(|l| l.get_mut(k)) {
                        changed |= text_box(ui, &mut e.name, w, "Name").changed();
                        changed |= text_box(ui, &mut e.note, w, "A note: what it is, what to work on").changed();
                        changed |= text_box(ui, &mut e.group, w, "Group (solo licks, head...)").changed();
                    }
                }
            }
        }
        if changed {
            save_json("loops.json", &self.sections);
        }
        if let Some(k) = drop {
            if let Some(l) = self.sections.get_mut(&t.id) {
                l.remove(k);
            }
            self.sec_cur = None;
            self.sec_edit = None;
            save_json("loops.json", &self.sections);
        }
    }

    /// TRAINER, as settings rows (tap one to change it): the count-off before each pass and the song's tempo for it
    /// (tap it in), then the speed trainer - on, every so many loops, by how much, then faster or up a 4th.
    fn trainer_tab(&mut self, ui: &mut egui::Ui) {
        let co = if self.count_in == 0 { "Off".to_string() } else { format!("{} beats", self.count_in) };
        if srow(ui, "Count-off", &co) {
            self.count_in = match self.count_in {
                0 => 2,
                2 => 4,
                _ => 0,
            };
            self.sync_player();
        }
        if srow(ui, "Song tempo (tap here to tap it in)", &format!("{:.0} bpm", self.song_bpm)) {
            if let Some(b) = self.tap() {
                self.song_bpm = b;
                self.sync_player();
            }
        }
        ui.add_space(8.0);
        let st = if self.trainer { format!("On  ·  pass {}/{}", self.passes, self.trainer_n) } else { "Off".to_string() };
        if srow(ui, "Speed trainer", &st) {
            self.trainer = !self.trainer;
            self.passes = 0;
        }
        if srow(ui, "Every", &format!("{} loops", self.trainer_n)) {
            const N: [u32; 6] = [1, 2, 3, 4, 6, 8];
            self.trainer_n = N[(N.iter().position(|x| *x == self.trainer_n).unwrap_or(0) + 1) % N.len()];
        }
        if srow(ui, "By", &format!("+{}%", self.trainer_step)) {
            const S: [u32; 4] = [2, 5, 10, 15];
            self.trainer_step = S[(S.iter().position(|x| *x == self.trainer_step).unwrap_or(0) + 1) % S.len()];
        }
        if srow(ui, "Then", if self.trainer_keys { "Up a 4th, 12 keys" } else { "Faster" }) {
            self.trainer_keys = !self.trainer_keys;
            self.keys_done = 0;
            self.passes = 0;
        }
        dim(
            ui,
            if self.trainer_keys { "Every few passes the loop goes up a 4th, through all 12 keys" } else { "Starts at the speed set on Loop and stops at 100%" },
            1.0,
            pal().dim,
        );
    }

    /// PITCH, as settings rows: the pitch (a step either way, or a few cents), the ear mode, the recording's tuning,
    /// saving it as a WAV, then the equalizer.
    fn ear_tab(&mut self, ui: &mut egui::Ui) {
        let pv = format!("{:+.2} semitones", self.cents as f32 / 100.0);
        srow(ui, "Pitch", &pv);
        ui.horizontal_wrapped(|ui| {
            for (s, d) in [("-1", -100), ("-10c", -10), ("+10c", 10), ("+1", 100), ("Reset", 0)] {
                if pill(ui, s, false).clicked() {
                    self.cents = if d == 0 { 0 } else { (self.cents + d).clamp(-1200, 1200) };
                    self.practice = true;
                    self.sync_player();
                }
            }
        });
        if srow(ui, "Ear", CHAN_NAMES[self.chan as usize % 6]) {
            self.chan = (self.chan + 1) % 6;
            self.sync_player();
        }
        let id = self.cur_track().map(|t| t.id).unwrap_or(0);
        match self.tuning.get(&id).copied() {
            Some(c) if c.abs() >= 5 => {
                if srow(ui, &format!("Tuning: {:+} cents from A440", c), if self.cents == -c { "Tuned" } else { "Tune to A440" }) {
                    self.cents = -c;
                    self.practice = true;
                    self.sync_player();
                }
            }
            Some(_) => {
                srow(ui, "Tuning", "At A440");
            }
            None => {
                if srow(ui, "Tuning", if self.tuning_busy { "Listening..." } else { "Find it" }) && !self.tuning_busy {
                    self.find_tuning();
                }
            }
        }
        let what = if self.loop_a.zip(self.loop_b).is_some_and(|(a, b)| b > a) { "the loop" } else { "the song" };
        if srow(ui, &format!("Save {} as a WAV", what), if self.exporting { "Saving..." } else { "Save" }) && !self.exporting {
            self.export_wav();
        }
        ui.add_space(8.0);
        section_header(ui, "Equalizer");
        ui.horizontal_wrapped(|ui| {
            if check_box(ui, "On", self.eq_on).clicked() {
                self.eq_on = !self.eq_on;
                self.player.eq.set(self.eq, 0.0, self.eq_on);
                self.save_settings();
            }
            for (name, g) in EQ_PRESETS {
                if pill(ui, name, self.eq == g).clicked() {
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

    /// The song's bytes, from the phone (a file, or the copy kept when it played).
    pub(crate) fn song_bytes(&self, id: i64) -> Option<Vec<u8>> {
        match self.files.iter().find(|f| f.0.id == id) {
            Some(f) => std::fs::read(&f.1).ok(),
            None => cache::get(id),
        }
    }

    /// Listen for how far the recording is from A440 (in the background).
    fn find_tuning(&mut self) {
        let Some(id) = self.cur_track().map(|t| t.id) else { return };
        let Some(bytes) = self.song_bytes(id) else {
            self.set_note("Play it once first so it's kept, then check the tuning");
            return;
        };
        self.tuning_busy = true;
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Tuning(id, tuning::detect(bytes)));
            ctx.request_repaint();
        });
    }

    /// Save the loop (or the whole song) as a WAV the way it sounds now, into the phone's Music folder.
    fn export_wav(&mut self) {
        let Some(t) = self.cur_track().cloned() else { return };
        let Some(bytes) = self.song_bytes(t.id) else {
            self.set_note("Play it once first so it's kept, then save it");
            return;
        };
        let (a, b) = match (self.loop_a, self.loop_b) {
            (Some(a), Some(b)) if b > a => (a, b),
            _ => (0.0, 0.0),
        };
        let fx = player::Fx {
            speed: self.speed,
            cents: if self.practice { self.cents } else { 0 },
            chan: if self.practice { self.chan } else { 0 },
            balance: self.balance,
            eq: (self.eq, 0.0, self.eq_on),
        };
        let clean = |s: &str| s.chars().filter(|c| c.is_alphanumeric() || " -_".contains(*c)).collect::<String>();
        let name = format!("{} - {}{}.wav", clean(&t.artists_text()), clean(&t.title), if b > a { format!(" {}-{}", a as u32, b as u32) } else { String::new() });
        let path = music_dir().join("Saved loops").join(name);
        self.exporting = true;
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let r = (|| -> Result<String, String> {
                let mut src = decode::SymSource::new(bytes)?;
                let (ch, rate) = (rodio::Source::channels(&src), rodio::Source::sample_rate(&src));
                if a > 0.0 {
                    rodio::Source::try_seek(&mut src, Duration::from_secs_f32(a)).map_err(|_| "can't seek in this file".to_string())?;
                }
                let data: Vec<i16> = if b > a { src.take(((b - a) * rate as f32) as usize * ch as usize).collect() } else { src.collect() };
                let (out, ch, rate) = player::render(rodio::buffer::SamplesBuffer::new(ch, rate, data), &fx);
                if let Some(d) = path.parent() {
                    std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
                }
                let len = (out.len() * 2) as u32;
                let mut w: Vec<u8> = Vec::with_capacity(44 + len as usize);
                w.extend_from_slice(b"RIFF");
                w.extend_from_slice(&(36 + len).to_le_bytes());
                w.extend_from_slice(b"WAVEfmt ");
                w.extend_from_slice(&16u32.to_le_bytes());
                w.extend_from_slice(&1u16.to_le_bytes());
                w.extend_from_slice(&ch.to_le_bytes());
                w.extend_from_slice(&rate.to_le_bytes());
                w.extend_from_slice(&(rate * ch as u32 * 2).to_le_bytes());
                w.extend_from_slice(&(ch * 2).to_le_bytes());
                w.extend_from_slice(&16u16.to_le_bytes());
                w.extend_from_slice(b"data");
                w.extend_from_slice(&len.to_le_bytes());
                for s in out {
                    w.extend_from_slice(&s.to_le_bytes());
                }
                std::fs::write(&path, w).map_err(|e| e.to_string())?;
                Ok(path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
            })();
            let _ = tx.send(Msg::Exported(r));
            ctx.request_repaint();
        });
    }

    /// BOOKMARKS: this song's fixed points - add one where the song is, jump to one, name it, take it away.
    fn marks_tab(&mut self, ui: &mut egui::Ui, t: &Track) {
        let now = self.pos();
        ui.horizontal_wrapped(|ui| {
            if pill(ui, &format!("+ BOOKMARK AT {}", fmt_t(now)), false).clicked() {
                let l = self.marks.entry(t.id).or_default();
                l.push((now, String::new()));
                l.sort_by(|a, b| a.0.total_cmp(&b.0));
                save_json("bookmarks.json", &self.marks);
            }
            if self.marks.get(&t.id).is_some_and(|l| !l.is_empty()) && pill(ui, "Remove all", false).clicked() {
                self.marks.remove(&t.id);
                save_json("bookmarks.json", &self.marks);
            }
        });
        let n = self.marks.get(&t.id).map_or(0, |l| l.len());
        if n == 0 {
            note_text(ui, "No bookmarks on this song yet. They are fixed points to find your way: the solo, the key change, the bar you keep missing.", pal().ink2);
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
                if pill_w(ui, &fmt_t(at), 70.0, false).clicked() {
                    self.player.send(Cmd::Seek(at));
                }
                let w = ui.available_width() - 48.0;
                if let Some(l) = self.marks.get_mut(&t.id) {
                    if text_box(ui, &mut l[k].1, w.max(60.0), "Name it (solo, bridge...)").changed() {
                        changed = true;
                    }
                }
                if pill_w(ui, "X", 40.0, false).clicked() {
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

/// A settings row: what it is at the left, its value (and that it can be tapped) at the right. True when tapped.
pub(crate) fn srow(ui: &mut egui::Ui, left: &str, right: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 42.0), Sense::click());
    ui.painter().rect_filled(r.shrink2(Vec2::new(0.0, 1.0)), 6.0, if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige_lt });
    let rw = text_w(right, 1.2) + 26.0;
    ptext_fit(ui.painter(), Pos2::new(r.min.x + 12.0, r.center().y), Align::Min, left, 1.3, r.width() - rw - 16.0, pal().ink);
    ptext(ui.painter(), Pos2::new(r.max.x - 22.0, r.center().y), Align::Max, right, 1.2, pal().ink2);
    ptext(ui.painter(), Pos2::new(r.max.x - 10.0, r.center().y), Align::Max, ">", 1.2, pal().dim);
    resp.clicked()
}
