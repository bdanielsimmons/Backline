//! The little sound Backline makes when it opens: its own chime (made here in code: glassy bell tones rising
//! over a soft pad, washed in reverb, in the spirit of the glossy "aero" years), or a short file of your choice.

use crate::decode::SymSource;
use rodio::Source;
use std::f32::consts::TAU;

const RATE: u32 = 44100;

/// Play the opening sound on `device` at `vol` (0..1), off the main thread. `file`: your own sound (its first
/// eight seconds); None: Backline's chime.
pub fn play(file: Option<String>, device: Option<String>, vol: f32) {
    // muted (a test copy, a demo recording): no chime
    if crate::player::MUTED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    std::thread::spawn(move || {
        let Ok((_stream, handle)) = crate::player::open_named(device.as_deref()) else { return };
        let Ok(sink) = rodio::Sink::try_new(&handle) else { return };
        sink.set_volume(vol.clamp(0.0, 1.0));
        match file.and_then(|f| std::fs::read(f).ok()).and_then(|b| SymSource::new(b).ok()) {
            Some(src) => sink.append(src.take_duration(std::time::Duration::from_secs(8))),
            None => sink.append(rodio::buffer::SamplesBuffer::new(2, RATE, chime())),
        }
        sink.sleep_until_end();
    });
}

/// A bell: a sine carrier with a sine modulator (FM) whose brightness fades faster than the note.
fn bell(out: &mut [f32], start: f32, freq: f32, len: f32, amp: f32) {
    let s0 = (start * RATE as f32) as usize;
    let n = (len * RATE as f32) as usize;
    for i in 0..n {
        let Some(o) = out.get_mut(s0 + i) else { break };
        let t = i as f32 / RATE as f32;
        let env = (-t * 3.2 / len).exp() * (i.min(120) as f32 / 120.0);
        let index = 2.2 * (-t * 9.0).exp();
        let m = (t * freq * 3.5 * TAU).sin() * index;
        *o += (t * freq * TAU + m).sin() * env * amp;
    }
}

/// Backline's opening chime, stereo: a rising E major 9 arpeggio of glassy bells with high sparkles, over a
/// slowly swelling pad, through a roomy reverb.
pub fn chime() -> Vec<f32> {
    let secs = 4.2;
    let n = (secs * RATE as f32) as usize;
    let mut dry = vec![0.0f32; n];
    // the arpeggio: E5 G#5 B5 D#6 F#6, then a high B6 to finish
    for (k, f) in [659.25f32, 830.61, 987.77, 1244.51, 1479.98, 1975.53].iter().enumerate() {
        bell(&mut dry, 0.05 + k as f32 * 0.085, *f, 1.6, if k == 5 { 0.10 } else { 0.16 });
    }
    // sparkles: two very high, quiet pings
    bell(&mut dry, 0.55, 2959.96, 0.6, 0.05);
    bell(&mut dry, 0.70, 3951.07, 0.5, 0.035);
    // the pad: E4 and B4, gently detuned, swelling in and fading out
    for i in 0..n {
        let t = i as f32 / RATE as f32;
        let env = (t / 0.9).min(1.0) * (1.0 - ((t - 1.2) / 2.4).clamp(0.0, 1.0));
        let mut v = 0.0;
        for (f, d) in [(329.63f32, 0.0f32), (329.63, 1.3), (493.88, -0.9), (493.88, 0.7)] {
            v += ((f + d) * t * TAU).sin();
        }
        dry[i] += v * 0.025 * env;
    }
    reverb(&dry)
}

