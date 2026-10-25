//! Floating tools (focus timer, metronome), the lead sheet with its band, and the typed number boxes.

use crate::band;
use crate::chart;
use crate::extras::{Knob, Opt};
use crate::font::{ptext, ptext_fit, text_w};
use crate::views::{dim_line, field, label, F_CHART, F_IREAL};
use crate::{bh, check_box, fill_rect, inset, lcd_box, outline, pal, para, retro_btn, retro_btn_w, Action, App, Tip};
use eframe::egui::{self, Align, Color32, Pos2, Rect, Sense, Vec2};
use std::time::{Duration, Instant};

/// Playback speed presets, in percent (any other speed can be typed).
pub(crate) const SPEEDS: [u32; 7] = [50, 75, 100, 125, 150, 200, 250];

/// "1X", "1.25X", "0.5X"
pub(crate) fn speed_label(pct: u32) -> String {
    let s = format!("{:.2}", pct as f32 / 100.0);
    format!("{}X", s.trim_end_matches('0').trim_end_matches('.'))
}

/// The next preset up, wrapping round to the slowest.
pub(crate) fn next_speed(pct: u32) -> u32 {
    SPEEDS.iter().copied().find(|&s| s > pct).unwrap_or(SPEEDS[0])
}

// text-field ids for the boxes below (the lists use 1..13)
/// The boxes that hold a number (tempo, beats, times...): no cut/copy/paste menu on them.
pub fn is_number_field(id: u32) -> bool {
    matches!(
        id,
        F_FOCUS..=F_SPEED
            | F_TR_LOOPS
            | F_TR_STEP
            | F_BAND_BPM
            | F_UNIT
            | F_INFO_KEY
            | F_INFO_BPM
            | F_T_YEAR
            | F_LN_BPM
            | F_LN_SUB
            | F_RD_BPM
            | F_PR_BPM
            | F_PITCH
            | F_LONG_EVERY
            | F_LONG_MINS
    )
}

pub const F_FOCUS: u32 = 30;
pub const F_REST: u32 = 31;
pub const F_BLOCKS: u32 = 32;
pub const F_BPM: u32 = 33;
pub const F_BEATS: u32 = 34;
pub const F_GAP_PLAY: u32 = 35;
pub const F_GAP_MUTE: u32 = 36;
pub const F_RAMP_BARS: u32 = 37;
pub const F_RAMP_BPM: u32 = 38;
pub const F_SHIFT: u32 = 39;
pub const F_LOOP_A: u32 = 40;
pub const F_LOOP_B: u32 = 41;
pub const F_SPEED: u32 = 42;
pub const F_TR_LOOPS: u32 = 43;
pub const F_TR_STEP: u32 = 44;
pub const F_CHART_Q: u32 = 45;
pub const F_BAND_BPM: u32 = 46;
pub const F_UNIT: u32 = 47;
pub const F_TUNE_IREAL: u32 = 48;
pub const F_LOOK: u32 = 49;
pub const F_WINAMP: u32 = 50;
pub const F_BAR: u32 = 51;
pub const F_CHORD: u32 = 52;
pub const F_INFO_TITLE: u32 = 53;
pub const F_INFO_ARTIST: u32 = 54;
pub const F_CL_CHANGES: u32 = 55;
pub const F_T_WRITER: u32 = 56;
pub const F_T_YEAR: u32 = 59;
pub const F_T_FROM: u32 = 60;
pub const F_T_STYLE: u32 = 61;
pub const F_PROG_NEW: u32 = 62;
pub const F_LN_BPM: u32 = 66;
pub const F_LN_SUB: u32 = 67;
pub const F_CB_NAME: u32 = 68;
pub const F_CLIP_NAME: u32 = 69;
pub const F_CLIP_NOTE: u32 = 70;
pub const F_TR_PROG: u32 = 71;
pub const F_RD_BPM: u32 = 72;
pub const F_TIDAL_PL: u32 = 73;
pub const F_GRIP_NAME: u32 = 74;
pub const F_CLIP_GROUP: u32 = 75;
pub const F_GROUP_NAME: u32 = 76;
/// the filter over your Tidal library (MY TRACKS, PLAYLISTS, ALBUMS, ARTISTS)
pub const F_LIB_FILTER: u32 = 77;
/// the song's tempo in practice (next to the count-off)
pub const F_PR_BPM: u32 = 78;
/// the name for an equalizer curve you save
pub const F_EQ_NAME: u32 = 79;
/// the Tidal search box
pub const F_SEARCH: u32 = 80;
/// the filter over your saved loops
pub const F_LOOPS_FILTER: u32 = 81;
/// the pitch, typed (semitones like +0.25, or cents like -30c)
pub const F_PITCH: u32 = 82;
pub const F_INFO_KEY: u32 = 57;
pub const F_INFO_ALBUM: u32 = 90;
pub const F_INFO_FILE: u32 = 91;
/// the tag of each take (plus its index)
pub const F_TAKE_TAG: u32 = 2000;
pub const F_INFO_BPM: u32 = 58;
/// the focus timer's long rest: after how many blocks, and how long
pub const F_LONG_EVERY: u32 = 96;
pub const F_LONG_MINS: u32 = 97;
/// MY PLAN: a new keeper
pub const F_PLAN_KEEPER: u32 = 98;
/// DIARY: practice added by hand (what, minutes)
pub const F_DIARY_ADD_NAME: u32 = 101;
pub const F_DIARY_ADD_MIN: u32 = 102;

