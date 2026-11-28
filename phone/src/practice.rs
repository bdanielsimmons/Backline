//! PRACTICE: the desktop's practice panel on a phone. The waveform (tap to jump; drag to make a loop, drag A or B
//! to move it, drag inside the loop to slide it), then its tabs: LOOP (A and B, the loop's keys, the speed, saved
//! loops), TRAINER (count-off, tempo, speed trainer), PITCH (semitones and cents, the ear mode) and BOOKMARKS.

use crate::*;

const TABS: [&str; 5] = ["LOOP", "TRAINER", "PITCH", "BOOKMARKS", "RECORD"];

impl App {
    pub(crate) fn practice_ui(&mut self, ui: &mut egui::Ui) {
        if self.cur.is_none() {
            ui.add_space(8.0);
            dim_centered(ui, "Pick a song on the Songs tab first");
            ui.add_space(8.0);
            centered_row(ui, "to_songs", |ui| {
                if retro_btn_w(ui, "SONGS", 90.0, false).clicked() {
                    self.view = 0;
                }
            });
            return;
        }
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
        // (the waveform takes the room the tools leave: the Loop tab's rows want about 330 points)
        let wave_h = (ui.available_height() - 330.0).clamp(132.0, 240.0);
        self.waveform(ui, wave_h);
        ui.add_space(8.0);
        // the metronome (opened from the deck's METRO key) in place of the practice tools
        if self.mt_open {
            self.metro_ui(ui);
            return;
        }
        // (the tabs a little smaller than the rest, to keep to one row)
        let mtab = self.mtab as usize;
        let picked = font::with_scale(UI_SCALE.min(0.82), || {
            let tw = TABS.iter().map(|n| text_w(n, 2.0) + 22.0).sum::<f32>() + 8.0;
            let room = ui.available_width();
            if tw < room {
                ui.horizontal(|ui| {
                    ui.add_space(((room - tw) / 2.0).floor());
                    ui.allocate_ui(Vec2::new(tw, 40.0), |ui| tab_row(ui, &TABS, mtab)).inner
                })
                .inner
            } else {
                tab_row(ui, &TABS, mtab)
            }
        });
        if let Some(i) = picked {
            self.mtab = i as u8;
            self.save_settings();
        }
        ui.add_space(4.0);
        egui::ScrollArea::vertical().id_salt("prac_tools").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            match self.mtab {
                0 => self.loop_tab(ui),
                1 => self.trainer_tab(ui),
                2 => self.pitch_tab(ui),
                3 => self.marks_tab(ui),
                _ => self.record_tab(ui),
            }
            ui.add_space(16.0);
        });
    }

    /// The waveform, `h` tall, drawn as the desktop's practice panel draws it.
    fn waveform(&mut self, ui: &mut egui::Ui, h: f32) {
        let (bar, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::click_and_drag());
        let p = ui.painter().clone();
        inset(&p, bar, pal().display);
        let inn = bar.shrink2(Vec2::new(5.0, 5.0));
        let top = bar.min.y;
        let dur = self.dur.max(0.001);
        let shown = self.pos();
        let active = !self.wave.is_empty();
        let (col_heard, col_ahead, ink) = (pal().ink2.gamma_multiply(0.85), pal().ink2.gamma_multiply(0.55), pal().ink);
        let label_ink = Color32::from_gray(235);
        // ---- two fingers: pinch to zoom in where they are, slide to look along the song (a loop being dragged by the
        // first finger is put back as it was)
        let now = ui.input(|i| i.time);
        if let Some(mt) = ui.input(|i| i.multi_touch()).filter(|m| m.num_touches >= 2 && bar.contains(m.start_pos)) {
            if let Some((_, _, a0, b0)) = self.wv_drag.take() {
                self.loop_a = (a0 >= 0.0).then_some(a0);
                self.loop_b = (b0 >= 0.0).then_some(b0);
            }
            let z0 = self.wv_zoom.clamp(1.0, 64.0);
            let span0 = dur / z0;
            let frac = ((mt.start_pos.x - inn.min.x) / inn.width()).clamp(0.0, 1.0);
            let under = self.wv_start + frac * span0;
            let z1 = (z0 * mt.zoom_delta).clamp(1.0, 64.0);
            let span1 = dur / z1;
            let slide = ui.input(|i| i.multi_touch().map_or(0.0, |m| m.translation_delta.x));
            self.wv_zoom = z1;
            self.wv_start = under - frac * span1 - slide / inn.width() * span1;
            self.pinch_at = now;
            ui.ctx().request_repaint();
        }
        let pinching = now - self.pinch_at < 0.35;
        // ---- the part in view: zoomed, it follows the song (the loop kept in view while it loops)
        let zoom = self.wv_zoom.clamp(1.0, 64.0);
        let span = dur / zoom;
        let mut start = self.wv_start;
        if zoom > 1.0 && self.wv_drag.is_none() && !pinching {
            match (self.loop_a, self.loop_b) {
                (Some(a), Some(b)) if self.loop_on && b - a <= span * 0.95 => {
                    if a < start || b > start + span {
                        start = a - (span - (b - a)) / 2.0;
                    }
                }
                _ if shown < start || shown > start + span => start = shown - span * 0.05,
                _ => {}
            }
        }
        let start = start.clamp(0.0, (dur - span).max(0.0));
        self.wv_start = start;
        let at = |t: f32| inn.min.x + inn.width() * ((t - start) / span).clamp(0.0, 1.0);
        let time_at = |x: f32| start + ((x - inn.min.x) / inn.width()).clamp(0.0, 1.0) * span;
        if !active {
            if let Some(e) = self.load_error.clone() {
                // (why it didn't come, and a key to try again)
                let lines = wrap(&format!("Couldn't get the song: {}", e), 1.3, inn.width() - 20.0);
                let mut y = inn.center().y - 14.0 - lines.len() as f32 * 9.0;
                for l in &lines {
                    ptext(&p, Pos2::new(inn.center().x, y), Align::Center, l, 1.3, pal().ink);
                    y += 18.0;
                }
                let kr = Rect::from_center_size(Pos2::new(inn.center().x, y + 18.0), Vec2::new(110.0, bh()));
                if chip(ui, kr, "TRY AGAIN", false, "load_again").clicked() {
                    if let Some(s) = self.cur.clone() {
                        self.cur = None;
                        self.load(s, true);
                    }
                }
                return;
            }
            let pct = self.dl_pct.load(Ordering::Relaxed);
            let what = if !self.loading {
                String::new()
            } else if pct > 0 {
                format!("Getting the song... {}%", pct)
            } else {
                "Getting the song...".to_string()
            };
            ptext(&p, inn.center(), Align::Center, &what, 1.3, pal().ink2);
            if self.loading {
                // (how far, as a bar of blocks under the words)
                let br = Rect::from_center_size(inn.center() + Vec2::new(0.0, 22.0), Vec2::new((inn.width() * 0.6).min(240.0), 10.0));
                studio::blocks(&p, br, pct as f32 / 100.0);
            }
            if self.loading {
                ui.ctx().request_repaint_after(Duration::from_millis(100));
            }
            return;
        }
        // ---- the song's shape: the part heard brighter
        let pitch = 2.0;
        let cols = (inn.width() / pitch).max(1.0) as usize;
        let (mid, half) = (inn.center().y, inn.height() * 0.5);
        let n = self.wave.len();
        for c in 0..cols {
            let (t0, t1) = (start + span * c as f32 / cols as f32, start + span * (c + 1) as f32 / cols as f32);
            let a = ((t0 / dur * n as f32) as usize).min(n - 1);
            let b = ((t1 / dur * n as f32) as usize).max(a + 1).min(n);
            let m = self.wave[a..b].iter().copied().max().unwrap_or(0) as f32 / 255.0;
            let x = inn.min.x + c as f32 * pitch;
            let col = if x < at(shown) { col_heard } else { col_ahead };
            fill_rect(&p, Rect::from_min_max(Pos2::new(x, (mid - m * half).min(mid - 0.5)), Pos2::new(x + 1.5, (mid + m * half).max(mid + 0.5))), col);
        }
        let roomy = inn.height() >= 120.0;
        let glass = pal().display;
        let lc = crate::skin::loop_col();
        let lc_fg = Color32::from_gray(14);
        // ---- the loop in the spotlight: everything outside it dimmed (more while it loops), a faint wash inside
        if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
            let k = if self.loop_on { 0.62 } else { 0.36 };
            let (xa, xb) = (at(a), at(b));
            if xa > inn.min.x {
                fill_rect(&p, Rect::from_min_max(Pos2::new(inn.min.x, inn.min.y), Pos2::new(xa, inn.max.y)), glass.gamma_multiply(k));
            }
            if xb < inn.max.x {
                fill_rect(&p, Rect::from_min_max(Pos2::new(xb, inn.min.y), Pos2::new(inn.max.x, inn.max.y)), glass.gamma_multiply(k));
            }
            if self.loop_on {
                fill_rect(&p, Rect::from_min_max(Pos2::new(xa, inn.min.y), Pos2::new(xb, inn.max.y)), lc.gamma_multiply(0.08));
            }
        }
        for t in [self.loop_a, self.loop_b].into_iter().flatten() {
            if t >= start && t <= start + span {
                fill_rect(&p, Rect::from_min_max(Pos2::new(at(t) - 1.0, top + 2.0), Pos2::new(at(t) + 1.0, bar.max.y - 2.0)), lc);
            }
        }
        // ---- saved loops: named bands along the foot (tap one to loop it); bookmarks: a line and a numbered flag
        let k = self.kept_now();
        let foot = bar.max.y - 27.0;
        let mut bands: Vec<(Rect, usize)> = Vec::new();
        let mut ends: Vec<f32> = Vec::new();
        let mut order: Vec<usize> = (0..k.loops.len()).collect();
        order.sort_by(|x, y| k.loops[*x].a.total_cmp(&k.loops[*y].a));
        let mut lanes_used = 0usize;
        for i in order {
            let sc = &k.loops[i];
            let lane = match ends.iter().position(|e| *e <= sc.a + 0.01) {
                Some(l) => l,
                None if ends.len() < 2 => {
                    ends.push(0.0);
                    ends.len() - 1
                }
                None => ends.iter().enumerate().min_by(|x, y| x.1.total_cmp(y.1)).map_or(0, |x| x.0),
            };
            ends[lane] = ends[lane].max(sc.b);
            if sc.b < start || sc.a > start + span {
                continue;
            }
            let (x0, x1) = (at(sc.a), at(sc.b).max(at(sc.a) + 2.0));
            if !roomy {
                fill_rect(&p, Rect::from_min_max(Pos2::new(x0, inn.max.y - 3.0), Pos2::new(x1, inn.max.y)), ink.gamma_multiply(0.6));
                continue;
            }
            lanes_used = lanes_used.max(lane + 1);
            let y1 = foot - lane as f32 * 14.0;
            let r = Rect::from_min_max(Pos2::new(x0, y1 - 12.0), Pos2::new(x1, y1));
            fill_rect(&p, r, glass.gamma_multiply(0.7));
            fill_rect(&p, r, ink.gamma_multiply(0.2));
            fill_rect(&p, Rect::from_min_max(r.min, Pos2::new(r.min.x + 2.0, r.max.y)), ink.gamma_multiply(0.85));
            let name = if sc.name.is_empty() { "Loop".to_string() } else { sc.name.clone() };
            if r.width() > 26.0 {
                let lab = font::fit(&name, 0.8, r.width() - 9.0);
                ptext(&p.with_clip_rect(r), Pos2::new(r.min.x + 5.0, r.center().y), Align::Min, &lab, 0.8, ink);
            }
            bands.push((r, i));
        }
        for (n, m) in k.marks.iter().enumerate() {
            if m.t < start || m.t > start + span {
                continue;
            }
            let x = at(m.t);
            fill_rect(&p, Rect::from_min_max(Pos2::new(x - 0.5, top + 16.0), Pos2::new(x + 0.5, bar.max.y - 2.0)), ink.gamma_multiply(0.7));
            let lab = (n + 1).to_string();
            let fr = Rect::from_min_size(Pos2::new(x, top + 16.0), Vec2::new(text_w(&lab, 0.8) + 8.0, 12.0));
            p.rect_filled(fr, 1.0, ink);
            ptext(&p, fr.center(), Align::Center, &lab, 0.8, glass);
        }
        // ---- where you are, with a light edge so it shows over the song
        fill_rect(&p, Rect::from_min_max(Pos2::new(at(shown) - 2.5, top + 2.0), Pos2::new(at(shown) + 2.5, bar.max.y - 2.0)), glass.gamma_multiply(0.85));
        fill_rect(&p, Rect::from_min_max(Pos2::new(at(shown) - 1.0, top + 2.0), Pos2::new(at(shown) + 1.0, bar.max.y - 2.0)), ink);
        let readout_txt = format!("{} / {}", fmt_t(shown), fmt_time(self.dur));
        ptext(&p, Pos2::new(bar.max.x - 8.0, top + 8.0), Align::Max, &readout_txt, 1.0, pal().ink);
        // ---- time marks along the top
        let step = [0.1f32, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0].into_iter().find(|s| span / s <= 6.0).unwrap_or(600.0);
        let mut t = (start / step).ceil() * step;
        let readout = bar.max.x - 8.0 - text_w(&readout_txt, 1.0) - 40.0;
        while t <= start + span + 1e-3 {
            let x = at(t);
            fill_rect(&p, Rect::from_min_max(Pos2::new(x, top + 2.0), Pos2::new(x + 1.0, top + 7.0)), pal().ink);
            if x < readout {
                let (m, s) = ((t / 60.0).floor() as u32, t % 60.0);
                let lab = if step < 1.0 { format!("{}:{:04.1}", m, s) } else { format!("{}:{:02}", m, s.round() as u32) };
                let lw = text_w(&lab, 0.7);
                p.rect_filled(Rect::from_min_size(Pos2::new(x + 1.5, top + 2.0), Vec2::new(lw + 4.0, 10.0)), 1.0, Color32::from_black_alpha(190));
                ptext(&p, Pos2::new(x + 3.0, top + 7.0), Align::Min, &lab, 0.7, label_ink);
            }
            t += step;
        }
        // ---- A and B named on their lines; the loop's length between them
        // (inside the loop, each on its own line's inner side; a loop too short for both: outside it instead)
        let lab_w = |lab: &str, t: f32| text_w(&format!("{} {}", lab, fmt_t(t)), 0.8) + 10.0;
        let outward = match (self.loop_a, self.loop_b) {
            (Some(a), Some(b)) => at(b) - at(a) < lab_w("A", a) + lab_w("B", b) + 4.0,
            _ => false,
        };
        // (still in each other's way, at an end of the song: B a row lower)
        let mut a_rect: Option<Rect> = None;
        for (v, lab) in [(self.loop_a, "A"), (self.loop_b, "B")] {
            if let Some(t) = v.filter(|t| *t >= start && *t <= start + span) {
                let txt = format!("{} {}", lab, fmt_t(t));
                let w = text_w(&txt, 0.8) + 10.0;
                let x = if (lab == "A") != outward { at(t) - 1.0 } else { at(t) - w + 1.0 };
                let x = x.clamp(bar.min.x + 2.0, bar.max.x - w - 2.0);
                let mut r = Rect::from_min_size(Pos2::new(x, top + 2.0), Vec2::new(w, 13.0));
                if lab == "A" {
                    a_rect = Some(r);
                } else if a_rect.is_some_and(|a| a.expand(1.0).intersects(r)) {
                    r = r.translate(Vec2::new(0.0, 15.0));
                }
                p.rect_filled(r, 1.0, lc);
                ptext(&p, r.center(), Align::Center, &txt, 0.8, lc_fg);
            }
        }
        if let (Some(a), Some(b), true) = (self.loop_a, self.loop_b, roomy) {
            let (xa, xb) = (at(a), at(b));
            let txt = format!("{:.1} s", b - a);
            let w = text_w(&txt, 0.9) + 12.0;
            if xb - xa > w + 8.0 {
                let y = foot - lanes_used as f32 * 14.0 - 10.0;
                let r = Rect::from_center_size(Pos2::new((xa + xb) / 2.0, y), Vec2::new(w, 15.0));
                p.rect_filled(r, 2.0, glass.gamma_multiply(0.85));
                ptext(&p, r.center(), Align::Center, &txt, 0.9, ink);
            }
        }
        // ---- the zoom keys along its foot
        let chip_w = |t: &str| (text_w(t, 1.0) + 16.0).max(26.0);
        let mut cx = bar.min.x + 6.0;
        let mut zoomed = None;
        for (k, t) in ["-", "+", "FIT"].into_iter().enumerate() {
            let w = chip_w(t);
            let cr = Rect::from_min_size(Pos2::new(cx, bar.max.y - 24.0), Vec2::new(w, 18.0));
            cx = cr.max.x + 4.0;
            if chip(ui, cr, t, k == 2 && zoom <= 1.0, &format!("wv_zoom_{}", k)).clicked() {
                zoomed = Some(k);
            }
        }
        // ---- touch: tap jumps there; a drag by A or B moves it, inside the loop slides it, anywhere else makes a loop
        let on_chips = |pp: Pos2| pp.x < cx && pp.y > bar.max.y - 26.0;
        let mut band_hit = None;
        if resp.clicked() && !pinching {
            if let Some(pp) = resp.interact_pointer_pos() {
                band_hit = bands.iter().find(|(r, _)| r.expand(3.0).contains(pp)).map(|b| b.1);
                if band_hit.is_none() && !on_chips(pp) {
                    self.seek(time_at(pp.x));
                }
            }
        }
        if let Some(i) = band_hit {
            if let Some(l) = k.loops.get(i) {
                self.loop_a = Some(l.a);
                self.loop_b = Some(l.b);
                self.loop_on = true;
                self.sync_player();
                self.seek(l.a);
            }
        }
        if let Some(z) = zoomed {
            match z {
                0 => self.wv_zoom = (zoom / 2.0).max(1.0),
                1 => {
                    self.wv_zoom = (zoom * 2.0).min(64.0);
                    self.wv_start = shown - dur / self.wv_zoom * 0.3;
                }
                _ => {
                    self.wv_zoom = 1.0;
                    self.wv_start = 0.0;
                }
            }
        }
        if resp.drag_started() && !pinching {
            if let Some(pp) = resp.interact_pointer_pos() {
                if !on_chips(pp) {
                    let t0 = time_at(pp.x);
                    let (a, b) = (self.loop_a.unwrap_or(-1.0), self.loop_b.unwrap_or(-1.0));
                    let kind = if self.loop_a.is_some() && (pp.x - at(a)).abs() < 20.0 {
                        1
                    } else if self.loop_b.is_some() && (pp.x - at(b)).abs() < 20.0 {
                        2
                    } else if a >= 0.0 && b > a && t0 > a && t0 < b {
                        3
                    } else {
                        0
                    };
                    self.wv_drag = Some((kind, t0, a, b));
                }
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
                    let (a, b) = (a.min(b), a.max(b));
                    if b - a < 0.2 {
                        // (a tap that moved a little: no loop, just a jump)
                        if kind == 0 {
                            self.loop_a = None;
                            self.loop_b = None;
                            self.loop_on = false;
                            self.seek(a);
                        }
                    } else {
                        self.loop_on = true;
                        self.passes = 0;
                        self.sync_player();
                        // (a new or slid loop starts from A; B dragged back before where the song is, too)
                        if kind == 0 || kind == 3 || (kind == 2 && self.pos() >= b) {
                            self.seek(a);
                        }
                    }
                    self.sync_player();
                }
            }
        }
    }

    /// LOOP: A and B (set where the song is, nudged a tenth with - and +), the loop's keys, the speed, saved loops.
    fn loop_tab(&mut self, ui: &mut egui::Ui) {
        let pos = self.pos();
        let active = !self.wave.is_empty();
        let both = self.loop_a.is_some() && self.loop_b.is_some();
        let a_txt = self.loop_a.map(fmt_t).unwrap_or_else(|| "-:--.-".to_string());
        let b_txt = self.loop_b.map(fmt_t).unwrap_or_else(|| "-:--.-".to_string());
        let mut nudge: Option<(u8, f32)> = None;
        // (A and B, each with its time and - +: a little smaller than the rest, to keep to one row)
        font::with_scale(UI_SCALE.min(0.84), || centered_row(ui, "ab_row", |ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            if retro_btn_w(ui, "A", 32.0, false).clicked() && active {
                self.loop_a = Some(self.loop_b.map_or(pos, |b| pos.min(b - 0.2)).max(0.0));
                self.loop_on = self.loop_b.is_some();
                self.sync_player();
            }
            if retro_btn_w(ui, "-", 26.0, false).clicked() {
                nudge = Some((1, -0.1));
            }
            lcd_box(ui, &a_txt, 66.0, pal().ink);
            if retro_btn_w(ui, "+", 26.0, false).clicked() {
                nudge = Some((1, 0.1));
            }
            divider(ui);
            if retro_btn_w(ui, "B", 32.0, false).clicked() && active {
                let b = self.loop_a.map_or(pos, |a| pos.max(a + 0.2));
                self.loop_b = Some(b);
                if self.loop_a.is_none() {
                    self.loop_a = Some(0.0);
                }
                self.loop_on = true;
                self.passes = 0;
                self.sync_player();
                // (B set where the song is: back to A, round the loop)
                if pos >= b - 0.05 {
                    self.seek(self.loop_a.unwrap_or(0.0));
                }
            }
            if retro_btn_w(ui, "-", 26.0, false).clicked() {
                nudge = Some((2, -0.1));
            }
            lcd_box(ui, &b_txt, 66.0, pal().ink);
            if retro_btn_w(ui, "+", 26.0, false).clicked() {
                nudge = Some((2, 0.1));
            }
        }));
        if let Some((end, d)) = nudge {
            let v = if end == 1 { &mut self.loop_a } else { &mut self.loop_b };
            *v = Some((v.unwrap_or(pos) + d).clamp(0.0, self.dur.max(0.1)));
            self.sync_player();
            // (B brought back before where the song is: back to A)
            if let (2, Some(a), Some(b), true) = (end, self.loop_a, self.loop_b, self.loop_on) {
                if b > a && pos >= b {
                    self.seek(a);
                }
            }
        }
        ui.add_space(4.0);
        let ink = pal().ink;
        centered_row(ui, "loop_btns", |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if icon_btn_w(ui, &IC_REW, false, ink, 40.0).clicked() {
                self.seek((pos - 2.0).max(0.0));
            }
            if icon_btn_w(ui, &IC_PREV, false, ink, 40.0).clicked() {
                self.seek(self.loop_a.unwrap_or(0.0));
            }
            divider(ui);
            if icon_btn_w(ui, &IC_REP, self.loop_on && both, ink, 40.0).clicked() && both {
                self.loop_on = !self.loop_on;
                self.passes = 0;
                self.sync_player();
            }
            if icon_btn_w(ui, &IC_X, false, ink, 40.0).clicked() {
                self.loop_a = None;
                self.loop_b = None;
                self.loop_on = false;
                self.sync_player();
            }
            divider(ui);
            for (fwd, icon) in [(false, &IC_BACK), (true, &IC_FWD)] {
                if icon_btn_w(ui, icon, false, ink, 40.0).clicked() && both {
                    // the whole loop on to the next phrase, or back to the one before (same length)
                    if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
                        let len = b - a;
                        let na = (a + if fwd { len } else { -len }).clamp(0.0, (self.dur - len).max(0.0));
                        self.loop_a = Some(na);
                        self.loop_b = Some(na + len);
                        self.sync_player();
                        self.seek(na);
                    }
                }
            }
        });
        ui.add_space(4.0);
        centered_row(ui, "speed_row", |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            label(ui, "SPEED", 0.0);
            let sp = retro_btn_w(ui, "v", 32.0, false);
            let menu = ui.id().with("speed_menu");
            if sp.clicked() {
                ui.memory_mut(|m| m.toggle_popup(menu));
            }
            egui::popup::popup_below_widget(ui, menu, &sp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(110.0);
                for pct in [50u32, 60, 70, 75, 80, 85, 90, 95, 100, 110, 125] {
                    let mark = if self.speed == pct { "> " } else { "   " };
                    if menu_item(ui, &format!("{}{}%", mark, pct)) {
                        self.set_speed(pct);
                    }
                }
            });
            if retro_btn_w(ui, "-", 28.0, false).clicked() {
                self.set_speed(self.speed.saturating_sub(5).max(25));
            }
            lcd_box(ui, &format!("{}%", self.speed), 64.0, pal().ink);
            if retro_btn_w(ui, "+", 28.0, false).clicked() {
                self.set_speed((self.speed + 5).min(200));
            }
        });
        ui.add_space(10.0);
        // ---- saved loops of this song: which one is looping now (or that it isn't saved yet)
        let k = self.kept_now();
        if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
            let saved = k.loops.iter().find(|s| (s.a - a).abs() < 0.05 && (s.b - b).abs() < 0.05).map(|s| s.name.clone());
            let what = match (&saved, self.loop_on) {
                (Some(n), true) => format!("Looping: {}", n),
                (Some(n), false) => format!("Loop set: {} (off)", n),
                (None, true) => "Looping, not saved yet. Name it and press Save Loop".to_string(),
                (None, false) => "Loop set and off, not saved yet".to_string(),
            };
            dim_line(ui, &what, 1.0, if saved.is_some() { pal().ink } else { pal().ink2 });
        }
        ui.horizontal(|ui| {
            let bw = 10.0 + 18.0 + 8.0 + text_w("SAVE LOOP", 2.0) + 10.0;
            let w = (ui.available_width() - bw - 8.0).max(120.0);
            let r = crate::songs::text_box(ui, &mut self.sec_name, w, "Name this loop (bridge, head, lick)", "sec_name");
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ibtn(ui, &IC_BOOK, "SAVE LOOP", false).clicked() || enter {
                self.save_loop();
            }
        });
        if k.loops.is_empty() {
            label(ui, "SAVED LOOPS OF THIS SONG SHOW UP HERE", 0.0);
            return;
        }
        let mut go = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            for (i, s) in k.loops.iter().enumerate() {
                let w = (text_w(&s.name, 2.0) + 20.0).clamp(44.0, 180.0);
                let on = self.loop_on && self.loop_a.is_some_and(|x| (x - s.a).abs() < 0.05) && self.loop_b.is_some_and(|x| (x - s.b).abs() < 0.05);
                if retro_btn_w(ui, &s.name, w, on).clicked() {
                    go = Some(i);
                }
            }
        });
        if let Some(i) = go {
            let s = &k.loops[i];
            self.loop_a = Some(s.a);
            self.loop_b = Some(s.b);
            self.loop_on = true;
            self.passes = 0;
            self.sec_del = Some(i);
            self.sync_player();
            self.seek(s.a);
        }
        // the loop picked last: delete it (asked first)
        if let Some(i) = self.sec_del.filter(|i| *i < k.loops.len()) {
            let name = k.loops[i].name.to_uppercase();
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                dim_line_w(ui, &format!("{}  {} - {}", name, fmt_t(k.loops[i].a), fmt_t(k.loops[i].b)));
                if retro_btn(ui, "DELETE", false).clicked() {
                    if let Some(kp) = self.kept_mut() {
                        if i < kp.loops.len() {
                            kp.loops.remove(i);
                        }
                    }
                    self.sec_del = None;
                    self.save_kept();
                }
            });
        }
    }

    pub(crate) fn set_speed(&mut self, pct: u32) {
        self.speed = pct.clamp(25, 200);
        self.sync_player();
        let sp = self.speed;
        if let Some(k) = self.kept_mut() {
            k.speed = sp;
        }
        self.save_kept();
    }

    fn save_loop(&mut self) {
        match (self.loop_a, self.loop_b) {
            (Some(a), Some(b)) if b > a => {
                let typed = self.sec_name.trim().to_string();
                let Some(k) = self.kept_mut() else { return };
                let name = if typed.is_empty() { format!("Loop {}", k.loops.len() + 1) } else { typed };
                k.loops.push(Loop { name, a, b });
                let n = k.loops.len();
                self.sec_name.clear();
                self.sec_del = Some(n - 1);
                self.save_kept();
                self.set_note("Loop saved");
            }
            _ => self.set_note("Set the loop's A and B first (or drag on the wave)"),
        }
    }

    /// TRAINER: the count-off before each pass and the song's tempo for it (tap it in), then the speed trainer.
    fn trainer_tab(&mut self, ui: &mut egui::Ui) {
        centered_row(ui, "trainer_0", |ui| {
            label(ui, "COUNT-OFF", 0.0);
            let ci = if self.count_in == 0 { "OFF".to_string() } else { format!("{} BEATS", self.count_in) };
            if retro_btn_w(ui, &ci, 92.0, self.count_in > 0).clicked() {
                self.count_in = match self.count_in {
                    0 => 2,
                    2 => 4,
                    _ => 0,
                };
                if self.count_in > 0 && self.bpm <= 0.0 {
                    self.set_note("The count-off needs the song's tempo: tap it in with TAP");
                } else if self.count_in > 0 {
                    self.set_note(&format!("{} clicks at {:.0} BPM before each pass (TAP or - + if that's off)", self.count_in, self.bpm));
                }
                self.sync_player();
                self.save_settings();
            }
        });
        centered_row(ui, "trainer_bpm", |ui| {
            label(ui, "BPM", 0.0);
            let s = if self.bpm > 0.0 { format!("{:.0}", self.bpm) } else { "---".to_string() };
            if retro_btn_w(ui, "-", 28.0, false).clicked() {
                self.set_bpm(if self.bpm > 0.0 { self.bpm - 1.0 } else { 120.0 });
            }
            lcd_box(ui, &s, 52.0, pal().ink);
            if retro_btn_w(ui, "+", 28.0, false).clicked() {
                self.set_bpm(if self.bpm > 0.0 { self.bpm + 1.0 } else { 120.0 });
            }
            if retro_btn_w(ui, "TAP", 56.0, false).clicked() {
                let now = ui.input(|i| i.time);
                self.tap_tempo(now);
            }
        });
        // (the count-off clicks at the song's tempo: none yet, it says so, and how to give it one)
        if self.count_in > 0 && self.bpm <= 0.0 {
            dim_centered(ui, "No tempo yet: tap TAP along with the song");
        }
        ui.add_space(8.0);
        centered_row(ui, "trainer_1", |ui| {
            label(ui, "TRAIN", 0.0);
            if retro_btn_w(ui, if self.trainer { "ON" } else { "OFF" }, 56.0, self.trainer).clicked() {
                self.trainer = !self.trainer;
                self.passes = 0;
                self.keys_done = 0;
                if self.trainer && !(self.loop_on && self.loop_a.is_some() && self.loop_b.is_some()) {
                    self.set_note("The trainer speeds up a loop: set one first (drag on the wave)");
                }
            }
            if self.trainer {
                lcd_box(ui, &format!("PASS {}/{}", self.passes, self.trainer_n), 100.0, pal().ink);
                if self.trainer_keys {
                    lcd_box(ui, &format!("KEY {}/12", self.keys_done + 1), 90.0, pal().ink);
                }
            }
        });
        centered_row(ui, "trainer_2", |ui| {
            label(ui, "THEN", 0.0);
            if retro_btn(ui, "FASTER", !self.trainer_keys).clicked() {
                self.trainer_keys = false;
            }
            if retro_btn(ui, "UP A 4TH - 12 KEYS", self.trainer_keys).clicked() {
                self.trainer_keys = true;
                self.keys_done = 0;
            }
        });
        centered_row(ui, "trainer_3", |ui| {
            label(ui, "EVERY", 0.0);
            if retro_btn_w(ui, "-", 28.0, false).clicked() {
                self.trainer_n = (self.trainer_n - 1).max(1);
                self.save_settings();
            }
            lcd_box(ui, &self.trainer_n.to_string(), 36.0, pal().ink);
            if retro_btn_w(ui, "+", 28.0, false).clicked() {
                self.trainer_n = (self.trainer_n + 1).min(16);
                self.save_settings();
            }
            label(ui, "LOOPS", 0.0);
            if retro_btn_w(ui, "-", 28.0, false).clicked() {
                self.trainer_step = (self.trainer_step - 1).max(1);
                self.save_settings();
            }
            lcd_box(ui, &format!("+{}%", self.trainer_step), 48.0, pal().ink);
            if retro_btn_w(ui, "+", 28.0, false).clicked() {
                self.trainer_step = (self.trainer_step + 1).min(20);
                self.save_settings();
            }
        });
        dim_centered(ui, if self.trainer_keys { "Each set a 4th higher, through all 12 keys" } else { "Starts at the speed set in Loop and stops at 100%" });
    }

    pub(crate) fn set_bpm(&mut self, b: f32) {
        self.bpm = b.clamp(30.0, 300.0).round();
        self.sync_player();
        let bpm = self.bpm;
        if let Some(k) = self.kept_mut() {
            k.bpm = bpm;
        }
        self.save_kept();
    }

    /// PITCH: up and down by semitones and cents (as the desktop's), back to the record's pitch, the ear mode.
    fn pitch_tab(&mut self, ui: &mut egui::Ui) {
        let mut d = 0;
        centered_row(ui, "pitch_0", |ui| {
            label(ui, "PITCH", 0.0);
            if retro_btn_w(ui, "-", 28.0, false).clicked() {
                d = -100;
            }
            if retro_btn_w(ui, "-10c", 44.0, false).clicked() {
                d = -10;
            }
            lcd_box(ui, &format!("{:+.2}", self.cents as f32 / 100.0), 64.0, pal().ink);
            if retro_btn_w(ui, "+10c", 44.0, false).clicked() {
                d = 10;
            }
            if retro_btn_w(ui, "+", 28.0, false).clicked() {
                d = 100;
            }
        });
        if d != 0 {
            self.set_cents(self.cents + d);
        }
        let line = if self.cents == 0 {
            "At the record's pitch. - and + move a semitone".to_string()
        } else {
            let semis = self.cents as f32 / 100.0;
            format!("{:+.2} semitones from the record", semis)
        };
        dim_centered(ui, &line);
        ui.add_space(4.0);
        centered_row(ui, "pitch_1", |ui| {
            if self.cents != 0 && retro_btn_w(ui, "0", 28.0, false).clicked() {
                self.set_cents(0);
            }
            ui.add_space(8.0);
            if retro_btn_w(ui, CHAN_NAMES[self.chan as usize % 6], 100.0, self.chan != 0).clicked() {
                self.chan = (self.chan + 1) % 6;
                self.sync_player();
            }
        });
        dim_centered(ui, "Ear mode: tap to cycle (No center takes out the vocals)");
    }

    fn set_cents(&mut self, c: i32) {
        self.cents = c.clamp(-1200, 1200);
        self.sync_player();
        let cents = self.cents;
        if let Some(k) = self.kept_mut() {
            k.cents = cents;
        }
        self.save_kept();
    }

    /// BOOKMARKS: a spot to come back to (named, or numbered), tap one to jump there.
    fn marks_tab(&mut self, ui: &mut egui::Ui) {
        let pos = self.pos();
        // ---- your notes on the song: what to work on, fingerings, where you got to
        let mut notes = self.kept_now().notes;
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 74.0), Sense::hover());
        inset(ui.painter(), r, pal().display);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(Vec2::new(6.0, 4.0))));
        child.visuals_mut().override_text_color = Some(pal().ink);
        child.visuals_mut().extreme_bg_color = pal().display;
        child.visuals_mut().selection.bg_fill = pal().row_sel;
        child.visuals_mut().text_cursor.stroke = Stroke::new(2.0_f32, pal().ink);
        let changed = child
            .add(
                egui::TextEdit::multiline(&mut notes)
                    .id_salt("song_notes")
                    .frame(false)
                    .hint_text(egui::RichText::new("Notes on this song: what to work on, fingerings, where you got to").color(pal().dim))
                    .desired_width(r.width() - 12.0)
                    .desired_rows(3)
                    .font(egui::FontId::proportional(15.0)),
            )
            .changed();
        if changed {
            if let Some(k) = self.kept_mut() {
                k.notes = notes;
            }
            self.save_kept();
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let bw = 10.0 + 18.0 + 8.0 + text_w("BOOKMARK", 2.0) + 10.0;
            let w = (ui.available_width() - bw - 8.0).max(120.0);
            let r = crate::songs::text_box(ui, &mut self.mark_name, w, "Name this spot (solo, chorus 2...)", "mark_name");
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ibtn(ui, &IC_BOOK, "BOOKMARK", false).clicked() || enter {
                if self.wave.is_empty() {
                    self.set_note("Wait for the song to come in first");
                } else {
                    let name = self.mark_name.trim().to_string();
                    if let Some(k) = self.kept_mut() {
                        k.marks.push(Mark { t: pos, name });
                        k.marks.sort_by(|a, b| a.t.total_cmp(&b.t));
                    }
                    self.mark_name.clear();
                    self.save_kept();
                    self.set_note(&format!("Bookmarked {}", fmt_t(pos)));
                }
            }
        });
        let k = self.kept_now();
        if k.marks.is_empty() {
            label(ui, "BOOKMARKS OF THIS SONG SHOW UP HERE", 0.0);
            return;
        }
        let mut go = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            for (i, m) in k.marks.iter().enumerate() {
                let t = if m.name.is_empty() { format!("{}  {}", i + 1, fmt_t(m.t)) } else { format!("{}  {}", i + 1, m.name) };
                let w = (text_w(&t, 2.0) + 20.0).clamp(60.0, 200.0);
                if retro_btn_w(ui, &t, w, self.mark_del == Some(i)).clicked() {
                    go = Some(i);
                }
            }
        });
        if let Some(i) = go {
            self.mark_del = Some(i);
            self.seek(k.marks[i].t);
        }
        if let Some(i) = self.mark_del.filter(|i| *i < k.marks.len()) {
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                let m = &k.marks[i];
                dim_line_w(ui, &format!("{}  {}", if m.name.is_empty() { format!("BOOKMARK {}", i + 1) } else { m.name.to_uppercase() }, fmt_t(m.t)));
                if retro_btn(ui, "DELETE", false).clicked() {
                    if let Some(kp) = self.kept_mut() {
                        if i < kp.marks.len() {
                            kp.marks.remove(i);
                        }
                    }
                    self.mark_del = None;
                    self.save_kept();
                }
            });
        }
    }
}

/// A dim line as wide as its words (beside a key in a row).
fn dim_line_w(ui: &mut egui::Ui, text: &str) {
    let w = text_w(text, 1.5) + 12.0;
    let (r, _) = ui.allocate_exact_size(Vec2::new(w.min(ui.available_width() - 90.0).max(60.0), bh()), Sense::hover());
    ptext_fit(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, text, 1.5, r.width() - 6.0, pal().ink2);
}
