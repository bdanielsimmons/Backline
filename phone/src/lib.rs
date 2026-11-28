//! Backline on an Android phone, kept small: a YouTube song into the practice tools. Find a song (search YouTube or
//! paste a link), and it plays with the desktop's practice panel: the waveform with the loop, slow it down, change
//! its pitch, the trainer, bookmarks.
//!
//! It wears Studio, and everything it draws comes from the desktop's own code (backline/src: the Studio kit, the
//! keys, tabs and buttons, the fonts, the player), so it looks like the desktop: its three windows, NOW PLAYING on
//! top and SONGS or PRACTICE under it, stacked for a phone.
#![allow(dead_code, unused_imports, unused_variables, clippy::all)]

// ---- the desktop's engine
#[path = "../../backline/src/audio/beats.rs"]
mod beats;
#[path = "../../backline/src/audio/decode.rs"]
mod decode;
#[path = "../../backline/src/audio/metro.rs"]
mod metro;
#[path = "../../backline/src/audio/player.rs"]
mod player;
#[path = "../../backline/src/audio/record.rs"]
mod record;
// ---- the desktop's look
#[path = "../../backline/src/ui/font.rs"]
mod font;
#[path = "../../backline/src/ui/icons.rs"]
mod icons;
#[path = "../../backline/src/ui/skin.rs"]
mod skin;
#[path = "../../backline/src/ui/studio.rs"]
mod studio;
#[path = "../../backline/src/ui/vicon.rs"]
mod vicon;
#[path = "../../backline/src/ui/viz.rs"]
mod viz;
#[path = "../../backline/src/ui/widgets.rs"]
mod widgets;
#[path = "../../backline/src/data/genskin.rs"]
mod genskin;
#[path = "../../backline/src/data/winamp.rs"]
mod winamp;
#[path = "../../backline/src/ui/winamp_draw.rs"]
mod winamp_ui;
// ---- the phone's own
mod bridge;
mod deck;
mod device;
#[cfg(target_os = "ios")]
mod ios;
mod loops;
mod metronome;
mod mine;
mod practice;
mod settings;
mod songs;
mod takes;
mod yt;
mod ytdlp;

use eframe::egui::{self, Align, Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use font::{fit, ptext, ptext_fit, snap, spx, text_w, thick, wrap};
use icons::*;
use player::{Cmd, Player};
use skin::*;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};
use widgets::*;
use takes::Take;
use yt::*;

// ---- what the desktop's drawing kit asks of the app around it
const HEAD_H: f32 = 18.0;
const ROW_H: f32 = 24.0;
pub(crate) fn practice_tools() -> bool {
    true
}
pub(crate) fn version() -> String {
    "v9".to_string()
}
pub(crate) fn app_bar_strip(_ui: &egui::Ui, _strip: Rect, _id: egui::Id) {}
pub(crate) fn title_h() -> f32 {
    20.0
}
pub(crate) const PAGE_PAD: f32 = 12.0;
mod tour {
    pub fn note(_name: &str, _r: eframe::egui::Rect) {}
    pub fn mark(_name: &str, _r: eframe::egui::Rect) {}
}
mod update {
    pub fn build() -> u64 {
        option_env!("BACKLINE_BUILD").and_then(|b| b.parse().ok()).unwrap_or(0)
    }
}
mod test_hooks {
    pub fn clip_ok(_r: eframe::egui::Rect) {}
    pub fn note_text(_p: &eframe::egui::Painter, _r: eframe::egui::Rect, _t: &str) {}
}
/// `clamp` that can't crash (the desktop's): the high end wins when the ends cross.
pub(crate) trait Within: Sized {
    fn within(self, lo: Self, hi: Self) -> Self;
}
impl Within for f32 {
    fn within(self, lo: f32, hi: f32) -> f32 {
        self.max(lo).min(hi)
    }
}
mod styles {
    pub fn amber() -> eframe::egui::Color32 {
        crate::skin::hw_screen().0
    }
}
/// The settings and the log, in the app's own storage (`BACKLINE_HOME`, set at start).
mod api {
    use std::path::PathBuf;
    pub fn config_dir() -> PathBuf {
        if let Some(home) = std::env::var_os("BACKLINE_HOME") {
            return PathBuf::from(home);
        }
        dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("backline-phone")
    }
    /// A line in log.txt: on a phone in the app's outside folder (readable over USB), else beside the settings.
    pub fn log(msg: &str) {
        use std::io::Write;
        let dir = std::env::var_os("BACKLINE_FILES").map(PathBuf::from).unwrap_or_else(config_dir);
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("log.txt")) {
            let _ = writeln!(f, "{}", msg);
        }
    }
}

/// How big the desktop's drawing is on the phone (the desktop's practice panel, as narrow as a phone, is 0.85).
pub(crate) const UI_SCALE: f32 = 0.92;

/// The ear modes (the player's own): which part of the stereo picture you hear.
pub(crate) const CHAN_NAMES: [&str; 6] = ["STEREO", "LEFT", "RIGHT", "MONO", "NO CENTER", "BASS ONLY"];

/// What the background work sends back.
pub(crate) enum Msg {
    /// a search's results (what was searched for)
    Found(String, Result<Vec<Song>, String>),
    /// a link looked up: the song, to add and play
    Added(Result<Song, String>),
    /// the song's audio (which load it was), its length and its waveform (0-255 peaks)
    Audio(u64, Result<(Vec<u8>, f32, Vec<u8>), String>),
    /// a thumbnail (the video, its pixels)
    Thumb(String, Option<egui::ColorImage>),
    /// a song's beat found from its audio (the desktop's beat finder): its tempo for the count-off and the trainer,
    /// every beat for the metronome's lock
    Beats(String, Option<beats::BeatInfo>),
    /// the music on the phone, read
    DeviceTracks(Result<Vec<Song>, String>),
    /// a song saved in the background (SAVING on)
    Saved(String, Result<(), String>),
    /// yt-dlp's version, and what its update said
    YtdlpVersion(Result<String, String>),
    YtdlpUpdated(Result<String, String>),
}

/// What the phone keeps between runs (settings.json).
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Settings {
    pub volume: f32,
    pub view: u8,
    pub mtab: u8,
    pub count_in: u32,
    pub trainer_n: u32,
    pub trainer_step: u32,
    pub last: String,
    /// SAVING: songs kept on the phone (on unless turned off)
    pub keep: bool,
    /// Studio's screen colours (Cream, Gray, Blue, Amber)
    pub screen: u8,
    /// MY TRACKS' list (0 Liked, 1 Saved, 2 Files, 3 Loops)
    pub mine_tab: u8,
    /// the song you had last, whole (a phone track isn't in your songs)
    pub last_song: Song,
    /// the folder open in Files (from the phone's top folder; empty: the top)
    pub folder: String,
    /// the metronome (the desktop's settings), its own tempo and volume
    pub mt: metro::Metro,
    pub mt_bpm: u32,
    pub mt_vol: f32,
    /// SHUF and REP (0 off, 1 all, 2 this song)
    pub shuffle: bool,
    pub repeat: u8,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { volume: 0.8, view: 0, mtab: 0, count_in: 0, trainer_n: 4, trainer_step: 5, last: String::new(), keep: true, screen: 0, mine_tab: 0, last_song: Song::default(), folder: String::new(), mt: metro::Metro::default(), mt_bpm: 100, mt_vol: 0.8, shuffle: false, repeat: 0 }
    }
}

