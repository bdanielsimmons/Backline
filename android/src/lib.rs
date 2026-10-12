//! Backline for Android: a lean player and practice app. The engine (Tidal, the player, decoding) is the desktop
//! app's own code, compiled in from backline/src, so a fix there is a fix here.
#![allow(dead_code, unused_imports, unused_variables, clippy::all)]

#[path = "../../backline/src/data/api.rs"]
mod api;
#[path = "../../backline/src/audio/band.rs"]
mod band;
#[path = "../../backline/src/data/chart.rs"]
mod chart;
#[path = "../../backline/src/audio/decode.rs"]
mod decode;
#[path = "../../backline/src/data/meta.rs"]
mod meta;
#[path = "../../backline/src/audio/player.rs"]
mod player;
#[path = "../../backline/src/audio/tuning.rs"]
mod tuning;

use api::{Api, DeviceCode, Track};
use eframe::egui::{self, Color32, RichText, Sense, Vec2};
use player::{Cmd, Player};
use std::sync::mpsc::{channel, Receiver, Sender};

enum Msg {
    Code(Result<DeviceCode, String>),
    LoggedIn(Result<(), String>),
    Found(Result<Vec<Track>, String>),
    /// the song's audio, and its length when the file had to tell us (0: the track already knows)
    Audio(u64, Result<(Vec<u8>, f32, Vec<u8>), String>),
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Player,
    Library,
}

struct App {
    api: Api,
    player: Player,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    ctx: egui::Context,
    tab: Tab,
    logged_in: bool,
    code: Option<(String, String)>,
    query: String,
    searching: bool,
    results: Vec<Track>,
    queue: Vec<Track>,
    cur: Option<usize>,
    gen: u64,
    loading: bool,
    paused: bool,
    note: String,
    loop_a: Option<f32>,
    loop_b: Option<f32>,
    loop_on: bool,
    speed: u32,
    show_queue: bool,
    seek_drag: Option<f32>,
    keyboard: bool,
    /// the library's tab: 0 Tidal, 1 files on the phone
    lib_tab: u8,
    /// songs on the phone: each track and its file
    files: Vec<(Track, std::path::PathBuf)>,
    /// the song's waveform: 400 peaks (0-255)
    wave: Vec<u8>,
    /// bookmarks on each song (by track id): where, and a name
    marks: std::collections::HashMap<i64, Vec<(f32, String)>>,
    /// the MORE sheet is up
    more: bool,
    cents: i32,
    count_in: u32,
    bpm: f32,
    taps: Vec<std::time::Instant>,
    eq: [f32; 10],
    eq_on: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> App {
        let ctx = cc.egui_ctx.clone();
        // phone-sized text and touch targets
        ctx.style_mut(|s| {
            use egui::{FontId, TextStyle};
            s.text_styles.insert(TextStyle::Body, FontId::proportional(18.0));
            s.text_styles.insert(TextStyle::Button, FontId::proportional(18.0));
            s.text_styles.insert(TextStyle::Heading, FontId::proportional(26.0));
            s.text_styles.insert(TextStyle::Small, FontId::proportional(14.0));
            s.spacing.button_padding = Vec2::new(14.0, 10.0);
            s.spacing.item_spacing = Vec2::new(10.0, 10.0);
            s.spacing.interact_size.y = 44.0;
        });
        let (tx, rx) = channel();
        let api = Api::new();
        App {
            logged_in: api.has_token(),
            api,
            player: Player::new(),
            tx,
            rx,
            ctx,
            tab: Tab::Library,
            code: None,
            query: String::new(),
            searching: false,
            results: Vec::new(),
            queue: Vec::new(),
            cur: None,
            gen: 0,
            loading: false,
            paused: false,
            note: String::new(),
            loop_a: None,
            loop_b: None,
            loop_on: false,
            speed: 100,
            show_queue: false,
            seek_drag: None,
            keyboard: false,
            lib_tab: 0,
            files: Vec::new(),
            wave: Vec::new(),
            marks: load_marks(),
            more: false,
            cents: 0,
            count_in: 0,
            bpm: 120.0,
            taps: Vec::new(),
            eq: [0.0; 10],
            eq_on: false,
        }
    }

