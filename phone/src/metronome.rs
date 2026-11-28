//! The METRONOME, the desktop's (its sound and settings are the desktop's own, `metro.rs`): its own tempo (TAP finds
//! it), beats per bar with each beat's volume (tap a beat), the beat unit and accent groups, subdivisions and presets,
//! its own volume, the gap and speed-up drills, and following the song: its speed, or locked to the song's own beats
//! (found from the audio, x2 and /2, an offset). Opened from the deck's METRO key; it shows in Practice in place of the
//! practice tools, and keeps clicking when closed.

use crate::*;

impl App {
    /// The metronome's tempo: its own (and what the speed-up drill has added), following the song's speed if asked.
    fn metro_tempo(&self, playing: bool) -> f32 {
        let base = self.mt_bpm as f32 + self.mt_add;
        if self.mt.follow && playing {
            base * self.speed as f32 / 100.0
        } else {
            base
        }
    }

    /// Start or stop it.
    pub(crate) fn metro_toggle(&mut self) {
        self.mt.on = !self.mt.on;
        self.mt_add = 0.0;
        self.mt_step_at = Instant::now();
        if !self.mt.on {
            // (silenced right now: the click loop plays forever until told to stop)
            self.player.send(Cmd::Pcm(0, Vec::new(), Vec::new(), 0.0));
            self.mt_vis = None;
        }
        self.mt_sig = 0;
        self.save_settings();
    }

    /// The song's beat looked for in the background (the count-off's tempo, the metronome's lock).
    pub(crate) fn find_beats(&mut self) {
        let (Some(c), Some(b)) = (self.cur.clone(), self.audio.clone()) else { return };
        if self.beat_busy {
            return;
        }
        self.beat_busy = true;
        self.spawn(move || Msg::Beats(c.vid, beats::detect_beats(b.to_vec())));
    }