pub(crate) struct App {
    pub ctx: egui::Context,
    pub player: Player,
    pub tx: Sender<Msg>,
    pub rx: Receiver<Msg>,
    /// the skin pictures Studio is worn with (kept: the frames read from them)
    pub wa_art: Option<winamp_ui::WaTex>,
    pub font_tex: egui::TextureHandle,
    /// 0: SONGS, 1: PRACTICE, 2: SETTINGS, 3: MY TRACKS
    pub view: u8,
    // ---- MY TRACKS: the songs you've liked (kept whole, phone tracks too), the music on the phone (read once
    // allowed: 0 not yet, 1 asked, 2 reading, 3 read), which list (Liked, Saved, On This Phone), its filter; the list a
    // song was started from (Previous and Next go through it)
    pub liked: Vec<Song>,
    pub device_tracks: Vec<Song>,
    pub device_state: u8,
    pub device_err: String,
    pub device_asked_at: f64,
    pub mine_tab: u8,
    pub mine_filter: String,
    pub play_list: Vec<Song>,
    /// the folder open in Files, and the song open in Loops (its loops and bookmarks)
    pub folder: String,
    pub loops_pick: Option<String>,
    /// SAVING on: songs kept on the phone (each song as it's played, and your other songs in the background)
    pub keep: bool,
    /// how far the song's download is (0-100), and the background saving: busy, songs that wouldn't come this run,
    /// when it last looked
    pub dl_pct: std::sync::Arc<std::sync::atomic::AtomicU32>,
    pub saver_busy: bool,
    pub saver_failed: HashSet<String>,
    pub saver_at: f64,
    /// songs asked to be saved (SAVE on a song: first in line, Saving on or off), the one saving now and how far
    pub save_queue: Vec<String>,
    pub saving: Option<String>,
    pub save_pct: std::sync::Arc<std::sync::atomic::AtomicU32>,
    /// yt-dlp's version and update (Settings), the saved songs' count and size (read now and then), Clear asked
    pub ytdlp_ver: Option<Result<String, String>>,
    pub ytdlp_note: String,
    pub ytdlp_busy: bool,
    pub ytdlp_updating: bool,
    pub stats: (usize, u64),
    pub stats_at: f64,
    pub ask_clear: bool,
    pub screen: u8,
    /// the screen kept on (while a song plays)
    pub awake: bool,
    // ---- recording yourself (RECORD): what's coming in, since when, the take to keep or not, waiting for Android's
    // yes to the mic; the takes, the one heard (since when), the one picked
    pub rec_buf: Option<std::sync::Arc<std::sync::Mutex<record::InBuf>>>,
    pub rec_t0: Option<Instant>,
    pub rec_ask: Option<usize>,
    pub rec_waiting: bool,
    pub rec_asked_at: f64,
    pub takes: Vec<Take>,
    pub take_playing: Option<(usize, Instant)>,
    pub take_del: Option<usize>,
    /// your songs (newest first) and what you've kept for each (loops, bookmarks, speed, pitch, tempo)
    pub songs: Vec<Song>,
    pub kept: HashMap<String, Kept>,
    // ---- finding songs
    pub query: String,
    pub results: Vec<Song>,
    pub searched: String,
    pub searching: bool,
    pub adding: bool,
    /// a YouTube link on the clipboard, offered (its video) and the last one looked at
    pub clip_offer: Option<String>,
    pub clip_seen: String,
    pub clip_check: u32,
    pub focused: bool,
    /// the song asked about before it's removed
    pub del_ask: Option<String>,
    pub thumbs: HashMap<String, egui::TextureHandle>,
    /// the pictures as PNG (for the phone's media controls), and what they were last told (the song, playing, its
    /// speed, with a cover; where it was, when)
    pub thumb_png: HashMap<String, Vec<u8>>,
    pub sess_sig: (String, bool, u32, bool),
    pub sess_pos: (f32, Instant),
    pub thumb_busy: HashSet<String>,
    // ---- playing
    pub cur: Option<Song>,
    pub gen: u64,
    pub loading: bool,
    pub paused: bool,
    pub stopped: bool,
    pub dur: f32,
    pub wave: Vec<u8>,
    /// the song's file (played again from the top after Stop) and whether to start it once it's in
    pub audio: Option<std::sync::Arc<Vec<u8>>>,
    pub want_play: bool,
    /// why the song's audio didn't come (shown in the waveform, with Try Again)
    pub load_error: Option<String>,
    pub volume: f32,
    pub seek_drag: Option<f32>,
    /// the text box being typed in (it keeps egui's focus)
    pub typing_id: Option<egui::Id>,
    // ---- practice
    pub mtab: u8,
    pub loop_a: Option<f32>,
    pub loop_b: Option<f32>,
    pub loop_on: bool,
    pub speed: u32,
    pub cents: i32,
    pub chan: u32,
    pub count_in: u32,
    pub bpm: f32,
    pub taps: Vec<f64>,
    pub trainer: bool,
    pub trainer_n: u32,
    pub trainer_step: u32,
    /// the trainer's THEN: faster (false), or up a 4th each set through all 12 keys (true), and the keys done
    pub trainer_keys: bool,
    pub keys_done: u32,
    pub passes: u32,
    pub wraps_seen: u32,
    pub sec_name: String,
    pub mark_name: String,
    pub sec_del: Option<usize>,
    pub mark_del: Option<usize>,
    pub wv_zoom: f32,
    pub wv_start: f32,
    /// a drag on the waveform: what it moves (0 a new loop, 1 A, 2 B, 3 the loop), where it began, A and B then
    pub wv_drag: Option<(u8, f32, f32, f32)>,
    /// when two fingers last pinched the waveform (taps and drags just after are the pinch's, not a jump or a loop)
    pub pinch_at: f64,
    // ---- the screen
    pub note: String,
    pub note_at: Option<(String, Instant)>,
    pub keyboard: bool,
    pub zoom_set: bool,
    pub bands: Vec<f32>,
    pub peaks: Vec<f32>,
    // ---- the metronome (metronome.rs): what it clicks, its own tempo (what the speed-up drill has added, since
    // when), its panel open, the accent grouping, its volume; what was last sent (built again only on a change), when
    // its cycle started (the pendulum, the lights); the song's beat (vid, beat length, first beat; every beat, the
    // usual length), looking for it, songs with none; where the song was last frame, the laid-out clicks' end
    pub mt: metro::Metro,
    pub mt_bpm: u32,
    pub mt_add: f32,
    pub mt_step_at: Instant,
    pub mt_open: bool,
    pub mt_group: usize,
    pub mt_vol: f32,
    pub mt_sig: u64,
    pub mt_gen: u32,
    pub mt_vis: Option<(Instant, f32, usize)>,
    pub beat: Option<(String, f32, f32)>,
    pub beat_grid: Option<(String, std::sync::Arc<Vec<f32>>, f32)>,
    pub beat_busy: bool,
    pub beat_failed: HashSet<String>,
    pub mt_pos: (f32, Instant),
    pub mt_lead_end: f32,
    pub mt_sync_at: Instant,
    // ---- SHUF and REP (0 off, 1 all, 2 this song); the shuffled order (and the list it was made from)
    pub shuffle: bool,
    pub repeat: u8,
    pub shuf_order: Vec<String>,
    pub shuf_key: u64,
    /// a loop (A, B) or a spot (B below A) to go to once the song is in (from Loops), and (the PC copy) a picture
    /// asked for
    pub pending_loop: Option<(f32, f32)>,
    pub shot_asked: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> App {
        let ctx = cc.egui_ctx.clone();
        let st: Settings = load_json("settings.json");
        font::set_ppp(ctx.pixels_per_point());
        let font_tex = font::init(&ctx);
        // Studio worn: its screens' colours, its fonts (the typewriter face first for the reading), its skin
        skin::STYLE.store(skin::STYLE_STUDIO, Ordering::Relaxed);
        skin::STUDIO_SCREEN.store(st.screen.min(3), Ordering::Relaxed);
        skin::set_custom(Some(studio::studio_pal()));
        ctx.set_fonts(phone_fonts());
        ctx.style_mut(|s| {
            s.interaction.tooltip_delay = 0.6;
            s.spacing.scroll.bar_width = 6.0;
        });
        let (tx, rx) = channel();
        let player = Player::new();
        player.send(Cmd::Volume(st.volume * st.volume));
        player.send(Cmd::MetroVolume(st.mt_vol * st.mt_vol));
        let songs: Vec<Song> = load_json("songs.json");
        let kept: HashMap<String, Kept> = load_json("practice.json");
        let mut app = App {
            ctx,
            player,
            tx,
            rx,
            wa_art: None,
            font_tex,
            view: st.view.min(3),
            liked: load_json("liked.json"),
            device_tracks: Vec::new(),
            device_state: 0,
            device_err: String::new(),
            device_asked_at: 0.0,
            mine_tab: st.mine_tab.min(3),
            mine_filter: String::new(),
            play_list: Vec::new(),
            folder: st.folder.clone(),
            loops_pick: None,
            keep: st.keep,
            dl_pct: Default::default(),
            saver_busy: false,
            saver_failed: HashSet::new(),
            saver_at: 0.0,
            save_queue: Vec::new(),
            saving: None,
            save_pct: Default::default(),
            ytdlp_ver: None,
            ytdlp_note: String::new(),
            ytdlp_busy: false,
            ytdlp_updating: false,
            stats: (0, 0),
            stats_at: -10.0,
            ask_clear: false,
            screen: st.screen.min(3),
            awake: false,
            rec_buf: None,
            rec_t0: None,
            rec_ask: None,
            rec_waiting: false,
            rec_asked_at: 0.0,
            takes: load_json("takes.json"),
            take_playing: None,
            take_del: None,
            songs,
            kept,
            query: String::new(),
            results: Vec::new(),
            searched: String::new(),
            searching: false,
            adding: false,
            clip_offer: None,
            clip_seen: String::new(),
            clip_check: 2,
            focused: true,
            del_ask: None,
            thumbs: HashMap::new(),
            thumb_png: HashMap::new(),
            sess_sig: Default::default(),
            sess_pos: (0.0, Instant::now()),
            thumb_busy: HashSet::new(),
            cur: None,
            gen: 0,
            loading: false,
            paused: false,
            stopped: true,
            dur: 0.0,
            wave: Vec::new(),
            audio: None,
            want_play: false,
            load_error: None,
            volume: st.volume,
            seek_drag: None,
            typing_id: None,
            mtab: st.mtab.min(4),
            loop_a: None,
            loop_b: None,
            loop_on: false,
            speed: 100,
            cents: 0,
            chan: 0,
            count_in: st.count_in,
            bpm: 0.0,
            taps: Vec::new(),
            trainer: false,
            trainer_n: st.trainer_n.clamp(1, 16),
            trainer_step: st.trainer_step.clamp(1, 20),
            trainer_keys: false,
            keys_done: 0,
            passes: 0,
            wraps_seen: 0,
            sec_name: String::new(),
            mark_name: String::new(),
            sec_del: None,
            mark_del: None,
            wv_zoom: 1.0,
            wv_start: 0.0,
            wv_drag: None,
            pinch_at: -10.0,
            note: String::new(),
            note_at: None,
            keyboard: false,
            zoom_set: false,
            bands: vec![0.0; 32],
            peaks: vec![0.0; 32],
            pending_loop: None,
            shot_asked: false,
            mt: metro::Metro { on: false, ..st.mt.clone() },
            mt_bpm: st.mt_bpm.clamp(20, 400),
            mt_add: 0.0,
            mt_step_at: Instant::now(),
            mt_open: false,
            mt_group: 0,
            mt_vol: st.mt_vol.clamp(0.0, 1.0),
            mt_sig: 0,
            mt_gen: 0,
            mt_vis: None,
            beat: None,
            beat_grid: None,
            beat_busy: false,
            beat_failed: HashSet::new(),
            mt_pos: (0.0, Instant::now()),
            mt_lead_end: 0.0,
            mt_sync_at: Instant::now(),
            shuffle: st.shuffle,
            repeat: st.repeat.min(2),
            shuf_order: Vec::new(),
            shuf_key: 0,
        };
        app.wear();
        // one of the phone's players (audio focus, the lock screen and notification, headphone buttons)
        bridge::init(app.player.sender(), app.ctx.clone());
        // the song you had last, ready (not playing)
        let last = app.songs.iter().find(|s| s.vid == st.last).cloned().or_else(|| (!st.last_song.vid.is_empty() && st.last_song.vid == st.last).then(|| st.last_song.clone()));
        if let Some(s) = last {
            app.load(s, false);
        } else if app.songs.is_empty() {
            app.view = 0;
        }
        #[cfg(not(target_os = "android"))]
        app.pc_hooks();
        #[cfg(target_os = "android")]
        app.ytdlp_start();
        app
    }

