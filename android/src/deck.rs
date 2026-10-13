//! The PLAYER screen: Backline's own deck (the desktop's, drawn the same way at the phone's width), and under it
//! the QUEUE window - or, in practice mode, the PRACTICE window.

use crate::*;

impl App {
    pub(crate) fn player_screen(&mut self, ui: &mut egui::Ui, r: Rect) {
        // the deck keeps its shape (300 x 138, as on the desktop); the rest of the screen is the window under it
        let inner_w = r.width() - 28.0;
        // a worn skin's deck is Winamp's shape (275 x 116); Backline's own is 300 x 138
        let deck_h = if self.wa_art.is_some() { 116.0 * inner_w / 275.0 } else { 138.0 * inner_w / 300.0 } + 44.0;
        let top = Rect::from_min_size(r.min, Vec2::new(r.width(), deck_h));
        let rest = Rect::from_min_max(Pos2::new(r.min.x, top.max.y + 4.0), r.max);
        self.window(ui, top, "PLAYER", |me, ui| me.deck(ui));
        // listening or practising: the two chips on the player's title line, as on the desktop
        let w = 74.0;
        let chip_y = top.min.y + 4.0;
        let r_prac = Rect::from_min_size(Pos2::new(top.max.x - 14.0 - w, chip_y), Vec2::new(w, 18.0));
        let r_lis = Rect::from_min_size(Pos2::new(r_prac.min.x - 6.0 - w, chip_y), Vec2::new(w, 18.0));
        if chip(ui, r_lis, "LISTEN", !self.practice, "c_lis").clicked() && self.practice {
            self.practice = false;
            self.sync_player();
            self.save_settings();
        }
        if chip(ui, r_prac, "PRACTICE", self.practice, "c_prac").clicked() && !self.practice {
            self.practice = true;
            self.sync_player();
            self.save_settings();
        }
        if self.practice {
            self.window(ui, rest, "PRACTICE", |me, ui| me.practice_ui(ui));
        } else {
            self.window(ui, rest, "QUEUE", |me, ui| me.queue_ui(ui));
        }
    }

