//! 3D Wave: a cloth of nineteen strands, each a chain of nineteen points, blown along from a pinned edge, bent
//! through a soft cage that rolls slowly with time, and drawn as a sheet of light (bright where it turns edge-on, added
//! up where its folds overlap) over a sky gradient, then given a soft glare. The physics, the cage, the camera, the
//! sheet's shading and the glare follow a reference step by step (each checked against it); the curves it reads from
//! pictures (the fold glow, the sky) are measured. It doesn't react to the music.

use eframe::egui;
use eframe::egui_glow;
use eframe::glow::{self, HasContext};

use crate::terrain::{fbo, program, tex};

/// Points of the cloth each way, and samples of the surface each way (sixteen spans of eight).
const N: usize = 19;
const G: usize = 128;

/// The settings (the set shown while music plays, matching the sky and glare below).
struct Set {
    damping: f32,
    length: f32,
    tension: f32,
    timestep: f32,
    perturb: f32,
    end: [f32; 3],
    pos: [f32; 3],
    axis: [f32; 3],
    turn: f32,
    scale1: [f32; 3],
    scale2: [f32; 3],
    offset: [f32; 3],
    param1: f32,
    fresnel: f32,
    bright: f32,
    bias: f32,
}

const MUSIC: Set = Set {
    damping: 0.0001,
    length: 0.306001,
    tension: 0.25,
    timestep: 3.72102,
    perturb: 0.0998587,
    end: [3.0, 0.4, 0.2],
    pos: [-7.5, 0.0, -5.2],
    axis: [-0.994157, 0.796751, 0.190364],
    turn: 13.1208,
    scale1: [5.13725, 1.00077, 1.0],
    scale2: [3.2, 0.99579, 3.41782],
    offset: [0.0, -0.469999, 0.0],
    param1: -1.33509,
    fresnel: 0.638971,
    bright: 0.701917,
    bias: 1.86707,
};

/// The sky: 6 x 5 control colours of a smooth (cubic B-spline) field over the screen, the bottom row first,
/// measured from a frame while music played; the tint it is multiplied by from the bottom (black) to the top; and the
/// sheet's tint from the left to the right.
const SKY: [[f32; 3]; 30] = [
    [-0.332, -0.064, -0.180], [-0.435, -0.175, -0.225], [-0.350, -0.180, -0.138], [-0.172, -0.078, -0.017], [-0.053, 0.000, 0.032], [-0.047, -0.016, -0.018],
    [0.110, 0.030, 0.075], [0.130, 0.059, 0.075], [0.097, 0.055, 0.041], [0.044, 0.023, 0.004], [0.012, 0.001, -0.009], [0.012, 0.006, 0.006],
    [0.266, 0.185, 0.373], [0.289, 0.175, 0.379], [0.282, 0.159, 0.358], [0.228, 0.131, 0.292], [0.148, 0.092, 0.200], [0.079, 0.055, 0.115],
    [0.331, 0.287, 0.453], [0.343, 0.320, 0.507], [0.389, 0.364, 0.596], [0.468, 0.413, 0.709], [0.537, 0.447, 0.795], [0.548, 0.441, 0.800],
    [0.566, 0.271, 0.598], [0.698, 0.300, 0.673], [0.754, 0.329, 0.724], [0.712, 0.351, 0.718], [0.663, 0.386, 0.738], [0.702, 0.468, 0.861],
];
const SKY_TINT: [[f32; 3]; 2] = [[0.0, 0.0, 0.0], [0.579, 0.435, 0.472]];
const SHEET_TINT: [[f32; 3]; 2] = [[1.2, 1.0, 1.1], [0.5, 0.0, 0.5]];

/// The fold glow: how much a spot lights up by how squarely it faces (times how stretched the sheet is there), as
/// control points (texel of 512, value), joined smoothly.
const GLOW: [(f32, f32); 14] = [
    (14.0, 0.0), (17.0, 0.0314), (20.0, 0.2275), (23.0, 0.6275), (26.0, 0.9216), (29.0, 0.9882), (33.0, 0.9255),
    (38.0, 0.698), (44.0, 0.4471), (52.0, 0.2471), (62.0, 0.1137), (76.0, 0.0392), (95.0, 0.0078), (116.0, 0.0),
];

/// A repeatable number in -1..1 for each count.
fn noise(n: u32) -> f32 {
    let n = (n << 13) ^ n;
    let m = n.wrapping_mul(n.wrapping_mul(n).wrapping_mul(15731).wrapping_add(789221)).wrapping_add(1376312589) & 0x7fff_ffff;
    1.0 - m as f32 / 1073741824.0
}

