//! Everything drawn for the practice extras: the FILES / YOUTUBE / TUNES / DIARY lists, the
//! practice panel, the lead sheet and the little text-field widget they share.

use crate::api::Track;
use crate::extras::{Knob, CHAN_NAMES};
use crate::font::{ptext, ptext_fit, spx, text_w};
use crate::sources;
use crate::stems;
use crate::store::{self, Ext};
use crate::tools::{F_LOOK, F_LOOP_A, F_LOOP_B, F_SPEED, F_TR_LOOPS, F_TR_STEP};
pub(crate) use crate::widgets::{dim_line, form_label, label};
use crate::{
    bh, cache, chip, col_header, col_on, fill_rect, fmt_t, fmt_time, inset, lcd_box, list_row, menu_item, outline, pal, para,
    play_buttons, retro_btn, retro_btn_w, section_header, tab_row, table_header, title_line, track_cells, window_deco, Action,
    App, Ed, RowState, Sec, Tip, PRACTICE,
};
use crate::{check_box, dropdown};
use eframe::egui::{self, Align, Pos2, Rect, Sense, Vec2};
use egui::{Color32, Stroke};
use std::collections::HashMap;
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
    // (a skin's font is drawn a size up: a step down here, so it sits inside the box)
    let px = if crate::winamp_ui::chrome_on() { 1.7 } else { 2.0 };
    let lh = 7.0 * spx(px) + 6.0;
    let inner = rect.shrink2(Vec2::new(8.0, 3.0));
    let mut out = FieldOut { changed: false, enter: false };
    let resp = ui.interact(rect, ui.id().with(("fld", id)), Sense::click_and_drag());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
    }
    let mut chars: Vec<char> = text.chars().collect();
    // right-click: cut, copy, paste and select all - in boxes for words and links (not the number boxes)
    if !crate::tools::is_number_field(id) {
        let has_sel = ed.id == id && ed.anchor.is_some_and(|a| a != ed.cur);
        let n = chars.len();
        // (what was picked: an edit to do as if typed, whether it takes the whole text, a paste)
        let mut picked: Option<(Option<egui::Event>, bool)> = None;
        let mut paste = false;
        resp.context_menu(|ui| {
            if (has_sel || n > 0) && menu_item(ui, "Cut") {
                picked = Some((Some(egui::Event::Cut), !has_sel));
            }
            if (has_sel || n > 0) && menu_item(ui, "Copy") {
                picked = Some((Some(egui::Event::Copy), !has_sel));
            }
            if menu_item(ui, "Paste") {
                paste = true;
            }
            if n > 0 && menu_item(ui, "Select all") {
                picked = Some((None, true));
            }
            if picked.is_some() || paste {
                ui.close_menu();
            }
        });
        if picked.is_some() || paste {
            if ed.id != id {
                ed.id = id;
                ed.cur = n;
                ed.anchor = None;
            }
        }
        if let Some((ev, all)) = picked {
            ed.menu = ev;
            if all {
                ed.anchor = Some(0);
                ed.cur = n;
            }
        }
        if paste {
            // (the window reads the clipboard and hands the text over next frame, as Ctrl+V does)
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
        }
    }
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

    // the place in the text under the mouse
    let at = |chars: &[char], pp: Pos2, hoff: f32, voff: f32, lines: &[(usize, usize)]| -> usize {
        let li = if multi { (((pp.y - inner.min.y + voff) / lh).floor().max(0.0) as usize).min(lines.len() - 1) } else { 0 };
        let (s, e) = lines[li];
        let x = pp.x - inner.min.x + hoff;
        let mut best = s;
        let mut best_d = f32::MAX;
        for k in s..=e {
            let d = (caret_x(chars, s, k, px) - x).abs();
            if d < best_d {
                best_d = d;
                best = k;
            }
        }
        best
    };
    let shift = ui.input(|i| i.modifiers.shift);
    if resp.triple_clicked() && focused {
        // a triple click picks everything
        ed.anchor = Some(0);
        ed.cur = chars.len();
    } else if resp.double_clicked() && focused {
        // a double click picks the word
        let c = ed.cur.min(chars.len());
        let word = |ch: &char| !ch.is_whitespace();
        let s = (0..c).rev().take_while(|k| word(&chars[*k])).last().unwrap_or(c);
        let e = (c..chars.len()).take_while(|k| word(&chars[*k])).last().map_or(c, |k| k + 1);
        ed.anchor = Some(s);
        ed.cur = e;
    } else if resp.clicked() || resp.drag_started() {
        let (lines, hoff, voff) = offsets(&chars, ed.cur, focused);
        let was = if focused { Some(ed.cur) } else { None };
        ed.id = id;
        if let Some(pp) = resp.interact_pointer_pos() {
            ed.cur = at(&chars, pp, hoff, voff, &lines);
        } else {
            ed.cur = chars.len();
        }
        // shift-click stretches the selection; a plain click or the start of a drag starts again here
        ed.anchor = if shift { was.or(ed.anchor) } else { Some(ed.cur) };
    } else if resp.dragged() && focused {
        // dragging across the text selects it
        let (lines, hoff, voff) = offsets(&chars, ed.cur, focused);
        if let Some(pp) = ui.input(|i| i.pointer.interact_pos()) {
            ed.cur = at(&chars, pp, hoff, voff, &lines);
        }
    } else if focused && ui.input(|i| i.pointer.primary_pressed()) && !resp.hovered() {
        ed.id = 0;
        ed.anchor = None;
    }

    if ed.id == id {
        // the system's text input on (accents from dead keys, other alphabets): it composes "^ e" into "ê" and hands
        // it over as one piece
        ui.ctx().output_mut(|o| o.ime = Some(egui::output::IMEOutput { rect, cursor_rect: rect }));
        let mut events = ui.input(|i| i.events.clone());
        if let Some(ev) = ed.menu.take() {
            events.insert(0, ev);
        }
        let mut cur = ed.cur.min(chars.len());
        let mut anchor = ed.anchor.map(|a| a.min(chars.len()));
        let mut edited = false;
        // the selection (start, end), if any
        let sel_of = |anchor: Option<usize>, cur: usize| anchor.filter(|a| *a != cur).map(|a| (a.min(cur), a.max(cur)));
        // take the selection out (true if there was one)
        let cut = |chars: &mut Vec<char>, cur: &mut usize, anchor: &mut Option<usize>| -> bool {
            match sel_of(*anchor, *cur) {
                Some((s, e)) => {
                    chars.drain(s..e);
                    *cur = s;
                    *anchor = None;
                    true
                }
                None => {
                    *anchor = None;
                    false
                }
            }
        };
        for ev in events {
            match ev {
                egui::Event::Copy | egui::Event::Cut => {
                    if let Some((s, e)) = sel_of(anchor, cur) {
                        ui.ctx().copy_text(chars[s..e].iter().collect());
                        if matches!(ev, egui::Event::Cut) {
                            edited |= cut(&mut chars, &mut cur, &mut anchor);
                        }
                    }
                }
                egui::Event::Text(t) | egui::Event::Paste(t) | egui::Event::Ime(egui::ImeEvent::Commit(t)) => {
                    // typing over a selection replaces it
                    edited |= cut(&mut chars, &mut cur, &mut anchor);
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
                egui::Event::Key { key: egui::Key::A, pressed: true, modifiers, .. } if modifiers.command => {
                    // select all
                    anchor = Some(0);
                    cur = chars.len();
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    let moving = matches!(
                        key,
                        egui::Key::ArrowLeft
                            | egui::Key::ArrowRight
                            | egui::Key::ArrowUp
                            | egui::Key::ArrowDown
                            | egui::Key::Home
                            | egui::Key::End
                    );
                    let sel = sel_of(anchor, cur);
                    if moving && modifiers.shift {
                        // shift + a move stretches the selection from where it began
                        anchor.get_or_insert(cur);
                    } else if moving && sel.is_some() && matches!(key, egui::Key::ArrowLeft | egui::Key::ArrowRight) {
                        // left or right with a selection: to its start or end
                        let (s, e) = sel.unwrap_or((cur, cur));
                        cur = if key == egui::Key::ArrowLeft { s } else { e };
                        anchor = None;
                        continue;
                    } else if moving {
                        anchor = None;
                    }
                    if matches!(key, egui::Key::Backspace | egui::Key::Delete) && sel.is_some() {
                        edited |= cut(&mut chars, &mut cur, &mut anchor);
                        continue;
                    }
                    match key {
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
                            let to = if key == egui::Key::ArrowUp {
                                li.checked_sub(1)
                            } else {
                                Some(li + 1).filter(|n| *n < lines.len())
                            };
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
                        egui::Key::Escape => {
                            ed.id = 0;
                            anchor = None;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        ed.cur = cur;
        ed.anchor = anchor;
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
    // the selection, shown inverted over the text
    if let Some((ss, se)) = ed.anchor.filter(|a| focused && *a != ed.cur).map(|a| (a.min(ed.cur), a.max(ed.cur))) {
        for (li, (s, e)) in lines.iter().enumerate() {
            let (a, b) = (ss.max(*s), se.min(*e));
            if a >= b {
                continue;
            }
            let y = if multi { inner.min.y - voff + li as f32 * lh + lh / 2.0 } else { rect.center().y };
            let x0 = inner.min.x - hoff + caret_x(&chars, *s, a, px);
            let x1 = inner.min.x - hoff + caret_x(&chars, *s, b, px);
            let band = Rect::from_min_max(Pos2::new(x0, y - lh / 2.0), Pos2::new(x1, y + lh / 2.0));
            fill_rect(&cp, band, pal().ink);
            // the line again in the field's colour, seen only inside the band
            let line: String = chars[*s..(*e).min(chars.len())].iter().collect();
            ptext(&cp.with_clip_rect(band.intersect(inner)), Pos2::new(inner.min.x - hoff, y), Align::Min, &line, px, pal().lcd);
        }
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
    if crate::winamp_ui::chrome_on() {
        let names: Vec<&str> = items.iter().map(|s| s.1).collect();
        return crate::segments(ui, &names, cur);
    }
    crate::font::keep_caps(|| {
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
            let resp = ui
                .interact(r, ui.id().with(("subtab", i, *name)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let on = i == cur;
            // a small raised button each, a gap between; the one you are on pressed in and lit
            let b = r.shrink2(Vec2::new(2.0, 1.0));
            if on {
                crate::inset(ui.painter(), b, pal().lcd);
            } else {
                crate::raised_h(ui.painter(), b, resp.is_pointer_button_down_on(), resp.hovered());
            }
            let col = if on {
                pal().bar_txt
            } else if resp.hovered() {
                pal().ink
            } else {
                pal().ink2
            };
            crate::ui_text(ui.painter(), b.center(), Align::Center, name, 2.0, w - 8.0, col);
            if resp.clicked() {
                hit = Some(i);
            }
            x += w;
        }
        hit
    })
}

/// The LOOPS page's songs, filtered and in order.
struct SpotData {
    songs: Vec<SpotSong>,
}

/// One song on the LOOPS page: its loops (with the time spent on each) and bookmarks, and its minutes.
struct SpotSong {
    id: i64,
    v: Option<store::Version>,
    loops: Vec<(usize, store::Section, u32)>,
    marks: Vec<(usize, store::Bookmark)>,
    /// practised this week, and in all (with its loops), in seconds
    mins: u32,
    total: u32,
    /// the day its newest loop was saved
    newest: String,
}

/// A song's waveform as kept on disk (50 peaks a second), read once and kept for the LOOPS page.
fn song_wave(id: i64) -> Option<std::rc::Rc<Vec<u8>>> {
    thread_local! {
        static WAVES: std::cell::RefCell<HashMap<i64, Option<std::rc::Rc<Vec<u8>>>>> = std::cell::RefCell::new(HashMap::new());
    }
    WAVES.with(|w| {
        w.borrow_mut()
            .entry(id)
            .or_insert_with(|| {
                std::fs::read(crate::api::config_dir().join("waves").join(format!("{}.pk", id)))
                    .ok()
                    .filter(|b| b.len() > 50)
                    .map(std::rc::Rc::new)
            })
            .clone()
    })
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
        const PRACTICE_SECS: [(Sec, &str); 8] = [
            (Sec::Tunes, "TUNES"),
            (Sec::Diary, "DIARY"),
            (Sec::Loops, "LOOPS"),
            (Sec::Reading, "READING"),
            (Sec::Chords, "CHORDS"),
            (Sec::Ear, "EAR"),
            (Sec::Lines, "LINES"),
            (Sec::Melodies, "CHORD MELODIES"),
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
        let mut items: Vec<(Sec, &str)> = vec![(Sec::Tidal, "TIDAL")];
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

    /// The box that filters your library down as you type (every word must match; typos forgiven), with X to
    /// clear it. One filter for the Tidal library, MY TRACKS and LISTS.
    pub(crate) fn filter_box(&mut self, ui: &mut egui::Ui, hint: &str) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            let (ic, _) = ui.allocate_exact_size(Vec2::new(22.0, bh()), Sense::hover());
            crate::icon_in(ui.painter(), ic.shrink(4.0), &crate::IC_FILTER, pal().ink2);
            // (the full width; room for the clear button only once there is something to clear)
            let w = (ui.available_width() - if self.lib_filter.is_empty() { 8.0 } else { 44.0 }).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let _ = field(ui, &mut self.ed, crate::tools::F_LIB_FILTER, &mut self.lib_filter, rect, hint, false);
            if !self.lib_filter.is_empty() && retro_btn_w(ui, "X", 30.0, false).tip("Clear the filter.").clicked() {
                self.lib_filter.clear();
            }
        });
        ui.add_space(4.0);
    }

    /// MY TRACKS: every song you liked, from every source (your Tidal likes, and the files, SoundCloud and YouTube
    /// songs you hearted), with a SOURCES row to leave any of them out.
    pub(crate) fn mine_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "MY TRACKS", 3.0, pal().ink);
        title_line(
            ui,
            if self.mine_stems { "every song split into stems, liked or not" } else { "every song you liked, from every source" },
            2.0,
            pal().ink2,
        );
        ui.horizontal_wrapped(|ui| {
            // the sources in a small list of ticks under one button, so the row never wraps
            let n_on = crate::SOURCES.iter().filter(|(k, _)| !self.mine_off.iter().any(|o| o == k)).count();
            let what = if n_on == crate::SOURCES.len() { "all".to_string() } else { format!("{} of {}", n_on, crate::SOURCES.len()) };
            let src = retro_btn(ui, &crate::font::as_label(&format!("Sources: {}", what)), n_on < crate::SOURCES.len())
                .tip("Which places' songs to list: Tidal, your files, SoundCloud, YouTube.");
            let pop = ui.id().with("mine_sources");
            if src.clicked() {
                ui.memory_mut(|m| m.toggle_popup(pop));
            }
            egui::popup_below_widget(ui, pop, &src, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                ui.set_min_width(170.0);
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
            ui.add_space(8.0);
            if crate::check_box(ui, "WITH STEMS", self.mine_stems)
                .tip("Only the songs already split into drums, bass, vocals and the rest, liked or not.")
                .clicked()
            {
                self.mine_stems = !self.mine_stems;
                self.dirty = true;
            }
        });
        let shows = |key: &str| !self.mine_off.iter().any(|k| k == key);
        let mut tracks: Vec<Track>;
        let mut waiting = false;
        if self.mine_stems {
            // every split song this PC knows by name (one split before songs were remembered gets its name when played)
            let mut unnamed = 0;
            tracks = Vec::new();
            for id in stems::split_ids() {
                match self.store.songs.get(&id) {
                    Some(v) if shows(&v.kind) => tracks.push(v.to_track()),
                    Some(_) => {}
                    None => unnamed += 1,
                }
            }
            tracks.sort_by(|a, b| a.artists_text().to_lowercase().cmp(&b.artists_text().to_lowercase()).then(a.title.cmp(&b.title)));
            if unnamed > 0 {
                dim_line(ui, &format!("{} MORE SPLIT SONGS SHOW HERE ONCE YOU PLAY THEM AGAIN", unnamed), 1.0, pal().dim);
            }
            if tracks.is_empty() {
                para(ui, "No songs split yet. Play a song, open PRACTICE > STEMS and press SPLIT THIS TRACK.", pal().ink2);
                return;
            }
        } else {
            tracks = self.store.hearts.iter().filter(|e| shows(&e.kind)).map(|e| e.to_track()).collect();
        }
        if shows("tidal") && !self.offline && !self.mine_stems {
            match &self.lib_page {
                Some(p) => tracks.extend(p.tracks.iter().cloned()),
                None => waiting = true,
            }
        }
        // offline mode: only what is on this computer (your files, and the songs kept)
        if self.go_offline {
            tracks.retain(|t| cache::has(t.id) || matches!(self.srcmap.get(&t.id), Some(crate::Src::File(_))));
            dim_line(ui, "OFFLINE MODE: ONLY THE SONGS ON THIS COMPUTER", 1.0, pal().dim);
        }
        ui.add_space(4.0);
        if waiting {
            dim_line(ui, "YOUR TIDAL LIKES SHOW ONCE TIDAL > MY LIBRARY HAS LOADED", 1.0, pal().dim);
        }
        if tracks.is_empty() {
            para(ui, "Nothing here yet. Click the heart on any song and it shows up here. That works for Tidal, your files, SoundCloud and YouTube.", pal().ink2);
            return;
        }
        self.filter_box(ui, "Filter by title, artist or album");
        let filter = self.lib_filter.trim().to_string();
        let all = tracks.len();
        if !filter.is_empty() {
            let keep = crate::filtered(&filter, &tracks, |t| vec![t.title.clone(), t.artists_text(), t.album.clone()], 3);
            tracks = keep.into_iter().map(|i| tracks[i].clone()).collect();
            title_line(ui, &format!("{} of {} songs match", tracks.len(), all), 2.0, pal().ink2);
            if tracks.is_empty() {
                title_line(ui, "No songs match.", 2.0, pal().ink2);
                return;
            }
        } else {
            title_line(ui, &format!("{} songs", tracks.len()), 2.0, pal().ink2);
        }
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
        if crate::winamp_ui::chrome_on() {
            // Continue: the song by the artist (a link to them, from Tidal) - where you were, and the speed if not 100%
            let p = ui.painter();
            let speed = if l.speed == 0 || l.speed == 100 { String::new() } else { format!(" at {}%", l.speed) };
            let parts = [
                ("Continue: ".to_string(), pal().ink2),
                (l.version.title.clone(), pal().ink),
                (" by ".to_string(), pal().ink2),
                (l.version.artist.clone(), pal().ink),
                (format!(" - {}{}", fmt_time(l.pos), speed), pal().ink2),
            ];
            let mut x = rect.min.x + 14.0;
            let mut artist = None;
            for (i, (s, col)) in parts.iter().enumerate() {
                let room = rect.max.x - 10.0 - x;
                if room < 20.0 {
                    break;
                }
                // (cut with "..." when it doesn't fit, at the same size as the rest of the line)
                crate::row_text(p, Pos2::new(x, rect.center().y), Align::Min, s, room, *col);
                let w = p
                    .layout_no_wrap(
                        s.to_string(),
                        egui::FontId::proportional((crate::ROW_H * 0.62 * crate::font::ui_scale()).max(11.0)),
                        *col,
                    )
                    .size()
                    .x
                    .min(room);
                if i == 3 && l.version.kind == "tidal" {
                    artist = Some(Rect::from_min_size(Pos2::new(x, rect.min.y + 4.0), Vec2::new(w, rect.height() - 8.0)));
                }
                x += w;
            }
            let mut on_artist = false;
            if let Some(ar) = artist {
                let a = ui
                    .interact(ar, ui.id().with("continue_artist"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                let y = ar.center().y + 0.42 * crate::font::text_px(2.0);
                let ucol = if a.hovered() { pal().bar_txt } else { pal().ink2 };
                ui.painter().line_segment([Pos2::new(ar.min.x, y), Pos2::new(ar.max.x, y)], egui::Stroke::new(1.0_f32, ucol));
                if a.clicked() {
                    acts.push(Action::GoTo(l.version.id, crate::GoTo::Artist));
                    on_artist = true;
                }
            }
            if !on_artist && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                acts.push(Action::Continue);
            }
            return;
        }
        ptext_fit(ui.painter(), rect.center(), Align::Center, &txt, 2.0, rect.width() - 16.0, pal().ink);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            acts.push(Action::Continue);
        }
    }

    fn right_tabs(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let inner = ui.max_rect();
        let chip_h = 16.0;
        let mut right = crate::title_right(inner);
        let mut place = |w: f32| {
            let r = crate::title_slot(inner, right, w, chip_h);
            right -= w + 6.0;
            r
        };
        let r_chart = place(48.0);
        let r_queue = place(48.0);
        if chip(ui, r_chart, "CHART", self.rtab == 1, "c_rchart").tip("The chord changes of the open tune.").clicked() {
            acts.push(Action::RightTab(1));
        }
        if chip(ui, r_queue, "QUEUE", self.rtab == 0, "c_rqueue").clicked() {
            acts.push(Action::RightTab(0));
        }
    }

    /// Called at the top of the queue window: the QUEUE / CHART chips in the window's title strip (`ui`, the
    /// panel's), the lead sheet on its page (`page`).
    pub(crate) fn right_header(&mut self, ui: &mut egui::Ui, page: &mut egui::Ui, acts: &mut Vec<Action>) -> bool {
        if !PRACTICE {
            return false;
        }
        self.right_tabs(ui, acts);
        if self.rtab == 1 {
            self.chart_ui(page, acts);
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
        // the column names once a page: not again over the liked songs when the page's own list has them above
        let above = match which {
            4 => !self.store.files.is_empty(),
            5 => !self.store.yt.is_empty(),
            6 => !self.store.sc.is_empty(),
            _ => false,
        };
        if !above {
            col_header(ui, true);
        }
        for (i, e) in list.iter().enumerate() {
            let state = if playing == Some(e.id) { RowState::Playing } else { RowState::Normal };
            let stored = if yt { cache::has(e.id) } else { true };
            crate::row_heart(self.liked.contains(&e.id));
            crate::row_stems(e.id);
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
                    // (Edit info... comes with the song's own items)
                    crate::song_items(ui, acts, &e.to_track(), liked, false);
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
        section_header(ui, "PLAYLISTS");
        crate::cards_header(ui, "LENGTH");
        let mut queue: Option<(usize, bool)> = None;
        for (k, (i, l)) in mine.iter().enumerate() {
            let length = || {
                if l.items.is_empty() {
                    String::new()
                } else {
                    let total: f32 = l.items.iter().map(|e| e.dur).sum();
                    let n = l.items.len();
                    let time = if total > 0.0 { format!(", {}", crate::fmt_long(total)) } else { String::new() };
                    format!("{} song{}{}", n, if n == 1 { "" } else { "s" }, time)
                }
            };
            if let Some(r) = list_row(ui, k, Some(k + 1), || l.name.clone(), length, RowState::Normal, false, false) {
                if r.clicked() {
                    self.open_saved_list(l, acts);
                }
                r.context_menu(|ui| {
                    if !l.items.is_empty() {
                        if menu_item(ui, "Play next") {
                            queue = Some((k, true));
                            ui.close_menu();
                        }
                        if menu_item(ui, "Add to queue") {
                            queue = Some((k, false));
                            ui.close_menu();
                        }
                    }
                    crate::link_item(ui, acts, Some(l.url.clone()));
                    if menu_item(ui, "Remove from my playlists") {
                        acts.push(Action::RemoveList(*i));
                        ui.close_menu();
                    }
                });
            }
        }
        // its songs, known to the player first, then into the queue
        if let Some((k, next)) = queue {
            let l = &mine[k].1;
            for e in &l.items {
                self.register(&e.to_version());
            }
            self.keep_all(l.items.iter().map(|e| e.id).collect());
            acts.push(Action::QueueTracks(l.name.clone(), l.items.iter().map(|e| e.to_track()).collect(), next));
        }
    }

    /// Open a saved playlist on its page: at once, from the songs kept with it (read from the web only the first
    /// time, or with REFRESH).
    fn open_saved_list(&mut self, l: &store::SavedList, acts: &mut Vec<Action>) {
        if l.items.is_empty() {
            self.read_list_again(&l.kind, &l.url, acts);
            return;
        }
        for e in &l.items {
            self.register(&e.to_version());
        }
        self.keep_all(l.items.iter().map(|e| e.id).collect());
        if l.kind == "sc" {
            self.sc_cur = Some(l.url.clone());
            self.sc_results = l.items.clone();
        } else {
            self.yt_cur = Some(l.url.clone());
            self.yt_title = l.name.clone();
            self.yt_results = l.items.clone();
            self.yt_open = true;
        }
        self.list_ask = None;
    }

    /// Read a playlist from the web again (songs may have been added to it).
    fn read_list_again(&mut self, kind: &str, url: &str, acts: &mut Vec<Action>) {
        if kind == "sc" {
            acts.push(Action::ScOpenSet(url.to_string()));
        } else {
            self.yt_in = url.to_string();
            acts.push(Action::AddYt);
        }
    }

    /// The summary under the day: where the time went over the week (month, year) - a column a day (a month)
    /// stacked by song, the split between tunes, free practice and reading, the top songs and the top loops. The
    /// top five songs get colours; the rest is OTHER, so a lot of practice never makes it busier.
    fn diary_summary(&mut self, ui: &mut egui::Ui, period: &Period, sum: &Summary) {
        ui.horizontal_wrapped(|ui| {
            const SPANS: [&str; 3] = ["THIS WEEK", "THIS MONTH", "THIS YEAR"];
            if let Some(i) = dropdown(ui, "diary_span", "SUMMARY", &SPANS, self.diary_span as usize, 130.0) {
                self.diary_span = i as u8;
                self.dirty = true;
            }
            ui.add_space(10.0);
            if check_box(ui, "COUNT READING", self.diary_reading)
                .tip("Count sight-reading time in these charts and totals, or leave it out.")
                .clicked()
            {
                self.diary_reading = !self.diary_reading;
                self.dirty = true;
            }
        });
        ui.add_space(4.0);
        title_line(ui, &format!("{}  -  {}", period.name, mins_text(sum.total)), 1.5, pal().ink2);
        dim_line(ui, &format!("PRACTISED ON {} OF {} DAYS", sum.days_practised, sum.days_so_far), 1.0, pal().dim);
        if sum.total == 0 {
            dim_line(ui, "NO PRACTICE IN THIS STRETCH YET", 1.0, pal().dim);
            return;
        }
        // a column a day (a month), stacked by song
        ui.add_space(4.0);
        let w = seen_w(ui);
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 96.0), Sense::hover());
        let p = ui.painter();
        let n = sum.cols.len().max(1) as f32;
        let gap = if n > 14.0 { 2.0 } else { 6.0 };
        let cw = ((r.width() - gap * (n - 1.0)) / n).max(2.0);
        let chart_h = r.height() - 16.0;
        let top = sum.cols.iter().map(|c| c.total).max().unwrap_or(1).max(1) as f32;
        for (k, c) in sum.cols.iter().enumerate() {
            let x = r.min.x + k as f32 * (cw + gap);
            let base = r.min.y + chart_h;
            p.rect_filled(Rect::from_min_size(Pos2::new(x, base - 1.0), Vec2::new(cw, 1.0)), 0.0, pal().dim.gamma_multiply(0.5));
            let mut y = base;
            for (_, secs, col) in sum.segments_for(self, &c.days) {
                let h = secs as f32 / top * chart_h;
                p.rect_filled(Rect::from_min_max(Pos2::new(x, y - h), Pos2::new(x + cw, y)), 0.0, col);
                y -= h;
            }
            if !c.label.is_empty() {
                ptext(p, Pos2::new(x + cw / 2.0, r.max.y - 6.0), Align::Center, &c.label, 1.0, pal().dim);
            }
        }
        // the split: tunes, free practice, reading
        ui.add_space(12.0);
        let split: Vec<(String, u32, Color32)> = [
            ("TUNES", sum.tunes, Color32::from_rgb(217, 164, 65)),
            ("FREE PRACTICE", sum.free, Color32::from_rgb(95, 168, 211)),
            ("READING", sum.reading, READING_COL),
        ]
        .into_iter()
        .filter(|s| s.1 > 0)
        .map(|(n, s, c)| (n.to_string(), s, c))
        .collect();
        dim_line(ui, "WHAT KIND OF PRACTICE", 1.0, pal().dim);
        stacked_bar(ui, &split, 10.0);
        legend(ui, &split);
        // the top songs, and the top loops
        ui.add_space(6.0);
        dim_line(ui, "TOP SONGS", 1.0, pal().dim);
        let most = sum.songs.first().map_or(1, |s| s.1).max(1);
        for (id, secs) in sum.songs.iter().take(5) {
            let name = self.song_name(*id);
            bar_row(ui, &name, *secs, most, sum.colour(*id), &mins_text(*secs));
        }
        if !sum.loops.is_empty() {
            ui.add_space(4.0);
            dim_line(ui, "TOP LOOPS", 1.0, pal().dim);
            let most = sum.loops.first().map_or(1, |l| l.3).max(1);
            for (id, a, b, secs, passes) in sum.loops.iter().take(5) {
                let loop_name = self
                    .store
                    .sections
                    .get(id)
                    .and_then(|s| s.iter().find(|s| (s.a - a).abs() < 0.06 && (s.b - b).abs() < 0.06))
                    .map(|s| s.name.clone())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| format!("{}-{}", fmt_time(*a), fmt_time(*b)));
                let song = self.store.songs.get(id).map(|v| v.title.clone()).unwrap_or_default();
                let _ = passes;
                let right = mins_text(*secs);
                bar_row(ui, &format!("{}  ({})", loop_name, song), *secs, most, sum.colour(*id), &right);
            }
        }
    }

    /// "Artist - Title" of a song in the diary.
    fn song_name(&self, id: i64) -> String {
        self.store.songs.get(&id).map(|v| format!("{} - {}", v.artist, v.title)).unwrap_or_else(|| "A SONG".to_string())
    }

    /// One song of a diary day: back to the day's songs, its name and time, a small PLAY, and each loop worked on
    /// that day - its name, where it is, the time on it and the passes through it (click one to play that loop).
    fn diary_song_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, date: &str, id: i64, secs: u32) {
        let v = self.store.songs.get(&id).cloned();
        let name = v.as_ref().map(|v| format!("{} - {}", v.artist, v.title)).unwrap_or_else(|| "A SONG".to_string());
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if crate::ibtn(ui, &crate::IC_BACK, "SONGS", false).tip("Back to the songs of the day.").clicked() {
                self.diary_song = None;
            }
            if let Some(v) = v.clone() {
                if retro_btn(ui, "PLAY", false).tip("Play the song.").clicked() {
                    acts.push(Action::PlaySaved(v, None));
                }
            }
        });
        title_line(ui, &name, 2.5, pal().ink);
        title_line(ui, &format!("{} practised on {}", mins_text(secs), date), 1.5, pal().ink2);
        ui.add_space(4.0);
        let saved = self.store.sections.get(&id).cloned().unwrap_or_default();
        let name_of = |a: f32, b: f32| {
            saved
                .iter()
                .find(|s| (s.a - a).abs() < 0.06 && (s.b - b).abs() < 0.06)
                .map(|s| s.name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| "LOOP".to_string())
        };
        let mut loops: Vec<(f32, f32, u32, u32)> = self
            .store
            .days
            .iter()
            .find(|d| d.date == date)
            .map(|d| {
                d.loop_time
                    .iter()
                    .filter_map(|(k, (s, n))| store::parse_loop_key(k).filter(|x| x.0 == id).map(|x| (x.1, x.2, *s, *n)))
                    .collect()
            })
            .unwrap_or_default();
        loops.sort_by(|a, b| b.2.cmp(&a.2).then(b.3.cmp(&a.3)));
        if loops.is_empty() {
            dim_line(ui, "NO LOOP TIMES FOR THIS DAY (THEY ARE KEPT FROM 2026-10-14 ON)", 1.0, pal().dim);
            if !saved.is_empty() {
                ui.add_space(4.0);
                dim_line(ui, "THE SONG'S SAVED LOOPS", 1.0, pal().dim);
                loops = saved.iter().map(|s| (s.a, s.b, 0, 0)).collect();
            }
        } else {
            dim_line(ui, "LOOPS WORKED ON (CLICK ONE TO PLAY IT)", 1.0, pal().dim);
        }
        for (k, (a, b, s, n)) in loops.iter().enumerate() {
            let label = format!("{}   {}-{}", name_of(*a, *b), fmt_time(*a), fmt_time(*b));
            let _ = n;
            let right = if *s == 0 { String::new() } else { mins_text(*s) };
            if let Some(r) = list_row(ui, k, None, || label.clone(), || right.clone(), RowState::Normal, false, false) {
                if let Some(v) = v.clone() {
                    if r.tip("Play this loop.").clicked() {
                        acts.push(Action::PlaySaved(v, Some((*a, *b))));
                    }
                }
            }
        }
    }

    /// A YouTube or SoundCloud playlist on its own page: a way back, its name and length, PLAY and SHUFFLE, all its
    /// songs, and taking it off your playlists (asked first).
    fn list_page(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, kind: &str) {
        let yt = kind == "yt";
        let url = if yt { self.yt_cur.clone() } else { self.sc_cur.clone() }.unwrap_or_default();
        let list: Vec<store::Ext> = if yt { self.yt_results.clone() } else { self.sc_results.clone() };
        let name =
            self.store.lists.iter().find(|l| l.url == url).map(|l| l.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(
                || if yt && !self.yt_title.is_empty() { self.yt_title.clone() } else { sources::list_name(&url) },
            );
        let back = |me: &mut Self| {
            if yt {
                me.yt_results.clear();
                me.yt_title.clear();
                me.yt_cur = None;
            } else {
                me.sc_results.clear();
                me.sc_cur = None;
            }
            me.list_ask = None;
        };
        ui.add_space(6.0);
        if crate::ibtn(ui, &crate::IC_BACK, "PLAYLISTS", false).tip("Back to your playlists.").clicked() {
            back(self);
            return;
        }
        ui.add_space(4.0);
        title_line(ui, &name, 3.0, pal().ink);
        let tracks: Vec<Track> = list.iter().map(|e| e.to_track()).collect();
        let total: f32 = list.iter().map(|e| e.dur).sum();
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
            play_buttons(ui, &tracks, acts, false);
            if retro_btn(ui, "REFRESH", false).tip("Read the playlist again to pick up songs added since.").clicked() {
                self.read_list_again(kind, &url, acts);
            }
            if self.list_ask.as_deref() != Some(url.as_str())
                && retro_btn(ui, "REMOVE PLAYLIST...", false)
                    .tip("Take it off your playlists. The songs you kept stay.")
                    .clicked()
            {
                self.list_ask = Some(url.clone());
            }
        });
        if self.list_ask.as_deref() == Some(url.as_str()) {
            ui.horizontal_wrapped(|ui| {
                label(ui, "TAKE IT OFF YOUR PLAYLISTS?", 240.0);
                if retro_btn(ui, "YES, REMOVE IT", false).clicked() {
                    if let Some(i) = self.store.lists.iter().position(|l| l.url == url) {
                        acts.push(Action::RemoveList(i));
                    }
                    back(self);
                }
                if retro_btn(ui, "KEEP IT", false).clicked() {
                    self.list_ask = None;
                }
            });
        }
        ui.add_space(4.0);
        dim_line(ui, "RIGHT-CLICK A SONG TO KEEP IT, QUEUE IT OR PUT IT IN A PLAYLIST", 1.0, pal().dim);
        self.ext_list(ui, acts, if yt { 7 } else { 3 });
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
        let n = self.store.files.len();
        // how many, then one ADD with its choices under it (the count short, so they share a line)
        ui.horizontal_wrapped(|ui| {
            label(ui, &format!("{} {}", n, if n == 1 { "file" } else { "files" }), 0.0);
            let add = retro_btn(ui, "+ ADD", false).tip("Add files or a folder, or look for new files in your folders. Nothing is copied: they play from where they are.");
            let pop = ui.id().with("files_add");
            if add.clicked() {
                ui.memory_mut(|m| m.toggle_popup(pop));
            }
            egui::popup_below_widget(ui, pop, &add, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(220.0);
                if menu_item(ui, "Add files...") {
                    acts.push(Action::AddFiles);
                }
                if menu_item(ui, "Add a folder...") {
                    acts.push(Action::AddFolder);
                }
                if menu_item(ui, "Look for new files in your folders") {
                    acts.push(Action::Rescan);
                }
            });
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
            para(ui, &format!("Drag audio files or folders onto this window, or use + ADD. Nothing is copied: files stay where they are.{}", tail), pal().ink2);
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
                .tip("Downloads the free yt-dlp tool from its official GitHub page.")
                .clicked()
            {
                acts.push(Action::GetYtDlp);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let w = (ui.available_width() - 78.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_YT, &mut self.yt_in, rect, "Paste a YouTube link or playlist", false);
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
            let n = crate::sources::YT_READ.load(std::sync::atomic::Ordering::Relaxed);
            let msg = if self.yt_busy && self.yt_msg.starts_with("READING THE PLAYLIST") && n > 0 {
                format!("READING THE PLAYLIST... {} SO FAR", n)
            } else {
                self.yt_msg.clone()
            };
            if self.yt_busy {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
            }
            para(ui, &msg, col);
        }
        // a playlist open: its own page (back to the rest with the arrow)
        if self.yt_cur.is_some() && !self.yt_results.is_empty() {
            self.list_page(ui, acts, "yt");
            ui.add_space(20.0);
            return;
        }
        self.saved_lists(ui, acts, "yt");
        if self.store.yt.is_empty() {
            para(ui, "One-time setup: press GET YT-DLP. Then paste a link and press ADD.", pal().ink2);
            if PRACTICE {
                para(ui, "Play-alongs, lessons, live takes: anything you want to loop and slow down.", pal().ink2);
            }
            para(
                ui,
                "Using YouTube this way may go against its terms. It is for personal use only, at your own discretion.",
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
        title_line(ui, "search, paste a link, or type a user and press THEIR PLAYLISTS", 2.0, pal().ink2);
        ui.add_space(4.0);
        let have = sources::ytdlp_path().is_some();
        if !have {
            ui.horizontal_wrapped(|ui| {
                label(ui, "ONE-TIME SETUP", 150.0);
                if retro_btn(ui, "GET YT-DLP", true).tip("Downloads the free yt-dlp tool, which also reads SoundCloud.").clicked()
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
            let o = field(ui, &mut self.ed, F_SC, &mut self.sc_in, rect, "Search SoundCloud, or paste a link", false);
            if o.enter || retro_btn_w(ui, "GO", 70.0, false).clicked() {
                acts.push(Action::ScGo);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "THEIR PLAYLISTS", false)
                .tip("Lists all public playlists of a user. Type their SoundCloud name or profile link above first.")
                .clicked()
            {
                acts.push(Action::ScSets);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let n = sources::SC_BROWSER.load(Ordering::Relaxed) as usize;
            label(ui, "SIGN-IN FROM", 130.0);
            if retro_btn_w(ui, sources::SC_BROWSERS[n], 110.0, n > 0)
                .tip("Uses the SoundCloud sign-in of a browser on this PC, for private playlists and Go+. Click to change. Firefox works best.")
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
            para(ui, "Paste your likes link or a playlist link and press GO. Your likes link is soundcloud.com/YOU/likes. Public lists show right here; click a track to play it.", pal().ink2);
            para(ui, "Without a sign-in, private and Go+ tracks may play as previews or not at all. Using SoundCloud this way may go against its terms. Practice use is at your own discretion.", pal().dim);
        }
        // a playlist open: its own page
        let sc_list = self.sc_cur.as_ref().is_some_and(|u| self.store.lists.iter().any(|l| &l.url == u));
        if sc_list && !self.sc_results.is_empty() {
            self.list_page(ui, acts, "sc");
            ui.add_space(20.0);
            return;
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
                if r.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Open this group.").clicked() {
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
                    self.ed.anchor = None;
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
                label(ui, "DELETE THIS GROUP? ITS LOOPS STAY, WITH NO GROUP", 360.0);
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
            para(ui, "No loops in it yet. Right-click a loop in LOOPS or the PRACTICE panel to move it here.", pal().ink2);
            return;
        }
        for (k, (id, i, s)) in members.iter().enumerate() {
            let v = self.store.songs.get(id).cloned();
            let song = v.as_ref().map(|v| format!("{} - {}", v.artist, v.title)).unwrap_or_default();
            let title = format!("{}   {}-{}", s.name.to_uppercase(), fmt_time(s.a), fmt_time(s.b));
            let sub = song.clone();
            if let Some(r) = list_row(ui, k, None, || title.clone(), || sub.clone(), RowState::Normal, false, false) {
                let r = r.on_hover_cursor(egui::CursorIcon::PointingHand).tip(if s.note.is_empty() {
                    "Click to play it on a loop.".to_string()
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
                    self.ed.anchor = None;
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
                        let w = 560.0_f32.min(screen.width() - 40.0);
                        ui.set_width(w);
                        title_line(ui, "WHAT'S NEW", 3.0, pal().ink);
                        egui::ScrollArea::vertical().max_height((screen.height() * 0.45).max(160.0)).show(ui, |ui| {
                            ui.set_width(w - 16.0);
                            for (tag, body) in &notes {
                                let this = tag.rsplit('.').next().and_then(|n| n.parse::<u64>().ok()) == Some(me);
                                section_header(
                                    ui,
                                    &format!("{}{}", tag.to_uppercase(), if this { "  (THIS VERSION)" } else { "" }),
                                );
                                for line in body.lines() {
                                    // the notes are written for GitHub; here, only what changes for you: one line a
                                    // change (the details under each, the headings and the download instructions are
                                    // left out)
                                    if line.trim().trim_start_matches('#').trim().eq_ignore_ascii_case("downloads") {
                                        break;
                                    }
                                    if line.trim_start().starts_with('#')
                                        || (line.starts_with(char::is_whitespace) && line.trim_start().starts_with("- "))
                                    {
                                        continue;
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
                                    crate::wrapped(ui, &text, col);
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

    /// A pack being opened, over the dimmed app: each piece in it, ticked (one that would replace yours starts
    /// unticked, and says so), then UNLOCK or cancel. Its loops and playlists come after, in the import window.
    pub(crate) fn pack_panel(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        let Some(mut p) = self.pack_in.take() else { return };
        let screen = ctx.screen_rect();
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("pack_dim"))).rect_filled(
            screen,
            0.0,
            egui::Color32::from_black_alpha(170),
        );
        let mut close = false;
        egui::Area::new(egui::Id::new("pack_panel")).order(egui::Order::Tooltip).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO).show(
            ctx,
            |ui| {
                egui::Frame::none().fill(pal().app_bg).stroke(egui::Stroke::new(2.0_f32, pal().edge)).inner_margin(16.0).show(ui, |ui| {
                    ui.set_width(440.0_f32.min(screen.width() - 40.0));
                    title_line(ui, "OPEN A PACK", 3.0, pal().ink);
                    para(ui, "Tick what to bring in. Each piece goes where it belongs and switches its part of Backline on.", pal().ink2);
                    ui.add_space(4.0);
                    for part in p.parts.iter_mut() {
                        let label = if part.replaces { format!("{}  (REPLACES YOURS)", part.label) } else { part.label.clone() };
                        if crate::check_box(ui, &label, part.on).clicked() {
                            part.on = !part.on;
                        }
                    }
                    if p.share.is_some() {
                        para(ui, "LOOPS & PLAYLISTS: you pick which ones next. They're added to yours, nothing is replaced.", pal().ink2);
                    }
                    if p.parts.iter().any(|x| x.replaces && x.on) {
                        para(ui, "A ticked piece marked REPLACES YOURS takes the place of the one you have.", pal().red);
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if retro_btn(ui, "UNLOCK", false).clicked() {
                            acts.push(Action::PackUnlock);
                        }
                        if retro_btn(ui, "CANCEL", false).clicked() {
                            close = true;
                        }
                    });
                });
            },
        );
        if !(close || ctx.input(|i| i.key_pressed(egui::Key::Escape))) {
            self.pack_in = Some(p);
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
                            if retro_btn(ui, "ALL LOOPS", false).tip("Every loop, no playlists.").clicked() {
                                pick = (all_songs.clone(), std::collections::HashSet::new());
                            }
                            if retro_btn(ui, "ALL PLAYLISTS", false).tip("Every playlist, no loops.").clicked() {
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
                                    &format!("{} FILES AREN'T ON THIS COMPUTER. THEIR LOOPS COME IN BUT WON'T PLAY.", t.missing),
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
                                "MADE BY A NEWER BACKLINE, SO SOME OF IT MAY BE LEFT OUT. UPDATE TO BRING IN EVERYTHING.",
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
                .tip("Saves every loop and playlist to one file. Email it or put it on a USB stick or OneDrive, then import it on the other computer.")
                .clicked()
            {
                acts.push(Action::ShareExport);
            }
            if retro_btn(ui, "IMPORT...", false)
                .tip("Brings in a file from another computer. It adds the loops and playlists you don't have and changes nothing you do.")
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
                        dim_line(ui, "NONE YET. TYPE A NAME BELOW AND PRESS NEW ON TIDAL.", 1.0, pal().dim)
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
            title_line(ui, "your own lists, mixing Tidal, SoundCloud, YouTube and files", 2.0, pal().ink2);
            ui.add_space(4.0);
        }
        ui.horizontal_wrapped(|ui| {
            let w = (ui.available_width() - 78.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_NEW_PL, &mut self.new_pl, rect, "New playlist name", false);
            if !self.pl_tidal && (o.enter || retro_btn_w(ui, "NEW", 70.0, false).tip("Makes a new Backline playlist.").clicked())
            {
                acts.push(Action::PlaylistNew(self.pl_pick.clone()));
            }
            // a new playlist on your Tidal account, with this song in it
            if let Some(t) = self.pl_pick.as_ref().filter(|t| t.id > 0 && self.api.has_token() && self.pl_loop.is_none()) {
                if retro_btn(ui, "NEW ON TIDAL", false)
                    .tip("Makes a playlist on your Tidal account, with the song in it.")
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
                    .tip("Turns the queue into a playlist. Type a name above first, or it gets a number.")
                    .clicked()
                {
                    acts.push(Action::PlaylistFromQueue);
                }
            });
            if self.store.playlists.is_empty() {
                para(ui, "Right-click any song and choose ADD TO A BACKLINE PLAYLIST. Playlists live on this PC only, and one list can mix every source.", pal().ink2);
            }
            ui.add_space(4.0);
            // your playlists, filtered by name as you type
            let filter = self.lib_filter.trim().to_string();
            if !self.store.playlists.is_empty() {
                self.filter_box(ui, "Filter your playlists");
            }
            let order: Vec<usize> = if filter.is_empty() {
                (0..self.store.playlists.len()).collect()
            } else {
                crate::filtered(&filter, &self.store.playlists, |p| vec![p.name.clone()], 4)
            };
            if !filter.is_empty() && order.is_empty() && !self.store.playlists.is_empty() {
                title_line(ui, "No playlists match.", 2.0, pal().ink2);
            }
            for (row, &i) in order.iter().enumerate() {
                let pl = &self.store.playlists[i];
                let n = pl.items.len();
                if let Some(r) =
                    list_row(ui, row, Some(i + 1), || pl.name.clone(), || format!("{} tracks", n), RowState::Normal, false, false)
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
        title_line(ui, &format!("{} tracks (right-click one to move or remove it)", items.len()), 2.0, pal().ink2);
        ui.add_space(4.0);
        if items.is_empty() {
            para(ui, "Empty. Right-click any song and choose ADD TO A BACKLINE PLAYLIST.", pal().ink2);
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
            crate::row_stems(v.id);
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
    /// While a tune name is typed: cards for the songs it could be (the recommended tunes, your own, and the songs in
    /// your library and queue), spelled right - accents and capitals - so a name typed fast comes out right. Click one
    /// to add it under that name, or KEEP AS TYPED.
    fn tune_suggestions(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let typed = self.new_tune.trim().to_string();
        // only the cards asked for (Enter or ADD), and only while that name is still in the box
        if typed.is_empty() || TUNE_ASKED.with(|a| *a.borrow() != typed) {
            return;
        }
        let found = TUNE_FOUND.with(|f| f.borrow().clone());
        // nothing to offer, or exactly what was typed: no cards
        if found.is_empty() || (found.len() == 1 && found[0].1 == typed) {
            return;
        }
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
            for (_, n, who) in &found {
                let w = (text_w(n, 2.0).max(text_w(who, 1.3)) + 28.0).clamp(120.0, 300.0);
                let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 44.0), Sense::click());
                let hot = resp.hovered();
                ui.painter().rect_filled(r, 8.0, if hot { pal().beige } else { pal().beige.lerp_to_gamma(pal().lcd, 0.35) });
                ui.painter().rect_stroke(r, 8.0, egui::Stroke::new(1.0_f32, pal().beige_h));
                crate::font::keep_caps(|| {
                    crate::row_text(ui.painter(), Pos2::new(r.min.x + 12.0, r.min.y + 15.0), Align::Min, n, w - 20.0, pal().ink);
                    crate::row_text(
                        ui.painter(),
                        Pos2::new(r.min.x + 12.0, r.min.y + 32.0),
                        Align::Min,
                        who,
                        w - 20.0,
                        pal().ink2,
                    );
                });
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Add it under this name.").clicked() {
                    self.new_tune = n.clone();
                    TUNE_ASKED.with(|a| a.borrow_mut().clear());
                    acts.push(Action::NewTune);
                }
            }
            if retro_btn(ui, "KEEP AS TYPED", false)
                .tip(format!("Add \"{}\" just as you typed it. (Enter does the same.)", typed))
                .clicked()
            {
                TUNE_ASKED.with(|a| a.borrow_mut().clear());
                acts.push(Action::NewTune);
            }
        });
        ui.add_space(4.0);
    }

    /// The tune names closest to `typed`, best first (at most 5): (score, name, who).
    fn tune_matches(&self, typed: &str) -> Vec<(u32, String, String)> {
        // (name, who) from every place a tune name could come from
        let mut pool: Vec<(String, String)> = Vec::new();
        for kinds in [crate::canon::SESSION_TUNES, crate::canon::GOSPEL_TUNES] {
            for (_, list) in kinds.iter() {
                for (n, who) in list.iter() {
                    pool.push((n.to_string(), who.to_string()));
                }
            }
        }
        for tn in &self.store.tunes {
            pool.push((tn.name.clone(), "In your tunes".to_string()));
        }
        let mut songs: Vec<&crate::api::Track> = self.queue.iter().collect();
        if let Some(pg) = &self.page {
            songs.extend(pg.tracks.iter());
        }
        for tr in songs {
            let name = crate::chart::clean_title(&tr.title);
            if !name.is_empty() {
                pool.push((name, tr.artists_text()));
            }
        }
        // the closest first, each name once
        let mut found: Vec<(u32, String, String)> = Vec::new();
        for (n, who) in pool {
            if let Some(s) = crate::filter_score(typed, &[&n]) {
                if !found.iter().any(|f| f.1.eq_ignore_ascii_case(&n)) {
                    found.push((s, n, who));
                }
            }
        }
        found.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.len().cmp(&b.1.len())));
        found.truncate(5);
        found
    }

    /// TUNES > RECOMMENDED: the tunes that get called, by kind (jazz or gospel). Click one to see its chart on the
    /// lead sheet - it only becomes one of yours with + MY TUNES.
    fn recommended_tunes(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "The tunes called at sessions, sorted by kind. Knowing a mix of kinds keeps you ready for anything. Click one to see its chart on the right, and press + MY TUNES to keep it.", pal().ink2);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "JAZZ", !self.tunes_gospel).clicked() {
                self.tunes_gospel = false;
            }
            if retro_btn(ui, "GOSPEL", self.tunes_gospel).clicked() {
                self.tunes_gospel = true;
            }
        });
        let kinds = if self.tunes_gospel { crate::canon::GOSPEL_TUNES } else { crate::canon::SESSION_TUNES };
        let showing = self.prac_chart.as_ref().map(|c| c.0.clone());
        for (kind, list) in kinds.iter() {
            section_header(ui, kind);
            for (name, who) in list.iter() {
                let mine = self.find_tune(name).is_some();
                let on = showing.as_deref().is_some_and(|s| s.eq_ignore_ascii_case(name));
                ui.horizontal_wrapped(|ui| {
                    if retro_btn(ui, &crate::font::as_label(&name), on)
                        .tip(format!("{}. Shows its chart on the lead sheet.", who))
                        .clicked()
                    {
                        acts.push(Action::PreviewTune(name.to_string()));
                    }
                    if mine {
                        label(ui, "IN YOUR TUNES", 0.0);
                    } else if retro_btn(ui, "+ MY TUNES", false).tip("Keep it in your repertoire.").clicked() {
                        acts.push(Action::AddRecommended(name.to_string()));
                    }
                    if self.store.routine_set.get(*kind).is_some_and(|s| s.eq_ignore_ascii_case(name)) {
                        label(ui, "IN YOUR SET", 0.0);
                    } else if retro_btn(ui, "MY SET", false)
                        .tip("The tune you are working on for this kind, in DIARY > ROUTINE.")
                        .clicked()
                    {
                        acts.push(Action::SetTune(kind.to_string(), name.to_string()));
                    }
                });
            }
        }
        ui.add_space(20.0);
    }

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
            // (how many: on the tab)
            let yours = format!("YOUR TUNES · {}", self.store.tunes.len());
            if let Some(i) = tab_row(ui, &[yours.as_str(), "RECOMMENDED"], self.tunes_tab as usize) {
                self.tunes_tab = i as u8;
            }
            ui.add_space(4.0);
            if self.tunes_tab == 1 {
                self.recommended_tunes(ui, acts);
                return;
            }
        }
        ui.horizontal_wrapped(|ui| {
            let w = (ui.available_width() - 78.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_NEW_TUNE, &mut self.new_tune, rect, "Add a tune, or paste an iReal link", false);
            let go = o.enter
                || retro_btn_w(ui, "ADD", 70.0, false)
                    .tip("Type a tune's name, or paste an iReal Pro link or playlist: every song in it becomes a tune with its chart.")
                    .clicked();
            if go && self.new_tune.trim().to_ascii_lowercase().starts_with("ireal") {
                // an iReal link: imported, not added as a name
                self.ireal_in = std::mem::take(&mut self.new_tune).trim().to_string();
                acts.push(Action::ImportIreal(None));
            } else if go {
                // looked up once, on Enter or ADD: a better spelling shows as cards to pick from; none, and it's added
                let typed = self.new_tune.trim().to_string();
                let found = if typed.chars().count() >= 3 { self.tune_matches(&typed) } else { Vec::new() };
                let exact = found.iter().any(|f| f.1 == typed);
                if found.is_empty() || exact || TUNE_ASKED.with(|a| *a.borrow() == typed) {
                    TUNE_ASKED.with(|a| a.borrow_mut().clear());
                    acts.push(Action::NewTune);
                } else {
                    TUNE_ASKED.with(|a| *a.borrow_mut() = typed.clone());
                    TUNE_FOUND.with(|f| *f.borrow_mut() = found);
                }
            }
        });
        self.tune_suggestions(ui, acts);
        ui.add_space(4.0);
        if self.store.tunes.is_empty() {
            para(ui, "Your repertoire, with the recordings you study for each tune. To add a recording, right-click a song in any list and choose ADD TO A TUNE. Or play it and press ADD CURRENT inside a tune.", pal().ink2);
        }
        // your tunes under the kind each is (a blues, rhythm changes, a ballad...): the kind it has in your set, else
        // the kind the session lists give it, else OTHER; the ones in your set marked
        let kinds: Vec<&str> = crate::canon::SESSION_TUNES.iter().chain(crate::canon::GOSPEL_TUNES.iter()).map(|k| k.0).collect();
        let kind_of = |name: &str| -> Option<&str> {
            if let Some((k, _)) = self.store.routine_set.iter().find(|(_, v)| v.eq_ignore_ascii_case(name)) {
                return kinds.iter().copied().find(|x| x == k);
            }
            crate::canon::SESSION_TUNES
                .iter()
                .chain(crate::canon::GOSPEL_TUNES.iter())
                .find(|(_, l)| l.iter().any(|(n, _)| n.eq_ignore_ascii_case(name)))
                .map(|k| k.0)
        };
        let mut order: Vec<(Option<&str>, usize)> =
            self.store.tunes.iter().enumerate().map(|(i, t)| (kind_of(&t.name), i)).collect();
        let rank = |k: &Option<&str>| k.map_or(usize::MAX, |k| kinds.iter().position(|x| *x == k).unwrap_or(usize::MAX - 1));
        order.sort_by_key(|(k, i)| (rank(k), *i));
        let mut last: Option<Option<&str>> = None;
        for (kind, i) in order {
            if last != Some(kind) {
                section_header(ui, kind.unwrap_or("OTHER"));
                last = Some(kind);
            }
            let t = &self.store.tunes[i];
            let in_set = self.store.routine_set.values().any(|v| v.eq_ignore_ascii_case(&t.name));
            let mins = self.store.tune_secs(&t.name) / 60;
            let r = list_row(
                ui,
                i,
                None,
                || {
                    if crate::winamp_ui::chrome_on() {
                        // (with a skin: how well you know it is the dots drawn on the row, not text)
                        format!("        {}", t.name)
                    } else {
                        format!(
                            "[{}] {}",
                            "#".repeat(t.status.min(3) as usize + 1) + &"-".repeat(3 - t.status.min(3) as usize),
                            t.name
                        )
                    }
                },
                // what you have for it, in words: in your set, recordings, time practised
                || {
                    let mut bits: Vec<String> = Vec::new();
                    if in_set {
                        bits.push("In your set".to_string());
                    }
                    bits.push(match t.versions.len() {
                        0 => "No recordings yet".to_string(),
                        1 => "1 recording".to_string(),
                        n => format!("{} recordings", n),
                    });
                    if mins > 0 {
                        bits.push(mins_text(mins * 60).to_lowercase());
                    }
                    bits.join(" · ")
                },
                RowState::Normal,
                false,
                false,
            );
            if let Some(r) = r.as_ref().filter(|_| crate::winamp_ui::chrome_on()) {
                // how well you know it: four small dots, filled up to its level
                let lv = t.status.min(3) as usize;
                let x0 = r.rect.min.x + 12.0;
                for k in 0..4 {
                    let c = Pos2::new(x0 + k as f32 * 7.0, r.rect.center().y);
                    if k <= lv {
                        ui.painter().circle_filled(c, 2.6, pal().bar_txt);
                    } else {
                        ui.painter().circle_stroke(c, 2.4, egui::Stroke::new(1.0_f32, pal().ink2));
                    }
                }
            }
            let r = r.map(|r| r.on_hover_text(COMFORT[t.status.min(3) as usize]));
            if let Some(r) = r {
                if r.clicked() {
                    match &self.pick {
                        Some(p) => acts.push(Action::AddToTune(i, p.clone())),
                        None => acts.push(Action::OpenTune(Some(i))),
                    }
                }
            }
        }
        ui.add_space(20.0);
    }

    /// Under a tune's name, after LOOK UP: its proper title when the name looks short of it or misspelled (from the
    /// recordings found, and the tunes Backline knows), to take with RENAME or leave with KEEP.
    fn name_guess_line(&mut self, ui: &mut egui::Ui, i: usize) {
        let name = self.store.tunes[i].name.clone();
        if self.look_busy || self.look_for != name || self.name_kept.contains(&name) {
            return;
        }
        if self.name_guess.0 != name || self.name_guess.1 != self.look_tracks.len() {
            let titles: Vec<&str> = self.look_tracks.iter().map(|t| t.0.title.as_str()).collect();
            let known: Vec<&str> = crate::canon::SESSION_TUNES
                .iter()
                .chain(crate::canon::GOSPEL_TUNES.iter())
                .flat_map(|k| k.1.iter().map(|t| t.0))
                .collect();
            let guess = crate::likely_title(&name, &titles, &known)
                // not one of your other tunes' names already
                .filter(|g| !self.store.tunes.iter().any(|t| t.name.eq_ignore_ascii_case(g)));
            self.name_guess = (name.clone(), self.look_tracks.len(), guess);
        }
        let Some(guess) = self.name_guess.2.clone() else { return };
        ui.horizontal_wrapped(|ui| {
            ui.add_space(76.0);
            label(ui, &format!("Did you mean: {}?", guess), 0.0);
            if retro_btn(ui, "RENAME", false).tip("Give the tune this name, everywhere it is used.").clicked() {
                self.store.rename_tune(&name, &guess);
                self.look_for = guess.clone();
                self.store_dirty = true;
            }
            if retro_btn(ui, "KEEP", false).tip("Keep the name as it is.").clicked() {
                self.name_kept.push(name.clone());
                self.dirty = true;
            }
        });
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
            if retro_btn(ui, "CHART", self.rtab == 1).tip("Show the chord changes on the right.").clicked() {
                acts.push(Action::RightTab(if self.rtab == 1 { 0 } else { 1 }));
            }
        });
        ui.add_space(4.0);
        if let Some(n) = comfort_meter(ui, status) {
            acts.push(Action::Status(i, n));
        }
        let o = field_row(ui, &mut self.ed, F_T_NAME, "NAME", &mut self.store.tunes[i].name, "Tune name", None);
        self.store_dirty |= o.changed;
        self.name_guess_line(ui, i);
        let o = field_row(ui, &mut self.ed, F_T_KEY, "KEY", &mut self.store.tunes[i].key, "Bb, Dm", None);
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
            "What to work on, the form, tricky bars",
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
        // who wrote it, when, from where, the style: one line, the boxes to fix them under DETAILS
        ui.horizontal_wrapped(|ui| {
            let t = &self.store.tunes[i];
            let bits: Vec<&str> =
                [&t.written_by, &t.year, &t.from, &t.style].iter().map(|v| v.trim()).filter(|v| !v.is_empty()).collect();
            let line = if bits.is_empty() { "Who wrote it, when, where from".to_string() } else { bits.join(" · ") };
            let room = (ui.available_width() - 110.0).max(60.0);
            let (r, _) = ui.allocate_exact_size(Vec2::new(room.min(text_w(&line, 2.0) + 12.0), bh()), Sense::hover());
            ptext_fit(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, &line, 2.0, room - 8.0, pal().ink2);
            if retro_btn(ui, if self.tune_more { "HIDE DETAILS" } else { "DETAILS" }, self.tune_more)
                .tip("Written by, year, where it's from and its style, to fix by hand.")
                .clicked()
            {
                self.tune_more = !self.tune_more;
            }
        });
        if self.tune_more {
            for (fid, lab, hint) in [
                (crate::tools::F_T_WRITER, "WRITTEN BY", "Composer and lyricist"),
                (crate::tools::F_T_YEAR, "YEAR", "When it was written or first recorded"),
                (crate::tools::F_T_FROM, "FROM", "The show, film or album it comes from"),
                (crate::tools::F_T_STYLE, "STYLE", "Swing, ballad, bossa, waltz"),
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
        }
        ui.add_space(4.0);
        // what to do with it: one row, the rest under "..."
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "PLAY ALL", false).clicked() {
                acts.push(Action::PlayTune(i, false));
            }
            if retro_btn(ui, "SHUFFLE", false).clicked() {
                acts.push(Action::PlayTune(i, true));
            }
            if crate::ibtn(ui, &crate::IC_SEARCH, if self.look_busy { "LOOKING..." } else { "LOOK UP" }, self.look_busy)
                .tip("Finds who wrote it and its most popular recordings on Tidal. It only searches when you press this.")
                .clicked()
            {
                acts.push(Action::LookUp(i));
            }
            if retro_btn(ui, "ADD CURRENT", false).tip("Add the recording that is playing now.").clicked() {
                acts.push(Action::AddCurrentToTune(i));
            }
            let more = retro_btn_w(ui, "...", 34.0, false).tip("What LOOK UP favors, play-alongs, remove the tune.");
            let pop = ui.id().with("tune_more_menu");
            if more.clicked() {
                ui.memory_mut(|m| m.toggle_popup(pop));
            }
            egui::popup_below_widget(ui, pop, &more, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(240.0);
                dim_line(ui, "LOOK UP FAVORS", 1.0, pal().dim);
                for (k, name) in ["Auto (jazz for a known standard)", "Jazz", "Gospel", "Any"].iter().enumerate() {
                    let mark = if self.look_style as usize % 4 == k { "> " } else { "   " };
                    if menu_item(ui, &format!("{}{}", mark, name)) {
                        self.look_style = k as u8;
                    }
                }
                ui.separator();
                if menu_item(ui, "   Play-alongs on YouTube...") {
                    acts.push(Action::PlayAlong(tname.clone()));
                }
                if menu_item(ui, "   Remove this tune...") {
                    if !self.del_arm {
                        acts.push(Action::RemoveTune(i));
                    }
                }
            });
        });
        // asked before it goes
        if self.del_arm {
            ui.horizontal_wrapped(|ui| {
                label(ui, &format!("Remove {}, with its recordings?", tname), 0.0);
                if retro_btn(ui, "REMOVE", true).clicked() {
                    acts.push(Action::RemoveTune(i));
                }
                if retro_btn(ui, "KEEP IT", false).clicked() {
                    self.del_arm = false;
                }
            });
        }
        // a search of your own, once LOOK UP has been used
        if self.look_busy || self.look_for == tname || !self.look_q.is_empty() {
            ui.horizontal_wrapped(|ui| {
                let w = (ui.available_width() - 4.0).max(120.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                let o = field(
                    ui,
                    &mut self.ed,
                    F_LOOK,
                    &mut self.look_q,
                    rect,
                    "Look up a title, or title and artist (empty: this tune)",
                    false,
                );
                if o.enter {
                    acts.push(Action::LookUp(i));
                }
            });
        }

        // ---- the recordings
        section_header(ui, &format!("RECORDINGS TO STUDY ({})", nvers));
        if nvers == 0 {
            para(
                ui,
                "Nothing here yet. Play a recording and press ADD CURRENT, or right-click any song and choose ADD TO A TUNE.",
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
            crate::row_stems(id);
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
            dim_line(ui, "SEARCHING... (WHO WROTE IT FIRST, THEN TIDAL: IT TAKES A FEW SECONDS)", 2.0, pal().ink2);
        } else if self.look_for == tname && self.look_tracks.is_empty() {
            // looked up, and nothing came back worth studying
            dim_line(ui, "NO RECORDINGS FOUND ON TIDAL. TRY ITS FULL TITLE, OR TITLE AND ARTIST, IN THE BOX ABOVE", 1.0, pal().ink2);
        }
        if self.look_for == tname && !self.look_tracks.is_empty() {
            section_header(ui, "RECORDINGS TO STUDY");
            dim_line(
                ui,
                "BEST FIRST, BY TIDAL POPULARITY AND ARTISTS YOU RATE UP, WITHOUT KARAOKE OR COMPILATIONS. CLICK TO PLAY, RIGHT-CLICK TO ADD.",
                1.0,
                pal().dim,
            );
            let pops: Vec<u32> = self.look_tracks.iter().map(|x| x.1).collect();
            let found: Vec<Track> = self.look_tracks.iter().map(|x| x.0.clone()).collect();
            table_header(ui, "TRACK", "POPULARITY", "LENGTH", true);
            for (k, t) in found.iter().enumerate() {
                let state = if playing == Some(t.id) { RowState::Playing } else { RowState::Normal };
                crate::row_stems(t.id);
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
                    let r = r.tip(format!("From the album {}.", t.album));
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
        self.name_old_loops();
        let data = self.spot_data();
        // the song picked: one asked for, else the song playing, else the first
        let playing = self.cur_track().map(|t| t.id);
        let has = |id: i64| data.songs.iter().any(|s| s.id == id);
        let pick = self.loops_pick.filter(|p| has(*p)).or(playing.filter(|p| has(*p))).or(data.songs.first().map(|s| s.id));
        if ui.available_width() >= 640.0 {
            // two panes: the songs on the left, the one picked on the right
            ui.horizontal_top(|ui| {
                let lw = 250.0;
                ui.allocate_ui_with_layout(Vec2::new(lw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(lw);
                    self.loops_controls(ui, acts);
                    ui.add_space(6.0);
                    self.loops_songs(ui, &data, pick);
                });
                let top = ui.cursor().min;
                ui.add_space(14.0);
                ui.painter().line_segment(
                    [Pos2::new(top.x + 6.0, top.y), Pos2::new(top.x + 6.0, top.y + 600.0)],
                    Stroke::new(1.0_f32, pal().row_alt),
                );
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    self.loops_detail(ui, acts, &data, pick);
                });
            });
        } else if let Some(open) = self.loops_pick.filter(|p| has(*p)) {
            // narrow: one song at a time, the list a click away
            if retro_btn(ui, "< ALL SONGS", false).clicked() {
                self.loops_pick = None;
            }
            ui.add_space(8.0);
            self.loops_detail(ui, acts, &data, Some(open));
        } else {
            self.loops_controls(ui, acts);
            ui.add_space(6.0);
            self.loops_songs(ui, &data, None);
            if self.loops_mode == 2 {
                ui.add_space(14.0);
                self.groups_view(ui, acts);
            }
        }
        ui.add_space(20.0);
    }

    /// The filter, which week, and the rest (what to show, the order, export / import) in "...".
    fn loops_controls(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let w = ui.available_width().min(250.0);
        ui.horizontal(|ui| {
            let (ic, _) = ui.allocate_exact_size(Vec2::new(22.0, bh()), Sense::hover());
            crate::icon_in(ui.painter(), ic.shrink(4.0), &crate::IC_FILTER, pal().ink2);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w - 30.0, bh()), Sense::hover());
            let _ = field(ui, &mut self.ed, crate::tools::F_LOOPS_FILTER, &mut self.loops_filter, rect, "Filter", false);
        });
        ui.horizontal(|ui| {
            if retro_btn(ui, "THIS WEEK", !self.loops_all).tip("Songs practiced or looped in the last 7 days.").clicked() {
                self.loops_all = false;
            }
            if retro_btn(ui, "ALL", self.loops_all).tip("Every song with a loop or a bookmark.").clicked() {
                self.loops_all = true;
            }
            let more = retro_btn_w(ui, "...", 34.0, false).tip("What to show, the order, export and import.");
            let pop = ui.id().with("loops_more");
            if more.clicked() {
                ui.memory_mut(|m| m.toggle_popup(pop));
            }
            egui::popup_below_widget(ui, pop, &more, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(230.0);
                dim_line(ui, "SHOW", 1.0, pal().dim);
                for (i, name) in ["Loops and bookmarks", "Loops", "Bookmarks"].iter().enumerate() {
                    let mark = if self.loops_show as usize == i { "> " } else { "   " };
                    if menu_item(ui, &format!("{}{}", mark, name)) {
                        self.loops_show = i as u8;
                        self.dirty = true;
                    }
                }
                dim_line(ui, "ORDER", 1.0, pal().dim);
                let cur = if self.loops_mode == 2 { 3 } else { self.loops_sort.min(2) as usize };
                for (i, name) in ["Recent", "A-Z", "Time spent", "By group"].iter().enumerate() {
                    let mark = if cur == i { "> " } else { "   " };
                    if menu_item(ui, &format!("{}{}", mark, name)) {
                        if i == 3 {
                            self.loops_mode = 2;
                        } else {
                            self.loops_mode = 0;
                            self.loops_sort = i as u8;
                        }
                        self.dirty = true;
                    }
                }
                dim_line(ui, "SHARE", 1.0, pal().dim);
                if menu_item(ui, "   Export loops & playlists...") {
                    acts.push(Action::ShareExport);
                }
                if menu_item(ui, "   Import from another computer...") {
                    acts.push(Action::ShareImport);
                }
            });
        });
    }

    /// The songs, filtered and in order, with their loops and bookmarks.
    fn spot_data(&self) -> SpotData {
        let today = store::today_days();
        let week_start = store::date_str(today - 6);
        let mut week: HashMap<i64, u32> = HashMap::new();
        for d in self.store.days.iter().filter(|d| d.date >= week_start) {
            for (id, s) in &d.songs {
                *week.entry(*id).or_default() += s;
            }
        }
        // time spent on each loop, from the diary
        let mut spent: HashMap<(i64, i32, i32), u32> = HashMap::new();
        for d in &self.store.days {
            for (k, v) in &d.loop_time {
                if let Some((id, a, b)) = store::parse_loop_key(k) {
                    *spent.entry((id, (a * 10.0).round() as i32, (b * 10.0).round() as i32)).or_default() += v.0;
                }
            }
        }
        let spent_on = |id: i64, s: &store::Section| {
            spent.get(&(id, (s.a * 10.0).round() as i32, (s.b * 10.0).round() as i32)).copied().unwrap_or(0)
        };
        let (show_loops, show_marks) = (self.loops_show != 2, self.loops_show != 1);
        // the songs: practised this week, or with a loop saved this week (ALL: every song with a loop or bookmark)
        let mut ids: Vec<i64> = week.keys().copied().collect();
        for (id, l) in &self.store.sections {
            if !l.is_empty() && (self.loops_all || l.iter().any(|s| s.saved >= week_start)) && !ids.contains(id) {
                ids.push(*id);
            }
        }
        if self.loops_all {
            for (id, l) in &self.store.marks {
                if !l.is_empty() && !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        let filter = self.loops_filter.trim().to_string();
        let mut songs: Vec<SpotSong> = Vec::new();
        for id in &ids {
            let v = self.store.songs.get(id).cloned();
            let song = v.as_ref().map(|v| format!("{} {} {}", v.artist, v.title, v.album)).unwrap_or_default();
            let song_hit = filter.is_empty() || crate::filter_score(&filter, &[&song]).is_some();
            let hit = |texts: &[&str]| song_hit || crate::filter_score(&filter, texts).is_some();
            let loops: Vec<(usize, store::Section, u32)> = if show_loops {
                self.store
                    .sections
                    .get(id)
                    .into_iter()
                    .flatten()
                    .enumerate()
                    .filter(|(_, s)| hit(&[&s.name, &s.group, &s.note]))
                    .map(|(i, s)| (i, s.clone(), spent_on(*id, s)))
                    .collect()
            } else {
                Vec::new()
            };
            let marks: Vec<(usize, store::Bookmark)> = if show_marks {
                self.store.marks.get(id).into_iter().flatten().cloned().enumerate().filter(|(_, m)| hit(&[&m.name])).collect()
            } else {
                Vec::new()
            };
            // a song only practised shows when nothing narrows the list
            if loops.is_empty() && marks.is_empty() && (!song_hit || !filter.is_empty() || self.loops_show == 2) {
                continue;
            }
            let mins = week.get(id).copied().unwrap_or(0);
            let total = loops.iter().map(|l| l.2).sum::<u32>() + mins;
            let newest = loops.iter().map(|l| l.1.saved.clone()).max().unwrap_or_default();
            songs.push(SpotSong { id: *id, v, loops, marks, mins, total, newest });
        }
        let name_of = |s: &SpotSong| s.v.as_ref().map(|v| v.title.to_lowercase()).unwrap_or_default();
        match self.loops_sort {
            1 => songs.sort_by(|a, b| name_of(a).cmp(&name_of(b)).then(a.id.cmp(&b.id))),
            2 => songs.sort_by(|a, b| b.total.cmp(&a.total).then(a.id.cmp(&b.id))),
            _ => songs.sort_by(|a, b| b.mins.cmp(&a.mins).then(b.newest.cmp(&a.newest)).then(a.id.cmp(&b.id))),
        }
        SpotData { songs }
    }

    /// The left pane: one two-line row per song.
    fn loops_songs(&mut self, ui: &mut egui::Ui, data: &SpotData, pick: Option<i64>) {
        let playing = self.cur_track().map(|t| t.id);
        let row = |ui: &mut egui::Ui, key: i64, title: &str, sub: &str| -> bool {
            let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
            let p = ui.painter();
            let on = pick == Some(key);
            if on {
                p.rect_filled(r, 6.0, pal().row_sel);
            } else if resp.hovered() {
                p.rect_filled(r, 6.0, pal().sel);
            }
            let x = r.min.x + 10.0;
            let col = if playing == Some(key) { pal().bar_txt } else { pal().ink };
            let clip = Rect::from_min_max(r.min, Pos2::new(r.max.x - 8.0, r.max.y)).intersect(p.clip_rect());
            ptext(&p.with_clip_rect(clip), Pos2::new(x, r.min.y + 15.0), Align::Min, title, 1.6, col);
            ptext_fit(p, Pos2::new(r.min.x + 10.0, r.min.y + 32.0), Align::Min, sub, 1.0, r.width() - 20.0, pal().ink2);
            resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
        };
        if data.songs.is_empty() {
            let msg = if !self.loops_filter.trim().is_empty() {
                "Nothing matches."
            } else if self.loops_all {
                "No loops or bookmarks yet. In PRACTICE, set A and B and press SAVE LOOP, or press + MARK."
            } else {
                "Nothing this week yet. Practice a song or save a loop and it shows up here. ALL has every song."
            };
            para(ui, msg, pal().ink2);
            return;
        }
        for s in &data.songs {
            let (title, artist) = match &s.v {
                Some(v) => (v.title.clone(), v.artist.clone()),
                None => ("A song (play it once to name it)".to_string(), String::new()),
            };
            let spots = s.loops.len() + s.marks.len();
            let mut sub = artist;
            if spots > 0 {
                sub = format!("{}{}{} {}", sub, if sub.is_empty() { "" } else { "  ·  " }, spots, if spots == 1 { "spot" } else { "spots" });
            }
            if s.mins >= 60 {
                // (lower case, as "spots" is: the line is mixed case and stays as written)
                sub = format!("{}  ·  {}", sub, mins_text(s.mins).to_lowercase());
            }
            if row(ui, s.id, &title, &sub) {
                self.loops_pick = Some(s.id);
            }
        }
    }

    /// The right pane: the song picked (its map, then each loop and bookmark), or the groups.
    fn loops_detail(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, data: &SpotData, pick: Option<i64>) {
        if self.loops_mode == 2 {
            self.groups_view(ui, acts);
            return;
        }
        let Some(c) = data.songs.iter().find(|s| Some(s.id) == pick) else { return };
        let playing = self.cur_track().map(|t| t.id) == Some(c.id);
        let (la, lb, now) = (self.loop_a, self.loop_b, self.pos());
        // ---- the song, and a button to play it
        let (title, artist) = match &c.v {
            Some(v) => (v.title.clone(), v.artist.clone()),
            None => ("A song (play it once to name it)".to_string(), String::new()),
        };
        let (hr, hresp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
        let play_r = Rect::from_center_size(Pos2::new(hr.max.x - 18.0, hr.center().y), Vec2::splat(30.0));
        {
            let p = ui.painter();
            let room = (hr.width() - 60.0).max(40.0);
            let clip = Rect::from_min_max(hr.min, Pos2::new(hr.min.x + room, hr.max.y)).intersect(p.clip_rect());
            let tw = ptext(&p.with_clip_rect(clip), Pos2::new(hr.min.x, hr.center().y), Align::Min, &title, 2.6, if playing { pal().bar_txt } else { pal().ink });
            if !artist.is_empty() && tw < room && room - tw > 70.0 {
                ptext_fit(p, Pos2::new(hr.min.x + tw + 12.0, hr.center().y + 2.0), Align::Min, &artist, 1.4, room - tw - 16.0, pal().ink2);
            }
            p.circle_filled(play_r.center(), 15.0, pal().bar_txt);
            let c0 = play_r.center();
            p.add(egui::Shape::convex_polygon(
                vec![c0 + Vec2::new(-4.0, -6.5), c0 + Vec2::new(7.0, 0.0), c0 + Vec2::new(-4.0, 6.5)],
                pal().app_bg,
                Stroke::NONE,
            ));
        }
        if let Some(v) = &c.v {
            let pr = ui.interact(play_r, ui.id().with(("spot_play", c.id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Play the song.");
            if pr.clicked() {
                acts.push(Action::PlaySaved(v.clone(), None));
            }
            let liked = self.liked.contains(&v.id);
            hresp.context_menu(|ui| {
                crate::song_items(ui, acts, &v.to_track(), liked, v.kind == "tidal");
            });
        }
        ui.add_space(8.0);
        // ---- the map: the song end to end, bookmarks named above their ticks, loops as bands, time below
        let len = c
            .v
            .as_ref()
            .map(|v| v.dur)
            .filter(|d| *d > 1.0)
            .or_else(|| song_wave(c.id).map(|w| w.len() as f32 / 50.0).filter(|d| *d > 1.0))
            .unwrap_or_else(|| {
                let lmax = c.loops.iter().map(|l| l.1.b).fold(0.0f32, f32::max);
                let mmax = c.marks.iter().map(|m| m.1.t).fold(0.0f32, f32::max);
                lmax.max(mmax) * 1.08 + 5.0
            });
        let (all, sresp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 16.0 + 60.0 + 16.0), Sense::click());
        let sr = Rect::from_min_size(Pos2::new(all.min.x, all.min.y + 16.0), Vec2::new(all.width(), 60.0));
        let at = |t: f32| sr.min.x + (t / len).clamp(0.0, 1.0) * sr.width();
        {
            let p = ui.painter();
            p.rect_filled(sr, 5.0, pal().row_alt);
            if let Some(w) = song_wave(c.id) {
                let n = sr.width().max(1.0) as usize;
                let wl = (w.len() as f32 / 50.0).max(0.1);
                let mut x = sr.min.x;
                for k in (0..n).step_by(2) {
                    let (t0, t1) = (k as f32 / n as f32 * len, (k + 2) as f32 / n as f32 * len);
                    let (i0, i1) = ((t0 / wl * w.len() as f32) as usize, (t1 / wl * w.len() as f32) as usize);
                    // the slice's average, not its loudest peak, and spread out: a loud mastered song still shows its shape
                    let slice = w.get(i0.min(w.len())..i1.max(i0 + 1).min(w.len())).unwrap_or(&[]);
                    let avg = slice.iter().map(|&x| x as f32).sum::<f32>() / slice.len().max(1) as f32 / 255.0;
                    let h = avg.powi(4) * (sr.height() / 2.0 - 3.0) + 1.0;
                    p.line_segment(
                        [Pos2::new(x, sr.center().y - h), Pos2::new(x, sr.center().y + h)],
                        Stroke::new(1.0_f32, pal().ink2.gamma_multiply(0.45)),
                    );
                    x += 2.0;
                }
            }
            for (_, s, _) in &c.loops {
                let band = Rect::from_min_max(Pos2::new(at(s.a), sr.min.y), Pos2::new(at(s.b).max(at(s.a) + 3.0), sr.max.y));
                let on = playing && la.is_some_and(|a| (a - s.a).abs() < 0.05) && lb.is_some_and(|b| (b - s.b).abs() < 0.05);
                p.rect_filled(band, 2.0, pal().bar_txt.gamma_multiply(if on { 0.6 } else { 0.32 }));
            }
            // bookmark names above their ticks, skipping one that would land on the last
            let mut last_end = f32::MIN;
            for (_, m) in &c.marks {
                let col = Color32::from_rgb(m.col[0], m.col[1], m.col[2]);
                let x = at(m.t);
                p.line_segment([Pos2::new(x, sr.min.y), Pos2::new(x, sr.max.y)], Stroke::new(2.0_f32, col));
                if !m.name.is_empty() && x - 2.0 > last_end {
                    let w = ptext(p, Pos2::new(x - 2.0, all.min.y + 7.0), Align::Min, &m.name, 1.0, col);
                    last_end = x + w + 6.0;
                }
            }
            if playing {
                let x = at(now);
                p.line_segment([Pos2::new(x, sr.min.y - 2.0), Pos2::new(x, sr.max.y + 2.0)], Stroke::new(1.5_f32, pal().ink));
            }
            for k in 0..4 {
                let t = len * k as f32 / 3.0;
                let al = match k {
                    0 => Align::Min,
                    3 => Align::Max,
                    _ => Align::Center,
                };
                ptext(p, Pos2::new(sr.min.x + sr.width() * k as f32 / 3.0, sr.max.y + 9.0), al, &fmt_time(t), 1.0, pal().dim);
            }
        }
        if let Some(v) = &c.v {
            let sresp = sresp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if let Some(pp) = sresp.hover_pos().filter(|pp| sr.contains(*pp)) {
                let t = ((pp.x - sr.min.x) / sr.width()).clamp(0.0, 1.0) * len;
                egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), ui.id().with(("strip_tip", c.id)), |ui| {
                    ui.label(format!("Play from {}", fmt_time(t)));
                });
                if sresp.clicked() {
                    acts.push(Action::PlayAt(v.clone(), t));
                }
            }
        }
        ui.add_space(10.0);
        // ---- each loop and bookmark: starred loops first, the rest in song order
        if c.loops.is_empty() && c.marks.is_empty() {
            para(ui, "Practiced, nothing saved yet. Set A and B in PRACTICE and press SAVE LOOP, or press + MARK.", pal().ink2);
            return;
        }
        enum Row<'a> {
            Loop(usize, &'a store::Section, u32),
            Mark(usize, &'a store::Bookmark),
        }
        let mut rows: Vec<(bool, f32, Row)> = c.loops.iter().map(|(i, s, w)| (s.fav, s.a, Row::Loop(*i, s, *w))).collect();
        rows.extend(c.marks.iter().map(|(i, m)| (false, m.t, Row::Mark(*i, m))));
        rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)));
        let mut drop_mark = None;
        for (_, _, row) in &rows {
            match row {
                Row::Loop(i, s, w) => {
                    let on = playing && la.is_some_and(|a| (a - s.a).abs() < 0.05) && lb.is_some_and(|b| (b - s.b).abs() < 0.05);
                    let h = if s.note.is_empty() { 32.0 } else { 50.0 };
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::click());
                    let p = ui.painter();
                    if on {
                        p.rect_filled(r, 6.0, pal().row_sel);
                    } else if resp.hovered() {
                        p.rect_filled(r, 6.0, pal().sel);
                    }
                    let cy = r.min.y + 16.0;
                    p.rect_filled(Rect::from_center_size(Pos2::new(r.min.x + 18.0, cy), Vec2::new(14.0, 8.0)), 2.0, pal().bar_txt.gamma_multiply(0.65));
                    let mut meta = format!("{}-{}", fmt_time(s.a), fmt_time(s.b));
                    if *w >= 60 {
                        meta = format!("{}   {}", meta, mins_text(*w));
                    }
                    if !s.group.is_empty() {
                        meta = format!("{}   {}", s.group.to_uppercase(), meta);
                    }
                    let mw = ptext(p, Pos2::new(r.max.x - 40.0, cy), Align::Max, &meta, 1.0, pal().ink2);
                    ptext_fit(p, Pos2::new(r.min.x + 36.0, cy), Align::Min, &s.name, 1.6, (r.width() - mw - 90.0).max(20.0), if on { pal().bar_txt } else { pal().ink });
                    if !s.note.is_empty() {
                        ptext_fit(p, Pos2::new(r.min.x + 36.0, r.min.y + 35.0), Align::Min, &s.note, 1.3, r.width() - 80.0, pal().dim);
                    }
                    let star_r = Rect::from_center_size(Pos2::new(r.max.x - 18.0, cy), Vec2::splat(24.0));
                    let sh = ui
                        .interact(star_r, ui.id().with(("loop_star", c.id, *i)), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .tip(if s.fav { "Starred. Click to unstar." } else { "Star it as one that matters, like a part for a gig coming up." });
                    let star_col = if s.fav {
                        STAR_COL
                    } else if sh.hovered() || resp.hovered() {
                        pal().dim
                    } else {
                        pal().dim.gamma_multiply(0.35)
                    };
                    crate::icon_in(ui.painter(), star_r.shrink(6.0), &crate::IC_STAR, star_col);
                    if sh.clicked() {
                        if let Some(x) = self.store.sections.get_mut(&c.id).and_then(|l| l.get_mut(*i)) {
                            x.fav = !x.fav;
                            self.store_dirty = true;
                        }
                    }
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Plays it on a loop. Right-click to rename it or add a note.");
                    if resp.clicked() && !sh.clicked() {
                        if let Some(v) = &c.v {
                            acts.push(Action::PlaySaved(v.clone(), Some((s.a, s.b))));
                        }
                    }
                    resp.context_menu(|ui| {
                        if menu_item(ui, "Rename / add a note...") {
                            self.clip_edit = Some((c.id, *i));
                            self.clip_name = s.name.clone();
                            self.clip_note = s.note.clone();
                            self.clip_group = s.group.clone();
                            ui.close_menu();
                        }
                        if let Some(v) = &c.v {
                            if menu_item(ui, "Add this loop to a playlist...") {
                                acts.push(Action::PlaylistPickLoop(v.to_track(), s.a, s.b, s.name.clone()));
                                ui.close_menu();
                            }
                        }
                        if menu_item(ui, "Forget this loop") {
                            acts.push(Action::ForgetLoop(c.id, *i));
                            ui.close_menu();
                        }
                    });
                    if self.clip_edit == Some((c.id, *i)) {
                        self.loop_editor(ui, acts);
                    }
                }
                Row::Mark(i, m) => {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), Sense::click());
                    let p = ui.painter();
                    if resp.hovered() {
                        p.rect_filled(r, 6.0, pal().sel);
                    }
                    let cy = r.center().y;
                    let col = Color32::from_rgb(m.col[0], m.col[1], m.col[2]);
                    p.rect_filled(Rect::from_center_size(Pos2::new(r.min.x + 18.0, cy), Vec2::new(3.0, 14.0)), 1.0, col);
                    let mw = ptext(p, Pos2::new(r.max.x - 40.0, cy), Align::Max, &fmt_time(m.t), 1.0, pal().ink2);
                    let name = if m.name.is_empty() { "Bookmark" } else { &m.name };
                    ptext_fit(p, Pos2::new(r.min.x + 36.0, cy), Align::Min, name, 1.6, (r.width() - mw - 90.0).max(20.0), if m.name.is_empty() { pal().ink2 } else { pal().ink });
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Plays the song from here. Right-click to remove it.");
                    if resp.clicked() {
                        if let Some(v) = &c.v {
                            acts.push(Action::PlayAt(v.clone(), m.t));
                        }
                    }
                    resp.context_menu(|ui| {
                        if menu_item(ui, "Remove this bookmark") {
                            drop_mark = Some(*i);
                            ui.close_menu();
                        }
                    });
                }
            }
        }
        if let Some(k) = drop_mark {
            if let Some(l) = self.store.marks.get_mut(&c.id) {
                if k < l.len() {
                    l.remove(k);
                    self.store_dirty = true;
                }
            }
        }
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
        // MY PLAN only when there is a plan file of your own
        let mine = crate::plan::have_plan();
        let tabs: &[&str] = if mine { &["DIARY", "ROUTINE", "LISTENING", "MY PLAN"] } else { &["DIARY", "ROUTINE", "LISTENING"] };
        if self.diary_tab as usize >= tabs.len() {
            self.diary_tab = 0;
        }
        if let Some(i) = tab_row(ui, tabs, self.diary_tab as usize) {
            self.diary_tab = i as u8;
        }
        ui.add_space(6.0);
        match self.diary_tab {
            3 => return self.plan_view(ui, acts),
            1 => return self.routine_view(ui, acts),
            2 => return self.listening_view(ui, acts),
            _ => {}
        }
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
        let practised = (0..7).filter(|k| day_secs(self, today - k) > 0).count();
        // three tiles, roomy; the days practised and all time under the pointer
        let tiles = [
            ("TODAY", mins_text(day_secs(self, today))),
            ("7 DAYS", mins_text(week)),
            ("THIS MONTH", mins_text(month)),
        ];
        let w = ui.available_width();
        let tw = ((w - 2.0 * 6.0) / 3.0).floor().max(60.0);
        let (sr, sresp) = ui.allocate_exact_size(Vec2::new(w, 46.0), Sense::hover());
        sresp.on_hover_text(format!(
            "Practised on {} of the last 7 days. All time: {}.",
            practised,
            mins_text(total).to_lowercase()
        ));
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
            if retro_btn_w(ui, "<", 34.0, false).tip("Previous month.").clicked() {
                self.diary_ym = if m == 1 { (y - 1, 12) } else { (y, m - 1) };
            }
            let (r, _) = ui.allocate_exact_size(Vec2::new(190.0, bh()), Sense::hover());
            ptext(ui.painter(), r.center(), Align::Center, &format!("{} {}", MONTHS[(m as usize - 1) % 12], y), 2.0, pal().ink);
            if retro_btn_w(ui, ">", 34.0, false).tip("Next month.").clicked() {
                self.diary_ym = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
            }
            if retro_btn(ui, "TODAY", false).tip("Jump to today.").clicked() {
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
        // the day's times changed, or added by hand
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "EDIT TIMES", self.diary_edit).tip("Change this day's times, or add practice the app didn't see (guitar away from the computer).").clicked() {
                self.diary_edit = !self.diary_edit;
            }
        });
        if self.diary_edit {
            let mut change: Option<(String, i64)> = None;
            for (name, s) in &rows_d {
                ui.horizontal(|ui| {
                    if retro_btn(ui, "X", false).tip("Take this off the day.").clicked() {
                        change = Some((name.clone(), -(*s as i64)));
                    }
                    if retro_btn(ui, "-5", false).clicked() {
                        change = Some((name.clone(), -300));
                    }
                    if retro_btn(ui, "+5", false).clicked() {
                        change = Some((name.clone(), 300));
                    }
                    dim_line(ui, &format!("{}  {}", name, mins_text(*s)), 2.0, pal().ink);
                });
            }
            let mut add = false;
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 150.0).max(120.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                add |= field(ui, &mut self.ed, crate::tools::F_DIARY_ADD_NAME, &mut self.diary_add.0, r, "what you practised", false).enter;
                let (r, _) = ui.allocate_exact_size(Vec2::new(60.0, bh()), Sense::hover());
                add |= field(ui, &mut self.ed, crate::tools::F_DIARY_ADD_MIN, &mut self.diary_add.1, r, "min", false).enter;
                add |= retro_btn(ui, "ADD", false).tip("Add this many minutes to the day.").clicked();
            });
            if add {
                let name = self.diary_add.0.trim().to_string();
                if let (false, Ok(m)) = (name.is_empty(), self.diary_add.1.trim().parse::<u32>()) {
                    change = Some((name, m as i64 * 60));
                    self.diary_add = (String::new(), String::new());
                }
            }
            if let Some((name, by)) = change {
                let d = self.store.day_mut(&date);
                let v = (*d.secs.get(&name).unwrap_or(&0) as i64 + by).max(0) as u32;
                if v == 0 {
                    d.secs.remove(&name);
                } else {
                    d.secs.insert(name, v);
                }
                self.store_dirty = true;
            }
        }
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
        // time spent looping (how many times round says little), and focus blocks when there were some
        let looped: u32 = self.store.days.iter().find(|d| d.date == date).map_or(0, |d| d.loop_time.values().map(|v| v.0).sum());
        let _ = loops;
        let mut bits = Vec::new();
        if looped > 0 {
            bits.push(format!("TIME LOOPED {}", mins_text(looped)));
        }
        if pomos > 0 {
            bits.push(format!("FOCUS BLOCKS {}", pomos));
        }
        if !bits.is_empty() {
            dim_line(ui, &bits.join("   "), 2.0, pal().ink2);
        }
        // the day in one bar, in the summary's colours
        let span = self.diary_span;
        let period = Period::around(sel, span);
        let sum = Summary::of(self, &period, self.diary_reading);
        let segs = sum.segments_for(self, &[sel]);
        if !segs.is_empty() {
            ui.add_space(4.0);
            stacked_bar(ui, &segs, 12.0);
            legend(ui, &segs);
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
        // a song opened: the loops worked on that day and for how long, and PLAY if you want to hear it
        if let Some(id) = self.diary_song.as_ref().filter(|(d, _)| *d == date).map(|(_, id)| *id) {
            self.diary_song_view(ui, acts, &date, id, songs.iter().find(|s| s.0 == id).map_or(0, |s| s.1));
        } else if !songs.is_empty() {
            ui.add_space(4.0);
            dim_line(ui, "SONGS PRACTISED", 1.0, pal().dim);
            for (k, (id, s)) in songs.iter().enumerate() {
                let v = self.store.songs.get(id).cloned();
                let name = v.as_ref().map(|v| format!("{} - {}", v.artist, v.title)).unwrap_or_else(|| "A SONG".to_string());
                // its time, and how much of it was looping
                let looped: u32 = self.store.days.iter().find(|d| d.date == date).map_or(0, |d| {
                    d.loop_time
                        .iter()
                        .filter(|(k, _)| store::parse_loop_key(k).is_some_and(|x| x.0 == *id))
                        .map(|(_, v)| v.0)
                        .sum()
                });
                let r = list_row(
                    ui,
                    k,
                    None,
                    || name.clone(),
                    || {
                        if looped > 0 {
                            format!("{}  ({} LOOPED)", mins_text(*s), mins_text(looped))
                        } else {
                            mins_text(*s)
                        }
                    },
                    RowState::Normal,
                    false,
                    false,
                );
                if let Some(r) = r {
                    if r.tip("The loops you worked on, and for how long.").clicked() {
                        self.diary_song = Some((date.clone(), *id));
                    }
                }
                let _ = v;
            }
        }

        // ---- the week (or month, or year) around the day
        ui.add_space(8.0);
        self.diary_summary(ui, &period, &sum);

        let mins_hint = format!("{}", day_secs(self, sel) / 60);
        // ---- journal for that day
        ui.add_space(4.0);
        if self.show_form {
            let hint_t = self.practice_label();
            let _ = field_row(ui, &mut self.ed, F_TUNE, "TUNE", &mut self.f_tune, &hint_t, None);
            let _ = field_row(ui, &mut self.ed, F_MINS, "MINUTES", &mut self.f_mins, &mins_hint, None);
            let _ = field_row(ui, &mut self.ed, F_BPM, "BPM", &mut self.f_bpm, "The tempo you reached", None);
            let _ = field_row(ui, &mut self.ed, F_NOTE, "NOTE", &mut self.f_note, "What went well? What next?", Some(70.0));
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "SAVE ENTRY", false).clicked() {
                    acts.push(Action::SaveEntry);
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.show_form = false;
                    self.entry_edit = None;
                    self.ed.id = 0;
                }
            });
        } else if retro_btn(ui, &format!("LOG PRACTICE FOR {}", date), false)
            .tip("Write down what you worked on. It works for past days too.")
            .clicked()
        {
            self.show_form = true;
        }
        ui.add_space(6.0);
        let mut any = false;
        let mut pick: Option<(usize, bool)> = None;
        for (i, e) in self.store.entries.iter().enumerate().rev().filter(|(_, e)| e.date == date) {
            any = true;
            ui.horizontal(|ui| {
                if retro_btn(ui, "EDIT", self.entry_edit == Some(i)).tip("Change this entry.").clicked() {
                    pick = Some((i, false));
                }
                if retro_btn(ui, "X", false).tip("Delete this entry.").clicked() {
                    pick = Some((i, true));
                }
            });
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
        match pick {
            Some((i, true)) => {
                self.store.entries.remove(i);
                self.entry_edit = None;
                self.show_form = false;
                self.store_dirty = true;
            }
            Some((i, false)) => {
                let e = self.store.entries[i].clone();
                self.f_tune = e.tune;
                self.f_mins = if e.mins > 0 { e.mins.to_string() } else { String::new() };
                self.f_bpm = if e.bpm > 0 { e.bpm.to_string() } else { String::new() };
                self.f_note = e.note;
                self.entry_edit = Some(i);
                self.show_form = true;
            }
            None => {}
        }
        if !any && self.store.entries.is_empty() {
            para(ui, "Practice time adds up by itself while you work on a loop in PRACTICE. To write down what you worked on and the tempo you reached, pick a day above and press LOG PRACTICE.", pal().ink2);
        }
        ui.add_space(20.0);
    }

    // ------------------------------------------------------------- PRACTICE
    /// Transcribing tools: waveform, A-B loop, saved sections, slow-down and the extras behind MORE.
    pub(crate) fn practice_ui(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let inner = ui.max_rect();
        window_deco(ui, inner, "PRACTICE");
        // the focus timer and the metronome: a pill each in the title strip (click the dot to start or stop, the words
        // to open its panel)
        let sc = crate::font::ui_scale();
        let (tw, mw, h) = (170.0 * sc, 120.0 * sc, 24.0);
        let mr = crate::title_slot(inner, crate::title_right(inner), mw, h);
        let tr = crate::title_slot(inner, mr.min.x - 8.0, tw, h);
        self.timer_pill(ui, tr, acts);
        self.metro_pill(ui, mr, acts);
        crate::tour::mark("TIMEBTNS", tr.union(mr));
        // the panel's contents scroll when the panel is short, rather than running past its edge
        // (no drag-to-scroll: dragging on the waveform sets loops)
        // everything inside grows and shrinks with the panel, gently (the square root of the size change), from a
        // little smaller to about twice as big
        // the page: PAGE_PAD in from the frame all round, so nothing runs into it
        let body = crate::page_rect(inner);
        let size = body.size();
        let s = (size.y / 250.0).min(size.x / 760.0).max(0.1).sqrt().clamp(0.85, 2.0);
        // the waveform's height is yours to drag (the handle under it)
        let wave_h = 54.0 * s * self.wv_h.clamp(0.6, 4.0);
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
                .tip("Drag to make the waveform taller or shorter. Double-click to reset it.");
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
            .tip("Click to jump. Shift + drag to select a loop, or inside a loop to slide it. Ctrl + drag A or B to move it. Alt + click to add or remove a bookmark. Right-click for more.");
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
                    .tip("Drag to look through the song. It keeps playing.");
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
                    "Adds a bookmark where the song is now (or press K, or alt + click the waveform). Right-click to set their colour. Click a flag to jump to it, or right-click it to recolour or remove it.",
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
                            "{}Bookmark {} at {}. Click to jump, alt + click to remove, right-click to rename or recolour.",
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
                    self.ed.anchor = None;
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
            if crate::chip(ui, chip_at(0.0, w1), "-", false, "wv_out").tip("Zoom out.").clicked() {
                zoom_by(self, 0.5);
            }
            if crate::chip(ui, chip_at(w1 + 3.0, w2), "+", false, "wv_in")
                .tip("Zoom in where it is playing, or use the mouse wheel on the waveform.")
                .clicked()
            {
                zoom_by(self, 2.0);
            }
            if self.wv_zoom > 1.0
                && crate::chip(ui, chip_at(w1 + w2 + 6.0, w3), "FIT", false, "wv_fit").tip("Show the whole song again.").clicked()
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
        // (short names, so the six fit on one row)
        if let Some(i) = tab_row(ui, &["LOOP", "TRAINER", "PITCH", "STEMS", "BOOKMARKS", "RECORD"], cur) {
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
        // three rows, each centred: A and B; the loop's buttons and moving about; the speed
        centered_row(ui, "ab_row", |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if retro_btn_w(ui, "A", 34.0, false).tip("Loop start here  ([ key)").clicked() && active {
                acts.push(Action::SetAAt(pos));
            }
            self.num_step(
                ui,
                acts,
                F_LOOP_A,
                78.0,
                Knob::LoopA,
                a_txt,
                "Nudge earlier or later, or click and type a time like 1:23.5.",
            );
            divider(ui);
            if retro_btn_w(ui, "B", 34.0, false).tip("Loop end here  (] key)").clicked() && active {
                acts.push(Action::SetBAt(pos));
            }
            self.num_step(
                ui,
                acts,
                F_LOOP_B,
                78.0,
                Knob::LoopB,
                b_txt,
                "Nudge earlier or later, or click and type a time like 2:05.",
            );
        });
        ui.add_space(6.0);
        centered_row(ui, "loop_btns", |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let ink = pal().ink;
            if crate::icon_btn_w(ui, &crate::IC_REW, false, ink, 34.0).tip("Back two seconds  (, key)").clicked() {
                acts.push(Action::SeekRel(-2.0));
            }
            if crate::icon_btn_w(ui, &crate::IC_PREV, false, ink, 34.0)
                .tip("Restart: back to the loop start (or the track start)  (. key)")
                .clicked()
            {
                acts.push(Action::Seek(self.loop_a.unwrap_or(0.0)));
            }
            divider(ui);
            if crate::icon_btn_w(ui, &crate::IC_REP, self.loop_on && both, ink, 34.0).tip("Loop on / off  (\\ key)").clicked() {
                acts.push(Action::LoopToggle);
            }
            if crate::icon_btn_w(ui, &crate::IC_X, false, ink, 34.0).tip("Clear the loop.").clicked() {
                acts.push(Action::LoopClear);
            }
            divider(ui);
            // the whole loop on to the next phrase, or back to the one before (same length)
            if crate::icon_btn_w(ui, &crate::IC_BACK, false, ink, 34.0).tip("Shift the loop back by its own length.").clicked()
                && both
            {
                acts.push(Action::ShiftLoop(false));
            }
            if crate::icon_btn_w(ui, &crate::IC_FWD, false, ink, 34.0)
                .tip("Shift the loop on by its own length, to the next phrase.")
                .clicked()
                && both
            {
                acts.push(Action::ShiftLoop(true));
            }
        });
        row_mark(ui, "LOOPROW", ab_top);
        ui.add_space(6.0);

        // ---- the speed
        let speed_top = ui.cursor().min.y;
        centered_row(ui, "speed_row", |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            label(ui, "SPEED", 0.0);
            // the usual speeds in a drop-down, and - / + for 1% at a time (click the number to type any, 25 to 250)
            let sp = retro_btn_w(ui, "v", 34.0, false).tip("Pick a speed.");
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
                "1% slower or faster, or click and type any speed from 25 to 250.",
            );
        });
        row_mark(ui, "SPEEDROW", speed_top);
        ui.add_space(14.0);

        // ---- saved loops for this track: which one is looping now (or that it isn't saved yet)
        if let (Some(a), Some(b), Some(id)) = (self.loop_a, self.loop_b, tid) {
            let saved = self
                .store
                .sections
                .get(&id)
                .and_then(|l| l.iter().find(|s| (s.a - a).abs() < 0.05 && (s.b - b).abs() < 0.05))
                .map(|s| s.name.to_uppercase());
            let what = match (&saved, self.loop_on) {
                (Some(n), true) => format!("LOOPING: {}", n),
                (Some(n), false) => format!("LOOP SET: {} (OFF)", n),
                (None, true) => "LOOPING, UNSAVED. NAME IT AND PRESS SAVE LOOP TO KEEP IT.".to_string(),
                (None, false) => "LOOP SET, OFF, UNSAVED".to_string(),
            };
            dim_line(ui, &what, 1.0, if saved.is_some() { pal().ink } else { pal().ink2 });
        }
        ui.horizontal_wrapped(|ui| {
            // the save button's real width; if the name box cannot keep a usable size, the button drops to the next line
            let bw = 10.0 + 18.0 + 8.0 + text_w("SAVE LOOP", 2.0) + 10.0;
            let w = (ui.available_width() - bw - 12.0).max(150.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_SEC, &mut self.sec_name, rect, "Name this loop (bridge, head, lick)", false);
            if o.enter {
                acts.push(Action::SaveSection);
            }
            if crate::ibtn(ui, &crate::IC_BOOK, "SAVE LOOP", false).tip("Keep this A-B loop with the track.").clicked() {
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
                        format!("{} to {}. Right-click to rename, add a note, group or delete.", fmt_t(s.a), fmt_t(s.b))
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
                            self.ed.anchor = None;
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
            "funky riff, the lick at 2:10",
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
            "solo licks, head, comping (empty: none)",
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
                if retro_btn(ui, &crate::font::as_label(&g), *g == self.clip_group).tip("Put it in this group.").clicked() {
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
        centered_row(ui, "marks_add", |ui| {
            if retro_btn(ui, &format!("+ BOOKMARK AT {}", fmt_t(now)), false)
                .tip(format!(
                    "Adds a bookmark where the song is now (or press {}, or alt + click the waveform).",
                    self.key_text(crate::prefs::Cmd::Bookmark)
                ))
                .clicked()
            {
                self.add_mark(id, now);
            }
            let n = self.store.marks.get(&id).map_or(0, |l| l.len());
            if n > 0 && retro_btn(ui, "REMOVE ALL", false).tip("Take every bookmark off this song.").clicked() {
                self.store.marks.remove(&id);
                self.store_dirty = true;
            }
        });
        ui.add_space(4.0);
        let n = self.store.marks.get(&id).map_or(0, |l| l.len());
        if n == 0 {
            para(ui, "No bookmarks on this song yet. They mark places to find your way: the solo, the key change, the bar you keep missing.", pal().ink2);
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
                sr.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Right-click for another colour.").context_menu(|ui| {
                    if let Some(c) = mark_swatches(ui, Some(m.col)) {
                        recolour = Some((k, c));
                        ui.close_menu();
                    }
                });
                if retro_btn_w(ui, &fmt_t(m.t), 70.0, false).tip("Jump here.").clicked() {
                    acts.push(Action::Seek(m.t));
                }
                let w = (ui.available_width() - 40.0).clamp(80.0, 360.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                let fid = 5000 + k as u32;
                if let Some(list) = self.store.marks.get_mut(&id) {
                    let o = field(ui, &mut self.ed, fid, &mut list[k].name, r, "Name it (solo, bridge, key change)", false);
                    if o.changed {
                        self.store_dirty = true;
                    }
                }
                if retro_btn_w(ui, "X", 28.0, false).tip("Remove this bookmark.").clicked() {
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

    /// The practice panel's other tabs: TRAINER (0), PITCH (1), STEMS (2), BOOKMARKS (3).
    fn more_tools(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let tid = self.cur_track().map(|t| t.id);
        if self.mtab == 3 {
            self.bookmarks_tab(ui, acts);
            return;
        }
        if self.mtab == 4 {
            self.record_tab(ui, acts);
            return;
        }
        match self.mtab {
            0 => {
                // the count-off is its own thing: clicks before every pass of the loop, with or without the trainer
                centered_row(ui, "trainer_0", |ui| {
                    label(ui, "COUNT-OFF", 92.0);
                    let ci = if self.count_in == 0 { "OFF".to_string() } else { format!("{} BEATS", self.count_in) };
                    if retro_btn_w(ui, &ci, 92.0, self.count_in > 0)
                        .tip(
                            "Clicks before every pass of the loop, at the song's tempo. Off, 2 or 4 beats, with or without the trainer.",
                        )
                        .clicked()
                    {
                        acts.push(Action::CountIn);
                    }
                    // the song's tempo the count-off clicks at: found by itself, or set here if that is wrong
                    ui.add_space(8.0);
                    label(ui, "BPM", 36.0);
                    let s = if self.bpm > 0 { self.bpm.to_string() } else { "---".to_string() };
                    self.num_step(
                        ui,
                        acts,
                        crate::tools::F_PR_BPM,
                        46.0,
                        Knob::Bpm,
                        s,
                        "The song's tempo, found by itself. If it's wrong, set it or type it.",
                    );
                    if retro_btn_w(ui, "TAP", 52.0, false).tip("Tap along with the music to find the tempo.").clicked() {
                        acts.push(Action::TapTempo);
                    }
                });
                ui.add_space(8.0);
                centered_row(ui, "trainer_1", |ui| {
                    label(ui, "TRAIN", 56.0);
                    if retro_btn_w(ui, if self.trainer { "ON" } else { "OFF" }, 56.0, self.trainer)
                        .tip("Plays the loop a set number of times, then speeds up by a set percent.")
                        .clicked()
                    {
                        acts.push(Action::Trainer);
                    }
                    if self.trainer {
                        lcd_box(ui, &format!("PASS {}/{}", self.passes, self.trainer_n), 100.0, pal().ink);
                        if self.trainer_keys {
                            lcd_box(ui, &format!("KEY {}/12", self.keys_done + 1), 90.0, pal().ink);
                        }
                    }
                });
                centered_row(ui, "trainer_2", |ui| {
                    label(ui, "THEN", 60.0);
                    if retro_btn(ui, "FASTER", !self.trainer_keys).tip("After the passes, the speed goes up.").clicked() {
                        acts.push(Action::TrainerKeys(false));
                    }
                    if retro_btn(ui, "UP A 4TH - 12 KEYS", self.trainer_keys)
                        .tip("After the passes, the loop moves up a 4th. Learn a lick from the record in every key, by ear.")
                        .clicked()
                    {
                        acts.push(Action::TrainerKeys(true));
                    }
                });
                centered_row(ui, "trainer_3", |ui| {
                    label(ui, "EVERY", 60.0);
                    self.num_step(
                        ui,
                        acts,
                        F_TR_LOOPS,
                        44.0,
                        Knob::Loops,
                        self.trainer_n.to_string(),
                        "Passes to play before each step. Or type it.",
                    );
                    label(ui, "LOOPS", 56.0);
                    self.num_step(
                        ui,
                        acts,
                        F_TR_STEP,
                        52.0,
                        Knob::Step,
                        format!("+{}%", self.trainer_step),
                        "How much faster each step is. Or type it.",
                    );
                });
                dim_centered(ui, "STARTS AT THE SPEED SET ABOVE AND STOPS AT 100%");
                ui.add_space(8.0);
                centered_row(ui, "trainer_4", |ui| {
                    if retro_btn_w(ui, "FOCUS", 76.0, self.focus_mode)
                        .tip("Hides the lists, leaving just the player and these tools.")
                        .clicked()
                    {
                        acts.push(Action::ToggleFocus);
                    }
                });
            }
            1 => {
                centered_row(ui, "pitch_0", |ui| {
                                        label(ui, "PITCH", 56.0);
                                        if retro_btn_w(ui, "-", 28.0, false)
                                            .tip("Lower the pitch a semitone. The speed stays.")
                                            .clicked()
                                        {
                                            acts.push(Action::Transpose(-100));
                                        }
                                        if retro_btn_w(ui, "-10c", 44.0, false)
                                            .tip("Lower the pitch 10 cents, a tenth of a semitone. For records not tuned to A440.")
                                            .clicked()
                                        {
                                            acts.push(Action::Transpose(-10));
                                        }
                                        // the pitch, to read or to type: semitones (+0.25, -2) or cents (30c, -15c); Enter sets it
                                        if self.ed.id != crate::tools::F_PITCH {
                                            self.pitch_text = format!("{:+.2}", self.semis as f32 / 100.0);
                                        }
                                        let (pr, _) = ui.allocate_exact_size(Vec2::new(78.0, bh()), Sense::hover());
                                        let o = field(ui, &mut self.ed, crate::tools::F_PITCH, &mut self.pitch_text, pr, "+0.00", false);
                                        if o.enter {
                                            let s = self.pitch_text.trim().to_lowercase().replace(' ', "");
                                            let cents = match s.strip_suffix('c') {
                                                Some(c) => c.parse::<f32>().ok(),
                                                None => s.parse::<f32>().ok().map(|x| x * 100.0),
                                            };
                                            if let Some(c) = cents {
                                                let want = (c.round() as i32).clamp(-1200, 1200);
                                                acts.push(Action::Transpose(want - self.semis));
                                            }
                                            self.ed.id = 0;
                                        }
                                        if retro_btn_w(ui, "+10c", 44.0, false)
                                            .tip("Raise the pitch 10 cents.")
                                            .clicked()
                                        {
                                            acts.push(Action::Transpose(10));
                                        }
                                        if retro_btn_w(ui, "+", 28.0, false).tip("Raise the pitch a semitone. The speed stays.").clicked() {
                                            acts.push(Action::Transpose(100));
                                        }
                                    });
                // the tuning, the ear mode and saving it: a second row
                centered_row(ui, "pitch_1", |ui| {
                                        let cur_id = self.cur_track().map(|t| t.id);
                                        match self.tuning {
                                            Some((id, c, _)) if Some(id) == cur_id && c.abs() > 4 => {
                                                if retro_btn_w(ui, &format!("TUNE TO A440 ({:+}c)", -c), 150.0, self.semis == -c)
                                                    .tip("Shifts the track to A440, so you can play along without retuning.")
                                                    .clicked()
                                                {
                                                    acts.push(Action::Transpose(-c - self.semis));
                                                }
                                            }
                                            _ => {
                                                if retro_btn_w(ui, if self.tuning_busy { "LISTENING..." } else { "FIND TUNING" }, 116.0, false)
                                                    .tip("Measures how far this recording is from A440 and offers to fix it. Old records and some modern tracks are off.")
                                                    .clicked()
                                                {
                                                    acts.push(Action::FindTuning);
                                                }
                                            }
                                        }
                                        if self.semis != 0 && retro_btn_w(ui, "0", 28.0, false).tip("Back to the original pitch.").clicked() {
                                            acts.push(Action::Transpose(-self.semis));
                                        }
                                        ui.add_space(8.0);
                                        if retro_btn_w(ui, CHAN_NAMES[self.chan as usize % 6], 100.0, self.chan != 0)
                                            .tip("Ear mode. LEFT or RIGHT CHANNEL plays just that side in both ears. Old jazz records often put the bass or piano on one side, while modern mixes barely differ. NO CENTER removes vocals, and BASS ONLY keeps the low end.")
                                            .clicked()
                                        {
                                            acts.push(Action::Chan);
                                        }
                                        if retro_btn_w(ui, if self.exporting { "SAVING..." } else { "EXPORT WAV" }, 100.0, false)
                                            .tip("Saves the song as you hear it to a WAV file, with stems, speed, pitch, ear mode and equalizer. It saves the A-B loop if there is one, or else the whole song.")
                                            .clicked()
                                        {
                                            acts.push(Action::Export);
                                        }
                                    });
            }
            _ => {
                let mut stem_note: Option<String> = None;
                centered_row(ui, "stems_0", |ui| {
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
                        .tip("Splits the track into drums, bass, vocals and other. Not during the tour.")
                        .clicked()
                    {
                        self.set_note("NOT DURING THE TOUR");
                    }
                } else if !stems::tool_ready() {
                    if retro_btn_w(ui, "GET STEMS TOOL (170 MB)", 230.0, false)
                        .tip("A one-time download of the tool that splits drums, bass, vocals and the rest. It runs on this PC.")
                        .clicked()
                    {
                        acts.push(Action::StemGet);
                    }
                } else if tid.map_or(false, stems::have_stems) {
                    for (i, name) in stems::NAMES.iter().enumerate() {
                        let r = stem_btn(ui, i, self.stem_on[i])
                            .tip(format!("{}. Click to turn it on or off, right-click to hear only this one.", name));
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
                        stem_note = Some(format!("THIS SONG'S STEMS: {}   ALL STEMS: {}", cache::fmt_size(this), cache::fmt_size(all)));
                    }
                } else {
                    let len = self.track_len();
                    if retro_btn_w(ui, "SPLIT THIS TRACK", 190.0, false)
                        .tip("Splits the track into drums, bass, vocals and other, on this PC. Kept afterwards.")
                        .clicked()
                    {
                        acts.push(Action::StemSplit);
                    }
                    // what it will cost before you press it
                    let t = stems::time_guess(len);
                    let time = if t < 90 { format!("{} SEC", t.div_ceil(5) * 5) } else { format!("{} MIN", t.div_ceil(60)) };
                    stem_note = Some(format!("ABOUT {} AND {} ON DISK", time, cache::fmt_size(stems::size_guess(len))));
                }
                });
                if let Some(t) = stem_note {
                    dim_centered(ui, &t);
                }
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

// ------------------------------------------------------------ the diary's summaries

/// A starred loop's star.
const STAR_COL: Color32 = Color32::from_rgb(232, 190, 70);

/// The colours of the top five songs in a summary (the rest is OTHER, in the dim ink), and of reading.
const SONG_COLS: [Color32; 5] = [
    Color32::from_rgb(217, 164, 65),
    Color32::from_rgb(95, 168, 211),
    Color32::from_rgb(214, 104, 83),
    Color32::from_rgb(111, 191, 115),
    Color32::from_rgb(167, 125, 194),
];
const READING_COL: Color32 = Color32::from_rgb(80, 190, 180);

/// A stretch of days for a summary: its name and its columns (a day each, or a month each for a year).
struct Period {
    name: String,
    cols: Vec<(String, Vec<i64>)>,
}

impl Period {
    /// The week (Monday to Sunday), month or year around a day.
    fn around(day: i64, span: u8) -> Period {
        match span {
            0 => {
                let mon = day - (day + 3).rem_euclid(7);
                const D: [&str; 7] = ["M", "T", "W", "T", "F", "S", "S"];
                Period {
                    name: format!("WEEK OF {}", store::date_str(mon)),
                    cols: (0..7).map(|k| (D[k as usize].to_string(), vec![mon + k])).collect(),
                }
            }
            1 => {
                let (y, m, _) = store::civil_from_days(day);
                let first = store::days_from_civil(y, m, 1);
                let next = if m == 12 { store::days_from_civil(y + 1, 1, 1) } else { store::days_from_civil(y, m + 1, 1) };
                Period {
                    name: format!("{:04}-{:02}", y, m),
                    cols: (first..next)
                        .map(|d| {
                            let n = d - first + 1;
                            (if n == 1 || n % 5 == 0 { n.to_string() } else { String::new() }, vec![d])
                        })
                        .collect(),
                }
            }
            _ => {
                let (y, _, _) = store::civil_from_days(day);
                const M: [&str; 12] = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"];
                Period {
                    name: format!("{}", y),
                    cols: (1..=12u32)
                        .map(|m| {
                            let first = store::days_from_civil(y, m, 1);
                            let next =
                                if m == 12 { store::days_from_civil(y + 1, 1, 1) } else { store::days_from_civil(y, m + 1, 1) };
                            (M[m as usize - 1].to_string(), (first..next).collect())
                        })
                        .collect(),
                }
            }
        }
    }
}

/// One column of a summary: its label, its days, its time.
struct Col {
    label: String,
    days: Vec<i64>,
    total: u32,
}

/// What a stretch of days adds up to.
struct Summary {
    cols: Vec<Col>,
    total: u32,
    tunes: u32,
    free: u32,
    reading: u32,
    /// the songs, most time first
    songs: Vec<(i64, u32)>,
    /// the loops (song, start, end, seconds, passes), most time first
    loops: Vec<(i64, f32, f32, u32, u32)>,
    days_practised: usize,
    days_so_far: usize,
    with_reading: bool,
}

impl Summary {
    fn of(app: &App, period: &Period, with_reading: bool) -> Summary {
        let today = store::today_days();
        let day_of = |d: i64| app.store.days.iter().find(|x| x.date == store::date_str(d));
        let read_secs = |d: &store::Day| d.secs.get("Sight reading").copied().unwrap_or(0);
        let day_total = |d: &store::Day| {
            let all: u32 = d.secs.values().sum();
            if with_reading {
                all
            } else {
                all - read_secs(d)
            }
        };
        let (mut tunes, mut free, mut reading) = (0, 0, 0);
        let mut songs: std::collections::HashMap<i64, u32> = Default::default();
        let mut loops: std::collections::HashMap<String, (u32, u32)> = Default::default();
        let (mut practised, mut so_far) = (0, 0);
        let mut cols = Vec::new();
        for (label, days) in &period.cols {
            let mut total = 0;
            for &d in days {
                if d <= today {
                    so_far += 1;
                }
                let Some(day) = day_of(d) else { continue };
                let t = day_total(day);
                total += t;
                if t > 0 {
                    practised += 1;
                }
                for (k, s) in &day.secs {
                    match k.as_str() {
                        "Sight reading" => reading += s,
                        "Free practice" => free += s,
                        _ => tunes += s,
                    }
                }
                for (id, s) in &day.songs {
                    *songs.entry(*id).or_insert(0) += s;
                }
                for (k, (s, n)) in &day.loop_time {
                    let e = loops.entry(k.clone()).or_insert((0, 0));
                    e.0 += s;
                    e.1 += n;
                }
            }
            cols.push(Col { label: label.clone(), days: days.clone(), total });
        }
        if !with_reading {
            reading = 0;
        }
        let mut songs: Vec<(i64, u32)> = songs.into_iter().filter(|s| s.1 > 0).collect();
        // (ties in a fixed order, so the lists never swap places between frames)
        songs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut loops: Vec<(i64, f32, f32, u32, u32)> = loops
            .into_iter()
            .filter_map(|(k, (s, n))| store::parse_loop_key(&k).map(|(id, a, b)| (id, a, b, s, n)))
            .filter(|l| l.3 > 0)
            .collect();
        loops.sort_by(|a, b| b.3.cmp(&a.3).then(a.0.cmp(&b.0)).then(a.1.total_cmp(&b.1)).then(a.2.total_cmp(&b.2)));
        let total = cols.iter().map(|c| c.total).sum();
        Summary {
            cols,
            total,
            tunes,
            free,
            reading,
            songs,
            loops,
            days_practised: practised,
            days_so_far: so_far.max(1),
            with_reading,
        }
    }

    /// A song's colour here: one of five for the top songs, the dim ink for the rest.
    fn colour(&self, id: i64) -> Color32 {
        self.songs.iter().take(5).position(|s| s.0 == id).map_or(pal().dim, |i| SONG_COLS[i])
    }

    /// Some days as (name, seconds, colour) pieces: the top songs, OTHER (the rest, and time with no song), and
    /// reading.
    fn segments_for(&self, app: &App, days: &[i64]) -> Vec<(String, u32, Color32)> {
        let mut top: Vec<(String, u32, Color32)> = Vec::new();
        let (mut other, mut reading) = (0u32, 0u32);
        let mut per: std::collections::HashMap<i64, u32> = Default::default();
        for &d in days {
            let Some(day) = app.store.days.iter().find(|x| x.date == store::date_str(d)) else { continue };
            let read = day.secs.get("Sight reading").copied().unwrap_or(0);
            let practice: u32 = day.secs.values().sum::<u32>() - read;
            let with_song: u32 = day.songs.values().sum();
            other += practice.saturating_sub(with_song);
            if self.with_reading {
                reading += read;
            }
            for (id, s) in &day.songs {
                *per.entry(*id).or_insert(0) += s;
            }
        }
        for (i, (id, _)) in self.songs.iter().take(5).enumerate() {
            if let Some(s) = per.remove(id).filter(|s| *s > 0) {
                top.push((app.song_name(*id), s, SONG_COLS[i]));
            }
        }
        other += per.values().sum::<u32>();
        if other > 0 {
            top.push(("OTHER".to_string(), other, pal().dim));
        }
        if reading > 0 {
            top.push(("READING".to_string(), reading, READING_COL));
        }
        top
    }
}

/// The width there is to see from here (a list wider than the page never pushes these past its edge).
fn seen_w(ui: &egui::Ui) -> f32 {
    ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 4.0).max(60.0)
}

/// A bar the width of the page, split into its pieces by time.
fn stacked_bar(ui: &mut egui::Ui, segs: &[(String, u32, Color32)], h: f32) {
    let w = seen_w(ui);
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let total: u32 = segs.iter().map(|s| s.1).sum::<u32>().max(1);
    let mut x = r.min.x;
    for (_, s, c) in segs {
        let sw = *s as f32 / total as f32 * r.width();
        ui.painter().rect_filled(Rect::from_min_max(Pos2::new(x, r.min.y), Pos2::new(x + sw, r.max.y)), 0.0, *c);
        x += sw;
    }
}

/// The pieces of a bar named under it: a swatch, the name, the time.
fn legend(ui: &mut egui::Ui, segs: &[(String, u32, Color32)]) {
    // laid out by hand, a line at a time, so nothing runs past the page's edge
    let w = seen_w(ui);
    let items: Vec<(String, Color32)> =
        segs.iter().map(|(n, s, c)| (format!("{}  {}", clip_text(n, 1.0, 220.0), mins_text(*s)), *c)).collect();
    let mut lines: Vec<Vec<(String, Color32)>> = vec![Vec::new()];
    let mut x = 0.0;
    for (t, c) in items {
        let iw = 16.0 + text_w(&t, 1.0) + 14.0;
        if x + iw > w && !lines.last().is_some_and(|l| l.is_empty()) {
            lines.push(Vec::new());
            x = 0.0;
        }
        x += iw;
        if let Some(l) = lines.last_mut() {
            l.push((t, c));
        }
    }
    for line in lines {
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 18.0), Sense::hover());
        let p = ui.painter();
        let mut x = r.min.x;
        for (t, c) in line {
            p.rect_filled(Rect::from_center_size(Pos2::new(x + 5.0, r.center().y), Vec2::splat(10.0)), 1.0, c);
            ptext(p, Pos2::new(x + 16.0, r.center().y), Align::Min, &t, 1.0, pal().ink2);
            x += 16.0 + text_w(&t, 1.0) + 14.0;
        }
    }
}

/// The text cut to fit `max_w` (with ... at the end), at one size - not shrunk.
fn clip_text(text: &str, px: f32, max_w: f32) -> String {
    if text_w(text, px) <= max_w {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let t: String = chars.iter().collect::<String>().trim_end().to_string() + "...";
        if text_w(&t, px) <= max_w {
            return t;
        }
    }
    String::new()
}

/// A row with a name, a bar as long as its share of the longest, and the time on the right.
fn bar_row(ui: &mut egui::Ui, name: &str, secs: u32, most: u32, col: Color32, right: &str) {
    let w = seen_w(ui);
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::hover());
    let p = ui.painter();
    let name_w = (r.width() * 0.45).min(320.0);
    let right_w = text_w(right, 1.0) + 8.0;
    ptext(p, Pos2::new(r.min.x, r.center().y), Align::Min, &clip_text(name, 1.0, name_w - 8.0), 1.0, pal().ink);
    let bar =
        Rect::from_min_max(Pos2::new(r.min.x + name_w, r.center().y - 4.0), Pos2::new(r.max.x - right_w, r.center().y + 4.0));
    p.rect_filled(bar, 0.0, pal().dim.gamma_multiply(0.25));
    let f = secs as f32 / most.max(1) as f32;
    p.rect_filled(Rect::from_min_size(bar.min, Vec2::new(bar.width() * f, bar.height())), 0.0, col);
    ptext(p, Pos2::new(r.max.x, r.center().y), Align::Max, right, 1.0, pal().ink2);
}

#[cfg(test)]
mod summary_tests {
    use super::*;

    #[test]
    fn periods_cover_the_right_days() {
        let wed = store::parse_date("2026-10-14").unwrap();
        let week = Period::around(wed, 0);
        assert_eq!(week.cols.len(), 7);
        assert_eq!(store::date_str(week.cols[0].1[0]), "2026-10-12", "weeks start on Monday");
        assert_eq!(store::date_str(week.cols[6].1[0]), "2026-10-18");
        assert_eq!(Period::around(wed, 1).cols.len(), 31);
        let year = Period::around(wed, 2);
        assert_eq!(year.cols.len(), 12);
        assert_eq!(year.cols.iter().map(|c| c.1.len()).sum::<usize>(), 365);
    }
}

/// A thin upright line between groups of controls in a row.
/// A dim line of words centred across the page or panel.
pub(crate) fn dim_centered(ui: &mut egui::Ui, text: &str) {
    let text = if crate::winamp_ui::chrome_on() { crate::winamp_ui::caps_words(text) } else { text.to_string() };
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 7.0 * spx(1.5) + 8.0), Sense::hover());
    ptext_fit(ui.painter(), r.center(), Align::Center, &text, 1.5, r.width() - 12.0, pal().ink2);
}

/// A row of controls centred across the page or panel (its width is taken from the frame before). Too wide for the
/// room, it wraps from the left instead.
pub(crate) fn centered_row(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    let key = ui.id().with(("centered_row", id));
    let w: f32 = ui.ctx().data(|d| d.get_temp(key)).unwrap_or(0.0);
    let room = ui.available_width();
    if w > room {
        let r = ui.horizontal_wrapped(add).response.rect;
        // (it fits on one line again, with room to spare: centre it from the next frame; a row that only just fits
        // stays as it is, so it never flips back and forth)
        if r.height() <= bh() + 6.0 && r.width() < room - 8.0 {
            ui.ctx().data_mut(|d| d.insert_temp(key, r.width()));
            ui.ctx().request_repaint();
        }
        return;
    }
    ui.horizontal(|ui| {
        let pad = ((room - w) / 2.0).max(0.0).floor();
        ui.add_space(pad);
        let x0 = ui.cursor().min.x;
        add(ui);
        let used = ui.min_rect().max.x - x0;
        if (used - w).abs() > 0.5 {
            ui.ctx().data_mut(|d| d.insert_temp(key, used));
            ui.ctx().request_repaint();
        }
    });
}

fn divider(ui: &mut egui::Ui) {
    let h = crate::bh();
    let (r, _) = ui.allocate_exact_size(Vec2::new(9.0, h), Sense::hover());
    ui.painter().line_segment(
        [Pos2::new(r.center().x, r.min.y + 4.0), Pos2::new(r.center().x, r.max.y - 4.0)],
        egui::Stroke::new(1.0_f32, pal().beige_lt),
    );
}

thread_local! {
    /// The tune name looked up with Enter or ADD (its cards show while it is still in the box), and what was found.
    static TUNE_ASKED: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    static TUNE_FOUND: std::cell::RefCell<Vec<(u32, String, String)>> = const { std::cell::RefCell::new(Vec::new()) };
}
