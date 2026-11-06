//! 3D Terrain: a sheet the music builds a row at a time as the camera glides over it (a row a unit): lows at the
//! edges, highs in the middle. Its sets, rows, camera and colours are worked out as its reference does
//! (terrain_exact.rs, the sets in terrain_sets.rs); here the sheet, the music's spectrum and the drawing: lit through a
//! colour ramp by how squarely each spot faces the camera, fogged, then softened by a depth-of-field blur stack, a leak
//! of the last frame and grain.

use eframe::egui;
use eframe::egui_glow;
use eframe::glow::{self, HasContext};

#[path = "terrain_exact.rs"]
mod exact;
#[path = "terrain_sets.rs"]
#[allow(dead_code)]
mod sets;

/// Columns across the sheet (the last repeats the one before it) and slots in its ring of rows
/// (one always empty: the next to fill).
const COLS: usize = 129;
const RING: usize = 129;
const ROWS: usize = 128;
/// Colours kept of each look's ramp.
pub(crate) const RAMP_N: usize = 16;

/// A look: everything the shaders need for a frame, as numbers (so two can be blended).
pub(crate) mod lk {
    /// eye x, eye y, and how far the eye sits behind the front
    pub const EYE: usize = 0;
    /// where the camera looks (a unit vector) and its vertical field of view in degrees
    pub const FWD: usize = 3;
    pub const FOV: usize = 6;
    /// the sheet's sideways stretch and height scale
    pub const XS: usize = 7;
    pub const HS: usize = 8;
    /// fog: how far it takes to close in, and where it starts
    pub const FOG_RANGE: usize = 9;
    pub const FOG_START: usize = 10;
    /// how many times the fine bumps repeat across the sheet
    pub const TILE: usize = 11;
    /// where new rows start settling in, and the leading edge, both from the front
    pub const LIFT: usize = 12;
    pub const CLIP: usize = 13;
    /// the swell around the front: half width, height, ripple frequency, bias
    pub const SWELL: usize = 14;
    pub const BUMP: usize = 18;
    pub const GLOW: usize = 19;
    pub const FOG: usize = 22;
    pub const SIDE: usize = 25;
    /// depth of field: the distance in focus, how fast it blurs away from it, the sky's blur
    pub const FOCUS: usize = 28;
    pub const COC: usize = 29;
    pub const SKY: usize = 30;
    /// the blur everywhere, and the last frame leaking back in: how much, turned how far, stretched how much
    pub const BLUR: usize = 31;
    pub const LEAK: usize = 32;
    pub const ROT: usize = 33;
    pub const LSCALE: usize = 34;
    /// rows (units) a second
    pub const SPEED: usize = 36;
    /// the colour ramp, edge-on to face-on
    pub const RAMP: usize = 37;
    pub const N: usize = RAMP + 3 * super::RAMP_N;
}
pub(crate) type Look = [f32; lk::N];

fn unit(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-5);
    [v[0] / l, v[1] / l, v[2] / l]
}

// ------------------------------------------------------------------------------------------------ the music
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let a = -2.0 * std::f32::consts::PI / len as f32;
        let (wr, wi) = (a.cos(), a.sin());
        for s in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let (p, q) = (s + k, s + k + len / 2);
                let (tr, ti) = (re[q] * cr - im[q] * ci, re[q] * ci + im[q] * cr);
                re[q] = re[p] - tr;
                im[q] = im[p] - ti;
                re[p] += tr;
                im[p] += ti;
                let nr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = nr;
            }
        }
        len <<= 1;
    }
}

// ------------------------------------------------------------------------------------------------ the sheet
/// The sheet's ring of rows, as the vertex program reads it: per point the smoothed height (py), the raw one
/// (pw), the random lift of the tile starting there (tw) and its row's place (z); and the normal of the smoothed
/// heights with the row's sideways offset.
struct Sheet {
    a: Vec<[f32; 4]>,
    b: Vec<[f32; 4]>,
    /// the next row's number (its slot is the number mod RING)
    next: i64,
    /// slots changed since the last upload
    dirty: Vec<usize>,
    seed: u64,
}

impl Sheet {
    fn new() -> Sheet {
        let s = Sheet {
            a: vec![[0.0; 4]; COLS * RING],
            b: vec![[0.0, 1.0, 0.0, 0.0]; COLS * RING],
            next: -(ROWS as i64),
            dirty: Vec::new(),
            seed: 0x2545f4914f6cdd1d,
        };
        s
    }

    fn rnd(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 40) as f32 / 16777216.0
    }

    /// A new row of heights `py` as drawn and `pw` (what the swell pulls them toward), shifted `xoff` across.
    fn push_row(&mut self, py: &[f32; COLS], pw: &[f32; COLS], xoff: f32) {
        let n = self.next;
        let slot = n.rem_euclid(RING as i64) as usize;
        let prev = (n - 1).rem_euclid(RING as i64) as usize;
        let older = (n - 2).rem_euclid(RING as i64) as usize;
        for c in 0..COLS {
            let tw = (self.rnd() * 2.0 - 1.0) * 500.0;
            self.a[slot * COLS + c] = [py[c], pw[c], tw, n as f32];
        }
        // normals: across from the neighbours; along, the step from the row before (the row before gets the full
        // difference now that it has a row on each side)
        for (s, back, fwd) in [(slot, prev, None), (prev, older, Some(slot))] {
            for c in 0..COLS {
                let h = |sl: usize, cc: usize| self.a[sl * COLS + cc.min(COLS - 1)][0];
                let dx = (h(s, c + 1) - h(s, c.saturating_sub(1))) / if c == 0 || c >= COLS - 1 { 1.0 } else { 2.0 };
                let dz = match fwd {
                    Some(f) => h(f, c) - h(back, c),
                    None => (h(s, c) - h(back, c)) * 2.0,
                };
                let l = (dx * dx + 1.0 + dz * dz).sqrt();
                let x = if s == slot { xoff } else { self.b[s * COLS + c][3] };
                self.b[s * COLS + c] = [-dx / l, 1.0 / l, -dz / l, x];
            }
        }
        self.dirty.push(slot);
        self.dirty.push(prev);
        self.next += 1;
    }
}

// ------------------------------------------------------------------------------------------------ the GPU side
// the row the sheet's places are counted from (shared by the sheet and its depth of field)
const PLACE: &str = r#"
uniform float zbase;
"#;

