//! The Styles on the Skins page (the owner's picks of 2026-11-24, after their mocks): Streaming, Ambient Color and
//! Hardware lay the window out themselves, with no Winamp deck: their own player (Streaming's bar along the whole
//! foot; Ambient's cover and controls at the top right; Hardware's NOW PLAYING unit with its amber screen and keys), the
//! queue as their own list, and their own surfaces (Streaming's panels on black, Ambient's see-through panels over the
//! album's colours, Hardware's matte body). The pages inside are the same as everywhere.

use super::*;
use crate::skin::{style, STYLE_AMBIENT, STYLE_HARDWARE, STYLE_STREAMING, STYLE_STUDIO};

/// Hardware's body, its panels, its screens and their amber.
pub(crate) const HW_BODY: Color32 = Color32::from_rgb(20, 21, 22);
const HW_PANEL: Color32 = Color32::from_rgb(28, 29, 31);
/// Hardware's screen glass and its writing (the colour picked on the Skins page).
pub(crate) fn lcd_bg() -> Color32 {
    crate::skin::hw_screen().1
}
pub(crate) fn amber() -> Color32 {
    crate::skin::hw_screen().0
}

/// The gap round and between a style's panels.
const GAP: f32 = 8.0;

/// White at strength `a`, laid over what's under it (egui's own white-with-alpha comes out far brighter than its
/// number reads).
fn white(a: u8) -> Color32 {
    Color32::from_rgba_premultiplied(a, a, a, a)
}

/// The style's writing and its quieter writing: white on the dark styles, the palette's own on Ambient's Light feel.
fn fg() -> Color32 {
    if style() == STYLE_STUDIO {
        return crate::studio::ink();
    }
    if style() == STYLE_AMBIENT && crate::skin::amb_light() {
        pal().ink
    } else {
        Color32::WHITE
    }
}
fn fg2() -> Color32 {
    if style() == STYLE_STUDIO {
        return crate::studio::dim();
    }
    if style() == STYLE_AMBIENT && crate::skin::amb_light() {
        pal().ink2
    } else {
        Color32::from_gray(176)
    }
}
/// What's drawn on `fg()` (the play button's symbol).
fn on_fg() -> Color32 {
    if style() == STYLE_STUDIO {
        return crate::studio::scr();
    }
    if style() == STYLE_AMBIENT && crate::skin::amb_light() {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}
/// A light wash over the glass (a row lit, a track's empty part): dark on the Light feel.
fn wash(a: u8) -> Color32 {
    if style() == STYLE_STUDIO {
        return crate::studio::ink().gamma_multiply(a as f32 / 510.0);
    }
    if style() == STYLE_AMBIENT && crate::skin::amb_light() {
        Color32::from_black_alpha(a)
    } else {
        white(a)
    }
}

/// Whether a Style lays the window out (Streaming, Ambient Color, Hardware).
pub(crate) fn styled() -> bool {
    matches!(style(), STYLE_AMBIENT | STYLE_STREAMING | STYLE_HARDWARE | STYLE_STUDIO)
}

/// A surface of the style worn with its name: Studio writes it in brackets on the screen's frame ("─[ PLAYLIST ]─");
/// the other styles' panels have no name on them.
pub(crate) fn panel_named(p: &egui::Painter, r: Rect, name: &str) {
    if style() == STYLE_STUDIO {
        crate::studio::frame(p, r.shrink(3.0), name);
    } else {
        panel(p, r);
    }
}

/// A surface of the style worn (the side list, the page, the player, the queue): Streaming's dark rounded panel on
/// black, Ambient's see-through one, Hardware's matte panel with a fine edge.
pub(crate) fn panel(p: &egui::Painter, r: Rect) {
    match style() {
        STYLE_STUDIO => crate::studio::frame(p, r.shrink(3.0), ""),
        STYLE_STREAMING => {
            p.rect_filled(r, 8.0, Color32::from_rgb(18, 18, 18));
        }
        STYLE_AMBIENT => {
            // (the feel: Dark the deepest glass, Deep lighter, Light a pale glass; deep enough that the page reads
            // over the album's colours)
            let glass = match crate::skin::AMB_FEEL.load(std::sync::atomic::Ordering::Relaxed) {
                1 => Color32::from_black_alpha(105),
                2 => Color32::from_rgba_premultiplied(150, 150, 150, 150),
                _ => Color32::from_black_alpha(140),
            };
            p.rect_filled(r, 10.0, glass);
        }
        STYLE_HARDWARE => {
            p.rect_filled(r, 6.0, HW_PANEL);
            p.rect_stroke(r, 6.0, Stroke::new(1.0_f32, Color32::from_black_alpha(200)));
            p.line_segment([r.left_top() + Vec2::new(6.0, 1.0), r.right_top() + Vec2::new(-6.0, 1.0)], Stroke::new(1.0_f32, white(10)));
            // (four small screws, as a front panel has)
            for c in [r.left_top() + Vec2::new(6.0, 6.0), r.right_top() + Vec2::new(-6.0, 6.0), r.left_bottom() + Vec2::new(6.0, -6.0), r.right_bottom() + Vec2::new(-6.0, -6.0)] {
                p.circle_filled(c, 1.6, Color32::from_gray(52));
            }
        }
        _ => {}
    }
}

/// A colour fading down to nothing over `r`, its top corners rounded by `rad` as the panel under it is (so no
/// square corner of colour stands out past the panel's).
pub(crate) fn wash_rounded(p: &egui::Painter, r: Rect, top: Color32, rad: f32) {
    let step = 2.0;
    let mut y = r.min.y;
    while y < r.max.y {
        let y1 = (y + step).min(r.max.y);
        let t = ((y - r.min.y) / r.height()).clamp(0.0, 1.0);
        let dy = y - r.min.y + step * 0.5;
        let inset = if dy < rad { rad - (rad * rad - (rad - dy) * (rad - dy)).max(0.0).sqrt() } else { 0.0 };
        p.rect_filled(Rect::from_min_max(Pos2::new(r.min.x + inset, y), Pos2::new(r.max.x - inset, y1)), 0.0, top.gamma_multiply(1.0 - t));
        y = y1;
    }
}

/// An amber screen (Hardware): dark, sunk in, its edge catching the light.
pub(crate) fn lcd(p: &egui::Painter, r: Rect) {
    if style() == STYLE_STUDIO {
        // (Studio: a screen within the screen, a shade darker, a fine line of ink round it)
        p.rect_filled(r, 0.0, crate::studio::key_col());
        p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, crate::studio::ink()));
        return;
    }
    p.rect_filled(r, 4.0, lcd_bg());
    p.rect_stroke(r, 4.0, Stroke::new(1.0_f32, Color32::from_black_alpha(200)));
    p.line_segment([r.left_bottom() + Vec2::new(4.0, 1.0), r.right_bottom() + Vec2::new(-4.0, 1.0)], Stroke::new(1.0_f32, white(12)));
}

/// Hardware's screens write in a face of their own (the Skins page's SCREEN TYPE): Sixtyfour's scanlines or VT323's
/// terminal. Studio keeps its own type.
fn screen_face() -> Option<(egui::FontFamily, f32)> {
    if style() != STYLE_HARDWARE {
        return None;
    }
    // (each face's size for the app's text size: Sixtyfour is wide and short, VT323 tall and narrow)
    Some(match crate::skin::HW_TYPE.load(std::sync::atomic::Ordering::Relaxed) {
        1 => (egui::FontFamily::Name("hw_term".into()), 1.45),
        _ => (egui::FontFamily::Name("hw_scan".into()), 0.84),
    })
}

/// How wide `text` is on a Hardware screen at the app's text size `px`.
pub(crate) fn screen_w(p: &egui::Painter, text: &str, px: f32) -> f32 {
    match screen_face() {
        Some((fam, k)) => p.layout_no_wrap(text.to_string(), egui::FontId::new(font::text_px(px) * k, fam), Color32::WHITE).size().x,
        None => font::keep_caps(|| text_w(text, px)),
    }
}

/// Words on a Hardware screen as written (a song's name, its artist): the screen's own face, lit with a faint glow,
/// cut with "..." when too long for `max_w`. `at` is the left / middle / right edge at the words' middle.
#[allow(clippy::too_many_arguments)]
pub(crate) fn screen_text(p: &egui::Painter, at: Pos2, align: Align, text: &str, px: f32, max_w: f32, col: Color32) {
    let Some((fam, k)) = screen_face() else {
        // (Studio shares these screens: its own writing, as before)
        let _ = col;
        lcd_text(p, at, align, text, px, max_w);
        return;
    };
    let fid = egui::FontId::new(font::text_px(px) * k, fam);
    let w = |s: &str| p.layout_no_wrap(s.to_string(), fid.clone(), Color32::WHITE).size().x;
    let mut shown = text.to_string();
    if w(&shown) > max_w {
        let mut c: Vec<char> = shown.chars().collect();
        while !c.is_empty() && w(&(c.iter().collect::<String>().trim_end().to_string() + "...")) > max_w {
            c.pop();
        }
        shown = c.iter().collect::<String>().trim_end().to_string() + "...";
    }
    let anchor = match align {
        Align::Min => egui::Align2::LEFT_CENTER,
        Align::Center => egui::Align2::CENTER_CENTER,
        Align::Max => egui::Align2::RIGHT_CENTER,
    };
    // (lit: a faint halo round the writing, then the words)
    let halo = col.gamma_multiply(0.08);
    for d in [Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0), Vec2::new(0.0, 1.0)] {
        p.text(at + d * 1.3, anchor, &shown, fid.clone(), halo);
    }
    p.text(at, anchor, &shown, fid, col);
}

/// Scanline screens (Hardware's SCREEN TYPE 0), its big digits: `draw` lit in bands two pixels tall with a pixel's gap between, as an
/// old tube's rows, whatever is behind (the glass, a row's highlight) showing in the gaps; the other type draws it
/// whole.
fn scanned(p: &egui::Painter, r: Rect, draw: impl Fn(&egui::Painter)) {
    // (only big writing: the bands would cut small letters' strokes away)
    if crate::skin::HW_TYPE.load(std::sync::atomic::Ordering::Relaxed) != 0 || r.height() < 22.0 {
        draw(p);
        return;
    }
    let clip = p.clip_rect();
    let mut y = r.min.y.floor();
    while y < r.max.y {
        let band = Rect::from_min_max(Pos2::new(r.min.x - 2.0, y), Pos2::new(r.max.x + 2.0, y + 2.0)).intersect(clip);
        if band.is_positive() {
            draw(&p.with_clip_rect(band));
        }
        y += 3.0;
    }
}

/// Hardware's time in 7-segment digits `h` tall from `at` (left middle), lit with a glow, the unlit segments faint
/// behind them (a deck's display). Returns how wide.
pub(crate) fn hw_digits(p: &egui::Painter, at: Pos2, h: f32, text: &str, col: Color32) -> f32 {
    let halo = col.gamma_multiply(0.07);
    for d in [Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0), Vec2::new(0.0, 1.0)] {
        crate::studio::digits(p, at + d * (h * 0.05).max(1.0), h, text, halo, Color32::TRANSPARENT);
    }
    let w = digits_w(h, text);
    let r = Rect::from_min_max(Pos2::new(at.x - 2.0, at.y - h * 0.5 - 1.0), Pos2::new(at.x + w + h * 0.2, at.y + h * 0.5 + 1.0));
    scanned(p, r, |pp| {
        crate::studio::digits(pp, at, h, text, col, col.gamma_multiply(0.07));
    });
    w
}

/// How wide `text` is in LCD digits `h` tall (`studio::digits`).
pub(crate) fn digits_w(h: f32, text: &str) -> f32 {
    let k = h / 25.0;
    text.chars()
        .map(|c| match c {
            ':' => 6.0 * k,
            '.' => 4.5 * k,
            _ => 15.5 * k,
        })
        .sum()
}

