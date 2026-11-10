//! The text box every page types into (`field`; one line or many, a caret and a selection, Ctrl+C/X/V/A, undo by
//! retyping), the same without its box (`field_bare`).

use super::*;

// -------------------------------------------------------------- text field
pub struct FieldOut {
    pub changed: bool,
    pub enter: bool,
}

/// Wrapped line ranges (in characters) of a multi-line text.
pub(crate) fn layout_lines(chars: &[char], px: f32, w: f32) -> Vec<(usize, usize)> {
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

pub(crate) fn caret_x(chars: &[char], s: usize, cur: usize, px: f32) -> f32 {
    if cur <= s {
        return 0.0;
    }
    let pre: String = chars[s..cur.min(chars.len())].iter().collect();
    text_w(&pre, px) + spx(px)
}

pub(crate) fn line_of(lines: &[(usize, usize)], cur: usize) -> usize {
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
    field_x(ui, ed, id, text, rect, hint, multi, false)
}

thread_local! {
    /// set while `field_bare` draws: the text and caret only, the caller's own box around them
    static FIELD_BARE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A one-line text box drawn without a box (the caller draws its own: the skins page's round search).
pub(crate) fn field_bare(ui: &mut egui::Ui, ed: &mut Ed, id: u32, text: &mut String, rect: Rect, hint: &str) -> FieldOut {
    FIELD_BARE.with(|b| b.set(true));
    let o = field(ui, ed, id, text, rect, hint, false);
    FIELD_BARE.with(|b| b.set(false));
    o
}

/// A text field; `fmt`: notes with formatting, its marks faint and the words they format shown so (see `notes_box`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn field_x(ui: &mut egui::Ui, ed: &mut Ed, id: u32, text: &mut String, rect: Rect, hint: &str, multi: bool, fmt: bool) -> FieldOut {
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
            if n > 0 && menu_item(ui, "Select All") {
                picked = Some((None, true));
            }
            if picked.is_some() || paste {
                ui.close_menu();
            }
        });
        if (picked.is_some() || paste)
            && ed.id != id {
                ed.id = id;
                ed.cur = n;
                ed.anchor = None;
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
    } else if focused && ui.input(|i| i.pointer.primary_pressed()) && !resp.hovered() && ed.hold != id {
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
    if !FIELD_BARE.with(|b| b.get()) {
        inset(p, rect, pal().display);
        if focused {
            outline(p, rect, 1.0, pal().ink2);
        }
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
    // formatting: the marks faint, links blue, bold drawn twice a hair apart, underlines (each run the line again,
    // seen only inside it, as the selection is)
    if fmt {
        let looks = crate::notes_fmt::looks(&chars);
        for (li, (s, e)) in lines.iter().enumerate() {
            let y = inner.min.y - voff + li as f32 * lh + lh / 2.0;
            if y < inner.min.y - lh || y > inner.max.y + lh {
                continue;
            }
            let line: String = chars[*s..(*e).min(chars.len())].iter().collect();
            let at = Pos2::new(inner.min.x - hoff, y);
            let mut a = *s;
            while a < *e {
                let lk = looks[a];
                let b = (a + 1..*e).find(|&k| looks[k] != lk).unwrap_or(*e);
                if lk != crate::notes_fmt::Look::default() && chars[a] != '\n' {
                    let x0 = at.x + caret_x(&chars, *s, a, px);
                    let x1 = at.x + caret_x(&chars, *s, b, px);
                    let band = Rect::from_min_max(Pos2::new(x0, y - lh / 2.0), Pos2::new(x1, y + lh / 2.0));
                    let bp = cp.with_clip_rect(band.intersect(inner));
                    if lk.mark || lk.link || lk.tag {
                        fill_rect(&bp, band, pal().display);
                        ptext(&bp, at, Align::Min, &line, px, if lk.mark { pal().dim } else { link_col() });
                    }
                    if lk.bold || lk.head {
                        ptext(&bp, at + Vec2::new(0.6, 0.0), Align::Min, &line, px, pal().ink);
                    }
                    if lk.under || lk.link {
                        let col = if lk.link { link_col() } else { pal().ink };
                        cp.hline(x0..=x1 - spx(px), y + 3.5 * spx(px) + 1.0, Stroke::new(1.0f32, col));
                    }
                }
                a = b;
            }
        }
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
            ptext(&cp.with_clip_rect(band.intersect(inner)), Pos2::new(inner.min.x - hoff, y), Align::Min, &line, px, pal().display);
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
