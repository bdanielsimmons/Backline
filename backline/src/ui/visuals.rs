//! Visuals: the 3D Terrain the music builds (ui/terrain.rs, on the graphics card), two old Windows screensavers
//! (3D Pipes, Starfield) drawn by egui, and the classic Windows Media Player visualizers (ui/retro.rs, numbered after
//! `PRESETS`). Shown full screen, behind Now Playing, in a window of their own (ui/popout.rs), and as a screensaver
//! when you step away.

use crate::Within;
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use eframe::egui_glow;
use std::sync::{Arc, Mutex};

/// A preset: its group in the menu, its name, and how it's drawn.
pub(crate) struct Preset {
    pub group: &'static str,
    pub name: &'static str,
    pub kind: Kind,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    Pipes,
    Stars,
    /// 3D (ui/terrain.rs): the terrain the music builds as the camera glides over it.
    Terrain,
    /// 3D (ui/wave.rs): the wave, a sheet of light drifting across. Kept, but not offered (see `hidden`).
    Wave,
}

pub(crate) const PRESETS: &[Preset] = &[
    Preset { group: "3D", name: "Terrain", kind: Kind::Terrain },
    Preset { group: "3D", name: "Wave", kind: Kind::Wave },
    Preset { group: "Screensavers", name: "3D Pipes", kind: Kind::Pipes },
    Preset { group: "Screensavers", name: "Starfield", kind: Kind::Stars },
];

/// The menu's groups, in order.
pub(crate) const GROUPS: &[&str] = &["3D", "Screensavers"];

/// Every preset: `PRESETS`, then the classic ones (ui/retro.rs) in their order.
pub(crate) fn count() -> usize {
    PRESETS.len() + crate::retro::list().len()
}

/// A classic preset's place in ui/retro.rs's list (None for one of `PRESETS`).
pub(crate) fn retro_of(i: usize) -> Option<usize> {
    i.checked_sub(PRESETS.len()).filter(|&k| k < crate::retro::list().len())
}

/// The menu's group for the classic ones, and how they're saved ("Retro: Family: Name").
pub(crate) const RETRO: &str = "Retro";

/// A preset by name, as saved ("Group: Name", or an older plain name); one no longer kept is the Terrain.
pub(crate) fn find(name: &str) -> usize {
    if let Some((fam, n)) = name.strip_prefix("Retro: ").and_then(|r| r.split_once(": ")) {
        let l = crate::retro::list();
        return l.iter().position(|e| e.group == fam && e.name == n).map_or(0, |k| PRESETS.len() + k);
    }
    let (g, n) = name.split_once(": ").unwrap_or(("", name));
    PRESETS
        .iter()
        .position(|p| p.name.eq_ignore_ascii_case(n) && (g.is_empty() || p.group == g))
        .or_else(|| PRESETS.iter().position(|p| p.name.eq_ignore_ascii_case(n)))
        .unwrap_or(0)
}

/// Kept in the code but not offered: in no menu, and passed over by the arrows.
pub(crate) fn hidden(i: usize) -> bool {
    i < PRESETS.len() && matches!(PRESETS[i].kind, Kind::Wave)
}

/// What a saved choice shows: itself, or the Terrain in place of one not offered.
pub(crate) fn offered(i: usize) -> usize {
    if hidden(i) || i >= count() {
        0
    } else {
        i
    }
}

/// The preset `by` steps on from `i` (`count() - 1` steps back one), passing over those not offered.
pub(crate) fn next_shown(i: usize, by: usize) -> usize {
    let n = count();
    let mut j = i.min(n - 1);
    for _ in 0..n {
        j = (j + by) % n;
        if !hidden(j) {
            return j;
        }
    }
    i
}

/// How a preset is saved: "Group: Name".
pub(crate) fn save_name(i: usize) -> String {
    if let Some(e) = retro_of(i).map(|k| &crate::retro::list()[k]) {
        return format!("{}: {}: {}", RETRO, e.group, e.name);
    }
    let p = &PRESETS[i.min(PRESETS.len() - 1)];
    format!("{}: {}", p.group, p.name)
}

/// What the music is doing this frame, for the shaders.
#[derive(Clone, Copy)]
pub(crate) struct Frame {
    pub t: f32,
    pub bass: f32,
    pub mid: f32,
    pub high: f32,
    pub wave: [f32; 64],
    pub spec: [f32; 32],
    pub peak: [f32; 32],
}

/// Automatic gain and beats: slow averages of the bass and the loudest swing, so the visuals react as strongly to a
/// quiet jazz trio as to a loud record (as WMP's did), and a kick on each beat (bass well over its recent level).
#[derive(Default, Clone, Copy)]
pub(crate) struct Agc {
    bass_avg: f32,
    wave_peak: f32,
    spec_peak: f32,
    beat: f32,
}