/// Hardware's big time on its amber screen with the song's length after it, half the size, in the same light (every
/// look shows how long the song is). Returns how wide, the length included.
pub(crate) fn hw_time(p: &egui::Painter, at: Pos2, shown: &str, dur: f32, big: f32, max_w: f32) -> f32 {
    if style() == STYLE_HARDWARE {
        // (7-segment digits as tall as the writing's capitals were)
        return hw_time_h(p, at, shown, dur, font::text_px(big) * 0.74, max_w);
    }
    let tw = font::keep_caps(|| text_w(shown, big));
    lcd_text(p, at, Align::Min, shown, big, max_w);
    if dur <= 0.0 {
        return tw;
    }
    let small = big * 0.5;
    let len = format!("/ {}", fmt_time(dur));
    let lw = font::keep_caps(|| text_w(&len, small));
    if tw + 8.0 + lw > max_w {
        return tw;
    }
    // (on the big digits' foot)
    let dy = (font::text_px(big) - font::text_px(small)) * 0.36;
    lcd_text(p, Pos2::new(at.x + tw + 8.0, at.y + dy), Align::Min, &len, small, lw + 4.0);
    tw + 8.0 + lw
}

/// Hardware's time in 7-segment digits `h` tall from `at` (left middle), the song's length half the size after a
/// slash, on their foot (left out when there's no room in `max_w`). Returns how wide.
pub(crate) fn hw_time_h(p: &egui::Painter, at: Pos2, shown: &str, dur: f32, h: f32, max_w: f32) -> f32 {
    {
        let col = amber();
        let tw = hw_digits(p, at, h, shown, col);
        if dur <= 0.0 {
            return tw;
        }
        let sh = h * 0.5;
        let len = fmt_time(dur);
        let lw = digits_w(sh, &len) + sh * 0.6;
        if tw + h * 0.15 + lw > max_w {
            return tw;
        }
        let foot = at.y + h * 0.44;
        let x0 = at.x + tw + h * 0.12;
        p.line_segment([Pos2::new(x0, foot), Pos2::new(x0 + sh * 0.34, foot - sh * 0.88)], Stroke::new((h * 0.05).max(1.3), col));
        let w2 = hw_digits(p, Pos2::new(x0 + sh * 0.55, foot - sh * 0.44), sh, &len, col);
        tw + h * 0.12 + sh * 0.55 + w2
    }
}

pub(crate) fn lcd_text(p: &egui::Painter, at: Pos2, align: Align, text: &str, px: f32, max_w: f32) {
    if screen_face().is_some() {
        // (Hardware: in the screen's face, in capitals)
        screen_text(p, at, align, &text.to_uppercase(), px, max_w, amber());
        return;
    }
    font::keep_caps(|| {
        // (too long for its room: cut short with ..., never run into what's beside it)
        let mut up = text.to_uppercase();
        if text_w(&up, px) > max_w {
            let mut c: Vec<char> = up.chars().collect();
            while !c.is_empty() && text_w(&(c.iter().collect::<String>() + "..."), px) > max_w {
                c.pop();
            }
            up = c.iter().collect::<String>().trim_end().to_string() + "...";
        }
        if style() == STYLE_STUDIO {
            // (Studio: plain ink, no glow)
            ptext_fit(p, at, align, &up, px, max_w, amber());
            return;
        }
        // (a mild glow round the writing, as a lit screen's: a faint halo, then the words)
        let halo = amber().gamma_multiply(0.09);
        for d in [Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0), Vec2::new(0.0, 1.0), Vec2::new(-0.7, -0.7), Vec2::new(0.7, 0.7), Vec2::new(-0.7, 0.7), Vec2::new(0.7, -0.7)] {
            ptext_fit(p, at + d * 1.4, align, &up, px, max_w, halo);
        }
        ptext_fit(p, at, align, &up, px, max_w, amber().gamma_multiply(0.3));
        ptext_fit(p, at + Vec2::new(0.4, 0.0), align, &up, px, max_w, amber());
    });
}

/// A label printed on the panel (Hardware): small spaced capitals.
fn printed(p: &egui::Painter, at: Pos2, text: &str) {
    if style() == STYLE_STUDIO {
        crate::studio::px(p, at, egui::Align2::LEFT_CENTER, &text.to_uppercase(), 8.5, crate::studio::dim());
        return;
    }
    font::keep_caps(|| ptext(p, at, Align::Min, &text.to_uppercase(), 1.0, Color32::from_gray(128)));
}

