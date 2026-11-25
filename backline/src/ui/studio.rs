//! Studio: Tidalite's look grown into a Backline style (the owner's picks of 2026-11-25, style sheets 7 and 8). It's
//! Hardware's relative: everything Hardware has, Studio has (`hardware()` is true for both), drawn its own way. Every
//! part is a screen (cream, gray, blue or amber) set in a black body and named in brackets on its frame; keys have a
//! fine outline; the time is LCD digits with the unlit segments faintly there; the song scrolls by in dot type; lists
//! are in a typewriter face. Its Now Playing is one big screen with tabs on its frame (Lyrics, Up Next, About, Loops,
//! Visuals), the visuals drawn in the screen's ink (bars, a scope; the terrain only in dev builds).

use super::*;

/// The body the screens sit in, a screen's frame and the rim round it, the words on the frame.
pub(crate) const BODY: Color32 = Color32::from_rgb(21, 22, 17);
const FRAME: Color32 = Color32::from_rgb(10, 11, 8);
const RIM: Color32 = Color32::from_rgb(59, 61, 49);
const TAB_OFF: Color32 = Color32::from_rgb(38, 39, 31);

/// Whether Studio is worn.
pub(crate) fn on() -> bool {
    crate::skin::style() == crate::skin::STYLE_STUDIO
}

/// The screens' colours now: (the glass, the keys, the ink, the names on the frames).
fn cols() -> (Color32, Color32, Color32, Color32) {
    let s = crate::skin::studio_screen();
    (s.1, s.2, s.3, s.4)
}
pub(crate) fn scr() -> Color32 {
    cols().0
}
pub(crate) fn key_col() -> Color32 {
    cols().1
}
pub(crate) fn ink() -> Color32 {
    cols().2
}
pub(crate) fn label() -> Color32 {
    cols().3
}
/// The ink faintly (an unlit segment, a light that's off).
pub(crate) fn ghost() -> Color32 {
    scr().lerp_to_gamma(ink(), 0.1)
}
/// The ink, quieter (names, small print).
pub(crate) fn dim() -> Color32 {
    ink().lerp_to_gamma(scr(), 0.42)
}

fn sc() -> f32 {
    font::ui_scale()
}

/// Studio's three faces, added to every font set (small: about 600 KB in all): pixel type (Silkscreen), dot type
/// (Doto) and a typewriter face (IBM Plex Mono), each with the proportional fonts behind it for any letter it lacks;
/// while Studio is worn, the typewriter face goes first for all the reading.
pub(crate) fn add_fonts(fonts: &mut egui::FontDefinitions) {
    let faces: [(&str, &'static [u8]); 3] = [
        ("studio_px", include_bytes!("../../assets/Silkscreen-Regular.ttf")),
        ("studio_dot", include_bytes!("../../assets/Doto.ttf")),
        ("studio_mono", include_bytes!("../../assets/IBMPlexMono-Regular.ttf")),
    ];
    let behind: Vec<String> = fonts.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default();
    for (name, bytes) in faces {
        fonts.font_data.insert(name.to_string(), egui::FontData::from_static(bytes));
        let mut fam = vec![name.to_string()];
        fam.extend(behind.iter().filter(|n| !n.starts_with("studio_")).cloned());
        fonts.families.insert(egui::FontFamily::Name(name.into()), fam);
    }
    if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        f.retain(|n| n != "studio_mono");
        if on() {
            f.insert(0, "studio_mono".to_string());
        }
    }
}

/// As a style goes on: Studio's typewriter face first for the reading while it's worn, the others' fonts otherwise
/// (the fonts set again only when that changes).
pub(crate) fn fonts_for_style(ctx: &egui::Context) {
    let mut fonts = crate::app_look::fonts_now();
    let first = fonts.families.get(&egui::FontFamily::Proportional).and_then(|f| f.first().cloned()).unwrap_or_default();
    if (first == "studio_mono") == on() && fonts.font_data.contains_key("studio_mono") {
        return;
    }
    add_fonts(&mut fonts);
    crate::set_fonts(ctx, fonts);
}

/// Pixel type (frame names, picker names, labels).
pub(crate) fn px(p: &egui::Painter, at: Pos2, anchor: egui::Align2, text: &str, size: f32, col: Color32) -> Rect {
    p.text(at, anchor, text, egui::FontId::new(size * sc(), egui::FontFamily::Name("studio_px".into())), col)
}
/// Dot type (the song scrolling by, the lyrics).
pub(crate) fn dot(p: &egui::Painter, at: Pos2, anchor: egui::Align2, text: &str, size: f32, col: Color32) -> Rect {
    p.text(at, anchor, text, egui::FontId::new(size * sc(), egui::FontFamily::Name("studio_dot".into())), col)
}
/// How wide `text` is in dot type.
pub(crate) fn dot_w(ctx: &egui::Context, text: &str, size: f32) -> f32 {
    ctx.fonts(|f| f.layout_no_wrap(text.to_string(), egui::FontId::new(size * sc(), egui::FontFamily::Name("studio_dot".into())), Color32::WHITE).size().x)
}

/// A screen: its glass, a black frame and a fine rim round it; `name` in brackets on the frame's top edge.
pub(crate) fn frame(p: &egui::Painter, r: Rect, name: &str) {
    p.rect_filled(r.expand(3.0), 2.0, RIM);
    p.rect_filled(r.expand(2.0), 1.0, FRAME);
    p.rect_filled(r, 0.0, scr());
    // (a little shade along the inside of the top edge: the glass sits behind the frame)
    p.rect_filled(Rect::from_min_size(r.min, Vec2::new(r.width(), 2.0)), 0.0, Color32::from_black_alpha(26));
    if !name.is_empty() {
        let t = format!("─[ {} ]─", name.to_uppercase());
        let size = 9.0;
        let w = p.ctx().fonts(|f| f.layout_no_wrap(t.clone(), egui::FontId::new(size * sc(), egui::FontFamily::Name("studio_px".into())), Color32::WHITE).size().x);
        let at = Pos2::new(r.min.x + 10.0, r.min.y - 1.0);
        p.rect_filled(Rect::from_min_size(Pos2::new(at.x - 3.0, at.y - 6.5 * sc()), Vec2::new(w + 6.0, 13.0 * sc())), 0.0, BODY);
        px(p, at, egui::Align2::LEFT_CENTER, &t, size, label());
    }
}