impl Agc {
    pub(crate) fn apply(&mut self, f: &mut Frame, dt: f32) {
        let k_slow = 1.0 - (-dt / 1.5).exp();
        self.bass_avg += (f.bass - self.bass_avg) * k_slow;
        let wp = f.wave.iter().fold(0f32, |m, v| m.max(v.abs()));
        let sp = f.spec.iter().fold(0f32, |m, v| m.max(*v));
        // peaks: up at once, down over a few seconds
        self.wave_peak = if wp > self.wave_peak { wp } else { self.wave_peak + (wp - self.wave_peak) * (1.0 - (-dt / 4.0).exp()) };
        self.spec_peak = if sp > self.spec_peak { sp } else { self.spec_peak + (sp - self.spec_peak) * (1.0 - (-dt / 4.0).exp()) };
        let wg = (0.8 / self.wave_peak.max(0.08)).min(6.0);
        let sg = (0.9 / self.spec_peak.max(0.12)).min(4.0);
        for v in f.wave.iter_mut() {
            *v = (*v * wg).clamp(-1.0, 1.0);
        }
        for v in f.spec.iter_mut().chain(f.peak.iter_mut()) {
            *v = (*v * sg).min(1.0);
        }
        // a beat: the bass jumps well over its recent average; it rings out over a fifth of a second
        let rel = f.bass / self.bass_avg.max(0.02);
        if rel > 1.35 && f.bass > 0.03 {
            self.beat = self.beat.max(((rel - 1.0) * 0.6).min(1.0));
        }
        self.beat *= (-dt / 0.2).exp();
        let level = (f.bass / self.bass_avg.max(0.02) * 0.35).min(1.0);
        f.bass = (0.35 * level + 0.9 * self.beat).min(1.4);
        f.mid = (f.mid * sg).min(1.0);
        f.high = (f.high * sg).min(1.2);
    }
}

impl Frame {
    /// From the player's analysis: bands and peaks (low to high, 0..1) and the waveform (-1..1).
    pub(crate) fn from_audio(t: f32, bands: &[f32], peaks: &[f32], wave: &[f32]) -> Self {
        let pick = |src: &[f32], n: usize| -> Vec<f32> {
            (0..n).map(|i| if src.is_empty() { 0.0 } else { src[(i * src.len() / n).min(src.len() - 1)] }).collect()
        };
        let avg = |a: f32, b: f32| -> f32 {
            let n = bands.len();
            if n == 0 {
                return 0.0;
            }
            let (i, j) = (((a * n as f32) as usize).min(n - 1), ((b * n as f32) as usize).within(1, n));
            bands[i..j.max(i + 1)].iter().sum::<f32>() / (j.max(i + 1) - i) as f32
        };
        let mut f = Frame {
            t,
            bass: avg(0.0, 0.15).powf(2.0) * 1.5,
            mid: avg(0.15, 0.55),
            high: avg(0.7, 1.0) * 1.3,
            wave: [0.0; 64],
            spec: [0.0; 32],
            peak: [0.0; 32],
        };
        f.wave.copy_from_slice(&pick(wave, 64));
        f.spec.copy_from_slice(&pick(bands, 32));
        f.peak.copy_from_slice(&pick(peaks, 32));
        f
    }
}

#[derive(Default)]
pub(crate) struct Gl {
    terrain: Option<Box<crate::terrain::Terrain>>,
    wave: Option<Box<crate::wave::Wave>>,
    /// the graphics are too old for the Terrain: Starfield instead
    failed: bool,
}

/// The Terrain into `rect` (slot: 0 full screen, 1 Now Playing, 4 the visuals' own window).
fn paint_terrain(p: &egui::Painter, rect: Rect, slot: u8, state: &Arc<Mutex<Gl>>, inp: Arc<crate::terrain::Input>) {
    let state = state.clone();
    let cb = egui_glow::CallbackFn::new(move |info, painter| {
        if let Ok(mut g) = state.lock() {
            let ok = unsafe { g.terrain.get_or_insert_with(Default::default).paint(painter.gl(), &info, slot, &inp) };
            if !ok {
                g.failed = true;
            }
        }
    });
    p.add(egui::PaintCallback { rect, callback: Arc::new(cb) });
}

