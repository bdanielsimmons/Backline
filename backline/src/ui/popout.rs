//! The visuals in a little window of their own (the button beside VISUALS in Now Playing, or the visuals' menu).
//! Popping them out sends the main window back to the library; opening Now Playing (or the visuals full screen) brings
//! them back into it, where they stay until popped out again. The window floats over other windows (the pin), moves
//! when dragged anywhere, and resizes from its edges and corners between MIN and MAX. Let go right at a screen's edge
//! (any screen, wherever Backline's own window is), it settles GAP in from it, so it tucks into a corner easily;
//! anywhere else, or with Shift held as it is let go, it stays exactly where it was put. Its controls show only while
//! the mouse moves over it, in Now Playing's look: the song's card at the top left and the visuals' name at the top
//! right; along the bottom the song's own controls in the middle (shuffle, previous, play, next, repeat; Space plays
//! and pauses too), the visuals' arrows at the left, the pin and close at the right. Close, a double-click or Esc puts
//! the visuals back: Now Playing opens with them, the main window brought forward.
//!
//! It runs on a thread of its own (Windows only, for now), with its own window, graphics and copy of egui, so neither
//! window ever waits on the other: each shows its frames at its own screen's refresh, and dragging one never holds up
//! the other. All it needs from the app is in `Pop`, which the app keeps up to date at each of its own frames; it
//! works out the music itself from the player's latest samples. It is moved and resized by hand (the cursor followed,
//! then `win::settle`), not by the system's own move: that would stop its drawing while it lasted, and its snapping
//! would square the window's corners.
// (only Windows has the window, for now)
#![cfg_attr(not(windows), allow(dead_code))]

use crate::player::{Ctl, Viz};
use crate::visuals::{self, Frame, PRESETS};
use crate::App;
use crate::Tip;
use eframe::egui::{self, Align, Color32, CursorIcon, Pos2, Rect, Sense, Vec2};
use std::sync::{Arc, Mutex, MutexGuard};

/// Can the visuals pop out here? (Windows only, for now.)
pub(crate) const CAN: bool = cfg!(windows);
/// Its own pictures in the visuals (0 full screen, 1 Now Playing, 2 the deck, 3 the playlist bar).
const SLOT: u8 = 4;
/// Its title.
const TITLE: &str = "Backline Visuals";
/// How big it opens the first time, and the least and the most it resizes to (points).
const START: Vec2 = Vec2::new(400.0, 225.0);
const MIN: Vec2 = Vec2::new(240.0, 135.0);
const MAX: Vec2 = Vec2::new(960.0, 540.0);
/// Let go with an edge this near a screen's edge, or past it (points), it settles GAP in from it; anywhere else (or
/// holding Shift as you let go) it stays exactly where it was put.
const SNAP: f32 = 20.0;
const GAP: f32 = 12.0;
/// The band along its edges that resizes it (points).
const EDGE: f32 = 6.0;
/// Its corners, as Windows 11 rounds them (the controls keep clear of them).
const ROUND: f32 = 8.0;

/// What the window and the app share.
pub(crate) struct Pop {
    /// what it shows of the app (the app keeps it up to date)
    now: Now,
    /// the app has put the visuals back, or is closing: the window's thread ends
    quit: bool,
    /// its thread has ended, its graphics let go of
    done: bool,
    // ---- from the window, for the app
    /// kept on top of other windows
    pin: bool,
    /// closed: its close button, a double-click, Esc, or the system's close (or it couldn't open)
    closed: bool,
    /// visuals picked with its arrows
    picked: Option<usize>,
    /// the song's controls pressed, for the app to carry out
    asks: Vec<Ask>,
    /// where it is (screen pixels: left, top, width, height)
    at: Option<[i32; 4]>,
    /// (Windows) the window, once it is made: the app wakes its thread through it
    hwnd: isize,
}

/// What the window shows of the app.
#[derive(Clone)]
struct Now {
    /// the visuals picked
    preset: usize,
    /// the song (a number that changes with it) and its name, and whether it is playing
    song: u64,
    title: String,
    artist: String,
    playing: bool,
    /// a song is going (playing or loading, not paused or stopped): the middle button shows pause
    going: bool,
    /// shuffle on, and repeat: 0 off, 1 all, 2 one
    shuffle: bool,
    repeat: u8,
    /// Now Playing's caption colours: soft black on near-white (else soft white on near-black)
    cap_dark: bool,
    /// the analysis' bar count and sensitivity (as the app's visualizers)
    bars: usize,
    gain: f32,
}

/// The song's controls in the window, carried out by the app (`pop_frame`).
#[derive(Clone, Copy)]
enum Ask {
    Shuffle,
    Prev,
    Toggle,
    Next,
    Repeat,
}

fn lock(pop: &Mutex<Pop>) -> MutexGuard<'_, Pop> {
    pop.lock().unwrap_or_else(|e| e.into_inner())
}

/// The window's own, on its thread: the music as it hears it, the screensavers' state and its graphics' pictures, and
/// when the mouse last moved over it.
struct Own {
    bands: Vec<f32>,
    peaks: Vec<f32>,
    wave: Vec<f32>,
    agc: visuals::Agc,
    travel: f32,
    bass: f32,
    last: f64,
    pipes: visuals::Pipes,
    stars: visuals::Stars,
    gl: Arc<Mutex<visuals::Gl>>,
    /// when the mouse last moved over it (the controls show for a while after)
    ui_at: f64,
}

impl Own {
    fn new() -> Self {
        Own {
            bands: Vec::new(),
            peaks: Vec::new(),
            wave: Vec::new(),
            agc: Default::default(),
            travel: 0.0,
            bass: 0.0,
            last: 0.0,
            pipes: Default::default(),
            stars: Default::default(),
            gl: Default::default(),
            ui_at: f64::NEG_INFINITY,
        }
    }

    /// The music for the frame at time `t`, and the time since the last (as `App::vis_frame`, from the player's
    /// latest samples).
    fn music(&mut self, now: &Now, viz: &Mutex<Viz>, t: f64) -> (Frame, f32) {
        let (samples, rate) = match viz.lock() {
            Ok(v) if now.playing => {
                let n = v.samples.len().min(2048);
                (v.samples[v.samples.len() - n..].to_vec(), v.rate as f32)
            }
            _ => (Vec::new(), 44100.0),
        };
        crate::analyse(&mut self.bands, &mut self.peaks, &mut self.wave, &samples, rate, now.bars, now.gain);
        let dt = ((t - self.last) as f32).clamp(0.0, 0.1);
        self.last = t;
        self.travel += dt * (0.4 + self.bass * 1.6);
        let mut f = Frame::from_audio(t as f32, self.travel, &self.bands, &self.peaks, &self.wave);
        self.agc.apply(&mut f, dt);
        self.bass = f.bass;
        (f, dt)
    }
}

/// What a frame of the window asks for (carried out once it is drawn).
#[derive(Default)]
struct Asked {
    /// the song's controls pressed, for the app
    asks: Vec<Ask>,
    /// visuals picked with its arrows
    picked: Option<usize>,
    /// the pin pressed
    pin: bool,
    /// closed: its close button, a double-click or Esc
    close: bool,
    /// held to move it (None), or to resize it from these edges (left, top, right, bottom)
    grab: Option<Option<[bool; 4]>>,
}

impl Asked {
    /// Another preset, `by` steps on from `preset` (or from the one just picked).
    fn step(&mut self, preset: usize, by: usize) {
        let n = PRESETS.len();
        self.picked = Some((self.picked.unwrap_or(preset).min(n - 1) + by) % n);
    }
}