    /// Once a frame (the desktop's `update_metro`): tempo changes, the speed-up drill, locking to the song.
    pub(crate) fn update_metro(&mut self) {
        let playing = self.cur.is_some() && !self.stopped && !self.paused && !self.loading;
        let vid = self.cur.as_ref().map(|c| c.vid.clone());
        let ratio = if playing { self.speed as f32 / 100.0 } else { 1.0 };
        let has_beat = self.beat.as_ref().is_some_and(|b| Some(&b.0) == vid.as_ref());
        let has_grid = self.beat_grid.as_ref().is_some_and(|g| Some(&g.0) == vid.as_ref());
        // the song's beats known and the song at its own speed: the clicks go into the song itself (see below)
        let in_song = self.mt.on && self.mt.sync && playing && (ratio - 1.0).abs() < 0.001 && has_beat && has_grid;
        if !in_song {
            self.player.ctl.set_song_clicks(None);
        }
        if !self.mt.on {
            if self.mt_sig != 0 {
                self.player.send(Cmd::Pcm(0, Vec::new(), Vec::new(), 0.0));
                self.mt_sig = 0;
                self.mt_vis = None;
            }
            return;
        }
        // locking needs the song's beat: looked for by itself once the song is in
        let sync_wanted = self.mt.sync && !vid.as_ref().is_some_and(|v| self.beat_failed.contains(v));
        if sync_wanted && playing && !self.beat_busy && !has_beat {
            self.find_beats();
        }
        let locked = sync_wanted && playing && has_beat;
        if sync_wanted && !locked {
            // (locking asked for, nothing to lock to yet)
            if self.mt_sig != 0 {
                self.player.send(Cmd::Pcm(0, Vec::new(), Vec::new(), 0.0));
                self.mt_sig = 0;
                self.mt_vis = None;
            }
            return;
        }
        if !locked && self.mt.ramp_bars > 0 {
            let tempo = self.metro_tempo(playing);
            let bar = self.mt.beats as f32 * 60.0 / self.mt.pulse(tempo.max(20.0));
            if self.mt_step_at.elapsed().as_secs_f32() >= self.mt.ramp_bars as f32 * bar {
                if tempo + self.mt.ramp_bpm as f32 <= self.mt.ramp_max as f32 {
                    self.mt_add += self.mt.ramp_bpm as f32;
                }
                self.mt_step_at = Instant::now();
            }
        }
        if locked {
            let pos = self.pos();
            let (last, at) = self.mt_pos;
            // (the clicks are laid on the song's beats ahead of time: only a seek, or nearing the end of what's laid
            // out, lays them again)
            let jumped = (pos - (last + at.elapsed().as_secs_f32() * ratio)).abs() > 0.25;
            let due = if has_grid { pos > self.mt_lead_end - 6.0 || pos < self.mt_lead_end - 60.0 } else { self.mt_sync_at.elapsed().as_secs() >= 12 };
            if (jumped || due) && !in_song {
                self.mt_gen += 1;
                self.mt_sync_at = Instant::now();
            }
            self.mt_pos = (pos, Instant::now());
        }
        // locked: the local beat length and the next real beat, from the list (x2 and /2 scale it)
        let grid_now = if locked {
            let pos = self.pos();
            let scale = self.beat.as_ref().map_or(1.0, |b| b.1);
            self.beat_grid.as_ref().filter(|g| Some(&g.0) == vid.as_ref()).and_then(|(_, grid, usual)| beats::next_beat(grid, pos).map(|(next, local)| (next, local * scale / usual)))
        } else {
            None
        };
        let period = self.beat.as_ref().map_or(0.5, |b| b.1);
        let tempo = if locked { 60.0 * ratio / period } else { self.metro_tempo(playing) };
        let m = &self.mt;
        let mut sig = (tempo * 10.0) as u64 ^ ((m.beats as u64) << 20) ^ ((m.sub as u64) << 24) ^ ((m.gap_play as u64) << 28);
        sig ^= (m.gap_mute as u64) << 34 ^ (locked as u64) << 40 ^ (self.mt_gen as u64) << 41 ^ (m.unit as u64) << 52;
        for l in &m.levels[..m.beats.clamp(1, 32) as usize] {
            sig = sig.wrapping_mul(31).wrapping_add(*l as u64 + 1);
        }
        sig |= 1 << 63;
        if sig == self.mt_sig {
            return;
        }
        self.mt_sig = sig;
        let mut cfg = self.mt.clone();
        let mut lead = Vec::new();
        let mut wait = 0.0f32;
        if locked {
            cfg.beats = 1;
            cfg.levels = [2; 32];
            cfg.unit = 4;
            cfg.gap_play = 0;
            if let Some((_, period0, phase)) = self.beat.clone() {
                let pos = self.pos();
                let next = match grid_now {
                    Some((g, _)) => g,
                    None => phase + ((pos - phase) / period0).ceil().max(0.0) * period0,
                };
                let each = period / ratio;
                wait = ((next - pos).max(0.0) / ratio + self.mt.shift_ms as f32 / 1000.0).rem_euclid(each);
                lead = vec![0i16; (wait * metro::RATE) as usize];
            }
        }
        let body = metro::click_loop(&cfg, tempo);
        // the song's beats known and the song at its own speed: the clicks go into the song itself, on its beats, so
        // nothing can drift (slowed down the song is stretched, which would smear a click in it: then the laid-out
        // stream below)
        if in_song {
            if let Some((_, grid, _)) = self.beat_grid.as_ref() {
                let (hi, lo) = metro::click_sounds();
                let beats: Vec<f64> = grid.iter().map(|b| *b as f64 + self.mt.shift_ms as f64 / 1000.0).collect();
                let n = self.mt.beats.max(1) as usize;
                let accent: Vec<bool> = (0..beats.len()).map(|k| k % n == 0).collect();
                self.player.ctl.set_song_clicks(Some(player::SongClicks { beats, accent, hi, lo, gain: 0.8 }));
            }
            self.mt_vis = Some((Instant::now() + Duration::from_secs_f32(wait), period, 1));
            self.player.send(Cmd::Pcm(0, Vec::new(), Vec::new(), 0.0));
            return;
        }
        if locked {
            if let Some((_, grid, _)) = self.beat_grid.as_ref().filter(|g| Some(&g.0) == vid.as_ref()) {
                let pos = self.pos();
                let shift = self.mt.shift_ms as f32 / 1000.0;
                lead = metro::grid_clicks(grid, pos, 40.0, ratio, shift, self.mt.beats.max(1), 1.0);
                self.mt_lead_end = pos + 40.0;
                wait = 0.0;
                if let Some((next, _)) = beats::next_beat(grid, pos + 40.0) {
                    // the loop after it starts on the beat after the laid-out stretch
                    let gap = (((next - (pos + 40.0)) / ratio) * metro::RATE) as usize;
                    lead.extend(std::iter::repeat_n(0i16, gap));
                }
            }
        }
        let cycle = cfg.beats.clamp(1, 32) as usize * if cfg.gap_play > 0 { (cfg.gap_play + cfg.gap_mute.max(1)) as usize } else { 1 };
        self.mt_vis = Some((Instant::now() + Duration::from_secs_f32(wait), 60.0 / cfg.pulse(tempo), cycle));
        self.player.send(Cmd::Pcm(0, lead, body, 0.8));
    }

