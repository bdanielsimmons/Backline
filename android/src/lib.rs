//! Backline for Android. The engine (Tidal, the player, decoding, theory) and the look (the pixel font, the skins,
//! the icons, the drawing kit, the neck and piano pictures) are the desktop app's own code, compiled in from
//! backline/src - a fix there is a fix here, and the phone looks like Backline. Only the screens are the phone's:
//! the same windows as the desktop, stacked for a phone.
#![allow(dead_code, unused_imports, unused_variables, clippy::all)]

// ---- the desktop engine
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
#[path = "../../backline/src/data/theory.rs"]
mod theory;
#[path = "../../backline/src/audio/tuning.rs"]
mod tuning;
// ---- the desktop look
#[path = "../../backline/src/ui/font.rs"]
mod font;
#[path = "../../backline/src/ui/fretboard.rs"]
mod fretboard;
#[path = "../../backline/src/ui/icons.rs"]
mod icons;
#[path = "../../backline/src/ui/skin.rs"]
mod skin;
#[path = "../../backline/src/ui/vicon.rs"]
mod vicon;
#[path = "../../backline/src/ui/viz.rs"]
mod viz;
#[path = "../../backline/src/ui/widgets.rs"]
mod widgets;
// ---- the phone's screens
mod chords;
mod deck;
mod library;
mod metro;
mod practice;
mod sources;

use api::{Api, Card, DeviceCode, Track};
use eframe::egui::{self, Align, Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use font::{fit, ptext, ptext_fit, snap, spx, text_w, thick, wrap};
use fretboard::*;
use icons::*;
use player::{Cmd, Player};
use skin::*;
use sources::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};
use widgets::*;

// the desktop's names the drawing kit asks for
const HEAD_H: f32 = 18.0;
const ROW_H: f32 = 24.0;
const PRACTICE: bool = true;
const VERSION: &str = "v9";
mod tour {
    pub fn note(_name: &str, _r: eframe::egui::Rect) {}
    pub fn mark(_name: &str, _r: eframe::egui::Rect) {}
}
mod update {
    pub fn build() -> u64 {
        option_env!("BACKLINE_BUILD").and_then(|b| b.parse().ok()).unwrap_or(0)
    }
}
// the Winamp-style skins (Backline's ORIGINAL look): the skin pictures, generated from Backline's palettes, and
// their drawing
#[path = "../../backline/src/data/genskin.rs"]
mod genskin;
#[path = "../../backline/src/data/winamp.rs"]
mod winamp;
#[path = "../../backline/src/ui/winamp_draw.rs"]
mod winamp_ui;

/// The ear modes (the player's own): which part of the stereo picture you hear.
pub(crate) const CHAN_NAMES: [&str; 6] = ["STEREO", "LEFT CHANNEL", "RIGHT CHANNEL", "MONO", "NO CENTER", "BASS ONLY"];
pub(crate) const EQ_PRESETS: [(&str, [f32; 10]); 4] = [
    ("FLAT", [0.0; 10]),
    ("BASS", [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("TREBLE", [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 4.0, 5.0, 6.0]),
    ("ROCK", [4.0, 3.0, 2.0, 0.0, -1.0, -1.0, 1.0, 3.0, 4.0, 5.0]),
];
pub(crate) const SPEEDS: [u32; 7] = [50, 75, 100, 125, 150, 200, 250];

pub(crate) enum Msg {
    Code(Result<DeviceCode, String>),
    LoggedIn(Result<(), String>),
    Found(Result<Vec<Track>, String>),
    Liked(Result<Vec<Track>, String>),
    Lists(Result<Vec<Card>, String>),
    ListTracks(String, Result<Vec<Track>, String>),
    YtAdded(Result<Vec<YtItem>, String>),
    YtAsk(String, String, u64),
    /// a cover picture: its address and pixels
    Cover(String, Option<egui::ColorImage>),
    /// the song's audio, its length (0: the track knows) and its waveform (0-255 peaks)
    Audio(u64, Result<(Vec<u8>, f32, Vec<u8>), String>),
    Meta(i64, meta::Info),
    /// the skin's font fetched (which one, its bytes)
    Font(usize, Result<Vec<u8>, String>),
}

/// The phone's screens, along the bottom.
#[derive(PartialEq, Clone, Copy)]
pub(crate) enum Tab {
    Player,
    Library,
    Chords,
    Metro,
}

#[derive(PartialEq, Clone, Copy)]
pub(crate) enum Repeat {
    Off,
    All,
    One,
}

/// A saved loop of a song.
#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub(crate) struct Section {
    pub name: String,
    pub a: f32,
    pub b: f32,
}

/// What is kept between runs.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Settings {
    pub skin: usize,
    pub volume: f32,
    pub practice: bool,
    pub eq: [f32; 10],
    pub eq_on: bool,
    pub mt_bpm: u32,
    pub mt_vol: f32,
    pub chord_details: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            skin: 9,
            volume: 0.8,
            practice: false,
            eq: [0.0; 10],
            eq_on: false,
            mt_bpm: 100,
            mt_vol: 0.8,
            chord_details: false,
        }
    }
}