    /// Studio as a skin (the desktop's own generator): the frames and the playlist font read from its pictures.
    fn wear(&mut self) {
        match genskin::wear(&studio::studio_pal()) {
            Ok((_, art)) => {
                skin::set_custom(Some(studio::studio_pal()));
                self.wa_art = Some(winamp_ui::WaTex::new(&self.ctx, art));
                winamp_ui::set_chrome(self.wa_art.as_ref());
            }
            Err(e) => self.set_note(&format!("Couldn't put the look on: {}", e)),
        }
    }

    pub(crate) fn save_settings(&self) {
        save_json(
            "settings.json",
            &Settings {
                volume: self.volume,
                view: self.view,
                mtab: self.mtab,
                count_in: self.count_in,
                trainer_n: self.trainer_n,
                trainer_step: self.trainer_step,
                last: self.cur.as_ref().map(|s| s.vid.clone()).unwrap_or_default(),
                keep: self.keep,
                screen: self.screen,
                mine_tab: self.mine_tab,
                last_song: self.cur.clone().unwrap_or_default(),
                folder: self.folder.clone(),
                mt: self.mt.clone(),
                mt_bpm: self.mt_bpm,
                mt_vol: self.mt_vol,
                shuffle: self.shuffle,
                repeat: self.repeat,
            },
        );
    }

    pub(crate) fn save_kept(&self) {
        save_json("practice.json", &self.kept);
    }

