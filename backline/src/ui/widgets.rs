//! Backline's drawing kit: the raised buttons, LCD boxes, tabs, check boxes, dropdowns, window decorations,
//! seven-segment digits, the logo and the text helpers every screen is built from. Shared by the desktop app and
//! the phone app (android/), so both look like Backline.
#![allow(dead_code)]

use crate::font::{self, fit, ptext, ptext_fit, snap, spx, text_w, thick, wrap};
use crate::icons::*;
use crate::skin::*;
use crate::{tour, update, vicon, winamp_ui};
use crate::{HEAD_H, PRACTICE, ROW_H, VERSION};
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
    raised_h(ui.painter(), rect, down, resp.hovered());
    let px = 2.0 * sc;
    let (iw, ih) = (icon[0].len() as f32 * px, icon.len() as f32 * px);
    let dy = if down { 1.0 } else { 0.0 };
    let o = Pos2::new((rect.center().x - iw / 2.0).round(), (rect.center().y - ih / 2.0 + dy).round());
    pixmap(ui.painter(), o, px, icon, col);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) fn icon_btn(ui: &mut egui::Ui, icon: &[&str], on: bool, col: Color32) -> egui::Response {
    icon_btn_w(ui, icon, on, col, 44.0)
}

/// Icon on the left, a short label on the right.
pub(crate) fn ibtn(ui: &mut egui::Ui, icon: &[&str], text: &str, on: bool) -> egui::Response {
    let sc = font::ui_scale();
    let ip = 2.0 * sc;
    let w = (10.0 + 8.0 + 10.0) * sc + icon[0].len() as f32 * ip + text_w(text, 2.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::click());
    let down = resp.is_pointer_button_down_on() || on;
    raised_h(ui.painter(), rect, down, resp.hovered());
    let dy = if down { 1.0 } else { 0.0 };
    let o = Pos2::new((rect.min.x + 10.0 * sc).round(), (rect.center().y - icon.len() as f32 * ip / 2.0 + dy).round());
    pixmap(ui.painter(), o, ip, icon, pal().ink);
    let tx = o.x + icon[0].len() as f32 * ip + 8.0 * sc;
    ui_text(ui.painter(), Pos2::new(tx, rect.center().y + dy), Align::Min, text, 2.0, rect.max.x - tx - 4.0, pal().ink);
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
    match style() {
        1 => {
            rfill(p, r, 4.0, fill);
            rline(p, r, 4.0, 1.0, pal().beige_dk);
            // soft shade along the top edge, like glass set into the frame
            rfill(
                p,
                Rect::from_min_size(r.min + Vec2::new(2.0, 1.0), Vec2::new(r.width() - 4.0, 2.0)),
                1.0,
                Color32::from_black_alpha(26),
            );
            return;
        }
        2 => {
            rfill(p, r, 6.0, fill);
            rline(p, r, 6.0, 1.0, pal().groove);
            return;
        }
        _ => {}
    }
    let t1 = thick(1.5);
    let t2 = thick(1.0);
    fill_rect(p, r, fill);
    fill_rect(p, Rect::from_min_size(r.min, Vec2::new(r.width(), t1)), pal().edge);
    fill_rect(p, Rect::from_min_size(r.min, Vec2::new(t1, r.height())), pal().edge);
    fill_rect(p, Rect::from_min_size(Pos2::new(r.min.x, r.max.y - t2), Vec2::new(r.width(), t2)), pal().beige_lt);
    fill_rect(p, Rect::from_min_size(Pos2::new(r.max.x - t2, r.min.y), Vec2::new(t2, r.height())), pal().beige_lt);
}