/// The cloth: points by strand (row) then along it (column); the last column is pinned, the first pushed along.
struct Cloth {
    pos: Vec<[f32; 3]>,
    vel: Vec<[f32; 3]>,
    prev: Vec<[f32; 3]>,
    seed: u32,
    time: f32,
    phase: f32,
    /// steps owed (sixty a second), and whether the last step's clock went round
    acc: f32,
    wrapped: bool,
}

impl Cloth {
    /// A tube: the strands round a circle with their springs at rest, the columns a spring apart along it. After half a
    /// minute it moves as the reference does from its own start.
    fn new(s: &Set) -> Cloth {
        let rad = (N - 1) as f32 * s.length / std::f32::consts::TAU;
        let mut pos = Vec::with_capacity(N * N);
        for r in 0..N {
            let a = std::f32::consts::TAU * r as f32 / (N - 1) as f32;
            for c in 0..N {
                pos.push([(N - 1 - c) as f32 * s.length * 0.95, 0.2 + rad * a.sin(), 0.1 + rad * a.cos()]);
            }
        }
        let prev = pos.clone();
        Cloth { pos, vel: vec![[0.0; 3]; N * N], prev, seed: 0, time: 0.0, phase: 0.0, acc: 0.0, wrapped: false }
    }

    fn step(&mut self, s: &Set) {
        self.prev.copy_from_slice(&self.pos);
        let dt = s.timestep * 0.0001;
        self.time += dt;
        if self.time > 10.0 {
            self.time = 0.0;
            self.wrapped = true;
        }
        let (l, k) = (s.length, s.tension);
        let (pos, vel) = (&self.pos, &mut self.vel);
        for r in 0..N {
            for c in 0..N {
                let i = r * N + c;
                // springs to the points one and two back along the strand and across the strands (twice as long and ten
                // times as stiff for two)
                for (j, rest, kk, ok) in [(i.wrapping_sub(1), l, k, c >= 1), (i.wrapping_sub(2), 2.0 * l, 10.0 * k, c >= 2), (i.wrapping_sub(N), l, k, r >= 1), (i.wrapping_sub(2 * N), 2.0 * l, 10.0 * k, r >= 2)] {
                    if !ok {
                        continue;
                    }
                    let d = [pos[j][0] - pos[i][0], pos[j][1] - pos[i][1], pos[j][2] - pos[i][2]];
                    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                    if len <= 1e-9 {
                        continue;
                    }
                    let f = (len - rest) * kk / len;
                    for a in 0..3 {
                        vel[i][a] += d[a] * f;
                        vel[j][a] -= d[a] * f;
                    }
                }
                for a in 0..3 {
                    self.seed = self.seed.wrapping_add(1);
                    vel[i][a] += noise(self.seed) * s.perturb;
                }
            }
        }
        let damp = s.damping * s.timestep;
        for (p, v) in self.pos.iter_mut().zip(self.vel.iter_mut()) {
            for a in 0..3 {
                p[a] += v[a] * dt;
                v[a] -= v[a] * damp;
            }
        }
        self.phase += (self.time - self.phase) * 0.1;
        for r in 0..N {
            self.vel[r * N][0] += s.timestep * 0.02;
            let t = r as f32 / N as f32 + self.phase;
            let j = r * N + N - 1;
            self.pos[j] = [0.0, ((t * 11.0).sin() * 0.5 + 0.5) * s.end[1], ((t * 15.0).cos() + 1.0) * 0.5 * s.end[2]];
            self.vel[j] = [0.0; 3];
        }
    }

    /// The points for this frame: between the last two steps, the strands spread wider or narrower along the length.
    fn points(&self, s: &Set) -> Vec<[f32; 3]> {
        let a = self.acc;
        self.prev
            .iter()
            .zip(self.pos.iter())
            .map(|(p, q)| {
                let x = p[0] + (q[0] - p[0]) * a;
                let k = profile(x, s.param1);
                [x, (p[1] + (q[1] - p[1]) * a) * k, (p[2] + (q[2] - p[2]) * a) * k]
            })
            .collect()
    }
}

/// How wide the strands spread at `x` along the cloth: pinched near the pinned edge, swinging out, then steady.
fn profile(x: f32, param1: f32) -> f32 {
    let d = x - param1;
    let (cs, c) = if d < 0.0 { (1.0, 0.0) } else { ((d * 1.5708).cos(), (d / 5.0).clamp(0.0, 1.0)) };
    1.3 - cs * (1.0 - c) * (1.0 - c) * (1.0 + 2.0 * c)
}

