//! Backline for Android. The engine (Tidal, the player, decoding, theory) and the look (the pixel font, the skins,
//! the icons, the drawing kit, the neck and piano pictures) are the desktop app's own code, compiled in from
//! backline/src - a fix there is a fix here, and the phone looks like Backline. Only the screens are the phone's:
//! the same windows as the desktop, stacked for a phone.
#![allow(dead_code, unused_imports, unused_variables, clippy::all)]

// ---- the desktop engine
#[path = "../../backline/src/data/api.rs"]
mod api;
#[path = "../../backline/src/data/cache.rs"]
mod cache;
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
#[path = "../../backline/src/ui/atonal_core.rs"]
mod atonal_core;
#[path = "../../backline/src/ui/music_data.rs"]
mod music_data;
mod chords;
mod deck;
mod lines_ui;
#[path = "../../backline/src/data/lines.rs"]
mod lines;
#[path = "../../backline/src/ui/triad_core.rs"]
mod triad_core;
#[path = "../../backline/src/ui/si_core.rs"]
mod si_core;
#[path = "../../backline/src/ui/staff.rs"]
mod staff;
mod menu;
use menu::{Playlist, QTrack};
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
    /// your own Tidal playlists (uuid, name), a Tidal page (album, artist, radio, home, a card opened), a song's
    /// credits, and a job done (what to say)
    OwnLists(Result<Vec<(String, String)>, String>),
    Page(Result<api::Page, String>),
    Credits(Result<Vec<(String, String)>, String>),
    Done(Result<String, String>),
    /// how far the song is from A440 (cents, confidence)
    Tuning(i64, Option<(i32, f32)>),
    /// the songs found on the phone, and its playlist files
    Files(Vec<(Track, std::path::PathBuf)>, Vec<(String, Vec<std::path::PathBuf>)>),
    /// a song's beat (its usual length, the first one, every beat), for locking the metronome to it
    Beats(i64, Option<band::BeatInfo>),
    /// a WAV saved (where) or not
    Exported(Result<String, String>),
    Code(Result<DeviceCode, String>),
    LoggedIn(Result<(), String>),
    Found(Result<api::Page, String>),
    /// your Tidal library (albums, artists, new from artists you follow)
    Library(Result<api::Page, String>),
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
    /// PLAY: the queue
    Player,
    /// MUSIC: Tidal, files, YouTube
    Library,
    Practice,
    Chords,
    Lines,
    /// opened from the Practice and Now Playing screens
    Metro,
    /// NOW PLAYING: the Winamp deck, full screen, no tabs (opened from the now-playing bar)
    Now,
}

#[derive(PartialEq, Clone, Copy)]
pub(crate) enum Repeat {
    Off,
    All,
    One,
}