/// One frame of the window, at time `t`: the visuals (when it can be seen), and its controls while the mouse moves
/// over it.
#[allow(clippy::too_many_arguments)]
fn ui(ctx: &egui::Context, own: &mut Own, now: &Now, pin: bool, seen: bool, t: f64, viz: &Mutex<Viz>, ctl: &Ctl) -> Asked {
    let mut a = Asked::default();
    let (esc, left, right, space) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::Escape),
            i.key_pressed(egui::Key::ArrowLeft),
            i.key_pressed(egui::Key::ArrowRight),
            i.key_pressed(egui::Key::Space),
        )
    });
    if esc {
        a.close = true;
        return a;
    }
    if left {
        a.step(now.preset, PRESETS.len() - 1);
    }
    if right {
        a.step(now.preset, 1);
    }
    if space {
        a.asks.push(Ask::Toggle);
    }
    egui::CentralPanel::default().frame(egui::Frame::none().fill(Color32::BLACK)).show(ctx, |ui| {
        let r = ui.max_rect();
        let id = ui.id().with("visuals_window");
        // held anywhere and dragged, it moves (the edges and the buttons, added after, come first where they are)
        let body = ui.interact(r, id, Sense::click_and_drag());
        if seen {
            let (f, dt) = own.music(now, viz, t);
            let preset = visuals::resolve(now.preset.min(PRESETS.len() - 1), f.t);
            let p = ui.painter().with_clip_rect(r);
            visuals::paint_preset(&p, r, SLOT, preset, f, dt, t, &own.gl, &mut own.pipes, &mut own.stars, |world, look| {
                visuals::hear(viz, ctl, now.song, t, world, look)
            });
        }
        if body.drag_started() {
            a.grab = Some(None);
        }
        if body.double_clicked() {
            a.close = true;
        }
        if let Some(sides) = edges(ui, r, id) {
            a.grab = Some(Some(sides));
        }
        // the controls, while the mouse moves over it (and a moment after)
        let over = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| r.contains(p));
        if over && ctx.input(|i| i.pointer.delta() != Vec2::ZERO) {
            own.ui_at = t;
        }
        let fade = if over { (1.0 - ((t - own.ui_at) as f32 - 2.5) / 0.4).clamp(0.0, 1.0) } else { 0.0 };
        // (a test copy's switch BACKLINE_POP_UI=1: the controls always shown, for pictures of them)
        let fade = if std::env::var_os("BACKLINE_POP_UI").is_some() { 1.0 } else { fade };
        if fade > 0.0 {
            controls(ui, r, id, now, pin, &mut a, fade);
        }
    });
    a
}

/// The bands along its edges and corners that resize it: the edges (left, top, right, bottom) of the one held down.
fn edges(ui: &egui::Ui, r: Rect, id: egui::Id) -> Option<[bool; 4]> {
    let (e, c) = (EDGE, EDGE * 2.0);
    let (l, t, rt, b) = (r.min.x, r.min.y, r.max.x, r.max.y);
    let (no, yes) = (false, true);
    let bands = [
        (Rect::from_min_max(Pos2::new(l + c, t), Pos2::new(rt - c, t + e)), [no, yes, no, no], CursorIcon::ResizeVertical),
        (Rect::from_min_max(Pos2::new(l + c, b - e), Pos2::new(rt - c, b)), [no, no, no, yes], CursorIcon::ResizeVertical),
        (Rect::from_min_max(Pos2::new(l, t + c), Pos2::new(l + e, b - c)), [yes, no, no, no], CursorIcon::ResizeHorizontal),
        (Rect::from_min_max(Pos2::new(rt - e, t + c), Pos2::new(rt, b - c)), [no, no, yes, no], CursorIcon::ResizeHorizontal),
        (Rect::from_min_max(Pos2::new(l, t), Pos2::new(l + c, t + c)), [yes, yes, no, no], CursorIcon::ResizeNwSe),
        (Rect::from_min_max(Pos2::new(rt - c, b - c), Pos2::new(rt, b)), [no, no, yes, yes], CursorIcon::ResizeNwSe),
        (Rect::from_min_max(Pos2::new(rt - c, t), Pos2::new(rt, t + c)), [no, yes, yes, no], CursorIcon::ResizeNeSw),
        (Rect::from_min_max(Pos2::new(l, b - c), Pos2::new(l + c, b)), [yes, no, no, yes], CursorIcon::ResizeNeSw),
    ];
    let mut held = None;
    for (k, (band, sides, cursor)) in bands.into_iter().enumerate() {
        let h = ui.interact(band, id.with(("edge", k)), Sense::drag()).on_hover_cursor(cursor);
        if h.is_pointer_button_down_on() {
            held = Some(sides);
        }
    }
    held
}

