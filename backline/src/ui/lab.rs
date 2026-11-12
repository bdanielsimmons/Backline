//! PROGRESSION LAB (CHORDS > HARMONY, experimental). Put the chords in first (grip each on the neck; the Chord
//! Analyzer names it), then TAP TIMING runs through them: a bar of clicks with four lights, the first chord comes in
//! on 1 and holds, and each tap (Space) moves to the next chord and plays it; one more tap ends it. The metronome
//! keeps going meanwhile. The chords show as bar rows, two bars a row, each block as long as its chord holds (drag
//! its right edge to fix one). Lengths read as note values on a grid ("a dotted 8th"), or seconds with Snap Off.
//! WRITTEN OUT draws it with the Chord Melodies notation (grips, staff, tab, in rhythm). A progression can be linked
//! to a song's loop: then TAP TIMING and PLAY can run over the record itself. The progression on the bench is
//! `Store::lab_bench`; the kept ones `Store::lab`.

use super::*;
use crate::fretboard::*;
use crate::store::{LabChord, LabLink, LabProg};
use crate::theory;
use crate::triads::{steps_events, Art, Step};
use crate::views::field;

/// SNAP's choices; a length moves by `snap_step` (Off still moves in 20ths of a beat, so an edge can be dragged).
pub(crate) const SNAPS: [&str; 4] = ["OFF", "8THS", "16THS", "TRIPLETS"];
const ARTS: [&str; 3] = ["STRUM", "HAMMER ON", "SLIDE IN"];
/// How it plays (and TAP TIMING runs) when it's linked to a loop: on its own (your chords and the click; a song
/// playing pauses), or the record's loop alone.
const PLAYS: [&str; 2] = ["UNACCOMPANIED", "WITH THE LOOP"];
/// a new chord's placeholder length, until it's timed
const PLACEHOLDER: f32 = 2.0;
/// how late a tap lands after the sound it answers (the sound's way out, and the hand)
const TAP_LATE: f32 = 0.05;

/// TAP TIMING under way.
pub(crate) struct LabRun {
    /// when it began, and the downbeat the first chord comes in on (None: along the record, the first tap starts it)
    pub start: Instant,
    pub down: Option<Instant>,
    /// the chord sounding (None: not started yet), and when each started
    pub cur: Option<usize>,
    pub at: Vec<Instant>,
    /// along the record: the song's speed (so lengths are put back to full speed)
    pub ratio: f32,
    /// each chord heard on the guitar as it comes
    pub guitar: bool,
}

pub(crate) fn snap_step(snap: u8) -> f32 {
    match snap {
        1 => 0.5,
        2 => 0.25,
        3 => 1.0 / 3.0,
        _ => 0.05,
    }
}

/// A length put on the grid (at least one step, at most 16 bars).
pub(crate) fn snap_to(beats: f32, snap: u8) -> f32 {
    let s = snap_step(snap);
    ((beats / s).round() * s).clamp(s, 64.0)
}

