//! Everything drawn for the practice extras: the FILES / YOUTUBE / TUNES / DIARY lists, the
//! practice panel, the lead sheet and the little text-field widget they share.

use crate::api::Track;
use crate::extras::{Knob, CHAN_NAMES};
use crate::font::{ptext, ptext_fit, spx, text_w};
use crate::sources;
use crate::stems;
use crate::store::{self, Ext};
use crate::tools::{F_LOOK, F_LOOP_A, F_LOOP_B, F_SPEED, F_TR_LOOPS, F_TR_STEP, F_TUNE_IREAL};
use crate::{
    bh, cache, chip, col_header, col_on, fill_rect, fmt_t, fmt_time, inset, lcd_box, list_row, menu_item, outline, pal, para,
    play_buttons, retro_btn, retro_btn_w, section_header, tab_row, table_header, title_line, track_cells, window_deco, Action,
    App, Ed, RowState, Sec, Tip, PRACTICE,
};
use eframe::egui::{self, Align, Pos2, Rect, Sense, Vec2};
use std::sync::atomic::Ordering;

/// "45 SEC", "12 MIN", "1 H 05 MIN"
/// 0 = a file, 2 = SoundCloud, 1 = YouTube.
fn heart_group(e: &Ext) -> u8 {
    if e.kind == "file" {
        0
    } else if e.src.contains("soundcloud.com") {
        2
    } else {
        1
    }
}

fn mins_text(secs: u32) -> String {
    match secs {
        0..=59 => format!("{} SEC", secs),
        60..=3599 => format!("{} MIN", secs / 60),
        _ => format!("{} H {:02} MIN", secs / 3600, secs % 3600 / 60),
    }
}

fn src_is_link(s: &str) -> bool {
    s.starts_with("http")
}

// text-field ids (only one field has the keyboard at a time)
const F_NEW_TUNE: u32 = 1;
const F_T_NAME: u32 = 2;
const F_T_KEY: u32 = 3;
const F_T_NOTES: u32 = 4;
const F_V_NOTE: u32 = 5;
const F_YT: u32 = 6;
const F_TUNE: u32 = 7;
const F_MINS: u32 = 8;
const F_BPM: u32 = 9;
const F_NOTE: u32 = 10;
const F_SEC: u32 = 11;
pub(crate) const F_CHART: u32 = 12;
pub(crate) const F_IREAL: u32 = 13;
const F_SC: u32 = 14;
const F_NEW_PL: u32 = 50;

// -------------------------------------------------------------- text field
pub struct FieldOut {
    pub changed: bool,
    pub enter: bool,
}

/// Wrapped line ranges (in characters) of a multi-line text.
fn layout_lines(chars: &[char], px: f32, w: f32) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    let mut last_space: Option<usize> = None;
    while i < chars.len() {
        if chars[i] == '\n' {
            lines.push((start, i));
            i += 1;
            start = i;
            last_space = None;
            continue;
        }
        let seg: String = chars[start..=i].iter().collect();
        if text_w(&seg, px) > w && i > start {
            let brk = match last_space {
                Some(sp) if sp > start => sp + 1,
                _ => i,
            };
            lines.push((start, brk));
            start = brk;
            i = brk;
            last_space = None;
            continue;
        }
        if chars[i] == ' ' {
            last_space = Some(i);
        }
        i += 1;
    }
    lines.push((start, chars.len()));
    lines
}

fn caret_x(chars: &[char], s: usize, cur: usize, px: f32) -> f32 {
    if cur <= s {
        return 0.0;
    }
    let pre: String = chars[s..cur.min(chars.len())].iter().collect();
    text_w(&pre, px) + spx(px)
}

fn line_of(lines: &[(usize, usize)], cur: usize) -> usize {
    let mut li = 0;
    for (k, (s, _)) in lines.iter().enumerate() {
        if *s <= cur {
            li = k;
        }
    }
    li
}

/// One text field drawn in the pixel font (single line, or several wrapped lines).
/// Edits are written straight into `text`.
pub fn field(ui: &mut egui::Ui, ed: &mut Ed, id: u32, text: &mut String, rect: Rect, hint: &str, multi: bool) -> FieldOut {
    let px = 2.0;
    let lh = 7.0 * spx(px) + 6.0;
    let inner = rect.shrink2(Vec2::new(8.0, 3.0));
    let mut out = FieldOut { changed: false, enter: false };
    let resp = ui.interact(rect, ui.id().with(("fld", id)), Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
    }
    let mut chars: Vec<char> = text.chars().collect();
    let focused = ed.id == id;
    ed.cur = if focused { ed.cur.min(chars.len()) } else { ed.cur };

    // scroll offsets that keep the caret visible (only while it has the keyboard)
    let offsets = |chars: &[char], cur: usize, on: bool| -> (Vec<(usize, usize)>, f32, f32) {
        let lines = if multi { layout_lines(chars, px, inner.width()) } else { vec![(0, chars.len())] };
        if !on {
            return (lines, 0.0, 0.0);
        }
        let li = line_of(&lines, cur);
        if multi {
            let voff = ((li as f32 + 1.0) * lh - inner.height()).max(0.0);
            (lines, 0.0, voff)
        } else {
            let cx = caret_x(chars, 0, cur, px);
            let hoff = (cx - (inner.width() - 6.0)).max(0.0);
            (lines, hoff, 0.0)
        }
    };

    if resp.clicked() {
        let (lines, hoff, voff) = offsets(&chars, ed.cur, focused);
        ed.id = id;
        if let Some(pp) = resp.interact_pointer_pos() {
            let li = if multi { (((pp.y - inner.min.y + voff) / lh).floor().max(0.0) as usize).min(lines.len() - 1) } else { 0 };
            let (s, e) = lines[li];
            let x = pp.x - inner.min.x + hoff;
            let mut best = s;
            let mut best_d = f32::MAX;
            for k in s..=e {
                let w = caret_x(&chars, s, k, px);
                let d = (w - x).abs();
                if d < best_d {
                    best_d = d;
                    best = k;
                }
            }
            ed.cur = best;
        } else {
            ed.cur = chars.len();
        }
    } else if focused && ui.input(|i| i.pointer.primary_pressed()) && !resp.hovered() {
        ed.id = 0;
    }

    if ed.id == id {
        let events = ui.input(|i| i.events.clone());
        let mut cur = ed.cur.min(chars.len());
        let mut edited = false;
        for ev in events {
            match ev {
                egui::Event::Text(t) | egui::Event::Paste(t) => {
                    for c in t.chars() {
                        if c == '\n' || c == '\r' {
                            if multi {
                                chars.insert(cur, '\n');
                                cur += 1;
                                edited = true;
                            }
                        } else if !c.is_control() {
                            chars.insert(cur, c);
                            cur += 1;
                            edited = true;
                        }
                    }
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } => match key {
                    egui::Key::Backspace => {
                        if modifiers.ctrl {
                            while cur > 0 && chars[cur - 1] == ' ' {
                                chars.remove(cur - 1);
                                cur -= 1;
                                edited = true;
                            }
                            while cur > 0 && chars[cur - 1] != ' ' && chars[cur - 1] != '\n' {
                                chars.remove(cur - 1);
                                cur -= 1;
                                edited = true;
                            }
                        } else if cur > 0 {
                            chars.remove(cur - 1);
                            cur -= 1;
                            edited = true;
                        }
                    }
                    egui::Key::Delete => {
                        if cur < chars.len() {
                            chars.remove(cur);
                            edited = true;
                        }
                    }
                    egui::Key::ArrowLeft => cur = cur.saturating_sub(1),
                    egui::Key::ArrowRight => cur = (cur + 1).min(chars.len()),
                    egui::Key::ArrowUp | egui::Key::ArrowDown if multi => {
                        let lines = layout_lines(&chars, px, inner.width());
                        let li = line_of(&lines, cur);
                        let col = cur - lines[li].0;
                        let to =
                            if key == egui::Key::ArrowUp { li.checked_sub(1) } else { Some(li + 1).filter(|n| *n < lines.len()) };
                        if let Some(t) = to {
                            let (s, e) = lines[t];
                            cur = (s + col).min(e);
                        }
                    }
                    egui::Key::Home => {
                        cur = if multi {
                            layout_lines(&chars, px, inner.width())[line_of(&layout_lines(&chars, px, inner.width()), cur)].0
                        } else {
                            0
                        };
                    }
                    egui::Key::End => {
                        cur = if multi {
                            let l = layout_lines(&chars, px, inner.width());
                            l[line_of(&l, cur)].1
                        } else {
                            chars.len()
                        };
                    }
                    egui::Key::Enter => {
                        if multi {
                            chars.insert(cur, '\n');
                            cur += 1;
                            edited = true;
                        } else {
                            out.enter = true;
                        }
                    }
                    egui::Key::Escape => ed.id = 0,
                    _ => {}
                },
                _ => {}
            }
        }
        ed.cur = cur;
        if edited {
            *text = chars.iter().collect();
            out.changed = true;
        }
    }

    // ---- draw
    let focused = ed.id == id;
    let (lines, hoff, voff) = offsets(&chars, ed.cur, focused);
    let p = ui.painter();
    inset(p, rect, pal().lcd);
    if focused {
        outline(p, rect, 1.0, pal().ink2);
    }
    let cp = p.with_clip_rect(inner);
    if chars.is_empty() {
        ptext(&cp, Pos2::new(inner.min.x, inner.min.y + lh / 2.0), Align::Min, hint, px, pal().dim);
    }
    for (li, (s, e)) in lines.iter().enumerate() {
        let y = inner.min.y - voff + li as f32 * lh + lh / 2.0;
        if y < inner.min.y - lh || y > inner.max.y + lh {
            continue;
        }
        let line: String = chars[*s..(*e).min(chars.len())].iter().collect();
        let cy = if multi { y } else { rect.center().y };
        ptext(&cp, Pos2::new(inner.min.x - hoff, cy), Align::Min, &line, px, pal().ink);
    }
    if focused && (ui.input(|i| i.time) * 2.0) as i64 % 2 == 0 {
        let li = line_of(&lines, ed.cur);
        let (s, _) = lines[li];
        let x = inner.min.x - hoff + caret_x(&chars, s, ed.cur, px);
        let cy = if multi { inner.min.y - voff + li as f32 * lh + lh / 2.0 } else { rect.center().y };
        fill_rect(&cp, Rect::from_min_size(Pos2::new(x, cy - 8.0), Vec2::new(spx(2.0), 16.0)), pal().ink);
    }
    out
}

