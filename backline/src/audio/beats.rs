//! A song's beat from its audio, shared with the phone app (phone/): the onsets, the usual beat length from how they
//! repeat, then every beat placed along the song (Ellis's beat tracker). band.rs passes these on as its own.

use crate::decode::SymSource;
use rodio::Source;

/// A song's beat: its usual length (seconds), where the first one falls, and every beat's time through the song -
/// followed as the band speeds up or slows down, so a click locked to it stays with them.
pub type BeatInfo = (f32, f32, std::sync::Arc<Vec<f32>>);

/// The beat of a song (the first 15 minutes of it), from its audio.
pub fn detect_beats(bytes: Vec<u8>) -> Option<BeatInfo> {
    let src = SymSource::new(bytes).ok()?;
    let ch = src.channels().max(1) as usize;
    let hop = (src.sample_rate() as usize / 200).max(1);
    let (mut env, mut acc, mut n, mut c, mut mono, mut prev) = (Vec::<f32>::new(), 0f32, 0usize, 0usize, 0f32, 0f32);
    for s in src {
        mono += s as f32 / 32768.0;
        c += 1;
        if c == ch {
            let x = mono / ch as f32;
            (c, mono) = (0, 0.0);
            acc += (x - prev).abs();
            prev = x;
            n += 1;
            if n == hop {
                env.push(acc / hop as f32);
                (acc, n) = (0.0, 0);
                if env.len() >= 200 * 900 {
                    break;
                }
            }
        }
    }
    if env.len() < 800 {
        return None;
    }
    let w = 8;
    let mut on = vec![0f32; env.len()];
    let mut run: f32 = env[..w].iter().sum();
    for i in w..env.len() {
        on[i] = (env[i] - run / w as f32).max(0.0);
        run += env[i] - env[i - w];
    }
    track_beats(&on, 200.0)
}

/// The beat in an onset strength curve `on` (`fps` values a second): the usual beat length from how onsets repeat,
/// then every beat placed by dynamic programming (as Ellis's beat tracker): each beat lands on a strong onset about
/// one beat after the one before, the gap allowed to stretch a little, so a band drifting faster or slower is
/// followed beat by beat rather than drifting away from one fixed tempo.
pub fn track_beats(on: &[f32], fps: f32) -> Option<BeatInfo> {
    if on.len() < (4.0 * fps) as usize {
        return None;
    }
    // the usual beat: the strongest repeat distance in the first 2.5 minutes, leaning gently towards 120 bpm
    let head = &on[..on.len().min((150.0 * fps) as usize)];
    let (lo, hi) = ((fps * 60.0 / 200.0) as usize, (fps * 60.0 / 60.0) as usize);
    let (mut best, mut lag0) = (0.0f32, (fps / 2.0) as usize);
    for lag in lo..=hi.min(head.len() / 2) {
        let sum: f32 = (0..head.len() - lag).map(|i| head[i] * head[i + lag]).sum::<f32>() / (head.len() - lag) as f32;
        let bpm = 60.0 * fps / lag as f32;
        let score = sum * (-0.5 * ((bpm / 120.0).log2() / 0.9).powi(2)).exp();
        if score > best {
            (best, lag0) = (score, lag);
        }
    }
    if best <= 0.0 {
        return None;
    }
    let p = lag0 as f32;
    // dynamic programming: the best chain of beats ending at each frame
    let norm = {
        let mut v: Vec<f32> = on.to_vec();
        v.sort_by(|a, b| b.total_cmp(a));
        v[v.len() / 50].max(1e-6)
    };
    let o: Vec<f32> = on.iter().map(|x| (x / norm).min(4.0)).collect();
    let tight = 100.0f32;
    let mut score = vec![0f32; o.len()];
    let mut back = vec![usize::MAX; o.len()];
    for t in 0..o.len() {
        let (a, b) = ((t as f32 - 2.0 * p).max(0.0) as usize, (t as f32 - p / 2.0).max(0.0) as usize);
        let mut top = 0.0f32;
        let mut from = usize::MAX;
        if b > a {
            for q in a..b {
                let r = ((t - q) as f32 / p).ln();
                let s = score[q] - tight * r * r;
                if s > top || from == usize::MAX {
                    top = s;
                    from = q;
                }
            }
        }
        score[t] = o[t] + top.max(0.0);
        back[t] = if top > 0.0 { from } else { usize::MAX };
    }
    // the chain ending at the strongest place in the last beat, followed back
    let last = o.len() - 1;
    let start = (last as f32 - p).max(0.0) as usize;
    let mut t = (start..=last).max_by(|a, b| score[*a].total_cmp(&score[*b]))?;
    let mut beats = vec![t as f32 / fps];
    while back[t] != usize::MAX {
        t = back[t];
        beats.push(t as f32 / fps);
    }
    beats.reverse();
    if beats.len() < 8 {
        return None;
    }
    // the usual beat length: the middle of the gaps
    let mut gaps: Vec<f32> = beats.windows(2).map(|w| w[1] - w[0]).collect();
    gaps.sort_by(f32::total_cmp);
    let period = gaps[gaps.len() / 2];
    Some((period, beats[0], std::sync::Arc::new(beats)))
}

/// The beat after `pos` in a beat list and the local beat length there (the average of the next few gaps).
pub fn next_beat(grid: &[f32], pos: f32) -> Option<(f32, f32)> {
    let k = grid.partition_point(|b| *b <= pos + 0.002);
    let next = *grid.get(k)?;
    let end = (k + 4).min(grid.len() - 1);
    let local = if end > k {
        (grid[end] - grid[k]) / (end - k) as f32
    } else if k > 0 {
        grid[k] - grid[k - 1]
    } else {
        return None;
    };
    Some((next, local))
}
