//! 3D Wave: a cloth of nineteen strands, each a chain of nineteen points, blown along from a pinned edge, bent
//! through a soft cage that rolls slowly with time, and drawn as a sheet of light (bright where it turns edge-on, added
//! up where its folds overlap) over a sky gradient, then given a soft glare; sparkles drift off the sheet where it moves,
//! tumbling and glinting. The physics, the cage, the camera, the sheet's shading, the glare and the sparkles follow a
//! reference step by step (each checked against it); what it reads from pictures (the fold glow, the sparkles' colour
//! swirl) is measured. It doesn't react to the music.

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

/// The sky (worked out by the shader it comes from, at the time of day of the reference frame: the cosine of the day
/// time), the tint it is multiplied by from the bottom (black) to the top, and the sheet's tint from the left to the right.
const DAY_COS: f32 = -0.9966;
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

    /// The points `a` of the way between the last two steps, the strands spread wider or narrower along the length.
    fn points(&self, s: &Set, a: f32) -> Vec<[f32; 3]> {
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

/// The settings' place and turn (by columns): world = turn p + pos.
fn model(s: &Set) -> [f32; 16] {
    let n = (s.axis[0] * s.axis[0] + s.axis[1] * s.axis[1] + s.axis[2] * s.axis[2]).sqrt();
    let [x, y, z] = s.axis.map(|v| v / n);
    let (sn, cs) = s.turn.to_radians().sin_cos();
    let k = 1.0 - cs;
    let r = [
        [cs + x * x * k, x * y * k - z * sn, x * z * k + y * sn],
        [y * x * k + z * sn, cs + y * y * k, y * z * k - x * sn],
        [z * x * k - y * sn, z * y * k + x * sn, cs + z * z * k],
    ];
    let mut m = [0f32; 16];
    for i in 0..3 {
        for j in 0..3 {
            m[j * 4 + i] = r[i][j];
        }
        m[12 + i] = s.pos[i];
    }
    m[15] = 1.0;
    m
}

/// The camera (by columns): perspective, 53 degrees up and down, 0.1 to 1000, from two units back.
fn camera(aspect: f32) -> [f32; 16] {
    let fy = 1.0 / (53.0f32.to_radians() * 0.5).tan();
    let (near, far) = (0.1f32, 1000.0f32);
    let mut m = [0f32; 16];
    m[0] = fy / aspect;
    m[5] = fy;
    m[10] = -(far + near) / (far - near);
    m[11] = -1.0;
    m[14] = -2.0 * far * near / (far - near) - 2.0 * m[10];
    m[15] = 2.0;
    m
}

/// `a` times `b` (both by columns).
fn mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut m = [0f32; 16];
    for c in 0..4 {
        for r in 0..4 {
            m[c * 4 + r] = (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum();
        }
    }
    m
}

/// The point `q` through `m`.
fn apply(m: &[f32; 16], q: [f32; 3]) -> [f32; 4] {
    [0, 1, 2, 3].map(|r| m[r] * q[0] + m[4 + r] * q[1] + m[8 + r] * q[2] + m[12 + r])
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

// the sky (soft glows low on the left and high on the right, fading into a dark band by a smooth step across the
// screen) times its tint, plus the sheet's light times its own, through the response
const COMP_FS: &str = r#"
in vec2 v;
uniform sampler2D sheet;
uniform float daycos;
uniform vec3 st0, st1, ht0, ht1;
vec3 sky(vec2 p){
  float a = exp(-.5 * dot(p - 1.5, p - 1.5));
  float q = (p.x - .4) * (p.x - .4) / 6. + (p.y - .1) * (p.y - .1);
  vec3 s = vec3(.4 * exp(.4 * p.y - .4) + .6 * exp(-16. * (1. + .5 * daycos) * q) + .375 * a,
                .3 * a + .25 * exp(-12. * q) + .3 * exp(.2 * p.y - .2),
                .45 * exp(-8. * q) + .6 * a + .5 * exp(.1 * p.y - .1));
  return s * (.4 * tanh(clamp(2.5 * (p.y - .2 * p.x - .4), -10., 10.)) + .4);
}
void main(){
  vec3 bg = sky(clamp(v, 0., 1.));
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
    facets: Option<glow::Program>,
    halos: Option<glow::Program>,
}

/// The wave's state: the cloth, the sparkles, the cage's clock (and the one it is fading from), and the GL objects.
pub(crate) struct Wave {
    cloth: Cloth,
    sparks: Sparks,
    /// the test switch BACKLINE_WAVE_SPARKS=1|2|3: the sparkles alone on black (1 their facets, 2 their halos, 3 both),
    /// for checks against a reference frame
    only: i32,
    /// a cage still running from before the clock went round, and how much of it is left (1 to 0)
    old: Option<(f32, f32)>,
    t_last: f64,
    failed: bool,
    progs: Progs,
    vao: Option<glow::VertexArray>,
    ibuf: Option<glow::Buffer>,
    net: Option<glow::Texture>,
    glow_tex: Option<glow::Texture>,
    irid: Option<glow::Texture>,
    parts: Option<glow::Texture>,
    slots: std::collections::HashMap<u8, Targets>,
}

impl Default for Wave {
    fn default() -> Self {
        let mut cloth = Cloth::new(&MUSIC);
        // a minute's motion first, so it opens already moving as it always does (the sparkles for the last ten seconds:
        // they fill their pool in five)
        for _ in 0..3000 {
            cloth.step(&MUSIC);
        }
        cloth.wrapped = false;
        let mut w = Wave {
            cloth,
            sparks: Sparks::new(),
            only: std::env::var("BACKLINE_WAVE_SPARKS").ok().and_then(|v| v.parse().ok()).unwrap_or(0),
            old: None,
            t_last: -1.0,
            failed: false,
            progs: Progs::default(),
            vao: None,
            ibuf: None,
            net: None,
            glow_tex: None,
            irid: None,
            parts: None,
            slots: Default::default(),
        };
        for _ in 0..600 {
            w.cloth.step(&MUSIC);
            w.spark_tick();
        }
        w.cloth.wrapped = false;
        w
    }
}

impl Wave {
    /// The cloth on to `t` (sixty steps a second), and the cage's clock (ten times the cloth's; when the cloth's goes
    /// round, the old cage fades out over about three seconds).
    fn step(&mut self, t: f64) {
        let dt = if self.t_last < 0.0 { 0.0 } else { (t - self.t_last).clamp(0.0, 0.25) as f32 };
        self.t_last = t;
        let before = self.cloth.time;
        self.cloth.acc += dt * 60.0;
        while self.cloth.acc > 1.0 {
            self.cloth.step(&MUSIC);
            self.cloth.acc -= 1.0;
            if self.cloth.wrapped {
                self.cloth.wrapped = false;
                self.old = Some((before * 10.0, 1.0));
            }
            self.spark_tick();
        }
        let c = &self.cloth;
        let moved = if c.time >= before { (c.time - before) * 10.0 } else { 0.0 };
        if let Some((ot, f)) = self.old {
            let f = f - dt * 60.0 * 0.005;
            self.old = if f <= 0.0 { None } else { Some((ot + moved.max(0.0), f)) };
        }
    }

    /// One sixtieth of a second of the sparkles, on the sheet as the step just left it.
    fn spark_tick(&mut self) {
        let m = model(&MUSIC);
        let world: Vec<[f32; 3]> = self.outs(1.0).into_iter().map(|q| apply(&m, q)).map(|v| [v[0], v[1], v[2]]).collect();
        self.sparks.tick(&world, &SPARK);
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
        let m = mul(&camera(aspect), &model(&MUSIC));
        self.outs(self.cloth.acc).into_iter().flat_map(|q| apply(&m, q)).collect()
    }

    /// Each cloth point through the cage (before the place and turn), `a` of the way from the last step but one to
    /// the last.
    fn outs(&self, a: f32) -> Vec<[f32; 3]> {
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
        self.cloth.points(s, a).into_iter().map(|p| bend(s, &cg, p)).collect()
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
            facets: program(gl, &h, PARTS_VS, FACET_FS),
            halos: program(gl, &h, PARTS_VS, HALO_FS),
        };
        let p = &self.progs;
        if [p.sheet, p.comp, p.source, p.gauss, p.sum, p.out, p.facets, p.halos].iter().any(|x| x.is_none()) {
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
        let it = tex(gl, 128, 128, glow::RGBA32F, true);
        let bytes: Vec<u8> = swirl().iter().flat_map(|c| [c[0], c[1], c[2], 1.0]).flat_map(|v| v.to_le_bytes()).collect();
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA32F as i32, 128, 128, 0, glow::RGBA, glow::FLOAT, Some(&bytes));
        self.irid = Some(it);
        self.parts = Some(tex(gl, POOL as i32, 2, glow::RGBA32F, false));
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
        gl.uniform_1_f32(u("daycos").as_ref(), DAY_COS);
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
        if self.only == 0 {
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        } else {
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }

        // 5. the sparkles, added on top: their facets, then the halos of the ones glinting (the test switch
        // BACKLINE_WAVE_PARTS=<file>: sparkles from a file instead, eight little-endian floats each, for checks against a
        // reference frame)
        let live: Vec<f32> = match std::env::var("BACKLINE_WAVE_PARTS").ok().and_then(|f| std::fs::read(f).ok()) {
            Some(b) => b.chunks_exact(4).take(POOL * 8).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect(),
            None => self.sparks.live(),
        };
        let n = live.len() / 8;
        if n > 0 {
            let mut rows = vec![0f32; POOL * 8];
            for i in 0..n {
                rows[i * 4..i * 4 + 4].copy_from_slice(&live[i * 8..i * 8 + 4]);
                rows[POOL * 4 + i * 4..POOL * 4 + i * 4 + 4].copy_from_slice(&live[i * 8 + 4..i * 8 + 8]);
            }
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, self.parts);
            let bytes: Vec<u8> = rows.iter().flat_map(|v| v.to_le_bytes()).collect();
            gl.tex_sub_image_2d(glow::TEXTURE_2D, 0, 0, 0, POOL as i32, 2, glow::RGBA, glow::FLOAT, glow::PixelUnpackData::Slice(&bytes));
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, self.irid);
            gl.enable(glow::BLEND);
            gl.blend_equation(glow::FUNC_ADD);
            gl.blend_func_separate(glow::ONE, glow::ONE, glow::ZERO, glow::ONE);
            let cam = camera(w as f32 / h as f32);
            for (pass, prog) in [(1, p.facets.unwrap()), (2, p.halos.unwrap())] {
                if self.only != 0 && self.only & pass == 0 {
                    continue;
                }
                gl.use_program(Some(prog));
                spark_uniforms(gl, prog, &cam, pass);
                gl.draw_arrays(glow::TRIANGLES, 0, (n * 6) as i32);
            }
            gl.disable(glow::BLEND);
        }
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

// ------------------------------------------------------------------------------------------------ the sparkles
/// The sparkles' settings (the set shown while music plays).
struct Spark {
    vel_min: f32,
    vel_mul: f32,
    vel_var: f32,
    cone: f32,
    neg_prob: f32,
    zscale: f32,
    per_frame: u32,
    prob: f32,
    aging: f32,
    aging_var: f32,
    friction: f32,
    spin: f32,
    dt: f32,
    gravity: f32,
    brown: f32,
    spot: [f32; 3],
    attn: [f32; 3],
    spec_pow: f32,
    spec_coeff: f32,
    lambert: f32,
    exposure: f32,
    fresnel: f32,
    color_control: f32,
    irid_exp: f32,
    alpha: f32,
    /// middle, near, far
    size: [f32; 3],
    /// focus: near (where, how far it takes, its curve, darkening, fuzz) and far (where, how far, curve, darkening)
    near: [f32; 5],
    far: [f32; 4],
    near_align: f32,
    size_align: f32,
    /// glare: how much, how big, and its falloff's shape
    glare: [f32; 4],
}

const SPARK: Spark = Spark {
    vel_min: 0.15064,
    vel_mul: 0.19,
    vel_var: 0.282567,
    cone: 51.8695,
    neg_prob: 0.173899,
    zscale: 0.0,
    per_frame: 16,
    prob: 0.479115,
    aging: 0.00285223,
    aging_var: 0.493003,
    friction: 0.030551,
    spin: 2.74,
    dt: 0.0088883,
    gravity: -6.8e-05,
    brown: 0.225311,
    spot: [4.16, 2.63, -7.6],
    attn: [1.0, 0.0, 0.0],
    spec_pow: 35.2904,
    spec_coeff: 74.74,
    lambert: 8.0,
    exposure: 0.0390458,
    fresnel: 1.33319,
    color_control: 1.0,
    irid_exp: 1.0,
    alpha: 1.0,
    size: [0.0464771, 0.0772033, 0.874062],
    near: [6.12435, 1.3193, 2.47206, 6.24028, 3.79],
    far: [12.0064, 3.54129, 1.83324, 13.9],
    near_align: 0.722,
    size_align: 13.19,
    glare: [0.201367, 5.44386, 0.99, 4.44],
};

const POOL: usize = 2048;
const DEAD: f32 = -666.0;
/// Where a sparkle may be (outside it dies).
const BOX: [[f32; 3]; 2] = [[-10.0, -10.0, -12.0], [10.0, 10.0, 7.0]];

/// The sparkles: a pool of 2048 (position and age, velocity and how fast it ages, the turn of its little facet), born
/// on the sheet where it moves and drifting off it; the free slots; the points picked last tick (each born this tick,
/// moving as the sheet did there); now and then a line of them along a strand.
struct Sparks {
    p0: Vec<[f32; 4]>,
    p1: Vec<[f32; 4]>,
    p2: Vec<[f32; 4]>,
    free: Vec<u32>,
    picked: Vec<([f32; 3], usize, usize)>,
    streak: (i32, usize, usize),
    seed: u32,
    tick: u64,
}

impl Sparks {
    fn new() -> Sparks {
        let mut s = Sparks {
            p0: vec![[0.0, 0.0, 0.0, DEAD]; POOL],
            p1: vec![[0.0; 4]; POOL],
            p2: vec![[0.0, 0.0, 0.0, 1.0]; POOL],
            free: (0..POOL as u32).collect(),
            picked: Vec::new(),
            streak: (0, 0, 0),
            seed: 0x2468ace,
            tick: 0,
        };
        // each slot's facet turned every which way, as after a long run (a slot keeps its turn from one sparkle to the
        // next, so from all facing the camera it takes half a minute to spread)
        for i in 0..POOL {
            let (a, b, c) = (s.u(), s.u() * std::f32::consts::TAU, s.u() * std::f32::consts::TAU);
            let (r1, r2) = ((1.0 - a).sqrt(), a.sqrt());
            s.p2[i] = [r1 * b.sin(), r1 * b.cos(), r2 * c.sin(), r2 * c.cos()];
        }
        s
    }

    /// -1..1
    fn rand(&mut self) -> f32 {
        self.seed = self.seed.wrapping_add(1);
        noise(self.seed)
    }

    /// 0..1
    fn u(&mut self) -> f32 {
        (self.rand() + 1.0) * 0.5
    }

    /// One tick on the sheet whose 19 x 19 net (in the world) is `net`: pick points, give birth at last tick's, move
    /// them all.
    fn tick(&mut self, net: &[[f32; 3]], s: &Spark) {
        self.tick += 1;
        // this tick's points: a handful now and then anywhere, and every other tick the next along a running line
        let mut picked = Vec::new();
        let r = self.rand();
        if (r + 1.0) * 0.5 < s.prob && s.per_frame > 0 {
            for _ in 0..s.per_frame {
                let (iu, iv) = ((self.u() * 127.0) as usize, (self.u() * 127.0) as usize);
                picked.push((surface(net, iu, iv), iu, iv));
            }
        }
        let (count, su, sv) = self.streak;
        if count == 0 {
            if self.u() < 0.06 {
                let (a, b) = ((self.u() * 127.0) as usize, (self.u() * 127.0) as usize);
                self.streak = ((self.rand() * 80.0) as i32 + 100, a, b);
            }
        } else if self.tick % 2 == 0 {
            if su < 128 {
                picked.push((surface(net, su, sv), su, sv));
                self.streak = (count - 1, su + 1, sv);
            } else {
                self.streak = (0, su, sv);
            }
        }
        // last tick's points are born now, moving as the sheet moved there, in a cone round that way
        for (was, iu, iv) in std::mem::replace(&mut self.picked, picked) {
            let cur = surface(net, iu, iv);
            let vs = [0, 1, 2].map(|k| (cur[k] - was[k]) / s.dt);
            let len = (vs[0] * vs[0] + vs[1] * vs[1] + vs[2] * vs[2]).sqrt();
            if len < 0.0001 {
                continue;
            }
            let mut d = vs.map(|v| v / len);
            if self.u() < s.neg_prob {
                d = d.map(|v| -v);
            }
            let cands = [[0.0, d[2], -d[1]], [-d[2], 0.0, d[0]], [d[1], -d[0], 0.0]];
            let mut a = cands[0];
            let mut al = 0.0;
            for c in cands {
                let l = c[0] * c[0] + c[1] * c[1] + c[2] * c[2];
                if l >= al {
                    a = c;
                    al = l;
                }
            }
            let a = a.map(|v| v / al.sqrt().max(1e-9));
            let b = [d[1] * a[2] - d[2] * a[1], d[2] * a[0] - d[0] * a[2], d[0] * a[1] - d[1] * a[0]];
            let th = self.u() * s.cone.to_radians();
            let ph = self.u() * std::f32::consts::TAU;
            let mut dir = [0, 1, 2].map(|k| th.cos() * d[k] + th.sin() * (ph.cos() * a[k] + ph.sin() * b[k]));
            // flattened toward the screen (the camera looks along z)
            dir[2] *= s.zscale;
            let speed = (len * s.vel_mul * (1.0 + s.vel_var * self.rand())).max(0.0).sqrt().max(s.vel_min);
            let rate = (s.aging * (1.0 + s.aging_var * self.rand())).max(0.001);
            if let Some(i) = self.free.pop() {
                let i = i as usize;
                self.p0[i] = [cur[0], cur[1], cur[2], 0.0];
                self.p1[i] = [dir[0] * speed, dir[1] * speed, dir[2] * speed, rate];
            }
        }
        // each moves: gravity, a random push, friction; ages; its facet tumbles (the pushes come from three streams
        // started afresh each tick and handed out in slot order, so the ones low in the pool keep theirs)
        let mut st = [0x98756161u32, 0x21324889, 0x82181158];
        let spin = s.spin / 60.0;
        for i in 0..POOL {
            if self.p0[i][3] == DEAD {
                continue;
            }
            let r = [0, 1, 2].map(|k| {
                st[k] = st[k].wrapping_mul(16807);
                f32::from_bits(0x4000_0000 | (st[k] >> 9)) - 3.0
            });
            let (p0, p1) = (&mut self.p0[i], &mut self.p1[i]);
            for k in 0..3 {
                let acc = if k == 1 { s.gravity } else { 0.0 } + s.brown * r[k];
                p1[k] += (acc - s.friction * p1[k]) * s.dt;
                p0[k] += p1[k] * s.dt;
            }
            p0[3] += p1[3];
            let age = p0[3];
            let tau = std::f32::consts::TAU;
            let w = [(tau * 0.37 * age).sin(), (tau * 0.17 * age).cos(), (tau * 0.31 * age).cos(), 0.0];
            let q = self.p2[i];
            let h = [
                w[3] * q[0] + w[0] * q[3] + w[1] * q[2] - w[2] * q[1],
                w[3] * q[1] - w[0] * q[2] + w[1] * q[3] + w[2] * q[0],
                w[3] * q[2] + w[0] * q[1] - w[1] * q[0] + w[2] * q[3],
                w[3] * q[3] - w[0] * q[0] - w[1] * q[1] - w[2] * q[2],
            ];
            let n = [0, 1, 2, 3].map(|k| q[k] + h[k] * 0.5 * spin);
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2] + n[3] * n[3]).sqrt().max(1e-9);
            self.p2[i] = n.map(|v| v / l);
            let out = (0..3).any(|k| p0[k] < BOX[0][k] || p0[k] > BOX[1][k]);
            if !(age < 0.99999) || out {
                p0[3] = DEAD;
                self.free.push(i as u32);
            }
        }
    }

    /// The live ones, in slot order, eight numbers each: position, fade (in over the first 2% of its life, out over
    /// the last 6%), and its facet's turn.
    fn live(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(POOL * 8);
        for i in 0..POOL {
            let p = self.p0[i];
            if p[3] == DEAD {
                continue;
            }
            let fade = p[3].min(0.02) * 50.0 * (1.0 - 16.6667 * (p[3] - 0.94).max(0.0));
            out.extend_from_slice(&[p[0], p[1], p[2], fade]);
            out.extend_from_slice(&self.p2[i]);
        }
        out
    }
}

