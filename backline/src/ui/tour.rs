//! The guided tour: a dimmed screen with the part being explained left bright, and a small card with
//! BACK / NEXT. Two short tours of six steps each: one for everyone, one for the practice tools. Every look reports
//! the same parts (LIBRARY, PLAYER, QUEUE, PRACTICE), so a tour reads the same in all of them.
//! A step can light up a part and set the scene first (Now Playing, practice mode, a practice page). The tour plays
//! its own song (tour_song.rs), very quietly, and shows things working on it: a loop at a slower speed, a chord,
//! the band. When it ends, everything comes back: what was showing, the queue, the song and where it was, the
//! speed, the loop and the equalizer.

use crate::Within;
use crate::{pal, retro_btn, title_line, Action, App, practice_tools};
use eframe::egui::{self, Color32, Pos2, Rect};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static RECTS: RefCell<HashMap<String, Rect>> = RefCell::new(HashMap::new());
}

/// Called at the start of every frame: windows that are not drawn this frame are not highlighted.
pub fn begin_frame() {
    RECTS.with(|r| r.borrow_mut().clear());
}

/// A window reports where it is (keyed by the first word of its title).
pub fn note(title: &str, r: Rect) {
    mark(title.split(' ').next().unwrap_or(""), r);
}

/// A control reports where it is, so a tour step can point right at it.
pub fn mark(key: &str, r: Rect) {
    RECTS.with(|m| m.borrow_mut().insert(key.to_string(), r));
}

fn find(key: &str) -> Option<Rect> {
    RECTS.with(|m| m.borrow().get(key).copied())
}

/// What should be on screen for a step.
#[derive(Clone, Copy, PartialEq)]
enum Scene {
    /// the normal windows, practice mode as it was
    Normal,
    /// the big album view
    Album,
    /// practice mode off (to show where it is switched on)
    PracticeOff,
    Practice,
    /// practice mode, the CHORDS page on its chord finder
    Chords,
    /// practice mode, the EAR training page
    Ear,
    /// practice mode, the lead sheet with Cherokee on it
    Sheet,
}

struct Step {
    /// the part to light up ("" = none, the card sits in the middle)
    key: &'static str,
    scene: Scene,
    title: &'static str,
    text: &'static str,
    /// the tour shows it working on the tour's song: its speed (0 = normal), the A-B loop on
    speed: u32,
    ab: bool,
    /// done once when the step opens (the tour's song pauses for it): 1 a chord strummed, 2 the band
    act: u8,
}

/// The changes the band plays in the tour (and the demo's one tune).
pub(crate) const CHEROKEE: &str = "T44 *A | Bb6 | Fm7 Bb7 | Ebmaj7 | Ebm7 Ab7 | Bb6 | C7 | Cm7 | F7 | Bb6 | Fm7 Bb7 | Ebmaj7 | Ebm7 Ab7 | Bb6 | Cm7 F7 | Bb6 | % | *B | C#m7 | F#7 | Bmaj7 | % | Bm7 | E7 | Amaj7 | % | Am7 | D7 | Gmaj7 | % | Gm7 | C7 | Cm7 | F7 |";

/// A step with nothing being demonstrated (fill in the rest).
const BASE: Step = Step { key: "", scene: Scene::Normal, title: "", text: "", speed: 0, ab: false, act: 0 };

const BASIC: &[Step] = &[
    Step {
        title: "WELCOME",
        text: "A one-minute tour, with a quiet song playing so you hear things work. NEXT or the right arrow moves on, ESC ends it, and your music comes back after.",
        ..BASE
    },
    Step {
        key: "LIBRARY",
        title: "YOUR MUSIC",
        text: "Tidal, your files, YouTube, SoundCloud and your own playlists. Click a song to play it, or right-click it for more.",
        ..BASE
    },
    Step {
        key: "PLAYER",
        title: "THE PLAYER",
        text: "Play, pause, skip and the volume. Click along the bar to jump in the song.",
        ..BASE
    },
    Step {
        key: "QUEUE",
        title: "UP NEXT",
        text: "The songs coming up. Drag one to move it.",
        ..BASE
    },
    Step {
        scene: Scene::Album,
        title: "NOW PLAYING",
        text: "Press A for the song up close: the cover big, with the lyrics and what's next. ESC comes back.",
        ..BASE
    },
    Step {
        title: "THAT'S IT",
        text: "SKINS, down the side, changes the whole look. F1 lists every key. DONE puts your music back.",
        ..BASE
    },
];

