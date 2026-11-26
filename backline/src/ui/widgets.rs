//! Backline's drawing kit: the raised buttons, LCD boxes, tabs, check boxes, dropdowns, window decorations,
//! seven-segment digits, the logo and the text helpers every screen is built from. Shared by the desktop app and
//! the phone app (android/), so both look like Backline.

use crate::font::{self, fit, ptext, ptext_fit, snap, spx, text_w, thick, wrap};
use crate::icons::*;
use crate::skin::*;
use crate::{tour, update, winamp_ui};
use crate::{practice_tools, version, HEAD_H, ROW_H};
use eframe::egui::{self, Align, Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

pub(crate) const BTN_H: f32 = 26.0;

/// Button height (it scales with a panel that scales its contents).
pub(crate) fn bh() -> f32 {
    BTN_H * font::ui_scale()
}

// --------------------------------------------------------------- marquee
/// How fast the player's title scrolls (points a second, as f32 bits), so other scrolling text can keep pace.
pub(crate) static TITLE_SCROLL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// The player title's scrolling speed (points a second), or a usual one before the player has drawn.
pub(crate) fn title_scroll() -> f32 {
    let v = f32::from_bits(TITLE_SCROLL.load(std::sync::atomic::Ordering::Relaxed));
    if v > 0.0 {
        v
    } else {
        9.0 * spx(1.5)
    }
}

/// Scrolling bitmap text inside `rect` (static if it fits).
pub(crate) fn marquee(ui: &egui::Ui, rect: Rect, text: &str, px: f32, color: Color32) {
    marquee_at(ui, rect, text, px, color, 9.0 * spx(px));
}

/// Scrolling bitmap text inside `rect`, moving `speed` points a second (static if it fits).
pub(crate) fn marquee_at(ui: &egui::Ui, rect: Rect, text: &str, px: f32, color: Color32, speed: f32) {
    let px = spx(px);
    let w = text_w(text, px);
    let clip = ui.painter().with_clip_rect(rect.shrink2(Vec2::new(2.0, 0.0)));
    let cy = rect.center().y;
    if w <= rect.width() - 8.0 {
        ptext(&clip, Pos2::new(rect.min.x + 5.0, cy), Align::Min, text, px, color);
    } else {
        let period = w + 8.0 * px;
        let t = ui.input(|i| i.time) as f32;
        let off = (t * speed) % period;
        let x = rect.min.x + 5.0 - off;
        ptext(&clip, Pos2::new(x, cy), Align::Min, text, px, color);
        ptext(&clip, Pos2::new(x + period, cy), Align::Min, text, px, color);
    }
}

/// A button showing a pixel icon, `w` wide. `col` tints it; `on` draws it pressed.
pub(crate) fn icon_btn_w(ui: &mut egui::Ui, icon: &[&str], on: bool, col: Color32, w: f32) -> egui::Response {
    let sc = font::ui_scale();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w * sc, bh()), Sense::click());
    let down = resp.is_pointer_button_down_on() || on;
    if winamp_ui::chrome_on() {
        quiet_h(ui.painter(), rect, down, resp.hovered());
    } else {
        raised_h(ui.painter(), rect, down, resp.hovered());
    }
    let px = 2.0 * sc;
    let (iw, ih) = (icon[0].len() as f32 * px, icon.len() as f32 * px);
    let dy = if down { 1.0 } else { 0.0 };
    let o = Pos2::new((rect.center().x - iw / 2.0).round(), (rect.center().y - ih / 2.0 + dy).round());
    pixmap(ui.painter(), o, px, icon, col);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Icon on the left, a short label on the right.