/// A point of the smooth sheet through the 19 x 19 `net`, at sample `iu` across (along a strand) and `iv` down.
fn surface(net: &[[f32; 3]], iu: usize, iv: usize) -> [f32; 3] {
    let (iu, iv) = (iu.min(127), iv.min(127));
    let (sr, sc) = ((iv / 8).min(15), (iu / 8).min(15));
    let (br, bc) = (bsw((iv - sr * 8) as f32 / 8.0), bsw((iu - sc * 8) as f32 / 8.0));
    let mut p = [0.0; 3];
    for a in 0..4 {
        for b in 0..4 {
            let q = net[(sr + a) * N + sc + b];
            for k in 0..3 {
                p[k] += br[a] * bc[b] * q[k];
            }
        }
    }
    p
}

/// The colours the sparkles' glints take as they tumble: a cycle of hues round a centre, twisted further the further
/// out (measured: one cycle of colours and how much it twists).
const SWIRL: [[f32; 3]; 72] = [
    [0.7394, 0.7659, 0.4551], [0.7347, 0.7085, 0.3647], [0.7203, 0.6361, 0.2772], [0.6824, 0.5595, 0.2323], [0.6462, 0.4923, 0.2282], [0.6791, 0.4926, 0.3001],
    [0.7774, 0.5557, 0.4455], [0.8621, 0.6008, 0.5883], [0.9030, 0.5920, 0.6947], [0.9164, 0.5640, 0.7784], [0.8968, 0.5330, 0.8391], [0.8475, 0.4982, 0.8774],
    [0.7945, 0.4776, 0.9102], [0.7400, 0.4748, 0.9418], [0.6870, 0.4833, 0.9640], [0.6422, 0.5080, 0.9718], [0.6013, 0.5396, 0.9666], [0.5559, 0.5521, 0.9578],
    [0.5070, 0.5441, 0.9439], [0.4555, 0.5303, 0.9305], [0.3971, 0.5153, 0.9191], [0.3440, 0.4949, 0.9099], [0.3167, 0.4605, 0.9051], [0.3078, 0.4199, 0.9020],
    [0.2929, 0.4019, 0.9053], [0.2671, 0.4150, 0.9116], [0.2391, 0.4354, 0.9149], [0.2061, 0.4500, 0.9074], [0.1761, 0.4661, 0.8964], [0.1805, 0.4913, 0.9040],
    [0.2187, 0.5257, 0.9290], [0.2700, 0.5519, 0.9526], [0.3305, 0.5636, 0.9712], [0.3942, 0.5622, 0.9794], [0.4467, 0.5425, 0.9583], [0.4913, 0.5031, 0.9140],
    [0.5285, 0.4330, 0.8856], [0.5553, 0.3205, 0.8824], [0.5910, 0.2203, 0.8769], [0.6572, 0.1816, 0.8444], [0.7359, 0.1799, 0.8002], [0.7908, 0.1877, 0.7701],
    [0.8257, 0.2053, 0.7522], [0.8460, 0.2227, 0.7260], [0.8489, 0.2313, 0.6858], [0.8428, 0.2405, 0.6382], [0.8421, 0.2606, 0.5814], [0.8415, 0.2894, 0.5162],
    [0.8236, 0.3228, 0.4466], [0.7811, 0.3620, 0.3700], [0.7435, 0.3976, 0.3040], [0.7335, 0.4166, 0.2673], [0.7412, 0.4297, 0.2492], [0.7666, 0.4586, 0.2325],
    [0.8094, 0.5025, 0.2177], [0.8519, 0.5572, 0.2150], [0.8875, 0.6266, 0.2326], [0.9166, 0.6996, 0.2604], [0.9325, 0.7619, 0.2875], [0.9383, 0.8144, 0.3168],
    [0.9475, 0.8666, 0.3589], [0.9634, 0.9231, 0.4154], [0.9774, 0.9670, 0.4914], [0.9885, 0.9890, 0.6007], [0.9940, 0.9959, 0.7278], [0.9823, 0.9922, 0.8242],
    [0.9563, 0.9816, 0.8898], [0.8902, 0.9251, 0.8782], [0.7471, 0.7802, 0.7398], [0.6079, 0.6362, 0.5681], [0.5988, 0.6287, 0.4892], [0.6887, 0.7243, 0.4782],
];
const TWIST: f32 = -1.55;