/// The controls in Now Playing's look, faded by `fade` (0..1): the song's card at the top left, the visuals' name at
/// the top right; along the bottom the song's controls in the middle (shuffle, previous, play, next, repeat), the
/// visuals' arrows at the left, and the pin (lit while it stays on top) and close at the right. A small window keeps
/// what matters most: the song's controls, then close, then shuffle and repeat, the pin, the arrows.
fn controls(ui: &egui::Ui, r: Rect, id: egui::Id, now: &Now, pin: bool, a: &mut Asked, fade: f32) {
    let mut p = ui.painter().with_clip_rect(r);
    p.set_opacity(fade);
    // (in from the edges, clear of the rounded corners)
    let m = ROUND + 2.0;
    let (pw, ph, gap, side) = (34.0, 28.0, 5.0, 8.0);
    let ink = crate::pal().ink;
    let (lit, dark) = (Color32::from_rgb(238, 238, 234), Color32::from_rgb(24, 24, 24));
    // ---- along the bottom: what fits, most important first
    let y = r.max.y - m - ph;
    let play_d = 34.0;
    let five = 4.0 * pw + play_d + 4.0 * gap;
    let three = 2.0 * pw + play_d + 2.0 * gap;
    let pair = 2.0 * pw + gap;
    // (the song's controls in the middle; beside them, the room each side has)
    let room = |w: f32| (r.center().x - w / 2.0 - side - (r.min.x + m), (r.max.x - m) - (r.center().x + w / 2.0 + side));
    let fits = |w: f32, left: f32, right: f32| {
        let (l, rt) = room(w);
        w <= r.width() - 2.0 * m && l >= left && rt >= right
    };
    let (all, arrows, pin_fits, close) = if fits(five, pair, pair) {
        (true, true, true, true)
    } else if fits(five, 0.0, pair) {
        (true, false, true, true)
    } else if fits(five, 0.0, pw) {
        (true, false, false, true)
    } else if fits(three, 0.0, pw) {
        (false, false, false, true)
    } else {
        (false, false, false, false)
    };
    let w = if all { five } else { three };
    let x0 = r.center().x - w / 2.0;
    let pill = |x: f32| Rect::from_min_size(Pos2::new(x, y), Vec2::new(pw, ph));
    let press = |rect: Rect, key: &str, on: bool| -> egui::Response {
        let h = ui.interact(rect, id.with(key), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        crate::raised_h(&p, rect, h.is_pointer_button_down_on(), h.hovered());
        // (on: a near-white pill, its symbol in near-black)
        if on {
            p.rect_filled(rect, ph / 2.0, lit);
        }
        h
    };
    let icon = |rect: Rect, name: &str, on: bool| {
        crate::vicon::named(&p, Rect::from_center_size(rect.center(), Vec2::splat(15.0)), name, if on { dark } else { ink });
    };
    let mut x = x0;
    if all {
        let sh = press(pill(x), "shuffle", now.shuffle).tip(if now.shuffle { "Shuffle: on" } else { "Shuffle: off" });
        icon(pill(x), "shuffle", now.shuffle);
        if sh.clicked() {
            a.asks.push(Ask::Shuffle);
        }
        x += pw + gap;
    }
    let prev = press(pill(x), "song_prev", false).tip("Previous");
    icon(pill(x), "prev", false);
    if prev.clicked() {
        a.asks.push(Ask::Prev);
    }
    x += pw + gap;
    let play = Rect::from_min_size(Pos2::new(x, y + (ph - play_d) / 2.0), Vec2::splat(play_d));
    let go = ui.interact(play, id.with("song_play"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let go = go.tip(if now.going { "Pause  (Space)" } else { "Play  (Space)" });
    p.circle_filled(play.center(), play_d / 2.0 + if go.hovered() { 1.0 } else { 0.0 }, lit);
    let name = if now.going { "pause" } else { "play" };
    let off = if name == "play" { 1.5 } else { 0.0 };
    crate::vicon::named(&p, Rect::from_center_size(play.center() + Vec2::new(off, 0.0), Vec2::splat(play_d * 0.42)), name, dark);
    if go.clicked() {
        a.asks.push(Ask::Toggle);
    }
    x += play_d + gap;
    let next = press(pill(x), "song_next", false).tip("Next");
    icon(pill(x), "skip", false);
    if next.clicked() {
        a.asks.push(Ask::Next);
    }
    x += pw + gap;
    if all {
        let on = now.repeat != 0;
        let tip = match now.repeat {
            0 => "Repeat: off",
            1 => "Repeat: all",
            _ => "Repeat: one",
        };
        let rep = press(pill(x), "repeat", on).tip(tip);
        icon(pill(x), "loop", on);
        // (repeat one: a small 1 on it, as in Now Playing)
        if now.repeat == 2 {
            let c = pill(x).right_top() + Vec2::new(-6.0, 5.0);
            p.circle_filled(c, 6.0, crate::pal().bar_txt);
            crate::font::keep_caps(|| crate::ptext(&p, c, Align::Center, "1", 1.0, crate::pal().lcd));
        }
        if rep.clicked() {
            a.asks.push(Ask::Repeat);
        }
    }
    if arrows {
        let (lr, rr) = (pill(r.min.x + m), pill(r.min.x + m + pw + gap));
        if press(lr, "vis_prev", false).tip("Previous visual  (Left)").clicked() {
            a.step(now.preset, PRESETS.len() - 1);
        }
        chevron(&p, lr.center(), true, ink);
        if press(rr, "vis_next", false).tip("Next visual  (Right)").clicked() {
            a.step(now.preset, 1);
        }
        chevron(&p, rr.center(), false, ink);
    }
    if close {
        let close_r = pill(r.max.x - m - pw);
        let c = press(close_r, "close", false).tip("Close  (Esc)");
        icon(close_r, "close", false);
        if c.clicked() {
            a.close = true;
        }
    }
    if pin_fits {
        let pin_r = pill(r.max.x - m - 2.0 * pw - gap);
        let tip = if pin { "Always on top: on" } else { "Always on top: off" };
        let h = press(pin_r, "pin", pin).tip(tip);
        icon(pin_r, "pin", pin);
        if h.clicked() {
            a.pin = true;
        }
    }
    // ---- along the top: the visuals' name at the right (when there is room), the song's card at the left
    let name = visuals::name_of(a.picked.unwrap_or(now.preset).min(PRESETS.len() - 1), ui.ctx().input(|i| i.time) as f32);
    let name_w = (crate::text_w(&name, 1.0) + 24.0).min(r.width() * 0.45);
    // (only with room beside it for the song's card)
    let show_name = r.width() - 2.0 * m - name_w - gap >= 170.0;
    if show_name {
        let label = Rect::from_min_size(Pos2::new(r.max.x - m - name_w, r.min.y + m), Vec2::new(name_w, ph));
        p.rect_filled(label, ph / 2.0, Color32::from_black_alpha(170));
        crate::ptext_fit(&p, label.center(), Align::Center, &name, 1.0, name_w - 20.0, Color32::from_white_alpha(235));
    }
    // the song's card: Now Playing's caption, soft white on near-black (or soft black on near-white)
    if !now.title.is_empty() {
        let (ink, plate) = if now.cap_dark {
            (Color32::from_rgb(24, 24, 24), Color32::from_rgba_unmultiplied(238, 238, 234, 235))
        } else {
            (Color32::from_rgb(238, 238, 234), Color32::from_rgba_unmultiplied(16, 16, 16, 225))
        };
        let room = r.width() - 2.0 * m - 22.0 - if show_name { name_w + gap } else { 0.0 };
        let tw = crate::text_w(&now.title, 1.5).max(crate::text_w(&now.artist, 1.05)).min(room).min(320.0);
        let card = Rect::from_min_size(r.min + Vec2::splat(m), Vec2::new(tw + 22.0, 44.0));
        p.rect_filled(card, 8.0, plate);
        crate::ptext_fit(&p, Pos2::new(card.min.x + 11.0, card.min.y + 15.0), Align::Min, &now.title, 1.5, tw, ink);
        crate::ptext_fit(&p, Pos2::new(card.min.x + 11.0, card.min.y + 31.0), Align::Min, &now.artist, 1.05, tw, ink.gamma_multiply(0.85));
    }
}

/// The button that pops the visuals out (beside VISUALS in Now Playing): a small screen in the corner of a big one,
/// drawn as the other icons are (in the pixel look, in pixels).
pub(crate) fn button(ui: &mut egui::Ui, col: Color32) -> egui::Response {
    const PIXELS: [&str; 9] = [
        "###########",
        "#.........#",
        "#.........#",
        "#.........#",
        "#....######",
        "#....######",
        "#....######",
        "#....######",
        "###########",
    ];
    let sc = crate::font::ui_scale();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(44.0 * sc, crate::bh()), Sense::click());
    let down = resp.is_pointer_button_down_on();
    crate::raised_h(ui.painter(), rect, down, resp.hovered());
    let c = rect.center() + Vec2::new(0.0, if down { 1.0 } else { 0.0 });
    let p = ui.painter();
    if crate::skin::style() != 0 || crate::winamp_ui::chrome_on() {
        let s = 21.0 * sc;
        let at = |x: f32, y: f32| c + Vec2::new((x - 0.5) * s, (y - 0.5) * s);
        p.rect_stroke(Rect::from_min_max(at(0.06, 0.16), at(0.94, 0.84)), 0.08 * s, egui::Stroke::new((s * 0.11).max(1.3), col));
        p.rect_filled(Rect::from_min_max(at(0.5, 0.48), at(0.84, 0.74)), 0.04 * s, col);
    } else {
        let px = 2.0 * sc;
        let size = Vec2::new(PIXELS[0].len() as f32 * px, PIXELS.len() as f32 * px);
        crate::icons::pixmap(p, (c - size / 2.0).round(), px, &PIXELS, col);
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// An arrow head at `c`, pointing left or right, drawn as the vector icons are.
fn chevron(p: &egui::Painter, c: Pos2, left: bool, col: Color32) {
    let k: f32 = if left { 1.0 } else { -1.0 };
    let pts = vec![c + Vec2::new(k * 3.0, -6.0), c + Vec2::new(-k * 3.0, 0.0), c + Vec2::new(k * 3.0, 6.0)];
    p.add(egui::Shape::line(pts, egui::Stroke::new(2.0f32, col)));
}

/// Where a window from `lo` to `hi` (one direction, screen pixels) settles on a screen's work area `wlo..whi`: `gap`
/// in from an edge it was let go within `snap` of, or past; else where it is.
fn place(lo: i32, hi: i32, wlo: i32, whi: i32, snap: i32, gap: i32) -> i32 {
    if lo < wlo + snap {
        wlo + gap
    } else if hi > whi - snap {
        whi - gap - (hi - lo)
    } else {
        lo
    }
}

/// A window `r` (left, top, right, bottom) resized by its `sides` (left, top, right, bottom) dragged `dx`, `dy`, the
/// other sides staying put, its width and height kept between `min` and `max`.
fn sized(r: [i32; 4], sides: [bool; 4], dx: i32, dy: i32, min: [i32; 2], max: [i32; 2]) -> [i32; 4] {
    let [mut l, mut t, mut rt, mut b] = r;
    if sides[0] {
        l = (l + dx).clamp(rt - max[0], rt - min[0]);
    }
    if sides[2] {
        rt = (rt + dx).clamp(l + min[0], l + max[0]);
    }
    if sides[1] {
        t = (t + dy).clamp(b - max[1], b - min[1]);
    }
    if sides[3] {
        b = (b + dy).clamp(t + min[1], t + max[1]);
    }
    [l, t, rt, b]
}

/// A frame's time, from the clock: one refresh after the last frame's while frames come once a refresh (so motion moves
/// evenly, whenever the thread happens to wake), drawn gently towards the clock; else the clock's own.
fn steady(last: f64, clock: f64, period: f64) -> f64 {
    let next = last + period;
    if (clock - next).abs() < period * 0.5 {
        next + (clock - next) * 0.05
    } else {
        clock
    }
}

// ------------------------------------------------------------------------------------------------ in the app
impl App {
    /// The visuals into a window of their own (the main window goes back to the library), or back.
    pub(crate) fn pop_toggle(&mut self) {
        if self.pop.is_some() {
            self.pop_close();
            return;
        }
        if !CAN {
            return;
        }
        self.art_view = false;
        if self.vis_full {
            self.vis_set_full(false, false);
        }
        // (a test copy's switch BACKLINE_POP_SIZE=<w>x<h>: that size, in points)
        let asked = std::env::var("BACKLINE_POP_SIZE").ok().and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some(Vec2::new(w.parse().ok()?, h.parse().ok()?))
        });
        let at = win::start(self.pop_px, asked, crate::onscreen::main_window());
        let pop = Arc::new(Mutex::new(Pop {
            now: self.pop_now(),
            quit: false,
            done: false,
            pin: self.pop_pin,
            closed: false,
            picked: None,
            asks: Vec::new(),
            at: None,
            hwnd: 0,
        }));
        // (its own copy of egui reads in the same fonts)
        win::open(pop.clone(), self.player.viz.clone(), self.player.ctl.clone(), self.ctx.clone(), crate::fonts_now(), at);
        self.pop = Some(pop);
    }

    /// The window closes (its thread ends) and the visuals go back into Now Playing.
    fn pop_close(&mut self) {
        if let Some(pop) = self.pop.take() {
            let mut s = lock(&pop);
            s.quit = true;
            win::wake(s.hwnd);
        }
        self.vis_np = true;
        self.dirty = true;
    }

    /// Backline is closing: the window's thread is told to end, and given a moment to let go of its graphics.
    pub(crate) fn pop_shut(&mut self) {
        let Some(pop) = self.pop.take() else { return };
        {
            let mut s = lock(&pop);
            s.quit = true;
            win::wake(s.hwnd);
        }
        let asked = std::time::Instant::now();
        while !lock(&pop).done && asked.elapsed() < std::time::Duration::from_millis(500) {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// Visuals in Now Playing in place of the cover (not while they are in their own window).
    pub(crate) fn vis_in_np(&self) -> bool {
        self.vis_np && self.pop.is_none()
    }

    /// What the window shows of the app right now.
    fn pop_now(&self) -> Now {
        let t = self.cur_track();
        Now {
            preset: self.vis_preset,
            song: t.as_ref().map_or(0, |t| t.id as u64),
            title: t.as_ref().map(|t| t.title.clone()).unwrap_or_default(),
            artist: t.map(|t| t.artists_text()).unwrap_or_default(),
            playing: self.cur.is_some() && !self.paused && !self.stopped && !self.buffering,
            going: self.cur.is_some() && !self.paused && !self.stopped,
            shuffle: self.shuffle,
            repeat: match self.repeat {
                crate::Repeat::Off => 0,
                crate::Repeat::All => 1,
                crate::Repeat::One => 2,
            },
            cap_dark: self.cap_dark,
            bars: self.viz[0].n.max(self.viz[1].n).clamp(8, 96),
            gain: self.viz[0].gain.max(self.viz[1].gain),
        }
    }

    /// Every frame while the window is open: what it changed and asked for, and what it needs from the app.
    pub(crate) fn pop_frame(&mut self, ctx: &egui::Context) {
        let Some(pop) = self.pop.clone() else { return };
        let (asks, closed) = {
            let mut s = lock(&pop);
            if s.at.is_some() && s.at != self.pop_px {
                self.pop_px = s.at;
                self.dirty = true;
            }
            if s.pin != self.pop_pin {
                self.pop_pin = s.pin;
                self.dirty = true;
            }
            if let Some(p) = s.picked.take() {
                self.vis_preset = p;
                self.dirty = true;
            }
            (std::mem::take(&mut s.asks), s.closed)
        };
        for a in asks {
            self.apply(match a {
                Ask::Shuffle => crate::Action::Shuffle,
                Ask::Prev => crate::Action::Prev,
                Ask::Toggle => crate::Action::Toggle,
                Ask::Next => crate::Action::Next,
                Ask::Repeat => crate::Action::Repeat,
            });
        }
        // closed, or Now Playing (or the visuals full screen) opened: the visuals go back into Now Playing, to stay
        if closed || self.art_view || (self.vis_full && !self.vis_saver) {
            // (closed from the window itself: Now Playing opens, and the main window comes forward)
            if closed && !self.art_view && !self.vis_full {
                self.art_view = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            self.pop_close();
            return;
        }
        let now = self.pop_now();
        lock(&pop).now = now;
    }
}

// ------------------------------------------------------------------------------------------------ Windows
#[cfg(windows)]
mod win {
    use super::{lock, steady, ui, Asked, Own, Pop, GAP, MAX, MIN, SNAP, START, TITLE};
    use crate::player::{Ctl, Viz};
    use eframe::egui::{self, CursorIcon, Pos2, Vec2};
    use eframe::{egui_glow, glow};
    use std::sync::{Arc, Mutex};
    use windows::core::{PCSTR, PCWSTR};
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplaySettingsW, GetDC, GetMonitorInfoW, GetStockObject, MonitorFromRect, MonitorFromWindow, ReleaseDC, BLACK_BRUSH, DEVMODEW,
        ENUM_CURRENT_SETTINGS, HBRUSH, HDC, MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTONULL, MONITOR_DEFAULTTOPRIMARY,
    };
    use windows::Win32::Graphics::OpenGL::{
        wglCreateContext, wglDeleteContext, wglGetProcAddress, wglMakeCurrent, ChoosePixelFormat, SetPixelFormat, SwapBuffers, HGLRC,
        PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW, PFD_MAIN_PLANE, PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA, PIXELFORMATDESCRIPTOR,
    };
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryA};
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyState, ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VIRTUAL_KEY, VK_CONTROL,
        VK_ESCAPE, VK_LEFT, VK_MENU, VK_RIGHT, VK_SHIFT, VK_SPACE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetCursorPos, GetWindowRect, IsIconic, LoadCursorW,
        MsgWaitForMultipleObjects, PeekMessageW, PostMessageW, RegisterClassExW, SetCursor, SetWindowPos, ShowWindow, TranslateMessage,
        CS_OWNDC, HCURSOR, HMENU, HTCLIENT, HWND_NOTOPMOST, HWND_TOPMOST, IDC_ARROW, IDC_HAND, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE,
        IDC_SIZEWE, MSG, PM_REMOVE, QS_ALLINPUT, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_SHOWNOACTIVATE, WINDOW_EX_STYLE,
        WM_APP, WM_CAPTURECHANGED, WM_CLOSE, WM_DPICHANGED, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
        WM_SETCURSOR, WM_SETFOCUS, WNDCLASSEXW, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    };

    /// (from the common controls' list, not worth taking all of them in for)
    const WM_MOUSELEAVE: u32 = 0x02A3;

    /// Its window class (made once a run).
    const CLASS: &str = "BacklineVisuals";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// Where the window opens (screen pixels: left, top, width, height): where it was last, if that is still on a
    /// screen, else in the bottom right corner of the screen the main window (`main`) is on, GAP in; `asked` (points)
    /// its size instead.
    pub(super) fn start(last: Option<[i32; 4]>, asked: Option<Vec2>, main: isize) -> [i32; 4] {
        let k = if main != 0 { unsafe { GetDpiForWindow(HWND(main)) }.max(48) as f32 / 96.0 } else { 1.0 };
        let px = |v: Vec2| [(v.x * k).round() as i32, (v.y * k).round() as i32];
        let (lo, hi) = (px(MIN), px(MAX));
        let size = asked.map(px).or(last.map(|r| [r[2], r[3]])).unwrap_or(px(START));
        let size = [size[0].clamp(lo[0], hi[0]), size[1].clamp(lo[1], hi[1])];
        if crate::hidden_test_copy() {
            // (a test copy's, like its main window: off every screen)
            return [-11000, 40, size[0], size[1]];
        }
        if let Some(r) = last {
            let rc = RECT { left: r[0], top: r[1], right: r[0] + size[0], bottom: r[1] + size[1] };
            if unsafe { MonitorFromRect(&rc, MONITOR_DEFAULTTONULL) }.0 != 0 {
                return [r[0], r[1], size[0], size[1]];
            }
        }
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let m = unsafe { MonitorFromWindow(HWND(main), MONITOR_DEFAULTTOPRIMARY) };
        let wa = if unsafe { GetMonitorInfoW(m, &mut mi) }.as_bool() {
            mi.rcWork
        } else {
            RECT { left: 0, top: 0, right: 1280, bottom: 720 }
        };
        let gap = (GAP * k).round() as i32;
        [wa.right - gap - size[0], wa.bottom - gap - size[1], size[0], size[1]]
    }

    /// The window's thread, started: it makes the window and draws it until it is closed (from the window, or by the
    /// app). If it can't, the app hears it closed.
    pub(super) fn open(pop: Arc<Mutex<Pop>>, viz: Arc<Mutex<Viz>>, ctl: Arc<Ctl>, main: egui::Context, fonts: egui::FontDefinitions, at: [i32; 4]) {
        let p = pop.clone();
        let spawned = std::thread::Builder::new().name("visuals window".into()).spawn(move || {
            if let Err(e) = run(&p, &viz, &ctl, &main, fonts, at) {
                crate::api::log(&format!("visuals window: {}", e));
                lock(&p).closed = true;
                main.request_repaint();
            }
            lock(&p).done = true;
        });
        if spawned.is_err() {
            let mut s = lock(&pop);
            s.closed = true;
            s.done = true;
        }
    }

    /// Wakes the window's thread (to see that it is to close).
    pub(super) fn wake(h: isize) {
        if h != 0 {
            unsafe { PostMessageW(HWND(h), WM_APP, WPARAM(0), LPARAM(0)) };
        }
    }

    /// The window and its graphics, let go of in order as it closes.
    struct Window {
        hwnd: HWND,
        dc: HDC,
        glrc: HGLRC,
        painter: Option<egui_glow::Painter>,
    }

    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                if let Some(mut p) = self.painter.take() {
                    p.destroy();
                }
                if self.glrc.0 != 0 {
                    wglMakeCurrent(HDC(0), HGLRC(0));
                    wglDeleteContext(self.glrc);
                }
                if self.dc.0 != 0 {
                    ReleaseDC(self.hwnd, self.dc);
                }
                DestroyWindow(self.hwnd);
            }
        }
    }

    /// Moved or resized by hand: a move (None) or a resize from these sides (left, top, right, bottom), and where the
    /// cursor and the window were as the button went down (screen pixels).
    struct Grab {
        sides: Option<[bool; 4]>,
        cursor: POINT,
        rect: RECT,
    }

    /// The window made and drawn, once a refresh while it can be seen (only looked at now and then while it can't),
    /// until it is closed.
    fn run(pop: &Mutex<Pop>, viz: &Mutex<Viz>, ctl: &Ctl, main: &egui::Context, fonts: egui::FontDefinitions, at: [i32; 4]) -> Result<(), String> {
        let pin = lock(pop).pin;
        let mut w = make(at, pin)?;
        lock(pop).hwnd = w.hwnd.0;
        // rounded corners, as Windows 11's own windows (nothing on older Windows)
        let round = DWMWCP_ROUND;
        let size = std::mem::size_of_val(&round) as u32;
        let _ = unsafe { DwmSetWindowAttribute(w.hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &round as *const _ as *const _, size) };
        let gl = unsafe { graphics(&mut w) }?;
        w.painter = Some(egui_glow::Painter::new(gl, "", None, true).map_err(|e| e.to_string())?);
        let ctx = egui::Context::default();
        ctx.set_fonts(fonts);
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let _glyphs = crate::font::own(&ctx);
        unsafe { ShowWindow(w.hwnd, SW_SHOWNOACTIVATE) };
        let mut own = Own::new();
        let started = std::time::Instant::now();
        let mut grab: Option<Grab> = None;
        let (mut looked, mut seen, mut period) = (f64::NEG_INFINITY, true, 1.0 / 60.0);
        let (mut t, mut cursor, mut put) = (0.0f64, CursorIcon::Default, [0i32; 4]);
        let (mut log, mut shots, mut swapped) = (Log::default(), Shots::new(), 0.0f64);
        loop {
            unsafe { pump() };
            let (now, pin, quit) = {
                let s = lock(pop);
                (s.now.clone(), s.pin, s.quit)
            };
            if quit {
                return Ok(());
            }
            // the system's close (Alt+F4)
            if with(|i| std::mem::take(&mut i.close)) == Some(true) {
                closed(pop, main);
                return Ok(());
            }
            let clock = started.elapsed().as_secs_f64();
            let ppp = unsafe { GetDpiForWindow(w.hwnd) }.max(48) as f32 / 96.0;
            let (down, press, moved) = with(|i| {
                i.ppp = ppp;
                (i.down, i.press, i.dpi_rect.take())
            })
            .unwrap_or_default();
            // moved onto a screen of another scale: the size the system suggests (and its place, unless it is held)
            if let Some(r) = moved {
                let flags = SWP_NOZORDER | SWP_NOACTIVATE;
                match &grab {
                    None => unsafe { SetWindowPos(w.hwnd, HWND(0), r.left, r.top, r.right - r.left, r.bottom - r.top, flags) },
                    Some(g) if g.sides.is_none() => unsafe { SetWindowPos(w.hwnd, HWND(0), 0, 0, r.right - r.left, r.bottom - r.top, flags | SWP_NOMOVE) },
                    _ => Default::default(),
                };
            }
            // held, it follows the cursor; let go after a move right at a screen's edge, it settles in from it (not
            // while Shift is held)
            if let Some(g) = &grab {
                if down {
                    follow(w.hwnd, g, ppp);
                } else {
                    if g.sides.is_none() && !shift() {
                        settle(w.hwnd, (SNAP * ppp).round() as i32, (GAP * ppp).round() as i32);
                    }
                    grab = None;
                }
            }
            // where it is, for the app to keep (once it is put down)
            if grab.is_none() {
                let mut rc = RECT::default();
                if unsafe { GetWindowRect(w.hwnd, &mut rc) }.as_bool() {
                    let r = [rc.left, rc.top, rc.right - rc.left, rc.bottom - rc.top];
                    if r != put {
                        put = r;
                        lock(pop).at = Some(r);
                    }
                }
            }
            // can it be seen? Not minimized, or covered by the windows in front of it (looked at twice a second at most)
            if clock - looked >= 0.5 {
                looked = clock;
                seen = !unsafe { IsIconic(w.hwnd) }.as_bool() && !crate::onscreen::covered_window(w.hwnd);
                period = period_of(w.hwnd);
            }
            let mut rc = RECT::default();
            let size = if unsafe { GetClientRect(w.hwnd, &mut rc) }.as_bool() {
                [(rc.right - rc.left).max(0) as u32, (rc.bottom - rc.top).max(0) as u32]
            } else {
                [0, 0]
            };
            if (!seen && grab.is_none()) || size[0] == 0 || size[1] == 0 {
                // no one can see it: nothing drawn, only a look now and then (or as soon as something happens to it)
                with(|i| i.events.clear());
                log.last = 0.0;
                unsafe { MsgWaitForMultipleObjects(None, false, 250, QS_ALLINPUT) };
                continue;
            }
            t = steady(t, clock, period);
            crate::font::set_own_ppp(ppp);
            let (events, focused) = with(|i| (std::mem::take(&mut i.events), i.focused)).unwrap_or_default();
            let Some(painter) = w.painter.as_mut() else { return Ok(()) };
            let mut raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(size[0] as f32, size[1] as f32) / ppp)),
                max_texture_side: Some(painter.max_texture_side()),
                time: Some(t),
                predicted_dt: period as f32,
                modifiers: mods(),
                events,
                focused,
                ..Default::default()
            };
            if let Some(v) = raw.viewports.get_mut(&egui::ViewportId::ROOT) {
                v.native_pixels_per_point = Some(ppp);
                v.focused = Some(focused);
                v.minimized = Some(false);
            }
            let mut asked = Asked::default();
            let out = ctx.run(raw, |ctx| asked = ui(ctx, &mut own, &now, pin, seen, t, viz, ctl));
            if asked.close {
                closed(pop, main);
                return Ok(());
            }
            let egui::FullOutput { platform_output, textures_delta, shapes, pixels_per_point, .. } = out;
            let prims = ctx.tessellate(shapes, pixels_per_point);
            painter.clear(size, [0.0, 0.0, 0.0, 1.0]);
            painter.paint_and_update_textures(size, pixels_per_point, &prims, &textures_delta);
            shots.take(painter, size, clock);
            unsafe { SwapBuffers(w.dc) };
            // (off every screen, or with graphics that don't wait for the refresh, it keeps to a frame a refresh itself)
            let early = period - (started.elapsed().as_secs_f64() - swapped);
            if early > period * 0.5 {
                std::thread::sleep(std::time::Duration::from_secs_f64(early));
            }
            swapped = started.elapsed().as_secs_f64();
            log.count(clock, period);
            // the cursor egui asks for (held, the resize's own)
            let want = match &grab {
                Some(Grab { sides: Some(s), .. }) => resize_cursor(*s),
                _ => platform_output.cursor_icon,
            };
            if want != cursor {
                cursor = want;
                let c = unsafe { LoadCursorW(HINSTANCE(0), cursor_of(want)) }.unwrap_or_default();
                if with(|i| {
                    i.cursor = c;
                    i.inside
                }) == Some(true)
                    || grab.is_some()
                {
                    unsafe { SetCursor(c) };
                }
            }
            // what it asked for
            if asked.pin {
                let on = !pin;
                lock(pop).pin = on;
                let after = if on { HWND_TOPMOST } else { HWND_NOTOPMOST };
                unsafe { SetWindowPos(w.hwnd, after, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) };
            }
            if let Some(sides) = asked.grab {
                if grab.is_none() && down {
                    grab = Some(Grab { sides, cursor: press.0, rect: press.1 });
                }
            }
            if !asked.asks.is_empty() || asked.picked.is_some() {
                let mut s = lock(pop);
                s.asks.extend(asked.asks);
                if let Some(p) = asked.picked {
                    s.picked = Some(p);
                    s.now.preset = p;
                }
                drop(s);
                main.request_repaint();
            }
        }
    }

    /// Closed from the window: the app hears it at once.
    fn closed(pop: &Mutex<Pop>, main: &egui::Context) {
        lock(pop).closed = true;
        main.request_repaint();
    }

    /// The window, made (not shown yet): a plain one floating over others (when pinned), with no taskbar button.
    fn make(at: [i32; 4], pin: bool) -> Result<Window, String> {
        unsafe {
            let inst = GetModuleHandleW(PCWSTR::null()).map_err(|e| e.to_string())?;
            let class = wide(CLASS);
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_OWNDC,
                lpfnWndProc: Some(proc_),
                hInstance: inst,
                hCursor: LoadCursorW(HINSTANCE(0), IDC_ARROW).unwrap_or_default(),
                hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                lpszClassName: PCWSTR(class.as_ptr()),
                ..Default::default()
            };
            // (made once a run: the second time it is already there)
            RegisterClassExW(&wc);
            let title = wide(TITLE);
            let ex = WS_EX_TOOLWINDOW | if pin { WS_EX_TOPMOST } else { WINDOW_EX_STYLE(0) };
            let style = WS_POPUP | WS_CLIPSIBLINGS | WS_CLIPCHILDREN;
            let h = CreateWindowExW(
                ex,
                PCWSTR(class.as_ptr()),
                PCWSTR(title.as_ptr()),
                style,
                at[0],
                at[1],
                at[2],
                at[3],
                HWND(0),
                HMENU(0),
                inst,
                None,
            );
            if h.0 == 0 {
                return Err("its window couldn't be made".into());
            }
            Ok(Window { hwnd: h, dc: GetDC(h), glrc: HGLRC(0), painter: None })
        }
    }

    /// The window's graphics: an OpenGL context of its own, current on this thread, that waits for the screen's
    /// refresh as it shows each frame.
    unsafe fn graphics(w: &mut Window) -> Result<Arc<glow::Context>, String> {
        let pfd = PIXELFORMATDESCRIPTOR {
            nSize: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            cAlphaBits: 8,
            cDepthBits: 24,
            cStencilBits: 8,
            iLayerType: PFD_MAIN_PLANE,
            ..Default::default()
        };
        let f = ChoosePixelFormat(w.dc, &pfd);
        if f == 0 || !SetPixelFormat(w.dc, f, &pfd).as_bool() {
            return Err("no pixel format for its graphics".into());
        }
        w.glrc = wglCreateContext(w.dc).map_err(|e| format!("no graphics: {}", e))?;
        if !wglMakeCurrent(w.dc, w.glrc).as_bool() {
            return Err("its graphics couldn't be used".into());
        }
        let gl32 = LoadLibraryA(PCSTR(b"opengl32.dll\0".as_ptr())).ok();
        let gl = glow::Context::from_loader_function_cstr(|name| {
            let name = PCSTR(name.as_ptr() as *const u8);
            let f = wglGetProcAddress(name).map_or(0, |f| f as usize);
            // (OpenGL 1.1's own come from opengl32.dll; some drivers answer 1, 2, 3 or -1 for them)
            if f > 3 && f != usize::MAX {
                return f as *const std::ffi::c_void;
            }
            gl32.and_then(|m| GetProcAddress(m, name)).map_or(std::ptr::null(), |f| f as usize as *const std::ffi::c_void)
        });
        if let Some(f) = wglGetProcAddress(PCSTR(b"wglSwapIntervalEXT\0".as_ptr())) {
            let set: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(f);
            set(1);
        }
        // (a test copy says what it draws with, and whether it waits for the refresh)
        if crate::hidden_test_copy() {
            use glow::HasContext;
            let wait = wglGetProcAddress(PCSTR(b"wglGetSwapIntervalEXT\0".as_ptr())).map_or(-1, |f| {
                let get: unsafe extern "system" fn() -> i32 = std::mem::transmute(f);
                get()
            });
            let (version, renderer) = (gl.get_parameter_string(glow::VERSION), gl.get_parameter_string(glow::RENDERER));
            crate::api::log(&format!("visuals window: OpenGL {} on {}, swap interval {}", version, renderer, wait));
        }
        Ok(Arc::new(gl))
    }

    /// The window's messages, waiting, handed to their window.
    unsafe fn pump() {
        let mut m = MSG::default();
        while PeekMessageW(&mut m, HWND(0), 0, 0, PM_REMOVE).as_bool() {
            TranslateMessage(&m);
            DispatchMessageW(&m);
        }
    }

    /// What the window's messages bring, on its thread: egui's events since the last frame, and the rest.
    struct Input {
        events: Vec<egui::Event>,
        /// pixels a point (the mouse is told in points)
        ppp: f32,
        /// where the mouse is (points), whether it is over the window, and whether the window has the keyboard
        pos: Pos2,
        inside: bool,
        focused: bool,
        /// the left button is down; where the cursor and the window were as it went down (screen pixels)
        down: bool,
        press: (POINT, RECT),
        /// the cursor shown over it
        cursor: HCURSOR,
        /// the system's close asked for (Alt+F4)
        close: bool,
        /// moved onto a screen of another scale: the size and place the system suggests
        dpi_rect: Option<RECT>,
    }

    thread_local! {
        static INPUT: std::cell::RefCell<Input> = std::cell::RefCell::new(Input {
            events: Vec::new(),
            ppp: 1.0,
            pos: Pos2::ZERO,
            inside: false,
            focused: false,
            down: false,
            press: Default::default(),
            cursor: HCURSOR(0),
            close: false,
            dpi_rect: None,
        });
    }

    /// `f` with the window's input (None while it is in use, or as the thread ends).
    fn with<R>(f: impl FnOnce(&mut Input) -> R) -> Option<R> {
        INPUT.try_with(|i| i.try_borrow_mut().ok().map(|mut i| f(&mut i))).ok().flatten()
    }

    /// The keys held with the others (Shift, Ctrl, Alt).
    fn mods() -> egui::Modifiers {
        let held = |k: VIRTUAL_KEY| unsafe { GetKeyState(k.0 as i32) } < 0;
        let ctrl = held(VK_CONTROL);
        egui::Modifiers { alt: held(VK_MENU), ctrl, shift: held(VK_SHIFT), mac_cmd: false, command: ctrl }
    }

    /// The left button, down or up at `pos`.
    fn button(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: mods() }
    }

    /// The window's messages: the mouse and the keys it uses, as egui's events; its cursor; a close; a new scale.
    unsafe extern "system" fn proc_(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        // (the mouse, in points: client pixels, signed, over the pixels a point)
        let at = || {
            let k = with(|i| i.ppp).unwrap_or(1.0);
            Pos2::new((lp.0 & 0xFFFF) as u16 as i16 as f32 / k, ((lp.0 >> 16) & 0xFFFF) as u16 as i16 as f32 / k)
        };
        match msg {
            WM_MOUSEMOVE => {
                let pos = at();
                let first = with(|i| {
                    i.events.push(egui::Event::PointerMoved(pos));
                    i.pos = pos;
                    !std::mem::replace(&mut i.inside, true)
                });
                // (told when it leaves)
                if first == Some(true) {
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: h,
                        dwHoverTime: 0,
                    };
                    TrackMouseEvent(&mut tme);
                }
                LRESULT(0)
            }
            WM_MOUSELEAVE => {
                with(|i| {
                    i.inside = false;
                    i.events.push(egui::Event::PointerGone);
                });
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                let pos = at();
                let (mut c, mut r) = (POINT::default(), RECT::default());
                GetCursorPos(&mut c);
                GetWindowRect(h, &mut r);
                // (held, it hears the mouse even outside itself; taken first, so a capture it had already can't let go
                // of this press)
                SetCapture(h);
                with(|i| {
                    i.down = true;
                    i.press = (c, r);
                    i.pos = pos;
                    i.events.push(button(pos, true));
                });
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                let pos = at();
                with(|i| {
                    if std::mem::take(&mut i.down) {
                        i.events.push(button(pos, false));
                    }
                });
                ReleaseCapture();
                LRESULT(0)
            }
            WM_CAPTURECHANGED => {
                // (the mouse taken away while held: let go)
                with(|i| {
                    if std::mem::take(&mut i.down) {
                        let pos = i.pos;
                        i.events.push(button(pos, false));
                    }
                });
                LRESULT(0)
            }
            WM_KEYDOWN | WM_KEYUP => {
                let key = match VIRTUAL_KEY(wp.0 as u16) {
                    VK_ESCAPE => Some(egui::Key::Escape),
                    VK_SPACE => Some(egui::Key::Space),
                    VK_LEFT => Some(egui::Key::ArrowLeft),
                    VK_RIGHT => Some(egui::Key::ArrowRight),
                    _ => None,
                };
                if let Some(key) = key {
                    let pressed = msg == WM_KEYDOWN;
                    let repeat = pressed && lp.0 & (1 << 30) != 0;
                    with(|i| i.events.push(egui::Event::Key { key, physical_key: None, pressed, repeat, modifiers: mods() }));
                }
                LRESULT(0)
            }
            WM_SETFOCUS | WM_KILLFOCUS => {
                let on = msg == WM_SETFOCUS;
                with(|i| {
                    i.focused = on;
                    i.events.push(egui::Event::WindowFocused(on));
                });
                LRESULT(0)
            }
            WM_SETCURSOR if (lp.0 & 0xFFFF) as u32 == HTCLIENT => {
                match with(|i| i.cursor).filter(|c| c.0 != 0) {
                    Some(c) => {
                        SetCursor(c);
                        LRESULT(1)
                    }
                    None => DefWindowProcW(h, msg, wp, lp),
                }
            }
            WM_DPICHANGED => {
                let r = *(lp.0 as *const RECT);
                with(|i| i.dpi_rect = Some(r));
                LRESULT(0)
            }
            WM_CLOSE => {
                with(|i| i.close = true);
                LRESULT(0)
            }
            _ => DefWindowProcW(h, msg, wp, lp),
        }
    }

    /// Held: the window moved with the cursor, or resized by the sides held, from where they were as it was grabbed.
    fn follow(h: HWND, g: &Grab, ppp: f32) {
        let mut c = POINT::default();
        let mut now = RECT::default();
        if !unsafe { GetCursorPos(&mut c).as_bool() && GetWindowRect(h, &mut now).as_bool() } {
            return;
        }
        let (dx, dy) = (c.x - g.cursor.x, c.y - g.cursor.y);
        let r = [g.rect.left, g.rect.top, g.rect.right, g.rect.bottom];
        let to = match g.sides {
            None => [r[0] + dx, r[1] + dy, r[2] + dx, r[3] + dy],
            Some(sides) => {
                let px = |v: Vec2| [(v.x * ppp).round() as i32, (v.y * ppp).round() as i32];
                super::sized(r, sides, dx, dy, px(MIN), px(MAX))
            }
        };
        if to != [now.left, now.top, now.right, now.bottom] {
            let flags = SWP_NOZORDER | SWP_NOACTIVATE | if g.sides.is_none() { SWP_NOSIZE } else { Default::default() };
            unsafe { SetWindowPos(h, HWND(0), to[0], to[1], to[2] - to[0], to[3] - to[1], flags) };
        }
    }

    /// Let go near (or past) the edge of its screen's work area, it settles `gap` pixels in from that edge.
    fn settle(h: HWND, snap: i32, gap: i32) {
        let mut rc = RECT::default();
        if !unsafe { GetWindowRect(h, &mut rc) }.as_bool() {
            return;
        }
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !unsafe { GetMonitorInfoW(MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST), &mut mi) }.as_bool() {
            return;
        }
        let wa = mi.rcWork;
        let x = super::place(rc.left, rc.right, wa.left, wa.right, snap, gap);
        let y = super::place(rc.top, rc.bottom, wa.top, wa.bottom, snap, gap);
        if (x, y) != (rc.left, rc.top) {
            unsafe { SetWindowPos(h, HWND(0), x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) };
        }
    }

    /// Is Shift held down right now?
    fn shift() -> bool {
        unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0 }
    }

    /// The time between refreshes of the screen the window is on (seconds; a 60th if it can't say).
    fn period_of(h: HWND) -> f64 {
        let mut mi = MONITORINFOEXW::default();
        mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        let mut dm = DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() };
        let m = unsafe { MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST) };
        let hz = unsafe {
            GetMonitorInfoW(m, &mut mi as *mut MONITORINFOEXW as *mut MONITORINFO).as_bool()
                && EnumDisplaySettingsW(PCWSTR(mi.szDevice.as_ptr()), ENUM_CURRENT_SETTINGS, &mut dm).as_bool()
        }
        .then_some(dm.dmDisplayFrequency)
        .filter(|&hz| hz > 1)
        .unwrap_or(60);
        1.0 / hz as f64
    }

    /// The system's cursor for egui's.
    fn cursor_of(c: CursorIcon) -> PCWSTR {
        match c {
            CursorIcon::PointingHand => IDC_HAND,
            CursorIcon::ResizeHorizontal | CursorIcon::ResizeEast | CursorIcon::ResizeWest | CursorIcon::ResizeColumn => IDC_SIZEWE,
            CursorIcon::ResizeVertical | CursorIcon::ResizeNorth | CursorIcon::ResizeSouth | CursorIcon::ResizeRow => IDC_SIZENS,
            CursorIcon::ResizeNwSe | CursorIcon::ResizeNorthWest | CursorIcon::ResizeSouthEast => IDC_SIZENWSE,
            CursorIcon::ResizeNeSw | CursorIcon::ResizeNorthEast | CursorIcon::ResizeSouthWest => IDC_SIZENESW,
            _ => IDC_ARROW,
        }
    }

    /// The cursor for a resize from these sides (left, top, right, bottom).
    fn resize_cursor(s: [bool; 4]) -> CursorIcon {
        match s {
            [true, true, _, _] | [_, _, true, true] => CursorIcon::ResizeNwSe,
            [_, true, true, _] | [true, _, _, true] => CursorIcon::ResizeNeSw,
            [_, true, _, _] | [_, _, _, true] => CursorIcon::ResizeVertical,
            _ => CursorIcon::ResizeHorizontal,
        }
    }

    /// (A test copy) how evenly it draws, said in its log every few seconds (BACKLINE_POP_LOG=<seconds>, 5 if not
    /// set): frames a second, and the gaps between frames longer than a refresh (each a refresh the screen showed
    /// the same frame twice).
    #[derive(Default)]
    struct Log {
        frames: u32,
        missed: u32,
        last: f64,
        since: f64,
    }

    impl Log {
        fn count(&mut self, clock: f64, period: f64) {
            if !crate::hidden_test_copy() {
                return;
            }
            if self.last > 0.0 {
                self.missed += (((clock - self.last) / period).round() as u32).saturating_sub(1);
            }
            self.last = clock;
            self.frames += 1;
            let every = std::env::var("BACKLINE_POP_LOG").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(5.0);
            if clock - self.since >= every {
                if self.since > 0.0 {
                    crate::api::log(&format!(
                        "visuals window: {:.0} frames a second on a {:.0} Hz screen, {} refreshes missed",
                        self.frames as f64 / (clock - self.since),
                        1.0 / period,
                        self.missed
                    ));
                }
                (self.frames, self.missed, self.since) = (0, 0, clock);
            }
        }
    }

    /// (A test copy) pictures of the window, as BACKLINE_SHOT's of the main one: BACKLINE_POP_SHOT=file.png, taken
    /// BACKLINE_SHOT_AT seconds after it opens (BACKLINE_SHOTS of them, BACKLINE_SHOT_EVERY apart).
    struct Shots {
        path: Option<String>,
        at: f64,
        n: u32,
        every: f64,
        taken: u32,
    }

    impl Shots {
        fn new() -> Self {
            let var = |k: &str| std::env::var(k).ok();
            Shots {
                path: var("BACKLINE_POP_SHOT"),
                at: var("BACKLINE_SHOT_AT").and_then(|s| s.parse().ok()).unwrap_or(8.0),
                n: var("BACKLINE_SHOTS").and_then(|s| s.parse().ok()).unwrap_or(1),
                every: var("BACKLINE_SHOT_EVERY").and_then(|s| s.parse().ok()).unwrap_or(0.1),
                taken: 0,
            }
        }

        fn take(&mut self, painter: &egui_glow::Painter, size: [u32; 2], clock: f64) {
            let Some(path) = &self.path else { return };
            if self.taken >= self.n || clock < self.at + self.taken as f64 * self.every {
                return;
            }
            let img = painter.read_screen_rgba(size);
            let out = if self.n > 1 { path.replace(".png", &format!("_{:03}.png", self.taken)) } else { path.clone() };
            std::thread::spawn(move || {
                let [w, h] = img.size;
                let px: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Some(buf) = image::RgbaImage::from_raw(w as u32, h as u32, px) {
                    let _ = buf.save(&out);
                }
            });
            self.taken += 1;
        }
    }
}