const SHEET_VS: &str = r#"
uniform sampler2D rowA, rowB;
uniform int oldest;
uniform mat4 mvp;
uniform vec3 eye;
uniform vec2 c8;
uniform vec3 fogp;
uniform vec3 c12;
uniform vec4 c13;
out vec2 tc0; out vec2 tc1; out vec2 tc2; out vec3 tc3; out vec3 tc4; out vec3 tc5; out vec3 tc6; out float tc7;
// the tangent: across the sheet, or along it when the normal points down the sheet
vec3 side(vec3 n){ return n.z < .99 ? normalize(vec3(n.y, -n.x, 0.)) : normalize(vec3(-n.z, 0., n.x)); }
float near(float z){ float a = clamp(abs(c12.x - z) / c13.x, 0., 1.); return 1. - a * a * (3. - 2. * a); }
void main(){
  // the sheet is separate tiles, four corners each, so tiles can settle in one by one
  int q = gl_VertexID / 4, corner = gl_VertexID % 4;
  int qc = q % 128, qk = q / 128;
  int c = qc + int(corner == 1 || corner == 2), k = qk + int(corner >= 2);
  int slot = (oldest + k) % 129, qslot = (oldest + qk) % 129;
  vec4 A = texelFetch(rowA, ivec2(c, slot), 0);
  vec4 B = texelFetch(rowB, ivec2(c, slot), 0);
  vec4 Q = texelFetch(rowA, ivec2(qc, qslot), 0);
  float cx = float(min(c, 127)) - 64., qx = float(qc) - 64.;
  float px = cx + B.w, pz = A.w - zbase;
  vec3 n = normalize(B.xyz);
  tc0 = vec2(float(c), float(slot)) / 127.;
  tc1 = tc0 * fogp.z;
  // tiles past the lift line settle in from a random height (one per tile), the middle of the sheet first
  float tz = Q.w - zbase + 1. - 50. * (qx / 64.) * (qx / 64.);
  float lift = max((tz - c12.y) * .01, 0.);
  float h0 = A.x + Q.z * lift * lift;
  vec3 P0 = vec3(px * c8.x, h0, pz);
  vec3 a = side(n), b = cross(n, a);
  vec3 P1 = P0 + a, P2 = P0 + b;
  float e0 = near(pz), e1 = near(P1.z), e2 = near(P2.z);
  float k1 = cos(abs(c12.x - P1.z) * c13.z), k2 = cos(abs(c12.x - P2.z) * c13.z);
  float Hs = h0 * (c8.y + c13.y * k1 * (c13.w + e0));
  float H1 = P1.y * (c8.y + c13.y * k1 * (c13.w + e1));
  float H2 = P2.y * (c8.y + c13.y * k2 * (c13.w + e2));
  float Hf = Hs + .2 * e0 * (A.y * c8.y - Hs);
  vec3 P = vec3(P0.x, Hf, pz);
  gl_Position = mvp * vec4(P, 1.);
  vec3 S0 = vec3(P0.x, Hs, pz);
  vec3 N = normalize(cross(vec3(P2.x, H2, P2.z) - S0, vec3(P1.x, H1, P1.z) - S0));
  vec3 V = eye - P;
  float dist = length(V);
  tc6 = V / dist;
  tc2 = vec2(1. - clamp((dist - fogp.y) * fogp.x, 0., 1.), .2 * e0 * abs(Hf));
  vec3 T = side(N);
  tc3 = T; tc4 = cross(N, T); tc5 = N;
  tc7 = pz - c12.z;
}
"#;

const SHEET_FS: &str = r#"
uniform sampler2D bump, ramp;
uniform float bumpk;
uniform vec2 rampfix;
uniform vec3 glow, fogc, sidec;
in vec2 tc0; in vec2 tc1; in vec2 tc2; in vec3 tc3; in vec3 tc4; in vec3 tc5; in vec3 tc6; in float tc7;
void main(){
  if (tc7 > 0.) discard;
  vec3 b = texture(bump, tc1).xyz - .5;
  vec3 n = tc3 * (b.x * bumpk) + tc4 * (b.y * bumpk) + tc5 * b.z;
  float ndv = abs(dot(tc6, n) / length(n));
  vec3 col = texture(ramp, vec2(ndv * rampfix.x + rampfix.y, .5)).rgb;
  col = mix(col, glow, tc2.y);
  col = mix(fogc, col, tc2.x);
  float e = abs(tc0.x - .5);
  col = mix(col, sidec, clamp(8. * e * e * e, 0., 1.));
  o = vec4(col, 1.);
}
"#;

// the depth-of-field pass: the same sheet, writing how far out of focus each point is
const COC_VS: &str = r#"
uniform sampler2D rowA, rowB;
uniform int oldest;
uniform mat4 mvp;
uniform vec3 eye;
uniform vec2 c8;
uniform vec3 c12;
uniform vec4 c13;
uniform vec2 focus;
out float coc; out float tc7;
void main(){
  int q = gl_VertexID / 4, corner = gl_VertexID % 4;
  int qc = q % 128, qk = q / 128;
  int c = qc + int(corner == 1 || corner == 2), k = qk + int(corner >= 2);
  int slot = (oldest + k) % 129, qslot = (oldest + qk) % 129;
  vec4 A = texelFetch(rowA, ivec2(c, slot), 0);
  vec4 B = texelFetch(rowB, ivec2(c, slot), 0);
  vec4 Q = texelFetch(rowA, ivec2(qc, qslot), 0);
  float cx = float(min(c, 127)) - 64., qx = float(qc) - 64.;
  float px = cx + B.w, pz = A.w - zbase;
  vec3 n = normalize(B.xyz);
  float tz = Q.w - zbase + 1. - 50. * (qx / 64.) * (qx / 64.);
  float lift = max((tz - c12.y) * .01, 0.);
  float h0 = A.x + Q.z * lift * lift;
  float a0 = clamp(abs(pz - c12.x) / c13.x, 0., 1.), e0 = 1. - a0 * a0 * (3. - 2. * a0);
  float az = n.z < .99 ? 0. : normalize(vec3(-n.z, 0., n.x)).z;
  float Hs = h0 * (c8.y + c13.y * cos(abs(c12.x - (pz + az)) * c13.z) * (c13.w + e0));
  float Hf = Hs + .2 * e0 * (A.y * c8.y - Hs);
  vec3 P = vec3(px * c8.x, Hf, pz);
  gl_Position = mvp * vec4(P, 1.);
  coc = clamp(abs(length(eye - P) - focus.x) * focus.y, 0., 1.);
  tc7 = pz - c12.z;
}
"#;