pub(crate) fn ibtn(ui: &mut egui::Ui, icon: &[&str], text: &str, on: bool) -> egui::Response {
    let sc = font::ui_scale();
    let ip = 2.0 * sc;
    let w = (10.0 + 8.0 + 10.0) * sc + icon[0].len() as f32 * ip + text_w(text, 2.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::click());
    let down = resp.is_pointer_button_down_on() || on;
    if winamp_ui::chrome_on() {
        quiet_h(ui.painter(), rect, down, resp.hovered());
    } else {
        raised_h(ui.painter(), rect, down, resp.hovered());
    }
    let dy = if down { 1.0 } else { 0.0 };
    let o = Pos2::new((rect.min.x + 10.0 * sc).round(), (rect.center().y - icon.len() as f32 * ip / 2.0 + dy).round());
    pixmap(ui.painter(), o, ip, icon, pal().ink);
    let tx = o.x + icon[0].len() as f32 * ip + 8.0 * sc;
    winamp_ui::as_title(|| {
        ui_text(ui.painter(), Pos2::new(tx, rect.center().y + dy), Align::Min, text, 2.0, rect.max.x - tx - 4.0, pal().ink)
    });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Interface text (buttons, tabs, headers, titles): in the worn Winamp skin's own font when there is one,
/// otherwise Backline's.
pub(crate) fn ui_text(p: &egui::Painter, pos: Pos2, align: Align, text: &str, px: f32, max_w: f32, col: Color32) {
    ptext_fit(p, pos, align, text, px, max_w, col);
}

/// Fit a pixel icon into `r` as large as whole dots allow.
pub(crate) fn icon_in(p: &egui::Painter, r: Rect, icon: &[&str], col: Color32) {
    // the dot size is a whole number of real screen pixels: crisp, and as fine as the button allows
    // with a margin all round: as big as fits in both directions, not just the height
    let ppp = font::ppp();
    let fit = ((r.height() - 5.0) / icon.len() as f32).min((r.width() - 8.0) / icon[0].len() as f32);
    let k = ((fit * ppp).floor().max(1.0)) / ppp;
    let (w, h) = (icon[0].len() as f32 * k, icon.len() as f32 * k);
    pixmap(p, Pos2::new((r.center().x - w / 2.0).round(), (r.center().y - h / 2.0).round()), k, icon, col);
}

// ------------------------------------------------------------ skin drawing
/// Filled rectangle, snapped to whole physical pixels (crisp edges, never thinner than 1 px).
pub(crate) fn fill_rect(p: &egui::Painter, r: Rect, c: Color32) {
    if r.width() <= 0.0 || r.height() <= 0.0 {
        return;
    }
    let e = thick(1.0);
    let min = Pos2::new(snap(r.min.x), snap(r.min.y));
    let mut max = Pos2::new(snap(r.max.x), snap(r.max.y));
    if max.x - min.x < e {
        max.x = min.x + e;
    }
    if max.y - min.y < e {
        max.y = min.y + e;
    }
    p.rect_filled(Rect::from_min_max(min, max), Rounding::same(0.0), c);
}

/// One-pixel-style outline drawn inside `r`.
pub(crate) fn outline(p: &egui::Painter, r: Rect, t: f32, c: Color32) {
    fill_rect(p, Rect::from_min_size(r.min, Vec2::new(r.width(), t)), c);
    fill_rect(p, Rect::from_min_size(Pos2::new(r.min.x, r.max.y - t), Vec2::new(r.width(), t)), c);
    fill_rect(p, Rect::from_min_size(r.min, Vec2::new(t, r.height())), c);
    fill_rect(p, Rect::from_min_size(Pos2::new(r.max.x - t, r.min.y), Vec2::new(t, r.height())), c);
}

/// Sunken panel (LCD windows, groove).
pub(crate) fn inset(p: &egui::Painter, r: Rect, fill: Color32) {
    let t1 = thick(1.5);
    let t2 = thick(1.0);
    fill_rect(p, r, fill);
    fill_rect(p, Rect::from_min_size(r.min, Vec2::new(r.width(), t1)), pal().edge);
    fill_rect(p, Rect::from_min_size(r.min, Vec2::new(t1, r.height())), pal().edge);
    fill_rect(p, Rect::from_min_size(Pos2::new(r.min.x, r.max.y - t2), Vec2::new(r.width(), t2)), pal().panel_lt);
    fill_rect(p, Rect::from_min_size(Pos2::new(r.max.x - t2, r.min.y), Vec2::new(t2, r.height())), pal().panel_lt);
}

/// The corner of buttons, chips, fields and tags with a skin worn: small, so they read as squared-off boxes rather
/// than round pills.
pub(crate) fn corner() -> f32 {
    (4.0 * font::ui_scale()).clamp(3.0, 7.0)
}

/// Raised bevelled button face.
pub(crate) fn raised_h(p: &egui::Painter, r: Rect, down: bool, hover: bool) {
    // with a skin worn: flat, small corners, a fine edge
    if winamp_ui::chrome_on() {
        let rad = corner();
        let fill = if down {
            pal().panel_dk
        } else if hover {
            pal().panel
        } else {
            pal().panel.lerp_to_gamma(pal().display, 0.35)
        };
        p.rect_filled(r, rad, fill);
        p.rect_stroke(r, rad, Stroke::new(1.0_f32, if hover { pal().panel_lt } else { pal().panel_hover }));
        return;
    }
    let (hi, lo) = if down { (pal().panel_dk, pal().panel_lt) } else { (pal().panel_lt, pal().panel_dk) };
    let face = if down {
        pal().sel
    } else if hover {
        pal().panel_hover
    } else {
        pal().panel
    };
    let t = thick(1.0);
    fill_rect(p, r, face);
    let i = r.shrink(t);
    fill_rect(p, Rect::from_min_size(i.min, Vec2::new(i.width(), t)), hi);
    fill_rect(p, Rect::from_min_size(i.min, Vec2::new(t, i.height())), hi);
    fill_rect(p, Rect::from_min_size(Pos2::new(i.min.x, i.max.y - t), Vec2::new(i.width(), t)), lo);
    fill_rect(p, Rect::from_min_size(Pos2::new(i.max.x - t, i.min.y), Vec2::new(t, i.height())), lo);
    outline(p, r, t, pal().edge);
}

pub(crate) fn raised(p: &egui::Painter, r: Rect, down: bool) {
    raised_h(p, r, down, false);
}

/// A button's face in the calm look (the owner's pick, 2026-11-24: plain words, no boxes): nothing at rest, a soft
/// shade under the pointer, a little deeper while held or switched on. Slider knobs and the like keep `raised`.
pub(crate) fn quiet_h(p: &egui::Painter, r: Rect, down: bool, hover: bool) {
    let a = if down {
        0.13
    } else if hover {
        0.07
    } else {
        return;
    };
    p.rect_filled(r, corner(), pal().ink.gamma_multiply(a));
}

/// Whether the Hardware style is worn: buttons are rubber keys, what's on has a light, the page's name sits on a lit
/// screen and the pickers have their names printed above them.
pub(crate) fn hardware() -> bool {
    // (Studio is Hardware's relative: everything Hardware has, it has, drawn its own way)
    matches!(crate::skin::style(), crate::skin::STYLE_HARDWARE | crate::skin::STYLE_STUDIO)
}

/// A rubber key (the Hardware style): raised on a shadow, a light edge along its top; pressed, it sinks.
pub(crate) fn key_face(p: &egui::Painter, r: Rect, down: bool, hover: bool) {
    if crate::studio::on() {
        crate::studio::key(p, r, false, down, hover);
        return;
    }
    let rad = corner();
    let lift = if down { 0.0 } else { 2.0 };
    p.rect_filled(r.translate(Vec2::new(0.0, 2.0)), rad, Color32::from_black_alpha(170));
    let k = r.translate(Vec2::new(0.0, 2.0 - lift));
    let face = if down {
        pal().panel.lerp_to_gamma(Color32::BLACK, 0.15)
    } else if hover {
        pal().panel.lerp_to_gamma(Color32::WHITE, 0.16)
    } else {
        pal().panel.lerp_to_gamma(Color32::WHITE, 0.09)
    };
    p.rect_filled(k, rad, face);
    p.line_segment(
        [Pos2::new(k.min.x + rad, k.min.y + 0.5), Pos2::new(k.max.x - rad, k.min.y + 0.5)],
        Stroke::new(1.0_f32, Color32::from_white_alpha(if down { 8 } else { 26 })),
    );
}

/// A small light that's on (the Hardware style), with a soft glow round it.
pub(crate) fn led(p: &egui::Painter, c: Pos2) {
    if crate::studio::on() {
        crate::studio::light(p, c, true, crate::studio::ink());
        return;
    }
    let a = pal().accent;
    p.circle_filled(c, 5.0, a.gamma_multiply(0.18));
    p.circle_filled(c, 2.4, a);
}

/// The mark under what's on (the tab you're on, a choice that's picked): a short line under its words, `w` wide,
/// along the foot of `r`. The Hardware style: a light under it instead.
pub(crate) fn on_line(p: &egui::Painter, r: Rect, w: f32) {
    if hardware() && !crate::studio::on() {
        led(p, Pos2::new(r.center().x, r.max.y - 1.0));
        return;
    }
    let w = w.min(r.width() - 4.0).max(8.0);
    let x = (r.center().x - w / 2.0).round();
    p.rect_filled(Rect::from_min_max(Pos2::new(x, r.max.y - 3.0), Pos2::new(x + w, r.max.y - 1.0)), 1.0, pal().ink);
}

/// A small chevron pointing down (a dropdown's mark), centred on `c`.
pub(crate) fn chevron(p: &egui::Painter, c: Pos2, s: f32, col: Color32) {
    let pts = vec![c + Vec2::new(-s, -s * 0.5), c + Vec2::new(0.0, s * 0.5), c + Vec2::new(s, -s * 0.5)];
    p.add(egui::Shape::line(pts, Stroke::new(1.5_f32, col)));
}

/// Rectangle with its four corners cut off by `n` (a pixel-art "rounded" box).
pub(crate) fn notch_fill(p: &egui::Painter, r: Rect, n: f32, c: Color32) {
    fill_rect(p, Rect::from_min_max(Pos2::new(r.min.x + n, r.min.y), Pos2::new(r.max.x - n, r.max.y)), c);
    fill_rect(p, Rect::from_min_max(Pos2::new(r.min.x, r.min.y + n), Pos2::new(r.max.x, r.max.y - n)), c);
}

// ------------------------------------------------------- window frames
// Every window is a panel (main.rs's panel_frame) whose margins hold its frame: FRAME_SIDE on the sides and foot,
// FRAME_TOP over it for the title strip. `inner` below is always the panel's own rect, inside those margins.

/// The frame's width at a window's sides and foot.
pub(crate) const FRAME_SIDE: f32 = 14.0;
/// The panel's top margin, where the title strip goes.
pub(crate) const FRAME_TOP: f32 = 30.0;
/// The title strip under a worn skin: taller, so its title bar keeps a band free for the window's controls.
pub(crate) const SKIN_TITLE_H: f32 = 42.0;
/// The room every window keeps between its frame and anything on its page (lists, waveforms, boxes), all round.
pub(crate) const PAGE_PAD: f32 = 12.0;

/// The title strip's height.
pub(crate) fn title_h() -> f32 {
    if winamp_ui::chrome_on() {
        SKIN_TITLE_H
    } else {
        FRAME_TOP
    }
}

/// The whole window drawn round the panel `inner`, frame and all.
pub(crate) fn frame_outer(inner: Rect) -> Rect {
    Rect::from_min_max(inner.min - Vec2::new(FRAME_SIDE, FRAME_TOP), inner.max + Vec2::splat(FRAME_SIDE))
}

/// The window's body inside its frame: the panel, less what a taller title strip takes from its top.
pub(crate) fn frame_body(inner: Rect) -> Rect {
    Rect::from_min_max(Pos2::new(inner.min.x, inner.min.y + title_h() - FRAME_TOP), inner.max)
}

/// Where a window's page goes: its body, PAGE_PAD in from the frame on every side.
pub(crate) fn page_rect(inner: Rect) -> Rect {
    frame_body(inner).shrink(PAGE_PAD)
}

/// A Ui for a window's page (top down, in page_rect). The panel's Ui still holds the title strip's controls.
pub(crate) fn page_ui(ui: &mut egui::Ui, inner: Rect) -> egui::Ui {
    ui.new_child(egui::UiBuilder::new().max_rect(page_rect(inner)).layout(egui::Layout::top_down(Align::Min)))
}

/// The title strip's band for controls: (its centre, the tallest control it takes). Under a skin, the free band of
/// the skin's title bar less an equal gap above and below; Backline's own frames centre them on the window's name.
pub(crate) fn title_band(inner: Rect) -> (f32, f32) {
    let outer = frame_outer(inner);
    if winamp_ui::chrome_on() {
        let strip = Rect::from_min_max(outer.min, Pos2::new(outer.max.x, outer.min.y + title_h()));
        // (Backline's looks: the bar's own middle line, as the window's name has)
        if winamp_ui::flat_frame().is_some() {
            return (winamp_ui::bar_mid(strip), winamp_ui::bar_room(strip));
        }
        let band = winamp_ui::title_band_rect(strip, FRAME_SIDE);
        (band.center().y, band.height() - 2.0 * winamp_ui::TITLE_GAP)
    } else {
        (outer.min.y + 12.0, 24.0)
    }
}

/// Where title-strip controls end on the right: in line with the page's right edge, clear of the frame.
pub(crate) fn title_right(inner: Rect) -> f32 {
    inner.max.x - PAGE_PAD
}

/// The rect for a control `w` wide and `h` tall (less, if the strip's band is lower) in a window's title strip,
/// its right end at `right` (title_right for the first): vertically centred in the band, an equal gap above and
/// below. Every control in a title strip is placed with this.
pub(crate) fn title_slot(inner: Rect, right: f32, w: f32, h: f32) -> Rect {
    let (cy, room) = title_band(inner);
    let h = h.min(room);
    let r = Rect::from_min_size(Pos2::new(right - w, cy - h / 2.0), Vec2::new(w, h));
    // (the window's name keeps clear of the strip's controls)
    winamp_ui::note_strip_control(frame_outer(inner), r.min.x);
    r
}

/// Black window frame with beige trim, title tab and beige body at `inner`; square, with stepped corners.
pub(crate) fn window_deco(ui: &egui::Ui, inner: Rect, title: &str) {
    let p = ui.painter();
    let u = thick(2.0);
    let outer = frame_outer(inner);
    tour::note(title, outer);
    // (the title strip moves the window when it's the app's title bar; the strip's own controls, drawn after, win)
    crate::app_bar_strip(
        ui,
        Rect::from_min_max(outer.min, Pos2::new(outer.max.x, frame_body(inner).min.y)),
        ui.id().with(("app_bar", title)),
    );
    // a worn Winamp skin dresses every window in its own frame
    if winamp_ui::draw_frame(p, outer, frame_body(inner), title) {
        return;
    }
    notch_fill(p, outer, u * 3.0, Color32::BLACK);
    notch_fill(p, outer.shrink(u * 1.5), u * 2.0, pal().accent);
    notch_fill(p, outer.shrink(u * 2.5), u * 2.0, Color32::BLACK);
    notch_fill(p, outer.shrink(u * 3.5), u, pal().groove);
    notch_fill(p, outer.shrink(u * 4.0), u, Color32::BLACK);
    // title tab (with a little pixel icon for the section)
    let label = title.to_string();
    let cy = outer.min.y + 12.0;
    let icon: Option<&[&str]> = match title.split(' ').next().unwrap_or("") {
        "LIBRARY" => Some(&WIN_LIBRARY),
        "QUEUE" => Some(&WIN_QUEUE),
        "LEAD" => Some(&WIN_SHEET),
        "PRACTICE" => Some(&WIN_PRACTICE),
        "PLAYER" => Some(&MARK),
        _ => None,
    };
    let iw = if icon.is_some() { 26.0 } else { 0.0 };
    let tw = (text_w(&label, 2.0) + 28.0 + iw).min(outer.width() - 80.0).max(60.0);
    // left-aligned tab with chamfered top corners, sitting on the frame edge
    let tab = Rect::from_min_size(Pos2::new(outer.min.x + 22.0, cy - 11.0), Vec2::new(tw, 22.0));
    notch_fill(p, tab.expand(3.0), u * 2.0, Color32::BLACK);
    notch_fill(p, tab, u * 1.5, pal().edge);
    ptext_fit(p, tab.center() + Vec2::new(iw / 2.0, 0.0), Align::Center, &label, 2.0, tw - 12.0 - iw, pal().accent);
    if let Some(rows) = icon {
        let o = Pos2::new((tab.min.x + 9.0).round(), (cy - rows.len() as f32).round());
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                // 'o' pixels get a mid tone between tab text and tab fill so they never vanish on any skin
                let col = match ch {
                    '#' => pal().accent,
                    'o' => pal().accent.lerp_to_gamma(pal().edge, 0.4),
                    _ => continue,
                };
                fill_rect(p, Rect::from_min_size(o + Vec2::new(x as f32 * 2.0, y as f32 * 2.0), Vec2::splat(2.0)), col);
            }
        }
    }
    // body
    fill_rect(p, inner.expand(2.0), pal().edge);
    fill_rect(p, inner, pal().panel);
}