const PRACTICE_TOUR: &[Step] = &[
    Step {
        key: "CHIP",
        scene: Scene::PracticeOff,
        title: "PRACTICE",
        text: "Tools for learning music from records. Click PRACTICE, or press P.",
        ..BASE
    },
    Step {
        key: "PRACTICE",
        scene: Scene::Practice,
        title: "LOOP AND SLOW DOWN",
        text: "A few seconds going round at 85%, at the same pitch. Press A and B to loop your own spot, or shift-drag on the waveform.",
        ab: true,
        speed: 85,
        ..BASE
    },
    Step {
        key: "CHORDSVIEW",
        scene: Scene::Chords,
        title: "CHORDS",
        text: "Any chord's notes, its grips on the neck and the scales that fit.",
        act: 1,
        ..BASE
    },
    Step {
        key: "SHEET",
        scene: Scene::Sheet,
        title: "THE BAND",
        text: "Every tune can have a chart, and a band plays it at your tempo. This is Cherokee.",
        act: 2,
        ..BASE
    },
    Step {
        key: "EARVIEW",
        scene: Scene::Ear,
        title: "EAR TRAINING",
        text: "Drills from intervals to chord qualities. The ones you mix up come back more often.",
        ..BASE
    },
    Step {
        key: "TABS",
        scene: Scene::Practice,
        title: "KEEP TRACK",
        text: "TUNES, LOOPS and the DIARY keep what you've practiced. DONE puts everything back.",
        ..BASE
    },
];

/// How things were when the tour started, to put back when it ends.
pub(crate) struct TourPlay {
    queue: std::sync::Arc<Vec<crate::Track>>,
    orig_queue: Option<Vec<crate::Track>>,
    cur: Option<usize>,
    pos: f32,
    playing: bool,
    shuffle: bool,
    speed: u32,
    lp: (Option<f32>, Option<f32>, bool),
    eq: ([f32; 10], bool, bool),
    last: Option<crate::store::Last>,
    /// repeat as it was (the tour plays its song on repeat one)
    repeat: crate::Repeat,
    /// the library list, the right-hand tab, the lead sheet's practice chart and the CHORDS tab as they were
    sec: crate::Sec,
    rtab: u8,
    prac_chart: Option<(String, String)>,
    chords_tab: u8,
    /// the side pages that were open (preferences; the BACKLINE page, Winamp skins or the log): the tour
    /// closes them so every window it points at can be seen, and opens them again at the end
    pages: (bool, Option<crate::Side>),
    /// the step whose action has been done, and whether the tour paused its song or started the band
    acted: Option<usize>,
    paused: bool,
    band: bool,
    /// the page that was showing (your library, Tidal home...): the tour shows only its own song, then puts it back
    page: Option<std::sync::Arc<crate::api::Page>>,
    /// the tour's song, once it is the one playing
    song: Option<i64>,
}

fn steps(kind: u8) -> &'static [Step] {
    if kind == 1 && practice_tools() {
        PRACTICE_TOUR
    } else {
        BASIC
    }
}

/// How loud the tour's song plays, as a share of the volume set: very quiet, background music for the tour (the
/// chord and the band it shows play at your volume, over it). A recorded tour keeps the song as quiet.
const TOUR_LEVEL: f32 = 0.25;

impl App {
    pub(crate) fn tour_start(&mut self, kind: u8) {
        self.show_help = false;
        self.tour = Some((kind, 0));
        self.tour_restore = Some((self.art_view, self.practice, self.more, self.mtab));
        self.tour_seen = true;
        self.dirty = true;
        // keep what was playing, then put on the tour song
        let active = self.cur.is_some() && !self.stopped;
        let snap = TourPlay {
            queue: self.queue.clone(),
            orig_queue: self.orig_queue.clone(),
            cur: self.cur,
            pos: self.pos(),
            playing: active && !self.paused,
            shuffle: self.shuffle,
            speed: self.speed,
            lp: (self.loop_a, self.loop_b, self.loop_on),
            eq: (self.eq_gains, self.eq_on, self.show_eq),
            last: self.store.last.clone(),
            repeat: self.repeat,
            sec: self.sec,
            rtab: self.rtab,
            prac_chart: self.prac_chart.clone(),
            chords_tab: self.chords_tab,
            pages: (self.show_prefs, self.side),
            acted: None,
            paused: false,
            band: false,
            page: self.page.clone(),
            song: None,
        };
        self.tour_play = Some(snap);
        // from the first moment: the library shows nothing of yours (the tour's song joins it once it is here)
        self.lib_tab = crate::Tab::Tracks;
        self.page =
            Some(std::sync::Arc::new(crate::api::Page { title: "My Tracks".to_string(), library: true, ..Default::default() }));
        self.sec = crate::Sec::Tidal;
        // the tour's song: right away when it was fetched before, otherwise once it arrives
        match crate::tour_song::ready() {
            Some(path) => self.tour_put_on(path),
            None => {
                self.bg(move |_| crate::Msg::TourSong(crate::tour_song::fetch()));
            }
        }
    }

