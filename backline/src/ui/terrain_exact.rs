//! The Terrain as its reference works it out, step by step: a ring of 128 rows, a new one each unit the camera
//! travels, made from the music's spectrum (lows at both edges and highs in the middle, the left channel on the left
//! half, the right on the right) on a valley, each column following on a spring and the row smoothed across; the
//! camera at a set offset from the travel point, its view turning a tenth of the way each frame; and seven groups of
//! settings sets (`super::sets`), a group's sets played in turn, each blending in from the last. The shading, focus
//! and trail passes are the Terrain's own (they already match).

use super::sets::{k, BASE, GROUPS, SETS};
use super::{lk, Look, Sheet, COLS, ROWS};

/// The far edge the sheet is cut at, past the travel point.
const CLIP: f32 = 126.9;

pub(super) struct Exact {
    /// the settings blending from (`from`, since `t_from`, over `blend` seconds) toward the set showing (`set`, in
    /// since `t_set`)
    from: [f32; k::N],
    t_from: f64,
    blend: f32,
    set: usize,
    t_set: f64,
    song: u64,
    /// how far the camera has travelled, and where the next row is due
    z: f64,
    threshold: f64,
    /// the newest row's heights and each column's spring
    prev: [f32; 128],
    vel: [f32; 128],
    /// where the camera looks (it turns toward its target a tenth of the way a frame)
    fwd: [f32; 3],
    seed: u64,
}

impl Exact {
    /// Starting afresh at `t`: everything dark and a set of a random group coming in over two seconds, on a flat sheet
    /// ten units before its first row.
    pub(super) fn new(t: f64, song: u64, sheet: &mut Sheet) -> Exact {
        let mut e = Exact {
            from: BASE,
            t_from: t,
            blend: 2.0,
            set: 0,
            t_set: t,
            song,
            z: -10.0,
            threshold: 0.0,
            prev: [0.0; 128],
            vel: [0.0; 128],
            fwd: [0.0, 0.0, 1.0],
            seed: 0x9e3779b97f4a7c15 ^ song.wrapping_mul(0x2545f4914f6cdd1d),
        };
        let g = (e.rnd01() * GROUPS as f32) as usize % GROUPS;
        e.set = e.random_in(g);
        sheet.next = 2;
        let flat = [0.0; COLS];
        for _ in 0..ROWS {
            sheet.push_row(&flat, &flat, 0.0);
        }
        let v = e.values(t);
        e.fwd = unit(sub(target(&v, 0.0), eye(&v, 0.0)));
        e
    }