/// A small Freeverb-style room: four damped combs and two allpasses per side (slightly different lengths
/// left and right, for width), mixed with the dry sound. Interleaved stereo out.
pub(crate) fn reverb(dry: &[f32]) -> Vec<f32> {
    let side = |spread: usize| -> Vec<f32> {
        let mut combs: Vec<(Vec<f32>, usize, f32)> =
            [1116usize, 1188, 1277, 1356].iter().map(|l| (vec![0.0; l + spread], 0usize, 0.0f32)).collect();
        let mut alls: Vec<(Vec<f32>, usize)> = [556usize, 441].iter().map(|l| (vec![0.0; l + spread], 0usize)).collect();
        let (fb, damp) = (0.86f32, 0.25f32);
        dry.iter()
            .map(|&x| {
                let input = x * 0.3;
                let mut acc = 0.0;
                for (buf, i, store) in combs.iter_mut() {
                    let y = buf[*i];
                    *store = y * (1.0 - damp) + *store * damp;
                    buf[*i] = input + *store * fb;
                    *i = (*i + 1) % buf.len();
                    acc += y;
                }
                for (buf, i) in alls.iter_mut() {
                    let b = buf[*i];
                    buf[*i] = acc + b * 0.5;
                    acc = b - acc;
                    *i = (*i + 1) % buf.len();
                }
                acc
            })
            .collect()
    };
    let (l, r) = (side(0), side(23));
    let mut out = Vec::with_capacity(dry.len() * 2);
    for i in 0..dry.len() {
        // fade the very end so the tail never clicks
        let tail = ((dry.len() - i) as f32 / 4410.0).min(1.0);
        out.push((dry[i] * 0.8 + l[i] * 0.45) * tail);
        out.push((dry[i] * 0.8 + r[i] * 0.45) * tail);
    }
    // brought to a comfortable level, well short of clipping
    let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-6);
    let k = 0.7 / peak;
    out.iter_mut().for_each(|v| *v *= k);
    out
}

/// A soft synth pad: a few gently detuned saws per note through a low-pass filter, swelling in over `att` seconds
/// and dying over `rel`, added to `out` from `start` for `len` seconds.
fn pad(out: &mut [f32], start: f32, notes: &[f32], len: f32, att: f32, rel: f32, amp: f32, cutoff: f32) {
    let s0 = (start * RATE as f32) as usize;
    let n = ((len + rel) * RATE as f32) as usize;
    let a = 1.0 - (-TAU * cutoff / RATE as f32).exp();
    for &f in notes {
        for (d, ph0) in [(-0.004f32, 0.0f32), (0.0, 0.33), (0.0045, 0.71)] {
            let fr = f * (1.0 + d);
            let (mut ph, mut lp1, mut lp2) = (ph0, 0.0f32, 0.0f32);
            for i in 0..n {
                let Some(o) = out.get_mut(s0 + i) else { break };
                let t = i as f32 / RATE as f32;
                ph = (ph + fr / RATE as f32).fract();
                let saw = ph * 2.0 - 1.0;
                lp1 += a * (saw - lp1);
                lp2 += a * (lp1 - lp2);
                let env = if t < att { (t / att).powf(1.6) } else if t < len { 1.0 } else { (1.0 - (t - len) / rel).max(0.0).powi(2) };
                *o += lp2 * env * amp / notes.len() as f32;
            }
        }
    }
}

/// Air: soft high noise through a gentle band, swelling up and away (the shimmer under a glossy pad).
fn air(out: &mut [f32], start: f32, len: f32, amp: f32) {
    let s0 = (start * RATE as f32) as usize;
    let n = (len * RATE as f32) as usize;
    let (mut seed, mut lp) = (12345u32, 0.0f32);
    for i in 0..n {
        let Some(o) = out.get_mut(s0 + i) else { break };
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let w = (seed >> 9) as f32 / (1u32 << 23) as f32 * 2.0 - 1.0;
        // (the noise less its low part: only the airy top)
        lp += 0.25 * (w - lp);
        let t = i as f32 / n as f32;
        let env = (t * std::f32::consts::PI).sin().powi(2);
        *o += (w - lp) * env * amp;
    }
}

/// `dry` through the room, then brought to a quiet level: `peak` of full scale at most.
fn finish(dry: &[f32], peak: f32) -> Vec<f32> {
    let mut out = reverb(dry);
    let now = out.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-6);
    let k = peak / 0.7 * (0.7 / now);
    out.iter_mut().for_each(|v| *v *= k);
    out
}