impl App {
    /// A box you can type into. Shows `shown`; returns the typed text when you press Enter or click away.
    pub(crate) fn entry(&mut self, ui: &mut egui::Ui, id: u32, w: f32, shown: &str) -> Option<String> {
        // never narrower than what it shows
        let w = (w * crate::font::ui_scale()).max(crate::text_w(shown, 2.0) + 18.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
        let editing = self.ed.id == id;
        if editing && self.ebuf_id != id {
            self.ebuf = shown.to_string();
            self.ebuf_id = id;
        }
        let out = if editing {
            field(ui, &mut self.ed, id, &mut self.ebuf, rect, "", false)
        } else {
            let mut tmp = shown.to_string();
            field(ui, &mut self.ed, id, &mut tmp, rect, "", false)
        };
        if self.ebuf_id == id && (out.enter || self.ed.id != id) {
            if out.enter {
                self.ed.id = 0;
            }
            self.ebuf_id = 0;
            return Some(std::mem::take(&mut self.ebuf));
        }
        None
    }

    /// `-  [typed number]  +`
    pub(crate) fn num_step(
        &mut self,
        ui: &mut egui::Ui,
        acts: &mut Vec<Action>,
        id: u32,
        w: f32,
        k: Knob,
        shown: String,
        tip: &str,
    ) {
        if retro_btn_w(ui, "-", 28.0, false).tip(tip).clicked() {
            acts.push(Action::Knob(k, -1));
        }
        if let Some(t) = self.entry(ui, id, w, &shown) {
            acts.push(Action::SetKnob(k, t));
        }
        if retro_btn_w(ui, "+", 28.0, false).clicked() {
            acts.push(Action::Knob(k, 1));
        }
        ui.add_space(6.0);
    }

    /// The speed presets and a box for any speed (25 to 250 percent), for a speed button's right-click menu.
    pub(crate) fn speed_menu(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        for s in SPEEDS {
            let mark = if self.speed == s { "> " } else { "   " };
            if crate::menu_item(ui, &format!("{}{}", mark, speed_label(s))) {
                acts.push(Action::Speed(s));
                ui.close_menu();
            }
        }
        ui.horizontal(|ui| {
            self.num_step(
                ui,
                acts,
                F_SPEED,
                64.0,
                Knob::Speed,
                format!("{}%", self.speed),
                "Type any speed as a percent, from 25 to 250.",
            );
        });
    }

    // ---------------------------------------------------------------- TIMER
    /// Flash border, the focus timer and the metronome (each opens from its pill; one big panel at a time).
    pub(crate) fn floating_tools(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        if !crate::PRACTICE {
            return;
        }
        self.timer_logic(acts);
        let flash = self.pomo_flash.map_or(false, |t| self.pomo == 3 || t.elapsed().as_secs() < 6);
        let blink = flash && (ctx.input(|i| i.time) * 2.5) as u32 % 2 == 0;
        if flash {
            ctx.request_repaint_after(Duration::from_millis(200));
            if blink {
                let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("flash")));
                outline(&p, ctx.screen_rect(), 6.0, pal().ink);
            }
        }
        // only one of the two big panels open: the one opened last stays
        if self.timer_open && self.metro_open {
            if self.tm.was_open.0 {
                self.timer_open = false;
            } else {
                self.metro_open = false;
            }
        }
        self.tm.was_open = (self.timer_open, self.metro_open);
        // the pills drawn this frame or the last (the practice window's title strip)
        let fr = ctx.cumulative_pass_nr();
        let fresh = |p: Option<(u64, Rect)>| p.filter(|(f, _)| *f + 1 >= fr).map(|x| x.1);
        let tpill = fresh(self.tm.timer_pill);
        let mpill = fresh(self.tm.metro_pill);
        let screen = ctx.screen_rect();
        // no pill on show (the practice window shut): a running timer keeps a pill in the corner
        let corner = if tpill.is_none() && (self.pomo != 0 || self.timer_open) {
            let size = Vec2::new(200.0, 26.0);
            let at = Pos2::new(screen.max.x - 10.0 - size.x, screen.max.y - 10.0 - size.y);
            egui::Area::new(egui::Id::new("timer_corner")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
                let (r, _) = ui.allocate_exact_size(size, Sense::hover());
                self.draw_timer_pill(ui, r, acts);
            });
            Some(Rect::from_min_size(at, size))
        } else {
            None
        };
        let place = |area: egui::Area, pill: Option<Rect>| match (pill, corner) {
            (Some(p), _) => area.pivot(egui::Align2::RIGHT_TOP).fixed_pos(Pos2::new(p.max.x, p.max.y + 6.0)),
            (None, Some(c)) => area.pivot(egui::Align2::RIGHT_BOTTOM).fixed_pos(Pos2::new(c.max.x, c.min.y - 6.0)),
            _ => area.anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0]),
        };
        let mut panel: Option<Rect> = None;
        if self.timer_open {
            let area = place(egui::Area::new(egui::Id::new("timer")).order(egui::Order::Foreground).constrain(true), tpill);
            panel = Some(area.show(ctx, |ui| self.timer_box(ui, acts, blink)).response.rect);
        } else if self.metro_open {
            let area = place(egui::Area::new(egui::Id::new("metro")).order(egui::Order::Foreground).constrain(true), mpill);
            panel = Some(area.show(ctx, |ui| self.metro_box(ui, acts)).response.rect);
        }
        // Esc shuts the open panel (a click elsewhere leaves it open: you play along with it; its X and its pill close it)
        if panel.is_some() {
            let _ = (tpill, mpill, corner);
            let esc = self.ed.id == 0 && ctx.input(|i| i.key_pressed(egui::Key::Escape));
            if esc {
                acts.push(if self.timer_open { Action::TimerPanel } else { Action::MetroPanel });
            }
        }
    }

    /// What the timer does around the blocks: the minutes into the diary, a long rest, the music, the next block.
    fn timer_logic(&mut self, acts: &mut Vec<Action>) {
        let (prev, now) = (self.tm.last, self.pomo);
        if prev != now {
            let today = crate::store::today_str();
            match (prev, now) {
                // a focus block begins
                (0 | 3, 1) => {
                    self.tm.total = self.pomo_focus as u64 * 60;
                    self.tm.base = self.store.secs_on(&today);
                    if prev == 3 && self.tm.paused_by_us && self.paused && self.cur.is_some() && !self.stopped {
                        acts.push(Action::PauseBtn);
                    }
                    self.tm.paused_by_us = false;
                }
                // a focus block done (the timer ran out, not STOP)
                (1, 2) | (1, 0) if self.pomo_flash.is_some() => {
                    if self.tm.log_diary {
                        // what the diary didn't already count while you played along
                        let logged = self.store.secs_on(&today).saturating_sub(self.tm.base);
                        let add = (self.tm.total as u32).saturating_sub(logged);
                        if add >= 60 {
                            let label = self.practice_label();
                            *self.store.day_mut(&today).secs.entry(label).or_insert(0) += add;
                            self.store_dirty = true;
                        }
                    }
                    if self.tm.was_playing && self.paused {
                        if !self.tm.rest_pause {
                            acts.push(Action::PauseBtn);
                        } else if now == 2 {
                            self.tm.paused_by_us = true;
                        }
                    }
                    if now == 2 {
                        let long = self.tm.long_every > 0 && self.pomo_n % self.tm.long_every == 0;
                        let mins = if long { self.tm.long_mins } else { self.pomo_break } as u64;
                        if long {
                            self.pomo_end = Some(Instant::now() + Duration::from_secs(mins * 60));
                            self.set_note(&format!("LONG REST FOR {} MIN. GET UP AND MOVE ABOUT", mins));
                        }
                        self.tm.total = mins * 60;
                    }
                }
                (2, 3) if self.tm.auto_next => acts.push(Action::Pomo),
                // carried over from last time
                (0, 4) => {
                    self.tm.total = if self.pomo_was == 1 { self.pomo_focus } else { self.pomo_break } as u64 * 60;
                    self.tm.base = self.store.secs_on(&today);
                }
                _ => {}
            }
            self.tm.last = now;
        }
        self.tm.was_playing = self.cur.is_some() && !self.stopped && !self.paused;
    }

    /// Seconds left in this part of the timer.
    fn pomo_secs_left(&self) -> u64 {
        match self.pomo {
            1 | 2 => self.pomo_end.map_or(0, |e| e.saturating_duration_since(Instant::now()).as_secs()),
            4 => self.pomo_left,
            _ => 0,
        }
    }

    /// Five more minutes on this block or rest.
    fn pomo_add5(&mut self) {
        match self.pomo {
            1 | 2 => {
                if let Some(e) = self.pomo_end.as_mut() {
                    *e += Duration::from_secs(300);
                }
            }
            4 => self.pomo_left += 300,
            _ => return,
        }
        self.tm.total += 300;
        self.dirty = true;
    }

    /// End this block (or rest) now.
    fn pomo_skip(&mut self) {
        let left = self.pomo_secs_left();
        match self.pomo {
            1 | 2 => {}
            4 => self.pomo = self.pomo_was.clamp(1, 2),
            _ => return,
        }
        self.tm.total = self.tm.total.saturating_sub(left);
        self.pomo_end = Some(Instant::now());
        self.dirty = true;
    }

    /// The timer as a small pill (for a title strip): the dot starts or pauses it, the words open the full timer.
    pub(crate) fn timer_pill(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        self.tm.timer_pill = Some((ui.ctx().cumulative_pass_nr(), r));
        self.draw_timer_pill(ui, r, acts);
    }

    fn draw_timer_pill(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        if matches!(self.pomo, 1 | 2 | 3) {
            ui.ctx().request_repaint_after(Duration::from_millis(500));
        }
        let id = ui.id().with("timer_pill");
        let split = r.min.x + r.height();
        let left = Rect::from_min_max(r.min, Pos2::new(split, r.max.y));
        let right = Rect::from_min_max(Pos2::new(split, r.min.y), r.max);
        let go_tip = match self.pomo {
            0 => "Start a focus block.",
            3 => "Start the next focus block.",
            4 => "Carry on.",
            _ => "Pause the timer.",
        };
        let lr = ui.interact(left, id.with("go"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(go_tip);
        let rr = ui
            .interact(right, id.with("open"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("Focus timer: work in blocks with short rests between. Click to open it.");
        let p = ui.painter().clone();
        pill_face(&p, r, self.timer_open, lr.hovered() || rr.hovered());
        let t = ui.input(|i| i.time);
        let dot = Rect::from_center_size(left.center() + Vec2::new(2.0, 0.0), Vec2::splat(r.height() * 0.42));
        match self.pomo {
            1 | 2 => {
                p.circle_filled(dot.center(), r.height() * 0.17, if self.pomo == 1 { pal().red } else { pal().ink2 });
            }
            3 => {
                if (t * 2.5) as u32 % 2 == 0 {
                    p.circle_filled(dot.center(), r.height() * 0.17, pal().red);
                }
            }
            4 => crate::vicon::named(&p, dot, "pause", pal().dim),
            _ => crate::vicon::named(&p, dot, "play", if lr.hovered() { pal().ink } else { pal().ink2 }),
        }
        let left_s = self.pomo_secs_left();
        let clock = format!("{}:{:02}", left_s / 60, left_s % 60);
        let (main, tail) = match self.pomo {
            1 => (format!("FOCUS {}", clock), format!("{}/{}", self.pomo_n, self.pomo_cycles)),
            2 => (format!("REST {}", clock), String::new()),
            3 => ("REST OVER".to_string(), String::new()),
            4 => (format!("PAUSED {}", clock), format!("{}/{}", self.pomo_n, self.pomo_cycles)),
            _ => ("TIMER".to_string(), String::new()),
        };
        pill_words(&p, right, &main, &tail, if self.pomo == 4 { pal().dim } else { pal().ink });
        if lr.clicked() {
            acts.push(match self.pomo {
                0 | 3 => Action::Pomo,
                _ => Action::PomoHold,
            });
        }
        if rr.clicked() {
            if !self.timer_open {
                self.metro_open = false;
            }
            acts.push(Action::TimerPanel);
        }
    }

    /// The full focus timer: a ring counting down, the controls, and the settings.
    fn timer_box(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, blink: bool) {
        panel_frame().show(ui, |ui| {
            let w = 320.0;
            ui.set_width(w);
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 10.0);
            let running = matches!(self.pomo, 1 | 2 | 3 | 4);
            // title, settings while it runs, close
            ui.horizontal(|ui| {
                let (tr, _) = ui.allocate_exact_size(Vec2::new(160.0, bh()), Sense::hover());
                panel_title(ui.painter(), tr, "FOCUS TIMER");
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ibutton(ui, "close", "Close", false).clicked() {
                        acts.push(Action::TimerPanel);
                    }
                    if running && ibutton(ui, "gear", "Settings", self.tm.show_set).clicked() {
                        self.tm.show_set = !self.tm.show_set;
                    }
                });
            });
            // the ring: what is left of this part
            let left = self.pomo_secs_left();
            let total = match self.tm.total {
                0 => (if self.pomo == 2 || self.pomo_was == 2 && self.pomo == 4 { self.pomo_break } else { self.pomo_focus }) as u64 * 60,
                t => t,
            };
            let (frac, clock, phase) = match self.pomo {
                1 | 2 | 4 => {
                    let ph = match (self.pomo, self.pomo_was) {
                        (1, _) | (4, 1) => "FOCUS",
                        _ => "REST",
                    };
                    let ph = if self.pomo == 4 { format!("{} - PAUSED", ph) } else { ph.to_string() };
                    ((left as f32 / total.max(1) as f32).clamp(0.0, 1.0), format!("{}:{:02}", left / 60, left % 60), ph)
                }
                3 => (0.0, "0:00".to_string(), "REST OVER".to_string()),
                _ => (1.0, format!("{}:00", self.pomo_focus), "READY".to_string()),
            };
            let (rr, _) = ui.allocate_exact_size(Vec2::new(w, 150.0), Sense::hover());
            let c = rr.center();
            let rad = 62.0;
            let p = ui.painter();
            p.circle_stroke(c, rad, egui::Stroke::new(7.0_f32, pal().groove));
            let col = match self.pomo {
                1 => pal().red,
                4 => pal().dim,
                _ => pal().ink2,
            };
            if frac > 0.0 {
                let n = (72.0 * frac).ceil().max(2.0) as usize;
                let pts: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * frac * i as f32 / n as f32;
                        c + Vec2::new(a.cos(), a.sin()) * rad
                    })
                    .collect();
                p.add(egui::Shape::line(pts, egui::Stroke::new(7.0_f32, col)));
            }
            let ink = if self.pomo == 3 && blink { pal().red } else { pal().ink };
            ptext(p, c + Vec2::new(0.0, -26.0), Align::Center, &phase, 1.0, pal().ink2);
            ptext(p, c, Align::Center, &clock, 3.0, ink);
            let blocks = match self.pomo {
                0 => format!("{} BLOCKS", self.pomo_cycles),
                3 => format!("NEXT: {} OF {}", self.pomo_n + 1, self.pomo_cycles),
                _ => format!("BLOCK {} OF {}", self.pomo_n, self.pomo_cycles),
            };
            ptext(p, c + Vec2::new(0.0, 26.0), Align::Center, &blocks, 1.0, pal().ink2);
            // the controls, in the middle
            ui.horizontal(|ui| {
                match self.pomo {
                    0 => {
                        ui.add_space((w - 120.0) / 2.0);
                        if icon_pill(ui, "play", "START", 120.0, false).tip("Start the first focus block.").clicked() {
                            acts.push(Action::Pomo);
                        }
                    }
                    3 => {
                        ui.add_space((w - 150.0 - 8.0 - bh()) / 2.0);
                        if icon_pill(ui, "play", "NEXT FOCUS", 150.0, blink).tip("Start the next focus block.").clicked() {
                            acts.push(Action::Pomo);
                        }
                        if ibutton(ui, "stop", "Stop the timer", false).clicked() {
                            acts.push(Action::Pomo);
                            acts.push(Action::Pomo);
                        }
                    }
                    _ => {
                        ui.add_space((w - 4.0 * bh() - 3.0 * 14.0) / 2.0);
                        ui.spacing_mut().item_spacing.x = 14.0;
                        let held = self.pomo == 4;
                        let (ic, tip) = if held { ("play", "Carry on") } else { ("pause", "Pause the timer") };
                        if ibutton(ui, ic, tip, held).clicked() {
                            acts.push(Action::PomoHold);
                        }
                        if ibutton(ui, "plus5", "Five more minutes", false).clicked() {
                            self.pomo_add5();
                        }
                        let skip = if self.pomo == 2 || held && self.pomo_was == 2 { "Skip the rest" } else { "End this block now" };
                        if ibutton(ui, "skip", skip, false).clicked() {
                            self.pomo_skip();
                        }
                        if ibutton(ui, "stop", "Stop and reset the timer", false).clicked() {
                            acts.push(Action::Pomo);
                        }
                    }
                }
            });
            if running && !self.tm.show_set {
                return;
            }
            ui.add_space(4.0);
            // presets, then your own lengths
            ui.horizontal(|ui| {
                label(ui, "PRESET", 70.0);
                for (f, r) in [(25u32, 5u32), (50, 10)] {
                    let on = self.pomo_focus == f && self.pomo_break == r;
                    if retro_btn_w(ui, &format!("{} / {}", f, r), 76.0, on)
                        .tip(format!("{} minutes of focus, {} of rest.", f, r))
                        .clicked()
                    {
                        acts.push(Action::SetKnob(Knob::Focus, f.to_string()));
                        acts.push(Action::SetKnob(Knob::Rest, r.to_string()));
                    }
                }
            });
            ui.horizontal(|ui| {
                label(ui, "FOCUS", 70.0);
                let s = self.pomo_focus.to_string();
                self.num_step(ui, acts, F_FOCUS, 44.0, Knob::Focus, s, "Minutes of focus.");
                label(ui, "MIN", 30.0);
            });
            ui.horizontal(|ui| {
                label(ui, "REST", 70.0);
                let s = self.pomo_break.to_string();
                self.num_step(ui, acts, F_REST, 44.0, Knob::Rest, s, "Minutes of rest.");
                label(ui, "MIN", 30.0);
            });
            ui.horizontal(|ui| {
                label(ui, "BLOCKS", 70.0);
                let s = self.pomo_cycles.to_string();
                self.num_step(ui, acts, F_BLOCKS, 44.0, Knob::Blocks, s, "How many focus blocks to do.");
            });
            ui.horizontal(|ui| {
                label(ui, "LONG REST", 70.0);
                let v = self.tm.long_every;
                let shown = if v == 0 { "OFF".to_string() } else { v.to_string() };
                if let Some(n) = self.step_u32(ui, F_LONG_EVERY, v, 0, 16, 44.0, shown, "A longer rest after every this many blocks (0 is off).") {
                    self.tm.long_every = n;
                    self.dirty = true;
                }
                if self.tm.long_every > 0 {
                    let v = self.tm.long_mins;
                    if let Some(n) = self.step_u32(ui, F_LONG_MINS, v, 1, 90, 40.0, v.to_string(), "Minutes of the long rest.") {
                        self.tm.long_mins = n;
                        self.dirty = true;
                    }
                    label(ui, "MIN", 30.0);
                }
            });
            ui.add_space(2.0);
            if check_box(ui, "SOUND", self.pomo_sound)
                .tip("Play a soft chime when a block ends. Off, the timer just flashes.")
                .clicked()
            {
                acts.push(Action::Opt(Opt::PomoSound));
            }
            let flags: [(&str, &str, u8); 3] = [
                ("START NEXT BLOCK BY ITSELF", "When a rest ends, the next focus block starts without waiting for you.", 0),
                ("PAUSE THE MUSIC IN RESTS", "The song pauses when a block ends and plays on when the next one starts.", 1),
                ("ADD FOCUS TIME TO THE DIARY", "A finished block counts as practice in today's diary (time already counted from playing isn't counted twice).", 2),
            ];
            for (name, tip, k) in flags {
                let on = [self.tm.auto_next, self.tm.rest_pause, self.tm.log_diary][k as usize];
                if check_box(ui, name, on).tip(tip).clicked() {
                    match k {
                        0 => self.tm.auto_next = !on,
                        1 => self.tm.rest_pause = !on,
                        _ => self.tm.log_diary = !on,
                    }
                    self.dirty = true;
                }
            }
        });
    }

    /// `-  [typed number]  +` for a whole number kept here; returns the new value.
    #[allow(clippy::too_many_arguments)]
    fn step_u32(
        &mut self,
        ui: &mut egui::Ui,
        id: u32,
        v: u32,
        lo: u32,
        hi: u32,
        w: f32,
        shown: String,
        tip: &str,
    ) -> Option<u32> {
        let mut out = None;
        if retro_btn_w(ui, "-", 28.0, false).tip(tip).clicked() {
            out = Some(v.saturating_sub(1).max(lo));
        }
        if let Some(t) = self.entry(ui, id, w, &shown) {
            let t = t.trim();
            if let Ok(n) = t.parse::<u32>() {
                out = Some(n.clamp(lo, hi));
            } else if t.eq_ignore_ascii_case("off") {
                out = Some(lo);
            }
        }
        if retro_btn_w(ui, "+", 28.0, false).clicked() {
            out = Some((v + 1).min(hi));
        }
        ui.add_space(6.0);
        out
    }

    // ------------------------------------------------------------ METRONOME
    /// The metronome as a small pill: the left part starts or stops it, the rest opens the full metronome.
    pub(crate) fn metro_pill(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        self.tm.metro_pill = Some((ui.ctx().cumulative_pass_nr(), r));
        let on = self.mt.on;
        let id = ui.id().with("metro_pill");
        let split = r.min.x + r.height();
        let left = Rect::from_min_max(r.min, Pos2::new(split, r.max.y));
        let right = Rect::from_min_max(Pos2::new(split, r.min.y), r.max);
        let lr = ui.interact(left, id.with("go"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(if on {
            "Stop the metronome."
        } else {
            "Start the metronome."
        });
        let rr = ui
            .interact(right, id.with("open"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("Metronome: meter, accents, subdivisions, trainer. Click to open it.");
        let p = ui.painter().clone();
        pill_face(&p, r, self.metro_open, lr.hovered() || rr.hovered());
        let dot = Rect::from_center_size(left.center() + Vec2::new(2.0, 0.0), Vec2::splat(r.height() * 0.4));
        if on {
            ui.ctx().request_repaint();
            // the dot flashes on the beat
            let (beat, frac, muted) = self.metro_pos();
            let hit = if beat.is_some() && !muted { (1.0 - frac / 0.35).clamp(0.0, 1.0) } else { 0.0 };
            p.circle_filled(dot.center(), r.height() * (0.14 + 0.06 * hit), mix(pal().ink2, pal().red, hit));
        } else {
            crate::vicon::named(&p, dot, "play", if lr.hovered() { pal().ink } else { pal().ink2 });
        }
        // a note, the tempo, the meter
        let note = Rect::from_center_size(Pos2::new(right.min.x + 9.0, right.center().y), Vec2::splat(r.height() * 0.5));
        crate::vicon::named(&p, note, "note", pal().ink2);
        let words = Rect::from_min_max(Pos2::new(note.max.x + 2.0, r.min.y), r.max);
        pill_words(&p, words, &self.mt_bpm.to_string(), &format!("{}/{}", self.mt.beats, self.mt.unit), pal().ink);
        if lr.clicked() {
            acts.push(Action::Opt(Opt::MetroOn));
        }
        if rr.clicked() {
            if !self.metro_open {
                self.timer_open = false;
            }
            acts.push(Action::MetroPanel);
        }
    }

    fn metro_box(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        panel_frame().show(ui, |ui| {
            let full_w = (ui.ctx().screen_rect().width() - 48.0).clamp(300.0, 680.0);
            ui.set_width(full_w);
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 10.0);
            let on = self.mt.on;
            // the top bar: start, tempo, tap, the meter, close
            ui.horizontal_wrapped(|ui| {
                if icon_pill(ui, if on { "stop" } else { "play" }, if on { "STOP" } else { "START" }, 96.0, on).clicked() {
                    acts.push(Action::Opt(Opt::MetroOn));
                }
                ui.add_space(6.0);
                label(ui, "BPM", 36.0);
                let s = self.mt_bpm.to_string();
                self.num_step(
                    ui,
                    acts,
                    F_BPM,
                    52.0,
                    Knob::MetroBpm,
                    s,
                    "The metronome's own tempo. The song's tempo is set apart.",
                );
                if retro_btn_w(ui, "TAP", 52.0, false).tip("Tap along with the music to find the tempo.").clicked() {
                    acts.push(Action::TapTempo);
                }
                let gs = band::groupings(self.mt.beats);
                let meter = match self.mt_group {
                    k if k > 0 && k <= gs.len() => format!(
                        "{}/{}  {}",
                        self.mt.beats,
                        self.mt.unit,
                        gs[k - 1].iter().map(|n| n.to_string()).collect::<Vec<_>>().join("+")
                    ),
                    _ => format!("{}/{}", self.mt.beats, self.mt.unit),
                };
                lcd_box(ui, &meter, 60.0, pal().ink);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ibutton(ui, "close", "Close", false).clicked() {
                        acts.push(Action::MetroPanel);
                    }
                });
            });
            // the pendulum and the beats: each bar's height is that beat's volume; click one to change it
            ui.horizontal(|ui| {
                let h = 76.0;
                let (pr, _) = ui.allocate_exact_size(Vec2::new(h, h), Sense::hover());
                self.pendulum(ui, pr);
                let w = ui.available_width();
                self.beat_bars(ui, acts, w, h);
            });
            if on {
                ui.ctx().request_repaint();
            }
            // four boxes: meter, sound, trainer, with the track
            let two = full_w >= 600.0;
            let col_w = if two { (full_w - 10.0) / 2.0 } else { full_w };
            let meter_box = |me: &mut Self, ui: &mut egui::Ui, acts: &mut Vec<Action>| {
                group_box(ui, "METER", col_w, |ui| me.metro_meter(ui, acts));
            };
            if two {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        meter_box(self, ui, acts);
                        group_box(ui, "TRAINER", col_w, |ui| self.metro_trainer(ui, acts));
                    });
                    ui.vertical(|ui| {
                        group_box(ui, "SOUND", col_w, |ui| self.metro_sound(ui, acts));
                        group_box(ui, "WITH THE TRACK", col_w, |ui| self.metro_track(ui, acts));
                    });
                });
            } else {
                meter_box(self, ui, acts);
                group_box(ui, "SOUND", col_w, |ui| self.metro_sound(ui, acts));
                group_box(ui, "TRAINER", col_w, |ui| self.metro_trainer(ui, acts));
                group_box(ui, "WITH THE TRACK", col_w, |ui| self.metro_track(ui, acts));
            }
        });
    }

    /// Beats per bar, what one beat is, and the accent groups.
    fn metro_meter(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.horizontal_wrapped(|ui| {
            let s = self.mt.beats.to_string();
            self.num_step(
                ui,
                acts,
                F_BEATS,
                36.0,
                Knob::Beats,
                s,
                "Beats per bar, 1 to 32. You can also type a whole meter here, like 11/17.",
            );
            label(ui, "/", 14.0);
            let shown = self.mt.unit.to_string();
            if let Some(txt) = self.entry(ui, F_UNIT, 44.0, &shown) {
                acts.push(Action::SetKnob(Knob::Unit, txt));
            }
        });
        ui.horizontal_wrapped(|ui| {
            for n in [4u8, 8, 16] {
                if retro_btn_w(ui, &n.to_string(), 38.0, self.mt.unit == n)
                    .tip("What one beat is: 4 means quarter notes, 8 means eighths. You can type any number (11/17 works).")
                    .clicked()
                {
                    acts.push(Action::Opt(Opt::UnitSet(n)));
                }
            }
        });
        let gs = band::groupings(self.mt.beats);
        if gs.len() > 1 {
            ui.horizontal_wrapped(|ui| {
                label(ui, "ACCENT", 60.0);
                let txt = match self.mt_group {
                    k if k > 0 && k <= gs.len() => gs[k - 1].iter().map(|n| n.to_string()).collect::<Vec<_>>().join("+"),
                    _ => "GROUPS".to_string(),
                };
                if retro_btn_w(ui, &txt, 140.0, self.mt_group > 0)
                    .tip("Accent a grouping for odd meters, such as 7 = 2+2+3 or 22 = 3+3+3+3+3+3+4. Click to try the next one.")
                    .clicked()
                {
                    acts.push(Action::Opt(Opt::Group));
                }
            });
        }
    }

    /// Subdivisions, beat-volume presets and the metronome's own volume.
    fn metro_sound(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let sub = ["NO SUBDIV", "EIGHTHS", "TRIPLETS", "SIXTEENTHS"][(self.mt.sub as usize).min(3)];
        if retro_btn_w(ui, sub, 130.0, self.mt.sub > 0)
            .tip("Add quiet ticks between the beats. Click for the next kind.")
            .clicked()
        {
            acts.push(Action::Opt(Opt::Sub));
        }
        ui.horizontal_wrapped(|ui| {
            for (i, name) in ["ALL", "2 & 4", "1 ONLY"].iter().enumerate() {
                if retro_btn_w(ui, name, 66.0, false).tip("Preset beat volumes. Click a beat to set its own.").clicked() {
                    acts.push(Action::Opt(Opt::Preset(i as u8)));
                }
            }
        });
        // its own volume: louder or softer against the song, without touching the main volume
        ui.horizontal(|ui| {
            label(ui, "VOL", 36.0);
            let (r, resp) =
                ui.allocate_exact_size(Vec2::new((ui.available_width() - 8.0).clamp(80.0, 180.0), bh()), Sense::click_and_drag());
            inset(ui.painter(), r, pal().lcd);
            let inner = r.shrink(4.0);
            let fill = Rect::from_min_size(inner.min, Vec2::new(inner.width() * self.mt_vol, inner.height()));
            ui.painter().rect_filled(fill, if crate::winamp_ui::chrome_on() { 3.0 } else { 0.0 }, pal().ink2);
            let resp = resp
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
                .tip("The metronome's own volume. The main volume doesn't change it.");
            if let Some(p) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
                self.mt_vol = ((p.x - inner.min.x) / inner.width()).clamp(0.0, 1.0);
                self.player.send(crate::player::Cmd::MetroVolume(self.mt_vol * self.mt_vol));
                self.dirty = true;
            }
        });
    }

    /// Bars of click then bars of silence; getting faster every few bars.
    fn metro_trainer(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.horizontal_wrapped(|ui| {
            label(ui, "GAP PLAY", 80.0);
            let p = if self.mt.gap_play == 0 { "OFF".to_string() } else { self.mt.gap_play.to_string() };
            self.num_step(ui, acts, F_GAP_PLAY, 44.0, Knob::GapPlay, p, "Bars of click before the silence (0 is off).");
        });
        ui.horizontal_wrapped(|ui| {
            label(ui, "THEN MUTE", 80.0);
            let m = self.mt.gap_mute.to_string();
            self.num_step(ui, acts, F_GAP_MUTE, 36.0, Knob::GapMute, m, "Bars of silence.");
        });
        ui.horizontal_wrapped(|ui| {
            label(ui, "FASTER", 80.0);
            let r = if self.mt.ramp_bars == 0 { "OFF".to_string() } else { self.mt.ramp_bars.to_string() };
            self.num_step(ui, acts, F_RAMP_BARS, 44.0, Knob::RampBars, r, "The tempo goes up every this many bars (0 is off).");
            label(ui, "BARS", 40.0);
        });
        ui.horizontal_wrapped(|ui| {
            label(ui, "BY", 80.0);
            let b = self.mt.ramp_bpm.to_string();
            self.num_step(ui, acts, F_RAMP_BPM, 36.0, Knob::RampBpm, b, "BPM added each time.");
            label(ui, "BPM", 40.0);
        });
    }

    /// Following the playing track: its speed, its beat.
    fn metro_track(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if self.cur.is_none() || self.stopped {
            dim_line(ui, "PLAY A TRACK TO CLICK ALONG WITH IT.", 1.0, pal().dim);
            return;
        }
        if check_box(ui, "FOLLOW SPEED", self.mt.follow).tip("The click slows down or speeds up with the track.").clicked() {
            acts.push(Action::Opt(Opt::Follow));
        }
        ui.horizontal_wrapped(|ui| {
            let tip = "Find the beat of this track and click along with it.";
            if check_box(ui, "LOCK TO TRACK", self.mt.sync).tip(tip).clicked() {
                acts.push(Action::Opt(Opt::Sync));
            }
            let busy = self.beat_busy;
            if retro_btn_w(ui, if busy { "LISTENING..." } else { "FIND BEAT" }, 110.0, busy).clicked() {
                acts.push(Action::Opt(Opt::FindBeats));
            }
        });
        if self.mt.sync {
            let tid = self.cur_track().map(|x| x.id);
            let (msg, red) = if self.beat_busy {
                ("LISTENING FOR THE BEAT...", false)
            } else if tid.map_or(false, |i| self.beat_failed.contains(&i)) {
                ("NO STEADY BEAT FOUND, SO YOUR OWN TEMPO IS USED", true)
            } else if self.beat.map(|b| b.0) == tid && self.beat.is_some() {
                ("LOCKED TO THE TRACK", false)
            } else {
                ("WAITING. PLAY THE TRACK ONCE TO STORE IT, THEN THE BEAT IS FOUND.", false)
            };
            dim_line(ui, msg, 1.0, if red { pal().red } else { pal().ink2 });
        }
        if self.beat.is_some() {
            ui.horizontal_wrapped(|ui| {
                label(ui, "TRACK BEAT", 90.0);
                let bpm = self.beat.map_or(0.0, |b| 60.0 / b.1);
                lcd_box(ui, &format!("{:.0} BPM", bpm), 80.0, pal().ink);
                if retro_btn_w(ui, "x2", 40.0, false).tip("Found half the tempo? Double it.").clicked() {
                    acts.push(Action::Opt(Opt::Double));
                }
                if retro_btn_w(ui, "/2", 40.0, false).tip("Found double the tempo? Halve it.").clicked() {
                    acts.push(Action::Opt(Opt::Half));
                }
            });
            ui.horizontal_wrapped(|ui| {
                label(ui, "SHIFT", 90.0);
                let s = format!("{}", self.mt.shift_ms);
                self.num_step(ui, acts, F_SHIFT, 52.0, Knob::Shift, s, "Nudge the click earlier or later, in milliseconds.");
                label(ui, "MS", 30.0);
            });
        }
    }

    /// Tempo shown in the box: the playing track's, else the metronome's own.
    fn knob_bpm(&self) -> u32 {
        if self.bpm > 0 {
            self.bpm
        } else {
            self.mt_bpm
        }
    }

    /// Where the metronome is in the bar: the beat, how far into it, and whether the gap trainer has it silent.
    fn metro_pos(&self) -> (Option<usize>, f32, bool) {
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

    /// A swinging pendulum in `rect`; its tip flashes on the beat.
    fn pendulum(&self, ui: &egui::Ui, rect: Rect) {
        let (_, frac, muted) = self.metro_pos();
        inset(ui.painter(), rect, pal().lcd);
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
        let rod = |ang: f32, col: egui::Color32| {
            let tip = pivot + Vec2::new(ang.sin() * len, -ang.cos() * len);
            p.line_segment([pivot, tip], egui::Stroke::new(2.0_f32, col));
            tip
        };
        // two faint afterimages so the motion reads as smooth
        if let Some(e) = phase {
            for (back, col) in [(0.16f32, pal().lcd_ghost), (0.08, pal().groove)] {
                let _ = rod(0.62 * (std::f32::consts::PI * (e - back)).cos(), col);
            }
        }
        let tip = rod(swing * 0.62, pal().ink2);
        let weight = pivot + (tip - pivot) * 0.7;
        fill_rect(p, Rect::from_center_size(weight, Vec2::new(14.0 * k, 10.0 * k)), pal().ink);
        let hit = if self.mt.on && !muted { (1.0 - frac / 0.35).clamp(0.0, 1.0) } else { 0.0 };
        p.circle_filled(tip, 6.0 * k + 1.0, pal().ink);
        p.circle_filled(tip, 4.0 * k + 0.5, mix(pal().lcd, pal().red, hit));
        fill_rect(p, Rect::from_center_size(pivot, Vec2::new(30.0 * k, 8.0 * k)), pal().ink);
    }

    /// The beats as bars: each one's height is its volume (loud, normal, quiet, silent), the accent groups set apart.
    /// Click a beat to step its volume.
    fn beat_bars(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, w: f32, h: f32) {
        let beats = self.mt.beats.clamp(1, 32) as usize;
        let (beat_no, frac, muted) = self.metro_pos();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
        let gs = band::groupings(self.mt.beats);
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
        let round = if crate::winamp_ui::chrome_on() { 3.0 } else { 0.0 };
        for b in 0..beats {
            if starts[b] {
                x += big - gap;
            }
            let slot = Rect::from_min_max(Pos2::new(x, top), Pos2::new(x + bw, rect.max.y));
            let lvl = self.mt.levels[b].min(3) as usize;
            let now = beat_no == Some(b) && !muted && lvl > 0;
            let resp = ui
                .interact(slot, ui.id().with(("beat", b)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .tip("Click to change this beat's volume: loud, normal, quiet or silent.");
            let p = ui.painter();
            let well = Rect::from_min_max(Pos2::new(x, top), Pos2::new(x + bw, base));
            p.rect_filled(well, round, pal().lcd);
            let full = well.height() - 4.0;
            let fh = [0.0, 0.34, 0.67, 1.0][lvl] * full;
            if lvl == 0 {
                p.rect_stroke(
                    Rect::from_min_max(Pos2::new(x + 2.0, base - 6.0), Pos2::new(x + bw - 2.0, base - 2.0)),
                    1.0,
                    egui::Stroke::new(1.0_f32, pal().dim),
                );
            } else {
                let bar = Rect::from_min_max(Pos2::new(x + 2.0, base - 2.0 - fh), Pos2::new(x + bw - 2.0, base - 2.0));
                let col = if now {
                    mix(pal().red, pal().ink2, (frac / 0.6).clamp(0.0, 1.0))
                } else if resp.hovered() {
                    pal().ink
                } else {
                    pal().ink2
                };
                p.rect_filled(bar, round, col);
            }
            if now {
                p.rect_stroke(well, round, egui::Stroke::new(2.0_f32, pal().red));
            }
            if num_h > 0.0 {
                let col = if now { pal().red } else { pal().dim };
                ptext(p, Pos2::new(slot.center().x, base + num_h / 2.0 + 1.0), Align::Center, &(b + 1).to_string(), 1.0, col);
            }
            if resp.clicked() {
                acts.push(Action::Opt(Opt::Level(b)));
            }
            x += bw + gap;
        }
        if muted {
            ptext(ui.painter(), rect.min + Vec2::new(6.0, 8.0), Align::Min, "MUTED", 1.0, pal().red);
        }
    }

    // ---------------------------------------------------------- LEAD SHEET
    /// The lead sheet: a header (name, key, transpose, edit), the chart, and the band as one bar at the foot.
    pub(crate) fn chart_ui(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        crate::tour::mark("SHEET", ui.clip_rect());
        // follow the player (the default), or keep what is on show; a stopped song keeps its sheet up
        let playing = self.cur_track();
        let ti = self.chart_tune();
        let playing_title = playing.as_ref().map(|t| t.title.clone());
        // what is on show: a saved tune, or whatever is playing (looked up on its own)
        let (name, key, bpm, info, text, live) = match (ti, playing_title) {
            // a practice progression from TUNES
            _ if self.prac_chart.is_some() => {
                let (title, chart) = self.prac_chart.clone().unwrap_or_default();
                (title, String::new(), 0, "Practice progression".to_string(), chart, false)
            }
            (Some(i), _) => {
                let t = &self.store.tunes[i];
                (t.name.clone(), t.key.clone(), t.bpm, t.credit_line(), t.chart.clone(), false)
            }
            (None, Some(title)) => {
                let q = chart::clean_title(&title);
                if self.live_q != q && !self.chart_busy {
                    acts.push(Action::LiveChart(q.clone(), None));
                }
                let (txt, key, comp) = if self.live_q == q { self.live.clone().unwrap_or_default() } else { Default::default() };
                let info = if comp.is_empty() { String::new() } else { format!("written by {}", comp) };
                (q, key, 0, info, txt, true)
            }
            (None, None) => {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| self.sheet_lock(ui, None));
                });
                ui.add_space(8.0);
                para(ui, "Play a song or open a tune in TUNES to see its changes here. They are looked up for you.", pal().ink2);
                para(ui, "Paste an iReal Pro link to bring in the changes and the composer.", pal().ink2);
                ui.horizontal(|ui| {
                    let w = (ui.available_width() - 90.0).max(80.0);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                    let _ = field(ui, &mut self.ed, F_IREAL, &mut self.ireal_in, rect, "irealb://...", false);
                    if retro_btn_w(ui, "IMPORT", 80.0, false).clicked() {
                        acts.push(Action::ImportIreal(None));
                    }
                });
                return;
            }
        };
        // a tune with no chart yet is looked up once on its own
        if let (Some(i), true) = (ti, text.trim().is_empty()) {
            if !self.chart_tried.contains(&name) && !self.chart_busy {
                self.chart_tried.insert(name.clone());
                acts.push(Action::FindChart(i, None));
            }
        }
        let busy = self.chart_busy;
        let chart = self.chart_for(&text);
        let has = !chart.bars.is_empty();

        // ---- the header: name, key, transpose, numerals, look again, edit, follow / lock
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                self.sheet_lock(ui, playing.as_ref().map(|t| t.id));
                if let Some(i) = ti {
                    let tip = if self.chart_edit { "Done editing" } else { "Edit the changes" };
                    if ibutton(ui, "pencil", tip, self.chart_edit).clicked() {
                        self.chart_edit = !self.chart_edit;
                        self.ed.id = 0;
                    }
                    if ibutton(ui, "refresh", if busy { "Looking..." } else { "Look the chart up again" }, busy).clicked() {
                        acts.push(Action::FindChart(i, None));
                    }
                } else if self.prac_chart.is_some() {
                    if ibutton(ui, "close", "Close: back to the tune or the song playing", false).clicked() {
                        self.prac_chart = None;
                    }
                } else {
                    if ibutton(ui, "refresh", if busy { "Looking..." } else { "Look the chart up again" }, busy).clicked() {
                        self.live_q.clear();
                    }
                    if has
                        && retro_btn(ui, "SAVE TO TUNES", false)
                            .tip("Keep this song as a tune, with its chart. Then you can edit it.")
                            .clicked()
                    {
                        acts.push(Action::SaveLive);
                    }
                }
                if retro_btn(ui, "I-IV-V", self.chart_rn)
                    .tip("Show Roman numerals under the chords, relative to the key.")
                    .clicked()
                {
                    acts.push(Action::ToggleNumerals);
                }
                ui.add_space(6.0);
                if retro_btn_w(ui, "+", 28.0, false).tip("Up a half step").clicked() {
                    self.chart_tr = (self.chart_tr + 1).rem_euclid(12);
                }
                // the key chip: every key, and the horn transpositions
                let tonic = chart::tonic(&key, &chart);
                let names = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];
                let chip = match tonic {
                    Some(t) => format!("KEY {}", names[(t + self.chart_tr).rem_euclid(12) as usize]),
                    None if self.chart_tr == 0 => "CONCERT".to_string(),
                    None => format!("+{}", self.chart_tr),
                };
                let kr = retro_btn(ui, &chip, self.chart_tr != 0)
                    .tip("The key it is shown in. Click for every key and the horn parts.");
                let mut keys: Vec<(String, i32)> =
                    vec![("CONCERT (C)".into(), 0), ("Bb HORNS".into(), 2), ("Eb HORNS".into(), 9), ("F HORN".into(), 7)];
                let t0 = tonic.unwrap_or(0);
                for (n, nm) in names.iter().enumerate() {
                    keys.push((format!("IN KEY OF {}", nm), (n as i32 - t0).rem_euclid(12)));
                }
                let cur = keys.iter().position(|k| k.1 == self.chart_tr).unwrap_or(0);
                let mut pick = None;
                popup(ui, "sheet_key", &kr, 220.0, false, |ui| {
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for (i, k) in keys.iter().enumerate() {
                            let mark = if i == cur { "> " } else { "   " };
                            if crate::menu_item(ui, &format!("{}{}", mark, k.0)) {
                                pick = Some(k.1);
                            }
                        }
                    });
                    pick.is_some()
                });
                if let Some(t) = pick {
                    self.chart_tr = t;
                }
                if retro_btn_w(ui, "-", 28.0, false).tip("Down a half step").clicked() {
                    self.chart_tr = (self.chart_tr - 1).rem_euclid(12);
                }
                ui.add_space(8.0);
                // the name takes what is left
                let (nr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bh()), Sense::hover());
                crate::winamp_ui::as_title(|| {
                    crate::ui_text(
                        ui.painter(),
                        Pos2::new(nr.min.x + 2.0, nr.center().y),
                        Align::Min,
                        &name,
                        3.0,
                        nr.width() - 4.0,
                        pal().ink,
                    )
                });
            });
        });
        let mut sub = String::new();
        if !key.is_empty() {
            sub.push_str(&format!("Key {}    ", key));
        }
        if bpm > 0 {
            sub.push_str(&format!("{} BPM    ", bpm));
        }
        if let Some(first) = info.lines().next() {
            sub.push_str(first);
        }
        if !sub.trim().is_empty() {
            dim_line(ui, sub.trim(), 1.0, pal().ink2);
        }
        if has && self.band_on && self.band_chart != text {
            self.band_stop();
        }

        // ---- the body above, the band at the foot
        let mut rest = ui.available_rect_before_wrap();
        // (never past the window's foot, whatever room the layout claims)
        if !rest.max.y.is_finite() || rest.max.y > ui.clip_rect().max.y {
            rest.max.y = ui.clip_rect().max.y.max(rest.min.y);
        }
        let fkey = egui::Id::new("sheet_foot_h");
        let foot_h = if has { ui.ctx().data(|d| d.get_temp::<f32>(fkey)).unwrap_or(bh() + 14.0) } else { 0.0 };
        let body = Rect::from_min_max(rest.min, Pos2::new(rest.max.x, (rest.max.y - foot_h).max(rest.min.y)));
        let clip = ui.clip_rect();
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body), |ui| {
            ui.set_clip_rect(body.intersect(clip));
            self.sheet_body(ui, acts, &chart, &name, &key, &text, ti, live, busy);
        });
        if has {
            let foot = Rect::from_min_max(Pos2::new(rest.min.x, rest.max.y - foot_h), rest.max);
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(foot), |ui| {
                ui.set_clip_rect(foot.intersect(clip));
                let h = self.band_bar(ui, acts);
                if (h - foot_h).abs() > 0.5 {
                    ui.ctx().data_mut(|d| d.insert_temp(fkey, h));
                    ui.ctx().request_repaint();
                }
            });
        }
    }

    /// Follow the player (the default) or keep this lead sheet: a lock that lights when it keeps it.
    fn sheet_lock(&mut self, ui: &mut egui::Ui, playing: Option<i64>) {
        let tip = if self.chart_lock {
            "Kept: this lead sheet stays, whatever plays. Click to follow the player again."
        } else {
            "Following the player: the lead sheet changes with the song playing. Click to keep this one."
        };
        // an open or shut padlock and its word: clear at a glance which it is
        let (icon, word) = if self.chart_lock { ("lock", "LOCKED") } else { ("unlock", "FOLLOWING") };
        let w = text_w(word, 1.6) + 34.0;
        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::click());
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        let p = ui.painter();
        let col = if self.chart_lock { pal().bar_txt } else { pal().ink2 };
        p.rect_filled(r, r.height() / 2.0, if self.chart_lock { pal().lcd } else { pal().beige.lerp_to_gamma(pal().lcd, 0.35) });
        p.rect_stroke(
            r,
            r.height() / 2.0,
            egui::Stroke::new(1.0_f32, if self.chart_lock { pal().bar_txt } else { pal().beige_h }),
        );
        crate::vicon::named(
            p,
            Rect::from_center_size(Pos2::new(r.min.x + 14.0, r.center().y), Vec2::splat(r.height() * 0.5)),
            icon,
            col,
        );
        crate::winamp_ui::as_title(|| {
            crate::ui_text(p, Pos2::new(r.min.x + 26.0, r.center().y), Align::Min, word, 1.6, w - 28.0, col)
        });
        let _ = ibutton;
        if resp.tip(tip).clicked() {
            if self.chart_lock {
                self.chart_lock = false;
                if let Some(id) = playing {
                    self.chart_follow(id);
                }
            } else {
                self.chart_lock = true;
            }
            self.dirty = true;
        }
    }

    /// The middle of the lead sheet: the chord the band is on, then the chart (or its editor, or a search for it).
    #[allow(clippy::too_many_arguments)]
    fn sheet_body(
        &mut self,
        ui: &mut egui::Ui,
        acts: &mut Vec<Action>,
        chart: &chart::Chart,
        name: &str,
        key: &str,
        text: &str,
        ti: Option<usize>,
        live: bool,
        busy: bool,
    ) {
        let has = !chart.bars.is_empty();
        // the chord the band is on, on a piano and a guitar box
        if has && self.band_on {
            let ch = crate::chart::parse_friendly(text);
            self.instrument_view(ui, &ch);
        }
        ui.add_space(2.0);

        if self.chart_edit && ti.is_some() {
            let ti = ti.unwrap_or(0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                para(ui, "Type the changes, with a | between bars. Mark sections like *A and time like T44. Use { } for repeats, [1 and [2 for endings, and @coda @segno @fine @dc. A % repeats the last bar. For example:", pal().ink2);
                para(ui, "T44 *A { Dm7 G7 | Cmaj7 } | *B | Em7b5 A7 | Dm7 G7 |", pal().ink);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 150.0), Sense::hover());
                let o = field(ui, &mut self.ed, F_CHART, &mut self.store.tunes[ti].chart, rect, "T44 *A | Dm7 G7 | Cmaj7 |", true);
                self.store_dirty |= o.changed;
                // a plain chord list pasted from a website (no bar lines, notes in between) made into a chart
                ui.horizontal_wrapped(|ui| {
                    if retro_btn(ui, "TIDY PASTED CHORDS", false)
                        .tip("Pasted a plain list of chords? Each chord becomes a bar and notes between them become sections. The title and credits are left out. Then fix the timing by hand.")
                        .clicked()
                    {
                        let tidy = crate::chart::from_loose(&self.store.tunes[ti].chart);
                        if tidy.trim() != "T44" {
                            self.store.tunes[ti].chart = tidy;
                            self.store_dirty = true;
                        }
                    }
                    label(ui, "EACH CHORD BECOMES ONE BAR", 260.0);
                });
                ui.add_space(8.0);
                para(ui, "Or paste an iReal Pro link (irealb://). It fills in the changes, key, tempo and composer.", pal().ink2);
                ui.horizontal(|ui| {
                    let w = (ui.available_width() - 90.0).max(80.0);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                    let _ = field(ui, &mut self.ed, F_IREAL, &mut self.ireal_in, rect, "irealb://...", false);
                    if retro_btn_w(ui, "IMPORT", 80.0, false).clicked() {
                        acts.push(Action::ImportIreal(Some(ti)));
                    }
                });
                if retro_btn_w(ui, "DONE", 80.0, true).clicked() {
                    self.chart_edit = false;
                    self.ed.id = 0;
                }
            });
            return;
        }

        if !has {
            ui.add_space(6.0);
            if busy {
                para(ui, "Looking for the changes...", pal().ink2);
            } else {
                para(ui, "No chart found for this name yet. The free chart list covers jazz standards. Know it by another name or a line of the lyric? Try that here.", pal().ink2);
            }
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 90.0).max(80.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                let o = field(ui, &mut self.ed, F_CHART_Q, &mut self.chart_q, rect, "Other title or lyrics of the song", false);
                if (o.enter || retro_btn_w(ui, "FIND", 80.0, false).clicked()) && !self.chart_q.trim().is_empty() {
                    let q = self.chart_q.trim().to_string();
                    acts.push(match ti {
                        Some(i) => Action::FindChart(i, Some(q)),
                        None => Action::LiveChart(name.to_string(), Some(q)),
                    });
                }
            });
            if !self.chart_sugg.is_empty() {
                dim_line(ui, "DID YOU MEAN:", 2.0, pal().ink2);
                for s in self.chart_sugg.clone() {
                    if retro_btn_w(ui, &s, (text_w(&s, 2.0) + 24.0).min(ui.available_width()), false).clicked() {
                        acts.push(match ti {
                            Some(i) => Action::FindChart(i, Some(s.clone())),
                            None => Action::LiveChart(name.to_string(), Some(s.clone())),
                        });
                    }
                }
            }
            para(ui, "Or click the pencil to type in the changes or paste an iReal Pro link.", pal().ink2);
            if live {
                para(ui, "Save the song with SAVE TO TUNES first, then the pencil works on it.", pal().dim);
            }
            return;
        }
        self.chart_grid(ui, chart, key, ti);
    }

    /// The band as one bar: play / stop, the loop, tempo, style, the mixer and the rest. Returns its height.
    fn band_bar(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) -> f32 {
        let top = ui.max_rect().min.y;
        let r = ui.max_rect();
        ui.painter().line_segment(
            [Pos2::new(r.min.x, top + 1.0), Pos2::new(r.max.x, top + 1.0)],
            egui::Stroke::new(1.0_f32, pal().groove),
        );
        ui.add_space(7.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
            let on = self.band_on;
            if icon_pill(ui, if on { "stop" } else { "play" }, if on { "STOP BAND" } else { "PLAY BAND" }, 130.0, on)
                .tip("A little band plays the changes: bass, chords and drums.")
                .clicked()
            {
                acts.push(Action::Opt(Opt::Band));
            }
            if let Some((a, b)) = self.chart_sel {
                if retro_btn(ui, &format!("BARS {}-{}  X", a + 1, b + 1), true)
                    .tip("Drag across bars to loop part of the tune. To play the whole tune again, click here, right-click a bar, double-click the sheet or press Esc over it.")
                    .clicked()
                {
                    self.chart_sel = None;
                }
            }
            ui.add_space(4.0);
            label(ui, "BPM", 36.0);
            let s = self.knob_bpm().to_string();
            self.num_step(ui, acts, F_BAND_BPM, 46.0, Knob::Bpm, s, "The band's tempo.");
            // the style: a list that opens upward, inside the window
            let cur = self.band_style as usize % band::STYLES.len();
            let sr = retro_btn(ui, band::STYLES[cur], false).tip("The band's style.");
            let mut pick = None;
            popup(ui, "band_style", &sr, 200.0, true, |ui| {
                for (i, st) in band::STYLES.iter().enumerate() {
                    let mark = if i == cur { "> " } else { "   " };
                    if crate::menu_item(ui, &format!("{}{}", mark, st)) {
                        pick = Some(i);
                    }
                }
                pick.is_some()
            });
            if let Some(i) = pick {
                acts.push(Action::Opt(Opt::BandStyleSet(i as u8)));
            }
            // the mixer: each part on / off and how loud, and how many times through
            let mr = ibutton(ui, "sliders", "Mixer: bass, chords and drums, and how many times through", false);
            popup(ui, "band_mix", &mr, 330.0, true, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 8.0);
                for (i, n) in ["BASS", "CHORDS", "DRUMS"].iter().enumerate() {
                    ui.horizontal(|ui| {
                        if retro_btn_w(ui, n, 84.0, self.band_parts[i]).tip("Turn this part on or off.").clicked() {
                            acts.push(Action::Opt(Opt::Part(i)));
                        }
                        ui.add_space(6.0);
                        if retro_btn_w(ui, "-", 28.0, false).tip("Softer").clicked() {
                            acts.push(Action::Opt(Opt::PartLevel(i, false)));
                        }
                        lcd_box(ui, &format!("{}%", (self.band_lv[i] * 100.0).round()), 60.0, pal().ink);
                        if retro_btn_w(ui, "+", 28.0, false).tip("Louder").clicked() {
                            acts.push(Action::Opt(Opt::PartLevel(i, true)));
                        }
                    });
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    label(ui, "LOOPS", 84.0);
                    ui.add_space(6.0);
                    if retro_btn_w(ui, "-", 28.0, false).clicked() {
                        acts.push(Action::Opt(Opt::BandLoops(false)));
                    }
                    let loops = if self.band_loops == 0 { "ENDLESS".to_string() } else { format!("{}x", self.band_loops) };
                    lcd_box(ui, &loops, 84.0, pal().ink);
                    if retro_btn_w(ui, "+", 28.0, false).tip("Times through the chart before the band stops.").clicked() {
                        acts.push(Action::Opt(Opt::BandLoops(true)));
                    }
                });
                false
            });
            // the rest: keep the band as a file
            let xr = ibutton(ui, "more", "More", false);
            popup(ui, "band_more", &xr, 220.0, true, |ui| {
                let hit = retro_btn_w(ui, "EXPORT WAV", 190.0, false)
                    .tip("Save the band as a WAV file. It has the count-in, the chart as many times as LOOPS says (once when endless) and an ending.")
                    .clicked();
                if hit {
                    acts.push(Action::Opt(Opt::BandExport));
                }
                hit
            });
        });
        ui.add_space(4.0);
        ui.min_rect().max.y - top
    }

    /// The bars of the chart: repeat signs, endings, coda marks, numerals and the band's position.
    fn chart_grid(&mut self, ui: &mut egui::Ui, chart: &chart::Chart, key: &str, ti: Option<usize>) {
        if self.chart_drag.is_some() && !ui.input(|i| i.pointer.primary_down()) {
            self.chart_drag = None;
        }
        // Esc with the mouse over the sheet lets go of the loop too
        if self.chart_sel.is_some() && ui.rect_contains_pointer(ui.max_rect()) && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.chart_sel = None;
        }
        if self.chart_sel.is_some_and(|(_, b)| b >= chart.bars.len()) {
            self.chart_sel = None;
        }
        let mut req: Option<(usize, String)> = None;
        let tr = self.chart_tr;
        let tonic = if self.chart_rn { chart::tonic(key, chart) } else { None };
        // which bar the band is on, and which beat
        let (now_bar, now_beat) = if self.band_on {
            ui.ctx().request_repaint();
            match self.band_where() {
                Some((k, beat)) => (self.band_order.get(k).copied(), beat as usize),
                None => (None, 0),
            }
        } else {
            (None, 0)
        };
        // bars inside a 1st / 2nd ending bracket
        let mut in_end = vec![false; chart.bars.len()];
        let mut open = false;
        for (i, b) in chart.bars.iter().enumerate() {
            if !b.ending.is_empty() {
                open = true;
            } else if i > 0 && (chart.bars[i - 1].end_rep || !b.label.is_empty() || b.start_rep) {
                open = false;
            }
            in_end[i] = open;
        }
        // dragging across bars picks them, so it must not scroll the sheet
        egui::ScrollArea::vertical().auto_shrink([false, false]).drag_to_scroll(false).show(ui, |ui| {
            // picking bars past the edge of the sheet: held near (or past) its top or bottom, it scrolls along, faster
            // the further out the pointer is
            if self.chart_drag.is_some() && ui.input(|i| i.pointer.primary_down()) {
                let view = ui.clip_rect();
                if let Some(pp) = ui.input(|i| i.pointer.interact_pos()) {
                    let edge = 36.0;
                    let dy = if pp.y > view.max.y - edge {
                        -((pp.y - (view.max.y - edge)) * 0.35).clamp(2.0, 30.0)
                    } else if pp.y < view.min.y + edge {
                        ((view.min.y + edge - pp.y) * 0.35).clamp(2.0, 30.0)
                    } else {
                        0.0
                    };
                    if dy != 0.0 {
                        ui.scroll_with_delta(Vec2::new(0.0, dy));
                        ui.ctx().request_repaint();
                    }
                }
            }
            ui.add_space(4.0);
            let w = ui.available_width() - 6.0;
            let per: usize = if w >= 760.0 { 8 } else { 4 };
            let gap = 6.0;
            let cw = ((w - gap * (per as f32 - 1.0)) / per as f32).floor().max(40.0);
            // the bars grow to fill the sheet's height (up to twice their size), the chord names with them
            let base = if tonic.is_some() { 64.0 } else { 54.0 };
            let rows = chart.bars.len().div_ceil(per).max(1) as f32;
            let ch = ((ui.available_height() - 12.0) / rows - 8.0).clamp(base, base * 2.0).floor();
            let z = ch / base;
            for (ri, row) in chart.bars.chunks(per).enumerate() {
                let (rr, _) = ui.allocate_exact_size(Vec2::new(w, ch + 8.0), Sense::hover());
                if !ui.is_rect_visible(rr) {
                    continue;
                }
                for (ci, bar) in row.iter().enumerate() {
                    let idx = ri * per + ci;
                    let cell =
                        Rect::from_min_size(Pos2::new(rr.min.x + ci as f32 * (cw + gap), rr.min.y + 4.0), Vec2::new(cw, ch));
                    let p = ui.painter();
                    let now = now_bar == Some(idx);
                    let picked = self.chart_sel.is_some_and(|(a, b)| a <= idx && idx <= b);
                    inset(p, cell, if now { pal().sel } else { pal().lcd });
                    if picked {
                        p.rect_filled(cell.shrink(2.0), 2.0, pal().bar_txt.gamma_multiply(0.18));
                        outline(p, cell, 1.0, pal().bar_txt);
                    }
                    // drag across bars to loop just those (as in iReal Pro)
                    // one response for both: a click edits the bar, a drag picks bars
                    let drag = ui.interact(cell, ui.id().with(("bar", idx)), Sense::click_and_drag());
                    if drag.drag_started() {
                        self.chart_drag = Some(idx);
                    }
                    if let Some(start) = self.chart_drag {
                        // (over the bar, or past the sheet's edge with this the last bar shown that way)
                        let over = ui.input(|i| i.pointer.interact_pos()).is_some_and(|pos| {
                            let view = ui.clip_rect();
                            cell.contains(pos)
                                || (pos.y > view.max.y && cell.max.y > view.max.y - ch && cell.x_range().contains(pos.x))
                                || (pos.y < view.min.y && cell.min.y < view.min.y + ch && cell.x_range().contains(pos.x))
                        });
                        if over && ui.input(|i| i.pointer.primary_down()) {
                            let sel = (start.min(idx), start.max(idx));
                            if self.chart_sel != Some(sel) {
                                self.chart_sel = Some(sel);
                            }
                        }
                    }
                    // a double click anywhere on the sheet lets go of the loop
                    if drag.double_clicked() && self.chart_sel.is_some() {
                        self.chart_sel = None;
                    }
                    let looping = self.chart_sel.is_some();
                    if ti.is_none() && looping {
                        drag.context_menu(|ui| {
                            if crate::menu_item(ui, "Clear the loop (play the whole tune)") {
                                self.chart_sel = None;
                                ui.close_menu();
                            }
                        });
                    }
                    if ti.is_some() {
                        let rsp = drag.clone().on_hover_cursor(egui::CursorIcon::PointingHand);
                        if rsp.clicked() {
                            self.bar_edit = Some((idx, bar.chords.join(" ")));
                            self.ed.id = F_BAR;
                            self.ed.anchor = None;
                            self.ed.cur = bar.chords.join(" ").chars().count();
                        }
                        rsp.context_menu(|ui| {
                            if looping && crate::menu_item(ui, "Clear the loop (play the whole tune)") {
                                self.chart_sel = None;
                                ui.close_menu();
                            }
                            for (lab, op) in [
                                ("Insert a bar after (copy)", "ins"),
                                ("Delete this bar", "del"),
                                ("Repeat starts here  { ", "{"),
                                ("Repeat ends here  } ", "}"),
                                ("1st ending", "[1"),
                                ("2nd ending", "[2"),
                                ("Section A", "*A"),
                                ("Section B", "*B"),
                                ("Section C", "*C"),
                                ("No section label", "*"),
                                ("Segno", "@segno"),
                                ("Coda", "@coda"),
                                ("To Coda", "@tocoda"),
                                ("Fine", "@fine"),
                                ("D.C. al Coda", "@dccoda"),
                                ("D.S. al Coda", "@dscoda"),
                                ("D.C. al Fine", "@dcfine"),
                            ] {
                                if crate::menu_item(ui, lab) {
                                    req = Some((idx, op.to_string()));
                                    ui.close_menu();
                                }
                            }
                            if crate::menu_item(ui, "Edit the chords") {
                                req = Some((idx, "text".to_string()));
                                ui.close_menu();
                            }
                        });
                    }
                    // ending bracket
                    if in_end[idx] {
                        fill_rect(
                            p,
                            Rect::from_min_size(cell.min + Vec2::new(2.0, 2.0), Vec2::new(cell.width() - 4.0, 2.0)),
                            pal().ink,
                        );
                        if !bar.ending.is_empty() {
                            fill_rect(p, Rect::from_min_size(cell.min + Vec2::new(2.0, 2.0), Vec2::new(2.0, 16.0)), pal().ink);
                            ptext(p, cell.min + Vec2::new(8.0, 12.0), Align::Min, &bar.ending, 1.0, pal().ink);
                        }
                    }
                    // repeat signs
                    for (on, left) in [(bar.start_rep, true), (bar.end_rep, false)] {
                        if !on {
                            continue;
                        }
                        let x = if left { cell.min.x + 5.0 } else { cell.max.x - 8.0 };
                        fill_rect(p, Rect::from_min_size(Pos2::new(x, cell.min.y + 5.0), Vec2::new(3.0, ch - 10.0)), pal().ink);
                        let dx = if left { x + 6.0 } else { x - 5.0 };
                        for dy in [ch * 0.38, ch * 0.62] {
                            fill_rect(p, Rect::from_min_size(Pos2::new(dx, cell.min.y + dy), Vec2::splat(3.0)), pal().ink);
                        }
                    }
                    // a change of meter: the time signature, stacked the way it is printed
                    if !bar.time.is_empty() {
                        let (num, den) = chart::meter(&bar.time);
                        let x = cell.min.x + if bar.start_rep { 22.0 } else { 9.0 };
                        let y = cell.center().y;
                        ptext(p, Pos2::new(x, y - 7.0), Align::Center, &num.to_string(), 1.5, pal().ink);
                        ptext(p, Pos2::new(x, y + 7.0), Align::Center, &den.to_string(), 1.5, pal().ink);
                    }
                    let mut top = 0.0;
                    if !bar.label.is_empty() {
                        let lw = text_w(&bar.label, 2.0) + 10.0;
                        let tag = Rect::from_min_size(cell.min + Vec2::new(14.0, 4.0), Vec2::new(lw, 16.0));
                        fill_rect(p, tag, pal().edge);
                        ptext(p, tag.center(), Align::Center, &bar.label, 2.0, pal().trim);
                        top = 6.0;
                    }
                    if !bar.marks.is_empty() {
                        ptext_fit(
                            p,
                            Pos2::new(cell.max.x - 6.0, cell.min.y + 10.0),
                            Align::Max,
                            &bar.marks.join(" "),
                            1.0,
                            cell.width() - 40.0,
                            pal().red,
                        );
                    }
                    let cy = cell.center().y + top - if tonic.is_some() { 4.0 } else { 0.0 };
                    if bar.repeat {
                        // a chord that goes on: the repeat sign, drawn as a pixel slash with two dots
                        let c = Pos2::new(cell.center().x, cy);
                        for k in -3..=3i32 {
                            fill_rect(
                                p,
                                Rect::from_center_size(c + Vec2::new(k as f32 * 4.0, -k as f32 * 4.0), Vec2::splat(4.0)),
                                pal().ink2,
                            );
                        }
                        fill_rect(p, Rect::from_center_size(c + Vec2::new(-13.0, -9.0), Vec2::splat(4.0)), pal().ink2);
                        fill_rect(p, Rect::from_center_size(c + Vec2::new(13.0, 9.0), Vec2::splat(4.0)), pal().ink2);
                    } else {
                        let n = bar.chords.len().max(1);
                        let slot = (cell.width() - 24.0) / n as f32;
                        for (k, c) in bar.chords.iter().enumerate() {
                            let shown = chart::transpose(c, tr);
                            let px = (if n == 1 { 3.0 } else { 2.0 }) * z;
                            let cx = cell.min.x + 12.0 + slot * (k as f32 + 0.5);
                            if let Some(t) = tonic {
                                ptext_fit(
                                    p,
                                    Pos2::new(cx, cy + 19.0 * z),
                                    Align::Center,
                                    &chart::roman(c, t),
                                    z,
                                    slot - 4.0,
                                    pal().ink2,
                                );
                            }
                            ptext_fit(
                                p,
                                Pos2::new(cx, cy - if tonic.is_some() { 5.0 * z } else { 0.0 }),
                                Align::Center,
                                &shown,
                                px,
                                slot - 4.0,
                                pal().ink,
                            );
                        }
                    }
                    ptext(p, Pos2::new(cell.max.x - 5.0, cell.max.y - 6.0), Align::Max, &(idx + 1).to_string(), 1.0, pal().dim);
                    if now {
                        outline(p, cell, 2.0, pal().red);
                        // beat pips along the bottom
                        for b in 0..chart.bar_len(idx).ceil() as usize {
                            let r = Rect::from_min_size(
                                Pos2::new(cell.min.x + 12.0 + b as f32 * 8.0, cell.max.y - 8.0),
                                Vec2::splat(4.0),
                            );
                            fill_rect(p, r, if b == now_beat { pal().red } else { pal().groove });
                        }
                    }
                }
            }
            ui.add_space(14.0);
        });
        // the little bar editor: type the chords (Enter), or join / split / hold / delete with a click
        if let (Some((bi, _)), Some(_)) = (self.bar_edit.clone(), ti) {
            let mut close = ui.input(|i| i.key_pressed(egui::Key::Escape));
            let mut op: Option<String> = None;
            egui::Area::new(egui::Id::new("bar_editor"))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    egui::Frame::none().fill(pal().beige).stroke(egui::Stroke::new(2.0_f32, pal().edge)).inner_margin(12.0).show(
                        ui,
                        |ui| {
                            ui.set_width(420.0);
                            crate::title_line(ui, &format!("BAR {}", bi + 1), 2.5, pal().ink);
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bh()), Sense::hover());
                            if let Some((_, buf)) = self.bar_edit.as_mut() {
                                let o = field(ui, &mut self.ed, F_BAR, buf, rect, "Dm7 G7", false);
                                if o.enter {
                                    op = Some(format!("set:{}", buf));
                                }
                            }
                            crate::views::dim_line(
                                ui,
                                "TYPE THE CHORDS WITH A SPACE BETWEEN THEM. PRESS ENTER TO KEEP THEM.",
                                1.0,
                                pal().dim,
                            );
                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                for (lab, o, tip) in [
                                    ("JOIN NEXT", "join", "Move the next bar's chords into this one, for two chords in a bar."),
                                    ("SPLIT", "split", "With two or more chords, move the second half to a bar of its own."),
                                    ("+ BAR", "ins", "Hold this chord one more bar, as a copy after it."),
                                    ("DELETE", "del", "Take this bar out."),
                                ] {
                                    if retro_btn(ui, lab, false).tip(tip).clicked() {
                                        op = Some(o.to_string());
                                    }
                                }
                                if retro_btn(ui, "DONE", true).clicked() {
                                    close = true;
                                }
                            });
                        },
                    );
                });
            if let (Some(o), Some(tune)) = (op, ti) {
                let mut c = chart.clone();
                c.edit(bi, &o);
                self.store.tunes[tune].chart = c.to_text();
                self.store_dirty = true;
                let after = crate::chart::parse_friendly(&self.store.tunes[tune].chart);
                match after.bars.get(bi) {
                    // stay on the bar after joining / splitting / holding; close after typing or deleting
                    Some(b) if !o.starts_with("set:") && o != "del" => {
                        self.bar_edit = Some((bi, b.chords.join(" ")));
                    }
                    _ => close = true,
                }
            }
            if close {
                self.bar_edit = None;
                self.ed.id = 0;
            }
        }
        if let (Some((i, op)), Some(tune)) = (req, ti) {
            if op == "text" {
                self.chart_edit = true;
                self.ed.id = 0;
            } else {
                let mut c = chart.clone();
                c.edit(i, &op);
                self.store.tunes[tune].chart = c.to_text();
                self.store_dirty = true;
            }
        }
    }
}

