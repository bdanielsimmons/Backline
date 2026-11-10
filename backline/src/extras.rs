//! The logic behind the practice extras: files / YouTube sources, the tune list, the practice
//! diary, saved sections, the speed trainer, metronome and the rest. (Their actions are carried out in actions.rs,
//! their messages taken in by messages.rs; the drawing is in the ui/ files.)

use crate::api::{self, Track};
use crate::band;
use crate::player::Cmd;
use crate::sources::{self, Src};
use crate::stems;
use crate::store::{self, Ext, Last, Tune, Version};
use crate::{cache, Action, App, Msg, Sec};
use eframe::egui;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const CHAN_NAMES: [&str; 6] = ["STEREO", "LEFT CHANNEL", "RIGHT CHANNEL", "MONO", "NO CENTER", "BASS ONLY"];

/// The settable numbers (the +/- buttons and the typed boxes next to them).
#[derive(Clone, Copy, PartialEq)]
pub enum Knob {
    Focus,
    Rest,
    Blocks,
    Loops,
    Step,
    Speed,
    LoopA,
    LoopB,
    Bpm,
    /// the metronome's own tempo (it doesn't follow the song's)
    MetroBpm,
    Beats,
    Unit,
    GapPlay,
    GapMute,
    RampBars,
    RampBpm,
    Shift,
}

/// Switches and small cycles (metronome, band, timer).
#[derive(Clone, Copy)]
pub enum Opt {
    MetroOn,
    Preset(u8),
    Level(usize),
    Sub,
    Follow,
    Sync,
    Double,
    Half,
    FindBeats,
    Band,
    BandStyleSet(u8),
    Part(usize),
    /// a part's level: which part, and up (true) or down
    PartLevel(usize, bool),
    /// times through the chart: up or down (0 = endless)
    BandLoops(bool),
    /// write the backing track to a WAV file
    BandExport,
    PomoSound,
    UnitSet(u8),
    Group,
}

/// "1:23.5", "83" -> seconds
pub fn parse_time(t: &str) -> Option<f32> {
    // the box shows "A 1:23.4" / "B 1:23.4": a leading letter is just the label
    let t = t.trim().trim_start_matches(|c: char| c.is_ascii_alphabetic()).trim();
    match t.split_once(':') {
        Some((m, s)) => Some(m.trim().parse::<f32>().ok()? * 60.0 + s.trim().parse::<f32>().ok()?),
        None => t.parse::<f32>().ok(),
    }
    .filter(|v| *v >= 0.0)
}

pub(crate) fn knob_step(k: Knob) -> f32 {
    match k {
        Knob::Focus | Knob::Shift => 5.0,
        Knob::LoopA | Knob::LoopB => 0.2,
        _ => 1.0,
    }
}

pub(crate) fn wave_path(id: i64) -> PathBuf {
    api::config_dir().join("waves").join(format!("{}.pk", id))
}

/// Adding files: 0 nothing going on, 1 the file window is open, 2 reading what was picked.
pub(crate) static FILES_STAGE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Find the files below the given paths (folders are searched, and remembered for RESCAN).
/// The songs in these paths (folders searched), the folders, and the playlist files among them: each one's name
/// and its songs' ids (the songs come in with the rest).
pub(crate) fn scan_paths(paths: Vec<PathBuf>) -> (Vec<Ext>, Vec<String>, Vec<(String, Vec<i64>)>) {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut folders: Vec<String> = Vec::new();
    let mut lists: Vec<(String, Vec<i64>)> = Vec::new();
    for p in paths {
        if p.is_dir() {
            folders.push(p.to_string_lossy().to_string());
            sources::scan_folder(&p, 0, &mut files);
        } else if sources::is_playlist_file(&p) {
            if let Some((name, songs)) = sources::read_playlist_file(&p) {
                let ids = songs.iter().map(|s| sources::ext_for_file(s).id).collect();
                files.extend(songs);
                lists.push((name, ids));
            }
        } else if sources::is_audio(&p) {
            files.push(p);
        }
    }
    let mut seen: HashSet<PathBuf> = HashSet::new();
    files.retain(|f| seen.insert(f.clone()));
    (files.iter().map(|f| sources::ext_for_file(f)).collect(), folders, lists)
}

/// Tune name from a track title: "Cherokee (Remastered 2005)" -> "Cherokee".
pub(crate) fn tune_name_from(title: &str) -> String {
    let mut s = title.to_string();
    for sep in [" (", " [", " - ", " – "] {
        if let Some(i) = s.find(sep) {
            s.truncate(i);
        }
    }
    let s = s.trim().to_string();
    if s.is_empty() {
        title.trim().to_string()
    } else {
        s
    }
}

/// A saved loop's file name: when it was saved, the song, which loop and where - e.g.
/// "2026-10-11 00-41-05 - Average White Band - Cut the Cake - FUNKY RIFF (0m05s-0m12s).wav". Nothing in it that a
/// file name can't hold.
pub(crate) fn loop_file_name(
    now: (i32, u32, u32, u32, u32, u32),
    artist: &str,
    title: &str,
    what: &str,
    a: f32,
    b: f32,
    stems: bool,
) -> String {
    let (y, mo, d, h, mi, s) = now;
    let t = |x: f32| format!("{}m{:02}s", x as u32 / 60, x as u32 % 60);
    let song = if artist.trim().is_empty() { title.to_string() } else { format!("{} - {}", artist, title) };
    let raw = format!(
        "{:04}-{:02}-{:02} {:02}-{:02}-{:02} - {} - {} ({}-{}){}.wav",
        y,
        mo,
        d,
        h,
        mi,
        s,
        song,
        what,
        t(a),
        t(b),
        if stems { " stems" } else { "" }
    );
    // tidy: no doubled spaces, no trailing dots (Windows drops them)
    let clean = safe_name(&raw).split_whitespace().collect::<Vec<_>>().join(" ");
    clean.replace(". .wav", ".wav")
}

pub(crate) fn safe_name(s: &str) -> String {
    s.chars().map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c }).collect::<String>().trim().to_string()
}

impl App {
    /// Put on a skin (downloaded or drawn by Backline). `announce`: say so in the status line (not at start-up).
    pub(crate) fn wear_skin(
        &mut self,
        r: Result<(crate::winamp::WaSkin, crate::skin::Pal, crate::winamp::Art), String>,
        announce: bool,
    ) {
        self.wa_loading = None;
        match r {
            Ok((s, p, art)) => {
                crate::skin::set_custom(Some(p));
                self.wa_art = Some(crate::winamp_ui::WaTex::new(&self.ctx, art));
                // a Winamp skin you just put on shows where its buttons are for a moment (some hide them in the
                // picture); Backline's own looks are plain, so not those
                if announce && !s.md5.starts_with("gen:") {
                    self.wa_map_until = self.ctx.input(|i| i.time) + 3.0;
                }
                crate::winamp_ui::set_chrome(self.wa_art.as_ref());
                self.apply_pl_font();
                crate::setup_style(&self.ctx);
                if announce {
                    let what = if s.md5.starts_with("gen:") { "LOOK" } else { "WINAMP SKIN" };
                    self.set_note(&format!("{}: {}", what, s.name.to_uppercase()));
                }
                if !s.md5.starts_with("gen:") {
                    // remembered among the recent ones, newest first
                    self.wa_recent.retain(|r| r.0 != s.md5);
                    self.wa_recent.insert(0, (s.md5.clone(), s.name.clone()));
                    self.wa_recent.truncate(8);
                }
                self.wa_worn = Some((s.md5, s.name));
                self.dirty = true;
            }
            Err(e) => self.set_err(format!("WINAMP SKIN: {}", e)),
        }
    }

    // ------------------------------------------------------------ start-up
    pub(crate) fn init_store(&mut self) {
        // files deleted or moved since last time are gone from the lists (a drive not plugged in keeps its files:
        // only a file whose folder is there but the file is not)
        let gone: Vec<i64> = self
            .store
            .files
            .iter()
            .filter(|e| {
                let p = std::path::Path::new(&e.src);
                !p.exists() && p.parent().is_some_and(|d| d.exists())
            })
            .map(|e| e.id)
            .collect();
        for id in gone {
            self.file_gone(id);
        }
        let n = self.store.takes.len();
        self.store.takes.retain(|t| std::path::Path::new(&t.path).exists());
        if self.store.takes.len() != n {
            self.store_dirty = true;
        }
        self.register_all();
        if !self.store.folders.is_empty() {
            self.rescan();
        }
    }

    /// Every song the lists know, known to the player (where each one plays from).
    pub(crate) fn register_all(&mut self) {
        let mut vs: Vec<Version> = Vec::new();
        vs.extend(self.store.files.iter().map(|e| e.to_version()));
        vs.extend(self.store.yt.iter().map(|e| e.to_version()));
        vs.extend(self.store.sc.iter().map(|e| e.to_version()));
        // (hearted ones not on Tidal: MY TRACKS lists them among the Tidal ones, its SOURCE column says which)
        vs.extend(self.store.hearts.iter().map(|e| e.to_version()));
        for t in &self.store.tunes {
            vs.extend(t.versions.iter().cloned());
        }
        if let Some(l) = &self.store.last {
            vs.push(l.version.clone());
        }
        // (what you played lately too: the RECENT tab plays them)
        vs.extend(self.store.recent.iter().map(|r| r.0.clone()));
        for v in &vs {
            self.register(v);
        }
    }

    /// Remember where a non-Tidal recording comes from, so it can be played from any list.
    pub(crate) fn register(&mut self, v: &Version) {
        match v.kind.as_str() {
            "file" => {
                // (a song of the other computer's, copied here: played from the copy)
                let p = match crate::peer_files::copy_of(&self.store.copied, v) {
                    Some(here) if !std::path::Path::new(&v.src).exists() => PathBuf::from(here),
                    _ => PathBuf::from(&v.src),
                };
                self.srcmap.insert(v.id, Src::File(p));
                crate::file_id(v.id, true);
            }
            "yt" => {
                self.srcmap.insert(v.id, Src::Yt(v.src.clone()));
            }
            _ => {}
        }
    }

    pub(crate) fn version_of(&self, t: &Track) -> Version {
        let (kind, src) = match self.srcmap.get(&t.id) {
            Some(Src::File(p)) => ("file", p.to_string_lossy().to_string()),
            Some(Src::Yt(v)) => ("yt", v.clone()),
            None => ("tidal", String::new()),
        };
        Version {
            kind: kind.to_string(),
            id: t.id,
            src,
            title: t.title.clone(),
            artist: t.artist.clone(),
            album: t.album.clone(),
            cover: t.cover.clone(),
            dur: t.duration,
            ..Default::default()
        }
    }