    /// Run `f` off the screen's thread and send what it returns back.
    pub(crate) fn spawn(&self, f: impl FnOnce() -> Msg + Send + 'static) {
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(f());
            ctx.request_repaint();
        });
    }

    pub(crate) fn set_note(&mut self, s: &str) {
        self.note = s.to_string();
    }

    pub(crate) fn pos(&self) -> f32 {
        self.seek_drag.unwrap_or_else(|| self.player.ctl.pos())
    }

    /// The song playing's own practice settings (made when first asked for; the song kept with them, and when).
    pub(crate) fn kept_mut(&mut self) -> Option<&mut Kept> {
        let s = self.cur.clone()?;
        let k = self.kept.entry(s.vid.clone()).or_default();
        k.song = s;
        k.at = now_secs();
        Some(k)
    }

    pub(crate) fn kept_now(&self) -> Kept {
        self.cur.as_ref().and_then(|s| self.kept.get(&s.vid)).cloned().unwrap_or_default()
    }

    /// A song into the player: its audio fetched (kept on the phone after the first time), its loop, speed and
    /// pitch as you left them. `play`: start it once it's in.
    pub(crate) fn load(&mut self, s: Song, play: bool) {
        let same = self.cur.as_ref().is_some_and(|c| c.vid == s.vid);
        // (a take running ends with its own song)
        if !same && self.rec_buf.is_some() {
            self.rec_toggle();
        }
        if same && !self.loading && !self.wave.is_empty() {
            if play && (self.stopped || self.paused) {
                self.play_pause();
            }
            return;
        }
        self.player.send(Cmd::Stop);
        self.pending_loop = None;
        self.gen += 1;
        let g = self.gen;
        let k = self.kept.get(&s.vid).cloned().unwrap_or_default();
        self.speed = if k.speed == 0 { 100 } else { k.speed };
        self.cents = k.cents;
        self.bpm = k.bpm;
        self.loop_a = None;
        self.loop_b = None;
        self.loop_on = false;
        self.passes = 0;
        self.trainer = false;
        self.wv_zoom = 1.0;
        self.wv_start = 0.0;
        self.sec_del = None;
        self.mark_del = None;
        self.dur = s.secs;
        self.wave.clear();
        self.audio = None;
        self.load_error = None;
        self.loading = true;
        self.paused = false;
        self.stopped = true;
        self.want_play = play;
        self.cur = Some(s.clone());
        self.sync_player();
        self.save_settings();
        let song = s.clone();
        let (keep, pct) = (self.keep, self.dl_pct.clone());
        pct.store(0, Ordering::Relaxed);
        self.spawn(move || Msg::Audio(g, song_audio(&song, keep, &pct).map(|b| {
            let (d, w) = wave(&b);
            (b, d, w)
        })));
    }

    /// The loop, speed, pitch, ear mode and count-off, to the player.
    pub(crate) fn sync_player(&self) {
        let (a, b) = (self.loop_a.unwrap_or(0.0), self.loop_b.unwrap_or(0.0));
        self.player.ctl.set_loop(self.loop_on && b > a, a, b);
        self.player.ctl.set_speed(self.speed);
        self.player.ctl.set_semis(self.cents);
        self.player.ctl.set_chan(self.chan);
        self.player.ctl.set_count_in(if self.bpm > 0.0 { self.count_in } else { 0 }, self.bpm);
    }

    pub(crate) fn play_pause(&mut self) {
        if self.loading || self.cur.is_none() {
            return;
        }
        if self.stopped {
            if let Some(b) = self.audio.clone() {
                self.player.send(Cmd::Play(b.to_vec()));
                self.sync_player();
                let at = self.loop_a.filter(|_| self.loop_on).unwrap_or(0.0);
                if at > 0.0 {
                    self.player.send(Cmd::Seek(at));
                }
                self.stopped = false;
                self.paused = false;
                // (other players pause)
                bridge::focus(true);
            }
            return;
        }
        self.paused = !self.paused;
        self.player.send(if self.paused { Cmd::Pause } else { Cmd::Resume });
        if !self.paused {
            bridge::focus(true);
        }
    }

    /// The phone's media controls kept up with what's playing: a new song (and its cover), playing or paused, its
    /// speed, a jump; told only on a change.
    fn sync_session(&mut self) {
        let Some(c) = self.cur.clone() else {
            if !self.sess_sig.0.is_empty() {
                self.sess_sig = Default::default();
                bridge::session(None);
            }
            return;
        };
        let playing = !self.stopped && !self.paused && !self.loading;
        let pos = self.pos();
        let spd = self.speed as f32 / 100.0;
        let has_art = self.thumb_png.contains_key(&c.vid);
        // (a jump: the song somewhere other than where it'd be by now)
        let (p0, t0) = self.sess_pos;
        let expect = if self.sess_sig.1 { p0 + t0.elapsed().as_secs_f32() * spd } else { p0 };
        let jumped = (pos - expect).abs() > 1.5;
        let sig = (c.vid.clone(), playing, self.speed, has_art);
        if sig == self.sess_sig && !jumped {
            return;
        }
        let new_look = sig.0 != self.sess_sig.0 || sig.3 != self.sess_sig.3;
        self.sess_sig = sig;
        self.sess_pos = (pos, Instant::now());
        let (who, name) = parts(&c);
        let art = if new_look { Some(self.thumb_png.get(&c.vid).map(|v| v.as_slice()).unwrap_or(&[])) } else { None };
        bridge::session(Some(bridge::Now { title: &name, artist: &who, dur: self.dur, pos, playing, speed: spd, art }));
    }

    pub(crate) fn stop(&mut self) {
        bridge::focus(false);
        self.player.send(Cmd::Stop);
        self.stopped = true;
        self.paused = false;
    }

    pub(crate) fn seek(&mut self, t: f32) {
        if self.stopped {
            self.play_pause();
        }
        self.player.send(Cmd::Seek(t.clamp(0.0, self.dur.max(0.0))));
    }

    /// The song before or after this one in the list it was started from (your songs, a My Tracks list): in its
    /// shuffled order with SHUF on, round to the other end with REP on.
    pub(crate) fn step_song(&mut self, d: i32) {
        if self.cur.is_none() {
            return;
        }
        if d < 0 && self.pos() > 3.0 {
            let a = self.loop_a.filter(|_| self.loop_on).unwrap_or(0.0);
            self.seek(a);
            return;
        }
        if let Some(s) = self.song_beside(d, self.repeat > 0) {
            self.load(s, true);
        }
    }

    /// The song `d` along from this one (in the shuffled order with SHUF on; `wrap`: round to the other end).
    fn song_beside(&mut self, d: i32, wrap: bool) -> Option<Song> {
        let c = self.cur.clone()?;
        let list = if self.play_list.iter().any(|s| s.vid == c.vid) { self.play_list.clone() } else { self.songs.clone() };
        let order: Vec<String> = if self.shuffle { self.shuffled(&list, &c.vid) } else { list.iter().map(|s| s.vid.clone()).collect() };
        let i = order.iter().position(|v| *v == c.vid)? as i32;
        let n = order.len() as i32;
        let j = if wrap { (i + d).rem_euclid(n.max(1)) } else { i + d };
        if j < 0 || j >= n || j == i {
            return None;
        }
        let vid = &order[j as usize];
        list.iter().find(|s| s.vid == *vid).cloned()
    }

    /// The list in a shuffled order (made once a list, the song playing first; made again when the list changes).
    fn shuffled(&mut self, list: &[Song], first: &str) -> Vec<String> {
        let key = list.iter().fold(1469598103934665603u64, |h, s| (h ^ yt::hash_str(&s.vid)).wrapping_mul(1099511628211));
        if key != self.shuf_key || !self.shuf_order.iter().any(|v| v == first) {
            let mut v: Vec<String> = list.iter().map(|s| s.vid.clone()).filter(|x| x != first).collect();
            let mut seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7) | 1;
            for i in (1..v.len()).rev() {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                v.swap(i, (seed % (i as u64 + 1)) as usize);
            }
            v.insert(0, first.to_string());
            self.shuf_order = v;
            self.shuf_key = key;
        }
        self.shuf_order.clone()
    }

    /// A song played to its end: again (REP: this song), else the next one in its list (round to the top with REP
    /// all), else it stays stopped.
    fn song_ended(&mut self) {
        if self.repeat == 2 {
            self.want_play = true;
            self.play_pause();
            return;
        }
        if let Some(s) = self.song_beside(1, self.repeat == 1) {
            self.load(s, true);
        } else if self.repeat == 1 {
            // (REP all with one song: it again)
            self.want_play = true;
            self.play_pause();
        }
    }

    /// SHUF: on or off.
    pub(crate) fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        self.shuf_key = 0;
        self.set_note(if self.shuffle { "Shuffle on" } else { "Shuffle off" });
        self.save_settings();
    }

    /// REP: off, all (round to the top of the list), this song.
    pub(crate) fn cycle_repeat(&mut self) {
        self.repeat = (self.repeat + 1) % 3;
        self.set_note(["Repeat off", "Repeat all", "Repeat this song"][self.repeat as usize]);
        self.save_settings();
    }

    /// A song liked (kept in Liked, a phone track too), or not any more.
    pub(crate) fn toggle_like(&mut self, s: &Song) {
        if self.liked.iter().any(|x| x.vid == s.vid) {
            self.liked.retain(|x| x.vid != s.vid);
        } else {
            self.liked.insert(0, s.clone());
        }
        save_json("liked.json", &self.liked);
    }

    pub(crate) fn is_liked(&self, vid: &str) -> bool {
        self.liked.iter().any(|x| x.vid == vid)
    }

    pub(crate) fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(0.0, 1.0);
        self.player.send(Cmd::Volume(self.volume * self.volume));
    }

    /// Add a song to your list (on top; once) and play it on the practice screen.
    pub(crate) fn add_and_play(&mut self, s: Song) {
        self.songs.retain(|x| x.vid != s.vid);
        self.songs.insert(0, s.clone());
        save_json("songs.json", &self.songs);
        self.view = 1;
        self.play_list = self.songs.clone();
        self.load(s, true);
    }

    /// What's typed (or pasted): a link is that video, anything else a search.
    pub(crate) fn go(&mut self) {
        let q = self.query.trim().to_string();
        if q.is_empty() {
            return;
        }
        if let Some(vid) = video_id(&q) {
            self.add_link(vid);
            self.query.clear();
            return;
        }
        self.searching = true;
        self.searched = q.clone();
        self.spawn(move || Msg::Found(q.clone(), yt_search(&q)));
    }

    pub(crate) fn add_link(&mut self, vid: String) {
        if let Some(s) = self.songs.iter().find(|s| s.vid == vid).cloned() {
            self.add_and_play(s);
            return;
        }
        self.adding = true;
        self.set_note("Looking up the video...");
        self.spawn(move || Msg::Added(yt_details(&vid)));
    }

    fn drain(&mut self) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Found(q, r) => {
                    if q != self.searched {
                        continue;
                    }
                    self.searching = false;
                    match r {
                        Ok(v) => {
                            if v.is_empty() {
                                self.set_note("Nothing found. Try other words.");
                            }
                            self.results = v;
                        }
                        Err(e) => self.set_note(&format!("YouTube search didn't work: {}", e)),
                    }
                }
                Msg::Added(r) => {
                    self.adding = false;
                    match r {
                        Ok(s) => {
                            self.note.clear();
                            self.add_and_play(s);
                        }
                        Err(e) => self.set_note(&format!("Couldn't open that video: {}", e)),
                    }
                }
                Msg::Audio(g, r) => {
                    if g != self.gen {
                        continue;
                    }
                    self.loading = false;
                    match r {
                        Ok((b, d, w)) => {
                            if d > 0.0 {
                                self.dur = d;
                                // (the length YouTube gave can be a second off: the song's own is kept)
                                if let Some(c) = self.cur.as_mut() {
                                    c.secs = d;
                                }
                                if let Some(s) = self.songs.iter_mut().find(|s| Some(&s.vid) == self.cur.as_ref().map(|c| &c.vid)) {
                                    s.secs = d;
                                }
                            }
                            self.wave = w;
                            self.audio = Some(std::sync::Arc::new(b));
                            self.stopped = true;
                            if self.want_play {
                                self.play_pause();
                            }
                            // no tempo yet (the count-off clicks in time with it), or the metronome locked to the song:
                            // its beat found in the background
                            let known = self.beat.as_ref().map(|b| &b.0) == self.cur.as_ref().map(|c| &c.vid);
                            if (self.bpm <= 0.0 || (self.mt.on && self.mt.sync)) && !known {
                                self.find_beats();
                            }
                        }
                        Err(e) => {
                            api::log(&format!("audio {}: {}", self.cur.as_ref().map(|c| c.vid.as_str()).unwrap_or(""), e));
                            self.load_error = Some(e);
                        }
                    }
                }
                Msg::DeviceTracks(r) => {
                    self.device_state = 3;
                    match r {
                        Ok(v) => {
                            self.device_tracks = v;
                            self.device_err.clear();
                        }
                        Err(e) => self.device_err = e,
                    }
                }
                Msg::Saved(vid, r) => {
                    self.saver_busy = false;
                    self.saving = None;
                    self.stats_at = -10.0;
                    let asked = self.save_queue.contains(&vid);
                    self.save_queue.retain(|v| *v != vid);
                    match r {
                        Ok(()) if asked => self.set_note("Saved on this phone. It plays with no connection"),
                        Ok(()) => {}
                        Err(e) => {
                            api::log(&format!("saving {}: {}", vid, e));
                            if asked {
                                self.set_note(&format!("Couldn't save it: {}", e));
                            }
                            self.saver_failed.insert(vid);
                        }
                    }
                }
                Msg::YtdlpVersion(r) => {
                    self.ytdlp_ver = Some(r);
                    self.ytdlp_busy = false;
                }
                Msg::YtdlpUpdated(r) => {
                    self.ytdlp_busy = false;
                    self.ytdlp_updating = false;
                    self.ytdlp_note = match r {
                        Ok(t) => t,
                        Err(e) => format!("Couldn't update: {}", e),
                    };
                    self.ytdlp_ver = None;
                }
                Msg::Beats(vid, found) => {
                    self.beat_busy = false;
                    if let Some((period, first, grid)) = found.clone() {
                        self.beat = Some((vid.clone(), period, first));
                        self.beat_grid = Some((vid.clone(), grid, period));
                        self.mt_sig = 0;
                    } else {
                        self.beat_failed.insert(vid.clone());
                    }
                    // the song's tempo (kept with the song, unless you've set one meanwhile)
                    if let Some(b) = found.map(|f| 60.0 / f.0).filter(|b| (30.0..=300.0).contains(b)) {
                        api::log(&format!("tempo of {}: {:.1} BPM", vid, b));
                        if self.bpm <= 0.0 && self.cur.as_ref().is_some_and(|c| c.vid == vid) {
                            self.bpm = b.round();
                            self.sync_player();
                            let bpm = self.bpm;
                            if let Some(k) = self.kept_mut() {
                                k.bpm = bpm;
                            }
                            self.save_kept();
                        }
                    } else {
                        api::log(&format!("no tempo found for {}", vid));
                    }
                }
                Msg::Thumb(vid, img) => {
                    if let Some(img) = img {
                        // (as PNG too, for the lock screen and the notification)
                        if let Some(rgba) = image::RgbaImage::from_raw(img.size[0] as u32, img.size[1] as u32, img.pixels.iter().flat_map(|c| c.to_array()).collect()) {
                            let mut png = std::io::Cursor::new(Vec::new());
                            if rgba.write_to(&mut png, image::ImageFormat::Png).is_ok() {
                                self.thumb_png.insert(vid.clone(), png.into_inner());
                            }
                        }
                        let t = self.ctx.load_texture(format!("thumb_{}", vid), img, egui::TextureOptions::LINEAR);
                        self.thumbs.insert(vid, t);
                    }
                }
            }
        }
    }

    /// A song's picture: a video's thumbnail, a phone track's album art (fetched once; none until it's in).
    pub(crate) fn thumb(&mut self, s: &Song) -> Option<egui::TextureId> {
        if let Some(t) = self.thumbs.get(&s.vid) {
            return Some(t.id());
        }
        if self.thumb_busy.insert(s.vid.clone()) {
            let song = s.clone();
            self.spawn(move || {
                let img = if device::is_file(&song.vid) { device::art(&song) } else { yt_thumb(&song.vid) };
                Msg::Thumb(song.vid.clone(), img)
            });
        }
        None
    }

    /// Once a frame: the song's end, the trainer's passes, the spectrum.
    fn tick(&mut self) {
        let mut ended = false;
        if let Ok(mut sh) = self.player.shared.lock() {
            if sh.ended {
                sh.ended = false;
                self.stopped = true;
                self.paused = false;
                ended = true;
            }
            if let Some(e) = sh.error.take() {
                self.note = format!("Couldn't play it: {}", e);
            }
        }
        if ended {
            self.song_ended();
        }
        // what the phone asked for (the lock screen, the notification, headphones, another player taking the sound)
        for ev in bridge::take() {
            match ev {
                bridge::Ev::Paused { focus } => {
                    if !self.stopped {
                        self.paused = true;
                    }
                    if focus {
                        self.set_note("Paused: another app is playing");
                    }
                }
                bridge::Ev::Resumed => {
                    if !self.stopped {
                        self.paused = false;
                    }
                }
                bridge::Ev::Play => {
                    if self.cur.is_some() {
                        self.want_play = true;
                        if self.stopped {
                            self.play_pause();
                        }
                    }
                }
                bridge::Ev::Next => self.step_song(1),
                bridge::Ev::Prev => self.step_song(-1),
                bridge::Ev::Stop => self.stop(),
                bridge::Ev::Seek(t) => self.seek(t),
            }
        }
        bridge::set_state(if self.cur.is_none() || self.stopped { 0 } else if self.paused { 2 } else { 1 });
        self.sync_session();
        // the trainer: every so many passes of the loop, a little faster, up to the song's own speed
        let w = self.player.ctl.wraps();
        if w != self.wraps_seen {
            let n = w.wrapping_sub(self.wraps_seen);
            self.wraps_seen = w;
            if self.trainer && self.loop_on {
                self.passes += n;
                self.check_trainer();
            }
        }
        self.update_bands();
        self.update_metro();
        self.apply_pending();
        self.save_in_background();
        self.rec_tick();
        // the screen stays on while a song plays (a phone on the music stand)
        let playing = self.cur.is_some() && !self.stopped && !self.paused;
        if playing != self.awake {
            self.awake = playing;
            // (off the screen's thread: Android's own calls can take a moment)
            std::thread::spawn(move || keep_awake(playing));
        }
    }

    /// SAVING on: your songs not on the phone yet, saved one at a time in the background (never while a song is
    /// coming in).
    fn save_in_background(&mut self) {
        if self.saver_busy || self.loading {
            return;
        }
        let now = self.ctx.input(|i| i.time);
        if now - self.saver_at < 3.0 && self.save_queue.is_empty() {
            return;
        }
        self.saver_at = now;
        // the songs asked for first (SAVE on a song), then (Saving on) the rest of your songs
        self.save_queue.retain(|v| !saved(v));
        let next = self.save_queue.first().cloned().or_else(|| {
            self.keep
                .then(|| self.songs.iter().find(|s| !device::is_file(&s.vid) && !self.saver_failed.contains(&s.vid) && !saved(&s.vid)).map(|s| s.vid.clone()))
                .flatten()
        });
        let Some(vid) = next else { return };
        // (the song playing is here whole already: written straight to the phone)
        if let (Some(c), Some(b)) = (&self.cur, &self.audio) {
            if c.vid == vid {
                let file = audio_file(&vid);
                let r = file.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&file, &b[..])).map_err(|e| e.to_string());
                let _ = self.tx.send(Msg::Saved(vid, r));
                return;
            }
        }
        self.saver_busy = true;
        self.saving = Some(vid.clone());
        let pct = self.save_pct.clone();
        pct.store(0, Ordering::Relaxed);
        self.spawn(move || Msg::Saved(vid.clone(), yt_audio(&vid, true, &pct).map(|_| ())));
    }

    /// SAVE on a song: put on your songs (if it isn't) and saved on the phone next, Saving on or off.
    pub(crate) fn save_song(&mut self, s: &Song) {
        if device::is_file(&s.vid) {
            return;
        }
        if saved(&s.vid) {
            self.set_note("Saved on this phone. It plays with no connection");
            return;
        }
        if !self.songs.iter().any(|x| x.vid == s.vid) {
            self.songs.insert(0, s.clone());
            save_json("songs.json", &self.songs);
        }
        self.saver_failed.remove(&s.vid);
        if !self.save_queue.contains(&s.vid) {
            self.save_queue.push(s.vid.clone());
        }
        let (_, name) = parts(s);
        self.set_note(&format!("Saving {} on this phone", name));
    }

    /// A saved loop (or, `b` below `a`, a bookmark's spot) from Loops: its song played on the practice screen and
    /// the loop set once the song is in.
    pub(crate) fn play_spot(&mut self, s: Song, a: f32, b: f32) {
        self.view = 1;
        if b > a {
            self.mtab = 0;
        }
        self.play_list = vec![s.clone()];
        self.load(s, true);
        self.pending_loop = Some((a, b));
        self.save_settings();
    }

    /// The loop or spot asked for, once the song is in.
    fn apply_pending(&mut self) {
        let Some((a, b)) = self.pending_loop else { return };
        if self.loading || self.wave.is_empty() {
            return;
        }
        self.pending_loop = None;
        if b > a {
            self.loop_a = Some(a);
            self.loop_b = Some(b);
            self.loop_on = true;
            self.passes = 0;
            self.sync_player();
        }
        self.seek(a);
    }

    /// yt-dlp readied in the background as the app opens (Python unpacked the first time, so the first song comes
    /// sooner), and brought up to date once a day.
    fn ytdlp_start(&mut self) {
        let due = ytdlp::update_due();
        self.ytdlp_busy = true;
        self.ytdlp_updating = due;
        self.spawn(move || {
            let v = ytdlp::version();
            if due && v.is_ok() {
                return Msg::YtdlpUpdated(ytdlp::update());
            }
            Msg::YtdlpVersion(v)
        });
    }

    /// The trainer, after each pass (the desktop's): every so many passes a little faster up to full speed, or up a 4th
    /// through all 12 keys (kept within half an octave either way; the cycle comes home after 12).
    fn check_trainer(&mut self) {
        if self.passes < self.trainer_n {
            return;
        }
        self.passes = 0;
        if self.trainer_keys {
            self.keys_done += 1;
            if self.keys_done >= 12 {
                self.keys_done = 0;
                self.trainer = false;
                self.cents = 0;
                self.sync_player();
                self.set_note("All 12 keys done. Nice work");
            } else {
                let mut c = self.cents + 500;
                if c > 600 {
                    c -= 1200;
                }
                self.cents = c;
                self.sync_player();
                self.set_note(&format!("Up a 4th: key {} of 12", self.keys_done + 1));
            }
            return;
        }
        if self.speed < 100 {
            self.speed = (self.speed + self.trainer_step).min(100);
            self.sync_player();
            self.set_note(&if self.speed >= 100 { "Full speed. Nice work".to_string() } else { format!("Speed up: {}%", self.speed) });
        }
    }

    /// The spectrum for the deck's bars, from what the player has just played (the old phone app's, tuned).
    fn update_bands(&mut self) {
        let active = self.cur.is_some() && !self.paused && !self.stopped && !self.loading;
        let (samples, rate) = match self.player.viz.lock() {
            Ok(v) if active => {
                let n = v.samples.len().min(2048);
                (v.samples[v.samples.len() - n..].to_vec(), v.rate as f32)
            }
            _ => (Vec::new(), 44100.0),
        };
        let n = samples.len();
        let nb = self.bands.len();
        let mut target = vec![0.0f32; nb];
        if n >= 512 && rate > 0.0 {
            let denom = (n - 1) as f32;
            let xs: Vec<f32> = samples.iter().enumerate().map(|(i, v)| v * (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / denom).cos())).collect();
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
        for i in 0..nb {
            let old = self.bands[i];
            self.bands[i] = if target[i] > old { target[i] } else { (old - 0.06).max(target[i]) };
            self.peaks[i] = self.bands[i].max(self.peaks[i] - 0.012);
        }
    }

    /// A YouTube link on the clipboard (copied with Share > Copy link): offered on the Songs screen as you come back
    /// to the app.
    fn check_clipboard(&mut self) {
        let Some(text) = yt::clipboard_quiet() else { return };
        let Some(vid) = video_id(&text) else { return };
        if vid == self.clip_seen {
            return;
        }
        self.clip_seen = vid.clone();
        if self.cur.as_ref().is_some_and(|c| c.vid == vid) {
            return;
        }
        self.clip_offer = Some(vid);
        self.view = 0;
    }
}