/// The focus timer's extra settings (kept in settings.json) and what it keeps track of between frames.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct TimerMore {
    /// the next focus block starts by itself when a rest ends
    pub auto_next: bool,
    /// the music pauses when a block ends and plays on when the next one starts
    pub rest_pause: bool,
    /// a finished block counts as practice in the diary
    pub log_diary: bool,
    /// a long rest after every this many blocks (0 = never), of this many minutes
    pub long_every: u32,
    pub long_mins: u32,
    #[serde(skip)]
    pub last: u8,
    /// the length of this block or rest, in seconds (with +5 MIN and skips)
    #[serde(skip)]
    pub total: u64,
    /// the diary's seconds for today when the block began
    #[serde(skip)]
    pub base: u32,
    #[serde(skip)]
    pub was_playing: bool,
    #[serde(skip)]
    pub paused_by_us: bool,
    #[serde(skip)]
    pub show_set: bool,
    #[serde(skip)]
    pub was_open: (bool, bool),
    /// where the pills were drawn, and in which frame
    #[serde(skip)]
    pub timer_pill: Option<(u64, Rect)>,
    #[serde(skip)]
    pub metro_pill: Option<(u64, Rect)>,
}

impl Default for TimerMore {
    fn default() -> Self {
        TimerMore {
            auto_next: false,
            rest_pause: true,
            log_diary: true,
            long_every: 0,
            long_mins: 15,
            last: 0,
            total: 0,
            base: 0,
            was_playing: false,
            paused_by_us: false,
            show_set: false,
            was_open: (false, false),
            timer_pill: None,
            metro_pill: None,
        }
    }
}

