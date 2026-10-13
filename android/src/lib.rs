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
#[path = "../../backline/src/data/theory.rs"]
mod theory;
#[path = "../../backline/src/audio/tuning.rs"]
mod tuning;
// Backline's look: the pixel font, the skins, the icons and the drawing kit (buttons, LCDs, tabs...)
#[path = "../../backline/src/ui/font.rs"]
mod font;
#[path = "../../backline/src/ui/icons.rs"]
mod icons;
#[path = "../../backline/src/ui/skin.rs"]
mod skin;
#[path = "../../backline/src/ui/widgets.rs"]
mod widgets;
#[path = "../../backline/src/ui/vicon.rs"]
mod vicon;
#[path = "../../backline/src/ui/fretboard.rs"]
mod fretboard;
#[path = "../../backline/src/ui/viz.rs"]
mod viz;
#[allow(unused_imports)]
use eframe::egui::{Align, Pos2, Rect, Rounding, Stroke};
#[allow(unused_imports)]
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
#[allow(unused_imports)]
use font::{fit, ptext, ptext_fit, snap, spx, text_w, thick, wrap};
#[allow(unused_imports)]
use skin::*;

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
#[allow(unused_imports)]
use icons::*;
#[allow(unused_imports)]
use widgets::*;

/// Winamp skins are a desktop thing: the phone never wears one, so the skin font is never asked for.
mod winamp_ui {
    use eframe::egui::{self, Align, Color32, Pos2};
    pub(crate) fn text_override() -> bool {
        false
    }
    pub(crate) fn sans_width(_text: &str, _size: f32) -> f32 {
        0.0
    }
    pub(crate) fn sans_text(_p: &egui::Painter, _pos: Pos2, _align: Align, _text: &str, _size: f32, _max_w: f32, _col: Color32) -> f32 {
        0.0
    }
    pub(crate) fn chrome_on() -> bool {
        false
    }
    pub(crate) fn draw_frame(_p: &egui::Painter, _outer: eframe::egui::Rect, _inner: eframe::egui::Rect, _title: &str) -> bool {
        false
    }
}

use api::{Api, Card, DeviceCode, Track};
use eframe::egui::{self, Color32, RichText, Sense, Vec2};
use player::{Cmd, Player};
use std::sync::mpsc::{channel, Receiver, Sender};

enum Msg {
    Code(Result<DeviceCode, String>),
    LoggedIn(Result<(), String>),
    Found(Result<Vec<Track>, String>),
    Liked(Result<Vec<Track>, String>),
    Lists(Result<Vec<Card>, String>),
    ListTracks(String, Result<Vec<Track>, String>),
    /// YT: clips read from a link (a video, or a playlist's videos)
    YtAdded(Result<Vec<YtItem>, String>),
    /// YT: a playlist with this many videos - ask before adding them all (link, name, count)
    YtAsk(String, String, u64),
    /// a cover picture fetched: its address and pixels
    Cover(String, Option<egui::ColorImage>),
    /// the song's audio, and its length when the file had to tell us (0: the track already knows)
    Audio(u64, Result<(Vec<u8>, f32, Vec<u8>), String>),
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Player,
    Library,
    Chords,
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
    /// the Tidal tab's page: 0 search, 1 liked, 2 playlists
    tidal_page: u8,
    liked: Vec<Track>,
    lists: Vec<Card>,
    /// a playlist opened: its name and songs
    list_open: Option<(String, Vec<Track>)>,
    busy: bool,
    /// CHORDS: 0 find, 1 analyze, 2 triads
    ch_tab: u8,
    ch_query: String,
    an_frets: [Option<u8>; 6],
    tr_root: i32,
    tr_q: usize,
    tr_set: usize,
    /// the metronome (the desktop's own click), on since when (for the beat lights)
    metro: band::Metro,
    metro_since: Option<std::time::Instant>,
    /// YT: the clips added, the link being typed, a big playlist waiting for a yes
    yt: Vec<YtItem>,
    yt_link: String,
    yt_ask: Option<(String, String, u64)>,
    /// the cover shown on the player: its address and picture
    cover: Option<(String, Option<egui::TextureHandle>)>,
    /// when the note on top was put up (it goes after a few seconds)
    note_at: Option<(String, std::time::Instant)>,
    /// PRACTICE on: the waveform and the loop tools; off: the cover and the bars (listening)
    practice: bool,
    /// the queue played to its end: play starts the song again
    at_end: bool,
    bars: [f32; 24],
}

/// A YouTube clip in the YT list.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct YtItem {
    vid: String,
    title: String,
    channel: String,
    secs: f32,
}