/// The cage's shape at (u, w) at time `t`: across, and up.
fn cage_at(u: f32, w: f32, t: f32) -> (f32, f32) {
    let e = (2.0 * (u - 0.5)).exp();
    let th = (e - 1.0) / (e + 1.0);
    let amp = 0.24 * (th + 1.0) * ((2.0 * t).sin() + 3.0) * (-0.0001 * t).exp();
    let c = u - ((0.1 * t).sin() + 0.2) * 0.833333;
    let y = amp * (7.85 * u - 2.5 * t - 1.25).sin() + 0.5 * (0.25 * t).sin() + (-50.0 * c * c).exp() + (t - 6.28 * w).sin() / 8.0;
    (0.2 * u + 1.3 * u * u - 0.15, y)
}

/// The cage's 8 x 4 control points at time `t` (none depend on the middle axis): (x, y) each, placed by the settings.
fn cage(s: &Set, t: f32) -> [[(f32, f32); 4]; 8] {
    let mut out = [[(0.0, 0.0); 4]; 8];
    for (i, row) in out.iter_mut().enumerate() {
        for (k, p) in row.iter_mut().enumerate() {
            let (x, y) = cage_at(i as f32 * 0.125, k as f32 * 0.25, t);
            *p = (x * s.scale1[0] + s.offset[0], y * s.scale1[1] + s.offset[1]);
        }
    }
    out
}

/// Uniform cubic B-spline weights at `t`.
fn bsw(t: f32) -> [f32; 4] {
    let (t2, t3) = (t * t, t * t * t);
    [(1.0 - t) * (1.0 - t) * (1.0 - t) / 6.0, (3.0 * t3 - 6.0 * t2 + 4.0) / 6.0, (-3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0) / 6.0, t3 / 6.0]
}

/// A cloth point through the cage: where it sits in the cage's box, plus the cage's smooth field there, scaled out.
fn bend(s: &Set, cg: &[[(f32, f32); 4]; 8], p: [f32; 3]) -> [f32; 3] {
    let sp = [0, 1, 2].map(|a| (p[a] - s.offset[a]) / s.scale1[a]);
    let cells = [8.0, 4.0, 4.0];
    let mut cell = [0i32; 3];
    let mut w = [[0.0f32; 4]; 3];
    for a in 0..3 {
        let x = sp[a].clamp(0.0, 0.999) * cells[a];
        cell[a] = x.floor() as i32;
        w[a] = bsw(x - x.floor());
    }
    let (mut dx, mut dy) = (0.0, 0.0);
    for a in 0..4 {
        let i = (cell[0] + a as i32 - 1).clamp(0, 7) as usize;
        for c in 0..4 {
            let k = (cell[2] + c as i32 - 1).clamp(0, 3) as usize;
            let wt = w[0][a] * w[2][c];
            dx += wt * cg[i][k].0;
            dy += wt * cg[i][k].1;
        }
    }
    [(sp[0] + dx) * s.scale2[0], (sp[1] + dy) * s.scale2[1], (sp[2] + s.offset[2]) * s.scale2[2]]
}

/// Perspective (53 degrees up and down) times a step back of two, times the settings' place and turn; by columns.
fn mvp(s: &Set, aspect: f32) -> [f32; 16] {
    let n = (s.axis[0] * s.axis[0] + s.axis[1] * s.axis[1] + s.axis[2] * s.axis[2]).sqrt();
    let [x, y, z] = s.axis.map(|v| v / n);
    let (sn, cs) = s.turn.to_radians().sin_cos();
    let k = 1.0 - cs;
    let r = [
        [cs + x * x * k, x * y * k - z * sn, x * z * k + y * sn],
        [y * x * k + z * sn, cs + y * y * k, y * z * k - x * sn],
        [z * x * k - y * sn, z * y * k + x * sn, cs + z * z * k],
    ];
    let fy = 1.0 / (53.0f32.to_radians() * 0.5).tan();
    let fx = fy / aspect;
    let (near, far) = (0.1f32, 1000.0f32);
    let (za, zb) = (-(far + near) / (far - near), -2.0 * far * near / (far - near));
    // world = r p + pos; eye = world - (0, 0, 2)
    let t = [s.pos[0], s.pos[1], s.pos[2] - 2.0];
    let rows = [
        [fx * r[0][0], fx * r[0][1], fx * r[0][2], fx * t[0]],
        [fy * r[1][0], fy * r[1][1], fy * r[1][2], fy * t[1]],
        [za * r[2][0], za * r[2][1], za * r[2][2], za * t[2] + zb],
        [-r[2][0], -r[2][1], -r[2][2], -t[2]],
    ];
    let mut m = [0f32; 16];
    for (i, row) in rows.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            m[j * 4 + i] = *v;
        }
    }
    m
}