pub(crate) const SEGS: [(f32, f32, f32, f32); 7] = [
    (1.0, 0.0, 6.0, 2.0),  // a
    (6.0, 1.0, 2.0, 6.0),  // b
    (6.0, 7.0, 2.0, 6.0),  // c
    (1.0, 12.0, 6.0, 2.0), // d
    (0.0, 7.0, 2.0, 6.0),  // e
    (0.0, 1.0, 2.0, 6.0),  // f
    (1.0, 6.0, 6.0, 2.0),  // g
];

pub(crate) fn seg_mask(d: u32) -> [u8; 7] {
    match d {
        0 => [1, 1, 1, 1, 1, 1, 0],
        1 => [0, 1, 1, 0, 0, 0, 0],
        2 => [1, 1, 0, 1, 1, 0, 1],
        3 => [1, 1, 1, 1, 0, 0, 1],
        4 => [0, 1, 1, 0, 0, 1, 1],
        5 => [1, 0, 1, 1, 0, 1, 1],
        6 => [1, 0, 1, 1, 1, 1, 1],
        7 => [1, 1, 1, 0, 0, 0, 0],
        8 => [1, 1, 1, 1, 1, 1, 1],
        _ => [1, 1, 1, 1, 0, 1, 1],
    }
}

/// Seven-segment LCD digit with faint unlit segments.
pub(crate) fn seg_digit(p: &egui::Painter, o: Pos2, k: f32, d: u32, lit: bool) {
    let m = seg_mask(d);
    for (i, (x, y, w, h)) in SEGS.iter().enumerate() {
        let r = Rect::from_min_size(o + Vec2::new(x * k, y * k), Vec2::new(w * k, h * k));
        fill_rect(p, r, pal().display_ghost);
        if lit && m[i] == 1 {
            fill_rect(p, r, pal().ink);
        }
    }
}

/// Filled disc built from square pixels of size `u` (pixel-art circle).
pub(crate) fn pix_disc(p: &egui::Painter, c: Pos2, r: f32, u: f32, col: Color32) {
    let n = (r / u).round() as i32;
    for k in -n..n {
        let y = (k as f32 + 0.5) * u;
        let hw = ((r * r - y * y).max(0.0)).sqrt();
        let hw = (hw / u).round() * u;
        if hw > 0.0 {
            fill_rect(
                p,
                Rect::from_min_max(Pos2::new(c.x - hw, c.y + k as f32 * u), Pos2::new(c.x + hw, c.y + (k + 1) as f32 * u)),
                col,
            );
        }
    }
}

/// Round transport button drawn in big pixels, with a pixel icon and a pixel shine.
pub(crate) fn round_btn(ui: &egui::Ui, center: Pos2, radius: f32, icon: &[&str], px: f32, id: &str) -> egui::Response {
    let rect = Rect::from_center_size(center, Vec2::splat(radius * 2.0));
    let resp = ui.interact(rect, ui.id().with(id), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let down = resp.is_pointer_button_down_on();
    let u = thick((radius / 9.0).round().max(2.0));
    let off = if down { u } else { 0.0 };
    let p = ui.painter();
    let face = if resp.hovered() { pal().btn_hi } else { pal().btn_face };
    pix_disc(p, center, radius, u, Color32::BLACK);
    pix_disc(p, center + Vec2::new(0.0, off), radius - u, u, face);
    if !down {
        // lower edge shade, then the shine
        pix_disc(p, center + Vec2::new(0.0, u * 0.5), radius - u * 2.0, u, face);
        pix_disc(
            p,
            center + Vec2::new(-radius * 0.3, -radius * 0.34),
            radius * 0.3,
            u,
            Color32::from_rgba_unmultiplied(255, 255, 225, 70),
        );
    }
    let px = px.floor().max(1.0);
    let o = Pos2::new(snap(center.x - 3.5 * px), snap(center.y - 3.5 * px + off));
    pixmap(p, o, px, icon, Color32::from_rgb(34, 32, 16));
    resp
}

/// A little pixel gem with a tide line through it.
pub(crate) fn logo_mark(p: &egui::Painter, origin: Pos2, px: f32) {
    for (y, row) in MARK.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            let col = match ch {
                '#' => pal().ink,
                'o' => pal().ink2,
                _ => continue,
            };
            fill_rect(p, Rect::from_min_size(origin + Vec2::new(x as f32 * px, y as f32 * px), Vec2::splat(px)), col);
        }
    }
}

/// Logo strip at the top of the library window (its words in the skin's font when a skin is worn).
pub(crate) fn logo(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 52.0), Sense::hover());
    let p = ui.painter();
    inset(p, rect, pal().display);
    logo_mark(p, Pos2::new(rect.min.x + 12.0, rect.center().y - 13.5), 3.0);
    let (tsz, ty, gy) = (3.0, -8.0, 13.0);
    ptext(p, Pos2::new(rect.min.x + 50.0, rect.center().y + ty), Align::Min, "BACKLINE", tsz, pal().ink);
    let tag =
        if practice_tools() { "A RETRO PLAYER FOR TIDAL AND MORE - MADE FOR PRACTICE" } else { "A RETRO PLAYER FOR TIDAL AND MORE" };
    let version = if update::build() > 0 { format!("{} .{}", version(), update::build()) } else { version().to_string() };
    // the tagline stops short of the version number, whatever the font's width
    let vw = text_w(&version, 1.0);
    let tag_w = rect.width() - 51.0 - vw - 24.0;
    ptext_fit(p, Pos2::new(rect.min.x + 51.0, rect.center().y + gy), Align::Min, tag, 1.0, tag_w, pal().ink2);
    ptext(p, Pos2::new(rect.max.x - 10.0, rect.center().y + gy), Align::Max, &version, 1.0, pal().dim);
}

/// Folder-style tabs with a baseline; the open tab joins the panel below. Returns the clicked tab.
/// Tabs as one joined strip of segments (a skin worn): the one you are on pressed in and lit. "|" in `items`
/// starts a new strip beside the last. Returns the clicked one.
pub(crate) fn segments(ui: &mut egui::Ui, items: &[&str], cur: usize) -> Option<usize> {
    segments_tips(ui, items, cur, &[])
}

/// `segments`, each with a hover hint from `tips` (by the same place in the list; empty for none).
pub(crate) fn segments_tips(ui: &mut egui::Ui, items: &[&str], cur: usize, tips: &[String]) -> Option<usize> {
    let h = 26.0 * font::ui_scale();
    let pad = 22.0;
    let gap = 10.0;
    // (the brand's name, spelled as a word among the others)
    let items: Vec<&str> = items.iter().map(|s| if *s == "BACKLINE" { "Backline" } else { *s }).collect();
    let items = &items[..];
    let width = ui.available_width() - 4.0;
    // the tabs at full size, onto as many rows as they need (never squeezed to unreadable): each row its own strip
    let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
    let mut used = 0.0;
    for (i, name) in items.iter().enumerate() {
        if *name == "|" {
            used += gap;
            continue;
        }
        let w = text_w(name, 2.0) + pad;
        // (a strip may tighten a little before a tab goes onto the next row)
        if used + w > width / 0.86 && !rows.last().unwrap().is_empty() {
            rows.push(Vec::new());
            used = 0.0;
        }
        rows.last_mut().unwrap().push(i);
        used += w;
    }
    let n_rows = rows.len() as f32;
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), (h + 4.0) * n_rows + 2.0), Sense::hover());
    let p = ui.painter().clone();
    let mut hit = None;
    let mut runs: Vec<(f32, f32, f32)> = Vec::new();
    let mut cells: Vec<(usize, Rect)> = Vec::new();
    for (ri, list) in rows.iter().enumerate() {
        let y = area.min.y + 3.0 + ri as f32 * (h + 4.0);
        let mut x = area.min.x + 2.0;
        let row_w: f32 = list.iter().map(|i| text_w(items[*i], 2.0) + pad).sum::<f32>()
            + gap * list.windows(2).filter(|p| items[p[0] + 1..p[1]].contains(&"|")).count() as f32;
        let k = (width / row_w).min(1.0);
        let mut run_start = x;
        let mut prev: Option<usize> = None;
        for &i in list {
            // a bar in the list between this one and the one before: a new strip
            if prev.is_some_and(|p| items[p + 1..i].contains(&"|")) {
                runs.push((run_start, x, y));
                x += gap * k;
                run_start = x;
            }
            let w = (text_w(items[i], 2.0) + pad) * k;
            cells.push((i, Rect::from_min_max(Pos2::new(x, y), Pos2::new(x + w, y + h))));
            x += w;
            prev = Some(i);
        }
        runs.push((run_start, x, y));
    }
    // plain words (the owner's calm look, 2026-11-24): the one you're on bright and underlined, the rest quieter;
    // a short upright line between groups
    for w in runs.windows(2) {
        let ((_, b, y0), (a, _, y1)) = (w[0], w[1]);
        if y0 == y1 && a > b {
            let x = ((a + b) / 2.0).round();
            p.rect_filled(
                Rect::from_min_max(Pos2::new(x, y0 + h * 0.25), Pos2::new(x + 1.0, y0 + h * 0.75)),
                0.0,
                pal().ink2.gamma_multiply(0.45),
            );
        }
    }
    for (i, r) in cells.iter() {
        let resp =
            ui.interact(*r, ui.id().with(("seg", *i, items[*i])), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let resp = match tips.get(*i) {
            Some(t) if !t.is_empty() => resp.tip(t),
            _ => resp,
        };
        let on = *i == cur;
        if crate::studio::on() {
            // (Studio: the one you're on inverted, the pointer's shaded)
            let k = r.shrink2(Vec2::new(1.0, 2.0));
            if on {
                p.rect_filled(k, 0.0, crate::studio::ink());
            } else if resp.hovered() {
                p.rect_filled(k, 0.0, crate::studio::key_col());
            }
            let col = if on { crate::studio::scr() } else { pal().ink };
            winamp_ui::as_title(|| ui_text(&p, r.center(), Align::Center, items[*i], 2.0, r.width() - 6.0, col));
            if resp.clicked() {
                hit = Some(*i);
            }
            continue;
        }
        let col = if on || resp.hovered() { pal().ink } else { pal().ink2 };
        let tw = winamp_ui::as_title(|| {
            ui_text(&p, r.center(), Align::Center, items[*i], 2.0, r.width() - 6.0, col);
            text_w(items[*i], 2.0)
        });
        if on {
            on_line(&p, *r, tw);
        }
        if resp.clicked() {
            hit = Some(*i);
        }
    }
    hit
}

