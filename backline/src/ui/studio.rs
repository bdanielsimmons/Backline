//! Studio: Tidalite's look grown into a Backline style (the owner's picks of 2026-11-25, style sheets 7 and 8). It's
//! Hardware's relative: everything Hardware has, Studio has (`hardware()` is true for both), drawn its own way. Every
//! part is a screen (cream, gray, blue or amber) set in a black body and named in brackets on its frame; keys have a
//! fine outline; the time is LCD digits with the unlit segments faintly there; the song scrolls by in dot type; lists
//! are in a typewriter face. Its Now Playing is one big screen with tabs on its frame (Lyrics, Up Next, About, Loops,
//! Visuals), the visuals drawn in the screen's ink (bars, a scope, the terrain).

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
    let faces: [(&str, &'static [u8]); 5] = [
        ("studio_px", include_bytes!("../../assets/Silkscreen-Regular.ttf")),
        ("studio_dot", include_bytes!("../../assets/Doto.ttf")),
        ("studio_mono", include_bytes!("../../assets/IBMPlexMono-Regular.ttf")),
        // (Hardware's screens: Scanline and Terminal, `styles::screen_text`)
        ("hw_scan", include_bytes!("../../assets/Sixtyfour.ttf")),
        ("hw_term", include_bytes!("../../assets/VT323-Regular.ttf")),
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
        let size = 10.0;
        let w = p.ctx().fonts(|f| f.layout_no_wrap(t.clone(), egui::FontId::new(size * sc(), egui::FontFamily::Name("studio_px".into())), Color32::WHITE).size().x);
        // (a frame too narrow for its name, as the folded side list: no name rather than half of one)
        if w + 20.0 > r.width() {
            return;
        }
        let at = Pos2::new(r.min.x + 10.0, r.min.y - 1.0);
        p.rect_filled(Rect::from_min_size(Pos2::new(at.x - 3.0, at.y - 7.0 * sc()), Vec2::new(w + 6.0, 14.0 * sc())), 0.0, BODY);
        px(p, at, egui::Align2::LEFT_CENTER, &t, size, label());
    }
}

/// Tabs on a screen's frame, at its top right (the window's switches, Now Playing's views): the one that's on in the
/// glass's colour. As big as the frame's width allows (a narrow frame: smaller). Returns the one clicked.
pub(crate) fn frame_tabs(ui: &mut egui::Ui, r: Rect, items: &[(&str, bool, &str)], salt: &str) -> Option<usize> {
    let p = ui.painter().clone();
    let widths = |size: f32| -> Vec<f32> {
        let font = egui::FontId::new(size * sc(), egui::FontFamily::Name("studio_px".into()));
        items.iter().map(|(t, _, _)| ui.fonts(|f| f.layout_no_wrap(t.to_uppercase(), font.clone(), Color32::WHITE).size().x) + size * 1.2).collect()
    };
    let gaps = 3.0 * (items.len().saturating_sub(1)) as f32;
    let mut size = 13.0;
    while size > 8.0 && widths(size).iter().sum::<f32>() + gaps > r.width() - 24.0 {
        size -= 0.5;
    }
    let ws = widths(size);
    let mut x = r.max.x - 4.0 - ws.iter().sum::<f32>() - gaps;
    let h = (size + 7.0) * sc();
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

/// The time big in LCD digits, `h` tall from `at` (left middle), and the song's length after it, small, at the digits'
/// foot ("/ 6:23"), as Studio shows the time wherever it's big. Returns how wide, the length included.
pub(crate) fn time_digits(p: &egui::Painter, at: Pos2, h: f32, shown: &str, dur: f32) -> f32 {
    let tw = digits(p, at, h, shown, ink(), ghost());
    if dur <= 0.0 {
        return tw;
    }
    // (the length in the same digits, half the size, after a slash, on the big digits' foot)
    let sh = h * 0.5;
    let foot = at.y + h * 0.44;
    let x0 = at.x + tw + h * 0.1;
    p.line_segment([Pos2::new(x0, foot), Pos2::new(x0 + sh * 0.34, foot - sh * 0.88)], Stroke::new((h * 0.06).max(1.4), ink()));
    let lw = digits(p, Pos2::new(x0 + sh * 0.5, foot - sh * 0.44), sh, &fmt_time(dur), ink(), ghost());
    tw + h * 0.1 + sh * 0.5 + lw
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

/// A spectrum as LCD columns of blocks, the unlit blocks faintly there: one column per band. Over each, its cap as
/// Classic draws it: a thin line a hair above the bar, pushed up with it and falling back at a steady pace (`peaks`).
pub(crate) fn bars(p: &egui::Painter, r: Rect, bands: &[f32], peaks: &[f32], gap: f32, block: f32) {
    let n = bands.len().max(1);
    let w = (r.width() - gap * (n - 1) as f32) / n as f32;
    let step = block + 2.0;
    let rows = ((r.height() + 2.0) / step).floor().max(2.0) as usize;
    let full = rows as f32 * step - 2.0;
    // (thinner than a block: a line on the small ones, a little thicker on Now Playing's big ones)
    let thick = (block / 2.0).floor().max(1.0);
    for (i, v) in bands.iter().enumerate() {
        let x = r.min.x + i as f32 * (w + gap);
        let lit = (v.clamp(0.0, 1.0) * rows as f32).round() as usize;
        for s in 0..rows {
            let y = r.max.y - (s as f32 + 1.0) * step + 2.0;
            p.rect_filled(Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, block)), 0.0, if s < lit { ink() } else { ghost() });
        }
        // the cap: never lower than the bar's top, with a 2 px gap between them
        let bar_h = if lit > 0 { lit as f32 * step - 2.0 } else { 0.0 };
        let peak_h = peaks.get(i).map_or(0.0, |k| k.clamp(0.0, 1.0) * full).round();
        let lift = bar_h.max(peak_h);
        if lift > 1.0 {
            let bottom = (r.max.y - lift - 2.0).max(r.min.y + thick);
            p.rect_filled(Rect::from_min_max(Pos2::new(x, bottom - thick), Pos2::new(x + w, bottom)), 0.0, ink());
        }
    }
}