/// A saved loop of a song.
#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub(crate) struct Section {
    pub name: String,
    pub a: f32,
    pub b: f32,
    /// your note on it, and the group it's kept in within the song (as on the desktop)
    pub note: String,
    pub group: String,
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
    /// MUSIC: 9 the home page (the source tiles and what was played lately), else 0 Tidal, 1 files, 2 YouTube
    pub lib_tab: u8,
    /// what was played lately, newest first (this session)
    pub recent: Vec<Track>,
    /// Tidal searches already made (query -> songs), and the one on its way
    pub search_cache: HashMap<String, api::Page>,
    /// the search's albums, artists and playlists; Tidal's home page; your library page; a long list's filter
    pub result_rows: Vec<(String, Vec<api::Card>)>,
    pub home: Option<api::Page>,
    pub lib_page: Option<api::Page>,
    pub list_filter: String,
    /// the card each open page came from (to save an album or follow an artist), and the one on its way
    pub page_cards: Vec<Option<api::Card>>,
    pub page_pending: Option<api::Card>,
    /// the open page's cover (its address, the picture once it's here)
    pub page_cover: Option<(String, Option<egui::TextureHandle>)>,
    pub search_q: String,
    /// MY TRACKS: 0 liked, 1 playlists, 2 downloaded (the songs kept on this phone)
    pub my_page: u8,
    /// the song menu's page (0 the menu, 1 playlists, 2 credits), the playlist song it was opened on, a new
    /// playlist's name, the credits, and your own Tidal playlists
    pub menu_mode: u8,
    pub menu_pl: Option<(usize, usize)>,
    pub menu_new: String,
    pub credits: Vec<(String, String)>,
    pub own_lists: Vec<(String, String)>,
    /// your playlists on the phone, and the one open
    pub playlists: Vec<Playlist>,
    pub pl_open: Option<usize>,
    /// where the restored song was (it starts from there), the last run's place loaded, the last save of it
    pub resume_at: Option<f32>,
    pub session_loaded: bool,
    pub session_t: Option<Instant>,
    /// Tidal pages opened (albums, artists, playlists, radios), the newest on top, and one on its way
    pub pages: Vec<api::Page>,
    pub page_busy: bool,
    /// the queue in reorder mode, and the song being dragged (its place, where the finger is)
    pub q_reorder: bool,
    pub q_drag: Option<(usize, f32)>,
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
    /// the playlist files found (name, songs), and a look under way
    pub file_lists: Vec<(String, Vec<std::path::PathBuf>)>,
    pub files_busy: bool,
    /// names you gave songs (id -> title, artist), and the edit open in the song menu (title, artist, key, bpm)
    pub names: HashMap<i64, (String, String)>,
    pub edit: (String, String, String, String),
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
    /// the trainer goes up a 4th each time (through all 12 keys) instead of faster
    pub trainer_keys: bool,
    pub keys_done: u32,
    /// the waveform's zoom (1 the whole song) and the time at its left edge; a drag on it: what it moves (0 a new
    /// loop, 1 A, 2 B, 3 the whole loop), where it started, and A and B then
    pub wv_zoom: f32,
    pub wv_off: f32,
    pub wv_drag: Option<(u8, f32, f32, f32)>,
    /// the saved loop playing (its place in the song's list), the one being edited, the one asked to delete
    pub sec_cur: Option<usize>,
    pub sec_edit: Option<usize>,
    pub sec_del: Option<usize>,
    /// how far each song is from A440 (cents), and a check or an export under way
    pub tuning: HashMap<i64, i32>,
    pub tuning_busy: bool,
    pub exporting: bool,
    pub wraps: u32,
    pub marks: HashMap<i64, Vec<(f32, String)>>,
    pub eq: [f32; 10],
    pub eq_on: bool,
    // ---- chords
    pub ch_tab: u8,
    pub cf_root: Option<i32>,
    pub cf_fam: Option<usize>,
    /// FIND A CHORD: the chord picked, shown on the long neck (false: the picker), and which voicing is lit
    pub cf_neck: bool,
    pub cf_voice: usize,
    /// FIND A CHORD: the staff and piano shown over the neck
    pub cf_staff: bool,
    /// your own grips, by chord (the desktop's SAVE GRIP), and the analyzer's neck size (0 normal, 1 big)
    pub my_grips: HashMap<String, Vec<[Option<u8>; 6]>>,
    pub an_big: bool,
    /// LINES: alternate triads (0) or an interval cycle (1), the root, the scale or cycle, the start (of the
    /// places the first note can go), the finger, and whether the pickers are pulled up
    pub ln_kind: u8,
    pub ln_root: i32,
    pub ln_scale: usize,
    pub ln_cycle: usize,
    pub ln_start: usize,
    pub ln_finger: u8,
    pub ln_sheet: bool,
    /// when the line being heard ends (its Play button reads Stop till then)
    pub ln_end: Option<Instant>,
    /// LINES: the two tetrachords joined, the tempo, notes to a beat, and round and round
    pub ln_lower: usize,
    pub ln_upper: usize,
    pub ln_bpm: u32,
    pub ln_sub: u32,
    pub ln_loop: bool,
    /// LINES' page (0 atonal lines, 1 superimpose); SUPERIMPOSE: the chord (root, kind, which), its scale, the
    /// usual choices or any shape, the shape and its root when any, the usual choice picked
    pub ln_page: u8,
    pub si_root: i32,
    pub si_fam: usize,
    pub si_chord: usize,
    pub si_scale: usize,
    pub si_any: bool,
    pub si_kind: usize,
    pub si_kroot: i32,
    pub si_pick: usize,
    /// the gear's page is open
    pub settings_open: bool,
    /// asked once already: sign out of Tidal, clear the kept songs
    pub sure_out: bool,
    pub sure_clear: bool,
    /// a pick list open from the bottom (which chip, its choices, the one chosen now), and a choice just made
    pub picker: Option<(String, Vec<String>, usize)>,
    pub picked: Option<(String, usize)>,
    /// the metronome's full settings showing (meter, accents, sound, gap, faster, volume)
    pub mt_more: bool,
    /// the beat found in a song (whose), the scale x2 and /2 put on it, a search under way, the songs with none,
    /// what the clicks were last laid for, and the speed the click last followed
    pub mt_beat: Option<(i64, band::BeatInfo)>,
    pub mt_scale: f32,
    pub mt_beat_busy: bool,
    pub mt_beat_failed: Vec<i64>,
    pub mt_laid: u64,
    pub mt_speed_seen: u32,
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
    /// TRIADS: the page (map, voice leading, over a bass), the changes typed, where on the neck to start, the bass
    pub tr_page: u8,
    pub tr_prog: String,
    pub tr_pos: u8,
    pub tr_bass: i32,
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
        // the songs' keys and tempos found before (or set by hand)
        meta::load(&load_json("meta.json"));
        // the songs kept on this phone, and their details for the Downloaded list
        cache::scan();
        cache::start_tagging(|_| None);
        SKIN.store(st.skin % PALS.len(), Ordering::Relaxed);
        font::set_ppp(ctx.pixels_per_point());
        let font_tex = font::init(&ctx);
        // reading text in Inter, a face made for screens (Chakra Petch behind it); the pixel font is the deck's alone
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .font_data
            .insert("chakra".to_string(), egui::FontData::from_static(include_bytes!("../../backline/assets/ChakraPetch-Regular.ttf")));
        fonts.font_data.insert("inter".to_string(), egui::FontData::from_static(include_bytes!("../../backline/assets/Inter-Regular.ttf")));
        if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            f.insert(0, "chakra".to_string());
            f.insert(0, "inter".to_string());
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
            lib_tab: 3,
            recent: Vec::new(),
            search_cache: HashMap::new(),
            result_rows: Vec::new(),
            home: None,
            lib_page: None,
            list_filter: String::new(),
            page_cards: Vec::new(),
            page_pending: None,
            page_cover: None,
            search_q: String::new(),
            my_page: 0,
            menu_mode: 0,
            menu_pl: None,
            menu_new: String::new(),
            credits: Vec::new(),
            own_lists: Vec::new(),
            playlists: load_json("playlists.json"),
            pl_open: None,
            resume_at: None,
            session_loaded: false,
            session_t: None,
            pages: Vec::new(),
            page_busy: false,
            q_reorder: false,
            q_drag: None,
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
            trainer_keys: false,
            keys_done: 0,
            wv_zoom: 1.0,
            wv_off: 0.0,
            wv_drag: None,
            sec_cur: None,
            sec_edit: None,
            sec_del: None,
            tuning: HashMap::new(),
            tuning_busy: false,
            exporting: false,
            wraps: 0,
            marks: load_json("bookmarks.json"),
            eq: st.eq,
            eq_on: st.eq_on,
            ch_tab: 0,
            cf_root: None,
            cf_fam: None,
            cf_neck: false,
            cf_voice: 0,
            cf_staff: false,
            my_grips: load_json("my_grips.json"),
            an_big: false,
            ln_kind: 0,
            ln_root: 0,
            ln_scale: 0,
            ln_cycle: 0,
            ln_start: 4,
            ln_finger: 2,
            ln_sheet: false,
            ln_end: None,
            ln_lower: 0,
            ln_upper: 0,
            ln_bpm: 100,
            ln_sub: 2,
            ln_loop: false,
            ln_page: 0,
            si_root: 0,
            si_fam: 0,
            si_chord: 4,
            si_scale: 0,
            si_any: false,
            si_kind: 0,
            si_kroot: 0,
            si_pick: 0,
            settings_open: false,
            sure_out: false,
            sure_clear: false,
            picker: None,
            picked: None,
            mt_more: false,
            mt_beat: None,
            file_lists: Vec::new(),
            files_busy: false,
            names: load_json("names.json"),
            edit: Default::default(),
            mt_scale: 1.0,
            mt_beat_busy: false,
            mt_beat_failed: Vec::new(),
            mt_laid: 0,
            mt_speed_seen: 100,
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
            tr_page: 0,
            tr_prog: "1 6 2 5".to_string(),
            tr_pos: 1,
            tr_bass: 0,
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
        if self.cur != Some(i) {
            self.resume_at = None;
        }
        let Some(t) = self.queue.get(i).cloned() else { return };
        self.recent.retain(|r| r.id != t.id || r.title != t.title);
        self.recent.insert(0, t.clone());
        self.recent.truncate(30);
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
                match cache::get(t.id) {
                    Some(b) => Ok(b),
                    None => {
                        let got = yt_audio(&vid);
                        if let Ok(b) = &got {
                            let _ = cache::put(t.id, b);
                        }
                        got
                    }
                }
            } else if let Some(p) = file {
                std::fs::read(&p).map_err(|e| e.to_string())
            } else if let Some(b) = cache::get(t.id) {
                Ok(b)
            } else {
                let got = api.stream_url(t.id, false).and_then(|u| api.fetch(&u));
                if let Ok(b) = &got {
                    cache::know(std::slice::from_ref(&t));
                    let _ = cache::put(t.id, b);
                }
                got
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
            self.player.send(Cmd::StopOnce);
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
                Msg::OwnLists(r) => match r {
                    Ok(l) => self.own_lists = l,
                    Err(e) => self.note = format!("Couldn't get your playlists: {}", e),
                },
                Msg::Page(r) => {
                    self.page_busy = false;
                    match r {
                        Ok(p) => {
                            cache::know(&p.tracks);
                            self.pages.push(p);
                            self.page_cards.push(self.page_pending.take());
                        }
                        Err(e) => self.note = format!("Couldn't open it: {}", e),
                    }
                }
                Msg::Library(r) => {
                    self.busy = false;
                    match r {
                        Ok(p) if p.title == "home" => self.home = Some(p),
                        Ok(p) => self.lib_page = Some(p),
                        Err(e) => self.note = format!("Couldn't load it: {}", e),
                    }
                }
                Msg::Credits(r) => match r {
                    Ok(c) if !c.is_empty() => self.credits = c,
                    Ok(_) => self.credits = vec![("Credits".to_string(), "None listed".to_string())],
                    Err(e) => self.credits = vec![("Couldn't get them".to_string(), e)],
                },
                Msg::Done(r) => {
                    self.note = match r {
                        Ok(s) => s,
                        Err(e) => format!("That didn't work: {}", e),
                    }
                }
                Msg::Beats(id, r) => {
                    self.mt_beat_busy = false;
                    match r {
                        Some(b) => {
                            self.mt_beat = Some((id, b));
                            self.mt_scale = 1.0;
                            self.mt_laid = 0;
                        }
                        None => {
                            self.mt_beat_failed.push(id);
                            self.note = "Couldn't find a steady beat in this one".to_string();
                        }
                    }
                }
                Msg::Files(mut songs, lists) => {
                    self.files_busy = false;
                    for s in songs.iter_mut() {
                        self.renamed(&mut s.0);
                    }
                    self.files = songs;
                    self.file_lists = lists;
                }
                Msg::Tuning(id, r) => {
                    self.tuning_busy = false;
                    match r {
                        Some((c, _)) => {
                            self.tuning.insert(id, c);
                            self.note = if c.abs() < 5 { "It's at A440".to_string() } else { format!("{:+} cents from A440", c) };
                        }
                        None => self.note = "Couldn't hear the tuning in this one".to_string(),
                    }
                }
                Msg::Exported(r) => {
                    self.exporting = false;
                    self.note = match r {
                        Ok(p) => format!("Saved {}", p),
                        Err(e) => format!("Couldn't save it: {}", e),
                    };
                }
                Msg::Found(r) => {
                    self.searching = false;
                    match r {
                        Ok(p) => {
                            cache::know(&p.tracks);
                            self.results = p.tracks.clone();
                            self.result_rows = p.rows.clone();
                            self.search_cache.insert(self.search_q.clone(), p);
                        }
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
                    if self.page_cover.as_ref().is_some_and(|c| c.0 == url) {
                        let tex = img.clone().map(|i| self.ctx.load_texture(&url, i, egui::TextureOptions::LINEAR));
                        self.page_cover = Some((url.clone(), tex));
                    }
                    if self.cover.as_ref().is_some_and(|c| c.0 == url) {
                        let tex = img.map(|i| self.ctx.load_texture(&url, i, egui::TextureOptions::LINEAR));
                        self.cover = Some((url, tex));
                    }
                }
                Msg::Meta(id, info) => {
                    meta::set_detected(id, info);
                    save_json("meta.json", &meta::saved());
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
                            if let Some(p) = self.resume_at.take() {
                                self.player.send(Cmd::Seek(p));
                            }
                            self.save_session();
                            // the next Tidal song in line is fetched ahead and kept, so it starts at once
                            let next = self.cur.and_then(|i| self.queue.get(i + 1)).cloned();
                            if let Some(n) = next.filter(|n| n.id > 0 && !cache::has(n.id) && !self.files.iter().any(|f| f.0.id == n.id)) {
                                self.spawn(move |api| {
                                    if let Ok(b) = api.stream_url(n.id, false).and_then(|u| api.fetch(&u)) {
                                        cache::know(std::slice::from_ref(&n));
                                        let _ = cache::put(n.id, &b);
                                    }
                                    Msg::Audio(u64::MAX, Err(String::new()))
                                });
                            }
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
        self.metro_track();
        // the sleep timer: fade out over the last 20 seconds, then pause (the volume comes back for next time)
        if let Some(at) = self.sleep_at {
            let left = at.saturating_duration_since(Instant::now()).as_secs_f32();
            if left <= 0.0 {
                self.sleep_at = None;
                if !self.paused {
                    self.play_pause();
                }
                self.player.send(Cmd::Volume(self.volume * self.volume));
                self.set_note("Sleep timer done");
            } else if left < 20.0 {
                let k = left / 20.0;
                self.player.send(Cmd::Volume(self.volume * self.volume * k));
            }
        }
        if !self.paused && !self.stopped && self.session_t.is_none_or(|t| t.elapsed().as_secs() >= 15) {
            self.session_t = Some(Instant::now());
            if self.cur.is_some() {
                self.save_session();
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
                        if self.trainer_keys {
                            // up a 4th each time, kept within half an octave either way; home after 12
                            self.keys_done += 1;
                            if self.keys_done >= 12 {
                                self.keys_done = 0;
                                self.trainer = false;
                                self.cents = 0;
                                self.set_note("All 12 keys done. Nice work");
                            } else {
                                let mut s = self.cents + 500;
                                if s > 600 {
                                    s -= 1200;
                                }
                                self.cents = s;
                                self.set_note(&format!("Up a 4th: key {} of 12", self.keys_done + 1));
                            }
                            self.sync_player();
                        } else if self.speed < 100 {
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
            ptext(ui.painter(), Pos2::new(r.min.x + 34.0, r.center().y), Align::Min, "Backline", 2.0, pal().trim);
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let k = SKIN.load(Ordering::Relaxed);
                if pill(ui, "Skin", false).on_hover_text(SKIN_NAMES[k]).clicked() {
                    SKIN.store((k + 1) % PALS.len(), Ordering::Relaxed);
                    self.wear();
                    self.save_settings();
                }
            });
        });
    }

    /// The screens along the bottom: a line icon and a word for each, the current one lit.
    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        let items = ["Music", "Play", "Practice", "Chords", "Lines"];
        let tabs = [Tab::Library, Tab::Player, Tab::Practice, Tab::Chords, Tab::Lines];
        let cur = tabs.iter().position(|t| *t == self.tab || (*t == Tab::Practice && self.tab == Tab::Metro));
        let w = ui.available_width() / 5.0;
        let (bar, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::hover());
        for (k, name) in items.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(bar.min.x + k as f32 * w, bar.min.y), Vec2::new(w, 44.0));
            let resp = ui.interact(r, ui.id().with(("tabbar", k)), Sense::click());
            let on = cur == Some(k);
            let col = if on { pal().ink } else { pal().dim };
            tab_icon(ui.painter(), Pos2::new(r.center().x, r.min.y + 14.0), k, if on { pal().red } else { col });
            ptext(ui.painter(), Pos2::new(r.center().x, r.min.y + 35.0), Align::Center, name, 1.2, col);
            if resp.clicked() {
                if tabs[k] == Tab::Practice && !self.practice {
                    self.practice = true;
                    self.sync_player();
                    self.save_settings();
                }
                self.tab = tabs[k];
            }
        }
    }

    /// While browsing: the song playing, in a plain bar above the tabs (tap it for Now Playing).
    fn now_bar(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.cur_track().cloned() else { return };
        self.want_cover(&t);
        let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
        let p = ui.painter();
        p.rect_filled(r, 8.0, pal().beige_lt);
        let art = Rect::from_min_size(r.min + Vec2::new(6.0, 6.0), Vec2::splat(34.0));
        match self.cover.as_ref().and_then(|c| c.1.as_ref()) {
            Some(tex) => {
                let mut img = egui::epaint::RectShape::filled(art, 6.0, Color32::WHITE);
                img.fill_texture_id = tex.id();
                img.uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                p.add(img);
            }
            None => {
                p.rect_filled(art, 6.0, pal().row_sel);
                tab_icon(p, art.center(), 0, pal().bar_txt);
            }
        }
        let tx = art.max.x + 8.0;
        let tw = r.max.x - 44.0 - tx;
        ptext_fit(p, Pos2::new(tx, r.min.y + 16.0), Align::Min, &t.title, 1.4, tw, pal().ink);
        ptext_fit(p, Pos2::new(tx, r.min.y + 31.0), Align::Min, &t.artists_text(), 1.2, tw, pal().dim);
        // how far in, as a thin line along the foot
        let len = self.cur_track().map_or(0.0, |t| t.duration as f32);
        if len > 0.0 {
            let k = (self.pos() / len).clamp(0.0, 1.0);
            p.rect_filled(Rect::from_min_max(Pos2::new(r.min.x + 4.0, r.max.y - 2.0), Pos2::new(r.min.x + 4.0 + (r.width() - 8.0) * k, r.max.y)), 1.0, pal().red);
        }
        let pr = Rect::from_center_size(Pos2::new(r.max.x - 22.0, r.center().y), Vec2::splat(36.0));
        let pause = !(self.paused || self.stopped);
        play_glyph(p, pr.center(), pause, pal().ink);
        let pp = ui.interact(pr, ui.id().with("np_play"), Sense::click());
        if pp.clicked() {
            self.play_pause();
        } else if resp.clicked() {
            self.tab = Tab::Now;
        }
    }

    /// NOW PLAYING: the cover big, the title, a slim LCD strip (time and spectrum, the one place for the pixel
    /// font), the seek bar, the transport, and the way to practice or the metronome.
    fn now_screen(&mut self, ui: &mut egui::Ui, r: Rect) {
        let r = r.shrink2(Vec2::new(16.0, 0.0));
        let back = Rect::from_min_size(r.min + Vec2::new(-6.0, 2.0), Vec2::splat(36.0));
        chevron(ui.painter(), back.center(), pal().trim);
        ptext(ui.painter(), Pos2::new(r.center().x, back.center().y), Align::Center, "Now playing", 1.3, pal().trim);
        if ui.interact(back, ui.id().with("now_back"), Sense::click()).clicked() {
            self.tab = Tab::Player;
        }
        let Some(t) = self.cur_track().cloned() else {
            ptext(ui.painter(), r.center(), Align::Center, "Nothing playing. Pick a song in Music.", 1.4, pal().trim);
            return;
        };
        // the cover
        let side = r.width().min(r.height() - 430.0).max(110.0);
        let cv = Rect::from_min_size(Pos2::new(r.center().x - side / 2.0, back.max.y + 10.0), Vec2::splat(side));
        self.want_cover(&t);
        let p = ui.painter().clone();
        match self.cover.as_ref().and_then(|c| c.1.as_ref()) {
            Some(tex) => {
                let mut img = egui::epaint::RectShape::filled(cv, 10.0, Color32::WHITE);
                img.fill_texture_id = tex.id();
                img.uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                p.add(img);
            }
            None => {
                p.rect_filled(cv, 10.0, pal().row_sel);
                tab_icon(&p, cv.center(), 0, pal().bar_txt);
            }
        }
        let mut y = cv.max.y + 22.0;
        ptext_fit(&p, Pos2::new(r.min.x, y), Align::Min, &t.title, 2.0, r.width(), pal().bar_txt);
        y += 22.0;
        ptext_fit(&p, Pos2::new(r.min.x, y), Align::Min, &t.artists_text(), 1.4, r.width(), pal().trim);
        y += 18.0;
        // the LCD: the song scrolling by in the pixel font, and the spectrum the full width under it
        let lcd = Rect::from_min_size(Pos2::new(r.min.x, y), Vec2::new(r.width(), 70.0));
        p.rect_filled(lcd, 8.0, pal().lcd);
        let pos = self.pos();
        font::set_modern(false);
        let name = Rect::from_min_size(lcd.min + Vec2::new(10.0, 6.0), Vec2::new(lcd.width() - 20.0, 18.0));
        // (always moving, Winamp's way, even when the name would fit)
        let txt = format!("{} - {}  ***  ", t.artists_text(), t.title);
        let clip = p.with_clip_rect(name);
        let period = text_w(&txt, 1.2).max(name.width());
        let off = (ui.input(|i| i.time) as f32 * 24.0) % period;
        for k in 0..2 {
            ptext(&clip, Pos2::new(name.min.x - off + k as f32 * period, name.center().y), Align::Min, &txt, 1.2, pal().ink);
        }
        font::set_modern(true);
        let sp = Rect::from_min_max(Pos2::new(lcd.min.x + 10.0, name.max.y + 4.0), lcd.max - Vec2::new(10.0, 6.0));
        viz::viz_draw(&p, sp, &self.bands, &self.peaks, &self.viz_wave, 0, 1.0, false, 1.0, None);
        y = lcd.max.y + 16.0;
        // the seek bar: drag or tap to move
        let len = t.duration.max(1.0);
        let sk = Rect::from_min_size(Pos2::new(r.min.x, y), Vec2::new(r.width(), 18.0));
        let sr = ui.interact(sk, ui.id().with("now_seek"), Sense::click_and_drag());
        if let Some(at) = sr.interact_pointer_pos() {
            if sr.clicked() || sr.drag_stopped() {
                let k = ((at.x - sk.min.x) / sk.width()).clamp(0.0, 1.0);
                self.player.send(Cmd::Seek(k * len));
            }
        }
        let k = (pos / len).clamp(0.0, 1.0);
        let line = Rect::from_center_size(sk.center(), Vec2::new(sk.width(), 4.0));
        p.rect_filled(line, 2.0, pal().beige_dk);
        p.rect_filled(Rect::from_min_max(line.min, Pos2::new(line.min.x + line.width() * k, line.max.y)), 2.0, pal().red);
        p.circle_filled(Pos2::new(line.min.x + line.width() * k, line.center().y), 6.0, pal().bar_txt);
        let left = len - pos;
        ptext(&p, Pos2::new(r.min.x, sk.max.y + 8.0), Align::Min, &format!("{}:{:02}", pos as u32 / 60, pos as u32 % 60), 1.1, pal().trim);
        ptext(&p, Pos2::new(r.max.x, sk.max.y + 8.0), Align::Max, &format!("-{}:{:02}", left as u32 / 60, left as u32 % 60), 1.1, pal().trim);
        y = sk.max.y + 26.0;
        // the transport: shuffle, back, play, on, repeat
        let mid = Pos2::new(r.center().x, y + 32.0);
        let play = Rect::from_center_size(mid, Vec2::splat(64.0));
        p.circle_filled(mid, 32.0, pal().red);
        play_glyph(&p, mid, !(self.paused || self.stopped), pal().bar_txt);
        if ui.interact(play, ui.id().with("now_play"), Sense::click()).clicked() {
            self.play_pause();
        }
        for (dx, fwd) in [(-72.0, false), (72.0, true)] {
            let c = mid + Vec2::new(dx, 0.0);
            skip_glyph(&p, c, fwd, pal().bar_txt);
            if ui.interact(Rect::from_center_size(c, Vec2::splat(44.0)), ui.id().with(("now_skip", fwd)), Sense::click()).clicked() {
                if fwd {
                    self.next();
                } else {
                    self.prev();
                }
            }
        }
        let sh = mid + Vec2::new(-136.0, 0.0);
        shuffle_glyph(&p, sh, if self.shuffle { pal().red } else { pal().trim });
        if ui.interact(Rect::from_center_size(sh, Vec2::splat(44.0)), ui.id().with("now_shuf"), Sense::click()).clicked() {
            self.shuffle = !self.shuffle;
            self.set_note(if self.shuffle { "Shuffle on" } else { "Shuffle off" });
        }
        let rp = mid + Vec2::new(136.0, 0.0);
        loop_glyph(&p, rp, if self.repeat != Repeat::Off { pal().red } else { pal().trim });
        if self.repeat == Repeat::One {
            ptext(&p, rp + Vec2::new(0.0, 1.0), Align::Center, "1", 0.8, pal().red);
        }
        if ui.interact(Rect::from_center_size(rp, Vec2::splat(44.0)), ui.id().with("now_rep"), Sense::click()).clicked() {
            self.repeat = match self.repeat {
                Repeat::Off => Repeat::All,
                Repeat::All => Repeat::One,
                Repeat::One => Repeat::Off,
            };
            self.set_note(match self.repeat {
                Repeat::Off => "Repeat off",
                Repeat::All => "Repeat the queue",
                Repeat::One => "Repeat this song",
            });
        }
        // like, speed, sleep, and the volume
        let row = play.max.y + 14.0;
        let chip = |ui: &mut egui::Ui, x: f32, w: f32, id: &str, on: bool| -> (Rect, bool) {
            let cr = Rect::from_min_size(Pos2::new(x, row), Vec2::new(w, 34.0));
            let resp = ui.interact(cr, ui.id().with(id), Sense::click());
            ui.painter().rect_filled(cr, 17.0, if on || resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige });
            (cr, resp.clicked())
        };
        let mut x = r.min.x;
        let tidal = t.id > 0 && !self.files.iter().any(|f| f.0.id == t.id) && !self.yt.iter().any(|y| yt_id(&y.vid) == t.id);
        if tidal && self.logged_in {
            let liked = self.liked_ids.contains(&t.id);
            let (cr, hit) = chip(ui, x, 44.0, "now_like", false);
            src_icon(&p, cr.center(), 4, if liked { pal().red } else { pal().ink });
            if hit {
                self.set_like(&t, !liked);
            }
            x += 52.0;
        }
        let s_txt = {
            let v = format!("{:.2}", self.speed as f32 / 100.0);
            format!("{}x", v.trim_end_matches('0').trim_end_matches('.'))
        };
        let (cr, hit) = chip(ui, x, 58.0, "now_speed", self.speed != 100);
        ptext(&p, cr.center(), Align::Center, &s_txt, 1.3, pal().ink);
        if hit {
            self.speed = SPEEDS.iter().copied().find(|&s| s > self.speed).unwrap_or(SPEEDS[0]);
            self.sync_player();
        }
        x += 66.0;
        let sleeping = self.sleep_at.map(|s| s.saturating_duration_since(Instant::now()).as_secs().div_ceil(60));
        let (cr, hit) = chip(ui, x, 76.0, "now_sleep", sleeping.is_some());
        ptext(&p, cr.center(), Align::Center, &sleeping.map_or("Sleep".to_string(), |m| format!("{} min", m)), 1.3, pal().ink);
        if hit {
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
            self.player.send(Cmd::Volume(self.volume * self.volume));
            self.set_note(&match next {
                Some(m) => format!("Sleep in {} min", m),
                None => "Sleep timer off".to_string(),
            });
        }
        x += 84.0;
        // the volume: drag along it
        let vr = Rect::from_min_max(Pos2::new(x + 4.0, row), Pos2::new(r.max.x, row + 34.0));
        let vresp = ui.interact(vr, ui.id().with("now_vol"), Sense::click_and_drag());
        if let Some(pp) = vresp.interact_pointer_pos().filter(|_| vresp.dragged() || vresp.clicked()) {
            self.set_volume((pp.x - vr.min.x) / vr.width());
        }
        let vl = Rect::from_center_size(vr.center(), Vec2::new(vr.width(), 4.0));
        p.rect_filled(vl, 2.0, pal().beige_dk);
        p.rect_filled(Rect::from_min_max(vl.min, Pos2::new(vl.min.x + vl.width() * self.volume, vl.max.y)), 2.0, pal().trim);
        p.circle_filled(Pos2::new(vl.min.x + vl.width() * self.volume, vl.center().y), 7.0, pal().bar_txt);
        // Practice and Metronome: two even buttons, an icon and a word each
        let play = Rect::from_min_max(play.min, Pos2::new(play.max.x, row + 34.0));
        let bw = (r.width() - 10.0) / 2.0;
        for (k, (name, icon)) in [("Practice", 2), ("Metronome", 9)].iter().enumerate() {
            let br = Rect::from_min_size(Pos2::new(r.min.x + k as f32 * (bw + 10.0), play.max.y + 14.0), Vec2::new(bw, 40.0));
            let resp = ui.interact(br, ui.id().with(("now_btn", k)), Sense::click());
            p.rect_filled(br, 20.0, if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige });
            let c = Pos2::new(br.center().x - text_w(name, 1.4) / 2.0 - 4.0, br.center().y);
            if *icon == 9 {
                metro_glyph(&p, c + Vec2::new(-8.0, 0.0), pal().ink);
            } else {
                tab_icon(&p, c + Vec2::new(-8.0, 0.0), *icon, pal().ink);
            }
            ptext(&p, Pos2::new(c.x + 8.0, br.center().y), Align::Min, name, 1.4, pal().ink);
            if resp.clicked() {
                if k == 0 {
                    if !self.practice {
                        self.practice = true;
                        self.sync_player();
                        self.save_settings();
                    }
                    self.tab = Tab::Practice;
                } else {
                    self.tab = Tab::Metro;
                }
            }
        }
    }

    /// A screen: its title at the top with a gear (the skin) at the right, then the flat panel. Music has no
    /// title (its switch is its heading); the gear sits at the top right all the same.
    fn screen(&mut self, ui: &mut egui::Ui, r: Rect, title: &str, body: impl FnOnce(&mut Self, &mut egui::Ui)) {
        let head = Rect::from_min_size(r.min, Vec2::new(r.width(), if title.is_empty() { 0.0 } else { 40.0 }));
        if !title.is_empty() {
            ptext(ui.painter(), Pos2::new(head.min.x + 14.0, head.center().y), Align::Min, title, 2.4, pal().bar_txt);
            let g = Pos2::new(head.max.x - 26.0, head.center().y);
            gear_glyph(ui.painter(), g, pal().trim);
            if ui.interact(Rect::from_center_size(g, Vec2::splat(40.0)), ui.id().with("gear"), Sense::click()).clicked() {
                self.settings_open = !self.settings_open;
            }
        }
        let rest = Rect::from_min_max(Pos2::new(r.min.x, head.max.y), r.max);
        if self.settings_open {
            self.flat(ui, rest, |me, ui| me.settings_ui(ui));
        } else {
            self.flat(ui, rest, body);
        }
    }

    /// SETTINGS (the gear): the skin as colour swatches; Tidal (signed in, sign out) and the songs kept on the phone
    /// (how much room they take, clear them).
    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let cur = SKIN.load(Ordering::Relaxed);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            dim(ui, "Skin", 1.0, pal().ink2);
            let w = ui.available_width();
            let tw = (w - 8.0) / 2.0;
            let (area, _) = ui.allocate_exact_size(Vec2::new(w, 2.0 * 64.0 + 8.0), Sense::hover());
            for (n, k) in [0usize, 1, 2, 9].iter().enumerate() {
                let r = Rect::from_min_size(area.min + Vec2::new((n % 2) as f32 * (tw + 8.0), (n / 2) as f32 * 72.0), Vec2::new(tw, 64.0));
                let resp = ui.interact(r, ui.id().with(("swatch", k)), Sense::click());
                let sk = &PALS[*k];
                ui.painter().rect_filled(r, 10.0, sk.beige);
                ui.painter().rect_filled(Rect::from_min_size(r.min + Vec2::new(10.0, 10.0), Vec2::new(26.0, 16.0)), 4.0, sk.lcd);
                ui.painter().circle_filled(r.min + Vec2::new(50.0, 18.0), 7.0, sk.red);
                ptext(ui.painter(), Pos2::new(r.min.x + 12.0, r.max.y - 16.0), Align::Min, &crate::lines_ui::nice(SKIN_NAMES[*k]), 1.3, sk.ink);
                if *k == cur {
                    ui.painter().rect_stroke(r.expand(2.0), 12.0, Stroke::new(2.5, pal().red));
                }
                if resp.clicked() {
                    SKIN.store(*k, Ordering::Relaxed);
                    self.wear();
                    self.save_settings();
                }
            }
            ui.add_space(12.0);
            dim(ui, "Tidal", 1.0, pal().ink2);
            let row = |ui: &mut egui::Ui, left: &str, right: &str| -> bool {
                let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::click());
                ui.painter().rect_filled(r.shrink2(Vec2::new(0.0, 1.0)), 6.0, if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige_lt });
                ptext(ui.painter(), Pos2::new(r.min.x + 12.0, r.center().y), Align::Min, left, 1.3, pal().ink);
                ptext(ui.painter(), Pos2::new(r.max.x - 12.0, r.center().y), Align::Max, right, 1.2, pal().ink2);
                resp.clicked()
            };
            if self.logged_in {
                if row(ui, "Signed in", if self.sure_out { "Sure? Tap again" } else { "Sign out" }) {
                    if self.sure_out {
                        self.api.logout();
                        self.logged_in = false;
                        self.liked.clear();
                        self.liked_ids.clear();
                        self.lists.clear();
                        self.home = None;
                        self.lib_page = None;
                        self.sure_out = false;
                        self.set_note("Signed out of Tidal");
                    } else {
                        self.sure_out = true;
                    }
                }
            } else if row(ui, "Not signed in", "Sign in in Music") {
                self.settings_open = false;
                self.tab = Tab::Library;
                self.lib_tab = 3;
            }
            let (n, bytes) = cache::stats();
            if row(ui, &format!("Kept songs: {}", n), &if self.sure_clear { "Sure? Tap again".to_string() } else { format!("{}  ·  Clear", cache::fmt_size(bytes)) }) && n > 0 {
                if self.sure_clear {
                    cache::clear();
                    self.sure_clear = false;
                    self.set_note("Kept songs cleared");
                } else {
                    self.sure_clear = true;
                }
            }
            ui.add_space(12.0);
            if pill(ui, "Done", false).clicked() {
                self.settings_open = false;
                self.sure_out = false;
                self.sure_clear = false;
            }
        });
    }

    /// A small choice chip (its value and a little arrow); tapping it opens its choices as a list from the bottom.
    /// Returns the choice made, the frame after it's made.
    pub(crate) fn pick_chip(&mut self, ui: &mut egui::Ui, id: &str, items: &[&str], cur: usize) -> Option<usize> {
        if let Some((pid, i)) = self.picked.take() {
            if pid == id {
                return Some(i);
            }
            self.picked = Some((pid, i));
        }
        let label = items.get(cur).copied().unwrap_or("");
        let w = text_w(label, 1.2) + 38.0;
        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 32.0), Sense::click());
        ui.painter().rect_filled(r, 16.0, if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige_lt });
        ptext(ui.painter(), Pos2::new(r.min.x + 13.0, r.center().y), Align::Min, label, 1.2, pal().ink);
        // a small down arrow
        let c = Pos2::new(r.max.x - 14.0, r.center().y);
        let s = Stroke::new(1.6, pal().ink2);
        ui.painter().line_segment([c + Vec2::new(-4.0, -2.0), c + Vec2::new(0.0, 2.5)], s);
        ui.painter().line_segment([c + Vec2::new(0.0, 2.5), c + Vec2::new(4.0, -2.0)], s);
        if resp.clicked() {
            self.picker = Some((id.to_string(), items.iter().map(|s| s.to_string()).collect(), cur));
        }
        None
    }

    /// The open pick list: the choices as rows sliding up from the bottom; tap one to choose it, outside to close.
    fn picker_ui(&mut self, ctx: &egui::Context) {
        let Some((id, items, cur)) = self.picker.clone() else { return };
        let screen = ctx.screen_rect();
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("pick_dim"))).rect_filled(screen, 0.0, Color32::from_black_alpha(150));
        let mut close = ctx.input(|i| i.pointer.any_click()) && !ctx.is_pointer_over_area();
        let mut chose = None;
        egui::Area::new(egui::Id::new("picker")).order(egui::Order::Tooltip).anchor(egui::Align2::CENTER_BOTTOM, Vec2::ZERO).show(ctx, |ui| {
            egui::Frame::none().fill(pal().beige).rounding(egui::Rounding { nw: 16.0, ne: 16.0, sw: 0.0, se: 0.0 }).inner_margin(egui::Margin { left: 14.0, right: 14.0, top: 14.0, bottom: 34.0 }).show(ui, |ui| {
                ui.set_width(screen.width() - 28.0);
                egui::ScrollArea::vertical().id_salt("pick_scroll").max_height(screen.height() * 0.6).show(ui, |ui| {
                    for (k, it) in items.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::click());
                        if k == cur {
                            ui.painter().rect_filled(r, 8.0, pal().row_sel);
                        } else if resp.is_pointer_button_down_on() {
                            ui.painter().rect_filled(r, 8.0, pal().beige_lt);
                        }
                        ptext(ui.painter(), Pos2::new(r.min.x + 12.0, r.center().y), Align::Min, it, 1.4, if k == cur { pal().bar_txt } else { pal().ink });
                        if resp.clicked() {
                            chose = Some(k);
                        }
                    }
                });
            });
        });
        if let Some(k) = chose {
            self.picked = Some((id, k));
            close = true;
        }
        if close {
            self.picker = None;
        }
    }

    /// A screen in the flat look: the skin's panel colour, rounded, its body padded inside.
    fn flat(&mut self, ui: &mut egui::Ui, outer: Rect, body: impl FnOnce(&mut Self, &mut egui::Ui)) {
        let inner = outer.shrink2(Vec2::new(6.0, 2.0));
        ui.painter().rect_filled(inner, 10.0, pal().beige);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner.shrink(10.0)).layout(egui::Layout::top_down(Align::Min)));
        child.set_clip_rect(inner);
        body(self, &mut child);
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
                ctx.set_zoom_factor(ctx.zoom_factor() * w / 414.0);
                self.zoom_set = true;
            }
        }
        font::set_ppp(ctx.pixels_per_point());
        font::set_modern(true);
        if !self.session_loaded {
            self.session_loaded = true;
            self.load_session();
        }
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
        // (and after a pause, until the spectrum has fallen to nothing)
        let falling = self.bands.iter().chain(self.peaks.iter()).any(|v| *v > 0.001);
        if !self.note.is_empty() || (self.cur.is_some() && !self.paused) || self.mt.on || falling {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
        let fill = egui::Frame::none().fill(pal().app_bg);
        // no top bar: just the room the phone's status bar needs
        egui::TopBottomPanel::top("top").frame(fill.inner_margin(egui::Margin { left: 8.0, right: 8.0, top: 30.0, bottom: 0.0 })).show(ctx, |_| {});
        let now = self.tab == Tab::Now;
        egui::TopBottomPanel::bottom("bottom")
            .frame(fill.inner_margin(egui::Margin { left: 8.0, right: 8.0, top: 6.0, bottom: 26.0 }))
            .show(ctx, |ui| {
                if !self.note.is_empty() {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bh()), Sense::click());
                    inset(ui.painter(), r, pal().lcd);
                    let n = self.note.clone();
                    let up = n.to_uppercase();
                    let bad = up.contains("FAILED") || up.contains("CAN'T") || up.contains("COULDN'T") || up.contains("DIDN'T WORK") || up.starts_with("YOUTUBE:");
                    marquee(ui, r.shrink2(Vec2::new(4.0, 0.0)), &n, 1.0, if bad { pal().red } else { pal().ink });
                    if resp.clicked() {
                        self.note.clear();
                    }
                    ui.add_space(4.0);
                }
                if !now {
                    self.now_bar(ui);
                    ui.add_space(4.0);
                    self.bottom_bar(ui);
                }
            });
        self.song_menu_ui(ctx);
        self.picker_ui(ctx);
        egui::CentralPanel::default().frame(fill).show(ctx, |ui| {
            let r = ui.max_rect();
            match self.tab {
                Tab::Player => self.screen(ui, r, "Queue", |me, ui| me.queue_ui(ui)),
                Tab::Library => self.screen(ui, r, "", |me, ui| me.library_ui(ui)),
                Tab::Practice => self.screen(ui, r, "Practice", |me, ui| me.practice_ui(ui)),
                Tab::Chords => self.screen(ui, r, "Chords", |me, ui| me.chords_ui(ui)),
                Tab::Lines => self.screen(ui, r, "Lines", |me, ui| me.lines_ui(ui)),
                Tab::Metro => self.screen(ui, r, "Metronome", |me, ui| me.metro_ui(ui)),
                Tab::Now => self.now_screen(ui, r),
            }
        });
    }
}