fn mix(a: Color32, b: Color32, k: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k) as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// The frame of a floating panel.
fn panel_frame() -> egui::Frame {
    let chrome = crate::winamp_ui::chrome_on();
    egui::Frame::none()
        .fill(pal().app_bg)
        .stroke(egui::Stroke::new(if chrome { 1.0_f32 } else { 2.0 }, pal().edge))
        .rounding(if chrome { 12.0 } else { 0.0 })
        .inner_margin(14.0)
}

/// A panel's name, top left.
fn panel_title(p: &egui::Painter, r: Rect, text: &str) {
    if crate::winamp_ui::chrome_on() {
        crate::spaced_caps(p, Pos2::new(r.min.x + 2.0, r.center().y - 6.0), text, 1.4, pal().ink);
    } else {
        ptext(p, Pos2::new(r.min.x + 2.0, r.center().y), Align::Min, text, 2.0, pal().ink);
    }
}

/// The face of a pill: round with a skin worn, a raised button without.
fn pill_face(p: &egui::Painter, r: Rect, open: bool, hot: bool) {
    if crate::winamp_ui::chrome_on() {
        let fill = if open {
            pal().lcd
        } else if hot {
            pal().lcd.lerp_to_gamma(pal().beige, 0.25)
        } else {
            pal().lcd.lerp_to_gamma(pal().beige, 0.4)
        };
        p.rect_filled(r, r.height() / 2.0, fill);
        p.rect_stroke(r, r.height() / 2.0, egui::Stroke::new(1.0_f32, if open { pal().bar_txt } else { pal().edge }));
    } else {
        crate::raised_h(p, r, open, hot);
    }
}

