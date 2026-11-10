//! A notes box that reads formatted (headings, lists, bold, underline, links) and writes in the text box with its
//! marks faint (`notes_box`), the tool strip over it (`note_tools`). The marks themselves: notes_fmt.rs.

use super::*;

// -------------------------------------------------------------- notes with formatting
// (the marks and the tools are in notes_fmt.rs; here the box that shows them)

/// Links in notes: a blue that reads on the box's face, dark or light.
pub(crate) fn link_col() -> Color32 {
    let l = pal().lcd;
    let lum = 0.3 * l.r() as f32 + 0.59 * l.g() as f32 + 0.11 * l.b() as f32;
    if lum < 110.0 {
        Color32::from_rgb(120, 175, 255)
    } else {
        Color32::from_rgb(25, 90, 200)
    }
}

/// A word (and the spaces after it) of notes laid out to read: `at` its left end and the middle of its line, `w` its
/// width without the spaces, `raw` where it starts in the text (chars).
pub(crate) struct Piece {
    pub(crate) at: Pos2,
    pub(crate) text: String,
    pub(crate) px: f32,
    pub(crate) w: f32,
    pub(crate) look: crate::notes_fmt::Look,
    pub(crate) url: Option<String>,
    pub(crate) raw: usize,
    /// where the next word starts in the text (an underline runs on to it)
    pub(crate) cont: usize,
    /// one of your tunes named here (which, in the list `lay_notes` was given)
    pub(crate) tune: Option<usize>,
}

/// Notes laid out to read, the marks gone: the words, the list dots, the lines (heading rules and lines between
/// sections, as y with x from..to), and how tall (from the top, at 0).
#[derive(Default)]
pub(crate) struct Laid {
    pub(crate) pieces: Vec<Piece>,
    pub(crate) dots: Vec<Pos2>,
    pub(crate) rules: Vec<(f32, f32)>,
    pub(crate) h: f32,
}

/// Lays notes out `w` wide in the box's text size `px`, the tunes `tunes` linked where they're named.
pub(crate) fn lay_notes(text: &str, px: f32, w: f32, tunes: &[String]) -> Laid {
    use crate::notes_fmt::{block_of, mentions, spans, Block};
    crate::font::keep_caps(|| {
        let mut laid = Laid::default();
        let lh = 7.0 * spx(px) + 6.0;
        let space = (text_w("a a", px) - text_w("aa", px)).max(spx(px) * 2.0);
        let mut y = 0.0f32;
        let mut raw = 0usize;
        let lines: Vec<&str> = text.trim_end().split('\n').collect();
        for (li, line) in lines.iter().enumerate() {
            let chars: Vec<char> = line.chars().collect();
            let (block, from) = block_of(&chars);
            let named = mentions(&chars, tunes);
            match block {
                Block::Rule => {
                    laid.rules.push((y + lh / 2.0, 0.0));
                    y += lh;
                }
                _ => {
                    let head = block == Block::Heading;
                    let (tp, th) = if head { (px + 0.4, 7.0 * spx(px + 0.4) + 10.0) } else { (px, lh) };
                    if head && li > 0 {
                        y += 4.0;
                    }
                    let x0 = if block == Block::Item { 18.0 } else { 0.0 };
                    if block == Block::Item {
                        laid.dots.push(Pos2::new(8.0, y + th / 2.0));
                    }
                    // the words, each with the spaces after it, kept whole on a line where they fit
                    let mut x = x0;
                    for sp in spans(&chars, from).into_iter().filter(|s| !s.look.mark) {
                        let look = crate::notes_fmt::Look { head, ..sp.look };
                        let mut k = sp.s;
                        while k < sp.e {
                            let ws = (k..sp.e).find(|&j| chars[j] == ' ').unwrap_or(sp.e);
                            let we = (ws..sp.e).find(|&j| chars[j] != ' ').unwrap_or(sp.e);
                            let word: String = chars[k..ws].iter().collect();
                            // (a tag is a chip: room each side)
                            let ww = if word.is_empty() { 0.0 } else { text_w(&word, tp) + if look.tag { 2.0 * TAG_PAD } else { 0.0 } };
                            let full = ww + space * (we - ws) as f32;
                            if word.is_empty() && x == x0 && k > from {
                                k = we;
                                continue;
                            }
                            if x + ww > w && x > x0 && !word.is_empty() {
                                x = x0;
                                y += th;
                            }
                            if !word.is_empty() {
                                laid.pieces.push(Piece {
                                    at: Pos2::new(x, y + th / 2.0),
                                    text: word,
                                    px: tp,
                                    w: ww,
                                    look,
                                    url: sp.url.clone(),
                                    raw: raw + k,
                                    cont: raw + we,
                                    tune: named.iter().find(|m| k >= m.0 && k < m.1).map(|m| m.2),
                                });
                            }
                            x += full;
                            k = we;
                        }
                    }
                    if head {
                        laid.rules.push((y + th - 1.0, 1.0));
                    }
                    y += th;
                }
            }
            raw += chars.len() + 1;
        }
        laid.h = y;
        laid
    })
}