pub(crate) struct App {
    pub api: Api,
    pub player: Player,
    pub tx: Sender<Msg>,
    pub rx: Receiver<Msg>,
    pub ctx: egui::Context,
    pub font_tex: egui::TextureHandle,
    pub tab: Tab,
    pub note: String,
    pub note_at: Option<(String, Instant)>,
    pub keyboard: bool,
    pub zoom_set: bool,
    // ---- library
    pub logged_in: bool,
    pub code: Option<(String, String)>,
    pub lib_tab: u8,
    pub tidal_page: u8,
    pub query: String,
    pub searching: bool,
    pub results: Vec<Track>,
    pub liked: Vec<Track>,
    pub liked_ids: std::collections::HashSet<i64>,
    pub lists: Vec<Card>,
    pub list_open: Option<(String, Vec<Track>)>,
    pub busy: bool,
    pub files: Vec<(Track, std::path::PathBuf)>,
    pub yt: Vec<YtItem>,
    pub yt_link: String,
    pub yt_ask: Option<(String, String, u64)>,
    // ---- playing
    pub queue: Vec<Track>,
    pub cur: Option<usize>,
    pub gen: u64,
    pub loading: bool,
    pub paused: bool,
    pub stopped: bool,
    pub at_end: bool,
    pub seek_drag: Option<f32>,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: Repeat,
    pub sleep_at: Option<Instant>,
    pub cover: Option<(String, Option<egui::TextureHandle>)>,
    pub wave: Vec<u8>,
    pub bands: Vec<f32>,
    pub peaks: Vec<f32>,
    pub viz_wave: Vec<f32>,
    pub viz_mode: u8,
    // ---- practice
    pub practice: bool,
    pub mtab: u8,
    pub loop_a: Option<f32>,
    pub loop_b: Option<f32>,
    pub loop_on: bool,
    pub speed: u32,
    pub cents: i32,
    pub chan: u32,
    pub count_in: u32,
    pub song_bpm: f32,
    pub sections: HashMap<i64, Vec<Section>>,
    pub sec_name: String,
    pub trainer: bool,
    pub trainer_n: u32,
    pub trainer_step: u32,
    pub passes: u32,
    pub wraps: u32,
    pub marks: HashMap<i64, Vec<(f32, String)>>,
    pub eq: [f32; 10],
    pub eq_on: bool,
    // ---- chords
    pub ch_tab: u8,
    pub cf_root: Option<i32>,
    pub cf_fam: Option<usize>,
    pub chord_q: String,
    pub shape_cache: HashMap<String, Vec<[Option<u8>; 6]>>,
    pub an_frets: [Option<u8>; 6],
    pub an_pick: Option<String>,
    pub chord_sp: theory::Spelling,
    pub chord_details: bool,
    pub tr_root: i32,
    pub tr_q: usize,
    pub tr_set: usize,
    pub tr_sel: usize,
    // ---- metronome
    pub mt: band::Metro,
    pub mt_bpm: u32,
    pub mt_vol: f32,
    pub mt_vis: Option<(Instant, f32, usize)>,
    pub mt_group: usize,
    pub taps: Vec<Instant>,
    /// the skin worn: Backline's palette in the ORIGINAL look (its pictures, as on the desktop)
    pub wa_art: Option<winamp_ui::WaTex>,
    /// what the skin's title line says while a slider is moved
    pub wa_hint: Option<String>,
    pub balance: i32,
    pub kbps: u32,
    /// a song held down: its menu (the song, and where it sits in the queue when it is one of the queue's)
    pub song_menu: Option<(Track, Option<usize>)>,
    /// a run of chords sounding (HEAR THEM UP THE NECK) until then: its button says STOP meanwhile
    pub seq_end: Option<Instant>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> App {
        let ctx = cc.egui_ctx.clone();
        let st: Settings = load_json("settings.json");
        SKIN.store(st.skin % PALS.len(), Ordering::Relaxed);
        font::set_ppp(ctx.pixels_per_point());
        let font_tex = font::init(&ctx);
        // smooth text (the sleek skins, tooltips, typing) in Chakra Petch, as on the desktop
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .font_data
            .insert("chakra".to_string(), egui::FontData::from_static(include_bytes!("../../backline/assets/ChakraPetch-Regular.ttf")));
        if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            f.insert(0, "chakra".to_string());
        }
        ctx.set_fonts(fonts);
        ctx.style_mut(|s| {
            s.interaction.tooltip_delay = 0.6;
            s.spacing.scroll.bar_width = 6.0;
        });
        let (tx, rx) = channel();
        let api = Api::new();
        let player = Player::new();
        player.send(Cmd::Volume(st.volume * st.volume));
        player.send(Cmd::MetroVolume(st.mt_vol * st.mt_vol));
        player.eq.set(st.eq, 0.0, st.eq_on);
        App {
            logged_in: api.has_token(),
            api,
            player,
            tx,
            rx,
            ctx,
            font_tex,
            tab: Tab::Library,
            note: String::new(),
            note_at: None,
            keyboard: false,
            zoom_set: false,
            code: None,
            lib_tab: 0,
            tidal_page: 0,
            query: String::new(),
            searching: false,
            results: Vec::new(),
            liked: Vec::new(),
            liked_ids: Default::default(),
            lists: Vec::new(),
            list_open: None,
            busy: false,
            files: Vec::new(),
            yt: load_json("yt.json"),
            yt_link: String::new(),
            yt_ask: None,
            queue: Vec::new(),
            cur: None,
            gen: 0,
            loading: false,
            paused: false,
            stopped: true,
            at_end: false,
            seek_drag: None,
            volume: st.volume,
            shuffle: false,
            repeat: Repeat::Off,
            sleep_at: None,
            cover: None,
            wave: Vec::new(),
            bands: vec![0.0; 24],
            peaks: vec![0.0; 24],
            viz_wave: vec![0.0; 64],
            viz_mode: 0,
            practice: st.practice,
            mtab: 0,
            loop_a: None,
            loop_b: None,
            loop_on: false,
            speed: 100,
            cents: 0,
            chan: 0,
            count_in: 0,
            song_bpm: 120.0,
            sections: load_json("loops.json"),
            sec_name: String::new(),
            trainer: false,
            trainer_n: 4,
            trainer_step: 5,
            passes: 0,
            wraps: 0,
            marks: load_json("bookmarks.json"),
            eq: st.eq,
            eq_on: st.eq_on,
            ch_tab: 0,
            cf_root: None,
            cf_fam: None,
            chord_q: String::new(),
            shape_cache: HashMap::new(),
            an_frets: [None; 6],
            an_pick: None,
            chord_sp: theory::Spelling { slash: true, ..Default::default() },
            chord_details: st.chord_details,
            tr_root: 0,
            tr_q: 0,
            tr_set: 0,
            tr_sel: 0,
            mt: band::Metro::default(),
            mt_bpm: st.mt_bpm,
            mt_vol: st.mt_vol,
            mt_vis: None,
            mt_group: 0,
            taps: Vec::new(),
            wa_art: None,
            wa_hint: None,
            balance: 0,
            kbps: 0,
            song_menu: None,
            seq_end: None,
        }
    }

    /// Wear the palette picked in the ORIGINAL look: the skin drawn from it (the desktop's own generator), its
    /// window frames, its font (Silkscreen, fetched once).
    pub(crate) fn wear(&mut self) {
        let n = SKIN.load(Ordering::Relaxed) % PALS.len();
        match genskin::wear(&PALS[n], 1) {
            Ok((p, art)) => {
                skin::set_custom(Some(p));
                self.wa_art = Some(winamp_ui::WaTex::new(&self.ctx, art));
                winamp_ui::set_chrome(self.wa_art.as_ref());
                setup_style(&self.ctx);
                let skin_font = self.wa_art.as_ref().and_then(|t| t.font.clone());
                let choice = skin_font
                    .as_deref()
                    .and_then(|f| winamp_ui::PL_FONTS.iter().position(|x| x.0.eq_ignore_ascii_case(f)))
                    .unwrap_or(8);
                match winamp_ui::font_now(choice, skin_font.as_deref()) {
                    Some(b) => winamp_ui::set_playlist_font(&self.ctx, Some(b), choice),
                    None => {
                        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                        std::thread::spawn(move || {
                            let _ = tx.send(Msg::Font(choice, winamp_ui::fetch_font(choice)));
                            ctx.request_repaint();
                        });
                    }
                }
            }
            Err(e) => self.note = format!("SKIN: {}", e),
        }
    }

    pub(crate) fn save_settings(&self) {
        save_json(
            "settings.json",
            &Settings {
                skin: SKIN.load(Ordering::Relaxed),
                volume: self.volume,
                practice: self.practice,
                eq: self.eq,
                eq_on: self.eq_on,
                mt_bpm: self.mt_bpm,
                mt_vol: self.mt_vol,
                chord_details: self.chord_details,
            },
        );
    }

    /// Run `f` off the screen's thread and send what it returns back.
    pub(crate) fn spawn(&self, f: impl FnOnce(Api) -> Msg + Send + 'static) {
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(f(api));
            ctx.request_repaint();
        });
    }

    pub(crate) fn cur_track(&self) -> Option<&Track> {
        self.cur.and_then(|i| self.queue.get(i))
    }

    pub(crate) fn pos(&self) -> f32 {
        self.player.ctl.pos()
    }

    pub(crate) fn set_note(&mut self, s: &str) {
        self.note = s.to_string();
    }

    /// Play the queue's song `i`: its audio (Tidal, a file, a YouTube clip), the loop set back.
    pub(crate) fn play(&mut self, i: usize) {
        let Some(t) = self.queue.get(i).cloned() else { return };
        self.at_end = false;
        self.cur = Some(i);
        self.gen += 1;
        let g = self.gen;
        self.loading = true;
        self.paused = false;
        self.stopped = false;
        self.loop_a = None;
        self.loop_b = None;
        self.loop_on = false;
        self.passes = 0;
        self.sync_player();
        self.player.send(Cmd::Stop);
        self.wave.clear();
        let file = self.files.iter().find(|f| f.0.id == t.id).map(|f| f.1.clone());
        let yt = self.yt.iter().find(|y| yt_id(&y.vid) == t.id).map(|y| y.vid.clone());
        self.spawn(move |api| {
            let bytes = if let Some(vid) = yt {
                yt_audio(&vid)
            } else if let Some(p) = file {
                std::fs::read(&p).map_err(|e| e.to_string())
            } else {
                api.stream_url(t.id, false).and_then(|u| api.fetch(&u))
            };
            Msg::Audio(
                g,
                bytes.map(|b| {
                    let (d, w) = wave(&b);
                    (b, d, w)
                }),
            )
        });
    }

    /// The loop, speed, pitch, ear mode and count-off, to the player.
    pub(crate) fn sync_player(&self) {
        let (a, b) = (self.loop_a.unwrap_or(0.0), self.loop_b.unwrap_or(0.0));
        let on = self.practice && self.loop_on && b > a;
        self.player.ctl.set_loop(on, a, b);
        self.player.ctl.set_speed(self.speed);
        self.player.ctl.set_semis(if self.practice { self.cents } else { 0 });
        self.player.ctl.set_chan(if self.practice { self.chan } else { 0 });
        self.player.ctl.set_count_in(if self.practice { self.count_in } else { 0 }, self.song_bpm);
    }

    pub(crate) fn next(&mut self) {
        let Some(i) = self.cur else { return };
        if self.shuffle && self.queue.len() > 1 {
            let mut k = (rand() as usize) % self.queue.len();
            if k == i {
                k = (k + 1) % self.queue.len();
            }
            self.play(k);
        } else if i + 1 < self.queue.len() {
            self.play(i + 1);
        } else if self.repeat == Repeat::All {
            self.play(0);
        }
    }

    pub(crate) fn prev(&mut self) {
        let Some(i) = self.cur else { return };
        if self.pos() > 3.0 || i == 0 {
            self.player.send(Cmd::Seek(self.loop_a.unwrap_or(0.0)));
        } else {
            self.play(i - 1);
        }
    }

    pub(crate) fn play_pause(&mut self) {
        if self.loading {
            return;
        }
        if self.at_end || self.stopped {
            if let Some(i) = self.cur {
                self.play(i);
            }
            return;
        }
        self.paused = !self.paused;
        self.player.send(if self.paused { Cmd::Pause } else { Cmd::Resume });
    }

    pub(crate) fn stop(&mut self) {
        self.player.send(Cmd::Stop);
        self.stopped = true;
        self.paused = false;
    }

    pub(crate) fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(0.0, 1.0);
        self.player.send(Cmd::Volume(self.volume * self.volume));
        self.save_settings();
    }

    /// These notes heard (a soft strum).
    pub(crate) fn hear(&self, notes: &[i32]) {
        if !notes.is_empty() {
            self.player.send(Cmd::Once(band::chord_sound(notes), 0.9));
        }
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
                    self.note = format!("SIGN-IN FAILED: {}", e);
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
                        Err(e) => self.note = format!("SEARCH FAILED: {}", e),
                    }
                }
                Msg::Liked(r) => {
                    self.busy = false;
                    match r {
                        Ok(t) => {
                            self.liked_ids = t.iter().map(|t| t.id).collect();
                            self.liked = t;
                        }
                        Err(e) => self.note = format!("COULDN'T LOAD YOUR LIKED SONGS: {}", e),
                    }
                }
                Msg::Lists(r) => {
                    self.busy = false;
                    match r {
                        Ok(l) => self.lists = l,
                        Err(e) => self.note = format!("COULDN'T LOAD YOUR PLAYLISTS: {}", e),
                    }
                }
                Msg::ListTracks(name, r) => {
                    self.busy = false;
                    match r {
                        Ok(t) => self.list_open = Some((name, t)),
                        Err(e) => self.note = format!("COULDN'T OPEN {}: {}", name.to_uppercase(), e),
                    }
                }
                Msg::YtAdded(r) => {
                    self.busy = false;
                    match r {
                        Ok(items) => {
                            let n = items.len();
                            for it in items {
                                if !self.yt.iter().any(|y| y.vid == it.vid) {
                                    self.yt.push(it);
                                }
                            }
                            save_json("yt.json", &self.yt);
                            self.yt_link.clear();
                            self.note = format!("ADDED {} FROM YOUTUBE", n);
                        }
                        Err(e) => self.note = format!("YOUTUBE: {}", e),
                    }
                }
                Msg::YtAsk(link, name, n) => {
                    self.busy = false;
                    self.yt_ask = Some((link, name, n));
                }
                Msg::Cover(url, img) => {
                    if self.cover.as_ref().is_some_and(|c| c.0 == url) {
                        let tex = img.map(|i| self.ctx.load_texture(&url, i, egui::TextureOptions::LINEAR));
                        self.cover = Some((url, tex));
                    }
                }
                Msg::Meta(id, info) => {
                    meta::set_detected(id, info);
                    if self.cur_track().map(|t| t.id) == Some(id) {
                        if let Some(b) = meta::get(id).and_then(|i| i.bpm) {
                            self.song_bpm = b as f32;
                            self.sync_player();
                        }
                    }
                }
                Msg::Audio(g, r) if g == self.gen => {
                    self.loading = false;
                    match r {
                        Ok((b, d, w)) => {
                            self.wave = w;
                            let id = self.cur_track().map(|t| t.id).unwrap_or(0);
                            if d > 0.0 {
                                if let Some(t) = self.cur.and_then(|i| self.queue.get_mut(i)) {
                                    t.duration = d;
                                }
                            }
                            // its tempo and key, worked out once (as on the desktop)
                            if meta::claim(id) {
                                let bytes = b.clone();
                                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                                std::thread::spawn(move || {
                                    let _ = tx.send(Msg::Meta(id, meta::detect(id, bytes)));
                                    ctx.request_repaint();
                                });
                            }
                            if let Some(bpm) = meta::get(id).and_then(|i| i.bpm) {
                                self.song_bpm = bpm as f32;
                            }
                            self.kbps = if d > 1.0 { (b.len() as f32 * 8.0 / 1000.0 / d) as u32 } else { 0 };
                            self.wraps = self.player.ctl.wraps();
                            self.sync_player();
                            self.player.send(Cmd::Play(b));
                        }
                        Err(e) => {
                            self.stopped = true;
                            self.note = format!("CAN'T PLAY: {}", e);
                        }
                    }
                }
                Msg::Font(choice, r) => {
                    if let Ok(b) = r {
                        winamp_ui::set_playlist_font(&self.ctx, Some(b), choice);
                    }
                }
                Msg::Audio(..) => {}
            }
        }
        // the song ended: on to the next (or round again)
        let ended = {
            let mut s = self.player.shared.lock().unwrap();
            std::mem::take(&mut s.ended)
        };
        if ended && !self.loading {
            if self.repeat == Repeat::One {
                if let Some(i) = self.cur {
                    self.play(i);
                }
            } else {
                let last = self.cur.map_or(true, |i| i + 1 >= self.queue.len());
                if last && self.repeat == Repeat::Off && !self.shuffle {
                    self.paused = true;
                    self.at_end = true;
                } else {
                    self.next();
                }
            }
        }
        // the sleep timer: pause when it runs out
        if self.sleep_at.is_some_and(|t| Instant::now() >= t) {
            self.sleep_at = None;
            if !self.paused {
                self.play_pause();
            }
        }
        // the trainer: every N times round the loop, a little faster (up to 100%)
        if self.practice && self.loop_on {
            let w = self.player.ctl.wraps();
            if w != self.wraps {
                self.wraps = w;
                if self.trainer {
                    self.passes += 1;
                    if self.passes >= self.trainer_n.max(1) {
                        self.passes = 0;
                        if self.speed < 100 {
                            self.speed = (self.speed + self.trainer_step).min(100);
                            self.sync_player();
                        }
                    }
                }
            }
        }
    }

    /// The visualizer's bars, peaks and wave from what is sounding (the desktop's own analysis).
    fn update_bands(&mut self) {
        let active = self.cur.is_some() && !self.paused && !self.stopped && !self.loading;
        let (samples, rate) = if active {
            let v = self.player.viz.lock().unwrap();
            let n = v.samples.len().min(2048);
            (v.samples[v.samples.len() - n..].to_vec(), v.rate as f32)
        } else {
            (Vec::new(), 44100.0)
        };
        let n = samples.len();
        let nb = self.bands.len();
        let mut target = vec![0.0f32; nb];
        if n >= 512 && rate > 0.0 {
            let denom = (n - 1) as f32;
            let xs: Vec<f32> =
                samples.iter().enumerate().map(|(i, v)| v * (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / denom).cos())).collect();
            for i in 0..nb {
                let f = 50.0 * (12000.0f32 / 50.0).powf(i as f32 / (nb - 1) as f32);
                if f >= rate / 2.0 {
                    continue;
                }
                let amp = goertzel(&xs, f, rate) * 2.0;
                let db = 20.0 * (amp + 1e-6).log10();
                target[i] = ((db + 64.0 + 23.0 * i as f32 / nb as f32) / 54.0).clamp(0.0, 1.0);
            }
        }
        let wn = self.viz_wave.len();
        for k in 0..wn {
            let tv = if n >= wn {
                let (a, b) = (k * n / wn, ((k + 1) * n / wn).max(k * n / wn + 1));
                let mut best = 0.0f32;
                for v in &samples[a..b.min(n)] {
                    if v.abs() > best.abs() {
                        best = *v;
                    }
                }
                (best * 1.6).clamp(-1.0, 1.0)
            } else {
                0.0
            };
            self.viz_wave[k] = self.viz_wave[k] * 0.45 + tv * 0.55;
        }
        for i in 0..nb {
            let old = self.bands[i];
            self.bands[i] = if target[i] > old { target[i] } else { (old - 0.06).max(target[i]) };
            self.peaks[i] = self.bands[i].max(self.peaks[i] - 0.012);
        }
    }

    /// The top bar: Backline's logo, and the skin (all seventeen of the desktop's).
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // the desktop's logo mark and name, sized for a phone
            let (r, _) = ui.allocate_exact_size(Vec2::new(150.0, 30.0), Sense::hover());
            logo_mark(ui.painter(), Pos2::new(r.min.x + 2.0, r.min.y + 3.0), 2.7);
            // (on the app background, in the window titles' colour)
            ptext(ui.painter(), Pos2::new(r.min.x + 34.0, r.center().y), Align::Min, "BACKLINE", 3.0, pal().trim);
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let k = SKIN.load(Ordering::Relaxed);
                if retro_btn(ui, &format!("SKIN: {}", SKIN_NAMES[k]), false).clicked() {
                    SKIN.store((k + 1) % PALS.len(), Ordering::Relaxed);
                    self.wear();
                    self.save_settings();
                }
            });
        });
    }

    /// The screens along the bottom, as the desktop's big tabs.
    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        let items = ["PLAYER", "MUSIC", "CHORDS", "METRO"];
        let tabs = [Tab::Player, Tab::Library, Tab::Chords, Tab::Metro];
        let cur = tabs.iter().position(|t| *t == self.tab).unwrap_or(0);
        let w = (ui.available_width() - 4.0 * 3.0) / 4.0 - 0.5;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (k, name) in items.iter().enumerate() {
                if retro_btn_w(ui, name, w, k == cur).clicked() {
                    self.tab = tabs[k];
                }
            }
        });
    }

    /// While browsing: the song playing along the bottom, in an LCD (tap it for the player).
    fn now_bar(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.cur_track().cloned() else { return };
        ui.horizontal(|ui| {
            let w = ui.available_width() - 2.0 * (bh() + 6.0);
            let (r, resp) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::click());
            inset(ui.painter(), r, pal().lcd);
            marquee(ui, r.shrink2(Vec2::new(4.0, 0.0)), &format!("{} - {}", t.artists_text(), t.title), 1.0, pal().ink);
            if resp.clicked() {
                self.tab = Tab::Player;
            }
            let icon: &[&str] = if self.paused || self.stopped { &IC_PLAY } else { &IC_PAUSE };
            if icon_btn_w(ui, icon, false, pal().ink, bh()).clicked() {
                self.play_pause();
            }
            if icon_btn_w(ui, &IC_NEXT, false, pal().ink, bh()).clicked() {
                self.next();
            }
        });
    }

    /// The menu of a song held down (the desktop's right-click): play it next, add it to the queue, and in the
    /// queue move it or take it out.
    fn song_menu_ui(&mut self, ctx: &egui::Context) {
        let Some((t, qi)) = self.song_menu.clone() else { return };
        let screen = ctx.screen_rect();
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("menu_dim")))
            .rect_filled(screen, 0.0, Color32::from_black_alpha(150));
        let mut close = false;
        // it slides up from the bottom
        let k = ctx.animate_bool_with_time(egui::Id::new("song_menu_in"), true, 0.18);
        let drop_px = (1.0 - k) * 320.0;
        // a tap on the dimmed screen closes it
        if ctx.input(|i| i.pointer.any_click()) && !ctx.is_pointer_over_area() {
            close = true;
        }
        egui::Area::new(egui::Id::new("song_menu")).order(egui::Order::Tooltip).anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, drop_px)).show(ctx, |ui| {
            egui::Frame::none().fill(pal().beige).stroke(Stroke::new(2.0, pal().edge)).inner_margin(egui::Margin { left: 12.0, right: 12.0, top: 12.0, bottom: 34.0 }).show(ui, |ui| {
                ui.set_width(screen.width() - 24.0);
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                title_line(ui, &fit(&t.title.to_uppercase(), 2.0, screen.width() - 80.0), 2.0, pal().ink);
                dim_line(ui, &t.artists_text().to_uppercase(), 1.0, pal().ink2);
                let w = ui.available_width();
                let cur = self.cur;
                if retro_btn_w(ui, "PLAY NEXT", w, false).clicked() {
                    let at = cur.map_or(0, |c| c + 1).min(self.queue.len());
                    if let Some(i) = qi {
                        // a queue song moved to just after the one playing
                        let s = self.queue.remove(i);
                        let at = if i < at { at - 1 } else { at };
                        self.queue.insert(at, s);
                        self.fix_cur(i, at);
                    } else {
                        self.queue.insert(at, t.clone());
                    }
                    self.set_note("PLAYS NEXT");
                    close = true;
                }
                if qi.is_none() && retro_btn_w(ui, "ADD TO QUEUE", w, false).clicked() {
                    self.queue.push(t.clone());
                    self.set_note("ADDED TO THE QUEUE");
                    close = true;
                }
                if let Some(i) = qi {
                    ui.horizontal(|ui| {
                        let hw = (w - 6.0) / 2.0;
                        if retro_btn_w(ui, "MOVE UP", hw, false).clicked() && i > 0 {
                            self.queue.swap(i, i - 1);
                            self.fix_cur(i, i - 1);
                            self.song_menu = Some((t.clone(), Some(i - 1)));
                        }
                        if retro_btn_w(ui, "MOVE DOWN", hw, false).clicked() && i + 1 < self.queue.len() {
                            self.queue.swap(i, i + 1);
                            self.fix_cur(i, i + 1);
                            self.song_menu = Some((t.clone(), Some(i + 1)));
                        }
                    });
                    if retro_btn_w(ui, "TAKE OUT OF THE QUEUE", w, false).clicked() {
                        self.queue.remove(i);
                        match self.cur {
                            Some(c) if c == i => {
                                self.stop();
                                self.cur = None;
                            }
                            Some(c) if c > i => self.cur = Some(c - 1),
                            _ => {}
                        }
                        close = true;
                    }
                }
                // a YouTube clip: off the YT list too
                if let Some(k) = self.yt.iter().position(|y| yt_id(&y.vid) == t.id) {
                    if retro_btn_w(ui, "TAKE OFF THE YT LIST", w, false).clicked() {
                        self.yt.remove(k);
                        save_json("yt.json", &self.yt);
                        close = true;
                    }
                }
                if retro_btn_w(ui, "CANCEL", w, false).clicked() {
                    close = true;
                }
            });
        });
        if close {
            self.song_menu = None;
            ctx.animate_bool_with_time(egui::Id::new("song_menu_in"), false, 0.0);
        }
    }

    /// The song playing keeps its place when the queue moves under it (a song moved from `from` to `to`).
    fn fix_cur(&mut self, from: usize, to: usize) {
        if let Some(c) = self.cur {
            self.cur = Some(if c == from {
                to
            } else if from < c && to >= c {
                c - 1
            } else if from > c && to <= c {
                c + 1
            } else {
                c
            });
        }
    }

    /// A window as the desktop draws them (its title on a tab), filling `outer`; the contents go inside.
    pub(crate) fn window(&mut self, ui: &mut egui::Ui, outer: Rect, title: &str, body: impl FnOnce(&mut Self, &mut egui::Ui)) {
        let inner = Rect::from_min_max(outer.min + Vec2::new(14.0, 30.0), outer.max - Vec2::new(14.0, 14.0));
        window_deco(ui, inner, title);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner.shrink(4.0)).layout(egui::Layout::top_down(Align::Min)));
        child.set_clip_rect(inner);
        body(self, &mut child);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Backline's drawing is sized for a window about 340 points across: the phone is zoomed to that
        if !self.zoom_set {
            let w = ctx.screen_rect().width();
            if w > 50.0 {
                ctx.set_zoom_factor(ctx.zoom_factor() * w / 340.0);
                self.zoom_set = true;
            }
        }
        font::set_ppp(ctx.pixels_per_point());
        font::set_modern(style() != 0);
        self.drain();
        self.update_bands();
        // the phone's keyboard comes up while a text box is being typed in
        let want = ctx.wants_keyboard_input();
        if want != self.keyboard {
            self.keyboard = want;
            show_keyboard(want);
        }
        // a note stays a few seconds
        if self.note_at.as_ref().map(|n| &n.0) != Some(&self.note) {
            self.note_at = Some((self.note.clone(), Instant::now()));
        }
        if let Some((_, at)) = &self.note_at {
            if !self.note.is_empty() && at.elapsed().as_secs_f32() > 6.0 {
                self.note.clear();
            }
        }
        if !self.note.is_empty() || (self.cur.is_some() && !self.paused) || self.mt.on {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
        let fill = egui::Frame::none().fill(pal().app_bg);
        egui::TopBottomPanel::top("top")
            .frame(fill.inner_margin(egui::Margin { left: 8.0, right: 8.0, top: 30.0, bottom: 4.0 }))
            .show(ctx, |ui| self.top_bar(ui));
        egui::TopBottomPanel::bottom("bottom")
            .frame(fill.inner_margin(egui::Margin { left: 8.0, right: 8.0, top: 6.0, bottom: 26.0 }))
            .show(ctx, |ui| {
                if !self.note.is_empty() {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bh()), Sense::click());
                    inset(ui.painter(), r, pal().lcd);
                    let n = self.note.clone();
                    let bad = n.contains("FAILED") || n.contains("CAN'T") || n.contains("COULDN'T") || n.starts_with("YOUTUBE:");
                    marquee(ui, r.shrink2(Vec2::new(4.0, 0.0)), &n, 1.0, if bad { pal().red } else { pal().ink });
                    if resp.clicked() {
                        self.note.clear();
                    }
                    ui.add_space(4.0);
                }
                if self.tab != Tab::Player {
                    self.now_bar(ui);
                    ui.add_space(4.0);
                }
                self.bottom_bar(ui);
            });
        self.song_menu_ui(ctx);
        egui::CentralPanel::default().frame(fill).show(ctx, |ui| {
            let r = ui.max_rect();
            match self.tab {
                Tab::Player => self.player_screen(ui, r),
                Tab::Library => self.window(ui, r, "LIBRARY", |me, ui| me.library_ui(ui)),
                Tab::Chords => self.window(ui, r, "CHORDS", |me, ui| me.chords_ui(ui)),
                Tab::Metro => self.window(ui, r, "METRONOME", |me, ui| me.metro_ui(ui)),
            }
        });
    }
}

fn rand() -> u64 {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) as u64;
    n ^ (n >> 17).wrapping_mul(0x9E37_79B9_7F4A_7C15)
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
    // a test phone (a file named MUTE in Backline's folder) plays silently, like the desktop's test copies
    if std::env::var_os("BACKLINE_FILES").is_some_and(|d| std::path::Path::new(&d).join("MUTE").exists()) {
        player::MUTED.store(true, Ordering::Relaxed);
    }
    let _ = ANDROID.set(app.clone());
    let options = eframe::NativeOptions {
        event_loop_builder: Some(Box::new(move |b| {
            b.with_android_app(app);
        })),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Backline",
        options,
        Box::new(|cc| {
            let mut app = App::new(cc);
            app.wear();
            Ok(Box::new(app))
        }),
    );
}
