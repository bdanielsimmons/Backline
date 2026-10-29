//! Visuals: MilkDrop / Windows Media Player style pictures that move to the music, drawn on the graphics card, and
//! two old Windows screensavers (3D Pipes, Starfield) drawn by egui. Shown full screen, behind Now Playing, in the
//! deck's little visualizer, and as a screensaver when you step away.
//!
//! The feedback trick (most presets): every frame takes the last one, warps it a little (zoom, spin, swirl), fades
//! it, and draws the music on top. Each spot that shows visuals has its own pair of pictures to do that with.

use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use eframe::egui_glow;
use eframe::glow::{self, HasContext};
use std::sync::{Arc, Mutex};

/// A preset: its group in the menu, its name, and how it's drawn.
pub(crate) struct Preset {
    pub group: &'static str,
    pub name: &'static str,
    pub kind: Kind,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    /// A shader; `true`: it builds on the last frame (feedback).
    Gpu(&'static str, bool),
    Pipes,
    Stars,
}

pub(crate) const PRESETS: &[Preset] = &[
    Preset { group: "Milk", name: "Warp Tunnel", kind: Kind::Gpu(TUNNEL, true) },
    Preset { group: "Milk", name: "Trails", kind: Kind::Gpu(TRAILS, true) },
    Preset { group: "Milk", name: "Warp Stars", kind: Kind::Gpu(STARS, true) },
    Preset { group: "Milk", name: "Kaleidoscope", kind: Kind::Gpu(KALEIDO, true) },
    Preset { group: "Milk", name: "Plasma", kind: Kind::Gpu(PLASMA, false) },
    Preset { group: "Bars & Waves", name: "Bars", kind: Kind::Gpu(BARS, false) },
    Preset { group: "Bars & Waves", name: "Scope", kind: Kind::Gpu(SCOPE, true) },
    Preset { group: "Bars & Waves", name: "Fire Storm", kind: Kind::Gpu(FIRE, true) },
    Preset { group: "Bars & Waves", name: "Silk", kind: Kind::Gpu(SILK, true) },
    Preset { group: "Ambience", name: "Swirl", kind: Kind::Gpu(SWIRL, true) },
    Preset { group: "Ambience", name: "Water", kind: Kind::Gpu(WATER, true) },
    Preset { group: "Ambience", name: "Down the Drain", kind: Kind::Gpu(DRAIN, true) },
    Preset { group: "Ambience", name: "Tornado", kind: Kind::Gpu(TORNADO, true) },
    Preset { group: "Spikes", name: "Spike", kind: Kind::Gpu(SPIKE, true) },
    Preset { group: "Spikes", name: "Amoeba", kind: Kind::Gpu(AMOEBA, true) },
    Preset { group: "Spikes", name: "Smoke Rings", kind: Kind::Gpu(SMOKE, true) },
    Preset { group: "Screensavers", name: "3D Pipes", kind: Kind::Pipes },
    Preset { group: "Screensavers", name: "Starfield", kind: Kind::Stars },
];

pub(crate) const GROUPS: &[&str] = &["Milk", "Bars & Waves", "Ambience", "Spikes", "Screensavers"];

pub(crate) fn find(name: &str) -> usize {
    PRESETS.iter().position(|p| p.name.eq_ignore_ascii_case(name)).unwrap_or(0)
}

/// What the music is doing this frame, for the shaders.
#[derive(Clone, Copy)]
pub(crate) struct Frame {
    pub t: f32,
    pub travel: f32,
    pub bass: f32,
    pub mid: f32,
    pub high: f32,
    pub wave: [f32; 64],
    pub spec: [f32; 32],
    pub peak: [f32; 32],
}

impl Frame {
    /// From the player's analysis: bands and peaks (low to high, 0..1) and the waveform (-1..1).
    pub(crate) fn from_audio(t: f32, travel: f32, bands: &[f32], peaks: &[f32], wave: &[f32]) -> Self {
        let pick = |src: &[f32], n: usize| -> Vec<f32> {
            (0..n).map(|i| if src.is_empty() { 0.0 } else { src[(i * src.len() / n).min(src.len() - 1)] }).collect()
        };
        let avg = |a: f32, b: f32| -> f32 {
            let n = bands.len();
            if n == 0 {
                return 0.0;
            }
            let (i, j) = (((a * n as f32) as usize).min(n - 1), ((b * n as f32) as usize).clamp(1, n));
            bands[i..j.max(i + 1)].iter().sum::<f32>() / (j.max(i + 1) - i) as f32
        };
        let mut f = Frame {
            t,
            travel,
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

// ------------------------------------------------------------------------------------------------ the GPU side
const HEAD: &str = r#"
#ifdef GL_ES
precision highp float;
#endif
in vec2 v; out vec4 o;
uniform sampler2D prev; uniform vec2 res; uniform float t, bass, mid, high, travel;
uniform float wave[64]; uniform float spec[32]; uniform float peak[32];
float W(float x){ x = clamp(x, 0., 1.) * 63.; int i = int(x); return mix(wave[i], wave[min(i + 1, 63)], fract(x)); }
float S(float x){ x = clamp(x, 0., 1.) * 31.; int i = int(x); return mix(spec[i], spec[min(i + 1, 31)], fract(x)); }
vec3 pal(float x){ return .5 + .5 * cos(6.2831 * (x + vec3(0., .33, .67))); }
float asp(){ return res.x / res.y; }
vec2 P(){ vec2 p = (v - .5) * 2.; p.x *= asp(); return p; }
vec3 back(vec2 p){ p.x /= asp(); return texture(prev, p * .5 + .5).rgb; }
float hash(vec2 q){ return fract(sin(dot(q, vec2(127.1, 311.7))) * 43758.5453); }
mat2 rot(float a){ return mat2(cos(a), -sin(a), sin(a), cos(a)); }
vec3 fade(vec3 c, float k){ return max(c * k - 1.5 / 255., 0.); }
"#;

const VERT: &str = r#"
out vec2 v;
void main(){ vec2 a = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2)); v = a; gl_Position = vec4(a * 2. - 1., 0., 1.); }
"#;

const COPY: &str = r#"
#ifdef GL_ES
precision highp float;
#endif
in vec2 v; out vec4 o; uniform sampler2D prev;
void main(){ o = vec4(texture(prev, v).rgb, 1.); }
"#;

const TUNNEL: &str = r#"void main(){ vec2 p = P();
  vec3 c = fade(back(rot(.012 + .03 * mid) * p * (.965 - .045 * bass)), .962);
  float ang = atan(p.y, p.x) / 6.2831 + .5; float r = .32 + .2 * W(abs(ang * 2. - 1.)) + .12 * bass;
  c += pal(t * .08 + ang) * exp(-abs(length(p) - r) * 85.) * (.6 + bass);
  o = vec4(c, 1.); }"#;

const TRAILS: &str = r#"void main(){ vec2 p = P();
  vec2 q = p * .985 + .012 * vec2(sin(p.y * 3. + t), cos(p.x * 3. + t * 1.3));
  vec3 c = fade(back(q), .955);
  float y = W(v.x) * .55; c += pal(t * .06 + v.x * .5) * exp(-abs(p.y - y) * 65.) * (.55 + mid);
  o = vec4(c, 1.); }"#;

