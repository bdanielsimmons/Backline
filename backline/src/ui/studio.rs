//! Studio: Tidalite's look grown into a Backline style (the owner's picks of 2026-11-25, style sheets 7 and 8). It's
//! Hardware's relative: everything Hardware has, Studio has (`hardware()` is true for both), drawn its own way. Every
//! part is a screen (cream, gray, blue or amber) set in a black body and named in brackets on its frame; keys have a
//! fine outline; the time is LCD digits with the unlit segments faintly there; the song scrolls by in dot type; lists
//! are in a typewriter face. Its Now Playing is one big screen with tabs on its frame (Lyrics, Up Next, About, Loops,
//! Visuals), the visuals drawn in the screen's ink (bars, a scope, the terrain).
//!
//! This file is the kit (colours, faces, screens, keys, lights, LCD digits, the marquee) and is shared with the phone
//! app (phone/); the desktop's Studio deck and Now Playing are `ui/studio_app.rs`.

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

pub(crate) fn sc() -> f32 {
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

/// Studio's palette: everything on its screens (the glass the pages' and lists' colour, the ink their writing, the
/// keys a shade darker), the body black round them.
pub(crate) fn studio_pal() -> crate::skin::Pal {
    let (_, glass, keys, ink, _) = crate::skin::studio_screen();
    let mut p = crate::skin::look_pal(glass, glass, ink);
    p.app_bg = BODY;
    p.panel = glass;
    p.panel_lt = glass.lerp_to_gamma(ink, 0.06);
    p.panel_dk = keys;
    p.panel_hover = keys;
    p.display = glass;
    p.display_ghost = glass.lerp_to_gamma(ink, 0.08);
    p.groove = keys;
    p.sel = glass.lerp_to_gamma(ink, 0.16);
    p.ink = ink;
    p.ink2 = ink.lerp_to_gamma(glass, 0.22);
    p.dim = ink.lerp_to_gamma(glass, 0.45);
    p.btn_face = keys;
    p.btn_hi = keys.lerp_to_gamma(glass, 0.5);
    p.row_alt = glass.lerp_to_gamma(ink, 0.045);
    p.row_sel = glass.lerp_to_gamma(ink, 0.16);
    p.bar_txt = ink;
    p.accent = ink;
    p.edge = Color32::from_rgb(10, 11, 8);
    p
}