/// The bottom bar's line icons: Music (a note), Play (a list), Practice (a wave), Chords (a chord box), Lines
/// (strings with dots).
fn tab_icon(p: &egui::Painter, c: Pos2, k: usize, col: Color32) {
    let s = Stroke::new(1.6, col);
    let at = |x: f32, y: f32| c + Vec2::new(x, y);
    match k {
        0 => {
            let head: Vec<Pos2> = (0..16)
                .map(|i| {
                    let a = i as f32 / 16.0 * std::f32::consts::TAU;
                    let (x, y) = (4.6 * a.cos(), 3.3 * a.sin());
                    let r = -0.45f32;
                    at(-3.0 + x * r.cos() - y * r.sin(), 6.0 + x * r.sin() + y * r.cos())
                })
                .collect();
            p.add(egui::Shape::convex_polygon(head, col, Stroke::NONE));
            p.line_segment([at(1.2, 4.8), at(1.2, -9.0)], s);
            p.add(egui::Shape::line(vec![at(1.2, -9.0), at(5.5, -5.5), at(7.0, -1.0), at(5.0, 2.5)], s));
        }
        1 => {
            for y in [-6.0, 0.0, 6.0] {
                p.line_segment([at(-2.0, y), at(9.0, y)], s);
            }
            p.add(egui::Shape::convex_polygon(vec![at(-9.0, -7.0), at(-4.0, -4.0), at(-9.0, -1.0)], col, Stroke::NONE));
        }
        2 => {
            let pts = [(-10.0, 0.0), (-6.0, 0.0), (-4.0, -7.0), (-1.0, 7.0), (2.0, -4.0), (4.0, 2.0), (6.0, 0.0), (10.0, 0.0)];
            p.add(egui::Shape::line(pts.iter().map(|(x, y)| at(*x, *y)).collect(), s));
        }
        3 => {
            p.rect_stroke(Rect::from_center_size(c, Vec2::new(14.0, 17.0)), 1.0, s);
            for y in [-3.0, 2.5] {
                p.line_segment([at(-7.0, y), at(7.0, y)], s);
            }
            for x in [-2.3, 2.3] {
                p.line_segment([at(x, -8.5), at(x, 8.5)], s);
            }
        }
        _ => {
            for (i, y) in [-6.0, 0.0, 6.0].iter().enumerate() {
                p.line_segment([at(-10.0, *y), at(10.0, *y)], s);
                p.circle_filled(at([-5.0, 1.0, 5.0][i], *y), 2.4, col);
            }
        }
    }
}