/// The fold glow as a 512-texel table (a smooth monotone curve through `GLOW`).
fn glow_table() -> Vec<f32> {
    let xs: Vec<f32> = GLOW.iter().map(|p| p.0).collect();
    let ys: Vec<f32> = GLOW.iter().map(|p| p.1).collect();
    let n = xs.len();
    let h: Vec<f32> = (0..n - 1).map(|i| xs[i + 1] - xs[i]).collect();
    let d: Vec<f32> = (0..n - 1).map(|i| (ys[i + 1] - ys[i]) / h[i]).collect();
    let mut m = vec![0.0f32; n];
    m[0] = d[0];
    for i in 1..n - 1 {
        if d[i - 1] * d[i] > 0.0 {
            let (w1, w2) = (2.0 * h[i] + h[i - 1], h[i] + 2.0 * h[i - 1]);
            m[i] = (w1 + w2) / (w1 / d[i - 1] + w2 / d[i]);
        }
    }
    (0..512)
        .map(|t| {
            let x = t as f32;
            if x <= xs[0] || x >= xs[n - 1] {
                return 0.0;
            }
            let i = (0..n - 1).find(|&i| x <= xs[i + 1]).unwrap_or(n - 2);
            let u = (x - xs[i]) / h[i];
            let (u2, u3) = (u * u, u * u * u);
            (2.0 * u3 - 3.0 * u2 + 1.0) * ys[i] + (u3 - 2.0 * u2 + u) * h[i] * m[i] + (-2.0 * u3 + 3.0 * u2) * ys[i + 1] + (u3 - u2) * h[i] * m[i + 1]
        })
        .collect()
}

// the sheet: 128 x 128 samples of the smooth surface through the 19 x 19 net (already on screen), its slope across
// and along giving the facing; each sample lights by how squarely it faces and how stretched it is there
const SHEET_VS: &str = r#"
uniform sampler2D net;
uniform float bias;
out float vf;
out float vlx;
out float vfade;
vec4 bw(float t) { float t2 = t * t, t3 = t2 * t; return vec4((1. - t) * (1. - t) * (1. - t), 3. * t3 - 6. * t2 + 4., -3. * t3 + 3. * t2 + 3. * t + 1., t3) / 6.; }
vec4 dw(float t) { return vec4(-(1. - t) * (1. - t), 3. * t * t - 4. * t, -3. * t * t + 2. * t + 1., t * t) * .5; }
float edge(int k) { return min(1., 10. * (.5 - abs(.5 - float(k) / 127.))); }
void main(){
  int r = gl_VertexID / 128, c = gl_VertexID - r * 128;
  int sr = min(r / 8, 15), sc = min(c / 8, 15);
  float tr = float(r - sr * 8) / 8., tc = float(c - sc * 8) / 8.;
  vec4 br = bw(tr), bc = bw(tc), dr = dw(tr), dc = dw(tc);
  vec4 P = vec4(0.), Pr = vec4(0.), Pc = vec4(0.);
  for (int a = 0; a < 4; a++) for (int b = 0; b < 4; b++) {
    vec4 q = texelFetch(net, ivec2(sc + b, sr + a), 0);
    P += br[a] * bc[b] * q; Pr += dr[a] * bc[b] * q; Pc += br[a] * dc[b] * q;
  }
  vec3 n = cross(Pr.xyz, Pc.xyz);
  float ip = inversesqrt(max(dot(P.xyz, P.xyz), 1e-12));
  float pn = abs(dot(P.xyz * ip, n));
  vf = pn * ip * bias;
  vlx = pn * inversesqrt(max(dot(n, n), 1e-24)) * vf;
  vfade = edge(r) * edge(c);
  gl_Position = P;
}
"#;

const SHEET_FS: &str = r#"
in float vf;
in float vlx;
in float vfade;
uniform sampler2D glow;
uniform vec2 fb;
void main(){
  // the soft term: high when the sheet is seen thin and far, gone as it comes square on
  float y = vf * 4. - .5;
  float s = y < 1. ? mix(.49804, .019608, clamp(y, 0., 1.)) : mix(.019608, 0., clamp(y - 1., 0., 1.));
  float g = texture(glow, vec2(vlx, .5)).r;
  o = vec4((fb.y * vf * s + g * fb.x * vf) * vfade, 0., 0., 1.);
}
"#;