    /// Is this the tour's own song, playing during a tour? Then it is a demo: nothing you saved shows on it (loops,
    /// sections, stems) and nothing it does is saved (practice time, loops, EQ).
    pub(crate) fn is_demo(&self, id: i64) -> bool {
        self.tour_play.as_ref().and_then(|s| s.song) == Some(id)
    }

    /// Your library page as it was before the tour (more of your songs arriving during a tour go there).
    pub(crate) fn tour_library(&mut self) -> Option<&mut std::sync::Arc<crate::api::Page>> {
        self.tour_play.as_mut()?.page.as_mut().filter(|p| p.library)
    }

    /// The song playing is the tour's own.
    pub(crate) fn demo_playing(&self) -> bool {
        self.cur_track().is_some_and(|t| self.is_demo(t.id))
    }

    /// Put on the tour's song (if the tour is still on and nothing has been put on yet).
    pub(crate) fn tour_put_on(&mut self, path: std::path::PathBuf) {
        if self.tour.is_none() || self.tour_play.as_ref().is_none_or(|s| s.song.is_some()) {
            return;
        }
        let mut track = crate::sources::ext_for_file(&path).to_track();
        // its own cover, drawn for Backline
        track.cover = crate::tour_song::COVER.to_string();
        // a tour shows nobody's own music: its song is all the library holds (your page comes back at the end)
        self.demo_track = Some(track.clone());
        self.page = Some(std::sync::Arc::new(crate::api::Page {
            title: "My Tracks".to_string(),
            tracks: vec![track.clone()],
            library: true,
            total: 1,
            ..Default::default()
        }));
        self.sec = crate::Sec::Tidal;
        if let Some(s) = self.tour_play.as_mut() {
            s.song = Some(track.id);
        }
        self.srcmap.insert(track.id, crate::Src::File(path));
        self.queue = std::sync::Arc::new(vec![track]);
        self.orig_queue = None;
        self.shuffle = false;
        // round and round, so it never runs out mid-tour
        self.repeat = crate::Repeat::One;
        self.loop_a = None;
        self.loop_b = None;
        self.loop_on = false;
        self.apply(Action::PlayIndex(0));
        self.tour_volume(true);
    }

    /// The tour's song plays very quietly, under the tour; the chord and the band it shows (the song paused for them)
    /// at your own volume. Your volume comes back when it ends.
    fn tour_volume(&self, quiet: bool) {
        let v = if quiet { self.volume * TOUR_LEVEL } else { self.volume };
        self.player.send(crate::player::Cmd::Volume(v * v));
    }

    /// Show the step's feature working on the tour's song (only what differs, so nothing restarts).
    fn tour_demo(&mut self, speed: u32, ab: bool) {
        let Some(snap) = &self.tour_play else { return };
        if snap.song.is_none() {
            return;
        }
        let song = snap.song;
        if self.cur_track().map(|t| t.id) != song {
            return; // something else was picked meanwhile: leave it alone
        }
        let want = if speed > 0 { speed } else { 100 };
        if self.speed != want {
            self.apply(Action::Speed(want));
        }
        if ab {
            let (a, b) = crate::tour_song::loop_span();
            if self.loop_a != Some(a) || self.loop_b != Some(b) {
                self.apply(Action::SetAAt(a));
                self.apply(Action::SetBAt(b));
                // and go there, so the loop is what you hear
                self.apply(Action::Seek(a));
            }
            if !self.loop_on {
                self.apply(Action::LoopToggle);
            }
        } else if self.loop_a.is_some() || self.loop_b.is_some() {
            self.loop_a = None;
            self.loop_b = None;
            self.loop_on = false;
            self.sync_loop();
        }
    }