const STARS: &str = r#"void main(){ vec2 p = P();
  vec3 c = max(back(p * (.95 - .05 * bass)) * .86 - 2. / 255., 0.);
  for (int i = 0; i < 4; i++) {
    float d = fract(travel * .12 + float(i) * .25), sc = mix(18., .7, d);
    vec2 uv = p * sc + float(i) * 7.3, id = floor(uv), f = fract(uv) - .5;
    float h = hash(id); vec2 off = (vec2(hash(id + 1.3), hash(id + 2.7)) - .5) * .6;
    float s = smoothstep(.07, 0., length(f - off)) * d * step(.55, h);
    c += s * mix(vec3(.75, .85, 1.), pal(h), .45) * (1.2 + high);
  }
  o = vec4(c, 1.); }"#;

const KALEIDO: &str = r#"void main(){ vec2 p = rot(t * .1) * P();
  float r = length(p), a = atan(p.y, p.x), seg = 6.2831 / 6.; a = abs(mod(a, seg) - seg * .5);
  vec2 k = r * vec2(cos(a), sin(a));
  float f = sin(k.x * 9. + t * 2.) * sin(k.y * 9. - t * 1.5) + sin(r * 12. - t * 3. - bass * 5.);
  vec3 col = pal(f * .3 + t * .05 + r * .5) * smoothstep(.25, 1., abs(f)) * (.35 + bass);
  o = vec4(fade(back(p * 1.012), .9) + col * .4, 1.); }"#;