/// The swirl as a 128 x 128 picture (rows from the bottom).
fn swirl() -> Vec<[f32; 3]> {
    let mut out = Vec::with_capacity(128 * 128);
    for y in 0..128 {
        for x in 0..128 {
            let (dx, dy) = ((x as f32 + 0.5) / 64.0 - 1.0, (y as f32 + 0.5) / 64.0 - 1.0);
            let r2 = dx * dx + dy * dy;
            let t = ((dy.atan2(dx) - TWIST * r2) / std::f32::consts::TAU).rem_euclid(1.0) * 72.0 - 0.5;
            let j = t.floor();
            let f = t - j;
            let (a, b) = (SWIRL[(j as i32).rem_euclid(72) as usize], SWIRL[(j as i32 + 1).rem_euclid(72) as usize]);
            out.push([0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * f));
        }
    }
    out
}

unsafe fn spark_uniforms(gl: &glow::Context, p: glow::Program, cam: &[f32; 16], pass: i32) {
    let s = &SPARK;
    let u = |n: &str| gl.get_uniform_location(p, n);
    gl.uniform_1_i32(u("parts").as_ref(), 0);
    gl.uniform_1_i32(u("irid").as_ref(), 1);
    gl.uniform_1_i32(u("pass").as_ref(), pass);
    gl.uniform_matrix_4_f32_slice(u("cam").as_ref(), false, cam);
    gl.uniform_3_f32(u("eye").as_ref(), 0.0, 0.0, 2.0);
    gl.uniform_3_f32_slice(u("spot").as_ref(), &s.spot);
    gl.uniform_3_f32_slice(u("attn").as_ref(), &s.attn);
    gl.uniform_3_f32_slice(u("lbmin").as_ref(), &BOX[0]);
    gl.uniform_3_f32_slice(u("lbmax").as_ref(), &BOX[1]);
    gl.uniform_4_f32(u("ps").as_ref(), s.size[0], s.size[1], s.size[2], 0.0);
    gl.uniform_2_f32(u("tr").as_ref(), s.fresnel, s.alpha);
    gl.uniform_4_f32(u("fo").as_ref(), s.near[0], s.near[0] + s.near[1], s.far[0], s.far[0] + s.far[1]);
    gl.uniform_3_f32(u("fc").as_ref(), s.near[2], s.far[2], 53.0f32.to_radians());
    gl.uniform_3_f32(u("nc").as_ref(), s.size_align, s.near[4], s.near_align);
    gl.uniform_2_f32(u("dk").as_ref(), s.near[3], s.far[3]);
    gl.uniform_4_f32(u("glr").as_ref(), s.glare[0], s.glare[1], s.glare[2], s.glare[3]);
    gl.uniform_4_f32(u("light").as_ref(), s.lambert, s.spec_coeff, s.exposure, s.color_control);
    gl.uniform_2_f32(u("sp").as_ref(), s.spec_pow, s.irid_exp);
}