/// Raised bevelled button face.
pub(crate) fn raised_h(p: &egui::Painter, r: Rect, down: bool, hover: bool) {
    match style() {
        1 => {
            // glossy glass button: gradient body, bright sheen on the top half, thin blue outline
            let (top, bot) = if down {
                (Color32::from_rgb(150, 196, 232), Color32::from_rgb(196, 226, 248))
            } else if hover {
                (Color32::from_rgb(255, 255, 255), Color32::from_rgb(190, 228, 250))
            } else {
                (Color32::from_rgb(250, 253, 255), Color32::from_rgb(196, 222, 244))
            };
            rgrad(p, r, 6.0, top, bot);
            if !down {
                let sheen = Rect::from_min_size(r.min + Vec2::new(2.0, 1.5), Vec2::new(r.width() - 4.0, r.height() * 0.44));
                rfill(p, sheen, 4.0, Color32::from_white_alpha(if hover { 120 } else { 90 }));
            }
            rline(p, r, 6.0, 1.0, if hover { pal().btn_face } else { pal().edge });
            return;
        }
        2 => {
            let face = if down {
                pal().sel
            } else if hover {
                pal().beige_lt
            } else {
                pal().beige_h
            };
            rfill(p, r, 6.0, face);
            rline(p, r, 6.0, 1.0, if hover || down { pal().btn_face } else { pal().groove });
            return;
        }
        _ => {}
    }
    let (hi, lo) = if down { (pal().beige_dk, pal().beige_lt) } else { (pal().beige_lt, pal().beige_dk) };
    let face = if down {
        pal().sel
    } else if hover {
        pal().beige_h
    } else {
        pal().beige
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

/// Rectangle with its four corners cut off by `n` (a pixel-art "rounded" box).
pub(crate) fn notch_fill(p: &egui::Painter, r: Rect, n: f32, c: Color32) {
    fill_rect(p, Rect::from_min_max(Pos2::new(r.min.x + n, r.min.y), Pos2::new(r.max.x - n, r.max.y)), c);
    fill_rect(p, Rect::from_min_max(Pos2::new(r.min.x, r.min.y + n), Pos2::new(r.max.x, r.max.y - n)), c);
}

/// Black window frame with beige trim, title tab and beige body at `inner`; square, with stepped corners.
pub(crate) fn window_deco(ui: &egui::Ui, inner: Rect, title: &str) {
    let p = ui.painter();
    let u = thick(2.0);
    let outer = Rect::from_min_max(inner.min - Vec2::new(14.0, 30.0), inner.max + Vec2::new(14.0, 14.0));
    tour::note(title, outer);
    // a worn Winamp skin dresses every window in its own frame
    if winamp_ui::draw_frame(p, outer, inner, title) {
        return;
    }
    if style() != 0 {
        modern_deco(p, outer, inner, title);
        return;
    }
    notch_fill(p, outer, u * 3.0, Color32::BLACK);
    notch_fill(p, outer.shrink(u * 1.5), u * 2.0, pal().trim);
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
    ptext_fit(p, tab.center() + Vec2::new(iw / 2.0, 0.0), Align::Center, &label, 2.0, tw - 12.0 - iw, pal().trim);
    if let Some(rows) = icon {
        let o = Pos2::new((tab.min.x + 9.0).round(), (cy - rows.len() as f32).round());
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                // 'o' pixels get a mid tone between tab text and tab fill so they never vanish on any skin
                let col = match ch {
                    '#' => pal().trim,
                    'o' => pal().trim.lerp_to_gamma(pal().edge, 0.4),
                    _ => continue,
                };
                fill_rect(p, Rect::from_min_size(o + Vec2::new(x as f32 * 2.0, y as f32 * 2.0), Vec2::splat(2.0)), col);
            }
        }
    }
    // body
    fill_rect(p, inner.expand(2.0), pal().edge);
    fill_rect(p, inner, pal().beige);
}