/// Small tabs on a screen's frame, at its top right (the window's switches, Now Playing's views): the one that's on
/// in the glass's colour. Returns the one clicked.
pub(crate) fn frame_tabs(ui: &mut egui::Ui, r: Rect, items: &[(&str, bool, &str)], salt: &str) -> Option<usize> {
    let p = ui.painter().clone();
    let size = 8.0;
    let font = egui::FontId::new(size * sc(), egui::FontFamily::Name("studio_px".into()));
    let ws: Vec<f32> = items.iter().map(|(t, _, _)| ui.fonts(|f| f.layout_no_wrap(t.to_uppercase(), font.clone(), Color32::WHITE).size().x) + 8.0).collect();
    let mut x = r.max.x - 4.0 - ws.iter().sum::<f32>() - 3.0 * (items.len().saturating_sub(1)) as f32;
    let h = 13.0 * sc();
    let y = r.min.y - 1.0 - h / 2.0;
    let mut hit = None;
    for (i, (t, on, tip)) in items.iter().enumerate() {
        let tr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(ws[i], h));
        x = tr.max.x + 3.0;
        let resp = ui.interact(tr, ui.id().with((salt, i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let resp = if tip.is_empty() { resp } else { resp.tip(*tip) };
        let (bg, fg) = if *on {
            (scr(), ink())
        } else if resp.hovered() {
            (TAB_OFF.lerp_to_gamma(label(), 0.2), label())
        } else {
            (TAB_OFF, label().gamma_multiply(0.75))
        };
        p.rect_filled(tr, 0.0, bg);
        p.rect_stroke(tr, 0.0, Stroke::new(1.0_f32, if *on { FRAME } else { RIM }));
        px(&p, tr.center() + Vec2::new(0.0, 0.5), egui::Align2::CENTER_CENTER, &t.to_uppercase(), size, fg);
        if resp.clicked() {
            hit = Some(i);
        }
    }
    hit
}

/// A key with a fine outline: the keys' colour, lighter under the pointer, a shade darker while pressed; `on`, filled
/// with the ink (whatever's on it then in the glass's colour: `key_fg`).
pub(crate) fn key(p: &egui::Painter, r: Rect, on: bool, down: bool, hover: bool) {
    let fill = if on {
        ink()
    } else if down {
        key_col().lerp_to_gamma(ink(), 0.22)
    } else if hover {
        key_col().lerp_to_gamma(scr(), 0.55)
    } else {
        key_col()
    };
    p.rect_filled(r, 0.0, fill);
    p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, ink()));
}
/// What's written on a key: the ink, or the glass's colour on a key that's on.
pub(crate) fn key_fg(on: bool) -> Color32 {
    if on {
        scr()
    } else {
        ink()
    }
}

/// A light (a small square of ink, hollow when off).
pub(crate) fn light(p: &egui::Painter, c: Pos2, lit: bool, col: Color32) {
    let r = Rect::from_center_size(c, Vec2::splat(6.0));
    if lit {
        p.rect_filled(r, 0.0, col);
    }
    p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, col));
}

/// LCD digits, `h` tall, from `at` (left middle): every segment there, the unlit ones faint. ':' and '-' and ' '
/// (a blank digit) read too. Returns how wide.
pub(crate) fn digits(p: &egui::Painter, at: Pos2, h: f32, text: &str, lit_col: Color32, ghost_col: Color32) -> f32 {
    let k = h / 25.0;
    let (w, hh, t, g) = (12.0 * k, 22.0 * k, 2.6 * k, 0.6 * k);
    let adv = 15.5 * k;
    let skew = 0.12;
    let top = at.y - hh / 2.0;
    let mut x = at.x;
    // a point of the digit at (dx, dy) from its top left, slanted as LCD digits are
    let pt = |x0: f32, dx: f32, dy: f32| Pos2::new(x0 + dx + (hh - dy) * skew, top + dy);
    let seg = |x0: f32, s: char| -> Vec<Pos2> {
        let hz = |dy: f32| vec![pt(x0, g, dy), pt(x0, g + t / 2.0, dy - t / 2.0), pt(x0, w - g - t / 2.0, dy - t / 2.0), pt(x0, w - g, dy), pt(x0, w - g - t / 2.0, dy + t / 2.0), pt(x0, g + t / 2.0, dy + t / 2.0)];
        let vt = |dx: f32, dy0: f32, len: f32| vec![pt(x0, dx, dy0 + g), pt(x0, dx + t / 2.0, dy0 + g + t / 2.0), pt(x0, dx + t / 2.0, dy0 + len - g - t / 2.0), pt(x0, dx, dy0 + len - g), pt(x0, dx - t / 2.0, dy0 + len - g - t / 2.0), pt(x0, dx - t / 2.0, dy0 + g + t / 2.0)];
        match s {
            'a' => hz(0.0),
            'g' => hz(hh / 2.0),
            'd' => hz(hh),
            'f' => vt(0.0, 0.0, hh / 2.0),
            'b' => vt(w, 0.0, hh / 2.0),
            'e' => vt(0.0, hh / 2.0, hh / 2.0),
            _ => vt(w, hh / 2.0, hh / 2.0),
        }
    };
    for ch in text.chars() {
        if ch == ':' {
            for dy in [hh * 0.3, hh * 0.72] {
                let c = pt(x, 1.6 * k, dy);
                p.rect_filled(Rect::from_center_size(c, Vec2::splat(t)), 0.0, lit_col);
            }
            x += 6.0 * k;
            continue;
        }
        if ch == '.' {
            p.rect_filled(Rect::from_center_size(pt(x, 1.2 * k, hh - t / 2.0), Vec2::splat(t)), 0.0, lit_col);
            x += 4.5 * k;
            continue;
        }
        let lit = match ch {
            '0' => "abcdef",
            '1' => "bc",
            '2' => "abged",
            '3' => "abgcd",
            '4' => "fgbc",
            '5' => "afgcd",
            '6' => "afgedc",
            '7' => "abc",
            '8' => "abcdefg",
            '9' => "abcfgd",
            '-' => "g",
            _ => "",
        };
        for s in ['a', 'b', 'c', 'd', 'e', 'f', 'g'] {
            let on = lit.contains(s);
            p.add(egui::Shape::convex_polygon(seg(x, s), if on { lit_col } else { ghost_col }, Stroke::NONE));
        }
        x += adv;
    }
    x - at.x
}