const COC_FS: &str = r#"
in float coc; in float tc7;
void main(){ if (tc7 > 0.) discard; o = vec4(coc, 0., 0., 0.); }
"#;

// a quad over the whole target; `v` runs 0..1
const QUAD_VS: &str = r#"
out vec2 v;
void main(){ vec2 a = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2)); v = a; gl_Position = vec4(a * 2. - 1., 0., 1.); }
"#;

const FILL_FS: &str = r#"
uniform vec4 val;
void main(){ o = val; }
"#;

// the depth-of-field map softened: four taps half a texel off on the diagonals
const BLUR4_FS: &str = r#"
in vec2 v;
uniform sampler2D src;
uniform vec2 off;
void main(){
  o = (texture(src, v + vec2(off.x, -off.y)) + texture(src, v - off) + texture(src, v + off) + texture(src, v + vec2(-off.x, off.y))) * .25;
}
"#;

// shrinking the picture for the blur stack: the average over each target texel's footprint
const SHRINK_FS: &str = r#"
in vec2 v;
uniform sampler2D src;
uniform vec2 step;
uniform int taps;
void main(){
  vec4 s = vec4(0.);
  for (int i = 0; i < taps; i++) s += texture(src, v + step * (float(i) - float(taps - 1) * .5));
  o = s / float(taps);
}
"#;

// one level of the blur stack: four diagonal taps, weighted by how much of each is sheet
const STACK_FS: &str = r#"
in vec2 v;
uniform sampler2D src;
uniform vec2 off;
void main(){
  vec4 a = texture(src, v + off), b = texture(src, v - off), c = texture(src, v + vec2(off.x, -off.y)), d = texture(src, v - vec2(off.x, -off.y));
  float w = a.a + b.a + c.a + d.a;
  o = w > 0. ? vec4((a.rgb * a.a + b.rgb * b.a + c.rgb * c.a + d.rgb * d.a) / w, w * .25) : vec4(0.);
}
"#;

// the last pass, laid over the sharp picture: the blur stack by how far out of focus each spot is (never less
// than the look's blur), the last frame leaking back in turned and stretched, and grain
const COMP_FS: &str = r#"
in vec2 v;
uniform sampler2D coc, prev, grain, s0, s1, s2, s3;
uniform float blur0, leak, gscale;
uniform vec2 rot, lscale, goff;
void main(){
  float b = clamp(texture(coc, v).x * .75 + blur0, 0., 1.);
  float B6 = b * 6.;
  float lev = clamp((B6 - 2.) * .25, 0., 1.);
  float w = clamp(B6 * .5, 0., 1.);
  vec2 uv = v - B6 * .5 * vec2(.0004, .000225);
  float L = clamp((lev + .1) * 4. - .5, 0., 3.);
  vec3 c0 = texture(s0, uv).rgb, c1 = texture(s1, uv).rgb, c2 = texture(s2, uv).rgb, c3 = texture(s3, uv).rgb;
  vec3 bl = L < 1. ? mix(c0, c1, L) : L < 2. ? mix(c1, c2, L - 1.) : mix(c2, c3, L - 2.);
  float lw = w * leak;
  vec3 acc = (bl - .5 / 128.) * (w - lw);
  float aw = w + leak - lw;
  vec2 q = (v - .5) * lscale;
  vec2 r = vec2(q.x * rot.y + q.y * rot.x, -q.x * rot.x + q.y * rot.y) + .5;
  acc += texture(prev, r).rgb * leak;
  vec4 g = texture(grain, gl_FragCoord.xy * gscale + goff);
  o = vec4(acc / max(aw, 1e-4), aw) + g / 64.;
}
"#;

const COPY_FS: &str = r#"
in vec2 v;
uniform sampler2D src;
void main(){ o = vec4(texture(src, v).rgb, 1.); }
"#;

/// One spot's pictures (full screen, Now Playing, ...), sized to it.
struct Targets {
    w: i32,
    h: i32,
    /// the picture (with depth) and the last one, for the leak
    main: (glow::Framebuffer, glow::Texture, glow::Renderbuffer),
    prev: (glow::Framebuffer, glow::Texture),
    /// depth of field (with its own depth) and its blur partner
    coc: (glow::Framebuffer, glow::Texture, glow::Renderbuffer),
    coc2: (glow::Framebuffer, glow::Texture),
    /// the picture shrunk across, then down, and the four blur levels
    wide: (glow::Framebuffer, glow::Texture),
    small: (glow::Framebuffer, glow::Texture),
    stack: [(glow::Framebuffer, glow::Texture); 4],
    sizes: [(i32, i32); 3],
}

pub(crate) unsafe fn tex(gl: &glow::Context, w: i32, h: i32, fmt: u32, linear: bool) -> glow::Texture {
    let t = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D, Some(t));
    let (ifmt, ty) = if fmt == glow::RGBA16F { (glow::RGBA16F, glow::HALF_FLOAT) } else if fmt == glow::RGBA32F { (glow::RGBA32F, glow::FLOAT) } else { (glow::RGBA8, glow::UNSIGNED_BYTE) };
    gl.tex_image_2d(glow::TEXTURE_2D, 0, ifmt as i32, w, h, 0, glow::RGBA, ty, None);
    let f = if linear { glow::LINEAR } else { glow::NEAREST } as i32;
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, f);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, f);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
    t
}

pub(crate) unsafe fn fbo(gl: &glow::Context, t: glow::Texture, depth: Option<glow::Renderbuffer>) -> glow::Framebuffer {
    let f = gl.create_framebuffer().unwrap();
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(f));
    gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(t), 0);
    if let Some(d) = depth {
        gl.framebuffer_renderbuffer(glow::FRAMEBUFFER, glow::DEPTH_ATTACHMENT, glow::RENDERBUFFER, Some(d));
    }
    f
}

unsafe fn depth(gl: &glow::Context, w: i32, h: i32) -> glow::Renderbuffer {
    let r = gl.create_renderbuffer().unwrap();
    gl.bind_renderbuffer(glow::RENDERBUFFER, Some(r));
    gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH_COMPONENT24, w, h);
    r
}