// a quad over the whole target; `v` runs 0..1
const QUAD_VS: &str = r#"
out vec2 v;
void main(){ vec2 a = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2)); v = a; gl_Position = vec4(a * 2. - 1., 0., 1.); }
"#;

// the sky times its tint, plus the sheet's light times its own, through the response
const COMP_FS: &str = r#"
in vec2 v;
uniform sampler2D sheet;
uniform vec3 sky[30];
uniform vec3 st0, st1, ht0, ht1;
vec4 bw(float t) { float t2 = t * t, t3 = t2 * t; return vec4((1. - t) * (1. - t) * (1. - t), 3. * t3 - 6. * t2 + 4., -3. * t3 + 3. * t2 + 3. * t + 1., t3) / 6.; }
void main(){
  float x = clamp(v.x, 0., 1.) * 3., y = clamp(v.y, 0., 1.) * 2.;
  int i = min(int(x), 2), j = min(int(y), 1);
  vec4 wx = bw(x - float(i)), wy = bw(y - float(j));
  vec3 bg = vec3(0.);
  for (int b = 0; b < 4; b++) for (int a = 0; a < 4; a++) bg += wy[b] * wx[a] * sky[(j + b) * 6 + i + a];
  float l = texture(sheet, v).r;
  vec3 p = 8. * (bg * mix(st0, st1, v.y) + l * mix(ht0, ht1, v.x));
  // and how much of it glares: the brightest channel past a point, in a straight line
  float g = max(max(p.r, p.g), p.b);
  o = vec4(max(p * .18986 - .09297, 0.), max(g - 1.74, 0.) * .02327);
}
"#;

// the glare's source on its 128 x 64 grid: the picture times its glare amount times eight, averaged over each
// texel's footprint (`px` a picture pixel, `n` taps each way)
const SOURCE_FS: &str = r#"
in vec2 v;
uniform sampler2D src;
uniform vec2 px;
uniform int n;
void main(){
  vec3 s = vec3(0.);
  for (int j = 0; j < n; j++) for (int i = 0; i < n; i++) {
    vec4 c = texture(src, v + (vec2(float(i), float(j)) - float(n - 1) * .5) * px);
    s += c.rgb * c.a * 8.;
  }
  o = vec4(s / float(n * n), 1.);
}
"#;

// one level of the glare: thirteen taps along `dir`, a Gaussian of its own for each colour (red tightest)
const GAUSS_FS: &str = r#"
in vec2 v;
uniform sampler2D src;
uniform vec2 dir;
uniform vec3 wt[7];
void main(){
  vec3 s = texture(src, v).rgb * wt[0];
  for (int k = 1; k < 7; k++) s += (texture(src, v + dir * float(k)).rgb + texture(src, v - dir * float(k)).rgb) * wt[k];
  o = vec4(s, 1.);
}
"#;

// the six levels summed, the larger ones counting less
const SUM_FS: &str = r#"
in vec2 v;
uniform sampler2D l0, l1, l2, l3, l4, l5;
uniform float lw[6];
void main(){
  o = vec4(texture(l0, v).rgb * lw[0] + texture(l1, v).rgb * lw[1] + texture(l2, v).rgb * lw[2] + texture(l3, v).rgb * lw[3]
    + texture(l4, v).rgb * lw[4] + texture(l5, v).rgb * lw[5], 1.);
}
"#;

const OUT_FS: &str = r#"
in vec2 v;
uniform sampler2D pic, glare;
void main(){ o = vec4(texture(pic, v).rgb + texture(glare, v).rgb, 1.); }
"#;

/// The glare's Gaussian weights for taps 0..6 (red, green, blue) and the weights of its six levels, finest first.
const GAUSS: [[f32; 3]; 7] = [
    [0.200565, 0.18348, 0.161003], [0.176998, 0.165472, 0.148625], [0.121649, 0.121375, 0.116912], [0.065114, 0.072411, 0.078369],
    [0.027144, 0.035136, 0.044765], [0.008812, 0.013866, 0.021789], [0.0, 0.0, 0.009038],
];
const LEVELS: [f32; 6] = [1.891034, 1.395585, 1.029943, 0.760099, 0.560954, 0.413985];

/// One spot's pictures, sized to it: the sheet's light and the picture before the glare (with its glare amount);
/// then the glare's own, the same at any size: its source (128 x 64), each level blurred across then up and down
/// (128 x 64 down to 4 x 2), and the sum.
struct Targets {
    w: i32,
    h: i32,
    sheet: (glow::Framebuffer, glow::Texture),
    pic: (glow::Framebuffer, glow::Texture),
    src: (glow::Framebuffer, glow::Texture),
    across: [(glow::Framebuffer, glow::Texture); 6],
    down: [(glow::Framebuffer, glow::Texture); 6],
    sum: (glow::Framebuffer, glow::Texture),
}