/// The phone's fonts, as the desktop has them with Studio worn: Studio's three faces (the typewriter face first for
/// the reading), Inter and Chakra Petch behind them.
fn phone_fonts() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("chakra".to_string(), egui::FontData::from_static(include_bytes!("../../backline/assets/ChakraPetch-Regular.ttf")));
    fonts.font_data.insert("inter".to_string(), egui::FontData::from_static(include_bytes!("../../backline/assets/Inter-Regular.ttf")));
    if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        f.insert(0, "chakra".to_string());
        f.insert(0, "inter".to_string());
    }
    // (the typewriter face again at the very end, for the box lines round the frames' names and other marks the
    // phone's own fonts lack; the same bytes, nothing added to the app's size)
    fonts.font_data.insert("plex_marks".to_string(), egui::FontData::from_static(include_bytes!("../../backline/assets/IBMPlexMono-Regular.ttf")));
    if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        f.push("plex_marks".to_string());
    }
    fonts.families.insert(egui::FontFamily::Name("skin".into()), vec!["inter".to_string(), "chakra".to_string(), "plex_marks".to_string()]);
    studio::add_fonts(&mut fonts);
    fonts
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(not(target_os = "android"))]
        self.pc_shot(ctx);
        // Backline's drawing is sized for a window about 414 points across: the phone is zoomed to that
        if !self.zoom_set {
            let w = ctx.screen_rect().width();
            if w > 50.0 {
                ctx.set_zoom_factor(ctx.zoom_factor() * w / 414.0);
                self.zoom_set = true;
            }
        }
        font::set_ppp(ctx.pixels_per_point());
        font::set_modern(true);
        self.drain();
        self.tick();
        // coming back to the app: a YouTube link copied meanwhile is offered (asked a moment after, when Android
        // lets the app read the clipboard)
        let focused = ctx.input(|i| i.focused);
        if focused && !self.focused {
            self.clip_check = 3;
        }
        self.focused = focused;
        if self.clip_check > 0 {
            self.clip_check -= 1;
            if self.clip_check == 0 {
                self.check_clipboard();
            }
            ctx.request_repaint();
        }
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
            if !self.note.is_empty() && at.elapsed().as_secs_f32() > 6.0 && !self.adding {
                self.note.clear();
            }
        }
        let falling = self.bands.iter().chain(self.peaks.iter()).any(|v| *v > 0.001);
        if self.mt.on || !self.note.is_empty() || (self.cur.is_some() && !self.stopped && !self.paused) || falling || self.loading || self.searching {
            ctx.request_repaint_after(Duration::from_millis(33));
        } else if self.saving.is_some() {
            // (a song saving: how far it is, on its SAVE key)
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        // (room for the phone's status bar on top and its buttons along the bottom; an iPhone's notch or island and
        // home bar, as it says)
        #[allow(unused_mut)]
        let (mut top, mut bottom) = (34.0, 30.0);
        #[cfg(target_os = "ios")]
        if let Some((w, _, t, b)) = ios::screen() {
            let k = ctx.screen_rect().width() / w.max(1.0);
            top = t * k + 6.0;
            bottom = (b * k).max(8.0) + 2.0;
        }
        egui::CentralPanel::default().frame(egui::Frame::none().fill(studio::BODY)).show(ctx, |ui| {
            let full = ui.max_rect();
            let r = Rect::from_min_max(Pos2::new(full.min.x + 10.0, full.min.y + top), Pos2::new(full.max.x - 10.0, full.max.y - bottom));
            font::with_scale(UI_SCALE, || self.screens(ui, r));
        });
    }

    #[cfg(not(target_os = "ios"))]
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_settings();
        self.save_kept();
    }

    #[cfg(target_os = "ios")]
    fn on_exit(&mut self) {
        self.save_settings();
        self.save_kept();
    }

    /// (an iPhone: the window library gives egui only the part of the screen clear of the notch and the home bar,
    /// yet draws on all of it from the top; egui is given the whole screen, and the room on top and along the bottom
    /// is kept clear below)
    #[cfg(target_os = "ios")]
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        if let Some((w, h, _, _)) = ios::screen() {
            let native = raw.viewports.get(&raw.viewport_id).and_then(|v| v.native_pixels_per_point).unwrap_or_else(ios::scale);
            let k = ios::scale() / (native * ctx.zoom_factor()).max(0.1);
            raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(w * k, h * k)));
        }
    }
}