    /// A tune's tempo: the one saved with it, else one written in its notes ("300 bpm", "it's at 300").
    pub(crate) fn tune_bpm(&self, i: usize) -> Option<u32> {
        let t = self.store.tunes.get(i)?;
        (t.bpm > 0).then_some(t.bpm).or_else(|| tempo_in_notes(&t.notes))
    }

    pub(crate) fn cur_tune(&self) -> Option<usize> {
        self.cur_track().and_then(|t| self.store.tune_of(t.id))
    }

    pub(crate) fn find_tune(&self, name: &str) -> Option<usize> {
        let w = name.trim().to_lowercase();
        self.store.tunes.iter().position(|t| t.name.trim().to_lowercase() == w)
    }

    /// The tune the lead sheet follows: the one you opened, or the playing track's tune. None = a
    /// track that is in no tune (its chart is looked up on its own).
    pub(crate) fn chart_tune(&self) -> Option<usize> {
        if self.chart_live || self.prac_chart.is_some() {
            return None;
        }
        self.chart_pick.filter(|i| *i < self.store.tunes.len()).or_else(|| self.cur_tune())
    }

    /// Show the lead sheet: its own page in the library, so Back returns to where you were and the queue's window
    /// stays the queue. In focus mode (no library) it takes the queue's window, as a tab beside the queue.
    pub(crate) fn open_chart(&mut self) {
        if self.lib_hidden {
            self.rtab = 1;
            return;
        }
        self.rtab = 0;
        self.side = None;
        if self.sec != Sec::Chart {
            self.apply(Action::Section(Sec::Chart));
        }
    }

    /// The queue's window shows the lead sheet (focus mode only: elsewhere the chart has its page).
    pub(crate) fn chart_in_queue(&self) -> bool {
        crate::PRACTICE && self.lib_hidden && self.rtab == 1
    }

    /// Put the lead sheet on the song playing: its tune when it is in one, else its changes looked up.
    pub(crate) fn chart_follow(&mut self, id: i64) {
        self.prac_chart = None;
        self.chart_sel = None;
        match self.store.tune_of(id) {
            Some(i) => {
                self.chart_pick = Some(i);
                self.chart_live = false;
            }
            None => self.chart_live = true,
        }
    }

    /// Chart text on show right now.
    pub(crate) fn chart_text(&self) -> Option<String> {
        if let Some((_, c)) = &self.prac_chart {
            return Some(c.clone());
        }
        match self.chart_tune() {
            Some(i) => Some(self.store.tunes[i].chart.clone()),
            None => self.live.as_ref().map(|l| l.0.clone()),
        }
        .filter(|t| !t.trim().is_empty())
    }

    /// Keep a song's details (for LOOPS and the diary), once.
    pub(crate) fn remember_song(&mut self, t: &Track) {
        if !self.store.songs.contains_key(&t.id) {
            let v = self.version_of(t);
            self.store.songs.insert(t.id, v);
            self.store_dirty = true;
        }
    }

    /// Play a kept song - in the queue where it already is, else straight after the song playing - and go to one of
    /// its loops when asked.
    pub(crate) fn play_saved(&mut self, v: &Version, lp: Option<(f32, f32)>) {
        if let Some((a, b)) = lp {
            self.loops.insert(v.id, (a, b));
            self.practice = true;
        }
        // already playing: just go to the loop
        if self.cur_track().map(|t| t.id) == Some(v.id) {
            if let Some((a, b)) = lp {
                self.loop_a = Some(a);
                self.loop_b = Some(b);
                self.loop_on = true;
                self.new_current_loop();
                self.sync_loop();
                self.player.send(Cmd::Seek(a));
            }
            return;
        }
        self.register(v);
        let at = match self.queue.iter().position(|t| t.id == v.id) {
            Some(i) => i,
            None => {
                let mut q: Vec<Track> = (*self.queue).clone();
                let at = self.cur.map(|c| c + 1).unwrap_or(0).min(q.len());
                q.insert(at, v.to_track());
                self.queue = Arc::new(q);
                self.orig_queue = None;
                at
            }
        };
        self.play_index(at);
        if let Some((a, _)) = lp {
            if self.buffering {
                self.pending_seek = Some(a);
            } else {
                self.player.send(Cmd::Seek(a));
            }
        }
    }

    pub(crate) fn practice_label(&self) -> String {
        match self.cur_tune() {
            Some(i) => self.store.tunes[i].name.clone(),
            None => "Free practice".to_string(),
        }
    }

    // ------------------------------------------------- per-frame bookkeeping
    pub(crate) fn tick_practice(&mut self) {
        let dt = self.acc_at.elapsed().as_secs_f32().min(1.0);
        self.acc_at = Instant::now();
        let playing = self.cur.is_some() && !self.stopped && !self.paused && !self.buffering;
        // only real loop work counts: a loop that keeps going round - not practice mode left open, nor a loop
        // switched on that never comes back
        let w = self.player.ctl.wraps();
        if w > self.last_wraps && self.practice && playing {
            self.on_loops(w - self.last_wraps);
            self.last_wrap_at = Some(Instant::now());
        }
        let span = match (self.loop_a, self.loop_b) {
            (Some(a), Some(b)) if self.loop_on => Some((b - a).abs()),
            _ => None,
        };
        let repeating = span.is_some_and(|s| {
            self.last_wrap_at.is_some_and(|t| t.elapsed().as_secs_f32() < s * 100.0 / self.speed.max(25) as f32 + 3.0)
        });
        if self.practice && playing && repeating {
            self.acc += dt;
        }
        self.last_wraps = w;
        if self.acc >= 5.0 {
            self.flush_practice();
        }
        self.update_metro(playing);
        self.pomo_tick();
        self.plan_tick();
        if playing && self.last_cap.elapsed() > Duration::from_secs(60) {
            self.capture_last();
        }
        // no internet: try Tidal again now and then, less often each time (never in offline mode)
        if self.net_down && !self.go_offline && self.net_try.elapsed() > Duration::from_secs(self.net_wait) {
            self.net_try = Instant::now();
            let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
            std::thread::spawn(move || {
                let _ = match api.refresh_info() {
                    Ok(()) => tx.send(crate::Msg::LoggedIn),
                    Err(e) if crate::api::no_internet(&e) => tx.send(crate::Msg::NoInternet),
                    Err(e) => tx.send(crate::Msg::NeedLogin(e)),
                };
                ctx.request_repaint();
            });
        }
        if self.store_dirty && self.store_saved.elapsed() > Duration::from_secs(3) {
            self.store.save();
            self.store_dirty = false;
            self.store_saved = Instant::now();
            // something new may have come in: keep it too
            self.keep_sweep = true;
        }
        crate::extras::KEEP_ON.store(self.keep_cache, std::sync::atomic::Ordering::Relaxed);
        if self.keep_sweep {
            self.keep_sweep = false;
            self.keep_everything();
        }
    }

    /// Move whole seconds of practice into today's diary.
    pub(crate) fn flush_practice(&mut self) {
        let secs = self.acc as u32;
        if secs == 0 {
            return;
        }
        // the tour's demo song is not practice
        if self.demo_playing() {
            self.acc = 0.0;
            return;
        }
        self.acc -= secs as f32;
        let label = self.practice_label();
        let song = self.cur_track();
        let lp = self.loop_now();
        let date = store::today_str();
        let d = self.store.day_mut(&date);
        *d.secs.entry(label).or_insert(0) += secs;
        // and on which song, so free practice still says what you worked on
        if let Some(t) = song.as_ref() {
            *d.songs.entry(t.id).or_insert(0) += secs;
        }
        // and on which loop of it
        if let Some(k) = lp {
            d.loop_time.entry(k).or_insert((0, 0)).0 += secs;
        }
        if let Some(t) = song {
            self.remember_song(&t);
        }
        self.store_dirty = true;
    }

    /// The loop being practised now (the song and its A-B), as a day keeps it; None when no loop is on. It is the
    /// current loop until it is cleared or another is picked: its ends moved, today's time on it goes with them
    /// (adjusting a loop doesn't make a new one).
    pub(crate) fn loop_now(&mut self) -> Option<String> {
        if !(self.practice && self.loop_on) {
            return None;
        }
        let (a, b) = self.loop_a.zip(self.loop_b)?;
        let k = store::loop_key(self.cur_track()?.id, a, b);
        if let Some(old) = self.loop_ident.clone().filter(|o| *o != k) {
            let same_song = old.split(':').next() == k.split(':').next();
            if same_song {
                self.store.day_mut(&store::today_str()).move_loop(&old, &k);
                self.store_dirty = true;
            }
        }
        self.loop_ident = Some(k.clone());
        Some(k)
    }

    /// A loop picked (or the song changed, or the loop cleared): a new current loop - one of the song's saved loops
    /// when its ends are one's, so editing it moves that saved loop too.
    pub(crate) fn new_current_loop(&mut self) {
        self.loop_ident = None;
        self.loop_saved = None;
        let Some(t) = self.cur_track() else { return };
        let Some((a, b)) = self.loop_a.zip(self.loop_b) else { return };
        if let Some(i) =
            self.store.sections.get(&t.id).and_then(|l| l.iter().position(|s| (s.a - a).abs() < 0.01 && (s.b - b).abs() < 0.01))
        {
            self.loop_saved = Some((t.id, i));
        }
    }

    pub(crate) fn on_loops(&mut self, n: u32) {
        if self.demo_playing() {
            return;
        }
        let date = store::today_str();
        let key = self.loop_now();
        let d = self.store.day_mut(&date);
        d.loops += n;
        if let Some(k) = key {
            d.loop_time.entry(k).or_insert((0, 0)).1 += n;
        }
        self.store_dirty = true;
        if self.trainer {
            self.passes += n;
            self.check_trainer();
        }
    }

    pub(crate) fn check_trainer(&mut self) {
        if self.passes < self.trainer_n {
            return;
        }
        self.passes = 0;
        if self.trainer_keys {
            // up a 4th each time, kept within half an octave either way; the cycle comes home after 12
            self.keys_done += 1;
            if self.keys_done >= 12 {
                self.keys_done = 0;
                self.trainer = false;
                self.semis = 0;
                self.sync_loop();
                self.set_note("ALL 12 KEYS DONE. NICE WORK");
            } else {
                let mut s = self.semis + 500;
                if s > 600 {
                    s -= 1200;
                }
                self.semis = s;
                self.sync_loop();
                self.set_note(&format!("UP A 4TH: KEY {} OF 12", self.keys_done + 1));
            }
            return;
        }
        if self.speed < 100 {
            self.speed = (self.speed + self.trainer_step).min(100);
            self.sync_loop();
            if self.speed >= 100 {
                self.set_note("FULL SPEED. NICE WORK");
            } else {
                self.set_note(&format!("SPEED UP: {}%", self.speed));
            }
        }
    }