    /// Where the metronome is in the bar: the beat, how far into it, and whether the gap drill has it silent.
    pub(crate) fn metro_pos(&self) -> (Option<usize>, f32, bool) {
        let beats = self.mt.beats.clamp(1, 32) as usize;
        match (self.mt.on, self.mt_vis) {
            (true, Some((t0, spb, cycle))) => {
                let now = Instant::now();
                if now < t0 {
                    (None, 0.0, false)
                } else {
                    let e = (now - t0).as_secs_f32() / spb.max(0.05);
                    let n = e.floor() as usize % cycle.max(1);
                    let silent = self.mt.gap_play > 0 && n >= self.mt.gap_play as usize * beats;
                    (Some(n % beats), e.fract(), silent)
                }
            }
            _ => (None, 0.0, false),
        }
    }

    /// TAP (the metronome's and the trainer's): the music's tempo from your taps, for the song and the metronome both
    /// (as at the speed you're playing it, taken back to the record's).
    pub(crate) fn tap_tempo(&mut self, now: f64) {
        self.taps.retain(|t| now - *t < 2.0);
        self.taps.push(now);
        if self.taps.len() > 8 {
            self.taps.remove(0);
        }
        if self.taps.len() >= 3 {
            let span = (now - self.taps[0]) as f32;
            let heard = 60.0 / (span / (self.taps.len() - 1) as f32).max(0.05);
            let at = (heard / (self.speed as f32 / 100.0)).round();
            self.set_bpm(at);
            self.mt_bpm = (at as u32).clamp(20, 400);
            self.save_settings();
        } else {
            self.set_note("Keep tapping...");
        }
    }