/// The Wave into `rect` (as `paint_terrain`).
fn paint_wave(p: &egui::Painter, rect: Rect, slot: u8, state: &Arc<Mutex<Gl>>, t: f64) {
    let state = state.clone();
    let cb = egui_glow::CallbackFn::new(move |info, painter| {
        if let Ok(mut g) = state.lock() {
            let ok = unsafe { g.wave.get_or_insert_with(Default::default).paint(painter.gl(), &info, slot, t) };
            if !ok {
                g.failed = true;
            }
        }
    });
    p.add(egui::PaintCallback { rect, callback: Arc::new(cb) });
}

/// `preset` in `r`; `hear` says what the terrain hears, asked only by it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_preset(
    p: &egui::Painter,
    r: Rect,
    slot: u8,
    preset: usize,
    f: Frame,
    dt: f32,
    gl: &Arc<Mutex<Gl>>,
    pipes: &mut Pipes,
    stars: &mut Stars,
    hear: impl FnOnce() -> crate::terrain::Input,
) {
    if let Some(k) = retro_of(preset) {
        crate::retro::paint(p, r, k);
        return;
    }
    let preset = preset.min(PRESETS.len() - 1);
    let old_gl = gl.lock().is_ok_and(|g| g.failed);
    match PRESETS[preset].kind {
        Kind::Terrain if !old_gl => paint_terrain(p, r, slot, gl, Arc::new(hear())),
        Kind::Wave if !old_gl => paint_wave(p, r, slot, gl, f.t as f64),
        Kind::Pipes => pipes.draw(p, r, dt, f.bass),
        Kind::Stars | Kind::Terrain | Kind::Wave => stars.draw(p, r, dt, f.bass),
    }
}

/// What the terrain hears and sees at time `t`: the player's latest samples, the song as it plays (for reading it
/// ahead of what is heard), and `song`, a number that changes with it.
pub(crate) fn hear(viz: &Mutex<crate::player::Viz>, ctl: &crate::player::Ctl, song: u64, t: f64) -> crate::terrain::Input {
    let (samples, first, rate, file) = match viz.lock() {
        Ok(v) => (v.samples.clone(), v.first.clone(), v.rate as f32, v.song.clone()),
        Err(_) => (Vec::new(), Vec::new(), 44100.0, None),
    };
    let music = file.map(|file| crate::terrain::Music { file, at: ctl.pos() as f64, speed: ctl.speed(), ab: ctl.ab() });
    // (test switch BACKLINE_3D_EVERY=<seconds>: a new group of sets that often, as if the song changed)
    let song = match std::env::var("BACKLINE_3D_EVERY").ok().and_then(|v| v.parse::<f64>().ok()) {
        Some(s) if s > 0.0 => song.wrapping_add((t / s) as u64),
        _ => song,
    };
    crate::terrain::Input { t, samples, first, rate, song, music }
}

/// A preset as "Group: Name" (a classic one as "Family: Name").
pub(crate) fn name_of(i: usize, _t: f32) -> String {
    if let Some(e) = retro_of(i).map(|k| &crate::retro::list()[k]) {
        return format!("{}: {}", e.group, e.name);
    }
    let p = &PRESETS[i.min(PRESETS.len() - 1)];
    format!("{}: {}", p.group, p.name)
}

// ------------------------------------------------------------------------------------------------ the screensavers
/// 3D Pipes: pipes growing through a box of cells, a ball at every bend, clearing and starting over when full.
pub(crate) struct Pipes {
    used: std::collections::HashSet<(i32, i32, i32)>,
    segs: Vec<([i32; 3], [i32; 3], usize, bool)>,
    heads: Vec<([i32; 3], usize, usize, bool)>,
    acc: f32,
    fade: f32,
    seed: u64,
}

const NX: i32 = 12;
const NY: i32 = 8;
const NZ: i32 = 8;
const PIPE_COLS: [[u8; 3]; 7] =
    [[216, 52, 44], [47, 191, 74], [47, 111, 224], [232, 198, 42], [42, 200, 208], [200, 74, 216], [232, 232, 232]];
const DIRS: [[i32; 3]; 6] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];

impl Default for Pipes {
    fn default() -> Self {
        let mut p = Pipes {
            used: Default::default(),
            segs: Vec::new(),
            heads: Vec::new(),
            acc: 0.0,
            fade: 0.0,
            seed: 0x9e3779b97f4a7c15,
        };
        p.reset();
        p
    }
}