/// A small switch of views, side by side in one fine outline: each a line icon by name (`vicon::named`), or its
/// words when the name starts with "=" ("=2-5-1"); the one showing filled in. Each says its name on hover (`tips`).
/// Returns the one clicked.
pub(crate) fn icon_switch(ui: &mut egui::Ui, items: &[&str], cur: usize, tips: &[&str]) -> Option<usize> {
    let h = bh() - 2.0;
    let ws: Vec<f32> = items.iter().map(|it| it.strip_prefix('=').map_or(h + 2.0, |t| text_w(t, 1.75) + 14.0)).collect();
    let (row, _) = ui.allocate_exact_size(Vec2::new(ws.iter().sum::<f32>(), h), Sense::hover());
    let p = ui.painter().clone();
    let rad = corner();
    let mut x = row.min.x;
    let mut hit = None;
    for (i, it) in items.iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(x, row.min.y), Vec2::new(ws[i], h));
        x += ws[i];
        let resp = ui.interact(r, ui.id().with(("iswitch", items.join("|"), i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let resp = match tips.get(i) {
            Some(t) if !t.is_empty() => resp.tip(t),
            _ => resp,
        };
        let on = i == cur;
        let studio = crate::studio::on();
        if studio {
            if on {
                p.rect_filled(r, 0.0, crate::studio::ink());
            } else if resp.hovered() {
                p.rect_filled(r, 0.0, crate::studio::key_col());
            }
        } else if hardware() {
            if on {
                led(&p, Pos2::new(r.center().x, r.min.y - 2.0));
            }
        } else if on {
            p.rect_filled(r.shrink(2.0), rad * 0.75, pal().ink.gamma_multiply(0.16));
        } else if resp.hovered() {
            p.rect_filled(r.shrink(2.0), rad * 0.75, pal().ink.gamma_multiply(0.07));
        }
        let col = if studio && on {
            crate::studio::scr()
        } else if on || resp.hovered() {
            pal().ink
        } else {
            pal().ink2
        };
        match it.strip_prefix('=') {
            Some(t) => {
                font::keep_caps(|| ptext(&p, r.center(), Align::Center, t, 1.75, col));
            }
            None => crate::vicon::named(&p, Rect::from_center_size(r.center(), Vec2::splat(h * 0.56)), it, col),
        }
        if resp.clicked() {
            hit = Some(i);
        }
    }
    if crate::studio::on() {
        p.rect_stroke(row, 0.0, Stroke::new(1.0_f32, crate::studio::ink()));
    } else if !hardware() {
        p.rect_stroke(row, rad, Stroke::new(1.0_f32, pal().ink2.gamma_multiply(0.35)));
    }
    hit
}

/// A bare line icon (`vicon::named`) as a button: no box, a soft shade under the pointer; its name on hover.
pub(crate) fn line_icon_btn(ui: &mut egui::Ui, icon: &str, tip: &str) -> egui::Response {
    let h = bh();
    let (r, resp) = ui.allocate_exact_size(Vec2::new(h + 2.0, h), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
    quiet_h(ui.painter(), r, resp.is_pointer_button_down_on(), resp.hovered());
    let col = if resp.hovered() { pal().ink } else { pal().ink2 };
    crate::vicon::named(ui.painter(), Rect::from_center_size(r.center(), Vec2::splat(h * 0.58)), icon, col);
    resp
}

pub(crate) fn tab_row(ui: &mut egui::Ui, items: &[&str], cur: usize) -> Option<usize> {
    tab_row_tips(ui, items, cur, &[])
}

/// `tab_row`, each tab with a hover hint from `tips` (by the same place in the list; empty for none).
pub(crate) fn tab_row_tips(ui: &mut egui::Ui, items: &[&str], cur: usize, tips: &[String]) -> Option<usize> {
    if winamp_ui::chrome_on() {
        return segments_tips(ui, items, cur, tips);
    }
    font::keep_caps(|| {
        let h = 26.0 * font::ui_scale();
        let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h + 6.0), Sense::hover());
        let t = thick(2.0);
        let base = row.max.y - 4.0;
        fill_rect(ui.painter(), Rect::from_min_max(Pos2::new(row.min.x, base), Pos2::new(row.max.x, base + t)), pal().edge);
        let (mut x, mut hit) = (row.min.x + 4.0, None);
        // too many tabs for the width: tighten the padding first, then squeeze them evenly (the text shrinks to fit)
        let avail = (row.width() - 8.0).max(60.0);
        let gaps = 3.0 * items.len().saturating_sub(1) as f32;
        let total = |pad: f32| items.iter().map(|n| text_w(n, 2.0) + pad).sum::<f32>() + gaps;
        let mut pad = 24.0;
        while pad > 10.0 && total(pad) > avail {
            pad -= 2.0;
        }
        let shrink = (avail / total(pad)).min(1.0);
        for (i, name) in items.iter().enumerate() {
            let w = (text_w(name, 2.0) + pad) * shrink;
            let on = i == cur;
            let r = Rect::from_min_max(
                Pos2::new(x, if on { row.min.y } else { row.min.y + 5.0 }),
                Pos2::new(x + w, base + if on { t } else { 0.0 }),
            );
            let resp = ui
                .interact(r, ui.id().with(("tab", items.join("|"), i)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let resp = match tips.get(i) {
                Some(t) if !t.is_empty() => resp.tip(t),
                _ => resp,
            };
            let p = ui.painter();
            // in the skin's colours (a piece of its title bar would bring its stripes through the words)
            {
                fill_rect(
                    p,
                    r,
                    if on {
                        pal().panel_lt
                    } else if resp.hovered() {
                        pal().panel_hover
                    } else {
                        pal().panel_dk
                    },
                );
                fill_rect(p, Rect::from_min_size(r.min, Vec2::new(r.width(), t)), pal().edge);
                fill_rect(p, Rect::from_min_size(r.min, Vec2::new(t, r.height())), pal().edge);
                fill_rect(p, Rect::from_min_size(Pos2::new(r.max.x - t, r.min.y), Vec2::new(t, r.height())), pal().edge);
                // chipped top corners
                fill_rect(p, Rect::from_min_size(r.min, Vec2::splat(t)), pal().panel);
                fill_rect(p, Rect::from_min_size(Pos2::new(r.max.x - t, r.min.y), Vec2::splat(t)), pal().panel);
            }
            ui_text(
                p,
                Pos2::new(r.center().x, r.min.y + (base - r.min.y) / 2.0 + 1.0),
                Align::Center,
                name,
                2.0,
                w - 8.0,
                if on { pal().ink } else { pal().ink2 },
            );
            if resp.clicked() {
                hit = Some(i);
            }
            x += w + 3.0;
        }
        hit
    })
}

/// Small pixel check box with a label. Returns the click.
pub(crate) fn check_box(ui: &mut egui::Ui, text: &str, on: bool) -> egui::Response {
    let w = text_w(text, 2.0) + 30.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::click());
    let b = Rect::from_center_size(Pos2::new(rect.min.x + 11.0, rect.center().y), Vec2::splat(16.0));
    inset(ui.painter(), b, pal().display);
    if on {
        pixmap(ui.painter(), Pos2::new(b.min.x + 3.0, b.min.y + 4.0), 2.0, &CHECK[..], pal().ink);
    }
    ptext(
        ui.painter(),
        Pos2::new(rect.min.x + 26.0, rect.center().y),
        Align::Min,
        text,
        2.0,
        if resp.hovered() { pal().ink } else { pal().ink2 },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) fn retro_btn_w(ui: &mut egui::Ui, text: &str, w: f32, active: bool) -> egui::Response {
    let w = (w * font::ui_scale()).max(text_w(text, 2.0) + 16.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::click());
    let down = resp.is_pointer_button_down_on() || active;
    if winamp_ui::chrome_on() {
        // plain words: brighter under the pointer, underlined while it's the one picked (Hardware: a rubber key, a
        // light on it while it's the one picked)
        if crate::studio::on() {
            // (Studio: a key in a fine outline, inverted while it's the one picked)
            crate::studio::key(ui.painter(), rect.shrink2(Vec2::new(1.0, 2.0)), active, resp.is_pointer_button_down_on(), resp.hovered());
            let col = crate::studio::key_fg(active);
            winamp_ui::as_title(|| ui_text(ui.painter(), rect.center(), Align::Center, text, 2.0, rect.width() - 8.0, col));
            return resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        if hardware() {
            key_face(ui.painter(), rect.shrink2(Vec2::new(1.0, 2.0)), resp.is_pointer_button_down_on(), resp.hovered());
            let dy = if resp.is_pointer_button_down_on() { 1.0 } else { 0.0 };
            winamp_ui::as_title(|| {
                ui_text(ui.painter(), rect.center() + Vec2::new(0.0, dy), Align::Center, text, 2.0, rect.width() - 8.0, pal().ink)
            });
            if active {
                led(ui.painter(), Pos2::new(rect.min.x + 7.0, rect.min.y + 7.0));
            }
            return resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        quiet_h(ui.painter(), rect, resp.is_pointer_button_down_on(), resp.hovered());
        let col = if active || resp.hovered() { pal().ink } else { pal().ink2 };
        let tw = winamp_ui::as_title(|| {
            ui_text(ui.painter(), rect.center(), Align::Center, text, 2.0, rect.width() - 8.0, col);
            text_w(text, 2.0)
        });
        if active {
            on_line(ui.painter(), rect, tw);
        }
        return resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    raised_h(ui.painter(), rect, down, resp.hovered());
    let dy = if down { 1.0 } else { 0.0 };
    winamp_ui::as_title(|| {
        ui_text(ui.painter(), rect.center() + Vec2::new(0.0, dy), Align::Center, text, 2.0, rect.width() - 8.0, pal().ink)
    });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Button sized to its label.
pub(crate) fn retro_btn(ui: &mut egui::Ui, text: &str, active: bool) -> egui::Response {
    let w = (text_w(text, 2.0) + 20.0).max(44.0);
    retro_btn_w(ui, text, w, active)
}

/// A heading in small spaced capitals with an accent line under it (a skin worn): `px` the text size. Returns how
/// wide it came out.
pub(crate) fn spaced_caps(p: &egui::Painter, at: Pos2, text: &str, px: f32, col: Color32) -> f32 {
    font::keep_caps(|| {
        let up = text.to_uppercase();
        let gap = 1.6 * px;
        let mut x = at.x;
        for ch in up.chars() {
            let s = ch.to_string();
            // drawn twice, a hair apart: a little weight, as a bold face would
            ptext(p, Pos2::new(x, at.y), Align::Min, &s, px, col);
            let w = ptext(p, Pos2::new(x + 0.6, at.y), Align::Min, &s, px, col);
            x += w + gap;
        }
        let w = (x - gap - at.x).max(0.0);
        p.rect_filled(Rect::from_min_size(Pos2::new(at.x, at.y + 5.0 * px + 4.0), Vec2::new(w, 2.0)), 0.0, pal().bar_txt);
        w
    })
}

pub(crate) fn section_header(ui: &mut egui::Ui, text: &str) {
    if winamp_ui::chrome_on() {
        ui.add_space(8.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), Sense::hover());
        spaced_caps(ui.painter(), Pos2::new(rect.min.x + 4.0, rect.min.y + 9.0), text, 1.4, pal().ink);
        return;
    }
    font::keep_caps(|| {
        ui.add_space(6.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 20.0), Sense::hover());
        fill_rect(ui.painter(), rect, pal().edge);
        ui_text(
            ui.painter(),
            Pos2::new(rect.min.x + 8.0, rect.center().y),
            Align::Min,
            text,
            2.0,
            rect.width() - 16.0,
            pal().accent,
        );
    })
}

/// Thin sub-heading bar used inside the queue.
pub(crate) fn mini_header(ui: &mut egui::Ui, text: &str) {
    font::keep_caps(|| {
        let w = ui.available_width();
        let probe = Rect::from_min_size(ui.cursor().min, Vec2::new(w, HEAD_H));
        if !ui.is_rect_visible(probe) {
            ui.allocate_space(Vec2::new(w, HEAD_H));
            return;
        }
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, HEAD_H), Sense::hover());
        if winamp_ui::chrome_on() {
            spaced_caps(ui.painter(), Pos2::new(rect.min.x + 4.0, rect.min.y + 4.0), text, 1.1, pal().ink2);
            return;
        }
        fill_rect(ui.painter(), rect, pal().groove);
        ptext(ui.painter(), Pos2::new(rect.min.x + 8.0, rect.center().y), Align::Min, text, 1.0, pal().bar_txt);
    })
}

/// One piece of text in a list row: the skin's playlist font when a Winamp skin is on, Backline's pixel font
/// otherwise; cut to fit `max_w`. Returns how wide it came out.
pub(crate) fn row_text(p: &egui::Painter, pos: Pos2, align: Align, text: &str, max_w: f32, col: Color32) -> f32 {
    if winamp_ui::chrome_on() {
        // smooth text (Inter), cut with "..." to fit
        let font = egui::FontId::proportional((ROW_H * 0.62 * font::ui_scale()).max(11.0));
        let fits = |s: &str| p.layout_no_wrap(s.to_string(), font.clone(), col).size().x <= max_w;
        let mut s = text.to_string();
        if !fits(&s) {
            while !s.is_empty() && !fits(&format!("{}...", s)) {
                s.pop();
            }
            s = format!("{}...", s.trim_end());
        }
        let g = p.layout_no_wrap(s, font, col);
        let w = g.size().x;
        let x = match align {
            Align::Min => pos.x,
            Align::Center => pos.x - w / 2.0,
            Align::Max => pos.x - w,
        };
        p.galley(Pos2::new(x, pos.y - g.size().y / 2.0), g, col);
        w
    } else {
        ptext(p, pos, align, &fit(text, 2.0, max_w), 2.0, col)
    }
}

/// Retro-styled entry for right-click menus. Returns true when clicked.
pub(crate) fn menu_item(ui: &mut egui::Ui, text: &str) -> bool {
    let sc = font::ui_scale();
    let w = (text_w(text, 2.0) + 28.0 * sc).max(150.0 * sc);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 24.0 * sc), Sense::click());
    if resp.hovered() {
        fill_rect(ui.painter(), rect, pal().sel);
    }
    // (a control: Title Case, as buttons and tabs are)
    winamp_ui::as_title(|| ptext(ui.painter(), Pos2::new(rect.min.x + 10.0, rect.center().y), Align::Min, text, 2.0, pal().ink));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// A row in a menu that opens a submenu, looking like `menu_item` (the same text and highlight) with an arrow.
/// egui's own submenu button does the opening; this draws over it.
pub(crate) fn sub_menu(ui: &mut egui::Ui, text: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    let sc = font::ui_scale();
    let w = (text_w(text, 2.0) + 40.0 * sc).max(150.0 * sc);
    ui.set_min_width(w);
    let r = ui.menu_button(egui::RichText::new(" ").size(16.0 * sc), add_contents).response;
    let rect = Rect::from_min_size(r.rect.min, Vec2::new(r.rect.width().max(w), r.rect.height()));
    let p = ui.painter();
    p.rect_filled(rect, 0.0, if r.hovered() { pal().sel } else { ui.visuals().window_fill() });
    winamp_ui::as_title(|| ptext(p, Pos2::new(rect.min.x + 10.0, rect.center().y), Align::Min, text, 2.0, pal().ink));
    let (c, s) = (Pos2::new(rect.max.x - 10.0 * sc, rect.center().y), 4.0 * sc);
    p.add(egui::Shape::convex_polygon(
        vec![c + Vec2::new(-s * 0.6, -s), c + Vec2::new(s * 0.6, 0.0), c + Vec2::new(-s * 0.6, s)],
        pal().ink,
        Stroke::NONE,
    ));
}

/// Dropdown button: shows `name: current` with a small arrow; the list opens as an overlay
/// right under it (no layout change). Returns the index chosen this frame.
pub(crate) fn dropdown(ui: &mut egui::Ui, id: &str, title: &str, items: &[&str], sel: usize, min_w: f32) -> Option<usize> {
    dropdown_at(ui, id, title, items, sel, min_w, false)
}

/// One choice on a line of its own: its name, then a dropdown of the options. Returns the index chosen this frame.
pub(crate) fn choice_line(ui: &mut egui::Ui, id: &str, name: &str, items: &[&str], sel: usize, w: f32) -> Option<usize> {
    let mut out = None;
    ui.horizontal(|ui| {
        form_label(ui, name);
        // as wide as its longest option (so it doesn't jump about), never past the page; the text stays full size
        let widest = items.iter().map(|s| text_w(s, 2.0)).fold(0.0, f32::max) + 56.0;
        let room = (ui.available_width() - 4.0).max(120.0);
        out = dropdown_fixed(ui, id, items, sel, widest.max(w.min(widest)).min(room));
    });
    out
}

/// A dropdown of exactly `w` wide whatever it shows (a long choice is cut to fit), so what sits beside it stays put.
pub(crate) fn dropdown_fixed(ui: &mut egui::Ui, id: &str, items: &[&str], sel: usize, w: f32) -> Option<usize> {
    dropdown_at(ui, id, "", items, sel, w, true)
}

pub(crate) fn dropdown_at(
    ui: &mut egui::Ui,
    id: &str,
    title: &str,
    items: &[&str],
    sel: usize,
    min_w: f32,
    fixed: bool,
) -> Option<usize> {
    dropdown_list(ui, id, title, items, sel, min_w, fixed, &[], &[], None)
}

/// `menu_list` at a set width `w`: a face too long for it is cut short with ...
pub(crate) fn menu_list_w(
    ui: &mut egui::Ui,
    id: &str,
    face: &str,
    items: &[&str],
    sel: usize,
    heads: &[(usize, &str)],
    w: f32,
) -> Option<usize> {
    dropdown_list(ui, id, "", items, sel, w, true, heads, &[], Some(face))
}

/// A dropdown whose list is in headed groups (`heads`: the item each group starts at, and its heading), each item
/// with a coloured dot before it when `dots` has one for it. Returns the index chosen this frame.
#[allow(clippy::too_many_arguments)]
pub(crate) fn dropdown_grouped(
    ui: &mut egui::Ui,
    id: &str,
    title: &str,
    items: &[&str],
    sel: usize,
    heads: &[(usize, &str)],
    dots: &[Color32],
) -> Option<usize> {
    dropdown_list(ui, id, title, items, sel, 0.0, false, heads, dots, None)
}

/// One step of a page's path ("Theory", "Chords", "Chord Book"): plain words, all one size; the last one (`last`,
/// the page you're on) brighter and a little heavier. With `items`, a small chevron after it, and a click lists them
/// (in headed groups, `heads`) to go to one. Returns the one picked this frame.
pub(crate) fn crumb(
    ui: &mut egui::Ui,
    id: &str,
    face: &str,
    items: &[&str],
    sel: usize,
    heads: &[(usize, &str)],
    last: bool,
) -> Option<usize> {
    let menu = !items.is_empty();
    // (Hardware writes the path in capitals)
    let tw = if hardware() { font::keep_caps(|| text_w(&face.to_uppercase(), 2.0)) } else { text_w(face, 2.0) };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(tw + if menu { 26.0 } else { 12.0 }, bh()), Sense::click());
    let key = egui::Id::new(("dd", id));
    let open = menu && ui.ctx().data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    let hot = menu && resp.hovered();
    let resp = if menu { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
    quiet_h(ui.painter(), rect, open || (menu && resp.is_pointer_button_down_on()), hot);
    let col = if last || open || hot { pal().ink } else { pal().ink2 };
    let at = Pos2::new(rect.min.x + 6.0, rect.center().y);
    if hardware() {
        // (the Hardware style: the whole path on one lit screen, `page_path` lays it under; the page brightest)
        let (amber, _) = crate::skin::hw_screen();
        let a = if last || open || hot { amber } else { amber.gamma_multiply(0.62) };
        font::keep_caps(|| {
            let up = face.to_uppercase();
            ptext(ui.painter(), at, Align::Min, &up, 2.0, a.gamma_multiply(0.3));
            ptext(ui.painter(), at + Vec2::new(0.4, 0.0), Align::Min, &up, 2.0, a);
        });
        if menu {
            chevron(ui.painter(), Pos2::new(at.x + text_w(&face.to_uppercase(), 2.0) + 9.0, rect.center().y + 1.0), 3.0, amber.gamma_multiply(0.6));
            return dd_popup(ui, key, rect, &resp, 0.0, items, sel, heads, &[]);
        }
        return None;
    }
    ptext(ui.painter(), at, Align::Min, face, 2.0, col);
    if last {
        // (the page you're on: the same words again a hair over, a little weight)
        ptext(ui.painter(), at + Vec2::new(0.5, 0.0), Align::Min, face, 2.0, col);
    }
    if !menu {
        return None;
    }
    chevron(ui.painter(), Pos2::new(at.x + tw + 9.0, rect.center().y + 1.0), 3.0, pal().ink2.gamma_multiply(0.8));
    dd_popup(ui, key, rect, &resp, 0.0, items, sel, heads, &[])
}

/// The mark between two steps of a page's path: a small chevron pointing on.
pub(crate) fn crumb_sep(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(10.0, bh()), Sense::hover());
    let (c, s) = (r.center(), 3.5);
    let pts = vec![c + Vec2::new(-s * 0.5, -s), c + Vec2::new(s * 0.5, 0.0), c + Vec2::new(-s * 0.5, s)];
    let col = if hardware() { crate::skin::hw_screen().0.gamma_multiply(0.5) } else { pal().ink2.gamma_multiply(0.7) };
    ui.painter().add(egui::Shape::line(pts, Stroke::new(1.5_f32, col)));
}