    /// Put back what was playing before the tour: the queue, the song (playing again where it was if it was
    /// playing), the speed, the loop and the equalizer.
    fn tour_unplay(&mut self) {
        let Some(s) = self.tour_play.take() else { return };
        if self.seq_play.is_some() {
            self.apply(Action::StopSequence);
        }
        if s.band && self.band_on {
            self.apply(Action::Opt(crate::extras::Opt::Band));
        }
        self.sec = s.sec;
        self.rtab = s.rtab;
        self.prac_chart = s.prac_chart.clone();
        self.chords_tab = s.chords_tab;
        (self.show_prefs, self.side) = s.pages;
        if s.song.is_some() {
            self.apply(Action::StopBtn);
            self.player.send(crate::player::Cmd::Volume(self.volume * self.volume));
        }
        self.queue = s.queue;
        self.orig_queue = s.orig_queue;
        self.shuffle = s.shuffle;
        self.repeat = s.repeat;
        (self.eq_gains, self.eq_on, self.show_eq) = s.eq;
        self.apply_eq();
        self.store.last = s.last;
        self.store_dirty = true;
        // your page as it was (with any of your songs that arrived during the tour)
        self.page = s.page.clone();
        self.demo_track = None;
        if s.song.is_none() {
            return;
        }
        self.cur = s.cur;
        match s.cur {
            Some(i) if s.playing => {
                self.apply(Action::PlayIndex(i));
                if s.pos > 1.0 {
                    if self.buffering {
                        self.pending_seek = Some(s.pos);
                    } else {
                        self.player.send(crate::player::Cmd::Seek(s.pos));
                    }
                }
            }
            _ => {}
        }
        self.speed = s.speed;
        (self.loop_a, self.loop_b, self.loop_on) = s.lp;
        self.sync_loop();
    }

    /// Put the screen in the state a step needs (only what differs, so nothing flickers).
    fn tour_scene(&mut self, scene: Scene) {
        let Some((_, practice, more, mtab)) = self.tour_restore else { return };
        let want_art = scene == Scene::Album;
        let want_practice = match scene {
            Scene::PracticeOff => false,
            Scene::Practice | Scene::Chords | Scene::Ear | Scene::Sheet => true,
            _ => practice,
        };
        if self.art_view != want_art {
            self.art_view = want_art;
        }
        if practice_tools() && self.practice != want_practice {
            self.apply(Action::TogglePractice);
        }
        // (the practice panel shows its loop and speed, not a MORE tab)
        (self.more, self.mtab) = if scene == Scene::Practice { (false, mtab) } else { (more, mtab) };
        match scene {
            Scene::Chords => {
                self.sec = crate::Sec::Chords;
                self.chords_tab = 0;
            }
            Scene::Ear => {
                self.sec = crate::Sec::Ear;
            }
            Scene::Sheet => {
                self.sec = crate::Sec::Chart;
                if self.prac_chart.as_ref().map(|c| c.0.as_str()) != Some("CHEROKEE") {
                    self.prac_chart = Some(("CHEROKEE".to_string(), CHEROKEE.to_string()));
                }
            }
            _ => {}
        }
    }

    /// A step's one-off: the tour's song pauses, then a chord is strummed or the band started. Leaving those steps
    /// stops the band and lets the song go on.
    fn tour_act(&mut self, i: usize, act: u8) {
        let Some(s) = self.tour_play.as_ref() else { return };
        if s.song.is_none() {
            return;
        }
        let (was_paused, had_band) = (s.paused, s.band);
        if s.acted == Some(i) {
            // the song stays paused through these steps, whatever else happens
            if act != 0 && !self.paused && !self.stopped {
                self.apply(Action::PauseBtn);
            }
            return;
        }
        if let Some(s) = self.tour_play.as_mut() {
            s.acted = Some(i);
        }
        if act == 0 {
            if had_band && self.band_on {
                self.apply(Action::Opt(crate::extras::Opt::Band));
            }
            self.tour_volume(true);
            if was_paused && self.paused {
                self.apply(Action::PlayBtn);
            }
            if let Some(s) = self.tour_play.as_mut() {
                (s.paused, s.band) = (false, false);
            }
            return;
        }
        if !self.paused && !self.stopped {
            self.apply(Action::PauseBtn);
            if let Some(s) = self.tour_play.as_mut() {
                s.paused = true;
            }
        }
        self.tour_volume(false);
        if act != 2 && had_band && self.band_on {
            self.apply(Action::Opt(crate::extras::Opt::Band));
            if let Some(s) = self.tour_play.as_mut() {
                s.band = false;
            }
        }
        match act {
            1 => {
                self.chord_q = "Cmaj7".to_string();
                if let Some(c) = crate::theory::chord_notes("Cmaj7") {
                    self.apply(Action::PlayNotes(crate::chords::piano_voicing(&c)));
                }
            }
            _ => {
                if !self.band_on {
                    self.apply(Action::Opt(crate::extras::Opt::Band));
                    if let Some(s) = self.tour_play.as_mut() {
                        s.band = true;
                    }
                }
            }
        }
    }