    /// Run `f` off the screen's thread and send what it returns back.
    fn spawn(&self, f: impl FnOnce(Api) -> Msg + Send + 'static) {
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(f(api));
            ctx.request_repaint();
        });
    }

    fn cur_track(&self) -> Option<&Track> {
        self.cur.and_then(|i| self.queue.get(i))
    }

    fn play(&mut self, i: usize) {
        let Some(t) = self.queue.get(i).cloned() else { return };
        self.cur = Some(i);
        self.gen += 1;
        let g = self.gen;
        self.loading = true;
        self.paused = false;
        self.loop_a = None;
        self.loop_b = None;
        self.loop_on = false;
        self.sync_loop();
        self.player.send(Cmd::Stop);
        self.wave.clear();
        // a song on the phone: read it (and how long it is); a Tidal one: fetched
        if let Some(path) = self.files.iter().find(|f| f.0.id == t.id).map(|f| f.1.clone()) {
            self.spawn(move |_| {
                Msg::Audio(
                    g,
                    std::fs::read(&path).map_err(|e| e.to_string()).map(|b| {
                        let (d, w) = wave(&b);
                        (b, d, w)
                    }),
                )
            });
        } else {
            self.spawn(move |api| Msg::Audio(g, api.stream_url(t.id, false).and_then(|u| api.fetch(&u)).map(|b| {
                let (d, w) = wave(&b);
                (b, d, w)
            })));
        }
    }

    /// The songs on the phone: Backline's own Music folder (no permission needed), and the phone's Music and
    /// Download folders where Android lets us read them.
    fn scan_files(&mut self) {
        let mut dirs = vec![music_dir()];
        dirs.extend(["/sdcard/Music", "/sdcard/Download"].iter().map(std::path::PathBuf::from));
        self.files.clear();
        for d in dirs {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("").to_lowercase();
                if !["mp3", "m4a", "flac", "wav", "ogg", "aac"].contains(&ext.as_str()) {
                    continue;
                }
                let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("?").to_string();
                let id = -(1 + (hash(&p.to_string_lossy()) & 0x7fff_ffff_ffff) as i64);
                let (artist, title) = match name.split_once(" - ") {
                    Some((a, b)) => (a.to_string(), b.to_string()),
                    None => (String::new(), name.clone()),
                };
                let t = Track { id, title, artist, ..Default::default() };
                self.files.push((t, p));
            }
        }
        self.files.sort_by(|a, b| a.0.title.to_lowercase().cmp(&b.0.title.to_lowercase()));
    }

    fn sync_loop(&self) {
        let (a, b) = (self.loop_a.unwrap_or(0.0), self.loop_b.unwrap_or(0.0));
        self.player.ctl.set_loop(self.loop_on && b > a, a, b);
        self.player.ctl.set_speed(self.speed);
    }

    fn drain(&mut self) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Code(Ok(dc)) => {
                    self.code = Some((dc.user_code.clone(), dc.url.clone()));
                    let _ = webbrowser::open(&dc.url);
                    self.spawn(move |api| Msg::LoggedIn(api.finish_login(&dc)));
                }
                Msg::Code(Err(e)) | Msg::LoggedIn(Err(e)) => {
                    self.code = None;
                    self.note = format!("Sign-in failed: {}", e);
                }
                Msg::LoggedIn(Ok(())) => {
                    self.code = None;
                    self.logged_in = true;
                    self.note.clear();
                }
                Msg::Found(r) => {
                    self.searching = false;
                    match r {
                        Ok(t) => self.results = t,
                        Err(e) => self.note = format!("Search failed: {}", e),
                    }
                }
                Msg::Audio(g, r) if g == self.gen => {
                    self.loading = false;
                    match r {
                        Ok((b, d, w)) => {
                            self.wave = w;
                            if d > 0.0 {
                                if let Some(t) = self.cur.and_then(|i| self.queue.get_mut(i)) {
                                    t.duration = d;
                                }
                            }
                            self.player.send(Cmd::Play(b));
                        }
                        Err(e) => self.note = format!("Can't play: {}", e),
                    }
                }
                Msg::Audio(..) => {}
            }
        }
        // the song ended: on to the next
        let ended = {
            let mut s = self.player.shared.lock().unwrap();
            std::mem::take(&mut s.ended)
        };
        if ended && !self.loading {
            if let Some(i) = self.cur.filter(|i| i + 1 < self.queue.len()) {
                self.play(i + 1);
            }
        }
    }

    // ------------------------------------------------------------------ screens
    fn login_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            match &self.code {
                None => {
                    if ui.add(egui::Button::new("SIGN IN TO TIDAL").min_size(Vec2::new(240.0, 56.0))).clicked() {
                        self.code = Some((String::new(), String::new()));
                        self.spawn(|api| Msg::Code(api.start_login()));
                    }
                    ui.add_space(10.0);
                    ui.label(RichText::new("Your own Tidal subscription is needed.").weak());
                }
                Some((code, url)) if !code.is_empty() => {
                    ui.label("Approve Backline in the page that opened, or go to");
                    ui.label(RichText::new(url.as_str()).strong());
                    ui.add_space(10.0);
                    ui.label("and enter the code");
                    ui.label(RichText::new(code.as_str()).size(34.0).strong());
                }
                Some(_) => {
                    ui.spinner();
                }
            }
            if !self.note.is_empty() {
                ui.add_space(16.0);
                ui.label(RichText::new(self.note.as_str()).color(Color32::LIGHT_RED));
            }
        });
    }

    fn library_ui(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |c| {
            for (k, name) in ["TIDAL", "FILES"].into_iter().enumerate() {
                if c[k].add_sized(Vec2::new(c[k].available_width(), 40.0), egui::SelectableLabel::new(self.lib_tab == k as u8, name)).clicked() {
                    self.lib_tab = k as u8;
                    if k == 1 {
                        self.scan_files();
                    }
                }
            }
        });
        if self.lib_tab == 1 {
            self.files_ui(ui);
            return;
        }
        if !self.logged_in {
            self.login_ui(ui);
            return;
        }
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Search Tidal").desired_width(ui.available_width() - 90.0));
            let go = ui.button("GO").clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if go && !self.query.trim().is_empty() {
                let q = self.query.trim().to_string();
                self.searching = true;
                self.spawn(move |api| Msg::Found(api.search(&q).map(|p| p.tracks)));
            }
        });
        if self.searching {
            ui.spinner();
        }
        let mut play = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (i, t) in self.results.iter().enumerate() {
                if track_row(ui, t, self.cur_track().map(|c| c.id) == Some(t.id)).clicked() {
                    play = Some(i);
                }
            }
        });
        if let Some(i) = play {
            self.queue = self.results.clone();
            self.play(i);
        }
    }

    fn files_ui(&mut self, ui: &mut egui::Ui) {
        if ui.button("LOOK AGAIN").clicked() {
            self.scan_files();
        }
        if self.files.is_empty() {
            ui.label(RichText::new(format!("No songs found. Put audio files in your Music or Download folder, or in {}", music_dir().display())).weak());
            return;
        }
        let mut play = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (i, (t, _)) in self.files.iter().enumerate() {
                if track_row(ui, t, self.cur_track().map(|c| c.id) == Some(t.id)).clicked() {
                    play = Some(i);
                }
            }
        });
        if let Some(i) = play {
            self.queue = self.files.iter().map(|f| f.0.clone()).collect();
            self.play(i);
        }
    }

    fn player_ui(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.cur_track().cloned() else {
            ui.add_space(80.0);
            ui.vertical_centered(|ui| ui.label(RichText::new("Pick a song in the library").weak()));
            return;
        };
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t.title.as_str()).size(24.0).strong());
            ui.label(RichText::new(t.artists_text()).weak());
        });
        ui.add_space(20.0);
        // where the song is: drag to move
        let pos = self.seek_drag.unwrap_or_else(|| self.player.ctl.pos());
        let dur = t.duration.max(1.0);
        let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 90.0), Sense::click_and_drag());
        let p = ui.painter();
        p.rect_filled(r, 4.0, Color32::from_gray(32));
        let x_of = |t: f32| r.min.x + r.width() * (t / dur).clamp(0.0, 1.0);
        // the waveform, the part played brighter
        if !self.wave.is_empty() {
            let n = self.wave.len();
            let w = r.width() / n as f32;
            for (k, v) in self.wave.iter().enumerate() {
                let x = r.min.x + k as f32 * w;
                let h = (*v as f32 / 255.0) * (r.height() * 0.45);
                let played = x < x_of(pos);
                let c = if played { Color32::from_gray(210) } else { Color32::from_gray(100) };
                p.rect_filled(egui::Rect::from_x_y_ranges(x..=x + (w - 0.5).max(0.5), r.center().y - h..=r.center().y + h), 0.0, c);
            }
        }
        // bookmarks: a line and a number
        for (k, (bt, _)) in self.marks.get(&t.id).into_iter().flatten().enumerate() {
            let x = x_of(*bt);
            p.line_segment([egui::pos2(x, r.min.y), egui::pos2(x, r.max.y)], egui::Stroke::new(2.0, Color32::from_rgb(240, 190, 60)));
            p.text(egui::pos2(x + 3.0, r.min.y + 2.0), egui::Align2::LEFT_TOP, (k + 1).to_string(), egui::FontId::proportional(13.0), Color32::from_rgb(240, 190, 60));
        }
        if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
            let c = if self.loop_on { Color32::from_rgb(90, 160, 240) } else { Color32::from_gray(110) };
            p.rect_filled(egui::Rect::from_x_y_ranges(x_of(a)..=x_of(b), r.min.y + 8.0..=r.max.y - 8.0), 2.0, c.gamma_multiply(0.5));
        }
        p.line_segment([egui::pos2(x_of(pos), r.min.y), egui::pos2(x_of(pos), r.max.y)], egui::Stroke::new(2.0, Color32::WHITE));
        if let Some(pp) = resp.interact_pointer_pos() {
            if resp.dragged() || resp.clicked() {
                self.seek_drag = Some(((pp.x - r.min.x) / r.width()).clamp(0.0, 1.0) * dur);
            }
        }
        if resp.drag_stopped() || resp.clicked() {
            if let Some(s) = self.seek_drag.take() {
                self.player.send(Cmd::Seek(s));
            }
        }
        ui.horizontal(|ui| {
            ui.label(fmt_t(pos));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(fmt_t(dur)));
        });
        ui.add_space(10.0);
        // the transport
        ui.columns(3, |c| {
            if big(&mut c[0], "⏮").clicked() {
                if let Some(i) = self.cur {
                    if pos > 3.0 || i == 0 {
                        self.player.send(Cmd::Seek(0.0));
                    } else {
                        self.play(i - 1);
                    }
                }
            }
            let label = if self.loading { "…" } else if self.paused { "▶" } else { "⏸" };
            if big(&mut c[1], label).clicked() && !self.loading {
                self.paused = !self.paused;
                self.player.send(if self.paused { Cmd::Pause } else { Cmd::Resume });
            }
            if big(&mut c[2], "⏭").clicked() {
                if let Some(i) = self.cur.filter(|i| i + 1 < self.queue.len()) {
                    self.play(i + 1);
                }
            }
        });
        ui.add_space(16.0);
        // practice: the loop and the speed
        ui.horizontal_wrapped(|ui| {
            let now = self.player.ctl.pos();
            // fixed widths, so nothing moves as the times fill in
            let fixed = |ui: &mut egui::Ui, on: bool, s: String| ui.add_sized(Vec2::new(78.0, 44.0), egui::SelectableLabel::new(on, s));
            if fixed(ui, self.loop_a.is_some(), format!("A {}", self.loop_a.map(fmt_t).unwrap_or_default())).clicked() {
                self.loop_a = Some(now);
                self.sync_loop();
            }
            if fixed(ui, self.loop_b.is_some(), format!("B {}", self.loop_b.map(fmt_t).unwrap_or_default())).clicked() {
                self.loop_b = Some(now);
                self.loop_on = self.loop_a.is_some_and(|a| now > a);
                self.sync_loop();
            }
            if fixed(ui, self.loop_on, "LOOP".into()).clicked() && self.loop_a.is_some() && self.loop_b.is_some() {
                self.loop_on = !self.loop_on;
                self.sync_loop();
            }
            if (self.loop_a.is_some() || self.loop_b.is_some()) && ui.button("CLEAR").clicked() {
                self.loop_a = None;
                self.loop_b = None;
                self.loop_on = false;
                self.sync_loop();
            }
        });
        ui.horizontal(|ui| {
            ui.label("SPEED");
            if ui.button("−").clicked() {
                self.speed = self.speed.saturating_sub(5).max(25);
                self.sync_loop();
            }
            ui.label(RichText::new(format!("{}%", self.speed)).strong());
            if ui.button("+").clicked() {
                self.speed = (self.speed + 5).min(200);
                self.sync_loop();
            }
        });
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if ui.button("+ MARK").clicked() {
                let at = self.player.ctl.pos();
                let l = self.marks.entry(t.id).or_default();
                l.push((at, String::new()));
                l.sort_by(|a, b| a.0.total_cmp(&b.0));
                save_marks(&self.marks);
            }
            if ui.selectable_label(self.more, "MORE").clicked() {
                self.more = !self.more;
                self.show_queue = false;
            }
            if ui.selectable_label(self.show_queue, "QUEUE").clicked() {
                self.show_queue = !self.show_queue;
                self.more = false;
            }
        });
        if self.more {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.more_ui(ui, &t));
        }
        if self.show_queue {
            let mut play = None;
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for (i, t) in self.queue.iter().enumerate() {
                    if track_row(ui, t, Some(i) == self.cur).clicked() {
                        play = Some(i);
                    }
                }
            });
            if let Some(i) = play {
                self.play(i);
            }
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
    }

    /// MORE: pitch, the count-off, bookmarks and the equalizer.
    fn more_ui(&mut self, ui: &mut egui::Ui, t: &Track) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.add_sized(Vec2::new(90.0, 44.0), egui::Label::new("PITCH"));
            for (s, d) in [("−1", -100), ("−10c", -10), ("+10c", 10), ("+1", 100)] {
                if ui.button(s).clicked() {
                    self.cents = (self.cents + d).clamp(-1200, 1200);
                    self.player.ctl.set_semis(self.cents);
                }
            }
            ui.label(RichText::new(format!("{:+.2}", self.cents as f32 / 100.0)).strong());
        });
        ui.horizontal(|ui| {
            ui.add_sized(Vec2::new(90.0, 44.0), egui::Label::new("COUNT-OFF"));
            let label = if self.count_in == 0 { "OFF".to_string() } else { format!("{} BEATS", self.count_in) };
            if ui.add_sized(Vec2::new(100.0, 44.0), egui::SelectableLabel::new(self.count_in > 0, label)).clicked() {
                self.count_in = match self.count_in {
                    0 => 2,
                    2 => 4,
                    _ => 0,
                };
            }
            ui.label(format!("{:.0} BPM", self.bpm));
            // tap the tempo in
            if ui.button("TAP").clicked() {
                let now = std::time::Instant::now();
                self.taps.retain(|x| now.duration_since(*x).as_secs_f32() < 3.0);
                self.taps.push(now);
                if self.taps.len() >= 2 {
                    let span = now.duration_since(self.taps[0]).as_secs_f32();
                    self.bpm = (60.0 * (self.taps.len() - 1) as f32 / span).clamp(30.0, 300.0).round();
                }
            }
        });
        self.player.ctl.set_count_in(self.count_in, self.bpm);
        // bookmarks
        ui.separator();
        ui.label(RichText::new("BOOKMARKS").strong());
        let mut drop = None;
        let mut jump = None;
        for (k, (at, _)) in self.marks.get(&t.id).into_iter().flatten().enumerate() {
            ui.horizontal(|ui| {
                if ui.add_sized(Vec2::new(120.0, 44.0), egui::Button::new(format!("{}   {}", k + 1, fmt_t(*at)))).clicked() {
                    jump = Some(*at);
                }
                if ui.button("✕").clicked() {
                    drop = Some(k);
                }
            });
        }
        if self.marks.get(&t.id).map_or(true, |l| l.is_empty()) {
            ui.label(RichText::new("+ MARK drops one where the song is.").weak());
        }
        if let Some(at) = jump {
            self.player.send(Cmd::Seek(at));
        }
        if let Some(k) = drop {
            if let Some(l) = self.marks.get_mut(&t.id) {
                l.remove(k);
            }
            save_marks(&self.marks);
        }
        // the equalizer: ten bands, flat in the middle
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new("EQ").strong());
            if ui.selectable_label(self.eq_on, if self.eq_on { "ON" } else { "OFF" }).clicked() {
                self.eq_on = !self.eq_on;
            }
            if ui.button("FLAT").clicked() {
                self.eq = [0.0; 10];
            }
        });
        let bands = ["31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];
        ui.horizontal(|ui| {
            let w = (ui.available_width() / 10.0 - 6.0).max(20.0);
            for (k, b) in bands.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.set_width(w);
                    ui.add(egui::Slider::new(&mut self.eq[k], -12.0..=12.0).vertical().show_value(false));
                    ui.label(RichText::new(*b).size(11.0));
                });
            }
        });
        self.player.eq.set(self.eq, 0.0, self.eq_on);
    }

    /// While browsing: the song playing, small, along the bottom; tap it for the player.
    fn now_bar(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.cur_track().cloned() else { return };
        ui.horizontal(|ui| {
            let r = ui.add(
                egui::Label::new(RichText::new(format!("{}  ·  {}", t.title, t.artist)).strong())
                    .truncate()
                    .sense(Sense::click()),
            );
            if r.clicked() {
                self.tab = Tab::Player;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(if self.paused { "▶" } else { "⏸" }).clicked() {
                    self.paused = !self.paused;
                    self.player.send(if self.paused { Cmd::Pause } else { Cmd::Resume });
                }
            });
        });
    }
}