#[allow(clippy::too_many_arguments)]
fn dropdown_list(
    ui: &mut egui::Ui,
    id: &str,
    title: &str,
    items: &[&str],
    sel: usize,
    min_w: f32,
    fixed: bool,
    heads: &[(usize, &str)],
    dots: &[Color32],
    face: Option<&str>,
) -> Option<usize> {
    // (a button that always reads the same shows that, not the choice)
    let cur = face.unwrap_or_else(|| items.get(sel).copied().unwrap_or(""));
    // (a title in capitals reads as a word beside a choice in mixed case: "Input: Microphone", not "INPUT: Microphone")
    // (each half cased on its own: a mixed-case title would leave a choice in capitals as it is)
    let (title, cur) = if winamp_ui::chrome_on() {
        (winamp_ui::display_case(title).into_owned(), winamp_ui::display_case(cur).into_owned())
    } else {
        (title.to_string(), cur.to_string())
    };
    let text = if title.is_empty() { cur.clone() } else { format!("{}: {}", title, cur) };
    // (as wide as its words: with no box round it, more would only read as a gap)
    let natural = if winamp_ui::chrome_on() {
        let t = if title.is_empty() { 0.0 } else { text_w(&title, 2.0) + text_w(" ", 2.0) + 2.0 };
        t + text_w(&cur, 2.0) + 30.0
    } else {
        text_w(&text, 2.0) + 38.0
    };
    let w = if fixed { min_w } else { natural.max(min_w) };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::click());
    let key = egui::Id::new(("dd", id));
    let open = ui.ctx().data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    let down = resp.is_pointer_button_down_on() || open;
    // (too long for its room: cut short with ..., never shrunk to unreadable)
    let cut = |text: &str, room: f32| -> String {
        if text_w(text, 2.0) <= room {
            return text.to_string();
        }
        let mut c: Vec<char> = text.chars().collect();
        while !c.is_empty() && text_w(&(c.iter().collect::<String>() + "..."), 2.0) > room {
            c.pop();
        }
        c.iter().collect::<String>().trim_end().to_string() + "..."
    };
    if winamp_ui::chrome_on() {
        // a choice read as part of a sentence: its name quiet, what's picked bright, a small chevron after it; no box
        quiet_h(ui.painter(), rect, down, resp.hovered());
        let x0 = rect.min.x + 8.0;
        let end = rect.max.x - 22.0;
        let mut x = x0;
        if hardware() && !title.is_empty() {
            // (the Hardware style: its name printed small above it, the choice under)
            font::keep_caps(|| {
                ptext(ui.painter(), Pos2::new(x0, rect.min.y + 4.0), Align::Min, &title.to_uppercase(), 1.0, pal().dim)
            });
            let v = cut(&cur, (end - x0).max(10.0));
            let vw = ptext(ui.painter(), Pos2::new(x0, rect.center().y + 5.0), Align::Min, &v, 1.75, pal().ink);
            chevron(ui.painter(), Pos2::new((x0 + vw + 10.0).min(rect.max.x - 10.0), rect.center().y + 5.0), 3.0, pal().ink2);
            return dd_popup(ui, key, rect, &resp, w, items, sel, heads, dots);
        }
        if !title.is_empty() {
            let t = cut(&title, (end - x).max(10.0));
            x += ptext(ui.painter(), Pos2::new(x, rect.center().y), Align::Min, &t, 2.0, pal().ink2) + text_w(" ", 2.0) + 2.0;
        }
        let v = cut(&cur, (end - x).max(10.0));
        let vw = ptext(ui.painter(), Pos2::new(x, rect.center().y), Align::Min, &v, 2.0, pal().ink);
        chevron(ui.painter(), Pos2::new((x + vw + 11.0).min(rect.max.x - 10.0), rect.center().y + 1.0), 3.5, pal().ink2);
    } else {
        raised_h(ui.painter(), rect, down, resp.hovered());
        let dy = if down { 1.0 } else { 0.0 };
        let shown = cut(&text, w - 38.0);
        ptext(ui.painter(), Pos2::new(rect.min.x + 10.0, rect.center().y + dy), Align::Min, &shown, 2.0, pal().ink);
        pixmap(
            ui.painter(),
            Pos2::new(rect.max.x - 20.0, rect.center().y - 3.0 + dy),
            2.0,
            &["#######", ".#####.", "..###..", "...#..."],
            pal().ink,
        );
    }
    dd_popup(ui, key, rect, &resp, w, items, sel, heads, dots)
}

