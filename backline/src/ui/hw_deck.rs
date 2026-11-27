//! Hardware's Now Playing, at the left of the amp (the owner's picks of 2026-11-27, the "Reel deck" and "Reel motion"
//! style sheets, A): the album's cover standing upright on the floor in full view, and a reel-to-reel tape deck
//! playing the song.
//!
//! The tape really moves. One number drives it all, where the tape is shown (`hw_tape`, 0..1 of the song): the left
//! reel's pack empties into the right's (each pack's area is the tape on it), and each reel turns by the tape it lets
//! out or takes up over its own pack's radius, so the smaller pack spins faster, as on a real deck (a 10.5" reel at
//! 7.5 ips: about 14 rpm full, 30 rpm near the hub). Playing, the tape follows the song at its speed; a jump (a seek, a
//! loop going round, a new song) winds there as a transport does, getting up to speed and slowing to stop on it, the
//! spokes blurring when fast; pause brakes the reels and lifts the pinch roller off the capstan. The tape runs from
//! the left reel's left side down round a guide, along the heads, between the capstan and the pinch roller, round a
//! guide and up into the right reel's right side, both reels turning anticlockwise; it is drawn between the reels'
//! flanges, so it comes out from behind the front one. The reels sit on plain hub adapters: no art on them.
//!
//! Everything in the amp's units (`hw_amp::Mu`): the cover and the deck stand left of the amp, from `GEAR_X0`.

use super::*;
use crate::hw_amp::{ball, cabinet, caps, vgrad, Mu, FACE_INK};

/// The cover: its left, top and size, beside the deck; and in the close-up, as tall as the deck.
const COVER: (f32, f32, f32) = (-238.0, 328.0, 210.0);
const BIG: (f32, f32, f32) = (-440.0, 138.0, 400.0);
/// Where the deck stands on the floor (its shadow).
pub(crate) const DECK_FEET: (f32, f32) = (-16.0, 288.0);
/// The deck's body: left, top, width, height (its reels stand a little above it).
const DECK: (f32, f32, f32, f32) = (-16.0, 178.0, 304.0, 360.0);
/// The reels: their radius, centres, and their packs' radius empty (the hub) and full, as shares of it.
const R: f32 = 70.0;
const REEL_Y: f32 = 222.0;
const REEL_X: [f32; 2] = [62.0, 210.0];
const HUB: f32 = 0.43;
const FULL: f32 = 0.93;
/// A 10.5" NAB reel's parts as shares of its radius: the adapter it sits on, its hub (where the tape winds, NAB's 4.5"
/// of 10.5"), and where its three windows end at the rim.
const ADAPTER: f32 = 0.37;
const WIN_OUT: f32 = 0.87;
/// The tape path (the TEAC / Akai way, from the references): the tension arms' rollers at the deck's outer corners
/// under the reels, big guide rollers inside them, the heads under their cover, the capstan with the pinch roller
/// below the tape; the tape runs along all their feet (`TAPE_Y`).
const TAPE_Y: f32 = 320.0;
const TENSION: [(f32, f32); 2] = [(12.0, 6.0), (260.0, 6.0)];
const GUIDES: [(f32, f32); 2] = [(40.0, 8.5), (234.0, 8.5)];
const CAPSTAN_X: f32 = 206.0;
/// How fast the tape moves while playing, in units a second: a full reel then turns about 0.24 times a second.
const PLAY_V: f32 = 0.24 * std::f32::consts::TAU * FULL * R;
/// The spokes' angular width (three of them), and the light's direction on the brushed metal.
const SPOKE: f32 = 0.98;
const LIGHT: f32 = -2.2;

/// Where everything stands in Hardware's Now Playing (in units): the cover (left, top, size), how far the amp has
/// moved right and how much of it shows, and the part of the room in view (left, right, top, bottom).
pub(crate) struct Scene {
    pub(crate) cover: (f32, f32, f32),
    pub(crate) amp_dx: f32,
    pub(crate) amp_a: f32,
    pub(crate) view: [f32; 4],
}