/// "LABEL  [ text field ]" on one row (or a taller multi-line box).
pub(crate) fn field_row(
    ui: &mut egui::Ui,
    ed: &mut Ed,
    id: u32,
    label: &str,
    text: &mut String,
    hint: &str,
    tall: Option<f32>,
) -> FieldOut {
    let h = tall.unwrap_or(bh());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let ly = if tall.is_some() { rect.min.y + 13.0 } else { rect.center().y };
    ptext(ui.painter(), Pos2::new(rect.min.x + 4.0, ly), Align::Min, label, 2.0, pal().ink2);
    let fr = Rect::from_min_max(Pos2::new(rect.min.x + 76.0, rect.min.y), rect.max);
    field(ui, ed, id, text, fr, hint, tall.is_some())
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

// little 7x7 pictures for the stems: drums, bass (a woofer), other (keys), vocals (a mic)
const STEM_ICONS: [[&str; 7]; 4] = [
    [".#####.", "#######", "#.....#", "#.....#", "#.....#", ".#####.", "#.....#"],
    [".#####.", "#.....#", "#.###.#", "#.###.#", "#.###.#", "#.....#", ".#####."],
    ["#######", "#.#.#.#", "#.#.#.#", "#.#.#.#", "#.#.#.#", "#.#.#.#", "#######"],
    [".#####.", "#######", "#######", ".#####.", "...#...", "...#...", ".#####."],
];

/// Icon button for one stem: pressed in (bright) while the stem is playing.
fn stem_btn(ui: &mut egui::Ui, i: usize, on: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(44.0, bh()), Sense::click());
    crate::raised_h(ui.painter(), rect, resp.is_pointer_button_down_on() || on, resp.hovered() && !on);
    let dy = if on { 1.0 } else { 0.0 };
    let o = Pos2::new((rect.center().x - 7.0).round(), (rect.center().y - 7.0 + dy).round());
    crate::pixmap(ui.painter(), o, 2.0, &STEM_ICONS[i][..], if on { pal().ink } else { pal().dim });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn stars_text(n: u8) -> String {
    let n = n.min(5) as usize;
    format!("{}{}", "*".repeat(n), ".".repeat(5 - n))
}

/// The lighter second row under a tab row: plain words, the open one underlined. Squeezes to fit a narrow window.
fn sub_tabs(ui: &mut egui::Ui, items: &[(Sec, &str)], cur: usize) -> Option<usize> {
    let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::hover());
    let pad = 18.0;
    let total: f32 = items.iter().map(|s| if s.1 == "|" { 14.0 } else { text_w(s.1, 2.0) + pad }).sum();
    let shrink = ((row.width() - 8.0) / total).min(1.0);
    let (mut x, mut hit) = (row.min.x + 4.0, None);
    for (i, (_, name)) in items.iter().enumerate() {
        // "|" is a bar between groups of tabs, not a tab
        if *name == "|" {
            let bx = x + 6.0 * shrink;
            fill_rect(
                ui.painter(),
                Rect::from_min_max(Pos2::new(bx, row.min.y + 4.0), Pos2::new(bx + 2.0, row.max.y - 4.0)),
                pal().ink2,
            );
            x += 14.0 * shrink;
            continue;
        }
        let w = (text_w(name, 2.0) + pad) * shrink;
        let r = Rect::from_min_max(Pos2::new(x, row.min.y), Pos2::new(x + w, row.max.y));
        let resp =
            ui.interact(r, ui.id().with(("subtab", i, *name)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let on = i == cur;
        let col = if on || resp.hovered() { pal().ink } else { pal().ink2 };
        crate::ui_text(ui.painter(), r.center() - Vec2::new(0.0, 1.0), Align::Center, name, 2.0, w - 4.0, col);
        if on {
            let u = Rect::from_min_max(Pos2::new(r.min.x + 4.0, r.max.y - 3.0), Pos2::new(r.max.x - 4.0, r.max.y - 1.0));
            fill_rect(ui.painter(), u, pal().ink);
        }
        if resp.clicked() {
            hit = Some(i);
        }
        x += w;
    }
    hit
}

/// The bookmark colours: bright enough to read a dark number on, apart enough to tell apart.
const MARK_COLS: [[u8; 3]; 8] = [
    [235, 80, 80],
    [245, 150, 50],
    [240, 215, 70],
    [110, 210, 100],
    [80, 200, 210],
    [90, 140, 245],
    [175, 120, 240],
    [240, 120, 190],
];

/// The bookmark colours as a row of swatches in a menu, the current one outlined; the one clicked.
fn mark_swatches(ui: &mut egui::Ui, cur: Option<[u8; 3]>) -> Option<[u8; 3]> {
    let mut out = None;
    ui.horizontal(|ui| {
        for c in MARK_COLS {
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
            ui.painter().rect_filled(r.shrink(2.0), 2.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
            if cur == Some(c) || resp.hovered() {
                ui.painter().rect_stroke(r, 2.0, egui::Stroke::new(1.5_f32, pal().ink));
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                out = Some(c);
            }
        }
    });
    // any colour: a pixel colour wheel (round the wheel the hue, out from the middle the strength)
    let cells = 15;
    let cell = 8.0_f32;
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(cells as f32 * cell), Sense::click());
    let c0 = r.center();
    let rad = cells as f32 * cell / 2.0;
    let col_at = |p: Pos2| -> Option<[u8; 3]> {
        let d = p - c0;
        let dist = d.length() / rad;
        if dist > 1.0 {
            return None;
        }
        let hue = (d.y.atan2(d.x) / std::f32::consts::TAU).rem_euclid(1.0);
        let c = egui::Color32::from(egui::ecolor::Hsva::new(hue, (dist * 1.15).min(1.0) * 0.85, 1.0, 1.0));
        Some([c.r(), c.g(), c.b()])
    };
    for y in 0..cells {
        for x in 0..cells {
            let cr = Rect::from_min_size(r.min + Vec2::new(x as f32 * cell, y as f32 * cell), Vec2::splat(cell));
            if let Some(c) = col_at(cr.center()) {
                ui.painter().rect_filled(cr, 0.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
            }
        }
    }
    if let Some(pp) =
        resp.on_hover_cursor(egui::CursorIcon::Crosshair).interact_pointer_pos().filter(|_| ui.input(|i| i.pointer.any_click()))
    {
        // the cell's own colour, as drawn
        let cx = ((pp.x - r.min.x) / cell).floor() * cell + r.min.x + cell / 2.0;
        let cy = ((pp.y - r.min.y) / cell).floor() * cell + r.min.y + cell / 2.0;
        if let Some(c) = col_at(Pos2::new(cx, cy)) {
            out = Some(c);
        }
    }
    out
}

impl App {
    // ------------------------------------------------------------ switchers
    /// Two levels: MUSIC (the sources and your lists) and PRACTICE (tunes, diary) on top, that group's lists under it.
    /// Each group comes back to the list you last had open in it. The plain build has only MUSIC, so only its row shows.
    pub(crate) fn section_bar(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        const PRACTICE_SECS: [(Sec, &str); 6] = [
            (Sec::Tunes, "TUNES"),
            (Sec::Diary, "DIARY"),
            (Sec::Loops, "LOOPS"),
            (Sec::Reading, "READING"),
            (Sec::Chords, "CHORDS"),
            (Sec::Lines, "LINES"),
        ];
        // the BACKLINE tab covers its own page and the pages opened from it
        let side = self.show_tl || self.show_winamp || self.show_log || self.show_cache;
        let in_practice = PRACTICE_SECS.iter().any(|s| s.0 == self.sec);
        let tabs: &[&str] = if PRACTICE { &["MUSIC", "PRACTICE", "BACKLINE"] } else { &["MUSIC", "BACKLINE"] };
        let last = tabs.len() - 1;
        let cur = if side { last } else { (PRACTICE && in_practice) as usize };
        let top = ui.cursor().min.y;
        if let Some(i) = tab_row(ui, tabs, cur) {
            if i == last {
                acts.push(Action::BacklinePage);
            } else if i != cur {
                let to_practice = PRACTICE && i == 1;
                let music: Vec<Sec> = [Sec::Mine, Sec::Lists, Sec::Tidal, Sec::Files, Sec::Sc, Sec::Yt]
                    .into_iter()
                    .filter(|s| *s != Sec::Tidal || !self.offline)
                    .collect();
                let back = if to_practice { self.last_practice } else { self.last_music };
                let ok = if to_practice { PRACTICE_SECS.iter().any(|s| s.0 == back) } else { music.contains(&back) };
                let to = if ok {
                    back
                } else if to_practice {
                    Sec::Tunes
                } else {
                    music[0]
                };
                acts.push(Action::Section(to));
            }
        }
        let r = ui.min_rect();
        crate::tour::mark("APPMENU", Rect::from_min_max(Pos2::new(r.min.x, top), Pos2::new(r.max.x, ui.cursor().min.y)));
        if side {
            // the BACKLINE tab's own pages
            let items = [(Sec::Tidal, "SKINS & SETTINGS"), (Sec::Tidal, "WINAMP SKINS"), (Sec::Tidal, "LOG")];
            let cur = if self.show_winamp {
                1
            } else if self.show_log {
                2
            } else {
                0
            };
            if let Some(i) = sub_tabs(ui, &items, cur) {
                if i != cur {
                    acts.push(match i {
                        0 => Action::BacklinePage,
                        1 => Action::WaOpen,
                        _ => Action::ShowLog,
                    });
                }
            }
            return;
        }
        if PRACTICE && in_practice {
            let cur = PRACTICE_SECS.iter().position(|s| s.0 == self.sec).unwrap_or(0);
            if let Some(i) = sub_tabs(ui, &PRACTICE_SECS, cur) {
                acts.push(Action::Section(PRACTICE_SECS[i].0));
            }
            return;
        }
        // everything you keep (MY TRACKS, LISTS), then each source on its own
        // each source, then a bar, then everything you keep (MY TRACKS, LISTS)
        let mut items: Vec<(Sec, &str)> = Vec::new();
        if !self.offline {
            items.push((Sec::Tidal, "TIDAL"));
        }
        items.extend([
            (Sec::Files, "FILES"),
            (Sec::Sc, "SOUNDCLOUD"),
            (Sec::Yt, "YT"),
            (Sec::Mine, "|"),
            (Sec::Mine, "MY TRACKS"),
            (Sec::Lists, "LISTS"),
        ]);
        let cur = items.iter().position(|s| s.0 == self.sec && s.1 != "|").unwrap_or(0);
        if let Some(i) = sub_tabs(ui, &items, cur) {
            match items[i].0 {
                // TIDAL goes back to the Tidal page you had
                Sec::Tidal if self.page.is_none() => acts.push(Action::Home),
                s => acts.push(Action::Section(s)),
            }
        }
        // Tidal's own space: its home and your Tidal library, nothing from other sources mixed in
        if self.sec == Sec::Tidal {
            let on_library = self.page.as_ref().is_some_and(|p| p.library);
            let sub = [(Sec::Tidal, "HOME"), (Sec::Tidal, "MY LIBRARY")];
            if let Some(i) = sub_tabs(ui, &sub, on_library as usize) {
                acts.push(if i == 1 { Action::Library } else { Action::Home });
            }
        }
    }

    /// MY TRACKS: every song you liked, from every source (your Tidal likes, and the files, SoundCloud and YouTube
    /// songs you hearted), with a SOURCES row to leave any of them out.
    pub(crate) fn mine_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "MY TRACKS", 3.0, pal().ink);
        title_line(ui, "every song you liked, from every source", 2.0, pal().ink2);
        ui.horizontal_wrapped(|ui| {
            label(ui, "SOURCES", 74.0);
            for (key, name) in crate::SOURCES {
                let on = !self.mine_off.iter().any(|k| k == key);
                if crate::check_box(ui, name, on).clicked() {
                    if on {
                        self.mine_off.push(key.to_string());
                    } else {
                        self.mine_off.retain(|k| k != key);
                    }
                    self.dirty = true;
                }
            }
        });
        let shows = |key: &str| !self.mine_off.iter().any(|k| k == key);
        let mut tracks: Vec<Track> = self.store.hearts.iter().filter(|e| shows(&e.kind)).map(|e| e.to_track()).collect();
        let mut waiting = false;
        if shows("tidal") && !self.offline {
            match &self.lib_page {
                Some(p) => tracks.extend(p.tracks.iter().cloned()),
                None => waiting = true,
            }
        }
        ui.add_space(4.0);
        if waiting {
            dim_line(ui, "YOUR TIDAL LIKES SHOW ONCE TIDAL > MY LIBRARY HAS LOADED", 1.0, pal().dim);
        }
        if tracks.is_empty() {
            para(ui, "Nothing here yet. Click the heart on any song - Tidal, your files, SoundCloud or YouTube - and it shows up here.", pal().ink2);
            return;
        }
        title_line(ui, &format!("{} songs", tracks.len()), 2.0, pal().ink2);
        crate::play_buttons(ui, &tracks, acts, false);
        ui.add_space(4.0);
        let playing = self.cur_track().map(|t| t.id);
        crate::tracks_list(ui, &tracks, playing, &self.liked, acts);
        ui.add_space(20.0);
    }

    /// "Pick up where you left off" in one press.
    pub(crate) fn continue_bar(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if !PRACTICE || !(self.cur.is_none() || self.stopped) {
            return;
        }
        let Some(l) = &self.store.last else { return };
        let txt = format!(
            "CONTINUE: {} - {}  @ {}  {}%",
            l.version.artist,
            l.version.title,
            fmt_time(l.pos),
            if l.speed == 0 { 100 } else { l.speed }
        );
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click());
        crate::raised_h(ui.painter(), rect, resp.is_pointer_button_down_on(), resp.hovered());
        ptext_fit(ui.painter(), rect.center(), Align::Center, &txt, 2.0, rect.width() - 16.0, pal().ink);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            acts.push(Action::Continue);
        }
    }

    fn right_tabs(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let inner = ui.max_rect();
        let chip_h = 16.0;
        let ty = inner.min.y - 18.0 - chip_h / 2.0;
        let mut right = inner.max.x - 8.0;
        let mut place = |w: f32| {
            let r = Rect::from_min_size(Pos2::new(right - w, ty), Vec2::new(w, chip_h));
            right -= w + 6.0;
            r
        };
        let r_chart = place(48.0);
        let r_queue = place(48.0);
        if chip(ui, r_chart, "CHART", self.rtab == 1, "c_rchart").tip("Chord changes of the open tune").clicked() {
            acts.push(Action::RightTab(1));
        }
        if chip(ui, r_queue, "QUEUE", self.rtab == 0, "c_rqueue").clicked() {
            acts.push(Action::RightTab(0));
        }
    }

    /// Called at the top of the queue window.
    pub(crate) fn right_header(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) -> bool {
        if !PRACTICE {
            return false;
        }
        self.right_tabs(ui, acts);
        if self.rtab == 1 {
            self.chart_ui(ui, acts);
            true
        } else {
            false
        }
    }

    // ---------------------------------------------------------------- FILES
    /// `which`: 0 files, 1 YouTube clips, 2 kept SoundCloud tracks, 3 SoundCloud search results
    fn ext_list(&self, ui: &mut egui::Ui, acts: &mut Vec<Action>, which: u8) {
        let yt = which != 0 && which != 4;
        let hearts: Vec<Ext>;
        let list = match which {
            0 => &self.store.files,
            1 => &self.store.yt,
            2 => &self.store.sc,
            4..=6 => {
                hearts = self.store.hearts.iter().filter(|e| heart_group(e) == which - 4).cloned().collect();
                &hearts
            }
            7 => &self.yt_results,
            _ => &self.sc_results,
        };
        let playing = self.cur_track().map(|t| t.id);
        col_header(ui, true);
        for (i, e) in list.iter().enumerate() {
            let state = if playing == Some(e.id) { RowState::Playing } else { RowState::Normal };
            let stored = if yt { cache::has(e.id) } else { true };
            crate::row_heart(self.liked.contains(&e.id));
            let r = list_row(
                ui,
                i,
                Some(i + 1),
                || track_cells(e.id, &e.title, &e.artist, ""),
                || {
                    if e.dur == 0.0 && e.cover.is_empty() && (which == 2 || which == 3 || which == 6) {
                        "(loading info)".to_string()
                    } else if col_on(2) {
                        fmt_time(e.dur)
                    } else {
                        String::new()
                    }
                },
                state,
                false,
                stored,
            );
            if crate::heart_clicked() {
                acts.push(Action::ToggleLike(e.to_track()));
            }
            if let Some(r) = r {
                if r.clicked() {
                    let tracks: Vec<Track> = list.iter().map(|x: &Ext| x.to_track()).collect();
                    acts.push(Action::Play(tracks, i));
                }
                let liked = self.liked.contains(&e.id);
                r.context_menu(|ui| {
                    crate::song_items(ui, acts, &e.to_track(), liked, false);
                    if menu_item(ui, "Edit info...") {
                        acts.push(Action::EditInfo(e.id, e.title.clone(), e.artist.clone()));
                        ui.close_menu();
                    }
                    if e.kind == "file"
                        && menu_item(ui, if cfg!(target_os = "macos") { "Show in Finder" } else { "Show in Explorer" })
                    {
                        sources::reveal(&e.src);
                        ui.close_menu();
                    }
                    crate::link_item(ui, acts, crate::track_link(&e.to_track(), Some(e)));
                    if which >= 4 {
                        // (taking it out of LIKED is Remove from My Tracks, above)
                    } else if which == 3 || which == 7 {
                        if which == 3 && menu_item(ui, "Keep in my SoundCloud list") {
                            acts.push(Action::ScKeep(e.id));
                            ui.close_menu();
                        }
                        if which == 7 && menu_item(ui, "Keep in my clips") {
                            acts.push(Action::YtKeep(e.id));
                            ui.close_menu();
                        }
                    } else if menu_item(ui, "Remove from this list") {
                        acts.push(Action::RemoveExt(e.id));
                        ui.close_menu();
                    }
                });
            }
        }
    }

    /// Saved playlist links of one kind ("sc" / "yt"); click opens, right-click removes.
    fn saved_lists(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, kind: &str) {
        let mine: Vec<(usize, store::SavedList)> =
            self.store.lists.iter().enumerate().filter(|(_, l)| l.kind == kind).map(|(i, l)| (i, l.clone())).collect();
        if mine.is_empty() {
            return;
        }
        section_header(ui, "MY PLAYLISTS (CLICK TO OPEN, RIGHT-CLICK TO REMOVE)");
        for (k, (i, l)) in mine.iter().enumerate() {
            if let Some(r) =
                list_row(ui, k, Some(k + 1), || l.name.clone(), || "OPEN".to_string(), RowState::Normal, false, false)
            {
                if r.clicked() {
                    if kind == "sc" {
                        acts.push(Action::ScOpenSet(l.url.clone()));
                    } else {
                        self.yt_in = l.url.clone();
                        acts.push(Action::AddYt);
                    }
                }
                r.context_menu(|ui| {
                    crate::link_item(ui, acts, Some(l.url.clone()));
                    if menu_item(ui, "Remove from my playlists") {
                        acts.push(Action::RemoveList(*i));
                        ui.close_menu();
                    }
                });
            }
        }
    }

    /// The hearts you gave to this kind of source (0 files, 1 YouTube, 2 SoundCloud); stored on this PC only.
    fn liked_block(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, group: u8) {
        let n = self.store.hearts.iter().filter(|e| heart_group(e) == group).count();
        if n == 0 {
            return;
        }
        section_header(ui, "LIKED (ON THIS PC)");
        let tracks: Vec<Track> = self.store.hearts.iter().filter(|e| heart_group(e) == group).map(|e| e.to_track()).collect();
        ui.add_space(4.0);
        play_buttons(ui, &tracks, acts, false);
        ui.add_space(4.0);
        self.ext_list(ui, acts, 4 + group);
    }

    pub(crate) fn files_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "MY FILES", 3.0, pal().ink);
        title_line(ui, &format!("{} files - played from where they are", self.store.files.len()), 2.0, pal().ink2);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "ADD FILES", false).clicked() {
                acts.push(Action::AddFiles);
            }
            if retro_btn(ui, "ADD FOLDER", false).clicked() {
                acts.push(Action::AddFolder);
            }
            if retro_btn(ui, "RESCAN", false).tip("Look for new files in the folders you added").clicked() {
                acts.push(Action::Rescan);
            }
        });
        if self.files_busy {
            let stage = crate::extras::FILES_STAGE.load(Ordering::Relaxed);
            dim_line(
                ui,
                if stage == 1 { "PICK THE FOLDER OR FILES IN THE WINDOW THAT OPENED..." } else { "READING THE FILES..." },
                2.0,
                pal().ink2,
            );
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        }
        if self.store.files.is_empty() && !self.files_busy {
            ui.add_space(6.0);
            let tail = if PRACTICE { " Loops, speed and sections all work on them." } else { "" };
            para(ui, &format!("Drag audio files or whole folders onto this window, or use the buttons. Nothing is copied - files stay where they are.{}", tail), pal().ink2);
        }
        if !self.store.folders.is_empty() {
            section_header(ui, "WATCHED FOLDERS (CLICK TO STOP)");
            for (i, f) in self.store.folders.iter().enumerate() {
                if let Some(r) = list_row(ui, i, None, || f.clone(), || "FORGET".to_string(), RowState::Normal, false, false) {
                    if r.clicked() {
                        acts.push(Action::ForgetFolder(i));
                    }
                }
            }
        }
        if !self.store.files.is_empty() {
            section_header(ui, "FILES");
            let tracks: Vec<Track> = self.store.files.iter().map(|e| e.to_track()).collect();
            ui.add_space(4.0);
            play_buttons(ui, &tracks, acts, false);
            ui.add_space(4.0);
            self.ext_list(ui, acts, 0);
        }
        self.liked_block(ui, acts, 0);
        ui.add_space(20.0);
    }

    // -------------------------------------------------------------- YOUTUBE
    pub(crate) fn yt_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "YOUTUBE", 3.0, pal().ink);
        title_line(ui, "audio only, for practice", 2.0, pal().ink2);
        ui.add_space(4.0);
        let have = sources::ytdlp_path().is_some();
        ui.horizontal_wrapped(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(150.0, bh()), Sense::hover());
            ptext(
                ui.painter(),
                Pos2::new(rect.min.x + 2.0, rect.center().y),
                Align::Min,
                if have { "YT-DLP: READY" } else { "YT-DLP: MISSING" },
                2.0,
                if have { pal().ink } else { pal().red },
            );
            if retro_btn(ui, if have { "UPDATE YT-DLP" } else { "GET YT-DLP" }, !have)
                .tip("Downloads the free yt-dlp tool from its official GitHub page")
                .clicked()
            {
                acts.push(Action::GetYtDlp);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let w = (ui.available_width() - 78.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_YT, &mut self.yt_in, rect, "Paste a YouTube link or playlist...", false);
            if o.enter || retro_btn_w(ui, "ADD", 70.0, false).clicked() {
                acts.push(Action::AddYt);
            }
        });
        if self.yt_busy || !self.yt_msg.is_empty() {
            let col = if self.yt_msg.starts_with("ERROR") || self.yt_msg.starts_with("PRESS") || self.yt_msg.starts_with("THAT") {
                pal().red
            } else {
                pal().ink2
            };
            para(ui, &self.yt_msg.clone(), col);
        }
        self.saved_lists(ui, acts, "yt");
        if !self.yt_results.is_empty() {
            section_header(ui, "PLAYLIST");
            let tracks: Vec<Track> = self.yt_results.iter().map(|e| e.to_track()).collect();
            let total: f32 = self.yt_results.iter().map(|e| e.dur).sum();
            ui.add_space(4.0);
            let name = if self.yt_title.is_empty() { "YouTube playlist".to_string() } else { self.yt_title.clone() };
            title_line(ui, &name, 2.5, pal().ink);
            title_line(
                ui,
                &format!(
                    "{} songs{}",
                    tracks.len(),
                    if total > 0.0 { format!(", {}", crate::fmt_long(total)) } else { String::new() }
                ),
                2.0,
                pal().ink2,
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if crate::ibtn(ui, &crate::IC_PLAY, "PLAY", false).clicked() {
                    acts.push(Action::Play(tracks.clone(), 0));
                }
                if crate::ibtn(ui, &crate::IC_SHUF, "SHUFFLE", false).clicked() {
                    acts.push(Action::PlayShuffled(tracks.clone()));
                }
                if retro_btn(ui, if self.yt_open { "HIDE SONGS" } else { "SHOW SONGS" }, self.yt_open).clicked() {
                    self.yt_open = !self.yt_open;
                }
                let saved = self.yt_cur.as_ref().map_or(true, |u| self.store.lists.iter().any(|l| &l.url == u));
                if !saved && retro_btn(ui, "SAVE", false).tip("Keep it in MY PLAYLISTS, to open it again later").clicked() {
                    if let Some(u) = self.yt_cur.clone() {
                        acts.push(Action::SaveList(1, u));
                    }
                }
                if retro_btn_w(ui, "X", 30.0, false).tip("Put this playlist away").clicked() {
                    self.yt_results.clear();
                    self.yt_title.clear();
                    self.yt_cur = None;
                    self.yt_open = false;
                }
            });
            if self.yt_open && !self.yt_results.is_empty() {
                ui.add_space(4.0);
                dim_line(ui, "RIGHT-CLICK A SONG TO KEEP IT", 1.0, pal().dim);
                self.ext_list(ui, acts, 7);
            }
        }
        if self.store.yt.is_empty() {
            para(ui, "One-time setup: press GET YT-DLP. Then paste a link and press ADD.", pal().ink2);
            if PRACTICE {
                para(ui, "Play-along videos, lessons, live takes - anything you want to loop and slow down.", pal().ink2);
            }
            para(
                ui,
                "Note: using YouTube this way may go against YouTube's terms. Personal use only, at your own discretion.",
                pal().dim,
            );
        } else {
            section_header(ui, "MY CLIPS");
            let tracks: Vec<Track> = self.store.yt.iter().map(|e| e.to_track()).collect();
            ui.add_space(4.0);
            play_buttons(ui, &tracks, acts, false);
            ui.add_space(4.0);
            self.ext_list(ui, acts, 1);
        }
        self.liked_block(ui, acts, 1);
        ui.add_space(20.0);
    }

    // ----------------------------------------------------------- SOUNDCLOUD
    pub(crate) fn sc_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "SOUNDCLOUD", 3.0, pal().ink);
        title_line(ui, "search it, paste a link, or type a user and press THEIR PLAYLISTS", 2.0, pal().ink2);
        ui.add_space(4.0);
        let have = sources::ytdlp_path().is_some();
        if !have {
            ui.horizontal_wrapped(|ui| {
                label(ui, "ONE-TIME SETUP", 150.0);
                if retro_btn(ui, "GET YT-DLP", true).tip("Downloads the free yt-dlp tool, which also reads SoundCloud").clicked()
                {
                    acts.push(Action::GetYtDlp);
                }
            });
            if self.yt_busy || !self.yt_msg.is_empty() {
                para(ui, &self.yt_msg.clone(), pal().ink2);
            }
        }
        ui.horizontal_wrapped(|ui| {
            let w = (ui.available_width() - 78.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_SC, &mut self.sc_in, rect, "Search SoundCloud, or paste a link...", false);
            if o.enter || retro_btn_w(ui, "GO", 70.0, false).clicked() {
                acts.push(Action::ScGo);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "THEIR PLAYLISTS", false)
                .tip("Type a SoundCloud user name (or paste their profile link) above, then press this to list all their public playlists")
                .clicked()
            {
                acts.push(Action::ScSets);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let n = sources::SC_BROWSER.load(Ordering::Relaxed) as usize;
            label(ui, "SIGN-IN FROM", 130.0);
            if retro_btn_w(ui, sources::SC_BROWSERS[n], 110.0, n > 0)
                .tip("Use the SoundCloud sign-in of a browser on this PC (private playlists, Go+). Click to change. Firefox works most reliably.")
                .clicked()
            {
                acts.push(Action::ScBrowser);
            }
        });
        if !self.sc_msg.is_empty() {
            let col = if self.sc_msg.starts_with("ERROR") || self.sc_msg.starts_with("PRESS") { pal().red } else { pal().ink2 };
            para(ui, &self.sc_msg.clone(), col);
        }
        if self.sc_results.is_empty() && self.store.sc.is_empty() {
            para(ui, "Your own account: paste your profile's likes link (soundcloud.com/YOU/likes) or a playlist link and press GO - public ones list right here. Click a track to play.", pal().ink2);
            para(ui, "Note: no sign-in is used, so private and Go+ tracks may play as previews or not at all. Using SoundCloud this way may go against its terms; personal practice use is at your discretion.", pal().dim);
        }
        self.saved_lists(ui, acts, "sc");
        if !self.sc_sets.is_empty() {
            section_header(ui, "PLAYLISTS (CLICK TO OPEN)");
            for (i, (title, url)) in self.sc_sets.iter().enumerate() {
                if let Some(r) =
                    list_row(ui, i, Some(i + 1), || title.clone(), || "OPEN".to_string(), RowState::Normal, false, false)
                {
                    if r.clicked() {
                        acts.push(Action::ScOpenSet(url.clone()));
                    }
                }
            }
        }
        if !self.sc_results.is_empty() {
            section_header(ui, "RESULTS (RIGHT-CLICK TO KEEP)");
            let tracks: Vec<Track> = self.sc_results.iter().map(|e| e.to_track()).collect();
            ui.add_space(4.0);
            if let Some(u) = self.sc_cur.clone().filter(|u| tracks.len() > 1 && !self.store.lists.iter().any(|l| l.url == *u)) {
                ui.horizontal_wrapped(|ui| {
                    if retro_btn(ui, "SAVE THIS PLAYLIST", false).tip("Keep the link so you can open it again later").clicked() {
                        acts.push(Action::SaveList(0, u.clone()));
                    }
                });
            }
            play_buttons(ui, &tracks, acts, false);
            ui.add_space(4.0);
            self.ext_list(ui, acts, 3);
        }
        if !self.store.sc.is_empty() {
            section_header(ui, "MY SOUNDCLOUD");
            let tracks: Vec<Track> = self.store.sc.iter().map(|e| e.to_track()).collect();
            ui.add_space(4.0);
            play_buttons(ui, &tracks, acts, false);
            ui.add_space(4.0);
            self.ext_list(ui, acts, 2);
        }
        self.liked_block(ui, acts, 2);
        ui.add_space(20.0);
    }

    /// LOOPS > GROUPS: your groups as a list (name and how many loops); open one for its own page: its loops, RENAME
    /// and DELETE, and a way back. + NEW GROUP makes one.
    fn groups_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if let Some(g) = self.grp_open.clone().filter(|g| self.store.groups().contains(g) || g.is_empty()) {
            self.group_page(ui, acts, &g);
            return;
        }
        self.grp_open = None;
        let groups = self.store.groups();
        let count = |g: &str| self.store.sections.values().flatten().filter(|s| s.group == g).count();
        let mut rows: Vec<(String, usize)> = groups.iter().map(|g| (g.clone(), count(g))).collect();
        let loose = count("");
        if loose > 0 {
            rows.push((String::new(), loose));
        }
        for (k, (g, n)) in rows.iter().enumerate() {
            let title = if g.is_empty() { "NO GROUP".to_string() } else { g.to_uppercase() };
            let sub = format!("{} {}", n, if *n == 1 { "LOOP" } else { "LOOPS" });
            if let Some(r) = list_row(ui, k, None, || title.clone(), || sub.clone(), RowState::Normal, false, false) {
                if r.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Open this group").clicked() {
                    self.grp_open = Some(g.clone());
                    self.grp_edit = None;
                    self.grp_ask = None;
                }
            }
        }
        self.new_group_row(ui);
        ui.add_space(20.0);
    }

    /// One group's own page: its loops (click to play, right-click to rename, note, move or forget), RENAME and
    /// DELETE (asked first; its loops stay), and back to the list.
    fn group_page(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, g: &str) {
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "< ALL GROUPS", false).clicked() {
                self.grp_open = None;
                self.grp_edit = None;
                self.grp_ask = None;
            }
            if !g.is_empty() {
                if retro_btn(ui, "RENAME", self.grp_edit.is_some()).clicked() {
                    self.grp_edit = Some(g.to_string());
                    self.grp_name = g.to_string();
                    self.ed.id = crate::tools::F_GROUP_NAME;
                }
                if retro_btn(ui, "DELETE...", self.grp_ask.is_some()).clicked() {
                    self.grp_ask = Some(g.to_string());
                }
            }
        });
        title_line(ui, &if g.is_empty() { "NO GROUP".to_string() } else { g.to_uppercase() }, 3.0, pal().ink);
        if self.grp_edit.as_deref() == Some(g) {
            ui.horizontal_wrapped(|ui| {
                label(ui, "NEW NAME", 90.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(200.0, bh()), Sense::hover());
                let o = field(ui, &mut self.ed, crate::tools::F_GROUP_NAME, &mut self.grp_name, r, "new name", false);
                if (o.enter || retro_btn(ui, "SAVE", false).clicked()) && !self.grp_name.trim().is_empty() {
                    let new = self.grp_name.trim().to_string();
                    self.store.rename_group(g, &new);
                    self.store_dirty = true;
                    self.grp_edit = None;
                    self.grp_open = Some(new);
                    self.ed.id = 0;
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.grp_edit = None;
                    self.ed.id = 0;
                }
            });
        }
        if self.grp_ask.as_deref() == Some(g) {
            ui.horizontal_wrapped(|ui| {
                label(ui, "DELETE THIS GROUP? ITS LOOPS STAY, IN NO GROUP", 360.0);
                if retro_btn(ui, "YES, DELETE IT", false).clicked() {
                    self.store.rename_group(g, "");
                    self.store_dirty = true;
                    self.grp_ask = None;
                    self.grp_open = None;
                }
                if retro_btn(ui, "KEEP IT", false).clicked() {
                    self.grp_ask = None;
                }
            });
        }
        let groups = self.store.groups();
        let mut members: Vec<(i64, usize, store::Section)> = self
            .store
            .sections
            .iter()
            .flat_map(|(id, l)| l.iter().enumerate().map(move |(i, s)| (*id, i, s.clone())))
            .filter(|x| x.2.group == g)
            .collect();
        members.sort_by(|a, b| a.2.name.cmp(&b.2.name).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
        if members.is_empty() {
            para(
                ui,
                "No loops in it yet. Right-click a loop (here in LOOPS, or in the practice panel) and move it here.",
                pal().ink2,
            );
            return;
        }
        for (k, (id, i, s)) in members.iter().enumerate() {
            let v = self.store.songs.get(id).cloned();
            let song = v.as_ref().map(|v| format!("{} - {}", v.artist, v.title)).unwrap_or_default();
            let title = format!("{}   {}-{}", s.name.to_uppercase(), fmt_time(s.a), fmt_time(s.b));
            let sub = song.clone();
            if let Some(r) = list_row(ui, k, None, || title.clone(), || sub.clone(), RowState::Normal, false, false) {
                let r = r.on_hover_cursor(egui::CursorIcon::PointingHand).tip(if s.note.is_empty() {
                    "Click to play it, looping".to_string()
                } else {
                    s.note.clone()
                });
                if r.clicked() {
                    if let Some(v) = &v {
                        acts.push(Action::PlaySaved(v.clone(), Some((s.a, s.b))));
                    }
                }
                r.context_menu(|ui| {
                    if menu_item(ui, "Rename / add a note...") {
                        self.clip_edit = Some((*id, *i));
                        self.clip_name = s.name.clone();
                        self.clip_note = s.note.clone();
                        self.clip_group = s.group.clone();
                        ui.close_menu();
                    }
                    for other in groups.iter().filter(|o| **o != s.group) {
                        if menu_item(ui, &format!("Move to {}", other)) {
                            acts.push(Action::LoopGroup(*id, *i, other.clone()));
                            ui.close_menu();
                        }
                    }
                    if !s.group.is_empty() && menu_item(ui, "Out of the group") {
                        acts.push(Action::LoopGroup(*id, *i, String::new()));
                        ui.close_menu();
                    }
                    if menu_item(ui, "Forget this loop") {
                        acts.push(Action::ForgetLoop(*id, *i));
                        ui.close_menu();
                    }
                });
            }
            if self.clip_edit == Some((*id, *i)) {
                self.loop_editor(ui, acts);
            }
        }
        ui.add_space(20.0);
    }

    /// + NEW GROUP, opening a name box.
    fn new_group_row(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if !self.grp_adding {
                if retro_btn(ui, "+ NEW GROUP", false).clicked() {
                    self.grp_adding = true;
                    self.ed.id = crate::tools::F_GROUP_NAME + 100;
                }
                return;
            }
            let (r, _) = ui.allocate_exact_size(Vec2::new(200.0, bh()), Sense::hover());
            let o =
                field(ui, &mut self.ed, crate::tools::F_GROUP_NAME + 100, &mut self.grp_new, r, "the new group's name", false);
            if (o.enter || retro_btn(ui, "ADD", false).clicked()) && !self.grp_new.trim().is_empty() {
                let mut l = self.store.groups();
                if !l.iter().any(|g| g == self.grp_new.trim()) {
                    l.push(self.grp_new.trim().to_string());
                }
                self.store.loop_groups = Some(l);
                self.grp_new.clear();
                self.grp_adding = false;
                self.ed.id = 0;
                self.store_dirty = true;
            }
            if retro_btn(ui, "CANCEL", false).clicked() {
                self.grp_adding = false;
                self.grp_new.clear();
                self.ed.id = 0;
            }
        });
    }

    /// WHAT'S NEW, over the dimmed app: the notes of this version and the few before it, newest first.
    pub(crate) fn news_panel(&mut self, ctx: &egui::Context) {
        let Some(notes) = self.news.clone() else { return };
        let screen = ctx.screen_rect();
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("news_dim"))).rect_filled(
            screen,
            0.0,
            egui::Color32::from_black_alpha(170),
        );
        let me = crate::update::build();
        let mut close = false;
        egui::Area::new(egui::Id::new("news_panel"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Frame::none().fill(pal().app_bg).stroke(egui::Stroke::new(2.0_f32, pal().edge)).inner_margin(16.0).show(
                    ui,
                    |ui| {
                        ui.set_width(560.0_f32.min(screen.width() - 40.0));
                        title_line(ui, "WHAT'S NEW", 3.0, pal().ink);
                        egui::ScrollArea::vertical().max_height((screen.height() * 0.6).max(160.0)).show(ui, |ui| {
                            for (tag, body) in &notes {
                                let this = tag.rsplit('.').next().and_then(|n| n.parse::<u64>().ok()) == Some(me);
                                section_header(
                                    ui,
                                    &format!("{}{}", tag.to_uppercase(), if this { "  - THIS VERSION" } else { "" }),
                                );
                                for line in body.lines() {
                                    // the notes are written for GitHub: headings and bullets become plain lines; the
                                    // download instructions (the same every time) are left out
                                    if line.trim().trim_start_matches('#').trim().eq_ignore_ascii_case("downloads") {
                                        break;
                                    }
                                    let l = line.trim().trim_start_matches('#').trim().replace('`', "");
                                    if l.is_empty() || l.starts_with("**Full Changelog") || l.starts_with("<!--") {
                                        continue;
                                    }
                                    let l = l.replace("**", "");
                                    let (text, col) = match l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) {
                                        Some(b) => (format!("  - {}", b), pal().ink2),
                                        None => (l.to_string(), pal().ink),
                                    };
                                    para(ui, &text, col);
                                }
                                ui.add_space(6.0);
                            }
                        });
                        ui.add_space(6.0);
                        if retro_btn(ui, "GOT IT", false).clicked() {
                            close = true;
                        }
                    },
                );
            });
        if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.news = None;
            self.seen_build = crate::update::build();
            self.dirty = true;
        }
    }

    /// The export / import preview, over the dimmed app: what is in the file (songs, loops, playlists) and, for an
    /// import, what is new to you and what can't play here - then go ahead or cancel.
    pub(crate) fn share_panel(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        let Some((importing, s)) = self.share_view.clone() else {
            self.share_pick = None;
            return;
        };
        // everything ticked to begin with
        if self.share_pick.is_none() {
            let songs = s.loops.iter().filter(|(_, l)| !l.is_empty()).map(|(id, _)| *id).collect();
            let lists = s.playlists.iter().map(|p| p.name.clone()).collect();
            self.share_pick = Some((songs, lists));
        }
        let screen = ctx.screen_rect();
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("share_dim"))).rect_filled(
            screen,
            0.0,
            egui::Color32::from_black_alpha(170),
        );
        let mut songs: Vec<(i64, String, usize)> = s
            .loops
            .iter()
            .filter(|(_, l)| !l.is_empty())
            .map(|(id, l)| {
                let name = s.songs.get(id).map(|v| format!("{} - {}", v.artist, v.title)).unwrap_or_else(|| "a song".into());
                (*id, name, l.len())
            })
            .collect();
        songs.sort_by(|a, b| a.1.cmp(&b.1));
        let lists: Vec<(String, usize)> = s.playlists.iter().map(|p| (p.name.clone(), p.items.len())).collect();
        let (picked_songs, picked_lists) = self.share_pick.clone().unwrap_or_default();
        let n_loops: usize = songs.iter().filter(|x| picked_songs.contains(&x.0)).map(|x| x.2).sum();
        let n_songs = songs.iter().filter(|x| picked_songs.contains(&x.0)).count();
        let n_lists = lists.iter().filter(|x| picked_lists.contains(&x.0)).count();
        // what an import would add, counting only what's ticked
        let took = if importing {
            let mut t = s.clone();
            t.loops.retain(|id, _| picked_songs.contains(id));
            t.songs.retain(|id, _| picked_songs.contains(id));
            t.playlists.retain(|p| picked_lists.contains(&p.name));
            Some(self.store.preview_share(&t))
        } else {
            None
        };
        let mut pick = self.share_pick.take().unwrap_or_default();
        let newer = s.version > store::SHARE_VERSION;
        let mut close = false;
        // above the dimming (which is on the foreground layer)
        egui::Area::new(egui::Id::new("share_panel"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Frame::none().fill(pal().app_bg).stroke(egui::Stroke::new(2.0_f32, pal().edge)).inner_margin(16.0).show(
                    ui,
                    |ui| {
                        ui.set_width(520.0_f32.min(screen.width() - 40.0));
                        title_line(
                            ui,
                            if importing { "IMPORT LOOPS & PLAYLISTS" } else { "EXPORT LOOPS & PLAYLISTS" },
                            3.0,
                            pal().ink,
                        );
                        para(ui, &format!("TICKED: {} LOOPS ON {} SONGS, {} PLAYLISTS", n_loops, n_songs, n_lists), pal().ink);
                        ui.horizontal_wrapped(|ui| {
                            let all_songs: std::collections::HashSet<i64> = songs.iter().map(|x| x.0).collect();
                            let all_lists: std::collections::HashSet<String> = lists.iter().map(|x| x.0.clone()).collect();
                            if retro_btn(ui, "EVERYTHING", false).clicked() {
                                pick = (all_songs.clone(), all_lists.clone());
                            }
                            if retro_btn(ui, "ALL LOOPS", false).tip("Every loop, no playlists").clicked() {
                                pick = (all_songs.clone(), std::collections::HashSet::new());
                            }
                            if retro_btn(ui, "ALL PLAYLISTS", false).tip("Every playlist, no loops").clicked() {
                                pick = (std::collections::HashSet::new(), all_lists.clone());
                            }
                            if retro_btn(ui, "NONE", false).clicked() {
                                pick = Default::default();
                            }
                        });
                        if let Some(t) = &took {
                            para(
                                ui,
                                &format!(
                                    "NEW TO YOU: {} LOOPS, {} PLAYLISTS, {} SONGS FOR PLAYLISTS YOU HAVE",
                                    t.loops, t.playlists, t.items
                                ),
                                pal().ink,
                            );
                            if t.relinked > 0 {
                                para(ui, &format!("{} FILES WILL BE MATCHED TO YOUR OWN COPIES", t.relinked), pal().ink2);
                            }
                            if t.missing > 0 {
                                para(
                                    ui,
                                    &format!("{} FILES AREN'T ON THIS COMPUTER (THEIR LOOPS COME IN, BUT WON'T PLAY)", t.missing),
                                    pal().ink2,
                                );
                            }
                            if t.tidal > 0 && !self.api.has_token() {
                                para(ui, &format!("{} TIDAL SONGS NEED A TIDAL LOGIN TO PLAY", t.tidal), pal().ink2);
                            }
                        }
                        if newer {
                            para(
                                ui,
                                "MADE BY A NEWER BACKLINE: SOME OF IT MAY BE LEFT OUT - UPDATE TO BRING IN EVERYTHING",
                                pal().red,
                            );
                        }
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical().max_height((screen.height() * 0.45).max(120.0)).show(ui, |ui| {
                            if !songs.is_empty() {
                                section_header(ui, "LOOPS");
                                for (id, name, n) in &songs {
                                    let on = pick.0.contains(id);
                                    let label = format!("{}  ({} {})", name, n, if *n == 1 { "LOOP" } else { "LOOPS" });
                                    if crate::check_box(ui, &label, on).clicked() {
                                        if on {
                                            pick.0.remove(id);
                                        } else {
                                            pick.0.insert(*id);
                                        }
                                    }
                                }
                            }
                            if !lists.is_empty() {
                                section_header(ui, "PLAYLISTS");
                                for (name, n) in &lists {
                                    let on = pick.1.contains(name);
                                    if crate::check_box(ui, &format!("{}  ({} SONGS)", name, n), on).clicked() {
                                        if on {
                                            pick.1.remove(name);
                                        } else {
                                            pick.1.insert(name.clone());
                                        }
                                    }
                                }
                            }
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let go = if importing { "IMPORT" } else { "SAVE..." };
                            if retro_btn(ui, go, false).clicked() {
                                acts.push(if importing { Action::ShareTake } else { Action::ShareSave });
                            }
                            if retro_btn(ui, "CANCEL", false).clicked() {
                                close = true;
                            }
                        });
                    },
                );
            });
        self.share_pick = Some(pick);
        if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.share_view = None;
            self.share_pick = None;
        }
    }

    /// Take your loops and playlists to another computer: a file to save, send however you like, and open there.
    fn share_buttons(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "EXPORT LOOPS & PLAYLISTS...", false)
                .tip("Save every loop and playlist to one file: email it, put it on a USB stick or OneDrive, then IMPORT it on the other computer")
                .clicked()
            {
                acts.push(Action::ShareExport);
            }
            if retro_btn(ui, "IMPORT...", false)
                .tip("Bring in a file from another computer: adds the loops and playlists you don't have, changes nothing you do")
                .clicked()
            {
                acts.push(Action::ShareImport);
            }
        });
    }

    // ------------------------------------------------------------ PLAYLISTS
    pub(crate) fn playlists_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        let open = self.pl_open.filter(|&i| i < self.store.playlists.len());
        let picking = self.pl_pick.is_some();
        if let (Some(i), false) = (open, picking) {
            self.playlist_detail(ui, acts, i);
            return;
        }
        if let Some(t) = self.pl_pick.clone() {
            let what = match &self.pl_loop {
                Some((a, b, name)) => {
                    format!("THE LOOP \"{}\" ({}-{}) OF {} - {}", name, fmt_time(*a), fmt_time(*b), t.artist, t.title)
                }
                None => format!("\"{} - {}\"", t.artist, t.title),
            };
            let which = if self.pl_tidal { "TIDAL PLAYLIST" } else { "PLAYLIST" };
            para(ui, &format!("ADD {} TO WHICH {}?", what, which), pal().ink);
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.pl_pick = None;
                    self.pl_loop = None;
                    self.pl_tidal = false;
                }
            });
            ui.add_space(4.0);
            let backline: &[store::Playlist] = if self.pl_tidal { &[] } else { &self.store.playlists };
            if !backline.is_empty() {
                section_header(ui, "BACKLINE PLAYLISTS");
            }
            for (i, pl) in backline.iter().enumerate() {
                let n = pl.items.len();
                if let Some(r) =
                    list_row(ui, i, Some(i + 1), || pl.name.clone(), || format!("{} tracks", n), RowState::Normal, false, false)
                {
                    if r.clicked() {
                        acts.push(Action::PlaylistAdd(i, t.clone()));
                    }
                }
            }
            if t.id > 0 && self.pl_loop.is_none() && self.api.has_token() {
                ui.add_space(8.0);
                section_header(ui, "YOUR TIDAL PLAYLISTS");
                if self.tidal_lists.is_none() && !self.tidal_lists_busy {
                    self.tidal_lists_busy = true;
                    let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                    std::thread::spawn(move || {
                        let _ = tx.send(crate::Msg::TidalLists(api.own_playlists()));
                        ctx.request_repaint();
                    });
                }
                match &self.tidal_lists {
                    None => dim_line(ui, "LOADING...", 1.0, pal().dim),
                    Some(l) if l.is_empty() => {
                        dim_line(ui, "NONE YET - TYPE A NAME BELOW AND PRESS NEW ON TIDAL", 1.0, pal().dim)
                    }
                    Some(l) => {
                        for (k, (uuid, name)) in l.iter().enumerate() {
                            if let Some(r) =
                                list_row(ui, k, Some(k + 1), || name.clone(), String::new, RowState::Normal, false, false)
                            {
                                if r.clicked() {
                                    acts.push(Action::TidalListAdd(uuid.clone(), name.clone(), t.id));
                                }
                            }
                        }
                    }
                }
            }
            ui.add_space(6.0);
        } else {
            title_line(ui, "PLAYLISTS", 3.0, pal().ink);
            title_line(ui, "your own lists - mix Tidal, SoundCloud, YouTube and files", 2.0, pal().ink2);
            ui.add_space(4.0);
        }
        ui.horizontal_wrapped(|ui| {
            let w = (ui.available_width() - 78.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_NEW_PL, &mut self.new_pl, rect, "New playlist name...", false);
            if !self.pl_tidal && (o.enter || retro_btn_w(ui, "NEW", 70.0, false).tip("A new Backline playlist").clicked()) {
                acts.push(Action::PlaylistNew(self.pl_pick.clone()));
            }
            // a new playlist on your Tidal account, with this song in it
            if let Some(t) = self.pl_pick.as_ref().filter(|t| t.id > 0 && self.api.has_token() && self.pl_loop.is_none()) {
                if retro_btn(ui, "NEW ON TIDAL", false)
                    .tip("Make this a playlist on your Tidal account, with the song in it")
                    .clicked()
                    && !self.new_pl.trim().is_empty()
                {
                    acts.push(Action::TidalListAdd(String::new(), self.new_pl.trim().to_string(), t.id));
                }
            }
        });
        if self.pl_pick.is_none() {
            self.share_buttons(ui, acts);
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "SAVE QUEUE AS PLAYLIST", false)
                    .tip("Turns what is in the queue right now into a playlist (type a name above first, or it gets a number)")
                    .clicked()
                {
                    acts.push(Action::PlaylistFromQueue);
                }
            });
            if self.store.playlists.is_empty() {
                para(ui, "Right-click any song, anywhere, and choose Add to a playlist. Playlists live on this PC only; they can hold tracks from every source at once.", pal().ink2);
            }
            ui.add_space(4.0);
            for (i, pl) in self.store.playlists.iter().enumerate() {
                let n = pl.items.len();
                if let Some(r) =
                    list_row(ui, i, Some(i + 1), || pl.name.clone(), || format!("{} tracks", n), RowState::Normal, false, false)
                {
                    if r.clicked() {
                        acts.push(Action::PlaylistOpen(Some(i)));
                    }
                    r.context_menu(|ui| {
                        if menu_item(ui, "Delete this playlist") {
                            acts.push(Action::PlaylistDelete(i));
                            ui.close_menu();
                        }
                    });
                }
            }
        }
        ui.add_space(20.0);
    }

    fn playlist_detail(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, pi: usize) {
        let name = self.store.playlists[pi].name.clone();
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "< ALL PLAYLISTS", false).clicked() {
                acts.push(Action::PlaylistOpen(None));
            }
        });
        title_line(ui, &name.to_uppercase(), 3.0, pal().ink);
        let items = self.store.playlists[pi].items.clone();
        title_line(ui, &format!("{} tracks - right-click a track to move or remove it", items.len()), 2.0, pal().ink2);
        ui.add_space(4.0);
        if items.is_empty() {
            para(ui, "Empty. Right-click songs anywhere and choose Add to a playlist.", pal().ink2);
            return;
        }
        let tracks: Vec<Track> = items.iter().map(|v| v.to_track()).collect();
        play_buttons(ui, &tracks, acts, false);
        ui.add_space(4.0);
        let playing = self.cur_track().map(|t| t.id);
        col_header(ui, true);
        for (k, v) in items.iter().enumerate() {
            let state = if playing == Some(v.id) { RowState::Playing } else { RowState::Normal };
            let stored = v.kind == "file" || cache::has(v.id);
            crate::row_heart(self.liked.contains(&v.id));
            // a loop shows as its name and where it is, the song after it
            let title = match v.loop_ab {
                Some((a, b)) => format!("LOOP: {} ({}-{})  {}", v.loop_name, fmt_time(a), fmt_time(b), v.title),
                None => v.title.clone(),
            };
            let len = v.loop_ab.map_or(v.dur, |(a, b)| b - a);
            let r = list_row(
                ui,
                k,
                Some(k + 1),
                || track_cells(v.id, &title, &v.artist, &v.album),
                || if col_on(2) { fmt_time(len) } else { String::new() },
                state,
                false,
                stored,
            );
            if crate::heart_clicked() {
                acts.push(Action::ToggleLike(v.to_track()));
            }
            if let Some(r) = r {
                if r.clicked() {
                    match v.loop_ab {
                        // a loop plays looping, from its start
                        Some(ab) => acts.push(Action::PlaySaved(v.clone(), Some(ab))),
                        None => acts.push(Action::Play(tracks.clone(), k)),
                    }
                }
                let tidal = self.ext_of(v.id).is_none();
                let liked = self.liked.contains(&v.id);
                r.context_menu(|ui| {
                    let t = v.to_track();
                    if v.loop_ab.is_some() && menu_item(ui, "Play the whole song") {
                        acts.push(Action::PlaySaved(v.clone(), None));
                        ui.close_menu();
                    }
                    crate::song_items(ui, acts, &t, liked, tidal);
                    if k > 0 && menu_item(ui, "Move up") {
                        acts.push(Action::PlaylistMove(pi, k, -1));
                        ui.close_menu();
                    }
                    if k + 1 < items.len() && menu_item(ui, "Move down") {
                        acts.push(Action::PlaylistMove(pi, k, 1));
                        ui.close_menu();
                    }
                    let ext = self.ext_of(v.id);
                    crate::link_item(ui, acts, crate::track_link(&t, ext.as_ref()));
                    if menu_item(ui, "Remove from this playlist") {
                        acts.push(Action::PlaylistRemove(pi, k));
                        ui.close_menu();
                    }
                });
            }
        }
        ui.add_space(20.0);
    }

    // ---------------------------------------------------------------- TUNES
    pub(crate) fn tunes_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if let Some(i) = self.tune_open {
            if i < self.store.tunes.len() && self.pick.is_none() {
                self.tune_detail(ui, acts, i);
                return;
            }
        }
        ui.add_space(6.0);
        if let Some(t) = self.pick.clone() {
            para(ui, &format!("ADD \"{} - {}\" TO WHICH TUNE?", t.artist, t.title), pal().ink);
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "+ NEW TUNE FROM THIS", false).clicked() {
                    acts.push(Action::NewTuneFrom(t.clone()));
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.pick = None;
                }
            });
            ui.add_space(6.0);
        } else {
            title_line(ui, "TUNES", 3.0, pal().ink);
            title_line(ui, &format!("{} tunes - your repertoire", self.store.tunes.len()), 2.0, pal().ink2);
            ui.add_space(4.0);
        }
        ui.horizontal_wrapped(|ui| {
            let w = (ui.available_width() - 78.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_NEW_TUNE, &mut self.new_tune, rect, "New tune name...", false);
            if o.enter || retro_btn_w(ui, "ADD", 70.0, false).clicked() {
                acts.push(Action::NewTune);
            }
        });
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bh()), Sense::hover());
        let _ =
            field(ui, &mut self.ed, F_TUNE_IREAL, &mut self.tunes_ireal, rect, "iReal Pro link or playlist: irealb://...", false);
        if retro_btn(ui, "IMPORT IREAL", false)
            .tip("Paste an iReal Pro link or a whole playlist above - every song becomes a tune with its chart")
            .clicked()
        {
            self.ireal_in = std::mem::take(&mut self.tunes_ireal);
            acts.push(Action::ImportIreal(None));
        }
        ui.add_space(4.0);
        if self.store.tunes.is_empty() {
            para(ui, "Your repertoire lives here, with the recordings you study for each tune. Add a recording from any list (right-click a song, then Add to a tune), or play one and press ADD CURRENT inside a tune.", pal().ink2);
        }
        if !self.store.tunes.is_empty() {
            table_header(ui, "TUNE", "RECORDINGS", "PRACTICED", false);
        }
        for (i, t) in self.store.tunes.iter().enumerate() {
            let mins = self.store.tune_secs(&t.name) / 60;
            let r = list_row(
                ui,
                i,
                None,
                || {
                    format!(
                        "[{}] {}",
                        "#".repeat(t.status.min(3) as usize + 1) + &"-".repeat(3 - t.status.min(3) as usize),
                        t.name
                    )
                },
                || format!("{}\t{}", t.versions.len(), mins_text(mins * 60)),
                RowState::Normal,
                false,
                false,
            );
            if let Some(r) = r {
                if r.clicked() {
                    match &self.pick {
                        Some(p) => acts.push(Action::AddToTune(i, p.clone())),
                        None => acts.push(Action::OpenTune(Some(i))),
                    }
                }
            }
        }
        if self.pick.is_none() {
            ui.add_space(10.0);
            self.progressions_list(ui, acts);
        }
        ui.add_space(20.0);
    }

    fn tune_detail(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, i: usize) {
        ui.add_space(6.0);
        let (status, nvers, secs, tname) = {
            let t = &self.store.tunes[i];
            (t.status, t.versions.len(), self.store.tune_secs(&t.name), t.name.clone())
        };
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "< TUNES", false).clicked() {
                acts.push(Action::OpenTune(None));
            }
            if retro_btn(ui, "CHART", self.rtab == 1).tip("Show the chord changes on the right").clicked() {
                acts.push(Action::RightTab(if self.rtab == 1 { 0 } else { 1 }));
            }
        });
        ui.add_space(4.0);
        if let Some(n) = comfort_meter(ui, status) {
            acts.push(Action::Status(i, n));
        }
        let o = field_row(ui, &mut self.ed, F_T_NAME, "NAME", &mut self.store.tunes[i].name, "Tune name", None);
        self.store_dirty |= o.changed;
        let o = field_row(ui, &mut self.ed, F_T_KEY, "KEY", &mut self.store.tunes[i].key, "e.g. Bb, Dm", None);
        self.store_dirty |= o.changed;
        ui.horizontal_wrapped(|ui| {
            label(ui, "BPM", 74.0);
            if retro_btn_w(ui, "-", 28.0, false).clicked() {
                let t = &mut self.store.tunes[i];
                t.bpm = if t.bpm == 0 { 100 } else { t.bpm.saturating_sub(5).max(30) };
                self.store_dirty = true;
            }
            let b = self.store.tunes[i].bpm;
            lcd_box(ui, &if b == 0 { "---".to_string() } else { b.to_string() }, 64.0, pal().ink);
            if retro_btn_w(ui, "+", 28.0, false).clicked() {
                let t = &mut self.store.tunes[i];
                t.bpm = if t.bpm == 0 { 100 } else { (t.bpm + 5).min(300) };
                self.store_dirty = true;
            }
            ui.add_space(8.0);
            lcd_box(ui, &format!("{} MIN PRACTICED", secs / 60), 150.0, pal().ink2);
        });
        let o = field_row(
            ui,
            &mut self.ed,
            F_T_NOTES,
            "NOTES",
            &mut self.store.tunes[i].notes,
            "What to work on, forms, tricky bars...",
            Some(84.0),
        );
        self.store_dirty |= o.changed;
        // who wrote it, when, where from: looked up once for you (MusicBrainz), yours to fix
        {
            let t = &self.store.tunes[i];
            if t.written_by.is_empty() || t.year.is_empty() {
                acts.push(Action::TuneDetails(i));
            }
        }
        for (fid, lab, hint) in [
            (crate::tools::F_T_WRITER, "WRITTEN BY", "Composer, and lyricist"),
            (crate::tools::F_T_YEAR, "YEAR", "When it was written or first recorded"),
            (crate::tools::F_T_FROM, "FROM", "The show, film or album it comes from"),
            (crate::tools::F_T_STYLE, "STYLE", "Swing, ballad, bossa, waltz..."),
        ] {
            let t = &mut self.store.tunes[i];
            let val = match fid {
                crate::tools::F_T_WRITER => &mut t.written_by,
                crate::tools::F_T_YEAR => &mut t.year,
                crate::tools::F_T_FROM => &mut t.from,
                _ => &mut t.style,
            };
            // a wider label column than the rows above: "WRITTEN BY" needs it
            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bh()), Sense::hover());
            let lw = (text_w("WRITTEN BY", 2.0) + 14.0).max(76.0);
            ptext(ui.painter(), Pos2::new(rect.min.x + 4.0, rect.center().y), Align::Min, lab, 2.0, pal().ink2);
            let fr = Rect::from_min_max(Pos2::new(rect.min.x + lw, rect.min.y), rect.max);
            let o = field(ui, &mut self.ed, fid, val, fr, hint, false);
            self.store_dirty |= o.changed;
        }
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "PLAY ALL", false).clicked() {
                acts.push(Action::PlayTune(i, false));
            }
            if retro_btn(ui, "SHUFFLE", false).clicked() {
                acts.push(Action::PlayTune(i, true));
            }
            if crate::ibtn(ui, &crate::IC_SEARCH, if self.look_busy { "LOOKING..." } else { "LOOK UP" }, self.look_busy)
                .tip("Who wrote it, and the most popular recordings on Tidal. Only searches when you press this.")
                .clicked()
            {
                acts.push(Action::LookUp(i));
            }
            if retro_btn(ui, ["AUTO", "JAZZ", "GOSPEL", "ANY"][self.look_style as usize % 4], self.look_style > 0)
                .tip("What kind of recordings LOOK UP favors. AUTO = jazz when the tune is a known standard.")
                .clicked()
            {
                acts.push(Action::LookStyle);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let w = (ui.available_width() - 4.0).max(120.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(
                ui,
                &mut self.ed,
                F_LOOK,
                &mut self.look_q,
                rect,
                "LOOK UP something else: a title, or title + artist (empty = this tune)",
                false,
            );
            if o.enter {
                acts.push(Action::LookUp(i));
            }
        });
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "PLAY-ALONG", false)
                .tip("Search YouTube for backing tracks of this tune (opens your browser)")
                .clicked()
            {
                acts.push(Action::PlayAlong(tname.clone()));
            }
        });
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "ADD CURRENT", false).tip("Add the recording that is playing now").clicked() {
                acts.push(Action::AddCurrentToTune(i));
            }
            if retro_btn(ui, if self.del_arm { "SURE? CLICK AGAIN" } else { "REMOVE TUNE" }, self.del_arm).clicked() {
                acts.push(Action::RemoveTune(i));
            }
        });

        // ---- the recordings
        section_header(ui, &format!("RECORDINGS TO STUDY ({})", nvers));
        if nvers == 0 {
            para(
                ui,
                "Nothing here yet. Play a recording and press ADD CURRENT, or right-click any song and choose Add to a tune.",
                pal().ink2,
            );
        }
        let vtracks: Vec<Track> = self.store.tunes[i].versions.iter().map(|v| v.to_track()).collect();
        let playing = self.cur_track().map(|t| t.id);
        if nvers > 0 {
            table_header(ui, "TRACK", "LENGTH", "RATING", true);
        }
        for vi in 0..nvers {
            let (artist, title, kind, stars, note, id) = {
                let v = &self.store.tunes[i].versions[vi];
                (v.artist.clone(), v.title.clone(), v.kind.clone(), v.stars, v.note.clone(), v.id)
            };
            let state = if playing == Some(id) { RowState::Playing } else { RowState::Normal };
            let tag = match kind.as_str() {
                "yt" if src_is_link(&self.store.tunes[i].versions[vi].src) => "[SC] ",
                "yt" => "[YT] ",
                "file" => "[FILE] ",
                _ => "",
            };
            let r = list_row(
                ui,
                vi,
                Some(vi + 1),
                || format!("{}{} - {}", tag, artist, title),
                || format!("{}\t{}", fmt_time(vtracks[vi].duration), stars_text(stars)),
                state,
                false,
                cache::has(id),
            );
            if let Some(r) = r {
                if r.clicked() {
                    acts.push(Action::Play(vtracks.clone(), vi));
                }
                let liked = self.liked.contains(&vtracks[vi].id);
                r.context_menu(|ui| {
                    if menu_item(ui, "Rate / add a note") {
                        self.edit_ver = Some(vi);
                        ui.close_menu();
                    }
                    let tidal = self.ext_of(vtracks[vi].id).is_none();
                    crate::song_items(ui, acts, &vtracks[vi], liked, tidal);
                    crate::link_item(ui, acts, crate::track_link(&vtracks[vi], self.ext_of(vtracks[vi].id).as_ref()));
                    if menu_item(ui, "Remove from this tune") {
                        acts.push(Action::RemoveVersion(i, vi));
                        ui.close_menu();
                    }
                });
            }
            if self.edit_ver == Some(vi) {
                ui.horizontal_wrapped(|ui| {
                    label(ui, "RATING", 74.0);
                    for n in 1..=5u8 {
                        if retro_btn_w(ui, &n.to_string(), 34.0, stars == n).clicked() {
                            self.store.tunes[i].versions[vi].stars = if stars == n { 0 } else { n };
                            self.store_dirty = true;
                        }
                    }
                    if retro_btn(ui, "DONE", false).clicked() {
                        self.edit_ver = None;
                        self.ed.id = 0;
                    }
                });
                let o = field_row(
                    ui,
                    &mut self.ed,
                    F_V_NOTE,
                    "NOTE",
                    &mut self.store.tunes[i].versions[vi].note,
                    "Why it's worth studying",
                    None,
                );
                self.store_dirty |= o.changed;
                ui.add_space(4.0);
            } else if !note.is_empty() {
                dim_line(ui, &format!("> {}", note), 2.0, pal().ink2);
            }
        }

        // ---- other versions (only after LOOK UP)
        if self.look_busy {
            dim_line(ui, "SEARCHING...", 2.0, pal().ink2);
        }
        if self.look_for == tname && !self.look_tracks.is_empty() {
            section_header(ui, "RECORDINGS TO STUDY");
            dim_line(
                ui,
                "BEST FIRST: TIDAL POPULARITY, WITHOUT KARAOKE/COMPILATIONS, ARTISTS YOU RATE UP. CLICK TO PLAY, RIGHT-CLICK TO ADD.",
                1.0,
                pal().dim,
            );
            let pops: Vec<u32> = self.look_tracks.iter().map(|x| x.1).collect();
            let found: Vec<Track> = self.look_tracks.iter().map(|x| x.0.clone()).collect();
            table_header(ui, "TRACK", "POPULARITY", "LENGTH", true);
            for (k, t) in found.iter().enumerate() {
                let state = if playing == Some(t.id) { RowState::Playing } else { RowState::Normal };
                let r = list_row(
                    ui,
                    k,
                    Some(k + 1),
                    || format!("{} - {}", t.artists_text(), t.title),
                    || format!("{}\t{}", pops[k], fmt_time(t.duration)),
                    state,
                    false,
                    cache::has(t.id),
                );
                if let Some(r) = r {
                    let r = r.tip(format!("From the album {}", t.album));
                    if r.clicked() {
                        acts.push(Action::Play(found.clone(), k));
                    }
                    let liked = self.liked.contains(&t.id);
                    r.context_menu(|ui| {
                        if menu_item(ui, "Add to this tune") {
                            acts.push(Action::AddToTune(i, t.clone()));
                            ui.close_menu();
                        }
                        crate::song_items(ui, acts, t, liked, true);
                    });
                }
            }
        }
        ui.add_space(20.0);
    }

    // ---------------------------------------------------------------- DIARY
    /// LOOPS (under PRACTICE): every song you saved a loop on or practised, without making it a tune - this week's
    /// first, or all of them. Click a loop to jump straight to it, looping.
    pub(crate) fn loops_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "LOOPS", 3.0, pal().ink);
        para(ui, "The songs you saved loops on or practised, no tune needed. Click a loop to jump straight to it. Click a song to play it. Right-click for more.", pal().ink2);
        self.share_buttons(ui, acts);
        self.name_old_loops();
        let today = store::today_days();
        let week_start = store::date_str(today - 6);
        let mut week: std::collections::HashMap<i64, u32> = std::collections::HashMap::new();
        for d in self.store.days.iter().filter(|d| d.date >= week_start) {
            for (id, s) in &d.songs {
                *week.entry(*id).or_default() += s;
            }
        }
        // how they're shown: each loop, by song, or by group
        if let Some(v) = crate::tab_row(ui, &["EACH LOOP", "BY SONG", "GROUPS"], self.loops_mode as usize) {
            self.loops_mode = v as u8;
        }
        if self.loops_mode == 2 {
            self.groups_view(ui, acts);
            return;
        }
        self.loops_by_song = self.loops_mode == 1;
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "THIS WEEK", !self.loops_all).tip("Songs practised, or looped, in the last 7 days").clicked() {
                self.loops_all = false;
            }
            if retro_btn(ui, "ALL", self.loops_all).tip("Every saved loop").clicked() {
                self.loops_all = true;
            }
        });
        ui.add_space(4.0);
        let mut ids: Vec<i64> = week.keys().copied().collect();
        for (id, l) in &self.store.sections {
            if (self.loops_all || l.iter().any(|s| s.saved >= week_start)) && !ids.contains(id) {
                ids.push(*id);
            }
        }
        if !self.loops_by_song {
            self.clips_list(ui, acts, &ids, &week_start);
            return;
        }
        let newest =
            |id: &i64| self.store.sections.get(id).and_then(|l| l.iter().map(|s| s.saved.clone()).max()).unwrap_or_default();
        ids.sort_by(|a, b| week.get(b).cmp(&week.get(a)).then(newest(b).cmp(&newest(a))).then(a.cmp(b)));
        if ids.is_empty() {
            para(
                ui,
                if self.loops_all {
                    "No saved loops yet. In PRACTICE, set A and B on a song and press SAVE."
                } else {
                    "Nothing this week yet. Practise a loop, or save one, and the song shows up here. ALL has every song with a saved loop."
                },
                pal().ink2,
            );
            return;
        }
        let playing = self.cur_track().map(|t| t.id);
        for (k, id) in ids.iter().enumerate() {
            let v = self.store.songs.get(id).cloned();
            let name = v
                .as_ref()
                .map(|v| format!("{} - {}", v.artist, v.title))
                .unwrap_or_else(|| "A SONG (PLAY IT ONCE TO NAME IT)".to_string());
            let mins = week.get(id).copied().unwrap_or(0);
            let state = if playing == Some(*id) { RowState::Playing } else { RowState::Normal };
            let r = list_row(
                ui,
                k,
                None,
                || name.clone(),
                || if mins >= 60 { format!("{} THIS WEEK", mins_text(mins)) } else { String::new() },
                state,
                false,
                false,
            );
            if let (Some(r), Some(v)) = (r, &v) {
                if r.clicked() {
                    acts.push(Action::PlaySaved(v.clone(), None));
                }
                let (liked, tidal) = (self.liked.contains(&v.id), v.kind == "tidal");
                r.context_menu(|ui| {
                    crate::song_items(ui, acts, &v.to_track(), liked, tidal);
                });
            }
            let loops = self.store.sections.get(id).cloned().unwrap_or_default();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                ui.add_space(28.0);
                if loops.is_empty() {
                    dim_line(ui, "NO LOOPS SAVED - PRACTISED ONLY", 1.0, pal().dim);
                }
                for (i, s) in loops.iter().enumerate() {
                    let label = format!("{}  {}-{}", s.name.to_uppercase(), fmt_time(s.a), fmt_time(s.b));
                    let b = retro_btn(ui, &label, false).tip("Play the song at this loop, looping. Right-click to forget it");
                    if b.clicked() {
                        if let Some(v) = &v {
                            acts.push(Action::PlaySaved(v.clone(), Some((s.a, s.b))));
                        }
                    }
                    b.context_menu(|ui| {
                        if let Some(v) = &v {
                            if menu_item(ui, "Add this loop to a playlist...") {
                                acts.push(Action::PlaylistPickLoop(v.to_track(), s.a, s.b, s.name.clone()));
                                ui.close_menu();
                            }
                        }
                        if menu_item(ui, "Forget this loop") {
                            acts.push(Action::ForgetLoop(*id, i));
                            ui.close_menu();
                        }
                    });
                }
            });
            ui.add_space(8.0);
        }
        ui.add_space(20.0);
    }

    /// Every loop on its own, newest first: its name, the song it is from, where, and your note. Click to play it;
    /// right-click to rename it, note it, play the whole song or forget it.
    fn clips_list(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, ids: &[i64], week_start: &str) {
        let mut clips: Vec<(i64, usize, store::Section)> = Vec::new();
        for id in ids {
            for (i, s) in self.store.sections.get(id).into_iter().flatten().enumerate() {
                // this week: loops saved this week, or on a song practised this week
                if self.loops_all || s.saved.as_str() >= week_start || s.saved.is_empty() {
                    clips.push((*id, i, s.clone()));
                }
            }
        }
        // newest first; loops saved the same day in a fixed order (the songs come from a map, in no set order)
        clips.sort_by(|a, b| b.2.saved.cmp(&a.2.saved).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
        if clips.is_empty() {
            para(
                ui,
                "No loops here yet. In PRACTICE, set A and B on a song, name it (FUNKY RIFF...) and press SAVE.",
                pal().ink2,
            );
            return;
        }
        let playing = (self.cur_track().map(|t| t.id), self.loop_a, self.loop_b);
        for (k, (id, i, s)) in clips.iter().enumerate() {
            let v = self.store.songs.get(id).cloned();
            let from = v
                .as_ref()
                .map(|v| format!("{} - {}", v.artist, v.title))
                .unwrap_or_else(|| "a song (play it once to name it)".to_string());
            let on = playing.0 == Some(*id)
                && playing.1.is_some_and(|a| (a - s.a).abs() < 0.05)
                && playing.2.is_some_and(|b| (b - s.b).abs() < 0.05);
            let h = if s.note.is_empty() { 44.0 } else { 62.0 };
            let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::click());
            let p = ui.painter();
            if on {
                p.rect_filled(r, 2.0, pal().row_sel);
            } else if resp.hovered() {
                p.rect_filled(r, 2.0, pal().sel);
            } else if k % 2 == 1 {
                p.rect_filled(r, 2.0, pal().row_alt);
            }
            let x = r.min.x + 12.0;
            let col = if on { pal().bar_txt } else { pal().ink };
            ptext(p, Pos2::new(x, r.min.y + 13.0), Align::Min, &s.name.to_uppercase(), 2.0, col);
            let when = if s.saved.is_empty() { String::new() } else { format!("   SAVED {}", s.saved) };
            let when = if s.group.is_empty() { when } else { format!("   IN {}{}", s.group.to_uppercase(), when) };
            let sub = format!("FROM {}   {}-{}{}", from.to_uppercase(), fmt_time(s.a), fmt_time(s.b), when);
            ptext_fit(p, Pos2::new(x, r.min.y + 31.0), Align::Min, &sub, 1.0, r.width() - 24.0, pal().ink2);
            if !s.note.is_empty() {
                ptext_fit(p, Pos2::new(x, r.min.y + 48.0), Align::Min, &s.note, 2.0, r.width() - 24.0, pal().dim);
            }
            let resp = resp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .tip("Play it, looping. Right-click to rename it or add a note");
            if resp.clicked() {
                if let Some(v) = &v {
                    acts.push(Action::PlaySaved(v.clone(), Some((s.a, s.b))));
                }
            }
            let liked = v.as_ref().is_some_and(|v| self.liked.contains(&v.id));
            resp.context_menu(|ui| {
                if menu_item(ui, "Rename / add a note...") {
                    self.clip_edit = Some((*id, *i));
                    self.clip_name = s.name.clone();
                    self.clip_note = s.note.clone();
                    self.clip_group = s.group.clone();
                    ui.close_menu();
                }
                if let Some(v) = &v {
                    if menu_item(ui, "Add this loop to a playlist...") {
                        acts.push(Action::PlaylistPickLoop(v.to_track(), s.a, s.b, s.name.clone()));
                        ui.close_menu();
                    }
                    if menu_item(ui, "Play the whole song") {
                        acts.push(Action::PlaySaved(v.clone(), None));
                        ui.close_menu();
                    }
                    crate::song_items(ui, acts, &v.to_track(), liked, v.kind == "tidal");
                }
                if menu_item(ui, "Forget this loop") {
                    acts.push(Action::ForgetLoop(*id, *i));
                    ui.close_menu();
                }
            });
            // renaming / noting this one
            if self.clip_edit == Some((*id, *i)) {
                self.loop_editor(ui, acts);
            }
        }
        ui.add_space(20.0);
    }

    /// Loops saved before songs were remembered: name them from wherever the song is known (tunes, files, lists,
    /// the queue).
    fn name_old_loops(&mut self) {
        let missing: Vec<i64> = self.store.sections.keys().copied().filter(|id| !self.store.songs.contains_key(id)).collect();
        for id in missing {
            let found = self
                .store
                .tunes
                .iter()
                .flat_map(|t| t.versions.iter())
                .find(|v| v.id == id)
                .cloned()
                .or_else(|| {
                    self.store
                        .files
                        .iter()
                        .chain(&self.store.yt)
                        .chain(&self.store.sc)
                        .chain(&self.store.hearts)
                        .find(|e| e.id == id)
                        .map(|e| e.to_version())
                })
                .or_else(|| self.store.last.as_ref().and_then(|l| l.queue.iter().find(|v| v.id == id).cloned()))
                .or_else(|| self.queue.iter().find(|t| t.id == id).map(|t| self.version_of(t)));
            if let Some(v) = found {
                self.store.songs.insert(id, v);
                self.store_dirty = true;
            }
        }
    }

    pub(crate) fn diary_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "PRACTICE DIARY", 3.0, pal().ink);
        let today = store::today_days();
        if self.diary_sel == 0 {
            self.diary_sel = today;
        }
        if self.diary_ym.0 == 0 {
            let (y, m, _) = store::civil_from_days(self.diary_sel);
            self.diary_ym = (y, m);
        }
        let live = self.acc as u32;
        let day_secs = |app: &App, d: i64| app.store.secs_on(&store::date_str(d)) + if d == today { live } else { 0 };

        // ---- summary
        let week: u32 = (0..7).map(|k| day_secs(self, today - k)).sum();
        let (cy, cm, _) = store::civil_from_days(today);
        let month_prefix = format!("{:04}-{:02}", cy, cm);
        let month: u32 = self
            .store
            .days
            .iter()
            .filter(|d| d.date.starts_with(&month_prefix))
            .map(|d| d.secs.values().sum::<u32>())
            .sum::<u32>()
            + live;
        let total: u32 = self.store.days.iter().map(|d| d.secs.values().sum::<u32>()).sum::<u32>() + live;
        let streak = self.store.streak(today);
        let tiles = [
            ("TODAY", mins_text(day_secs(self, today))),
            ("7 DAYS", mins_text(week)),
            ("THIS MONTH", mins_text(month)),
            ("STREAK", format!("{} DAY{}", streak, if streak == 1 { "" } else { "S" })),
            ("ALL TIME", mins_text(total)),
        ];
        let w = ui.available_width();
        let tw = ((w - 4.0 * 6.0) / 5.0).floor().max(60.0);
        let (sr, _) = ui.allocate_exact_size(Vec2::new(w, 46.0), Sense::hover());
        for (k, (name, val)) in tiles.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(sr.min.x + k as f32 * (tw + 6.0), sr.min.y), Vec2::new(tw, 46.0));
            inset(ui.painter(), r, pal().lcd);
            ptext(ui.painter(), Pos2::new(r.center().x, r.min.y + 13.0), Align::Center, name, 1.0, pal().dim);
            ptext_fit(ui.painter(), Pos2::new(r.center().x, r.min.y + 31.0), Align::Center, val, 2.0, tw - 8.0, pal().ink);
        }

        // ---- calendar
        ui.add_space(8.0);
        let (y, m) = self.diary_ym;
        const MONTHS: [&str; 12] = [
            "JANUARY",
            "FEBRUARY",
            "MARCH",
            "APRIL",
            "MAY",
            "JUNE",
            "JULY",
            "AUGUST",
            "SEPTEMBER",
            "OCTOBER",
            "NOVEMBER",
            "DECEMBER",
        ];
        ui.horizontal_wrapped(|ui| {
            if retro_btn_w(ui, "<", 34.0, false).tip("Previous month").clicked() {
                self.diary_ym = if m == 1 { (y - 1, 12) } else { (y, m - 1) };
            }
            let (r, _) = ui.allocate_exact_size(Vec2::new(190.0, bh()), Sense::hover());
            ptext(ui.painter(), r.center(), Align::Center, &format!("{} {}", MONTHS[(m as usize - 1) % 12], y), 2.0, pal().ink);
            if retro_btn_w(ui, ">", 34.0, false).tip("Next month").clicked() {
                self.diary_ym = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
            }
            if retro_btn(ui, "TODAY", false).tip("Jump to today").clicked() {
                self.diary_sel = today;
                self.diary_ym = (cy, cm);
            }
        });
        ui.add_space(4.0);
        let first = store::days_from_civil(y, m, 1);
        let next = if m == 12 { store::days_from_civil(y + 1, 1, 1) } else { store::days_from_civil(y, m + 1, 1) };
        let ndays = next - first;
        let lead = (first + 4).rem_euclid(7);
        let gw = ui.available_width().min(7.0 * 64.0);
        let cw = (gw / 7.0).floor();
        let rows = (lead + ndays + 6) / 7;
        let (gr, _) = ui.allocate_exact_size(Vec2::new(cw * 7.0, 16.0 + rows as f32 * 32.0), Sense::hover());
        for (k, n) in ["S", "M", "T", "W", "T", "F", "S"].iter().enumerate() {
            ptext(ui.painter(), Pos2::new(gr.min.x + k as f32 * cw + cw / 2.0, gr.min.y + 7.0), Align::Center, n, 1.0, pal().dim);
        }
        let busiest = (0..ndays).map(|k| day_secs(self, first + k)).max().unwrap_or(0).max(1);
        for k in 0..ndays {
            let d = first + k;
            let pos = lead + k;
            let r = Rect::from_min_size(
                Pos2::new(gr.min.x + (pos % 7) as f32 * cw + 2.0, gr.min.y + 16.0 + (pos / 7) as f32 * 32.0 + 2.0),
                Vec2::new(cw - 4.0, 28.0),
            );
            let secs = day_secs(self, d);
            let resp =
                ui.interact(r, ui.id().with(("diaryday", d)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let level = if secs == 0 { 0 } else { 1 + (secs * 3 / busiest).min(2) };
            let face = match level {
                0 => pal().lcd,
                1 => pal().row_alt,
                2 => pal().ink2,
                _ => pal().ink,
            };
            inset(ui.painter(), r, face);
            if d == self.diary_sel {
                outline(ui.painter(), r, 2.0, pal().red);
            } else if d == today {
                outline(ui.painter(), r, 1.0, pal().ink);
            }
            let txt_col = if level >= 2 { pal().lcd } else { pal().ink };
            ptext(ui.painter(), Pos2::new(r.min.x + 5.0, r.min.y + 8.0), Align::Min, &format!("{}", k + 1), 1.0, txt_col);
            if secs >= 60 {
                ptext(
                    ui.painter(),
                    Pos2::new(r.max.x - 4.0, r.max.y - 7.0),
                    Align::Max,
                    &format!("{}m", secs / 60),
                    1.0,
                    txt_col,
                );
            }
            if resp.clicked() {
                self.diary_sel = d;
            }
        }

        // ---- the picked day
        let sel = self.diary_sel;
        let date = store::date_str(sel);
        ui.add_space(6.0);
        section_header(ui, &format!("{} {}  -  {}", store::weekday(sel), date, mins_text(day_secs(self, sel))));
        let (mut rows_d, mut loops, mut pomos) = (Vec::new(), 0, 0);
        if let Some(d) = self.store.days.iter().find(|d| d.date == date) {
            rows_d = d.secs.iter().map(|(k, v)| (k.clone(), *v)).collect::<Vec<_>>();
            (loops, pomos) = (d.loops, d.pomos);
        }
        rows_d.sort_by(|a, b| b.1.cmp(&a.1));
        let tot: u32 = rows_d.iter().map(|r| r.1).sum();
        if rows_d.is_empty() {
            dim_line(ui, "NO PRACTICE TIME TRACKED THIS DAY", 2.0, pal().ink2);
        }
        for (k, (name, s)) in rows_d.iter().take(12).enumerate() {
            let _ = list_row(
                ui,
                k,
                None,
                || name.clone(),
                || format!("{}  {}%", mins_text(*s), s * 100 / tot.max(1)),
                RowState::Normal,
                false,
                false,
            );
        }
        if loops + pomos > 0 {
            dim_line(ui, &format!("LOOPS {}   FOCUS BLOCKS {}", loops, pomos), 2.0, pal().ink2);
        }
        // the songs worked on that day (tunes or free practice); click one to play it
        let mut songs: Vec<(i64, u32)> = self
            .store
            .days
            .iter()
            .find(|d| d.date == date)
            .map(|d| d.songs.iter().map(|(k, v)| (*k, *v)).collect())
            .unwrap_or_default();
        songs.sort_by(|a, b| b.1.cmp(&a.1));
        if !songs.is_empty() {
            ui.add_space(4.0);
            dim_line(ui, "SONGS PRACTISED", 1.0, pal().dim);
            for (k, (id, s)) in songs.iter().enumerate() {
                let v = self.store.songs.get(id).cloned();
                let name = v.as_ref().map(|v| format!("{} - {}", v.artist, v.title)).unwrap_or_else(|| "A SONG".to_string());
                let n_loops = self.store.sections.get(id).map_or(0, |l| l.len());
                let r = list_row(
                    ui,
                    k,
                    None,
                    || name.clone(),
                    || {
                        if n_loops > 0 {
                            format!("{}  {} LOOP{}", mins_text(*s), n_loops, if n_loops == 1 { "" } else { "S" })
                        } else {
                            mins_text(*s)
                        }
                    },
                    RowState::Normal,
                    false,
                    false,
                );
                if let (Some(r), Some(v)) = (r, v) {
                    if r.tip("Play it").clicked() {
                        acts.push(Action::PlaySaved(v, None));
                    }
                }
            }
        }

        let mins_hint = format!("{}", day_secs(self, sel) / 60);
        // ---- journal for that day
        ui.add_space(4.0);
        if self.show_form {
            let hint_t = self.practice_label();
            let _ = field_row(ui, &mut self.ed, F_TUNE, "TUNE", &mut self.f_tune, &hint_t, None);
            let _ = field_row(ui, &mut self.ed, F_MINS, "MINUTES", &mut self.f_mins, &mins_hint, None);
            let _ = field_row(ui, &mut self.ed, F_BPM, "BPM", &mut self.f_bpm, "tempo you reached", None);
            let _ = field_row(ui, &mut self.ed, F_NOTE, "NOTE", &mut self.f_note, "What went well? What next?", Some(70.0));
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "SAVE ENTRY", false).clicked() {
                    acts.push(Action::SaveEntry);
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.show_form = false;
                    self.ed.id = 0;
                }
            });
        } else if retro_btn(ui, &format!("LOG PRACTICE FOR {}", date), false)
            .tip("Write down what you worked on - works for past days too")
            .clicked()
        {
            self.show_form = true;
        }
        ui.add_space(6.0);
        let mut any = false;
        for e in self.store.entries.iter().rev().filter(|e| e.date == date) {
            any = true;
            let head = format!(
                "{}{}{}",
                e.tune,
                if e.mins > 0 { format!("  {}m", e.mins) } else { String::new() },
                if e.bpm > 0 { format!("  {} bpm", e.bpm) } else { String::new() }
            );
            dim_line(ui, &head, 2.0, pal().ink);
            if !e.note.is_empty() {
                para(ui, &e.note, pal().ink2);
            }
        }
        if !any && self.store.entries.is_empty() {
            para(ui, "Practice time adds up by itself while you work on a loop in PRACTICE mode. Pick any day above and use LOG PRACTICE to write down what you worked on and the tempo you reached.", pal().ink2);
        }
        ui.add_space(20.0);
    }

    // ------------------------------------------------------------- PRACTICE
    /// Transcribing tools: waveform, A-B loop, saved sections, slow-down and the extras behind MORE.
    pub(crate) fn practice_ui(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let inner = ui.max_rect();
        window_deco(ui, inner, "PRACTICE");
        // the focus timer and the metronome have their own corner, in the title strip
        let tools = Rect::from_min_size(Pos2::new(inner.max.x - 96.0, inner.min.y - 27.0), Vec2::new(96.0, 24.0));
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(tools), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.horizontal(|ui| {
                let ink = pal().ink;
                let tb = crate::icon_btn_w(ui, &crate::IC_TOMATO, self.pomo > 0 || self.timer_open, ink, 44.0)
                    .tip("Focus timer (pomodoro): work in blocks with short rests between");
                if tb.clicked() {
                    acts.push(Action::TimerPanel);
                }
                let mb = crate::icon_btn_w(ui, &crate::IC_METRO, self.metro_open, ink, 44.0)
                    .tip("Metronome with beats, subdivisions and a pendulum");
                if mb.clicked() {
                    acts.push(Action::MetroPanel);
                }
                crate::tour::mark("TIMEBTNS", tb.rect.union(mb.rect));
            });
        });
        // the panel's contents scroll when the panel is short, rather than running past its edge
        // (no drag-to-scroll: dragging on the waveform sets loops)
        // everything inside grows and shrinks with the panel, gently (the square root of the size change), from a
        // little smaller to about twice as big
        let size = ui.available_size();
        let s = (size.y / 250.0).min(size.x / 760.0).max(0.1).sqrt().clamp(0.85, 2.0);
        // the waveform's height is yours to drag (the handle under it)
        let wave_h = 54.0 * s * self.wv_h.clamp(0.6, 4.0);
        // the contents stop a little above the window's bottom edge, so they never run into it
        let r = ui.available_rect_before_wrap();
        let body = Rect::from_min_max(r.min, Pos2::new(r.max.x, r.max.y - 6.0));
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body), |ui| {
            crate::font::with_scale(s, || {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .drag_to_scroll(false)
                    .show(ui, |ui| self.practice_body(ui, acts, wave_h));
            });
        });
    }

    /// A bookmark on this song at `t`: the colour picked for them, or a random one of the swatches.
    pub(crate) fn add_mark(&mut self, id: i64, t: f32) {
        let col = self.mark_col.unwrap_or_else(|| {
            let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
            let used: Vec<[u8; 3]> = self.store.marks.get(&id).map(|l| l.iter().map(|m| m.col).collect()).unwrap_or_default();
            // a colour not on this song yet, when there is one
            let free: Vec<[u8; 3]> = MARK_COLS.iter().copied().filter(|c| !used.contains(c)).collect();
            let pool = if free.is_empty() { MARK_COLS.to_vec() } else { free };
            pool[n as usize % pool.len()]
        });
        let list = self.store.marks.entry(id).or_default();
        list.push(crate::store::Bookmark { t, col, name: String::new() });
        list.sort_by(|a, b| a.t.total_cmp(&b.t));
        self.store_dirty = true;
        self.set_note(&format!("BOOKMARK AT {}", fmt_t(t)));
        self.keep_for_practice(id);
    }

    fn practice_body(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, wave_h: f32) {
        let dur = self.track_len();
        let pos = self.pos();
        let tid = self.cur_track().map(|t| t.id);
        let active = self.cur.is_some() && !self.stopped && dur > 0.0;
        let shown = self.seek_drag.unwrap_or(pos);
        let both = self.loop_a.is_some() && self.loop_b.is_some();
        let shift = ui.input(|i| i.modifiers.shift);

        // ---- waveform timeline
        let (bar, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), wave_h), Sense::click_and_drag());
        // a handle along its bottom edge: drag it down for a taller waveform, up for a shorter one; double-click resets
        {
            let grip = Rect::from_min_max(Pos2::new(bar.min.x, bar.max.y - 2.0), Pos2::new(bar.max.x, bar.max.y + 5.0));
            let g = ui
                .interact(grip, ui.id().with("wv_height"), Sense::click_and_drag())
                .on_hover_cursor(egui::CursorIcon::ResizeVertical)
                .tip("Drag to make the waveform taller or shorter (double-click: back to normal)");
            if g.dragged() {
                let base = wave_h / self.wv_h.clamp(0.6, 4.0);
                self.wv_h = (self.wv_h + g.drag_delta().y / base.max(1.0)).clamp(0.6, 4.0);
                self.dirty = true;
            }
            if g.double_clicked() {
                self.wv_h = 1.0;
                self.dirty = true;
            }
            let shade = if g.hovered() || g.dragged() { pal().ink } else { pal().ink2.gamma_multiply(0.5) };
            ui.painter().rect_filled(
                Rect::from_center_size(Pos2::new(bar.center().x, bar.max.y + 1.5), Vec2::new(36.0, 3.0)),
                1.0,
                shade,
            );
        }
        let resp = resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("Click to jump. Shift + drag to select a loop (inside a loop: slide it). Ctrl + drag A or B to move it. Alt + click for a bookmark (on one: remove it). Right-click for more.");
        inset(ui.painter(), bar, pal().lcd);
        let inn = bar.shrink2(Vec2::new(5.0, 5.0));
        // zoom (like Audacity's): the mouse wheel over the waveform zooms in and out where the mouse is; + and -
        // zoom where the song is playing, FIT shows it all. Only the time gets finer - the height stays
        let dur1 = dur.max(0.001);
        let mut zoom = self.wv_zoom.clamp(1.0, 400.0);
        let mut start = self.wv_start;
        let wheel = if resp.hovered() && active { ui.input(|i| i.smooth_scroll_delta.y) } else { 0.0 };
        if wheel != 0.0 {
            // the wheel is the waveform's here, not the panel's scroll
            ui.input_mut(|i| {
                i.smooth_scroll_delta = Vec2::ZERO;
                i.raw_scroll_delta = Vec2::ZERO;
            });
            if let Some(m) = ui.input(|i| i.pointer.hover_pos()) {
                let frac = ((m.x - inn.min.x) / inn.width()).clamp(0.0, 1.0);
                let under = start + frac * dur1 / zoom;
                zoom = (zoom * (wheel / 120.0).exp2()).clamp(1.0, 400.0);
                start = under - frac * dur1 / zoom;
            }
        }
        // the zoom buttons, each as wide as its text (at any text size), one after the other
        let chip_w = |t: &str| (crate::font::text_w(t, 1.0) + 12.0).max(18.0);
        let chip_at = |k: f32, w: f32| Rect::from_min_size(Pos2::new(bar.min.x + 6.0 + k, bar.max.y - 24.0), Vec2::new(w, 16.0));
        let span = dur1 / zoom;
        // zoomed: the view keeps up with the song (turning the page as it reaches the edge), unless you are dragging
        let dragging = self.sel_anchor.is_some() || self.marker_drag != 0 || self.seek_drag.is_some() || wheel != 0.0;
        // clicking the waveform or zooming: the view follows the song again
        if resp.clicked() || resp.drag_started() || wheel != 0.0 {
            self.wv_free = false;
        }
        if zoom > 1.0 && !dragging && !self.wv_free && (shown < start || shown > start + span) {
            start = shown - span * 0.05;
        }
        let start = start.clamp(0.0, (dur1 - span).max(0.0));
        self.wv_zoom = zoom;
        self.wv_start = start;
        let at = |t: f32| inn.min.x + inn.width() * ((t - start) / span).clamp(0.0, 1.0);
        let time_at = |x: f32| start + ((x - inn.min.x) / inn.width()).clamp(0.0, 1.0) * span;
        if active {
            let p = ui.painter().clone();
            // waveform
            if let (Some(id), (wid, Some(w))) = (tid, &self.wave) {
                if *wid == id && w.len() > 2 {
                    let cols = (inn.width() / 2.0).max(1.0) as usize;
                    let mid = inn.center().y;
                    // the part of the song in view
                    let n = w.len() as f32;
                    let (v0, v1) = (start / dur1 * n, (start + span) / dur1 * n);
                    for c in 0..cols {
                        let a = (v0 + (v1 - v0) * c as f32 / cols as f32) as usize;
                        let b = ((v0 + (v1 - v0) * (c + 1) as f32 / cols as f32) as usize).max(a + 1).min(w.len());
                        let m = w[a.min(w.len() - 1)..b].iter().copied().max().unwrap_or(0);
                        let h = (m as f32 / 255.0) * (inn.height() * 0.5);
                        let x = inn.min.x + c as f32 * 2.0;
                        fill_rect(
                            &p,
                            Rect::from_min_max(Pos2::new(x, mid - h - 0.5), Pos2::new(x + 1.5, mid + h + 0.5)),
                            pal().ink2.gamma_multiply(0.55),
                        );
                    }
                }
            }
            // saved sections as faint bands along the bottom
            if let Some(list) = tid.filter(|id| !self.is_demo(*id)).and_then(|id| self.store.sections.get(&id)) {
                for s in list {
                    fill_rect(
                        &p,
                        Rect::from_min_max(Pos2::new(at(s.a), inn.max.y - 3.0), Pos2::new(at(s.b).max(at(s.a) + 1.0), inn.max.y)),
                        pal().ink.gamma_multiply(0.6),
                    );
                }
            }
            if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
                let col = if self.loop_on { crate::skin::loop_col().gamma_multiply(0.35) } else { pal().ink.gamma_multiply(0.2) };
                fill_rect(&p, Rect::from_min_max(Pos2::new(at(a), inn.min.y), Pos2::new(at(b), inn.max.y)), col);
            }
            for (v, lab) in [(self.loop_a, "A"), (self.loop_b, "B")] {
                if let Some(t) = v {
                    fill_rect(
                        &p,
                        Rect::from_min_max(Pos2::new(at(t) - 1.0, bar.min.y + 2.0), Pos2::new(at(t) + 1.0, bar.max.y - 2.0)),
                        crate::skin::loop_col(),
                    );
                    ptext(&p, Pos2::new(at(t) + 4.0, inn.min.y + 5.0), Align::Min, lab, 1.0, crate::skin::loop_col());
                }
            }
            fill_rect(
                &p,
                Rect::from_min_max(Pos2::new(at(shown) - 1.0, bar.min.y + 2.0), Pos2::new(at(shown) + 1.0, bar.max.y - 2.0)),
                pal().ink,
            );
            ptext(
                &p,
                Pos2::new(bar.max.x - 8.0, bar.min.y + 8.0),
                Align::Max,
                &format!("{} / {}", fmt_t(shown), fmt_t(dur)),
                1.0,
                pal().ink,
            );
            // time marks along the top, as close as the zoom allows (a second, a tenth of one...)
            let step = [0.1f32, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0]
                .into_iter()
                .find(|s| span / s <= 9.0)
                .unwrap_or(600.0);
            let mut t = (start / step).ceil() * step;
            // (no labels under the time readout in the corner)
            let readout = bar.max.x - 8.0 - crate::font::text_w(&format!("{} / {}", fmt_t(shown), fmt_t(dur)), 1.0) - 40.0;
            while t <= start + span + 1e-3 {
                let x = at(t);
                fill_rect(&p, Rect::from_min_max(Pos2::new(x, bar.min.y + 2.0), Pos2::new(x + 1.0, bar.min.y + 7.0)), pal().ink);
                if x < readout {
                    let (m, s) = ((t / 60.0).floor() as u32, t % 60.0);
                    let lab = if step < 1.0 { format!("{}:{:04.1}", m, s) } else { format!("{}:{:02}", m, s.round() as u32) };
                    let lw = crate::font::text_w(&lab, 0.7);
                    p.rect_filled(
                        Rect::from_min_size(Pos2::new(x + 1.5, bar.min.y + 2.0), Vec2::new(lw + 4.0, 10.0)),
                        1.0,
                        egui::Color32::from_black_alpha(190),
                    );
                    ptext(&p, Pos2::new(x + 3.0, bar.min.y + 7.0), Align::Min, &lab, 0.7, egui::Color32::from_gray(235));
                }
                t += step;
            }
            // zoomed: a scrollbar along the bottom to move through the song (drag it, or click where to look)
            if self.wv_zoom > 1.0 {
                let track = Rect::from_min_max(Pos2::new(inn.min.x, bar.max.y - 7.0), Pos2::new(inn.max.x, bar.max.y - 3.0));
                let r = ui
                    .interact(track.expand2(Vec2::new(0.0, 4.0)), ui.id().with("wv_scroll"), Sense::click_and_drag())
                    .on_hover_cursor(egui::CursorIcon::Grab)
                    .tip("Drag to look through the song (it keeps playing)");
                fill_rect(&p, track, pal().ink.gamma_multiply(0.15));
                let w = (track.width() * span / dur1).max(14.0);
                let x0 = track.min.x + (track.width() - w) * (start / (dur1 - span).max(0.001));
                let thumb = Rect::from_min_size(Pos2::new(x0, track.min.y - 1.0), Vec2::new(w, track.height() + 2.0));
                fill_rect(&p, thumb, if r.hovered() || r.dragged() { pal().ink } else { pal().ink2 });
                if let Some(pp) = r.interact_pointer_pos().filter(|_| r.dragged() || r.clicked()) {
                    let frac = ((pp.x - track.min.x - w / 2.0) / (track.width() - w).max(1.0)).clamp(0.0, 1.0);
                    self.wv_start = frac * (dur1 - span).max(0.0);
                    self.wv_free = true;
                }
            }
            // zoom buttons, over the waveform's corner (they take effect from the next frame)
            let zoom_by = |me: &mut Self, by: f32| {
                let frac = ((shown - start) / span).clamp(0.0, 1.0);
                me.wv_zoom = (me.wv_zoom * by).clamp(1.0, 400.0);
                me.wv_start = shown - frac * dur1 / me.wv_zoom;
                me.wv_free = false;
            };
            let (w1, w2, w3) = (chip_w("-"), chip_w("+"), chip_w("FIT"));
            // bookmarks: + MARK drops one where the song is; right-click it for the colour they get
            if let Some(id) = tid.filter(|id| !self.is_demo(*id)) {
                let w4 = chip_w("+ MARK");
                let mb = crate::chip(ui, chip_at(w1 + w2 + w3 + 9.0, w4), "+ MARK", false, "wv_mark").tip(
                    "A bookmark where the song is now (or the K key, or alt + click on the waveform). Right-click: their colour. Click a bookmark's flag to jump to it, right-click it to recolour or remove it",
                );
                if mb.clicked() {
                    self.add_mark(id, shown);
                }
                let one = self.mark_col;
                mb.context_menu(|ui| {
                    if menu_item(ui, if one.is_none() { "* Random colours" } else { "  Random colours" }) {
                        self.mark_col = None;
                        self.dirty = true;
                        ui.close_menu();
                    }
                    crate::views::dim_line(ui, "OR ALL ONE COLOUR", 1.0, pal().dim);
                    if let Some(c) = mark_swatches(ui, one) {
                        self.mark_col = Some(c);
                        self.dirty = true;
                        ui.close_menu();
                    }
                    if self.store.marks.get(&id).is_some_and(|m| !m.is_empty()) && menu_item(ui, "Remove this song's bookmarks") {
                        self.store.marks.remove(&id);
                        self.store_dirty = true;
                        ui.close_menu();
                    }
                });
                // each bookmark: a line in its colour and a numbered flag along the bottom
                let marks = self.store.marks.get(&id).cloned().unwrap_or_default();
                let mut drop = None;
                let mut recolour = None;
                let mut rename = None;
                for (k, m) in marks.iter().enumerate() {
                    if m.t < start || m.t > start + span {
                        continue;
                    }
                    let col = egui::Color32::from_rgb(m.col[0], m.col[1], m.col[2]);
                    let x = at(m.t);
                    fill_rect(
                        &p,
                        Rect::from_min_max(Pos2::new(x - 1.0, bar.min.y + 12.0), Pos2::new(x + 1.0, bar.max.y - 2.0)),
                        col,
                    );
                    // a short flag (the number, or the name cut to fit, never past the next bookmark); the whole
                    // name is in its tip
                    let next_x = marks.get(k + 1).map_or(inn.max.x, |n| at(n.t));
                    let room = (next_x - x - 3.0).clamp(14.0, 70.0);
                    let full = if m.name.is_empty() { (k + 1).to_string() } else { m.name.clone() };
                    let lab = crate::font::fit(&full, 1.0, room - 8.0);
                    let fw = (crate::font::text_w(&lab, 1.0) + 8.0).min(room);
                    let flag = Rect::from_min_size(Pos2::new(x, bar.max.y - 40.0), Vec2::new(fw, 13.0));
                    p.rect_filled(flag, 1.0, col);
                    ptext(&p, flag.center(), Align::Center, &lab, 1.0, egui::Color32::from_gray(12));
                    let fr = ui
                        .interact(flag.expand(2.0), ui.id().with(("wv_bm", k)), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .tip(format!(
                            "{}Bookmark {} at {} - click to jump, alt + click to remove, right-click to rename or recolour",
                            if m.name.is_empty() {
                                String::new()
                            } else {
                                format!(
                                    "{}
",
                                    m.name
                                )
                            },
                            k + 1,
                            fmt_t(m.t)
                        ));
                    if fr.clicked() && ui.input(|i| i.modifiers.alt) {
                        drop = Some(k);
                    } else if fr.clicked() {
                        acts.push(Action::Seek(m.t));
                    }
                    fr.context_menu(|ui| {
                        if menu_item(ui, "Rename...") {
                            rename = Some(k);
                            ui.close_menu();
                        }
                        if let Some(c) = mark_swatches(ui, Some(m.col)) {
                            recolour = Some((k, c));
                            ui.close_menu();
                        }
                        if menu_item(ui, "Remove this bookmark") {
                            drop = Some(k);
                            ui.close_menu();
                        }
                    });
                }
                // rename: the BOOKMARKS tab, its name ready to type
                if let Some(k) = rename {
                    self.more = true;
                    self.mtab = 3;
                    self.ed.id = 5000 + k as u32;
                }
                if let Some(list) = self.store.marks.get_mut(&id) {
                    if let Some((k, c)) = recolour {
                        list[k].col = c;
                        self.store_dirty = true;
                    }
                    if let Some(k) = drop {
                        list.remove(k);
                        self.store_dirty = true;
                    }
                }
            }
            if crate::chip(ui, chip_at(0.0, w1), "-", false, "wv_out").tip("Zoom out").clicked() {
                zoom_by(self, 0.5);
            }
            if crate::chip(ui, chip_at(w1 + 3.0, w2), "+", false, "wv_in")
                .tip("Zoom in where it is playing (or the mouse wheel over the waveform)")
                .clicked()
            {
                zoom_by(self, 2.0);
            }
            if self.wv_zoom > 1.0
                && crate::chip(ui, chip_at(w1 + w2 + 6.0, w3), "FIT", false, "wv_fit").tip("The whole song again").clicked()
            {
                self.wv_zoom = 1.0;
                self.wv_start = 0.0;
                self.wv_free = false;
            }

            // shift + drag selects a loop; plain click / drag seeks
            if resp.drag_started() && shift {
                if let Some(pp) = resp.interact_pointer_pos() {
                    self.sel_anchor = Some(time_at(pp.x));
                }
            }
            // ctrl + drag near A or B moves that marker
            let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
            if resp.drag_started() && ctrl {
                if let Some(pp) = resp.interact_pointer_pos() {
                    self.marker_drag = [(1u8, self.loop_a), (2u8, self.loop_b)]
                        .into_iter()
                        .filter_map(|(k, v)| v.map(|t| (k, (at(t) - pp.x).abs())))
                        .filter(|c| c.1 < 14.0)
                        .min_by(|a, b| a.1.total_cmp(&b.1))
                        .map_or(0, |c| c.0);
                }
            }
            // shift + drag inside the loop slides the whole loop, keeping its length
            if resp.drag_started() && shift && !ctrl {
                if let (Some(a), Some(b), Some(pp)) = (self.loop_a, self.loop_b, resp.interact_pointer_pos()) {
                    let t = time_at(pp.x);
                    if t > a && t < b {
                        self.marker_drag = 3;
                        self.pan_off = t - a;
                        self.pan_len = b - a;
                        self.sel_anchor = None;
                    }
                }
            }
            if self.marker_drag != 0 {
                if let Some(pp) = ui.input(|i| i.pointer.hover_pos()) {
                    let t = time_at(pp.x);
                    if self.marker_drag == 3 {
                        let a = (t - self.pan_off).clamp(0.0, (dur - self.pan_len).max(0.0));
                        acts.push(Action::SetAAt(a));
                        acts.push(Action::SetBAt(a + self.pan_len));
                    } else if self.marker_drag == 1 {
                        acts.push(Action::SetAAt(self.loop_b.map_or(t, |b| t.min(b - 0.2))));
                    } else {
                        acts.push(Action::SetBAt(self.loop_a.map_or(t, |a| t.max(a + 0.2))));
                    }
                }
                if resp.drag_stopped() {
                    self.marker_drag = 0;
                }
            } else if let Some(anc) = self.sel_anchor {
                if let Some(pp) = ui.input(|i| i.pointer.hover_pos()) {
                    let (x1, x2) = (at(anc), pp.x.clamp(inn.min.x, inn.max.x));
                    let (lo, hi) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
                    fill_rect(
                        &p,
                        Rect::from_min_max(Pos2::new(lo, inn.min.y), Pos2::new(hi, inn.max.y)),
                        crate::skin::loop_col().gamma_multiply(0.3),
                    );
                }
                if resp.drag_stopped() {
                    if let Some(pp) = resp.interact_pointer_pos() {
                        let t = time_at(pp.x);
                        let (a, b) = if anc < t { (anc, t) } else { (t, anc) };
                        if b - a > 0.3 {
                            acts.push(Action::SetAAt(a));
                            acts.push(Action::SetBAt(b));
                        }
                    }
                    self.sel_anchor = None;
                }
            } else if resp.clicked() && ui.input(|i| i.modifiers.alt) {
                // alt + click: a bookmark there - or, on one, takes it away (the song plays on)
                if let (Some(id), Some(pp)) = (tid.filter(|id| !self.is_demo(*id)), resp.interact_pointer_pos()) {
                    let near = self.store.marks.get(&id).and_then(|l| l.iter().position(|m| (at(m.t) - pp.x).abs() <= 6.0));
                    match near {
                        Some(k) => {
                            if let Some(l) = self.store.marks.get_mut(&id) {
                                l.remove(k);
                            }
                            self.store_dirty = true;
                            self.set_note("BOOKMARK REMOVED");
                        }
                        None => self.add_mark(id, time_at(pp.x)),
                    }
                }
            } else {
                if resp.dragged() || resp.clicked() {
                    if let Some(pp) = resp.interact_pointer_pos() {
                        self.seek_drag = Some(time_at(pp.x));
                    }
                }
                if resp.drag_stopped() || resp.clicked() {
                    if let Some(t) = self.seek_drag.take() {
                        acts.push(Action::Seek(t));
                    }
                }
            }
            if resp.secondary_clicked() {
                if let Some(pp) = resp.interact_pointer_pos() {
                    self.menu_t = time_at(pp.x);
                }
            }
            let (mt, lo, pr) = (self.menu_t, self.loop_on, self.practice);
            let mut mark_here = false;
            resp.context_menu(|ui| {
                crate::loop_menu(ui, acts, mt, both, lo, pr);
                if menu_item(ui, &format!("Bookmark here  {}", fmt_t(mt))) {
                    mark_here = true;
                    ui.close_menu();
                }
            });
            if let Some(id) = tid.filter(|id| mark_here && !self.is_demo(*id)) {
                self.add_mark(id, mt);
            }
        } else {
            ptext(ui.painter(), bar.center(), Align::Center, "PLAY A TRACK TO START", 2.0, pal().dim);
        }
        ui.add_space(6.0);

        // ---- the panel's tabs: the loop and speed tools first, then the bigger tools (TRAINER, PITCH & EAR, STEMS)
        let cur = if self.more { self.mtab as usize + 1 } else { 0 };
        if let Some(i) = tab_row(ui, &["LOOP & SPEED", "TRAINER", "PITCH & EAR", "STEMS", "BOOKMARKS"], cur) {
            self.more = i > 0;
            if i > 0 {
                self.mtab = (i - 1) as u8;
            }
        }
        ui.add_space(4.0);
        if self.more {
            let top = ui.cursor().min.y;
            self.more_tools(ui, acts);
            let r = ui.min_rect();
            crate::tour::mark("MOREBOX", Rect::from_min_max(Pos2::new(r.min.x, top), Pos2::new(r.max.x, ui.cursor().min.y)));
            return;
        }

        // ---- A and B, then the loop's own buttons  (each block reports where it is, for the practice tour)
        let row_mark = |ui: &egui::Ui, key: &str, top: f32| {
            let r = ui.min_rect();
            crate::tour::mark(key, Rect::from_min_max(Pos2::new(r.min.x, top), Pos2::new(r.max.x, ui.cursor().min.y)));
        };
        let ab_top = ui.cursor().min.y;
        let a_txt = self.loop_a.map(fmt_t).unwrap_or_else(|| "-:--.-".to_string());
        let b_txt = self.loop_b.map(fmt_t).unwrap_or_else(|| "-:--.-".to_string());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if retro_btn_w(ui, "A", 44.0, false).tip("Loop start here  ([ key)").clicked() && active {
                acts.push(Action::SetAAt(pos));
            }
            self.num_step(
                ui,
                acts,
                F_LOOP_A,
                92.0,
                Knob::LoopA,
                a_txt,
                "Nudge earlier / later - or click and type a time like 1:23.5",
            );
            ui.add_space(14.0);
            if retro_btn_w(ui, "B", 44.0, false).tip("Loop end here  (] key)").clicked() && active {
                acts.push(Action::SetBAt(pos));
            }
            self.num_step(
                ui,
                acts,
                F_LOOP_B,
                92.0,
                Knob::LoopB,
                b_txt,
                "Nudge earlier / later - or click and type a time like 2:05",
            );
            ui.add_space(14.0);
            let ink = pal().ink;
            if crate::icon_btn_w(ui, &crate::IC_REP, self.loop_on && both, ink, 44.0).tip("Loop on / off  (\\ key)").clicked() {
                acts.push(Action::LoopToggle);
            }
            if crate::icon_btn_w(ui, &crate::IC_X, false, ink, 44.0).tip("Clear the loop").clicked() {
                acts.push(Action::LoopClear);
            }
        });
        row_mark(ui, "LOOPROW", ab_top);
        ui.add_space(8.0);

        // ---- moving around, the speed, and MORE
        let speed_top = ui.cursor().min.y;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let ink = pal().ink;
            if crate::ibtn(ui, &crate::IC_REW, "2 SEC", false).tip("Back two seconds  (, key)").clicked() {
                acts.push(Action::SeekRel(-2.0));
            }
            if crate::ibtn(ui, &crate::IC_PREV, "RESTART", false)
                .tip("Back to the loop start (or the track start)  (. key)")
                .clicked()
            {
                acts.push(Action::Seek(self.loop_a.unwrap_or(0.0)));
            }
            let _ = ink;
            ui.add_space(14.0);
            label(ui, "SPEED", 64.0);
            // the usual speeds in a drop-down, and - / + for 1% at a time (click the number to type any, 25 to 250)
            let sp = retro_btn_w(ui, "v", 34.0, false).tip("Pick a speed");
            let menu = ui.id().with("speed_menu");
            if sp.clicked() {
                ui.memory_mut(|m| m.toggle_popup(menu));
            }
            egui::popup::popup_below_widget(ui, menu, &sp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(110.0);
                for pct in [25u32, 30, 40, 50, 60, 70, 75, 80, 85, 90, 95, 100, 110, 125, 150] {
                    let mark = if self.speed == pct { "> " } else { "   " };
                    if menu_item(ui, &format!("{}{}%", mark, pct)) {
                        acts.push(Action::Speed(pct));
                    }
                }
            });
            self.num_step(
                ui,
                acts,
                F_SPEED,
                64.0,
                Knob::Speed,
                format!("{}%", self.speed),
                "1% slower / faster - or click and type any speed from 25 to 250",
            );
        });
        row_mark(ui, "SPEEDROW", speed_top);
        ui.add_space(10.0);

        // ---- saved loops for this track
        ui.horizontal_wrapped(|ui| {
            // the save button's real width; if the name box cannot keep a usable size, the button drops to the next line
            let bw = 10.0 + 18.0 + 8.0 + text_w("SAVE LOOP", 2.0) + 10.0;
            let w = (ui.available_width() - bw - 12.0).max(150.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_SEC, &mut self.sec_name, rect, "Name this loop (bridge, head, lick...)", false);
            if o.enter {
                acts.push(Action::SaveSection);
            }
            if crate::ibtn(ui, &crate::IC_BOOK, "SAVE LOOP", false).tip("Keep this A-B loop with the track").clicked() {
                acts.push(Action::SaveSection);
            }
        });
        self.saved_loops(ui, acts, tid);

        ui.add_space(6.0);
    }

    /// The practice panel's saved loops of this song, by group: click one to loop it; right-click to rename it, add a
    /// note, put it in a group or delete it (asked first).
    fn saved_loops(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, tid: Option<i64>) {
        let Some(id) = tid.filter(|id| !self.is_demo(*id)) else { return };
        let secs: Vec<crate::store::Section> = self.store.sections.get(&id).cloned().unwrap_or_default();
        if secs.is_empty() {
            label(ui, "SAVED LOOPS OF THIS TRACK SHOW UP HERE", 300.0);
            return;
        }
        // groups in the order they first appear, loops with no group first
        let mut groups: Vec<String> = vec![String::new()];
        for s in &secs {
            if !groups.contains(&s.group) {
                groups.push(s.group.clone());
            }
        }
        let mut named: Vec<String> = self.store.groups();
        for g in groups.iter().filter(|g| !g.is_empty()) {
            if !named.contains(g) {
                named.push(g.clone());
            }
        }
        for g in &groups {
            let members: Vec<usize> = (0..secs.len()).filter(|i| secs[*i].group == *g).collect();
            if members.is_empty() {
                continue;
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                if !g.is_empty() {
                    label(ui, &g.to_uppercase(), (text_w(&g.to_uppercase(), 1.0) + 16.0).min(160.0));
                }
                for &i in &members {
                    let s = &secs[i];
                    let w = (text_w(&s.name, 2.0) + 20.0).clamp(44.0, 180.0);
                    let on = self.loop_on
                        && self.loop_a.is_some_and(|x| (x - s.a).abs() < 0.05)
                        && self.loop_b.is_some_and(|x| (x - s.b).abs() < 0.05);
                    let tip = if s.note.is_empty() {
                        format!("{} - {}   (right-click: rename, note, group, delete)", fmt_t(s.a), fmt_t(s.b))
                    } else {
                        format!("{} - {}   {}", fmt_t(s.a), fmt_t(s.b), s.note)
                    };
                    let r = retro_btn_w(ui, &s.name, w, on).tip(tip);
                    if r.clicked() {
                        acts.push(Action::GoSection(i));
                    }
                    r.context_menu(|ui| {
                        if menu_item(ui, "Rename / add a note...") {
                            self.clip_edit = Some((id, i));
                            self.clip_name = s.name.clone();
                            self.clip_note = s.note.clone();
                            self.clip_group = s.group.clone();
                            ui.close_menu();
                        }
                        for other in named.iter().filter(|o| **o != s.group) {
                            if menu_item(ui, &format!("Move to {}", other)) {
                                acts.push(Action::LoopGroup(id, i, other.clone()));
                                ui.close_menu();
                            }
                        }
                        if menu_item(ui, "Move to a new group...") {
                            self.clip_edit = Some((id, i));
                            self.clip_name = s.name.clone();
                            self.clip_note = s.note.clone();
                            self.clip_group = String::new();
                            self.ed.id = crate::tools::F_CLIP_GROUP;
                            ui.close_menu();
                        }
                        if !s.group.is_empty() && menu_item(ui, "Out of the group") {
                            acts.push(Action::LoopGroup(id, i, String::new()));
                            ui.close_menu();
                        }
                        if menu_item(ui, "Delete this loop...") {
                            self.loop_ask = Some((id, i));
                            ui.close_menu();
                        }
                    });
                }
            });
        }
        if let Some((_, i)) = self.loop_ask.filter(|(lid, _)| *lid == id) {
            ui.horizontal_wrapped(|ui| {
                let name = secs.get(i).map(|s| s.name.to_uppercase()).unwrap_or_default();
                label(ui, &format!("DELETE {}?", name), (text_w(&name, 1.0) + 90.0).min(320.0));
                if retro_btn(ui, "YES, DELETE IT", false).clicked() {
                    acts.push(Action::DeleteSection(i));
                    self.loop_ask = None;
                }
                if retro_btn(ui, "KEEP IT", false).clicked() {
                    self.loop_ask = None;
                }
            });
        }
        if self.clip_edit.is_some_and(|(lid, _)| lid == id) {
            self.loop_editor(ui, acts);
        }
    }

    /// Editing a loop: its name, a note, the group it's in (with this song's other groups to pick from).
    fn loop_editor(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let Some((id, i)) = self.clip_edit else { return };
        let o1 = field_row(
            ui,
            &mut self.ed,
            crate::tools::F_CLIP_NAME,
            "NAME",
            &mut self.clip_name,
            "funky riff, the lick at 2:10...",
            None,
        );
        let o2 = field_row(
            ui,
            &mut self.ed,
            crate::tools::F_CLIP_NOTE,
            "NOTE",
            &mut self.clip_note,
            "what it is, what to work on",
            None,
        );
        let o3 = field_row(
            ui,
            &mut self.ed,
            crate::tools::F_CLIP_GROUP,
            "GROUP",
            &mut self.clip_group,
            "solo licks, head, comping... (empty: none)",
            None,
        );
        let mut groups: Vec<String> = self.store.groups();
        for s in self.store.sections.get(&id).into_iter().flatten() {
            if !s.group.is_empty() && !groups.contains(&s.group) {
                groups.push(s.group.clone());
            }
        }
        let mut done = o1.enter || o2.enter || o3.enter;
        ui.horizontal_wrapped(|ui| {
            ui.add_space(12.0);
            for g in &groups {
                if retro_btn(ui, &g.to_uppercase(), *g == self.clip_group).tip("Put it in this group").clicked() {
                    self.clip_group = g.clone();
                }
            }
            if retro_btn(ui, "SAVE", false).clicked() {
                done = true;
            }
            if retro_btn(ui, "CANCEL", false).clicked() {
                self.clip_edit = None;
                self.ed.id = 0;
            }
        });
        if done {
            acts.push(Action::EditLoop(id, i, self.clip_name.clone(), self.clip_note.clone(), self.clip_group.clone()));
            self.clip_edit = None;
            self.ed.id = 0;
        }
        ui.add_space(6.0);
    }

    /// BOOKMARKS: this song's bookmarks in time order - jump to one, name it, recolour or remove it.
    fn bookmarks_tab(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let Some(id) = self.cur_track().map(|t| t.id).filter(|id| !self.is_demo(*id)) else {
            crate::views::dim_line(ui, "PLAY A SONG TO BOOKMARK IT", 1.0, pal().dim);
            return;
        };
        let now = self.pos();
        if let Some(t) = self.cur_track() {
            let song = if t.artist.is_empty() { t.title.clone() } else { format!("{} - {}", t.artist, t.title) };
            dim_line(ui, &format!("BOOKMARKS IN {}", song.to_uppercase()), 2.0, pal().ink);
        }
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, &format!("+ BOOKMARK AT {}", fmt_t(now)), false)
                .tip(format!(
                    "A bookmark where the song is now (or the {} key, or alt + click on the waveform)",
                    self.key_text(crate::prefs::Cmd::Bookmark)
                ))
                .clicked()
            {
                self.add_mark(id, now);
            }
            let n = self.store.marks.get(&id).map_or(0, |l| l.len());
            if n > 0 && retro_btn(ui, "REMOVE ALL", false).tip("Take every bookmark off this song").clicked() {
                self.store.marks.remove(&id);
                self.store_dirty = true;
            }
        });
        ui.add_space(4.0);
        let n = self.store.marks.get(&id).map_or(0, |l| l.len());
        if n == 0 {
            para(ui, "No bookmarks on this song yet. They are fixed points to find your way: the solo, the key change, the bar you keep missing.", pal().ink2);
            return;
        }
        let mut drop = None;
        let mut recolour = None;
        for k in 0..n {
            let m = self.store.marks[&id][k].clone();
            ui.horizontal(|ui| {
                let col = egui::Color32::from_rgb(m.col[0], m.col[1], m.col[2]);
                // its colour: right-click for another
                let (sw, sr) = ui.allocate_exact_size(Vec2::new(22.0, bh()), Sense::click());
                ui.painter().rect_filled(sw.shrink2(Vec2::new(2.0, 4.0)), 2.0, col);
                ptext(ui.painter(), sw.center(), Align::Center, &(k + 1).to_string(), 1.0, egui::Color32::from_gray(12));
                sr.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Right-click: another colour").context_menu(|ui| {
                    if let Some(c) = mark_swatches(ui, Some(m.col)) {
                        recolour = Some((k, c));
                        ui.close_menu();
                    }
                });
                if retro_btn_w(ui, &fmt_t(m.t), 70.0, false).tip("Jump here").clicked() {
                    acts.push(Action::Seek(m.t));
                }
                let w = (ui.available_width() - 40.0).clamp(80.0, 360.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                let fid = 5000 + k as u32;
                if let Some(list) = self.store.marks.get_mut(&id) {
                    let o = field(ui, &mut self.ed, fid, &mut list[k].name, r, "Name it (solo, bridge, key change...)", false);
                    if o.changed {
                        self.store_dirty = true;
                    }
                }
                if retro_btn_w(ui, "X", 28.0, false).tip("Remove this bookmark").clicked() {
                    drop = Some(k);
                }
            });
        }
        if let Some(list) = self.store.marks.get_mut(&id) {
            if let Some((k, c)) = recolour {
                list[k].col = c;
                self.store_dirty = true;
            }
            if let Some(k) = drop {
                list.remove(k);
                self.store_dirty = true;
            }
        }
    }

    /// The practice panel's other tabs: TRAINER (0), PITCH & EAR (1), STEMS (2), BOOKMARKS (3).
    fn more_tools(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let tid = self.cur_track().map(|t| t.id);
        if self.mtab == 3 {
            self.bookmarks_tab(ui, acts);
            return;
        }
        match self.mtab {
            0 => {
                // the count-off is its own thing: clicks before every pass of the loop, with or without the trainer
                ui.horizontal_wrapped(|ui| {
                    label(ui, "COUNT-OFF", 92.0);
                    let ci = if self.count_in == 0 { "OFF".to_string() } else { format!("{} BEATS", self.count_in) };
                    if retro_btn_w(ui, &ci, 92.0, self.count_in > 0)
                        .tip(
                            "Clicks before every pass of the loop, at the song's tempo - with or without the trainer (off, 2, 4)",
                        )
                        .clicked()
                    {
                        acts.push(Action::CountIn);
                    }
                });
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    label(ui, "TRAIN", 56.0);
                    if retro_btn_w(ui, if self.trainer { "ON" } else { "OFF" }, 56.0, self.trainer)
                        .tip("Practice the loop a set number of times, then speed up by a set percent")
                        .clicked()
                    {
                        acts.push(Action::Trainer);
                    }
                    if self.trainer {
                        lcd_box(ui, &format!("PASS {}/{}", self.passes, self.trainer_n), 100.0, pal().ink);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    label(ui, "EVERY", 60.0);
                    self.num_step(
                        ui,
                        acts,
                        F_TR_LOOPS,
                        44.0,
                        Knob::Loops,
                        self.trainer_n.to_string(),
                        "Loops to play before each speed-up - or type it",
                    );
                    label(ui, "LOOPS", 56.0);
                    self.num_step(
                        ui,
                        acts,
                        F_TR_STEP,
                        52.0,
                        Knob::Step,
                        format!("+{}%", self.trainer_step),
                        "How much faster each step - or type it",
                    );
                });
                dim_line(ui, "STARTS AT THE SPEED SET ABOVE AND STOPS AT 100%", 1.0, pal().dim);
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if retro_btn_w(ui, "FOCUS", 76.0, self.focus_mode)
                        .tip("Hide the lists - just the player and these tools")
                        .clicked()
                    {
                        acts.push(Action::ToggleFocus);
                    }
                });
            }
            1 => {
                ui.horizontal_wrapped(|ui| {
                                        label(ui, "PITCH", 56.0);
                                        if retro_btn_w(ui, "-", 28.0, false)
                                            .tip("Lower the pitch a semitone (speed stays)")
                                            .clicked()
                                        {
                                            acts.push(Action::Transpose(-100));
                                        }
                                        if retro_btn_w(ui, "-10c", 44.0, false)
                                            .tip("Fine tune: lower the pitch 10 cents (a tenth of a semitone) - for records not tuned to A440")
                                            .clicked()
                                        {
                                            acts.push(Action::Transpose(-10));
                                        }
                                        lcd_box(ui, &format!("{:+.2}", self.semis as f32 / 100.0), 64.0, pal().ink);
                                        if retro_btn_w(ui, "+10c", 44.0, false)
                                            .tip("Fine tune: raise the pitch 10 cents")
                                            .clicked()
                                        {
                                            acts.push(Action::Transpose(10));
                                        }
                                        if retro_btn_w(ui, "+", 28.0, false).tip("Raise the pitch a semitone (speed stays)").clicked() {
                                            acts.push(Action::Transpose(100));
                                        }
                                        let cur_id = self.cur_track().map(|t| t.id);
                                        match self.tuning {
                                            Some((id, c, _)) if Some(id) == cur_id && c.abs() > 4 => {
                                                if retro_btn_w(ui, &format!("TUNE TO A440 ({:+}c)", -c), 150.0, self.semis == -c)
                                                    .tip("Shifts the track to standard A440 pitch so you can play along without retuning")
                                                    .clicked()
                                                {
                                                    acts.push(Action::Transpose(-c - self.semis));
                                                }
                                            }
                                            _ => {
                                                if retro_btn_w(ui, if self.tuning_busy { "LISTENING..." } else { "FIND TUNING" }, 116.0, false)
                                                    .tip("Measures how far this recording is from A440 (old records and some modern tracks are off) and offers to fix it")
                                                    .clicked()
                                                {
                                                    acts.push(Action::FindTuning);
                                                }
                                            }
                                        }
                                        if self.semis != 0 && retro_btn_w(ui, "0", 28.0, false).tip("Back to the original pitch").clicked() {
                                            acts.push(Action::Transpose(-self.semis));
                                        }
                                        ui.add_space(8.0);
                                        if retro_btn_w(ui, CHAN_NAMES[self.chan as usize % 6], 100.0, self.chan != 0)
                                            .tip("Ear mode. LEFT / RIGHT CHANNEL plays just that side in both ears (old jazz records often hide the bass or piano on one side; modern mixes barely differ). NO CENTER removes vocals; BASS ONLY keeps the low end.")
                                            .clicked()
                                        {
                                            acts.push(Action::Chan);
                                        }
                                        if retro_btn_w(ui, if self.exporting { "SAVING..." } else { "EXPORT WAV" }, 100.0, false)
                                            .tip("Save the song as you hear it - stems on or off, speed, pitch, ear mode, equalizer - as a WAV: the A-B loop when there is one, else the whole song")
                                            .clicked()
                                        {
                                            acts.push(Action::Export);
                                        }
                                    });
            }
            _ => {
                ui.horizontal_wrapped(|ui| {
                    label(ui, "STEMS", 56.0);
                    if self.stem_busy == 1 {
                    lcd_box(ui, &stems::stage_text(), 300.0, pal().ink2);
                } else if self.stem_busy == 2 {
                    let (d, t) = (stems::DONE.load(Ordering::Relaxed), stems::TOTAL.load(Ordering::Relaxed));
                    lcd_box(ui, &format!("{} {}/{}", stems::stage_text(), d, t), 240.0, pal().ink2);
                    if retro_btn(ui, "CANCEL", false).clicked() {
                        acts.push(Action::StemCancel);
                    }
                } else if tid.is_some_and(|id| self.is_demo(id)) {
                    // the tour's song shows the button you would see on any song not split yet
                    if retro_btn_w(ui, "SPLIT THIS TRACK", 190.0, false)
                        .tip("Separates a track into drums, bass, other and vocals. Not during the tour.")
                        .clicked()
                    {
                        self.set_note("NOT DURING THE TOUR");
                    }
                } else if !stems::tool_ready() {
                    if retro_btn_w(ui, "GET STEMS TOOL (170 MB)", 230.0, false)
                        .tip("One-time download: the AI model that separates drums, bass, vocals and the rest. Runs on this PC.")
                        .clicked()
                    {
                        acts.push(Action::StemGet);
                    }
                } else if tid.map_or(false, stems::have_stems) {
                    for (i, name) in stems::NAMES.iter().enumerate() {
                        let r = stem_btn(ui, i, self.stem_on[i])
                            .tip(format!("{}: click on / off, right-click for only this one", name));
                        if r.clicked() {
                            acts.push(Action::StemToggle(i));
                        }
                        r.context_menu(|ui| {
                            if menu_item(ui, "Only this stem") {
                                acts.push(Action::StemSolo(i));
                                ui.close_menu();
                            }
                        });
                    }
                    if retro_btn_w(ui, "ALL", 44.0, false).clicked() {
                        acts.push(Action::StemAll);
                    }
                    // what they take on this computer
                    if let Some(id) = tid {
                        let (this, all) = stems::sizes(id);
                        crate::views::dim_line(
                            ui,
                            &format!("THIS SONG'S STEMS: {}   ALL STEMS: {}", cache::fmt_size(this), cache::fmt_size(all)),
                            1.0,
                            pal().dim,
                        );
                    }
                } else if retro_btn_w(ui, "SPLIT THIS TRACK", 190.0, false)
                    .tip(format!(
                        "Separates the track into drums, bass, other and vocals. Takes a few minutes; kept afterwards (about {} for this song).",
                        cache::fmt_size(stems::size_guess(self.track_len()))
                    ))
                    .clicked()
                {
                    acts.push(Action::StemSplit);
                }
                });
            }
        }
    }
}