    /// 0..1
    fn rnd01(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 40) as f32 / 16777216.0
    }

    fn random_in(&mut self, g: usize) -> usize {
        let members: Vec<usize> = (0..SETS.len()).filter(|&i| SETS[i].group == g).collect();
        members[(self.rnd01() * members.len() as f32) as usize % members.len()]
    }

    /// The settings at `t`: partway from where the last change started to the set showing.
    fn values(&self, t: f64) -> [f32; k::N] {
        let a = (((t - self.t_from) / self.blend.max(0.01) as f64) as f32).clamp(0.0, 1.0);
        let a = a * a * (3.0 - 2.0 * a);
        let to = &SETS[self.set].v;
        let mut v = [0.0; k::N];
        for i in 0..k::N {
            v[i] = self.from[i] + (to[i] - self.from[i]) * a;
        }
        v
    }

    /// Over to another set at `t` (a random group other than this one, or the next set of this group), blending from
    /// where things are now.
    fn change(&mut self, t: f64, new_group: bool) {
        self.from = self.values(t);
        self.t_from = t;
        let g = SETS[self.set].group;
        self.set = if new_group {
            let mut r = (self.rnd01() * (GROUPS - 1) as f32) as usize % (GROUPS - 1);
            if r >= g {
                r += 1;
            }
            self.random_in(r)
        } else {
            let members: Vec<usize> = (0..SETS.len()).filter(|&i| SETS[i].group == g).collect();
            let at = members.iter().position(|&i| i == self.set).unwrap_or(0);
            members[(at + 1) % members.len()]
        };
        self.blend = SETS[self.set].blend;
        self.t_set = t;
    }

    /// One frame: `dt` seconds on, `spec` the music now (128 columns, edge to edge, see `columns`). New rows go into
    /// `sheet`; returns the look to draw, the line's place (the sheet's front) and the colour table (512 colours).
    pub(super) fn step(&mut self, t: f64, dt: f64, song: u64, spec: &[f32; 128], sheet: &mut Sheet) -> (Look, f64, Vec<f32>) {
        if song != self.song {
            self.song = song;
            self.change(t, true);
        } else if t - self.t_set > SETS[self.set].hold as f64 {
            self.change(t, false);
        }
        let v = self.values(t);
        self.z += dt * v[k::POS_SPEED].max(0.0) as f64;
        // a row each unit travelled, as many as sixty frames a second would make
        let cap = ((dt * 60.0).ceil() as usize).max(1);
        let mut made = 0;
        while self.threshold < self.z && made < cap {
            self.threshold += 1.0;
            self.row(&v, spec, sheet);
            made += 1;
        }
        if self.threshold < self.z - ROWS as f64 {
            self.threshold = self.z.floor();
        }
        // the camera turns toward its target a tenth of the way a frame
        let want = unit(sub(target(&v, self.z), eye(&v, self.z)));
        let a = 1.0 - 0.9f32.powf((dt * 60.0) as f32);
        self.fwd = unit([0, 1, 2].map(|i| self.fwd[i] + (want[i] - self.fwd[i]) * a));
        (self.look(&v), self.z + v[k::LINE_AHEAD] as f64, ramp(&v))
    }

    /// A new row: the valley plus the music plus a little noise, each column following on a spring (quick in the
    /// middle, slow at the edges), then smoothed across five times; drawn one unit past the newest.
    fn row(&mut self, v: &[f32; k::N], spec: &[f32; 128], sheet: &mut Sheet) {
        let (vh, vc, vw, vs) = (v[k::VALLEY_HEIGHT], v[k::VALLEY_CENTRE], v[k::VALLEY_WIDTH], v[k::VALLEY_SHARP]);
        let mut y = [0.0f32; COLS];
        let mut w = [0.0f32; COLS];
        for i in 0..128 {
            let xn = i as f32 / 64.0 - 1.0;
            let s = (((xn - vc).abs() - vw) * vs).clamp(0.0, 1.0);
            let h = vh * s * s * (3.0 - 2.0 * s) + spec[i] + 0.02 * self.rnd01();
            let kk = 0.7 + 0.295 * xn.abs();
            self.vel[i] += (h - self.prev[i]) * (1.0 - kk);
            y[i] = self.prev[i] + self.vel[i];
            w[i] = h;
            self.vel[i] *= 0.5 * kk;
        }
        for _ in 0..5 {
            for i in 0..128 {
                y[i] += 0.25 * (y[i.saturating_sub(1)] + y[(i + 1).min(127)] - 2.0 * y[i]);
            }
        }
        self.prev.copy_from_slice(&y[..128]);
        y[128] = y[127];
        w[128] = w[127];
        sheet.next = self.threshold as i64 + 129;
        sheet.push_row(&y, &w, -0.25 * v[k::TERRAIN_SLIDE]);
    }

    /// The settings as the Terrain's look: the eye and the swell's band measured from the line (the sheet's front).
    fn look(&self, v: &[f32; k::N]) -> Look {
        let mut l = [0f32; lk::N];
        let fade = v[k::FADE_COLOUR];
        l[lk::EYE] = v[k::POS_X];
        l[lk::EYE + 1] = v[k::POS_Y];
        l[lk::EYE + 2] = v[k::LINE_AHEAD] - v[k::POS_Z];
        l[lk::FWD..lk::FWD + 3].copy_from_slice(&self.fwd);
        l[lk::FOV] = v[k::ZOOM];
        l[lk::XS] = v[k::SCALE_X];
        l[lk::HS] = 0.3 * v[k::TERRAIN_HEIGHT];
        l[lk::FOG_RANGE] = (v[k::FOG_MAX] - v[k::FOG_MIN]).max(0.01);
        l[lk::FOG_START] = v[k::FOG_MIN];
        l[lk::TILE] = v[k::TEXTURE_SCALE];
        l[lk::LIFT] = v[k::TIME_AHEAD] - v[k::LINE_AHEAD];
        l[lk::CLIP] = CLIP - v[k::LINE_AHEAD];
        l[lk::SWELL..lk::SWELL + 4].copy_from_slice(&[v[k::LINE_WIDTH].max(0.01), v[k::LINE_HEIGHT], v[k::LINE_LENGTH], v[k::LINE_BIAS]]);
        l[lk::BUMP] = v[k::BUMP_SCALE];
        for c in 0..3 {
            l[lk::GLOW + c] = v[k::LINE_R + c] * fade;
            l[lk::FOG + c] = v[k::FOG_R + c] * fade;
            l[lk::SIDE + c] = v[k::FOG_R + c] * fade;
        }
        l[lk::FOCUS] = v[k::FOCAL_DEPTH];
        l[lk::COC] = 1.0 / v[k::FOCAL_VARIANCE].max(0.01);
        l[lk::SKY] = v[k::SKY_BLUR];
        l[lk::BLUR] = v[k::LOD];
        l[lk::LEAK] = v[k::FEEDBACK].max(0.0);
        l[lk::ROT] = v[k::FEEDBACK_ROT];
        l[lk::LSCALE] = v[k::FEEDBACK_X];
        l[lk::LSCALE + 1] = v[k::FEEDBACK_Y];
        l[lk::SPEED] = v[k::POS_SPEED];
        l
    }
}