/// Draws laid-out notes with their top-left at `o`, `w` wide, faded to `alpha`; `hot` the word under the pointer (its
/// link, tag or tune shows it); `find` (lowercase) marked wherever it comes.
pub(crate) fn draw_notes(p: &egui::Painter, o: Pos2, w: f32, laid: &Laid, alpha: f32, hot: Option<usize>, find: &str) {
    let fade = |c: Color32| if alpha < 1.0 { c.gamma_multiply(alpha) } else { c };
    let hot = hot.and_then(|i| laid.pieces.get(i));
    crate::font::keep_caps(|| {
        for (i, pc) in laid.pieces.iter().enumerate() {
            let at = o + pc.at.to_vec2();
            let lh = 7.0 * spx(pc.px) + 4.0;
            let tx = at.x + if pc.look.tag { TAG_PAD } else { 0.0 };
            // the search, marked (a word at a time)
            let low = pc.text.to_lowercase();
            if !find.is_empty() && low.len() == pc.text.len() {
                let mut from = 0;
                while let Some(j) = low[from..].find(find) {
                    let a = from + j;
                    let b = a + find.len();
                    if !pc.text.is_char_boundary(a) || !pc.text.is_char_boundary(b) {
                        break;
                    }
                    let x0 = tx + if a > 0 { text_w(&pc.text[..a], pc.px) } else { 0.0 };
                    let x1 = tx + text_w(&pc.text[..b], pc.px);
                    p.rect_filled(Rect::from_min_max(Pos2::new(x0 - 1.0, at.y - lh / 2.0), Pos2::new(x1 + 1.0, at.y + lh / 2.0)), 2.0, fade(MARK));
                    from = b;
                }
            }
            let is_hot = hot.is_some_and(|h| {
                std::ptr::eq(h, pc) || (h.url.is_some() && h.url == pc.url) || (h.tune.is_some() && h.tune == pc.tune && h.raw.abs_diff(pc.raw) < 64)
            });
            if pc.look.tag {
                // a #tag: a chip in the links' colour
                let r = Rect::from_min_max(Pos2::new(at.x, at.y - lh / 2.0 + 1.0), Pos2::new(at.x + pc.w, at.y + lh / 2.0 - 1.0));
                p.rect_filled(r, r.height() / 2.0, fade(link_col().gamma_multiply(if is_hot { 0.34 } else { 0.18 })));
                ptext(p, Pos2::new(tx, at.y), Align::Min, &pc.text, pc.px, fade(link_col()));
                continue;
            }
            let col = fade(if pc.look.link { link_col() } else { pal().ink });
            ptext(p, at, Align::Min, &pc.text, pc.px, col);
            if pc.look.bold || pc.look.head {
                ptext(p, at + Vec2::new(0.6, 0.0), Align::Min, &pc.text, pc.px, col);
            }
            if pc.look.under || pc.look.link {
                let y = at.y + 3.5 * spx(pc.px) + 1.0;
                let thick = if is_hot { 2.0f32 } else { 1.0 };
                // (on under the space to the next word of the same link or underline, on the same line)
                let to = match laid.pieces.get(i + 1) {
                    Some(nx) if nx.raw == pc.cont && nx.look == pc.look && nx.url == pc.url && (nx.at.y - pc.at.y).abs() < 0.5 => o.x + nx.at.x,
                    _ => at.x + pc.w,
                };
                p.hline(at.x..=to, y, Stroke::new(thick, col));
            } else if pc.tune.is_some() {
                // one of your tunes: dotted under, solid while the pointer is on it
                let y = at.y + 3.5 * spx(pc.px) + 1.0;
                let to = match laid.pieces.get(i + 1) {
                    Some(nx) if nx.raw == pc.cont && nx.tune == pc.tune && (nx.at.y - pc.at.y).abs() < 0.5 => o.x + nx.at.x,
                    _ => at.x + pc.w,
                };
                if is_hot {
                    p.hline(at.x..=to, y, Stroke::new(1.0f32, col));
                } else {
                    let mut x = at.x;
                    while x < to {
                        p.hline(x..=(x + 2.0).min(to), y, Stroke::new(1.0f32, fade(pal().ink2)));
                        x += 4.0;
                    }
                }
            }
        }
    });
    for d in &laid.dots {
        p.circle_filled(o + d.to_vec2(), 2.3, fade(pal().ink2));
    }
    for (y, kind) in &laid.rules {
        let col = fade(if *kind > 0.0 { pal().edge } else { pal().ink2 });
        p.hline(o.x..=o.x + w, o.y + y, Stroke::new(1.0f32, col));
    }
}