/// Window frame for the aero and sleek skins.
pub(crate) fn modern_deco(p: &egui::Painter, outer: Rect, inner: Rect, title: &str) {
    let icon: Option<&[&str]> = match title.split(' ').next().unwrap_or("") {
        "LIBRARY" => Some(&WIN_LIBRARY),
        "QUEUE" => Some(&WIN_QUEUE),
        "LEAD" => Some(&WIN_SHEET),
        "PRACTICE" => Some(&WIN_PRACTICE),
        "PLAYER" => Some(&MARK),
        _ => None,
    };
    let cy = outer.min.y + 14.0;
    let (txt_col, icon_col) = if style() == 1 { (pal().bar_txt, pal().bar_txt) } else { (pal().ink2, pal().ink2) };
    if style() == 1 {
        // glass title bar over a blue frame, like an XP / Vista window
        rfill(p, outer.translate(Vec2::new(0.0, 2.0)), 9.0, Color32::from_black_alpha(60));
        rgrad(p, outer, 9.0, Color32::from_rgb(92, 178, 236), Color32::from_rgb(28, 108, 188));
        let bar = Rect::from_min_size(outer.min + Vec2::new(2.0, 2.0), Vec2::new(outer.width() - 4.0, 24.0));
        rfill(p, bar, 7.0, Color32::from_white_alpha(46));
        rline(p, outer, 9.0, 1.0, pal().edge);
        rfill(p, inner.expand(2.0), 3.0, pal().edge);
        rfill(p, inner, 2.0, pal().beige);
        // a sliver of glare across the top of the body
        rfill(
            p,
            Rect::from_min_size(inner.min + Vec2::new(1.0, 1.0), Vec2::new(inner.width() - 2.0, 3.0)),
            1.0,
            Color32::from_white_alpha(70),
        );
    } else {
        rfill(p, outer, 10.0, pal().beige_dk);
        rline(p, outer, 10.0, 1.0, pal().groove);
        rfill(p, inner, 6.0, pal().beige);
        rline(p, inner, 6.0, 1.0, pal().groove);
    }
    let mut x = outer.min.x + 14.0;
    if let Some(rows) = icon {
        let k = 2.0;
        let ir = Rect::from_center_size(Pos2::new(x + 10.0, cy), Vec2::splat(rows.len() as f32 * k));
        if !vicon::draw(p, ir, rows, icon_col) {
            pixmap(p, ir.min, k, rows, icon_col);
        }
        x += 26.0;
    }
    ptext(p, Pos2::new(x, cy), Align::Min, title, 2.0, txt_col);
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
        fill_rect(p, r, pal().lcd_ghost);
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
    if style() != 0 {
        vicon::gem(p, Rect::from_min_size(origin, Vec2::splat(px * 9.0)), pal().ink, pal().ink2);
        return;
    }
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
    inset(p, rect, pal().lcd);
    logo_mark(p, Pos2::new(rect.min.x + 12.0, rect.center().y - 13.5), 3.0);
    let (tsz, ty, gy) = if style() == 0 { (3.0, -8.0, 13.0) } else { (2.6, -9.0, 14.0) };
    ptext(p, Pos2::new(rect.min.x + 50.0, rect.center().y + ty), Align::Min, "BACKLINE", tsz, pal().ink);
    let tag =
        if PRACTICE { "A RETRO PLAYER FOR TIDAL AND MORE - MADE FOR PRACTICE" } else { "A RETRO PLAYER FOR TIDAL AND MORE" };
    let version = if update::build() > 0 { format!("{} .{}", VERSION, update::build()) } else { VERSION.to_string() };
    // the tagline stops short of the version number, whatever the font's width
    let vw = text_w(&version, 1.0);
    let tag_w = rect.width() - 51.0 - vw - 24.0;
    ptext_fit(p, Pos2::new(rect.min.x + 51.0, rect.center().y + gy), Align::Min, tag, 1.0, tag_w, pal().ink2);
    ptext(p, Pos2::new(rect.max.x - 10.0, rect.center().y + gy), Align::Max, &version, 1.0, pal().dim);
}

/// Folder-style tabs with a baseline; the open tab joins the panel below. Returns the clicked tab.
pub(crate) fn tab_row(ui: &mut egui::Ui, items: &[&str], cur: usize) -> Option<usize> {
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
        let p = ui.painter();
        if style() != 0 {
            let r = Rect::from_min_max(r.min + Vec2::new(0.0, if on { 0.0 } else { 1.0 }), Pos2::new(r.max.x, r.max.y - 2.0));
            if style() == 1 {
                if on || resp.hovered() {
                    raised_h(p, r, false, on);
                } else {
                    rfill(p, r, 6.0, pal().beige_dk);
                }
            } else if on {
                rfill(p, r, 7.0, pal().btn_face);
            } else if resp.hovered() {
                rfill(p, r, 7.0, pal().beige_h);
            }
            let col = if on && style() == 2 {
                pal().bar_txt
            } else if on {
                pal().ink
            } else {
                pal().ink2
            };
            ptext_fit(p, r.center(), Align::Center, name, 2.0, w - 8.0, col);
            if resp.clicked() {
                hit = Some(i);
            }
            x += w + 3.0;
            continue;
        }
        // in the skin's colours (a piece of its title bar would bring its stripes through the words)
        {
            fill_rect(
                p,
                r,
                if on {
                    pal().beige_lt
                } else if resp.hovered() {
                    pal().beige_h
                } else {
                    pal().beige_dk
                },
            );
            fill_rect(p, Rect::from_min_size(r.min, Vec2::new(r.width(), t)), pal().edge);
            fill_rect(p, Rect::from_min_size(r.min, Vec2::new(t, r.height())), pal().edge);
            fill_rect(p, Rect::from_min_size(Pos2::new(r.max.x - t, r.min.y), Vec2::new(t, r.height())), pal().edge);
            // chipped top corners
            fill_rect(p, Rect::from_min_size(r.min, Vec2::splat(t)), pal().beige);
            fill_rect(p, Rect::from_min_size(Pos2::new(r.max.x - t, r.min.y), Vec2::splat(t)), pal().beige);
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
}

/// Small pixel check box with a label. Returns the click.
pub(crate) fn check_box(ui: &mut egui::Ui, text: &str, on: bool) -> egui::Response {
    let w = text_w(text, 2.0) + 30.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::click());
    let b = Rect::from_center_size(Pos2::new(rect.min.x + 11.0, rect.center().y), Vec2::splat(16.0));
    inset(ui.painter(), b, pal().lcd);
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
    raised_h(ui.painter(), rect, down, resp.hovered());
    let dy = if down { 1.0 } else { 0.0 };
    ui_text(ui.painter(), rect.center() + Vec2::new(0.0, dy), Align::Center, text, 2.0, rect.width() - 8.0, pal().ink);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Button sized to its label.
pub(crate) fn retro_btn(ui: &mut egui::Ui, text: &str, active: bool) -> egui::Response {
    let w = (text_w(text, 2.0) + 20.0).max(44.0);
    retro_btn_w(ui, text, w, active)
}

pub(crate) fn section_header(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 20.0), Sense::hover());
    fill_rect(ui.painter(), rect, pal().edge);
    ui_text(ui.painter(), Pos2::new(rect.min.x + 8.0, rect.center().y), Align::Min, text, 2.0, rect.width() - 16.0, pal().trim);
}