impl Pipes {
    fn rnd(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 40) as f32 / 16777216.0
    }
    fn reset(&mut self) {
        self.used.clear();
        self.segs.clear();
        self.heads.clear();
        for _ in 0..3 {
            self.new_pipe();
        }
    }
    fn free(&self, c: [i32; 3]) -> bool {
        c[0] >= 0 && c[1] >= 0 && c[2] >= 0 && c[0] < NX && c[1] < NY && c[2] < NZ && !self.used.contains(&(c[0], c[1], c[2]))
    }
    fn new_pipe(&mut self) {
        for _ in 0..40 {
            let c = [(self.rnd() * NX as f32) as i32, (self.rnd() * NY as f32) as i32, (self.rnd() * NZ as f32) as i32];
            if self.free(c) {
                self.used.insert((c[0], c[1], c[2]));
                // a colour not already on screen, when there is one left
                let taken: Vec<usize> = self.heads.iter().map(|h| h.2).collect();
                let mut col = (self.rnd() * PIPE_COLS.len() as f32) as usize % PIPE_COLS.len();
                for _ in 0..PIPE_COLS.len() {
                    if !taken.contains(&col) {
                        break;
                    }
                    col = (col + 1) % PIPE_COLS.len();
                }
                let d = (self.rnd() * 6.0) as usize % 6;
                self.heads.push((c, d, col, true));
                self.segs.push((c, c, col, true));
                return;
            }
        }
    }
    fn grow(&mut self) {
        for k in 0..self.heads.len() {
            let (c, d, col, alive) = self.heads[k];
            if !alive {
                continue;
            }
            let step = |d: usize| [c[0] + DIRS[d][0], c[1] + DIRS[d][1], c[2] + DIRS[d][2]];
            let mut nd = d;
            if self.rnd() < 0.25 || !self.free(step(d)) {
                let opts: Vec<usize> = (0..6).filter(|&o| self.free(step(o))).collect();
                if opts.is_empty() {
                    self.heads[k].3 = false;
                    continue;
                }
                nd = opts[(self.rnd() * opts.len() as f32) as usize % opts.len()];
            }
            let n = step(nd);
            self.used.insert((n[0], n[1], n[2]));
            self.segs.push((c, n, col, nd != d));
            self.heads[k] = (n, nd, col, true);
        }
        if self.heads.iter().filter(|h| h.3).count() < 3 && self.heads.len() < 8 {
            self.new_pipe();
        }
        if self.segs.len() > 260 || self.heads.iter().all(|h| !h.3) {
            self.fade = 1.0;
        }
    }

    pub(crate) fn draw(&mut self, p: &egui::Painter, r: Rect, dt: f32, bass: f32) {
        p.rect_filled(r, 0.0, Color32::BLACK);
        self.acc += dt * (1.0 + bass * 2.5);
        while self.acc > 0.1 {
            self.acc -= 0.1;
            if self.fade <= 0.0 {
                self.grow();
            }
        }
        let mut alpha = 1.0;
        if self.fade > 0.0 {
            self.fade -= dt * 0.8;
            alpha = self.fade.max(0.0);
            if self.fade <= 0.0 {
                self.fade = 0.0;
                self.reset();
            }
        }
        // a fixed camera a little above and to the side, as the screensaver had
        let (ay, ax) = (0.5f32, 0.35f32);
        let scale = r.height().min(r.width() * 0.62) * 1.35;
        let proj = |c: [i32; 3]| -> (Pos2, f32, f32) {
            let (mut x, mut y, mut z) =
                (c[0] as f32 - (NX - 1) as f32 / 2.0, c[1] as f32 - (NY - 1) as f32 / 2.0, c[2] as f32 - (NZ - 1) as f32 / 2.0);
            (x, z) = (x * ay.cos() + z * ay.sin(), -x * ay.sin() + z * ay.cos());
            (y, z) = (y * ax.cos() - z * ax.sin(), y * ax.sin() + z * ax.cos());
            let f = scale / (z + 16.0);
            (Pos2::new(r.center().x + x * f, r.center().y - y * f), f, z)
        };
        // tubes and balls, far first so near ones cover them; a ball sits a little in front of the tubes it joins,
        // so their cut ends hide inside it, and every pipe's tip is rounded off with a ball of its own width
        enum It {
            Tube(Pos2, f32, Pos2, f32, usize),
            Ball(Pos2, f32, usize),
        }
        let mut items: Vec<(f32, It)> = Vec::new();
        // the nearest tube touching each cell: a ball there is drawn just in front of it, never under a tube's cut end
        let mut near: std::collections::HashMap<[i32; 3], f32> = Default::default();
        for &(a, b, _, _) in &self.segs {
            if a != b {
                let z = (proj(a).2 + proj(b).2) / 2.0;
                for c in [a, b] {
                    let e = near.entry(c).or_insert(f32::MAX);
                    *e = e.min(z);
                }
            }
        }
        let ball_z = |c: [i32; 3], z: f32| near.get(&c).copied().unwrap_or(z).min(z) - 0.05;
        for &(a, b, col, joint) in &self.segs {
            let ((pa, fa, za), (pb, fb, zb)) = (proj(a), proj(b));
            if a != b {
                items.push(((za + zb) / 2.0, It::Tube(pa, 0.3 * fa, pb, 0.3 * fb, col)));
            }
            if joint {
                items.push((ball_z(a, za), It::Ball(pa, 0.3 * fa * 1.3, col)));
            }
        }
        for &(c, _, col, _) in &self.heads {
            let (pc, fc, zc) = proj(c);
            items.push((ball_z(c, zc), It::Ball(pc, 0.3 * fc * 1.06, col)));
        }
        items.sort_by(|a, b| b.0.total_cmp(&a.0));
        let shade = |c: [u8; 3], k: f32| {
            let m = |v: u8| ((v as f32 * k).min(255.0) * alpha) as u8;
            Color32::from_rgb(m(c[0]), m(c[1]), m(c[2]))
        };
        // light from the top left: across a tube (lit side first), a soft rim, a bright streak, the colour, a dark rim
        const ACROSS: [(f32, f32); 7] = [(-1.0, 0.45), (-0.75, 0.95), (-0.4, 1.5), (-0.2, 1.75), (0.15, 1.0), (0.6, 0.6), (1.0, 0.28)];
        let light = Vec2::new(-0.7, -0.7);
        let mut mesh = egui::Mesh::default();
        for (_, it) in items {
            match it {
                It::Tube(a, ra, b, rb, col) => {
                    let c = PIPE_COLS[col];
                    let d = b - a;
                    if d.length() < 0.5 {
                        continue;
                    }
                    let mut n = Vec2::new(-d.y, d.x).normalized();
                    if n.x + n.y > 0.0 {
                        n = -n;
                    }
                    let base = mesh.vertices.len() as u32;
                    for (t, k) in ACROSS {
                        mesh.colored_vertex(a - n * t * ra, shade(c, k));
                        mesh.colored_vertex(b - n * t * rb, shade(c, k));
                    }
                    for i in 0..ACROSS.len() as u32 - 1 {
                        let (p0, p1, q0, q1) = (base + 2 * i, base + 2 * i + 1, base + 2 * i + 2, base + 2 * i + 3);
                        mesh.add_triangle(p0, p1, q1);
                        mesh.add_triangle(p0, q1, q0);
                    }
                }
                It::Ball(at, rr, col) => {
                    // bright off-centre towards the light, dark at its rim
                    let c = PIPE_COLS[col];
                    let ctr = mesh.vertices.len() as u32;
                    mesh.colored_vertex(at + light * rr * 0.35, shade(c, 1.9));
                    const SEG: u32 = 32;
                    for ring in [(0.55f32, 1.15f32), (1.0, 0.3)] {
                        for k in 0..SEG {
                            let ang = k as f32 / SEG as f32 * std::f32::consts::TAU;
                            let dir = Vec2::new(ang.cos(), ang.sin());
                            let towards = (dir.dot(light.normalized()) * 0.5 + 0.5) * 0.5;
                            mesh.colored_vertex(at + dir * rr * ring.0 + light * rr * 0.35 * (1.0 - ring.0), shade(c, ring.1 + towards));
                        }
                    }
                    for k in 0..SEG {
                        let (i0, i1) = (ctr + 1 + k, ctr + 1 + (k + 1) % SEG);
                        mesh.add_triangle(ctr, i0, i1);
                        let (o0, o1) = (i0 + SEG, i1 + SEG);
                        mesh.add_triangle(i0, o0, o1);
                        mesh.add_triangle(i0, o1, i1);
                    }
                }
            }
        }
        p.with_clip_rect(r).add(egui::Shape::mesh(mesh));
    }
}

