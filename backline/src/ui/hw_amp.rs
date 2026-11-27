//! Hardware's Now Playing as an amp (the owner's picks from four style sheets, 2026-11-25): at the left the album's
//! cover standing on the floor and a reel-to-reel deck playing the song (`hw_deck.rs`, 2026-11-27); at the right a
//! black amp: a black panel with two VU meters, a jewel light, STANDBY and cream knobs that work (volume, bass,
//! treble, balance, speed), and DISPLAY push buttons; under it a second black panel showing the meters (tuning,
//! stereo, loudness) and a 32-band analyzer, the analyzer as light behind smoked glass, the lyrics through dark cloth,
//! Up Next, the credits or the song's loops on an amber screen. Practising, it holds the practice tools.
//!
//! Everything is laid out in the style sheet's own units (the amp at 300..936 by 34..538, the deck and the cover to its
//! left from `hw_deck::GEAR_X0`) and scaled to fit, so the gear keeps its shape at any window size.

use super::*;

/// The style sheet's units placed on screen: `o` is where its (12, 34) lands (the gear's top left), `s` how big one
/// unit is.
#[derive(Clone, Copy)]
pub(crate) struct Mu {
    pub(crate) o: Pos2,
    pub(crate) s: f32,
}

impl Mu {
    pub(crate) fn p(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(self.o.x + (x - 12.0) * self.s, self.o.y + (y - 34.0) * self.s)
    }
    pub(crate) fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.p(x, y), Vec2::new(w * self.s, h * self.s))
    }
    pub(crate) fn v(&self, n: f32) -> f32 {
        n * self.s
    }
}

pub(crate) const INK: Color32 = Color32::from_rgb(239, 226, 194);
pub(crate) const FACE_INK: Color32 = Color32::from_rgb(35, 29, 20);
const FACE_SOFT: Color32 = Color32::from_rgb(90, 76, 54);
const FACE_RED: Color32 = Color32::from_rgb(179, 39, 28);

/// What shows under the panel (the DISPLAY buttons, two rows of three: the amp's own, then what the other looks keep
/// in Now Playing's side panel).
const SHOW_NAMES: [&str; 6] = ["METERS", "LIGHT", "LYRICS", "UP NEXT", "CREDITS", "LOOPS"];