    /// Backline's own deck: the LCD time, the spectrum, the title and what is next, tempo and key, the cover, like,
    /// volume, speed, the sleep timer, the seek bar (with the loop on it in practice), the transport, shuffle and
    /// repeat - the desktop's, piece for piece.
    fn deck(&mut self, ui: &mut egui::Ui) {
        // a worn skin draws the whole deck from its own pictures, as on the desktop
        if self.wa_art.is_some() {
            self.winamp_deck(ui);
            return;
        }
        let inner = ui.max_rect().expand(4.0);
        let s = (inner.width() / 300.0).min(inner.height() / 138.0).max(0.5);
        let ox = inner.min.x + (inner.width() - 300.0 * s) / 2.0;
        let oy = inner.min.y + (inner.height() - 138.0 * s) / 2.0;
        let rc = move |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(Pos2::new(ox + x * s, oy + y * s), Vec2::new(w * s, h * s));
        let px_big = (1.5 * s).round().max(1.0);
        let px_sm = (1.0 * s).round().max(1.0);
        let now_t = ui.input(|i| i.time);
        let track = self.cur_track().cloned();
        let pos = self.pos();
        let dur = track.as_ref().map(|t| t.duration).unwrap_or(0.0);
        let shown = self.seek_drag.unwrap_or(pos);
        let active = self.cur.is_some() && !self.stopped;
        let painter = ui.painter().clone();
        let p = &painter;

        // ---- LCD time
        let lcd = rc(6.0, 6.0, 100.0, 40.0);
        inset(p, lcd, pal().lcd);
        let icon: &[&str] = if !active {
            &STOP_S[..]
        } else if self.paused {
            &PAUSE_S[..]
        } else {
            &PLAY_S[..]
        };
        pixmap(p, Pos2::new(snap(ox + 10.0 * s), snap(oy + 11.0 * s)), (0.9 * s).round().max(1.0), icon, pal().ink);
        let blink = !(active && self.paused) || (now_t * 2.0) as i64 % 2 == 0;
        let e = if active { shown.max(0.0) as u32 } else { 0 };
        let (mm, ss) = ((e / 60).min(99), e % 60);
        let digits = [mm / 10, mm % 10, ss / 10, ss % 10];
        let xs = [22.0f32, 38.0, 62.0, 78.0];
        for i in 0..4 {
            seg_digit(p, Pos2::new(ox + xs[i] * s, oy + 14.0 * s), 1.5 * s, digits[i], blink);
        }
        fill_rect(p, rc(54.0, 20.0, 3.0, 3.0), pal().ink);
        fill_rect(p, rc(54.0, 29.0, 3.0, 3.0), pal().ink);

        // ---- spectrum (tap: bars / waveform / both)
        let sp = rc(6.0, 50.0, 100.0, 28.0);
        inset(p, sp, pal().lcd);
        viz::viz_draw(p, sp.shrink2(Vec2::new(3.0, 3.0)), &self.bands, &self.peaks, &self.viz_wave, self.viz_mode, 1.0, false, 1.0, None);
        if ui.interact(sp, ui.id().with("spectrum"), Sense::click()).clicked() {
            self.viz_mode = (self.viz_mode + 1) % 3;
        }

        // ---- title + what is next
        let tb = rc(112.0, 6.0, 182.0, 14.0);
        inset(p, tb, pal().lcd);
        let title_txt = match &track {
            Some(t) => format!("{} - {}  ({})", t.artists_text(), t.title, fmt_time(t.duration)),
            None => "Backline - a retro player for Tidal".to_string(),
        };
        marquee(ui, tb, &title_txt, px_big, pal().ink);
        widgets::TITLE_SCROLL.store((9.0 * spx(px_big)).to_bits(), Ordering::Relaxed);
        let sb = rc(112.0, 22.0, 182.0, 10.0);
        inset(p, sb, pal().lcd);
        let next = self.cur.and_then(|i| self.queue.get(i + 1)).cloned();
        let (stxt, scol): (String, Color32) = if self.loading {
            ("LOADING...".to_string(), pal().ink2)
        } else if let (true, Some(n)) = (active, &next) {
            (format!("NEXT: {} - {}", n.artists_text(), n.title), pal().ink2)
        } else if active {
            ("END OF QUEUE".to_string(), pal().ink2)
        } else {
            (String::new(), pal().ink2)
        };
        marquee(ui, sb, &stxt, px_sm, scol);

        // ---- tempo and key
        let info = track.as_ref().and_then(|t| meta::get(t.id)).filter(|_| active).unwrap_or_default();
        let kb = rc(112.0, 34.0, 66.0, 12.0);
        inset(p, kb, pal().lcd);
        let kb_txt = info.bpm.map_or("--- BPM".to_string(), |b| format!("{} BPM", b));
        ptext_fit(p, kb.center(), Align::Center, &kb_txt, px_sm, kb.width() - 6.0, pal().ink);
        let kh = rc(182.0, 34.0, 68.0, 12.0);
        inset(p, kh, pal().lcd);
        let kh_txt = if info.key.is_some() { meta::key_text(&info) } else { "KEY --".to_string() };
        ptext_fit(p, kh.center(), Align::Center, &kh_txt, px_sm, kh.width() - 6.0, pal().ink);

        // ---- cover art
        let cv = rc(258.0, 34.0, 36.0, 36.0);
        inset(p, cv, pal().edge);
        if let Some(t) = &track {
            self.want_cover(t);
            if let Some(tex) = self.cover.as_ref().and_then(|c| c.1.as_ref()) {
                p.image(tex.id(), cv.shrink(2.0), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            }
        }

        // ---- like (Tidal songs)
        let hb = rc(258.0, 72.0, 36.0, 10.0);
        let is_liked = track.as_ref().is_some_and(|t| self.liked_ids.contains(&t.id));
        let tidal = track.as_ref().is_some_and(|t| t.id > 0);
        let hr = ui.interact(hb, ui.id().with("heart"), Sense::click());
        raised_h(p, hb, hr.is_pointer_button_down_on(), false);
        let hpx = (0.9 * s).round().max(1.0);
        pixmap(p, Pos2::new(snap(hb.center().x - 3.5 * hpx), snap(hb.center().y - 3.0 * hpx)), hpx, &HEART[..], if is_liked { pal().red } else { pal().beige_dk });
        if hr.clicked() && tidal && self.logged_in {
            if let Some(t) = &track {
                let (id, like) = (t.id, !is_liked);
                if like {
                    self.liked_ids.insert(id);
                } else {
                    self.liked_ids.remove(&id);
                }
                let api = self.api.clone();
                std::thread::spawn(move || {
                    let _ = api.set_liked(id, like);
                });
            }
        }

        // ---- volume
        let vg = rc(112.0, 54.0, 138.0, 7.0);
        for i in 0..28 {
            let h = 1.0 + i as f32 * 0.09;
            fill_rect(p, rc(113.0 + i as f32 * 4.85, 52.0 - h, 1.0, h), pal().ink2);
        }
        inset(p, vg, pal().groove);
        let thumb_v = 9.0 * s;
        let vx = vg.min.x + (vg.width() - thumb_v) * self.volume;
        fill_rect(p, Rect::from_min_max(vg.min + Vec2::new(1.5, 1.5), Pos2::new(vx + thumb_v / 2.0, vg.max.y - 1.0)), pal().ink2);
        raised(p, Rect::from_min_size(Pos2::new(vx, oy + 50.0 * s), Vec2::new(thumb_v, 15.0 * s)), false);
        let vr = ui.interact(rc(112.0, 46.0, 138.0, 20.0), ui.id().with("vol"), Sense::click_and_drag());
        if vr.dragged() || vr.clicked() {
            if let Some(pp) = vr.interact_pointer_pos() {
                let v = ((pp.x - vg.min.x - thumb_v / 2.0) / (vg.width() - thumb_v)).clamp(0.0, 1.0);
                self.set_volume(v);
            }
        }
        // speaker (mute), speed, sleep timer
        let spk_r = rc(112.0, 67.0, 16.0, 12.0);
        let spk_icon: &[&str] = if self.volume <= 0.0 {
            &IC_SPK0
        } else if self.volume < 0.5 {
            &IC_SPK1
        } else {
            &IC_SPK2
        };
        let k = (spk_r.height() / spk_icon.len() as f32).floor().max(1.0);
        pixmap(p, spk_r.min, k, spk_icon, pal().ink2);
        if ui.interact(spk_r.expand(3.0), ui.id().with("mute"), Sense::click()).clicked() {
            self.set_volume(if self.volume > 0.0 { 0.0 } else { 0.6 });
        }
        let spd = rc(134.0, 67.0, 40.0, 12.0);
        let spr = ui.interact(spd, ui.id().with("speed"), Sense::click());
        raised_h(p, spd, self.speed != 100 || spr.is_pointer_button_down_on(), false);
        let s_txt = {
            let v = format!("{:.2}", self.speed as f32 / 100.0);
            format!("{}X", v.trim_end_matches('0').trim_end_matches('.'))
        };
        ptext_fit(p, spd.center(), Align::Center, &s_txt, px_sm, spd.width() - 4.0, pal().ink);
        if spr.clicked() {
            self.speed = SPEEDS.iter().copied().find(|&x| x > self.speed).unwrap_or(SPEEDS[0]);
            self.sync_player();
        }
        let slp = rc(178.0, 67.0, 40.0, 12.0);
        let sleeping = self.sleep_at.map(|t| t.saturating_duration_since(Instant::now()).as_secs().div_ceil(60));
        let slr = ui.interact(slp, ui.id().with("sleep"), Sense::click());
        raised_h(p, slp, sleeping.is_some() || slr.is_pointer_button_down_on(), false);
        match sleeping {
            Some(m) => ptext_fit(p, slp.center(), Align::Center, &format!("{}M", m), px_sm, slp.width() - 4.0, pal().ink),
            None => icon_in(p, slp, &IC_MOON, pal().ink),
        }
        if slr.clicked() {
            // 15 / 30 / 45 / 60 / 90 minutes, then off
            let next = match sleeping {
                None => Some(15),
                Some(m) if m <= 15 => Some(30),
                Some(m) if m <= 30 => Some(45),
                Some(m) if m <= 45 => Some(60),
                Some(m) if m <= 60 => Some(90),
                _ => None,
            };
            self.sleep_at = next.map(|m| Instant::now() + Duration::from_secs(m * 60));
            self.set_note(&match next {
                Some(m) => format!("SLEEP IN {} MIN", m),
                None => "SLEEP TIMER OFF".to_string(),
            });
        }
        // the equalizer (it lives in PRACTICE > PITCH & EAR)
        let ob = rc(224.0, 67.0, 26.0, 12.0);
        let obr = ui.interact(ob, ui.id().with("eqb"), Sense::click());
        raised_h(p, ob, self.eq_on || obr.is_pointer_button_down_on(), false);
        icon_in(p, ob, &IC_EQ, pal().ink);
        if obr.clicked() {
            self.practice = true;
            self.mtab = 2;
            self.sync_player();
        }

        // ---- seek bar (the loop on it in practice)
        let sk = rc(6.0, 84.0, 288.0, 9.0);
        let thumb_w = 22.0 * s;
        let skr = ui.interact(rc(6.0, 78.0, 288.0, 21.0), ui.id().with("seek"), Sense::click_and_drag());
        if active && dur > 0.0 {
            if skr.dragged() || skr.clicked() {
                if let Some(pp) = skr.interact_pointer_pos() {
                    let f = ((pp.x - sk.min.x - thumb_w / 2.0) / (sk.width() - thumb_w)).clamp(0.0, 1.0);
                    self.seek_drag = Some(f * dur);
                }
            }
            if skr.drag_stopped() || skr.clicked() {
                if let Some(t) = self.seek_drag.take() {
                    self.player.send(Cmd::Seek(t));
                }
            }
        }
        let frac = if active && dur > 0.0 { (shown / dur).clamp(0.0, 1.0) } else { 0.0 };
        inset(p, sk, pal().groove);
        let tx = sk.min.x + (sk.width() - thumb_w) * frac;
        fill_rect(p, Rect::from_min_max(sk.min + Vec2::new(1.5, 1.5), Pos2::new(tx + thumb_w / 2.0, sk.max.y - 1.0)), pal().ink2);
        if active && dur > 0.0 && self.practice {
            let at = |t: f32| sk.min.x + thumb_w / 2.0 + (sk.width() - thumb_w) * (t / dur).clamp(0.0, 1.0);
            if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
                let col = if self.loop_on { loop_col().gamma_multiply(0.5) } else { pal().ink.gamma_multiply(0.3) };
                fill_rect(p, Rect::from_min_max(Pos2::new(at(a), sk.min.y + 1.5), Pos2::new(at(b), sk.max.y - 1.0)), col);
            }
            for (v, lab) in [(self.loop_a, "A"), (self.loop_b, "B")] {
                if let Some(t) = v {
                    let x = at(t);
                    fill_rect(p, Rect::from_min_max(Pos2::new(x - 1.0, sk.min.y - 3.0 * s), Pos2::new(x + 1.0, sk.max.y + 1.0)), pal().red);
                    ptext(p, Pos2::new(x + 3.0, sk.min.y - 5.0 * s), Align::Min, lab, px_sm, pal().red);
                }
            }
        }
        raised(p, Rect::from_min_size(Pos2::new(tx, sk.min.y - 1.0 * s), Vec2::new(thumb_w, sk.height() + 2.0 * s)), false);

        // ---- transport
        let cy = oy + 112.0 * s;
        let r = 10.5 * s;
        let px = (1.3 * s).max(1.0);
        if round_btn(ui, Pos2::new(ox + 20.0 * s, cy), r, &PREV[..], px, "b_prev").clicked() {
            self.prev();
        }
        if round_btn(ui, Pos2::new(ox + 46.0 * s, cy), r, &PLAY[..], px, "b_play").clicked() && (self.paused || self.stopped) {
            self.play_pause();
        }
        if round_btn(ui, Pos2::new(ox + 72.0 * s, cy), r, &PAUSE[..], px, "b_pause").clicked() && !self.paused && !self.stopped {
            self.play_pause();
        }
        if round_btn(ui, Pos2::new(ox + 98.0 * s, cy), r, &STOP[..], px, "b_stop").clicked() {
            self.stop();
        }
        if round_btn(ui, Pos2::new(ox + 124.0 * s, cy), r, &NEXT[..], px, "b_next").clicked() {
            self.next();
        }
        // ---- shuffle / repeat
        let sh = rc(150.0, 104.0, 30.0, 16.0);
        let shr = ui.interact(sh, ui.id().with("shuf"), Sense::click());
        raised_h(p, sh, self.shuffle || shr.is_pointer_button_down_on(), false);
        icon_in(p, sh, &IC_SHUF, if self.shuffle { pal().red } else { pal().ink });
        if shr.clicked() {
            self.shuffle = !self.shuffle;
        }
        let rp = rc(186.0, 104.0, 30.0, 16.0);
        let rpr = ui.interact(rp, ui.id().with("rep"), Sense::click());
        raised_h(p, rp, self.repeat != Repeat::Off || rpr.is_pointer_button_down_on(), false);
        let rep_icon: &[&str] = if self.repeat == Repeat::One { &IC_REP1 } else { &IC_REP };
        icon_in(p, rp, rep_icon, if self.repeat != Repeat::Off { pal().red } else { pal().ink });
        if rpr.clicked() {
            self.repeat = match self.repeat {
                Repeat::Off => Repeat::All,
                Repeat::All => Repeat::One,
                Repeat::One => Repeat::Off,
            };
        }
        // ---- where the song comes from
        let strip = rc(222.0, 104.0, 72.0, 16.0);
        inset(p, strip, pal().lcd);
        let src = match &track {
            None => "",
            Some(t) if self.yt.iter().any(|y| yt_id(&y.vid) == t.id) => "YOUTUBE",
            Some(t) if self.files.iter().any(|f| f.0.id == t.id) => "FILE",
            Some(_) => "TIDAL",
        };
        ptext_fit(p, strip.center(), Align::Center, src, px_sm, strip.width() - 6.0, pal().ink2);
        let _ = (px_big, rc(0.0, 0.0, 0.0, 0.0));
    }