/// The Windows 3.1 starfield: white stars flying out of the middle, faster when the music hits.
pub(crate) struct Stars {
    s: Vec<[f32; 3]>,
    seed: u64,
}

impl Default for Stars {
    fn default() -> Self {
        let mut st = Stars { s: Vec::new(), seed: 0x2545f4914f6cdd1d };
        for _ in 0..320 {
            let v = [st.rnd() * 2.0 - 1.0, st.rnd() * 2.0 - 1.0, st.rnd()];
            st.s.push(v);
        }
        st
    }
}

impl Stars {
    fn rnd(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 40) as f32 / 16777216.0
    }
    pub(crate) fn draw(&mut self, p: &egui::Painter, r: Rect, dt: f32, bass: f32) {
        p.rect_filled(r, 0.0, Color32::BLACK);
        let sp = 0.25 + bass * 0.6;
        let k = r.width() * 0.25;
        for i in 0..self.s.len() {
            self.s[i][2] -= dt * sp;
            let [x, y, z] = self.s[i];
            let at = Pos2::new(r.center().x + x / z.max(0.01) * k, r.center().y + y / z.max(0.01) * k);
            if z <= 0.02 || !r.contains(at) {
                self.s[i] = [self.rnd() * 2.0 - 1.0, self.rnd() * 2.0 - 1.0, 1.0];
                continue;
            }
            let size = ((1.0 - z) * r.width() / 260.0).max(0.8);
            let a = ((1.0 - z) * 1.4).min(1.0);
            p.rect_filled(Rect::from_center_size(at, Vec2::splat(size)), 0.0, Color32::from_white_alpha((a * 255.0) as u8));
        }
    }
}