/// The word of laid-out notes under `pos` (relative to the top-left), and where in the text the pointer is (chars).
pub(crate) fn note_hit(laid: &Laid, pos: Vec2, px: f32) -> Option<(usize, usize)> {
    let lh = 7.0 * spx(px) + 6.0;
    let (at, pc) = laid.pieces.iter().enumerate().find(|(_, pc)| {
        let h = if pc.px > px { 7.0 * spx(pc.px) + 10.0 } else { lh };
        (pos.y - pc.at.y).abs() <= h / 2.0 && pos.x >= pc.at.x - 2.0 && pos.x <= pc.at.x + pc.w + 2.0
    })?;
    let chars: Vec<char> = pc.text.chars().collect();
    let k = (0..=chars.len())
        .min_by(|a, b| {
            let x = |n: usize| crate::font::keep_caps(|| text_w(&chars[..n].iter().collect::<String>(), pc.px));
            (x(*a) - (pos.x - pc.at.x)).abs().total_cmp(&(x(*b) - (pos.x - pc.at.x)).abs())
        })
        .unwrap_or(0);
    Some((at, pc.raw + k))
}

/// How wide a formatting tool's button is.
pub(crate) const TOOL_W: f32 = 28.0;

/// Opens a link from notes in the web browser (web addresses only).
pub(crate) fn open_link(url: &str) {
    if url.starts_with("https://") || url.starts_with("http://") {
        let _ = webbrowser::open(url);
    }
}

/// The formatting tools for the notes box `id`, on the row they're put in: heading, bold, underline, list, link, a
/// line. They work where the caret is in the box (at the end when it isn't being written in), as do Ctrl+B, Ctrl+U
/// and Ctrl+K while it is. Returns whether the text changed.
pub(crate) fn note_tools(ui: &mut egui::Ui, ed: &mut Ed, id: u32, text: &mut String) -> bool {
    use crate::notes_fmt::{apply, Tool, TOOLS};
    let mut used: Option<Tool> = None;
    if ed.id == id {
        ui.input(|i| {
            for (key, tool) in [(egui::Key::B, Tool::Bold), (egui::Key::U, Tool::Underline), (egui::Key::K, Tool::Link)] {
                if i.modifiers.command && i.key_pressed(key) {
                    used = Some(tool);
                }
            }
        });
    }
    let mut over = false;
    // (one group, close together: it moves to the next row whole when the row is full)
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (tool, tip) in TOOLS {
            let label = match tool {
                Tool::Heading => "H",
                Tool::Bold => "B",
                Tool::Underline => "U",
                Tool::Tag => "#",
                _ => " ",
            };
            let r = retro_btn_w(ui, label, TOOL_W, false);
            let (rc, col) = (r.rect, pal().ink);
            let p = ui.painter();
            let c = rc.center();
            let s = Stroke::new(1.5f32, col);
            match tool {
                Tool::Underline => {
                    p.hline(c.x - 5.0..=c.x + 5.0, c.y + 8.0, s);
                }
                Tool::List => {
                    for k in [-5.0, 0.0, 5.0] {
                        p.circle_filled(Pos2::new(c.x - 6.0, c.y + k), 1.4, col);
                        p.hline(c.x - 2.0..=c.x + 7.0, c.y + k, s);
                    }
                }
                Tool::Link => {
                    // two links of a chain
                    let r1 = Rect::from_center_size(c + Vec2::new(-3.0, 2.0), Vec2::new(9.0, 5.0));
                    let r2 = Rect::from_center_size(c + Vec2::new(3.0, -2.0), Vec2::new(9.0, 5.0));
                    p.rect_stroke(r1, 2.5, s);
                    p.rect_stroke(r2, 2.5, s);
                }
                Tool::Line => {
                    p.hline(c.x - 7.0..=c.x + 7.0, c.y, s);
                }
                Tool::Heading | Tool::Bold | Tool::Tag => {}
            }
            over |= r.hovered();
            if r.tip(tip).clicked() {
                used = Some(tool);
            }
        }
    });
    // (a press on the tools keeps the box being written in, and what's picked in it)
    if over {
        ed.hold_next = id;
    }
    let Some(tool) = used else { return false };
    let n = text.chars().count();
    if ed.id != id {
        ed.id = id;
        ed.cur = n;
        ed.anchor = None;
    }
    let cur = ed.cur.min(n);
    let a = ed.anchor.map_or(cur, |a| a.min(n));
    let (new, (s, e)) = apply(text, (a.min(cur), a.max(cur)), tool);
    *text = new;
    ed.anchor = Some(s);
    ed.cur = e;
    true
}