impl App {
    /// The window as a Style lays it out: its backdrop; the player along the whole foot (the owner: every style has
    /// one); a column at the right (Ambient's cover, then the equalizer when it's open, the practice panel when
    /// practising, the playlist; Hardware the same without the cover; Streaming's only for the equalizer or the
    /// practice panel, its queue sliding in over the page); the library the rest.
    pub(crate) fn styled_layout(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        let st = style();
        let screen = ctx.screen_rect();
        self.style_backdrop(ctx, screen);
        let none = egui::Frame::none();
        // (the room the windows keep round their pages, as the other looks' frames do: the bars and the gaps)
        // (one even gap round and between every panel)
        let margins = egui::Margin { left: GAP, right: GAP, top: crate::FRAME_TOP, bottom: GAP };
        let bar_h = if crate::hardware() { 96.0 } else { 88.0 };
        egui::TopBottomPanel::bottom("st_player").exact_height(bar_h).resizable(false).show_separator_line(false).frame(none).show(ctx, |ui| {
            let r = ui.max_rect();
            if st == STYLE_STREAMING {
                self.player_bar(ui, r, acts);
            } else {
                let pr = Rect::from_min_max(r.min + Vec2::new(GAP, 0.0), r.max - Vec2::splat(GAP));
                panel_named(ui.painter(), pr, "Player");
                self.player_bar(ui, pr, acts);
            }
        });
        // (every style has its column now: the deck at its top, the owner's ask for Streaming too)
        let col_w = if self.practice {
            (screen.width() * 0.36).within(crate::layout::PRACTICE_COL_MIN, 760.0)
        } else {
            (screen.width() * 0.27).clamp(360.0, 440.0)
        };
        // (a narrow window, half a screen: the column gives way so the page keeps room to be read; the places down
        // the side fold to their strip of icons first, in library_ui)
        let col_w = col_w.min(screen.width() - 520.0 * font::ui_scale()).max(320.0);
        // (or as wide as you dragged it, as Classic's stack: the line between the page and the column; a double-click
        // gives it its own width back)
        let set = if self.practice { self.style_prac_col_w } else { self.style_col_w };
        // (the library keeps at least 440 points: its pages read; a window too narrow for both, the column gives way)
        let most = (screen.width() - 440.0 * font::ui_scale()).max(300.0);
        let col_w = if set > 0.0 { set.within(300.0, most) } else { col_w };
        let edge = Rect::from_min_size(Pos2::new(screen.max.x - col_w - 7.0, screen.min.y + crate::FRAME_TOP), Vec2::new(12.0, screen.height() - crate::FRAME_TOP - bar_h));
        match crate::chrome::splitter(ctx, "split_style_col", edge, true) {
            Some(Some(x)) => {
                let w = (screen.max.x - x).within(300.0, most);
                if self.practice {
                    self.style_prac_col_w = w;
                } else {
                    self.style_col_w = w;
                }
                self.dirty = true;
            }
            Some(None) => {
                if self.practice {
                    self.style_prac_col_w = 0.0;
                } else {
                    self.style_col_w = 0.0;
                }
                self.dirty = true;
            }
            None => {}
        }
        {
            egui::SidePanel::right("st_col").exact_width(col_w).resizable(false).show_separator_line(false).frame(none).show(ctx, |ui| {
                let r = ui.max_rect();
                let pad = GAP;
                let x0 = r.min.x;
                let x1 = r.max.x - pad;
                let mut y = r.min.y + crate::FRAME_TOP;
                // (its top strip moves the window, as the others' bars do)
                crate::app_bar_strip(ui, Rect::from_min_max(Pos2::new(x0, r.min.y), Pos2::new(x1, y)), ui.id().with("st_bar_strip"));
                // the deck at the top right, the classic player's controls made easy (the owner); practising keeps it,
                // the practice panel goes under it
                {
                    // (as tall as you drag the line under it, everything on it scaled to match: bigger text, bigger
                    // keys; a double-click gives it its own height back)
                    const DECK_H: f32 = 266.0;
                    let most = (r.max.y - y - 180.0).max(DECK_H);
                    let deck_h = if self.style_deck_h > 0.0 { self.style_deck_h.within(220.0, most) } else { DECK_H };
                    let dr = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, y + deck_h));
                    panel_named(ui.painter(), dr, "Now Playing");
                    // (the scale: the height's, held back by the width, at least what it was)
                    let by_h = deck_h / DECK_H;
                    let by_w = (dr.width() - 32.0) / 340.0;
                    let k = by_h.min(by_w.max(1.0)).clamp(0.85, 2.2);
                    font::with_scale(font::ui_scale() * k, || {
                        if st == STYLE_STUDIO {
                            self.studio_deck(ui, dr.shrink(16.0), acts);
                        } else {
                            self.style_deck(ui, dr.shrink(14.0), acts);
                        }
                    });
                    let edge = Rect::from_center_size(Pos2::new(dr.center().x, dr.max.y + pad / 2.0), Vec2::new(dr.width(), 10.0));
                    match crate::chrome::splitter(ui.ctx(), "split_style_deck", edge, false) {
                        Some(Some(at)) => {
                            self.style_deck_h = (at - y).within(220.0, most);
                            self.dirty = true;
                        }
                        Some(None) => {
                            self.style_deck_h = 0.0;
                            self.dirty = true;
                        }
                        None => {}
                    }
                    y = dr.max.y + pad;
                }
                // the equalizer
                if self.show_eq {
                    let er = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, (y + 340.0).min(r.max.y - pad)));
                    self.style_eq(ui, er);
                    y = er.max.y + pad;
                }
                // the practice panel along the column's foot, as the classic stack has it (the owner); the playlist
                // (or Streaming's Next Up) between it and the deck
                let mut foot = r.max.y - pad;
                if self.practice {
                    let room = foot - y;
                    let list_on = if st == STYLE_STREAMING { true } else { !self.queue_hidden };
                    // (the panel most of the room, the playlist keeping a few songs above it)
                    let h = if list_on { (room * 0.68).max(430.0).min(room) } else { room };
                    let outer = Rect::from_min_max(Pos2::new(x0, foot - h), Pos2::new(x1, foot));
                    panel_named(ui.painter(), outer, "Practice");
                    // (the panel's own page inside the room a frame keeps, so its title strip and controls sit as ever)
                    let inner = Rect::from_min_max(outer.min + Vec2::new(crate::FRAME_SIDE, crate::FRAME_TOP), outer.max - Vec2::splat(crate::FRAME_SIDE));
                    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                    self.practice_ui(&mut c, acts);
                    foot = outer.min.y - pad;
                }
                // Streaming: what's next, a few songs, the whole queue sliding in from there
                if st == STYLE_STREAMING && foot - y > 100.0 {
                    // (a small card: the next five songs at most, sized to them; the whole queue slides in from it)
                    let left = self.queue.len().saturating_sub(self.cur.map_or(0, |c| c + 1)).clamp(1, 5) as f32;
                    let want = 14.0 + 30.0 + left * 54.0 + 14.0;
                    let nr = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, if self.practice { foot } else { (y + want).min(foot) }));
                    panel(ui.painter(), nr);
                    self.next_up(ui, nr.shrink(14.0), acts);
                }
                // the playlist (the deck's PL switch hides it)
                if st != STYLE_STREAMING && !self.queue_hidden && foot - y > 80.0 {
                    let qr = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, foot));
                    panel_named(ui.painter(), qr, "Playlist");
                    self.style_queue(ui, qr.shrink(14.0), acts);
                }
                ui.allocate_rect(r, Sense::hover());
            });
        }
        egui::CentralPanel::default().frame(none.inner_margin(margins)).show(ctx, |ui| {
            let r = ui.max_rect();
            self.lib_rect = r;
            let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(r));
            let z = self.lib_zoom;
            font::with_scale(font::ui_scale() * z, || self.library_ui(&mut inner, acts));
            ui.allocate_rect(r, Sense::hover());
        });
        // Streaming: the queue slides in over the page's right side (the owner's pick), a click outside it (not on the
        // player) puts it away
        if st == STYLE_STREAMING {
            let id = egui::Id::new("st_queue_slide");
            // (it glides in: a little longer, easing out as it arrives, fading in as it comes)
            let lin = ctx.animate_bool_with_time(id, self.stream_queue, 0.28);
            let t = 1.0 - (1.0 - lin).powi(3);
            if lin > 0.0 {
                let w = 360.0_f32.min(screen.width() * 0.45);
                let top = screen.min.y + crate::FRAME_TOP + 6.0;
                let bottom = screen.max.y - bar_h - 8.0;
                // (beside the column, over the page's right side)
                let x = screen.max.x - col_w - 6.0 - w * t;
                let qr = Rect::from_min_size(Pos2::new(x, top), Vec2::new(w, bottom - top));
                let area = egui::Area::new(id.with("area")).order(egui::Order::Foreground).fixed_pos(qr.min).show(ctx, |ui| {
                    ui.set_min_size(qr.size());
                    ui.set_opacity(t);
                    let p = ui.painter();
                    p.rect_filled(qr.translate(Vec2::new(-6.0, 4.0)).expand(4.0), 12.0, Color32::from_black_alpha(90));
                    p.rect_filled(qr, 8.0, Color32::from_rgb(24, 24, 24));
                    self.style_queue(ui, qr.shrink(14.0), acts);
                    ui.allocate_rect(qr, Sense::hover());
                });
                // (a click on the page puts it away; not one on the column or the bar, where the buttons that open it
                // are, or the same click that opened it would close it again)
                let page = Rect::from_min_max(screen.min, Pos2::new(screen.max.x - col_w, screen.max.y - bar_h));
                let outside = ctx.input(|i| i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|p| !qr.contains(p) && page.contains(p)));
                if self.stream_queue && outside && !area.response.hovered() && self.q_drag.is_none() {
                    self.stream_queue = false;
                }
            }
        }
    }

    /// What's behind the windows: Streaming black; Hardware its matte body; Ambient the album's colours, soft from
    /// corner to corner and darkened so the writing reads.
    fn style_backdrop(&self, ctx: &egui::Context, screen: Rect) {
        let p = ctx.layer_painter(egui::LayerId::background());
        match style() {
            STYLE_STREAMING => {
                p.rect_filled(screen, 0.0, Color32::BLACK);
            }
            STYLE_HARDWARE => {
                p.rect_filled(screen, 0.0, HW_BODY);
            }
            STYLE_STUDIO => {
                p.rect_filled(screen, 0.0, crate::studio::BODY);
            }
            _ => {
                let feel = crate::skin::AMB_FEEL.load(std::sync::atomic::Ordering::Relaxed);
                let shade = move |c: Color32| match feel {
                    1 => c.lerp_to_gamma(Color32::BLACK, 0.3),
                    2 => c.lerp_to_gamma(Color32::WHITE, 0.55),
                    _ => c.lerp_to_gamma(Color32::BLACK, 0.55),
                };
                let now = ctx.input(|i| i.time);
                let shown = self.amb_shown(now);
                if self.amb_fading(now) {
                    ctx.request_repaint();
                }
                let [a, b, c, d] = shown.map(shade);
                let mut m = egui::Mesh::default();
                m.colored_vertex(screen.left_top(), a);
                m.colored_vertex(screen.right_top(), b);
                m.colored_vertex(screen.left_bottom(), c);
                m.colored_vertex(screen.right_bottom(), d);
                m.add_triangle(0, 1, 2);
                m.add_triangle(1, 3, 2);
                p.add(m);
            }
        }
    }

    /// How far Ambient's colours are through their fade (0 to 1), eased in and out.
    fn amb_t(&self, now: f64) -> f32 {
        let fade = crate::skin::AMB_FADES[crate::skin::AMB_FADE.load(std::sync::atomic::Ordering::Relaxed) as usize % 3].1 as f64;
        let t = ((now - self.amb_t0) / fade).clamp(0.0, 1.0) as f32;
        t * t * (3.0 - 2.0 * t)
    }

    /// Whether Ambient's colours are still fading.
    fn amb_fading(&self, now: f64) -> bool {
        self.amb_t(now) < 1.0
    }

    /// Ambient's backdrop colours as shown now: on their way from the last album's to this one's.
    pub(crate) fn amb_shown(&self, now: f64) -> [Color32; 4] {
        let t = self.amb_t(now);
        let mut out = self.amb_tint;
        for (i, c) in out.iter_mut().enumerate() {
            *c = self.amb_from[i].lerp_to_gamma(self.amb_tint[i], t);
        }
        out
    }

    /// The song's controls: shuffle, previous, a round play / pause, next, repeat, centred on `c` (`d` the play
    /// button's size). Hardware has its own keys (`hw_player`).
    fn transport(&mut self, ui: &mut egui::Ui, c: Pos2, d: f32, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        let id = ui.id().with("st_transport");
        let playing = self.cur.is_some() && !self.stopped && !self.paused;
        let gap = d * 1.45;
        let rep_on = !matches!(self.repeat, crate::Repeat::Off);
        let items = [
            ("shuffle", Action::Shuffle, if self.shuffle { "Shuffle: on" } else { "Shuffle: off" }, self.shuffle),
            ("prev", Action::Prev, "Previous", false),
            (if playing { "pause" } else { "play" }, if playing { Action::PauseBtn } else { Action::PlayBtn }, if playing { "Pause" } else { "Play" }, false),
            ("skip", Action::Next, "Next", false),
            ("loop", Action::Repeat, "Repeat", rep_on),
        ];
        for (n, (icon, a, tip, lit)) in items.into_iter().enumerate() {
            let at = Pos2::new(c.x + (n as f32 - 2.0) * gap, c.y);
            let s = if n == 2 { d } else { d * 0.62 };
            let r = Rect::from_center_size(at, Vec2::splat(s));
            let h = ui.interact(r, id.with(n), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
            if n == 2 {
                p.circle_filled(at, s / 2.0 + if h.hovered() { 1.0 } else { 0.0 }, fg());
                let off = if playing { 0.0 } else { s * 0.04 };
                crate::vicon::named(&p, Rect::from_center_size(at + Vec2::new(off, 0.0), Vec2::splat(s * 0.42)), icon, on_fg());
            } else {
                let col = if lit {
                    pal().accent
                } else if h.hovered() {
                    fg()
                } else {
                    fg2()
                };
                crate::vicon::named(&p, r, icon, col);
                if lit {
                    p.circle_filled(Pos2::new(at.x, at.y + s * 0.62), 1.8, col);
                }
            }
            if h.clicked() {
                acts.push(a);
            }
        }
    }

    /// How far into the song: the time so far, a line you can click or drag along, the length. Returns its rect.
    fn progress(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>, col: Color32) {
        let p = ui.painter().clone();
        let dur = self.cur_track().map_or(0.0, |t| t.duration);
        let pos = if self.cur.is_some() { self.pos() } else { 0.0 };
        let tw = 44.0;
        let studio = style() == STYLE_STUDIO;
        let tcol = if studio { crate::studio::dim() } else { fg2() };
        crate::font::keep_caps(|| {
            ptext(&p, Pos2::new(r.min.x + tw - 6.0, r.center().y), Align::Max, &fmt_time(pos), 1.4, tcol);
            ptext(&p, Pos2::new(r.max.x - tw + 6.0, r.center().y), Align::Min, &fmt_time(dur), 1.4, tcol);
        });
        if studio {
            let line = Rect::from_min_max(Pos2::new(r.min.x + tw, r.center().y - 5.0), Pos2::new(r.max.x - tw, r.center().y + 5.0));
            let h = ui.interact(line.expand2(Vec2::new(0.0, 4.0)), ui.id().with("st_seek"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if h.clicked() || h.dragged() {
                if let (Some(pp), true) = (h.interact_pointer_pos(), dur > 0.0) {
                    acts.push(Action::Seek(((pp.x - line.min.x) / line.width()).clamp(0.0, 1.0) * dur));
                }
            }
            if dur > 0.0 {
                crate::widgets::seek_label(ui, &h, line, |x| ((x - line.min.x) / line.width()).clamp(0.0, 1.0) * dur);
            }
            crate::studio::blocks(&p, line, if dur > 0.0 { pos / dur } else { 0.0 });
            if self.cur.is_some() && !self.stopped && !self.paused {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
            }
            return;
        }
        let line = Rect::from_min_max(Pos2::new(r.min.x + tw, r.center().y - 2.0), Pos2::new(r.max.x - tw, r.center().y + 2.0));
        let h = ui.interact(line.expand2(Vec2::new(0.0, 6.0)), ui.id().with("st_seek"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand);
        if h.clicked() || h.dragged() {
            if let (Some(pp), true) = (h.interact_pointer_pos(), dur > 0.0) {
                acts.push(Action::Seek(((pp.x - line.min.x) / line.width()).clamp(0.0, 1.0) * dur));
            }
        }
        if dur > 0.0 {
            crate::widgets::seek_label(ui, &h, line, |x| ((x - line.min.x) / line.width()).clamp(0.0, 1.0) * dur);
        }
        let f = if dur > 0.0 { (pos / dur).clamp(0.0, 1.0) } else { 0.0 };
        p.rect_filled(line, 2.0, wash(75));
        p.rect_filled(Rect::from_min_size(line.min, Vec2::new(line.width() * f, line.height())), 2.0, if h.hovered() { pal().accent } else { col });
        if h.hovered() {
            p.circle_filled(Pos2::new(line.min.x + line.width() * f, line.center().y), 6.0, fg());
        }
        if self.cur.is_some() && !self.stopped && !self.paused {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    /// The volume as a short line to click or drag along, a speaker before it.
    fn volume_line(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        if style() == STYLE_STUDIO {
            crate::vicon::named(&p, Rect::from_center_size(Pos2::new(r.min.x + 9.0, r.center().y), Vec2::splat(16.0)), "headphones", crate::studio::ink());
            let line = Rect::from_min_max(Pos2::new(r.min.x + 26.0, r.center().y - 5.0), Pos2::new(r.max.x, r.center().y + 5.0));
            let h = ui.interact(line.expand2(Vec2::new(0.0, 5.0)), ui.id().with("st_vol"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Volume");
            if h.clicked() || h.dragged() {
                if let Some(pp) = h.interact_pointer_pos() {
                    acts.push(Action::Volume(((pp.x - line.min.x) / line.width()).clamp(0.0, 1.0)));
                }
            }
            crate::studio::blocks(&p, line, self.volume);
            return;
        }
        crate::vicon::named(&p, Rect::from_center_size(Pos2::new(r.min.x + 9.0, r.center().y), Vec2::splat(16.0)), "headphones", fg2());
        let line = Rect::from_min_max(Pos2::new(r.min.x + 26.0, r.center().y - 2.0), Pos2::new(r.max.x, r.center().y + 2.0));
        let h = ui.interact(line.expand2(Vec2::new(0.0, 6.0)), ui.id().with("st_vol"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Volume");
        if h.clicked() || h.dragged() {
            if let Some(pp) = h.interact_pointer_pos() {
                acts.push(Action::Volume(((pp.x - line.min.x) / line.width()).clamp(0.0, 1.0)));
            }
        }
        p.rect_filled(line, 2.0, wash(75));
        p.rect_filled(Rect::from_min_size(line.min, Vec2::new(line.width() * self.volume.clamp(0.0, 1.0), line.height())), 2.0, if h.hovered() { pal().accent } else { fg() });
    }

    /// The player along the window's foot, in the style worn: the song (cover, name, artist, heart) at the left, its
    /// controls and how far in at the middle, the equalizer, the queue (Streaming) and the volume at the right.
    /// Hardware's is its unit: the time big on a screen with the song, the transport as keys, the volume printed.
    fn player_bar(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        let st = style();
        let p = ui.painter().clone();
        let id = ui.id().with("st_player");
        // (Streaming's bar runs to the window's edges: more room at its ends, so nothing sits on the edge)
        let r = r.shrink2(Vec2::new(if st == STYLE_STREAMING { 22.0 } else { 14.0 }, 8.0));
        let mut right = r.max.x;
        // at the right: the volume, the queue (Streaming), the equalizer
        let vol = Rect::from_min_max(Pos2::new(right - 130.0, r.center().y - 8.0), Pos2::new(right, r.center().y + 8.0));
        if crate::hardware() {
            printed(&p, Pos2::new(vol.min.x - 6.0, vol.min.y - 10.0), "Vol");
        }
        self.volume_line(ui, vol, acts);
        right = vol.min.x - 18.0;
        if st == STYLE_STREAMING {
            let q = Rect::from_center_size(Pos2::new(right - 13.0, r.center().y), Vec2::splat(26.0));
            let tip = if self.stream_queue { "Hide queue" } else { "Show queue" };
            if self.small_toggle(ui, q, "queue", self.stream_queue, tip, id.with("queue")) {
                self.stream_queue = !self.stream_queue;
                self.dirty = true;
            }
            right = q.min.x - 8.0;
        }
        let e = Rect::from_center_size(Pos2::new(right - 13.0, r.center().y), Vec2::splat(26.0));
        if self.small_toggle(ui, e, "sliders", self.show_eq, if self.show_eq { "Hide equalizer" } else { "Show equalizer" }, id.with("eq")) {
            acts.push(Action::ToggleEq);
        }
        let vz = Rect::from_center_size(Pos2::new(e.min.x - 21.0, r.center().y), Vec2::splat(26.0));
        if self.small_toggle(ui, vz, "wave", self.mini_viz, if self.mini_viz { "Hide visualizer" } else { "Show visualizer" }, id.with("viz")) {
            self.mini_viz = !self.mini_viz;
            self.dirty = true;
        }
        let e = vz;
        if crate::hardware() {
            let studio = st == STYLE_STUDIO;
            // the cover, then the screen: the time big, the song beside it
            let cover = Rect::from_min_size(r.min, Vec2::splat(r.height()));
            let bar_dur = self.cur_track().map_or(0.0, |t| t.duration);
            match self.cur_track().map(|t| t.cover) {
                // (Studio: the cover as every Studio cover is, in color or in the screen's ink; a click switches)
                Some(_) if studio => {
                    self.studio_cover(ui, cover, "studio_bar", 160);
                    crate::studio::cover_switch(ui, cover, acts, "bar");
                }
                Some(c) if !c.is_empty() => crate::paint_held(ui, &mut self.images, "st_bar", &crate::cover_url(&c, 160), cover, false),
                _ => crate::paint_page_art(ui, &mut self.images, "", cover, false),
            }
            p.rect_stroke(cover, 3.0, Stroke::new(1.0_f32, Color32::from_black_alpha(200)));
            // (the keys need their room: a narrow window shrinks the screen, then the song on it, then the screen)
            let key_room = 5.0 * 54.0 + 40.0;
            let scr_w = (r.width() * 0.28).clamp(200.0, 340.0).min(e.min.x - cover.max.x - 10.0 - key_room);
            let screen_on = scr_w >= 150.0;
            let scr = Rect::from_min_max(Pos2::new(cover.max.x + 10.0, r.min.y), Pos2::new(cover.max.x + 10.0 + scr_w.max(0.0), r.max.y));
            if screen_on {
                lcd(&p, scr);
            }
            let pos = if self.cur.is_some() { self.pos() } else { 0.0 };
            if !screen_on {
            } else if studio {
                let t = self.deck_time();
                let shown = if t.starts_with('-') { t } else { format!(" {}", t) };
                crate::studio::time_digits(&p, Pos2::new(scr.min.x + 8.0, scr.center().y), 30.0, &shown, bar_dur);
            } else {
                hw_time(&p, Pos2::new(scr.min.x + 12.0, scr.center().y), &fmt_time(pos), bar_dur, 3.6, (scr.width() - 20.0).min(190.0));
            }
            if let Some(tr) = self.cur_track().filter(|_| scr.width() - 190.0 >= 60.0) {
                let w = scr.width() - 190.0;
                // (the song as written, over its artist, in the screen's face)
                screen_text(&p, Pos2::new(scr.max.x - 10.0, scr.center().y - 9.0), Align::Max, &tr.title, 1.3, w, amber());
                screen_text(&p, Pos2::new(scr.max.x - 10.0, scr.center().y + 9.0), Align::Max, &tr.artists_text(), 1.1, w, amber().gamma_multiply(0.65));
            }
            if ui.interact(scr, id.with("screen"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Now Playing  (A)").clicked() {
                acts.push(Action::ToggleArt);
            }
            // the keys, and how far in under them
            let mid = (if screen_on { scr.max.x } else { cover.max.x } + 20.0 + e.min.x - 20.0) / 2.0;
            let kw = 48.0;
            let playing = self.cur.is_some() && !self.stopped && !self.paused;
            let keys = [("prev", Action::Prev, "Previous", false), ("play", Action::PlayBtn, "Play", playing), ("pause", Action::PauseBtn, "Pause", self.paused), ("stop", Action::StopBtn, "Stop", false), ("skip", Action::Next, "Next", false)];
            for (n, (icon, a, tip, lit)) in keys.into_iter().enumerate() {
                let kr = Rect::from_min_size(Pos2::new(mid - 2.5 * (kw + 6.0) + n as f32 * (kw + 6.0), r.min.y + 4.0), Vec2::new(kw, 28.0));
                let h = ui.interact(kr, id.with(("key", n)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
                if studio {
                    // (Studio: a key with a fine outline, filled with the ink while it's on)
                    crate::studio::key(&p, kr, lit, h.is_pointer_button_down_on(), h.hovered());
                    crate::vicon::named(&p, Rect::from_center_size(kr.center(), Vec2::splat(13.0)), icon, crate::studio::key_fg(lit));
                } else {
                    let down = h.is_pointer_button_down_on();
                    crate::key_face_on(&p, kr, down, h.hovered(), lit);
                    let c = kr.center() + Vec2::new(0.0, if down { 1.0 } else { 0.0 });
                    crate::vicon::named(&p, Rect::from_center_size(c, Vec2::splat(13.0)), icon, crate::key_ink(lit));
                }
                if h.clicked() {
                    acts.push(a);
                }
            }
            let pw = (5.0 * (kw + 6.0) + 120.0).min(e.min.x - if screen_on { scr.max.x } else { cover.max.x } - 40.0);
            let prog = Rect::from_center_size(Pos2::new(mid, r.max.y - 12.0), Vec2::new(pw, 16.0));
            self.progress(ui, prog, acts, amber());
            return;
        }
        if let Some(tr) = self.cur_track() {
            let cs = r.height() - 8.0;
            let cover = Rect::from_min_size(Pos2::new(r.min.x, r.center().y - cs / 2.0), Vec2::splat(cs));
            if tr.cover.is_empty() {
                crate::paint_page_art(ui, &mut self.images, "", cover, false);
            } else {
                crate::paint_held(ui, &mut self.images, "st_bar", &crate::cover_url(&tr.cover, 160), cover, false);
            }
            let tx = cover.max.x + 12.0;
            let room = (r.center().x - 200.0 - tx).max(60.0);
            let tw = crate::row_text(&p, Pos2::new(tx, r.center().y - 9.0), Align::Min, &tr.title, room, fg());
            crate::row_text(&p, Pos2::new(tx, r.center().y + 10.0), Align::Min, &tr.artists_text(), room, fg2());
            let hr = Rect::from_min_size(Pos2::new(tx + tw.min(room) + 10.0, r.center().y - 19.0), Vec2::new(crate::HEART_W, 20.0));
            if crate::draw_heart(ui, hr, self.liked.contains(&tr.id), false, id.with("heart")) {
                acts.push(Action::ToggleLike(tr.clone()));
            }
            // (a click on the song: Now Playing)
            let song = Rect::from_min_max(cover.min, Pos2::new(tx + room, cover.max.y));
            if ui.interact(song, id.with("song"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Now Playing  (A)").clicked() {
                acts.push(Action::ToggleArt);
            }
        }
        self.transport(ui, Pos2::new(r.center().x, r.min.y + 20.0), 34.0, acts);
        let pw = (r.width() * 0.4).clamp(260.0, 620.0);
        let prog = Rect::from_center_size(Pos2::new(r.center().x, r.max.y - 12.0), Vec2::new(pw, 16.0));
        self.progress(ui, prog, acts, fg());
    }

    /// A small icon that switches something on and off (the equalizer, the queue, the visualizer): lit, with a dot
    /// under it, while on. Returns the click.
    fn small_toggle(&mut self, ui: &mut egui::Ui, r: Rect, icon: &str, on: bool, tip: &str, id: egui::Id) -> bool {
        let p = ui.painter().clone();
        let h = ui.interact(r, id, Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
        let col = if on {
            if crate::hardware() { amber() } else { pal().accent }
        } else if h.hovered() {
            fg()
        } else {
            fg2()
        };
        crate::vicon::named(&p, r.shrink(r.width() * 0.2), icon, col);
        if on {
            p.circle_filled(Pos2::new(r.center().x, r.max.y + 1.0), 1.8, col);
        }
        h.clicked()
    }

    /// While music plays, the visualizers keep moving (about 30 frames a second).
    fn viz_tick(&self, ctx: &egui::Context) {
        if self.cur.is_some() && !self.stopped && !self.paused {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }
    }

    /// The equalizer on a panel of the style worn: its heading and a close, then Backline's own equalizer (on / off,
    /// AUTO, presets, the sliders).
    fn style_eq(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        panel_named(&p, r, "Equalizer");
        let inner = r.shrink(14.0);
        if style() == STYLE_STUDIO {
            // (its name is on its frame)
        } else if style() == STYLE_HARDWARE {
            printed(&p, Pos2::new(inner.min.x, inner.min.y + 6.0), "Equalizer");
        } else {
            ptext(&p, Pos2::new(inner.min.x, inner.min.y + 8.0), Align::Min, "Equalizer", 1.9, fg());
        }
        let x = Rect::from_min_size(Pos2::new(inner.max.x - 20.0, inner.min.y - 2.0), Vec2::splat(20.0));
        let h = ui.interact(x, ui.id().with("st_eq_close"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Close");
        crate::vicon::named(&p, x.shrink(5.0), "close", if h.hovered() { fg() } else { fg2() });
        if h.clicked() {
            self.show_eq = false;
        }
        let body = Rect::from_min_max(Pos2::new(inner.min.x, inner.min.y + 18.0), inner.max);
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(Align::Min)));
        c.set_clip_rect(body.intersect(ui.clip_rect()));
        self.eq_view(&mut c);
    }

    /// The queue as the style shows it: a heading with how many songs are still to come and how long they run, then
    /// the songs, each with its cover (the ones played greyed, the one playing lit); a click plays one, a drag (held
    /// left button or the middle one) moves it. Hardware's on an amber screen, numbered, in capitals.
    pub(crate) fn style_queue(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        let studio = style() == STYLE_STUDIO;
        let hw = style() == STYLE_HARDWARE;
        let p = ui.painter().clone();
        let queue = self.queue.clone();
        let cur = self.cur;
        let head = Rect::from_min_size(r.min, Vec2::new(r.width(), 24.0));
        if studio {
            // (its name is on its frame)
        } else if hw {
            printed(&p, Pos2::new(head.min.x, head.center().y), "Playlist");
        } else {
            ptext(&p, Pos2::new(head.min.x + 2.0, head.center().y), Align::Min, "Playlist", 2.1, fg());
        }
        // where you are: the song's place out of them all, the time left out of the whole list's (the owner's words)
        let total: f32 = queue.iter().map(|t| t.duration).sum();
        // (how far into the whole queue, counting up: the songs before this one and this one so far)
        let done: f32 = match cur {
            Some(c) => queue.iter().take(c).map(|t| t.duration).sum::<f32>() + self.pos(),
            None => 0.0,
        };
        let info = if queue.is_empty() {
            String::new()
        } else {
            // (compact, the owner's way: "Song 4/4926 · 0:13:07/354:14:59", how far in out of the whole)
            let at = cur.map_or(String::new(), |c| format!("Song {}/{} · ", c + 1, queue.len()));
            format!("{}{}/{}", at, crate::fmt_long(done), crate::fmt_long(total))
        };
        // (no room beside the heading: on a line of its own under it)
        let px = if hw { 1.0 } else { 1.4 };
        let head_w = if hw || studio { 70.0 } else { text_w("Playlist", 2.1) + 16.0 };
        let below = !studio && head_w + text_w(&info, px) + 8.0 > r.width();
        let at = if below { Pos2::new(head.min.x + 2.0, head.max.y + 6.0) } else { Pos2::new(head.max.x - 4.0, head.center().y) };
        let align = if below { Align::Min } else { Align::Max };
        if studio {
            crate::studio::px(&p, Pos2::new(head.min.x, head.center().y), egui::Align2::LEFT_CENTER, &info.to_uppercase(), 9.5, crate::studio::dim());
            p.line_segment([Pos2::new(head.min.x, head.max.y), Pos2::new(head.max.x, head.max.y)], Stroke::new(1.0_f32, crate::studio::ink()));
        } else if hw {
            font::keep_caps(|| ptext_fit(&p, at, align, &info.to_uppercase(), px, r.width() - 4.0, Color32::from_gray(128)));
        } else {
            ptext_fit(&p, at, align, &info, px, r.width() - 4.0, fg2());
        }
        let list = Rect::from_min_max(Pos2::new(r.min.x, head.max.y + if below { 18.0 } else { 4.0 }), r.max);
        if hw {
            lcd(&p, list);
        }
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(list.shrink(if hw { 6.0 } else { 0.0 })).layout(egui::Layout::top_down(Align::Min)));
        egui::ScrollArea::vertical().id_salt("st_queue").auto_shrink([false, false]).show(&mut c, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let w0 = ui.available_width();
            // (roomier rows: bigger covers, more space between, the song over its artist)
            let rh = if hw { 42.0 } else if studio { 40.0 } else { 50.0 };
            let mut rects: Vec<(usize, Rect)> = Vec::new();
            for (i, tr) in queue.iter().enumerate() {
                let (row0, h) = ui.allocate_exact_size(Vec2::new(w0, rh), Sense::click_and_drag());
                rects.push((i, row0));
                if cur == Some(i) {
                    if self.q_jump {
                        ui.scroll_to_rect(row0, Some(Align::Center));
                    }
                    self.q_cur_seen = ui.is_rect_visible(row0);
                }
                if !ui.is_rect_visible(row0) {
                    continue;
                }
                if h.drag_started_by(egui::PointerButton::Primary) || h.drag_started_by(egui::PointerButton::Middle) {
                    self.q_drag = Some(i);
                }
                let now = cur == Some(i);
                let played = cur.is_some_and(|c| i < c);
                let dim = if played { 0.42 } else { 1.0 };
                let dur = fmt_time(tr.duration);
                // (each song's cover at its left)
                let cs = rh - 10.0;
                let cr = Rect::from_min_size(Pos2::new(row0.min.x + if hw { 3.0 } else { 6.0 }, row0.center().y - cs / 2.0), Vec2::splat(cs));
                let url = if tr.cover.is_empty() { String::new() } else { format!("thumb:{}", crate::api::cover_url(&tr.cover, 80)) };
                crate::row_cover(ui, cr, &url);
                if played {
                    ui.painter().rect_filled(cr, 3.0, Color32::from_black_alpha(130));
                }
                let row = Rect::from_min_max(Pos2::new(cr.max.x + 2.0, row0.min.y), row0.max);
                let w = row.width();
                let p = ui.painter();
                if studio {
                    // (Studio: the song playing inverted, the ones played faded, the pointer's row shaded)
                    let (ink, glass) = (crate::studio::ink(), crate::studio::scr());
                    if now {
                        p.rect_filled(row, 0.0, ink);
                    } else if h.hovered() {
                        p.rect_filled(row, 0.0, crate::studio::key_col());
                    }
                    let col = if now { glass } else { ink.gamma_multiply(dim) };
                    let col2 = if now { glass.gamma_multiply(0.75) } else { crate::studio::dim().gamma_multiply(dim) };
                    let dw = ptext(p, Pos2::new(row.max.x - 6.0, row.center().y), Align::Max, &dur, 1.5, col2);
                    let room = w - dw - 24.0;
                    crate::row_text(p, Pos2::new(row.min.x + 8.0, row.center().y - 8.0), Align::Min, &tr.title, room, col);
                    crate::row_text(p, Pos2::new(row.min.x + 8.0, row.center().y + 9.0), Align::Min, &tr.artists_text(), room, col2);
                } else if hw {
                    // (Hardware: the song over its artist on the screen, as every look has it; the length at the right)
                    if now || h.hovered() {
                        p.rect_filled(row, 2.0, amber().gamma_multiply(if now { 0.14 } else { 0.06 }));
                    }
                    let col = amber().gamma_multiply(dim);
                    let dw = screen_w(p, &dur, 1.2);
                    screen_text(p, Pos2::new(row.max.x - 6.0, row.center().y), Align::Max, &dur, 1.2, dw + 4.0, col);
                    let room = w - dw - 22.0;
                    screen_text(p, Pos2::new(row.min.x + 6.0, row.center().y - 8.0), Align::Min, &tr.title, 1.25, room, col);
                    screen_text(p, Pos2::new(row.min.x + 6.0, row.center().y + 9.0), Align::Min, &tr.artists_text(), 1.05, room, col.gamma_multiply(0.6));
                } else {
                    if now || h.hovered() {
                        p.rect_filled(row, 6.0, wash(if now { 34 } else { 16 }));
                    }
                    if now {
                        p.circle_filled(Pos2::new(row.min.x + 6.0, row.center().y), 2.5, pal().accent);
                    }
                    let col = if now { fg() } else { fg().gamma_multiply(0.88 * dim) };
                    let dw = ptext(p, Pos2::new(row.max.x - 8.0, row.center().y), Align::Max, &dur, 1.6, fg2().gamma_multiply(dim));
                    let room = w - dw - 30.0;
                    crate::row_text(p, Pos2::new(row.min.x + 14.0, row.center().y - 9.0), Align::Min, &tr.title, room, col);
                    crate::row_text(p, Pos2::new(row.min.x + 14.0, row.center().y + 10.0), Align::Min, &tr.artists_text(), room, fg2().gamma_multiply(dim));
                }
                if h.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    acts.push(Action::PlayIndex(i));
                }
            }
            self.q_jump = false;
            if cur.is_none() {
                self.q_cur_seen = true;
            }
            // a song being moved: a line where it will land, moved on release
            if let Some(from) = self.q_drag {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                let released = ui.input(|i| i.pointer.any_released());
                if let Some(pp) = ui.input(|i| i.pointer.hover_pos()) {
                    let ins = rects
                        .iter()
                        .find(|(_, rc)| pp.y < rc.center().y)
                        .map(|(i, rc)| (*i, rc.min.y))
                        .or_else(|| rects.last().map(|(i, rc)| (*i + 1, rc.max.y)));
                    if let Some((ins_i, y)) = ins {
                        let col = if hw { amber() } else { pal().accent };
                        ui.painter().rect_filled(Rect::from_min_size(Pos2::new(list.min.x, y - 1.0), Vec2::new(list.width(), 2.0)), 1.0, col);
                        if released {
                            acts.push(Action::MoveQueue(from, ins_i));
                        }
                    }
                }
                if released || !ui.input(|i| i.pointer.any_down()) {
                    self.q_drag = None;
                }
            }
        });
    }
}

/// A style in little (its picture on the Skins page's card, and in Settings while it's worn), filling `r`.
pub(crate) fn style_picture(p: &egui::Painter, r: Rect, kind: u8) {
    let k = r.width() / 200.0;
    let bx = |x: f32, y: f32, ww: f32, hh: f32| Rect::from_min_size(r.min + Vec2::new(x * k, y * k), Vec2::new(ww * k, hh * k));
    let rgb = Color32::from_rgb;
    match kind {
        STYLE_STREAMING => {
            p.rect_filled(r, 6.0, Color32::BLACK);
            p.rect_filled(bx(4.0, 4.0, 38.0, 92.0), 3.0, rgb(21, 21, 21));
            p.rect_filled(bx(46.0, 4.0, 150.0, 92.0), 3.0, rgb(21, 21, 21));
            crate::library_win::color_wash(p, bx(46.0, 4.0, 150.0, 50.0), rgb(110, 60, 28));
            for x in [58.0, 92.0, 126.0] {
                p.rect_filled(bx(x, 52.0, 30.0, 30.0), 2.0, rgb(38, 38, 38));
            }
            p.rect_filled(bx(4.0, 100.0, 192.0, 16.0), 2.0, rgb(12, 12, 12));
            p.circle_filled(bx(96.0, 103.0, 10.0, 10.0).center(), 5.0 * k, Color32::WHITE);
        }
        STYLE_AMBIENT => {
            let mut m = egui::Mesh::default();
            for (pt, c) in [(r.left_top(), rgb(122, 58, 26)), (r.right_top(), rgb(90, 30, 80)), (r.left_bottom(), rgb(58, 20, 64)), (r.right_bottom(), rgb(26, 10, 36))] {
                m.colored_vertex(pt, c);
            }
            m.add_triangle(0, 1, 2);
            m.add_triangle(1, 3, 2);
            p.add(m);
            for b in [bx(4.0, 4.0, 36.0, 112.0), bx(44.0, 4.0, 108.0, 112.0), bx(156.0, 4.0, 40.0, 40.0), bx(156.0, 48.0, 40.0, 68.0)] {
                p.rect_filled(b, 3.0, Color32::from_black_alpha(80));
            }
            p.rect_filled(bx(160.0, 8.0, 18.0, 18.0), 2.0, rgb(208, 112, 47));
        }
        STYLE_HARDWARE => {
            let (fg, bg) = crate::skin::hw_screen();
            p.rect_filled(r, 6.0, HW_BODY);
            for b in [bx(4.0, 4.0, 36.0, 112.0), bx(44.0, 4.0, 108.0, 112.0), bx(156.0, 4.0, 40.0, 40.0)] {
                p.rect_filled(b, 3.0, HW_PANEL);
            }
            p.rect_filled(bx(48.0, 8.0, 100.0, 12.0), 2.0, bg);
            p.rect_filled(bx(159.0, 8.0, 34.0, 14.0), 2.0, bg);
            for (i, x) in [159.0, 169.0, 179.0].into_iter().enumerate() {
                p.rect_filled(bx(x, 26.0, 8.0, 6.0), 1.0, if i == 1 { fg } else { rgb(58, 61, 66) });
            }
            p.rect_filled(bx(156.0, 48.0, 40.0, 68.0), 3.0, bg);
            for i in 0..5 {
                p.circle_filled(bx(8.0, 14.0 + i as f32 * 10.0, 4.0, 4.0).center(), 1.5 * k, if i == 3 { fg } else { rgb(60, 52, 40) });
            }
        }
        STYLE_STUDIO => {
            // (Studio: its screens in the glass's colour, set in the black body, a dark rim round each)
            let (_, glass, keys, ink, _) = crate::skin::studio_screen();
            p.rect_filled(r, 6.0, crate::studio::BODY);
            for b in [bx(5.0, 7.0, 34.0, 109.0), bx(44.0, 7.0, 107.0, 109.0), bx(156.0, 7.0, 39.0, 44.0), bx(156.0, 56.0, 39.0, 60.0)] {
                p.rect_filled(b.expand(1.5), 0.0, Color32::from_rgb(59, 61, 49));
                p.rect_filled(b, 0.0, glass);
            }
            p.rect_filled(bx(48.0, 12.0, 40.0, 7.0), 0.0, ink);
            for i in 0..6 {
                p.rect_filled(bx(48.0, 24.0 + i as f32 * 9.0, 99.0, 5.0), 0.0, if i == 0 { ink } else { keys });
            }
            p.rect_filled(bx(160.0, 12.0, 14.0, 14.0), 0.0, keys);
            p.rect_filled(bx(177.0, 12.0, 14.0, 8.0), 0.0, ink);
        }
        _ => {
            p.rect_filled(r, 6.0, rgb(43, 43, 44));
            p.rect_filled(bx(4.0, 4.0, 140.0, 112.0), 3.0, rgb(13, 13, 13));
            p.rect_filled(bx(4.0, 4.0, 34.0, 112.0), 3.0, rgb(26, 26, 27));
            p.rect_filled(bx(148.0, 4.0, 48.0, 30.0), 3.0, rgb(27, 27, 27));
            p.rect_filled(bx(151.0, 9.0, 18.0, 9.0), 1.0, Color32::BLACK);
            p.rect_filled(bx(148.0, 38.0, 48.0, 78.0), 3.0, rgb(13, 13, 13));
        }
    }
}

/// A style as a whole window in little, for the Skins page's first step: Classic (0, the deck and the playlist beside
/// the library), Streaming, Ambient Color, Hardware; its name under it, a ring round the one worn. Returns the click.
pub(crate) fn style_card(ui: &mut egui::Ui, kind: u8, name: &str, w: f32, on: bool) -> bool {
    let h = w * 0.6;
    let name_h = 26.0 * font::ui_scale();
    let (cell, resp) = ui.allocate_exact_size(Vec2::new(w, h + name_h), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let p = ui.painter();
    let r = Rect::from_min_size(cell.min, Vec2::new(w, h));
    style_picture(p, r, kind);
    if on {
        p.rect_stroke(r.expand(3.0), 8.0, Stroke::new(2.0_f32, pal().ink));
    } else if resp.hovered() {
        p.rect_stroke(r.expand(3.0), 8.0, Stroke::new(1.0_f32, pal().ink2));
    }
    // (smaller to fit a narrow card, never into the next one's name)
    font::ptext_fit(p, Pos2::new(cell.min.x + 2.0, r.max.y + name_h * 0.55), Align::Min, name, 1.9, w - 4.0, if on || resp.hovered() { pal().ink } else { pal().ink2 });
    resp.clicked()
}

/// A row of colour choices (the Skins page's second step): each a small swatch (`main` on `back`), its name under it,
/// a ring round the one picked. Returns the one clicked.
pub(crate) fn swatches(ui: &mut egui::Ui, items: &[(&str, Color32, Color32)], cur: usize, dot: bool) -> Option<usize> {
    let sc = font::ui_scale();
    let mut hit = None;
    // (as many to a row as fit: a narrow page wraps them, never runs them off its side)
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(14.0 * sc, 10.0 * sc);
        for (i, (name, main, back)) in items.iter().enumerate() {
            let w = (text_w(name, 1.6) + 8.0).max(64.0 * sc);
            let (cell, resp) = ui.allocate_exact_size(Vec2::new(w, 64.0 * sc), Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            let p = ui.painter();
            let sw = Rect::from_min_size(cell.min + Vec2::new((w - 56.0 * sc) / 2.0, 2.0), Vec2::new(56.0 * sc, 36.0 * sc));
            p.rect_filled(sw, 5.0, *back);
            if dot {
                p.circle_filled(sw.center(), 9.0 * sc, main.gamma_multiply(0.25));
                p.circle_filled(sw.center(), 5.0 * sc, *main);
            } else {
                p.rect_filled(sw.shrink(9.0 * sc), 3.0, *main);
            }
            if i == cur {
                p.rect_stroke(sw.expand(3.0), 7.0, Stroke::new(2.0_f32, pal().ink));
            } else if resp.hovered() {
                p.rect_stroke(sw.expand(3.0), 7.0, Stroke::new(1.0_f32, pal().ink2));
            }
            ptext(p, Pos2::new(cell.center().x, sw.max.y + 13.0 * sc), Align::Center, name, 1.6, if i == cur { pal().ink } else { pal().ink2 });
            if resp.clicked() {
                hit = Some(i);
            }
        }
    });
    hit
}

/// The mini visualizer: the music's levels as a row of bars in `r` (Hardware's as lit segments, as a meter's).
fn mini_viz(p: &egui::Painter, r: Rect, bands: &[f32], col: Color32, segments: bool) {
    if bands.is_empty() || r.width() < 20.0 {
        return;
    }
    let n = ((r.width() / 7.0) as usize).clamp(8, 32);
    let gap = 2.0;
    let bw = (r.width() - gap * (n - 1) as f32) / n as f32;
    for i in 0..n {
        let v = bands[(i * bands.len() / n).min(bands.len() - 1)].clamp(0.0, 1.0);
        let x = r.min.x + i as f32 * (bw + gap);
        if segments {
            let seg = 3.0;
            let lit = ((v * r.height()) / (seg + 1.0)).floor() as i32;
            for s in 0..lit.max(1) {
                let y = r.max.y - (s as f32 + 1.0) * (seg + 1.0);
                p.rect_filled(Rect::from_min_size(Pos2::new(x, y), Vec2::new(bw, seg)), 0.0, col.gamma_multiply(if s < lit { 1.0 } else { 0.25 }));
            }
        } else {
            let h = (v * r.height()).max(2.0);
            p.rect_filled(Rect::from_min_max(Pos2::new(x, r.max.y - h), Pos2::new(x + bw, r.max.y)), 1.5, col);
        }
    }
}

impl App {
    /// The mini player in a style, laid out as the deck is (the owner held the classic deck up as the clean one): the
    /// time with the visualizer beside it and the song over them (Streaming and Ambient with its cover; Hardware all on
    /// its screen), then how far in, then the controls in the middle with the volume. At its top: the visualizer, the
    /// equalizer and the playlist switched on and off, back to the full window, quit. The equalizer and the playlist
    /// open under it (the window grows, as the classic stack does).
    pub(crate) fn style_mini(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let st = style();
        let hw = crate::hardware();
        let full = ui.max_rect();
        let p = ui.painter().clone();
        let ctx = ui.ctx().clone();
        self.style_backdrop(&ctx, full);
        let deck_h = (full.width() / 275.0 * 116.0).min(full.height());
        let card = Rect::from_min_size(full.min, Vec2::new(full.width(), deck_h)).shrink(6.0);
        panel(&p, card);
        // the top strip: moves the window; its switches and buttons at the right
        crate::app_bar_strip(ui, Rect::from_min_size(card.min, Vec2::new(card.width() - 130.0, 22.0)), ui.id().with("st_mini_bar"));
        let mut x = card.max.x - 6.0;
        for (n, (icon, tip)) in [("close", "Quit Backline"), ("max", "Exit mini player")].into_iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(x - 20.0, card.min.y + 3.0), Vec2::splat(20.0));
            x = r.min.x - 2.0;
            let h = ui.interact(r, ui.id().with(("st_mini_btn", n)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
            crate::quiet_h(&p, r, h.is_pointer_button_down_on(), h.hovered());
            crate::vicon::named(&p, r.shrink(5.0), icon, if h.hovered() { fg() } else { fg2() });
            if h.clicked() {
                acts.push(if n == 0 { Action::QuitApp } else { Action::ToggleMini });
            }
        }
        x -= 8.0;
        let pl_on = !self.queue_hidden;
        let toggles = [("list", pl_on, "Playlist"), ("sliders", self.show_eq, "Equalizer"), ("wave", self.mini_viz, "Visualizer")];
        for (n, (icon, on, tip)) in toggles.into_iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(x - 20.0, card.min.y + 3.0), Vec2::splat(20.0));
            x = r.min.x - 4.0;
            if self.small_toggle(ui, r, icon, on, tip, ui.id().with(("st_mini_tg", n))) {
                match n {
                    0 => self.queue_hidden = !self.queue_hidden,
                    1 => acts.push(Action::ToggleEq),
                    _ => self.mini_viz = !self.mini_viz,
                }
                self.dirty = true;
            }
        }
        let inner = Rect::from_min_max(card.min + Vec2::new(10.0, 26.0), card.max - Vec2::new(10.0, 8.0));
        let row_a = Rect::from_min_size(inner.min, Vec2::new(inner.width(), (inner.height() * 0.5).max(40.0)));
        let pos = if self.cur.is_some() { self.pos() } else { 0.0 };
        let (bands, peaks, _) = self.viz_data(0);
        let playing = self.cur.is_some() && !self.stopped && !self.paused;
        if crate::studio::on() {
            // one screen: the time in segments, the song in dot type at its top right, the bars under it
            use crate::studio as sd;
            lcd(&p, row_a);
            let dh = (row_a.height() * 0.5).clamp(18.0, 52.0);
            sd::digits(&p, Pos2::new(row_a.min.x + 12.0, row_a.center().y), dh, &fmt_time(pos), sd::ink(), sd::ghost());
            let tx = row_a.min.x + row_a.width() * 0.42;
            if let Some(tr) = self.cur_track() {
                let mq = Rect::from_min_max(Pos2::new(tx, row_a.min.y + 4.0), Pos2::new(row_a.max.x - 8.0, row_a.min.y + 24.0));
                sd::marquee(ui, mq, &format!("{} - {}", tr.title.to_uppercase(), tr.artists_text().to_uppercase()), 14.0, sd::ink());
            }
            if self.mini_viz {
                let vr = Rect::from_min_max(Pos2::new(tx, row_a.min.y + 30.0), row_a.max - Vec2::new(8.0, 6.0));
                sd::bars(&p, vr, &crate::viz::resample(&bands, 24, 1.0), &crate::viz::resample(&peaks, 24, 1.0), 2.0, 2.0);
            }
        } else if hw {
            // all on the screen: the time big, the song small at its top right, the meter under it
            lcd(&p, row_a);
            let big = (row_a.height() / 16.0).clamp(1.8, 4.2);
            if style() == STYLE_HARDWARE {
                // (the time in 7-segment digits)
                hw_digits(&p, Pos2::new(row_a.min.x + 12.0, row_a.center().y), (row_a.height() * 0.5).clamp(18.0, 52.0), &fmt_time(pos), amber());
            } else {
                lcd_text(&p, Pos2::new(row_a.min.x + 10.0, row_a.center().y), Align::Min, &fmt_time(pos), big, row_a.width() * 0.4);
            }
            let tx = row_a.min.x + row_a.width() * 0.42;
            if let Some(tr) = self.cur_track() {
                screen_text(&p, Pos2::new(tx, row_a.min.y + 11.0), Align::Min, &format!("{} · {}", tr.title, tr.artists_text()), 1.15, row_a.max.x - tx - 8.0, amber());
            }
            if self.mini_viz {
                mini_viz(&p, Rect::from_min_max(Pos2::new(tx, row_a.min.y + 22.0), row_a.max - Vec2::new(8.0, 6.0)), &bands, amber(), true);
            }
        } else {
            let cover = Rect::from_min_size(row_a.min, Vec2::splat(row_a.height()));
            if let Some(tr) = self.cur_track() {
                if tr.cover.is_empty() {
                    crate::paint_page_art(ui, &mut self.images, "", cover, false);
                } else {
                    crate::paint_held(ui, &mut self.images, "st_mini", &crate::cover_url(&tr.cover, 320), cover, false);
                }
                let tx = cover.max.x + 10.0;
                let room = row_a.max.x - tx;
                crate::row_text(&p, Pos2::new(tx, row_a.min.y + 8.0), Align::Min, &format!("{} - {}", tr.artists_text(), tr.title), room, fg());
                let big = (row_a.height() / 22.0).clamp(1.6, 3.4);
                let tw = font::keep_caps(|| ptext(&p, Pos2::new(tx, row_a.max.y - row_a.height() * 0.3), Align::Min, &fmt_time(pos), big, fg()));
                if self.mini_viz {
                    let vr = Rect::from_min_max(Pos2::new(tx + tw + 14.0, row_a.min.y + 20.0), Pos2::new(row_a.max.x, row_a.max.y - 2.0));
                    mini_viz(&p, vr, &bands, if st == STYLE_AMBIENT { fg().gamma_multiply(0.8) } else { pal().accent }, false);
                }
            }
        }
        if self.mini_viz {
            self.viz_tick(&ctx);
        }
        // how far in
        let prog = Rect::from_min_max(Pos2::new(inner.min.x - 4.0, row_a.max.y + 6.0), Pos2::new(inner.max.x + 4.0, row_a.max.y + 22.0));
        self.progress(ui, prog, acts, if hw { amber() } else { fg() });
        // the controls in the middle, the volume at the right
        let cy = (prog.max.y + inner.max.y) / 2.0 + 2.0;
        let room_h = inner.max.y - prog.max.y;
        let vol = Rect::from_min_max(Pos2::new(inner.max.x - (inner.width() * 0.2).clamp(60.0, 110.0), cy - 8.0), Pos2::new(inner.max.x, cy + 8.0));
        if hw {
            let kh = (room_h - 6.0).clamp(16.0, 28.0);
            let kw = (inner.width() * 0.11).clamp(30.0, 48.0);
            let keys = [("prev", Action::Prev), ("play", Action::PlayBtn), ("pause", Action::PauseBtn), ("stop", Action::StopBtn), ("skip", Action::Next)];
            for (n, (icon, a)) in keys.into_iter().enumerate() {
                let kr = Rect::from_min_size(Pos2::new(inner.center().x - 2.5 * (kw + 5.0) + n as f32 * (kw + 5.0), cy - kh / 2.0), Vec2::new(kw, kh));
                let h = ui.interact(kr, ui.id().with(("st_mini_key", n)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                if crate::studio::on() {
                    let lit = n == 1 && playing;
                    crate::studio::key(&p, kr, lit, h.is_pointer_button_down_on(), h.hovered());
                    crate::vicon::named(&p, Rect::from_center_size(kr.center(), Vec2::splat(12.0)), icon, crate::studio::key_fg(lit));
                } else {
                    let lit = n == 1 && playing;
                    let down = h.is_pointer_button_down_on();
                    crate::key_face_on(&p, kr, down, h.hovered(), lit);
                    let c = kr.center() + Vec2::new(0.0, if down { 1.0 } else { 0.0 });
                    crate::vicon::named(&p, Rect::from_center_size(c, Vec2::splat(12.0)), icon, crate::key_ink(lit));
                }
                if h.clicked() {
                    acts.push(a);
                }
            }
        } else {
            self.transport(ui, Pos2::new(inner.center().x, cy), 30.0_f32.min(room_h - 2.0).max(18.0), acts);
        }
        if vol.min.x > inner.center().x + 150.0 {
            self.volume_line(ui, vol, acts);
        }
        // under it: the equalizer, then the playlist
        let mut y = card.max.y + 6.0;
        if self.show_eq {
            let eq_h = (170.0 * full.width() / 275.0).max(330.0);
            let er = Rect::from_min_max(Pos2::new(full.min.x + 6.0, y), Pos2::new(full.max.x - 6.0, (y + eq_h - 6.0).min(full.max.y - 6.0)));
            if er.height() > 60.0 {
                self.style_eq(ui, er);
            }
            y = er.max.y + 6.0;
        }
        if pl_on && full.max.y - y > 60.0 {
            let qr = Rect::from_min_max(Pos2::new(full.min.x + 6.0, y), full.max - Vec2::splat(6.0));
            panel(&p, qr);
            self.style_queue(ui, qr.shrink(10.0), acts);
        }
        ui.allocate_rect(full, Sense::hover());
    }
}

impl App {
    /// The time as the deck shows it: elapsed, or (clicked) what's left with a minus.
    pub(crate) fn deck_time(&self) -> String {
        let pos = if self.cur.is_some() { self.pos() } else { 0.0 };
        if self.time_left {
            let dur = self.cur_track().map_or(0.0, |t| t.duration);
            format!("-{}", fmt_time((dur - pos).max(0.0)))
        } else {
            fmt_time(pos)
        }
    }

    /// What the deck says about the stream: its bitrate, sample rate and channels ("1411 kbps · 44 kHz · Stereo").
    pub(crate) fn deck_info(&self) -> String {
        let active = self.cur.is_some() && !self.stopped;
        if !active {
            return String::new();
        }
        let rate = self.player.shared.lock().map(|s| s.rate).unwrap_or(0);
        let mono = self.practice && self.chan != 0;
        format!("{} kbps · {} kHz · {}", self.kbps.min(9999), (rate / 1000).min(999), if mono { "Mono" } else { "Stereo" })
    }

    /// A style's deck at the top right (the owner: the classic player's controls, made easy, in every style): the time
    /// (a click shows what's left), the stream's details, the song and the visualizer; how far in; previous, play,
    /// pause, stop, next and open files; shuffle, repeat, the equalizer and the playlist; the volume and the balance.
    /// Hardware's on its screen with keys; the others with their cover, plain icons and lines.
    pub(crate) fn style_deck(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        let hw = style() == STYLE_HARDWARE;
        let p = ui.painter().clone();
        let id = ui.id().with("st_deck");
        let tr = self.cur_track();
        let (bands, _, _) = self.viz_data(0);
        // ---- the cover, then the screen (Hardware) or the song beside it
        let s = font::ui_scale();
        let top = Rect::from_min_size(r.min, Vec2::new(r.width(), (r.height() * 0.46).clamp(90.0 * s, 128.0 * s)));
        let cover = Rect::from_min_size(top.min, Vec2::splat(top.height()));
        match tr.as_ref().map(|t| t.cover.as_str()) {
            Some(c) if !c.is_empty() => crate::paint_held(ui, &mut self.images, "st_deck", &crate::cover_url(c, 320), cover, false),
            _ => crate::paint_page_art(ui, &mut self.images, "", cover, false),
        }
        if hw {
            p.rect_stroke(cover, 3.0, Stroke::new(1.0_f32, Color32::from_black_alpha(200)));
        }
        if ui.interact(cover, id.with("cover"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Now Playing  (A)").clicked() {
            acts.push(Action::ToggleArt);
        }
        let time_r;
        if hw {
            // the screen: the song on top, the time big with the meter beside it, the stream's details along the foot
            let scr = Rect::from_min_max(Pos2::new(cover.max.x + 10.0, top.min.y), top.max);
            lcd(&p, scr);
            let w = scr.width() - 20.0;
            if let Some(tr) = &tr {
                screen_text(&p, Pos2::new(scr.min.x + 10.0, scr.min.y + 13.0 * s), Align::Min, &tr.title, 1.3, w, amber());
                screen_text(&p, Pos2::new(scr.min.x + 10.0, scr.min.y + 29.0 * s), Align::Min, &tr.artists_text(), 1.05, w, amber().gamma_multiply(0.65));
            }
            let mid = scr.min.y + scr.height() * 0.58;
            let big = ((scr.height() / s - 50.0) / 9.0).clamp(2.6, 4.4);
            let tw = hw_time(&p, Pos2::new(scr.min.x + 10.0, mid), &self.deck_time(), tr.as_ref().map_or(0.0, |t| t.duration), big, w * 0.7);
            time_r = Rect::from_center_size(Pos2::new(scr.min.x + 10.0 + tw / 2.0, mid), Vec2::new(tw + 8.0, 34.0 * s));
            if self.mini_viz {
                let vr = Rect::from_min_max(Pos2::new(scr.min.x + 22.0 + tw, mid - 16.0 * s), Pos2::new(scr.max.x - 10.0, mid + 14.0 * s));
                if vr.width() > 24.0 {
                    mini_viz(&p, vr, &bands, amber(), true);
                }
                self.viz_tick(ui.ctx());
            }
            lcd_text(&p, Pos2::new(scr.min.x + 10.0, scr.max.y - 11.0 * s), Align::Min, &self.deck_info(), 1.0, w);
        } else {
            let tx = cover.max.x + 12.0;
            let room = top.max.x - tx;
            if let Some(tr) = &tr {
                crate::row_text(&p, Pos2::new(tx, top.min.y + 9.0 * s), Align::Min, &tr.title, room, fg());
                crate::row_text(&p, Pos2::new(tx, top.min.y + 27.0 * s), Align::Min, &tr.artists_text(), room, fg2());
            }
            let big = (top.height() / s / 34.0).clamp(1.8, 3.2);
            let ty = top.max.y - top.height() * 0.28;
            let tw = font::keep_caps(|| ptext(&p, Pos2::new(tx, ty), Align::Min, &self.deck_time(), big, fg()));
            time_r = Rect::from_min_size(Pos2::new(tx, ty - 16.0 * s), Vec2::new(tw + 8.0, 32.0 * s));
            if self.mini_viz {
                mini_viz(&p, Rect::from_min_max(Pos2::new(tx + tw + 12.0, top.min.y + 42.0 * s), Pos2::new(top.max.x, top.max.y - 6.0)), &bands, if style() == STYLE_AMBIENT { fg().gamma_multiply(0.8) } else { pal().accent }, false);
                self.viz_tick(ui.ctx());
            }
        }
        // the time: a click switches elapsed and what's left
        if ui.interact(time_r, id.with("time"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Elapsed / remaining").clicked() {
            self.time_left = !self.time_left;
            self.dirty = true;
        }
        // (the plain decks: the stream's details under the cover, at the right; the times along the bar under them, clear
        // of them at any text size)
        let mut y0 = top.max.y + if hw { 8.0 } else { 18.0 };
        if !hw {
            let info = self.deck_info();
            if !info.is_empty() {
                let ih = font::text_px(1.25);
                let iy = top.max.y + 4.0 + ih / 2.0;
                ptext_fit(&p, Pos2::new(r.max.x, iy), Align::Max, &info, 1.25, r.width(), fg2());
                // (the bar's times are centred 8 below its top)
                y0 = y0.max(iy + ih / 2.0 + 6.0 + font::text_px(1.4) / 2.0 - 8.0);
            }
        }
        // ---- how far in
        let prog = Rect::from_min_max(Pos2::new(r.min.x - 4.0, y0), Pos2::new(r.max.x + 4.0, y0 + 16.0 * s));
        self.progress(ui, prog, acts, if hw { amber() } else { fg() });
        // ---- the transport, open files at its end
        let ky = prog.max.y + 8.0 * s;
        let playing = self.cur.is_some() && !self.stopped && !self.paused;
        let keys = [
            ("prev", Action::Prev, "Previous", false),
            ("play", Action::PlayBtn, "Play", playing),
            ("pause", Action::PauseBtn, "Pause", self.paused),
            ("stop", Action::StopBtn, "Stop", false),
            ("skip", Action::Next, "Next", false),
            ("folder", Action::AddFiles, "Open files", false),
        ];
        let kh = 34.0 * s;
        let kw = (r.width() - 5.0 * 6.0) / 6.0;
        for (n, (icon, a, tip, lit)) in keys.into_iter().enumerate() {
            let kr = Rect::from_min_size(Pos2::new(r.min.x + n as f32 * (kw + 6.0), ky), Vec2::new(kw, kh));
            let h = ui.interact(kr, id.with(("key", n)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
            if hw {
                let down = h.is_pointer_button_down_on();
                crate::key_face_on(&p, kr, down, h.hovered(), lit);
                let c = kr.center() + Vec2::new(0.0, if down { 1.0 } else { 0.0 });
                crate::vicon::named(&p, Rect::from_center_size(c, Vec2::splat(15.0 * s)), icon, crate::key_ink(lit));
            } else {
                crate::quiet_h(&p, kr, h.is_pointer_button_down_on(), h.hovered());
                if n == 1 {
                    p.circle_filled(kr.center(), kh / 2.0, fg());
                    crate::vicon::named(&p, Rect::from_center_size(kr.center() + Vec2::new(1.0, 0.0), Vec2::splat(12.0 * s)), icon, on_fg());
                } else {
                    crate::vicon::named(&p, Rect::from_center_size(kr.center(), Vec2::splat(15.0 * s)), icon, if lit || h.hovered() { fg() } else { fg2() });
                }
            }
            if h.clicked() {
                acts.push(a);
            }
        }
        // ---- shuffle, repeat, the equalizer, the playlist; the volume and the balance
        let ry = ky + kh + 12.0 * s;
        let rep_on = !matches!(self.repeat, crate::Repeat::Off);
        let toggles = [
            ("shuffle", self.shuffle, "Shuffle"),
            ("loop", rep_on, "Repeat"),
            ("sliders", self.show_eq, "Equalizer"),
            ("list", !self.queue_hidden, "Playlist"),
        ];
        let tw = 30.0 * s;
        for (n, (icon, on, tip)) in toggles.into_iter().enumerate() {
            let tr = Rect::from_min_size(Pos2::new(r.min.x + n as f32 * (tw + 8.0), ry), Vec2::splat(tw));
            if hw {
                let h = ui.interact(tr, id.with(("tg", n)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
                let down = h.is_pointer_button_down_on();
                crate::key_face_on(&p, tr, down, h.hovered(), on);
                crate::vicon::named(&p, tr.shrink(6.0 * s).translate(Vec2::new(0.0, if down { 1.0 } else { 0.0 })), icon, crate::key_ink(on));
                if h.clicked() {
                    self.deck_toggle(n, acts);
                }
            } else if self.small_toggle(ui, tr, icon, on, tip, id.with(("tg", n))) {
                self.deck_toggle(n, acts);
            }
        }
        // the volume, then the balance (a click on its middle: centred)
        let sx = r.min.x + 4.0 * (tw + 8.0) + 8.0;
        let vol = Rect::from_min_max(Pos2::new(sx, ry - 1.0), Pos2::new(r.max.x, ry + 13.0 * s));
        self.volume_line(ui, vol, acts);
        let bal = Rect::from_min_max(Pos2::new(sx + 26.0 * s, ry + 18.0 * s), Pos2::new(r.max.x, ry + 30.0 * s));
        if hw {
            printed(&p, Pos2::new(sx, bal.center().y), "Bal");
        } else {
            ptext(&p, Pos2::new(sx, bal.center().y), Align::Min, "L R", 1.1, fg2());
        }
        let line = Rect::from_min_max(Pos2::new(bal.min.x, bal.center().y - 2.0), Pos2::new(bal.max.x, bal.center().y + 2.0));
        let h = ui.interact(bal, id.with("bal"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Balance · Double-click to center");
        if h.double_clicked() {
            acts.push(Action::Balance(0));
        } else if h.clicked() || h.dragged() {
            if let Some(pp) = h.interact_pointer_pos() {
                let b = (((pp.x - line.min.x) / line.width()).clamp(0.0, 1.0) * 200.0 - 100.0) as i32;
                acts.push(Action::Balance(if b.abs() < 8 { 0 } else { b }));
            }
        }
        p.rect_filled(line, 2.0, wash(75));
        p.rect_filled(Rect::from_center_size(line.center(), Vec2::new(1.0, 10.0)), 0.0, fg2());
        let bx = line.min.x + line.width() * ((self.balance as f32 + 100.0) / 200.0);
        p.circle_filled(Pos2::new(bx, line.center().y), 5.0, if hw { amber() } else { fg() });
    }

    /// One of the deck's switches: shuffle, repeat, the equalizer, the playlist (Streaming: its queue sliding in).
    fn deck_toggle(&mut self, n: usize, acts: &mut Vec<Action>) {
        match n {
            0 => acts.push(Action::Shuffle),
            1 => acts.push(Action::Repeat),
            2 => acts.push(Action::ToggleEq),
            _ if style() == STYLE_STREAMING => {
                self.stream_queue = !self.stream_queue;
                self.dirty = true;
            }
            _ => {
                self.queue_hidden = !self.queue_hidden;
                self.dirty = true;
            }
        }
    }

    /// Streaming's column under the deck: the next few songs in roomy rows (a click plays one), and the way to the
    /// whole queue, which slides in over the page.
    fn next_up(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        ptext(&p, Pos2::new(r.min.x + 2.0, r.min.y + 10.0), Align::Min, "Next Up", 2.1, fg());
        let qb = Rect::from_min_size(Pos2::new(r.max.x - 110.0, r.min.y - 2.0), Vec2::new(110.0, 24.0));
        let h = ui.interact(qb, ui.id().with("next_up_queue"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        crate::quiet_h(&p, qb, h.is_pointer_button_down_on(), h.hovered());
        ptext(&p, Pos2::new(qb.max.x - 6.0, qb.center().y), Align::Max, "Open Queue", 1.5, if h.hovered() { fg() } else { fg2() });
        if h.clicked() {
            self.stream_queue = !self.stream_queue;
        }
        let queue = self.queue.clone();
        let from = self.cur.map_or(0, |c| c + 1);
        let rh = 54.0;
        let mut y = r.min.y + 30.0;
        if from >= queue.len() {
            ptext(&p, Pos2::new(r.min.x + 2.0, y + 14.0), Align::Min, "End of the queue.", 1.5, fg2());
            return;
        }
        // (a row partly in view is drawn, cut at the panel's foot, so none pops in as the window grows)
        let clip = Rect::from_min_max(Pos2::new(r.min.x, r.min.y + 26.0), r.max);
        let p = p.with_clip_rect(clip);
        let mut rects: Vec<(usize, Rect)> = Vec::new();
        for (i, tr) in queue.iter().enumerate().skip(from).take(5) {
            if y >= r.max.y {
                break;
            }
            let row = Rect::from_min_size(Pos2::new(r.min.x, y), Vec2::new(r.width(), rh - 4.0));
            rects.push((i, row));
            let hr = ui.interact(row.intersect(clip), ui.id().with(("next_up", i)), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand);
            // (a held left button or the middle one moves it, as in the playlist)
            if hr.drag_started_by(egui::PointerButton::Primary) || hr.drag_started_by(egui::PointerButton::Middle) {
                self.q_drag = Some(i);
            }
            if hr.hovered() {
                p.rect_filled(row, 6.0, wash(16));
            }
            let cs = row.height() - 8.0;
            let cr = Rect::from_min_size(Pos2::new(row.min.x + 4.0, row.center().y - cs / 2.0), Vec2::splat(cs));
            let url = if tr.cover.is_empty() { String::new() } else { format!("thumb:{}", crate::api::cover_url(&tr.cover, 80)) };
            {
                let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(row));
                cu.set_clip_rect(clip.intersect(ui.clip_rect()));
                crate::row_cover(&cu, cr, &url);
            }
            let tx = cr.max.x + 12.0;
            let dw = ptext(&p, Pos2::new(row.max.x - 6.0, row.center().y), Align::Max, &fmt_time(tr.duration), 1.6, fg2());
            let room = row.max.x - dw - 18.0 - tx;
            crate::row_text(&p, Pos2::new(tx, row.center().y - 9.0), Align::Min, &tr.title, room, fg());
            crate::row_text(&p, Pos2::new(tx, row.center().y + 10.0), Align::Min, &tr.artists_text(), room, fg2());
            if hr.clicked() {
                acts.push(Action::PlayIndex(i));
            }
            y += rh;
        }
        // a song being moved: a line where it will land, moved on release
        if let Some(from) = self.q_drag.filter(|f| rects.iter().any(|(i, _)| i == f)) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let released = ui.input(|i| i.pointer.any_released());
            if let Some(pp) = ui.input(|i| i.pointer.hover_pos()) {
                let ins = rects
                    .iter()
                    .find(|(_, rc)| pp.y < rc.center().y)
                    .map(|(i, rc)| (*i, rc.min.y))
                    .or_else(|| rects.last().map(|(i, rc)| (*i + 1, rc.max.y)));
                if let Some((ins_i, ly)) = ins {
                    p.rect_filled(Rect::from_min_size(Pos2::new(r.min.x, ly - 1.0), Vec2::new(r.width(), 2.0)), 1.0, pal().accent);
                    if released {
                        acts.push(Action::MoveQueue(from, ins_i));
                    }
                }
            }
            if released || !ui.input(|i| i.pointer.any_down()) {
                self.q_drag = None;
            }
        }
    }
}

impl App {
    /// While Now Playing is open: its blurred cover made from the cover playing, away from the frame (the cover's small
    /// picture squeezed to 24 by 24, then softened twice, drawn stretched with smooth filtering).
    pub(crate) fn np_blur_tick(&mut self) {
        if !self.art_view {
            return;
        }
        let cover = self.cur_track().map(|t| t.cover).unwrap_or_default();
        if cover.is_empty() {
            return;
        }
        let url = format!("thumb:{}", crate::api::cover_url(&cover, 80));
        if url == self.np_blur_for {
            return;
        }
        self.np_blur_for = url.clone();
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let Some(img) = crate::picture(&api, &url) else { return };
            let _ = tx.send(Msg::NpBlur(url, blur_small(&img)));
            ctx.request_repaint();
        });
    }
}

/// A picture squeezed to 24 by 24 (each a box average) and softened with two passes of a 3 by 3 blur.
fn blur_small(img: &egui::ColorImage) -> egui::ColorImage {
    const N: usize = 24;
    let [w, h] = img.size;
    let mut px = vec![[0f32; 3]; N * N];
    for y in 0..N {
        for x in 0..N {
            let (x0, x1) = (x * w / N, ((x + 1) * w / N).max(x * w / N + 1).min(w));
            let (y0, y1) = (y * h / N, ((y + 1) * h / N).max(y * h / N + 1).min(h));
            let mut s = [0f32; 3];
            let mut n = 0f32;
            for yy in y0..y1 {
                for xx in x0..x1 {
                    let c = img.pixels[yy * w + xx];
                    s[0] += c.r() as f32;
                    s[1] += c.g() as f32;
                    s[2] += c.b() as f32;
                    n += 1.0;
                }
            }
            px[y * N + x] = [s[0] / n.max(1.0), s[1] / n.max(1.0), s[2] / n.max(1.0)];
        }
    }
    for _ in 0..2 {
        let src = px.clone();
        for y in 0..N {
            for x in 0..N {
                let mut s = [0f32; 3];
                let mut n = 0f32;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let (xx, yy) = (x as i32 + dx, y as i32 + dy);
                        if xx >= 0 && yy >= 0 && (xx as usize) < N && (yy as usize) < N {
                            let c = src[yy as usize * N + xx as usize];
                            for k in 0..3 {
                                s[k] += c[k];
                            }
                            n += 1.0;
                        }
                    }
                }
                px[y * N + x] = [s[0] / n, s[1] / n, s[2] / n];
            }
        }
    }
    let rgba: Vec<u8> = px.iter().flat_map(|c| [c[0] as u8, c[1] as u8, c[2] as u8, 255]).collect();
    egui::ColorImage::from_rgba_unmultiplied([N, N], &rgba)
}