/// The list a dropdown opens (under `rect`, or above it when there's more room there), toggled by a click on `resp`;
/// `w` the least width. Returns the item picked this frame.
#[allow(clippy::too_many_arguments)]
fn dd_popup(
    ui: &mut egui::Ui,
    key: egui::Id,
    rect: Rect,
    resp: &egui::Response,
    w: f32,
    items: &[&str],
    sel: usize,
    heads: &[(usize, &str)],
    dots: &[Color32],
) -> Option<usize> {
    let open = ui.ctx().data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    // the first frame the list shows, it scrolls to the item chosen now
    let fresh = key.with("fresh");
    if resp.clicked() {
        ui.ctx().data_mut(|d| {
            d.insert_temp(key, !open);
            d.insert_temp(fresh, !open);
        });
    }
    let just_opened = open && ui.ctx().data_mut(|d| d.remove_temp::<bool>(fresh)).unwrap_or(false);
    let mut pick = None;
    if open {
        // never taller than the room on screen: under the button, or above it when there is more room there;
        // a long list scrolls, opening on the item chosen now
        let screen = ui.ctx().screen_rect();
        let full = items.len() as f32 * 24.0 + heads.len() as f32 * 22.0 + 4.0;
        let below = screen.max.y - rect.max.y - 10.0;
        let above = rect.min.y - screen.min.y - 10.0;
        let up = full > below && above > below;
        let room = if up { above } else { below }.max(120.0);
        let h = full.min(room);
        let at = if up { Pos2::new(rect.min.x, rect.min.y - 1.0 - h) } else { Pos2::new(rect.min.x, rect.max.y + 1.0) };
        // (room for the dots, when there are any)
        let indent = if dots.is_empty() { 0.0 } else { 16.0 };
        let iw =
            items.iter().map(|it| text_w(it, 2.0) + 28.0 + indent).fold(w - 4.0, f32::max) + if full > room { 12.0 } else { 0.0 };
        let area = egui::Area::new(key.with("area")).order(egui::Order::Foreground).fixed_pos(at).show(ui.ctx(), |ui| {
            egui::Frame::none()
                .fill(pal().panel_lt)
                .stroke(egui::Stroke::new(1.0_f32, pal().edge))
                .rounding(corner())
                .inner_margin(egui::Margin::same(3.0))
                .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                egui::ScrollArea::vertical().max_height(h - 4.0).auto_shrink([true, true]).show(ui, |ui| {
                    for (i, it) in items.iter().enumerate() {
                        // a group's heading, small and quiet, above its first item
                        if let Some((_, h)) = heads.iter().find(|h| h.0 == i) {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(iw, 22.0), Sense::hover());
                            font::keep_caps(|| {
                                ptext(ui.painter(), Pos2::new(r.min.x + 10.0, r.center().y + 2.0), Align::Min, h, 1.0, pal().dim)
                            });
                        }
                        let (r, rr) = ui.allocate_exact_size(Vec2::new(iw, 24.0), Sense::click());
                        if i == sel || rr.hovered() {
                            ui.painter().rect_filled(r, corner() * 0.75, pal().sel);
                        }
                        if let Some(c) = dots.get(i) {
                            ui.painter().circle_filled(Pos2::new(r.min.x + 16.0, r.center().y), 4.0, *c);
                        }
                        // (two spaces before it: one of the item above's own pages, set in under it)
                        let (step, it) = match it.strip_prefix("  ") {
                            Some(s) => (20.0, s),
                            None => (0.0, *it),
                        };
                        ptext(ui.painter(), Pos2::new(r.min.x + 10.0 + indent + step, r.center().y), Align::Min, it, 2.0, pal().ink);
                        if i == sel && just_opened {
                            rr.scroll_to_me(Some(Align::Center));
                        }
                        if rr.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            pick = Some(i);
                        }
                    }
                });
            });
        });
        let outside = ui.ctx().input(|i| i.pointer.any_click()) && !resp.hovered() && !area.response.hovered();
        if pick.is_some() || outside || ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
            ui.ctx().data_mut(|d| d.insert_temp(key, false));
        }
    }
    pick
}