/// `text` scrolling right to left in dot type across `r` (whole and still when it fits).
pub(crate) fn marquee(ui: &egui::Ui, r: Rect, text: &str, size: f32, col: Color32) {
    let p = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
    crate::test_hooks::clip_ok(p.clip_rect());
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
    let album = if tr.album.is_empty() { String::new() } else { format!(" · {}", tr.album.to_uppercase()) };
    let mut s = format!("{} - {}{} ({})", tr.artists_text().to_uppercase(), tr.title.to_uppercase(), album, fmt_time(tr.duration));
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
        let (bands, peaks, _) = self.viz_data(0);
        let s = sc();
        // ---- the cover, the time, the song, the stream
        let cs = (r.height() * 0.32).clamp(56.0 * s, 92.0 * s);
        let cover = Rect::from_min_size(r.min, Vec2::splat(cs));
        self.studio_cover(ui, cover, "studio_deck", 320);
        if ui.interact(cover, id.with("cover"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Now Playing  (A)").clicked() {
            acts.push(Action::ToggleArt);
        }
        let tx = cover.max.x + 10.0;
        let th = (cs * 0.42).clamp(24.0, 38.0);
        let time = self.deck_time();
        let shown = if time.starts_with('-') { time.clone() } else { format!(" {}", time) };
        let dur_now = tr.as_ref().map_or(0.0, |t| t.duration);
        let tw = time_digits(&p, Pos2::new(tx, r.min.y + th / 2.0 + 2.0), th, &shown, dur_now);
        let time_r = Rect::from_min_size(Pos2::new(tx, r.min.y), Vec2::new(tw + 6.0, th + 4.0));
        if ui.interact(time_r, id.with("time"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Elapsed / remaining").clicked() {
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
        let ly = mq.max.y + 10.0 * s;
        for part in info.split(" · ").filter(|t| !t.is_empty()) {
            let t = part.to_uppercase();
            let lit = t == "STEREO" || t == "MONO";
            let w = p.ctx().fonts(|f| f.layout_no_wrap(t.clone(), egui::FontId::new(8.5 * s, egui::FontFamily::Name("studio_px".into())), Color32::WHITE).size().x) + 8.0;
            if x + w > r.max.x {
                break;
            }
            let lr = Rect::from_min_size(Pos2::new(x, ly - 7.0 * s), Vec2::new(w, 14.0 * s));
            if lit {
                p.rect_filled(lr, 0.0, ink());
            }
            p.rect_stroke(lr, 0.0, Stroke::new(1.0_f32, if lit { ink() } else { dim() }));
            px(&p, lr.center() + Vec2::new(0.0, 0.5), egui::Align2::CENTER_CENTER, &t, 8.5, if lit { scr() } else { dim() });
            x = lr.max.x + 5.0;
        }
        // ---- the visualizer, how far in
        let mut y = cover.max.y.max(ly + 8.0 * s) + 8.0;
        if self.mini_viz {
            // (as tall as the room left over: what's under it takes 26 + 46 rows)
            let vh = (r.max.y - y - 8.0 - (26.0 * s + 46.0 * s)).clamp(24.0, 400.0);
            let vr = Rect::from_min_max(Pos2::new(r.min.x, y), Pos2::new(r.max.x, y + vh));
            let b32 = crate::viz::resample(&bands, 32, 1.0);
            bars(&p, vr, &b32, &crate::viz::resample(&peaks, 32, 1.0), 2.0, 3.0);
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
        if dur > 0.0 {
            crate::widgets::seek_label(ui, &h, seek, |x| ((x - seek.min.x) / seek.width()).clamp(0.0, 1.0) * dur);
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
        px(&p, Pos2::new(vx, y + kh / 2.0), egui::Align2::LEFT_CENTER, "VOL", 8.5, dim());
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
        let toggles = [("SHUF", self.shuffle, "Shuffle"), ("REP", rep_on, "Repeat"), ("EQ", self.show_eq, "Equalizer"), ("PL", !self.queue_hidden, "Playlist")];
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
        px(&p, Pos2::new(bx, y + 11.0 * s), egui::Align2::LEFT_CENTER, "BAL", 8.5, dim());
        let bal = Rect::from_min_max(Pos2::new(bx + 24.0 * s, y + 11.0 * s - 5.0), Pos2::new(r.max.x, y + 11.0 * s + 5.0));
        if bal.width() > 24.0 {
            let bh = ui.interact(bal.expand2(Vec2::new(0.0, 5.0)), id.with("bal"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Balance · Double-click to center");
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
        let now = ui.input(|i| i.time);
        // (just switched between color and ink: the one switched to fades in over the other)
        let sw = ((now - self.studio_cover_at) / 0.35).clamp(0.0, 1.0) as f32;
        if sw < 1.0 {
            ui.ctx().request_repaint();
        }
        let quad = |id: egui::TextureId, a: f32| {
            let mut m = egui::Mesh::with_texture(id);
            m.add_rect_with_uv(r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::from_white_alpha((a * 255.0) as u8));
            p.add(m);
        };
        match tr.as_ref().map(|t| t.cover.clone()) {
            Some(c) if !c.is_empty() => {
                let url = crate::cover_url(&c, size);
                // (dotted at the size it's shown: a dot to a screen pixel)
                let dith = if ink_cover || sw < 1.0 {
                    let px = (r.width() * ui.ctx().pixels_per_point()).round().clamp(32.0, 1280.0) as usize;
                    self.studio_dither(ui.ctx(), slot, &c, px)
                } else {
                    None
                };
                if sw >= 1.0 && !ink_cover {
                    crate::paint_held(ui, &mut self.images, slot, &url, r, false);
                } else if sw >= 1.0 {
                    match dith {
                        Some(id) => quad(id, 1.0),
                        None => {
                            p.rect_filled(r, 0.0, ghost());
                        }
                    }
                } else {
                    let color = self.images.held(slot, &url, false, now).0;
                    let (under, over) = if ink_cover { (color, dith) } else { (dith, color) };
                    p.rect_filled(r, 0.0, ghost());
                    if let Some(id) = under {
                        quad(id, 1.0);
                    }
                    if let Some(id) = over {
                        quad(id, sw);
                    }
                }
            }
            _ => {
                p.rect_filled(r, 0.0, ghost());
                crate::vicon::named(&p, r.shrink(r.width() * 0.3), "note", dim());
            }
        }
        p.rect_stroke(r.expand(2.0), 0.0, Stroke::new(1.0_f32, ink()));
    }

    /// The cover printed in the screen's ink for one place (`slot`), `px` screen pixels across: the whole cover dotted
    /// in the ink, a dot to a pixel (made off the frame, again when the size or the screen colour changes; meanwhile
    /// the last one of the same cover shows).
    fn studio_dither(&mut self, ctx: &egui::Context, slot: &'static str, cover: &str, px: usize) -> Option<egui::TextureId> {
        let url = crate::api::cover_url(cover, if px > 640 { 1280 } else { 640 });
        let (_, glass, _, inkc, _) = crate::skin::studio_screen();
        let same = format!("{}|{:?}{:?}", cover, glass, inkc);
        let key = format!("{}|{}", same, px);
        // (a new cover dissolving in over the last: a step of it every frame)
        if let Some((old, new, t0)) = self.studio_dith_anim.get(slot).cloned() {
            let t = (((ctx.input(|i| i.time) - t0) / 0.5) as f32).clamp(0.0, 1.0);
            if let Some(h) = self.studio_dith.get_mut(slot) {
                if t >= 1.0 || old.size != new.size {
                    h.2.set((*new).clone(), egui::TextureOptions::NEAREST);
                    self.studio_dith_anim.remove(slot);
                } else {
                    h.2.set(dissolve(&old, &new, t * t * (3.0 - 2.0 * t)), egui::TextureOptions::NEAREST);
                    ctx.request_repaint();
                }
            }
        }
        // (the one this place last had stays up, even another song's, until the new one is made and dissolves in)
        let held = self.studio_dith.get(slot).map(|h| (h.0 == same && h.1 == key, h.2.id()));
        if let Some((true, id)) = held {
            return Some(id);
        }
        let now = ctx.input(|i| i.time);
        let wait = match self.studio_dith_busy.get(slot) {
            Some((k, _)) if *k == key => true,
            // (resized: the one being made finishes first, unless it's taking a while)
            Some((k, t)) => k.starts_with(&format!("{}|", same)) && now - t < 0.5,
            None => false,
        };
        if !wait {
            self.studio_dith_busy.insert(slot, (key.clone(), now));
            let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), ctx.clone());
            let inkc = ink();
            // (a screen whose ink is lighter than its glass - Amber, Blue - prints the picture's light parts in ink, so
            // it isn't a negative)
            let lum = |c: Color32| 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
            let light = lum(inkc) > lum(scr());
            std::thread::spawn(move || {
                let Some(img) = crate::picture(&api, &url) else { return };
                let _ = tx.send(Msg::StudioDither(slot, key, dither(&img, inkc, px, light)));
                ctx.request_repaint();
            });
        }
        held.map(|h| h.1)
    }

    /// Studio's Now Playing: one big screen with tabs on its frame. With none picked: the cover, the song in big dot
    /// type, who plays it and the album, key, tempo, tuning and loops in little boxes, what's next, the spectrum.
    /// Lyrics: the cover, song and time small at the left, the words in dot type, the line being sung inverted. Up
    /// Next, About and Loops as the other styles have them. Visuals: drawn in the ink (bars, a scope, the
    /// terrain). Practising: the waveform big, its keys, the practice tabs under it.
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
            ("Lyrics", self.studio_np == 1, "Lyrics"),
            ("Up Next", self.studio_np == 2, "Up next"),
            ("About", self.studio_np == 3, "About this song"),
            ("Loops", self.studio_np == 4, "Loops and bookmarks"),
            ("Visuals", self.studio_np == 5, "Visuals"),
        ];
        if let Some(i) = frame_tabs(ui, r, &tabs, "studio_np_tab") {
            let v = i as u8 + 1;
            if v == 2 && (self.studio_np != 2 || !self.q_cur_seen) {
                // Up Next opened, or clicked again with the song playing out of sight: the list goes to it
                self.studio_np = 2;
                self.q_jump = true;
            } else {
                self.studio_np = if self.studio_np == v { 0 } else { v };
            }
            self.dirty = true;
        }
        let inner = r.shrink(18.0);
        match self.studio_np {
            1 => self.studio_lyrics(ui, inner, dt, acts),
            2 => self.style_queue(ui, inner, acts),
            3 => self.np_about(ui, &p, inner, ink()),
            4 => self.np_loops(ui, &p, inner, acts, ink()),
            5 => self.studio_visuals(ui, inner),
            _ => self.studio_np_main(ui, inner, acts),
        }
    }

    /// The main view: the cover, the song big, the rest small.
    fn studio_np_main(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        let s = sc();
        let cs = r.height().min(r.width() * 0.44).max(80.0);
        let cover = Rect::from_min_size(Pos2::new(r.min.x, r.center().y - cs / 2.0), Vec2::splat(cs));
        self.studio_cover(ui, cover, "studio_np", 640);
        cover_switch(ui, cover, acts, "np");
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
        let who = tr.artist_album();
        let wr = px(&p, Pos2::new(x0, y), egui::Align2::LEFT_CENTER, &who.to_uppercase(), 10.0, dim());
        self.album_link(ui, wr.expand2(Vec2::new(0.0, 2.0)), "studio_np", ink(), acts);
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
        // up next: the whole queue as every look's Up Next has it (the ones played dim, the one playing inverted), as
        // many as there's room for; it opens on the song playing with the one before it above, the wheel scrolls back
        // and on, a click plays one
        let bars_top = r.max.y - (right.height() * 0.3).clamp(52.0 * s, 200.0);
        let all = self.queue.len();
        if all > 0 {
            p.line_segment([Pos2::new(x0, y), Pos2::new(r.max.x, y)], Stroke::new(1.0_f32, ink()));
            px(&p, Pos2::new(x0, y + 12.0 * s), egui::Align2::LEFT_CENTER, "UP NEXT", 9.0, dim());
            let list = Rect::from_min_max(Pos2::new(x0, y + 22.0 * s), Pos2::new(r.max.x, bars_top - 8.0 * s));
            if list.height() > 24.0 * s {
                let rh = 26.0 * s;
                let cur = self.cur;
                // (a new song: back to it)
                let seen_id = egui::Id::new("studio_next_on");
                let on = cur.map_or(0, |c| c + 1);
                let jump = ui.ctx().data(|d| d.get_temp::<usize>(seen_id)) != Some(on);
                if jump {
                    ui.ctx().data_mut(|d| d.insert_temp(seen_id, on));
                }
                let mut c = ui.new_child(egui::UiBuilder::new().max_rect(list).layout(egui::Layout::top_down(egui::Align::Min)));
                let mut area = egui::ScrollArea::vertical().id_salt("studio_next").auto_shrink([false, false]);
                if jump {
                    area = area.vertical_scroll_offset(cur.map_or(0, |c| c.saturating_sub(1)) as f32 * rh);
                }
                area.show_rows(&mut c, rh, all, |ui, rows| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    for i in rows {
                        let Some(n) = self.queue.get(i) else { continue };
                        let (row, h) = ui.allocate_exact_size(Vec2::new(ui.available_width(), rh), Sense::click());
                        let h = h.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Play");
                        let now = cur == Some(i);
                        let played = cur.is_some_and(|c| i < c);
                        if now {
                            ui.painter().rect_filled(row, 0.0, ink());
                        } else if h.hovered() {
                            ui.painter().rect_filled(row, 0.0, key_col());
                        }
                        let col = if now { scr() } else if played { ink().gamma_multiply(0.42) } else { ink() };
                        let nr = Rect::from_min_max(Pos2::new(row.min.x + 4.0, row.min.y), Pos2::new(row.max.x - 64.0 * s, row.max.y));
                        marquee(ui, nr, &format!("{} · {}", n.title.to_uppercase(), n.artist_album().to_uppercase()), 15.0, col);
                        dot(ui.painter(), Pos2::new(row.max.x - 4.0, row.center().y), egui::Align2::RIGHT_CENTER, &fmt_time(n.duration), 15.0, col);
                        if h.clicked() {
                            acts.push(Action::PlayIndex(i));
                        }
                    }
                });
            }
        }
        // the spectrum along the foot
        let (bands, peaks, _) = self.viz_data(1);
        let br = Rect::from_min_max(Pos2::new(x0, bars_top), r.max);
        bars(&p, br, &crate::viz::resample(&bands, 32, 1.0), &crate::viz::resample(&peaks, 32, 1.0), 3.0, 3.0);
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
        cover_switch(ui, cover, acts, "np");
        if let Some(tr) = self.cur_track() {
            let mq = Rect::from_min_size(Pos2::new(r.min.x, cover.max.y + 14.0), Vec2::new(lw, 22.0 * s));
            marquee(ui, mq, &format!("{} - {}", tr.title.to_uppercase(), tr.artist_album().to_uppercase()), 16.0, ink());
            let time = self.deck_time();
            let shown = if time.starts_with('-') { time } else { format!(" {}", time) };
            time_digits(&p, Pos2::new(r.min.x, mq.max.y + 26.0 * s), 34.0 * s, &shown, tr.duration);
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
    /// stands still), or the terrain, the last few seconds of the spectrum as a wire grid running
    /// away from you. Three small keys at the top right pick.
    fn studio_visuals(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let s = sc();
        let names = ["Bars", "Scope", "Terrain", "3D"];
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
            2 => {
                let resp = ui.interact(body, ui.id().with("studio_terr_menu"), Sense::click());
                // (no tip once the mouse plays with it: it would sit over the ripples)
                let resp = if self.studio_terr_mouse == 0 { resp.on_hover_text("Right-click for terrain settings") } else { resp };
                self.studio_terrain(&p, body, resp.hover_pos(), resp.clicked());
                resp.context_menu(|ui| self.studio_terr_menu(ui));
            }
            3 => {
                // the 3D visuals, as the other looks show them (right-click: pick another)
                self.vis_paint(ui, body, 1);
                ui.interact(body, ui.id().with("studio_3d"), Sense::click()).context_menu(|ui| self.vis_menu(ui));
            }
            _ => {
                let (bands, peaks, _) = self.viz_data(1);
                bars(&p, body, &crate::viz::resample(&bands, 32, 1.0), &crate::viz::resample(&peaks, 32, 1.0), 6.0, 6.0);
            }
        }
        // the song in a card at the bottom left while the mouse moves, as every look shows it over its visuals
        self.studio_vis_card(ui, body);
    }

    /// The terrain's right-click menu: the camera, the mouse, the rolling ground, and how fine it's drawn.
    fn studio_terr_menu(&mut self, ui: &mut egui::Ui) {
        let mark = |on: bool| if on { "> " } else { "   " };
        crate::views::dim_line(ui, "CAMERA", 1.0, crate::pal().ink2);
        for (i, n) in ["Still", "Sway", "Change Angles"].iter().enumerate() {
            if crate::menu_item(ui, &format!("{}{}", mark(self.studio_terr_cam == i as u8), n)) {
                self.studio_terr_cam = i as u8;
                self.dirty = true;
                ui.close_menu();
            }
        }
        ui.separator();
        crate::views::dim_line(ui, "MOUSE", 1.0, crate::pal().ink2);
        for (i, n) in ["Nothing", "Ripples (Click for a Big One)", "Look Around", "Ripples and Look Around"].iter().enumerate() {
            if crate::menu_item(ui, &format!("{}{}", mark(self.studio_terr_mouse == i as u8), n)) {
                self.studio_terr_mouse = i as u8;
                self.studio_terr_waves.clear();
                self.dirty = true;
                ui.close_menu();
            }
        }
        ui.separator();
        if crate::menu_item(ui, &format!("{}Rolling Ground", mark(self.studio_terr_swell))) {
            self.studio_terr_swell = !self.studio_terr_swell;
            self.dirty = true;
            ui.close_menu();
        }
        ui.separator();
        crate::sub_menu(ui, "Bands Across", |ui| {
            for n in [48usize, 96, 160, 240] {
                if crate::menu_item(ui, &format!("{}{}", mark(self.studio_terr_n == n), n)) {
                    self.studio_terr_n = n;
                    self.studio_terr.clear();
                    self.dirty = true;
                    ui.close_menu();
                }
            }
        });
        crate::sub_menu(ui, "Rows Deep", |ui| {
            for n in [64usize, 96, 160, 240] {
                if crate::menu_item(ui, &format!("{}{}", mark(self.studio_terr_rows == n), n)) {
                    self.studio_terr_rows = n;
                    self.studio_terr.clear();
                    self.dirty = true;
                    ui.close_menu();
                }
            }
        });
    }

    /// The terrain: each moment's spectrum a row, the newest at the front, rows running back into
    /// the distance; drawn back to front, each row hiding what's behind it, as a wire grid in the ink. The camera can
    /// sway or glide between angles, the ground roll in slow waves, and the mouse ripple it or turn the view
    /// (`hover`: where the mouse is over it; `clicked`: a big ripple there).
    fn studio_terrain(&mut self, p: &egui::Painter, r: Rect, hover: Option<Pos2>, clicked: bool) {
        // (many rows close together, each sliding back a little every frame, and many points across: the wave smooth
        // and moving gently, the owner's ask)
        // (straight from the analysis, as fine as it's asked for here, not through the visualizer's bar count)
        let n_rows: usize = self.studio_terr_rows.clamp(16, 240);
        let n_cols: usize = self.studio_terr_n.clamp(16, 256);
        // (a new row every 30 ms: the rows close together, a few seconds deep)
        const EVERY: f64 = 0.03;
        let k = self.viz[1].gain / self.viz[0].gain.max(self.viz[1].gain);
        let now = crate::viz::resample(&self.bands, n_cols, k);
        let t = p.ctx().input(|i| i.time);
        // (rows keep coming in when the music pauses or stops: the land flows on, flattening out; the owner's ask)
        if t - self.studio_terr_at > EVERY || self.studio_terr.is_empty() {
            // (a little of the last row in the new one, so a moment's flicker doesn't jump)
            let row: Vec<f32> = match self.studio_terr.first() {
                Some(prev) if prev.len() == n_cols => now.iter().zip(prev).map(|(a, b)| a * 0.6 + b * 0.4).collect(),
                _ => now,
            };
            self.studio_terr_at = if t - self.studio_terr_at > EVERY * 3.0 { t } else { self.studio_terr_at + EVERY };
            self.studio_terr.insert(0, row);
            self.studio_terr.truncate(n_rows + 1);
            self.studio_terr_pushed += 1;
        }
        // (how far the rows have slid since the newest came in)
        let frac = ((t - self.studio_terr_at) / EVERY).clamp(0.0, 1.0) as f32;
        let slid = self.studio_terr_pushed as f64 + frac as f64;

        // the motion's own clock (a long gap - the page out of sight - doesn't jump it)
        let dt = (t - self.studio_terr_last).clamp(0.0, 0.1) as f32;
        self.studio_terr_last = t;
        self.studio_terr_clock += dt as f64;
        let mc = self.studio_terr_clock as f32;
        let mut cam = match self.studio_terr_cam {
            1 => TerrCam {
                pan: 0.05 * (0.11 * mc).sin() + 0.02 * (0.29 * mc + 1.0).sin(),
                hz: 0.18 + 0.06 * (0.07 * mc).sin(),
                roll: 0.05 * (0.09 * mc + 0.5).sin(),
                turn: 0.4 * (0.13 * mc + 1.0).sin(),
                lift: 1.0 + 0.12 * (0.05 * mc).sin(),
                spread: 1.08,
            },
            2 => TerrCam::shot(mc),
            _ => TerrCam::REST,
        };
        // the view leaning toward the mouse (back to rest when it leaves), eased so it glides
        let look_on = matches!(self.studio_terr_mouse, 2 | 3);
        let aim = match hover.filter(|_| look_on) {
            Some(h) => [((h.x - r.center().x) / r.width()).clamp(-0.5, 0.5), ((h.y - r.center().y) / r.height()).clamp(-0.5, 0.5)],
            None => [0.0, 0.0],
        };
        let ease = 1.0 - (-dt * 3.0).exp();
        for a in 0..2 {
            self.studio_terr_look[a] += (aim[a] - self.studio_terr_look[a]) * ease;
        }
        let [lx, ly] = self.studio_terr_look;
        cam.pan += lx * 0.12;
        cam.turn -= lx * 0.9;
        cam.roll += lx * 0.06;
        cam.hz = (cam.hz - ly * 0.22).clamp(0.0, 0.5);
        cam.lift *= 1.0 + ly * 0.4;

        let c = r.center();
        let (sn, cs) = cam.roll.sin_cos();
        let rot = |q: Pos2, s: f32| {
            let v = q - c;
            Pos2::new(c.x + v.x * cs - v.y * s, c.y + v.x * s + v.y * cs)
        };
        // a spot on the land, before the roll: u across (-0.5 the left edge, 0.5 the right), d rows deep, v its height
        let flat = |u: f32, d: f32, v: f32| {
            let z = (1.0 - d / (n_rows - 1) as f32).max(0.0);
            let persp = 0.28 + 0.72 * z;
            let x = c.x + (u - cam.pan) * r.width() * 1.05 * cam.spread * persp + cam.turn * (1.0 - z) * r.width() * 0.3;
            let base = r.min.y + r.height() * (cam.hz + (0.96 - cam.hz) * z);
            Pos2::new(x, base - v * r.height() * 0.42 * persp * cam.lift)
        };

        // the mouse rippling the land: small ripples as it moves, a big one on a click, where it points on the ground
        let rip_on = matches!(self.studio_terr_mouse, 1 | 3);
        if let Some(h) = hover.filter(|_| rip_on) {
            let moved = (h - self.studio_terr_mouse_at.0).length();
            if clicked || (moved > 60.0 && t - self.studio_terr_mouse_at.1 > 0.3) {
                let q = rot(h, -sn);
                let z = (((q.y - r.min.y) / r.height() - cam.hz) / (0.96 - cam.hz)).clamp(0.0, 1.0);
                let persp = 0.28 + 0.72 * z;
                let u = (q.x - c.x - cam.turn * (1.0 - z) * r.width() * 0.3) / (r.width() * 1.05 * cam.spread * persp) + cam.pan;
                let big = if clicked { 0.3 } else { 0.07 };
                self.studio_terr_waves.push((u, (1.0 - z) * (n_rows - 1) as f32, slid, t, big));
                self.studio_terr_mouse_at = (h, t);
            }
        }
        self.studio_terr_waves.retain(|w| t - w.3 < 2.5);
        if self.studio_terr_waves.len() > 8 {
            let extra = self.studio_terr_waves.len() - 8;
            self.studio_terr_waves.drain(..extra);
        }
        // (each ripple rides back with the rows it fell on, as a stone's rings drift down a river)
        let waves: Vec<(f32, f32, f32, f32)> = self
            .studio_terr_waves
            .iter()
            .map(|w| (w.0, w.1 + (slid - w.2) as f32, (t - w.3) as f32, w.4))
            .filter(|w| w.1 < n_rows as f32 + 40.0)
            .collect();
        let deep = 1.6 / (n_rows - 1) as f32;
        let swell = self.studio_terr_swell;
        let height = |u: f32, d: f32, v: f32| {
            let mut v = v;
            if swell {
                // (two slow swells crossing, running away from you)
                let zf = d / (n_rows - 1) as f32;
                v += 0.075 * (0.6 * (7.0 * u + 9.0 * zf - 1.3 * mc).sin() + 0.4 * (-4.5 * u + 5.0 * zf - 0.8 * mc + 1.0).sin());
            }
            for &(wu, wd, age, amp) in &waves {
                let dist = ((u - wu).powi(2) + ((d - wd) * deep).powi(2)).sqrt();
                let off = dist - 0.22 * age;
                if off.abs() < 0.1 {
                    v += amp * (-(off / 0.035).powi(2)).exp() * (off * 70.0).cos() * (-age * 1.6).exp() / (1.0 + 5.0 * dist);
                }
            }
            v
        };

        // (with the camera moving, the land runs on past its edges - the spectrum mirrored - so its sides stay out of
        // sight)
        let moving = self.studio_terr_cam != 0 || lx.abs() + ly.abs() > 0.002;
        let ext = if moving { (n_cols / 3) as i32 } else { 0 };
        let n = n_cols as i32;
        let cols: Vec<(f32, usize)> = (-ext..n + ext)
            .map(|i| {
                let src = if i < 0 { -i } else if i >= n { 2 * n - 2 - i } else { i };
                (i as f32 / (n - 1) as f32 - 0.5, src.clamp(0, n - 1) as usize)
            })
            .collect();
        let cp = p.with_clip_rect(r);
        let rows = &self.studio_terr;
        let below = r.height() * 1.5;
        let mut prev: Option<Vec<Pos2>> = None;
        for j in (0..rows.len()).rev() {
            let d = j as f32 + frac;
            if d > (n_rows - 1) as f32 {
                continue;
            }
            let row = &rows[j];
            let flats: Vec<Pos2> = cols.iter().map(|&(u, s)| flat(u, d, height(u, d, row.get(s).copied().unwrap_or(0.0)))).collect();
            let pts: Vec<Pos2> = flats.iter().map(|q| rot(*q, sn)).collect();
            // (the ground under the line, in the glass's colour, hides the rows behind)
            let mut m = egui::Mesh::default();
            for (q, f) in pts.iter().zip(&flats) {
                m.colored_vertex(*q, scr());
                m.colored_vertex(rot(Pos2::new(f.x, f.y + below), sn), scr());
            }
            for i in 0..pts.len() - 1 {
                let a = (2 * i) as u32;
                m.add_triangle(a, a + 2, a + 1);
                m.add_triangle(a + 1, a + 2, a + 3);
            }
            cp.add(m);
            let near = 1.0 - d / n_rows as f32;
            cp.add(egui::Shape::line(pts.clone(), Stroke::new(0.5 + 1.3 * near, ink().gamma_multiply(0.45 + 0.55 * near))));
            // (the lines running back, to the row behind, every eighth point)
            if let Some(back) = &prev {
                for i in (0..pts.len()).filter(|i| (*i as i32 - ext).rem_euclid(8) == 0) {
                    cp.line_segment([pts[i], back[i]], Stroke::new(0.5_f32, ink().gamma_multiply(0.45)));
                }
            }
            prev = Some(pts);
        }
        // (at most 60 frames a second: smooth, without drawing flat out; paused or playing, it keeps moving)
        p.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    }

    /// Over the visuals, while the mouse moves (and a few seconds after): the song in a card at the bottom left, its
    /// cover, title and artist, as the other looks show it over theirs.
    fn studio_vis_card(&mut self, ui: &mut egui::Ui, r: Rect) {
        let Some(tr) = self.cur_track() else { return };
        let idle = (ui.input(|i| i.time) - self.np_move_at) as f32;
        let show = (1.0 - (idle - 3.0) / 0.6).clamp(0.0, 1.0);
        if show <= 0.0 {
            return;
        }
        let s = sc();
        let card = Rect::from_min_size(Pos2::new(r.min.x + 6.0, r.max.y - 104.0 * s), Vec2::new((440.0 * s).min(r.width() * 0.6), 96.0 * s));
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(card));
        c.set_opacity(show);
        let p = c.painter().clone();
        p.rect_filled(card, 0.0, scr());
        p.rect_stroke(card, 0.0, Stroke::new(1.0_f32, ink()));
        let cs = card.height() - 20.0;
        let cr = Rect::from_min_size(card.min + Vec2::splat(10.0), Vec2::splat(cs));
        self.studio_cover(&c, cr, "studio_vis_card", 160);
        let tx = cr.max.x + 14.0;
        let room = (card.max.x - tx - 10.0).max(20.0);
        marquee(&c, Rect::from_min_max(Pos2::new(tx, card.center().y - 26.0 * s), Pos2::new(tx + room, card.center().y - 2.0 * s)), &tr.title.to_uppercase(), 16.0, ink());
        px(&p, Pos2::new(tx, card.center().y + 14.0 * s), egui::Align2::LEFT_CENTER, &tr.artist_album().to_uppercase(), 9.0, dim());
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
        cover_switch(ui, cover, acts, "np");
        if let Some(tr) = self.cur_track() {
            let mq = Rect::from_min_max(Pos2::new(cover.max.x + 14.0, head.min.y), Pos2::new(head.max.x - 230.0 * s, head.min.y + 24.0 * s));
            marquee(ui, mq, &format!("{} - {}", tr.title.to_uppercase(), tr.artist_album().to_uppercase()), 18.0, ink());
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
            px(&p, Pos2::new(mq.min.x, head.max.y - 10.0 * s), egui::Align2::LEFT_CENTER, &parts.join("  ·  "), 9.5, dim());
            let time = self.deck_time();
            let shown = if time.starts_with('-') { time } else { format!(" {}", time) };
            let tw = time_digits(&p, Pos2::new(head.max.x - 220.0 * s, head.center().y), 40.0 * s, &shown, tr.duration);
            let tr_r = Rect::from_min_size(Pos2::new(head.max.x - 220.0 * s, head.min.y), Vec2::new(tw + 6.0, head.height()));
            if ui.interact(tr_r, ui.id().with("studio_prac_time"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Elapsed / remaining").clicked() {
                self.time_left = !self.time_left;
                self.dirty = true;
            }
        }
        // the waveform (the whole song above it), then the practice tabs and their tools: the same in every style
        let body = Rect::from_min_max(Pos2::new(r.min.x, head.max.y + 12.0), r.max);
        self.practice_stack(ui, body, acts, true);
    }
}

/// Where the terrain's camera stands: `pan` sideways (the land's widths), `hz` how far down the far rows sit (of the
/// screen's height), `roll` its lean (radians), `turn` the far rows swung aside (of the width), `lift` the hills'
/// height, `spread` the land's width.
#[derive(Clone, Copy)]
struct TerrCam {
    pan: f32,
    hz: f32,
    roll: f32,
    turn: f32,
    lift: f32,
    spread: f32,
}

impl TerrCam {
    const REST: TerrCam = TerrCam { pan: 0.0, hz: 0.18, roll: 0.0, turn: 0.0, lift: 1.0, spread: 1.0 };
    /// The angles Change Angles glides between: as it starts, low and wide, high above, banking, close in.
    const SHOTS: [TerrCam; 5] = [
        TerrCam::REST,
        TerrCam { pan: -0.05, hz: 0.32, roll: -0.04, turn: 0.35, lift: 1.35, spread: 1.25 },
        TerrCam { pan: 0.05, hz: 0.03, roll: 0.05, turn: -0.25, lift: 0.75, spread: 0.95 },
        TerrCam { pan: -0.06, hz: 0.15, roll: 0.11, turn: 0.6, lift: 1.1, spread: 1.15 },
        TerrCam { pan: 0.06, hz: 0.26, roll: -0.08, turn: -0.55, lift: 1.4, spread: 1.5 },
    ];

    fn mix(a: TerrCam, b: TerrCam, k: f32) -> TerrCam {
        let m = |x: f32, y: f32| x + (y - x) * k;
        TerrCam {
            pan: m(a.pan, b.pan),
            hz: m(a.hz, b.hz),
            roll: m(a.roll, b.roll),
            turn: m(a.turn, b.turn),
            lift: m(a.lift, b.lift),
            spread: m(a.spread, b.spread),
        }
    }

    /// Change Angles, `t` seconds in: each angle held a while, then a slow glide to the next, never the same twice
    /// running; drifting a little while it holds.
    fn shot(t: f32) -> TerrCam {
        const ORDER: [usize; 10] = [0, 1, 3, 2, 4, 1, 2, 0, 3, 4];
        const HOLD: f32 = 12.0;
        const GLIDE: f32 = 5.0;
        let n = (t / HOLD).floor().max(0.0) as usize;
        let to = TerrCam::SHOTS[ORDER[n % ORDER.len()]];
        let from = if n == 0 { to } else { TerrCam::SHOTS[ORDER[(n - 1) % ORDER.len()]] };
        let k = ((t - n as f32 * HOLD) / GLIDE).clamp(0.0, 1.0);
        let mut c = TerrCam::mix(from, to, k * k * (3.0 - 2.0 * k));
        c.pan += 0.02 * (0.2 * t).sin();
        c.roll += 0.012 * (0.3 * t + 1.0).sin();
        c.turn += 0.08 * (0.17 * t).sin();
        c
    }
}

/// Between two covers dotted in ink, `t` of the way (0..1): each dot the new cover's once a hash of its place falls
/// under `t`, so the new cover sparkles in over the old one.
fn dissolve(old: &egui::ColorImage, new: &egui::ColorImage, t: f32) -> egui::ColorImage {
    let pixels = old
        .pixels
        .iter()
        .zip(&new.pixels)
        .enumerate()
        .map(|(i, (a, b))| {
            // (a well-mixed hash of the dot's place: no pattern in which dots go first)
            let mut x = i as u32;
            x ^= x >> 16;
            x = x.wrapping_mul(0x7feb_352d);
            x ^= x >> 15;
            x = x.wrapping_mul(0x846c_a68b);
            x ^= x >> 16;
            if ((x >> 8) as f32 / 16_777_216.0) < t {
                *b
            } else {
                *a
            }
        })
        .collect();
    egui::ColorImage { size: new.size, pixels }
}

/// A click on a cover (Now Playing's, the player bar's) switches every Studio cover between its own colors and the
/// screen's ink.
pub(crate) fn cover_switch(ui: &egui::Ui, r: Rect, acts: &mut Vec<Action>, salt: &str) {
    let ink = crate::skin::STUDIO_COVER.load(std::sync::atomic::Ordering::Relaxed) == 1;
    let tip = if ink { "Show cover in color" } else { "Show cover in screen ink" };
    let h = ui.interact(r, ui.id().with(("studio_cover_switch", salt)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
    if h.clicked() {
        acts.push(Action::StyleColor(5, if ink { 0 } else { 1 }));
    }
}

/// A picture (the whole of it, framed as the color cover shows it) dotted in `ink` on clear, `n` dots across, each dot one of four shades of the ink (as
/// an old four-shade LCD): its brightness spread by error diffusion (Floyd-Steinberg, the rows in turn left to right and
/// back), so faces and lettering stay recognizable.
pub(crate) fn dither(img: &egui::ColorImage, ink: Color32, n: usize, light_ink: bool) -> egui::ColorImage {
    let n = n.max(1);
    let [w, h] = img.size;
    let gray = image::GrayImage::from_fn(w.max(1) as u32, h.max(1) as u32, |x, y| {
        let c = img.pixels.get(y as usize * w + x as usize).copied().unwrap_or(Color32::WHITE);
        image::Luma([(0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32) as u8])
    });
    let small = image::imageops::resize(&gray, n as u32, n as u32, image::imageops::FilterType::CatmullRom);
    // (brightness lifted a little, so the mid-greys keep their detail)
    let mut v: Vec<f32> = small.pixels().map(|p| (p.0[0] as f32 / 255.0).powf(0.9)).map(|g| if light_ink { 1.0 - g } else { g }).collect();
    let mut px = vec![Color32::TRANSPARENT; n * n];
    for y in 0..n {
        let right = y % 2 == 0;
        // the next dot along the row, and the one before it
        let ahead = |x: usize| if right { (x + 1 < n).then_some(x + 1) } else { x.checked_sub(1) };
        let behind = |x: usize| if right { x.checked_sub(1) } else { (x + 1 < n).then_some(x + 1) };
        for i in 0..n {
            let x = if right { i } else { n - 1 - i };
            let k = y * n + x;
            // (the nearest of the four shades: clear, light, dark, the ink itself)
            let q = (v[k].clamp(0.0, 1.0) * 3.0).round() / 3.0;
            if q < 1.0 {
                let a = ((1.0 - q) * 255.0).round() as u8;
                px[k] = Color32::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), a);
            }
            let err = v[k] - q;
            if let Some(a) = ahead(x) {
                v[y * n + a] += err * 7.0 / 16.0;
            }
            if y + 1 < n {
                if let Some(b) = behind(x) {
                    v[(y + 1) * n + b] += err * 3.0 / 16.0;
                }
                v[(y + 1) * n + x] += err * 5.0 / 16.0;
                if let Some(a) = ahead(x) {
                    v[(y + 1) * n + a] += err / 16.0;
                }
            }
        }
    }
    egui::ColorImage { size: [n, n], pixels: px }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dither_keeps_the_tone() {
        // a mid grey comes out about half inked, white clear, black all ink, at the size asked
        let grey = |g: u8| egui::ColorImage::new([40, 30], Color32::from_gray(g));
        let ink = Color32::BLACK;
        let inked = |img: &egui::ColorImage| img.pixels.iter().map(|c| c.a() as f32 / 255.0).sum::<f32>() / img.pixels.len() as f32;
        let d = dither(&grey(128), ink, 64, false);
        assert_eq!(d.size, [64, 64]);
        let f = inked(&d);
        assert!((0.38..0.56).contains(&f), "{}", f);
        assert_eq!(inked(&dither(&grey(255), ink, 32, false)), 0.0);
        assert_eq!(inked(&dither(&grey(0), ink, 32, false)), 1.0);
        // (light ink on dark glass: white is all ink, black none, so the picture isn't a negative)
        assert_eq!(inked(&dither(&grey(255), ink, 32, true)), 1.0);
        assert_eq!(inked(&dither(&grey(0), ink, 32, true)), 0.0);
    }
}