// ------------------------------------------------------------------------------------------------ in the app
use crate::{Action, App};

impl App {
    /// This frame's music for the visuals (made once a frame, however many spots show them), and the time since the
    /// last frame.
    fn vis_frame(&mut self, ctx: &egui::Context) -> (Frame, f32) {
        let nr = ctx.cumulative_pass_nr();
        if let Some((n, f, dt)) = self.vis_cache {
            if n == nr {
                return (f, dt);
            }
        }
        let now = ctx.input(|i| i.time);
        let dt = ((now - self.vis_last) as f32).clamp(0.0, 0.1);
        self.vis_last = now;
        let mut f = Frame::from_audio(now as f32, &self.bands, &self.peaks, &self.viz_wave);
        self.vis_agc.apply(&mut f, dt);
        self.vis_cache = Some((nr, f, dt));
        (f, dt)
    }

    /// The chosen visuals in `r` (slot: 0 full screen, 1 Now Playing, 2 the deck, 3 the playlist bar's).
    pub(crate) fn vis_paint(&mut self, ui: &egui::Ui, r: Rect, slot: u8) {
        // (no one can see the window: nothing is worked out or drawn, and nothing asks to be drawn again)
        if !crate::onscreen::seen() {
            return;
        }
        let (f, dt) = self.vis_frame(ui.ctx());
        let preset = offered(self.vis_preset);
        self.retro_feed(preset);
        let p = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
        let t = ui.ctx().input(|i| i.time);
        let song = self.cur_track().map_or(0, |t| t.id as u64);
        let (viz, ctl) = (&self.player.viz, &self.player.ctl);
        let (pipes, stars) = (&mut self.vis_pipes, &mut self.vis_stars);
        paint_preset(&p, r, slot, preset, f, dt, &self.vis_gl, pipes, stars, || hear(viz, ctl, song, t));
        ui.ctx().request_repaint();
    }

    /// What the visuals are set to, as "Group: Name" (a random one also says what it is showing now).
    pub(crate) fn vis_name(&self) -> String {
        name_of(self.vis_preset, self.vis_cache.map_or(0.0, |c| c.1.t))
    }

    pub(crate) fn vis_menu(&mut self, ui: &mut egui::Ui) {
        // what is picked, first; then the groups (the one holding it marked)
        crate::views::dim_line(ui, "NOW SHOWING", 1.0, crate::pal().ink2);
        crate::views::dim_line(ui, &self.vis_name(), 1.6, crate::pal().ink);
        ui.separator();
        for g in GROUPS {
            let only: Vec<usize> = (0..PRESETS.len()).filter(|&i| PRESETS[i].group == *g && !hidden(i)).collect();
            if only.len() == 1 {
                let i = only[0];
                let mark = if i == self.vis_preset { "* " } else { "  " };
                if crate::menu_item(ui, &format!("{}{}", mark, PRESETS[i].name)) {
                    self.vis_preset = i;
                    self.dirty = true;
                    ui.close_menu();
                }
                continue;
            }
            let title = *g;
            let mark = if self.vis_preset < PRESETS.len() && PRESETS[self.vis_preset].group == *g { "* " } else { "  " };
            crate::sub_menu(ui, &format!("{}{}", mark, title), |ui| {
                for (i, p) in PRESETS.iter().enumerate().filter(|(i, p)| p.group == *g && !hidden(*i)) {
                    let mark = if i == self.vis_preset { "* " } else { "  " };
                    if crate::menu_item(ui, &format!("{}{}", mark, p.name)) {
                        self.vis_preset = i;
                        self.dirty = true;
                        ui.close_menu();
                    }
                }
            });
        }
        self.retro_menu(ui);
        ui.separator();
        if crate::menu_item(ui, "  Full screen  (G)") {
            self.vis_set_full(!self.vis_full, false);
            ui.close_menu();
        }
        let mark = if self.pop.is_some() { "* " } else { "  " };
        if crate::popout::CAN && crate::menu_item(ui, &format!("{}Pop Out", mark)) {
            self.pop_toggle();
            ui.close_menu();
        }
        crate::views::dim_line(ui, "SCREENSAVER (NOT WHILE PRACTICING)", 1.0, crate::pal().ink2);
        let idle = [
            (0u32, "Off"),
            (2, "After 2 minutes"),
            (5, "After 5 minutes"),
            (10, "After 10 minutes"),
        ];
        for (m, label) in idle {
            let mark = if self.vis_idle == m { "* " } else { "  " };
            if crate::menu_item(ui, &format!("{}{}", mark, label)) {
                self.vis_idle = m;
                self.dirty = true;
                ui.close_menu();
            }
        }
    }