impl YtItem {
    fn track(&self) -> Track {
        Track { id: yt_id(&self.vid), title: self.title.clone(), artist: self.channel.clone(), duration: self.secs, ..Default::default() }
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> App {
        let ctx = cc.egui_ctx.clone();
        theme(&ctx);
        ctx.style_mut(|s| s.visuals.selection.bg_fill = pal::ACCENT);
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
            tidal_page: 0,
            liked: Vec::new(),
            lists: Vec::new(),
            list_open: None,
            busy: false,
            ch_tab: 0,
            ch_query: "Cmaj7".to_string(),
            an_frets: [None; 6],
            tr_root: 0,
            tr_q: 0,
            tr_set: 0,
            metro: band::Metro::default(),
            metro_since: None,
            yt: load_json("yt.json"),
            yt_link: String::new(),
            yt_ask: None,
            cover: None,
            note_at: None,
            practice: false,
            at_end: false,
            bars: [0.0; 24],
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
        self.at_end = false;
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
        // a YouTube clip: its audio (kept, so it plays again without the internet)
        if let Some(vid) = self.yt.iter().find(|y| yt_id(&y.vid) == t.id).map(|y| y.vid.clone()) {
            self.spawn(move |_| {
                Msg::Audio(
                    g,
                    yt_audio(&vid).map(|b| {
                        let (d, w) = wave(&b);
                        (b, d, w)
                    }),
                )
            });
            return;
        }
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

    /// The picture for this song: the album cover (Tidal), the video's thumbnail (YT); none for files.
    fn cover_url(&self, t: &Track) -> Option<String> {
        if let Some(y) = self.yt.iter().find(|y| yt_id(&y.vid) == t.id) {
            return Some(format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", y.vid));
        }
        (!t.cover.is_empty()).then(|| api::cover_url(&t.cover, 640))
    }

    /// The player's cover card: the picture with the song's name over a dark fade on its bottom half (or, with no
    /// picture, Backline's own card).
    fn cover_card(&mut self, ui: &mut egui::Ui, t: &Track, tall: f32) {
        let url = self.cover_url(t);
        if self.cover.as_ref().map(|c| Some(&c.0)) != Some(url.as_ref()) {
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
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, (w * tall).min(360.0)), Sense::hover());
        let p = ui.painter_at(r);
        p.rect_filled(r, 6.0, pal::PANEL);
        match self.cover.as_ref().and_then(|c| c.1.as_ref()) {
            Some(tex) => {
                // fill the card, cropped to its shape
                let ts = tex.size_vec2();
                let scale = (r.width() / ts.x).max(r.height() / ts.y);
                let shown = ts * scale;
                let img = egui::Rect::from_center_size(r.center(), shown);
                p.image(tex.id(), img, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
            }
            None => {
                // Backline's card: a soft light rectangle with the diamond
                let inner = r.shrink(r.width() * 0.18);
                p.rect_filled(inner, 4.0, Color32::from_gray(225));
                let c = inner.center() - Vec2::new(0.0, inner.height() * 0.08);
                let d = inner.height() * 0.16;
                p.add(egui::Shape::convex_polygon(
                    vec![c + Vec2::new(0.0, -d), c + Vec2::new(d, 0.0), c + Vec2::new(0.0, d), c + Vec2::new(-d, 0.0)],
                    Color32::from_gray(150),
                    egui::Stroke::NONE,
                ));
            }
        }
        // the dark fade over the bottom half, then the song's name on it
        let half = egui::Rect::from_min_max(egui::pos2(r.min.x, r.center().y), r.max);
        let steps = 24;
        for k in 0..steps {
            let y0 = half.min.y + half.height() * k as f32 / steps as f32;
            let y1 = half.min.y + half.height() * (k + 1) as f32 / steps as f32;
            let a = ((k as f32 + 1.0) / steps as f32).powf(1.3) * 235.0;
            p.rect_filled(egui::Rect::from_x_y_ranges(r.min.x..=r.max.x, y0..=y1), 0.0, Color32::from_rgba_unmultiplied(12, 13, 17, a as u8));
        }
        p.text(r.left_bottom() + Vec2::new(14.0, -40.0), egui::Align2::LEFT_BOTTOM, &t.title, egui::FontId::proportional(24.0), pal::INK);
        p.text(r.left_bottom() + Vec2::new(14.0, -14.0), egui::Align2::LEFT_BOTTOM, t.artists_text().to_uppercase(), egui::FontId::proportional(14.0), pal::INK2);
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
                Msg::Liked(r) => {
                    self.busy = false;
                    match r {
                        Ok(t) => self.liked = t,
                        Err(e) => self.note = format!("Couldn't load your liked songs: {}", e),
                    }
                }
                Msg::Lists(r) => {
                    self.busy = false;
                    match r {
                        Ok(l) => self.lists = l,
                        Err(e) => self.note = format!("Couldn't load your playlists: {}", e),
                    }
                }
                Msg::ListTracks(name, r) => {
                    self.busy = false;
                    match r {
                        Ok(t) => self.list_open = Some((name, t)),
                        Err(e) => self.note = format!("Couldn't open {}: {}", name, e),
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
                            self.note = format!("Added {} from YouTube", n);
                        }
                        Err(e) => self.note = format!("YouTube: {}", e),
                    }
                }
                Msg::Cover(url, img) => {
                    if self.cover.as_ref().is_some_and(|c| c.0 == url) {
                        let tex = img.map(|i| self.ctx.load_texture(&url, i, egui::TextureOptions::LINEAR));
                        self.cover = Some((url, tex));
                    }
                }
                Msg::YtAsk(link, name, n) => {
                    self.busy = false;
                    self.yt_ask = Some((link, name, n));
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
            match self.cur.filter(|i| i + 1 < self.queue.len()) {
                Some(i) => self.play(i + 1),
                None => {
                    self.paused = true;
                    self.at_end = true;
                }
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
        ui.columns(3, |c| {
            for (k, name) in ["TIDAL", "FILES", "YT"].into_iter().enumerate() {
                if c[k].add_sized(Vec2::new(c[k].available_width(), 40.0), chip(self.lib_tab == k as u8, name)).clicked() {
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
        if self.lib_tab == 2 {
            self.yt_ui(ui);
            return;
        }
        if !self.logged_in {
            self.login_ui(ui);
            return;
        }
        ui.horizontal(|ui| {
            for (k, name) in ["SEARCH", "LIKED", "PLAYLISTS"].into_iter().enumerate() {
                if ui.add(chip(self.tidal_page == k as u8, name)).clicked() {
                    self.tidal_page = k as u8;
                    self.list_open = None;
                    if k == 1 && self.liked.is_empty() && !self.busy {
                        self.busy = true;
                        self.spawn(|api| {
                            // every liked song, a page at a time
                            let mut all = Vec::new();
                            let mut r = Ok(());
                            loop {
                                match api.fav_tracks(all.len()) {
                                    Ok((t, _, total)) => {
                                        let n = t.len();
                                        all.extend(t);
                                        if n == 0 || all.len() >= total || all.len() >= 2000 {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        r = Err(e);
                                        break;
                                    }
                                }
                            }
                            Msg::Liked(r.map(|_| all))
                        });
                    }
                    if k == 2 && self.lists.is_empty() && !self.busy {
                        self.busy = true;
                        self.spawn(|api| Msg::Lists(api.my_playlists()));
                    }
                }
            }
        });
        if self.busy {
            ui.spinner();
        }
        if self.tidal_page == 1 {
            let mut play = None;
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for (i, t) in self.liked.iter().enumerate() {
                    if track_row(ui, t, self.cur_track().map(|c| c.id) == Some(t.id)).clicked() {
                        play = Some(i);
                    }
                }
            });
            if let Some(i) = play {
                self.queue = self.liked.clone();
                self.play(i);
            }
            return;
        }
        if self.tidal_page == 2 {
            if let Some((name, tracks)) = self.list_open.clone() {
                ui.horizontal(|ui| {
                    if ui.button("< BACK").clicked() {
                        self.list_open = None;
                    }
                    ui.label(RichText::new(name.as_str()).strong());
                });
                let mut play = None;
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    for (i, t) in tracks.iter().enumerate() {
                        if track_row(ui, t, self.cur_track().map(|c| c.id) == Some(t.id)).clicked() {
                            play = Some(i);
                        }
                    }
                });
                if let Some(i) = play {
                    self.queue = tracks;
                    self.play(i);
                }
                return;
            }
            let mut open = None;
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for c in &self.lists {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 52.0), Sense::click());
                    ui.painter().text(r.min + Vec2::new(8.0, 8.0), egui::Align2::LEFT_TOP, &c.title, egui::FontId::proportional(17.0), Color32::from_gray(235));
                    ui.painter().text(r.min + Vec2::new(8.0, 30.0), egui::Align2::LEFT_TOP, &c.subtitle, egui::FontId::proportional(13.0), Color32::from_gray(150));
                    if resp.clicked() {
                        open = Some(c.clone());
                    }
                }
            });
            if let Some(c) = open {
                self.busy = true;
                self.spawn(move |api| Msg::ListTracks(c.title.clone(), api.open(&c).map(|p| p.tracks)));
            }
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

    /// YT: paste a YouTube link (a video or a playlist) and its audio plays like any song.
    fn yt_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.yt_link).hint_text("A YouTube link").desired_width(ui.available_width() - 170.0));
            if ui.button("PASTE").clicked() {
                if let Some(s) = clipboard() {
                    self.yt_link = s.trim().to_string();
                }
            }
            if ui.button("ADD").clicked() && !self.busy {
                self.yt_add(self.yt_link.trim().to_string(), None);
            }
        });
        if self.busy {
            ui.spinner();
        }
        // a big playlist: only with a yes
        if let Some((link, name, n)) = self.yt_ask.clone() {
            ui.label(RichText::new(format!("\"{}\" has {} videos.", name, n)).strong());
            ui.horizontal(|ui| {
                if ui.button(format!("ADD ALL {}", n)).clicked() {
                    self.yt_ask = None;
                    self.yt_add(link.clone(), Some(n as usize));
                }
                if ui.button("FIRST 25").clicked() {
                    self.yt_ask = None;
                    self.yt_add(link.clone(), Some(25));
                }
                if ui.button("CANCEL").clicked() {
                    self.yt_ask = None;
                }
            });
        }
        if self.yt.is_empty() {
            ui.label(RichText::new("Copy a link in the YouTube app (Share > Copy link), then PASTE and ADD.").weak());
            return;
        }
        let mut play = None;
        let mut drop = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (i, y) in self.yt.iter().enumerate() {
                let t = y.track();
                let r = track_row(ui, &t, self.cur_track().map(|c| c.id) == Some(t.id));
                if r.clicked() {
                    play = Some(i);
                }
                // hold a clip to take it off the list
                if r.long_touched() || r.secondary_clicked() {
                    drop = Some(i);
                }
            }
        });
        if let Some(i) = drop {
            self.yt.remove(i);
            save_json("yt.json", &self.yt);
            self.note = "Taken off the list".to_string();
        } else if let Some(i) = play {
            self.queue = self.yt.iter().map(|y| y.track()).collect();
            self.play(i);
        }
    }

    /// Read a link: a video is added; a playlist is added up to `limit` videos, asking first when it is big.
    fn yt_add(&mut self, link: String, limit: Option<usize>) {
        if link.is_empty() {
            return;
        }
        self.busy = true;
        self.spawn(move |_| {
            let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
                Ok(r) => r,
                Err(e) => return Msg::YtAdded(Err(e.to_string())),
            };
            rt.block_on(async move {
                let rp = match yt_client() {
                    Ok(r) => r,
                    Err(e) => return Msg::YtAdded(Err(e)),
                };
                if let Some(list) = list_id(&link) {
                    let mut pl = match rp.query().playlist(&list).await {
                        Ok(p) => p,
                        Err(e) => return Msg::YtAdded(Err(e.to_string())),
                    };
                    let n = pl.video_count;
                    let Some(limit) = limit.or((n <= 25).then_some(25)) else {
                        return Msg::YtAsk(link.clone(), pl.name.clone(), n);
                    };
                    let _ = pl.videos.extend_limit(rp.query(), limit).await;
                    let items = pl
                        .videos
                        .items
                        .iter()
                        .take(limit)
                        .map(|v| YtItem {
                            vid: v.id.clone(),
                            title: v.name.clone(),
                            channel: v.channel.as_ref().map(|c| c.name.clone()).unwrap_or_default(),
                            secs: v.duration.unwrap_or(0) as f32,
                        })
                        .collect();
                    return Msg::YtAdded(Ok(items));
                }
                let Some(vid) = video_id(&link) else {
                    return Msg::YtAdded(Err("that doesn't look like a YouTube link".to_string()));
                };
                match rp.query().player(&vid).await {
                    Ok(p) => Msg::YtAdded(Ok(vec![YtItem {
                        vid,
                        title: p.details.name.clone().unwrap_or_default(),
                        channel: p.details.channel_name.clone().unwrap_or_default(),
                        secs: p.details.duration as f32,
                    }])),
                    Err(e) => Msg::YtAdded(Err(e.to_string())),
                }
            })
        });
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
            ui.add_space(30.0);
            ui.vertical_centered(|ui| ui.label(RichText::new("Pick a song in the library").weak()));
            ui.add_space(30.0);
            self.metro_ui(ui);
            return;
        };
        // listening or practising: PRACTICE turns the tools on
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_sized(Vec2::new(110.0, 36.0), chip(self.practice, "PRACTICE")).clicked() {
                    self.practice = !self.practice;
                    if !self.practice {
                        self.more = false;
                    }
                }
            });
        });
        self.cover_card(ui, &t, if self.practice { 0.42 } else { 0.85 });
        ui.add_space(10.0);
        if !self.practice {
            self.listen_ui(ui, &t);
            return;
        }
        // where the song is: drag to move
        let pos = self.seek_drag.unwrap_or_else(|| self.player.ctl.pos());
        let dur = t.duration.max(1.0);
        let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 90.0), Sense::click_and_drag());
        let p = ui.painter();
        p.rect_filled(r, 4.0, pal::LCD);
        let x_of = |t: f32| r.min.x + r.width() * (t / dur).clamp(0.0, 1.0);
        // the waveform, the part played brighter
        if !self.wave.is_empty() {
            let n = self.wave.len();
            let w = r.width() / n as f32;
            for (k, v) in self.wave.iter().enumerate() {
                let x = r.min.x + k as f32 * w;
                let h = (*v as f32 / 255.0) * (r.height() * 0.45);
                let played = x < x_of(pos);
                let c = if played { pal::ACCENT_HI } else { pal::GROOVE };
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
        self.transport(ui, pos);
        ui.add_space(16.0);
        // practice: the loop and the speed
        ui.horizontal_wrapped(|ui| {
            let now = self.player.ctl.pos();
            // fixed widths, so nothing moves as the times fill in
            let fixed = |ui: &mut egui::Ui, on: bool, s: String| ui.add_sized(Vec2::new(78.0, 44.0), chip(on, s));
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
            if ui.add(chip(self.more, "MORE")).clicked() {
                self.more = !self.more;
                self.show_queue = false;
            }
            if ui.add(chip(self.show_queue, "QUEUE")).clicked() {
                self.show_queue = !self.show_queue;
                self.more = false;
            }
        });
        if self.more {
            self.more_ui(ui, &t);
        }
        if self.show_queue {
            self.queue_list(ui);
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
    }

    /// Listening: the bars, where the song is, the transport and the queue - nothing else.
    fn listen_ui(&mut self, ui: &mut egui::Ui, t: &Track) {
        self.bars_ui(ui);
        let pos = self.seek_drag.unwrap_or_else(|| self.player.ctl.pos());
        let dur = t.duration.max(1.0);
        // a thin line for where the song is: drag to move
        let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click_and_drag());
        let p = ui.painter();
        let line = egui::Rect::from_x_y_ranges(r.min.x..=r.max.x, r.center().y - 2.0..=r.center().y + 2.0);
        p.rect_filled(line, 2.0, pal::GROOVE);
        let x = r.min.x + r.width() * (pos / dur).clamp(0.0, 1.0);
        p.rect_filled(egui::Rect::from_x_y_ranges(r.min.x..=x, line.min.y..=line.max.y), 2.0, pal::ACCENT_HI);
        p.circle_filled(egui::pos2(x, r.center().y), 7.0, pal::INK);
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
            ui.label(RichText::new(fmt_t(pos)).color(pal::DIM));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new(fmt_t(dur)).color(pal::DIM)));
        });
        self.transport(ui, pos);
        ui.add_space(8.0);
        if ui.add(chip(self.show_queue, "QUEUE")).clicked() {
            self.show_queue = !self.show_queue;
        }
        if self.show_queue {
            self.queue_list(ui);
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }

    /// The bars: what is sounding, in 24 bands from the bass up (each falls back slowly).
    fn bars_ui(&mut self, ui: &mut egui::Ui) {
        let (samples, rate) = {
            let v = self.player.viz.lock().unwrap();
            let n = v.samples.len();
            (v.samples[n.saturating_sub(2048)..].to_vec(), v.rate.max(1) as f32)
        };
        let playing = !self.paused && !self.loading && samples.len() >= 512;
        for (k, b) in self.bars.iter_mut().enumerate() {
            let mut level = 0.0;
            if playing {
                // this band's loudness (Goertzel), on a log scale from 50 Hz to 12 kHz
                let f = 50.0 * (12000.0f32 / 50.0).powf(k as f32 / 23.0);
                let w = std::f32::consts::TAU * f / rate;
                let c = 2.0 * w.cos();
                let (mut s1, mut s2) = (0.0f32, 0.0f32);
                for x in &samples {
                    let s0 = x + c * s1 - s2;
                    s2 = s1;
                    s1 = s0;
                }
                let mag = (s1 * s1 + s2 * s2 - c * s1 * s2).max(0.0).sqrt() / samples.len() as f32;
                level = ((20.0 * (mag + 1e-6).log10() + 60.0) / 50.0).clamp(0.0, 1.0);
            }
            *b = if level > *b { level } else { *b * 0.88 };
        }
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 64.0), Sense::hover());
        let p = ui.painter();
        let n = self.bars.len() as f32;
        let gap = 3.0;
        let bw = (r.width() - gap * (n - 1.0)) / n;
        for (k, b) in self.bars.iter().enumerate() {
            let h = (b * r.height()).max(3.0);
            let x = r.min.x + k as f32 * (bw + gap);
            let bar = egui::Rect::from_min_max(egui::pos2(x, r.max.y - h), egui::pos2(x + bw, r.max.y));
            // brighter as it rises
            let col = lerp(pal::ACCENT, pal::ACCENT_HI, *b);
            p.rect_filled(bar, 2.0, col);
        }
    }

    /// Previous, play / pause, next.
    fn transport(&mut self, ui: &mut egui::Ui, pos: f32) {
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
                if self.at_end {
                    self.at_end = false;
                    if let Some(i) = self.cur {
                        self.play(i);
                    }
                } else {
                    self.paused = !self.paused;
                    self.player.send(if self.paused { Cmd::Pause } else { Cmd::Resume });
                }
            }
            if big(&mut c[2], "⏭").clicked() {
                if let Some(i) = self.cur.filter(|i| i + 1 < self.queue.len()) {
                    self.play(i + 1);
                }
            }
        });
    }

    /// The queue, in the player's own page (it scrolls with it).
    fn queue_list(&mut self, ui: &mut egui::Ui) {
        let mut play = None;
        for (i, t) in self.queue.iter().enumerate() {
            if track_row(ui, t, Some(i) == self.cur).clicked() {
                play = Some(i);
            }
        }
        if let Some(i) = play {
            self.play(i);
        }
    }

    /// MORE: pitch, the count-off, bookmarks and the equalizer.
    fn more_ui(&mut self, ui: &mut egui::Ui, t: &Track) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.add_sized(Vec2::new(56.0, 44.0), egui::Label::new("PITCH"));
            for (s, d) in [("-1", -100), ("-10c", -10), ("+10c", 10), ("+1", 100)] {
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
            if ui.add_sized(Vec2::new(100.0, 44.0), chip(self.count_in > 0, label)).clicked() {
                self.count_in = match self.count_in {
                    0 => 2,
                    2 => 4,
                    _ => 0,
                };
            }
            ui.label(format!("{:.0} BPM", self.bpm));
            if ui.button("TAP").clicked() {
                self.tap();
            }
        });
        self.player.ctl.set_count_in(self.count_in, self.bpm);
        ui.separator();
        self.metro_ui(ui);
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
                if ui.button("X").clicked() {
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
            if ui.add(chip(self.eq_on, if self.eq_on { "ON" } else { "OFF" })).clicked() {
                self.eq_on = !self.eq_on;
            }
            if ui.button("FLAT").clicked() {
                self.eq = [0.0; 10];
            }
        });
        // ten slim bars: drag one up or down (+-12 dB), the middle line is flat
        let bands = ["31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];
        let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 170.0), Sense::click_and_drag());
        let p = ui.painter();
        let area = egui::Rect::from_min_max(r.min, egui::pos2(r.max.x, r.max.y - 20.0));
        let col_w = area.width() / 10.0;
        let mid = area.center().y;
        p.line_segment([egui::pos2(area.min.x, mid), egui::pos2(area.max.x, mid)], egui::Stroke::new(1.0, pal::GROOVE));
        if let Some(pp) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let k = (((pp.x - area.min.x) / col_w).floor() as usize).min(9);
            self.eq[k] = (((mid - pp.y) / (area.height() / 2.0)) * 12.0).clamp(-12.0, 12.0).round();
            self.eq_on = true;
        }
        for (k, b) in bands.iter().enumerate() {
            let cx = area.min.x + col_w * (k as f32 + 0.5);
            let track = egui::Rect::from_center_size(egui::pos2(cx, mid), Vec2::new(6.0, area.height()));
            p.rect_filled(track, 3.0, pal::LCD);
            let y = mid - self.eq[k] / 12.0 * (area.height() / 2.0);
            let fill = egui::Rect::from_x_y_ranges(cx - 3.0..=cx + 3.0, y.min(mid)..=y.max(mid));
            let c = if self.eq_on { pal::ACCENT } else { pal::GROOVE };
            p.rect_filled(fill, 3.0, c);
            p.circle_filled(egui::pos2(cx, y), 9.0, if self.eq_on { pal::ACCENT_HI } else { pal::DIM });
            p.text(egui::pos2(cx, r.max.y - 8.0), egui::Align2::CENTER_CENTER, *b, egui::FontId::proportional(11.0), pal::DIM);
        }
        self.player.eq.set(self.eq, 0.0, self.eq_on);
    }

    /// CHORDS: find a chord's shapes, name what you finger, triads up the neck.
    fn chords_ui(&mut self, ui: &mut egui::Ui) {
        ui.columns(3, |c| {
            for (k, name) in ["FIND", "ANALYZE", "TRIADS"].into_iter().enumerate() {
                if c[k].add_sized(Vec2::new(c[k].available_width(), 40.0), chip(self.ch_tab == k as u8, name)).clicked() {
                    self.ch_tab = k as u8;
                }
            }
        });
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.ch_tab {
            0 => self.find_ui(ui),
            1 => self.analyze_ui(ui),
            _ => self.triads_ui(ui),
        });
    }

    fn find_ui(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::TextEdit::singleline(&mut self.ch_query).hint_text("A chord: Dm7, G7#9, Fmaj7/A...").desired_width(ui.available_width()));
        let Some(c) = theory::chord_notes(self.ch_query.trim()) else {
            ui.label(RichText::new("Type a chord name.").weak());
            return;
        };
        let notes: Vec<String> = c.named.iter().map(|(n, i)| format!("{} ({})", n, i)).collect();
        ui.label(RichText::new(notes.join("   ")).strong());
        let shapes = theory::guitar_shapes(&c, &theory::STANDARD);
        if shapes.is_empty() {
            ui.label(RichText::new("No shape found for this one.").weak());
        }
        let root = c.root;
        self.grid(ui, &shapes.iter().map(|g| (String::new(), *g)).collect::<Vec<_>>(), root);
    }

    fn analyze_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Tap the frets you're holding (left of the nut: open). Tap again to lift.").weak());
        if let Some((s, f)) = neck(ui, &self.an_frets) {
            self.an_frets[s] = if self.an_frets[s] == f { None } else { f };
            if let Some(f) = self.an_frets[s] {
                self.player.send(Cmd::Once(band::chord_sound(&[theory::STANDARD[s] + f as i32]), 0.9));
            }
        }
        let notes: Vec<i32> = (0..6).filter_map(|s| self.an_frets[s].map(|f| theory::STANDARD[s] + f as i32)).collect();
        ui.horizontal(|ui| {
            if ui.button("STRUM").clicked() && !notes.is_empty() {
                self.player.send(Cmd::Once(band::chord_sound(&notes), 0.9));
            }
            if ui.button("CLEAR").clicked() {
                self.an_frets = [None; 6];
            }
        });
        if notes.is_empty() {
            return;
        }
        let pcs: Vec<i32> = notes.iter().map(|m| m.rem_euclid(12)).collect();
        let bass = notes.iter().min().map(|m| m.rem_euclid(12));
        let sp = theory::Spelling { slash: true, ..Default::default() };
        for (k, s) in theory::suggestions(&pcs, bass).iter().take(6).enumerate() {
            ui.label(if k == 0 { RichText::new(s.name(sp)).size(26.0).strong() } else { RichText::new(s.name(sp)) });
        }
    }

    fn triads_ui(&mut self, ui: &mut egui::Ui) {
        const KEYS: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];
        const QUAL: [(&str, [i32; 3]); 6] =
            [("MAJ", [0, 4, 7]), ("MIN", [0, 3, 7]), ("DIM", [0, 3, 6]), ("AUG", [0, 4, 8]), ("SUS2", [0, 2, 7]), ("SUS4", [0, 5, 7])];
        const SETS: [(&str, [usize; 3]); 7] = [
            ("1 2 3", [3, 4, 5]),
            ("2 3 4", [2, 3, 4]),
            ("3 4 5", [1, 2, 3]),
            ("4 5 6", [0, 1, 2]),
            ("1 2 4", [2, 4, 5]),
            ("2 3 5", [1, 3, 4]),
            ("3 4 6", [0, 2, 3]),
        ];
        ui.horizontal_wrapped(|ui| {
            for (k, n) in KEYS.iter().enumerate() {
                if ui.add_sized(Vec2::new(44.0, 40.0), chip(self.tr_root == k as i32, *n)).clicked() {
                    self.tr_root = k as i32;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (k, (n, _)) in QUAL.iter().enumerate() {
                if ui.add(chip(self.tr_q == k, *n)).clicked() {
                    self.tr_q = k;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("STRINGS");
            for (k, (n, _)) in SETS.iter().enumerate() {
                if ui.add(chip(self.tr_set == k, *n)).clicked() {
                    self.tr_set = k;
                }
            }
        });
        let q = QUAL[self.tr_q].1;
        let pcs: Vec<i32> = q.iter().map(|x| (self.tr_root + x).rem_euclid(12)).collect();
        let grips = theory::grips_on(&pcs, &SETS[self.tr_set].1, false, 4);
        let root = self.tr_root;
        let named: Vec<(String, [Option<u8>; 6])> = grips
            .iter()
            .map(|g| {
                let low = (0..6).find_map(|s| g[s].map(|f| (theory::STANDARD[s] + f as i32 - root).rem_euclid(12))).unwrap_or(0);
                let inv = if low == 0 { "ROOT" } else if low == q[1] { "3RD BELOW" } else { "5TH BELOW" };
                (inv.to_string(), *g)
            })
            .collect();
        self.grid(ui, &named, root);
    }

    /// Chord boxes, two to a row; tap one to hear it.
    fn grid(&mut self, ui: &mut egui::Ui, grips: &[(String, [Option<u8>; 6])], root: i32) {
        let w = (ui.available_width() - 12.0) / 2.0;
        let mut hear = None;
        for row in grips.chunks(2) {
            ui.horizontal(|ui| {
                for (label, g) in row {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 200.0), Sense::click());
                    chord_box(ui.painter(), r, label, g, root);
                    if resp.clicked() {
                        hear = Some(*g);
                    }
                }
            });
        }
        if let Some(g) = hear {
            let notes: Vec<i32> = (0..6).filter_map(|s| g[s].map(|f| theory::STANDARD[s] + f as i32)).collect();
            self.player.send(Cmd::Once(band::chord_sound(&notes), 0.9));
        }
    }

    /// Tap the tempo in: the beats tapped in the last few seconds set it.
    fn tap(&mut self) {
        let now = std::time::Instant::now();
        self.taps.retain(|x| now.duration_since(*x).as_secs_f32() < 3.0);
        self.taps.push(now);
        if self.taps.len() >= 2 {
            let span = now.duration_since(self.taps[0]).as_secs_f32();
            self.bpm = (60.0 * (self.taps.len() - 1) as f32 / span).clamp(30.0, 300.0).round();
        }
    }

    /// The metronome sounds as set (or stops), from the top of the bar.
    fn metro_apply(&mut self) {
        if self.metro.on {
            let body = band::click_loop(&self.metro, self.bpm);
            self.player.send(Cmd::Pcm(0, Vec::new(), body, 0.8));
            self.metro_since = Some(std::time::Instant::now());
        } else {
            self.player.send(Cmd::Pcm(0, Vec::new(), Vec::new(), 0.0));
            self.metro_since = None;
        }
    }

    /// METRONOME: on / off, the tempo (with TAP), beats in a bar, the subdivision, and a light for each beat.
    fn metro_ui(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new("METRONOME").strong());
            if ui.add_sized(Vec2::new(70.0, 44.0), chip(self.metro.on, if self.metro.on { "ON" } else { "OFF" })).clicked() {
                self.metro.on = !self.metro.on;
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            for (s, d) in [("-5", -5.0), ("-1", -1.0)] {
                if ui.button(s).clicked() {
                    self.bpm = (self.bpm + d).clamp(30.0, 300.0);
                    changed = true;
                }
            }
            ui.add_sized(Vec2::new(70.0, 44.0), egui::Label::new(RichText::new(format!("{:.0}", self.bpm)).size(24.0).strong()));
            for (s, d) in [("+1", 1.0), ("+5", 5.0)] {
                if ui.button(s).clicked() {
                    self.bpm = (self.bpm + d).clamp(30.0, 300.0);
                    changed = true;
                }
            }
            if ui.button("TAP").clicked() {
                self.tap();
                changed = true;
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("BEATS");
            for n in 2..=7u32 {
                if ui.add_sized(Vec2::new(36.0, 40.0), chip(self.metro.beats == n, n.to_string())).clicked() {
                    self.metro.beats = n;
                    changed = true;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (k, name) in ["♩", "8THS", "TRIPS", "16THS"].into_iter().enumerate() {
                if ui.add(chip(self.metro.sub == k as u8, name)).clicked() {
                    self.metro.sub = k as u8;
                    changed = true;
                }
            }
        });
        // a light for each beat of the bar, the one sounding lit
        let n = self.metro.beats.clamp(1, 32) as usize;
        let lit = self.metro_since.map(|s| (s.elapsed().as_secs_f32() * self.bpm / 60.0) as usize % n);
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
        let step = (r.width() / n as f32).min(60.0);
        for k in 0..n {
            let c = egui::pos2(r.min.x + step * (k as f32 + 0.5), r.center().y);
            let on = lit == Some(k);
            let col = if on && k == 0 { Color32::from_rgb(230, 80, 80) } else if on { Color32::WHITE } else { Color32::from_gray(70) };
            ui.painter().circle_filled(c, 12.0, col);
        }
        if self.metro.on {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
        }
        if changed {
            self.metro_apply();
        }
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
        p.rect_filled(r, 3.0, pal::SEL);
        p.rect_filled(egui::Rect::from_min_size(r.min, Vec2::new(3.0, r.height())), 0.0, pal::ACCENT);
    } else if resp.is_pointer_button_down_on() {
        p.rect_filled(r, 3.0, pal::PANEL);
    }
    p.line_segment([r.left_bottom(), r.right_bottom()], egui::Stroke::new(1.0, pal::PANEL));
    let f = egui::FontId::proportional(17.0);
    let s = egui::FontId::proportional(13.0);
    let title = clip(ui, &t.title, &f, r.width() - 20.0);
    p.text(r.min + Vec2::new(12.0, 9.0), egui::Align2::LEFT_TOP, title, f, pal::INK);
    let sub = if t.duration > 0.0 { format!("{}  ·  {}", t.artist.to_uppercase(), fmt_t(t.duration)) } else { t.artist.to_uppercase() };
    p.text(r.min + Vec2::new(12.0, 33.0), egui::Align2::LEFT_TOP, sub, s, pal::DIM);
    resp
}

/// A chord box: strings up and down, five frets from the lowest one fretted, the root's dots in red.
fn chord_box(p: &egui::Painter, r: egui::Rect, label: &str, g: &[Option<u8>; 6], root: i32) {
    let ink = Color32::from_gray(200);
    p.text(egui::pos2(r.center().x, r.min.y + 4.0), egui::Align2::CENTER_TOP, label, egui::FontId::proportional(13.0), Color32::from_gray(160));
    let fretted: Vec<u8> = g.iter().flatten().copied().filter(|f| *f > 0).collect();
    let lo = fretted.iter().min().copied().unwrap_or(1);
    let base = if fretted.iter().max().copied().unwrap_or(0) <= 5 { 1 } else { lo };
    let bx = egui::Rect::from_min_max(egui::pos2(r.min.x + 30.0, r.min.y + 46.0), egui::pos2(r.max.x - 14.0, r.max.y - 10.0));
    let sw = bx.width() / 5.0;
    let fh = bx.height() / 5.0;
    for s in 0..6 {
        let x = bx.min.x + s as f32 * sw;
        p.line_segment([egui::pos2(x, bx.min.y), egui::pos2(x, bx.max.y)], egui::Stroke::new(1.0, ink));
    }
    for f in 0..=5 {
        let y = bx.min.y + f as f32 * fh;
        let wdt = if f == 0 && base == 1 { 4.0 } else { 1.0 };
        p.line_segment([egui::pos2(bx.min.x, y), egui::pos2(bx.max.x, y)], egui::Stroke::new(wdt, ink));
    }
    if base > 1 {
        p.text(egui::pos2(bx.min.x - 6.0, bx.min.y + fh / 2.0), egui::Align2::RIGHT_CENTER, base.to_string(), egui::FontId::proportional(14.0), ink);
    }
    for s in 0..6 {
        let x = bx.min.x + s as f32 * sw;
        match g[s] {
            None => {
                p.text(egui::pos2(x, bx.min.y - 12.0), egui::Align2::CENTER_CENTER, "x", egui::FontId::proportional(15.0), Color32::from_gray(140));
            }
            Some(0) => {
                p.circle_stroke(egui::pos2(x, bx.min.y - 12.0), 6.0, egui::Stroke::new(1.5, ink));
            }
            Some(f) => {
                let y = bx.min.y + (f as f32 - base as f32 + 0.5) * fh;
                let is_root = (theory::STANDARD[s] + f as i32 - root).rem_euclid(12) == 0;
                p.circle_filled(egui::pos2(x, y), (sw * 0.32).min(11.0), if is_root { Color32::from_rgb(230, 80, 80) } else { Color32::from_gray(235) });
            }
        }
    }
}

/// A neck to finger a chord on: six strings (high E on top), the open strings left of the nut and 12 frets.
/// Returns the string and fret tapped (fret None is not used; Some(0) is open).
fn neck(ui: &mut egui::Ui, frets: &[Option<u8>; 6]) -> Option<(usize, Option<u8>)> {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 230.0), Sense::click());
    let p = ui.painter();
    let cols = 13.0;
    let cw = r.width() / cols;
    let sh = r.height() / 6.0;
    let ink = Color32::from_gray(150);
    for s in 0..6 {
        let y = r.min.y + (5 - s) as f32 * sh + sh / 2.0;
        p.line_segment([egui::pos2(r.min.x + cw, y), egui::pos2(r.max.x, y)], egui::Stroke::new(1.0, ink));
    }
    for f in 1..13 {
        let x = r.min.x + f as f32 * cw;
        p.line_segment([egui::pos2(x, r.min.y), egui::pos2(x, r.max.y)], egui::Stroke::new(if f == 1 { 4.0 } else { 1.0 }, ink));
    }
    for (f, dots) in [(3, 1), (5, 1), (7, 1), (9, 1), (12, 2)] {
        let x = r.min.x + (f as f32 + 0.5) * cw;
        if dots == 1 {
            p.circle_filled(egui::pos2(x, r.center().y), 4.0, Color32::from_gray(70));
        }
    }
    for s in 0..6 {
        if let Some(f) = frets[s] {
            let x = r.min.x + (f as f32 + 0.5) * cw;
            let y = r.min.y + (5 - s) as f32 * sh + sh / 2.0;
            p.circle_filled(egui::pos2(x, y), (sh * 0.35).min(14.0), Color32::from_rgb(90, 160, 240));
        }
    }
    let pos = resp.interact_pointer_pos().filter(|_| resp.clicked())?;
    let f = ((pos.x - r.min.x) / cw).floor().clamp(0.0, 12.0) as u8;
    let s = 5 - ((pos.y - r.min.y) / sh).floor().clamp(0.0, 5.0) as usize;
    Some((s, Some(f)))
}