/// A notes box with formatting (the diary's, a tune's): the words formatted to read; a click on them writes (the marks
/// faint while you do), a click on a link opens it, on a #tag or one of your tunes (`tunes`) asks for its days or its
/// page. As tall as what's written, `min_h` at least, `max_h` at most while writing. Returns what was done, where it
/// is, and what a click asks for.
#[allow(clippy::too_many_arguments)]
pub(crate) fn notes_box(
    ui: &mut egui::Ui,
    ed: &mut Ed,
    id: u32,
    text: &mut String,
    hint: &str,
    min_h: f32,
    max_h: f32,
    tunes: &[String],
) -> (FieldOut, Rect, Option<NoteClick>) {
    let px = if crate::winamp_ui::chrome_on() { 1.7 } else { 2.0 };
    let lh = 7.0 * spx(px) + 6.0;
    let w = ui.available_width();
    if ed.id == id || text.trim().is_empty() {
        let chars: Vec<char> = text.chars().collect();
        let lines = layout_lines(&chars, px, w - 16.0).len().max(1);
        let h = ((lines + 1) as f32 * lh + 6.0).clamp(min_h, max_h);
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
        return (field_x(ui, ed, id, text, r, hint, true, true), r, None);
    }
    let laid = lay_notes(text, px, w - 16.0, tunes);
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, (laid.h + 6.0 + lh / 2.0).max(min_h)), Sense::click());
    let o = r.min + Vec2::new(8.0, 3.0);
    let hit = resp.hover_pos().and_then(|m| note_hit(&laid, m - o, px));
    let pc = hit.map(|h| &laid.pieces[h.0]);
    let url = pc.and_then(|p| p.url.clone());
    let tag = pc.filter(|p| p.look.tag).map(|p| p.text.to_lowercase());
    let tune = pc.and_then(|p| p.tune);
    let p = ui.painter();
    inset(p, r, pal().lcd);
    draw_notes(&p.with_clip_rect(r.shrink(2.0)), o, w - 16.0, &laid, 1.0, hit.map(|h| h.0), "");
    let resp = if let Some(u) = &url {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(u)
    } else if let Some(t) = &tag {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(format!("Every day with {}.", t))
    } else if let Some(i) = tune {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(format!("Open {}.", tunes[i]))
    } else {
        resp.on_hover_cursor(egui::CursorIcon::Text)
    };
    let mut asked = None;
    if resp.clicked() {
        if let Some(u) = url {
            open_link(&u);
        } else if let Some(t) = tag {
            asked = Some(NoteClick::Tag(t));
        } else if let Some(i) = tune {
            asked = Some(NoteClick::Tune(i));
        } else {
            // writing, the caret where the click was
            ed.id = id;
            ed.cur = hit.map_or(text.chars().count(), |h| h.1);
            ed.anchor = Some(ed.cur);
        }
    }
    (FieldOut { changed: false, enter: false }, r, asked)
}

/// What a click on the words of notes asks for: a tag's days, or a tune's page.
pub(crate) enum NoteClick {
    Tag(String),
    Tune(usize),
}