/// The Music page's source icons: Tidal (a sound wave), files (a folder), YouTube (a video), my tracks (a
/// bookshelf), a heart.
pub(crate) fn src_icon(p: &egui::Painter, c: Pos2, k: usize, col: Color32) {
    let s = Stroke::new(2.0, col);
    let at = |x: f32, y: f32| c + Vec2::new(x, y);
    match k {
        0 => {
            for (x, h) in [(-9.0, 3.0), (-4.5, 8.0), (0.0, 12.0), (4.5, 6.0), (9.0, 9.0)] {
                p.line_segment([at(x, -h), at(x, h)], s);
            }
        }
        1 => {
            let pts = vec![at(-11.0, -7.0), at(-4.0, -7.0), at(-2.0, -4.0), at(11.0, -4.0), at(11.0, 8.0), at(-11.0, 8.0), at(-11.0, -7.0)];
            p.add(egui::Shape::line(pts, s));
        }
        2 => {
            // a video: a rounded screen with a play triangle in it
            p.rect_stroke(Rect::from_center_size(c, Vec2::new(26.0, 18.0)), 5.0, s);
            p.add(egui::Shape::convex_polygon(vec![at(-3.5, -5.0), at(5.5, 0.0), at(-3.5, 5.0)], col, Stroke::NONE));
        }
        3 => {
            // a bookshelf: three books standing and one leaning, on a shelf
            p.line_segment([at(-12.0, 9.0), at(12.0, 9.0)], s);
            p.rect_stroke(Rect::from_min_max(at(-10.0, -8.0), at(-5.0, 8.0)), 1.0, s);
            p.rect_stroke(Rect::from_min_max(at(-4.0, -5.0), at(1.0, 8.0)), 1.0, s);
            p.add(egui::Shape::line(vec![at(3.0, 8.0), at(7.0, -7.0), at(11.0, -6.0), at(7.0, 8.0)], s));
        }
        _ => {
            let mut pts = Vec::new();
            for i in 0..=40 {
                let a = i as f32 / 40.0 * std::f32::consts::TAU;
                let (x, y) = (16.0 * a.sin().powi(3), -(13.0 * a.cos() - 5.0 * (2.0 * a).cos() - 2.0 * (3.0 * a).cos() - (4.0 * a).cos()));
                pts.push(at(x * 0.62, y * 0.62 - 1.0));
            }
            p.add(egui::Shape::line(pts, s));
        }
    }
}