// each sparkle a quad: a little facet tumbling (pass 1, turning partly to face the camera when small and near) or a
// halo facing the camera that grows when the facet glints toward it (pass 2); focus and distance darken them
const PARTS_VS: &str = r#"
uniform sampler2D parts;
uniform mat4 cam;
uniform vec3 eye, spot, lbmin, lbmax;
uniform vec4 ps, fo, glr;
uniform vec3 fc, nc;
uniform vec2 tr, dk, sp;
uniform int pass;
out vec2 uv;
out vec2 blur;
out vec3 nn;
out vec3 pw;
out float alpha;
vec3 qx(vec4 q){ return vec3(1. - 2. * (q.y * q.y + q.z * q.z), 2. * (q.x * q.y + q.w * q.z), 2. * (q.z * q.x - q.w * q.y)); }
vec3 qy(vec4 q){ return vec3(2. * (q.x * q.y - q.w * q.z), 1. - 2. * (q.x * q.x + q.z * q.z), 2. * (q.y * q.z + q.w * q.x)); }
vec3 qz(vec4 q){ return vec3(2. * (q.x * q.z + q.w * q.y), 2. * (q.y * q.z - q.w * q.x), 1. - 2. * (q.x * q.x + q.y * q.y)); }
void main(){
  int i = gl_VertexID / 6, k = gl_VertexID - (gl_VertexID / 6) * 6;
  vec2 corner = k == 0 || k == 3 ? vec2(0., 0.) : k == 1 ? vec2(1., 0.) : k == 5 ? vec2(0., 1.) : vec2(1., 1.);
  vec4 p0 = texelFetch(parts, ivec2(i, 0), 0);
  vec4 q = normalize(texelFetch(parts, ivec2(i, 1), 0));
  vec3 p = p0.xyz;
  vec3 e = min(p - lbmin, lbmax - p);
  float bx = clamp(min(min(e.x, e.y), e.z) * 5., 0., 1.);
  float boxfade = bx * bx * (3. - 2. * bx);
  vec3 N = qz(q);
  float d = length(eye - p);
  vec3 V = (eye - p) / d;
  float nb = (fo.y - clamp(d, fo.x, fo.y)) / (fo.y - fo.x);
  float fb = (clamp(d, fo.z, fo.w) - fo.z) / (fo.w - fo.z);
  float nbp = pow(nb, fc.x), fbp = pow(fb, fc.y);
  float size = mix(mix(ps.x, ps.y, nbp), ps.z, fbp);
  float band = 1. + clamp((d - (fo.z - .2)) * 5., 0., 1.) - clamp((d - (fo.z - .6)) * 5., 0., 1.);
  float rampC = clamp((d - (fo.z - .4)) * 5., 0., 1.);
  float sf = 2. * atan(size / d) / fc.z;
  float dark = (1. - clamp(fb * sf * dk.y, 0., 1.)) * (1. - clamp(nb * sf * dk.x, 0., 1.));
  vec2 c = corner - .5;
  vec3 P;
  if (pass == 1) {
    float align = clamp(sf * nc.x + nbp * nc.z + rampC, 0., 1.);
    vec4 qa = normalize(q + align * (vec4(0., 0., 0., 1.) - q));
    P = p + size * (c.x * qx(qa) + c.y * qy(qa));
    float fres = 1. - pow(1. - abs(dot(V, qz(qa))), tr.x);
    alpha = fres * tr.y * p0.w * band * dark * boxfade;
  } else {
    vec3 Hc = normalize(normalize(spot - p) + V);
    float halo = size * pow(abs(dot(N, Hc)), sp.x) * glr.y;
    P = p + halo * vec3(c, 0.);
    float fres = 1. - pow(1. - abs(V.z), tr.x);
    alpha = p0.w * fres * tr.y * band * dark * boxfade;
  }
  gl_Position = cam * vec4(P, 1.);
  uv = corner;
  blur = vec2(nbp, fbp);
  nn = N;
  pw = P;
}
"#;