    /// The deck drawn from the worn skin, the desktop's winamp_main at the phone's width: the play light, the time in
    /// the skin's digits, the scrolling title (with key and tempo), kbps and kHz, mono / stereo, the skin's own
    /// visualizer, volume and balance, EQ and PL, the seek bar, the transport, eject, shuffle and repeat.
    fn winamp_deck(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.wa_art.take() else { return };
        let area = ui.max_rect().expand(4.0);
        let s = area.width() / 275.0;
        let pl = winamp_ui::Place { o: Pos2::new(area.min.x, area.center().y - 58.0 * s), s };
        let p = ui.painter().clone();
        let track = self.cur_track().cloned();
        let pos = self.seek_drag.unwrap_or_else(|| self.pos());
        let dur = track.as_ref().map(|t| t.duration).unwrap_or(0.0);
        let active = self.cur.is_some() && !self.stopped;
        let playing = active && !self.paused;
        let rate = self.player.shared.lock().unwrap().rate;
        let id = ui.id().with("winamp_main");
        let hit = |ui: &mut egui::Ui, name: &str, r: Rect, drag: bool| {
            ui.interact(r, id.with(name), if drag { Sense::click_and_drag() } else { Sense::click() })
        };
        t.spr(&p, "main", (0.0, 0.0, 275.0, 116.0), pl.r(0.0, 0.0, 275.0, 116.0));
        t.spr(&p, "titlebar", (27.0, 0.0, 275.0, 14.0), pl.r(0.0, 0.0, 275.0, 14.0));
        // playing / paused / stopped
        let st = if playing {
            0.0
        } else if active {
            9.0
        } else {
            18.0
        };
        t.spr(&p, "playpaus", (st, 0.0, 9.0, 9.0), pl.r(26.0, 28.0, 9.0, 9.0));
        // the time, in the skin's big digits
        let digits = if t.size("nums_ex").is_some() { "nums_ex" } else { "numbers" };
        let secs = pos.max(0.0) as u32;
        let (m, sc) = ((secs / 60) % 100, secs % 60);
        if active {
            for (x, d) in [(48.0, m / 10), (60.0, m % 10), (78.0, sc / 10), (90.0, sc % 10)] {
                t.spr(&p, digits, (d as f32 * 9.0, 0.0, 9.0, 13.0), pl.r(x, 26.0, 9.0, 13.0));
            }
        }
        // the scrolling title (the key and tempo ride along once known); a slider being moved says where it is
        let title = match (&self.wa_hint, &track) {
            (Some(h), _) => h.clone(),
            (None, Some(tr)) => {
                let info = meta::get(tr.id).unwrap_or_default();
                let mut extra = String::new();
                if info.key.is_some() {
                    extra += &format!("  *  KEY {}", meta::key_text(&info));
                }
                if let Some(b) = info.bpm {
                    extra += &format!("  *  {} BPM", b);
                }
                format!("{} - {} ({}){}", tr.artists_text(), tr.title, fmt_time(tr.duration), extra)
            }
            (None, None) => "BACKLINE - A RETRO PLAYER FOR TIDAL".to_string(),
        };
        let marquee_r = pl.r(111.0, 24.0, 154.0, 6.0);
        let w = title.chars().count() as f32 * 5.0;
        let shift = if w > 154.0 && self.wa_hint.is_none() { ((ui.input(|i| i.time) as f32 * 22.0) % (w + 25.0)).floor() } else { 0.0 };
        let clip = p.with_clip_rect(marquee_r);
        winamp_ui::wa_text(&t, &clip, pl, 111.0 - shift, 24.0, &title);
        if w > 154.0 && shift > 0.0 {
            winamp_ui::wa_text(&t, &clip, pl, 111.0 - shift + w + 25.0, 24.0, &title);
        }
        self.wa_hint = None;
        if active {
            winamp_ui::wa_text(&t, &p, pl, 111.0, 43.0, &format!("{:>3}", self.kbps.min(999)));
            winamp_ui::wa_text(&t, &p, pl, 156.0, 43.0, &format!("{:>2}", (rate / 1000).min(99)));
        }
        t.spr(&p, "monoster", (29.0, 12.0, 27.0, 12.0), pl.r(212.0, 41.0, 27.0, 12.0));
        t.spr(&p, "monoster", (0.0, if active { 0.0 } else { 12.0 }, 29.0, 12.0), pl.r(239.0, 41.0, 29.0, 12.0));
        // the visualizer in the skin's own colours (tap: bars / wave / both)
        if hit(ui, "viz", pl.r(24.0, 43.0, 76.0, 16.0), false).clicked() {
            self.viz_mode = (self.viz_mode + 1) % 3;
        }
        if active {
            winamp_ui::skin_viz(&p, pl.o + Vec2::new(24.0, 43.0) * pl.s, pl.s, &t.vis, &self.bands, &self.peaks, &self.viz_wave, self.viz_mode);
        }
        // volume
        let vr = pl.r(107.0, 57.0, 68.0, 13.0);
        let frames = |sheet: &str| t.size(sheet).map_or(28.0, |s| ((s.y + 2.0) / 15.0).floor().clamp(1.0, 28.0));
        let frame = (self.volume * (frames("volume") - 1.0)).round();
        t.spr(&p, "volume", (0.0, frame * 15.0, 68.0, 13.0), vr);
        let vh = hit(ui, "volume", vr.expand2(Vec2::new(0.0, 6.0)), true);
        if t.size("volume").map_or(false, |s| s.y >= 433.0) {
            let src = if vh.dragged() { (0.0, 422.0, 14.0, 11.0) } else { (15.0, 422.0, 14.0, 11.0) };
            t.spr(&p, "volume", src, pl.r(107.0 + self.volume * 54.0, 58.0, 14.0, 11.0));
        }
        if vh.dragged() || vh.clicked() {
            if let Some(pp) = vh.interact_pointer_pos() {
                self.set_volume(((pp.x - vr.min.x - pl.s * 7.0) / (pl.s * 54.0)).clamp(0.0, 1.0));
            }
            self.wa_hint = Some(format!("VOLUME: {}%", (self.volume * 100.0).round()));
        }
        // balance (double-tap: centre)
        let br = pl.r(177.0, 57.0, 38.0, 13.0);
        let bal = if t.size("balance").is_some() { "balance" } else { "volume" };
        let bframe = (self.balance.unsigned_abs() as f32 / 100.0 * (frames(bal) - 1.0)).round();
        t.spr(&p, bal, (9.0, bframe * 15.0, 38.0, 13.0), br);
        let bh = hit(ui, "balance", br.expand2(Vec2::new(0.0, 6.0)), true);
        if t.size(bal).map_or(false, |s| s.y >= 433.0) {
            let src = if bh.dragged() { (0.0, 422.0, 14.0, 11.0) } else { (15.0, 422.0, 14.0, 11.0) };
            t.spr(&p, bal, src, pl.r(177.0 + (self.balance + 100) as f32 / 200.0 * 24.0, 58.0, 14.0, 11.0));
        }
        if bh.double_clicked() {
            self.balance = 0;
            self.player.ctl.set_balance(0);
        } else if bh.dragged() || bh.clicked() {
            if let Some(pp) = bh.interact_pointer_pos() {
                let f = ((pp.x - br.min.x - pl.s * 7.0) / (pl.s * 24.0)).clamp(0.0, 1.0);
                let b = (f * 200.0 - 100.0).round() as i32;
                self.balance = if b.abs() < 10 { 0 } else { b };
                self.player.ctl.set_balance(self.balance);
            }
            self.wa_hint = Some(match self.balance {
                0 => "BALANCE: CENTER".to_string(),
                b if b < 0 => format!("BALANCE: {}% LEFT", -b),
                b => format!("BALANCE: {}% RIGHT", b),
            });
        }
        // EQ (the equalizer, in PRACTICE > PITCH & EAR) and PL (the queue)
        let eq = pl.r(219.0, 58.0, 23.0, 12.0);
        let eqh = hit(ui, "eq", eq, false);
        let eq_on = self.practice && self.mtab == 2;
        let eq_src = match (eq_on, eqh.is_pointer_button_down_on()) {
            (false, false) => (0.0, 61.0),
            (true, false) => (0.0, 73.0),
            (false, true) => (46.0, 61.0),
            (true, true) => (46.0, 73.0),
        };
        t.spr(&p, "shufrep", (eq_src.0, eq_src.1, 23.0, 12.0), eq);
        if eqh.clicked() {
            self.practice = true;
            self.mtab = 2;
            self.sync_player();
        }
        let plr = pl.r(242.0, 58.0, 23.0, 12.0);
        let plh = hit(ui, "pl", plr, false);
        let pl_src = match (!self.practice, plh.is_pointer_button_down_on()) {
            (false, false) => (23.0, 61.0),
            (true, false) => (23.0, 73.0),
            (false, true) => (69.0, 61.0),
            (true, true) => (69.0, 73.0),
        };
        t.spr(&p, "shufrep", (pl_src.0, pl_src.1, 23.0, 12.0), plr);
        if plh.clicked() {
            self.practice = false;
            self.sync_player();
        }
        // the seek bar
        let pr = pl.r(16.0, 72.0, 248.0, 10.0);
        t.spr(&p, "posbar", (0.0, 0.0, 248.0, 10.0), pr);
        let ph = hit(ui, "seek", pr.expand2(Vec2::new(0.0, 5.0)), true);
        if active && dur > 0.0 {
            let f = (pos / dur).clamp(0.0, 1.0);
            let src = if ph.dragged() { (278.0, 0.0, 29.0, 10.0) } else { (248.0, 0.0, 29.0, 10.0) };
            t.spr(&p, "posbar", src, pl.r(16.0 + f * 219.0, 72.0, 29.0, 10.0));
            if ph.dragged() || ph.clicked() {
                if let Some(pp) = ph.interact_pointer_pos() {
                    self.seek_drag = Some(((pp.x - pr.min.x - 14.5 * pl.s) / (219.0 * pl.s)).clamp(0.0, 1.0) * dur);
                }
                let at = self.seek_drag.unwrap_or(pos);
                self.wa_hint = Some(format!("SEEK TO: {}/{} ({}%)", fmt_time(at), fmt_time(dur), (at / dur * 100.0).round()));
            }
            if ph.drag_stopped() || ph.clicked() {
                if let Some(sk) = self.seek_drag.take() {
                    self.player.send(Cmd::Seek(sk));
                }
            }
        }
        // transport
        for (name, x, sx, w) in [("prev", 16.0, 0.0, 23.0), ("play", 39.0, 23.0, 23.0), ("pause", 62.0, 46.0, 23.0), ("stop", 85.0, 69.0, 23.0), ("next", 108.0, 92.0, 22.0)] {
            let r = pl.r(x, 88.0, w, 18.0);
            let h = hit(ui, name, r, false);
            let down = if h.is_pointer_button_down_on() { 18.0 } else { 0.0 };
            t.spr(&p, "cbuttons", (sx, down, w, 18.0), r);
            if h.clicked() {
                match name {
                    "prev" => self.prev(),
                    "play" if self.paused || self.stopped || self.at_end => self.play_pause(),
                    "pause" if !self.paused && !self.stopped => self.play_pause(),
                    "stop" => self.stop(),
                    "next" => self.next(),
                    _ => {}
                }
            }
        }
        // eject: the library
        let ej = pl.r(136.0, 89.0, 22.0, 16.0);
        let ejh = hit(ui, "eject", ej, false);
        t.spr(&p, "cbuttons", (114.0, if ejh.is_pointer_button_down_on() { 16.0 } else { 0.0 }, 22.0, 16.0), ej);
        if ejh.clicked() {
            self.tab = Tab::Library;
        }
        // shuffle and repeat
        let sh = pl.r(164.0, 89.0, 47.0, 15.0);
        let shh = hit(ui, "shuffle", sh, false);
        let sy = if self.shuffle { 30.0 } else { 0.0 } + if shh.is_pointer_button_down_on() { 15.0 } else { 0.0 };
        t.spr(&p, "shufrep", (28.0, sy, 47.0, 15.0), sh);
        if shh.clicked() {
            self.shuffle = !self.shuffle;
        }
        let rp = pl.r(210.0, 89.0, 28.0, 15.0);
        let rph = hit(ui, "repeat", rp, false);
        let ry = if self.repeat != Repeat::Off { 30.0 } else { 0.0 } + if rph.is_pointer_button_down_on() { 15.0 } else { 0.0 };
        t.spr(&p, "shufrep", (0.0, ry, 28.0, 15.0), rp);
        if self.repeat == Repeat::One {
            p.rect_filled(pl.r(231.0, 90.0, 6.0, 7.0), 0.0, t.vis.first().copied().unwrap_or(Color32::BLACK));
            winamp_ui::wa_text(&t, &p, pl, 232.0, 90.5, "1");
        }
        if rph.clicked() {
            self.repeat = match self.repeat {
                Repeat::Off => Repeat::All,
                Repeat::All => Repeat::One,
                Repeat::One => Repeat::Off,
            };
        }
        self.wa_art = Some(t);
    }