const PLASMA: &str = r#"void main(){ vec2 p = P();
  float f = sin(p.x * 3. + t) + sin(p.y * 4. - t * 1.2) + sin((p.x + p.y) * 3. + t * .7) + sin(length(p) * 6. - t * 2. - bass * 6.);
  o = vec4(pal(f * .25 + t * .05) * (.5 + .6 * bass), 1.); }"#;

// Bars & Waves
const BARS: &str = r#"void main(){ float n = 32., i = floor(v.x * n), fx = fract(v.x * n);
  float h = spec[int(i)] * .62, pk = peak[int(i)] * .62, base = .28, gap = step(.12, fx) * step(fx, .88);
  vec3 c = vec3(.01, .01, .03);
  if (v.y > base) { float y = v.y - base;
    if (y < h) c = mix(vec3(.1, .85, .35), vec3(1., .85, .2), y / .62) * gap;
    if (abs(y - pk - .006) < .006) c = vec3(.95) * gap;
  } else { float y = (base - v.y) * 1.6;
    if (y < h) c = mix(vec3(.1, .85, .35), vec3(1., .85, .2), y / .62) * gap * .3 * (1. - y * 1.4); }
  o = vec4(c, 1.); }"#;

const SCOPE: &str = r#"void main(){ vec2 p = P();
  vec3 c = fade(back(p), .72);
  float d = abs(p.y - W(v.x) * .7);
  c += vec3(.3, 1., .5) * exp(-d * 120.) * 1.2 + vec3(.1, .5, .3) * exp(-d * 22.) * .35;
  o = vec4(c, 1.); }"#;

const FIRE: &str = r#"void main(){
  float sc = res.y / 720.;
  vec2 q = v - vec2((hash(v * res + t) - .5) * 4. * sc, (3. + hash(v.yx * res - t) * 3.) * sc) / res;
  vec3 c = texture(prev, q).rgb * vec3(.99, .962, .9) - .6 / 255.;
  if (v.y < .05) { float h = S(v.x) * 1.6 * (.55 + .7 * hash(vec2(floor(v.x * 90.), floor(t * 20.))));
    c = max(c, vec3(1., .78, .35) * h * (1. - v.y * 10.)); }
  o = vec4(max(c, 0.), 1.); }"#;

const SILK: &str = r#"void main(){ vec2 p = P();
  vec3 c = fade(back(vec2(p.x * .995, p.y * .99 - .004)), .955) * vec3(1., .8, .86);
  float top = -1. + S(v.x) * 1.7 + .05 * sin(v.x * 9. + t);
  if (p.y < top) { float k = (top - p.y);
    c = mix(vec3(.55, 1., 1.), vec3(.05, .55, .85), smoothstep(0., .9, k)); }
  c += vec3(1., .25, .55) * exp(-abs(p.y - top) * 60.) * .8;
  o = vec4(c, 1.); }"#;