    pub(crate) fn capture_last(&mut self) {
        if self.stopped {
            return;
        }
        if let Some(t) = self.cur_track() {
            let v = self.version_of(&t);
            // (the same queue as last time: its songs as they were kept, not made again one by one)
            let qp = Arc::as_ptr(&self.queue) as usize;
            let queue: Vec<_> = match self.store.last.take() {
                Some(l) if self.last_q == qp && l.queue.len() == self.queue.len() => l.queue,
                _ => self.queue.iter().map(|q| self.version_of(q)).collect(),
            };
            self.last_q = qp;
            let index = self.cur.unwrap_or(0);
            self.store.last = Some(Last { version: v, pos: self.pos(), speed: self.speed, queue, index });
            self.store_dirty = true;
            self.last_cap = Instant::now();
        }
    }

    /// Tempo the metronome clicks at right now (beats per real minute).
    pub(crate) fn metro_tempo(&self, playing: bool) -> f32 {
        // its own tempo: the song's BPM and the band's are set apart from it
        let base = self.mt_bpm as f32 + self.mt_add;
        if self.mt.follow && self.practice && playing {
            base * self.speed as f32 / 100.0
        } else {
            base
        }
    }

    /// Keep the metronome and the band going: tempo changes, speed-up drill, locking to the track.
    pub(crate) fn update_metro(&mut self, playing: bool) {
        self.band_tick();
        let tid = self.cur_track().map(|t| t.id);
        let ratio = if self.practice && playing { self.speed as f32 / 100.0 } else { 1.0 };
        // the song's beats known and the song at its own speed: the clicks go into the song itself (see below)
        let in_song = self.mt.on && self.mt.sync && playing && (ratio - 1.0).abs() < 0.001
            && self.beat.is_some_and(|b| Some(b.0) == tid)
            && self.beat_grid.as_ref().is_some_and(|g| Some(g.0) == tid);
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
        // locking needs the track's beat: look for it by itself once the audio is stored
        let sync_wanted = self.mt.sync && !tid.is_some_and(|t| self.beat_failed.contains(&t));
        if sync_wanted && playing && !self.beat_busy && self.beat.map(|b| b.0) != tid {
            if let Some(id) = tid {
                let stored = cache::has(id) || matches!(self.srcmap.get(&id), Some(Src::File(_)));
                if stored {
                    self.opt(Opt::FindBeats);
                }
            }
        }
        let locked = sync_wanted && playing && self.beat.is_some_and(|b| Some(b.0) == tid);
        if sync_wanted && !locked {
            // locking was asked for but there is nothing to lock to yet
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
            // (the song's beats known: the clicks are laid on them ahead of time, so only a seek, or nearing the end
            // of what is laid out, lays them again - never a re-aim in the middle, which would stutter)
            let gridded = self.beat_grid.as_ref().is_some_and(|g| Some(g.0) == tid);
            let jumped = (pos - (last + at.elapsed().as_secs_f32() * ratio)).abs() > 0.25;
            let due = if gridded {
                pos > self.mt_lead_end - 6.0 || pos < self.mt_lead_end - 60.0
            } else {
                self.mt_sync_at.elapsed().as_secs() >= 12
            };
            // (in the song, the audio side follows seeks and loops by itself)
            if (jumped || due) && !in_song {
                self.mt_gen += 1;
                self.mt_sync_at = Instant::now();
            }
            self.mt_pos = (pos, Instant::now());
        }
        // locked to a song whose beats are known: the local beat length and the next real beat, from the list (x2 and
        // /2 scale it as they scale the usual one)
        let grid_now = if locked {
            self.beat_grid.as_ref().filter(|g| Some(g.0) == tid).and_then(|(_, grid, usual)| {
                let scale = self.beat.map_or(1.0, |b| b.1) / usual;
                band::next_beat(grid, self.pos()).map(|(next, local)| (next, local * scale))
            })
        } else {
            None
        };
        // (the usual beat length for what is sent: the local one changes every frame and would rebuild the click)
        let period = self.beat.map_or(0.5, |b| b.1);
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
            if let Some((_, period0, phase)) = self.beat {
                let pos = self.pos();
                let next = match grid_now {
                    Some((g, _)) => g,
                    None => phase + ((pos - phase) / period0).ceil().max(0.0) * period0,
                };
                let each = period / ratio;
                wait = ((next - pos).max(0.0) / ratio + self.mt.shift_ms as f32 / 1000.0).rem_euclid(each);
                lead = vec![0i16; (wait * band::RATE) as usize];
            }
        }
        let mut body = band::click_loop(&cfg, tempo);
        // the song's beats known and the song at its own speed: the clicks go into the song itself, on its beats, so
        // nothing can drift; the separate click stream stays quiet. (Slowed down, the song is stretched, which would
        // smear a click inside it: then the laid-out stream below is used.)
        if in_song {
            if let Some((_, grid, _)) = self.beat_grid.as_ref() {
                let (hi, lo) = band::click_sounds();
                let beats: Vec<f64> = grid.iter().map(|b| *b as f64 + self.mt.shift_ms as f64 / 1000.0).collect();
                let n = self.mt.beats.max(1) as usize;
                let accent: Vec<bool> = (0..beats.len()).map(|k| k % n == 0).collect();
                self.player.ctl.set_song_clicks(Some(crate::player::SongClicks { beats, accent, hi, lo, gain: 0.8 }));
            }
            self.mt_vis = Some((Instant::now() + Duration::from_secs_f32(wait), period, 1));
            self.player.send(Cmd::Pcm(0, Vec::new(), Vec::new(), 0.0));
            return;
        }
        if locked {
            if let Some((_, grid, _)) = self.beat_grid.as_ref().filter(|g| Some(g.0) == tid) {
                let pos = self.pos();
                let shift = self.mt.shift_ms as f32 / 1000.0;
                let vol = 1.0;
                lead = band::grid_clicks(grid, pos, 40.0, ratio, shift, self.mt.beats.max(1), vol);
                self.mt_lead_end = pos + 40.0;
                wait = 0.0;
                if let Some((next, _)) = band::next_beat(grid, pos + 40.0) {
                    // the loop after it starts on the beat after the laid-out stretch
                    let gap = (((next - (pos + 40.0)) / ratio) * band::RATE) as usize;
                    lead.extend(std::iter::repeat_n(0i16, gap));
                }
                let _ = &mut body;
            }
        }
        let cycle =
            cfg.beats.clamp(1, 32) as usize * if cfg.gap_play > 0 { (cfg.gap_play + cfg.gap_mute.max(1)) as usize } else { 1 };
        self.mt_vis = Some((Instant::now() + Duration::from_secs_f32(wait), 60.0 / cfg.pulse(tempo), cycle));
        self.player.send(Cmd::Pcm(0, lead, body, 0.8));
    }

    // ------------------------------------------------------------------ band
    pub(crate) fn band_sig_now(&self) -> u64 {
        let bpm = (if self.bpm > 0 { self.bpm } else { self.mt_bpm }) as u64;
        let lv: u64 = self.band_levels().iter().enumerate().map(|(i, l)| ((l * 10.0).round() as u64) << (i * 5)).sum();
        // picking other bars to loop changes what the band plays
        let sel = self.chart_sel.map_or(0, |(a, b)| 1 + a as u64 * 997 + b as u64) << 48;
        bpm ^ (self.band_style as u64) << 12 ^ lv << 20 ^ (self.band_loops as u64) << 40 ^ sel
    }

    /// The bars the band plays: the ones dragged across on the lead sheet (in a loop), else the whole tune with its
    /// repeats and endings.
    pub(crate) fn band_bars(&self, chart: &crate::chart::Chart) -> Vec<usize> {
        match self.chart_sel {
            Some((a, b)) if a <= b && b < chart.bars.len() => (a..=b).collect(),
            _ => chart.play_order(),
        }
    }

    /// A song's new title and artist (files, YouTube, SoundCloud): everywhere Backline keeps it (your lists,
    /// liked songs, playlists, tunes, the queue, where you left off), and in the file's own tags for your files.
    pub(crate) fn save_info(&mut self, id: i64, title: String, artist: String) {
        let mut id = id;
        let (title, artist) = (title.trim().to_string(), artist.trim().to_string());
        if title.is_empty() {
            return;
        }
        // a file: its album too, and a new file name renames it on disk (the file keeps its place in Backline)
        let (album, new_name) = std::mem::take(&mut self.edit_extra);
        let mut file: Option<String> = None;
        if let Some(e) = self.store.files.iter().find(|e| e.id == id).cloned() {
            let old = PathBuf::from(&e.src);
            let want = new_name.trim();
            if !want.is_empty() && Some(want) != old.file_name().and_then(|n| n.to_str()) {
                let mut want = want.to_string();
                if let (false, Some(ext)) = (want.contains('.'), old.extension().and_then(|x| x.to_str())) {
                    want = format!("{}.{}", want, ext);
                }
                let new_path = old.with_file_name(&want);
                match std::fs::rename(&old, &new_path) {
                    Ok(()) => id = self.file_moved(id, &new_path),
                    Err(err) => self.set_err(format!("Couldn't rename the file: {}", err)),
                }
            }
        }
        for list in [&mut self.store.files, &mut self.store.yt, &mut self.store.sc, &mut self.store.hearts] {
            for e in list.iter_mut().filter(|e| e.id == id) {
                e.title = title.clone();
                e.artist = artist.clone();
                if e.kind == "file" {
                    e.album = album.clone();
                    file = Some(e.src.clone());
                }
            }
        }
        for e in self.yt_results.iter_mut().chain(self.sc_results.iter_mut()).filter(|e| e.id == id) {
            e.title = title.clone();
            e.artist = artist.clone();
        }
        let fix = |v: &mut store::Version| {
            if v.id == id {
                v.title = title.clone();
                v.artist = artist.clone();
                if v.kind == "file" {
                    v.album = album.clone();
                }
            }
        };
        for p in &mut self.store.playlists {
            p.items.iter_mut().for_each(fix);
        }
        for t in &mut self.store.tunes {
            t.versions.iter_mut().for_each(fix);
        }
        if let Some(l) = self.store.last.as_mut() {
            fix(&mut l.version);
        }
        let fix_track = |t: &mut Track| {
            if t.id == id {
                t.title = title.clone();
                t.artist = artist.clone();
                if file.is_some() {
                    t.album = album.clone();
                }
            }
        };
        Arc::make_mut(&mut self.queue).iter_mut().for_each(fix_track);
        if let Some(q) = self.orig_queue.as_mut() {
            q.iter_mut().for_each(fix_track);
        }
        self.store_dirty = true;
        self.serial += 1;
        self.set_note(&format!("RENAMED: {} - {}", artist, title));
        // your own file: its tags too, so other players see the new names
        if let Some(path) = file {
            let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
            std::thread::spawn(move || {
                let _ = tx.send(Msg::TagsSaved(write_tags(&path, &title, &artist, &album)));
                ctx.request_repaint();
            });
        }
    }

    /// A file of yours moved or renamed: its id (made from its path) changes, and everything that was kept under
    /// the old one - the file entry, likes, loops, bookmarks, key and tempo, playlists, tunes, the queue - follows.
    pub(crate) fn file_moved(&mut self, old: i64, new_path: &std::path::Path) -> i64 {
        let src = new_path.to_string_lossy().to_string();
        let new = store::hash_id(&format!("file:{}", src));
        if new == old {
            return old;
        }
        self.srcmap.insert(new, Src::File(new_path.to_path_buf()));
        crate::file_id(new, true);
        for list in [&mut self.store.files, &mut self.store.hearts] {
            for e in list.iter_mut().filter(|e| e.id == old) {
                e.id = new;
                e.src = src.clone();
            }
        }
        let fix = |v: &mut store::Version| {
            if v.id == old {
                v.id = new;
                v.src = src.clone();
            }
        };
        for p in &mut self.store.playlists {
            p.items.iter_mut().for_each(fix);
        }
        for t in &mut self.store.tunes {
            t.versions.iter_mut().for_each(fix);
        }
        for (v, _) in &mut self.store.recent {
            fix(v);
        }
        if let Some(l) = self.store.last.as_mut() {
            fix(&mut l.version);
        }
        if let Some(mut v) = self.store.songs.remove(&old) {
            fix(&mut v);
            self.store.songs.insert(new, v);
        }
        if let Some(v) = self.store.sections.remove(&old) {
            self.store.sections.insert(new, v);
        }
        if let Some(v) = self.store.marks.remove(&old) {
            self.store.marks.insert(new, v);
        }
        if let Some(v) = self.store.bpm.remove(&old) {
            self.store.bpm.insert(new, v);
        }
        if let Some(v) = self.store.meta.remove(&old) {
            self.store.meta.insert(new, v);
        }
        if let Some(v) = self.loops.remove(&old) {
            self.loops.insert(new, v);
        }
        let fix_track = |t: &mut Track| {
            if t.id == old {
                t.id = new;
            }
        };
        Arc::make_mut(&mut self.queue).iter_mut().for_each(fix_track);
        if let Some(q) = self.orig_queue.as_mut() {
            q.iter_mut().for_each(fix_track);
        }
        let _ = std::fs::rename(wave_path(old), wave_path(new));
        if let Some(e) = self.edit_info.as_mut().filter(|e| e.0 == old) {
            e.0 = new;
        }
        self.store_dirty = true;
        new
    }

    /// A song off this computer: your own file goes to the Recycle Bin (and out of Backline's lists), a streamed
    /// song's saved copy is deleted (it is downloaded again when played).
    pub(crate) fn delete_song(&mut self, t: &Track) {
        let name = t.title.to_uppercase();
        if let Some(Src::File(p)) = self.srcmap.get(&t.id).cloned() {
            if self.cur_track().map(|c| c.id) == Some(t.id) {
                self.player.send(Cmd::Stop);
            }
            match trash::delete(&p) {
                Ok(()) => {
                    self.file_gone(t.id);
                    self.set_note(&format!("MOVED TO THE RECYCLE BIN: {}", name));
                }
                Err(e) => self.set_err(format!("Couldn't delete {}: {}", p.display(), e)),
            }
        } else if cache::has(t.id) {
            cache::remove(t.id);
            self.set_note(&format!("SAVED COPY DELETED: {} (IT DOWNLOADS AGAIN WHEN PLAYED)", name));
        } else {
            self.set_note("THIS SONG ISN'T SAVED ON THIS COMPUTER");
        }
    }

    /// A file of yours that is no longer on disk: out of the file list, likes, playlists, tunes and the queue
    /// (its loops and notes stay, in case it comes back under the same path).
    pub(crate) fn file_gone(&mut self, id: i64) {
        crate::file_id(id, false);
        self.store.files.retain(|e| e.id != id);
        self.store.hearts.retain(|e| e.id != id);
        for p in &mut self.store.playlists {
            p.items.retain(|v| v.id != id);
        }
        for t in &mut self.store.tunes {
            t.versions.retain(|v| v.id != id);
        }
        self.store.recent.retain(|(v, _)| v.id != id);
        let was = self.cur.and_then(|c| self.queue.get(c).map(|t| t.id));
        Arc::make_mut(&mut self.queue).retain(|t| t.id != id);
        if let Some(q) = self.orig_queue.as_mut() {
            q.retain(|t| t.id != id);
        }
        if was == Some(id) {
            self.cur = None;
        } else if let Some(c) = self.cur {
            self.cur = self.queue.iter().position(|t| Some(t.id) == was).or(Some(c.min(self.queue.len().saturating_sub(1))));
        }
        self.srcmap.remove(&id);
        self.store_dirty = true;
        self.serial += 1;
    }

    /// The song's file, selected in the file manager: a file of yours, or a Tidal song stored here.
    pub(crate) fn show_file(&mut self, id: i64) {
        let path = match self.srcmap.get(&id) {
            Some(Src::File(p)) => Some(p.clone()),
            _ => cache::file_of(id),
        };
        let Some(path) = path.as_ref().filter(|p| p.exists()).cloned() else {
            self.set_err(match (path, self.srcmap.get(&id)) {
                (Some(p), Some(Src::File(_))) => format!("FILE NOT FOUND ({})", p.display()),
                _ => "CAN'T SHOW THE FILE: THIS SONG STREAMS AND ISN'T SAVED. TURN ON SAVING IN SETTINGS".to_string(),
            });
            return;
        };
        // (a path with spaces has to reach Explorer as /select,"path": quoted whole, Explorer opens Documents)
        if let Err(e) = crate::sources::reveal(&path.to_string_lossy()) {
            self.set_err(format!("Couldn't open the file manager: {}", e));
        }
    }

    /// Each part's level, with a muted part at 0.
    pub(crate) fn band_levels(&self) -> [f32; 3] {
        [0, 1, 2].map(|i| if self.band_parts[i] { self.band_lv[i] } else { 0.0 })
    }

    /// The backing track as a file: the count-in, the chart as many times as LOOPS says (once when endless),
    /// and an ending. Asks where to save it.
    pub(crate) fn band_export(&mut self) {
        let Some(text) = self.chart_text() else {
            self.set_note("NO CHART TO EXPORT YET");
            return;
        };
        let chart = crate::chart::parse_friendly(&text);
        if chart.bars.is_empty() {
            return;
        }
        let once = self.band_bars(&chart);
        let order: Vec<usize> = (0..self.band_loops.max(1)).flat_map(|_| once.iter().copied()).collect();
        let bpm = (if self.bpm > 0 { self.bpm } else { self.mt_bpm }) as f32;
        let (style, lv, beats) = (self.band_style, self.band_levels(), chart.beats);
        let name = self.chart_tune().map(|i| self.store.tunes[i].name.clone()).unwrap_or_else(|| "backing track".to_string());
        let file = format!(
            "{} - {} {} bpm x{}.wav",
            crate::extras::safe_name(&name),
            band::STYLES[style as usize % band::STYLES.len()].to_lowercase(),
            bpm.round(),
            self.band_loops.max(1)
        );
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let Some(path) = rfd::FileDialog::new()
                .set_title("Save the backing track")
                .set_file_name(&file)
                .add_filter("WAV audio", &["wav"])
                .save_file()
            else {
                return;
            };
            let mut pcm = band::count_in(beats, bpm);
            pcm.extend(band::render(&chart, &order, bpm, style, lv, true));
            let r = std::fs::write(&path, band::wav(&pcm)).map(|_| path.to_string_lossy().to_string()).map_err(|e| e.to_string());
            let _ = tx.send(Msg::BandSaved(r));
            ctx.request_repaint();
        });
    }

    pub(crate) fn band_start(&mut self) {
        let Some(text) = self.chart_text() else {
            self.set_note("NO CHART TO PLAY YET");
            return;
        };
        let chart = crate::chart::parse_friendly(&text);
        if chart.bars.is_empty() {
            return;
        }
        let order = self.band_bars(&chart);
        let bpm = (if self.bpm > 0 { self.bpm } else { self.mt_bpm }) as f32;
        // the parts are rendered again only when the chart, bars, tempo or style changed; levels just remix them
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (&text, &order, bpm.to_bits(), self.band_style).hash(&mut h);
            h.finish()
        };
        let stems = match &self.band_stems {
            Some((k, s)) if *k == key => s.clone(),
            _ => {
                let s = std::sync::Arc::new(band::render_parts(&chart, &order, bpm, self.band_style, false));
                self.band_stems = Some((key, s.clone()));
                s
            }
        };
        let mut body = band::mix_parts(&stems, self.band_levels());
        let bar = chart.beats as f32 * 60.0 / bpm;
        let starts = chart.starts(&order);
        let chorus_secs = starts.last().copied().unwrap_or(0.0) * 60.0 / bpm;
        // already playing: carry on from the same spot in the chart (no count-in, no jump back to the top)
        let resume = if self.band_on && self.band_order.len() == order.len() && !body.is_empty() {
            let t = self.band_t0.elapsed().as_secs_f32() - self.band_lead;
            let old_chorus = self.band_secs();
            (t > 0.0 && old_chorus > 0.0).then(|| {
                let passes = (t / old_chorus).floor();
                let frac = (t / old_chorus).fract();
                (passes, frac)
            })
        } else {
            None
        };
        let lead = match resume {
            Some((passes, frac)) => {
                let at = ((frac * body.len() as f32) as usize).min(body.len() - 1);
                body.rotate_left(at);
                let chorus = chorus_secs;
                // keep the clock in step: as if it had started that long ago at the new tempo
                self.band_t0 = Instant::now() - Duration::from_secs_f32(bar + (passes + frac) * chorus);
                Vec::new()
            }
            None => {
                self.band_t0 = Instant::now();
                band::count_in(chart.beats, bpm)
            }
        };
        self.band_bar = bar;
        self.band_beat = 60.0 / bpm.max(1.0);
        self.band_starts = starts;
        self.band_lead = bar;
        self.band_order = order;
        self.band_on = true;
        self.band_chart = text;
        self.band_sig = self.band_sig_now();
        self.player.send(Cmd::Pcm(1, lead, body, 0.9));
    }

    pub(crate) fn band_stop(&mut self) {
        if self.band_on {
            self.player.send(Cmd::Pcm(1, Vec::new(), Vec::new(), 0.0));
            self.band_on = false;
        }
    }

    /// How long the band takes to play its bars once through, in seconds.
    pub(crate) fn band_secs(&self) -> f32 {
        self.band_starts.last().copied().unwrap_or(0.0) * self.band_beat
    }

    /// Where the band is now: (position in its bar order, beats into that bar). None before it starts.
    pub(crate) fn band_where(&self) -> Option<(usize, f32)> {
        let t = self.band_t0.elapsed().as_secs_f32() - self.band_lead;
        let total = self.band_starts.last().copied().unwrap_or(0.0);
        if t < 0.0 || self.band_order.is_empty() || total <= 0.0 {
            return None;
        }
        let beat = (t / self.band_beat.max(0.01)) % total;
        let k = self.band_starts.windows(2).position(|w| beat >= w[0] && beat < w[1]).unwrap_or(0);
        Some((k, beat - self.band_starts[k]))
    }

    /// Restart the band when its tempo, style or parts changed; stop it after its LOOPS times through.
    pub(crate) fn band_tick(&mut self) {
        if self.band_on && self.band_sig != self.band_sig_now() {
            self.band_start();
        }
        if self.band_on && self.band_loops > 0 {
            let chorus = self.band_secs();
            let done = self.band_lead + chorus * self.band_loops as f32;
            if self.band_t0.elapsed().as_secs_f32() >= done {
                self.band_stop();
                self.set_note(&format!("THE BAND PLAYED IT {} TIMES", self.band_loops));
            }
        }
    }

    // ----------------------------------------------------------- track events
    /// A new track is about to start: restore its loop and tempo, reset per-track practice state.
    pub(crate) fn on_track_change(&mut self, t: &Track) {
        self.flush_practice();
        // equalizer AUTO: this song's own EQ, if one was kept (never for the tour's demo song)
        if self.eq_auto && !self.is_demo(t.id) {
            if let Some(g) = self.eq_songs.get(&t.id).copied() {
                self.eq_gains = g;
                self.eq_on = true;
                self.apply_eq();
            }
        }
        // a song with loops or bookmarks saved before songs were remembered: remember it now
        let marked = self.store.marks.get(&t.id).is_some_and(|l| !l.is_empty());
        if (self.store.sections.contains_key(&t.id) || marked || stems::is_split(t.id)) && !self.is_demo(t.id) {
            self.remember_song(t);
        }
        match self.loops.get(&t.id).copied().filter(|_| !self.is_demo(t.id)) {
            Some((a, b)) => {
                self.loop_a = Some(a);
                self.loop_b = Some(b);
                self.loop_on = self.practice;
            }
            None => {
                self.loop_a = None;
                self.loop_b = None;
                self.loop_on = false;
            }
        }
        self.new_current_loop();
        // the lead sheet follows the player (unless LOCKED): the new song's tune, or its changes looked up
        if !self.chart_lock {
            self.chart_follow(t.id);
        }
        self.semis = 0;
        // a different song starts at its own speed (the same one again, as on repeat, keeps yours)
        if self.wave.0 != t.id {
            self.speed = 100;
        }
        self.stem_on = [true; 4];
        // a new song shows whole
        self.wv_zoom = 1.0;
        self.wv_start = 0.0;
        self.passes = 0;
        self.pending_seek = None;
        self.sel_anchor = None;
        self.last_wraps = self.player.ctl.wraps();
        // the tempo you set for it, or its tune's, or the one Tidal gave or Backline heard
        self.bpm = self
            .store
            .bpm
            .get(&t.id)
            .copied()
            .or_else(|| self.store.tune_of(t.id).and_then(|i| self.tune_bpm(i)))
            .or_else(|| crate::meta::get(t.id).and_then(|m| m.bpm).map(u32::from))
            .unwrap_or(0);
        // (the same song again - repeat one, back to its start - keeps its outline: not worked out again)
        if self.wave.0 != t.id {
            self.wave = (t.id, None);
            self.wave_fine = (t.id, None);
        }
        self.sync_loop();
    }

    /// The audio was handed to the player.
    pub(crate) fn after_play(&mut self) {
        if let Some(t) = self.pending_seek.take() {
            self.player.send(Cmd::Seek(t));
        }
        self.last_wraps = self.player.ctl.wraps();
    }

    /// Tempo and key for a track Tidal told us nothing about: listened for once, then remembered.
    pub(crate) fn start_meta(&mut self, id: i64, bytes: &[u8]) {
        if !crate::meta::claim(id) {
            return;
        }
        let bytes = bytes.to_vec();
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Meta(id, crate::meta::detect(id, bytes)));
            ctx.request_repaint();
        });
    }

    /// The song's beat, worked out in the background as it starts (from the audio already here, streamed or not),
    /// so FIND BEAT answers at once for any song.
    pub(crate) fn start_beats(&mut self, id: i64, bytes: &[u8]) {
        if self.beat_known.contains_key(&id) || self.is_demo(id) {
            return;
        }
        let bytes = bytes.to_vec();
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::BeatsBg(id, band::detect_beats(bytes)));
            ctx.request_repaint();
        });
    }

    pub(crate) fn start_wave(&mut self, id: i64, bytes: &[u8]) {
        if self.wave.0 == id && self.wave.1.is_some() && self.wave_fine.0 == id && self.wave_fine.1.is_some() {
            return;
        }
        self.wave = (id, None);
        self.wave_fine = (id, None);
        // the envelope is kept on disk; the fine outline (for zooming in) is worked out again each time
        let mut cached = false;
        if let Ok(b) = std::fs::read(wave_path(id)) {
            if b.len() > 50 {
                self.wave = (id, Some(b));
                cached = true;
            }
        }
        let bytes = bytes.to_vec();
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let d = sources::wave_data(bytes);
            if !cached {
                let w = d.as_ref().map(|d| d.0.clone());
                if let Some(w) = &w {
                    let p = wave_path(id);
                    if let Some(d) = p.parent() {
                        let _ = std::fs::create_dir_all(d);
                    }
                    let _ = std::fs::write(&p, w);
                }
                let _ = tx.send(Msg::Wave(id, w));
            }
            if let Some((_, f)) = d {
                let _ = tx.send(Msg::WaveFine(id, f));
            }
            ctx.request_repaint();
        });
    }

    /// The note view for the song playing: from the stems switched on when it's split, else the whole song (read
    /// again in the background). Made once per song and set of stems.
    pub(crate) fn start_notes(&mut self, id: i64) {
        let split = stems::have_stems(id) && !self.is_demo(id);
        let on = self.stem_on;
        let mask = if split { on.iter().enumerate().fold(0u8, |m, (i, o)| m | ((*o as u8) << i)) } else { 0 };
        if self.notes.0 == id && self.notes.1 == mask && (self.notes_busy || self.notes.2.is_some()) {
            return;
        }
        // (tried already and couldn't: not again, every frame)
        if self.notes_tried == (id, mask) {
            return;
        }
        self.notes_tried = (id, mask);
        self.notes = (id, mask, None);
        self.notes_tex.clear();
        if split && mask == 0 {
            return;
        }
        self.notes_busy = true;
        let src = self.srcmap.get(&id).cloned();
        let have = self.playing_bytes(id);
        let (lossless, api, tx, ctx) = (self.prefer_lossless, self.api.clone(), self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let m = if split {
                stems::load(id).ok().map(|s| crate::notes::from_stems(&s, on))
            } else {
                let bytes = match have {
                    Some(b) => Ok(b.to_vec()),
                    None => crate::fetch_audio(&api, id, lossless, false, src),
                };
                bytes.ok().and_then(crate::notes::from_bytes)
            };
            let _ = tx.send(Msg::Notes(id, mask, m));
            ctx.request_repaint();
        });
    }

    // ------------------------------------------------------------ sources
    /// Every song you still have somewhere: your clips, liked songs, saved playlists, your playlists, tunes, the queue
    /// and the song left playing.
    pub(crate) fn songs_in_use(&self) -> HashSet<i64> {
        let mut u: HashSet<i64> = HashSet::new();
        u.extend(self.store.yt.iter().chain(self.store.sc.iter()).chain(self.store.hearts.iter()).map(|e| e.id));
        for l in &self.store.lists {
            u.extend(l.shown().iter().map(|e| e.id));
        }
        for p in &self.store.playlists {
            u.extend(p.items.iter().map(|v| v.id));
        }
        for t in &self.store.tunes {
            u.extend(t.versions.iter().map(|v| v.id));
        }
        if let Some(l) = &self.store.last {
            u.insert(l.version.id);
        }
        u.extend(self.queue.iter().map(|t| t.id));
        if let Some(t) = self.cur_track() {
            u.insert(t.id);
        }
        u
    }

    /// The saved copies of these YouTube and SoundCloud songs (Tidal's are kept as they play, by their own rule) that
    /// nothing has any more: deleted, so nothing is left behind on the disk. How many.
    pub(crate) fn drop_unused_copies(&mut self, ids: &[i64]) -> usize {
        let candidates: Vec<i64> = ids.iter().copied().filter(|&id| id < 0 && cache::has(id)).collect();
        if candidates.is_empty() {
            return 0;
        }
        let used = self.songs_in_use();
        let mut n = 0;
        for id in candidates.into_iter().filter(|id| !used.contains(id)) {
            cache::remove(id);
            n += 1;
        }
        if n > 0 {
            api::log(&format!("deleted {} saved copies nothing has any more", n));
        }
        n
    }

    /// With keeping songs on: these songs are kept one by one in the background, so they play at once and with no
    /// internet (the ones kept already, or waiting their turn, are skipped). How many were added to the line.
    pub(crate) fn keep_all(&self, ids: Vec<i64>) -> usize {
        if !self.keep_cache || self.go_offline {
            return 0;
        }
        let todo: Vec<(i64, Option<Src>)> = ids
            .into_iter()
            .filter(|id| !cache::has(*id) && !matches!(self.srcmap.get(id), Some(Src::File(_))))
            .map(|id| (id, self.srcmap.get(&id).cloned()))
            .collect();
        keep_queue(&self.api, todo)
    }

    /// With keeping songs on: every YouTube and SoundCloud song you have (brought in, in a playlist, liked) kept on
    /// this computer, in the background, so it plays at once and offline. Tidal songs are kept as they play.
    pub(crate) fn keep_everything(&mut self) -> usize {
        if !self.keep_cache {
            return 0;
        }
        for e in self.store.yt.clone().iter().chain(self.store.sc.clone().iter()).chain(self.store.hearts.clone().iter()) {
            self.register(&e.to_version());
        }
        let mut ids: Vec<i64> = Vec::new();
        ids.extend(self.store.hearts.iter().map(|e| e.id));
        ids.extend(self.store.yt.iter().map(|e| e.id));
        ids.extend(self.store.sc.iter().map(|e| e.id));
        for pl in &self.store.playlists {
            ids.extend(pl.items.iter().filter(|v| v.kind == "yt").map(|v| v.id));
        }
        let mut seen = HashSet::new();
        ids.retain(|id| matches!(self.srcmap.get(id), Some(Src::Yt(_))) && seen.insert(*id));
        // linked to another computer: only songs brought in here are kept on their own (ones that came in the sync
        // download when they're played)
        if crate::sync::linked() {
            ids.retain(|id| self.here.contains(id));
        }
        self.keep_all(ids)
    }

    /// SYNC: files that came from your other computer, read again (the plan's log at once: a tick before the next
    /// look would start from nothing and write that over it).
    pub(crate) fn sync_files(&mut self, names: Vec<String>) {
        for n in names {
            match n.as_str() {
                "plan.json" | "plan_log.json" | "plan_moves.json" | "plan_keepers.json" => self.plan.reread(),
                "atonal_lines.json" => self.at.seen = None,
                "reading_book.txt" if self.rd_book_loaded => self.rd_reload(),
                "pattern_book.txt" => crate::lines::reload_book(),
                _ => {}
            }
        }
    }

    /// SYNC: library parts that came from your other computer, put in place of these ones and saved at once (the
    /// sync reads library.json again only then). Lists may have changed under what's open.
    pub(crate) fn sync_library(&mut self, parts: Vec<(String, serde_json::Value)>) {
        let fresh = match serde_json::to_value(&self.store) {
            Ok(serde_json::Value::Object(mut m)) => {
                for (k, v) in parts {
                    m.insert(k, v);
                }
                serde_json::from_value::<store::Store>(serde_json::Value::Object(m))
                    .map_err(|e| api::log(&format!("sync: the library parts didn't fit: {}", e)))
                    .ok()
            }
            _ => None,
        };
        let ok = fresh.is_some();
        if let Some(s) = fresh {
            self.store = s;
            if self.pl_open.is_some_and(|i| i >= self.store.playlists.len()) {
                self.pl_open = None;
            }
            if self.tune_open.is_some_and(|i| i >= self.store.tunes.len()) {
                self.tune_open = None;
            }
            if self.entry_edit.is_some_and(|i| i >= self.store.entries.len()) {
                self.entry_edit = None;
            }
            self.register_all();
            self.liked.extend(self.store.hearts.iter().map(|e| e.id));
            crate::meta::load(&self.store.meta);
            // (a song taken out of everything on the other computer leaves no saved copy behind here)
            self.drop_unused_copies(&cache::ids());
            // (the day's notes read again, in case they came over too)
            self.diary_note = Default::default();
            // (the page stays where it is: the other computer practicing sends its time every few seconds, and a
            // page sent back to the top each time can't be scrolled)
            self.store.save();
            self.store_dirty = false;
            self.store_saved = Instant::now();
        }
        crate::sync::library_saved(ok);
    }

    pub(crate) fn handle_paths(&mut self, mut paths: Vec<PathBuf>) {
        // a pack dropped on the window opens as a pack, not as music
        if let Some(i) = paths.iter().position(|p| crate::pack::is_pack(p)) {
            let p = paths.remove(i);
            let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
            std::thread::spawn(move || {
                let _ = tx.send(Msg::PackIn(crate::pack::open(&p)));
                ctx.request_repaint();
            });
        }
        if paths.is_empty() {
            return;
        }
        self.files_busy = true;
        self.sec = Sec::Files;
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            FILES_STAGE.store(2, std::sync::atomic::Ordering::Relaxed);
            ctx.request_repaint();
            let (files, folders, lists) = scan_paths(paths);
            FILES_STAGE.store(0, std::sync::atomic::Ordering::Relaxed);
            let _ = tx.send(Msg::Added(files, folders, lists));
            ctx.request_repaint();
        });
    }

    pub(crate) fn pick_files(&mut self, folder: bool) {
        if self.files_busy {
            return;
        }
        self.files_busy = true;
        FILES_STAGE.store(1, std::sync::atomic::Ordering::Relaxed);
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let paths: Vec<PathBuf> = if folder {
                rfd::FileDialog::new().set_title("Add a folder of music").pick_folder().into_iter().collect()
            } else {
                let mut kinds: Vec<&str> = sources::AUDIO_EXT.to_vec();
                kinds.extend(["m3u", "m3u8", "pls"]);
                rfd::FileDialog::new()
                    .set_title("Add audio files or playlist files")
                    .add_filter("Audio and playlists", &kinds)
                    .pick_files()
                    .unwrap_or_default()
            };
            let (files, folders, lists) = scan_paths(paths);
            let _ = tx.send(Msg::Added(files, folders, lists));
            ctx.request_repaint();
        });
    }

    pub(crate) fn rescan(&mut self) {
        let folders = self.store.folders.clone();
        let known: HashSet<i64> = self.store.files.iter().map(|e| e.id).chain(self.store.removed_files.iter().copied()).collect();
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let mut found: Vec<PathBuf> = Vec::new();
            for f in &folders {
                sources::scan_folder(&PathBuf::from(f), 0, &mut found);
            }
            let new: Vec<Ext> = found
                .iter()
                .filter(|p| !known.contains(&store::hash_id(&format!("file:{}", p.to_string_lossy()))))
                .map(|p| sources::ext_for_file(p))
                .collect();
            if !new.is_empty() {
                let _ = tx.send(Msg::Scanned(new));
                ctx.request_repaint();
            }
        });
    }

    pub(crate) fn add_exts(&mut self, exts: Vec<Ext>) -> usize {
        let mut n = 0;
        for e in exts {
            if self.store.files.iter().any(|x| x.id == e.id) {
                continue;
            }
            self.register(&e.to_version());
            self.store.files.push(e);
            n += 1;
        }
        if n > 0 {
            self.store_dirty = true;
        }
        n
    }

    /// Only the songs' loops and the playlists ticked in the preview.
    pub(crate) fn share_ticked(&mut self, mut s: crate::store::Share) -> crate::store::Share {
        if let Some((songs, lists)) = self.share_pick.take() {
            s.loops.retain(|id, _| songs.contains(id));
            s.songs.retain(|id, _| songs.contains(id));
            s.playlists.retain(|p| lists.contains(&p.name));
        }
        s
    }

    /// Bring an import in (from its preview) and say what it added and what it found.
    pub(crate) fn take_import(&mut self, s: crate::store::Share) {
        let newer = s.version > crate::store::SHARE_VERSION;
        let t = self.store.take_share(s);
        self.store_dirty = true;
        self.serial += 1;
        let mut note =
            format!("IMPORTED: {} NEW LOOPS, {} NEW PLAYLISTS, {} SONGS ADDED TO PLAYLISTS", t.loops, t.playlists, t.items);
        if t.relinked > 0 {
            note += &format!(". {} FILES MATCHED TO YOURS", t.relinked);
        }
        if t.missing > 0 {
            note += &format!(". {} FILES NOT FOUND. ADD THEM IN MY FILES, THEN IMPORT AGAIN", t.missing);
        }
        if t.tidal > 0 && !self.api.has_token() {
            note += &format!(". {} TIDAL SONGS NEED A TIDAL LOGIN TO PLAY", t.tidal);
        }
        if newer {
            note += ". MADE WITH A NEWER VERSION OF BACKLINE. UPDATE TO IMPORT EVERYTHING";
        }
        self.set_note(&note);
    }

    // -------------------------------------------------------------- tunes
    pub(crate) fn add_version(&mut self, i: usize, v: Version) {
        if i >= self.store.tunes.len() {
            return;
        }
        self.register(&v);
        let t = &mut self.store.tunes[i];
        if t.versions.iter().any(|x| x.id == v.id) {
            let n = t.name.clone();
            self.set_note(&format!("ALREADY IN {}", n.to_uppercase()));
            return;
        }
        let n = t.name.clone();
        t.versions.push(v);
        self.store_dirty = true;
        self.set_note(&format!("ADDED TO {}", n.to_uppercase()));
    }

    /// Who wrote a tune and when, into the tune's empty fields (what you typed is kept).
    pub(crate) fn fill_details(&mut self, name: &str, w: &sources::Work) {
        let Some(i) = self.find_tune(name) else { return };
        let t = &mut self.store.tunes[i];
        if t.written_by.is_empty() && !w.writers.is_empty() {
            t.written_by = w.writers.clone();
            self.store_dirty = true;
        }
        if t.year.is_empty() && !w.year.is_empty() {
            t.year = w.year.clone();
            self.store_dirty = true;
        }
    }

    pub(crate) fn new_tune_named(&mut self, name: &str) -> usize {
        if let Some(i) = self.find_tune(name) {
            return i;
        }
        self.store.tunes.push(Tune { name: name.trim().to_string(), ..Default::default() });
        self.store_dirty = true;
        self.store.tunes.len() - 1
    }

    /// RECORD: a take starts (the player keeps what it plays, the input comes in) or stops - then you alone and you
    /// over what played are written as WAVs under takes/, and added to the tune as recordings.
    pub(crate) fn rec_toggle(&mut self) {
        if self.rec_buf.is_none() {
            // the last take not listened to yet: ask first
            if self.rec_last.is_some_and(|l| !l.1) && !self.rec_ask {
                self.rec_ask = true;
                return;
            }
            self.rec_ask = false;
            crate::player::rec_start_heard();
            self.rec_buf = Some(crate::record::start(self.rec_dev.clone()));
            self.rec_t0 = Some(Instant::now());
            if self.rec_band && !self.band_on {
                if let Some(i) = (self.rec_tune as usize).checked_sub(1) {
                    self.chart_pick = Some(i);
                }
                self.band_start();
            }
            return;
        }
        crate::record::stop();
        let buf = self.rec_buf.take();
        let t0 = self.rec_t0.take().unwrap_or_else(Instant::now);
        let secs = t0.elapsed().as_secs_f64();
        let kept = crate::player::rec_take();
        if self.rec_band && self.band_on {
            self.band_stop();
        }
        let (rate, samples, started) = match buf.as_ref().map(|b| b.lock().unwrap_or_else(|e| e.into_inner())) {
            Some(b) => (b.rate, b.samples.clone(), b.started),
            None => (44100, Vec::new(), None),
        };
        api::log(&format!("take stopped: {:.1} s, {} samples in at {} Hz", secs, samples.len(), rate));
        if samples.len() < rate as usize / 2 {
            self.set_note("NOTHING RECORDED. CHECK THE INPUT");
            return;
        }
        let you = crate::record::resample(&samples, rate, 44100);
        let back = kept.map(|r| crate::tour::mix(&r, t0, secs)).unwrap_or_default();
        let offset = started.map(|s| (s.saturating_duration_since(t0).as_secs_f64() * 44100.0) as i64).unwrap_or(0);
        let mixed = crate::record::over(&you, &back, offset);
        // the files, named by when and what
        let tune_i = (self.rec_tune as usize).checked_sub(1).filter(|i| *i < self.store.tunes.len());
        let tune_name = tune_i.map(|i| self.store.tunes[i].name.clone()).unwrap_or_else(|| "Free play".to_string());
        let (y, mo, d, h, mi, _) = crate::store::now_local();
        let stamp = format!("{:04}-{:02}-{:02} {:02}{:02}", y, mo, d, h, mi);
        let dir = api::config_dir().join("takes");
        let _ = std::fs::create_dir_all(&dir);
        let safe: String = tune_name.chars().map(|c| if c.is_alphanumeric() || c == ' ' { c } else { '-' }).collect();
        let p_mix = dir.join(format!("{} {}.wav", stamp, safe));
        let p_you = dir.join(format!("{} {} (you alone).wav", stamp, safe));
        if let Err(e) = std::fs::write(&p_mix, crate::tour::wav(&mixed, 44100, 2))
            .and_then(|_| std::fs::write(&p_you, crate::tour::wav(&you, 44100, 1)))
        {
            self.set_note(&format!("COULDN'T SAVE THE TAKE: {}", e));
            return;
        }
        // kept on its own list (not among the tune's recordings)
        let path = p_mix.to_string_lossy().to_string();
        let id = store::hash_id(&format!("file:{}", path));
        self.store.takes.push(store::Take {
            id,
            path,
            dry: p_you.to_string_lossy().to_string(),
            tune: tune_i.map(|_| tune_name.clone()).unwrap_or_default(),
            stamp,
            secs: you.len() as f32 / 44100.0,
            tag: String::new(),
        });
        self.store_dirty = true;
        self.rec_last = Some((id, false));
        api::log(&format!("take kept: {}", p_mix.display()));
        self.set_note(&format!("TAKE SAVED: {}", tune_name.to_uppercase()));
    }

    /// The last take, kept or deleted, then the new take starts.
    pub(crate) fn rec_last(&mut self, keep: bool) {
        if let (false, Some((id, _))) = (keep, self.rec_last) {
            self.take_delete(id);
        }
        self.rec_last = None;
        self.rec_ask = false;
        self.rec_toggle();
    }

    /// A take and its two files, gone (the player lets go of it first if it is playing).
    pub(crate) fn take_delete(&mut self, id: i64) {
        let Some(i) = self.store.takes.iter().position(|t| t.id == id) else { return };
        let t = self.store.takes.remove(i);
        if self.cur_track().map(|c| c.id) == Some(id) {
            self.player.send(Cmd::Stop);
        }
        for p in [&t.path, &t.dry] {
            let _ = std::fs::remove_file(p);
        }
        self.take_ask = None;
        self.store_dirty = true;
    }

    /// A take played back, with the practice tools.
    pub(crate) fn take_play(&mut self, id: i64, dry: bool) {
        let Some(t) = self.store.takes.iter().find(|t| t.id == id).cloned() else { return };
        if let Some(l) = self.rec_last.as_mut().filter(|l| l.0 == id) {
            l.1 = true;
        }
        let v = t.version(dry);
        self.register(&v);
        self.play_saved(&v, None);
    }

    /// Focus -> rest -> wait for you. No sound: the window just flashes.
    pub(crate) fn pomo_tick(&mut self) {
        let Some(end) = self.pomo_end else { return };
        if Instant::now() < end {
            return;
        }
        self.pomo_flash = Some(Instant::now());
        self.dirty = true;
        if self.pomo_sound {
            self.player.send(Cmd::Chime);
        }
        self.ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Informational));
        if self.pomo == 1 {
            let date = store::today_str();
            self.store.day_mut(&date).pomos += 1;
            self.store_dirty = true;
            if self.cur.is_some() && !self.stopped && !self.paused {
                self.apply(Action::PauseBtn);
            }
            if self.pomo_n >= self.pomo_cycles {
                self.pomo = 0;
                self.pomo_end = None;
                self.set_note(&format!("ALL {} FOCUS BLOCKS DONE. GOOD WORK", self.pomo_cycles));
            } else {
                self.pomo = 2;
                self.pomo_end = Some(Instant::now() + Duration::from_secs(self.pomo_break as u64 * 60));
                self.set_note(&format!("BREAK: {} MIN. STRETCH AND HAVE SOME WATER", self.pomo_break));
            }
        } else {
            self.pomo = 3;
            self.pomo_end = None;
            self.set_note(&format!("BREAK OVER. START FOCUS {} OF {} WHEN READY", self.pomo_n + 1, self.pomo_cycles));
        }
    }

    /// Play the mix of the stems that are switched on, from the same spot.
    pub(crate) fn apply_mix(&mut self, on: [bool; 4]) {
        let Some(t) = self.cur_track() else { return };
        if self.stopped || self.buffering {
            self.set_note("PLAY THE TRACK FIRST");
            return;
        }
        if !stems::have_stems(t.id) || self.is_demo(t.id) {
            self.set_note("SPLIT THIS TRACK FIRST");
            return;
        }
        // the four stems are read, mixed and let go: only the mix being played stays in memory
        match stems::load(t.id) {
            Ok(s) => {
                let bytes = stems::mix_wav(&s, on);
                drop(s);
                self.player.send(Cmd::Swap(bytes, self.pos(), self.paused));
                self.stem_on = on;
            }
            Err(e) => self.set_err(format!("STEMS: {}", e)),
        }
    }

    pub(crate) fn knob_val(&self, k: Knob) -> f32 {
        match k {
            Knob::Focus => self.pomo_focus as f32,
            Knob::Rest => self.pomo_break as f32,
            Knob::Blocks => self.pomo_cycles as f32,
            Knob::Loops => self.trainer_n as f32,
            Knob::Step => self.trainer_step as f32,
            Knob::Speed => self.speed as f32,
            Knob::LoopA => self.loop_a.unwrap_or(0.0),
            Knob::LoopB => self.loop_b.unwrap_or(0.0),
            Knob::Bpm => (if self.bpm > 0 { self.bpm } else { self.mt_bpm }) as f32,
            Knob::MetroBpm => self.mt_bpm as f32,
            Knob::Beats => self.mt.beats as f32,
            Knob::Unit => self.mt.unit as f32,
            Knob::GapPlay => self.mt.gap_play as f32,
            Knob::GapMute => self.mt.gap_mute as f32,
            Knob::RampBars => self.mt.ramp_bars as f32,
            Knob::RampBpm => self.mt.ramp_bpm as f32,
            Knob::Shift => self.mt.shift_ms as f32,
        }
    }

    pub(crate) fn knob_set(&mut self, k: Knob, v: f32) {
        let u = |lo: f32, hi: f32| v.round().clamp(lo, hi) as u32;
        match k {
            Knob::Focus => self.pomo_focus = u(1.0, 180.0),
            Knob::Rest => self.pomo_break = u(1.0, 60.0),
            Knob::Blocks => self.pomo_cycles = u(1.0, 16.0),
            Knob::Loops => self.trainer_n = u(1.0, 99.0),
            Knob::Step => self.trainer_step = u(1.0, 50.0),
            Knob::Speed => self.apply(Action::Speed(u(25.0, 250.0))),
            Knob::LoopA => self.apply(Action::SetAAt(v.max(0.0))),
            Knob::LoopB => self.apply(Action::SetBAt(v.max(0.0))),
            Knob::MetroBpm => {
                self.mt_bpm = u(30.0, 400.0);
                self.mt_add = 0.0;
                self.mt_step_at = Instant::now();
            }
            Knob::Bpm => {
                let b = u(30.0, 400.0);
                self.bpm = b;
                if self.cur.is_some() && !self.stopped {
                    self.set_bpm(b as i32);
                }
                // an open tune keeps the tempo you set for it
                if let Some(i) = self.tune_open.or_else(|| self.chart_tune()) {
                    if let Some(t) = self.store.tunes.get_mut(i) {
                        t.bpm = b;
                        self.store_dirty = true;
                    }
                }
                // the count-off clicks at the new tempo
                self.sync_loop();
            }
            Knob::Beats => {
                self.mt.beats = u(1.0, 32.0);
                self.mt.preset(0);
                self.mt_group = 0;
            }
            Knob::Unit => self.mt.unit = u(1.0, 64.0) as u8,
            Knob::GapPlay => self.mt.gap_play = u(0.0, 32.0),
            Knob::GapMute => self.mt.gap_mute = u(1.0, 16.0),
            Knob::RampBars => self.mt.ramp_bars = u(0.0, 32.0),
            Knob::RampBpm => self.mt.ramp_bpm = u(1.0, 30.0),
            Knob::Shift => {
                self.mt.shift_ms = v.round().clamp(-300.0, 300.0) as i32;
                self.mt_gen += 1;
            }
        }
    }

    pub(crate) fn opt(&mut self, o: Opt) {
        match o {
            Opt::MetroOn => {
                self.mt.on = !self.mt.on;
                self.mt_add = 0.0;
                self.mt_step_at = Instant::now();
                if !self.mt.on {
                    // silence it right now (the click loop plays forever until told to stop)
                    self.player.send(Cmd::Pcm(0, Vec::new(), Vec::new(), 0.0));
                    self.mt_vis = None;
                }
                self.mt_sig = 0;
            }
            Opt::Preset(n) => {
                self.mt.preset(n);
                self.mt_group = 0;
            }
            Opt::UnitSet(n) => self.mt.unit = n.clamp(1, 64),
            Opt::Group => {
                let gs = band::groupings(self.mt.beats);
                if gs.is_empty() {
                    return;
                }
                self.mt_group = (self.mt_group + 1) % (gs.len() + 1);
                match self.mt_group {
                    0 => self.mt.preset(0),
                    k => self.mt.apply_group(&gs[k - 1]),
                }
            }
            Opt::Level(i) => {
                self.mt_group = 0;
                let l = &mut self.mt.levels[i.min(31)];
                *l = if *l == 0 { 3 } else { *l - 1 };
            }
            Opt::Sub => self.mt.sub = (self.mt.sub + 1) % 4,
            Opt::Follow => self.mt.follow = !self.mt.follow,
            Opt::Sync => {
                self.mt.sync = !self.mt.sync;
                if self.mt.sync && self.beat.map(|b| b.0) != self.cur_track().map(|t| t.id) {
                    self.opt(Opt::FindBeats);
                }
            }
            Opt::Double | Opt::Half => {
                if let Some(b) = self.beat.as_mut() {
                    b.1 *= if matches!(o, Opt::Double) { 0.5 } else { 2.0 };
                    self.mt_gen += 1;
                }
            }
            Opt::FindBeats => {
                let Some(t) = self.cur_track() else {
                    self.set_note("PLAY A TRACK FIRST");
                    return;
                };
                if self.beat_busy {
                    return;
                }
                self.beat_failed.remove(&t.id);
                // worked out already as the song started (or on its way): no file needed
                if let Some(r) = self.beat_known.get(&t.id).cloned() {
                    self.beat_busy = true;
                    let _ = self.tx.send(Msg::Beats(t.id, r));
                    return;
                }
                if self.cur.is_some() && !self.stopped {
                    self.beat_wait = Some(t.id);
                    self.beat_busy = true;
                    self.set_note("FINDING THE BEAT...");
                    return;
                }
                let bytes = match self.srcmap.get(&t.id) {
                    Some(Src::File(p)) => std::fs::read(p).ok(),
                    _ => cache::get(t.id),
                };
                let Some(bytes) = bytes else {
                    self.set_note("PLAY THE TRACK ONCE FIRST, THEN FIND THE BEAT");
                    return;
                };
                self.beat_busy = true;
                self.set_note("FINDING THE BEAT...");
                let (tx, ctx, id) = (self.tx.clone(), self.ctx.clone(), t.id);
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::Beats(id, band::detect_beats(bytes)));
                    ctx.request_repaint();
                });
            }
            Opt::Band => {
                if self.band_on {
                    self.band_stop();
                } else {
                    self.band_start();
                }
            }
            Opt::BandStyleSet(i) => self.band_style = i,
            Opt::Part(i) => self.band_parts[i.min(2)] = !self.band_parts[i.min(2)],
            Opt::PartLevel(i, up) => {
                let l = &mut self.band_lv[i.min(2)];
                *l = ((*l + if up { 0.1 } else { -0.1 }) * 10.0).round().clamp(0.0, 15.0) / 10.0;
            }
            Opt::BandLoops(up) => {
                self.band_loops = if up { (self.band_loops + 1).min(99) } else { self.band_loops.saturating_sub(1) }
            }
            Opt::BandExport => self.band_export(),
            Opt::PomoSound => self.pomo_sound = !self.pomo_sound,
        }
    }

    pub(crate) fn set_bpm(&mut self, b: i32) {
        let b = b.clamp(30, 400) as u32;
        self.bpm = b;
        if let Some(t) = self.cur_track() {
            self.store.bpm.insert(t.id, b);
            if let Some(i) = self.store.tune_of(t.id) {
                if self.store.tunes[i].bpm == 0 {
                    self.store.tunes[i].bpm = b;
                }
            }
            self.store_dirty = true;
        }
        self.sync_loop();
    }

    pub(crate) fn chart_for(&mut self, text: &str) -> crate::chart::Chart {
        if self.chart_cache.0 != text {
            self.chart_cache = (text.to_string(), crate::chart::parse_friendly(text));
        }
        self.chart_cache.1.clone()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.flush_practice();
        self.capture_last();
        self.store.save();
    }
}