/// The words of a pill, and a small dot before what follows them.
fn pill_words(p: &egui::Painter, r: Rect, main: &str, tail: &str, col: Color32) {
    let px = if r.height() >= 20.0 { 2.0 } else { 1.5 };
    let x = r.min.x + 4.0;
    let room = r.max.x - x - 10.0;
    let mw = text_w(main, px);
    crate::winamp_ui::as_title(|| crate::ui_text(p, Pos2::new(x, r.center().y), Align::Min, main, px, room, col));
    if !tail.is_empty() && mw + 14.0 + text_w(tail, px) <= room {
        let dx = x + mw + 7.0;
        p.circle_filled(Pos2::new(dx, r.center().y), 1.6, pal().ink2);
        crate::winamp_ui::as_title(|| {
            crate::ui_text(p, Pos2::new(dx + 7.0, r.center().y), Align::Min, tail, px, r.max.x - dx - 14.0, pal().ink2)
        });
    }
}

/// A square button with a line icon (`crate::vicon::named`); its name on hover. `lit` shows it on.
fn ibutton(ui: &mut egui::Ui, icon: &str, tip: &str, lit: bool) -> egui::Response {
    let s = bh();
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(s), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hot = resp.hovered();
    let p = ui.painter();
    let col = if crate::winamp_ui::chrome_on() {
        if lit {
            p.circle_filled(r.center(), s / 2.0, pal().lcd);
        } else if hot {
            p.circle_filled(r.center(), s / 2.0, Color32::from_white_alpha(14));
        }
        if lit {
            pal().bar_txt
        } else if hot {
            pal().ink
        } else {
            pal().ink2
        }
    } else {
        crate::raised_h(p, r, lit || resp.is_pointer_button_down_on(), hot);
        pal().ink
    };
    crate::vicon::named(p, r.shrink(s * 0.27), icon, col);
    resp.tip(tip)
}