/// The long neck, Oolimo's way: the strings down the screen (low E at the left), frets 0 to 15, the inlays, the
/// fret numbers at the left. The lit voicing's notes are named by their role in the chord; the chord's other notes
/// show faint all the way up.
pub(crate) fn long_neck(p: &egui::Painter, r: Rect, c: &theory::ChordNotes, lit: Option<&[Option<u8>; 6]>) {
    const OPEN: [i32; 6] = [40, 45, 50, 55, 59, 64];
    let frets = 15;
    let x0 = r.min.x + 26.0;
    let sw = (r.width() - 40.0) / 5.0;
    let y0 = r.min.y + 18.0;
    let fh = (r.height() - 24.0) / frets as f32;
    p.rect_filled(r, 10.0, pal().beige_lt);
    let line = pal().beige_dk;
    for f in [3, 5, 7, 9, 15] {
        p.circle_filled(Pos2::new(x0 + 2.5 * sw, y0 + (f as f32 - 0.5) * fh), 3.5, line);
    }
    for x in [1.5, 3.5] {
        p.circle_filled(Pos2::new(x0 + x * sw, y0 + 11.5 * fh), 3.5, line);
    }
    for f in 0..=frets {
        let y = y0 + f as f32 * fh;
        p.line_segment([Pos2::new(x0, y), Pos2::new(x0 + 5.0 * sw, y)], Stroke::new(if f == 0 { 4.0 } else { 1.2 }, line));
        if f > 0 {
            ptext(p, Pos2::new(r.min.x + 12.0, y - fh / 2.0), Align::Center, &f.to_string(), 0.9, pal().dim);
        }
    }
    for s in 0..6 {
        let x = x0 + s as f32 * sw;
        p.line_segment([Pos2::new(x, y0), Pos2::new(x, y0 + frets as f32 * fh)], Stroke::new(1.8 - s as f32 * 0.2, pal().ink2));
    }
    let rad = (sw * 0.36).min(fh * 0.42).min(11.0);
    let dot = |s: usize, f: u8, strong: bool| {
        let x = x0 + s as f32 * sw;
        let y = if f == 0 { y0 - 9.0 } else { y0 + (f as f32 - 0.5) * fh };
        let pc = (OPEN[s] + f as i32).rem_euclid(12);
        let (deg, col) = degree_in(pc, c.root, &c.pcs);
        if strong {
            if pc == c.root.rem_euclid(12) {
                p.rect_filled(Rect::from_center_size(Pos2::new(x, y), Vec2::splat(rad * 2.0)), 3.0, col);
            } else {
                p.circle_filled(Pos2::new(x, y), rad, col);
            }
            ptext(p, Pos2::new(x, y), Align::Center, deg, 1.0, Color32::from_gray(15));
        } else {
            p.circle_filled(Pos2::new(x, y), rad * 0.45, col.gamma_multiply(0.5));
        }
    };
    for s in 0..6 {
        for f in 0..=frets as u8 {
            let pc = (OPEN[s] + f as i32).rem_euclid(12);
            let on_lit = lit.is_some_and(|g| g[s] == Some(f));
            if c.pcs.iter().any(|q| q.rem_euclid(12) == pc) && !on_lit {
                dot(s, f, false);
            }
        }
    }
    if let Some(g) = lit {
        for s in 0..6 {
            match g[s] {
                Some(f) => dot(s, f, true),
                None => {
                    let x = x0 + s as f32 * sw;
                    ptext(p, Pos2::new(x, y0 - 9.0), Align::Center, "x", 1.1, pal().red);
                }
            }
        }
    }
}