/// Thin sub-heading bar used inside the queue.
pub(crate) fn mini_header(ui: &mut egui::Ui, text: &str) {
    let w = ui.available_width();
    let probe = Rect::from_min_size(ui.cursor().min, Vec2::new(w, HEAD_H));
    if !ui.is_rect_visible(probe) {
        ui.allocate_space(Vec2::new(w, HEAD_H));
        return;
    }
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, HEAD_H), Sense::hover());
    fill_rect(ui.painter(), rect, pal().groove);
    ptext(ui.painter(), Pos2::new(rect.min.x + 8.0, rect.center().y), Align::Min, text, 1.0, pal().bar_txt);
}

/// One piece of text in a list row: the skin's playlist font when a Winamp skin is on, Backline's pixel font
/// otherwise; cut to fit `max_w`. Returns how wide it came out.
pub(crate) fn row_text(p: &egui::Painter, pos: Pos2, align: Align, text: &str, max_w: f32, col: Color32) -> f32 {
    if winamp_ui::chrome_on() {
        winamp_ui::sans_text(p, pos, align, text, ROW_H * 0.7, max_w, col)
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
    ptext(ui.painter(), Pos2::new(rect.min.x + 10.0, rect.center().y), Align::Min, text, 2.0, pal().ink);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// Dropdown button: shows `name: current` with a small arrow; the list opens as an overlay
/// right under it (no layout change). Returns the index chosen this frame.
pub(crate) fn dropdown(ui: &mut egui::Ui, id: &str, title: &str, items: &[&str], sel: usize, min_w: f32) -> Option<usize> {
    dropdown_at(ui, id, title, items, sel, min_w, false)
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
    let cur = items.get(sel).copied().unwrap_or("");
    let text = if title.is_empty() { cur.to_string() } else { format!("{}: {}", title, cur) };
    let w = if fixed { min_w } else { (text_w(&text, 2.0) + 38.0).max(min_w) };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::click());
    let key = egui::Id::new(("dd", id));
    let open = ui.ctx().data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    let down = resp.is_pointer_button_down_on() || open;
    raised_h(ui.painter(), rect, down, resp.hovered());
    let dy = if down { 1.0 } else { 0.0 };
    ptext_fit(ui.painter(), Pos2::new(rect.min.x + 10.0, rect.center().y + dy), Align::Min, &text, 2.0, w - 38.0, pal().ink);
    pixmap(
        ui.painter(),
        Pos2::new(rect.max.x - 20.0, rect.center().y - 3.0 + dy),
        2.0,
        &["#######", ".#####.", "..###..", "...#..."],
        pal().ink,
    );
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
        let full = items.len() as f32 * 24.0 + 4.0;
        let below = screen.max.y - rect.max.y - 10.0;
        let above = rect.min.y - screen.min.y - 10.0;
        let up = full > below && above > below;
        let room = if up { above } else { below }.max(120.0);
        let h = full.min(room);
        let at = if up { Pos2::new(rect.min.x, rect.min.y - 1.0 - h) } else { Pos2::new(rect.min.x, rect.max.y + 1.0) };
        let iw = items.iter().map(|it| text_w(it, 2.0) + 28.0).fold(w - 4.0, f32::max) + if full > room { 12.0 } else { 0.0 };
        let area = egui::Area::new(key.with("area")).order(egui::Order::Foreground).fixed_pos(at).show(ui.ctx(), |ui| {
            egui::Frame::none().fill(pal().beige_lt).stroke(egui::Stroke::new(2.0_f32, pal().edge)).show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                egui::ScrollArea::vertical().max_height(h - 4.0).auto_shrink([true, true]).show(ui, |ui| {
                    for (i, it) in items.iter().enumerate() {
                        let (r, rr) = ui.allocate_exact_size(Vec2::new(iw, 24.0), Sense::click());
                        if i == sel || rr.hovered() {
                            fill_rect(ui.painter(), r, pal().sel);
                        }
                        ptext(ui.painter(), Pos2::new(r.min.x + 10.0, r.center().y), Align::Min, it, 2.0, pal().ink);
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
        fill_rect(&p, r, pal().lcd);
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
    raised_h(ui.painter(), rect, active || r.is_pointer_button_down_on(), r.hovered() && !active);
    ui_text(ui.painter(), rect.center(), Align::Center, text, 1.0, rect.width() - 6.0, pal().ink);
    r
}

/// Sunken LCD readout box.
pub(crate) fn lcd_box(ui: &mut egui::Ui, text: &str, w: f32, col: Color32) {
    // never narrower than its text (a wide font or a bigger number would be cut off)
    let w = (w * font::ui_scale()).max(text_w(text, 2.0) + 16.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
    inset(ui.painter(), rect, pal().lcd);
    ptext_fit(ui.painter(), rect.center(), Align::Center, text, 2.0, w - 12.0, col);
}

/// Wrapped paragraph in the list well.
pub(crate) fn para(ui: &mut egui::Ui, text: &str, col: Color32) {
    let w = ui.available_width() - 20.0;
    for line in wrap(text, 2.0, w) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
        ptext(ui.painter(), Pos2::new(rect.min.x + 8.0, rect.center().y), Align::Min, &line, 2.0, col);
    }
    ui.add_space(6.0);
}

/// Break a long unbroken string (a file path) into lines that fit.
pub(crate) fn chunk(text: &str, px: f32, w: f32) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        let mut t = cur.clone();
        t.push(ch);
        if text_w(&t, px) > w && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        cur.push(ch);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Single heading line, shortened with "..." if it doesn't fit.
pub(crate) fn title_line(ui: &mut egui::Ui, text: &str, px: f32, col: Color32) {
    let h = 7.0 * spx(px) + 10.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let t = fit(text, px, rect.width() - 4.0);
    ui_text(ui.painter(), Pos2::new(rect.min.x + 2.0, rect.center().y), Align::Min, &t, px, rect.width() - 4.0, col);
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

pub(crate) fn label(ui: &mut egui::Ui, text: &str, w: f32) {
    let w = (w * crate::font::ui_scale()).max(crate::text_w(text, 2.0) + 6.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
    ptext(ui.painter(), Pos2::new(rect.min.x + 2.0, rect.center().y), Align::Min, text, 2.0, pal().ink2);
}

pub(crate) fn dim_line(ui: &mut egui::Ui, text: &str, px: f32, col: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 7.0 * spx(px) + 6.0), Sense::hover());
    ptext_fit(ui.painter(), Pos2::new(rect.min.x + 6.0, rect.center().y), Align::Min, text, px, rect.width() - 12.0, col);
}

pub(crate) fn setup_style(ctx: &egui::Context) {
    let mut v = egui::Visuals::light();
    v.panel_fill = pal().app_bg;
    v.window_fill = pal().beige;
    v.window_stroke = Stroke::new(1.0_f32, pal().edge);
    let rad = if style() != 0 { 7.0 } else { 0.0 };
    v.window_rounding = Rounding::same(rad);
    v.menu_rounding = Rounding::same(rad);
    v.extreme_bg_color = pal().lcd;
    v.faint_bg_color = pal().beige_dk;
    v.hyperlink_color = pal().ink;
    v.selection.bg_fill = pal().sel;
    v.selection.stroke = Stroke::new(1.0_f32, pal().edge);
    v.widgets.noninteractive.bg_stroke = Stroke::NONE;
    v.widgets.noninteractive.bg_fill = pal().beige;
    v.widgets.noninteractive.weak_bg_fill = pal().beige;
    v.widgets.inactive.bg_fill = pal().beige_dk;
    v.widgets.inactive.weak_bg_fill = pal().beige_dk;
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
    ctx.set_visuals(v);

    let mut s = (*ctx.style()).clone();
    s.spacing.item_spacing = Vec2::new(6.0, 4.0);
    s.spacing.button_padding = Vec2::new(8.0, 3.0);
    s.spacing.scroll = egui::style::ScrollStyle::solid();
    s.spacing.scroll.bar_width = 12.0;
    ctx.set_style(s);
}