impl Targets {
    unsafe fn new(gl: &glow::Context, w: i32, h: i32) -> Targets {
        let mk = |w: i32, h: i32| {
            let t = tex(gl, w, h, glow::RGBA16F, true);
            (fbo(gl, t, None), t)
        };
        Targets {
            w,
            h,
            sheet: mk(w, h),
            pic: mk(w, h),
            src: mk(128, 64),
            across: [0, 1, 2, 3, 4, 5].map(|l| mk(128 >> l, 64 >> l)),
            down: [0, 1, 2, 3, 4, 5].map(|l| mk(128 >> l, 64 >> l)),
            sum: mk(128, 64),
        }
    }

    unsafe fn free(&self, gl: &glow::Context) {
        for (f, t) in [self.sheet, self.pic, self.src, self.sum].into_iter().chain(self.across).chain(self.down) {
            gl.delete_framebuffer(f);
            gl.delete_texture(t);
        }
    }
}

#[derive(Default)]
struct Progs {
    sheet: Option<glow::Program>,
    comp: Option<glow::Program>,
    source: Option<glow::Program>,
    gauss: Option<glow::Program>,
    sum: Option<glow::Program>,
    out: Option<glow::Program>,
}

/// The wave's state: the cloth, the cage's clock (and the one it is fading from), and the GL objects.
pub(crate) struct Wave {
    cloth: Cloth,
    /// a cage still running from before the clock went round, and how much of it is left (1 to 0)
    old: Option<(f32, f32)>,
    t_last: f64,
    failed: bool,
    progs: Progs,
    vao: Option<glow::VertexArray>,
    ibuf: Option<glow::Buffer>,
    net: Option<glow::Texture>,
    glow_tex: Option<glow::Texture>,
    slots: std::collections::HashMap<u8, Targets>,
}

impl Default for Wave {
    fn default() -> Self {
        let mut cloth = Cloth::new(&MUSIC);
        // a minute's motion first, so it opens already moving as it always does
        for _ in 0..3600 {
            cloth.step(&MUSIC);
        }
        cloth.wrapped = false;
        Wave {
            cloth,
            old: None,
            t_last: -1.0,
            failed: false,
            progs: Progs::default(),
            vao: None,
            ibuf: None,
            net: None,
            glow_tex: None,
            slots: Default::default(),
        }
    }
}

impl Wave {
    /// The cloth on to `t` (sixty steps a second), and the cage's clock (ten times the cloth's; when the cloth's goes
    /// round, the old cage fades out over about three seconds).
    fn step(&mut self, t: f64) {
        let dt = if self.t_last < 0.0 { 0.0 } else { (t - self.t_last).clamp(0.0, 0.25) as f32 };
        self.t_last = t;
        let c = &mut self.cloth;
        let before = c.time;
        c.acc += dt * 60.0;
        while c.acc > 1.0 {
            c.step(&MUSIC);
            c.acc -= 1.0;
            if c.wrapped {
                c.wrapped = false;
                self.old = Some((before * 10.0, 1.0));
            }
        }
        let moved = if c.time >= before { (c.time - before) * 10.0 } else { 0.0 };
        if let Some((ot, f)) = self.old {
            let f = f - dt * 60.0 * 0.005;
            self.old = if f <= 0.0 { None } else { Some((ot + moved.max(0.0), f)) };
        }
    }