/// A length as note values ("DOTTED 8TH", "HALF + 8TH", "QUARTER + TRIPLET 8TH"), or None when it isn't one.
pub(crate) fn note_words(b: f32) -> Option<String> {
    let twelfths = (b * 12.0).round() as i32;
    if (b * 12.0 - twelfths as f32).abs() > 0.05 || twelfths <= 0 {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    if twelfths % 3 == 0 {
        // in 16ths
        const NAMES: [(i32, &str); 8] = [
            (16, "WHOLE"),
            (12, "DOTTED HALF"),
            (8, "HALF"),
            (6, "DOTTED QUARTER"),
            (4, "QUARTER"),
            (3, "DOTTED 8TH"),
            (2, "8TH"),
            (1, "16TH"),
        ];
        let mut left = twelfths / 3;
        while left > 0 {
            let (v, n) = NAMES.iter().find(|(v, _)| *v <= left).copied().unwrap_or((1, "16TH"));
            parts.push(n.to_string());
            left -= v;
        }
    } else if twelfths % 4 == 0 {
        // whole beats and a triplet part
        let (beats, thirds) = (twelfths / 12, (twelfths % 12) / 4);
        match beats {
            0 => {}
            1 => parts.push("QUARTER".into()),
            2 => parts.push("HALF".into()),
            3 => parts.push("DOTTED HALF".into()),
            4 => parts.push("WHOLE".into()),
            n => parts.push(format!("{} BEATS", n)),
        }
        parts.push(if thirds == 1 { "TRIPLET 8TH".into() } else { "TRIPLET QUARTER".into() });
    } else {
        return None;
    }
    Some(parts.join(" + "))
}

/// How long a chord holds, as said on the page: note values on a grid, seconds with Snap Off.
pub(crate) fn hold_text(beats: f32, snap: u8, bpm: f32) -> String {
    let secs = || format!("HOLDS {:.2} SECONDS", beats * 60.0 / bpm.max(30.0));
    if snap == 0 {
        return secs();
    }
    match note_words(beats) {
        Some(w) => format!("HOLDS {} {}", if w.starts_with('8') { "AN" } else { "A" }, w),
        None => secs(),
    }
}

/// What Backline calls a grip (the analyzer's best reading), if it's a chord at all.
pub(crate) fn grip_name(grip: &[Option<u8>; 6], sp: theory::Spelling) -> Option<String> {
    let notes = grip_notes(grip);
    let bass = notes.first().map(|m| m.rem_euclid(12));
    theory::suggestions(&notes, bass).first().map(|s| s.name(sp))
}

/// A chord's name as shown: the one you picked, else Backline's reading, else a dash.
fn shown(c: &LabChord, sp: theory::Spelling) -> String {
    if !c.name.is_empty() {
        return c.name.clone();
    }
    if let Some(n) = grip_name(&c.grip, sp) {
        return n;
    }
    let notes = grip_notes(&c.grip);
    if notes.is_empty() {
        return "REST".to_string();
    }
    // a single note (or notes that aren't a chord): their names, in brackets so they don't read as a chord
    let mut names: Vec<&str> = Vec::new();
    for m in &notes {
        let n = theory::note_name(m.rem_euclid(12), sp.sharps);
        if !names.contains(&n) {
            names.push(n);
        }
    }
    format!("({})", names.join(" "))
}

fn art_of(a: u8) -> Art {
    match a {
        1 => Art::Hammer,
        2 => Art::Slide,
        _ => Art::Strum,
    }
}

fn steps_of(p: &LabProg, sp: theory::Spelling) -> Vec<Step> {
    p.chords.iter().map(|c| Step { grip: c.grip, name: shown(c, sp), art: art_of(c.art), beats: c.beats }).collect()
}

fn blank() -> LabChord {
    LabChord { grip: [None; 6], name: String::new(), beats: PLACEHOLDER, timed: false, art: 0 }
}

/// The lengths from the times each chord started (and the end), in beats at `bpm`, put on the grid (or kept to a
/// hundredth with Snap Off). `ratio`: the song's speed when tapped along a slowed record.
#[cfg(test)]
pub(crate) fn lengths_from_taps(taps: &[f32], ratio: f32, bpm: f32, snap: u8) -> Vec<f32> {
    taps.windows(2).map(|w| one_length(w[1] - w[0], ratio, bpm, snap)).collect()
}

fn one_length(secs: f32, ratio: f32, bpm: f32, snap: u8) -> f32 {
    let b = secs.max(0.0) * ratio * bpm / 60.0;
    if snap == 0 {
        ((b * 100.0).round() / 100.0).clamp(0.05, 64.0)
    } else {
        snap_to(b, snap)
    }
}

/// A chord's time split into notes that can be written (4/4): over bar lines, and into values that start where
/// they can be read (a quarter on a beat, a half on 1 or 3...). On the snap's grid (triplets in thirds, all else in
/// 16ths). (start, length) in beats.
pub(crate) fn written_pieces(start: f32, len: f32, snap: u8) -> Vec<(f32, f32)> {
    let trip = snap == 3;
    let per = if trip { 3 } else { 4 }; // grid steps a beat
    let bar = 4 * per;
    let s = (start * per as f32).round() as i32;
    let e = ((start + len) * per as f32).round().max(s as f32 + 1.0) as i32;
    // (length in steps, where it may start: position in the bar modulo)
    let values: &[(i32, i32)] =
        if trip { &[(12, 12), (6, 6), (3, 3), (2, 1), (1, 1)] } else { &[(16, 16), (12, 4), (8, 8), (6, 2), (4, 4), (3, 4), (2, 2), (1, 1)] };
    let mut out = Vec::new();
    let mut a = s;
    while a < e {
        let in_bar = a.rem_euclid(bar);
        let room = (e - a).min(bar - in_bar);
        let v = values.iter().find(|(v, m)| *v <= room && in_bar % m == 0 && in_bar + v <= bar).map_or(1, |x| x.0);
        out.push((a as f32 / per as f32, v as f32 / per as f32));
        a += v;
    }
    out
}

/// The progression as bars for the Chord Melodies notation: each chord's notes in rhythm (tied where a chord
/// carries on), its name over its first note; and which chord each written beat is, (bar, beat) -> chord.
pub(crate) fn as_bars(p: &LabProg, sp: theory::Spelling, snap: u8) -> (Vec<crate::transcription::Bar>, Vec<crate::transcription::Grip>, Vec<Vec<usize>>) {
    let mut bars: Vec<crate::transcription::Bar> = Vec::new();
    let mut which: Vec<Vec<usize>> = Vec::new();
    let mut grips: Vec<crate::transcription::Grip> = Vec::new();
    let mut at = 0.0_f32;
    for (i, c) in p.chords.iter().enumerate() {
        let name = shown(c, sp);
        // a chord symbol over it only when it is a chord (a single note or a rest gets none)
        let symbol = (!c.name.is_empty() || grip_name(&c.grip, sp).is_some()).then(|| name.clone());
        let notes: Vec<(i32, i32, i32)> =
            c.grip.iter().enumerate().filter_map(|(s, f)| f.map(|f| (theory::STANDARD[s] + f as i32, s as i32, f as i32))).collect();
        if symbol.is_some() && notes.len() >= 2 && !grips.iter().any(|g| g.name == name) {
            grips.push(crate::transcription::Grip { name: name.clone(), frets: c.grip.map(|f| f.map_or(-1, |f| f as i8)) });
        }
        for (k, (s, l)) in written_pieces(at, c.beats, snap).into_iter().enumerate() {
            let b = (s / 4.0 + 1e-4).floor() as usize;
            while bars.len() <= b {
                bars.push(crate::transcription::Bar { time: (4, 4), key: None, beats: Vec::new() });
                which.push(Vec::new());
            }
            bars[b].beats.push(crate::transcription::Beat {
                at: s - b as f32 * 4.0,
                len: l,
                notes: notes.iter().map(|(m, st, f)| [*m, *st, *f, i32::from(k > 0)]).collect(),
                chord: if k == 0 { symbol.clone() } else { None },
            });
            which[b].push(i);
        }
        at += c.beats;
    }
    (bars, grips, which)
}

/// One bar of clicks at `bpm` (the first accented), or of silence.
fn click_bar(bpm: f32, sound: bool) -> Vec<i16> {
    let spb = (60.0 / bpm.max(30.0) * crate::band::RATE) as usize;
    let mut v = vec![0i16; spb * 4];
    if sound {
        let (hi, lo) = crate::band::click_sounds();
        for b in 0..4 {
            let s = if b == 0 { &hi } else { &lo };
            for (k, x) in s.iter().enumerate() {
                if let Some(o) = v.get_mut(b * spb + k) {
                    *o = (x * 0.9 * 32767.0) as i16;
                }
            }
        }
    }
    v
}

impl App {
    fn lab_linked_here(&self) -> bool {
        let link = self.store.lab_bench.link.as_ref();
        link.is_some_and(|l| self.cur_track().map(|t| t.id) == Some(l.song))
    }

    /// The chord sounding while the bench plays, if it is playing (the Lab's clicks stop when it ends).
    fn lab_lit(&mut self, ui: &egui::Ui) -> Option<usize> {
        let (t0, starts) = self.lab_play.as_ref()?;
        let el = t0.elapsed().as_secs_f32();
        if el > starts.last().copied().unwrap_or(0.0) + 0.3 {
            self.lab_play = None;
            self.lab_quiet_clicks();
            return None;
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        starts.windows(2).position(|w| el >= w[0] && el < w[1])
    }

    /// The Lab gives the metronome's sound back (the metronome takes it up again if it's on).
    fn lab_quiet_clicks(&mut self) {
        if self.lab_clicks {
            self.player.send(Cmd::Pcm(0, Vec::new(), Vec::new(), 0.0));
            self.lab_clicks = false;
            self.mt_sig = 0;
        }
    }

    fn lab_hear(&mut self, i: usize) {
        if let Some(c) = self.store.lab_bench.chords.get(i) {
            let notes = grip_notes(&c.grip);
            if !notes.is_empty() {
                self.player.send(Cmd::StopOnce);
                self.player.send(Cmd::Once(crate::band::chord_sound(&notes), 0.9));
            }
        }
    }

    /// Go to the linked loop (start the song if it isn't the one playing) and play from its start.
    fn lab_go_to_loop(&mut self, acts: &mut Vec<Action>) {
        let Some(l) = self.store.lab_bench.link.clone() else { return };
        let Some(v) = self.store.songs.get(&l.song).cloned() else {
            self.lab_note = "THAT SONG ISN'T IN YOUR LIBRARY ANY MORE.".into();
            return;
        };
        self.play_saved(&v, Some((l.a, l.b)));
        if self.lab_linked_here() && self.paused {
            acts.push(Action::Toggle);
        }
    }

    fn lab_stop(&mut self) {
        self.player.send(Cmd::StopOnce);
        self.lab_play = None;
        self.lab_quiet_clicks();
    }

    /// Over the linked loop's record (WITH THE LOOP picked), or on its own.
    fn lab_over_loop(&self) -> bool {
        self.lab_with_loop && self.store.lab_bench.link.is_some()
    }

    /// On its own: a song playing pauses, so only your chords and the click are heard (like the Trainer).
    fn lab_quiet_song(&mut self, acts: &mut Vec<Action>) {
        if self.cur.is_some() && !self.paused && !self.stopped {
            acts.push(Action::Toggle);
        }
    }

    fn lab_play_all(&mut self, acts: &mut Vec<Action>) {
        self.lab_stop();
        let over = self.lab_over_loop();
        // over the record, the record alone: the guitar on top of it never lines up exactly
        let guitar = !over;
        if over {
            self.lab_go_to_loop(acts);
        } else {
            self.lab_quiet_song(acts);
        }
        let p = &self.store.lab_bench;
        let spb = 60.0 / p.bpm.max(30.0);
        let (ev, starts) = steps_events(&steps_of(p, self.chord_sp), spb);
        let total = starts.last().copied().unwrap_or(1.0);
        if guitar {
            self.player.send(Cmd::Once(crate::band::string_events(&ev, total + 1.8), 0.9));
        }
        if !over && self.lab_metro {
            let lead = vec![0i16; (0.1 * crate::band::RATE) as usize];
            self.player.send(Cmd::Pcm(0, lead, click_bar(p.bpm, true), 0.8));
            self.lab_clicks = true;
        }
        self.lab_play = Some((Instant::now(), starts));
    }

    /// Each frame, wherever you are: the Lab's clicks stop and TAP TIMING ends once the page hasn't been drawn
    /// for a moment (you went elsewhere), and the clicks stop when PLAY has finished.
    pub(crate) fn lab_frame(&mut self) {
        let away = self.lab_seen.is_none_or(|t| t.elapsed().as_secs_f32() > 1.0);
        if self.lab_run.is_some() && away {
            self.lab_end_timing(false);
        }
        // (finished with REPEAT on and the page open: the page starts it again)
        let over = self.lab_play.as_ref().is_some_and(|(t0, st)| t0.elapsed().as_secs_f32() > st.last().copied().unwrap_or(0.0) + 0.3);
        if (over && (away || !self.lab_repeat)) || (away && self.lab_play.is_some()) {
            self.lab_play = None;
        }
        if self.lab_clicks && self.lab_run.is_none() && self.lab_play.is_none() {
            self.lab_quiet_clicks();
        }
    }

    /// TAP TEMPO: each tap clicks; two or more (none more than 2 seconds apart) set the tempo from their spacing.
    fn lab_tempo_tap(&mut self) {
        let now = Instant::now();
        if self.lab_tempo_taps.last().is_some_and(|t| now.duration_since(*t).as_secs_f32() > 2.0) {
            self.lab_tempo_taps.clear();
        }
        self.lab_tempo_taps.push(now);
        if self.lab_tempo_taps.len() > 6 {
            self.lab_tempo_taps.remove(0);
        }
        let (hi, _) = crate::band::click_sounds();
        self.player.send(Cmd::Once(hi.iter().map(|x| (x * 0.9 * 32767.0) as i16).collect(), 0.9));
        let t = &self.lab_tempo_taps;
        if t.len() >= 2 {
            let each = t[t.len() - 1].duration_since(t[0]).as_secs_f32() / (t.len() - 1) as f32;
            if each > 0.0 {
                self.store.lab_bench.bpm = (60.0 / each).round().clamp(30.0, 240.0);
                self.store_dirty = true;
            }
            self.lab_note = format!("TEMPO {} BPM FROM YOUR TAPS.", self.store.lab_bench.bpm.round() as i32);
        } else {
            self.lab_note = "KEEP TAPPING ON THE BEAT.".into();
        }
    }

    /// Whether Space belongs to TAP TIMING right now (so it doesn't also play or pause the song).
    pub(crate) fn lab_tapping(&self) -> bool {
        self.lab_run.is_some()
    }

    pub(crate) fn lab_start_timing(&mut self, acts: &mut Vec<Action>) {
        self.lab_stop();
        let bpm = self.store.lab_bench.bpm.max(30.0);
        let spb = 60.0 / bpm;
        let record = self.lab_over_loop();
        let now = Instant::now();
        for c in self.store.lab_bench.chords.iter_mut() {
            c.timed = false;
            c.beats = PLACEHOLDER;
        }
        self.lab_sel = 0;
        if record {
            // along the record: from the loop's start, the first tap is the first chord
            self.lab_go_to_loop(acts);
            let ratio = if self.practice { self.speed as f32 / 100.0 } else { 1.0 };
            self.lab_run = Some(LabRun { start: now, down: None, cur: None, at: Vec::new(), ratio, guitar: false });
            self.lab_note = "TAP (OR PRESS SPACE) AS EACH CHORD STARTS IN THE RECORD. ONE MORE TAP ENDS IT.".into();
            return;
        }
        self.lab_quiet_song(acts);
        // a bar of clicks, then the first chord on 1 (laid on the same clock as the clicks, so it lands on it)
        self.player.send(Cmd::Pcm(0, click_bar(bpm, true), click_bar(bpm, self.lab_metro), 0.8));
        self.lab_clicks = true;
        if let Some(c) = self.store.lab_bench.chords.first() {
            let notes = grip_notes(&c.grip);
            if !notes.is_empty() {
                let mut pcm = vec![0i16; (4.0 * spb * crate::band::RATE) as usize];
                pcm.extend(crate::band::chord_sound(&notes));
                self.player.send(Cmd::StopOnce);
                self.player.send(Cmd::Once(pcm, 0.9));
            }
        }
        let down = now + Duration::from_secs_f32(4.0 * spb);
        self.lab_run = Some(LabRun { start: now, down: Some(down), cur: None, at: Vec::new(), ratio: 1.0, guitar: true });
        self.lab_note = "COUNT-OFF... THE FIRST CHORD COMES IN ON 1.".into();
    }

    /// The downbeat reached: the first chord is on.
    fn lab_run_tick(&mut self) {
        let Some(run) = self.lab_run.as_mut() else { return };
        if let (None, Some(d)) = (run.cur, run.down) {
            if Instant::now() >= d {
                run.cur = Some(0);
                run.at.push(d);
                self.lab_note = "TAP (OR PRESS SPACE) AS EACH CHORD CHANGES. ONE MORE TAP AFTER THE LAST ENDS IT.".into();
            }
        }
        self.lab_last_note();
    }

    /// On the last chord: say the next tap ends it.
    fn lab_last_note(&mut self) {
        let last = self.store.lab_bench.chords.len().saturating_sub(1);
        if self.lab_run.as_ref().and_then(|r| r.cur) == Some(last) {
            self.lab_note = "LAST CHORD: TAP ONCE MORE (OR PRESS SPACE) TO END.".into();
        }
    }

    pub(crate) fn lab_tap(&mut self) {
        let n = self.store.lab_bench.chords.len();
        let (bpm, snap) = (self.store.lab_bench.bpm, self.lab_snap);
        let Some(run) = self.lab_run.as_ref() else { return };
        let now = Instant::now() - Duration::from_secs_f32(TAP_LATE);
        let (cur, counting, guitar, ratio) = (run.cur, run.down.is_some(), run.guitar, run.ratio);
        let began = cur.and_then(|c| run.at.get(c).copied());
        match cur {
            // still counting off: too early
            None if counting => {}
            // along the record: this tap starts the first chord
            None => {
                if let Some(r) = self.lab_run.as_mut() {
                    r.at.push(now);
                    r.cur = Some(0);
                }
                self.lab_sel = 0;
                if guitar {
                    self.lab_hear(0);
                }
                self.lab_note = "TAP (OR PRESS SPACE) AS EACH CHORD CHANGES. ONE MORE TAP AFTER THE LAST ENDS IT.".into();
                self.lab_last_note();
            }
            Some(c) => {
                let secs = began.map_or(0.0, |b| now.saturating_duration_since(b).as_secs_f32());
                if let Some(ch) = self.store.lab_bench.chords.get_mut(c) {
                    ch.beats = one_length(secs, ratio, bpm, snap);
                    ch.timed = true;
                }
                self.store_dirty = true;
                if c + 1 < n {
                    if let Some(r) = self.lab_run.as_mut() {
                        r.at.push(now);
                        r.cur = Some(c + 1);
                    }
                    self.lab_sel = c + 1;
                    if guitar {
                        self.lab_hear(c + 1);
                    }
                    self.lab_last_note();
                } else {
                    self.lab_end_timing(true);
                }
            }
        }
    }

    fn lab_end_timing(&mut self, done: bool) {
        let guitar = self.lab_run.as_ref().is_some_and(|r| r.guitar);
        self.lab_run = None;
        self.lab_quiet_clicks();
        if guitar {
            self.player.send(Cmd::StopOnce);
        }
        self.lab_sel = 0;
        self.lab_note = match (done, self.lab_snap) {
            (false, _) => "STOPPED. THE CHORDS YOU TAPPED KEEP THEIR TIMING.".into(),
            (true, 0) => "TIMED, IN FREE TIME: EXACTLY AS YOU TAPPED.".into(),
            _ => "TIMED. PICK A CHORD TO SEE HOW LONG IT HOLDS, OR DRAG ITS EDGE TO FIX IT.".into(),
        };
    }

    /// The PROGRESSION LAB page.
    pub(crate) fn lab(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let sp = self.chord_sp;
        if self.store.lab_bench.bpm <= 0.0 {
            self.store.lab_bench.bpm = self.band_bpm();
        }
        if self.store.lab_bench.chords.is_empty() {
            self.store.lab_bench.chords.push(blank());
        }
        let n = self.store.lab_bench.chords.len();
        self.lab_sel = self.lab_sel.min(n - 1);
        self.lab_seen = Some(Instant::now());
        // Ctrl+Z takes back the last change (the player's Z is held back on this page, see `key_commands`)
        if self.lab_run.is_none() && self.ed.id == 0 && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
            self.lab_undo_last();
        }
        self.lab_run_tick();
        crate::test_hooks::lab_auto(self, acts);
        let was_playing = self.lab_play.is_some();
        let lit = self.lab_lit(ui);
        // REPEAT: round again
        if was_playing && self.lab_play.is_none() && self.lab_repeat && self.lab_run.is_none() {
            self.lab_play_all(acts);
        }
        if self.lab_run.is_some() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
            // Space taps (the player's own Space is held back meanwhile, see `key_commands`); Esc stops
            if self.ed.id == 0 && ui.input(|i| i.key_pressed(egui::Key::Space)) {
                self.lab_tap();
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.lab_end_timing(false);
            }
        }
        // (a test copy can open scrolled down: BACKLINE_SCROLL=points)
        let mut sa = egui::ScrollArea::vertical().id_salt("lab_scroll").auto_shrink([false, false]);
        if let Some(y) = std::env::var("BACKLINE_SCROLL").ok().and_then(|s| s.parse::<f32>().ok()) {
            sa = sa.vertical_scroll_offset(y);
        }
        sa.show(ui, |ui| {
            self.lab_body(ui, acts, lit, sp);
            ui.add_space(20.0);
        });
        self.lab_history();
    }

    /// The Lab's page is the one open.
    pub(crate) fn lab_here(&self) -> bool {
        self.sec == Sec::Chords && self.chords_tab == 9
    }

    /// Undo's memory: after each frame, a change to the bench keeps what it was before (a drag or a tapped timing
    /// counts once, when it's done; the name alone doesn't count).
    fn lab_history(&mut self) {
        if self.lab_drag.is_some() || self.lab_run.is_some() {
            return;
        }
        let now = &self.store.lab_bench;
        let Some(was) = self.lab_was.as_ref() else {
            self.lab_was = Some(now.clone());
            return;
        };
        if was == now {
            return;
        }
        let mut renamed = was.clone();
        renamed.name = now.name.clone();
        if &renamed != now {
            self.lab_undo.push(was.clone());
            if self.lab_undo.len() > 60 {
                self.lab_undo.remove(0);
            }
        }
        self.lab_was = Some(now.clone());
    }

    fn lab_undo_last(&mut self) {
        let Some(p) = self.lab_undo.pop() else {
            self.lab_note = "NOTHING TO UNDO.".into();
            return;
        };
        self.lab_stop();
        self.store.lab_bench = p.clone();
        self.lab_was = Some(p);
        self.lab_sel = self.lab_sel.min(self.store.lab_bench.chords.len().saturating_sub(1));
        self.store_dirty = true;
        self.lab_note = "UNDONE.".into();
    }

    fn lab_body(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, lit: Option<usize>, sp: theory::Spelling) {
        let running = self.lab_run.is_some();
        let bpm = self.store.lab_bench.bpm;
        ui.horizontal_wrapped(|ui| {
            experimental_chip(ui);
            ui.add_space(6.0);
            crate::views::dim_line(ui, "PUT THE CHORDS IN, THEN TAP TIMING RUNS THROUGH THEM WITH THE METRONOME.", 1.0, pal().dim);
        });
        ui.add_space(6.0);
        // name, timing, playing, keeping
        ui.horizontal_wrapped(|ui| {
            let fw = (ui.available_width() - 470.0).clamp(150.0, 300.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(fw, bh()), Sense::hover());
            let mut name = self.store.lab_bench.name.clone();
            let out = field(ui, &mut self.ed, crate::tools::F_LAB_NAME, &mut name, rect, "Name it", false);
            if out.changed {
                self.store.lab_bench.name = name;
                self.store_dirty = true;
            }
            let tip = "A bar of clicks, then each chord in turn: tap (or press Space) as it changes, and once more after \
                the last. Esc stops. Linked to a loop with Record or Both heard, it runs over the record instead.";
            // what the next tap does: wait through the count-off, start (along a record), next, or end on the last chord
            let last = self.store.lab_bench.chords.len().saturating_sub(1);
            let tap_label = match self.lab_run.as_ref() {
                Some(r) if r.cur.is_none() && r.down.is_some() => "WAIT...",
                Some(r) if r.cur.is_none() => "START (SPACE)",
                Some(r) if r.cur == Some(last) => "END (SPACE)",
                Some(_) => "NEXT (SPACE)",
                None => "TAP TIMING",
            };
            if retro_btn(ui, tap_label, running).tip(tip).clicked() {
                if running {
                    self.lab_tap();
                } else {
                    self.lab_start_timing(acts);
                }
            }
            if running && retro_btn(ui, "STOP", false).clicked() {
                self.lab_end_timing(false);
            }
            let playing = lit.is_some();
            if !running && retro_btn(ui, if playing { "STOP" } else { "PLAY" }, playing).tip("Hear it at its tempo").clicked() {
                if playing {
                    self.lab_stop();
                } else {
                    self.lab_play_all(acts);
                }
            }
            if !running && retro_btn(ui, "REPEAT", self.lab_repeat).tip("Play goes round and round until you stop it").clicked() {
                self.lab_repeat = !self.lab_repeat;
                self.dirty = true;
            }
            let named = !self.store.lab_bench.name.trim().is_empty();
            if retro_btn(ui, "KEEP IT", false).tip("Keep it under its name (a kept one by the same name is replaced)").clicked() || out.enter {
                if named {
                    let p = self.store.lab_bench.clone();
                    let key = p.name.trim().to_lowercase();
                    match self.store.lab.iter().position(|k| k.name.trim().to_lowercase() == key) {
                        Some(at) => self.store.lab[at] = p,
                        None => self.store.lab.push(p),
                    }
                    self.store_dirty = true;
                    self.lab_note = format!("KEPT: {}", self.store.lab_bench.name.trim());
                } else {
                    self.lab_note = "GIVE IT A NAME FIRST.".into();
                }
            }
            if retro_btn(ui, "NEW", false).tip("Start a new progression (the kept ones stay)").clicked() && !running {
                let bpm = self.store.lab_bench.bpm;
                self.store.lab_bench = LabProg { name: String::new(), bpm, chords: vec![blank()], link: None };
                self.lab_sel = 0;
                self.lab_note.clear();
                self.store_dirty = true;
            }
            if retro_btn(ui, "UNDO", false).tip("Take back the last change (Ctrl+Z)").clicked() && !running {
                self.lab_undo_last();
            }
        });
        // tempo, metronome, snap, key
        ui.horizontal_wrapped(|ui| {
            crate::views::form_label(ui, "TEMPO");
            if retro_btn_w(ui, "-", 28.0, false).clicked() && !running {
                self.store.lab_bench.bpm = (bpm - 2.0).max(30.0);
                self.store_dirty = true;
            }
            crate::views::label(ui, &format!("{} BPM", bpm.round() as i32), 70.0);
            if retro_btn_w(ui, "+", 28.0, false).clicked() && !running {
                self.store.lab_bench.bpm = (bpm + 2.0).min(240.0);
                self.store_dirty = true;
            }
            ui.add_space(8.0);
            if retro_btn(ui, "TAP TEMPO", false).tip("Tap it on the beat a few times and the tempo follows").clicked() && !running {
                self.lab_tempo_tap();
            }
            ui.add_space(8.0);
            if retro_btn(ui, "METRONOME", self.lab_metro).tip("Clicks while you tap the timing and when it plays").clicked() {
                self.lab_metro = !self.lab_metro;
                self.dirty = true;
            }
            ui.add_space(8.0);
            if let Some(s) = dropdown(ui, "lab_snap", "SNAP", &SNAPS, self.lab_snap as usize, 130.0) {
                self.lab_snap = s as u8;
                if s > 0 {
                    let snap = self.lab_snap;
                    for c in self.store.lab_bench.chords.iter_mut().filter(|c| c.timed) {
                        c.beats = snap_to(c.beats, snap);
                    }
                    self.store_dirty = true;
                }
                self.dirty = true;
            }
            ui.add_space(8.0);
            for (label, d, tip) in [("KEY -", -1i32, "Everything a fret down"), ("KEY +", 1, "Everything a fret up")] {
                let all: Vec<u8> = self.store.lab_bench.chords.iter().flat_map(|c| c.grip.iter().flatten().copied()).collect();
                let can = !all.is_empty() && if d < 0 { all.iter().all(|f| *f > 0) } else { all.iter().all(|f| *f < 15) };
                if retro_btn(ui, label, false).tip(tip).clicked() && can && !running {
                    for c in self.store.lab_bench.chords.iter_mut() {
                        for f in c.grip.iter_mut().flatten() {
                            *f = (*f as i32 + d) as u8;
                        }
                        if !c.name.is_empty() {
                            c.name = crate::chart::transpose(&c.name, d);
                        }
                    }
                    self.store_dirty = true;
                }
            }
        });
        self.lab_link_row(ui, acts);
        ui.add_space(4.0);
        // the four lights and what's happening
        ui.horizontal(|ui| {
            let beat = self.lab_run.as_ref().filter(|r| r.down.is_some()).map(|r| {
                let spb = 60.0 / bpm.max(30.0);
                ((r.start.elapsed().as_secs_f32() / spb).floor() as usize) % 4
            });
            let beat = beat.or_else(|| {
                let (t0, st) = self.lab_play.as_ref()?;
                let el = t0.elapsed().as_secs_f32() - st.first().copied().unwrap_or(0.0);
                (el >= 0.0).then(|| ((el / (60.0 / bpm.max(30.0))).floor() as usize) % 4)
            });
            for k in 0..4 {
                let (r, _) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), Sense::hover());
                let on = beat == Some(k);
                let col = if on && k == 0 { pal().ink } else if on { pal().accent } else { pal().edge };
                if on {
                    ui.painter().circle_filled(r.center(), 10.0, col);
                } else {
                    ui.painter().circle_stroke(r.center(), 10.0, Stroke::new(2.0_f32, col));
                }
                ptext(ui.painter(), r.center(), Align::Center, &(k + 1).to_string(), 1.0, if on { pal().panel_dk } else { pal().dim });
            }
            ui.add_space(10.0);
            if !self.lab_note.is_empty() {
                crate::views::dim_line(ui, &self.lab_note.clone(), 1.0, pal().accent);
            }
        });
        ui.add_space(4.0);
        self.lab_rows(ui, lit);
        ui.add_space(10.0);
        self.lab_picked(ui, acts, lit, sp);
        ui.add_space(10.0);
        // written out
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "WRITTEN OUT", self.lab_written).tip("Your grips on a staff and tab, in rhythm").clicked() {
                self.lab_written = !self.lab_written;
                self.dirty = true;
            }
            if self.lab_written && self.lab_snap == 0 {
                crate::views::dim_line(ui, "WRITTEN TO THE NEAREST 16TH: IT PLAYS EXACTLY AS YOU TAPPED.", 1.0, pal().dim);
            }
        });
        if self.lab_written && self.store.lab_bench.chords.iter().any(|c| c.grip.iter().any(|f| f.is_some())) {
            let (bars, grips, which) = as_bars(&self.store.lab_bench, sp, self.lab_snap);
            let now = lit.or(self.lab_run.as_ref().and_then(|r| r.cur)).and_then(|c| {
                which.iter().enumerate().find_map(|(b, w)| w.iter().position(|x| *x == c).map(|k| (b, k)))
            });
            ui.add_space(4.0);
            let out = self.draw_bars(ui, &bars, &grips, sp.sharps, now, false, &[], 0, Some([true, true, true, true, false]));
            if ui.input(|i| i.pointer.primary_clicked()) && !running {
                if let Some(c) = out.grip.or(out.hover).and_then(|(b, k)| which.get(b).and_then(|w| w.get(k)).copied()) {
                    self.lab_sel = c;
                    self.lab_hear(c);
                }
            }
        }
        ui.add_space(10.0);
        self.lab_kept(ui);
    }

    /// The loop it's linked to (go to it, what's heard, unlink), or a way to link one from the song playing.
    fn lab_link_row(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let running = self.lab_run.is_some();
        ui.horizontal_wrapped(|ui| {
            if let Some(l) = self.store.lab_bench.link.clone() {
                crate::views::form_label(ui, "FROM");
                let title = self.store.songs.get(&l.song).map_or(l.title.clone(), |v| v.title.clone());
                let label = format!("{} · {} ({}–{})", title, l.name, fmt_time(l.a), fmt_time(l.b));
                if retro_btn(ui, &label, self.lab_linked_here()).tip("Go to this loop in the song").clicked() && !running {
                    self.lab_go_to_loop(acts);
                }
                if let Some(h) = dropdown(ui, "lab_play_as", "PLAY", &PLAYS, usize::from(self.lab_with_loop), 190.0) {
                    self.lab_with_loop = h == 1;
                    self.dirty = true;
                }
                if retro_btn(ui, "UNLINK", false).tip("This progression stops pointing at the loop").clicked() && !running {
                    self.store.lab_bench.link = None;
                    self.store_dirty = true;
                }
                return;
            }
            let Some(t) = self.cur_track() else {
                crate::views::dim_line(ui, "TO LINK A LOOP: PLAY THE SONG AND OPEN ONE OF ITS LOOPS.", 1.0, pal().dim);
                return;
            };
            let mut loops: Vec<(String, f32, f32)> = self
                .store
                .sections
                .get(&t.id)
                .map(|l| l.iter().map(|s| (s.name.clone(), s.a, s.b)).collect())
                .unwrap_or_default();
            if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
                if !loops.iter().any(|l| (l.1 - a).abs() < 0.05 && (l.2 - b).abs() < 0.05) {
                    loops.insert(0, ("THE LOOP NOW".to_string(), a.min(b), a.max(b)));
                }
            }
            if loops.is_empty() {
                crate::views::dim_line(ui, "TO LINK A LOOP: SET OR OPEN A LOOP ON THIS SONG.", 1.0, pal().dim);
                return;
            }
            let mut names: Vec<String> = vec!["PICK ONE".to_string()];
            names.extend(loops.iter().map(|(n, a, b)| format!("{} ({}–{})", n, fmt_time(*a), fmt_time(*b))));
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            if let Some(k) = dropdown(ui, "lab_link", "LINK TO A LOOP", &refs, 0, 260.0) {
                if k > 0 && !running {
                    let (name, a, b) = loops[k - 1].clone();
                    self.remember_song(&t);
                    self.store.lab_bench.link = Some(LabLink { song: t.id, title: t.title.clone(), artist: t.artist.clone(), a, b, name: name.clone() });
                    // the song's own tempo, when Backline found its beat
                    if let Some((id, period, _)) = self.beat {
                        if id == t.id && period > 0.0 {
                            self.store.lab_bench.bpm = (60.0 / period).round().clamp(30.0, 240.0);
                        }
                    }
                    if self.store.lab_bench.name.trim().is_empty() {
                        self.store.lab_bench.name = format!("{} - {}", t.title, name);
                    }
                    self.store_dirty = true;
                    self.lab_note = format!("LINKED TO {}. PICK WITH THE LOOP TO PLAY OR TAP OVER THE RECORD.", name);
                }
            }
        });
    }

    /// The bar rows: two bars a row (one when narrow), each chord a block as long as it holds, its name inside when
    /// it fits, else in a tag under (or over) it. Click to pick (and hear it); drag a block's right edge to fix its
    /// length. Untimed chords are dim placeholders; the one sounding is lit.
    fn lab_rows(&mut self, ui: &mut egui::Ui, lit: Option<usize>) {
        let sp = self.chord_sp;
        let snap = self.lab_snap;
        let bpm = self.store.lab_bench.bpm.max(30.0);
        let chords = self.store.lab_bench.chords.clone();
        let run_cur = self.lab_run.as_ref().and_then(|r| r.cur);
        // the lengths shown: the chord being tapped grows as it holds
        let lens: Vec<f32> = chords
            .iter()
            .enumerate()
            .map(|(i, c)| match (run_cur, self.lab_run.as_ref()) {
                (Some(k), Some(r)) if k == i => (r.at[k].elapsed().as_secs_f32() * r.ratio * bpm / 60.0).max(0.1),
                _ => c.beats,
            })
            .collect();
        let mut starts = Vec::with_capacity(lens.len());
        let mut at = 0.0_f32;
        for l in &lens {
            starts.push(at);
            at += l;
        }
        let total = at;
        let w = ui.available_width().max(200.0);
        let per = if w < 520.0 { 4.0 } else { 8.0 };
        let px = (w - 8.0) / per;
        let rows = ((total / per) - 1e-3).ceil().max(1.0) as usize;
        let row_h = 96.0;
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, rows as f32 * row_h), Sense::click_and_drag());
        let p = ui.painter().clone();
        let pointer = resp.hover_pos().or_else(|| resp.interact_pointer_pos());
        let mut on_edge = None;
        let mut on_block = None;
        let lane = |r: usize| {
            let top = rect.min.y + r as f32 * row_h + 22.0;
            (top, top + 48.0)
        };
        let x_at = |b: f32| rect.min.x + 4.0 + b * px;
        for r in 0..rows {
            let (top, bot) = lane(r);
            let r0 = r as f32 * per;
            p.rect_filled(Rect::from_min_max(Pos2::new(rect.min.x + 4.0, top), Pos2::new(x_at(per), bot)), 3.0, pal().panel_dk);
            for k in 0..=(per as usize) {
                let bar = (r0 as usize + k) % 4 == 0;
                let x = x_at(k as f32);
                p.line_segment([Pos2::new(x, top), Pos2::new(x, bot)], Stroke::new(if bar { 2.0_f32 } else { 1.0_f32 }, if bar { pal().ink2 } else { pal().edge }));
                if bar && k < per as usize {
                    ptext(&p, Pos2::new(x + 3.0, top - 10.0), Align::LEFT, &format!("BAR {}", (r0 as usize + k) / 4 + 1), 1.0, pal().dim);
                }
            }
            let mut last_tag = f32::NEG_INFINITY;
            for (i, c) in chords.iter().enumerate() {
                let (a, z) = (starts[i].max(r0), (starts[i] + lens[i]).min(r0 + per));
                if z <= a + 1e-4 {
                    continue;
                }
                let first = (a - starts[i]).abs() < 1e-4;
                let last = ((starts[i] + lens[i]) - z).abs() < 1e-4;
                let b = Rect::from_min_max(Pos2::new(x_at(a - r0) + 2.0, top + 4.0), Pos2::new((x_at(z - r0) - 2.0).max(x_at(a - r0) + 4.0), bot - 4.0));
                let timed = c.timed || run_cur == Some(i);
                let sounding = lit == Some(i) || run_cur == Some(i);
                let picked = i == self.lab_sel;
                p.rect_filled(b, 6.0, if picked { pal().panel_hover } else if timed { pal().panel_lt } else { pal().panel });
                let stroke = if sounding {
                    Stroke::new(2.0_f32, pal().accent)
                } else if picked {
                    Stroke::new(2.0_f32, pal().ink)
                } else {
                    Stroke::new(1.0_f32, pal().edge)
                };
                p.rect_stroke(b, 6.0, stroke);
                let name = shown(c, sp);
                let col = if timed { pal().ink } else { pal().dim };
                if first {
                    if crate::font::text_w(&name, 1.4) + 14.0 <= b.width() {
                        ptext(&p, Pos2::new(b.min.x + 8.0, b.center().y), Align::LEFT, &name, 1.4, col);
                    } else {
                        // a tag under the block, or over it if the one before is too close
                        let tw = crate::font::text_w(&name, 1.1) + 10.0;
                        let under = b.min.x > last_tag + 4.0;
                        let y = if under { bot + 10.0 } else { top - 22.0 + 10.0 };
                        let tag = Rect::from_min_size(Pos2::new(b.min.x, y - 8.0), Vec2::new(tw, 16.0));
                        p.rect_filled(tag, 4.0, pal().panel_lt);
                        p.rect_stroke(tag, 4.0, Stroke::new(1.0_f32, if picked { pal().ink } else { pal().edge }));
                        ptext(&p, Pos2::new(tag.min.x + 5.0, tag.center().y), Align::LEFT, &name, 1.1, col);
                        if under {
                            last_tag = tag.max.x;
                        }
                        if let Some(pt) = pointer {
                            if tag.contains(pt) {
                                on_block = Some(i);
                            }
                        }
                    }
                }
                // the handle on the chord's right edge
                if last && self.lab_run.is_none() {
                    p.line_segment([Pos2::new(b.max.x - 4.0, b.center().y - 8.0), Pos2::new(b.max.x - 4.0, b.center().y + 8.0)], Stroke::new(3.0_f32, pal().dim));
                    let h = Rect::from_min_max(Pos2::new(b.max.x - 9.0, b.min.y), Pos2::new(b.max.x + 4.0, b.max.y));
                    if pointer.is_some_and(|pt| h.contains(pt)) {
                        on_edge = Some(i);
                    }
                }
                if pointer.is_some_and(|pt| b.contains(pt)) && on_edge != Some(i) {
                    on_block = Some(i);
                }
            }
        }
        // the playhead while it plays
        if let (Some((t0, st)), Some(_)) = (self.lab_play.as_ref(), lit) {
            let el = t0.elapsed().as_secs_f32() - st.first().copied().unwrap_or(0.0);
            let beat = (el.max(0.0) * bpm / 60.0).min(total - 1e-3);
            let r = (beat / per).floor() as usize;
            let (top, bot) = lane(r.min(rows - 1));
            let x = x_at(beat - r as f32 * per);
            p.line_segment([Pos2::new(x, top - 2.0), Pos2::new(x, bot + 2.0)], Stroke::new(2.0_f32, pal().accent));
        }
        if self.lab_run.is_some() {
            return;
        }
        if on_edge.is_some() || self.lab_drag.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        } else if on_block.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.drag_started() {
            if let (Some(i), Some(pt)) = (on_edge, resp.interact_pointer_pos()) {
                self.lab_drag = Some((i, chords[i].beats, pt.x, px));
                self.lab_sel = i;
            }
        }
        if let (Some((i, b0, x0, px)), Some(pt)) = (self.lab_drag, resp.interact_pointer_pos()) {
            if resp.dragged() {
                let b = snap_to(b0 + (pt.x - x0) / px, snap);
                if let Some(c) = self.store.lab_bench.chords.get_mut(i) {
                    if (c.beats - b).abs() > 1e-4 || !c.timed {
                        c.beats = b;
                        c.timed = true;
                        self.store_dirty = true;
                    }
                }
            }
        }
        if resp.drag_stopped() {
            self.lab_drag = None;
        }
        if resp.clicked() {
            if let Some(i) = on_block.or(on_edge) {
                self.lab_sel = i;
                self.lab_hear(i);
            }
        }
    }

    /// The picked chord: its name and how long it holds, moving it, the neck to grip it on, what to call it.
    fn lab_picked(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, lit: Option<usize>, sp: theory::Spelling) {
        let running = self.lab_run.is_some();
        let shown_i = lit.or(self.lab_run.as_ref().and_then(|r| r.cur)).unwrap_or(self.lab_sel);
        let cur = self.store.lab_bench.chords[shown_i.min(self.store.lab_bench.chords.len() - 1)].clone();
        let bpm = self.store.lab_bench.bpm;
        let i = self.lab_sel;
        ui.horizontal_wrapped(|ui| {
            let name = shown(&cur, sp);
            let (r, _) = ui.allocate_exact_size(Vec2::new(crate::font::text_w(&name, 3.0).max(40.0) + 12.0, 34.0), Sense::hover());
            ptext(ui.painter(), Pos2::new(r.min.x, r.center().y), Align::LEFT, &name, 3.0, pal().ink);
            let holding = self.lab_run.as_ref().and_then(|r| r.cur) == Some(shown_i);
            let on_last = shown_i + 1 == self.store.lab_bench.chords.len();
            let hold = if holding && on_last {
                "LAST CHORD: TAP TO END".to_string()
            } else if holding {
                "HOLDING: TAP WHEN IT CHANGES".to_string()
            } else if cur.timed {
                hold_text(cur.beats, self.lab_snap, bpm)
            } else {
                "NOT TIMED YET".to_string()
            };
            crate::views::label(ui, &hold, crate::font::text_w(&hold, 1.2) + 16.0);
        });
        ui.horizontal_wrapped(|ui| {
            let step = snap_step(self.lab_snap);
            if retro_btn(ui, "- SHORTER", false).tip("Hold it for less").clicked() && !running {
                let c = &mut self.store.lab_bench.chords[i];
                c.beats = snap_to(c.beats - step, self.lab_snap);
                c.timed = true;
                self.store_dirty = true;
            }
            if retro_btn(ui, "+ LONGER", false).tip("Hold it for more").clicked() && !running {
                let c = &mut self.store.lab_bench.chords[i];
                c.beats = snap_to(c.beats + step, self.lab_snap);
                c.timed = true;
                self.store_dirty = true;
            }
            ui.add_space(8.0);
            if retro_btn(ui, "< EARLIER", false).tip("Swap it with the chord before").clicked() && i > 0 && !running {
                self.store.lab_bench.chords.swap(i, i - 1);
                self.lab_sel = i - 1;
                self.store_dirty = true;
            }
            if retro_btn(ui, "LATER >", false).tip("Swap it with the chord after").clicked() && i + 1 < self.store.lab_bench.chords.len() && !running {
                self.store.lab_bench.chords.swap(i, i + 1);
                self.lab_sel = i + 1;
                self.store_dirty = true;
            }
            ui.add_space(8.0);
            if retro_btn(ui, "ADD CHORD", false).tip("A new chord after this one, starting from the same grip: move the notes that change").clicked()
                && self.store.lab_bench.chords.len() < 64
                && !running
            {
                let mut c = self.store.lab_bench.chords[i].clone();
                c.beats = PLACEHOLDER;
                c.timed = false;
                c.art = 0;
                self.store.lab_bench.chords.insert(i + 1, c);
                self.lab_sel = i + 1;
                self.store_dirty = true;
            }
            if retro_btn(ui, "REMOVE", false).tip("Take this chord out").clicked() && !running {
                let chords = &mut self.store.lab_bench.chords;
                if chords.len() > 1 {
                    chords.remove(i);
                } else {
                    chords[0] = blank();
                }
                self.lab_sel = i.saturating_sub(1);
                self.store_dirty = true;
            }
            if retro_btn(ui, "CLEAR ALL", false).tip("Take every chord out (Undo brings them back)").clicked() && !running {
                self.store.lab_bench.chords = vec![blank()];
                self.lab_sel = 0;
                self.store_dirty = true;
                self.lab_note = "CLEARED. UNDO BRINGS THEM BACK.".into();
            }
        });
        ui.add_space(4.0);
        let zoom = self.neck_zoom;
        let w = ui.available_width();
        let (nr, _) = ui.allocate_exact_size(Vec2::new(w, 132.0 * zoom), Sense::hover());
        let nr = nr.shrink2(Vec2::new(4.0, 6.0));
        let neck_rect = Rect::from_min_max(nr.min, Pos2::new(nr.max.x, nr.max.y - 12.0));
        let notes = grip_notes(&cur.grip);
        let found = theory::suggestions(&notes, notes.first().map(|m| m.rem_euclid(12)));
        let root = found
            .iter()
            .find(|s| s.name(sp) == cur.name)
            .or_else(|| found.first())
            .map(|s| s.root)
            .or_else(|| notes.first().map(|m| m.rem_euclid(12)));
        let alpha = self.ghost_alpha;
        let editable = lit.is_none() && !running;
        if let Some((s, f)) = neck_alpha(ui, neck_rect, &cur.grip, 15, editable, "lab_neck", root, &[], false, alpha) {
            let c = &mut self.store.lab_bench.chords[self.lab_sel];
            c.grip[s] = f;
            c.name.clear();
            self.store_dirty = true;
            if let Some(f) = f {
                acts.push(Action::PlayNotes(vec![theory::STANDARD[s] + f as i32]));
            }
        }
        if !found.is_empty() {
            ui.horizontal_wrapped(|ui| {
                crate::views::form_label(ui, "CALL IT");
                for (k, s) in found.iter().take(5).enumerate() {
                    let nm = s.name(sp);
                    let on = cur.name == nm || (cur.name.is_empty() && k == 0);
                    if retro_btn(ui, &nm, on).clicked() && !running {
                        self.store.lab_bench.chords[self.lab_sel].name = if k == 0 { String::new() } else { nm };
                        self.store_dirty = true;
                    }
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "STRUM", false).tip("Hear this chord").clicked() {
                self.lab_hear(self.lab_sel);
            }
            if retro_btn(ui, "CLEAR", false).tip("Take every note off the neck").clicked() && !running {
                let c = &mut self.store.lab_bench.chords[self.lab_sel];
                c.grip = [None; 6];
                c.name.clear();
                self.store_dirty = true;
            }
            let frets: Vec<u8> = cur.grip.iter().flatten().copied().collect();
            let (lo, hi) = (frets.iter().min().copied(), frets.iter().max().copied());
            for (label, d, tip) in [("< MOVE", -1i32, "The whole shape a fret down"), ("MOVE >", 1, "The whole shape a fret up")] {
                let can = matches!((lo, hi), (Some(l), Some(h)) if (d < 0 && l > 0) || (d > 0 && h < 15));
                if retro_btn(ui, label, false).tip(tip).clicked() && can && !running {
                    let c = &mut self.store.lab_bench.chords[self.lab_sel];
                    for f in c.grip.iter_mut().flatten() {
                        *f = (*f as i32 + d) as u8;
                    }
                    if !c.name.is_empty() {
                        c.name = crate::chart::transpose(&c.name, d);
                    }
                    self.store_dirty = true;
                    self.lab_hear(self.lab_sel);
                }
            }
            ui.add_space(8.0);
            let art = (cur.art as usize).min(ARTS.len() - 1);
            if let Some(a) = dropdown(ui, "lab_art", "PLAYED", &ARTS, art, 150.0) {
                self.store.lab_bench.chords[self.lab_sel].art = a as u8;
                self.store_dirty = true;
            }
        });
    }

    /// The kept progressions (click one to open it on the bench), and the open one to delete.
    fn lab_kept(&mut self, ui: &mut egui::Ui) {
        if self.store.lab.is_empty() {
            return;
        }
        let running = self.lab_run.is_some();
        ui.horizontal_wrapped(|ui| {
            crate::views::form_label(ui, "KEPT");
            let open = self.store.lab_bench.name.trim().to_lowercase();
            let mut load = None;
            for (k, p) in self.store.lab.iter().enumerate() {
                let label = if p.link.is_some() { format!("{} · LOOP", p.name) } else { p.name.clone() };
                if retro_btn(ui, &label, p.name.trim().to_lowercase() == open).tip("Open it").clicked() {
                    load = Some(k);
                }
            }
            if let Some(k) = load.filter(|_| !running) {
                self.lab_stop();
                self.store.lab_bench = self.store.lab[k].clone();
                self.lab_sel = 0;
                self.lab_note.clear();
                self.store_dirty = true;
            }
            if let Some(at) = self.store.lab.iter().position(|p| p.name.trim().to_lowercase() == open) {
                ui.add_space(8.0);
                if retro_btn(ui, "DELETE IT", false).tip("Remove the kept copy (what's open stays open)").clicked() && !running {
                    self.store.lab.remove(at);
                    self.store_dirty = true;
                }
            }
        });
    }
}