    /// Fetch the song's cover once (Tidal's album art, or the clip's thumbnail).
    fn want_cover(&mut self, t: &Track) {
        let url = if let Some(y) = self.yt.iter().find(|y| yt_id(&y.vid) == t.id) {
            Some(format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", y.vid))
        } else {
            (!t.cover.is_empty()).then(|| api::cover_url(&t.cover, 160))
        };
        if self.cover.as_ref().map(|c| Some(&c.0)) == Some(url.as_ref()) {
            return;
        }
        self.cover = url.clone().map(|u| (u, None));
        if let Some(u) = url {
            self.spawn(move |api| {
                let img = api.fetch(&u).ok().and_then(|b| image::load_from_memory(&b).ok()).map(|i| {
                    let i = i.to_rgba8();
                    egui::ColorImage::from_rgba_unmultiplied([i.width() as usize, i.height() as usize], i.as_raw())
                });
                Msg::Cover(u, img)
            });
        }
    }

    /// The QUEUE window: the songs in line, numbered, the one playing marked (tap one to play it).
    fn queue_ui(&mut self, ui: &mut egui::Ui) {
        if self.queue.is_empty() {
            para(ui, "Nothing in line yet. Pick songs in the LIBRARY: the list you play from becomes the queue.", pal().ink2);
            return;
        }
        let mut play = None;
        let queue = self.queue.clone();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (i, t) in queue.iter().enumerate() {
                if song_row(ui, i, t, Some(i) == self.cur).clicked() {
                    play = Some(i);
                }
            }
        });
        if let Some(i) = play {
            self.play(i);
        }
    }
}