/// Backline's SLEEK DARK skin: charcoal with one blue accent, in Chakra Petch (the desktop's own smooth font,
/// carried in the app so it looks the same on any phone).
mod pal {
    use eframe::egui::Color32;
    pub const BG: Color32 = Color32::from_rgb(12, 13, 17);
    pub const PANEL: Color32 = Color32::from_rgb(28, 30, 37);
    pub const PANEL_HI: Color32 = Color32::from_rgb(42, 45, 55);
    pub const LCD: Color32 = Color32::from_rgb(18, 19, 25);
    pub const GROOVE: Color32 = Color32::from_rgb(44, 47, 58);
    pub const INK: Color32 = Color32::from_rgb(236, 238, 245);
    pub const INK2: Color32 = Color32::from_rgb(168, 173, 187);
    pub const DIM: Color32 = Color32::from_rgb(142, 146, 158);
    pub const ACCENT: Color32 = Color32::from_rgb(70, 122, 255);
    pub const ACCENT_HI: Color32 = Color32::from_rgb(120, 160, 255);
    pub const SEL: Color32 = Color32::from_rgb(40, 52, 84);
    pub const RED: Color32 = Color32::from_rgb(255, 92, 102);
    pub const TRIM: Color32 = Color32::from_rgb(140, 148, 168);
}