    /// The 19 x 19 net on screen: each cloth point through the cage and the camera. (The test switch
    /// BACKLINE_WAVE_NET=<file>: a net from a file instead, 361 points of four little-endian floats, for checks against
    /// a reference frame.)
    fn net(&self, aspect: f32) -> Vec<f32> {
        if let Some(b) = std::env::var("BACKLINE_WAVE_NET").ok().and_then(|f| std::fs::read(f).ok()) {
            if b.len() == N * N * 16 {
                return b.chunks(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
            }
        }
        let s = &MUSIC;
        let cg = cage(s, self.cloth.time * 10.0);
        let cg = match self.old {
            Some((ot, f)) => {
                let old = cage(s, ot);
                let k = f * f * (3.0 - 2.0 * f);
                let mut m = cg;
                for (row, orow) in m.iter_mut().zip(old.iter()) {
                    for (p, q) in row.iter_mut().zip(orow.iter()) {
                        *p = (p.0 + (q.0 - p.0) * k, p.1 + (q.1 - p.1) * k);
                    }
                }
                m
            }
            None => cg,
        };
        let m = mvp(s, aspect);
        let mut out = Vec::with_capacity(N * N * 4);
        for p in self.cloth.points(s) {
            let q = bend(s, &cg, p);
            for r in 0..4 {
                out.push(m[r] * q[0] + m[4 + r] * q[1] + m[8 + r] * q[2] + m[12 + r]);
            }
        }
        out
    }

    unsafe fn init(&mut self, gl: &glow::Context) -> bool {
        if self.failed {
            return false;
        }
        if self.vao.is_some() {
            return true;
        }
        let h = egui_glow::ShaderVersion::get(gl).version_declaration().to_string();
        self.progs = Progs {
            sheet: program(gl, &h, SHEET_VS, SHEET_FS),
            comp: program(gl, &h, QUAD_VS, COMP_FS),
            source: program(gl, &h, QUAD_VS, SOURCE_FS),
            gauss: program(gl, &h, QUAD_VS, GAUSS_FS),
            sum: program(gl, &h, QUAD_VS, SUM_FS),
            out: program(gl, &h, QUAD_VS, OUT_FS),
        };
        let p = &self.progs;
        if [p.sheet, p.comp, p.source, p.gauss, p.sum, p.out].iter().any(|x| x.is_none()) {
            self.failed = true;
            return false;
        }
        // two triangles per tile of the 128 x 128 samples
        let mut idx: Vec<u16> = Vec::with_capacity((G - 1) * (G - 1) * 6);
        for r in 0..G - 1 {
            for c in 0..G - 1 {
                let a = (r * G + c) as u16;
                let b = a + G as u16;
                idx.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
            }
        }
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));
        let ib = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ib));
        let bytes: Vec<u8> = idx.iter().flat_map(|v| v.to_le_bytes()).collect();
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
        gl.bind_vertex_array(None);
        self.vao = Some(vao);
        self.ibuf = Some(ib);
        self.net = Some(tex(gl, N as i32, N as i32, glow::RGBA32F, false));
        let gt = tex(gl, 512, 1, glow::RGBA32F, true);
        let bytes: Vec<u8> = glow_table().iter().flat_map(|v| [*v, 0.0, 0.0, 1.0]).flat_map(|v| v.to_le_bytes()).collect();
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA32F as i32, 512, 1, 0, glow::RGBA, glow::FLOAT, Some(&bytes));
        self.glow_tex = Some(gt);
        gl.bind_texture(glow::TEXTURE_2D, None);
        true
    }

    pub(crate) unsafe fn paint(&mut self, gl: &glow::Context, info: &egui::PaintCallbackInfo, slot: u8, t: f64) -> bool {
        if !self.init(gl) {
            return false;
        }
        if t != self.t_last {
            self.step(t);
        }
        let vp = info.viewport_in_pixels();
        let (w, h) = (vp.width_px.max(16), vp.height_px.max(16));
        if self.slots.get(&slot).map_or(true, |s| s.w != w || s.h != h) {
            if let Some(old) = self.slots.remove(&slot) {
                old.free(gl);
            }
            self.slots.insert(slot, Targets::new(gl, w, h));
        }
        let net = self.net(w as f32 / h as f32);
        gl.bind_texture(glow::TEXTURE_2D, self.net);
        let bytes: Vec<u8> = net.iter().flat_map(|v| v.to_le_bytes()).collect();
        gl.tex_sub_image_2d(glow::TEXTURE_2D, 0, 0, 0, N as i32, N as i32, glow::RGBA, glow::FLOAT, glow::PixelUnpackData::Slice(&bytes));
        let t = &self.slots[&slot];
        let p = &self.progs;
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::DEPTH_TEST);
        gl.disable(glow::CULL_FACE);
        gl.color_mask(true, true, true, true);
        gl.bind_vertex_array(self.vao);

        // 1. the sheet's light, added up where it overlaps
        let sp = p.sheet.unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.sheet.0));
        gl.viewport(0, 0, t.w, t.h);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
        gl.enable(glow::BLEND);
        gl.blend_equation(glow::FUNC_ADD);
        gl.blend_func(glow::ONE, glow::ONE);
        gl.use_program(Some(sp));
        let u = |n: &str| gl.get_uniform_location(sp, n);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, self.net);
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, self.glow_tex);
        gl.uniform_1_i32(u("net").as_ref(), 0);
        gl.uniform_1_i32(u("glow").as_ref(), 1);
        gl.uniform_1_f32(u("bias").as_ref(), MUSIC.bias);
        gl.uniform_2_f32(u("fb").as_ref(), MUSIC.fresnel, MUSIC.bright);
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, self.ibuf);
        gl.draw_elements(glow::TRIANGLES, ((G - 1) * (G - 1) * 6) as i32, glow::UNSIGNED_SHORT, 0);
        gl.disable(glow::BLEND);

        // 2. the sky and the sheet through the response
        let cp = p.comp.unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.pic.0));
        gl.use_program(Some(cp));
        let u = |n: &str| gl.get_uniform_location(cp, n);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(t.sheet.1));
        gl.uniform_1_i32(u("sheet").as_ref(), 0);
        let sky: Vec<f32> = SKY.iter().flatten().copied().collect();
        gl.uniform_3_f32_slice(u("sky").as_ref(), &sky);
        for (name, c) in [("st0", SKY_TINT[0]), ("st1", SKY_TINT[1]), ("ht0", SHEET_TINT[0]), ("ht1", SHEET_TINT[1])] {
            gl.uniform_3_f32(u(name).as_ref(), c[0], c[1], c[2]);
        }
        gl.draw_arrays(glow::TRIANGLES, 0, 3);

        // 3. the glare: its source on a 128 x 64 grid; six levels (the source read at each level's spacing), each blurred
        // across then up and down; summed
        let sp = p.source.unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.src.0));
        gl.viewport(0, 0, 128, 64);
        gl.use_program(Some(sp));
        gl.bind_texture(glow::TEXTURE_2D, Some(t.pic.1));
        gl.uniform_1_i32(gl.get_uniform_location(sp, "src").as_ref(), 0);
        let taps = ((t.w as f32 / 128.0).max(t.h as f32 / 64.0).ceil() as i32).clamp(1, 16);
        gl.uniform_2_f32(gl.get_uniform_location(sp, "px").as_ref(), 1.0 / (128 * taps) as f32, 1.0 / (64 * taps) as f32);
        gl.uniform_1_i32(gl.get_uniform_location(sp, "n").as_ref(), taps);
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        let gp = p.gauss.unwrap();
        gl.use_program(Some(gp));
        gl.uniform_1_i32(gl.get_uniform_location(gp, "src").as_ref(), 0);
        let wts: Vec<f32> = GAUSS.iter().flatten().copied().collect();
        gl.uniform_3_f32_slice(gl.get_uniform_location(gp, "wt").as_ref(), &wts);
        for l in 0..6 {
            let (lw, lh) = (128 >> l, 64 >> l);
            gl.viewport(0, 0, lw, lh);
            for (dst, src, dir) in [(t.across[l], t.src, (1.0 / lw as f32, 0.0)), (t.down[l], t.across[l], (0.0, 1.0 / lh as f32))] {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(dst.0));
                gl.bind_texture(glow::TEXTURE_2D, Some(src.1));
                gl.uniform_2_f32(gl.get_uniform_location(gp, "dir").as_ref(), dir.0, dir.1);
                gl.draw_arrays(glow::TRIANGLES, 0, 3);
            }
        }
        let sm = p.sum.unwrap();
        gl.use_program(Some(sm));
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.sum.0));
        gl.viewport(0, 0, 128, 64);
        for l in 0..6 {
            gl.active_texture(glow::TEXTURE0 + l as u32);
            gl.bind_texture(glow::TEXTURE_2D, Some(t.down[l].1));
            gl.uniform_1_i32(gl.get_uniform_location(sm, &format!("l{}", l)).as_ref(), l as i32);
        }
        gl.uniform_1_f32_slice(gl.get_uniform_location(sm, "lw").as_ref(), &LEVELS);
        gl.draw_arrays(glow::TRIANGLES, 0, 3);

        // 4. onto the screen inside egui's clip
        let op = p.out.unwrap();
        gl.use_program(Some(op));
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(t.pic.1));
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, Some(t.sum.1));
        gl.uniform_1_i32(gl.get_uniform_location(op, "pic").as_ref(), 0);
        gl.uniform_1_i32(gl.get_uniform_location(op, "glare").as_ref(), 1);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
        let clip = info.clip_rect_in_pixels();
        gl.enable(glow::SCISSOR_TEST);
        gl.scissor(clip.left_px, clip.from_bottom_px, clip.width_px, clip.height_px);
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        for i in 0..6 {
            gl.active_texture(glow::TEXTURE0 + i);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
        gl.active_texture(glow::TEXTURE0);
        gl.bind_vertex_array(None);
        gl.use_program(None);
        gl.enable(glow::BLEND);
        true
    }
}