impl Targets {
    unsafe fn new(gl: &glow::Context, w: i32, h: i32) -> Targets {
        // sizes for 1920 x 1080, kept in proportion: depth of field 512 x 288, the stack 256 x 144
        let cw = (w * 512 / 1920).max(8);
        let ch = (h * 288 / 1080).max(8);
        let sw = (w * 256 / 1920).max(4);
        let sh = (h * 144 / 1080).max(4);
        let md = depth(gl, w, h);
        let mt = tex(gl, w, h, glow::RGBA8, true);
        let pt = tex(gl, w, h, glow::RGBA8, true);
        let cd = depth(gl, cw, ch);
        let ct = tex(gl, cw, ch, glow::RGBA16F, true);
        let c2 = tex(gl, cw, ch, glow::RGBA16F, true);
        let wt = tex(gl, sw, h, glow::RGBA16F, true);
        let st = tex(gl, sw, sh, glow::RGBA16F, true);
        let stack = [0, 1, 2, 3].map(|_| {
            let t = tex(gl, sw, sh, glow::RGBA16F, true);
            (fbo(gl, t, None), t)
        });
        Targets {
            w,
            h,
            main: (fbo(gl, mt, Some(md)), mt, md),
            prev: (fbo(gl, pt, None), pt),
            coc: (fbo(gl, ct, Some(cd)), ct, cd),
            coc2: (fbo(gl, c2, None), c2),
            wide: (fbo(gl, wt, None), wt),
            small: (fbo(gl, st, None), st),
            stack,
            sizes: [(cw, ch), (sw, h), (sw, sh)],
        }
    }

    unsafe fn free(&self, gl: &glow::Context) {
        for (f, t) in [(self.main.0, self.main.1), self.prev, (self.coc.0, self.coc.1), self.coc2, self.wide, self.small]
            .into_iter()
            .chain(self.stack.iter().copied())
        {
            gl.delete_framebuffer(f);
            gl.delete_texture(t);
        }
        gl.delete_renderbuffer(self.main.2);
        gl.delete_renderbuffer(self.coc.2);
    }
}

#[derive(Default)]
struct Progs {
    sheet: Option<glow::Program>,
    coc: Option<glow::Program>,
    fill: Option<glow::Program>,
    blur4: Option<glow::Program>,
    shrink: Option<glow::Program>,
    stack: Option<glow::Program>,
    comp: Option<glow::Program>,
    copy: Option<glow::Program>,
}

pub(crate) unsafe fn program(gl: &glow::Context, version: &str, vs: &str, fs: &str) -> Option<glow::Program> {
    let head = "#ifdef GL_ES\nprecision highp float;\nprecision highp int;\nprecision highp sampler2D;\n#endif\n";
    let p = gl.create_program().ok()?;
    let mut shaders = Vec::new();
    for (kind, src) in [
        (glow::VERTEX_SHADER, format!("{}{}{}", version, head, vs)),
        (glow::FRAGMENT_SHADER, format!("{}{}out vec4 o;\n{}", version, head, fs)),
    ] {
        let s = gl.create_shader(kind).ok()?;
        gl.shader_source(s, &src);
        gl.compile_shader(s);
        if !gl.get_shader_compile_status(s) {
            crate::api::log(&format!("3D: shader didn't compile: {}", gl.get_shader_info_log(s)));
            return None;
        }
        gl.attach_shader(p, s);
        shaders.push(s);
    }
    gl.link_program(p);
    for s in shaders {
        gl.detach_shader(p, s);
        gl.delete_shader(s);
    }
    if !gl.get_program_link_status(p) {
        crate::api::log(&format!("3D: shader didn't link: {}", gl.get_program_info_log(p)));
        return None;
    }
    Some(p)
}

/// What the terrain needs each frame besides the GL context.
pub(crate) struct Input {
    /// seconds, steadily rising
    pub t: f64,
    /// the latest samples (-1..1): the last channel (the only one in mono) and the first; and their rate
    pub samples: Vec<f32>,
    pub first: Vec<f32>,
    pub rate: f32,
    /// changes when the song does (a new group of sets comes in)
    pub song: u64,
    /// the song playing, for reading it ahead of what is heard (None: nothing playing)
    pub music: Option<Music>,
}

/// A song playing: its file (with a number that changes with it), where it is (song seconds), how fast it plays and
/// its A-B loop (seconds) while one is on.
pub(crate) struct Music {
    pub file: (u64, std::sync::Arc<Vec<u8>>),
    pub at: f64,
    pub speed: f32,
    pub ab: Option<(f32, f32)>,
}

impl Music {
    /// The song's place `secs` seconds from now as heard (around the loop while one is on).
    fn ahead(&self, secs: f32) -> f64 {
        let to = self.at + (secs * self.speed) as f64;
        match self.ab {
            Some((a, b)) if self.at < b as f64 && to > b as f64 => a as f64 + (to - b as f64) % (b - a).max(0.05) as f64,
            _ => to,
        }
    }
}

/// The 3D Terrain's state: the sheet, the music, the look, and the GL objects (one set of pictures per spot).
pub(crate) struct Terrain {
    sheet: Sheet,
    /// the reference's own way, its colour table this frame (512 colours), and the song as it reads it (ahead of what
    /// is heard)
    exact: Option<Box<exact::Exact>>,
    exact_ramp: Vec<f32>,
    ahead: crate::player::Ahead,
    /// how far the front has moved (rows), the last frame's time and the look now
    front: f64,
    t_last: f64,
    look: Look,
    /// a reference frame from a file, drawn as it was (the test switch BACKLINE_TERRAIN_FRAME)
    dump: Option<(Look, Vec<f32>, f64, i64)>,
    failed: bool,
    head: String,
    progs: Progs,
    vao: Option<glow::VertexArray>,
    ibuf: Option<glow::Buffer>,
    rows: Option<(glow::Texture, glow::Texture)>,
    bump: Option<glow::Texture>,
    grain: Option<glow::Texture>,
    ramp: Option<glow::Texture>,
    slots: std::collections::HashMap<u8, Targets>,
    seed: u64,
    frame: u64,
}