/// The small EXPERIMENTAL mark: an outlined tag in the accent colour.
fn experimental_chip(ui: &mut egui::Ui) {
    let t = "EXPERIMENTAL";
    let w = crate::font::text_w(t, 1.0) + 16.0;
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 18.0), Sense::hover());
    let p = ui.painter();
    p.rect_stroke(r, 9.0, Stroke::new(1.0_f32, pal().accent));
    ptext(p, r.center(), Align::Center, t, 1.0, pal().accent);
    resp.tip("New and still changing: say what works and what doesn't.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taps_become_lengths() {
        // at 60 BPM a second is a beat: chords start at 0, 2, 3.5, 4 s and it ends at 8 s
        let taps = [0.0, 2.0, 3.5, 4.0, 8.0];
        assert_eq!(lengths_from_taps(&taps, 1.0, 60.0, 2), vec![2.0, 1.5, 0.5, 4.0]);
        // a little off lands on the nearest 16th
        assert_eq!(lengths_from_taps(&[0.0, 1.12], 1.0, 60.0, 2), vec![1.0]);
        // free time keeps it to a hundredth
        assert_eq!(lengths_from_taps(&[0.0, 1.12], 1.0, 60.0, 0), vec![1.12]);
        // tapped along a record at 50%: the real length is half
        assert_eq!(lengths_from_taps(&[0.0, 4.0], 0.5, 60.0, 2), vec![2.0]);
    }

    #[test]
    fn lengths_read_as_note_values() {
        assert_eq!(hold_text(0.75, 2, 72.0), "HOLDS A DOTTED 8TH");
        assert_eq!(hold_text(0.5, 2, 72.0), "HOLDS AN 8TH");
        assert_eq!(hold_text(2.5, 2, 72.0), "HOLDS A HALF + 8TH");
        assert_eq!(hold_text(4.0 / 3.0, 3, 72.0), "HOLDS A QUARTER + TRIPLET 8TH");
        assert_eq!(hold_text(1.5, 0, 60.0), "HOLDS 1.50 SECONDS");
        assert_eq!(note_words(1.13), None);
    }

    #[test]
    fn single_notes_and_rests_are_steps_too() {
        let sp = theory::Spelling::default();
        let one = LabChord { grip: [None, Some(6), None, None, None, None], beats: 1.0, timed: true, ..Default::default() };
        let rest = LabChord { grip: [None; 6], beats: 1.0, timed: true, ..Default::default() };
        let n = shown(&one, sp);
        assert!(n.starts_with('(') && n.ends_with(')'), "{}", n);
        assert_eq!(shown(&rest, sp), "REST");
        let p = LabProg { name: String::new(), bpm: 60.0, chords: vec![one, rest], link: None };
        let (bars, grips, _) = as_bars(&p, sp, 2);
        // no chord symbol or grip box for a single note; the rest is a beat with no notes
        assert!(grips.is_empty());
        assert_eq!(bars[0].beats[0].notes.len(), 1);
        assert!(bars[0].beats[0].chord.is_none());
        assert!(bars[0].beats[1].notes.is_empty());
    }

    #[test]
    fn snapping() {
        assert_eq!(snap_to(1.3, 1), 1.5);
        assert!((snap_to(0.7, 3) - 2.0 / 3.0).abs() < 1e-5);
        assert_eq!(snap_to(0.0, 2), 0.25);
    }

    #[test]
    fn chords_are_written_in_readable_pieces() {
        // a dotted quarter on 1
        assert_eq!(written_pieces(0.0, 1.5, 2), vec![(0.0, 1.5)]);
        // from the "and" of 4 over the bar line: an 8th tied to a half
        assert_eq!(written_pieces(3.5, 2.5, 2), vec![(3.5, 0.5), (4.0, 2.0)]);
        // a quarter starting on the "e" of 1: a dotted 8th... no, a 16th-led split that lands back on the beat
        let p = written_pieces(0.25, 1.0, 2);
        assert_eq!(p.iter().map(|x| x.1).sum::<f32>(), 1.0);
        assert!(p.iter().all(|(s, l)| (s / 4.0).floor() == ((s + l - 1e-3) / 4.0).floor()), "{:?}", p);
        // triplets: a quarter and a triplet 8th
        let t = written_pieces(0.0, 4.0 / 3.0, 3);
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn a_progression_becomes_bars() {
        let c = |g: [Option<u8>; 6], beats: f32| LabChord { grip: g, name: String::new(), beats, timed: true, art: 0 };
        let p = LabProg {
            name: "walk".into(),
            bpm: 72.0,
            chords: vec![
                c([None, None, Some(6), Some(5), Some(4), Some(4)], 3.5),
                c([None, None, Some(6), Some(6), Some(6), Some(4)], 2.5),
            ],
            link: None,
        };
        let (bars, grips, which) = as_bars(&p, theory::Spelling::default(), 2);
        assert_eq!(bars.len(), 2);
        assert_eq!(grips.len(), 2);
        // the second chord starts on the "and" of 4 and is tied into bar 2
        let last = bars[0].beats.last().unwrap();
        assert_eq!((last.at, last.len), (3.5, 0.5));
        assert!(last.chord.is_some());
        assert!(bars[1].beats[0].notes.iter().all(|n| n[3] == 1));
        assert_eq!(which[1], vec![1]);
    }
}