/// The scene `t` of the way into the close-up (0 the whole row of gear, 1 the cover and the deck): the view moves in
/// and the cover grows to the deck's height while the amp slides away to the right.
pub(crate) fn scene(t: f32) -> Scene {
    let t = t * t * (3.0 - 2.0 * t);
    let l = |a: f32, b: f32| a + (b - a) * t;
    Scene {
        cover: (l(COVER.0, BIG.0), l(COVER.1, BIG.1), l(COVER.2, BIG.2)),
        amp_dx: 280.0 * t,
        amp_a: (1.0 - t * 2.4).max(0.0),
        view: [l(COVER.0 - 8.0, BIG.0 - 8.0), l(944.0, 296.0), l(26.0, BIG.1 - 16.0), 546.0],
    }
}

impl App {
    /// The album's cover standing upright on the floor beside the deck, in full view (`at`: its left, top and size);
    /// a click steps up close to it, or back.
    pub(crate) fn hw_cover(&mut self, ui: &mut egui::Ui, p: &egui::Painter, m: Mu, at: (f32, f32, f32), tex: Option<egui::TextureId>) {
        let r = m.r(at.0, at.1, at.2, at.2);
        // (a box's depth: its top and right edge, a touch of its shadow on the wall behind)
        let d = m.v(4.0);
        p.rect_filled(r.translate(Vec2::new(d * 1.6, -d * 0.4)), m.v(1.0), Color32::from_black_alpha(90));
        let top = vec![r.left_top(), r.left_top() + Vec2::new(d, -d), r.right_top() + Vec2::new(d, -d), r.right_top()];
        p.add(egui::Shape::convex_polygon(top, Color32::from_rgb(196, 189, 175), Stroke::NONE));
        let side = vec![r.right_top(), r.right_top() + Vec2::new(d, -d), r.right_bottom() + Vec2::new(d, -d), r.right_bottom()];
        p.add(egui::Shape::convex_polygon(side, Color32::from_rgb(120, 114, 104), Stroke::NONE));
        match tex {
            Some(id) => {
                egui::Image::new(egui::load::SizedTexture::new(id, r.size())).paint_at(ui, r);
            }
            None => {
                p.rect_filled(r, 0.0, Color32::from_rgb(30, 28, 26));
                paint_record(p, r.shrink(m.v(18.0)), None);
            }
        }
        // (a soft sheen across it, and its edges)
        let sheen = Rect::from_min_max(r.min, Pos2::new(r.max.x, r.min.y + r.height() * 0.45));
        vgrad(p, sheen, Color32::from_white_alpha(14), Color32::TRANSPARENT);
        p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::from_black_alpha(150)));
        let h = ui.interact(r, ui.id().with("hw_cover"), Sense::click());
        let tip = if self.hw_big { "Step back" } else { "Close-up: the cover bigger" };
        if h.on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip).clicked() {
            self.hw_big = !self.hw_big;
            self.dirty = true;
        }
    }

    /// The reel-to-reel deck: the reels and the tape, the heads, the capstan and pinch roller, a smoked glass display
    /// with the time and the song, and the transport's keys.
    pub(crate) fn hw_deck(&mut self, ui: &mut egui::Ui, p: &egui::Painter, m: Mu, dt: f32, acts: &mut Vec<Action>) {
        let now = ui.input(|i| i.time);
        let moved = self.tape_tick(dt, now);
        if moved != 0.0 || self.hw_tape_v != 0.0 {
            ui.ctx().request_repaint();
        }
        let (dx, dy, dw, dh) = DECK;
        let body = m.r(dx, dy, dw, dh);
        // ---- the body: tolex round a brushed aluminium faceplate
        cabinet(p, m, body, self.hw_tex.as_ref().map(|t| t[0].id()));
        let face = body.shrink(m.v(7.0));
        p.rect_filled(face, m.v(3.0), Color32::from_rgb(150, 146, 138));
        vgrad(p, face.shrink(m.v(1.0)), Color32::from_rgb(206, 202, 193), Color32::from_rgb(166, 161, 151));
        // (its brushing: fine horizontal lines, a little lighter and darker)
        let n = (face.height() / m.v(2.2)) as usize;
        for i in 0..n {
            let y = face.min.y + m.v(2.2) * i as f32 + m.v(1.0);
            let a = ((i * 37 + 11) % 13) as u8;
            let col = if i % 2 == 0 { Color32::from_white_alpha(6 + a) } else { Color32::from_black_alpha(4 + a / 2) };
            p.line_segment([Pos2::new(face.min.x + m.v(2.0), y), Pos2::new(face.max.x - m.v(2.0), y)], Stroke::new(1.0_f32, col));
        }
        p.rect_stroke(face, m.v(3.0), Stroke::new(1.0_f32, Color32::from_black_alpha(90)));

        // ---- the tape transport
        let (rl, rr) = packs(self.hw_tape);
        let centres = [m.p(REEL_X[0], REEL_Y), m.p(REEL_X[1], REEL_Y)];
        let k = m.s;
        // this frame's turn of each reel (for the blur)
        let spin = [moved / rl, moved / rr];
        self.hw_reel[0] -= spin[0];
        self.hw_reel[1] -= spin[1];
        for a in &mut self.hw_reel {
            *a = a.rem_euclid(std::f32::consts::TAU);
        }
        // the heads under their cover: the cover a dark brushed box over the tape, the three heads' tips (erase,
        // record, play) touching the tape from above, as the tape's oxide faces up on a deck turning anticlockwise
        let tape_y = TAPE_Y;
        let cov = Rect::from_min_max(m.p(dx + 84.0, tape_y - 30.0), m.p(dx + 200.0, tape_y - 5.0));
        p.rect_filled(cov.translate(Vec2::new(0.0, m.v(2.0))), m.v(3.0), Color32::from_black_alpha(80));
        p.rect_filled(cov, m.v(3.0), Color32::from_rgb(40, 39, 37));
        vgrad(p, cov.shrink(m.v(1.2)), Color32::from_rgb(92, 90, 86), Color32::from_rgb(50, 48, 46));
        p.line_segment([cov.left_top() + Vec2::new(m.v(3.0), m.v(1.5)), cov.right_top() + Vec2::new(-m.v(3.0), m.v(1.5))], Stroke::new(1.0_f32, Color32::from_white_alpha(40)));
        caps(p, cov.center() + Vec2::new(0.0, -m.v(1.0)), egui::Align2::CENTER_CENTER, "ERASE  ·  REC  ·  PLAY", m.v(5.6), m.v(1.0), Color32::from_rgb(196, 192, 184), false);
        for hx in [dx + 104.0, dx + 136.0, dx + 168.0] {
            let hr = Rect::from_min_max(m.p(hx, tape_y - 6.0), m.p(hx + 13.0, tape_y));
            p.rect_filled(hr, m.v(1.0), Color32::from_rgb(34, 33, 31));
            vgrad(p, hr.shrink2(Vec2::new(m.v(1.5), 0.0)), Color32::from_rgb(214, 212, 206), Color32::from_rgb(132, 130, 124));
        }
        // the tension arms (still while playing: levers from a pivot, a roller at their end) and the guide rollers
        let rollers: Vec<(Pos2, f32)> = TENSION.iter().chain(GUIDES.iter()).map(|(x, r)| (m.p(*x, tape_y - r), m.v(*r))).collect();
        for (i, (x, r)) in TENSION.iter().enumerate() {
            let c = m.p(*x, tape_y - r);
            let piv = m.p(x + if i == 0 { 16.0 } else { -16.0 }, tape_y + 16.0);
            p.line_segment([c + Vec2::new(0.0, m.v(1.0)), piv + Vec2::new(0.0, m.v(1.0))], Stroke::new(m.v(4.2), Color32::from_black_alpha(70)));
            p.line_segment([c, piv], Stroke::new(m.v(3.6), Color32::from_rgb(64, 62, 58)));
            ball(p, piv, m.v(4.0), Vec2::new(-0.3, -0.3), &[(0.0, Color32::from_rgb(120, 118, 112)), (1.0, Color32::from_rgb(40, 39, 37))]);
        }
        for (c, r) in &rollers {
            p.circle_filled(*c + Vec2::new(0.0, m.v(1.2)), *r, Color32::from_black_alpha(70));
            ball(p, *c, *r, Vec2::new(-0.3, -0.35), &[(0.0, Color32::from_rgb(238, 236, 230)), (0.7, Color32::from_rgb(156, 154, 148)), (1.0, Color32::from_rgb(92, 90, 86))]);
            p.circle_filled(*c, *r * 0.28, Color32::from_rgb(70, 68, 64));
        }
        let cap = m.p(CAPSTAN_X, tape_y - 2.6);
        ball(p, cap, m.v(2.4), Vec2::new(-0.3, -0.3), &[(0.0, Color32::from_rgb(240, 240, 240)), (1.0, Color32::from_rgb(120, 120, 120))]);

        // the reels' back flanges and packs
        for (i, c) in centres.iter().enumerate() {
            flange(p, *c, k, self.hw_reel[i], spin[i], true);
            pack(p, *c, k * if i == 0 { rl } else { rr }, k * HUB * R);
        }
        // the tape: from the left pack round the left guide, along the heads past the capstan, round the right guide
        // and up into the right pack (both reels anticlockwise)
        let path = [
            (centres[0], k * rl, -1.0),
            (rollers[0].0, rollers[0].1, -1.0),
            (rollers[2].0, rollers[2].1, -1.0),
            (rollers[3].0, rollers[3].1, -1.0),
            (rollers[1].0, rollers[1].1, -1.0),
            (centres[1], k * rr, -1.0),
        ];
        tape(p, &path, m.v(1.7));
        // the front flanges and the hub adapters the reels sit on
        for (i, c) in centres.iter().enumerate() {
            flange(p, *c, k, self.hw_reel[i], spin[i], false);
            adapter(p, *c, k, self.hw_reel[i], spin[i]);
        }
        // the pinch roller: pressed against the capstan, the tape between, lifted away when not playing
        let pin = m.p(CAPSTAN_X, tape_y + 9.6 + 5.0 * self.hw_pinch);
        p.circle_filled(pin + Vec2::new(0.0, m.v(1.2)), m.v(9.0), Color32::from_black_alpha(80));
        ball(p, pin, m.v(9.0), Vec2::new(-0.25, -0.35), &[(0.0, Color32::from_rgb(64, 62, 60)), (0.8, Color32::from_rgb(22, 21, 20)), (1.0, Color32::from_rgb(12, 12, 12))]);
        ball(p, pin, m.v(3.0), Vec2::new(-0.3, -0.3), &[(0.0, Color32::from_rgb(210, 208, 202)), (1.0, Color32::from_rgb(110, 108, 104))]);

        // ---- the display: smoked glass set flush in the faceplate, the time in 7-segment digits and the song
        let gl = m.r(dx + 18.0, 346.0, dw - 36.0, 84.0);
        p.rect_filled(gl, m.v(2.0), Color32::from_rgb(16, 11, 6));
        vgrad(p, Rect::from_min_max(gl.min, Pos2::new(gl.max.x, gl.min.y + m.v(10.0))), Color32::from_black_alpha(120), Color32::TRANSPARENT);
        p.rect_stroke(gl, m.v(2.0), Stroke::new(1.0_f32, Color32::from_black_alpha(160)));
        p.line_segment([gl.left_bottom() + Vec2::new(m.v(2.0), 1.0), gl.right_bottom() + Vec2::new(-m.v(2.0), 1.0)], Stroke::new(1.0_f32, Color32::from_white_alpha(60)));
        let amber = crate::styles::amber();
        let tr = self.cur_track();
        let dur = tr.as_ref().map_or(0.0, |t| t.duration);
        let pos = if self.cur.is_some() { self.pos() } else { 0.0 };
        let shown = if self.time_left { format!("-{}", fmt_time((dur - pos).max(0.0))) } else { fmt_time(pos) };
        let tl = Pos2::new(gl.min.x + m.v(12.0), gl.min.y + m.v(24.0));
        let tw = crate::styles::hw_time_h(p, tl, &shown, dur, m.v(26.0), gl.width() * 0.62);
        let time_hit = Rect::from_min_max(gl.min, Pos2::new(gl.min.x + tw + m.v(24.0), gl.min.y + m.v(42.0)));
        if ui.interact(time_hit, ui.id().with("hw_deck_time"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Elapsed / remaining").clicked() {
            self.time_left = !self.time_left;
            self.dirty = true;
        }
        // (what the transport is doing, as a deck's display says it)
        let state = if self.cur.is_none() || self.stopped {
            "STOP"
        } else if self.buffering {
            "LOAD"
        } else if self.hw_tape_v.abs() * dur.max(1.0) > 3.0 {
            if self.hw_tape_v > 0.0 { "FF" } else { "REW" }
        } else if self.paused {
            "PAUSE"
        } else {
            "PLAY"
        };
        crate::styles::screen_text(p, Pos2::new(gl.max.x - m.v(10.0), gl.min.y + m.v(13.0)), Align::Max, state, 0.95 * k, m.v(60.0), amber.gamma_multiply(0.8));
        let room = gl.width() - m.v(24.0);
        if let Some(t) = &tr {
            // (a long title or artist scrolls across the glass, as a deck's display does)
            crate::styles::screen_marquee(p, Pos2::new(gl.min.x + m.v(12.0), gl.min.y + m.v(52.0)), Align::Min, &t.title, 1.3 * k, room, amber);
            crate::styles::screen_marquee(p, Pos2::new(gl.min.x + m.v(12.0), gl.min.y + m.v(70.0)), Align::Min, &t.artists_text(), 1.05 * k, room, amber.gamma_multiply(0.6));
        } else {
            crate::styles::screen_text(p, Pos2::new(gl.min.x + m.v(12.0), gl.min.y + m.v(60.0)), Align::Min, "No tape", 1.2 * k, room, amber.gamma_multiply(0.6));
        }
        // (glare on the glass)
        let glare = vec![gl.left_top(), Pos2::new(gl.min.x + gl.width() * 0.42, gl.min.y), Pos2::new(gl.min.x + gl.width() * 0.3, gl.max.y), gl.left_bottom()];
        p.add(egui::Shape::convex_polygon(glare, Color32::from_white_alpha(7), Stroke::NONE));

        // ---- the transport's keys: rewind (previous), play, pause, stop, fast forward (next)
        let playing = self.cur.is_some() && !self.stopped && !self.paused;
        let keys: [(&str, Action, &str, bool); 5] = [
            ("prev", Action::Prev, "Previous", false),
            ("play", Action::PlayBtn, "Play", playing),
            ("pause", Action::PauseBtn, "Pause", self.paused && !self.stopped),
            ("stop", Action::StopBtn, "Stop", false),
            ("skip", Action::Next, "Next", false),
        ];
        let gap = 6.0;
        let kw = (dw - 36.0 - gap * 4.0) / 5.0;
        for (i, (icon, act, tip, on)) in keys.into_iter().enumerate() {
            let kr = m.r(dx + 18.0 + (kw + gap) * i as f32, 440.0, kw, 30.0);
            let h = ui.interact(kr, ui.id().with(("hw_deck_key", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
            crate::widgets::key_face_on(p, kr, h.is_pointer_button_down_on(), h.hovered(), on);
            let ir = Rect::from_center_size(kr.center() + Vec2::new(0.0, if h.is_pointer_button_down_on() { 1.0 } else { 0.0 }), Vec2::splat(kr.height() * 0.42));
            crate::vicon::named(p, ir, icon, crate::widgets::key_ink(on));
            if h.clicked() {
                acts.push(act);
            }
        }
        // ---- the print along its foot, and the lamp lit while playing
        let foot_y = 504.0;
        caps(p, m.p(dx + 20.0, foot_y), egui::Align2::LEFT_CENTER, "BACKLINE", m.v(10.0), m.v(2.6), FACE_INK, true);
        caps(p, m.p(dx + 20.0, foot_y + 14.0), egui::Align2::LEFT_CENTER, "STEREO TAPE DECK", m.v(6.0), m.v(1.4), FACE_INK.gamma_multiply(0.8), false);
        caps(p, m.p(dx + dw - 40.0, foot_y), egui::Align2::RIGHT_CENTER, "7½ IPS", m.v(8.0), m.v(1.4), FACE_INK, true);
        let lamp = m.p(dx + dw - 26.0, foot_y);
        let lit = playing && !self.buffering;
        p.circle_filled(lamp, m.v(4.6), Color32::from_rgb(70, 66, 60));
        let on_stops = [(0.0, Color32::from_rgb(255, 190, 170)), (0.5, Color32::from_rgb(240, 60, 40)), (1.0, Color32::from_rgb(150, 20, 10))];
        let off_stops = [(0.0, Color32::from_rgb(150, 90, 80)), (0.5, Color32::from_rgb(100, 30, 24)), (1.0, Color32::from_rgb(60, 14, 10))];
        ball(p, lamp, m.v(3.6), Vec2::new(-0.3, -0.3), if lit { &on_stops } else { &off_stops });
        if lit {
            p.circle_filled(lamp, m.v(7.0), Color32::from_rgba_unmultiplied(255, 60, 30, 24));
        }
    }

    /// Moves the shown tape (`hw_tape`) on by this frame: following the song while it plays, winding to where it is
    /// when it jumps, braking to a stop when it pauses. Returns how far the tape moved, in units (signed: towards the
    /// right reel is forward).
    fn tape_tick(&mut self, dt: f32, now: f64) -> f32 {
        let dt = dt.clamp(0.0, 0.05);
        let tr = self.cur_track();
        let (id, dur) = tr.as_ref().map_or((0, 1.0), |t| (t.id, t.duration.max(1.0)));
        let target = if self.cur.is_some() && !self.stopped { (self.pos() / dur).clamp(0.0, 1.0) } else { 0.0 };
        // (not drawn a while - Now Playing just opened: the tape is where the song is, no winding to get there)
        let fresh = now - self.hw_tape_t > 0.5;
        self.hw_tape_t = now;
        if fresh {
            self.hw_tape = target;
            self.hw_tape_v = 0.0;
            self.hw_tape_for = id;
        }
        self.hw_tape_for = id;
        let playing = self.cur.is_some() && !self.stopped && !self.paused && !self.buffering;
        self.hw_pinch += ((if playing { 0.0 } else { 1.0 }) - self.hw_pinch) * (1.0 - (-dt * 10.0).exp());
        let rate = if playing { self.speed as f32 / 100.0 / dur } else { 0.0 };
        let d = target - self.hw_tape;
        // (within a second and a half of the song: following it; further: winding there)
        let follow = d.abs() * dur < 1.5;
        let want = if follow {
            rate + d * 3.0
        } else {
            const WIND: f32 = 1.4;
            const ACC: f32 = 3.5;
            d.signum() * WIND.min((2.0 * ACC * d.abs()).sqrt())
        };
        // (a transport's inertia: it reaches the speed it wants quickly, but not at once)
        let tau = if follow { 0.12 } else { 0.08 };
        self.hw_tape_v += (want - self.hw_tape_v) * (1.0 - (-dt / tau).exp());
        if !playing && follow && self.hw_tape_v.abs() * dur < 0.002 {
            self.hw_tape_v = 0.0;
        }
        let before = self.hw_tape;
        self.hw_tape = (self.hw_tape + self.hw_tape_v * dt).clamp(0.0, 1.0);
        // the tape that moved, in units: a whole song is PLAY_V a second for its length
        (self.hw_tape - before) * PLAY_V * dur
    }
}

/// The packs' radii (units) with the tape at `f` of the song: the left's empties into the right's, each pack's area
/// being the tape wound on it.
fn packs(f: f32) -> (f32, f32) {
    let (h, full) = ((HUB * R).powi(2), (FULL * R).powi(2));
    let f = f.clamp(0.0, 1.0);
    ((h + (1.0 - f) * (full - h)).sqrt(), (h + f * (full - h)).sqrt())
}

/// Brushed metal's tone at angle `a` round a reel: brighter where the light catches the brushing (two lobes, along the
/// light's direction), staying put while the reel turns.
fn metal(a: f32, dark: bool, alpha: f32) -> Color32 {
    let l = (a - LIGHT).cos().abs().powi(6);
    let (lo, hi) = if dark { (70.0, 112.0) } else { (150.0, 236.0) };
    let v = lo + (hi - lo) * (0.25 + 0.75 * l);
    let c = Color32::from_rgb(v as u8, (v - 2.0) as u8, (v - 6.0).max(0.0) as u8);
    c.gamma_multiply(alpha)
}

/// An annular sector `r0..r1` from angle `a0` to `a1` round `c`, coloured by angle.
fn ring(mesh: &mut egui::Mesh, c: Pos2, r0: f32, r1: f32, a0: f32, a1: f32, col: &dyn Fn(f32) -> Color32) {
    let n = ((a1 - a0).abs() / 0.08).ceil().max(2.0) as u32;
    let base = mesh.vertices.len() as u32;
    for i in 0..=n {
        let a = a0 + (a1 - a0) * i as f32 / n as f32;
        let (s, co) = a.sin_cos();
        let cl = col(a);
        mesh.colored_vertex(c + Vec2::new(co, s) * r0, cl);
        mesh.colored_vertex(c + Vec2::new(co, s) * r1, cl);
    }
    for i in 0..n {
        let k = base + 2 * i;
        mesh.add_triangle(k, k + 1, k + 2);
        mesh.add_triangle(k + 1, k + 3, k + 2);
    }
}

/// How many copies of a turning part to draw for its motion blur, and the angle they spread over: none while it turns
/// slowly, a smear over the frame's turn when fast, a whole spoke's period (an even blur) at speed.
fn blur(turn: f32) -> (usize, f32) {
    let t = turn.abs().min(std::f32::consts::TAU / 3.0);
    if t < 0.1 {
        (1, 0.0)
    } else {
        (((t / 0.07).ceil() as usize).clamp(2, 12), t * turn.signum())
    }
}

/// A reel's flange (the back one, in shadow, or the front one): the rim, the ring round the hub, and three spokes
/// with windows between them, turned to `ang`; blurred by this frame's `turn`.
fn flange(p: &egui::Painter, c: Pos2, k: f32, ang: f32, turn: f32, back: bool) {
    let (rim0, hub0, hub1) = (WIN_OUT * R * k, ADAPTER * R * k, HUB * R * k);
    let mut mesh = egui::Mesh::default();
    let tone = |a: f32| metal(a, back, 1.0);
    ring(&mut mesh, c, rim0, R * k, 0.0, std::f32::consts::TAU, &tone);
    ring(&mut mesh, c, hub0, hub1, 0.0, std::f32::consts::TAU, &tone);
    let (n, spread) = blur(turn);
    // (blurred, the copies add up to about the spokes' share of the ring: a see-through grey, the pack behind)
    let alpha = if n == 1 { 1.0 } else { 1.15 / n as f32 };
    for j in 0..n {
        // (the copies spread back over where the spokes were during the frame)
        let a0 = ang + spread * j as f32 / n.max(1) as f32;
        let tone_j = |a: f32| metal(a, back, alpha);
        for s in 0..3 {
            let mid = a0 + s as f32 * std::f32::consts::TAU / 3.0;
            ring(&mut mesh, c, hub1 - 1.0, rim0 + 1.0, mid - SPOKE / 2.0, mid + SPOKE / 2.0, &tone_j);
        }
    }
    p.add(egui::Shape::mesh(mesh));
    // (edges: the rim's outer lip and inner edge, the hub ring's, each spoke's sides; faint when blurred)
    let edge = Color32::from_black_alpha(if back { 70 } else { 110 });
    p.circle_stroke(c, R * k, Stroke::new(1.0_f32, Color32::from_black_alpha(140)));
    p.circle_stroke(c, rim0, Stroke::new(1.0_f32, edge));
    p.circle_stroke(c, hub1, Stroke::new(1.0_f32, edge));
    if n == 1 {
        for s in 0..3 {
            let mid = ang + s as f32 * std::f32::consts::TAU / 3.0;
            for side in [-1.0, 1.0] {
                let a = mid + side * SPOKE / 2.0;
                let d = Vec2::new(a.cos(), a.sin());
                p.line_segment([c + d * hub1, c + d * rim0], Stroke::new(1.0_f32, edge));
            }
        }
    }
    if !back {
        // (the front rim's light lip)
        p.circle_stroke(c, R * k - 1.5, Stroke::new(1.0_f32, Color32::from_white_alpha(40)));
    }
}

/// The tape wound on a reel: a dark brown pack of radius `r`, its turns faintly showing, a sheen where the light falls.
fn pack(p: &egui::Painter, c: Pos2, r: f32, hub: f32) {
    p.circle_filled(c, r, Color32::from_rgb(46, 33, 24));
    let mut mesh = egui::Mesh::default();
    let sheen = |a: f32| {
        let l = (a - LIGHT).cos().abs().powi(10);
        Color32::from_rgba_premultiplied((40.0 * l) as u8, (30.0 * l) as u8, (20.0 * l) as u8, 0)
    };
    ring(&mut mesh, c, r * 0.55, r, 0.0, std::f32::consts::TAU, &sheen);
    p.add(egui::Shape::mesh(mesh));
    let mut rr = r - 3.0;
    let mut i = 0;
    while rr > hub * 1.02 && i < 8 {
        p.circle_stroke(c, rr, Stroke::new(1.0_f32, Color32::from_black_alpha(if i % 2 == 0 { 28 } else { 16 })));
        rr -= 4.0 + (i % 3) as f32;
        i += 1;
    }
    p.circle_stroke(c, r, Stroke::new(1.0_f32, Color32::from_rgb(70, 52, 38)));
}

/// The hub adapter a reel sits on: a dark disc with three silver locks turning with the reel, a cap in the middle.
fn adapter(p: &egui::Painter, c: Pos2, k: f32, ang: f32, turn: f32) {
    let r = ADAPTER * R * k;
    ball(p, c, r, Vec2::new(-0.25, -0.35), &[(0.0, Color32::from_rgb(74, 72, 68)), (0.85, Color32::from_rgb(34, 33, 31)), (1.0, Color32::from_rgb(20, 19, 18))]);
    let (n, spread) = blur(turn);
    let alpha = (1.6 / n as f32).min(1.0);
    for j in 0..n {
        let a0 = ang + spread * j as f32 / n as f32 + std::f32::consts::PI / 3.0;
        for s in 0..3 {
            let a = a0 + s as f32 * std::f32::consts::TAU / 3.0;
            let d = Vec2::new(a.cos(), a.sin());
            let q = Vec2::new(-d.y, d.x);
            let (a1, b1) = (c + d * r * 0.42, c + d * r * 0.9);
            let w = r * 0.16;
            let pts = vec![a1 + q * w, b1 + q * w, b1 - q * w, a1 - q * w];
            p.add(egui::Shape::convex_polygon(pts, Color32::from_rgb(196, 194, 188).gamma_multiply(alpha), Stroke::NONE));
        }
    }
    ball(p, c, r * 0.32, Vec2::new(-0.3, -0.3), &[(0.0, Color32::from_rgb(240, 238, 232)), (0.7, Color32::from_rgb(150, 148, 142)), (1.0, Color32::from_rgb(96, 94, 90))]);
}

/// Where the tape leaves circle `a` and meets circle `b`, going round each in its direction (`dir` +1 clockwise on
/// screen, -1 anticlockwise): the common tangent, so the tape never cuts through a pack or a guide as they change.
fn tangent(a: (Pos2, f32, f32), b: (Pos2, f32, f32)) -> (Pos2, Pos2) {
    let (c1, r1, o1) = a;
    let (c2, r2, o2) = b;
    let d = c2 - c1;
    let len = d.length().max(1e-3);
    let u = d / len;
    let v = Vec2::new(-u.y, u.x);
    let kk = ((r1 - r2 * o1 * o2) / len).clamp(-1.0, 1.0);
    let n = u * kk - v * (o1 * (1.0 - kk * kk).max(0.0).sqrt());
    (c1 + n * r1, c2 + n * (r2 * o1 * o2))
}

/// The tape along `path` (each a circle it goes round: centre, radius, direction): straight runs between them, and
/// the arcs where it wraps the guides between (not the packs at the ends, which it's part of).
fn tape(p: &egui::Painter, path: &[(Pos2, f32, f32)], w: f32) {
    let col = Color32::from_rgb(52, 36, 24);
    let mut legs: Vec<(Pos2, Pos2)> = Vec::new();
    for pair in path.windows(2) {
        legs.push(tangent(pair[0], pair[1]));
    }
    for (a, b) in &legs {
        p.line_segment([*a, *b], Stroke::new(w, col));
    }
    // the wraps round the guides: from where one leg arrives to where the next leaves, the short way in its direction
    for (i, (c, r, o)) in path.iter().enumerate().skip(1).take(path.len().saturating_sub(2)) {
        let (into, out) = (legs[i - 1].1, legs[i].0);
        let a0 = (into - *c).angle();
        let mut a1 = (out - *c).angle();
        if *o > 0.0 {
            while a1 < a0 {
                a1 += std::f32::consts::TAU;
            }
        } else {
            while a1 > a0 {
                a1 -= std::f32::consts::TAU;
            }
        }
        let n = (((a1 - a0).abs() / 0.15).ceil() as usize).max(2);
        let pts: Vec<Pos2> = (0..=n).map(|j| *c + Vec2::angled(a0 + (a1 - a0) * j as f32 / n as f32) * *r).collect();
        p.add(egui::Shape::line(pts, Stroke::new(w, col)));
    }
}