fn track_row(ui: &mut egui::Ui, t: &Track, now: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 58.0), Sense::click());
    let p = ui.painter();
    if now {
        p.rect_filled(r, 4.0, Color32::from_gray(45));
    }
    let f = egui::FontId::proportional(17.0);
    let s = egui::FontId::proportional(14.0);
    p.text(r.min + Vec2::new(8.0, 8.0), egui::Align2::LEFT_TOP, &t.title, f, Color32::from_gray(235));
    p.text(r.min + Vec2::new(8.0, 32.0), egui::Align2::LEFT_TOP, format!("{}  ·  {}", t.artist, fmt_t(t.duration)), s, Color32::from_gray(150));
    resp
}

fn big(ui: &mut egui::Ui, s: &str) -> egui::Response {
    ui.add_sized(Vec2::new(ui.available_width(), 64.0), egui::Button::new(RichText::new(s).size(28.0)))
}

/// A song's length and a 400-peak waveform, from its encoded bytes.
fn wave(bytes: &[u8]) -> (f32, Vec<u8>) {
    use rodio::Source;
    let Ok(src) = decode::SymSource::new(bytes.to_vec()) else { return (0.0, Vec::new()) };
    let (ch, rate) = (src.channels().max(1) as usize, src.sample_rate().max(1) as f32);
    let samples: Vec<i16> = src.collect();
    let frames = samples.len() / ch;
    let dur = frames as f32 / rate;
    let n = 400;
    let per = (frames / n).max(1);
    let peaks = (0..n)
        .map(|k| {
            let s = (k * per * ch).min(samples.len());
            let e = ((k + 1) * per * ch).min(samples.len());
            let m = samples[s..e].iter().step_by(ch * 8).map(|v| v.unsigned_abs()).max().unwrap_or(0);
            (m as u32 * 255 / 32768) as u8
        })
        .collect();
    (dur, peaks)
}