    /// The tour is over: everything back the way it was.
    fn tour_end(&mut self) {
        self.tour_unplay();
        if let Some((art, practice, more, mtab)) = self.tour_restore.take() {
            self.art_view = art;
            if practice_tools() && self.practice != practice {
                self.apply(Action::TogglePractice);
            }
            self.more = more;
            self.mtab = mtab;
        }
        self.tour = None;
    }

    pub(crate) fn tour_overlay(&mut self, ctx: &egui::Context) {
        let Some((kind, i)) = self.tour else { return };
        let list = steps(kind);
        let i = i.min(list.len() - 1);
        let step = &list[i];
        // the library window shows the library, not a side page, while the tour points at things
        (self.show_prefs, self.side) = (false, None);
        self.tour_scene(step.scene);
        self.tour_demo(step.speed, step.ab);
        self.tour_act(i, step.act);
        let screen = ctx.screen_rect();
        let target = if step.key.is_empty() { None } else { find(step.key) };
        // dim everything except the highlighted window
        // the tour has the floor: only its card takes clicks until it is done
        crate::credits::block_behind(ctx, "tour");
        let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("tour_dim")));
        let dim = Color32::from_black_alpha(165);
        match target {
            Some(t) => {
                let t = t.expand(4.0).intersect(screen);
                p.rect_filled(Rect::from_min_max(screen.min, Pos2::new(screen.max.x, t.min.y)), 0.0, dim);
                p.rect_filled(Rect::from_min_max(Pos2::new(screen.min.x, t.max.y), screen.max), 0.0, dim);
                p.rect_filled(Rect::from_min_max(Pos2::new(screen.min.x, t.min.y), Pos2::new(t.min.x, t.max.y)), 0.0, dim);
                p.rect_filled(Rect::from_min_max(Pos2::new(t.max.x, t.min.y), Pos2::new(screen.max.x, t.max.y)), 0.0, dim);
                p.rect_stroke(t, 4.0, egui::Stroke::new(3.0_f32, pal().bar_txt));
            }
            None => {
                p.rect_filled(screen, 0.0, dim);
            }
        }
        // card at the bottom, or at the top when the highlighted window is down there
        let low = target.is_some_and(|t| t.center().y > screen.center().y);
        let (align, off) =
            if low { (egui::Align2::CENTER_TOP, [0.0, 24.0]) } else { (egui::Align2::CENTER_BOTTOM, [0.0, -24.0]) };
        let mut go: i32 = 0;
        egui::Area::new(egui::Id::new("tour_card")).order(egui::Order::Tooltip).anchor(align, off).show(ctx, |ui| {
            // the card keeps its own colours (soft white on near black), so it reads the same whatever skin is worn
            let frame = egui::Frame::none()
                .fill(Color32::from_rgb(20, 20, 22))
                .stroke(egui::Stroke::new(2.0_f32, Color32::from_gray(110)))
                .rounding(0.0)
                .inner_margin(14.0);
            frame.show(ui, |ui| {
                ui.set_width(460.0_f32.min(screen.width() - 60.0));
                // the step's title, wrapped onto more lines when it is long (never cut off)
                let head = format!("{}   {}/{}", step.title, i + 1, list.len());
                for line in crate::wrap(&head, 2.5, ui.available_width() - 8.0) {
                    title_line(ui, &line, 2.5, Color32::from_rgb(240, 240, 236));
                }
                // (the whole text, plainly on the card - not folded to a line)
                crate::wrapped(ui, step.text, Color32::from_rgb(225, 225, 220));
                // the credit the song's licence asks for, while it plays: small and still, under the text
                if self.tour_play.as_ref().is_some_and(|s| s.song.is_some()) {
                    let w = ui.available_width() - 12.0;
                    for line in crate::wrap(crate::tour_song::CREDIT, 1.0, w) {
                        let (r, _) = ui.allocate_exact_size(egui::Vec2::new(w, 7.0 * crate::spx(1.0) + 4.0), egui::Sense::hover());
                        crate::ptext(ui.painter(), Pos2::new(r.min.x + 6.0, r.center().y), egui::Align::Min, &line, 1.0, Color32::from_gray(130));
                    }
                }
                ui.horizontal(|ui| {
                    if i > 0 && retro_btn(ui, "BACK", false).clicked() {
                        go = -1;
                    }
                    if retro_btn(ui, if i + 1 == list.len() { "DONE" } else { "NEXT" }, false).clicked() {
                        go = 1;
                    }
                    if i + 1 < list.len() && retro_btn(ui, "SKIP", false).clicked() {
                        go = 2;
                    }
                });
            });
        });
        if ctx.input(|k| k.key_pressed(egui::Key::Escape)) {
            go = 2;
        }
        if ctx.input(|k| k.key_pressed(egui::Key::ArrowRight)) {
            go = 1;
        }
        if ctx.input(|k| k.key_pressed(egui::Key::ArrowLeft)) && i > 0 {
            go = -1;
        }
        match go {
            1 if i + 1 < list.len() => self.tour = Some((kind, i + 1)),
            -1 => self.tour = Some((kind, i - 1)),
            0 => {}
            _ => self.tour_end(),
        }
        ctx.request_repaint();
    }
}