fn theme(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("chakra".to_string(), egui::FontData::from_static(include_bytes!("../../backline/assets/ChakraPetch-Regular.ttf")));
    // Chakra Petch first; the built-in fonts after it for the symbols it lacks (play, pause...)
    for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts.families.entry(fam).or_default().insert(0, "chakra".to_string());
    }
    ctx.set_fonts(fonts);
    let mut v = egui::Visuals::dark();
    v.panel_fill = pal::BG;
    v.window_fill = pal::PANEL;
    v.extreme_bg_color = pal::LCD;
    v.faint_bg_color = pal::PANEL;
    v.hyperlink_color = pal::ACCENT_HI;
    v.selection.bg_fill = pal::SEL;
    v.selection.stroke = egui::Stroke::new(1.0, Color32::WHITE);
    let r = egui::Rounding::same(3.0);
    for (w, fill, line) in [
        (&mut v.widgets.noninteractive, pal::BG, pal::GROOVE),
        (&mut v.widgets.inactive, pal::PANEL, pal::GROOVE),
        (&mut v.widgets.hovered, pal::PANEL_HI, pal::TRIM),
        (&mut v.widgets.active, pal::ACCENT, pal::ACCENT_HI),
        (&mut v.widgets.open, pal::PANEL_HI, pal::TRIM),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = egui::Stroke::new(1.0, line);
        w.rounding = r;
    }
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, pal::INK2);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, pal::INK);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, Color32::WHITE);
    ctx.set_visuals(v);
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// A toggle: a real button, lit in the accent when on.
fn chip(on: bool, text: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(text).selected(on)
}