    /// A classic preset about to be shown: its thread started, and the song's cover handed over (for Picture Viz).
    pub(crate) fn retro_feed(&mut self, preset: usize) {
        if retro_of(preset).is_none() {
            return;
        }
        crate::retro::start(&self.player.viz);
        let url = self.cur_track().map(|t| crate::api::cover_url(&t.cover, 640)).unwrap_or_default();
        crate::retro::set_cover(&url);
    }

    /// The classic visualizers in the menu, by family, and how many frames a second they draw.
    fn retro_menu(&mut self, ui: &mut egui::Ui) {
        if crate::retro::list().is_empty() {
            return;
        }
        let on = retro_of(self.vis_preset);
        let mark = if on.is_some() { "* " } else { "  " };
        // one list that scrolls: a click on a family folds its presets open under it (submenus popping out as the
        // pointer passes got in the way of scrolling); the family showing opens to begin with
        crate::sub_menu(ui, &format!("{}{}", mark, RETRO), |ui| {
            crate::views::dim_line(ui, "Ported by Ryan Circelli", 1.0, crate::pal().ink2);
            let h = (ui.ctx().screen_rect().height() * 0.7).max(200.0);
            let cur_fam = on.map(|k| crate::retro::list()[k].group);
            let open = self.retro_fam.or(cur_fam);
            egui::ScrollArea::vertical().id_salt("retro_list").max_height(h).show(ui, |ui| {
                for fam in crate::retro::families() {
                    let is_open = open == Some(fam);
                    if crate::menu_fold(ui, &format!("{}{}", if cur_fam == Some(fam) { "* " } else { "  " }, fam), is_open) {
                        self.retro_fam = Some(if is_open { "" } else { fam });
                    }
                    if !is_open {
                        continue;
                    }
                    for (k, e) in crate::retro::list().iter().enumerate().filter(|(_, e)| e.group == fam) {
                        let mark = if on == Some(k) { "* " } else { "  " };
                        if crate::menu_item(ui, &format!("      {}{}", mark, e.name)) {
                            self.vis_preset = PRESETS.len() + k;
                            self.retro_fam = None;
                            self.dirty = true;
                            ui.close_menu();
                        }
                    }
                }
            });
        });
        crate::sub_menu(ui, "  Refresh Rate", |ui| {
            for (fps, label) in crate::retro::RATES {
                let mark = if self.vis_fps == fps { "* " } else { "  " };
                if crate::menu_item(ui, &format!("{}{}", mark, label)) {
                    self.vis_fps = fps;
                    crate::retro::FPS.store(fps, std::sync::atomic::Ordering::Relaxed);
                    self.dirty = true;
                    ui.close_menu();
                }
            }
        });
    }