#[cfg(not(windows))]
mod win {
    use super::Pop;
    use crate::player::{Ctl, Viz};
    use eframe::egui::{self, Vec2};
    use std::sync::{Arc, Mutex};

    pub(super) fn start(_last: Option<[i32; 4]>, _asked: Option<Vec2>, _main: isize) -> [i32; 4] {
        [0; 4]
    }
    pub(super) fn open(pop: Arc<Mutex<Pop>>, _viz: Arc<Mutex<Viz>>, _ctl: Arc<Ctl>, _main: egui::Context, _fonts: egui::FontDefinitions, _at: [i32; 4]) {
        let mut s = super::lock(&pop);
        s.closed = true;
        s.done = true;
    }
    pub(super) fn wake(_h: isize) {}
}

#[cfg(test)]
mod tests {
    use super::{place, sized, steady};

    #[test]
    fn let_go_near_an_edge_it_settles_in_from_it() {
        // a screen 0..1000 across, a window 300 wide, settling 12 in from an edge let go within 20 of it
        assert_eq!(place(15, 315, 0, 1000, 20, 12), 12);
        assert_eq!(place(-50, 250, 0, 1000, 20, 12), 12);
        assert_eq!(place(690, 990, 0, 1000, 20, 12), 688);
        assert_eq!(place(800, 1100, 0, 1000, 20, 12), 688);
        // in the middle, or just outside the band: exactly where it was put
        assert_eq!(place(400, 700, 0, 1000, 20, 12), 400);
        assert_eq!(place(20, 320, 0, 1000, 20, 12), 20);
        assert_eq!(place(36, 336, 0, 1000, 20, 12), 36);
        // a second screen to the left (-2560..0)
        assert_eq!(place(-295, -5, -2560, 0, 20, 12), -302);
    }