/// Candidate opening sounds, quieter and softer than the old chime (by name, for choosing one).
pub fn candidates() -> Vec<(&'static str, Vec<f32>)> {
    let len = |s: f32| vec![0.0f32; (s * RATE as f32) as usize];
    // E2 B2 E3 G#3 B3 D#4 F#4 ... as frequencies
    let hz = |m: i32| 440.0 * 2f32.powf((m - 69) as f32 / 12.0);
    let mut out = Vec::new();

    // 1 GLASS PAD: one Emaj9 chord swelling in, a single soft bell on top
    let mut d = len(4.5);
    pad(&mut d, 0.0, &[hz(52), hz(59), hz(63), hz(66), hz(68)], 1.6, 0.7, 2.2, 0.55, 1400.0);
    bell(&mut d, 0.55, hz(80), 2.0, 0.05);
    out.push(("1 glass pad", finish(&d, 0.22)));

    // 2 AERO SWELL: the pad with air rising under it and a high harmonic ping
    let mut d = len(4.8);
    pad(&mut d, 0.0, &[hz(52), hz(59), hz(64), hz(68), hz(71), hz(75)], 1.8, 0.9, 2.4, 0.5, 1800.0);
    air(&mut d, 0.0, 2.6, 0.02);
    bell(&mut d, 0.8, hz(87), 1.6, 0.03);
    bell(&mut d, 0.95, hz(83), 1.6, 0.025);
    out.push(("2 aero swell", finish(&d, 0.22)));

    // 3 TWO CHORDS: Asus2 to Emaj9, a soft plagal lift
    let mut d = len(5.0);
    pad(&mut d, 0.0, &[hz(45), hz(57), hz(59), hz(64), hz(69)], 1.0, 0.5, 0.8, 0.5, 1300.0);
    pad(&mut d, 1.1, &[hz(52), hz(59), hz(63), hz(66), hz(68)], 1.4, 0.4, 2.2, 0.5, 1500.0);
    bell(&mut d, 1.2, hz(78), 1.8, 0.04);
    out.push(("3 two chords", finish(&d, 0.22)));

    // 4 SOFT BELLS: the old chime's rising bells, darker and much quieter, over the pad
    let mut d = len(4.5);
    pad(&mut d, 0.0, &[hz(52), hz(59), hz(63)], 1.6, 0.8, 2.0, 0.45, 1100.0);
    for (k, m) in [64, 68, 71, 75, 78].iter().enumerate() {
        bell(&mut d, 0.25 + k as f32 * 0.11, hz(*m), 1.4, 0.06);
    }
    out.push(("4 soft bells", finish(&d, 0.2)));
    out
}

#[cfg(test)]
mod tests {
    /// Writes the candidate opening sounds as WAV files to $BACKLINE_CHIMES (a folder), to listen to.
    #[test]
    #[ignore]
    fn write_candidates() {
        let Ok(dir) = std::env::var("BACKLINE_CHIMES") else { return };
        for (name, s) in super::candidates() {
            let pcm: Vec<i16> = s.iter().map(|v| (v.clamp(-1.0, 1.0) * 32767.0) as i16).collect();
            std::fs::write(format!("{}/{}.wav", dir, name), crate::tour::wav(&pcm, super::RATE, 2)).unwrap();
        }
        // and the old one, to compare
        let pcm: Vec<i16> = super::chime().iter().map(|v| (v.clamp(-1.0, 1.0) * 32767.0) as i16).collect();
        std::fs::write(format!("{}/0 current chime.wav", dir), crate::tour::wav(&pcm, super::RATE, 2)).unwrap();
    }

    #[test]
    fn the_chime_is_a_few_seconds_of_clean_stereo() {
        let c = super::chime();
        assert_eq!(c.len() % 2, 0);
        let secs = c.len() as f32 / 2.0 / super::RATE as f32;
        assert!((3.0..6.0).contains(&secs));
        let peak = c.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.1 && peak < 1.0, "audible and not clipping: {}", peak);
        assert!(c.iter().all(|v| v.is_finite()));
    }
}
