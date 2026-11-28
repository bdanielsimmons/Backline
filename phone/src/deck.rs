//! NOW PLAYING: the desktop's Studio deck at a phone's width. The cover (the video's thumbnail), the time in LCD
//! digits, the song scrolling by in dot type, labels for how it's playing, the bars, how far in, the keys and the
//! volume.

use crate::studio::*;
use crate::studio::marquee;
use crate::*;

impl App {
    pub(crate) fn deck(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let id = ui.id().with("deck");
        let s = sc();
        let cur = self.cur.clone();
        // ---- the cover, the time, the song, how it's playing
        let cs = 66.0 * s;
        let cover = Rect::from_min_size(r.min, Vec2::splat(cs));
        match cur.as_ref().and_then(|c| self.thumb(c)) {
            Some(t) => {
                p.image(t, cover, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            }
            None => {
                p.rect_filled(cover, 0.0, ghost());
            }
        }
        p.rect_stroke(cover, 0.0, Stroke::new(1.0_f32, ink()));
        let tx = cover.max.x + 10.0;
        let th = (cs * 0.42).clamp(24.0, 34.0);
        let pos = if cur.is_some() && !self.stopped { self.pos() } else { 0.0 };
        let shown = format!(" {}", fmt_time(pos));
        let tw = time_digits(&p, Pos2::new(tx, r.min.y + th / 2.0 + 2.0), th, &shown, self.dur);
        let mq = Rect::from_min_max(Pos2::new(tx, r.min.y + th + 8.0), Pos2::new(r.max.x, r.min.y + th + 8.0 + 18.0 * s));
        match &cur {
            Some(c) => {
                // who and the song, YouTube's clutter off ("(Official Video)"...), as the desktop's deck shows a song
                let (who, name) = parts(c);
                let line = if who.is_empty() {
                    format!("{} ({})", name.to_uppercase(), fmt_time(self.dur))
                } else {
                    format!("{} - {} ({})", who.to_uppercase(), name.to_uppercase(), fmt_time(self.dur))
                };
                marquee(ui, mq, &line, 15.0, ink());
            }
            None => {
                dot(&p, Pos2::new(mq.min.x, mq.center().y), egui::Align2::LEFT_CENTER, "PICK A SONG BELOW", 15.0, dim());
            }
        }
        // labels: where it comes from, and the practice settings that are on (lit)
        let mut x = tx;
        let ly = mq.max.y + 10.0 * s;
        let loading = if self.loading { Some("LOADING") } else { None };
        let kept_here = cur.as_ref().is_some_and(|c| !crate::device::is_file(&c.vid) && saved(&c.vid));
        let speed = format!("SPEED {}%", self.speed);
        let pitch = format!("PITCH {:+.1}", self.cents as f32 / 100.0);
        let parts: Vec<(&str, bool)> = [
            Some((if cur.as_ref().is_some_and(|c| crate::device::is_file(&c.vid)) { "PHONE" } else { "YOUTUBE" }, false)),
            kept_here.then_some(("SAVED", false)),
            loading.map(|l| (l, true)),
            Some((speed.as_str(), self.speed != 100)),
            (self.cents != 0).then_some((pitch.as_str(), true)),
            Some(("LOOP", self.loop_on && self.loop_a.is_some() && self.loop_b.is_some())),
        ]
        .into_iter()
        .flatten()
        .collect();
        for (t, lit) in parts {
            let w = p.ctx().fonts(|f| f.layout_no_wrap(t.to_string(), egui::FontId::new(8.5 * s, egui::FontFamily::Name("studio_px".into())), Color32::WHITE).size().x) + 8.0;
            if x + w > r.max.x {
                break;
            }
            let lr = Rect::from_min_size(Pos2::new(x, ly - 7.0 * s), Vec2::new(w, 14.0 * s));
            if lit {
                p.rect_filled(lr, 0.0, ink());
            }
            p.rect_stroke(lr, 0.0, Stroke::new(1.0_f32, if lit { ink() } else { dim() }));
            px(&p, lr.center() + Vec2::new(0.0, 0.5), egui::Align2::CENTER_CENTER, t, 8.5, if lit { scr() } else { dim() });
            x = lr.max.x + 5.0;
        }
        // ---- the bars, how far in
        let mut y = cover.max.y.max(ly + 8.0 * s) + 8.0;
        let keys_h = 28.0 * s;
        let tg_h = 24.0 * s;
        let vh = (r.max.y - y - 8.0 - 11.0 - 9.0 - keys_h - 6.0 - tg_h).clamp(12.0, 60.0);
        let vr = Rect::from_min_max(Pos2::new(r.min.x, y), Pos2::new(r.max.x, y + vh));
        bars(&p, vr, &self.bands, &self.peaks, 2.0, 3.0);
        y = vr.max.y + 8.0;
        let seek = Rect::from_min_max(Pos2::new(r.min.x, y), Pos2::new(r.max.x, y + 11.0));
        let dur = self.dur;
        let h = ui.interact(seek.expand2(Vec2::new(0.0, 8.0)), id.with("seek"), Sense::click_and_drag());
        if (h.dragged() || h.drag_started()) && dur > 0.0 {
            if let Some(pp) = h.interact_pointer_pos() {
                self.seek_drag = Some(((pp.x - seek.min.x) / seek.width()).clamp(0.0, 1.0) * dur);
            }
        }
        if h.drag_stopped() || h.clicked() {
            let t = h.interact_pointer_pos().map(|pp| ((pp.x - seek.min.x) / seek.width()).clamp(0.0, 1.0) * dur).or(self.seek_drag);
            self.seek_drag = None;
            if let (Some(t), true) = (t, dur > 0.0) {
                self.seek(t);
            }
        }
        blocks(&p, seek, if dur > 0.0 { self.pos() / dur } else { 0.0 });
        y = seek.max.y + 9.0;
        // ---- the keys, the volume
        let playing = cur.is_some() && !self.stopped && !self.paused;
        let names = [("prev", "Previous", false), ("play", "Play", playing), ("pause", "Pause", self.paused), ("stop", "Stop", false), ("skip", "Next", false)];
        let kw = 38.0 * s;
        for (n, (icon, _tip, lit)) in names.into_iter().enumerate() {
            let kr = Rect::from_min_size(Pos2::new(r.min.x + n as f32 * (kw + 4.0), y), Vec2::new(kw, keys_h));
            let h = ui.interact(kr, id.with(("key", n)), Sense::click());
            key(&p, kr, lit, h.is_pointer_button_down_on(), false);
            crate::vicon::named(&p, Rect::from_center_size(kr.center(), Vec2::splat(14.0 * s)), icon, key_fg(lit));
            if h.clicked() {
                match n {
                    0 => self.step_song(-1),
                    1 => {
                        if self.stopped || self.paused {
                            if self.cur.is_none() {
                                if let Some(s) = self.songs.first().cloned() {
                                    self.load(s, true);
                                }
                            } else {
                                self.want_play = true;
                                self.play_pause();
                            }
                        }
                    }
                    2 => {
                        if !self.stopped {
                            self.play_pause();
                        }
                    }
                    3 => self.stop(),
                    _ => self.step_song(1),
                }
            }
        }
        let vx = r.min.x + 5.0 * (kw + 4.0) + 8.0;
        px(&p, Pos2::new(vx, y + keys_h / 2.0), egui::Align2::LEFT_CENTER, "VOL", 8.5, dim());
        let vol = Rect::from_min_max(Pos2::new(vx + 24.0 * s, y + keys_h / 2.0 - 5.0), Pos2::new(r.max.x, y + keys_h / 2.0 + 5.0));
        let vh = ui.interact(vol.expand2(Vec2::new(0.0, 10.0)), id.with("vol"), Sense::click_and_drag());
        if vh.clicked() || vh.dragged() {
            if let Some(pp) = vh.interact_pointer_pos() {
                self.set_volume(((pp.x - vol.min.x) / vol.width()).clamp(0.0, 1.0));
            }
        }
        if vh.drag_stopped() || vh.clicked() {
            self.save_settings();
        }
        if vol.width() > 20.0 {
            blocks(&p, vol, self.volume);
        }
        // ---- SHUF and REP with their lights (the desktop's deck), then the metronome: start / stop, and its key (the
        // tempo and meter, its light on the beat; it opens the metronome in Practice)
        let y = y + keys_h + 6.0;
        let font = egui::FontId::new(10.0 * s, egui::FontFamily::Name("studio_mono".into()));
        let tw = |t: &str| p.ctx().fonts(|f| f.layout_no_wrap(t.to_string(), font.clone(), Color32::WHITE).size().x);
        let mut x = r.min.x;
        let rep = ["REP", "REP", "REP 1"][self.repeat.min(2) as usize];
        for (n, (name, on)) in [("SHUF", self.shuffle), (rep, self.repeat > 0)].into_iter().enumerate() {
            let kr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(tw(name) + 22.0, tg_h));
            x = kr.max.x + 4.0;
            let h = ui.interact(kr, id.with(("tg", n)), Sense::click());
            key(&p, kr, false, h.is_pointer_button_down_on(), false);
            light(&p, Pos2::new(kr.min.x + 8.0, kr.center().y), on, ink());
            p.text(Pos2::new(kr.min.x + 15.0, kr.center().y), egui::Align2::LEFT_CENTER, name, font.clone(), ink());
            if h.clicked() {
                if n == 0 {
                    self.toggle_shuffle();
                } else {
                    self.cycle_repeat();
                }
            }
        }
        let on = self.mt.on;
        let mt_label = format!("METRO {} {}/{}", self.mt_bpm, self.mt.beats, self.mt.unit);
        let mw = tw(&mt_label) + 22.0;
        let mk = Rect::from_min_size(Pos2::new(r.max.x - mw, y), Vec2::new(mw, tg_h));
        let gk = Rect::from_min_size(Pos2::new(mk.min.x - 4.0 - tg_h * 1.4, y), Vec2::new(tg_h * 1.4, tg_h));
        let gh = ui.interact(gk, id.with("mt_go"), Sense::click());
        key(&p, gk, on, gh.is_pointer_button_down_on(), false);
        crate::vicon::named(&p, Rect::from_center_size(gk.center(), Vec2::splat(11.0 * s)), if on { "stop" } else { "play" }, key_fg(on));
        let mh = ui.interact(mk, id.with("mt_open"), Sense::click());
        let open = self.view == 1 && self.mt_open;
        key(&p, mk, open, mh.is_pointer_button_down_on(), false);
        let (_, frac, muted) = self.metro_pos();
        light(&p, Pos2::new(mk.min.x + 8.0, mk.center().y), on && !muted && frac < 0.3, if open { scr() } else { ink() });
        p.text(Pos2::new(mk.min.x + 15.0, mk.center().y), egui::Align2::LEFT_CENTER, &mt_label, font.clone(), key_fg(open));
        if gh.clicked() {
            self.metro_toggle();
        }
        if mh.clicked() {
            if open {
                self.mt_open = false;
            } else {
                self.view = 1;
                self.mt_open = true;
            }
        }
    }
}
