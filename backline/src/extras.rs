//! The logic behind the practice extras: files / YouTube sources, the tune list, the practice
//! diary, saved sections, the speed trainer, metronome and the rest. (The drawing is in views.rs.)

use crate::api::{self, Track};
use crate::band;
use crate::player::Cmd;
use crate::sources::{self, Src};
use crate::stems;
use crate::store::{self, Entry, Ext, Last, Section, Tune, Version};
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

fn knob_step(k: Knob) -> f32 {
    match k {
        Knob::Focus | Knob::Shift => 5.0,
        Knob::LoopA | Knob::LoopB => 0.2,
        _ => 1.0,
    }
}

fn wave_path(id: i64) -> PathBuf {
    api::config_dir().join("waves").join(format!("{}.pk", id))
}

/// Adding files: 0 nothing going on, 1 the file window is open, 2 reading what was picked.
pub(crate) static FILES_STAGE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Find the files below the given paths (folders are searched, and remembered for RESCAN).
/// The songs in these paths (folders searched), the folders, and the playlist files among them: each one's name
/// and its songs' ids (the songs come in with the rest).
fn scan_paths(paths: Vec<PathBuf>) -> (Vec<Ext>, Vec<String>, Vec<(String, Vec<i64>)>) {
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
fn tune_name_from(title: &str) -> String {
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
                    self.set_note(&format!("WINAMP SKIN: {}", s.name.to_uppercase()));
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
        let mut vs: Vec<Version> = Vec::new();
        vs.extend(self.store.files.iter().map(|e| e.to_version()));
        vs.extend(self.store.yt.iter().map(|e| e.to_version()));
        vs.extend(self.store.sc.iter().map(|e| e.to_version()));
        for t in &self.store.tunes {
            vs.extend(t.versions.iter().cloned());
        }
        if let Some(l) = &self.store.last {
            vs.push(l.version.clone());
        }
        for v in &vs {
            self.register(v);
        }
        if !self.store.folders.is_empty() {
            self.rescan();
        }
    }

    /// Remember where a non-Tidal recording comes from, so it can be played from any list.
    pub(crate) fn register(&mut self, v: &Version) {
        match v.kind.as_str() {
            "file" => {
                self.srcmap.insert(v.id, Src::File(PathBuf::from(&v.src)));
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
        if playing && self.last_cap.elapsed() > Duration::from_secs(10) {
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
    fn loop_now(&mut self) -> Option<String> {
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

    fn on_loops(&mut self, n: u32) {
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

    fn check_trainer(&mut self) {
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
            let queue: Vec<_> = self.queue.iter().map(|q| self.version_of(q)).collect();
            let index = self.cur.unwrap_or(0);
            self.store.last = Some(Last { version: v, pos: self.pos(), speed: self.speed, queue, index });
            self.store_dirty = true;
            self.last_cap = Instant::now();
        }
    }

    /// Tempo the metronome clicks at right now (beats per real minute).
    fn metro_tempo(&self, playing: bool) -> f32 {
        // its own tempo: the song's BPM and the band's are set apart from it
        let base = self.mt_bpm as f32 + self.mt_add;
        if self.mt.follow && self.practice && playing {
            base * self.speed as f32 / 100.0
        } else {
            base
        }
    }

    /// Keep the metronome and the band going: tempo changes, speed-up drill, locking to the track.
    fn update_metro(&mut self, playing: bool) {
        self.band_tick();
        let tid = self.cur_track().map(|t| t.id);
        let ratio = if self.practice && playing { self.speed as f32 / 100.0 } else { 1.0 };
        // the song's beats known and the song at its own speed: the clicks go into the song itself (see below)
        let in_song = self.mt.on && self.mt.sync && playing && (ratio - 1.0).abs() < 0.001
            && self.beat.map_or(false, |b| Some(b.0) == tid)
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
        let sync_wanted = self.mt.sync && !tid.map_or(false, |t| self.beat_failed.contains(&t));
        if sync_wanted && playing && !self.beat_busy && self.beat.map(|b| b.0) != tid {
            if let Some(id) = tid {
                let stored = cache::has(id) || matches!(self.srcmap.get(&id), Some(Src::File(_)));
                if stored {
                    self.opt(Opt::FindBeats);
                }
            }
        }
        let locked = sync_wanted && playing && self.beat.map_or(false, |b| Some(b.0) == tid);
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
                    lead.extend(std::iter::repeat(0i16).take(gap));
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
    fn band_sig_now(&self) -> u64 {
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
    fn file_moved(&mut self, old: i64, new_path: &std::path::Path) -> i64 {
        let src = new_path.to_string_lossy().to_string();
        let new = store::hash_id(&format!("file:{}", src));
        if new == old {
            return old;
        }
        self.srcmap.insert(new, Src::File(new_path.to_path_buf()));
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
        for m in [&mut self.store.sections].into_iter() {
            if let Some(v) = m.remove(&old) {
                m.insert(new, v);
            }
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
    fn delete_song(&mut self, t: &Track) {
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
            self.set_note("NOTHING OF THIS SONG IS SAVED ON THIS COMPUTER");
        }
    }

    /// A file of yours that is no longer on disk: out of the file list, likes, playlists, tunes and the queue
    /// (its loops and notes stay, in case it comes back under the same path).
    fn file_gone(&mut self, id: i64) {
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
    fn show_file(&mut self, id: i64) {
        let path = match self.srcmap.get(&id) {
            Some(Src::File(p)) => Some(p.clone()),
            _ => cache::file_of(id),
        };
        let Some(path) = path.filter(|p| p.exists()) else {
            self.set_note("NOT ON THIS COMPUTER");
            return;
        };
        #[cfg(windows)]
        let r = std::process::Command::new("explorer.exe").arg(format!("/select,{}", path.display())).spawn();
        #[cfg(target_os = "macos")]
        let r = std::process::Command::new("open").arg("-R").arg(&path).spawn();
        #[cfg(not(any(windows, target_os = "macos")))]
        let r = std::process::Command::new("xdg-open").arg(path.parent().unwrap_or(&path)).spawn();
        if let Err(e) = r {
            self.set_err(format!("Couldn't open the file manager: {}", e));
        }
    }

    /// Each part's level, with a muted part at 0.
    fn band_levels(&self) -> [f32; 3] {
        [0, 1, 2].map(|i| if self.band_parts[i] { self.band_lv[i] } else { 0.0 })
    }

    /// The backing track as a file: the count-in, the chart as many times as LOOPS says (once when endless),
    /// and an ending. Asks where to save it.
    fn band_export(&mut self) {
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

    fn band_start(&mut self) {
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
    fn band_tick(&mut self) {
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
        // a song with loops saved before songs were remembered: remember it now
        if (self.store.sections.contains_key(&t.id) || stems::is_split(t.id)) && !self.is_demo(t.id) {
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
        self.wave = (t.id, None);
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
        if self.wave.0 == id && self.wave.1.is_some() {
            return;
        }
        self.wave = (id, None);
        if let Ok(b) = std::fs::read(wave_path(id)) {
            if b.len() > 50 {
                self.wave = (id, Some(b));
                return;
            }
        }
        let bytes = bytes.to_vec();
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let w = sources::wave_peaks(bytes);
            if let Some(w) = &w {
                let p = wave_path(id);
                if let Some(d) = p.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let _ = std::fs::write(&p, w);
            }
            let _ = tx.send(Msg::Wave(id, w));
            ctx.request_repaint();
        });
    }

    // ------------------------------------------------------------ sources
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
        self.keep_all(ids)
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

    fn pick_files(&mut self, folder: bool) {
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
        let known: HashSet<i64> = self.store.files.iter().map(|e| e.id).collect();
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

    fn add_exts(&mut self, exts: Vec<Ext>) -> usize {
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
    fn take_import(&mut self, s: crate::store::Share) {
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
            note += &format!(". {} FILES AREN'T ON THIS COMPUTER. ADD THEM IN MY FILES, THEN IMPORT AGAIN", t.missing);
        }
        if t.tidal > 0 && !self.api.has_token() {
            note += &format!(". {} TIDAL SONGS NEED A TIDAL LOGIN TO PLAY", t.tidal);
        }
        if newer {
            note += ". IT WAS MADE BY A NEWER BACKLINE, SO UPDATE TO BRING IN EVERYTHING";
        }
        self.set_note(&note);
    }

    pub(crate) fn drain_extra(&mut self, m: Msg) {
        match m {
            Msg::CmImported(r) => match r {
                Ok(names) => {
                    self.cm_list = None;
                    self.set_note(&format!(
                        "{} TRANSCRIPTION{} BROUGHT IN",
                        names.len(),
                        if names.len() == 1 { "" } else { "S" }
                    ));
                }
                Err(e) => self.set_err(format!("IMPORT: {}", e)),
            },
            Msg::News(r) => match r {
                Ok(n) if !n.is_empty() => self.news = Some(n),
                Ok(_) => self.set_note("NO RELEASE NOTES FOUND"),
                Err(e) => self.set_err(format!("WHAT'S NEW: {}", e)),
            },
            Msg::Outputs(default, list, bt) => {
                let first = self.sys_output.is_empty() && self.out_list.is_empty();
                let old_list = std::mem::replace(&mut self.out_list, list);
                let old_bt = std::mem::replace(&mut self.out_bt, bt);
                let old_default = std::mem::replace(&mut self.sys_output, default.clone());
                // the first look is where things already play
                if first || default.is_empty() {
                    return;
                }
                // where it was playing, and where it plays now: the output you picked while it is there, else the
                // system's default
                let was = self.out_device.clone().filter(|d| old_list.contains(d)).unwrap_or(old_default);
                let picked = self.out_device.clone().filter(|d| self.out_list.contains(d));
                let now = picked.clone().unwrap_or_else(|| default.clone());
                if now == was {
                    return;
                }
                // Bluetooth headphones or a speaker gone (out of range, switched off): the music pauses rather than
                // carry on out loud; anything else, it keeps playing on the new output
                let bt_gone = !self.out_list.contains(&was) && old_bt.contains(&was);
                if bt_gone && self.cur.is_some() && !self.stopped && !self.paused {
                    self.apply(Action::PauseBtn);
                }
                let keep = self.out_device.clone();
                self.apply(Action::SetDevice(picked));
                self.out_device = keep;
                if bt_gone {
                    self.set_note(&format!("{} DISCONNECTED, SO PLAYBACK PAUSED", was.to_uppercase()));
                } else {
                    self.set_note(&format!("NOW PLAYING THROUGH {}", now.to_uppercase()));
                }
            }
            Msg::ShareSaved(r) => match r {
                Ok(p) => self.set_note(&format!("SAVED: {}. IMPORT IT IN BACKLINE ON THE OTHER COMPUTER", p)),
                Err(e) => self.set_err(format!("EXPORT: {}", e)),
            },
            Msg::ShareIn(Ok(s)) => self.share_view = Some((true, s)),
            Msg::PackIn(Ok(p)) => {
                if p.parts.is_empty() && p.share.is_none() {
                    self.set_note("NOTHING NEW IN THAT PACK: YOU HAVE IT ALL ALREADY");
                } else {
                    self.show_prefs = false;
                    self.pack_in = Some(p);
                }
            }
            Msg::PackIn(Err(e)) => self.set_err(format!("PACK: {}", e)),
            Msg::PackSaved(r) => match r {
                Ok(p) => self.set_note(&format!("PACK SAVED: {}. SEND IT, AND DROP IT ON BACKLINE TO OPEN IT", p)),
                Err(e) => self.set_err(format!("PACK: {}", e)),
            },
            Msg::ShareIn(Err(e)) => self.set_err(format!("IMPORT: {}", e)),
            Msg::TidalLists(r) => {
                self.tidal_lists_busy = false;
                match r {
                    Ok(l) => self.tidal_lists = Some(l),
                    Err(e) => {
                        self.tidal_lists = Some(Vec::new());
                        self.set_err(format!("TIDAL PLAYLISTS: {}", e));
                    }
                }
            }
            Msg::TidalListMade(r) => match r {
                Ok(name) => {
                    self.set_note(&format!("MADE {} ON TIDAL", name.to_uppercase()));
                    self.tidal_lists = None;
                    self.apply(Action::RefreshLibrary);
                }
                Err(e) => self.set_err(format!("TIDAL PLAYLIST: {}", e)),
            },
            Msg::TidalListDone(r) => match r {
                Ok(note) => self.set_note(&note),
                Err(e) => self.set_err(format!("TIDAL PLAYLIST: {}", e)),
            },
            Msg::Added(files, folders, lists) => {
                self.files_busy = false;
                let (found, picked_folder) = (files.len(), !folders.is_empty());
                for f in folders {
                    if !self.store.folders.contains(&f) {
                        self.store.folders.push(f);
                    }
                }
                let n = self.add_exts(files);
                self.store_dirty = true;
                // playlist files: each a Backline playlist (in LISTS) of its songs, in order
                if !lists.is_empty() {
                    let mut names = Vec::new();
                    for (name, ids) in lists {
                        let items: Vec<Version> = ids
                            .iter()
                            .filter_map(|id| self.store.files.iter().find(|e| e.id == *id))
                            .map(|e| e.to_version())
                            .collect();
                        if items.is_empty() {
                            continue;
                        }
                        names.push(format!("{} ({} SONGS)", name.to_uppercase(), items.len()));
                        self.store.playlists.push(store::Playlist { name, items });
                    }
                    if !names.is_empty() {
                        self.set_note(&format!("PLAYLIST ADDED TO LISTS: {}", names.join(", ")));
                        return;
                    }
                }
                // say what happened: songs added, nothing there, already here, or nothing picked
                let note = if n > 0 {
                    format!("ADDED {} SONG{}", n, if n == 1 { "" } else { "S" })
                } else if found > 0 {
                    "THOSE SONGS ARE ALREADY IN YOUR FILES".to_string()
                } else if picked_folder {
                    "NO MUSIC FILES IN THAT FOLDER".to_string()
                } else {
                    "NOTHING ADDED".to_string()
                };
                self.set_note(&note);
            }
            Msg::Scanned(files) => {
                let n = self.add_exts(files);
                if n > 0 {
                    self.set_note(&format!("FOUND {} NEW FILES", n));
                }
            }
            Msg::YtAdded(r) => {
                self.yt_busy = false;
                match r {
                    Ok(e) => {
                        self.yt_msg = format!("ADDED: {}", e.title);
                        self.yt_in.clear();
                        self.register(&e.to_version());
                        if !self.store.yt.iter().any(|x| x.id == e.id) {
                            self.store.yt.insert(0, e);
                            self.store_dirty = true;
                        }
                    }
                    Err(e) => {
                        api::log(&format!("youtube: {}", e));
                        self.yt_msg = format!("ERROR: {}", api::plain(&e));
                    }
                }
            }
            Msg::YtList(url, r) => {
                self.yt_busy = false;
                match r {
                    Ok((name, list)) => {
                        self.yt_msg.clear();
                        self.yt_in.clear();
                        for e in &list {
                            self.register(&e.to_version());
                        }
                        // imported: kept with your playlists, and open on its own page
                        match self.store.lists.iter_mut().find(|l| l.url == url) {
                            Some(l) => l.items = list.clone(),
                            None => self.store.lists.push(store::SavedList {
                                kind: "yt".to_string(),
                                name: name.clone(),
                                url: url.clone(),
                                items: list.clone(),
                            }),
                        }
                        self.store_dirty = true;
                        self.keep_all(list.iter().map(|e| e.id).collect());
                        self.yt_results = list;
                        self.yt_title = name;
                        self.yt_open = true;
                        self.yt_cur = Some(url);
                    }
                    Err(e) => {
                        api::log(&format!("youtube playlist: {}", e));
                        self.yt_msg = format!("ERROR: {}", api::plain(&e));
                    }
                }
            }
            Msg::ScSets(r) => {
                self.sc_busy = false;
                match r {
                    Ok(list) => {
                        self.sc_msg = format!("{} PLAYLISTS. CLICK ONE TO OPEN IT", list.len());
                        self.sc_sets = list;
                    }
                    Err(e) => {
                        api::log(&format!("soundcloud sets: {}", e));
                        self.sc_msg = format!("ERROR: {}", api::plain(&e));
                    }
                }
            }
            Msg::Sc(r) => {
                self.sc_busy = false;
                match r {
                    Ok(list) => {
                        self.sc_msg = format!("{} TRACKS", list.len());
                        for e in &list {
                            self.register(&e.to_version());
                        }
                        // the real titles, covers and lengths: four at a time, shown as they arrive
                        let srcs: Vec<String> = list.iter().take(40).map(|e| e.src.clone()).collect();
                        for w in 0..4 {
                            let mine: Vec<String> = srcs.iter().skip(w).step_by(4).cloned().collect();
                            let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                            std::thread::spawn(move || {
                                for s in mine {
                                    if let Ok(e) = sources::yt_info(&s) {
                                        let _ = tx.send(Msg::ScMeta(e));
                                        ctx.request_repaint();
                                    }
                                }
                            });
                        }
                        // a playlist (a link with several songs): kept with your playlists, on its own page
                        if let Some(u) = self.sc_cur.clone().filter(|_| list.len() > 1) {
                            match self.store.lists.iter_mut().find(|l| l.url == u) {
                                Some(l) => l.items = list.clone(),
                                None => {
                                    let name = self
                                        .sc_sets
                                        .iter()
                                        .find(|(_, x)| *x == u)
                                        .map(|(t, _)| t.clone())
                                        .unwrap_or_else(|| sources::list_name(&u));
                                    self.store.lists.push(store::SavedList {
                                        kind: "sc".to_string(),
                                        name,
                                        url: u,
                                        items: list.clone(),
                                    });
                                }
                            }
                            self.store_dirty = true;
                            self.keep_all(list.iter().map(|e| e.id).collect());
                        }
                        self.sc_results = list;
                    }
                    Err(e) => {
                        api::log(&format!("soundcloud: {}", e));
                        self.sc_msg = format!("ERROR: {}", api::plain(&e));
                    }
                }
            }
            Msg::ScMeta(e) => {
                let kept = self.store.lists.iter_mut().flat_map(|l| l.items.iter_mut());
                for slot in self.sc_results.iter_mut().chain(self.store.sc.iter_mut()).chain(kept) {
                    if slot.id == e.id {
                        slot.title = e.title.clone();
                        slot.artist = e.artist.clone();
                        slot.dur = e.dur;
                        slot.cover = e.cover.clone();
                    }
                }
                if self.store.sc.iter().chain(self.store.lists.iter().flat_map(|l| l.items.iter())).any(|x| x.id == e.id) {
                    self.store_dirty = true;
                }
            }
            Msg::YtTool(r) => {
                self.yt_busy = false;
                match r {
                    Ok(p) => self.yt_msg = format!("YT-DLP READY: {}", p.display()),
                    Err(e) => {
                        api::log(&format!("yt-dlp download: {}", e));
                        self.yt_msg = format!("ERROR: {}", api::plain(&e));
                    }
                }
            }
            Msg::Wave(id, w) => {
                if self.wave.0 == id {
                    self.wave.1 = w;
                }
            }
            Msg::Exported(r) => {
                self.exporting = false;
                match r {
                    Ok(p) => self.set_note(&format!("SAVED: {}", p)),
                    Err(e) if e == "cancelled" => {}
                    Err(e) => self.set_err(format!("EXPORT FAILED: {}", e)),
                }
            }
            Msg::Stems(id, r) => {
                self.stem_busy = 0;
                match r {
                    Ok(()) => {
                        // remembered, so MY TRACKS > WITH STEMS can name it
                        if let Some(t) = self.queue.iter().find(|t| t.id == id).cloned() {
                            self.remember_song(&t);
                        }
                        self.set_note("STEMS READY. USE THEM IN THE STEMS TAB")
                    }
                    Err(e) if e == "cancelled" => self.set_note("SPLIT CANCELLED"),
                    Err(e) => self.set_err(format!("STEMS FAILED: {}", e)),
                }
            }
            Msg::StemTool(r) => {
                self.stem_busy = 0;
                match r {
                    Ok(()) => self.set_note("STEM TOOL READY. NOW SPLIT A TRACK"),
                    Err(e) => self.set_err(format!("STEM TOOL DOWNLOAD FAILED: {}", e)),
                }
            }
            Msg::Lookup(name, info, tracks) => {
                self.look_busy = false;
                match info {
                    Ok(w) => {
                        if w.writers.is_empty() {
                            self.set_note(&format!("NO WRITER LISTED FOR {}", w.title.to_uppercase()));
                        }
                        self.fill_details(&name, &w);
                    }
                    Err(e) => self.set_note(&format!("NO WRITER INFO FOUND ({})", e)),
                }
                self.look_for = name;
                self.look_tracks = tracks;
            }
            Msg::TuneDetails(name, r) => {
                if let Ok(w) = r {
                    self.fill_details(&name, &w);
                }
            }
            Msg::RecordFound(key, r) => match r {
                Ok(tracks) if !tracks.is_empty() => {
                    *self.store.listened.entry(key.clone()).or_insert(0) += 1;
                    self.store_dirty = true;
                    self.apply(Action::Play(tracks, 0));
                    self.set_note(&format!("LISTENING: {}", key.to_uppercase()));
                }
                Ok(_) => self.set_note(&format!("{}: NO SONGS ON TIDAL", key.to_uppercase())),
                Err(e) => self.set_note(&e.to_uppercase()),
            },
            Msg::Chart(name, r) => {
                self.chart_busy = false;
                // a recommended tune being looked at: its chart on the lead sheet, not one of your tunes
                if self.find_tune(&name).is_none() && self.preview_tune.as_deref() == Some(name.as_str()) {
                    match r {
                        Ok((chart, _, _)) => {
                            self.prac_chart = Some((name.to_uppercase(), chart));
                            self.chart_sel = None;
                            self.chart_tr = 0;
                            self.rtab = 1;
                            self.chart_sugg.clear();
                        }
                        Err((e, _)) => self.set_note(&format!("NO CHART FOR {}: {}", name.to_uppercase(), e.to_uppercase())),
                    }
                    return;
                }
                let Some(i) = self.find_tune(&name) else { return };
                match r {
                    Ok((chart, key, composer)) => {
                        let t = &mut self.store.tunes[i];
                        t.chart = chart;
                        if t.key.is_empty() {
                            t.key = key;
                        }
                        if t.written_by.is_empty() && !composer.is_empty() {
                            t.written_by = composer;
                        }
                        self.chart_sugg.clear();
                        self.store_dirty = true;
                        self.set_note(&format!("FOUND THE CHANGES FOR {}", name.to_uppercase()));
                    }
                    Err((e, sugg)) => {
                        self.chart_sugg = sugg;
                        self.set_note(&format!("NO CHART FOR {}: {}", name.to_uppercase(), e.to_uppercase()));
                    }
                }
            }
            Msg::UpdateFound(r) => match r {
                Ok(Some(info)) => {
                    if self.upd.as_ref().map_or(true, |u| u.build < info.build) {
                        self.upd = Some(info.clone());
                        self.upd_state = 1;
                        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                        std::thread::spawn(move || {
                            let _ = tx.send(Msg::UpdateStaged(crate::update::stage(&info)));
                            ctx.request_repaint();
                        });
                    }
                }
                Ok(None) => {
                    self.upd_note = if crate::update::build() == 0 {
                        "Updates are off. This copy has no build number, so it wasn't made by the release workflow.".to_string()
                    } else {
                        "Checked just now. You have the newest build.".to_string()
                    };
                }
                Err(e) => {
                    api::log(&format!("update check: {}", e));
                    self.upd_note = format!("Last check failed: {}", e);
                    self.upd_state = 3;
                }
            },
            Msg::Menu(id) => self.run_named(&id),
            Msg::Credits(r) => {
                if let Some(c) = &mut self.credits {
                    c.1 = Some(r);
                }
            }
            Msg::WaList(r, more) => {
                self.wa_busy = false;
                match r {
                    Ok(list) => {
                        self.wa_end = list.len() < 30;
                        if more {
                            self.wa_list.extend(list);
                        } else {
                            self.wa_list = list;
                        }
                    }
                    Err(e) => self.set_err(format!("WINAMP SKINS: {}", e)),
                }
            }
            Msg::WaSkin(r) => self.wear_skin(r, true),
            Msg::TagsSaved(r) => match r {
                Ok(name) => self.set_note(&format!("SAVED IN THE FILE TOO: {}", name)),
                Err(e) => self.set_err(format!("RENAMED IN BACKLINE, BUT THE FILE ITSELF COULD NOT BE CHANGED: {}", e)),
            },
            Msg::BandSaved(r) => match r {
                Ok(p) => self.set_note(&format!("SAVED: {}", p)),
                Err(e) => self.set_err(format!("COULD NOT SAVE THE BACKING TRACK: {}", e)),
            },
            Msg::TourSong(r) => match r {
                Ok(path) => self.tour_put_on(path),
                Err(e) => crate::api::log(&format!("tour song: {}", e)),
            },
            Msg::IntroFile(f) => {
                if f.is_some() {
                    self.intro_file = f;
                    self.intro = 2;
                    self.dirty = true;
                    self.apply(Action::Intro(4));
                }
            }
            Msg::FontReady(choice, r) => match r {
                Ok(_) => {
                    // kept on disk now: wear it if it is still the one wanted
                    let _ = choice;
                    self.apply_pl_font();
                    self.set_note("FONT READY");
                }
                Err(e) => self.set_err(format!("FONT: {}", e)),
            },
            Msg::Meta(id, found) => {
                crate::meta::set_detected(id, found);
                self.store.meta = crate::meta::saved();
                self.store_dirty = true;
                // the playing song's tempo just heard: the count-off can use it
                if self.bpm == 0 && self.cur_track().is_some_and(|t| t.id == id) {
                    if let Some(b) = crate::meta::get(id).and_then(|m| m.bpm) {
                        self.bpm = b as u32;
                        self.sync_loop();
                    }
                }
            }
            Msg::Tuning(id, r) => {
                self.tuning_busy = false;
                match r {
                    Some((c, conf)) if conf >= 0.04 => {
                        self.tuning = Some((id, c, conf));
                        let mut how = if c.abs() <= 4 {
                            "IT IS TUNED TO A440".to_string()
                        } else {
                            format!("THIS TRACK IS {} CENTS {} OF A440", c.abs(), if c > 0 { "SHARP" } else { "FLAT" })
                        };
                        if let Some(k) = crate::meta::get(id).and_then(|i| i.key) {
                            how += &format!(". KEY: {}", crate::meta::key_long(k).to_uppercase());
                        }
                        self.set_note(&how);
                    }
                    _ => self.set_note("COULDN'T TELL. THERE IS TOO LITTLE PITCHED MUSIC TO MEASURE"),
                }
            }
            Msg::UpdateStaged(r) => match r {
                Ok(()) => {
                    self.upd_state = 2;
                    self.set_note("UPDATE READY. CLICK THE BAR AT THE TOP");
                }
                Err(e) => {
                    api::log(&format!("update download: {}", e));
                    self.upd_state = 3;
                }
            },
            Msg::Live(q, r) => {
                self.chart_busy = false;
                if q == self.live_q {
                    match r {
                        Ok(found) => {
                            self.live = Some(found);
                            self.chart_sugg.clear();
                        }
                        Err((_, sugg)) => self.chart_sugg = sugg,
                    }
                }
            }
            Msg::BeatsBg(id, r) => {
                self.beat_known.insert(id, r.clone());
                // FIND BEAT was pressed before it was ready: answer it now
                if self.beat_wait == Some(id) {
                    self.beat_wait = None;
                    self.beat_busy = true;
                    let _ = self.tx.send(Msg::Beats(id, r.clone()));
                }
            }
            Msg::Beats(id, r) => {
                self.beat_busy = false;
                match r {
                    Some((period, phase, grid)) => {
                        self.beat_failed.remove(&id);
                        self.beat = Some((id, period, phase));
                        self.beat_grid = Some((id, grid, period));
                        self.mt_gen += 1;
                        self.set_note(&format!("FOUND THE BEAT: {:.0} BPM. NOW TURN ON LOCK", 60.0 / period));
                    }
                    None => {
                        self.beat_failed.insert(id);
                        self.mt_gen += 1;
                        self.set_note("NO STEADY BEAT FOUND, SO YOUR OWN TEMPO IS USED");
                    }
                }
            }
            _ => {}
        }
    }

    // -------------------------------------------------------------- tunes
    fn add_version(&mut self, i: usize, v: Version) {
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
    fn fill_details(&mut self, name: &str, w: &sources::Work) {
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
    fn rec_toggle(&mut self) {
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
            self.set_note("NOTHING CAME IN. CHECK THE INPUT");
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
        self.set_note(&format!("TAKE KEPT WITH {}", tune_name.to_uppercase()));
    }

    /// The last take, kept or deleted, then the new take starts.
    fn rec_last(&mut self, keep: bool) {
        if let (false, Some((id, _))) = (keep, self.rec_last) {
            self.take_delete(id);
        }
        self.rec_last = None;
        self.rec_ask = false;
        self.rec_toggle();
    }

    /// A take and its two files, gone (the player lets go of it first if it is playing).
    fn take_delete(&mut self, id: i64) {
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
    fn take_play(&mut self, id: i64, dry: bool) {
        let Some(t) = self.store.takes.iter().find(|t| t.id == id).cloned() else { return };
        if let Some(l) = self.rec_last.as_mut().filter(|l| l.0 == id) {
            l.1 = true;
        }
        let v = t.version(dry);
        self.register(&v);
        self.play_saved(&v, None);
    }

    pub(crate) fn apply_extra(&mut self, a: Action) {
        match a {
            Action::TrainerKeys(on) => {
                self.trainer_keys = on;
                self.keys_done = 0;
                self.passes = 0;
                if on && !self.trainer {
                    self.trainer = true;
                    self.practice = true;
                }
                self.set_note(if on {
                    "EVERY FEW PASSES THE LOOP GOES UP A 4TH, THROUGH ALL 12 KEYS"
                } else {
                    "EVERY FEW PASSES THE SPEED GOES UP"
                });
            }
            Action::PreviewTune(name) => {
                if let Some(i) = self.find_tune(&name) {
                    // one of yours: its own chart
                    self.apply(Action::OpenTune(Some(i)));
                    self.rtab = 1;
                    if self.store.tunes[i].chart.trim().is_empty() {
                        self.apply(Action::FindChart(i, None));
                    }
                    return;
                }
                if self.chart_busy {
                    return;
                }
                self.chart_busy = true;
                self.preview_tune = Some(name.clone());
                self.set_note(&format!("LOOKING UP {}", name.to_uppercase()));
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let r = sources::find_chart(&name);
                    let _ = tx.send(Msg::Chart(name, r));
                    ctx.request_repaint();
                });
            }
            Action::PlayRecord(artist, album) => {
                let key = format!("{} - {}", artist, album);
                self.set_note(&format!("FINDING {}", key.to_uppercase()));
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let r = api.find_album(&artist, &album);
                    let _ = tx.send(Msg::RecordFound(key, r));
                    ctx.request_repaint();
                });
            }
            Action::AddRecommended(name) => {
                let i = self.new_tune_named(&name);
                if self.prac_chart.as_ref().is_some_and(|c| c.0.eq_ignore_ascii_case(&name)) {
                    // the chart already on show comes with it
                    if let Some((_, c)) = self.prac_chart.take() {
                        self.store.tunes[i].chart = c;
                    }
                } else if self.store.tunes[i].chart.trim().is_empty() {
                    self.apply(Action::FindChart(i, None));
                }
                self.store_dirty = true;
                self.set_note(&format!("ADDED TO YOUR TUNES: {}", name.to_uppercase()));
            }
            Action::SetTune(kind, name) => {
                // picked for your set: one of your tunes now (learning), its chart looked up
                if name.is_empty() {
                    self.store.routine_set.remove(&kind);
                } else {
                    let i = self.new_tune_named(&name);
                    if self.store.tunes[i].chart.trim().is_empty() {
                        self.apply(Action::FindChart(i, None));
                    }
                    self.store.routine_set.insert(kind.clone(), self.store.tunes[i].name.clone());
                    self.set_note(&format!("{}: {}", kind, name.to_uppercase()));
                }
                self.store_dirty = true;
            }
            Action::RoutineDone(k) => {
                let date = store::today_str();
                let list = self.store.routine_done.entry(date).or_default();
                if list.contains(&k) {
                    list.retain(|x| *x != k);
                } else {
                    list.push(k);
                }
                self.store_dirty = true;
            }
            Action::ShowFile(id) => self.show_file(id),
            Action::FileGone(id) => self.file_gone(id),
            Action::RecToggle => self.rec_toggle(),
            Action::RecLast(keep) => self.rec_last(keep),
            Action::RecPlay(id, dry) => self.take_play(id, dry),
            Action::TakesPanel(open) => {
                self.takes_open = open;
                self.take_ask = None;
                self.ed.id = 0;
            }
            Action::TakeDelete(id) => self.take_delete(id),
            Action::Section(s) => {
                if matches!(
                    s,
                    Sec::Tunes | Sec::Diary | Sec::Loops | Sec::Reading | Sec::Chords | Sec::Lines | Sec::Melodies | Sec::Ear
                ) {
                    self.last_practice = s;
                    // coming over from a music page, a practice page turns practice mode on (its panel opens); between
                    // practice pages your Listen / Practice choice stays as you set it
                    let from_music = !matches!(
                        self.sec,
                        Sec::Tunes | Sec::Diary | Sec::Loops | Sec::Reading | Sec::Chords | Sec::Lines | Sec::Melodies | Sec::Ear
                    );
                    if crate::PRACTICE && !self.practice && from_music {
                        self.practice = true;
                        self.loop_on = self.loop_a.is_some() && self.loop_b.is_some();
                        self.sync_loop();
                    }
                } else {
                    self.last_music = s;
                }
                self.show_log = false;
                self.show_eq = false;
                self.show_cache = false;
                self.sec = s;
                self.serial += 1;
                if s != Sec::Tunes {
                    self.pick = None;
                }
                if s != Sec::Lists {
                    self.pl_pick = None;
                } else {
                    // saved file / YouTube / SoundCloud items need their sources known before they can play
                    let vs: Vec<Version> = self.store.playlists.iter().flat_map(|p| p.items.iter().cloned()).collect();
                    for v in &vs {
                        self.register(v);
                    }
                }
            }
            Action::AddFiles => self.pick_files(false),
            Action::AddFolder => self.pick_files(true),
            Action::Rescan => {
                self.rescan();
                self.set_note("LOOKING FOR NEW FILES...");
            }
            Action::ForgetFolder(i) => {
                if i < self.store.folders.len() {
                    self.store.folders.remove(i);
                    self.store_dirty = true;
                }
            }
            Action::AddYt => {
                if self.yt_busy {
                    return;
                }
                if sources::is_yt_list(&self.yt_in) {
                    if sources::ytdlp_path().is_none() {
                        self.yt_msg = "PRESS GET YT-DLP FIRST (ONE-TIME SETUP)".to_string();
                        return;
                    }
                    self.yt_busy = true;
                    self.yt_msg = "READING THE PLAYLIST...".to_string();
                    let url = sources::yt_list_url(&self.yt_in);
                    let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                    std::thread::spawn(move || {
                        let r = sources::yt_list(&url);
                        let _ = tx.send(Msg::YtList(url, r));
                        ctx.request_repaint();
                    });
                    return;
                }
                let Some(vid) = sources::yt_video_id(&self.yt_in) else {
                    self.yt_msg = "THAT DOESN'T LOOK LIKE A YOUTUBE LINK".to_string();
                    return;
                };
                if sources::ytdlp_path().is_none() {
                    self.yt_msg = "PRESS GET YT-DLP FIRST (ONE-TIME SETUP)".to_string();
                    return;
                }
                self.yt_busy = true;
                self.yt_msg = "READING THE VIDEO...".to_string();
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::YtAdded(sources::yt_info(&vid)));
                    ctx.request_repaint();
                });
            }
            Action::ScGo => {
                if self.sc_busy {
                    return;
                }
                let q = self.sc_in.trim().to_string();
                if q.is_empty() {
                    return;
                }
                if sources::ytdlp_path().is_none() {
                    self.sc_msg = "PRESS GET YT-DLP FIRST (ONE-TIME SETUP)".to_string();
                    return;
                }
                self.sc_busy = true;
                self.sc_cur = if q.starts_with("http") { Some(q.clone()) } else { None };
                self.sc_msg = "LOOKING...".to_string();
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let r = if q.starts_with("http") { sources::sc_list(&q) } else { sources::sc_search(&q) };
                    let _ = tx.send(Msg::Sc(r));
                    ctx.request_repaint();
                });
            }
            Action::ScSets => {
                if self.sc_busy {
                    return;
                }
                let q = self.sc_in.trim().to_string();
                if q.is_empty() {
                    self.sc_msg = "TYPE A USER NAME FIRST".to_string();
                    return;
                }
                if sources::ytdlp_path().is_none() {
                    self.sc_msg = "PRESS GET YT-DLP FIRST (ONE-TIME SETUP)".to_string();
                    return;
                }
                self.sc_busy = true;
                self.sc_msg = "LOOKING FOR PLAYLISTS...".to_string();
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::ScSets(sources::sc_sets(&q)));
                    ctx.request_repaint();
                });
            }
            Action::CopyName(name) => {
                self.ctx.output_mut(|o| o.copied_text = name.clone());
                self.set_note(&format!("COPIED: {}", name));
            }
            Action::DeleteSong(t) => self.delete_song(&t),
            Action::CopyLink(url) => {
                self.ctx.output_mut(|o| o.copied_text = url.clone());
                self.set_note(&format!("LINK COPIED: {}", url.trim_start_matches("https://")));
            }
            Action::ScOpenSet(url) => {
                self.sc_in = url;
                self.apply(Action::ScGo);
            }
            Action::ScBrowser => {
                use std::sync::atomic::Ordering;
                let n = (sources::SC_BROWSER.load(Ordering::Relaxed) + 1) % 4;
                sources::SC_BROWSER.store(n, Ordering::Relaxed);
                self.dirty = true;
                self.sc_msg = if n == 0 {
                    "BROWSER LOGIN OFF".to_string()
                } else {
                    format!("USING YOUR {} SIGN-IN. CLOSE THAT BROWSER IF IT FAILS.", sources::SC_BROWSERS[n as usize])
                };
            }
            Action::LookStyle => self.look_style = (self.look_style + 1) % 4,
            Action::PlaylistPick(t) => {
                self.set_note(&format!("PICK A PLAYLIST FOR: {}", t.title.to_uppercase()));
                self.pl_pick = Some(t);
                self.pl_loop = None;
                self.pl_tidal = false;
                self.apply(Action::Section(Sec::Lists));
            }
            Action::PlaylistPickTidal(t) => {
                self.set_note(&format!("PICK A TIDAL PLAYLIST FOR: {}", t.title.to_uppercase()));
                self.pl_pick = Some(t);
                self.pl_loop = None;
                self.pl_tidal = true;
                self.apply(Action::Section(Sec::Lists));
            }
            Action::TidalListNew(name) => {
                let name = name.trim().to_string();
                if name.is_empty() {
                    self.set_note("TYPE A NAME FOR THE PLAYLIST FIRST");
                    return;
                }
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let r = api.playlist_create(&name).map(|_| name);
                    let _ = tx.send(Msg::TidalListMade(r));
                    ctx.request_repaint();
                });
                self.tidal_new_name.clear();
                self.ed.id = 0;
            }
            Action::TidalListAdd(uuid, name, id) => {
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                let fresh = uuid.is_empty();
                if fresh {
                    // the new one shows in the list next time it is opened
                    self.tidal_lists = None;
                }
                std::thread::spawn(move || {
                    let r = if fresh { api.playlist_create(&name) } else { Ok(uuid) }
                        .and_then(|u| api.playlist_add(&u, id))
                        .map(|_| format!("ADDED TO {} ON TIDAL", name.to_uppercase()));
                    let _ = tx.send(Msg::TidalListDone(r));
                    ctx.request_repaint();
                });
                self.pl_pick = None;
                self.pl_loop = None;
                self.new_pl.clear();
            }
            Action::ShareExport => self.share_view = Some((false, self.store.share())),
            Action::PackMake => {
                let keys: Vec<&'static str> =
                    crate::pack::available().into_iter().map(|(k, _)| k).filter(|k| !self.pack_off.iter().any(|o| o == k)).collect();
                let share = if self.pack_loops { serde_json::to_string_pretty(&self.store.share()).ok() } else { None };
                if keys.is_empty() && share.is_none() {
                    return self.set_note("TICK SOMETHING TO PUT IN THE PACK");
                }
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let Some(path) = rfd::FileDialog::new()
                        .set_title("Save a pack for a friend")
                        .set_file_name("Backline pack.backline")
                        .add_filter("Backline pack", &["backline"])
                        .save_file()
                    else {
                        return;
                    };
                    let r = crate::pack::make(&path, &keys, share).map(|_| path.to_string_lossy().to_string());
                    let _ = tx.send(Msg::PackSaved(r));
                    ctx.request_repaint();
                });
            }
            Action::PackOpen => self.apply(Action::ShareImport),
            Action::PackUnlock => {
                let Some(p) = self.pack_in.take() else { return };
                match crate::pack::unlock(&p.parts) {
                    Ok(done) => {
                        let on = |k: &str| p.parts.iter().any(|x| x.on && x.key == k);
                        // what reads its file once: read it again
                        if on("reading") {
                            self.rd_reload();
                        }
                        if on("pattern") {
                            crate::lines::reload_book();
                        }
                        if on("melodies") {
                            self.cm_list = None;
                        }
                        if !done.is_empty() {
                            self.set_note(&format!("UNLOCKED: {}", done.join(", ")));
                        }
                        // the loops and playlists: the usual import window, to pick from
                        if let Some(s) = p.share {
                            self.share_view = Some((true, s));
                        }
                    }
                    Err(e) => self.set_err(format!("PACK: {}", e)),
                }
            }
            Action::Bookmark => {
                if let Some(t) = self.cur_track().filter(|t| !self.is_demo(t.id)) {
                    let at = self.pos();
                    self.add_mark(t.id, at);
                }
            }
            Action::News => {
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::News(crate::update::notes(5)));
                    ctx.request_repaint();
                });
            }
            Action::ShareTake => {
                if let Some((true, s)) = self.share_view.take() {
                    let s = self.share_ticked(s);
                    self.take_import(s);
                }
            }
            Action::ShareSave => {
                let Some((false, share)) = self.share_view.take() else { return };
                let share = self.share_ticked(share);
                let text = match serde_json::to_string_pretty(&share) {
                    Ok(t) => t,
                    Err(e) => return self.set_err(format!("EXPORT: {}", e)),
                };
                let (y, m, d, ..) = crate::store::now_local();
                let file = format!("Backline loops and playlists {:04}-{:02}-{:02}.backline", y, m, d);
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let Some(path) = rfd::FileDialog::new()
                        .set_title("Save your loops and playlists")
                        .set_file_name(&file)
                        .add_filter("Backline loops and playlists", &["backline"])
                        .save_file()
                    else {
                        return;
                    };
                    let r = std::fs::write(&path, text).map(|_| path.to_string_lossy().to_string()).map_err(|e| e.to_string());
                    let _ = tx.send(Msg::ShareSaved(r));
                    ctx.request_repaint();
                });
            }
            Action::ShareImport => {
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let Some(path) = rfd::FileDialog::new()
                        .set_title("Bring in loops and playlists")
                        .add_filter("Backline loops and playlists", &["backline", "json"])
                        .pick_file()
                    else {
                        return;
                    };
                    // a pack picked here opens as a pack
                    if crate::pack::is_pack(&path) {
                        let _ = tx.send(Msg::PackIn(crate::pack::open(&path)));
                        ctx.request_repaint();
                        return;
                    }
                    let r = std::fs::read_to_string(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|t| serde_json::from_str::<crate::store::Share>(&t).map_err(|e| e.to_string()))
                        .and_then(|s| {
                            if s.kind == "backline-share" {
                                Ok(s)
                            } else {
                                Err("that isn't a Backline loops and playlists file".to_string())
                            }
                        });
                    let _ = tx.send(Msg::ShareIn(r));
                    ctx.request_repaint();
                });
            }
            Action::PlaylistPickLoop(t, a, b, name) => {
                self.set_note(&format!("PICK A PLAYLIST FOR THE LOOP: {}", name.to_uppercase()));
                self.pl_pick = Some(t);
                self.pl_loop = Some((a, b, name));
                self.apply(Action::Section(Sec::Lists));
            }
            Action::PlaylistAdd(i, t) => {
                if i < self.store.playlists.len() {
                    let mut v = self.version_of(&t);
                    if let Some((a, b, name)) = self.pl_loop.take() {
                        v.loop_ab = Some((a, b));
                        v.loop_name = name;
                    }
                    self.store.playlists[i].items.push(v);
                    self.store_dirty = true;
                    let n = self.store.playlists[i].name.to_uppercase();
                    self.set_note(&format!("ADDED TO {}", n));
                }
                self.pl_pick = None;
                self.serial += 1;
            }
            Action::PlaylistNew(t) => {
                let typed = self.new_pl.trim().to_string();
                let name = if typed.is_empty() { format!("Playlist {}", self.store.playlists.len() + 1) } else { typed };
                let mut items: Vec<store::Version> = t.iter().map(|t| self.version_of(t)).collect();
                if let (Some((a, b, lname)), Some(v)) = (self.pl_loop.take(), items.first_mut()) {
                    v.loop_ab = Some((a, b));
                    v.loop_name = lname;
                }
                self.store.playlists.push(store::Playlist { name, items });
                self.new_pl.clear();
                self.pl_pick = None;
                self.pl_open = Some(self.store.playlists.len() - 1);
                self.store_dirty = true;
                self.serial += 1;
            }
            Action::PlaylistFromQueue => {
                if self.queue.is_empty() {
                    self.set_note("THE QUEUE IS EMPTY");
                    return;
                }
                let typed = self.new_pl.trim().to_string();
                let name = if typed.is_empty() { format!("Playlist {}", self.store.playlists.len() + 1) } else { typed };
                let items: Vec<Version> = self.queue.iter().map(|t| self.version_of(t)).collect();
                self.store.playlists.push(store::Playlist { name, items });
                self.new_pl.clear();
                self.store_dirty = true;
                self.set_note("QUEUE SAVED AS A PLAYLIST");
                self.serial += 1;
            }
            Action::PlaylistRemove(pi, ti) => {
                if let Some(p) = self.store.playlists.get_mut(pi) {
                    if ti < p.items.len() {
                        p.items.remove(ti);
                        self.store_dirty = true;
                    }
                }
            }
            Action::PlaylistMove(pi, ti, d) => {
                if let Some(p) = self.store.playlists.get_mut(pi) {
                    let to = ti as i32 + d;
                    if to >= 0 && (to as usize) < p.items.len() {
                        p.items.swap(ti, to as usize);
                        self.store_dirty = true;
                    }
                }
            }
            Action::PlaylistDelete(pi) => {
                if pi < self.store.playlists.len() {
                    self.store.playlists.remove(pi);
                    self.pl_open = None;
                    self.store_dirty = true;
                }
            }
            Action::PlaylistOpen(i) => {
                self.pl_open = i;
                self.serial += 1;
            }
            Action::FindTuning => {
                let Some(t) = self.cur_track() else {
                    self.set_note("PLAY A TRACK FIRST");
                    return;
                };
                if self.tuning_busy {
                    return;
                }
                let bytes = match self.srcmap.get(&t.id) {
                    Some(Src::File(p)) => std::fs::read(p).ok(),
                    _ => cache::get(t.id),
                };
                let Some(bytes) = bytes else {
                    self.set_note("PLAY IT ONCE FIRST SO IT IS STORED, THEN CHECK THE TUNING");
                    return;
                };
                self.tuning_busy = true;
                self.set_note("LISTENING FOR THE TUNING...");
                let (tx, ctx, id) = (self.tx.clone(), self.ctx.clone(), t.id);
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::Tuning(id, crate::tuning::detect(bytes)));
                    ctx.request_repaint();
                });
            }
            Action::SetMode(m) => self.set_mode(m),
            Action::QuitApp => {
                self.flush_practice();
                self.save_settings();
                self.store.save();
                std::process::exit(0);
            }
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
                if self.show_help {
                    self.show_prefs = false;
                }
            }
            Action::TogglePrefs => {
                self.show_prefs = !self.show_prefs;
                self.rebinding = None;
                if self.show_prefs {
                    self.show_help = false;
                }
            }
            Action::ApplyUpdate => {
                self.flush_practice();
                // the queue and the song come back after the restart
                self.capture_last();
                self.save_settings();
                self.store.save();
                match crate::update::apply() {
                    Ok(()) => std::process::exit(0),
                    Err(e) => {
                        api::log(&format!("update install: {}", e));
                        self.set_note("UPDATE FAILED. IT WILL TRY AGAIN");
                        self.upd_state = 3;
                    }
                }
            }
            Action::RemoveList(i) => {
                if i < self.store.lists.len() {
                    self.store.lists.remove(i);
                    self.store_dirty = true;
                }
            }
            Action::YtKeep(id) => {
                if let Some(e) = self.yt_results.iter().find(|e| e.id == id).cloned() {
                    if !self.store.yt.iter().any(|x| x.id == id) {
                        self.register(&e.to_version());
                        self.store.yt.insert(0, e);
                        self.store_dirty = true;
                    }
                    self.set_note("KEPT IN MY CLIPS");
                }
            }
            Action::ScKeep(id) => {
                if let Some(e) = self.sc_results.iter().find(|e| e.id == id).cloned() {
                    if !self.store.sc.iter().any(|x| x.id == id) {
                        self.store.sc.insert(0, e);
                        self.store_dirty = true;
                    }
                    self.set_note("KEPT IN MY SOUNDCLOUD");
                }
            }
            Action::GoOffline(on) => {
                self.go_offline = on;
                crate::api::OFFLINE.store(on, std::sync::atomic::Ordering::Relaxed);
                self.dirty = true;
                if !on {
                    // the covers that couldn't load offline come in now
                    self.images.retry_failed();
                }
                if on {
                    // as if there were no internet: your library as kept here, only what is on this computer
                    let _ = self.tx.send(crate::Msg::NoInternet);
                    self.set_note("OFFLINE MODE: ONLY WHAT'S ON THIS COMPUTER. GO ONLINE TO BRING TIDAL BACK");
                } else {
                    self.set_note("GOING ONLINE...");
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
            }
            Action::Offline(on) => {
                self.offline = on;
                self.dirty = true;
                if on {
                    self.sec = Sec::Files;
                } else {
                    self.sec = Sec::Tidal;
                }
                self.serial += 1;
            }
            Action::GetYtDlp => {
                if self.yt_busy {
                    return;
                }
                self.yt_busy = true;
                self.yt_msg = "DOWNLOADING YT-DLP...".to_string();
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::YtTool(sources::ytdlp_download()));
                    ctx.request_repaint();
                });
            }
            Action::RemoveExt(id) => {
                self.store.files.retain(|e| e.id != id);
                self.store.yt.retain(|e| e.id != id);
                self.store.sc.retain(|e| e.id != id);
                self.store_dirty = true;
            }

            // ---- tunes
            Action::PickTune(t) => {
                self.set_note(&format!("PICK A TUNE FOR: {}", t.title.to_uppercase()));
                self.pick = Some(t);
                self.sec = Sec::Tunes;
                self.tune_open = None;
                self.serial += 1;
            }
            Action::AddToTune(i, t) => {
                let v = self.version_of(&t);
                self.add_version(i, v);
                self.pick = None;
            }
            Action::NewTuneFrom(t) => {
                let name = tune_name_from(&t.title);
                let i = self.new_tune_named(&name);
                let v = self.version_of(&t);
                self.add_version(i, v);
                self.pick = None;
                self.tune_open = Some(i);
                self.chart_pick = Some(i);
                self.chart_live = false;
                self.sec = Sec::Tunes;
                self.serial += 1;
            }
            Action::TuneDetails(i) => {
                let Some(name) = self.store.tunes.get(i).map(|t| t.name.trim().to_string()) else { return };
                let artist = self.store.tunes[i].versions.first().map(|v| v.artist.clone());
                if name.is_empty() || !self.details_tried.insert(name.to_lowercase()) {
                    return;
                }
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let w = sources::lookup_work(&name, artist.as_deref());
                    let _ = tx.send(Msg::TuneDetails(name, w));
                    ctx.request_repaint();
                });
            }
            Action::NewTune => {
                let name = self.new_tune.trim().to_string();
                if name.is_empty() {
                    return;
                }
                let i = self.new_tune_named(&name);
                self.new_tune.clear();
                if let Some(t) = self.pick.take() {
                    let v = self.version_of(&t);
                    self.add_version(i, v);
                }
                self.tune_open = Some(i);
                // the lead sheet shows the new tune (blank until its changes are found or typed)
                self.chart_pick = Some(i);
                self.chart_live = false;
                self.serial += 1;
            }
            Action::OpenTune(i) => {
                self.tune_open = i;
                // a tune opened: the lead sheet is its again, the whole of it
                self.chart_sel = None;
                if i.is_some() {
                    self.prac_chart = None;
                }
                // the tune's own tempo for the metronome and the band (kept with it from now on)
                if let Some(b) = i.and_then(|i| self.tune_bpm(i)) {
                    self.mt_bpm = b;
                    self.mt_add = 0.0;
                    if self.cur.is_none() || self.stopped {
                        self.bpm = b;
                    }
                    if let Some(t) = i.and_then(|i| self.store.tunes.get_mut(i)) {
                        if t.bpm == 0 {
                            t.bpm = b;
                            self.store_dirty = true;
                        }
                    }
                }
                if i.is_some() {
                    self.chart_pick = i;
                    self.chart_live = false;
                }
                self.edit_ver = None;
                self.del_arm = false;
                self.chart_edit = false;
                self.ed.id = 0;
                self.serial += 1;
            }
            Action::RemoveTune(i) => {
                if !self.del_arm {
                    self.del_arm = true;
                    return;
                }
                self.del_arm = false;
                if i < self.store.tunes.len() {
                    self.store.tunes.remove(i);
                    self.store_dirty = true;
                }
                self.tune_open = None;
                self.edit_ver = None;
                self.serial += 1;
            }
            Action::RemoveVersion(i, vi) => {
                if let Some(t) = self.store.tunes.get_mut(i) {
                    if vi < t.versions.len() {
                        t.versions.remove(vi);
                        self.store_dirty = true;
                    }
                }
                self.edit_ver = None;
            }
            Action::PlayTune(i, shuffled) => {
                let vs: Vec<Version> = match self.store.tunes.get(i) {
                    Some(t) => t.versions.clone(),
                    None => return,
                };
                if vs.is_empty() {
                    self.set_note("NO RECORDINGS YET. ADD ONE FIRST");
                    return;
                }
                for v in &vs {
                    self.register(v);
                }
                let tracks: Vec<Track> = vs.iter().map(|v| v.to_track()).collect();
                if shuffled {
                    self.apply(Action::PlayShuffled(tracks));
                } else {
                    self.apply(Action::Play(tracks, 0));
                }
            }
            Action::AddCurrentToTune(i) => match self.cur_track() {
                Some(t) => {
                    let v = self.version_of(&t);
                    self.add_version(i, v);
                }
                None => self.set_note("PLAY A RECORDING FIRST"),
            },
            Action::FindChart(i, alt) => {
                let Some(t) = self.store.tunes.get(i) else { return };
                let name = t.name.clone();
                if self.chart_busy {
                    return;
                }
                self.chart_busy = true;
                self.chart_sugg.clear();
                let query = alt.unwrap_or_else(|| name.clone());
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::Chart(name, sources::find_chart(&query)));
                    ctx.request_repaint();
                });
            }
            Action::LiveChart(title, alt) => {
                if self.chart_busy {
                    return;
                }
                self.chart_busy = true;
                self.live_q = title.clone();
                self.live = None;
                self.chart_sugg.clear();
                let query = alt.unwrap_or_else(|| title.clone());
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::Live(title, sources::find_chart(&query)));
                    ctx.request_repaint();
                });
            }
            Action::SaveLive => {
                let (Some(t), Some((chart, key, composer))) = (self.cur_track(), self.live.clone()) else { return };
                let name = tune_name_from(&t.title);
                let i = self.new_tune_named(&name);
                let v = self.version_of(&t);
                self.add_version(i, v);
                let tu = &mut self.store.tunes[i];
                tu.chart = chart;
                tu.key = key;
                if tu.written_by.is_empty() && !composer.is_empty() {
                    tu.written_by = composer;
                }
                self.chart_live = false;
                self.chart_pick = Some(i);
                self.store_dirty = true;
                self.set_note("SAVED TO YOUR TUNES");
            }
            Action::ToggleNumerals => self.chart_rn = !self.chart_rn,
            Action::Status(i, n) => {
                if let Some(t) = self.store.tunes.get_mut(i) {
                    t.status = n.min(3);
                    self.store_dirty = true;
                }
            }
            Action::Knob(k, d) => {
                let v = self.knob_val(k) + d as f32 * knob_step(k);
                self.knob_set(k, v);
            }
            Action::SetKnob(k, text) if matches!(k, Knob::Beats) && text.contains('/') => {
                // a whole meter typed in one go, like 11/17
                let mut it = text.split('/').map(|s| s.trim().parse::<f32>().ok());
                match (it.next().flatten(), it.next().flatten()) {
                    (Some(b), Some(u)) => {
                        self.knob_set(Knob::Beats, b);
                        self.knob_set(Knob::Unit, u);
                    }
                    _ => self.set_note("TYPE A METER LIKE 11/17"),
                }
            }
            Action::SetKnob(k, text) => {
                let v = if matches!(k, Knob::LoopA | Knob::LoopB) {
                    parse_time(&text)
                } else {
                    text.trim().trim_end_matches('%').trim().parse::<f32>().ok()
                };
                match v {
                    _ if text.trim().is_empty() => {}
                    Some(v) => self.knob_set(k, v),
                    None => self.set_note("THAT IS NOT A NUMBER"),
                }
            }
            Action::Opt(o) => self.opt(o),
            Action::LookUp(i) => {
                if self.look_busy {
                    return;
                }
                let Some(t) = self.store.tunes.get(i) else { return };
                let name = t.name.clone();
                let known: Vec<String> = self
                    .store
                    .tunes
                    .iter()
                    .flat_map(|t| t.versions.iter().filter(|v| v.stars >= 3).map(|v| v.artist.clone()))
                    .collect();
                self.look_busy = true;
                self.look_for = name.clone();
                self.look_tracks.clear();
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                let want_style = self.look_style;
                // something typed beside LOOK UP is searched instead of the tune's own name
                let typed = self.look_q.trim().to_string();
                let tune_name = name.clone();
                // someone who recorded it, so a title other songs share finds the right one
                let artist = if typed.is_empty() {
                    self.store.tunes.get(i).and_then(|t| t.versions.first()).map(|v| v.artist.clone())
                } else {
                    None
                };
                let name = if typed.is_empty() { name } else { typed };
                std::thread::spawn(move || {
                    let info = sources::lookup_work(&name, artist.as_deref());
                    // a tune in the jazz standards list is jazz (and that list knows who wrote it)
                    let listed = sources::find_chart(&name).ok().map(|c| c.2);
                    let style = match want_style {
                        0 => u8::from(listed.is_some()),
                        1 => 1,
                        2 => 2,
                        _ => 0,
                    };
                    let composer = listed.unwrap_or_else(|| info.as_ref().map(|w| w.writers.clone()).unwrap_or_default());
                    let composer = composer.split(|c| c == ',' || c == '\n').next().unwrap_or("").replace(" (words)", "");
                    let tracks = api.popular(&name, &known, style, composer.trim()).unwrap_or_default();
                    let _ = tx.send(Msg::Lookup(tune_name, info, tracks));
                    ctx.request_repaint();
                });
            }

            // ---- sections and loops
            Action::SaveSection => {
                let (Some(a), Some(b)) = (self.loop_a, self.loop_b) else {
                    self.set_note("SET LOOP A AND B FIRST");
                    return;
                };
                let Some(t) = self.cur_track() else { return };
                if self.is_demo(t.id) {
                    self.set_note("NOT DURING THE TOUR");
                    return;
                }
                let typed = self.sec_name.trim().to_string();
                let list = self.store.sections.entry(t.id).or_default();
                // the same loop again: a new name renames it, no name just says so
                if let Some(s) = list.iter_mut().find(|s| (s.a - a).abs() < 0.05 && (s.b - b).abs() < 0.05) {
                    if typed.is_empty() || typed.eq_ignore_ascii_case(&s.name) {
                        let n = s.name.to_uppercase();
                        self.set_note(&format!("THIS LOOP IS ALREADY SAVED AS {}", n));
                    } else {
                        s.name = typed.clone();
                        self.sec_name.clear();
                        self.store_dirty = true;
                        self.set_note(&format!("RENAMED: {}", typed.to_uppercase()));
                    }
                    return;
                }
                let name = if typed.is_empty() { format!("LOOP {}", list.len() + 1) } else { typed };
                list.push(Section { name: name.clone(), a, b, saved: store::today_str(), ..Default::default() });
                self.keep_for_practice(t.id);
                self.remember_song(&t);
                // saving a loop is practice: the song is in this week's list
                let date = store::today_str();
                self.store.day_mut(&date).songs.entry(t.id).or_insert(0);
                self.sec_name.clear();
                self.store_dirty = true;
                self.set_note(&format!("SAVED: {}", name.to_uppercase()));
            }
            Action::GoSection(i) => {
                let Some(t) = self.cur_track() else { return };
                let Some(s) = self.store.sections.get(&t.id).and_then(|l| l.get(i)).cloned() else { return };
                self.practice = true;
                self.loop_a = Some(s.a);
                self.loop_b = Some(s.b);
                self.loop_on = true;
                self.loop_ident = None;
                self.loop_saved = Some((t.id, i));
                self.sync_loop();
                self.player.send(Cmd::Seek(s.a));
            }
            Action::PlaySaved(v, lp) => self.play_saved(&v, lp),
            Action::PlayAt(v, t) => {
                let here = self.cur_track().map(|t| t.id) == Some(v.id);
                self.play_saved(&v, None);
                if here || !self.buffering {
                    self.player.send(Cmd::Seek(t));
                } else {
                    self.pending_seek = Some(t);
                }
            }
            Action::EditLoop(id, i, name, note, group) => {
                if let Some(s) = self.store.sections.get_mut(&id).and_then(|l| l.get_mut(i)) {
                    if !name.trim().is_empty() {
                        s.name = name.trim().to_string();
                    }
                    s.note = note.trim().to_string();
                    s.group = group.trim().to_string();
                    self.store_dirty = true;
                }
                let g = group.trim().to_string();
                if !g.is_empty() && !self.store.groups().contains(&g) {
                    let mut l = self.store.groups();
                    l.push(g);
                    self.store.loop_groups = Some(l);
                }
            }
            Action::LoopGroup(id, i, group) => {
                if let Some(s) = self.store.sections.get_mut(&id).and_then(|l| l.get_mut(i)) {
                    s.group = group.trim().to_string();
                    self.store_dirty = true;
                }
            }
            Action::ForgetLoop(id, i) => {
                // (the saved loops are counted again: which one is current, found afresh)
                self.loop_saved = None;
                if let Some(l) = self.store.sections.get_mut(&id) {
                    if i < l.len() {
                        l.remove(i);
                        self.store_dirty = true;
                    }
                    if l.is_empty() {
                        self.store.sections.remove(&id);
                    }
                }
            }
            Action::DeleteSection(i) => {
                self.loop_saved = None;
                if let Some(t) = self.cur_track() {
                    if let Some(l) = self.store.sections.get_mut(&t.id) {
                        if i < l.len() {
                            l.remove(i);
                            self.store_dirty = true;
                        }
                        if l.is_empty() {
                            self.store.sections.remove(&t.id);
                        }
                    }
                }
            }

            // ---- practice tools
            Action::ToggleFocus => {
                self.focus_mode = !self.focus_mode;
                if self.focus_mode {
                    let has_chart = self.chart_tune().map(|i| !self.store.tunes[i].chart.is_empty()).unwrap_or(false);
                    self.rtab = if has_chart { 1 } else { 0 };
                }
                self.set_note(if self.focus_mode { "FOCUS: JUST THE PLAYER AND TOOLS" } else { "FULL LAYOUT" });
            }
            Action::Continue => {
                let Some(l) = self.store.last.clone() else { return };
                // the queue it was in when there is one, else just the song
                let (queue, index) = if l.queue.is_empty() { (vec![l.version.clone()], 0) } else { (l.queue.clone(), l.index) };
                for v in &queue {
                    self.register(v);
                }
                self.queue = Arc::new(queue.iter().map(|v| v.to_track()).collect());
                self.orig_queue = None;
                self.shuffle = false;
                self.practice = true;
                self.play_index(index.min(self.queue.len().saturating_sub(1)));
                // where you left off, at the speed you left it
                self.speed = if l.speed == 0 { 100 } else { l.speed.clamp(25, 250) };
                self.sync_loop();
                if l.pos > 3.0 {
                    let t = (l.pos - 2.0).max(0.0);
                    if self.buffering {
                        self.pending_seek = Some(t);
                    } else {
                        self.player.send(Cmd::Seek(t));
                    }
                }
            }
            Action::Trainer => {
                self.trainer = !self.trainer;
                self.passes = 0;
                if self.trainer {
                    self.practice = true;
                    self.set_note(&format!("EVERY {} PASSES THE SPEED GOES UP {}%", self.trainer_n, self.trainer_step));
                } else {
                    self.set_note("SPEED TRAINER OFF");
                }
                self.sync_loop();
            }
            Action::TapTempo => {
                let now = Instant::now();
                if let Some(l) = self.taps.last() {
                    if now.duration_since(*l) > Duration::from_millis(2000) {
                        self.taps.clear();
                    }
                }
                self.taps.push(now);
                if self.taps.len() > 8 {
                    self.taps.remove(0);
                }
                if self.taps.len() >= 3 {
                    let span = now.duration_since(self.taps[0]).as_secs_f32();
                    let n = (self.taps.len() - 1) as f32;
                    let heard = 60.0 / (span / n).max(0.05);
                    let factor = if self.practice { self.speed as f32 / 100.0 } else { 1.0 };
                    // tapping finds the music's tempo: the song's and the metronome's both take it
                    self.knob_set(Knob::Bpm, (heard / factor).round());
                    self.knob_set(Knob::MetroBpm, (heard / factor).round());
                } else {
                    self.set_note("KEEP TAPPING...");
                }
            }
            Action::CountIn => {
                self.count_in = match self.count_in {
                    0 => 2,
                    2 => 4,
                    _ => 0,
                };
                if self.count_in > 0 && self.bpm == 0 {
                    self.set_note("THE COUNT-OFF NEEDS A TEMPO. TAP IT OR SET THE BPM");
                }
                self.sync_loop();
            }
            Action::Chan => {
                self.chan = (self.chan + 1) % CHAN_NAMES.len() as u32;
                self.practice = true;
                self.sync_loop();
                self.set_note(&format!("EAR: {}", CHAN_NAMES[self.chan as usize]));
            }
            Action::Transpose(d) => {
                self.semis = (self.semis + d).clamp(-1200, 1200);
                if self.cur.is_some() {
                    self.practice = true;
                }
                self.sync_loop();
                self.set_note(&format!("PITCH {:+.2} SEMITONES (SPEED UNCHANGED)", self.semis as f32 / 100.0));
            }
            Action::Export => {
                // the A-B loop when there is one, else the whole song
                let (a, b) = match (self.loop_a, self.loop_b) {
                    (Some(a), Some(b)) if b > a => (a, b),
                    _ => (0.0, 0.0),
                };
                let whole = b <= a;
                let Some(t) = self.cur_track() else {
                    self.set_note("PLAY A SONG FIRST");
                    return;
                };
                if self.exporting {
                    return;
                }
                // as you hear it: speed, and in practice mode the pitch and ear mode; balance and the equalizer
                let fx = crate::player::Fx {
                    speed: self.speed,
                    cents: if self.practice { self.semis } else { 0 },
                    chan: if self.practice { self.chan } else { 0 },
                    balance: self.balance,
                    eq: (self.eq_gains, self.eq_pre, self.eq_on),
                };
                let (api, lossless, src) = (self.api.clone(), self.prefer_lossless, self.srcmap.get(&t.id).cloned());
                // what you hear: only the stems that are on, when the song has been split and some are off
                let stems = if self.stem_on.iter().any(|on| !on) && stems::have_stems(t.id) {
                    stems::load(t.id).ok().map(|s| (s.data, self.stem_on))
                } else {
                    None
                };
                // a folder next to Backline, a name that sorts by date and says what it is
                let dir = std::env::current_exe()
                    .ok()
                    .and_then(|e| e.parent().map(|p| p.join("Saved loops")))
                    .filter(|d| std::fs::create_dir_all(d).is_ok())
                    .unwrap_or_else(|| dirs::audio_dir().unwrap_or_else(api::config_dir).join("Saved loops"));
                let _ = std::fs::create_dir_all(&dir);
                let what = self
                    .store
                    .sections
                    .get(&t.id)
                    .and_then(|l| l.iter().find(|s| (s.a - a).abs() < 0.05 && (s.b - b).abs() < 0.05))
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| if whole { "whole song".to_string() } else { "loop".to_string() });
                let (fa, fb) = if whole { (0.0, t.duration) } else { (a, b) };
                let name = loop_file_name(store::now_local(), &t.artist, &t.title, &what, fa, fb, stems.is_some());
                let id = t.id;
                self.exporting = true;
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let Some(path) = rfd::FileDialog::new()
                        .set_title(if whole { "Save the song as a WAV" } else { "Save the loop as a WAV" })
                        .set_directory(&dir)
                        .set_file_name(&name)
                        .add_filter("WAV audio", &["wav"])
                        .save_file()
                    else {
                        let _ = tx.send(Msg::Exported(Err("cancelled".to_string())));
                        ctx.request_repaint();
                        return;
                    };
                    // the samples (the stems that are on, or the song itself; fetched if it isn't on this computer),
                    // then the player's own effects over them
                    let samples = match stems {
                        Some((data, on)) => Ok((sources::mix_stems_span(&data, on, a, b), 2u16, 44100u32)),
                        None => {
                            crate::fetch_audio(&api, id, lossless, false, src).and_then(|bytes| sources::decode_span(bytes, a, b))
                        }
                    };
                    let r = samples
                        .and_then(|(data, ch, rate)| {
                            let (out, ch, rate) = crate::player::render(rodio::buffer::SamplesBuffer::new(ch, rate, data), &fx);
                            sources::write_wav(&path, ch, rate, &out)
                        })
                        .map(|_| {
                            // show it in Explorer, so it is easy to find later
                            sources::reveal(&path.to_string_lossy());
                            path.display().to_string()
                        });
                    let _ = tx.send(Msg::Exported(r));
                    ctx.request_repaint();
                });
            }

            // ---- diary
            Action::SaveEntry => {
                let today = if self.diary_sel == 0 { store::today_str() } else { store::date_str(self.diary_sel) };
                let default_mins = self.store.secs_on(&today) / 60;
                let mins = self.f_mins.trim().parse::<u32>().unwrap_or(default_mins);
                let bpm = self.f_bpm.trim().parse::<u32>().unwrap_or(0);
                let tune = if self.f_tune.trim().is_empty() { self.practice_label() } else { self.f_tune.trim().to_string() };
                let note = self.f_note.trim().to_string();
                match self.entry_edit.take().filter(|i| *i < self.store.entries.len()) {
                    // an entry changed: it keeps its day
                    Some(i) => {
                        let e = &mut self.store.entries[i];
                        (e.tune, e.mins, e.bpm, e.note) = (tune, mins, bpm, note);
                    }
                    None => {
                        // a logged day with no tracked time still counts on the calendar
                        if mins > 0 && self.store.secs_on(&today) == 0 {
                            *self.store.day_mut(&today).secs.entry(tune.clone()).or_insert(0) += mins * 60;
                        }
                        self.store.entries.push(Entry { date: today, tune, mins, bpm, note });
                    }
                }
                self.f_mins.clear();
                self.f_bpm.clear();
                self.f_note.clear();
                self.f_tune.clear();
                self.show_form = false;
                self.ed.id = 0;
                self.store_dirty = true;
                self.set_note("LOGGED IN THE DIARY");
            }
            Action::RightTab(n) => self.rtab = n,
            Action::StemGet => {
                if self.stem_busy != 0 {
                    return;
                }
                self.stem_busy = 1;
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::StemTool(stems::download_tool()));
                    ctx.request_repaint();
                });
            }
            Action::StemSplit => {
                if self.stem_busy != 0 {
                    return;
                }
                let Some(t) = self.cur_track() else {
                    self.set_note("PLAY A TRACK FIRST");
                    return;
                };
                self.stem_busy = 2;
                self.set_note("SPLITTING. KEEP PRACTICING, IT TAKES A FEW MINUTES");
                let id = t.id;
                // the stored copy, or (not stored: storing tracks is off, or it never finished) the track fetched
                // again for the split
                let src = self.srcmap.get(&t.id).cloned();
                let (lossless, api) = (self.prefer_lossless, self.api.clone());
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    api::log(&format!("stems: split {} asked for", id));
                    let r = crate::fetch_audio(&api, id, lossless, false, src).and_then(|b| {
                        api::log(&format!("stems: {} bytes of audio, splitting", b.len()));
                        stems::separate(b, id)
                    });
                    api::log(&format!("stems: {} -> {:?}", id, r));
                    let _ = tx.send(Msg::Stems(id, r));
                    ctx.request_repaint();
                });
            }
            Action::StemCancel => stems::CANCEL.store(true, std::sync::atomic::Ordering::Relaxed),
            Action::StemToggle(i) => {
                if i < 4 {
                    let mut on = self.stem_on;
                    on[i] = !on[i];
                    self.apply_mix(on);
                }
            }
            Action::StemSolo(i) => {
                if i < 4 {
                    let mut on = [false; 4];
                    on[i] = true;
                    self.apply_mix(on);
                }
            }
            Action::StemAll => self.apply_mix([true; 4]),
            Action::StemClear => {
                let _ = std::fs::remove_dir_all(api::config_dir().join("stems"));
                stems::forget_split();
                self.stem_on = [true; 4];
                self.set_note("ALL STEMS DELETED");
            }
            Action::CmImport(own) => {
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let picked = if own {
                        rfd::FileDialog::new()
                            .set_title("Import transcriptions")
                            .add_filter("Backline transcription", &["json"])
                            .pick_files()
                    } else {
                        rfd::FileDialog::new().set_title("Import Guitar Pro files").add_filter("Guitar Pro", &["gp"]).pick_files()
                    };
                    let Some(files) = picked else { return };
                    let mut done = Vec::new();
                    let mut errs = Vec::new();
                    for f in files {
                        let stem = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        let r = if own {
                            crate::transcription::load(&f).and_then(|t| {
                                // its recording too, when it came with one beside it
                                let audio =
                                    (!t.audio.is_empty()).then(|| f.with_file_name(&t.audio)).and_then(|a| std::fs::read(a).ok());
                                crate::transcription::save_in(None, t, audio.as_deref())
                            })
                        } else {
                            std::fs::read(&f)
                                .map_err(|e| e.to_string())
                                .and_then(|b| crate::transcription::from_gp(&b, &stem))
                                .and_then(|(t, a)| crate::transcription::save_in(None, t, a.as_deref()))
                        };
                        match r {
                            Ok(p) => done.push(p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()),
                            Err(e) => errs.push(format!("{}: {}", stem, e)),
                        }
                    }
                    let _ = tx.send(crate::Msg::CmImported(if done.is_empty() { Err(errs.join("; ")) } else { Ok(done) }));
                    ctx.request_repaint();
                });
            }
            Action::PomoHold => {
                if matches!(self.pomo, 1 | 2) {
                    self.pomo_left = self.pomo_end.map_or(0, |e| e.saturating_duration_since(Instant::now()).as_secs());
                    self.pomo_was = self.pomo;
                    self.pomo = 4;
                    self.pomo_end = None;
                    self.set_note("FOCUS TIMER PAUSED. CONTINUE WHEN READY");
                } else if self.pomo == 4 {
                    self.pomo = self.pomo_was.clamp(1, 2);
                    self.pomo_end = Some(Instant::now() + Duration::from_secs(self.pomo_left.max(1)));
                    self.set_note("FOCUS TIMER CONTINUES");
                }
                self.dirty = true;
            }
            Action::Pomo => {
                self.dirty = true;
                if matches!(self.pomo, 1 | 2 | 4) {
                    self.pomo = 0;
                    self.pomo_end = None;
                    self.set_note("FOCUS TIMER STOPPED");
                } else {
                    self.pomo_n = if self.pomo == 3 { self.pomo_n + 1 } else { 1 };
                    self.pomo = 1;
                    self.pomo_end = Some(Instant::now() + Duration::from_secs(self.pomo_focus as u64 * 60));
                    self.set_note(&format!("FOCUS {} OF {}. ONE THING AT A TIME", self.pomo_n, self.pomo_cycles));
                }
                self.pomo_flash = None;
            }
            Action::TimerPanel => self.timer_open = !self.timer_open,
            Action::PlayAlong(name) => {
                let q: String = format!("{} backing track play along", name)
                    .bytes()
                    .map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{:02X}", b) })
                    .collect();
                let _ = webbrowser::open(&format!("https://www.youtube.com/results?search_query={}", q));
                self.set_note("OPENED YOUTUBE IN YOUR BROWSER");
            }
            Action::MetroPanel => self.metro_open = !self.metro_open,
            Action::ImportIreal(target) => {
                let text = self.ireal_in.trim().to_string();
                match crate::chart::parse_ireal_url(&text) {
                    Err(e) => self.set_err(format!("CAN'T READ THAT LINK: {}", e)),
                    Ok(songs) if songs.is_empty() => self.set_note("NO SONGS IN THAT LINK"),
                    Ok(songs) => {
                        let mut count = 0;
                        let single = songs.len() == 1;
                        let mut last = None;
                        for s in songs {
                            let idx = match target {
                                Some(t) if single && t < self.store.tunes.len() => t,
                                _ => self.new_tune_named(&s.title),
                            };
                            let tune = &mut self.store.tunes[idx];
                            tune.chart = crate::chart::to_friendly(&s.measures, &s.time);
                            if tune.key.is_empty() {
                                tune.key = s.key.clone();
                            }
                            if tune.bpm == 0 {
                                tune.bpm = s.bpm;
                            }
                            if tune.written_by.is_empty() && !s.composer.is_empty() {
                                tune.written_by = s.composer.clone();
                            }
                            if tune.style.is_empty() && !s.style.is_empty() {
                                tune.style = s.style.clone();
                            }
                            last = Some(idx);
                            count += 1;
                        }
                        self.store_dirty = true;
                        self.ireal_in.clear();
                        self.chart_edit = false;
                        self.ed.id = 0;
                        if let Some(i) = last {
                            self.tune_open = Some(i);
                        }
                        self.set_note(&format!("IMPORTED {} CHART{}", count, if count == 1 { "" } else { "S" }));
                    }
                }
            }
            _ => {}
        }
    }

    /// Focus -> rest -> wait for you. No sound: the window just flashes.
    fn pomo_tick(&mut self) {
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
                self.set_note(&format!("REST FOR {} MIN. STRETCH AND HAVE SOME WATER", self.pomo_break));
            }
        } else {
            self.pomo = 3;
            self.pomo_end = None;
            self.set_note(&format!("REST OVER. START FOCUS {} OF {} WHEN READY", self.pomo_n + 1, self.pomo_cycles));
        }
    }

    /// Play the mix of the stems that are switched on, from the same spot.
    fn apply_mix(&mut self, on: [bool; 4]) {
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

    fn knob_val(&self, k: Knob) -> f32 {
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

    fn knob_set(&mut self, k: Knob, v: f32) {
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

    fn opt(&mut self, o: Opt) {
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
                    self.set_note("LISTENING FOR THE BEAT...");
                    return;
                }
                let bytes = match self.srcmap.get(&t.id) {
                    Some(Src::File(p)) => std::fs::read(p).ok(),
                    _ => cache::get(t.id),
                };
                let Some(bytes) = bytes else {
                    self.set_note("PLAY IT ONCE FIRST SO IT IS STORED, THEN FIND THE BEAT");
                    return;
                };
                self.beat_busy = true;
                self.set_note("LISTENING FOR THE BEAT...");
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

    fn set_bpm(&mut self, b: i32) {
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
            let joined = |c: Option<&u8>| c.map_or(false, |c| matches!(c, b':' | b'/' | b'.' | b','));
            if !joined(s.checked_sub(1).and_then(|k| b.get(k)))
                && !(joined(b.get(i)) && b.get(i + 1).map_or(false, |c| c.is_ascii_digit()))
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
fn write_tags(path: &str, title: &str, artist: &str, album: &str) -> Result<String, String> {
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
type KeepLine = (std::collections::VecDeque<(i64, Option<Src>)>, HashSet<i64>, bool);
static KEEP: std::sync::LazyLock<std::sync::Mutex<KeepLine>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new((std::collections::VecDeque::new(), HashSet::new(), false)));
/// keeping songs is on (off stops the downloader after the song it is on)
pub(crate) static KEEP_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Add songs to the line to keep, and start the downloader if it is not running. How many were added.
fn keep_queue(api: &crate::api::Api, todo: Vec<(i64, Option<Src>)>) -> usize {
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