    /// The panel (in Practice, in place of the practice tools).
    pub(crate) fn metro_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if retro_btn(ui, "CLOSE", false).clicked() {
                    self.mt_open = false;
                }
                crate::songs::heading(ui, "METRONOME", ui.available_width());
            })
        });
        let now = ui.input(|i| i.time);
        if self.mt.on {
            ui.ctx().request_repaint();
        }
        egui::ScrollArea::vertical().id_salt("metro_scroll").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 8.0);
            // ---- start, tempo, tap, the meter
            centered_row(ui, "mt_top", |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let on = self.mt.on;
                if retro_btn_w(ui, if on { "STOP" } else { "START" }, 80.0, on).clicked() {
                    self.metro_toggle();
                }
                label(ui, "BPM", 0.0);
                if retro_btn_w(ui, "-", 28.0, false).clicked() {
                    self.mt_bpm = self.mt_bpm.saturating_sub(1).max(20);
                    self.save_settings();
                }
                lcd_box(ui, &self.mt_bpm.to_string(), 52.0, pal().ink);
                if retro_btn_w(ui, "+", 28.0, false).clicked() {
                    self.mt_bpm = (self.mt_bpm + 1).min(400);
                    self.save_settings();
                }
                if retro_btn_w(ui, "TAP", 56.0, false).clicked() {
                    self.tap_tempo(now);
                }
            });
            // ---- the pendulum and the beats: each bar's height is that beat's volume; tap one to change it
            ui.horizontal(|ui| {
                let h = 76.0;
                let (pr, _) = ui.allocate_exact_size(Vec2::new(h, h), Sense::hover());
                self.pendulum(ui, pr);
                let w = ui.available_width();
                self.beat_bars(ui, w, h);
            });
            // ---- meter
            section_header(ui, "METER");
            centered_row(ui, "mt_meter", |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                label(ui, "BEATS", 0.0);
                if retro_btn_w(ui, "-", 28.0, false).clicked() && self.mt.beats > 1 {
                    self.mt.beats -= 1;
                    self.mt_group = 0;
                    self.save_settings();
                }
                lcd_box(ui, &self.mt.beats.to_string(), 40.0, pal().ink);
                if retro_btn_w(ui, "+", 28.0, false).clicked() && self.mt.beats < 32 {
                    self.mt.beats += 1;
                    self.mt_group = 0;
                    self.save_settings();
                }
                label(ui, "/", 0.0);
                for n in [4u8, 8, 16] {
                    if retro_btn_w(ui, &n.to_string(), 36.0, self.mt.unit == n).clicked() {
                        self.mt.unit = n;
                        self.save_settings();
                    }
                }
            });
            let gs = metro::groupings(self.mt.beats);
            if gs.len() > 1 {
                centered_row(ui, "mt_group", |ui| {
                    label(ui, "ACCENT", 0.0);
                    let txt = match self.mt_group {
                        k if k > 0 && k <= gs.len() => gs[k - 1].iter().map(|n| n.to_string()).collect::<Vec<_>>().join("+"),
                        _ => "GROUPS".to_string(),
                    };
                    if retro_btn_w(ui, &txt, 140.0, self.mt_group > 0).clicked() {
                        self.mt_group = (self.mt_group + 1) % (gs.len() + 1);
                        match self.mt_group {
                            0 => self.mt.preset(0),
                            k => self.mt.apply_group(&gs[k - 1]),
                        }
                        self.save_settings();
                    }
                });
            }
            // ---- sound
            section_header(ui, "SOUND");
            centered_row(ui, "mt_sub", |ui| {
                let sub = ["NO SUBDIV", "EIGHTHS", "TRIPLETS", "SIXTEENTHS"][(self.mt.sub as usize).min(3)];
                if retro_btn_w(ui, sub, 130.0, self.mt.sub > 0).clicked() {
                    self.mt.sub = (self.mt.sub + 1) % 4;
                    self.save_settings();
                }
            });
            centered_row(ui, "mt_presets", |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for (i, name) in ["ALL", "2 & 4", "1 ONLY"].iter().enumerate() {
                    if retro_btn_w(ui, name, 76.0, false).clicked() {
                        self.mt.preset(i as u8);
                        self.mt_group = 0;
                        self.save_settings();
                    }
                }
            });
            // its own volume: louder or softer against the song, the main volume left as it is
            ui.horizontal(|ui| {
                label(ui, "VOL", 0.0);
                let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width() - 8.0, bh()), Sense::click_and_drag());
                inset(ui.painter(), r, pal().display);
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
            // ---- the drills: bars of click then bars of silence; faster every few bars
            section_header(ui, "TRAINER");
            let step = |ui: &mut egui::Ui, id: &str, name: &str, v: &mut u32, lo: u32, hi: u32, show: String, after: &str| -> bool {
                let mut changed = false;
                centered_row(ui, id, |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    label(ui, name, 0.0);
                    if retro_btn_w(ui, "-", 28.0, false).clicked() && *v > lo {
                        *v -= 1;
                        changed = true;
                    }
                    lcd_box(ui, &show, 52.0, pal().ink);
                    if retro_btn_w(ui, "+", 28.0, false).clicked() && *v < hi {
                        *v += 1;
                        changed = true;
                    }
                    if !after.is_empty() {
                        label(ui, after, 0.0);
                    }
                });
                changed
            };
            let mut m = self.mt.clone();
            let mut ch = false;
            let gp = if m.gap_play == 0 { "OFF".to_string() } else { m.gap_play.to_string() };
            ch |= step(ui, "mt_gp", "PLAY", &mut m.gap_play, 0, 32, gp, "BARS");
            if m.gap_play > 0 {
                let gm = m.gap_mute.to_string();
                ch |= step(ui, "mt_gm", "THEN MUTE", &mut m.gap_mute, 1, 32, gm, "BARS");
            }
            let rb = if m.ramp_bars == 0 { "OFF".to_string() } else { m.ramp_bars.to_string() };
            ch |= step(ui, "mt_rb", "FASTER EVERY", &mut m.ramp_bars, 0, 64, rb, "BARS");
            if m.ramp_bars > 0 {
                let rbpm = m.ramp_bpm.to_string();
                ch |= step(ui, "mt_rbpm", "BY", &mut m.ramp_bpm, 1, 40, rbpm, "BPM");
                let rmax = m.ramp_max.to_string();
                ch |= step(ui, "mt_rmax", "UP TO", &mut m.ramp_max, 40, 400, rmax, "BPM");
            }
            if ch {
                self.mt = m;
                self.mt_add = 0.0;
                self.mt_step_at = Instant::now();
                self.save_settings();
            }
            // ---- with the song: its speed, its beat
            section_header(ui, "WITH THE SONG");
            ui.horizontal_wrapped(|ui| {
                if check_box(ui, "FOLLOW SPEED", self.mt.follow).clicked() {
                    self.mt.follow = !self.mt.follow;
                    self.save_settings();
                }
                if check_box(ui, "LOCK TO THE SONG", self.mt.sync).clicked() {
                    self.mt.sync = !self.mt.sync;
                    let vid = self.cur.as_ref().map(|c| c.vid.clone());
                    if self.mt.sync && self.beat.as_ref().map(|b| &b.0) != vid.as_ref() {
                        self.find_beats();
                    }
                    self.save_settings();
                }
            });
            let vid = self.cur.as_ref().map(|c| c.vid.clone());
            if self.mt.sync {
                let (msg, red) = if self.cur.is_none() {
                    ("Play a song to click along with it", false)
                } else if self.beat_busy {
                    ("Finding the song's beat...", false)
                } else if vid.as_ref().is_some_and(|v| self.beat_failed.contains(v)) {
                    ("No steady beat found. Using its own tempo", true)
                } else if self.beat.as_ref().map(|b| &b.0) == vid.as_ref() && self.beat.is_some() {
                    ("Locked to the song", false)
                } else {
                    ("Waiting for the song", false)
                };
                dim_line(ui, msg, 1.0, if red { pal().red } else { pal().ink2 });
            }
            if let Some(b) = self.beat.clone().filter(|b| Some(&b.0) == vid.as_ref()) {
                centered_row(ui, "mt_beat", |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    label(ui, "SONG BEAT", 0.0);
                    lcd_box(ui, &format!("{:.0} BPM", 60.0 / b.1), 84.0, pal().ink);
                    for (t, k) in [("x2", 0.5f32), ("/2", 2.0)] {
                        if retro_btn_w(ui, t, 40.0, false).clicked() {
                            if let Some(bb) = self.beat.as_mut() {
                                bb.1 *= k;
                            }
                            self.mt_gen += 1;
                        }
                    }
                });
                centered_row(ui, "mt_shift", |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    label(ui, "OFFSET", 0.0);
                    if retro_btn_w(ui, "-", 28.0, false).clicked() {
                        self.mt.shift_ms -= 10;
                        self.mt_gen += 1;
                        self.save_settings();
                    }
                    lcd_box(ui, &format!("{} MS", self.mt.shift_ms), 76.0, pal().ink);
                    if retro_btn_w(ui, "+", 28.0, false).clicked() {
                        self.mt.shift_ms += 10;
                        self.mt_gen += 1;
                        self.save_settings();
                    }
                });
            }
            ui.add_space(16.0);
        });
    }

    /// A swinging pendulum in `rect` (the desktop's); its tip flashes on the beat.
    fn pendulum(&self, ui: &egui::Ui, rect: Rect) {
        let (_, frac, muted) = self.metro_pos();
        inset(ui.painter(), rect, pal().display);
        let p = ui.painter();
        let k = rect.height() / 132.0;
        let pivot = Pos2::new(rect.center().x, rect.max.y - 14.0 * k);
        let phase = match (self.mt.on, self.mt_vis) {
            (true, Some((t0, spb, _))) if Instant::now() >= t0 => Some((Instant::now() - t0).as_secs_f32() / spb.max(0.05)),
            _ => None,
        };
        let swing = phase.map_or(0.0, |e| (std::f32::consts::PI * e).cos());
        let len = 106.0 * k;
        for a in [-0.62f32, 0.0, 0.62] {
            let q = pivot + Vec2::new(a.sin() * (len + 6.0 * k), -a.cos() * (len + 6.0 * k));
            fill_rect(p, Rect::from_center_size(q, Vec2::new(2.0, 4.0)), pal().groove);
        }
        let rod = |ang: f32, col: Color32| {
            let tip = pivot + Vec2::new(ang.sin() * len, -ang.cos() * len);
            p.line_segment([pivot, tip], Stroke::new(2.0_f32, col));
            tip
        };
        // two faint afterimages so the motion reads as smooth
        if let Some(e) = phase {
            for (back, col) in [(0.16f32, pal().display_ghost), (0.08, pal().groove)] {
                let _ = rod(0.62 * (std::f32::consts::PI * (e - back)).cos(), col);
            }
        }
        let tip = rod(swing * 0.62, pal().ink2);
        let weight = pivot + (tip - pivot) * 0.7;
        fill_rect(p, Rect::from_center_size(weight, Vec2::new(14.0 * k, 10.0 * k)), pal().ink);
        let hit = if self.mt.on && !muted { (1.0 - frac / 0.35).clamp(0.0, 1.0) } else { 0.0 };
        p.circle_filled(tip, 6.0 * k + 1.0, pal().ink);
        p.circle_filled(tip, 4.0 * k + 0.5, mix(pal().display, pal().red, hit));
        fill_rect(p, Rect::from_center_size(pivot, Vec2::new(30.0 * k, 8.0 * k)), pal().ink);
    }

    /// The beats as bars (the desktop's): each one's height is its volume (loud, normal, quiet, silent), the accent
    /// groups set apart. Tap a beat to step its volume.
    fn beat_bars(&mut self, ui: &mut egui::Ui, w: f32, h: f32) {
        let beats = self.mt.beats.clamp(1, 32) as usize;
        let (beat_no, frac, muted) = self.metro_pos();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
        let gs = metro::groupings(self.mt.beats);
        let groups: Vec<u32> = match self.mt_group {
            k if k > 0 && k <= gs.len() => gs[k - 1].clone(),
            _ => vec![beats as u32],
        };
        let mut starts = vec![false; beats];
        let mut at = 0usize;
        for g in &groups {
            if at > 0 && at < beats {
                starts[at] = true;
            }
            at += *g as usize;
        }
        let gaps = starts.iter().filter(|s| **s).count() as f32;
        let (gap, big) = (4.0, 14.0);
        let bw = ((w - gap * (beats as f32 - 1.0) - (big - gap) * gaps) / beats as f32).clamp(6.0, 56.0);
        let total = bw * beats as f32 + gap * (beats as f32 - 1.0) + (big - gap) * gaps;
        let mut x = rect.min.x + ((w - total) / 2.0).max(0.0);
        let num_h = if bw >= 12.0 { 14.0 } else { 0.0 };
        let (top, base) = (rect.min.y + 2.0, rect.max.y - num_h - 2.0);
        let mut tapped = None;
        for b in 0..beats {
            if starts[b] {
                x += big - gap;
            }
            let slot = Rect::from_min_max(Pos2::new(x, top), Pos2::new(x + bw, rect.max.y));
            let lvl = self.mt.levels[b].min(3) as usize;
            let now = beat_no == Some(b) && !muted && lvl > 0;
            let resp = ui.interact(slot, ui.id().with(("beat", b)), Sense::click());
            let p = ui.painter();
            let well = Rect::from_min_max(Pos2::new(x, top), Pos2::new(x + bw, base));
            p.rect_filled(well, 0.0, pal().display);
            let full = well.height() - 4.0;
            let fh = [0.0, 0.34, 0.67, 1.0][lvl] * full;
            if lvl == 0 {
                p.rect_stroke(Rect::from_min_max(Pos2::new(x + 2.0, base - 6.0), Pos2::new(x + bw - 2.0, base - 2.0)), 1.0, Stroke::new(1.0_f32, pal().dim));
            } else {
                let bar = Rect::from_min_max(Pos2::new(x + 2.0, base - 2.0 - fh), Pos2::new(x + bw - 2.0, base - 2.0));
                let col = if now { mix(pal().red, pal().ink2, (frac / 0.6).clamp(0.0, 1.0)) } else { pal().ink2 };
                p.rect_filled(bar, 0.0, col);
            }
            if now {
                p.rect_stroke(well, 0.0, Stroke::new(2.0_f32, pal().red));
            }
            if num_h > 0.0 {
                ptext(p, Pos2::new(slot.center().x, base + num_h / 2.0 + 1.0), Align::Center, &(b + 1).to_string(), 1.0, if now { pal().red } else { pal().dim });
            }
            if resp.clicked() {
                tapped = Some(b);
            }
            x += bw + gap;
        }
        if muted {
            ptext(ui.painter(), rect.min + Vec2::new(6.0, 8.0), Align::Min, "MUTED", 1.0, pal().red);
        }
        if let Some(b) = tapped {
            self.mt_group = 0;
            let l = &mut self.mt.levels[b.min(31)];
            *l = if *l == 0 { 3 } else { *l - 1 };
            self.save_settings();
        }
    }
}

/// Between two colours (0: the first, 1: the second).
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}