impl Default for Terrain {
    fn default() -> Self {
        Terrain {
            sheet: Sheet::new(),
            exact: None,
            exact_ramp: Vec::new(),
            ahead: Default::default(),
            front: 0.0,
            t_last: -1.0,
            look: [0.0; lk::N],
            dump: None,
            failed: false,
            head: String::new(),
            progs: Progs::default(),
            vao: None,
            ibuf: None,
            rows: None,
            bump: None,
            grain: None,
            ramp: None,
            slots: Default::default(),
            seed: 0x9e3779b97f4a7c15,
            frame: 0,
        }
    }
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The shape the looks are made for. A narrower spot (a window made thin or tall) keeps all of this width in view and
/// sees more above and below, instead of losing the sides and looking zoomed in; a wider one sees more at the sides.
pub(crate) const WIDE: f32 = 16.0 / 9.0;

/// A column-major perspective times a look-at, leaning `roll` radians (the terrain's camera keeps level). `fov_deg`
/// is up and down at WIDE or wider.
fn camera(eye: [f32; 3], fwd: [f32; 3], fov_deg: f32, aspect: f32, roll: f32) -> [f32; 16] {
    let f = fwd;
    let up = [0.0f32, 1.0, 0.0];
    let mut r = [f[1] * up[2] - f[2] * up[1], f[2] * up[0] - f[0] * up[2], f[0] * up[1] - f[1] * up[0]];
    let rl = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt().max(1e-5);
    r = [r[0] / rl, r[1] / rl, r[2] / rl];
    let u = [r[1] * f[2] - r[2] * f[1], r[2] * f[0] - r[0] * f[2], r[0] * f[1] - r[1] * f[0]];
    let (sr, cr) = roll.sin_cos();
    let (r, u) = ([0, 1, 2].map(|i| r[i] * cr + u[i] * sr), [0, 1, 2].map(|i| u[i] * cr - r[i] * sr));
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let sy = 1.0 / (fov_deg.to_radians() * 0.5).tan() * (aspect / WIDE).min(1.0);
    let sx = sy / aspect;
    let (near, far) = (0.2f32, 2000.0f32);
    let (za, zb) = ((far + near) / (near - far), 2.0 * far * near / (near - far));
    // rows of the clip transform: x = sx (r . (P - eye)), y = sy (u . (P - eye)), w = f . (P - eye)
    let rows = [
        [sx * r[0], sx * r[1], sx * r[2], -sx * dot(r, eye)],
        [sy * u[0], sy * u[1], sy * u[2], -sy * dot(u, eye)],
        [-za * f[0], -za * f[1], -za * f[2], za * dot(f, eye) + zb],
        [f[0], f[1], f[2], -dot(f, eye)],
    ];
    let mut m = [0f32; 16];
    for (i, row) in rows.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            m[j * 4 + i] = *v;
        }
    }
    m
}