impl App {
    /// The two windows: NOW PLAYING's deck on top, SONGS or PRACTICE under it (switched by the tabs on its frame).
    fn screens(&mut self, ui: &mut egui::Ui, r: Rect) {
        let deck_h = 228.0;
        let deck = Rect::from_min_max(r.min, Pos2::new(r.max.x, r.min.y + deck_h));
        studio::frame(ui.painter(), deck, "Now Playing");
        self.deck(ui, deck.shrink2(Vec2::new(10.0, 10.0)));
        let low = Rect::from_min_max(Pos2::new(r.min.x, deck.max.y + 22.0), r.max);
        // (a note along the foot, when there is one)
        let low = if self.note.is_empty() {
            low
        } else {
            let nr = Rect::from_min_max(Pos2::new(low.min.x, low.max.y - 26.0), low.max);
            studio::frame(ui.painter(), nr, "");
            let resp = ui.interact(nr, ui.id().with("note"), Sense::click());
            studio::marquee(ui, nr.shrink2(Vec2::new(8.0, 0.0)), &self.note.to_uppercase(), 13.0, studio::ink());
            if resp.clicked() {
                self.note.clear();
            }
            Rect::from_min_max(low.min, Pos2::new(low.max.x, nr.min.y - 12.0))
        };
        let name = ["Songs", "Practice", "Settings", "My Tracks"][self.view.min(3) as usize];
        studio::frame(ui.painter(), low, name);
        // (the tabs in the order you go: find a song, your tracks, practise, settings)
        const ORDER: [u8; 4] = [0, 3, 1, 2];
        let tabs = [("Songs", self.view == 0, ""), ("My Tracks", self.view == 3, ""), ("Practice", self.view == 1, ""), ("Settings", self.view == 2, "")];
        if let Some(v) = studio::frame_tabs(ui, low, &tabs, "views") {
            self.view = ORDER[v];
            self.save_settings();
        }
        let inner = low.shrink2(Vec2::new(12.0, 12.0));
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(Align::Min)));
        c.set_clip_rect(inner.expand(2.0));
        match self.view {
            0 => self.songs_ui(&mut c),
            1 => self.practice_ui(&mut c),
            3 => self.mine_ui(&mut c),
            _ => self.settings_ui(&mut c),
        }
    }
}