/// A pill switch of a few choices filling `r`, the current one filled; returns the one tapped.
pub(crate) fn seg_tabs(ui: &mut egui::Ui, r: Rect, items: &[&str], cur: usize, id: &str) -> Option<usize> {
    let p = ui.painter();
    p.rect_filled(r.shrink2(Vec2::new(0.0, 2.0)), r.height() / 2.0, pal().beige_lt);
    let w = (r.width() - 6.0) / items.len() as f32;
    let mut hit = None;
    for (k, name) in items.iter().enumerate() {
        let c = Rect::from_min_size(Pos2::new(r.min.x + 3.0 + k as f32 * w, r.min.y + 5.0), Vec2::new(w, r.height() - 10.0));
        let resp = ui.interact(c, ui.id().with((id, k)), Sense::click());
        if k == cur {
            ui.painter().rect_filled(c, c.height() / 2.0, pal().ink);
        }
        ptext_fit(ui.painter(), c.center(), Align::Center, name, 1.3, w - 8.0, if k == cur { pal().beige } else { pal().ink2 });
        if resp.clicked() {
            hit = Some(k);
        }
    }
    hit
}

/// The shuffle sign: two crossing paths with arrowheads.
pub(crate) fn shuffle_glyph(p: &egui::Painter, c: Pos2, col: Color32) {
    let s = Stroke::new(2.0, col);
    let at = |x: f32, y: f32| c + Vec2::new(x, y);
    p.add(egui::Shape::line(vec![at(-11.0, -6.0), at(-4.0, -6.0), at(4.0, 6.0), at(9.0, 6.0)], s));
    p.add(egui::Shape::line(vec![at(-11.0, 6.0), at(-4.0, 6.0), at(4.0, -6.0), at(9.0, -6.0)], s));
    for y in [-6.0, 6.0] {
        p.add(egui::Shape::convex_polygon(vec![at(8.0, y - 4.0), at(13.0, y), at(8.0, y + 4.0)], col, Stroke::NONE));
    }
}