impl Terrain {
    fn rnd(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 40) as f32 / 16777216.0
    }

    /// Move the sheet and the look on to `inp.t`.
    fn step(&mut self, inp: &Input) {
        let dt = if self.t_last < 0.0 { 0.0 } else { (inp.t - self.t_last).clamp(0.0, 0.25) };
        self.t_last = inp.t;
        let e = self.exact.get_or_insert_with(|| Box::new(exact::Exact::new(inp.t, inp.song, &mut self.sheet)));
        // the music as far ahead of what is heard as the set reads it (else what has just played); the last channel
        // (the right in stereo) on the sheet's left half, which the camera sees on the right
        let read = inp.music.as_ref().and_then(|m| self.ahead.read(&m.file, m.ahead(e.ahead()), exact::SPAN));
        let (last, first, rate) = match &read {
            Some((l, f, r)) => (l, f, *r as f32),
            None => (&inp.samples, &inp.first, inp.rate),
        };
        let (a, _) = exact::spectra(last, rate);
        let (b, _) = if first.len() == last.len() { exact::spectra(first, rate) } else { (a, a) };
        let spec = exact::columns(&a, &b, e.tilt(inp.t));
        let (look, front, ramp) = e.step(inp.t, dt, inp.song, &spec, &mut self.sheet);
        self.look = look;
        self.front = front;
        self.exact_ramp = ramp;
    }

    unsafe fn init(&mut self, gl: &glow::Context) -> bool {
        if self.failed {
            return false;
        }
        if self.vao.is_some() {
            return true;
        }
        self.head = egui_glow::ShaderVersion::get(gl).version_declaration().to_string();
        let h = self.head.clone();
        self.progs = Progs {
            sheet: program(gl, &h, &format!("{}{}", PLACE, SHEET_VS), SHEET_FS),
            coc: program(gl, &h, &format!("{}{}", PLACE, COC_VS), COC_FS),
            fill: program(gl, &h, QUAD_VS, FILL_FS),
            blur4: program(gl, &h, QUAD_VS, BLUR4_FS),
            shrink: program(gl, &h, QUAD_VS, SHRINK_FS),
            stack: program(gl, &h, QUAD_VS, STACK_FS),
            comp: program(gl, &h, QUAD_VS, COMP_FS),
            copy: program(gl, &h, QUAD_VS, COPY_FS),
        };
        let p = &self.progs;
        if [p.sheet, p.coc, p.fill, p.blur4, p.shrink, p.stack, p.comp, p.copy].iter().any(|x| x.is_none()) {
            self.failed = true;
            return false;
        }
        // the sheet's triangles: two per tile, each tile between neighbouring rows with four corners of its own
        let mut idx: Vec<u16> = Vec::with_capacity((ROWS - 1) * (COLS - 1) * 6);
        for q in 0..(ROWS - 1) * (COLS - 1) {
            let a = (q * 4) as u16;
            idx.extend_from_slice(&[a, a + 1, a + 2, a, a + 2, a + 3]);
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
        self.rows = Some((tex(gl, COLS as i32, RING as i32, glow::RGBA32F, false), tex(gl, COLS as i32, RING as i32, glow::RGBA32F, false)));
        self.sheet.dirty = (0..RING).collect();
        // the fine bumps: a soft random normal map (128 x 128, wrapping), and the grain: even noise in every channel
        let bump = make_bump(&mut self.seed);
        let t = tex(gl, 128, 128, glow::RGBA8, true);
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, 128, 128, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&bump));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
        self.bump = Some(t);
        let mut g = Vec::with_capacity(128 * 128 * 4 * 4);
        for _ in 0..128 * 128 * 4 {
            g.extend_from_slice(&(self.rnd() * 2.0 - 1.0).to_le_bytes());
        }
        let t = tex(gl, 128, 128, glow::RGBA32F, false);
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA32F as i32, 128, 128, 0, glow::RGBA, glow::FLOAT, Some(&g));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
        self.grain = Some(t);
        self.ramp = Some(tex(gl, 512, 1, glow::RGBA16F, true));
        gl.bind_texture(glow::TEXTURE_2D, None);
        if let Ok(path) = std::env::var("BACKLINE_TERRAIN_FRAME") {
            self.dump = load_dump(&path);
            if let Some((_, _, _, _)) = &self.dump {
                let (a, b) = read_dump_rows(&path).unwrap_or_default();
                if a.len() == self.sheet.a.len() {
                    self.sheet.a = a;
                    self.sheet.b = b;
                }
            }
        }
        true
    }

    /// Draw the terrain into the spot `info` gives (its own pictures, made again when its size changes); false when
    /// the graphics can't (too old for its shaders).
    pub(crate) unsafe fn paint(&mut self, gl: &glow::Context, info: &egui::PaintCallbackInfo, slot: u8, inp: &Input) -> bool {
        if !self.init(gl) {
            return false;
        }
        if inp.t != self.t_last && self.dump.is_none() {
            self.step(inp);
        }
        self.frame += 1;
        let vp = info.viewport_in_pixels();
        let (w, h) = (vp.width_px.max(16), vp.height_px.max(16));
        if self.slots.get(&slot).map_or(true, |s| s.w != w || s.h != h) {
            if let Some(old) = self.slots.remove(&slot) {
                old.free(gl);
            }
            let t = Targets::new(gl, w, h);
            for f in [t.main.0, t.prev.0] {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(f));
                gl.disable(glow::SCISSOR_TEST);
                gl.viewport(0, 0, w, h);
                gl.clear_color(0.0, 0.0, 0.0, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }
            self.slots.insert(slot, t);
        }
        // the look (or the dumped frame's), its ramp, and the rows that changed
        let (look, zfront, oldest, rn) = match &self.dump {
            None if self.exact.is_some() && self.exact_ramp.len() == 512 * 3 => {
                gl.bind_texture(glow::TEXTURE_2D, self.ramp);
                let bytes: Vec<u8> = self.exact_ramp.chunks(3).flat_map(|c| [c[0], c[1], c[2], 1.0]).flat_map(|v| v.to_le_bytes()).collect();
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA16F as i32, 512, 1, 0, glow::RGBA, glow::FLOAT, Some(&bytes));
                (self.look, self.front, self.sheet.next - ROWS as i64, 0.0)
            }
            Some((l, ramp, front, oldest)) => {
                gl.bind_texture(glow::TEXTURE_2D, self.ramp);
                let bytes: Vec<u8> = ramp.chunks(3).flat_map(|c| [c[0], c[1], c[2], 1.0]).flat_map(|v| v.to_le_bytes()).collect();
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA16F as i32, 512, 1, 0, glow::RGBA, glow::FLOAT, Some(&bytes));
                (*l, *front, *oldest, 0.0)
            }
            None => {
                let l = self.look;
                let mut bytes = Vec::with_capacity(RAMP_N * 16);
                for i in 0..RAMP_N {
                    let c = &l[lk::RAMP + 3 * i..lk::RAMP + 3 * i + 3];
                    for v in [c[0], c[1], c[2], 1.0] {
                        bytes.extend_from_slice(&v.to_le_bytes());
                    }
                }
                gl.bind_texture(glow::TEXTURE_2D, self.ramp);
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA16F as i32, RAMP_N as i32, 1, 0, glow::RGBA, glow::FLOAT, Some(&bytes));
                (l, self.front, self.sheet.next - ROWS as i64, RAMP_N as f32)
            }
        };
        if let Some((ta, tb)) = self.rows {
            let mut d = std::mem::take(&mut self.sheet.dirty);
            d.sort_unstable();
            d.dedup();
            for s in d {
                for (t, src) in [(ta, &self.sheet.a), (tb, &self.sheet.b)] {
                    let row = &src[s * COLS..(s + 1) * COLS];
                    let bytes: Vec<u8> = row.iter().flat_map(|p| p.iter().flat_map(|v| v.to_le_bytes())).collect();
                    gl.bind_texture(glow::TEXTURE_2D, Some(t));
                    gl.tex_sub_image_2d(glow::TEXTURE_2D, 0, 0, s as i32, COLS as i32, 1, glow::RGBA, glow::FLOAT, glow::PixelUnpackData::Slice(&bytes));
                }
            }
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
        let (gx, gy) = (self.rnd(), self.rnd());
        let t = &self.slots[&slot];
        let zbase = zfront.floor();
        let front = (zfront - zbase) as f32;
        let eye = [look[lk::EYE], look[lk::EYE + 1], front - look[lk::EYE + 2]];
        let fwd = [look[lk::FWD], look[lk::FWD + 1], look[lk::FWD + 2]];
        let mvp = camera(eye, unit(fwd), look[lk::FOV], w as f32 / h as f32, 0.0);
        let c12 = [front, front + look[lk::LIFT], front + look[lk::CLIP]];
        let p = &self.progs;
        let (ta, tb) = self.rows.unwrap();
        let oldest_slot = oldest.rem_euclid(RING as i64) as i32;
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.disable(glow::CULL_FACE);
        gl.color_mask(true, true, true, true);
        gl.depth_mask(true);

        // 1. the sheet, over the fog colour
        let sp = p.sheet.unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.main.0));
        gl.viewport(0, 0, t.w, t.h);
        let fog = &look[lk::FOG..lk::FOG + 3];
        gl.clear_color(fog[0], fog[1], fog[2], 1.0);
        gl.clear_depth_f32(1.0);
        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LEQUAL);
        gl.use_program(Some(sp));
        let u = |n: &str| gl.get_uniform_location(sp, n);
        sheet_uniforms(gl, sp, ta, tb, oldest_slot, zbase as f32, &mvp, eye, &look, c12);
        gl.uniform_3_f32(u("fogp").as_ref(), 1.0 / look[lk::FOG_RANGE].max(0.01), look[lk::FOG_START], look[lk::TILE]);
        gl.uniform_1_f32(u("bumpk").as_ref(), look[lk::BUMP]);
        // a reference frame's ramp has 512 colours; ours keeps a few, each at its texel's centre
        if rn > 0.0 {
            gl.uniform_2_f32(u("rampfix").as_ref(), (rn - 1.0) / rn, 0.5 / rn);
        } else {
            gl.uniform_2_f32(u("rampfix").as_ref(), 1.0, 0.0);
        }
        gl.uniform_3_f32_slice(u("glow").as_ref(), &look[lk::GLOW..lk::GLOW + 3]);
        gl.uniform_3_f32_slice(u("fogc").as_ref(), fog);
        gl.uniform_3_f32_slice(u("sidec").as_ref(), &look[lk::SIDE..lk::SIDE + 3]);
        gl.uniform_1_i32(u("bump").as_ref(), 2);
        gl.uniform_1_i32(u("ramp").as_ref(), 3);
        gl.active_texture(glow::TEXTURE2);
        gl.bind_texture(glow::TEXTURE_2D, self.bump);
        gl.active_texture(glow::TEXTURE3);
        gl.bind_texture(glow::TEXTURE_2D, self.ramp);
        gl.bind_vertex_array(self.vao);
        gl.draw_elements(glow::TRIANGLES, ((ROWS - 1) * (COLS - 1) * 6) as i32, glow::UNSIGNED_SHORT, 0);

        // 2. depth of field: the sky's blur, then the sheet's
        let (cw, ch) = t.sizes[0];
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.coc.0));
        gl.viewport(0, 0, cw, ch);
        gl.clear(glow::DEPTH_BUFFER_BIT);
        gl.disable(glow::DEPTH_TEST);
        let fp = p.fill.unwrap();
        gl.use_program(Some(fp));
        gl.uniform_4_f32(gl.get_uniform_location(fp, "val").as_ref(), look[lk::SKY], 1.0, 1.0, 1.0);
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        gl.enable(glow::DEPTH_TEST);
        let cp = p.coc.unwrap();
        gl.use_program(Some(cp));
        sheet_uniforms(gl, cp, ta, tb, oldest_slot, zbase as f32, &mvp, eye, &look, c12);
        gl.uniform_2_f32(gl.get_uniform_location(cp, "focus").as_ref(), look[lk::FOCUS], look[lk::COC]);
        gl.draw_elements(glow::TRIANGLES, ((ROWS - 1) * (COLS - 1) * 6) as i32, glow::UNSIGNED_SHORT, 0);
        gl.disable(glow::DEPTH_TEST);
        gl.bind_vertex_array(self.vao);
        // softened five times
        let bp = p.blur4.unwrap();
        gl.use_program(Some(bp));
        gl.uniform_1_i32(gl.get_uniform_location(bp, "src").as_ref(), 0);
        gl.uniform_2_f32(gl.get_uniform_location(bp, "off").as_ref(), 0.5 / cw as f32, 0.5 / ch as f32);
        gl.active_texture(glow::TEXTURE0);
        for i in 0..5 {
            let (src, dst) = if i % 2 == 0 { (t.coc.1, t.coc2.0) } else { (t.coc2.1, t.coc.0) };
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(dst));
            gl.bind_texture(glow::TEXTURE_2D, Some(src));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
        // (five passes end in coc2)

        // 3. the picture shrunk, across then down
        let shp = p.shrink.unwrap();
        gl.use_program(Some(shp));
        let su = |n: &str| gl.get_uniform_location(shp, n);
        gl.uniform_1_i32(su("src").as_ref(), 0);
        let (sw, sh) = t.sizes[2];
        let kx = ((t.w as f32 / sw as f32).round() as i32).clamp(1, 16);
        gl.uniform_2_f32(su("step").as_ref(), 1.0 / t.w as f32, 0.0);
        gl.uniform_1_i32(su("taps").as_ref(), kx);
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.wide.0));
        gl.viewport(0, 0, sw, t.h);
        gl.bind_texture(glow::TEXTURE_2D, Some(t.main.1));
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        let ky = ((t.h as f32 / sh as f32).round() as i32).clamp(1, 16);
        gl.uniform_2_f32(su("step").as_ref(), 0.0, 1.0 / t.h as f32);
        gl.uniform_1_i32(su("taps").as_ref(), ky);
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.small.0));
        gl.viewport(0, 0, sw, sh);
        gl.bind_texture(glow::TEXTURE_2D, Some(t.wide.1));
        gl.draw_arrays(glow::TRIANGLES, 0, 3);

        // 4. the blur stack: each level blurs the one before, wider each time
        let stp = p.stack.unwrap();
        gl.use_program(Some(stp));
        gl.uniform_1_i32(gl.get_uniform_location(stp, "src").as_ref(), 0);
        let mut src = t.small.1;
        for (i, o) in [0.5f32, 1.0, 2.0, 4.0].iter().enumerate() {
            gl.uniform_2_f32(gl.get_uniform_location(stp, "off").as_ref(), o / sw as f32, o / sh as f32);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.stack[i].0));
            gl.bind_texture(glow::TEXTURE_2D, Some(src));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            src = t.stack[i].1;
        }

        // 5. the last pass, over the sharp picture
        let cmp = p.comp.unwrap();
        gl.use_program(Some(cmp));
        let cu = |n: &str| gl.get_uniform_location(cmp, n);
        for (i, (name, tx)) in [("coc", t.coc2.1), ("prev", t.prev.1), ("grain", self.grain.unwrap()), ("s0", t.stack[0].1), ("s1", t.stack[1].1), ("s2", t.stack[2].1), ("s3", t.stack[3].1)]
            .into_iter()
            .enumerate()
        {
            gl.active_texture(glow::TEXTURE0 + i as u32);
            gl.bind_texture(glow::TEXTURE_2D, Some(tx));
            gl.uniform_1_i32(cu(name).as_ref(), i as i32);
        }
        gl.uniform_1_f32(cu("blur0").as_ref(), look[lk::BLUR]);
        // a trail stretched far out of shape smears the middle of the last frame over the picture (streaks, a ghost of
        // the hills): it fades out as the stretch grows (a reference frame keeps it)
        let warp = |s: f32| s.max(1e-3).ln().abs();
        let stretch = warp(look[lk::LSCALE]).max(warp(look[lk::LSCALE + 1]));
        let fade = if self.dump.is_some() { 1.0 } else { 1.0 - smooth((stretch - 0.3) / 0.3) };
        gl.uniform_1_f32(cu("leak").as_ref(), look[lk::LEAK] * fade);
        // (the textures here run upward, so the turn goes the other way round)
        gl.uniform_2_f32(cu("rot").as_ref(), -look[lk::ROT].sin(), look[lk::ROT].cos());
        gl.uniform_2_f32(cu("lscale").as_ref(), look[lk::LSCALE], look[lk::LSCALE + 1]);
        // the grain as coarse as at 1080 lines, at a new place each frame
        gl.uniform_1_f32(cu("gscale").as_ref(), 1080.0 / t.h as f32 / 128.0);
        gl.uniform_2_f32(cu("goff").as_ref(), gx, gy);
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.main.0));
        gl.viewport(0, 0, t.w, t.h);
        gl.enable(glow::BLEND);
        gl.blend_equation(glow::FUNC_ADD);
        gl.blend_func_separate(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA, glow::ZERO, glow::ONE);
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        gl.disable(glow::BLEND);

        // 6. kept for the next frame's leak, and onto the screen inside egui's clip
        let cpy = p.copy.unwrap();
        gl.use_program(Some(cpy));
        gl.uniform_1_i32(gl.get_uniform_location(cpy, "src").as_ref(), 0);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(t.main.1));
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.prev.0));
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
        let clip = info.clip_rect_in_pixels();
        gl.enable(glow::SCISSOR_TEST);
        gl.scissor(clip.left_px, clip.from_bottom_px, clip.width_px, clip.height_px);
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        for i in 0..7 {
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

#[allow(clippy::too_many_arguments)]
unsafe fn sheet_uniforms(
    gl: &glow::Context,
    p: glow::Program,
    ta: glow::Texture,
    tb: glow::Texture,
    oldest: i32,
    zbase: f32,
    mvp: &[f32; 16],
    eye: [f32; 3],
    l: &Look,
    c12: [f32; 3],
) {
    let u = |n: &str| gl.get_uniform_location(p, n);
    gl.active_texture(glow::TEXTURE0);
    gl.bind_texture(glow::TEXTURE_2D, Some(ta));
    gl.active_texture(glow::TEXTURE1);
    gl.bind_texture(glow::TEXTURE_2D, Some(tb));
    gl.active_texture(glow::TEXTURE0);
    gl.uniform_1_i32(u("rowA").as_ref(), 0);
    gl.uniform_1_i32(u("rowB").as_ref(), 1);
    gl.uniform_1_i32(u("oldest").as_ref(), oldest);
    gl.uniform_1_f32(u("zbase").as_ref(), zbase);
    gl.uniform_matrix_4_f32_slice(u("mvp").as_ref(), false, mvp);
    gl.uniform_3_f32(u("eye").as_ref(), eye[0], eye[1], eye[2]);
    gl.uniform_2_f32(u("c8").as_ref(), l[lk::XS], l[lk::HS]);
    gl.uniform_3_f32(u("c12").as_ref(), c12[0], c12[1], c12[2]);
    gl.uniform_4_f32(u("c13").as_ref(), l[lk::SWELL], l[lk::SWELL + 1], l[lk::SWELL + 2], l[lk::SWELL + 3]);
}

/// The fine bumps: a normal map of soft random dents, 128 x 128 and wrapping (made here).
fn make_bump(seed: &mut u64) -> Vec<u8> {
    let n = 128usize;
    let mut rnd = || {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        (*seed >> 40) as f32 / 16777216.0
    };
    // a height field: random values blurred a few times (wrapping), then its slopes as a normal map
    let mut hgt: Vec<f32> = (0..n * n).map(|_| rnd()).collect();
    for _ in 0..3 {
        let src = hgt.clone();
        for y in 0..n {
            for x in 0..n {
                let s = |dx: usize, dy: usize| src[((y + dy) % n) * n + (x + dx) % n];
                hgt[y * n + x] = (s(0, 0) * 4.0 + s(1, 0) + s(n - 1, 0) + s(0, 1) + s(0, n - 1)) / 8.0;
            }
        }
    }
    let mut out = Vec::with_capacity(n * n * 4);
    for y in 0..n {
        for x in 0..n {
            let h = |dx: usize, dy: usize| hgt[((y + dy) % n) * n + (x + dx) % n];
            let dx = (h(1, 0) - h(n - 1, 0)) * 6.0;
            let dy = (h(0, 1) - h(0, n - 1)) * 6.0;
            let l = (dx * dx + dy * dy + 1.0).sqrt();
            let (nx, ny, nz) = (-dx / l, -dy / l, 1.0 / l);
            out.extend_from_slice(&[((nx * 0.5 + 0.5) * 255.0) as u8, ((ny * 0.5 + 0.5) * 255.0) as u8, ((nz * 0.5 + 0.5) * 255.0) as u8, 255]);
        }
    }
    out
}

/// A reference frame (a scratch file): its look, its full ramp, its front and oldest row.
fn load_dump(path: &str) -> Option<(Look, Vec<f32>, f64, i64)> {
    let b = std::fs::read(path).ok()?;
    if b.len() < 16 || &b[..4] != b"TRF1" {
        return None;
    }
    let i32_at = |o: usize| i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let f32_at = |o: usize| f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let oldest = i32_at(4) as i64;
    let nl = i32_at(8) as usize;
    let front = f32_at(12) as f64;
    let mut o = 16 + 2 * COLS * RING * 16;
    let mut l = [0f32; lk::N];
    for v in l.iter_mut().take(nl.min(lk::RAMP)) {
        *v = f32_at(o);
        o += 4;
    }
    let ramp: Vec<f32> = (0..1536).map(|i| f32_at(o + 4 * i)).collect();
    Some((l, ramp, front, oldest))
}

fn read_dump_rows(path: &str) -> Option<(Vec<[f32; 4]>, Vec<[f32; 4]>)> {
    let b = std::fs::read(path).ok()?;
    let f32_at = |o: usize| f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let mut a = Vec::with_capacity(COLS * RING);
    let mut bb = Vec::with_capacity(COLS * RING);
    let mut o = 16;
    for _ in 0..COLS * RING {
        a.push([f32_at(o), f32_at(o + 4), f32_at(o + 8), f32_at(o + 12)]);
        o += 16;
    }
    for _ in 0..COLS * RING {
        bb.push([f32_at(o), f32_at(o + 4), f32_at(o + 8), f32_at(o + 12)]);
        o += 16;
    }
    Some((a, bb))
}
