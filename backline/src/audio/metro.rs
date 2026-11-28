//! The metronome's sound, shared with the phone app (phone/): what it clicks (`Metro`: beats, each beat's volume,
//! the beat unit, subdivisions, gap and speed-up drills, following or locking to the song), a cycle of it to loop
//! (`click_loop`), clicks laid on a song's own beats (`grid_clicks`, `click_sounds`). band.rs passes these on.

pub const RATE: f32 = 44100.0;

// --------------------------------------------------------------------- metronome
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Metro {
    pub on: bool,
    /// beats per bar
    pub beats: u32,
    /// volume of each beat of the bar, 0 (silent) to 3 (loud); click a beat to step through them
    pub levels: [u8; 32],
    /// the note value of one pulse: 4 (quarters), 8 (eighths) or 16. BPM is always counted in quarter notes, so 22/8 at 120 clicks 22 eighths at 240 a minute
    pub unit: u8,
    /// ticks between the beats: 0 none, 1 eighths, 2 triplets, 3 sixteenths
    pub sub: u8,
    /// play this many bars, then be silent for `gap_mute` (0 = never silent)
    pub gap_play: u32,
    pub gap_mute: u32,
    /// every this many bars get `ramp_bpm` faster (0 = off), up to `ramp_max`
    pub ramp_bars: u32,
    pub ramp_bpm: u32,
    pub ramp_max: u32,
    /// follow the speed setting of the track that is playing
    pub follow: bool,
    /// lock to the beats found in the playing track
    pub sync: bool,
    /// fine shift of the lock, in milliseconds
    pub shift_ms: i32,
}

impl Default for Metro {
    fn default() -> Metro {
        Metro {
            on: false,
            beats: 4,
            levels: {
                let mut l = [2u8; 32];
                l[0] = 3;
                l
            },
            unit: 4,
            sub: 0,
            gap_play: 0,
            gap_mute: 1,
            ramp_bars: 0,
            ramp_bpm: 4,
            ramp_max: 240,
            follow: true,
            sync: false,
            shift_ms: 0,
        }
    }
}

pub(crate) fn click(buf: &mut [i16], start: usize, f: f32, vel: f32) {
    for i in 0..(RATE * 0.035) as usize {
        if start + i >= buf.len() {
            break;
        }
        let t = i as f32 / RATE;
        let s = (std::f32::consts::TAU * f * t).sin() * (-t * 130.0).exp();
        buf[start + i] = buf[start + i].saturating_add((s * 15000.0 * vel) as i16);
    }
}

impl Metro {
    /// Clicks per real minute for a tempo given in quarter notes.
    pub fn pulse(&self, bpm: f32) -> f32 {
        bpm * self.unit as f32 / 4.0
    }

    /// Accent a grouping such as 3+3+3+4: loud on the bar's first beat, medium on each group start, soft in between.
    pub fn apply_group(&mut self, g: &[u32]) {
        for l in self.levels.iter_mut() {
            *l = 1;
        }
        let mut at = 0usize;
        for (i, size) in g.iter().enumerate() {
            if at < 32 {
                self.levels[at] = if i == 0 { 3 } else { 2 };
            }
            at += *size as usize;
        }
    }

    /// Preset volumes: every beat, only 2 and 4 (the jazz way), or only the first beat.
    pub fn preset(&mut self, which: u8) {
        for b in 0..32 {
            self.levels[b] = match which {
                0 => {
                    if b == 0 {
                        3
                    } else {
                        2
                    }
                }
                1 => {
                    if b % 2 == 1 {
                        3
                    } else {
                        0
                    }
                }
                _ => {
                    if b == 0 {
                        3
                    } else {
                        0
                    }
                }
            };
        }
    }
}

/// Ways to split `n` pulses into groups of 2, 3 and 4 (7 = 2+2+3, 22 = 3+3+3+3+3+3+4 ...).
pub fn groupings(n: u32) -> Vec<Vec<u32>> {
    let mut out: Vec<Vec<u32>> = Vec::new();
    for size in [3u32, 2, 4] {
        if n < size {
            continue;
        }
        let (q, r) = (n / size, n % size);
        let mut g = vec![size; q as usize];
        match r {
            0 => {}
            1 => *g.last_mut().unwrap() += 1,
            _ => g.push(r),
        }
        let mut alt = g.clone();
        alt.rotate_right(1);
        for cand in [g, alt] {
            if cand.iter().sum::<u32>() == n && !out.contains(&cand) {
                out.push(cand);
            }
        }
    }
    out
}

/// One cycle of the metronome (a bar, or the play + silent bars of a gap drill), to repeat forever.
pub fn click_loop(m: &Metro, bpm: f32) -> Vec<i16> {
    let beats = m.beats.clamp(1, 32) as usize;
    let spb = 60.0 / m.pulse(bpm.clamp(20.0, 400.0)).clamp(20.0, 1000.0) * RATE;
    let bars = if m.gap_play > 0 { (m.gap_play + m.gap_mute.max(1)) as usize } else { 1 };
    let mut v = vec![0i16; ((bars * beats) as f32 * spb).round() as usize + 1];
    for bar in 0..bars {
        if m.gap_play > 0 && bar >= m.gap_play as usize {
            break;
        }
        for b in 0..beats {
            let at = ((bar * beats + b) as f32 * spb) as usize;
            let lvl = m.levels[b].min(3) as usize;
            if lvl > 0 {
                click(&mut v, at, [0.0, 1000.0, 1250.0, 1650.0][lvl], [0.0, 0.4, 0.7, 1.0][lvl]);
            }
            let div = [1usize, 2, 3, 4][(m.sub as usize).min(3)];
            for k in 1..div {
                click(&mut v, at + (spb * k as f32 / div as f32) as usize, 2300.0, 0.3);
            }
        }
    }
    v
}

/// Clicks on a song's own beats: every beat in `grid` (song seconds) from `from` for `secs` seconds of song, placed
/// at playback `ratio` (0.85 = slowed to 85%), `shift` seconds late, at `vol`. The first beat of every `beats` is
/// the accented one, counted from the first beat of the song.
pub fn grid_clicks(grid: &[f32], from: f32, secs: f32, ratio: f32, shift: f32, beats: u32, vol: f32) -> Vec<i16> {
    let len = ((secs / ratio) * RATE) as usize + 1;
    let mut v = vec![0i16; len];
    let first = grid.partition_point(|b| *b < from);
    for (k, b) in grid.iter().enumerate().skip(first) {
        if *b > from + secs {
            break;
        }
        let at = (((b - from) / ratio + shift).max(0.0) * RATE) as usize;
        let down = k % beats.max(1) as usize == 0;
        click(&mut v, at, if down { 1650.0 } else { 1250.0 }, if down { vol } else { vol * 0.7 });
    }
    v
}

/// The metronome's two clicks as sounds (the accented one higher), for mixing into a song on its beats.
pub fn click_sounds() -> (Vec<f32>, Vec<f32>) {
    let make = |f: f32, vel: f32| {
        let mut v = vec![0i16; (RATE * 0.06) as usize];
        click(&mut v, 0, f, vel);
        v.iter().map(|s| *s as f32 / 32768.0).collect::<Vec<f32>>()
    };
    (make(1650.0, 1.0), make(1250.0, 0.7))
}