/// Text cut with "..." to fit `w`.
fn clip(ui: &egui::Ui, s: &str, f: &egui::FontId, w: f32) -> String {
    let width = |t: &str| ui.fonts(|fo| fo.layout_no_wrap(t.to_string(), f.clone(), Color32::WHITE).size().x);
    if width(s) <= w {
        return s.to_string();
    }
    let mut c: Vec<char> = s.chars().collect();
    while !c.is_empty() && width(&format!("{}...", c.iter().collect::<String>())) > w {
        c.pop();
    }
    format!("{}...", c.iter().collect::<String>().trim_end())
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

/// A YouTube clip's track number (its own range, apart from Tidal's and the files').
fn yt_id(vid: &str) -> i64 {
    -(1 + (hash(&format!("yt:{}", vid)) & 0x7fff_ffff_ffff) as i64) - 0x1_0000_0000_0000
}

/// The video in a link: watch?v=, youtu.be/, shorts/, or the bare 11-letter id.
fn video_id(link: &str) -> Option<String> {
    let l = link.trim();
    let take = |s: &str| -> Option<String> {
        let id: String = s.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
        (id.len() == 11).then_some(id)
    };
    for key in ["v=", "youtu.be/", "shorts/", "embed/", "live/"] {
        if let Some(i) = l.find(key) {
            return take(&l[i + key.len()..]);
        }
    }
    take(l)
}

/// The playlist in a link (list=...), when it is only a playlist (a video in a playlist is the video).
fn list_id(link: &str) -> Option<String> {
    if link.contains("v=") || link.contains("youtu.be/") {
        return None;
    }
    let i = link.find("list=")?;
    let id: String = link[i + 5..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    (!id.is_empty()).then_some(id)
}

fn yt_client() -> Result<rustypipe::client::RustyPipe, String> {
    let dir = api::config_dir().join("yt_client");
    let _ = std::fs::create_dir_all(&dir);
    rustypipe::client::RustyPipe::builder().storage_dir(dir).build().map_err(|e| e.to_string())
}

/// A clip's audio (AAC in an m4a, which the player reads), fetched once and kept.
fn yt_audio(vid: &str) -> Result<Vec<u8>, String> {
    let dir = api::config_dir().join("yt");
    let file = dir.join(format!("{}.m4a", vid));
    if let Ok(b) = std::fs::read(&file) {
        return Ok(b);
    }
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string())?;
    let stream = rt.block_on(async {
        let rp = yt_client()?;
        let p = rp.query().player(vid).await.map_err(|e| e.to_string())?;
        p.audio_streams
            .iter()
            .filter(|s| s.format == rustypipe::model::AudioFormat::M4a)
            .max_by_key(|s| s.bitrate)
            .map(|s| (s.url.clone(), s.size))
            .ok_or_else(|| "no audio this phone can play".to_string())
    })?;
    // in pieces: YouTube slows down one long download
    let http = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(60)).build().map_err(|e| e.to_string())?;
    let (url, size) = stream;
    let mut out: Vec<u8> = Vec::with_capacity(size as usize);
    let piece = 4 * 1024 * 1024u64;
    let mut at = 0u64;
    while at < size.max(1) {
        let end = (at + piece - 1).min(size.saturating_sub(1));
        let b = http
            .get(&url)
            .header("Range", format!("bytes={}-{}", at, end))
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.bytes())
            .map_err(|e| e.to_string())?;
        if b.is_empty() {
            break;
        }
        at += b.len() as u64;
        out.extend_from_slice(&b);
        if size == 0 {
            break;
        }
    }
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(&file, &out);
    Ok(out)
}