// ------------------------------------------------------------------ recording a demo
// BACKLINE_DEMO=regular or practice runs that tour by itself and records it to BACKLINE_DEMO_OUT (an .mp4):
// the app's own frames and its own sound (muted), so nothing else on the screen or the speakers gets in.

/// Which tour to record, from BACKLINE_DEMO.
pub(crate) fn demo_kind() -> Option<u8> {
    match std::env::var("BACKLINE_DEMO").ok()?.as_str() {
        "practice" => Some(1),
        "regular" => Some(0),
        _ => None,
    }
}

const FPS: f64 = 30.0;

/// A demo being recorded.
pub(crate) struct DemoRec {
    kind: u8,
    start: std::time::Instant,
    /// 0 waiting to start, 1 touring, 2 ending
    phase: u8,
    step: usize,
    step_at: std::time::Instant,
    end_at: std::time::Instant,
    ffmpeg: Option<std::process::Child>,
    size: Option<[usize; 2]>,
    frames: u64,
    out: std::path::PathBuf,
}

impl DemoRec {
    pub(crate) fn new(kind: u8) -> DemoRec {
        let now = std::time::Instant::now();
        let out = std::env::var_os("BACKLINE_DEMO_OUT").map(std::path::PathBuf::from).unwrap_or_else(|| "demo.mp4".into());
        DemoRec { kind, start: now, phase: 0, step: 0, step_at: now, end_at: now, ffmpeg: None, size: None, frames: 0, out }
    }

    fn ffmpeg_exe() -> String {
        std::env::var("BACKLINE_FFMPEG").unwrap_or_else(|_| "ffmpeg".into())
    }

    /// One frame of the app: written as many times as the clock says, so the video keeps real time.
    fn frame(&mut self, img: &egui::ColorImage) {
        let [w, h] = img.size;
        if self.ffmpeg.is_none() {
            let video = self.out.with_extension("video.mp4");
            let child = std::process::Command::new(Self::ffmpeg_exe())
                .args([
                    "-y",
                    "-loglevel",
                    "error",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "rgba",
                    "-s",
                    &format!("{}x{}", w, h),
                    "-r",
                    "30",
                    "-i",
                    "-",
                ])
                .args([
                    "-vf",
                    "scale=trunc(iw/2)*2:trunc(ih/2)*2",
                    "-c:v",
                    "libx264",
                    "-preset",
                    "veryfast",
                    "-crf",
                    "20",
                    "-pix_fmt",
                    "yuv420p",
                ])
                .arg(&video)
                .stdin(std::process::Stdio::piped())
                .spawn();
            match child {
                Ok(c) => {
                    self.ffmpeg = Some(c);
                    self.size = Some([w, h]);
                }
                Err(e) => {
                    crate::api::log(&format!("demo: could not start ffmpeg: {}", e));
                    return;
                }
            }
        }
        if self.size != Some([w, h]) {
            return;
        }
        let want = (self.start.elapsed().as_secs_f64() * FPS) as u64;
        if want <= self.frames {
            return;
        }
        let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
        if let Some(stdin) = self.ffmpeg.as_mut().and_then(|c| c.stdin.as_mut()) {
            use std::io::Write;
            while self.frames < want {
                if stdin.write_all(&bytes).is_err() {
                    break;
                }
                self.frames += 1;
            }
        }
    }