    #[test]
    fn resized_from_a_side_the_others_stay_put() {
        let (r, min, max) = ([100, 100, 500, 325], [240, 135], [960, 540]);
        // the right edge and the bottom edge out; the left edge in
        assert_eq!(sized(r, [false, false, true, false], 50, 9, min, max), [100, 100, 550, 325]);
        assert_eq!(sized(r, [false, false, false, true], 9, 40, min, max), [100, 100, 500, 365]);
        assert_eq!(sized(r, [true, false, false, false], 60, 0, min, max), [160, 100, 500, 325]);
        // the top left corner, both ways
        assert_eq!(sized(r, [true, true, false, false], -20, -30, min, max), [80, 70, 500, 325]);
        // never smaller than the least or bigger than the most
        assert_eq!(sized(r, [true, false, false, false], 300, 0, min, max), [260, 100, 500, 325]);
        assert_eq!(sized(r, [false, false, true, true], 900, 900, min, max), [100, 100, 1060, 640]);
    }

    #[test]
    fn frames_once_a_refresh_move_evenly() {
        let p = 1.0 / 165.0;
        // woken a little early or late, a frame still comes one refresh after the last
        assert!((steady(1.0, 1.0 + p + 0.0004, p) - (1.0 + p)).abs() < 0.0001);
        assert!((steady(1.0, 1.0 + p - 0.0004, p) - (1.0 + p)).abs() < 0.0001);
        // a refresh missed, or the first frame: the clock's own time
        assert_eq!(steady(1.0, 1.0 + 2.0 * p, p), 1.0 + 2.0 * p);
        assert_eq!(steady(0.0, 5.0, p), 5.0);
    }
}