/// The loop sign: two arrows chasing round.
pub(crate) fn loop_glyph(p: &egui::Painter, c: Pos2, col: Color32) {
    let s = Stroke::new(2.0, col);
    let at = |x: f32, y: f32| c + Vec2::new(x, y);
    p.add(egui::Shape::line(vec![at(-10.0, 4.0), at(-10.0, -6.0), at(8.0, -6.0)], s));
    p.add(egui::Shape::line(vec![at(10.0, -4.0), at(10.0, 6.0), at(-8.0, 6.0)], s));
    p.add(egui::Shape::convex_polygon(vec![at(8.0, -10.0), at(13.0, -6.0), at(8.0, -2.0)], col, Stroke::NONE));
    p.add(egui::Shape::convex_polygon(vec![at(-8.0, 2.0), at(-13.0, 6.0), at(-8.0, 10.0)], col, Stroke::NONE));
}

/// A smooth play triangle, or pause bars.
pub(crate) fn play_glyph(p: &egui::Painter, c: Pos2, pause: bool, col: Color32) {
    if pause {
        for x in [-4.0, 4.0] {
            p.rect_filled(Rect::from_center_size(c + Vec2::new(x, 0.0), Vec2::new(4.0, 14.0)), 1.0, col);
        }
    } else {
        p.add(egui::Shape::convex_polygon(vec![c + Vec2::new(-5.0, -8.0), c + Vec2::new(8.0, 0.0), c + Vec2::new(-5.0, 8.0)], col, Stroke::NONE));
    }
}