// the facet lit by the spot: a soft light all over and a glint tinted by the swirl, shaped into a disc that softens
// out of focus
const FACET_FS: &str = r#"
in vec2 uv;
in vec2 blur;
in vec3 nn;
in vec3 pw;
in float alpha;
uniform sampler2D irid;
uniform vec3 eye, spot, attn, nc;
uniform vec4 light;
uniform vec2 sp;
void main(){
  vec3 t = texture(irid, nn.xy * .5 + .5).rgb;
  vec3 ir = pow(max(1. + light.w * (t - 1.), 0.), vec3(sp.y));
  vec3 Lf = normalize(spot - pw), Vf = normalize(eye - pw), H = normalize(Lf + Vf);
  float spec = pow(abs(dot(nn, H)), sp.x);
  float dl = length(spot - pw);
  float att = attn.x + attn.y * dl + attn.z * dl * dl;
  vec3 col = (abs(dot(nn, Lf)) * light.x + spec * light.y * ir) / att;
  float bl = clamp(blur.x + blur.y, 0., 1.);
  float bw = bl * .6345 + .05;
  float r = length((uv - .5) * 2.);
  float s = clamp((r - (.5 - bw)) / (2. * bw), 0., 1.);
  float mask = 1. - s * s * (3. - 2. * s);
  float fz = mix(.85, nc.y, blur.x);
  float shape = mask + bl * (1. - mask - exp(-mask * fz));
  float A = shape * alpha;
  if (A <= 0.) discard;
  o = vec4(A * (1. - exp(-light.z * col)), A);
}
"#;

// the halo: the glint's light only, falling off from the centre
const HALO_FS: &str = r#"
in vec2 uv;
in vec2 blur;
in vec3 nn;
in vec3 pw;
in float alpha;
uniform sampler2D irid;
uniform vec3 eye, spot, attn;
uniform vec4 light, glr;
uniform vec2 sp;
void main(){
  vec3 t = texture(irid, nn.xy * .5 + .5).rgb;
  vec3 ir = pow(max(1. + light.w * (t - 1.), 0.), vec3(sp.y));
  vec3 Lf = normalize(spot - pw), Vf = normalize(eye - pw), H = normalize(Lf + Vf);
  float spec = pow(abs(dot(nn, H)), sp.x);
  float dl = length(spot - pw);
  float att = attn.x + attn.y * dl + attn.z * dl * dl;
  vec3 e = 1. - exp(-light.z * spec * light.y * ir / att);
  float r = min(1., length((uv - .5) * 2.));
  float g = exp(-glr.w * pow(r, glr.z)) * glr.x;
  if (alpha * e.r <= 0.) discard;
  o = vec4(alpha * e * g, alpha * e.r);
}
"#;