const COMFORT: [&str; 4] = ["LEARNING", "OK", "GOOD", "COMFORTABLE"];

/// How well you know a tune: four pips, click one to set the level.
fn comfort_meter(ui: &mut egui::Ui, level: u8) -> Option<u8> {
    let level = level.min(3);
    let mut hit = None;
    ui.horizontal_wrapped(|ui| {
        label(ui, "KNOW IT", 74.0);
        for n in 0..4u8 {
            let (r, resp) = ui.allocate_exact_size(Vec2::new(30.0, bh()), Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(COMFORT[n as usize]);
            let on = n <= level;
            let p = ui.painter();
            inset(p, r, pal().beige_dk);
            if on {
                fill_rect(p, r.shrink(5.0), if level == 3 { pal().ink } else { pal().red });
            }
            if resp.clicked() {
                hit = Some(n);
            }
        }
        ui.add_space(8.0);
        lcd_box(ui, COMFORT[level as usize], 130.0, pal().ink);
    });
    hit
}

/// A long list opened in place, kept in a box of its own: at most about a third of the window tall, with its own
/// scroll, so opening it never pushes the rest of the page out of the way.
pub(crate) fn contained<R>(ui: &mut egui::Ui, id: impl std::hash::Hash, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let max = (ui.ctx().screen_rect().height() * 0.35).clamp(160.0, 380.0);
    egui::ScrollArea::vertical().id_salt(id).max_height(max).auto_shrink([false, true]).show(ui, add).inner
}