impl App {
    /// The amp, fitted into `r`.
    pub(crate) fn np_amp(&mut self, ui: &mut egui::Ui, r: Rect, tex: Option<egui::TextureId>, dt: f32, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        let now = ui.input(|i| i.time);
        // (the whole row of gear, from the cover's left edge to the amp's right, centred)
        let (gx0, gx1) = (crate::hw_deck::GEAR_X0, 936.0);
        let s = (r.width() / (gx1 - gx0 + 16.0)).min(r.height() / 520.0).max(0.3);
        let m = Mu { o: r.center() - Vec2::new(((gx0 + gx1) / 2.0 - 12.0) * s, 252.0 * s), s };
        self.hw_textures(ui.ctx());
        self.hw_measure(dt, now);
        let playing = self.cur.is_some() && !self.stopped && !self.paused;
        if playing || self.vu.iter().any(|v| *v > 0.002) {
            ui.ctx().request_repaint();
        }

        // ---- the room: the gear stands on a floor, each piece's shadow under it
        let floor_y = m.p(0.0, 538.0).y;
        let whole = ui.max_rect();
        vgrad(&p, Rect::from_min_max(Pos2::new(whole.min.x, floor_y - m.v(30.0)), whole.max), Color32::TRANSPARENT, Color32::from_black_alpha(120));
        for (x0, x1) in crate::hw_deck::FEET.iter().copied().chain([(300.0, 936.0)]) {
            let (a, b) = (m.p(x0, 538.0), m.p(x1, 538.0));
            shadow(&p, Pos2::new((a.x + b.x) / 2.0, a.y + m.v(4.0)), (b.x - a.x) * 0.56, m.v(14.0), 0.75);
        }

        // ---- at the left: the album's cover standing on the floor, and the reel-to-reel deck playing the song
        self.hw_cover(ui, &p, m, tex, acts);
        self.hw_deck(ui, &p, m, dt, acts);

        // ---- the amp's cabinet: black tolex
        cabinet(&p, m, m.r(300.0, 34.0, 636.0, 504.0), self.hw_tex.as_ref().map(|t| t[0].id()));

        // ---- practising: one tall black panel with the practice layout every style shares (tabs and all)
        if practice_tools() && self.practice {
            let pan = m.r(314.0, 48.0, 608.0, 478.0);
            black_panel(&p, m, pan);
            let body = pan.shrink(m.v(14.0));
            tour::mark("ARTPRACTICE", body);
            self.practice_stack(ui, body, acts, true);
            return;
        }

        // ---- the panel
        let pan = m.r(314.0, 48.0, 608.0, 230.0);
        black_panel(&p, m, pan);
        let pan_at = |x: f32, y: f32| m.p(314.0 + x, 48.0 + y);
        for (c, name) in ["LEFT", "RIGHT"].iter().enumerate() {
            let b = m.r(314.0 + 14.0 + 248.0 * c as f32, 58.0, 236.0, 140.0);
            bezel(&p, m, b, 5.0, true);
            let win = Rect::from_min_max(b.min + Vec2::new(m.v(11.0), m.v(9.0)), b.max - Vec2::new(m.v(11.0), m.v(9.0)));
            vu_face(&p, win, self.vu[c], name, now < self.hw_peak[c]);
        }
        // the jewel, lit while playing; STANDBY pauses
        let jc = pan_at(553.0, 46.0);
        jewel(&p, jc, m.v(12.0), playing);
        let tg = pan_at(553.0, 112.0);
        let th = ui
            .interact(Rect::from_center_size(tg, Vec2::new(m.v(30.0), m.v(40.0))), ui.id().with("hw_standby"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip(if playing { "Standby: pause" } else { "Play" });
        toggle(&p, m, tg, playing);
        caps(&p, pan_at(553.0, 136.0), egui::Align2::CENTER_TOP, "STANDBY", m.v(8.5), m.v(1.5), INK, true);
        if th.clicked() && self.cur.is_some() {
            acts.push(Action::Toggle);
        }
        // the knobs
        let ky = 195.0;
        let kx = [46.0, 118.0, 190.0, 262.0, 334.0, 406.0, 540.0];
        jacks(&p, m, pan_at(kx[0], ky));
        let bass = self.eq_gains[0];
        let treble = self.eq_gains[9];
        let knobs: [(&str, f32, u8); 5] = [
            ("VOLUME", 1.0 + 9.0 * self.volume.clamp(0.0, 1.0), 0),
            ("BASS", 1.0 + 9.0 * ((bass + 12.0) / 24.0), 0),
            ("TREBLE", 1.0 + 9.0 * ((treble + 12.0) / 24.0), 0),
            ("BALANCE", 1.0 + 9.0 * ((self.balance as f32 + 100.0) / 200.0), 1),
            ("SPEED", 1.0 + 9.0 * ((self.speed as f32 - 50.0) / 100.0).clamp(0.0, 1.0), 0),
        ];
        for (k, (name, v, kind)) in knobs.iter().enumerate() {
            let c = pan_at(kx[k + 1], ky);
            caps(&p, Pos2::new(c.x, c.y - m.v(39.0)), egui::Align2::CENTER_TOP, name, m.v(8.5), m.v(1.5), INK, true);
            knob_ring(&p, m, c, *kind);
            let hit = Rect::from_center_size(c, Vec2::splat(m.v(44.0)));
            let tip = match k {
                0 => format!("Volume {}%", (self.volume * 100.0).round()),
                1 => format!("Bass {:+.0} dB (EQ low bands)", bass),
                2 => format!("Treble {:+.0} dB (EQ high bands)", treble),
                3 => match self.balance {
                    0 => "Balance: center".to_string(),
                    b if b < 0 => format!("Balance: {}% left", -b),
                    b => format!("Balance: {}% right", b),
                },
                _ => format!("Speed {}%", self.speed),
            };
            let h = ui
                .interact(hit, ui.id().with(("hw_knob", k)), Sense::click_and_drag())
                .on_hover_cursor(egui::CursorIcon::ResizeVertical)
                .tip(format!("{} · Drag to turn · Double-click to reset", tip));
            let wheel = if h.hovered() { ui.input(|i| i.raw_scroll_delta.y) } else { 0.0 };
            let step = -h.drag_delta().y / m.v(120.0) * 9.0 + wheel.signum() * if wheel != 0.0 { 0.25 } else { 0.0 };
            if h.dragged() || h.double_clicked() || step != 0.0 {
                let nv = if h.double_clicked() { if k == 0 { *v } else { 5.5 } } else { (v + step).clamp(1.0, 10.0) };
                let f = (nv - 1.0) / 9.0;
                match k {
                    0 => acts.push(Action::Volume(f)),
                    1 | 2 => self.hw_tone(k == 1, (f * 24.0 - 12.0).clamp(-12.0, 12.0)),
                    3 => acts.push(Action::Balance(((f * 200.0 - 100.0) / 5.0).round() as i32 * 5)),
                    _ => acts.push(Action::Speed(((50.0 + 100.0 * f) / 5.0).round() as u32 * 5)),
                }
            }
            if h.dragged() {
                let b = Rect::from_center_size(Pos2::new(c.x, c.y - m.v(58.0)), Vec2::new(text_w(&tip, 1.4) + 18.0, 24.0));
                p.rect_filled(b, 4.0, Color32::from_rgb(30, 31, 34));
                p.rect_stroke(b, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(58, 60, 64)));
                ptext(&p, b.center(), Align::Center, tip.split(" (").next().unwrap_or(&tip), 1.4, Color32::WHITE);
            }
            cream_knob(&p, m, c, *v);
        }
        // DISPLAY: six push buttons with lamps in two columns (LYRICS on a song with none says so behind the cloth)
        let prac = practice_tools() && self.practice;
        let showing = self.hw_show.min(5);
        let dc = pan_at(kx[6], ky);
        caps(&p, Pos2::new(dc.x, dc.y - m.v(39.0)), egui::Align2::CENTER_TOP, "DISPLAY", m.v(8.5), m.v(1.5), INK, true);
        for (i, name) in SHOW_NAMES.iter().enumerate() {
            let y = dc.y - m.v(19.0) + m.v(17.0) * (i % 3) as f32;
            let x = dc.x - m.v(58.0) + m.v(66.0) * (i / 3) as f32;
            let br = Rect::from_center_size(Pos2::new(x, y), Vec2::splat(m.v(13.0)));
            let row = Rect::from_min_max(Pos2::new(br.min.x - 2.0, br.min.y - 2.0), Pos2::new(x + m.v(58.0), br.max.y + 2.0));
            let tip = match i {
                0 => "Meters and analyzer",
                1 => "Analyzer light",
                2 => "Lyrics",
                3 => "Up Next",
                4 => "Credits: about the song, and who made it",
                _ => "The song's loops and bookmarks",
            };
            let h = ui.interact(row, ui.id().with(("hw_show", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
            let on = showing == i as u8 && !prac;
            push_button(&p, br, on, h.is_pointer_button_down_on());
            let col = if on { INK } else { INK.gamma_multiply(if h.hovered() { 0.8 } else { 0.5 }) };
            caps(&p, Pos2::new(br.max.x + m.v(4.0), y), egui::Align2::LEFT_CENTER, name, m.v(7.0), m.v(1.0), col, true);
            if h.clicked() {
                self.hw_show = i as u8;
                self.dirty = true;
                if prac {
                    acts.push(Action::TogglePractice);
                }
            }
        }

        // ---- under the panel: a second black panel
        let low = m.r(314.0, 290.0, 608.0, 236.0);
        black_panel(&p, m, low);
        let inner = Rect::from_min_max(low.min + Vec2::splat(m.v(3.0)), low.max - Vec2::splat(m.v(3.0)));
        match showing {
            1 => {
                let g = inner.shrink(m.v(10.0));
                glass(&p, m, g);
                // (the analyzer light only: the 3D visuals belong to the other looks; they pop out from the "..." menu)
                let bands = crate::viz::resample(&self.bands, 64, 1.0);
                light_columns(&p.with_clip_rect(g.shrink(1.0)), g.shrink(m.v(4.0)), &bands);
                glare(&p, m, g);
            }
            2 => {
                let w = inner.shrink(m.v(10.0));
                let pos = self.pos();
                self.hw_lyrics(ui, &p, m, w, dt, pos, acts);
            }
            3..=5 => {
                // (Up Next, the credits, the loops: on an amber screen in a bezel)
                let b = inner.shrink(m.v(10.0));
                bezel(&p, m, b, 4.0, false);
                let w = b.shrink(m.v(6.0));
                crate::styles::lcd(&p, w);
                match showing {
                    3 => self.hw_upnext(ui, &p, m, w, dt, acts),
                    4 => self.hw_credits_screen(ui, &p, m, w, dt, acts),
                    _ => self.hw_loops_screen(ui, &p, m, w, dt, acts),
                }
                glare(&p, m, w);
            }
            _ => {
                let o = inner.min;
                let at = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(o + Vec2::new(m.v(x), m.v(y)), Vec2::new(m.v(w), m.v(h)));
                let tips = ["Left/right balance", "Stereo correlation", "Loudness (LUFS)"];
                for (k, sub) in ["dB", "CORRELATION", "LUFS"].iter().enumerate() {
                    let b = at(14.0 + 202.0 * k as f32, 9.0, 170.0, 96.0);
                    bezel(&p, m, b, 4.0, false);
                    let win = b.shrink(m.v(6.0));
                    small_meter(&p, win, k, self.hw_need[k], sub);
                    ui.interact(b, ui.id().with(("hw_meter", k)), Sense::hover()).tip(tips[k]);
                }
                let strip = at(14.0, 114.0, 574.0, 107.0);
                bezel(&p, m, strip, 4.0, false);
                let lcd_r = strip.shrink(m.v(6.0));
                crate::styles::lcd(&p, lcd_r);
                let bands = crate::viz::resample(&self.bands, 32, 1.0);
                let peaks = crate::viz::resample(&self.peaks, 32, 1.0);
                led_bars(&p, m, lcd_r, &bands, &peaks);
            }
        }
    }

    /// Up Next on the amp's screen: every song after this one, each with its cover, the title over the artist and the
    /// length; the wheel scrolls them (coming to rest on a whole song), a click plays one.
    fn hw_upnext(&mut self, ui: &mut egui::Ui, p: &egui::Painter, m: Mu, w: Rect, dt: f32, acts: &mut Vec<Action>) {
        let s = m.s;
        let col = crate::styles::amber();
        let from = self.cur.map_or(0, |c| c + 1);
        let ahead = self.queue.len().saturating_sub(from);
        if ahead == 0 {
            crate::styles::screen_text(p, w.center(), Align::Center, "Nothing after this song", 1.3 * s, w.width() - m.v(20.0), col.gamma_multiply(0.7));
            return;
        }
        if self.hw_q_from != from {
            // (a new song: back to the one after it)
            self.hw_q_from = from;
            self.hw_q_want = 0.0;
            self.hw_q_at = 0.0;
        }
        let rh = m.v(36.0);
        let list = w.shrink2(Vec2::new(m.v(6.0), m.v(6.0)));
        let most = (ahead as f32 - list.height() / rh).max(0.0);
        let (mut want, mut at) = (self.hw_q_want, self.hw_q_at);
        ease_scroll(ui, &mut want, &mut at, most, rh, w, dt);
        (self.hw_q_want, self.hw_q_at) = (want, at);
        let first = at.floor() as usize;
        let lp = p.with_clip_rect(list);
        let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(list));
        cu.set_clip_rect(list);
        let rows: Vec<(usize, Track)> = self.queue.iter().cloned().enumerate().skip(from + first).take((list.height() / rh) as usize + 2).collect();
        for (k, (i, tr)) in rows.iter().enumerate() {
            let y = list.min.y + rh * ((first + k) as f32 - at);
            if y > list.max.y || y + rh < list.min.y {
                continue;
            }
            let row = Rect::from_min_size(Pos2::new(list.min.x, y), Vec2::new(list.width() - m.v(8.0), rh));
            let h = ui.interact(row.intersect(list), ui.id().with(("hw_next", *i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Play");
            if h.hovered() {
                lp.rect_filled(row, m.v(2.0), col.gamma_multiply(0.1));
            }
            let cs = rh - m.v(8.0);
            let cr = Rect::from_min_size(Pos2::new(row.min.x + m.v(4.0), row.center().y - cs / 2.0), Vec2::splat(cs));
            let url = if tr.cover.is_empty() { String::new() } else { format!("thumb:{}", crate::api::cover_url(&tr.cover, 80)) };
            crate::row_cover(&cu, cr, &url);
            let len = fmt_time(tr.duration);
            let tw = crate::styles::screen_w(&lp, &len, 1.2 * s);
            let tx = cr.max.x + m.v(10.0);
            let room = row.max.x - tx - tw - m.v(16.0);
            crate::styles::screen_text(&lp, Pos2::new(tx, row.center().y - m.v(7.0)), Align::Min, &tr.title, 1.25 * s, room, col);
            crate::styles::screen_text(&lp, Pos2::new(tx, row.center().y + m.v(8.0)), Align::Min, &tr.artists_text(), 1.05 * s, room, col.gamma_multiply(0.6));
            crate::styles::screen_text(&lp, Pos2::new(row.max.x - m.v(6.0), row.center().y), Align::Max, &len, 1.2 * s, tw + 4.0, col);
            if h.clicked() {
                acts.push(Action::PlayIndex(*i));
            }
        }
        scroll_thumb(p, m, w, list, at, most, ahead as f32, rh);
    }

    /// The credits on the amp's screen: the song's details as the other looks' About has them, then who made it
    /// (Tidal's credits for a Tidal song, read once a song while this shows).
    fn hw_credits_screen(&mut self, ui: &mut egui::Ui, p: &egui::Painter, m: Mu, w: Rect, dt: f32, _acts: &mut Vec<Action>) {
        let s = m.s;
        let col = crate::styles::amber();
        let Some(tr) = self.cur_track() else {
            crate::styles::screen_text(p, w.center(), Align::Center, "Nothing playing", 1.3 * s, w.width() - m.v(20.0), col.gamma_multiply(0.7));
            return;
        };
        if tr.id > 0 && !self.go_offline && self.hw_credits.as_ref().is_none_or(|c| c.0 != tr.id) {
            let id = tr.id;
            self.hw_credits = Some((id, None));
            self.bg(move |api| Msg::HwCredits(id, api.credits(id)));
        }
        let info = crate::meta::get(tr.id).unwrap_or_default();
        let mut rows: Vec<(String, String)> = vec![("SONG".into(), tr.title.clone()), ("ARTIST".into(), tr.artists_text())];
        if !tr.album.is_empty() {
            rows.push(("ALBUM".into(), tr.album.clone()));
        }
        rows.push(("LENGTH".into(), fmt_time(tr.duration)));
        if info.key.is_some() {
            rows.push(("KEY".into(), crate::meta::key_text(&info)));
        }
        if let Some(b) = info.bpm {
            rows.push(("TEMPO".into(), format!("{} BPM", b)));
        }
        if let Some(a4) = info.a4 {
            rows.push(("TUNING".into(), format!("A = {:.1} Hz", a4)));
        }
        // (where it plays from: the sources map knows a file or a clip even when no list of yours has it)
        let src = match (self.srcmap.get(&tr.id), self.ext_of(tr.id)) {
            (_, Some(e)) if e.kind == "sc" => "SoundCloud",
            (Some(crate::Src::Yt(_)), _) => "YouTube",
            (Some(crate::Src::File(_)), _) => "A file on this computer",
            (None, Some(e)) if e.kind == "yt" => "YouTube",
            (None, Some(_)) => "A file on this computer",
            (None, None) => "Tidal",
        };
        rows.push(("FROM".into(), src.to_string()));
        match self.hw_credits.as_ref().filter(|c| c.0 == tr.id).map(|c| &c.1) {
            Some(None) => rows.push(("CREDITS".into(), "Loading...".into())),
            Some(Some(Ok(list))) => rows.extend(list.iter().map(|(role, names)| (role.to_uppercase(), names.clone()))),
            _ => {}
        }
        // (two columns of rows, the label dim at the left of each; the wheel scrolls when there are more)
        let rh = m.v(24.0);
        let list = w.shrink2(Vec2::new(m.v(14.0), m.v(8.0)));
        let cols = 2usize;
        let per = rows.len().div_ceil(cols);
        let fit = (list.height() / rh).floor().max(1.0);
        let most = (per as f32 - fit).max(0.0);
        let (mut want, mut at) = (self.hw_c_want, self.hw_c_at);
        ease_scroll(ui, &mut want, &mut at, most, rh, w, dt);
        (self.hw_c_want, self.hw_c_at) = (want, at);
        let lp = p.with_clip_rect(list);
        let cw = (list.width() - m.v(20.0)) / cols as f32;
        let lab_w = m.v(78.0);
        for (k, (lab, val)) in rows.iter().enumerate() {
            let (c, r) = (k / per, k % per);
            let y = list.min.y + rh * (r as f32 - at) + rh / 2.0;
            if y < list.min.y - rh || y > list.max.y + rh {
                continue;
            }
            let x = list.min.x + (cw + m.v(20.0)) * c as f32;
            crate::styles::screen_text(&lp, Pos2::new(x, y), Align::Min, lab, 0.95 * s, lab_w - m.v(6.0), col.gamma_multiply(0.55));
            crate::styles::screen_text(&lp, Pos2::new(x + lab_w, y), Align::Min, val, 1.15 * s, cw - lab_w, col);
        }
        scroll_thumb(p, m, w, list, at, most, per as f32, rh);
    }

    /// The song's loops and bookmarks on the amp's screen: a click loops one (or goes to the bookmark).
    fn hw_loops_screen(&mut self, ui: &mut egui::Ui, p: &egui::Painter, m: Mu, w: Rect, dt: f32, acts: &mut Vec<Action>) {
        let s = m.s;
        let col = crate::styles::amber();
        let Some(tr) = self.cur_track() else { return };
        let loops = self.store.sections.get(&tr.id).cloned().unwrap_or_default();
        let marks = self.store.marks.get(&tr.id).cloned().unwrap_or_default();
        if loops.is_empty() && marks.is_empty() {
            crate::styles::screen_text(p, w.center(), Align::Center, "No loops or bookmarks yet. Make them in Practice.", 1.2 * s, w.width() - m.v(20.0), col.gamma_multiply(0.7));
            return;
        }
        // (each loop: its name and from - to; then each bookmark: its name and where)
        let mut rows: Vec<(String, String, Action)> = loops
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let name = if l.name.is_empty() { format!("Loop {}", i + 1) } else { l.name.clone() };
                (name, format!("{} - {}", fmt_t(l.a), fmt_t(l.b)), Action::GoSection(i))
            })
            .collect();
        rows.extend(marks.iter().map(|b| {
            let name = if b.name.is_empty() { "Bookmark".to_string() } else { b.name.clone() };
            (name, fmt_t(b.t), Action::Seek(b.t))
        }));
        let rh = m.v(30.0);
        let list = w.shrink2(Vec2::new(m.v(8.0), m.v(6.0)));
        let most = (rows.len() as f32 - list.height() / rh).max(0.0);
        let (mut want, mut at) = (self.hw_l_want, self.hw_l_at);
        ease_scroll(ui, &mut want, &mut at, most, rh, w, dt);
        (self.hw_l_want, self.hw_l_at) = (want, at);
        let lp = p.with_clip_rect(list);
        let nl = loops.len();
        for (k, (name, when, act)) in rows.into_iter().enumerate() {
            let y = list.min.y + rh * (k as f32 - at);
            if y > list.max.y || y + rh < list.min.y {
                continue;
            }
            let row = Rect::from_min_size(Pos2::new(list.min.x, y), Vec2::new(list.width() - m.v(8.0), rh));
            let h = ui
                .interact(row.intersect(list), ui.id().with(("hw_loop", k)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .tip(if k < nl { "Loop it" } else { "Go there" });
            if h.hovered() {
                lp.rect_filled(row, m.v(2.0), col.gamma_multiply(0.1));
            }
            let tw = crate::styles::screen_w(&lp, &when, 1.15 * s);
            // (a bookmark marked with a small flag before its name)
            let x = row.min.x + m.v(8.0);
            let name = if k < nl { name } else { format!("> {}", name) };
            crate::styles::screen_text(&lp, Pos2::new(x, row.center().y), Align::Min, &name, 1.2 * s, row.width() - tw - m.v(30.0), col);
            crate::styles::screen_text(&lp, Pos2::new(row.max.x - m.v(6.0), row.center().y), Align::Max, &when, 1.15 * s, tw + 4.0, col.gamma_multiply(0.7));
            if h.clicked() {
                acts.push(act);
            }
        }
        scroll_thumb(p, m, w, list, at, most, (most + list.height() / rh).max(1.0), rh);
    }

    /// Bass or treble: the equalizer's three lowest (or highest) bands turned together, like an amp's tone knob.
    fn hw_tone(&mut self, bass: bool, g: f32) {
        let g = (g * 2.0).round() / 2.0;
        let w = [1.0, 0.75, 0.4];
        for (k, wk) in w.iter().enumerate() {
            let i = if bass { k } else { 9 - k };
            self.eq_gains[i] = (g * wk * 2.0).round() / 2.0;
        }
        self.eq_on = true;
        self.apply_eq();
        self.eq_remember();
        self.dirty = true;
    }

    /// The amp's textures, made once: the tolex's fine grain and the dark cloth's weave.
    fn hw_textures(&mut self, ctx: &egui::Context) {
        if self.hw_tex.is_some() {
            return;
        }
        let tolex = {
            const N: usize = 128;
            let mut px = vec![Color32::TRANSPARENT; N * N];
            let mut seed: u32 = 12345;
            for v in px.iter_mut() {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let r = (seed >> 24) as u8;
                *v = if r > 200 {
                    Color32::from_rgba_premultiplied(10, 10, 9, 10)
                } else if r < 70 {
                    Color32::from_rgba_premultiplied(0, 0, 0, 60)
                } else {
                    Color32::TRANSPARENT
                };
            }
            egui::ColorImage { size: [N, N], pixels: px }
        };
        let cloth = weave([62.0, 60.0, 57.0], [36.0, 35.0, 33.0]);
        self.hw_tex = Some([ctx.load_texture("hw_tolex", tolex, egui::TextureOptions::LINEAR_REPEAT), ctx.load_texture("hw_cloth", cloth, egui::TextureOptions::LINEAR_REPEAT)]);
    }

    /// What the meters show, measured from the last moment of sound: the VU levels (a VU's ballistics, 0 VU set to the
    /// song's own level), the peak lights (only when the record really clips), which side is louder, how alike the two
    /// sides are (correlation) and the loudness (BS.1770's K-weighting, eased over about 400 ms, as a momentary
    /// loudness meter).
    fn hw_measure(&mut self, dt: f32, now: f64) {
        let playing = self.cur.is_some() && !self.stopped && !self.paused;
        let song = self.cur_track().map_or(0, |t| t.id);
        if self.hw_ref.0 != song {
            self.hw_ref = (song, 0.0, 0);
        }
        let (mut lv, mut corr, mut ms, mut bal) = ([0.0f32; 2], None, 0.0f32, None);
        if playing {
            if let Ok(v) = self.player.viz.lock() {
                let rate = v.rate.max(8000) as f32;
                let n = v.samples.len().min(v.first.len());
                let (l, r) = (&v.first[v.first.len() - n..], &v.samples[v.samples.len() - n..]);
                let w = n.min((rate * 0.05) as usize);
                let (lw, rw) = (&l[n - w..], &r[n - w..]);
                if w > 0 {
                    let (mut sl, mut sr, mut slr) = (0.0f32, 0.0f32, 0.0f32);
                    // clipping: three samples in a row at full scale (a loud master touching 0 dBFS once isn't)
                    let (mut run, mut clip) = ([0u32; 2], [false; 2]);
                    for (a, b) in lw.iter().zip(rw) {
                        sl += a * a;
                        sr += b * b;
                        slr += a * b;
                        for (c, x) in [a, b].into_iter().enumerate() {
                            if x.abs() >= 0.999 {
                                run[c] += 1;
                                clip[c] |= run[c] >= 3;
                            } else {
                                run[c] = 0;
                            }
                        }
                    }
                    lv = [(sl / w as f32).sqrt(), (sr / w as f32).sqrt()];
                    if sl * sr > 1e-9 {
                        corr = Some((slr / (sl * sr).sqrt()).clamp(-1.0, 1.0));
                    }
                    if sl + sr > w as f32 * 1e-6 {
                        bal = Some((10.0 * (sr.max(1e-12) / sl.max(1e-12)).log10()).clamp(-14.0, 14.0));
                    }
                    for (c, on) in clip.into_iter().enumerate() {
                        if on {
                            self.hw_peak[c] = now + 0.5;
                        }
                    }
                    // the song's own level so far (both sides; silence and quiet moments left out)
                    let msw = (sl + sr) / (2.0 * w as f32);
                    let (_, sum, cnt) = self.hw_ref;
                    let mean = if cnt > 0 { sum / cnt as f64 } else { 0.0 };
                    if msw > 1e-5 && (cnt < 20 || msw as f64 > mean * 0.01) {
                        self.hw_ref.1 += msw as f64;
                        self.hw_ref.2 += 1;
                    }
                }
                if n > 1024 {
                    ms = k_weighted_ms(l, rate) + k_weighted_ms(r, rate);
                }
            }
        }
        // 0 VU: 3 dB over the song's average level, as a deck's level is set for what it records (records are mastered
        // 8 to 12 dB hotter than the studio's -18 dBFS, which kept the needles in the red); the studio's level until
        // there's anything to go by
        let refa = if self.hw_ref.2 > 0 { (self.hw_ref.1 / self.hw_ref.2 as f64).sqrt() as f32 * 10f32.powf(3.0 / 20.0) } else { 10f32.powf(-18.0 / 20.0) };
        let k = 1.0 - (-dt / 0.065).exp();
        for (c, l) in lv.iter().enumerate() {
            self.vu[c] += (l / refa - self.vu[c]) * k;
        }
        let ease = |cur: &mut f32, to: f32, tau: f32| *cur += (to - *cur) * (1.0 - (-dt / tau).exp());
        if let Some(c) = corr {
            ease(&mut self.hw_corr, c, 0.15);
        }
        ease(&mut self.hw_ms, ms, 0.4);
        let lufs = if self.hw_ms > 1e-10 { -0.691 + 10.0 * self.hw_ms.log10() } else { -60.0 };
        if let Some(b) = bal {
            ease(&mut self.hw_need[0], b, 0.3);
        } else if !playing {
            ease(&mut self.hw_need[0], 0.0, 0.3);
        }
        ease(&mut self.hw_need[1], if playing { self.hw_corr } else { -1.2 }, 0.15);
        ease(&mut self.hw_need[2], lufs.max(-40.0), 0.1);
    }

    /// The lyrics through dark cloth: every line the same size, the one being sung bright and the rest dim, scrolling
    /// so the sung line stays in the middle; the wheel scrolls them a while, a click on a line jumps there.
    #[allow(clippy::too_many_arguments)]
    fn hw_lyrics(&mut self, ui: &mut egui::Ui, p: &egui::Painter, m: Mu, w: Rect, dt: f32, pos: f32, acts: &mut Vec<Action>) {
        p.rect_filled(w, m.v(3.0), Color32::from_rgb(10, 9, 8));
        let cp = p.with_clip_rect(w);
        let size = m.v(19.0);
        let lh = size * 1.9;
        let cy = w.center().y;
        let hover = ui.input(|i| i.pointer.hover_pos()).filter(|pp| w.contains(*pp));
        let wheel = if hover.is_some() { ui.input(|i| i.raw_scroll_delta.y) } else { 0.0 };
        let warm = Color32::from_rgb(255, 236, 204);
        // (no words for this song, or not here yet: said behind the cloth, where the words would be)
        let track_id = self.cur_track().map(|t| t.id);
        let msg = match self.lyrics.as_ref() {
            None if track_id.is_none() => Some("LYRICS SHOW HERE"),
            None => Some("LOADING LYRICS..."),
            Some(l) if track_id != Some(l.id) || l.status == 0 => Some("LOADING LYRICS..."),
            Some(l) if l.status == 2 || l.lines.is_empty() => Some("NO LYRICS FOR THIS SONG"),
            _ => None,
        };
        if let Some(msg) = msg {
            let at = w.center();
            for (dx, dy) in [(-1.5, 0.0), (1.5, 0.0), (0.0, -1.5), (0.0, 1.5)] {
                cp.text(at + Vec2::new(dx, dy), egui::Align2::CENTER_CENTER, msg, egui::FontId::proportional(size * 0.8), add(Color32::from_rgb(255, 160, 60), 0.1));
            }
            cp.text(at, egui::Align2::CENTER_CENTER, msg, egui::FontId::proportional(size * 0.8), warm.gamma_multiply(0.75));
            if let Some(t) = &self.hw_tex {
                tiled(&cp, w, t[1].id(), m.v(32.0), Color32::from_white_alpha(210));
            }
            p.rect_stroke(w, m.v(3.0), Stroke::new(1.0_f32, Color32::from_black_alpha(220)));
            return;
        }
        let Some(l) = self.lyrics.as_ref() else { return };
        let n = l.lines.len();
        let idx: Option<usize> = if l.synced { l.lines.iter().rposition(|(tt, _)| *tt <= pos + 0.25) } else { None };
        self.lyric_free = (self.lyric_free - dt).max(0.0);
        let mut scroll = self.lyric_scroll;
        if wheel != 0.0 {
            scroll = (scroll - wheel / lh).within(0.0, (n.max(1) - 1) as f32);
            self.lyric_free = 4.0;
        } else if l.synced && self.lyric_free <= 0.0 {
            scroll += (idx.unwrap_or(0) as f32 - scroll) * (1.0 - (-dt * 7.0).exp());
        }
        let mut hover_i = None;
        if let (true, Some(pp)) = (l.synced, hover) {
            let k = (scroll + (pp.y - cy) / lh).round();
            if k >= 0.0 && (k as usize) < n && !l.lines[k as usize].1.is_empty() {
                hover_i = Some(k as usize);
            }
        }
        let lr = ui.interact(w, ui.id().with("hw_lyrics"), Sense::click());
        if hover_i.is_some() {
            lr.clone().on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        if let (true, Some(i)) = (lr.clicked(), hover_i) {
            acts.push(Action::Seek(l.lines[i].0));
            self.lyric_free = 0.0;
        }
        let mut lit: Vec<(Pos2, String, f32)> = Vec::new();
        for (i, (_, txt)) in l.lines.iter().enumerate() {
            if txt.is_empty() {
                continue;
            }
            let y = cy + (i as f32 - scroll) * lh;
            if y < w.min.y - lh || y > w.max.y + lh {
                continue;
            }
            let edge = (y - w.min.y).min(w.max.y - y) / (w.height() * 0.22);
            let ef = edge.clamp(0.0, 1.0);
            let a = if idx == Some(i) || !l.synced { 1.0 } else if hover_i == Some(i) { 0.7 } else { 0.3 } * ef * ef * (3.0 - 2.0 * ef);
            lit.push((Pos2::new(w.center().x, y), txt.clone(), a));
        }
        // the words as light behind the cloth: a soft glow, the words, then the weave over them, then a little of
        // their light again on the threads
        for (at, txt, a) in &lit {
            for (dx, dy) in [(-1.5, 0.0), (1.5, 0.0), (0.0, -1.5), (0.0, 1.5)] {
                cp.text(*at + Vec2::new(dx, dy), egui::Align2::CENTER_CENTER, txt, egui::FontId::proportional(size), add(Color32::from_rgb(255, 160, 60), 0.18 * a));
            }
            cp.text(*at, egui::Align2::CENTER_CENTER, txt, egui::FontId::proportional(size), warm.gamma_multiply(*a));
            cp.text(*at + Vec2::new(0.6, 0.0), egui::Align2::CENTER_CENTER, txt, egui::FontId::proportional(size), warm.gamma_multiply(*a));
        }
        if let Some(t) = &self.hw_tex {
            tiled(&cp, w, t[1].id(), m.v(32.0), Color32::from_white_alpha(210));
        }
        for (at, txt, a) in &lit {
            cp.text(*at, egui::Align2::CENTER_CENTER, txt, egui::FontId::proportional(size), add(warm, 0.55 * a * a));
        }
        p.rect_stroke(w, m.v(3.0), Stroke::new(1.0_f32, Color32::from_black_alpha(220)));
        self.lyric_scroll = scroll;
    }
}

/// Hardware's Now Playing behind the gear: a dark room, its wall a shade warmer up high, and a soft warm light on it.
pub(crate) fn room(p: &egui::Painter, whole: Rect) {
    vgrad(p, whole, Color32::from_rgb(28, 25, 22), Color32::from_rgb(11, 10, 9));
    const SEG: usize = 48;
    const RINGS: usize = 10;
    let c = Pos2::new(whole.center().x, whole.min.y + whole.height() * 0.3);
    let (rx, ry) = (whole.width() * 0.5, whole.height() * 0.55);
    let warm = Color32::from_rgb(52, 36, 20);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(c, add(warm, 1.0));
    for ring in 1..=RINGS {
        let t = ring as f32 / RINGS as f32;
        let f = 1.0 - t * t * (3.0 - 2.0 * t);
        for k in 0..SEG {
            let a = k as f32 / SEG as f32 * std::f32::consts::TAU;
            mesh.colored_vertex(c + Vec2::new(a.cos() * rx * t, a.sin() * ry * t), add(warm, f));
        }
    }
    for k in 0..SEG {
        mesh.add_triangle(0, (1 + k) as u32, (1 + (k + 1) % SEG) as u32);
    }
    for ring in 1..RINGS {
        for k in 0..SEG {
            let a0 = (1 + (ring - 1) * SEG + k) as u32;
            let a1 = (1 + (ring - 1) * SEG + (k + 1) % SEG) as u32;
            mesh.add_triangle(a0, a1, a1 + SEG as u32);
            mesh.add_triangle(a0, a1 + SEG as u32, a0 + SEG as u32);
        }
    }
    p.add(mesh);
}

/// A colour added onto what's under it (premultiplied with no alpha), at `a` of its strength.
pub(crate) fn add(c: Color32, a: f32) -> Color32 {
    let k = a.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied((c.r() as f32 * k) as u8, (c.g() as f32 * k) as u8, (c.b() as f32 * k) as u8, 0)
}

/// Mean square of `x` through BS.1770's K-weighting (a high shelf, then a high-pass), with the coefficients worked out
/// for `rate` as libebur128 does; the first 512 samples only settle the filters.
fn k_weighted_ms(x: &[f32], rate: f32) -> f32 {
    use std::f64::consts::PI;
    let fs = rate as f64;
    let (f0, g, q) = (1681.974450955533f64, 3.999843853973347f64, 0.7071752369554196f64);
    let k = (PI * f0 / fs).tan();
    let vh = 10f64.powf(g / 20.0);
    let vb = vh.powf(0.4996667741545416);
    let a0 = 1.0 + k / q + k * k;
    let s = [(vh + vb * k / q + k * k) / a0, 2.0 * (k * k - vh) / a0, (vh - vb * k / q + k * k) / a0, 2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0];
    let (f1, q1) = (38.13547087602444f64, 0.5003270373238773f64);
    let k1 = (PI * f1 / fs).tan();
    let d = 1.0 + k1 / q1 + k1 * k1;
    let h = [1.0, -2.0, 1.0, 2.0 * (k1 * k1 - 1.0) / d, (1.0 - k1 / q1 + k1 * k1) / d];
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let (mut u1, mut u2, mut z1, mut z2) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let (mut sum, mut n) = (0.0f64, 0usize);
    for (i, &v) in x.iter().enumerate() {
        let v = v as f64;
        let y = s[0] * v + s[1] * x1 + s[2] * x2 - s[3] * y1 - s[4] * y2;
        x2 = x1;
        x1 = v;
        y2 = y1;
        y1 = y;
        let z = h[0] * y + h[1] * u1 + h[2] * u2 - h[3] * z1 - h[4] * z2;
        u2 = u1;
        u1 = y;
        z2 = z1;
        z1 = z;
        if i >= 512 {
            sum += z * z;
            n += 1;
        }
    }
    if n == 0 {
        0.0
    } else {
        (sum / n as f64) as f32
    }
}

/// A grille cloth's weave: two sets of threads crossing at 45 degrees, over and under, with small gaps for the light
/// behind (a 64-pixel tile that repeats seamlessly; the gaps are clear).
fn weave(a: [f32; 3], b: [f32; 3]) -> egui::ColorImage {
    const T: usize = 64;
    const P: f32 = 8.0;
    const W: f32 = 0.6;
    let shade = |f: f32| 0.7 + 0.36 * (std::f32::consts::PI * f / W).sin();
    let vary = |n: f32| {
        let s = (n * 12.9898).sin() * 43758.547;
        0.93 + 0.14 * (s - s.floor())
    };
    let mut px = vec![Color32::TRANSPARENT; T * T];
    for y in 0..T {
        for x in 0..T {
            let u = x as f32 + y as f32 + 0.5;
            let v = x as f32 - y as f32 + (T * 4) as f32 + 0.5;
            let (fu, fv) = ((u % P) / P, (v % P) / P);
            let (cu, cv) = ((u / P).floor(), (v / P).floor());
            let (in_a, in_b) = (fu < W, fv < W);
            let (col, sh) = if in_a && in_b {
                if (cu + cv) as i32 % 2 == 0 {
                    (a, shade(fu) * vary(cu))
                } else {
                    (b, shade(fv) * vary(cv + 50.0))
                }
            } else if in_a {
                (a, shade(fu) * vary(cu) * 0.9)
            } else if in_b {
                (b, shade(fv) * vary(cv + 50.0) * 0.9)
            } else {
                continue;
            };
            let c = |k: usize| (col[k] * sh).min(255.0) as u8;
            px[y * T + x] = Color32::from_rgb(c(0), c(1), c(2));
        }
    }
    egui::ColorImage { size: [T, T], pixels: px }
}

/// `id` repeated over `r`, one copy every `tile` points.
fn tiled(p: &egui::Painter, r: Rect, id: egui::TextureId, tile: f32, tint: Color32) {
    let mut mesh = egui::Mesh::with_texture(id);
    let uv = |q: Pos2| Pos2::new((q.x - r.min.x) / tile, (q.y - r.min.y) / tile);
    for q in [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()] {
        mesh.vertices.push(egui::epaint::Vertex { pos: q, uv: uv(q), color: tint });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(mesh);
}

/// A rectangle filled top to bottom from `top` to `bot` (sharp corners).
pub(crate) fn vgrad(p: &egui::Painter, r: Rect, top: Color32, bot: Color32) {
    let mut mesh = egui::Mesh::default();
    for (q, c) in [(r.left_top(), top), (r.right_top(), top), (r.right_bottom(), bot), (r.left_bottom(), bot)] {
        mesh.colored_vertex(q, c);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(mesh);
}

/// A circle shaded like a ball or a lamp: `stops` (place 0..1 from the bright spot out, colour) round a spot at
/// `hi` (a share of the radius, from the centre). Its rim fades out over one pixel, so the edge is smooth.
pub(crate) fn ball(p: &egui::Painter, c: Pos2, r: f32, hi: Vec2, stops: &[(f32, Color32)]) {
    const SEG: usize = 72;
    const RINGS: usize = 10;
    let col_at = |f: f32| -> Color32 {
        let f = f.clamp(0.0, 1.0);
        for w in stops.windows(2) {
            if f <= w[1].0 {
                let t = (f - w[0].0) / (w[1].0 - w[0].0).max(1e-4);
                return w[0].1.lerp_to_gamma(w[1].1, t);
            }
        }
        stops.last().map_or(Color32::WHITE, |s| s.1)
    };
    let spot = c + hi * r;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(spot, col_at(0.0));
    // the rings from the bright spot out to the rim (half a pixel in, where the feather starts)
    let rim = (r - 0.5).max(0.5);
    for ring in 1..=RINGS {
        let t = ring as f32 / RINGS as f32;
        for k in 0..SEG {
            let a = k as f32 / SEG as f32 * std::f32::consts::TAU;
            let edge = c + Vec2::new(a.cos(), a.sin()) * rim;
            mesh.colored_vertex(spot + (edge - spot) * t, col_at(t));
        }
    }
    // the feather: the rim's colour fading to nothing a pixel further out
    for k in 0..SEG {
        let a = k as f32 / SEG as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(c + Vec2::new(a.cos(), a.sin()) * (r + 0.5), Color32::TRANSPARENT);
    }
    for k in 0..SEG {
        let (a, b) = (1 + k, 1 + (k + 1) % SEG);
        mesh.add_triangle(0, a as u32, b as u32);
    }
    for ring in 1..=RINGS {
        for k in 0..SEG {
            let a0 = (1 + (ring - 1) * SEG + k) as u32;
            let a1 = (1 + (ring - 1) * SEG + (k + 1) % SEG) as u32;
            let (b0, b1) = (a0 + SEG as u32, a1 + SEG as u32);
            mesh.add_triangle(a0, a1, b1);
            mesh.add_triangle(a0, b1, b0);
        }
    }
    p.add(mesh);
}

/// Spaced capitals (the panel's lettering), `anchor`ed at `at`; `bold` draws them twice, a hair apart.
#[allow(clippy::too_many_arguments)]
pub(crate) fn caps(p: &egui::Painter, at: Pos2, anchor: egui::Align2, text: &str, size: f32, spacing: f32, color: Color32, bold: bool) {
    let mut job = egui::text::LayoutJob::default();
    job.append(text, 0.0, egui::TextFormat { font_id: egui::FontId::proportional(size), extra_letter_spacing: spacing, color, ..Default::default() });
    let g = p.layout_job(job);
    // (the spacing after the last letter isn't part of the word: centre and right-align without it)
    let sz = g.size() - Vec2::new(spacing, 0.0);
    let r = anchor.anchor_size(at, sz);
    p.galley(r.min, g.clone(), color);
    if bold {
        p.galley(r.min + Vec2::new((size * 0.045).max(0.4), 0.0), g, color);
    }
}

/// A cabinet in black tolex: its grain, a dark edge, a faint light along the top.
pub(crate) fn cabinet(p: &egui::Painter, m: Mu, r: Rect, grain: Option<egui::TextureId>) {
    p.rect_filled(r, m.v(12.0), Color32::from_rgb(29, 27, 25));
    if let Some(id) = grain {
        tiled(p, r.shrink(m.v(3.0)), id, m.v(64.0), Color32::from_white_alpha(255));
    }
    p.rect_stroke(r, m.v(12.0), Stroke::new(1.0_f32, Color32::from_black_alpha(200)));
}

/// A soft shadow on the floor: an oval `rx` by `ry` round `c`, darkest in the middle, fading to nothing.
pub(crate) fn shadow(p: &egui::Painter, c: Pos2, rx: f32, ry: f32, a: f32) {
    const SEG: usize = 48;
    const RINGS: usize = 8;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(c, Color32::from_black_alpha((a * 255.0) as u8));
    for ring in 1..=RINGS {
        let t = ring as f32 / RINGS as f32;
        let f = 1.0 - t * t * (3.0 - 2.0 * t);
        for k in 0..SEG {
            let ang = k as f32 / SEG as f32 * std::f32::consts::TAU;
            mesh.colored_vertex(c + Vec2::new(ang.cos() * rx * t, ang.sin() * ry * t), Color32::from_black_alpha((a * f * 255.0) as u8));
        }
    }
    for k in 0..SEG {
        mesh.add_triangle(0, (1 + k) as u32, (1 + (k + 1) % SEG) as u32);
    }
    for ring in 1..RINGS {
        for k in 0..SEG {
            let a0 = (1 + (ring - 1) * SEG + k) as u32;
            let a1 = (1 + (ring - 1) * SEG + (k + 1) % SEG) as u32;
            mesh.add_triangle(a0, a1, a1 + SEG as u32);
            mesh.add_triangle(a0, a1 + SEG as u32, a0 + SEG as u32);
        }
    }
    p.add(mesh);
}

/// The amp's black panel: a dark gradient, a soft sheen at the top, and a fine cream line just inside its edge.
fn black_panel(p: &egui::Painter, m: Mu, r: Rect) {
    p.rect_filled(r.translate(Vec2::new(0.0, 1.0)), m.v(4.0), Color32::from_black_alpha(160));
    p.rect_filled(r, m.v(4.0), Color32::from_rgb(28, 26, 24));
    vgrad(p, r.shrink(m.v(2.0)), Color32::from_rgb(37, 35, 33), Color32::from_rgb(21, 20, 19));
    p.rect_stroke(r.shrink(m.v(5.0)), m.v(2.0), Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(239, 226, 194, 60)));
}

/// A meter's frame: a dark bezel with a light top edge (and screws at its corners when `screws`).
pub(crate) fn bezel(p: &egui::Painter, m: Mu, r: Rect, rad: f32, screws: bool) {
    p.rect_filled(r.translate(Vec2::new(0.0, m.v(2.0))), m.v(rad), Color32::from_black_alpha(140));
    p.rect_filled(r, m.v(rad), Color32::from_rgb(30, 28, 26));
    vgrad(p, r.shrink(m.v(1.5)), Color32::from_rgb(48, 45, 42), Color32::from_rgb(18, 17, 16));
    if screws {
        for (k, q) in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()].into_iter().enumerate() {
            let c = Pos2::new(q.x + if k % 2 == 0 { m.v(5.5) } else { -m.v(5.5) }, q.y + if k < 2 { m.v(4.5) } else { -m.v(4.5) });
            let rr = m.v(3.5);
            ball(p, c, rr, Vec2::new(-0.3, -0.3), &[(0.0, Color32::from_rgb(244, 244, 244)), (0.6, Color32::from_rgb(154, 154, 154)), (1.0, Color32::from_rgb(90, 90, 90))]);
            let a = [0.35f32, 1.2, 0.8, 1.9][k];
            let d = Vec2::new(a.cos(), a.sin()) * rr * 0.75;
            p.line_segment([c - d, c + d], Stroke::new(m.v(0.9), Color32::from_rgb(58, 58, 58)));
        }
    }
}

/// The cream VU meter: its face, the dB scale (placed by voltage, as a real VU's: -20 to +3, red from 0) over a
/// percent scale, VU and the side's name, the small print, a peak light, the needle with its shadow, the pivot's cap,
/// and the glass's glare. `amp` is the level as a share of 0 VU's.
fn vu_face(p: &egui::Painter, win: Rect, amp: f32, name: &str, peak: bool) {
    let k = (win.width() / 240.0).max(win.height() / 140.0);
    let f = |x: f32, y: f32| Pos2::new(win.center().x + (x - 120.0) * k, win.center().y + (y - 70.0) * k);
    let cp = p.with_clip_rect(win);
    face(&cp, win, f(120.0, 140.0), 140.0 * k);
    let (px, py, rr) = (120.0, 162.0, 112.0);
    let full = 10f32.powf(3.0 / 20.0);
    let vpos = |db: f32| 10f32.powf(db / 20.0) / full;
    let ang = |q: f32| (-48.0 + 96.0 * q).to_radians();
    let pt = |a: f32, r: f32| f(px + r * a.sin(), py - r * a.cos());
    let arc = |r: f32, q0: f32, q1: f32| -> Vec<Pos2> { (0..=40).map(|i| pt(ang(q0 + (q1 - q0) * i as f32 / 40.0), r)).collect() };
    let z = vpos(0.0);
    let t = |px_: f32| px_ * k;
    // the percent scale under the dB one
    cp.add(egui::Shape::line(arc(rr - 16.0, 0.0, z), Stroke::new(t(0.7), FACE_SOFT)));
    for pc in (0..=100).step_by(20) {
        let q = pc as f32 / 100.0 * z;
        cp.line_segment([pt(ang(q), rr - 16.0), pt(ang(q), rr - 20.0)], Stroke::new(t(0.7), FACE_SOFT));
        cp.text(pt(ang(q), rr - 27.0), egui::Align2::CENTER_CENTER, pc.to_string(), egui::FontId::proportional(t(5.8)), FACE_SOFT);
    }
    // the dB scale: dark to 0, red past it, a thick red band over the red
    cp.add(egui::Shape::line(arc(rr, 0.0, z), Stroke::new(t(1.2), FACE_INK)));
    cp.add(egui::Shape::line(arc(rr, z, 1.0), Stroke::new(t(1.2), FACE_RED)));
    cp.add(egui::Shape::line(arc(rr + 3.2, z, 1.0), Stroke::new(t(5.0), FACE_RED)));
    for db in [-15.0f32, -6.0, -4.0, -2.5, -1.5, -0.5, 0.5, 1.5, 2.5] {
        let a = ang(vpos(db));
        cp.line_segment([pt(a, rr), pt(a, rr + 4.5)], Stroke::new(t(0.9), if db > 0.0 { FACE_RED } else { FACE_INK }));
    }
    for db in [-20.0f32, -10.0, -7.0, -5.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0] {
        let a = ang(vpos(db));
        let col = if db > 0.0 { FACE_RED } else { FACE_INK };
        cp.line_segment([pt(a, rr), pt(a, rr + 8.5)], Stroke::new(t(1.2), col));
        let lab = if db == 3.0 { "+3".to_string() } else { (db.abs() as i32).to_string() };
        cp.text(pt(a, rr + 17.0), egui::Align2::CENTER_CENTER, lab, egui::FontId::proportional(t(10.5)), col);
    }
    caps(&cp, f(120.0, 96.0), egui::Align2::CENTER_CENTER, "VU", t(20.0), t(1.0), Color32::from_rgb(42, 35, 24), true);
    caps(&cp, f(120.0, 113.0), egui::Align2::CENTER_CENTER, name, t(7.5), t(2.6), Color32::from_rgb(42, 35, 24), true);
    cp.text(f(16.0, 97.0), egui::Align2::CENTER_CENTER, "−", egui::FontId::proportional(t(19.0)), FACE_INK);
    cp.text(f(224.0, 97.0), egui::Align2::CENTER_CENTER, "+", egui::FontId::proportional(t(19.0)), FACE_RED);
    caps(&cp, f(9.0, 131.0), egui::Align2::LEFT_CENTER, "0 VU = SONG LEVEL", t(5.6), t(0.6), FACE_SOFT, false);
    caps(&cp, f(231.0, 131.0), egui::Align2::RIGHT_CENTER, "VOLUME UNITS", t(5.6), t(0.6), FACE_SOFT, false);
    // the peak light
    let pk = f(222.0, 15.0);
    if peak {
        ball(&cp, pk, t(9.0), Vec2::ZERO, &[(0.0, Color32::from_rgba_premultiplied(120, 30, 18, 0)), (1.0, Color32::TRANSPARENT)]);
    }
    cp.circle_filled(pk, t(4.2), if peak { Color32::from_rgb(255, 74, 51) } else { Color32::from_rgb(77, 18, 13) });
    cp.circle_stroke(pk, t(4.2), Stroke::new(t(0.8), Color32::from_rgb(42, 10, 6)));
    caps(&cp, f(222.0, 27.0), egui::Align2::CENTER_CENTER, "PEAK", t(5.6), t(0.6), FACE_SOFT, false);
    // the needle and its shadow
    let a = ang((amp / full).clamp(0.0, 1.06));
    let (n0, n1) = (pt(a, -4.0), pt(a, rr + 10.0));
    let sh = Vec2::new(t(2.2), t(2.8));
    cp.line_segment([n0 + sh, n1 + sh], Stroke::new(t(2.2), Color32::from_black_alpha(40)));
    cp.line_segment([n0 + sh, n1 + sh], Stroke::new(t(1.3), Color32::from_black_alpha(45)));
    cp.line_segment([n0, n1], Stroke::new(t(1.3).max(1.0), Color32::from_rgb(21, 18, 14)));
    // the pivot's cap, and the glass
    cap(&cp, f(120.0, 140.0), t(16.0));
}

/// A meter face's cream, brightest low in the middle (where its light sits).
fn face(p: &egui::Painter, r: Rect, c: Pos2, rad: f32) {
    const GX: usize = 16;
    const GY: usize = 10;
    let col = |q: Pos2| {
        let d = ((q - c).length() / rad).clamp(0.0, 1.0);
        let (a, b, cc) = (Color32::from_rgb(255, 246, 214), Color32::from_rgb(240, 221, 170), Color32::from_rgb(208, 180, 124));
        if d < 0.55 {
            a.lerp_to_gamma(b, d / 0.55)
        } else {
            b.lerp_to_gamma(cc, (d - 0.55) / 0.45)
        }
    };
    let mut mesh = egui::Mesh::default();
    for j in 0..=GY {
        for i in 0..=GX {
            let q = Pos2::new(r.min.x + r.width() * i as f32 / GX as f32, r.min.y + r.height() * j as f32 / GY as f32);
            mesh.colored_vertex(q, col(q));
        }
    }
    for j in 0..GY {
        for i in 0..GX {
            let a = (j * (GX + 1) + i) as u32;
            let b = a + GX as u32 + 1;
            mesh.add_triangle(a, a + 1, b + 1);
            mesh.add_triangle(a, b + 1, b);
        }
    }
    p.add(mesh);
    // (a little shade round the window's edge, as the face sits behind the bezel)
    p.rect_stroke(r.shrink(1.0), 2.0, Stroke::new(2.0_f32, Color32::from_black_alpha(26)));
}

/// The half-round cap over a needle's pivot at the foot of a face.
fn cap(p: &egui::Painter, c: Pos2, r: f32) {
    let pts: Vec<Pos2> = (0..=24).map(|i| {
        let a = std::f32::consts::PI * (1.0 + i as f32 / 24.0);
        Pos2::new(c.x + r * a.cos(), c.y + r * a.sin())
    }).collect();
    p.add(egui::Shape::convex_polygon(pts.clone(), Color32::from_rgb(28, 24, 19), Stroke::NONE));
    let inner: Vec<Pos2> = pts.iter().map(|q| c + (*q - c) * 0.81 + Vec2::new(0.0, -0.5)).collect();
    p.add(egui::Shape::line(inner, Stroke::new(0.8_f32, Color32::from_rgb(92, 76, 56))));
}

/// One of the three small meters under the panel (0 tuning, 1 stereo, 2 loudness): the same cream face, its scale,
/// its name and unit, the needle at `v` (in the meter's own units). `rest`: nothing to show, the needle rests.
#[allow(clippy::type_complexity)]
fn small_meter(p: &egui::Painter, win: Rect, which: usize, v: f32, sub: &str) {
    let k = (win.width() / 200.0).max(win.height() / 120.0);
    let f = |x: f32, y: f32| Pos2::new(win.center().x + (x - 100.0) * k, win.center().y + (y - 60.0) * k);
    let cp = p.with_clip_rect(win);
    face(&cp, win, f(100.0, 120.0), 120.0 * k);
    let t = |n: f32| n * k;
    let (px, py, r, sw) = (100.0, 138.0, 86.0, 46.0);
    type Tick = (f32, &'static str, bool);
    let (min, max, ticks, minor, bands, title): (f32, f32, Vec<Tick>, Vec<f32>, Vec<(f32, f32, Color32)>, &str) = match which {
        0 => (
            -12.0,
            12.0,
            vec![(-12.0, "12", false), (-6.0, "6", false), (0.0, "0", false), (6.0, "6", false), (12.0, "12", false)],
            (-11..=11).map(|i| i as f32).collect(),
            vec![(-1.0, 1.0, Color32::from_rgb(47, 122, 69))],
            "BALANCE",
        ),
        1 => (
            -1.0,
            1.0,
            vec![(-1.0, "−1", true), (-0.5, "", false), (0.0, "0", false), (0.5, "", false), (1.0, "+1", false)],
            (-9..=9).map(|i| i as f32 * 0.1).collect(),
            vec![(-1.0, 0.0, FACE_RED)],
            "STEREO",
        ),
        _ => (
            -30.0,
            0.0,
            vec![(-30.0, "30", false), (-24.0, "24", false), (-18.0, "18", false), (-12.0, "12", false), (-6.0, "6", true), (0.0, "0", true)],
            (1..15).map(|i| -30.0 + i as f32 * 2.0).collect(),
            vec![(-6.0, 0.0, FACE_RED)],
            "LOUDNESS",
        ),
    };
    let ang = |x: f32| (-sw + 2.0 * sw * (x - min) / (max - min)).to_radians();
    let pt = |a: f32, rr: f32| f(px + rr * a.sin(), py - rr * a.cos());
    let arc = |rr: f32, a0: f32, a1: f32| -> Vec<Pos2> { (0..=30).map(|i| pt(a0 + (a1 - a0) * i as f32 / 30.0, rr)).collect() };
    for (a, b, c) in &bands {
        cp.add(egui::Shape::line(arc(r + 3.0, ang(*a), ang(*b)), Stroke::new(t(4.5), *c)));
    }
    cp.add(egui::Shape::line(arc(r, ang(min), ang(max)), Stroke::new(t(1.0), FACE_INK)));
    for x in minor {
        cp.line_segment([pt(ang(x), r), pt(ang(x), r + 4.0)], Stroke::new(t(0.7), FACE_INK));
    }
    for (x, lab, red) in ticks {
        let col = if red { FACE_RED } else { FACE_INK };
        cp.line_segment([pt(ang(x), r), pt(ang(x), r + 7.5)], Stroke::new(t(1.2), col));
        if !lab.is_empty() {
            cp.text(pt(ang(x), r + 15.0), egui::Align2::CENTER_CENTER, lab, egui::FontId::proportional(t(10.0)), col);
        }
    }
    caps(&cp, f(100.0, 88.0), egui::Align2::CENTER_CENTER, title, t(13.0), t(1.5), Color32::from_rgb(42, 35, 24), true);
    caps(&cp, f(100.0, 99.5), egui::Align2::CENTER_CENTER, sub, t(6.4), t(1.0), FACE_SOFT, false);
    if which == 0 {
        caps(&cp, f(22.0, 100.0), egui::Align2::CENTER_CENTER, "L", t(14.0), 0.0, FACE_INK, true);
        caps(&cp, f(178.0, 100.0), egui::Align2::CENTER_CENTER, "R", t(14.0), 0.0, FACE_INK, true);
    }
    let x = v.clamp(min - (max - min) * 0.06, max + (max - min) * 0.03);
    let a = ang(x);
    let (n0, n1) = (pt(a, 16.0), pt(a, r + 9.0));
    let sh = Vec2::new(t(2.0), t(2.4));
    cp.line_segment([n0 + sh, n1 + sh], Stroke::new(t(2.0), Color32::from_black_alpha(45)));
    cp.line_segment([n0, n1], Stroke::new(t(1.3).max(1.0), Color32::from_rgb(21, 18, 14)));
    cap(&cp, f(100.0, 120.0), t(13.0));
}

/// The analyzer: 32 bands of amber lights, ten high, with each band's peak held as a thin bright line, and the bands'
/// frequencies under them.
fn led_bars(p: &egui::Painter, m: Mu, r: Rect, bands: &[f32], peaks: &[f32]) {
    let amber = crate::styles::amber();
    let inn = Rect::from_min_max(r.min + Vec2::new(m.v(8.0), m.v(6.0)), r.max - Vec2::new(m.v(8.0), m.v(17.0)));
    let n = bands.len().max(1);
    let gap = m.v(3.0);
    let bw = (inn.width() - gap * (n - 1) as f32) / n as f32;
    let rows = 10usize;
    let sh = inn.height() / rows as f32;
    for (i, v) in bands.iter().enumerate() {
        let x = inn.min.x + i as f32 * (bw + gap);
        let lit = (v.clamp(0.0, 1.0) * rows as f32).round() as usize;
        for s in 0..rows {
            let y = inn.max.y - (s as f32 + 1.0) * sh;
            let rr = Rect::from_min_size(Pos2::new(x, y + sh * 0.14), Vec2::new(bw, sh * 0.72));
            if s < lit {
                p.rect_filled(rr.expand(1.5), 2.0, add(amber, 0.12));
                p.rect_filled(rr, 1.0, amber);
            } else {
                p.rect_filled(rr, 1.0, amber.gamma_multiply(0.13));
            }
        }
        let pk = peaks.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
        if pk * rows as f32 > lit as f32 + 0.5 {
            let y = inn.max.y - pk * inn.height();
            p.rect_filled(Rect::from_min_size(Pos2::new(x, y - 1.0), Vec2::new(bw, 2.0)), 0.0, amber.lerp_to_gamma(Color32::WHITE, 0.55));
        }
    }
    for (hz, lab) in [(31.0f32, "31"), (63.0, "63"), (125.0, "125"), (250.0, "250"), (500.0, "500"), (1000.0, "1k"), (2000.0, "2k"), (4000.0, "4k"), (8000.0, "8k"), (16000.0, "16k")] {
        let fx = (hz / 20.0).ln() / 1000f32.ln();
        caps(p, Pos2::new(inn.min.x + inn.width() * fx, r.max.y - m.v(8.5)), egui::Align2::CENTER_CENTER, lab, m.v(8.0), 0.0, amber.gamma_multiply(0.75), false);
    }
}

/// Smoked glass: a dark window, deepest at the foot, set into the panel.
pub(crate) fn glass(p: &egui::Painter, m: Mu, r: Rect) {
    p.rect_filled(r, m.v(3.0), Color32::from_rgb(8, 7, 6));
    vgrad(p, r.shrink(1.0), Color32::from_rgb(10, 9, 8), Color32::from_rgb(20, 16, 12));
}

/// The glass's edge, over what's behind it.
pub(crate) fn glare(p: &egui::Painter, m: Mu, r: Rect) {
    p.rect_stroke(r.shrink(1.0), m.v(3.0), Stroke::new(2.0_f32, Color32::from_black_alpha(120)));
}

/// The analyzer as light behind the glass: soft columns of warm light, bass at the left, each glowing up from the
/// foot as high as its band is loud.
fn light_columns(p: &egui::Painter, r: Rect, bands: &[f32]) {
    let n = bands.len().max(1);
    let gap = r.width() * 0.006;
    let w = (r.width() - gap * (n - 1) as f32) / n as f32;
    let col = |a: f32, c: (u8, u8, u8)| add(Color32::from_rgb(c.0, c.1, c.2), a);
    for (i, v) in bands.iter().enumerate() {
        let h = r.height() * v.clamp(0.0, 1.0);
        if h < 1.0 {
            continue;
        }
        let x = r.min.x + i as f32 * (w + gap);
        // the glow, wide and faint, then the column's body, then its bright core
        for (grow, a) in [(w * 1.1, 0.1f32), (w * 0.4, 0.28), (0.0, 0.85)] {
            let rr = Rect::from_min_max(Pos2::new(x - grow, r.max.y - h - grow * 0.6), Pos2::new(x + w + grow, r.max.y));
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(rr.left_top(), Color32::TRANSPARENT);
            mesh.colored_vertex(rr.right_top(), Color32::TRANSPARENT);
            mesh.colored_vertex(Pos2::new(rr.max.x, rr.min.y + rr.height() * 0.5), col(a * 0.75, (255, 170, 64)));
            mesh.colored_vertex(Pos2::new(rr.min.x, rr.min.y + rr.height() * 0.5), col(a * 0.75, (255, 170, 64)));
            mesh.colored_vertex(rr.right_bottom(), col(a, (255, 228, 170)));
            mesh.colored_vertex(rr.left_bottom(), col(a, (255, 228, 170)));
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            mesh.add_triangle(3, 2, 4);
            mesh.add_triangle(3, 4, 5);
            p.add(mesh);
        }
    }
}

/// The red jewel light: faceted, in a chrome ring, glowing when `on`.
fn jewel(p: &egui::Painter, c: Pos2, r: f32, on: bool) {
    if on {
        ball(p, c, r * 2.2, Vec2::ZERO, &[(0.0, Color32::from_rgba_premultiplied(150, 36, 18, 0)), (0.5, Color32::from_rgba_premultiplied(60, 12, 6, 0)), (1.0, Color32::TRANSPARENT)]);
    }
    p.circle_filled(c, r + r * 0.34, Color32::from_rgb(74, 74, 74));
    ball(p, c, r + r * 0.26, Vec2::new(-0.3, -0.4), &[(0.0, Color32::from_rgb(240, 242, 244)), (0.5, Color32::from_rgb(160, 165, 171)), (1.0, Color32::from_rgb(110, 114, 120))]);
    let stops: [(f32, Color32); 4] = if on {
        [(0.0, Color32::from_rgb(255, 216, 204)), (0.22, Color32::from_rgb(255, 90, 60)), (0.52, Color32::from_rgb(200, 28, 16)), (1.0, Color32::from_rgb(94, 8, 4))]
    } else {
        [(0.0, Color32::from_rgb(150, 90, 84)), (0.22, Color32::from_rgb(110, 30, 22)), (0.52, Color32::from_rgb(70, 12, 8)), (1.0, Color32::from_rgb(34, 4, 2))]
    };
    ball(p, c, r, Vec2::new(-0.28, -0.4), &stops);
    // the facets
    for k in 0..12 {
        let a = k as f32 / 12.0 * std::f32::consts::TAU;
        let d = Vec2::new(a.cos(), a.sin());
        p.line_segment([c + d * r * 0.2, c + d * r * 0.95], Stroke::new(0.7_f32, if k % 2 == 0 { Color32::from_rgba_premultiplied(30, 30, 30, 30) } else { Color32::from_black_alpha(40) }));
    }
}

/// The STANDBY switch: a chrome ball with its lever up (on) or down.
fn toggle(p: &egui::Painter, m: Mu, c: Pos2, up: bool) {
    let r = m.v(8.0);
    ball(p, c + Vec2::new(0.0, 1.0), r + 1.0, Vec2::ZERO, &[(0.0, Color32::from_black_alpha(120)), (1.0, Color32::TRANSPARENT)]);
    ball(p, c, r, Vec2::new(-0.2, -0.3), &[(0.0, Color32::from_rgb(250, 250, 250)), (0.7, Color32::from_rgb(141, 146, 152)), (1.0, Color32::from_rgb(85, 89, 94))]);
    let (dir, len) = (if up { -1.0 } else { 1.0 }, m.v(15.0));
    let lever = Rect::from_min_max(Pos2::new(c.x - m.v(3.0), c.y + if up { dir * len } else { 0.0 }), Pos2::new(c.x + m.v(3.0), c.y + if up { 0.0 } else { len }));
    p.rect_filled(lever.translate(Vec2::new(0.0, 1.0)), m.v(3.0), Color32::from_black_alpha(110));
    let mut mesh = egui::Mesh::default();
    let (a, b) = (Color32::from_rgb(139, 144, 150), Color32::from_rgb(244, 245, 246));
    for (q, col) in [(lever.left_top(), a), (Pos2::new(lever.center().x, lever.min.y), b), (lever.right_top(), a), (lever.right_bottom(), a), (Pos2::new(lever.center().x, lever.max.y), b), (lever.left_bottom(), a)] {
        mesh.colored_vertex(q, col);
    }
    for t in [[0, 1, 4], [0, 4, 5], [1, 2, 3], [1, 3, 4]] {
        mesh.add_triangle(t[0], t[1], t[2]);
    }
    p.add(mesh);
    let tip = if up { lever.center_top() } else { lever.center_bottom() };
    ball(p, tip, m.v(3.2), Vec2::new(-0.3, -0.3), &[(0.0, Color32::WHITE), (1.0, Color32::from_rgb(130, 135, 140))]);
}

/// The INPUT jacks (two chrome sockets, numbered).
fn jacks(p: &egui::Painter, m: Mu, c: Pos2) {
    caps(p, Pos2::new(c.x, c.y - m.v(39.0)), egui::Align2::CENTER_TOP, "INPUT", m.v(8.5), m.v(1.5), INK, true);
    for (k, dx) in [-12.0f32, 12.0].into_iter().enumerate() {
        let q = Pos2::new(c.x + m.v(dx), c.y);
        ball(p, q, m.v(7.5), Vec2::new(-0.2, -0.3), &[(0.0, Color32::from_rgb(228, 230, 232)), (0.6, Color32::from_rgb(122, 127, 133)), (1.0, Color32::from_rgb(200, 204, 208))]);
        p.circle_filled(q, m.v(3.6), Color32::from_rgb(5, 5, 5));
        caps(p, Pos2::new(q.x, c.y + m.v(13.0)), egui::Align2::CENTER_TOP, &(k + 1).to_string(), m.v(7.0), 0.0, INK, false);
    }
}

/// The numbers round a knob (1 to 10), or for BALANCE (`kind` 1) eleven ticks with L and R.
fn knob_ring(p: &egui::Painter, m: Mu, c: Pos2, kind: u8) {
    let at = |a: f32, r: f32| c + Vec2::new(a.to_radians().sin(), -a.to_radians().cos()) * m.v(r);
    if kind == 1 {
        for i in 0..=10 {
            let a = -150.0 + i as f32 * 30.0;
            p.line_segment([at(a, 23.0), at(a, if i == 5 { 19.0 } else { 20.5 })], Stroke::new(m.v(0.8), INK));
        }
        caps(p, c + Vec2::new(-m.v(15.0), m.v(21.5)), egui::Align2::CENTER_CENTER, "L", m.v(6.5), 0.0, INK, false);
        caps(p, c + Vec2::new(m.v(15.0), m.v(21.5)), egui::Align2::CENTER_CENTER, "R", m.v(6.5), 0.0, INK, false);
    } else {
        for i in 1..=10 {
            let a = -150.0 + (i - 1) as f32 * 300.0 / 9.0;
            caps(p, at(a, 24.0), egui::Align2::CENTER_CENTER, &i.to_string(), m.v(6.5), 0.0, INK, false);
        }
    }
}

/// A cream knob at `c` turned to `v` (1 to 10).
fn cream_knob(p: &egui::Painter, m: Mu, c: Pos2, v: f32) {
    let r = m.v(16.0);
    ball(p, c + Vec2::new(0.0, m.v(3.0)), r + m.v(2.5), Vec2::ZERO, &[(0.0, Color32::from_black_alpha(150)), (0.7, Color32::from_black_alpha(60)), (1.0, Color32::TRANSPARENT)]);
    ball(
        p,
        c,
        r,
        Vec2::new(-0.24, -0.36),
        &[(0.0, Color32::from_rgb(255, 251, 241)), (0.35, Color32::from_rgb(239, 227, 198)), (0.7, Color32::from_rgb(205, 187, 146)), (1.0, Color32::from_rgb(156, 138, 98))],
    );
    let a = (-150.0 + (v - 1.0) * 300.0 / 9.0f32).to_radians();
    let d = Vec2::new(a.sin(), -a.cos());
    p.line_segment([c + d * m.v(3.0), c + d * m.v(14.0)], Stroke::new(m.v(2.0), Color32::from_rgb(58, 42, 24)));
}

/// One DISPLAY push button: a small square key, its lamp lit when `on`.
fn push_button(p: &egui::Painter, r: Rect, on: bool, down: bool) {
    p.rect_filled(r.translate(Vec2::new(0.0, 1.5)), 2.0, Color32::from_black_alpha(150));
    p.rect_filled(r.expand(0.5), 2.0, Color32::BLACK);
    let (t, b) = if on || down { (Color32::from_rgb(42, 40, 38), Color32::from_rgb(21, 20, 19)) } else { (Color32::from_rgb(59, 57, 54), Color32::from_rgb(27, 26, 25)) };
    vgrad(p, r, t, b);
    let c = r.center();
    let lr = r.width() * 0.24;
    if on {
        ball(p, c, lr * 2.6, Vec2::ZERO, &[(0.0, add(Color32::from_rgb(255, 181, 71), 0.55)), (1.0, Color32::TRANSPARENT)]);
        p.circle_filled(c, lr, Color32::from_rgb(255, 181, 71));
    } else {
        p.circle_filled(c, lr, Color32::from_rgb(58, 36, 8));
    }
}

/// A list on a screen scrolled by the wheel over `over`: `want` (in rows) moved by the wheel, `at` easing after it
/// and coming to rest on a whole row (`most`: the furthest it goes).
fn ease_scroll(ui: &egui::Ui, want: &mut f32, at: &mut f32, most: f32, rh: f32, over: Rect, dt: f32) {
    if ui.input(|i| i.pointer.hover_pos()).is_some_and(|pp| over.contains(pp)) {
        let wheel = ui.input(|i| i.raw_scroll_delta.y);
        if wheel != 0.0 {
            *want -= wheel / rh;
        }
    }
    *want = want.within(0.0, most);
    let rest = want.round().min(most);
    *at += (rest - *at) * (1.0 - (-dt * 14.0).exp());
    if (rest - *at).abs() < 0.002 {
        *at = rest;
    } else {
        ui.ctx().request_repaint();
    }
}

/// More rows than fit: a thin thumb at the screen's right edge saying where in them you are.
#[allow(clippy::too_many_arguments)]
fn scroll_thumb(p: &egui::Painter, m: Mu, w: Rect, list: Rect, at: f32, most: f32, rows: f32, rh: f32) {
    if most <= 0.0 {
        return;
    }
    let track = Rect::from_min_max(Pos2::new(w.max.x - m.v(5.0), list.min.y + m.v(2.0)), Pos2::new(w.max.x - m.v(3.0), list.max.y - m.v(2.0)));
    let frac = (list.height() / rh / rows).min(1.0);
    let th = (track.height() * frac).max(m.v(10.0));
    let ty = track.min.y + (track.height() - th) * (at / most).clamp(0.0, 1.0);
    p.rect_filled(Rect::from_min_size(Pos2::new(track.min.x, ty), Vec2::new(track.width(), th)), 1.0, crate::styles::amber().gamma_multiply(0.45));
}