// Ambience
const SWIRL: &str = r#"void main(){ vec2 p = P(); float r = length(p);
  vec3 c = fade(back(rot(.05 * (1.2 - r) + .02 * mid) * p * .99), .962);
  float ang = atan(p.y, p.x) / 6.2831 + .5; float rr = .25 + .15 * W(abs(ang * 2. - 1.)) + .1 * bass;
  c += pal(t * .05 + r) * exp(-abs(r - rr) * 70.) * (.5 + bass);
  o = vec4(c, 1.); }"#;

const WATER: &str = r#"void main(){ vec2 p = P();
  vec2 q = p * .997 + .007 * vec2(sin(p.y * 10. + t * 2.), cos(p.x * 9. + t * 1.7));
  vec3 c = fade(back(q), .968) * vec3(.97, .99, 1.);
  float d = abs(p.y - W(v.x) * .35 * (1. + bass));
  c += vec3(.35, .7, 1.) * exp(-d * 90.) * (.5 + mid) + vec3(.8, .95, 1.) * exp(-d * 300.) * .5;
  o = vec4(c, 1.); }"#;

const DRAIN: &str = r#"void main(){ vec2 p = P();
  vec3 c = fade(back(rot(.06 + .05 * bass) * p * 1.03), .955);
  float ang = atan(p.y, p.x) / 6.2831 + .5, r = length(p);
  float rr = .95 + .12 * W(abs(ang * 2. - 1.));
  c += pal(t * .07 + ang * 2.) * exp(-abs(r - rr) * 60.) * (.6 + bass);
  o = vec4(c, 1.); }"#;

const TORNADO: &str = r#"void main(){ vec2 p = P(); float r = length(p);
  vec3 c = fade(back(rot(.09 * exp(-r * 1.4)) * p * .985 + vec2(0., -.008)), .955);
  for (int i = 0; i < 24; i++) { float k = float(i) / 24.;
    vec2 at = vec2(cos(k * 6.2831 + t), sin(k * 6.2831 + t)) * (.3 + .3 * S(k));
    c += pal(t * .05 + k) * smoothstep(.035, 0., length(p - at)) * .8; }
  o = vec4(c, 1.); }"#;

// Spikes
const SPIKE: &str = r#"void main(){ vec2 p = P(); float r = length(p), a = atan(p.y, p.x) / 6.2831 + .5;
  vec3 c = fade(back(p * .97), .85);
  float len = .12 + .9 * S(abs(a * 2. - 1.)), ray = smoothstep(.35, 0., abs(fract(a * 48.) - .5));
  c += pal(t * .1 + r * .6) * ray * step(r, len) * (.4 + bass);
  o = vec4(c, 1.); }"#;

const AMOEBA: &str = r#"void main(){ vec2 p = P(); float r = length(p), a = atan(p.y, p.x);
  float R = .38 + .3 * S(abs(fract(a / 6.2831 + .5) * 2. - 1.)) + .05 * sin(a * 5. + t * 2.) + .1 * bass;
  vec3 c = fade(back(p * .99), .8);
  if (r < R) c = mix(c, pal(t * .04 + r) * (.35 + .4 * r / R), .6);
  c += pal(t * .04 + .5) * exp(-abs(r - R) * 50.);
  o = vec4(c, 1.); }"#;

const SMOKE: &str = r#"void main(){ vec2 p = P();
  vec2 q = p * .995 + vec2(.004 * sin(p.y * 5. + t), -.006);
  vec3 c = fade(back(q), .97) * vec3(.99, .98, 1.);
  float slot = floor(t * 2.);
  for (int i = 0; i < 5; i++) { vec2 seed = vec2(float(i), slot);
    vec2 at = (vec2(hash(seed), hash(seed + 3.1)) * 2. - 1.) * vec2(asp(), 1.) * .7;
    float rr = .08 + .3 * bass * hash(seed + 7.7);
    c += pal(float(i) * .2 + t * .05) * exp(-abs(length(p - at) - rr) * 80.) * .5; }
  o = vec4(c, 1.); }"#;