    /// The video is done: close it, keep the sound as a WAV, and put the two together in the .mp4.
    fn finish(&mut self) {
        let video = self.out.with_extension("video.mp4");
        if let Some(mut c) = self.ffmpeg.take() {
            drop(c.stdin.take());
            let _ = c.wait();
        }
        let audio = self.out.with_extension("wav");
        let offset = 0.0;
        if let Some(mut r) = crate::player::rec_take() {
            // the song as quiet as the tour plays it, under the chord and the band (kept before the volume, as they are)
            for p in r.pieces.iter_mut().filter(|p| p.song) {
                p.gain = TOUR_LEVEL * TOUR_LEVEL;
            }
            let secs = self.frames as f64 / FPS;
            let _ = std::fs::write(&audio, crate::band::wav_of(&mix(&r, self.start, secs), MIX_RATE, 2));
        }
        let ok = std::process::Command::new(Self::ffmpeg_exe())
            .args(["-y", "-loglevel", "error", "-i"])
            .arg(&video)
            .args(["-itsoffset", &format!("{:.3}", offset), "-i"])
            .arg(&audio)
            .args(["-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-shortest"])
            .arg(&self.out)
            .status()
            .is_ok_and(|s| s.success());
        if ok {
            let _ = std::fs::remove_file(&video);
            let _ = std::fs::remove_file(&audio);
        }
        crate::api::log(&format!("demo: wrote {} ({} frames, sound from {:.2}s)", self.out.display(), self.frames, offset));
    }
}

const MIX_RATE: u32 = 44100;

/// Everything the recording kept, laid on one timeline from `start` for `secs` seconds: the song where it played
/// (with its pauses), the chords, lines and the band where they played. Stereo, 44.1 kHz.
pub(crate) fn mix(r: &crate::player::RecBuf, start: std::time::Instant, secs: f64) -> Vec<i16> {
    let n = (secs * MIX_RATE as f64) as usize;
    let mut acc = vec![0i32; n * 2];
    for p in &r.pieces {
        let ch = p.channels.max(1) as usize;
        let at = p.at.checked_duration_since(start).map_or(0.0, |d| d.as_secs_f64());
        let o0 = (at * MIX_RATE as f64) as usize;
        // how many output frames this piece fills: its own length, or until it was stopped for a repeating body
        let src_frames = p.data.len() / ch;
        let lead_out = (src_frames as f64 * MIX_RATE as f64 / p.rate.max(1) as f64) as usize;
        let total = match p.until {
            Some(u) if !p.body.is_empty() => {
                let end = u.checked_duration_since(start).map_or(0.0, |d| d.as_secs_f64());
                ((end - at).max(0.0) * MIX_RATE as f64) as usize
            }
            _ => lead_out,
        };
        let step = p.rate as f64 / MIX_RATE as f64;
        for k in 0..total {
            let o = o0 + k;
            if o >= n {
                break;
            }
            let f = (k as f64 * step) as usize;
            let (l, rgt) = if f < src_frames {
                let i = f * ch;
                (p.data[i] as f32, p.data[i + (ch - 1).min(1)] as f32)
            } else if !p.body.is_empty() {
                let v = p.body[(f - src_frames) % p.body.len()] as f32;
                (v, v)
            } else {
                break;
            };
            acc[o * 2] += (l * p.gain) as i32;
            acc[o * 2 + 1] += (rgt * p.gain) as i32;
        }
    }
    acc.into_iter().map(|v| v.clamp(-32768, 32767) as i16).collect()
}