/// What is on the phone's clipboard (PASTE), through Android's ClipboardManager.
fn clipboard() -> Option<String> {
    #[cfg(target_os = "android")]
    {
        use jni::objects::{JObject, JString};
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
        let name = env.new_string("clipboard").ok()?;
        let cm = env.call_method(&activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&name).into()]).ok()?.l().ok()?;
        let clip = env.call_method(&cm, "getPrimaryClip", "()Landroid/content/ClipData;", &[]).ok()?.l().ok()?;
        if clip.is_null() {
            return None;
        }
        let item = env.call_method(&clip, "getItemAt", "(I)Landroid/content/ClipData$Item;", &[0i32.into()]).ok()?.l().ok()?;
        let text = env.call_method(&item, "getText", "()Ljava/lang/CharSequence;", &[]).ok()?.l().ok()?;
        if text.is_null() {
            return None;
        }
        let s = env.call_method(&text, "toString", "()Ljava/lang/String;", &[]).ok()?.l().ok()?;
        let s: String = env.get_string(&JString::from(s)).ok()?.into();
        return Some(s);
    }
    #[allow(unreachable_code)]
    None
}

fn load_json<T: serde::de::DeserializeOwned + Default>(name: &str) -> T {
    std::fs::read_to_string(api::config_dir().join(name)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save_json<T: serde::Serialize>(name: &str, v: &T) {
    let _ = std::fs::create_dir_all(api::config_dir());
    if let Ok(s) = serde_json::to_string(v) {
        let _ = std::fs::write(api::config_dir().join(name), s);
    }
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
        egui::TopBottomPanel::top("brand").exact_height(30.0).show_separator_line(false).show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(RichText::new("◆ BACKLINE").size(15.0).color(pal::TRIM));
            });
        });
        egui::TopBottomPanel::bottom("nav").show(ctx, |ui| {
            if self.tab != Tab::Player {
                self.now_bar(ui);
                ui.separator();
            }
            ui.columns(3, |c| {
                for (k, (tab, name)) in [(Tab::Player, "PLAYER"), (Tab::Library, "LIBRARY"), (Tab::Chords, "CHORDS")].into_iter().enumerate() {
                    if c[k].add_sized(Vec2::new(c[k].available_width(), 48.0), chip(self.tab == tab, name)).clicked() {
                        self.tab = tab;
                    }
                }
            });
            ui.add_space(22.0);
        });
        // a note stays a few seconds (trouble a little longer)
        if self.note_at.as_ref().map(|n| &n.0) != Some(&self.note) {
            self.note_at = Some((self.note.clone(), std::time::Instant::now()));
        }
        if let Some((_, at)) = &self.note_at {
            let keep = if self.note.starts_with("Added") || self.note.starts_with("Taken") { 3.0 } else { 8.0 };
            if !self.note.is_empty() && at.elapsed().as_secs_f32() > keep {
                self.note.clear();
            } else if !self.note.is_empty() {
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.note.is_empty() {
                // news in grey, trouble in red; tap it away
                let good = self.note.starts_with("Added") || self.note.starts_with("Taken");
                let col = if good { Color32::from_gray(190) } else { Color32::LIGHT_RED };
                if ui.add(egui::Label::new(RichText::new(self.note.as_str()).color(col)).sense(Sense::click())).clicked() {
                    self.note.clear();
                }
            }
            match self.tab {
                // the player is one page that scrolls (MORE and the queue open below the tools)
                Tab::Player => {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.player_ui(ui));
                }
                Tab::Library => self.library_ui(ui),
                Tab::Chords => self.chords_ui(ui),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links() {
        assert_eq!(video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=1").as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(video_id("https://youtu.be/dQw4w9WgXcQ?si=x").as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(list_id("https://youtube.com/playlist?list=PLabc_-1").as_deref(), Some("PLabc_-1"));
        assert_eq!(list_id("https://youtube.com/watch?v=dQw4w9WgXcQ&list=PLabc"), None);
    }

    /// Online: fetches a clip's audio (nothing plays). cargo test -- --ignored
    #[test]
    #[ignore]
    fn yt_fetch() {
        let dir = std::env::temp_dir().join("backline_yt_test");
        std::env::set_var("BACKLINE_HOME", &dir);
        let b = yt_audio("jNQXAC9IVRw").expect("audio");
        let (d, w) = wave(&b);
        assert!(d > 10.0 && !w.is_empty(), "{} s", d);
    }
}