/// A bar of little lit blocks across `r` (how far in, the volume): lit up to `f` (0..1), the rest faint, a fine
/// outline round it.
pub(crate) fn blocks(p: &egui::Painter, r: Rect, f: f32) {
    let inner = r.shrink(2.0);
    let step = 6.0;
    let n = (inner.width() / step).floor().max(1.0) as usize;
    let lit = (f.clamp(0.0, 1.0) * n as f32).round() as usize;
    for i in 0..n {
        let b = Rect::from_min_size(Pos2::new(inner.min.x + i as f32 * step, inner.min.y), Vec2::new(step - 2.0, inner.height()));
        p.rect_filled(b, 0.0, if i < lit { ink() } else { ghost() });
    }
    p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, ink()));
}

/// A spectrum as LCD columns of blocks, the unlit blocks faintly there: one column per band.
pub(crate) fn bars(p: &egui::Painter, r: Rect, bands: &[f32], gap: f32, block: f32) {
    let n = bands.len().max(1);
    let w = (r.width() - gap * (n - 1) as f32) / n as f32;
    let rows = ((r.height() + 2.0) / (block + 2.0)).floor().max(2.0) as usize;
    for (i, v) in bands.iter().enumerate() {
        let x = r.min.x + i as f32 * (w + gap);
        let lit = (v.clamp(0.0, 1.0) * rows as f32).round() as usize;
        for s in 0..rows {
            let y = r.max.y - (s as f32 + 1.0) * (block + 2.0) + 2.0;
            p.rect_filled(Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, block)), 0.0, if s < lit { ink() } else { ghost() });
        }
    }
}