impl App {
    /// Recording a demo: start the tour, move it on at reading pace, capture every frame, then finish and close.
    pub(crate) fn demo_tick(&mut self, ctx: &egui::Context) {
        let Some(mut rec) = self.demo_rec.take() else { return };
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
        let shots: Vec<std::sync::Arc<egui::ColorImage>> = ctx.input(|i| {
            i.raw
                .events
                .iter()
                .filter_map(|e| if let egui::Event::Screenshot { image, .. } = e { Some(image.clone()) } else { None })
                .collect()
        });
        for s in &shots {
            rec.frame(s);
        }
        let now = std::time::Instant::now();
        match rec.phase {
            0 if rec.start.elapsed().as_secs_f32() > 1.5 => {
                // the demo's one tune, with a chart the band can play
                if !self.store.tunes.iter().any(|t| t.name == "Cherokee") {
                    self.store.tunes.push(crate::store::Tune {
                        name: "Cherokee".into(),
                        key: "Bb".into(),
                        chart: CHEROKEE.into(),
                        written_by: "Ray Noble".into(),
                        year: "1938".into(),
                        ..Default::default()
                    });
                }
                crate::player::rec_start();
                self.tour_start(rec.kind);
                rec.phase = 1;
                rec.step = 0;
                rec.step_at = now;
            }
            1 => match self.tour {
                Some((kind, i)) => {
                    if i != rec.step {
                        rec.step = i;
                        rec.step_at = now;
                    }
                    let list = steps(kind);
                    // long enough to read the card, and for what it shows to be heard: the chord, the band
                    let words = list[i].text.split_whitespace().count() as f32;
                    let least = match list[i].act {
                        2 => 7.0,
                        1 => 4.0,
                        _ => 3.0,
                    };
                    let stay = (1.5 + words / 6.0).within(least, 8.0);
                    // the first step waits for the tour's song to be playing
                    let waiting = i == 0 && self.tour_play.as_ref().is_some_and(|s| s.song.is_none());
                    if waiting {
                        rec.step_at = now;
                    } else if now.duration_since(rec.step_at).as_secs_f32() > stay {
                        if i + 1 < list.len() {
                            self.tour = Some((kind, i + 1));
                        } else {
                            self.tour_end();
                        }
                    }
                }
                None => {
                    rec.phase = 2;
                    rec.end_at = now;
                }
            },
            2 if now.duration_since(rec.end_at).as_secs_f32() > 1.5 => {
                rec.finish();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            _ => {}
        }
        ctx.request_repaint();
        self.demo_rec = Some(rec);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_points_at_something_that_reports_itself() {
        // the library and practice panels report themselves in every look (window_deco); Classic's player and queue
        // windows too, and the styles mark their player bar and queue (styles::styled_layout); single controls call
        // tour::mark("KEY", ...)
        let windows = ["LIBRARY", "PLAYER", "PRACTICE", "QUEUE"];
        let src = [
            include_str!("library_win.rs"),
            include_str!("chords.rs"),
            include_str!("tools.rs"),
            include_str!("ear.rs"),
        ]
        .concat();
        let styles = include_str!("styles.rs");
        for s in BASIC.iter().chain(PRACTICE_TOUR) {
            if s.key.is_empty() {
                continue;
            }
            let marked = src.contains(&format!("mark(\"{}\"", s.key));
            assert!(marked || windows.contains(&s.key), "tour step {} points at {}, which nothing reports", s.title, s.key);
            // (and the styles have them too)
            if ["PLAYER", "QUEUE"].contains(&s.key) {
                assert!(styles.contains(&format!("tour::mark(\"{}\"", s.key)), "the styles don't mark {}", s.key);
            }
        }
    }

    #[test]
    fn tours_are_short_and_practice_steps_have_practice_on() {
        assert!(BASIC.len() <= 6 && PRACTICE_TOUR.len() <= 6, "simpler is better: six steps at most");
        // the practice panel only exists in practice mode: a step pointing into it must switch it on
        for s in PRACTICE_TOUR.iter().filter(|s| s.key == "PRACTICE") {
            assert!(s.scene == Scene::Practice, "{}", s.title);
        }
        assert!(BASIC.iter().any(|s| s.scene == Scene::Album), "the main tour visits Now Playing");
        assert!(BASIC.last().unwrap().scene == Scene::Normal, "and comes back from it");
    }
}