impl App {
    /// The saved external item (YouTube / SoundCloud / file) behind a negative track id.
    pub(crate) fn ext_of(&self, id: i64) -> Option<crate::store::Ext> {
        if id > 0 {
            return None;
        }
        let s = &self.store;
        s.yt.iter().chain(&s.sc).chain(&s.files).chain(&s.hearts).find(|e| e.id == id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_times() {
        assert_eq!(parse_time("1:23.5"), Some(83.5));
        assert_eq!(parse_time("83"), Some(83.0));
        assert_eq!(parse_time(" 0:05 "), Some(5.0));
        // the box shows its label ("A 1:23.4"): typing over it must not break
        assert_eq!(parse_time("A 13.3"), Some(13.3));
        assert_eq!(parse_time("B 2:05"), Some(125.0));
        assert_eq!(parse_time("13.3"), Some(13.3), "13.3 stays 13.3 (it used to land on 13.4)");
        for bad in ["", "abc", "-5", "1:xx", ":"] {
            assert_eq!(parse_time(bad), None, "{:?}", bad);
        }
    }

    #[test]
    fn tune_names() {
        assert_eq!(tune_name_from("Cherokee (Remastered 2005)"), "Cherokee");
        assert_eq!(tune_name_from("Giant Steps - Alternate Take"), "Giant Steps");
        assert_eq!(tune_name_from("So What [Live]"), "So What");
        assert_eq!(tune_name_from("  Autumn Leaves  "), "Autumn Leaves");
        assert_eq!(tune_name_from("(Untitled)"), "(Untitled)", "never empty");
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(safe_name("AC/DC: Back in Black?"), "AC_DC_ Back in Black_");
        assert_eq!(safe_name(" a<b>c|d*e \"f\" "), "a_b_c_d_e _f_");
        assert_eq!(safe_name("ok - name (1s-2s).wav"), "ok - name (1s-2s).wav");
        assert_eq!(
            loop_file_name(
                (2026, 10, 11, 0, 41, 5),
                "Average White Band",
                "Cut the Cake (Single Edit)",
                "FUNKY RIFF",
                5.2,
                12.9,
                false
            ),
            "2026-10-11 00-41-05 - Average White Band - Cut the Cake (Single Edit) - FUNKY RIFF (0m05s-0m12s).wav"
        );
        assert_eq!(
            loop_file_name((2026, 1, 2, 3, 4, 5), "AC/DC", "What?: \"Yes\"", "loop", 61.0, 70.0, true),
            "2026-01-02 03-04-05 - AC_DC - What__ _Yes_ - loop (1m01s-1m10s) stems.wav"
        );
    }

    #[test]
    fn tags_are_written_into_files() {
        use lofty::prelude::*;
        let dir = std::env::temp_dir().join("backline_tag_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("song.wav");
        std::fs::write(&path, crate::band::wav(&vec![0i16; 4410])).unwrap();
        let p = path.to_string_lossy().to_string();
        write_tags(&p, "New Title", "New Artist", "New Album").unwrap();
        let back = lofty::read_from_path(&p).unwrap();
        let tag = back.primary_tag().unwrap();
        assert_eq!(tag.title().as_deref(), Some("New Title"));
        assert_eq!(tag.artist().as_deref(), Some("New Artist"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn knob_steps() {
        assert_eq!(knob_step(Knob::LoopA), 0.2);
        assert_eq!(knob_step(Knob::Speed), 1.0);
    }
}

/// A tempo written in a tune's notes: a number with "bpm" next to it, else the first plain number that could be
/// a tempo (40 to 400), as in "the song is at 300".
pub(crate) fn tempo_in_notes(notes: &str) -> Option<u32> {
    let low = notes.to_lowercase();
    let mut nums: Vec<(usize, usize, u32)> = Vec::new();
    let b = low.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            let s = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            // part of a bigger token (1:23, 4/4, 3.5) is not a tempo
            let joined = |c: Option<&u8>| c.is_some_and(|c| matches!(c, b':' | b'/' | b'.' | b','));
            if !joined(s.checked_sub(1).and_then(|k| b.get(k)))
                && !(joined(b.get(i)) && b.get(i + 1).is_some_and(|c| c.is_ascii_digit()))
            {
                if let Ok(v) = low[s..i].parse::<u32>() {
                    nums.push((s, i, v));
                }
            }
        } else {
            i += 1;
        }
    }
    let ok = |v: u32| (40..=400).contains(&v);
    nums.iter()
        .find(|(s, e, v)| ok(*v) && (low[*e..].trim_start().starts_with("bpm") || low[..*s].trim_end().ends_with("bpm")))
        .or_else(|| nums.iter().find(|(_, _, v)| ok(*v)))
        .map(|n| n.2)
}

#[cfg(test)]
mod tempo_tests {
    use super::tempo_in_notes;

    #[test]
    fn tempos_from_notes() {
        assert_eq!(tempo_in_notes("the song is at 300 lol"), Some(300));
        assert_eq!(tempo_in_notes("bridge at 1:23, take it 240 bpm"), Some(240));
        assert_eq!(tempo_in_notes("32 bars, AABA, 4/4"), None);
        assert_eq!(tempo_in_notes("bpm: 180"), Some(180));
        assert_eq!(tempo_in_notes(""), None);
    }
}

/// Write a title and an artist into an audio file's own tags (ID3, Vorbis comments, MP4 atoms... whatever the file
/// uses; a tag is added when it has none).
pub(crate) fn write_tags(path: &str, title: &str, artist: &str, album: &str) -> Result<String, String> {
    use lofty::prelude::*;
    let mut tagged = lofty::read_from_path(path).map_err(|e| e.to_string())?;
    if tagged.primary_tag().is_none() {
        let kind = tagged.primary_tag_type();
        tagged.insert_tag(lofty::tag::Tag::new(kind));
    }
    let tag = tagged.primary_tag_mut().ok_or("no tag")?;
    tag.set_title(title.to_string());
    tag.set_artist(artist.to_string());
    if album.trim().is_empty() {
        tag.remove_album();
    } else {
        tag.set_album(album.to_string());
    }
    tag.save_to_path(path, lofty::config::WriteOptions::default()).map_err(|e| e.to_string())?;
    Ok(std::path::Path::new(path).file_name().map_or(path.to_string(), |n| n.to_string_lossy().to_string()))
}

/// The songs waiting to be kept (one download at a time, in the background), the ones already lined up since
/// Backline started (so a song that failed is not tried over and over), and whether the downloader is running.
pub(crate) type KeepLine = (std::collections::VecDeque<(i64, Option<Src>)>, HashSet<i64>, bool);
pub(crate) static KEEP: std::sync::LazyLock<std::sync::Mutex<KeepLine>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new((std::collections::VecDeque::new(), HashSet::new(), false)));
/// keeping songs is on (off stops the downloader after the song it is on)
pub(crate) static KEEP_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Add songs to the line to keep, and start the downloader if it is not running. How many were added.
pub(crate) fn keep_queue(api: &crate::api::Api, todo: Vec<(i64, Option<Src>)>) -> usize {
    use std::sync::atomic::Ordering;
    let Ok(mut k) = KEEP.lock() else { return 0 };
    let mut added = 0;
    for (id, src) in todo {
        if k.1.insert(id) {
            k.0.push_back((id, src));
            added += 1;
        }
    }
    if added > 0 && !k.2 {
        k.2 = true;
        let api = api.clone();
        std::thread::spawn(move || loop {
            let next = KEEP.lock().ok().and_then(|mut k| {
                let n = if KEEP_ON.load(Ordering::Relaxed) { k.0.pop_front() } else { None };
                if n.is_none() {
                    // stopped or done: the rest may be lined up again later
                    for (id, _) in std::mem::take(&mut k.0) {
                        k.1.remove(&id);
                    }
                    k.2 = false;
                }
                n
            });
            let Some((id, src)) = next else { break };
            if !cache::has(id) {
                if let Err(e) = crate::fetch_audio(&api, id, false, true, src) {
                    crate::api::log(&format!("keep {}: {}", id, e));
                }
            }
        });
    }
    added
}