fn eye(v: &[f32; k::N], z: f64) -> [f32; 3] {
    [v[k::POS_X], v[k::POS_Y], (z + v[k::POS_Z] as f64) as f32]
}

fn target(v: &[f32; k::N], z: f64) -> [f32; 3] {
    [v[k::TARGET_X], v[k::TARGET_Y], (z + v[k::TARGET_Z] as f64) as f32]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-5);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// The colour by how squarely a spot faces the camera, 512 steps from edge-on to face-on:
/// ((1 - u) ^ (R, G, B) x scale + bias) x fade.
fn ramp(v: &[f32; k::N]) -> Vec<f32> {
    let mut out = Vec::with_capacity(512 * 3);
    for i in 0..512 {
        let u = 1.0 - i as f32 / 511.0;
        for c in 0..3 {
            out.push((u.powf(v[k::COLOUR_R + c]) * v[k::COLOUR_SCALE] + v[k::COLOUR_BIAS]) * v[k::FADE_COLOUR]);
        }
    }
    out
}

/// The music as the 128 columns a row is made from: two spectra (256 values each) read at 64 places, `a` low to high
/// across the first half and `b` high to low across the second (so the lows are at both edges); tilted toward the
/// highs by the set's TERRAIN WIDTH and brought down to a peak of one when louder.
pub(super) fn columns(a: &[f32; 256], b: &[f32; 256], tilt: f32) -> [f32; 128] {
    let mut s = [0.0f32; 128];
    for i in 0..64 {
        s[i] = a[i * 255 / 64];
        s[64 + i] = b[255 - (i * 255 + 63) / 64];
    }
    let mut peak = 1.0f32;
    for (i, v) in s.iter_mut().enumerate() {
        *v *= (tilt * (1.0 - (1.0 - i as f32 / 64.0).abs())).exp();
        peak = peak.max(*v);
    }
    s.map(|v| v / peak)
}

impl Exact {
    /// The set's tilt now (for `columns`).
    pub(super) fn tilt(&self, t: f64) -> f32 {
        self.values(t)[k::TERRAIN_WIDTH]
    }
}