/// A row of controls centred across the window (the desktop's `centered_row`). Too wide for the room, it wraps.
pub(crate) fn centered_row(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    let key = ui.id().with(("centered_row", id));
    let w: f32 = ui.ctx().data(|d| d.get_temp(key)).unwrap_or(0.0);
    let room = ui.available_width();
    if w > room {
        let r = ui.horizontal_wrapped(add).response.rect;
        if r.height() <= bh() + 6.0 && r.width() < room - 8.0 {
            ui.ctx().data_mut(|d| d.insert_temp(key, r.width()));
            ui.ctx().request_repaint();
        }
        return;
    }
    ui.horizontal(|ui| {
        ui.add_space(((room - w) / 2.0).max(0.0).floor());
        let r = ui.horizontal(add).response.rect;
        if (r.width() - w).abs() > 0.5 {
            ui.ctx().data_mut(|d| d.insert_temp(key, r.width()));
            ui.ctx().request_repaint();
        }
    });
}

/// A dim line of small print centred (the desktop's `dim_centered`).
pub(crate) fn dim_centered(ui: &mut egui::Ui, text: &str) {
    let text = winamp_ui::caps_words(text);
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 7.0 * spx(1.5) + 8.0), Sense::hover());
    ptext_fit(ui.painter(), r.center(), Align::Center, &text, 1.5, r.width() - 12.0, pal().ink2);
}