/// `text` scrolling right to left in dot type across `r` (whole and still when it fits).
pub(crate) fn marquee(ui: &egui::Ui, r: Rect, text: &str, size: f32, col: Color32) {
    let p = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
    let w = dot_w(ui.ctx(), text, size);
    if w <= r.width() {
        dot(&p, Pos2::new(r.min.x, r.center().y), egui::Align2::LEFT_CENTER, text, size, col);
        return;
    }
    let gap = dot_w(ui.ctx(), "   ***   ", size);
    let t = ui.input(|i| i.time) as f32;
    let off = (t * 28.0) % (w + gap);
    for k in 0..2 {
        let x = r.min.x - off + k as f32 * (w + gap);
        dot(&p, Pos2::new(x, r.center().y), egui::Align2::LEFT_CENTER, text, size, col);
        dot(&p, Pos2::new(x + w + gap / 2.0, r.center().y), egui::Align2::CENTER_CENTER, "***", size, col);
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
}

/// The song as the deck scrolls it: who, what, how long, and the key and tempo once they're known.
fn song_line(tr: &Track) -> String {
    let info = crate::meta::get(tr.id).unwrap_or_default();
    let mut s = format!("{} - {} ({})", tr.artists_text().to_uppercase(), tr.title.to_uppercase(), fmt_time(tr.duration));
    if info.key.is_some() {
        s += &format!("  * KEY {}", crate::meta::key_text(&info));
    }
    if let Some(b) = info.bpm {
        s += &format!("  * {} BPM", b);
    }
    s
}

impl App {
    /// Studio's deck: the cover, the time in LCD digits (a click: the time left), the song scrolling by in dot type
    /// with its key and tempo, the stream's details as little lit labels, the visualizer, how far in, the transport
    /// and Open, the volume; Shuffle, Repeat, EQ and PL with their lights, the balance.
    pub(crate) fn studio_deck(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        let id = ui.id().with("studio_deck");
        let tr = self.cur_track();
        let (bands, _, _) = self.viz_data(0);
        let s = sc();
        // ---- the cover, the time, the song, the stream
        let cs = (r.height() * 0.32).clamp(56.0, 92.0);
        let cover = Rect::from_min_size(r.min, Vec2::splat(cs));
        self.studio_cover(ui, cover, "studio_deck", 320);
        if ui.interact(cover, id.with("cover"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Now Playing  (A)").clicked() {
            acts.push(Action::ToggleArt);
        }
        let tx = cover.max.x + 10.0;
        let th = (cs * 0.42).clamp(24.0, 38.0);
        let time = self.deck_time();
        let shown = if time.starts_with('-') { time.clone() } else { format!(" {}", time) };
        let tw = digits(&p, Pos2::new(tx, r.min.y + th / 2.0 + 2.0), th, &shown, ink(), ghost());
        let time_r = Rect::from_min_size(Pos2::new(tx, r.min.y), Vec2::new(tw + 6.0, th + 4.0));
        if ui.interact(time_r, id.with("time"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Click for the time left").clicked() {
            self.time_left = !self.time_left;
            self.dirty = true;
        }
        let mq = Rect::from_min_max(Pos2::new(tx, time_r.max.y + 2.0), Pos2::new(r.max.x, time_r.max.y + 20.0 * s));
        if let Some(tr) = &tr {
            marquee(ui, mq, &song_line(tr), 15.0, ink());
        }
        // the stream's details, each a little label; Stereo (or Mono) lit
        let info = self.deck_info();
        let mut x = tx;
        let ly = mq.max.y + 9.0 * s;
        for part in info.split(" · ").filter(|t| !t.is_empty()) {
            let t = part.to_uppercase();
            let lit = t == "STEREO" || t == "MONO";
            let w = p.ctx().fonts(|f| f.layout_no_wrap(t.clone(), egui::FontId::new(7.0 * s, egui::FontFamily::Name("studio_px".into())), Color32::WHITE).size().x) + 8.0;
            if x + w > r.max.x {
                break;
            }
            let lr = Rect::from_min_size(Pos2::new(x, ly - 6.0 * s), Vec2::new(w, 12.0 * s));
            if lit {
                p.rect_filled(lr, 0.0, ink());
            }
            p.rect_stroke(lr, 0.0, Stroke::new(1.0_f32, if lit { ink() } else { dim() }));
            px(&p, lr.center() + Vec2::new(0.0, 0.5), egui::Align2::CENTER_CENTER, &t, 7.0, if lit { scr() } else { dim() });
            x = lr.max.x + 5.0;
        }
        // ---- the visualizer, how far in
        let mut y = cover.max.y.max(ly + 8.0 * s) + 8.0;
        if self.mini_viz {
            // (as tall as the room left over: what's under it takes 26 + 46 rows)
            let vh = (r.max.y - y - 8.0 - (26.0 + 46.0 * s)).clamp(24.0, 110.0);
            let vr = Rect::from_min_max(Pos2::new(r.min.x, y), Pos2::new(r.max.x, y + vh));
            let b32 = crate::viz::resample(&bands, 32, 1.0);
            bars(&p, vr, &b32, 2.0, 3.0);
            if self.cur.is_some() && !self.stopped && !self.paused {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
            }
            y = vr.max.y + 8.0;
        }
        let seek = Rect::from_min_max(Pos2::new(r.min.x, y), Pos2::new(r.max.x, y + 11.0));
        let dur = tr.as_ref().map_or(0.0, |t| t.duration);
        let pos = if self.cur.is_some() { self.pos() } else { 0.0 };
        let h = ui.interact(seek.expand2(Vec2::new(0.0, 4.0)), id.with("seek"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand);
        if (h.clicked() || h.dragged()) && dur > 0.0 {
            if let Some(pp) = h.interact_pointer_pos() {
                acts.push(Action::Seek(((pp.x - seek.min.x) / seek.width()).clamp(0.0, 1.0) * dur));
            }
        }
        blocks(&p, seek, if dur > 0.0 { pos / dur } else { 0.0 });
        y = seek.max.y + 9.0;
        // ---- the transport, Open, the volume
        let playing = self.cur.is_some() && !self.stopped && !self.paused;
        let keys = [
            ("prev", Action::Prev, "Previous", false),
            ("play", Action::PlayBtn, "Play", playing),
            ("pause", Action::PauseBtn, "Pause", self.paused),
            ("stop", Action::StopBtn, "Stop", false),
            ("skip", Action::Next, "Next", false),
            ("folder", Action::AddFiles, "Open files", false),
        ];
        let kh = 24.0 * s;
        let kw = 30.0 * s;
        for (n, (icon, a, tip, lit)) in keys.into_iter().enumerate() {
            let kr = Rect::from_min_size(Pos2::new(r.min.x + n as f32 * (kw + 3.0), y), Vec2::new(kw, kh));
            let h = ui.interact(kr, id.with(("key", n)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
            key(&p, kr, lit, h.is_pointer_button_down_on(), h.hovered());
            crate::vicon::named(&p, Rect::from_center_size(kr.center(), Vec2::splat(13.0 * s)), icon, key_fg(lit));
            if h.clicked() {
                acts.push(a);
            }
        }
        let vx = r.min.x + 6.0 * (kw + 3.0) + 8.0;
        px(&p, Pos2::new(vx, y + kh / 2.0), egui::Align2::LEFT_CENTER, "VOL", 7.0, dim());
        let vol = Rect::from_min_max(Pos2::new(vx + 24.0 * s, y + kh / 2.0 - 5.0), Pos2::new(r.max.x, y + kh / 2.0 + 5.0));
        let vh = ui.interact(vol.expand2(Vec2::new(0.0, 5.0)), id.with("vol"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(format!("Volume {}%", (self.volume * 100.0).round()));
        if vh.clicked() || vh.dragged() {
            if let Some(pp) = vh.interact_pointer_pos() {
                acts.push(Action::Volume(((pp.x - vol.min.x) / vol.width()).clamp(0.0, 1.0)));
            }
        }
        if vh.hovered() {
            let wheel = ui.input(|i| i.smooth_scroll_delta.y);
            if wheel != 0.0 {
                acts.push(Action::Volume((self.volume + wheel * 0.002).clamp(0.0, 1.0)));
            }
        }
        if vol.width() > 20.0 {
            blocks(&p, vol, self.volume);
        }
        y += kh + 6.0;
        // ---- Shuffle, Repeat, EQ, PL with their lights; the balance
        let rep_on = !matches!(self.repeat, crate::Repeat::Off);
        let toggles = [("SHUF", self.shuffle, "Shuffle"), ("REP", rep_on, "Repeat"), ("EQ", self.show_eq, "The equalizer"), ("PL", !self.queue_hidden, "The playlist")];
        let mut x = r.min.x;
        for (n, (name, on, tip)) in toggles.into_iter().enumerate() {
            let w = p.ctx().fonts(|f| f.layout_no_wrap(name.to_string(), egui::FontId::new(10.0 * s, egui::FontFamily::Name("studio_mono".into())), Color32::WHITE).size().x) + 22.0;
            let kr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, 22.0 * s));
            x = kr.max.x + 4.0;
            let h = ui.interact(kr, id.with(("tg", n)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
            key(&p, kr, false, h.is_pointer_button_down_on(), h.hovered());
            light(&p, Pos2::new(kr.min.x + 8.0, kr.center().y), on, ink());
            p.text(Pos2::new(kr.min.x + 15.0, kr.center().y), egui::Align2::LEFT_CENTER, name, egui::FontId::new(10.0 * s, egui::FontFamily::Name("studio_mono".into())), ink());
            if h.clicked() {
                match n {
                    0 => acts.push(Action::Shuffle),
                    1 => acts.push(Action::Repeat),
                    2 => acts.push(Action::ToggleEq),
                    _ => {
                        self.queue_hidden = !self.queue_hidden;
                        self.dirty = true;
                    }
                }
            }
        }
        let bx = x + 8.0;
        px(&p, Pos2::new(bx, y + 11.0 * s), egui::Align2::LEFT_CENTER, "BAL", 7.0, dim());
        let bal = Rect::from_min_max(Pos2::new(bx + 24.0 * s, y + 11.0 * s - 5.0), Pos2::new(r.max.x, y + 11.0 * s + 5.0));
        if bal.width() > 24.0 {
            let bh = ui.interact(bal.expand2(Vec2::new(0.0, 5.0)), id.with("bal"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Balance (double-click: centred)");
            if bh.double_clicked() {
                acts.push(Action::Balance(0));
            } else if bh.clicked() || bh.dragged() {
                if let Some(pp) = bh.interact_pointer_pos() {
                    let b = (((pp.x - bal.min.x) / bal.width()).clamp(0.0, 1.0) * 200.0 - 100.0) as i32;
                    acts.push(Action::Balance(if b.abs() < 8 { 0 } else { b }));
                }
            }
            p.rect_filled(bal.shrink(2.0), 0.0, ghost());
            p.rect_stroke(bal, 0.0, Stroke::new(1.0_f32, ink()));
            let x = bal.min.x + bal.width() * ((self.balance as f32 + 100.0) / 200.0);
            p.rect_filled(Rect::from_center_size(Pos2::new(x, bal.center().y), Vec2::new(3.0, bal.height() + 4.0)), 0.0, ink());
        }
    }

    /// A cover in Studio: in its own colours, or (picked on the Skins page) printed in the screen's ink.
    pub(crate) fn studio_cover(&mut self, ui: &egui::Ui, r: Rect, slot: &'static str, size: u32) {
        let p = ui.painter().clone();
        let tr = self.cur_track();
        let ink_cover = crate::skin::STUDIO_COVER.load(std::sync::atomic::Ordering::Relaxed) == 1;
        match tr.as_ref().map(|t| t.cover.clone()) {
            Some(c) if !c.is_empty() => {
                if ink_cover {
                    match self.studio_dither(ui.ctx(), &c) {
                        Some(id) => {
                            let mut m = egui::Mesh::with_texture(id);
                            m.add_rect_with_uv(r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                            p.add(m);
                        }
                        None => {
                            p.rect_filled(r, 0.0, ghost());
                        }
                    }
                } else {
                    crate::paint_held(ui, &mut self.images, slot, &crate::cover_url(&c, size), r, false);
                }
            }
            _ => {
                p.rect_filled(r, 0.0, ghost());
                crate::vicon::named(&p, r.shrink(r.width() * 0.3), "note", dim());
            }
        }
        p.rect_stroke(r.expand(2.0), 0.0, Stroke::new(1.0_f32, ink()));
    }

    /// The cover printed in the screen's ink: its picture squeezed to 128 by 128 and ordered-dithered into dots of ink
    /// on clear (made once per cover and screen colour, off the frame).
    fn studio_dither(&mut self, ctx: &egui::Context, cover: &str) -> Option<egui::TextureId> {
        let url = format!("thumb:{}", crate::api::cover_url(cover, 160));
        let key = format!("{}|{}", url, crate::skin::STUDIO_SCREEN.load(std::sync::atomic::Ordering::Relaxed));
        if self.studio_dith.0 == key {
            return self.studio_dith.1.as_ref().map(|t| t.id());
        }
        if self.studio_dith_busy == key {
            return None;
        }
        self.studio_dith_busy = key.clone();
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), ctx.clone());
        let inkc = ink();
        std::thread::spawn(move || {
            let Some(img) = crate::picture(&api, &url) else { return };
            let _ = tx.send(Msg::StudioDither(key, dither(&img, inkc)));
            ctx.request_repaint();
        });
        None
    }

    /// Studio's Now Playing: one big screen with tabs on its frame. With none picked: the cover, the song in big dot
    /// type, who plays it and the album, key, tempo, tuning and loops in little boxes, what's next, the spectrum.
    /// Lyrics: the cover, song and time small at the left, the words in dot type, the line being sung inverted. Up
    /// Next, About and Loops as the other styles have them. Visuals: drawn in the ink (bars, a scope; the terrain only
    /// in dev builds). Practising: the waveform big, its keys, the practice tabs under it.
    pub(crate) fn np_studio(&mut self, ui: &mut egui::Ui, area: Rect, dt: f32, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        let prac = practice_tools() && self.practice;
        let r = area.shrink2(Vec2::new(4.0, 10.0));
        frame(&p, r, if prac { "Practice" } else { "Now Playing" });
        if prac {
            self.studio_practice(ui, r.shrink(14.0), acts);
            return;
        }
        let tabs = [
            ("Lyrics", self.studio_np == 1, "The lyrics"),
            ("Up Next", self.studio_np == 2, "What plays next"),
            ("About", self.studio_np == 3, "About the song"),
            ("Loops", self.studio_np == 4, "Its loops and bookmarks"),
            ("Visuals", self.studio_np == 5, "The visuals, in the screen's ink"),
        ];
        if let Some(i) = frame_tabs(ui, r, &tabs, "studio_np_tab") {
            let v = i as u8 + 1;
            self.studio_np = if self.studio_np == v { 0 } else { v };
            self.dirty = true;
        }
        let inner = r.shrink(18.0);
        match self.studio_np {
            1 => self.studio_lyrics(ui, inner, dt, acts),
            2 => self.style_queue(ui, inner, acts),
            3 => self.np_about(ui, &p, inner, ink()),
            4 => self.np_loops(ui, &p, inner, acts, ink()),
            5 => self.studio_visuals(ui, inner),
            _ => self.studio_np_main(ui, inner),
        }
    }

    /// The main view: the cover, the song big, the rest small.
    fn studio_np_main(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let s = sc();
        let cs = r.height().min(r.width() * 0.44).max(80.0);
        let cover = Rect::from_min_size(Pos2::new(r.min.x, r.center().y - cs / 2.0), Vec2::splat(cs));
        self.studio_cover(ui, cover, "studio_np", 640);
        let x0 = cover.max.x + 26.0;
        let right = Rect::from_min_max(Pos2::new(x0, r.min.y), r.max);
        let Some(tr) = self.cur_track() else {
            dot(&p, right.left_center(), egui::Align2::LEFT_CENTER, "NOTHING PLAYING", 30.0, dim());
            return;
        };
        let big = (right.height() / 10.0).clamp(22.0, 44.0) / s;
        let line = Rect::from_min_max(Pos2::new(x0, r.min.y + 4.0), Pos2::new(r.max.x, r.min.y + 4.0 + big * s * 1.3));
        marquee(ui, line, &format!("{} - {}", tr.title.to_uppercase(), tr.artists_text().to_uppercase()), big, ink());
        let mut y = line.max.y + 10.0;
        let who = if tr.album.is_empty() { tr.artists_text() } else { format!("{} · {}", tr.artists_text(), tr.album) };
        px(&p, Pos2::new(x0, y), egui::Align2::LEFT_CENTER, &who.to_uppercase(), 8.0, dim());
        y += 22.0 * s;
        // what's known of it, each in a little box
        // (key, tempo and loops always have a box, dim dashes while unknown, like an unlit readout; the tuning only
        // when it's been measured)
        let info = crate::meta::get(tr.id).unwrap_or_default();
        let mut boxes: Vec<(String, bool)> = Vec::new();
        boxes.push(match info.key {
            Some(_) => (format!("KEY {}", crate::meta::key_text(&info)), true),
            None => ("KEY --".into(), false),
        });
        boxes.push(match info.bpm {
            Some(b) => (format!("{} BPM", b), true),
            None => ("--- BPM".into(), false),
        });
        if let Some(a) = info.a4 {
            boxes.push((format!("A = {:.0}", a), true));
        }
        let nl = self.store.sections.get(&tr.id).map_or(0, |l| l.len());
        boxes.push((format!("{} LOOP{}", nl, if nl == 1 { "" } else { "S" }), nl > 0));
        let mono = egui::FontId::new(12.0 * s, egui::FontFamily::Name("studio_mono".into()));
        let mut x = x0;
        for (b, known) in &boxes {
            let col = if *known { ink() } else { dim() };
            let w = p.ctx().fonts(|f| f.layout_no_wrap(b.clone(), mono.clone(), Color32::WHITE).size().x) + 16.0;
            let br = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, 24.0 * s));
            p.rect_stroke(br, 0.0, Stroke::new(1.0_f32, col));
            p.text(br.center(), egui::Align2::CENTER_CENTER, b, mono.clone(), col);
            x = br.max.x + 8.0;
        }
        y += 40.0 * s;
        // what's next: the next few songs
        let bars_top = r.max.y - (right.height() * 0.3).clamp(52.0 * s, 200.0);
        let from = self.cur.map_or(0, |c| c + 1);
        let next: Vec<Track> = self.queue.iter().skip(from).take(4).cloned().collect();
        if !next.is_empty() {
            p.line_segment([Pos2::new(x0, y), Pos2::new(r.max.x, y)], Stroke::new(1.0_f32, ink()));
            px(&p, Pos2::new(x0, y + 12.0 * s), egui::Align2::LEFT_CENTER, "UP NEXT", 7.0, dim());
            let mut ny = y + 32.0 * s;
            for n in &next {
                if ny > bars_top - 14.0 * s {
                    break;
                }
                let nr = Rect::from_min_max(Pos2::new(x0, ny - 11.0 * s), Pos2::new(r.max.x - 64.0 * s, ny + 11.0 * s));
                marquee(ui, nr, &format!("{} · {}", n.title.to_uppercase(), n.artists_text().to_uppercase()), 15.0, ink());
                dot(&p, Pos2::new(r.max.x, ny), egui::Align2::RIGHT_CENTER, &fmt_time(n.duration), 15.0, ink());
                ny += 26.0 * s;
            }
        }
        // the spectrum along the foot
        let (bands, _, _) = self.viz_data(1);
        let br = Rect::from_min_max(Pos2::new(x0, bars_top), r.max);
        bars(&p, br, &crate::viz::resample(&bands, 32, 1.0), 3.0, 3.0);
        if self.cur.is_some() && !self.stopped && !self.paused {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        }
    }

    /// The lyrics: the cover, the song and the time small at the left; the words in dot type, all one size, the line
    /// being sung inverted; the wheel scrolls them a while, a click on a line jumps there.
    fn studio_lyrics(&mut self, ui: &mut egui::Ui, r: Rect, dt: f32, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        let s = sc();
        let lw = (r.width() * 0.26).clamp(150.0, 240.0);
        let cover = Rect::from_min_size(r.min, Vec2::splat(lw));
        self.studio_cover(ui, cover, "studio_np", 640);
        if let Some(tr) = self.cur_track() {
            let mq = Rect::from_min_size(Pos2::new(r.min.x, cover.max.y + 14.0), Vec2::new(lw, 22.0 * s));
            marquee(ui, mq, &format!("{} - {}", tr.title.to_uppercase(), tr.artists_text().to_uppercase()), 16.0, ink());
            let time = self.deck_time();
            let shown = if time.starts_with('-') { time } else { format!(" {}", time) };
            digits(&p, Pos2::new(r.min.x, mq.max.y + 26.0 * s), 34.0 * s, &shown, ink(), ghost());
        }
        let lr = Rect::from_min_max(Pos2::new(r.min.x + lw + 30.0, r.min.y), r.max);
        let cp = p.with_clip_rect(lr);
        let size = 22.0;
        let lh = size * s * 1.6;
        let cy = lr.center().y;
        let hover = ui.input(|i| i.pointer.hover_pos()).filter(|pp| lr.contains(*pp));
        let wheel = if hover.is_some() { ui.input(|i| i.raw_scroll_delta.y) } else { 0.0 };
        let track_id = self.cur_track().map(|t| t.id);
        let pos = self.pos();
        let msg = match &self.lyrics {
            None => Some("LYRICS SHOW HERE"),
            Some(l) if track_id != Some(l.id) || l.status == 0 => Some("LOADING LYRICS..."),
            Some(l) if l.status == 2 || l.lines.is_empty() => Some("NO LYRICS FOR THIS SONG"),
            _ => None,
        };
        if let Some(m) = msg {
            dot(&cp, Pos2::new(lr.min.x, cy), egui::Align2::LEFT_CENTER, m, size, dim());
            return;
        }
        let Some(l) = self.lyrics.as_ref() else { return };
        let n = l.lines.len();
        let idx: Option<usize> = if l.synced { l.lines.iter().rposition(|(tt, _)| *tt <= pos + 0.25) } else { None };
        self.lyric_free = (self.lyric_free - dt).max(0.0);
        let mut scroll = self.lyric_scroll;
        if wheel != 0.0 {
            scroll = (scroll - wheel / lh).within(0.0, (n.max(1) - 1) as f32);
            self.lyric_free = 4.0;
        } else if l.synced && self.lyric_free <= 0.0 {
            scroll += (idx.unwrap_or(0) as f32 - scroll) * (1.0 - (-dt * 7.0).exp());
        }
        let mut hover_i = None;
        if let (true, Some(pp)) = (l.synced, hover) {
            let k = (scroll + (pp.y - cy) / lh).round();
            if k >= 0.0 && (k as usize) < n && !l.lines[k as usize].1.is_empty() {
                hover_i = Some(k as usize);
            }
        }
        let resp = ui.interact(lr, ui.id().with("studio_lyrics"), Sense::click());
        if hover_i.is_some() {
            resp.clone().on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        if let (true, Some(i)) = (resp.clicked(), hover_i) {
            acts.push(Action::Seek(l.lines[i].0));
            self.lyric_free = 0.0;
        }
        for (i, (_, txt)) in l.lines.iter().enumerate() {
            if txt.is_empty() {
                continue;
            }
            let y = cy + (i as f32 - scroll) * lh;
            if y < lr.min.y - lh || y > lr.max.y + lh {
                continue;
            }
            let now = idx == Some(i);
            if now {
                cp.rect_filled(Rect::from_min_max(Pos2::new(lr.min.x, y - lh / 2.0 + 3.0), Pos2::new(lr.max.x, y + lh / 2.0 - 3.0)), 0.0, ink());
            } else if hover_i == Some(i) {
                cp.rect_filled(Rect::from_min_max(Pos2::new(lr.min.x, y - lh / 2.0 + 3.0), Pos2::new(lr.max.x, y + lh / 2.0 - 3.0)), 0.0, ghost());
            }
            let col = if now { scr() } else if l.synced { dim() } else { ink() };
            dot(&cp, Pos2::new(lr.min.x + 12.0, y), egui::Align2::LEFT_CENTER, txt, size, col);
        }
        self.lyric_scroll = scroll;
    }

    /// The visuals in the screen's ink: LCD bars, or a scope (the sound's own wave, steadied on a rising edge so it
    /// stands still), or (dev builds only) the terrain, the last few seconds of the spectrum as a wire grid running
    /// away from you. Three small keys at the top right pick.
    fn studio_visuals(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let s = sc();
        let dev = crate::update::alpha() > 0;
        let names: Vec<&str> = if dev { vec!["Bars", "Scope", "Terrain"] } else { vec!["Bars", "Scope"] };
        let cur = (self.studio_vis as usize).min(names.len() - 1);
        let mut x = r.max.x;
        for (i, n) in names.iter().enumerate().rev() {
            let w = p.ctx().fonts(|f| f.layout_no_wrap(n.to_uppercase(), egui::FontId::new(10.0 * s, egui::FontFamily::Name("studio_mono".into())), Color32::WHITE).size().x) + 16.0;
            let kr = Rect::from_min_size(Pos2::new(x - w, r.min.y), Vec2::new(w, 22.0 * s));
            x = kr.min.x - 4.0;
            let h = ui.interact(kr, ui.id().with(("studio_vis", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            key(&p, kr, i == cur, h.is_pointer_button_down_on(), h.hovered());
            p.text(kr.center(), egui::Align2::CENTER_CENTER, n.to_uppercase(), egui::FontId::new(10.0 * s, egui::FontFamily::Name("studio_mono".into())), key_fg(i == cur));
            if h.clicked() {
                self.studio_vis = i as u8;
                self.dirty = true;
            }
        }
        let body = Rect::from_min_max(Pos2::new(r.min.x, r.min.y + 34.0 * s), r.max);
        let playing = self.cur.is_some() && !self.stopped && !self.paused;
        if playing {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        }
        match cur {
            1 => {
                // the scope: the newest moment of sound, both sides mixed, started where it rises through zero
                let wave: Vec<f32> = match self.player.viz.lock() {
                    Ok(v) if playing => {
                        let n = v.samples.len().min(v.first.len());
                        (0..n).map(|i| (v.samples[v.samples.len() - n + i] + v.first[v.first.len() - n + i]) * 0.5).collect()
                    }
                    _ => Vec::new(),
                };
                let mid = body.center().y;
                p.line_segment([Pos2::new(body.min.x, mid), Pos2::new(body.max.x, mid)], Stroke::new(1.0_f32, ghost()));
                let show = 900usize;
                if wave.len() > show + 2 {
                    let search = wave.len() - show - 1;
                    let start = (search.saturating_sub(1200)..search).rev().find(|&i| wave[i] <= 0.0 && wave[i + 1] > 0.0).unwrap_or(search);
                    let peak = wave[start..start + show].iter().fold(0.02f32, |m, v| m.max(v.abs()));
                    let gain = (0.42 / peak).min(6.0);
                    let pts: Vec<Pos2> = (0..show)
                        .step_by(2)
                        .map(|i| Pos2::new(body.min.x + body.width() * i as f32 / show as f32, mid - wave[start + i] * gain * body.height()))
                        .collect();
                    p.add(egui::Shape::line(pts, Stroke::new(2.0_f32, ink())));
                }
            }
            2 => self.studio_terrain(&p, body),
            _ => {
                let (bands, _, _) = self.viz_data(1);
                bars(&p, body, &crate::viz::resample(&bands, 32, 1.0), 6.0, 6.0);
            }
        }
    }

    /// The terrain (dev builds only): each moment's spectrum a row, the newest at the front, rows running back into
    /// the distance; drawn back to front, each row hiding what's behind it, as a wire grid in the ink.
    fn studio_terrain(&mut self, p: &egui::Painter, r: Rect) {
        const ROWS: usize = 26;
        const COLS: usize = 40;
        let (bands, _, _) = self.viz_data(1);
        let now = crate::viz::resample(&bands, COLS, 1.0);
        let t = p.ctx().input(|i| i.time);
        if t - self.studio_terr_at > 0.08 || self.studio_terr.is_empty() {
            self.studio_terr_at = t;
            self.studio_terr.insert(0, now);
            self.studio_terr.truncate(ROWS);
        }
        let rows = &self.studio_terr;
        let pt = |i: usize, j: usize, v: f32| {
            // j: 0 the front row; deeper rows narrower and higher up
            let z = 1.0 - j as f32 / (ROWS - 1) as f32;
            let persp = 0.28 + 0.72 * z;
            let x = r.center().x + (i as f32 / (COLS - 1) as f32 - 0.5) * r.width() * 1.05 * persp;
            let base = r.min.y + r.height() * (0.18 + 0.78 * z);
            Pos2::new(x, base - v * r.height() * 0.42 * persp)
        };
        for j in (0..rows.len()).rev() {
            let row = &rows[j];
            let pts: Vec<Pos2> = (0..COLS).map(|i| pt(i, j, row.get(i).copied().unwrap_or(0.0))).collect();
            // (the ground under the line, in the glass's colour, hides the rows behind)
            let mut m = egui::Mesh::default();
            for q in &pts {
                m.colored_vertex(*q, scr());
                m.colored_vertex(Pos2::new(q.x, r.max.y), scr());
            }
            for i in 0..COLS - 1 {
                let a = (2 * i) as u32;
                m.add_triangle(a, a + 2, a + 1);
                m.add_triangle(a + 1, a + 2, a + 3);
            }
            p.with_clip_rect(r).add(m);
            let w = 0.6 + 1.2 * (1.0 - j as f32 / ROWS as f32);
            p.with_clip_rect(r).add(egui::Shape::line(pts.clone(), Stroke::new(w, ink())));
            if j + 1 < rows.len() {
                let back = &rows[j + 1];
                for i in (0..COLS).step_by(3) {
                    p.with_clip_rect(r).line_segment([pts[i], pt(i, j + 1, back.get(i).copied().unwrap_or(0.0))], Stroke::new(0.6_f32, ink().gamma_multiply(0.6)));
                }
            }
        }
    }

    /// Practising in Now Playing: the cover, the song in dot type and the time in digits, then the practice layout every
    /// style shares (`practice_stack`).
    fn studio_practice(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        let s = sc();
        // the cover small, the song, what's known of it
        let head = Rect::from_min_size(r.min, Vec2::new(r.width(), 56.0 * s));
        let cover = Rect::from_min_size(head.min, Vec2::splat(head.height()));
        self.studio_cover(ui, cover, "studio_np", 160);
        if let Some(tr) = self.cur_track() {
            let mq = Rect::from_min_max(Pos2::new(cover.max.x + 14.0, head.min.y), Pos2::new(head.max.x - 160.0 * s, head.min.y + 24.0 * s));
            marquee(ui, mq, &format!("{} - {}", tr.title.to_uppercase(), tr.artists_text().to_uppercase()), 18.0, ink());
            let info = crate::meta::get(tr.id).unwrap_or_default();
            let mut parts: Vec<String> = Vec::new();
            if info.key.is_some() {
                parts.push(format!("KEY {}", crate::meta::key_text(&info)));
            }
            if let Some(b) = info.bpm {
                parts.push(format!("{} BPM", b));
            }
            if let Some(a) = info.a4 {
                parts.push(format!("A = {:.0}", a));
            }
            px(&p, Pos2::new(mq.min.x, head.max.y - 10.0 * s), egui::Align2::LEFT_CENTER, &parts.join("  ·  "), 8.0, dim());
            let time = self.deck_time();
            let shown = if time.starts_with('-') { time } else { format!(" {}", time) };
            let tw = digits(&p, Pos2::new(head.max.x - 150.0 * s, head.center().y), 40.0 * s, &shown, ink(), ghost());
            let tr_r = Rect::from_min_size(Pos2::new(head.max.x - 150.0 * s, head.min.y), Vec2::new(tw + 6.0, head.height()));
            if ui.interact(tr_r, ui.id().with("studio_prac_time"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Click for the time left").clicked() {
                self.time_left = !self.time_left;
                self.dirty = true;
            }
        }
        // the waveform (the whole song above it), then the practice tabs and their tools: the same in every style
        let body = Rect::from_min_max(Pos2::new(r.min.x, head.max.y + 12.0), r.max);
        self.practice_stack(ui, body, acts, true);
    }
}

/// A picture ordered-dithered into dots of `ink` on clear (128 by 128).
pub(crate) fn dither(img: &egui::ColorImage, ink: Color32) -> egui::ColorImage {
    const N: usize = 128;
    const BAYER: [u8; 64] = [
        0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36, 14, 46, 6, 38, 60, 28, 52, 20, 62, 30, 54, 22, 3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27, 49, 17, 57, 25,
        15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29, 53, 21,
    ];
    let [w, h] = img.size;
    let mut px = vec![Color32::TRANSPARENT; N * N];
    for y in 0..N {
        for x in 0..N {
            // (a box average of the picture under this dot, its brightness lifted a little so mid-greys keep detail)
            let (x0, x1) = (x * w / N, ((x + 1) * w / N).max(x * w / N + 1).min(w));
            let (y0, y1) = (y * h / N, ((y + 1) * h / N).max(y * h / N + 1).min(h));
            let (mut sum, mut n) = (0.0f32, 0.0f32);
            for yy in y0..y1 {
                for xx in x0..x1 {
                    let c = img.pixels[yy * w + xx];
                    sum += 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
                    n += 1.0;
                }
            }
            let g = (sum / n.max(1.0) / 255.0).powf(0.85);
            let t = (BAYER[(y % 8) * 8 + (x % 8)] as f32 + 0.5) / 64.0;
            if g < t {
                px[y * N + x] = ink;
            }
        }
    }
    egui::ColorImage { size: [N, N], pixels: px }
}
