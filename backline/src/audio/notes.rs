//! The note view: how strong each semitone is, fifty times a second, for drawing a song by pitch (a heat map).
//! Constant-Q: each note is listened to over 17 to 34 of its own cycles (a semitone or narrower), on audio taken down
//! to about 11 kHz. It shows the sound, overtones and all; it does not guess the notes.

use std::sync::Arc;

/// The lowest and highest notes shown: E1 (a bass's low E) to E6 (the guitar's 24th fret).
pub const LO: i32 = 28;
pub const HI: i32 = 88;
pub const ROWS: usize = (HI - LO + 1) as usize;
pub const FPS: f32 = 50.0;

pub struct NoteMap {
    pub cols: usize,
    /// Strength 0..255, column after column (row 0 is `LO`).
    pub v: Vec<u8>,
}

impl NoteMap {
    pub fn at(&self, col: usize, row: usize) -> u8 {
        self.v.get(col * ROWS + row).copied().unwrap_or(0)
    }
}

/// From a song file, whole.
pub fn from_bytes(bytes: Vec<u8>) -> Option<NoteMap> {
    let (x, sr) = crate::tuning::decode_mono(bytes, 36000.0)?;
    Some(analyse(&x, sr))
}

/// From the stems switched on (stereo, 44.1 kHz), mixed to one and taken down 4:1.
pub fn from_stems(st: &crate::stems::Stems, on: [bool; 4]) -> NoteMap {
    let n = st.data.iter().map(|d| d.len()).max().unwrap_or(0) / 2;
    let mut x = Vec::with_capacity(n / 4 + 1);
    let (mut acc, mut k) = (0f32, 0);
    for i in 0..n {
        for s in 0..4 {
            if on[s] {
                if let Some(p) = st.data[s].get(2 * i..2 * i + 2) {
                    acc += p[0] as f32 + p[1] as f32;
                }
            }
        }
        k += 1;
        if k == 4 {
            x.push(acc / (4.0 * 65536.0));
            (acc, k) = (0.0, 0);
        }
    }
    analyse(&x, 44100.0 / 4.0)
}

pub fn analyse(x: &[f32], sr: f32) -> NoteMap {
    let hop = sr / FPS;
    let cols = (x.len() as f32 / hop) as usize;
    let q = 1.0 / (2f32.powf(1.0 / 12.0) - 1.0);
    let x: Arc<[f32]> = x.into();
    // the rows shared out over a few threads
    let threads = std::thread::available_parallelism().map_or(2, |n| n.get()).clamp(1, 4);
    let rows: Vec<(usize, Vec<f32>)> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..threads)
            .map(|t| {
                let x = x.clone();
                s.spawn(move || {
                    let mut out = Vec::new();
                    for r in (t..ROWS).step_by(threads) {
                        let f = 440.0 * 2f32.powf((LO + r as i32 - 69) as f32 / 12.0);
                        // twice the cycles puts the next semitone on the window's null (a clean, one-row bar), but
                        // no longer than 0.2 s so quick lines don't smear; never under one semitone's worth
                        let n = ((2.0 * q * sr / f).min(0.2 * sr).max(q * sr / f) as usize).max(16);
                        let w: Vec<f32> = (0..n).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos()).collect();
                        let k = 2.0 * (std::f32::consts::TAU * f / sr).cos();
                        let mut row = vec![0f32; cols];
                        for (c, cell) in row.iter_mut().enumerate() {
                            let centre = (c as f32 * hop) as usize;
                            if centre < n / 2 || centre + n / 2 >= x.len() {
                                continue;
                            }
                            let st = centre - n / 2;
                            let (mut s1, mut s2) = (0f32, 0f32);
                            for i in 0..n {
                                let s0 = x[st + i] * w[i] + k * s1 - s2;
                                s2 = s1;
                                s1 = s0;
                            }
                            *cell = (s1 * s1 + s2 * s2 - k * s1 * s2).max(0.0).sqrt() * 2.0 / n as f32;
                        }
                        out.push((r, row));
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    });
    // loudness in dB, the top 0.3% at full and 42 dB below it at nothing
    let mut db = vec![-140f32; cols * ROWS];
    for (r, row) in rows {
        for (c, v) in row.into_iter().enumerate() {
            db[c * ROWS + r] = 20.0 * v.max(1e-7).log10();
        }
    }
    let mut sorted: Vec<f32> = db.iter().copied().filter(|v| *v > -139.0).collect();
    let top = if sorted.is_empty() {
        0.0
    } else {
        let i = ((sorted.len() as f32 * 0.997) as usize).min(sorted.len() - 1);
        *sorted.select_nth_unstable_by(i, |a, b| a.total_cmp(b)).1
    };
    let v = db.iter().map(|d| (((d - (top - 42.0)) / 42.0).clamp(0.0, 1.0).powf(1.6) * 255.0) as u8).collect();
    NoteMap { cols, v }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_lights_its_own_row() {
        // A2 (110 Hz) for a second, then C4 (261.6 Hz) for a second
        let sr = 11025.0;
        let x: Vec<f32> = (0..(2.0 * sr) as usize)
            .map(|i| {
                let t = i as f32 / sr;
                let f = if t < 1.0 { 110.0 } else { 261.63 };
                (std::f32::consts::TAU * f * t).sin() * 0.5
            })
            .collect();
        let m = analyse(&x, sr);
        assert_eq!(m.cols, 100);
        let loudest = |c: usize| (0..ROWS).max_by_key(|r| m.at(c, *r)).unwrap() as i32 + LO;
        assert_eq!(loudest(25), 45);
        assert_eq!(loudest(75), 60);
        // and the next semitone along is quieter, far quieter from the middle of the guitar up
        assert!((m.at(25, (46 - LO) as usize) as u32) * 4 < m.at(25, (45 - LO) as usize) as u32 * 3);
        assert!(m.at(75, (61 - LO) as usize) < m.at(75, (60 - LO) as usize) / 2);
    }
}

#[cfg(test)]
mod timing {
    /// How long a five-minute song takes (run by hand: cargo test --release notes_timing -- --ignored --nocapture).
    #[test]
    #[ignore]
    fn notes_timing() {
        let sr = 11025.0;
        let x: Vec<f32> = (0..(300.0 * sr) as usize).map(|i| ((i * 7919) % 1000) as f32 / 1000.0 - 0.5).collect();
        let t = std::time::Instant::now();
        let m = super::analyse(&x, sr);
        println!("{} columns in {:?}", m.cols, t.elapsed());
    }
}