/// The fine line between groups of keys in a row (the desktop's `divider`).
pub(crate) fn divider(ui: &mut egui::Ui) {
    let h = bh();
    let (r, _) = ui.allocate_exact_size(Vec2::new(9.0, h), Sense::hover());
    ui.painter().line_segment([Pos2::new(r.center().x, r.min.y + 4.0), Pos2::new(r.center().x, r.max.y - 4.0)], Stroke::new(1.0_f32, pal().panel_lt));
}

/// A copy on the PC in a phone-sized window, for trying it out and for pictures (`cargo run --example pc`).
/// PHONE_VIEW=songs|practice, PHONE_TAB=0-3, PHONE_SONG=<video id> (loaded), PHONE_PLAY=1, PHONE_LOOP=<a>-<b>,
/// PHONE_QUERY=<words> (searched), PHONE_SHOT=<file.png> after PHONE_SHOT_AT seconds (then it closes).
#[cfg(not(target_os = "android"))]
pub fn run_pc() {
    // (pictures are taken in silence)
    if std::env::var_os("PHONE_SHOT").is_some() {
        player::MUTED.store(true, Ordering::Relaxed);
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([414.0, 900.0]).with_title("Backline phone"),
        ..Default::default()
    };
    let _ = eframe::run_native("Backline phone", options, Box::new(|cc| Ok(Box::new(App::new(cc)))));
}

#[cfg(not(target_os = "android"))]
impl App {
    fn pc_hooks(&mut self) {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        if let Some(v) = env("PHONE_VIEW") {
            self.view = match v.as_str() {
                "practice" => 1,
                "settings" => 2,
                "mine" => 3,
                _ => 0,
            };
        }
        // (My Tracks: the list, the folder open in Files, the song open in Loops)
        if let Some(t) = env("PHONE_MINE").and_then(|t| t.parse().ok()) {
            self.mine_tab = t;
        }
        if let Some(f) = std::env::var("PHONE_FOLDER").ok() {
            self.folder = f;
        }
        if let Some(v) = env("PHONE_LOOPS") {
            self.loops_pick = Some(v);
        }
        // (the metronome's panel open; "on": clicking)
        if let Some(v) = env("PHONE_METRO") {
            self.view = 1;
            self.mt_open = true;
            self.mt.on = v == "on";
        }
        if let Some(t) = env("PHONE_TAB").and_then(|t| t.parse().ok()) {
            self.mtab = t;
        }
        if let Some(q) = env("PHONE_QUERY") {
            self.query = q;
            self.go();
        }
        if let Some(vid) = env("PHONE_SONG") {
            let s = self.songs.iter().find(|s| s.vid == vid).cloned().unwrap_or(Song { vid: vid.clone(), title: vid.clone(), ..Default::default() });
            self.load(s, env("PHONE_PLAY").is_some());
        }
        if let Some((a, b)) = env("PHONE_LOOP").and_then(|l| {
            let (a, b) = l.split_once('-')?;
            Some((a.parse::<f32>().ok()?, b.parse::<f32>().ok()?))
        }) {
            self.pending_loop = Some((a, b));
        }
        if let Some(n) = env("PHONE_NOTE") {
            self.note = n;
        }
    }

    fn pc_shot(&mut self, ctx: &egui::Context) {
        let Ok(out) = std::env::var("PHONE_SHOT") else { return };
        let at: f64 = std::env::var("PHONE_SHOT_AT").ok().and_then(|v| v.parse().ok()).unwrap_or(4.0);
        let now = ctx.input(|i| i.time);
        let got = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = got {
            let [w, h] = img.size;
            let buf: Vec<u8> = img.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b(), 255]).collect();
            if let Some(b) = image::RgbaImage::from_raw(w as u32, h as u32, buf) {
                let _ = b.save(&out);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if now >= at && !self.shot_asked {
            self.shot_asked = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

/// The iPhone app (examples/ios.rs): Studio drawn with Metal, the phone's sound session set up first.
#[cfg(target_os = "ios")]
pub fn run_ios() {
    ios::setup();
    let options = eframe::NativeOptions::default();
    if let Err(e) = eframe::run_native("Backline", options, Box::new(|cc| Ok(Box::new(App::new(cc))))) {
        api::log(&format!("couldn't start: {}", e));
    }
}

#[cfg(target_os = "android")]
static ANDROID: std::sync::OnceLock<winit::platform::android::activity::AndroidApp> = std::sync::OnceLock::new();

/// The screen kept on (a screen wake lock from Android's power service: callable from any thread, unlike the
/// window's keep-on flag, which went through a lock the app's main thread also waits on, and froze it).
fn keep_awake(on: bool) {
    #[cfg(target_os = "ios")]
    ios::keep_awake(on);
    #[cfg(target_os = "android")]
    {
        use jni::objects::{GlobalRef, JObject, JValue};
        static LOCK: std::sync::Mutex<Option<GlobalRef>> = std::sync::Mutex::new(None);
        let r = (|| -> Option<()> {
            let ctx = ndk_context::android_context();
            let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
            let mut env = vm.attach_current_thread().ok()?;
            let mut held = LOCK.lock().ok()?;
            if held.is_none() {
                let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
                let name = env.new_string("power").ok()?;
                let pm = env.call_method(&activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&name).into()]).ok()?.l().ok()?;
                let tag = env.new_string("backline:practice").ok()?;
                // SCREEN_BRIGHT_WAKE_LOCK (0xa) | ON_AFTER_RELEASE (0x20000000): the screen as bright as it was, and not
                // straight off when it's let go
                let wl = env
                    .call_method(&pm, "newWakeLock", "(ILjava/lang/String;)Landroid/os/PowerManager$WakeLock;", &[JValue::Int(0x0000_000a | 0x2000_0000), (&tag).into()])
                    .ok()?
                    .l()
                    .ok()?;
                env.call_method(&wl, "setReferenceCounted", "(Z)V", &[JValue::Bool(0)]).ok()?;
                *held = Some(env.new_global_ref(wl).ok()?);
            }
            let wl = held.as_ref()?;
            if on {
                // (never longer than an hour, should the app be closed without letting go)
                env.call_method(wl.as_obj(), "acquire", "(J)V", &[JValue::Long(3_600_000)]).ok()?;
            } else {
                env.call_method(wl.as_obj(), "release", "()V", &[]).ok()?;
            }
            Some(())
        })();
        if r.is_none() {
            api::log("couldn't keep the screen on");
        }
    }
    let _ = on;
}

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
    // settings, your songs and their audio live in the app's own storage
    if let Some(p) = app.internal_data_path() {
        std::env::set_var("BACKLINE_HOME", p);
    }
    // a test phone (a file named MUTE in the app's outside folder) plays silently
    if let Some(p) = app.external_data_path() {
        let _ = std::fs::create_dir_all(&p);
        if p.join("MUTE").exists() {
            player::MUTED.store(true, Ordering::Relaxed);
        }
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