/// A button with a line icon and a word: filled in when `on` (a skin worn), pressed in without.
fn icon_pill(ui: &mut egui::Ui, icon: &str, text: &str, w: f32, on: bool) -> egui::Response {
    let h = bh();
    let w = (w * crate::font::ui_scale()).max(text_w(text, 2.0) + h + 22.0);
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let p = ui.painter();
    let fg = if crate::winamp_ui::chrome_on() && on {
        p.rect_filled(r, (h / 2.0).min(14.0), pal().ink);
        pal().lcd
    } else {
        crate::raised_h(p, r, on || resp.is_pointer_button_down_on(), resp.hovered());
        pal().ink
    };
    let ic = Rect::from_center_size(Pos2::new(r.min.x + 12.0 + h * 0.2, r.center().y), Vec2::splat(h * 0.42));
    crate::vicon::named(p, ic, icon, fg);
    let tx = ic.max.x + 8.0;
    crate::winamp_ui::as_title(|| crate::ui_text(p, Pos2::new(tx, r.center().y), Align::Min, text, 2.0, r.max.x - tx - 8.0, fg));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A box with a small heading, `w` wide.
fn group_box(ui: &mut egui::Ui, title: &str, w: f32, add: impl FnOnce(&mut egui::Ui)) {
    let chrome = crate::winamp_ui::chrome_on();
    egui::Frame::none()
        .stroke(egui::Stroke::new(1.0_f32, pal().groove))
        .rounding(if chrome { 8.0 } else { 0.0 })
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_width(w - 22.0);
            let (hr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
            if chrome {
                crate::spaced_caps(ui.painter(), Pos2::new(hr.min.x + 2.0, hr.min.y + 2.0), title, 1.2, pal().ink2);
            } else {
                ptext(ui.painter(), Pos2::new(hr.min.x + 2.0, hr.center().y), Align::Min, title, 2.0, pal().ink);
            }
            add(ui);
        });
}