fn marks_file() -> std::path::PathBuf {
    api::config_dir().join("bookmarks.json")
}

fn load_marks() -> std::collections::HashMap<i64, Vec<(f32, String)>> {
    std::fs::read_to_string(marks_file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save_marks(m: &std::collections::HashMap<i64, Vec<(f32, String)>>) {
    let _ = std::fs::create_dir_all(api::config_dir());
    if let Ok(s) = serde_json::to_string(m) {
        let _ = std::fs::write(marks_file(), s);
    }
}

/// Backline's own Music folder on the phone: songs put here play without any permission.
fn music_dir() -> std::path::PathBuf {
    let base = std::env::var_os("BACKLINE_FILES").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("."));
    let d = base.join("Music");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn fmt_t(t: f32) -> String {
    let t = t.max(0.0) as u32;
    format!("{}:{:02}", t / 60, t % 60)
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain();
        // the phone's keyboard comes up while a text box is being typed in
        let want = ctx.wants_keyboard_input();
        if want != self.keyboard {
            self.keyboard = want;
            show_keyboard(want);
        }
        egui::TopBottomPanel::top("status_gap").exact_height(28.0).show_separator_line(false).show(ctx, |_| {});
        egui::TopBottomPanel::bottom("nav").show(ctx, |ui| {
            if self.tab != Tab::Player {
                self.now_bar(ui);
                ui.separator();
            }
            ui.columns(2, |c| {
                for (k, (tab, name)) in [(Tab::Player, "PLAYER"), (Tab::Library, "LIBRARY")].into_iter().enumerate() {
                    if c[k].add_sized(Vec2::new(c[k].available_width(), 48.0), egui::SelectableLabel::new(self.tab == tab, name)).clicked() {
                        self.tab = tab;
                    }
                }
            });
            ui.add_space(22.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.note.is_empty() {
                if ui.add(egui::Label::new(RichText::new(self.note.as_str()).color(Color32::LIGHT_RED)).sense(Sense::click())).clicked() {
                    self.note.clear();
                }
            }
            match self.tab {
                Tab::Player => self.player_ui(ui),
                Tab::Library => self.library_ui(ui),
            }
        });
    }
}

#[cfg(target_os = "android")]
static ANDROID: std::sync::OnceLock<winit::platform::android::activity::AndroidApp> = std::sync::OnceLock::new();

fn show_keyboard(on: bool) {
    #[cfg(target_os = "android")]
    if let Some(a) = ANDROID.get() {
        if on {
            a.show_soft_input(true);
        } else {
            a.hide_soft_input(true);
        }
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    // settings and the Tidal sign-in live in the app's own storage
    if let Some(p) = app.internal_data_path() {
        std::env::set_var("BACKLINE_HOME", p);
    }
    // the folder songs can be copied to (reachable over USB, no permission needed)
    if let Some(p) = app.external_data_path() {
        std::env::set_var("BACKLINE_FILES", p);
    }
    let _ = ANDROID.set(app.clone());
    let options = eframe::NativeOptions {
        event_loop_builder: Some(Box::new(move |b| {
            b.with_android_app(app);
        })),
        ..Default::default()
    };
    let _ = eframe::run_native("Backline", options, Box::new(|cc| Ok(Box::new(App::new(cc)))));
}