    /// Full screen visuals on or off (`saver`: started by being idle, so any touch ends it).
    pub(crate) fn vis_set_full(&mut self, on: bool, saver: bool) {
        if on == self.vis_full {
            return;
        }
        self.vis_full = on;
        self.vis_saver = on && saver;
        // (a hidden test copy fills its own window instead: full screen would bring it out in front of you)
        let hidden = crate::hidden_test_copy();
        if on {
            self.vis_was_fs = self.ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            if !hidden {
                self.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            }
            self.vis_shown_at = self.ctx.input(|i| i.time);
        } else if !self.vis_was_fs && !hidden {
            self.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        self.vis_idle_since = std::time::Instant::now();
    }

    /// Going idle while music plays starts the screensaver; any key, click or mouse move counts as not idle.
    pub(crate) fn vis_idle_check(&mut self, ctx: &egui::Context) {
        // only while you are in Backline: in another window you aren't idle, and going to one ends it (it would
        // otherwise start out of sight and run on behind what you are doing)
        if !ctx.input(|i| i.focused) || !crate::onscreen::seen() {
            self.vis_idle_since = std::time::Instant::now();
            if self.vis_full && self.vis_saver {
                self.vis_set_full(false, false);
            }
            return;
        }
        let busy = ctx.input(|i| {
            i.pointer.delta() != Vec2::ZERO || i.pointer.any_down() || !i.keys_down.is_empty() || i.raw_scroll_delta != Vec2::ZERO
        });
        if busy {
            self.vis_idle_since = std::time::Instant::now();
            // the screensaver ends at a touch (not in its first moment, so the move that started nothing doesn't)
            if self.vis_full && self.vis_saver && ctx.input(|i| i.time) - self.vis_shown_at > 0.5 {
                self.vis_set_full(false, false);
            }
            return;
        }
        // (never while practising: playing along with a loop for minutes isn't stepping away; nor while the visuals
        // are in their own window, already showing)
        let playing =
            self.cur.is_some() && !self.paused && !self.stopped && !(crate::practice_tools() && self.practice) && self.pop.is_none();
        if self.vis_idle > 0 && playing && !self.vis_full && self.vis_idle_since.elapsed().as_secs() >= self.vis_idle as u64 * 60
        {
            self.vis_set_full(true, true);
        }
        if self.vis_idle > 0 && playing && !self.vis_full {
            ctx.request_repaint_after(std::time::Duration::from_secs(5));
        }
    }

    /// Full screen: the visuals, and when the mouse moves, the song and the preset with arrows to change it.
    pub(crate) fn vis_full_ui(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let r = ui.max_rect();
        let resp = ui.allocate_rect(r, egui::Sense::click());
        self.vis_paint(ui, r, 0);
        let now = ui.input(|i| i.time);
        if ui.input(|i| i.pointer.delta() != Vec2::ZERO) {
            self.vis_ui_at = now;
        }
        let n = count();
        // (the arrow keys move through the song here too, as everywhere: the visuals change with < and > on screen)
        let esc = !self.panel_open() && ui.input(|i| i.key_pressed(egui::Key::Escape));
        if esc || resp.double_clicked() {
            self.vis_set_full(false, false);
            return;
        }
        resp.context_menu(|ui| self.vis_menu(ui));
        if self.vis_saver || now - self.vis_ui_at > 3.0 {
            return;
        }
        // the song, top left
        let p = ui.painter();
        if let Some(t) = self.cur_track() {
            let album = if t.album.is_empty() { String::new() } else { format!("  ·  {}", t.album) };
            let line = format!("{}  -  {}{}", t.artists_text(), t.title, album);
            crate::ptext(p, r.min + Vec2::new(28.0, 30.0), egui::Align::Min, &line, 1.6, Color32::from_white_alpha(230));
        }
        // the preset and arrows, bottom middle
        let name = self.vis_name();
        let w = crate::text_w(&name, 1.2) + 30.0;
        let mid = Pos2::new(r.center().x, r.max.y - 44.0);
        let bar = Rect::from_center_size(mid, Vec2::new(w + 120.0, 36.0));
        p.rect_filled(bar, 18.0, Color32::from_black_alpha(170));
        crate::ptext(p, mid, egui::Align::Center, &name, 1.2, Color32::from_white_alpha(235));
        let lr = Rect::from_center_size(Pos2::new(bar.min.x + 24.0, mid.y), Vec2::splat(30.0));
        let rr = Rect::from_center_size(Pos2::new(bar.max.x - 24.0, mid.y), Vec2::splat(30.0));
        for (rect, txt, by) in [(lr, "<", n - 1), (rr, ">", 1)] {
            let h = ui.interact(rect, ui.id().with(("vis_arrow", txt)), egui::Sense::click());
            crate::ptext(
                p,
                rect.center(),
                egui::Align::Center,
                txt,
                1.6,
                if h.hovered() { Color32::WHITE } else { Color32::from_white_alpha(170) },
            );
            if h.clicked() {
                self.vis_preset = next_shown(self.vis_preset, by);
                self.dirty = true;
            }
        }
        crate::ptext(
            p,
            Pos2::new(r.center().x, r.max.y - 14.0),
            egui::Align::Center,
            "< > on screen: previous / next  ·  right-click for all  ·  Esc or double-click to close",
            0.9,
            Color32::from_white_alpha(140),
        );
        let _ = acts;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every preset offered is saved by a name that brings back that same preset (the classic ones too).
    #[test]
    fn saved_names_come_back() {
        for i in 0..count() {
            if !hidden(i) {
                assert_eq!(find(&save_name(i)), i, "{}", save_name(i));
            }
        }
        // the arrows go through all of them and round again
        let (mut i, mut seen) = (0, 0);
        loop {
            i = next_shown(i, 1);
            seen += 1;
            if i == 0 || seen > count() {
                break;
            }
        }
        assert_eq!(seen, count() - PRESETS.iter().filter(|p| p.kind == Kind::Wave).count());
    }
}