/// Skip back or on: a triangle and a bar.
pub(crate) fn skip_glyph(p: &egui::Painter, c: Pos2, fwd: bool, col: Color32) {
    let d = if fwd { 1.0 } else { -1.0 };
    p.add(egui::Shape::convex_polygon(vec![c + Vec2::new(-6.0 * d, -8.0), c + Vec2::new(5.0 * d, 0.0), c + Vec2::new(-6.0 * d, 8.0)], col, Stroke::NONE));
    p.rect_filled(Rect::from_center_size(c + Vec2::new(7.0 * d, 0.0), Vec2::new(2.5, 16.0)), 1.0, col);
}

/// A short line in plain small type (in place of the desktop's dim_line, which draws larger on the phone).
pub(crate) fn dim(ui: &mut egui::Ui, text: &str, px: f32, col: Color32) {
    let size = (px * 1.15).max(1.05);
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 9.0 + size * 9.0), Sense::hover());
    ptext_fit(ui.painter(), Pos2::new(r.min.x + 2.0, r.center().y), Align::Min, text, size, r.width() - 4.0, col);
}

/// A note in plain, small type, wrapped to the width (in place of the desktop's boxed para).
pub(crate) fn note_text(ui: &mut egui::Ui, text: &str, col: Color32) {
    for line in wrap(text, 1.2, ui.available_width() - 8.0) {
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 19.0), Sense::hover());
        ptext(ui.painter(), Pos2::new(r.min.x + 2.0, r.center().y), Align::Min, &line, 1.2, col);
    }
    ui.add_space(4.0);
}

/// The phone's button: a small pill, its word in plain type, filled when on (in place of the desktop's retro_btn).
pub(crate) fn pill(ui: &mut egui::Ui, text: &str, on: bool) -> egui::Response {
    let w = text_w(text, 1.2) + 26.0;
    pill_w(ui, text, w, on)
}

/// A pill of a given width.
pub(crate) fn pill_w(ui: &mut egui::Ui, text: &str, w: f32, on: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w.max(32.0), 34.0), Sense::click());
    let fill = if on { pal().ink } else if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige_lt };
    ui.painter().rect_filled(r.shrink2(Vec2::new(0.0, 1.0)), 16.0, fill);
    ptext_fit(ui.painter(), r.center(), Align::Center, text, 1.2, r.width() - 10.0, if on { pal().beige } else { pal().ink });
    resp
}

/// A metronome: the case and its arm.
fn metro_glyph(p: &egui::Painter, c: Pos2, col: Color32) {
    let s = Stroke::new(1.6, col);
    let pts = vec![c + Vec2::new(-6.0, 8.0), c + Vec2::new(-2.5, -8.0), c + Vec2::new(2.5, -8.0), c + Vec2::new(6.0, 8.0), c + Vec2::new(-6.0, 8.0)];
    p.add(egui::Shape::line(pts, s));
    p.line_segment([c + Vec2::new(0.0, 4.0), c + Vec2::new(5.0, -6.0)], s);
}

/// A gear: a ring with eight teeth.
pub(crate) fn gear_glyph(p: &egui::Painter, c: Pos2, col: Color32) {
    let s = Stroke::new(2.0, col);
    p.circle_stroke(c, 6.0, s);
    p.circle_stroke(c, 2.2, Stroke::new(1.6, col));
    for k in 0..8 {
        let a = k as f32 / 8.0 * std::f32::consts::TAU;
        let d = Vec2::new(a.cos(), a.sin());
        p.line_segment([c + d * 6.5, c + d * 9.5], Stroke::new(2.6, col));
    }
}

/// A down arrow (close Now Playing).
fn chevron(p: &egui::Painter, c: Pos2, col: Color32) {
    let s = Stroke::new(2.2, col);
    p.line_segment([c + Vec2::new(-7.0, -3.0), c + Vec2::new(0.0, 4.0)], s);
    p.line_segment([c + Vec2::new(0.0, 4.0), c + Vec2::new(7.0, -3.0)], s);
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
    // what happened last run, where it can be read over USB: Tidal's log (sign-in errors and all), and whether the
    // sign-in file is there now (the app's own folder can't be read from outside)
    if let (Some(home), Some(files)) = (std::env::var_os("BACKLINE_HOME"), std::env::var_os("BACKLINE_FILES")) {
        let (home, files) = (std::path::PathBuf::from(home), std::path::PathBuf::from(files));
        let _ = std::fs::create_dir_all(&files);
        let _ = std::fs::copy(home.join("log.txt"), files.join("last_log.txt"));
        let signed = home.join("session.json").exists();
        let size = std::fs::metadata(home.join("session.json")).map(|m| m.len()).unwrap_or(0);
        let _ = std::fs::write(files.join("start.txt"), format!("sign-in file there at start: {} ({} bytes)
", signed, size));
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