/// Pixel-font hover hint (egui's own tooltip would use its default font).
pub(crate) trait Tip {
    fn tip(self, text: impl AsRef<str>) -> Self;
}

impl Tip for egui::Response {
    fn tip(self, text: impl AsRef<str>) -> Self {
        let on = self.hovered();
        if self.ctx.animate_bool_with_time(self.id.with("tip"), on, 0.45) < 1.0 || !on {
            return self;
        }
        let Some(m) = self.ctx.input(|i| i.pointer.hover_pos()) else { return self };
        let px = spx(1.0);
        // one line, wrapping only when it would not fit
        let lines = wrap(text.as_ref(), px, (self.ctx.screen_rect().width() - 24.0).clamp(120.0, 640.0));
        let h = lines.len() as f32 * (px * 9.0 + 5.0) + 12.0;
        let w = lines.iter().map(|l| text_w(l, px)).fold(0.0, f32::max) + 16.0;
        let screen = self.ctx.screen_rect();
        let x = (m.x + 12.0).min(screen.max.x - w - 4.0).max(4.0);
        let y = if m.y + 24.0 + h > screen.max.y { m.y - h - 8.0 } else { m.y + 20.0 };
        let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
        let p = self.ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("tip")));
        fill_rect(&p, r, pal().display);
        outline(&p, r, 1.0, pal().ink);
        for (i, l) in lines.iter().enumerate() {
            ptext(
                &p,
                Pos2::new(r.min.x + 8.0, r.min.y + 6.0 + px * 4.5 + i as f32 * (px * 9.0 + 5.0)),
                Align::Min,
                l,
                px,
                pal().ink,
            );
        }
        self
    }
}

/// Small raised chip button (used on window title lines).
pub(crate) fn chip(ui: &egui::Ui, rect: Rect, text: &str, active: bool, id: &str) -> egui::Response {
    let r = ui.interact(rect, ui.id().with(id), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    if winamp_ui::chrome_on() {
        quiet_h(ui.painter(), rect, active || r.is_pointer_button_down_on(), r.hovered() && !active);
    } else {
        raised_h(ui.painter(), rect, active || r.is_pointer_button_down_on(), r.hovered() && !active);
    }
    ui_text(ui.painter(), rect.center(), Align::Center, text, 1.0, rect.width() - 6.0, pal().ink);
    r
}

/// Sunken LCD readout box.
pub(crate) fn lcd_box(ui: &mut egui::Ui, text: &str, w: f32, col: Color32) {
    // never narrower than its text (a wide font or a bigger number would be cut off)
    let w = (w * font::ui_scale()).max(text_w(text, 2.0) + 16.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
    inset(ui.painter(), rect, pal().display);
    ptext_fit(ui.painter(), rect.center(), Align::Center, text, 2.0, w - 12.0, col);
}

/// Wrapped paragraph in the list well.
pub(crate) fn para(ui: &mut egui::Ui, text: &str, col: Color32) {
    if winamp_ui::chrome_on() {
        // a warning is shown whole; an explanation is one quiet line, the rest under the pointer
        if col == pal().ink2 {
            about_line(ui, text);
        } else {
            note_box(ui, text, col);
        }
        return;
    }
    let w = ui.available_width() - 20.0;
    for line in wrap(text, 2.0, w) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
        ptext(ui.painter(), Pos2::new(rect.min.x + 8.0, rect.center().y), Align::Min, &line, 2.0, col);
    }
    ui.add_space(6.0);
}

/// An explanation kept out of the way (a skin worn): an "i" and its first sentence on one dim line; the whole of it
/// shows when the pointer is over it.
fn about_line(ui: &mut egui::Ui, text: &str) {
    let text = winamp_ui::caps_words(text);
    let first = text
        .char_indices()
        .find(|(i, c)| matches!(c, '.' | '?' | '!') && text[i + 1..].starts_with(' ') && *i > 12)
        .map_or(text.as_str(), |(i, _)| &text[..i + 1]);
    let px = 1.5;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 7.0 * spx(px) + 10.0), Sense::hover());
    // (up to what is in view: a page in a scroll area can be wider than its window; nothing drawn past it)
    let room = rect.width().min(ui.clip_rect().max.x - rect.min.x - 14.0) - 12.0;
    let p = ui
        .painter()
        .with_clip_rect(Rect::from_min_size(rect.min, Vec2::new(room + 12.0, rect.height())).intersect(ui.clip_rect()));
    let p = &p;
    let hot = resp.hovered();
    let col = if hot { pal().ink } else { pal().ink2 };
    ptext_fit(p, Pos2::new(rect.min.x + 6.0, rect.center().y), Align::Min, first, px, room, col);
    if first.len() < text.len() {
        resp.on_hover_ui(|ui| {
            ui.set_max_width(420.0);
            ui.label(text.as_str());
        });
    }
}

/// A paragraph as a note (a skin worn): a box with an accent edge; its first sentence, in white, is the title and the
/// rest follows dimmer.
fn note_box(ui: &mut egui::Ui, text: &str, col: Color32) {
    let text = winamp_ui::caps_words(text);
    let w = (ui.available_width() - 8.0).min(820.0 * font::ui_scale());
    let (px_x, px_y) = (14.0, 9.0);
    let tw = w - 2.0 * px_x - 3.0;
    let lh = (font::text_px(2.0) * 1.4).round();
    // the first sentence (a full stop, question or exclamation mark, then a space)
    let cut = text
        .char_indices()
        .find(|(i, c)| matches!(c, '.' | '?' | '!') && text[i + 1..].starts_with(' ') && *i > 12)
        .map(|(i, _)| i + 1);
    let (head, rest) = match cut {
        Some(i) => (text[..i].trim(), text[i..].trim()),
        None => ("", text.trim()),
    };
    let hl = if head.is_empty() { Vec::new() } else { wrap(head, 2.0, tw) };
    let bl = if rest.is_empty() { Vec::new() } else { wrap(rest, 2.0, tw) };
    let gap = if hl.is_empty() || bl.is_empty() { 0.0 } else { 4.0 };
    let h = 2.0 * px_y + (hl.len() + bl.len()) as f32 * lh + gap;
    ui.add_space(2.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let rect = Rect::from_min_max(Pos2::new(rect.min.x + 4.0, rect.min.y), rect.max);
    let p = ui.painter();
    p.rect_filled(rect, 4.0, pal().display.lerp_to_gamma(pal().panel, 0.3));
    p.rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
        egui::Rounding { nw: 2.0, sw: 2.0, ne: 0.0, se: 0.0 },
        pal().bar_txt,
    );
    let x = rect.min.x + 3.0 + px_x;
    let mut y = rect.min.y + px_y + lh / 2.0;
    // (a warning keeps its own colour throughout)
    let (hc, bc) = if col == pal().ink2 { (pal().ink, pal().ink2) } else { (col, col) };
    for line in &hl {
        ptext(p, Pos2::new(x, y), Align::Min, line, 2.0, hc);
        ptext(p, Pos2::new(x + 0.5, y), Align::Min, line, 2.0, hc);
        y += lh;
    }
    y += gap;
    for line in &bl {
        ptext(p, Pos2::new(x, y), Align::Min, line, 2.0, if hl.is_empty() && col == pal().ink2 { pal().ink } else { bc });
        y += lh;
    }
    ui.add_space(8.0);
}

/// Every line of a paragraph, wrapped to the width (the notes of WHAT'S NEW: nothing tucked away under the pointer).
pub(crate) fn wrapped(ui: &mut egui::Ui, text: &str, col: Color32) {
    let text = if winamp_ui::chrome_on() { winamp_ui::caps_words(text) } else { text.to_string() };
    let w = ui.available_width() - 12.0;
    let lh = (font::text_px(2.0) * 1.35).max(18.0);
    for line in wrap(&text, 2.0, w) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), lh), Sense::hover());
        ptext(ui.painter(), Pos2::new(rect.min.x + 6.0, rect.center().y), Align::Min, &line, 2.0, col);
        // (lines of one paragraph sit together, whatever gap the page puts between things)
        let gap = ui.spacing().item_spacing.y;
        if gap > 0.0 {
            ui.add_space(-gap);
        }
    }
    ui.add_space(6.0);
}