/// A list row in Backline's way: number, title and artist in the pixel font, the length at the right; the
/// alternate rows a shade apart, the one playing in the selection colour.
pub(crate) fn song_row(ui: &mut egui::Ui, i: usize, t: &Track, now: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H + 14.0), Sense::click());
    let p = ui.painter();
    if now {
        fill_rect(p, r, pal().row_sel);
    } else if i % 2 == 1 {
        fill_rect(p, r, pal().row_alt);
    }
    let (ink, ink2) = if now { (pal().bar_txt, pal().bar_txt) } else { (pal().ink, pal().ink2) };
    let num = format!("{}", i + 1);
    ptext(p, Pos2::new(r.min.x + 22.0, r.min.y + 11.0), Align::Max, &num, 1.0, ink2);
    let right = if t.duration > 0.0 { fmt_time(t.duration) } else { String::new() };
    let rw = text_w(&right, 1.0);
    ptext(p, Pos2::new(r.max.x - 6.0, r.min.y + 11.0), Align::Max, &right, 1.0, ink2);
    let w = r.width() - 40.0 - rw;
    ptext_fit(p, Pos2::new(r.min.x + 30.0, r.min.y + 11.0), Align::Min, &fit(&t.title.to_uppercase(), 2.0, w), 2.0, w, ink);
    let artist = t.artists_text().to_uppercase();
    ptext_fit(p, Pos2::new(r.min.x + 30.0, r.min.y + 27.0), Align::Min, &fit(&artist, 1.0, w), 1.0, w, ink2);
    resp
}