/// The system's spectrum of the music, as it works it out: the latest 512 samples at 48 kHz (other rates held
/// sample by sample), a narrow bell window, a 512-point FFT (magnitudes over 512), blurred across 17 bins, then read at
/// 256 places spaced by pitch (about 18 Hz apart at the bottom, 64 to an octave above 1 kHz, up to 24 kHz), each as
/// 3 ln(1 + 512 m) / ln 513 (a full-scale sine reads about 1.8). It reads that far ahead of what is heard on the
/// original (each set's `ahead`); here only what has played is at hand. One channel, so left and right are the same.
pub(super) fn spectra(samples: &[f32], rate: f32) -> ([f32; 256], [f32; 256]) {
    const N: usize = 512;
    let mut out = [0f32; 256];
    if samples.is_empty() || rate <= 0.0 {
        return (out, out);
    }
    let (window, _) = bell(256);
    let (kern, ksum) = bell(8);
    let step = rate as f64 / 48000.0;
    let mut re = vec![0f32; N];
    let mut im = vec![0f32; N];
    let first = samples.len() as f64 - N as f64 * step;
    for (i, v) in re.iter_mut().enumerate() {
        let at = first + i as f64 * step;
        if at >= 0.0 {
            *v = samples[(at as usize).min(samples.len() - 1)] * window[i];
        }
    }
    super::fft(&mut re, &mut im);
    let mut sm = [0f32; 256];
    for k in 0..256 {
        let m = (re[k] * re[k] + im[k] * im[k]).sqrt() / N as f32;
        for (t, w) in kern.iter().enumerate() {
            let j = k as i32 - 8 + t as i32;
            if (0..256).contains(&j) {
                sm[j as usize] += m / ksum * w;
            }
        }
    }
    let top = 2f32.powf(3.98462) - 1.0;
    let ln513 = 513f32.ln();
    for (i, o) in out.iter_mut().enumerate() {
        let pos = (2f64.powf((i as f64 + 0.015625) * 0.015625) as f32 - 1.0) * 255.0 / top;
        let k = (pos.floor() as usize).min(255);
        let fr = pos - k as f32;
        let v = if fr <= 0.0 { sm[k] } else { (1.0 - fr) * sm[k] + fr * if k + 1 < 256 { sm[k + 1] } else { 0.0 } };
        *o = 3.0 * (1.0 + 512.0 * v).ln() / ln513;
    }
    (out, out)
}

/// The bell the analysis uses for its window (n = 256: 512 values) and its blur (n = 8: 17 taps): 2n + 1 values of
/// 2.7 ^ (-(2.355 (i - n) / n)^2 / 0.56) x 2.5 / sqrt(6.36159), the first set to zero; and their sum.
fn bell(n: usize) -> (Vec<f32>, f32) {
    let scale = 2.5 / 6.36159f32.sqrt();
    let mut p: Vec<f32> = (0..2 * n + 1)
        .map(|i| {
            let x = (i as f32 - n as f32) * 2.355 / n as f32;
            2.7f32.powf(-x * x / 0.56) * scale
        })
        .collect();
    p[0] = 0.0;
    let sum = p.iter().sum();
    (p, sum)
}

#[cfg(test)]
mod tests {
    use super::spectra;

    /// Test tones read as the reference analysis reads them (a full-scale 1 kHz sine peaks at 1.824 in place 45).
    #[test]
    fn spectrum_matches_the_reference() {
        let tone = |f: f32, a: f32| (0..512).map(|i| a * (std::f32::consts::TAU * f * i as f32 / 48000.0).sin()).collect::<Vec<f32>>();
        let (l, r) = spectra(&tone(1000.0, 1.0), 48000.0);
        for (i, want) in [(45, 1.824), (32, 1.287), (64, 0.289), (0, 0.001), (128, 0.0)] {
            assert!((l[i] - want).abs() < 0.01, "1 kHz, place {}: {} not {}", i, l[i], want);
        }
        assert_eq!(l, r);
        let (l, _) = spectra(&tone(100.0, 0.5), 48000.0);
        for (i, want) in [(10, 1.329), (0, 1.201), (32, 0.246)] {
            assert!((l[i] - want).abs() < 0.01, "100 Hz, place {}: {} not {}", i, l[i], want);
        }
        let (l, _) = spectra(&tone(5000.0, 0.5), 48000.0);
        assert!((l[128] - 1.374).abs() < 0.01, "5 kHz: {}", l[128]);
    }
}