/// Single heading line, shortened with "..." if it doesn't fit.
pub(crate) fn title_line(ui: &mut egui::Ui, text: &str, px: f32, col: Color32) {
    if winamp_ui::chrome_on() && px >= 2.5 && text.chars().any(|c| c.is_alphabetic()) && !text.chars().any(|c| c.is_lowercase()) {
        ui.add_space(4.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::hover());
        spaced_caps(ui.painter(), Pos2::new(rect.min.x + 4.0, rect.min.y + 12.0), text, 1.9, col);
        return;
    }
    // a line of words under a heading (a skin worn): it starts with a capital, and a button named in capitals reads
    // as a name
    let fixed;
    let text = if winamp_ui::chrome_on() && text.chars().any(|c| c.is_lowercase()) {
        let t = winamp_ui::caps_words(text);
        let mut c = t.chars();
        fixed = match c.next() {
            Some(f) if f.is_lowercase() => f.to_uppercase().chain(c).collect::<String>(),
            _ => t.clone(),
        };
        fixed.as_str()
    } else {
        text
    };
    font::keep_caps(|| {
        let h = 7.0 * spx(px) + 10.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
        let t = fit(text, px, rect.width() - 4.0);
        ui_text(ui.painter(), Pos2::new(rect.min.x + 2.0, rect.center().y), Align::Min, &t, px, rect.width() - 4.0, col);
    })
}

// -------------------------------------------------------------------- misc
pub(crate) fn fmt_time(s: f32) -> String {
    let s = s.max(0.0) as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Time with tenths, for loop points: 1:23.4
pub(crate) fn fmt_t(s: f32) -> String {
    // round to tenths first, so 59.96 shows as 1:00.0 and not 0:60.0
    let tenths = (s.max(0.0) * 10.0).round() as u32;
    format!("{}:{:02}.{}", tenths / 600, tenths / 10 % 60, tenths % 10)
}

pub(crate) fn fmt_long(s: f32) -> String {
    let s = s.max(0.0) as u32;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

pub(crate) fn goertzel(x: &[f32], f: f32, rate: f32) -> f32 {
    let w = 2.0 * std::f32::consts::PI * f / rate;
    let c = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    for &v in x {
        let s0 = v + c * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    let p = s1 * s1 + s2 * s2 - c * s1 * s2;
    p.max(0.0).sqrt() * 2.0 / x.len() as f32
}

/// A label at the start of a form's row. The rows of a page share one column, as wide as the longest label shown
/// (measured last frame), so the dropdowns after them line up.
pub(crate) fn form_label(ui: &mut egui::Ui, text: &str) {
    let key = egui::Id::new("form_label_col");
    let nr = ui.ctx().cumulative_pass_nr();
    let need = (100.0 * crate::font::ui_scale()).max(crate::text_w(text, 2.0) + 14.0);
    // (this frame, the widest so far this frame, the widest last frame)
    let (f, mut cur, mut prev): (u64, f32, f32) = ui.ctx().data(|d| d.get_temp(key)).unwrap_or((0, 0.0, 0.0));
    if f != nr {
        if (cur - prev).abs() > 0.5 {
            ui.ctx().request_repaint();
        }
        prev = cur;
        cur = 0.0;
    }
    cur = cur.max(need);
    ui.ctx().data_mut(|d| d.insert_temp(key, (nr, cur, prev)));
    let w = prev.max(need);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
    ptext(ui.painter(), Pos2::new(rect.min.x + 2.0, rect.center().y), Align::Min, text, 2.0, pal().ink2);
}

pub(crate) fn label(ui: &mut egui::Ui, text: &str, w: f32) {
    let w = (w * crate::font::ui_scale()).max(crate::text_w(text, 2.0) + 6.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
    ptext(ui.painter(), Pos2::new(rect.min.x + 2.0, rect.center().y), Align::Min, text, 2.0, pal().ink2);
}

pub(crate) fn dim_line(ui: &mut egui::Ui, text: &str, px: f32, col: egui::Color32) {
    if winamp_ui::chrome_on() && px <= 1.0 {
        let px = 1.5;
        let hint = col == pal().dim;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 7.0 * spx(px) + 8.0), Sense::hover());
        let x = rect.min.x + 6.0;
        let cy = rect.center().y;
        let col = if hint { pal().ink2 } else { col };
        ptext_fit(ui.painter(), Pos2::new(x, cy), Align::Min, &winamp_ui::caps_words(text), px, rect.max.x - x - 6.0, col);
        return;
    }
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 7.0 * spx(px) + 6.0), Sense::hover());
    ptext_fit(ui.painter(), Pos2::new(rect.min.x + 6.0, rect.center().y), Align::Min, text, px, rect.width() - 12.0, col);
}

pub(crate) fn setup_style(ctx: &egui::Context) {
    let mut v = egui::Visuals::light();
    v.panel_fill = pal().app_bg;
    v.window_fill = pal().panel;
    v.window_stroke = Stroke::new(1.0_f32, pal().edge);
    let rad = 0.0;
    v.window_rounding = Rounding::same(rad);
    v.menu_rounding = Rounding::same(rad);
    v.extreme_bg_color = pal().display;
    v.faint_bg_color = pal().panel_dk;
    v.hyperlink_color = pal().ink;
    v.selection.bg_fill = pal().sel;
    v.selection.stroke = Stroke::new(1.0_f32, pal().edge);
    v.widgets.noninteractive.bg_stroke = Stroke::NONE;
    v.widgets.noninteractive.bg_fill = pal().panel;
    v.widgets.noninteractive.weak_bg_fill = pal().panel;
    v.widgets.inactive.bg_fill = pal().panel_dk;
    v.widgets.inactive.weak_bg_fill = pal().panel_dk;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, pal().edge);
    v.widgets.hovered.bg_fill = pal().groove;
    v.widgets.hovered.weak_bg_fill = pal().groove;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, pal().edge);
    v.widgets.active.bg_fill = pal().ink2;
    v.widgets.active.weak_bg_fill = pal().ink2;
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, pal().edge);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.fg_stroke = Stroke::new(1.0_f32, pal().ink);
        w.rounding = Rounding::same(rad * 0.7);
    }
    if crate::studio::on() {
        // (Studio: menus and tips are little screens, a line of ink round them and a hard shadow under)
        v.window_stroke = Stroke::new(1.0_f32, pal().ink);
        let hard = egui::epaint::Shadow { offset: Vec2::new(3.0, 3.0), blur: 0.0, spread: 0.0, color: Color32::from_black_alpha(110) };
        v.popup_shadow = hard;
        v.window_shadow = hard;
        v.selection.bg_fill = pal().ink;
        v.selection.stroke = Stroke::new(1.0_f32, pal().display);
    }
    ctx.set_visuals(v);

    let mut s = (*ctx.style()).clone();
    s.spacing.item_spacing = Vec2::new(6.0, 4.0);
    s.spacing.button_padding = Vec2::new(8.0, 3.0);
    // (a Style: thin bars floating over the page, no track, so no dark strip down its edge)
    if crate::skin::style() != 0 {
        s.spacing.scroll = egui::style::ScrollStyle::floating();
        s.spacing.scroll.bar_width = 8.0;
    } else {
        s.spacing.scroll = egui::style::ScrollStyle::solid();
        s.spacing.scroll.bar_width = 12.0;
    }
    ctx.set_style(s);
}

/// A page drawn wider than its room (a row of buttons that doesn't wrap, pushing past the page's edge and widening
/// everything under it): said in the log, once a page and width, so it can be found and fixed. Rows of controls
/// should wrap (`horizontal_wrapped`, or a left-to-right layout `with_main_wrap(true)`), never run off the page.
pub(crate) fn overflow_check(ui: &egui::Ui, room: Rect, page: &str) {
    let over = ui.min_rect().max.x - room.max.x;
    if over <= 2.0 {
        return;
    }
    static SEEN: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    let key = format!("{} {:.0}", page, room.width());
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if !seen.contains(&key) {
        seen.push(key);
        crate::api::log(&format!("layout: {} is {:.0} px wider than its room ({:.0} px)", page, over, room.width()));
    }
}

/// Where a click on a seek bar would go: while the pointer is over the bar (or drags along it), the time there in a
/// small label above the pointer and a fine line at it, so you can pick a spot before you click. The same on every
/// bar in every look (`t_at`: the time at a point across the bar).
pub(crate) fn seek_label(ui: &egui::Ui, resp: &egui::Response, bar: Rect, t_at: impl Fn(f32) -> f32) {
    let Some(pp) = resp.hover_pos().or_else(|| resp.interact_pointer_pos().filter(|_| resp.dragged())) else {
        return;
    };
    let text = fmt_time(t_at(pp.x).max(0.0));
    let studio = crate::studio::on();
    let (fill, edge, ink) = if studio { (crate::studio::scr(), crate::studio::ink(), crate::studio::ink()) } else { (pal().display, pal().edge, pal().ink) };
    let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, resp.id.with("seek_label")));
    let sc = font::ui_scale();
    let (w, h) = (font::keep_caps(|| text_w(&text, 1.4)) + 14.0 * sc, 20.0 * sc);
    let screen = ui.ctx().screen_rect();
    let cx = pp.x.clamp(screen.min.x + w / 2.0 + 2.0, screen.max.x - w / 2.0 - 2.0);
    let cy = (bar.min.y - h / 2.0 - 6.0).max(screen.min.y + h / 2.0 + 2.0);
    let r = Rect::from_center_size(Pos2::new(cx, cy), Vec2::new(w, h));
    p.rect_filled(r, if studio { 0.0 } else { crate::corner() }, fill);
    p.rect_stroke(r, if studio { 0.0 } else { crate::corner() }, Stroke::new(1.0_f32, edge));
    font::keep_caps(|| ptext(&p, r.center(), Align::Center, &text, 1.4, ink));
    p.line_segment([Pos2::new(pp.x, bar.min.y - 2.0), Pos2::new(pp.x, bar.max.y + 2.0)], Stroke::new(1.0_f32, ink.gamma_multiply(0.8)));
}