/// A small panel that `anchor` opens and shuts, kept inside the window (`ui`'s clip rect): above the anchor when
/// `up`, else below. `add` returns true to shut it (a choice made). A click outside or Esc shuts it too.
fn popup(ui: &mut egui::Ui, id: &str, anchor: &egui::Response, w: f32, up: bool, add: impl FnOnce(&mut egui::Ui) -> bool) {
    let key = egui::Id::new(("tools_pop", id));
    let ctx = ui.ctx().clone();
    let mut open = ctx.data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    if anchor.clicked() {
        open = !open;
    }
    if !open {
        ctx.data_mut(|d| d.insert_temp(key, false));
        return;
    }
    let clip = ui.clip_rect();
    let hkey = key.with("h");
    let h = ctx.data(|d| d.get_temp::<f32>(hkey)).unwrap_or(120.0);
    let x = (anchor.rect.max.x - w).min(clip.max.x - w - 4.0).max(clip.min.x + 4.0);
    let y = if up { anchor.rect.min.y - 6.0 - h } else { anchor.rect.max.y + 6.0 };
    let y = y.min(clip.max.y - h - 4.0).max(clip.min.y + 4.0);
    let chrome = crate::winamp_ui::chrome_on();
    let mut shut = false;
    let area = egui::Area::new(key.with("area")).order(egui::Order::Foreground).fixed_pos(Pos2::new(x, y)).constrain(false).show(
        &ctx,
        |ui| {
            ui.set_clip_rect(clip);
            egui::Frame::none()
                .fill(pal().beige_lt)
                .stroke(egui::Stroke::new(if chrome { 1.0_f32 } else { 2.0 }, pal().edge))
                .rounding(if chrome { 10.0 } else { 0.0 })
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.set_width(w - 24.0);
                    shut = add(ui);
                });
        },
    );
    let pr = area.response.rect;
    ctx.data_mut(|d| d.insert_temp(hkey, pr.height()));
    let out = ctx.input(|i| {
        i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|q| !pr.contains(q) && !anchor.rect.contains(q))
    });
    if shut || out || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        open = false;
    }
    ctx.data_mut(|d| d.insert_temp(key, open));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_names_and_steps() {
        assert_eq!(speed_label(100), "1X");
        assert_eq!(speed_label(125), "1.25X");
        assert_eq!(speed_label(50), "0.5X");
        assert_eq!(speed_label(250), "2.5X");
        assert_eq!(speed_label(87), "0.87X");
        // clicking walks up the presets and wraps to the slowest
        assert_eq!(next_speed(100), 125);
        assert_eq!(next_speed(87), 100, "a typed speed goes to the next preset up");
        assert_eq!(next_speed(250), 50);
        assert!(SPEEDS.windows(2).all(|w| w[0] < w[1]) && SPEEDS.contains(&100));
    }
}