/// One spot's pair of pictures (feedback: draw the next from the last, then swap).
struct Slot {
    tex: [glow::Texture; 2],
    fbo: [glow::Framebuffer; 2],
    w: i32,
    h: i32,
    cur: usize,
    preset: usize,
}

#[derive(Default)]
pub(crate) struct Gl {
    head: String,
    progs: Vec<Option<glow::Program>>,
    copy: Option<glow::Program>,
    vao: Option<glow::VertexArray>,
    slots: std::collections::HashMap<u8, Slot>,
    failed: bool,
}

/// A program: the shared vertex shader and `frag`, each after the GL version line (`frag` brings its own header).
unsafe fn program(gl: &glow::Context, version: &str, frag: &str) -> Option<glow::Program> {
    let p = gl.create_program().ok()?;
    let mut shaders = Vec::new();
    for (kind, src) in
        [(glow::VERTEX_SHADER, format!("{}{}", version, VERT)), (glow::FRAGMENT_SHADER, format!("{}{}", version, frag))]
    {
        let s = gl.create_shader(kind).ok()?;
        gl.shader_source(s, &src);
        gl.compile_shader(s);
        if !gl.get_shader_compile_status(s) {
            crate::api::log(&format!("visuals: shader didn't compile: {}", gl.get_shader_info_log(s)));
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
        crate::api::log(&format!("visuals: shader didn't link: {}", gl.get_program_info_log(p)));
        return None;
    }
    Some(p)
}

impl Gl {
    unsafe fn draw(&mut self, gl: &glow::Context, info: &egui::PaintCallbackInfo, slot_id: u8, preset: usize, f: &Frame) {
        if self.failed {
            return;
        }
        if self.vao.is_none() {
            self.head = egui_glow::ShaderVersion::get(gl).version_declaration().to_string();
            self.vao = gl.create_vertex_array().ok();
            self.copy = program(gl, &self.head, COPY);
            self.progs = (0..PRESETS.len()).map(|_| None).collect();
            if self.vao.is_none() || self.copy.is_none() {
                self.failed = true;
                return;
            }
        }
        let Kind::Gpu(src, feedback) = PRESETS[preset].kind else { return };
        if self.progs[preset].is_none() {
            match program(gl, &self.head, &format!("{}{}", HEAD, src)) {
                Some(p) => self.progs[preset] = Some(p),
                None => {
                    self.failed = true;
                    return;
                }
            }
        }
        let prog = self.progs[preset].unwrap();
        let vp = info.viewport_in_pixels();
        let (w, h) = (vp.width_px.max(2), vp.height_px.max(2));
        // the slot's pictures, made again when its size or preset changes (a new preset starts from black)
        let fresh = self.slots.get(&slot_id).map_or(true, |s| s.w != w || s.h != h || s.preset != preset);
        if fresh {
            if let Some(old) = self.slots.remove(&slot_id) {
                for i in 0..2 {
                    gl.delete_texture(old.tex[i]);
                    gl.delete_framebuffer(old.fbo[i]);
                }
            }
            let mut tex = Vec::new();
            let mut fbo = Vec::new();
            for _ in 0..2 {
                let (Ok(t), Ok(b)) = (gl.create_texture(), gl.create_framebuffer()) else {
                    self.failed = true;
                    return;
                };
                gl.bind_texture(glow::TEXTURE_2D, Some(t));
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, w, h, 0, glow::RGBA, glow::UNSIGNED_BYTE, None);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(b));
                gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(t), 0);
                gl.disable(glow::SCISSOR_TEST);
                gl.viewport(0, 0, w, h);
                gl.clear_color(0.0, 0.0, 0.0, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
                tex.push(t);
                fbo.push(b);
            }
            self.slots.insert(slot_id, Slot { tex: [tex[0], tex[1]], fbo: [fbo[0], fbo[1]], w, h, cur: 0, preset });
        }
        let s = self.slots.get_mut(&slot_id).unwrap();
        gl.disable(glow::BLEND);
        gl.bind_vertex_array(self.vao);
        gl.use_program(Some(prog));
        let u = |n: &str| gl.get_uniform_location(prog, n);
        gl.uniform_2_f32(u("res").as_ref(), w as f32, h as f32);
        gl.uniform_1_f32(u("t").as_ref(), f.t);
        gl.uniform_1_f32(u("travel").as_ref(), f.travel);
        gl.uniform_1_f32(u("bass").as_ref(), f.bass);
        gl.uniform_1_f32(u("mid").as_ref(), f.mid);
        gl.uniform_1_f32(u("high").as_ref(), f.high);
        gl.uniform_1_f32_slice(u("wave").as_ref(), &f.wave);
        gl.uniform_1_f32_slice(u("spec").as_ref(), &f.spec);
        gl.uniform_1_f32_slice(u("peak").as_ref(), &f.peak);
        gl.uniform_1_i32(u("prev").as_ref(), 0);
        gl.active_texture(glow::TEXTURE0);
        // the next picture, from the last (or straight from the music)
        gl.bind_texture(glow::TEXTURE_2D, Some(s.tex[s.cur]));
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(s.fbo[1 - s.cur]));
        gl.disable(glow::SCISSOR_TEST);
        gl.viewport(0, 0, w, h);
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        let _ = feedback;
        s.cur = 1 - s.cur;
        // onto the screen, inside egui's clip
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
        let clip = info.clip_rect_in_pixels();
        gl.enable(glow::SCISSOR_TEST);
        gl.scissor(clip.left_px, clip.from_bottom_px, clip.width_px, clip.height_px);
        gl.use_program(self.copy);
        gl.uniform_1_i32(gl.get_uniform_location(self.copy.unwrap(), "prev").as_ref(), 0);
        gl.bind_texture(glow::TEXTURE_2D, Some(s.tex[s.cur]));
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.bind_vertex_array(None);
        gl.use_program(None);
        gl.enable(glow::BLEND);
    }
}

/// Draw a GPU preset into `rect` (slot: 0 full screen, 1 Now Playing, 2 the deck, 3 the deck's playlist bar).
pub(crate) fn paint_gpu(p: &egui::Painter, rect: Rect, slot: u8, preset: usize, f: Frame, state: &Arc<Mutex<Gl>>) {
    let state = state.clone();
    let cb = egui_glow::CallbackFn::new(move |info, painter| {
        if let Ok(mut g) = state.lock() {
            unsafe { g.draw(painter.gl(), &info, slot, preset, &f) };
        }
    });
    p.add(egui::PaintCallback { rect, callback: Arc::new(cb) });
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
                let col = (self.rnd() * PIPE_COLS.len() as f32) as usize % PIPE_COLS.len();
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
        let scale = r.height().min(r.width() * 0.62) * 1.1;
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
        for &(a, b, col, joint) in &self.segs {
            let ((pa, fa, za), (pb, fb, zb)) = (proj(a), proj(b));
            if a != b {
                items.push(((za + zb) / 2.0, It::Tube(pa, 0.3 * fa, pb, 0.3 * fb, col)));
            }
            if joint {
                items.push((za - 0.35, It::Ball(pa, 0.3 * fa * 1.3, col)));
            }
        }
        for &(c, _, col, _) in &self.heads {
            let (pc, fc, zc) = proj(c);
            items.push((zc - 0.3, It::Ball(pc, 0.3 * fc, col)));
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
        let bass = self.vis_cache.map_or(0.0, |c| c.1.bass);
        self.vis_travel += dt * (0.4 + bass * 1.6);
        let f = Frame::from_audio(now as f32, self.vis_travel, &self.bands, &self.peaks, &self.viz_wave);
        self.vis_cache = Some((nr, f, dt));
        (f, dt)
    }

    /// The chosen visuals in `r` (slot: 0 full screen, 1 Now Playing, 2 the deck, 3 the playlist bar's).
    pub(crate) fn vis_paint(&mut self, ui: &egui::Ui, r: Rect, slot: u8) {
        let (f, dt) = self.vis_frame(ui.ctx());
        let preset = self.vis_preset.min(PRESETS.len() - 1);
        let p = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
        match PRESETS[preset].kind {
            Kind::Gpu(..) => paint_gpu(&p, r, slot, preset, f, &self.vis_gl),
            Kind::Pipes => self.vis_pipes.draw(&p, r, dt, f.bass),
            Kind::Stars => self.vis_stars.draw(&p, r, dt, f.bass),
        }
        ui.ctx().request_repaint();
    }

    /// Every preset, by group (for right-click menus).
    pub(crate) fn vis_menu(&mut self, ui: &mut egui::Ui) {
        for g in GROUPS {
            ui.menu_button(*g, |ui| {
                for (i, p) in PRESETS.iter().enumerate().filter(|(_, p)| p.group == *g) {
                    let mark = if i == self.vis_preset { "* " } else { "  " };
                    if crate::menu_item(ui, &format!("{}{}", mark, p.name)) {
                        self.vis_preset = i;
                        self.dirty = true;
                        ui.close_menu();
                    }
                }
            });
        }
        ui.separator();
        let idle = [
            (0u32, "Screensaver: off"),
            (2, "Screensaver after 2 minutes"),
            (5, "Screensaver after 5 minutes"),
            (10, "Screensaver after 10 minutes"),
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

    /// Full screen visuals on or off (`saver`: started by being idle, so any touch ends it).
    pub(crate) fn vis_set_full(&mut self, on: bool, saver: bool) {
        if on == self.vis_full {
            return;
        }
        self.vis_full = on;
        self.vis_saver = on && saver;
        if on {
            self.vis_was_fs = self.ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            self.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            self.vis_shown_at = self.ctx.input(|i| i.time);
        } else if !self.vis_was_fs {
            self.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        self.vis_idle_since = std::time::Instant::now();
    }

    /// Going idle while music plays starts the screensaver; any key, click or mouse move counts as not idle.
    pub(crate) fn vis_idle_check(&mut self, ctx: &egui::Context) {
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
        let playing = self.cur.is_some() && !self.paused && !self.stopped;
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
        let n = PRESETS.len();
        let (left, right, esc) = ui.input(|i| {
            (i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight), i.key_pressed(egui::Key::Escape))
        });
        if esc || resp.double_clicked() {
            self.vis_set_full(false, false);
            return;
        }
        if left {
            self.vis_preset = (self.vis_preset + n - 1) % n;
            self.vis_ui_at = now;
            self.dirty = true;
        }
        if right {
            self.vis_preset = (self.vis_preset + 1) % n;
            self.vis_ui_at = now;
            self.dirty = true;
        }
        resp.context_menu(|ui| self.vis_menu(ui));
        if self.vis_saver || now - self.vis_ui_at > 3.0 {
            return;
        }
        // the song, top left
        let p = ui.painter();
        if let Some(t) = self.cur_track() {
            let line = format!("{}  -  {}", t.artists_text(), t.title);
            crate::ptext(p, r.min + Vec2::new(28.0, 30.0), egui::Align::Min, &line, 1.6, Color32::from_white_alpha(230));
        }
        // the preset and arrows, bottom middle
        let name = format!("{}: {}", PRESETS[self.vis_preset].group, PRESETS[self.vis_preset].name);
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
                self.vis_preset = (self.vis_preset + by) % n;
                self.dirty = true;
            }
        }
        crate::ptext(
            p,
            Pos2::new(r.center().x, r.max.y - 14.0),
            egui::Align::Center,
            "Arrows change it  ·  right-click for all  ·  Esc or double-click to leave",
            0.9,
            Color32::from_white_alpha(140),
        );
        let _ = acts;
    }
}
