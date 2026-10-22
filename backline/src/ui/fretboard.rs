//! Backline's instrument pictures, shared by the desktop and the phone app: the guitar neck (to look at or to
//! click), the piano, chord boxes with their notes coloured by role, and the colours themselves (the root red,
//! thirds blue, fifths green, sixths and sevenths orange, the colour notes purple).

use crate::font::ptext;
use crate::skin::*;
use crate::theory;
use eframe::egui::{self, Align, Color32, Pos2, Rect, Sense, Stroke, Vec2};

thread_local! {
    /// the analyzer's chord has a seventh over this root (its sixth is then named 13)
    pub(crate) static SEVENTH: std::cell::Cell<Option<i32>> = const { std::cell::Cell::new(None) };
}

thread_local! {
    /// The chord the piano is drawn over (its notes from the root): with it, the keys are named as the chord
    /// spells them (#11, not b5).
    static OVER_CHORD: std::cell::RefCell<Option<Vec<i32>>> = const { std::cell::RefCell::new(None) };
}

/// Draw with the notes named over this chord (`pcs` from its root) - for the piano.
pub(crate) fn over_chord<R>(pcs: &[i32], f: impl FnOnce() -> R) -> R {
    OVER_CHORD.with(|c| *c.borrow_mut() = Some(pcs.to_vec()));
    let r = f();
    OVER_CHORD.with(|c| *c.borrow_mut() = None);
    r
}

/// A note's name over the chord being drawn (if one is set), else by its plain degree.
fn key_name(m: i32, root: i32) -> &'static str {
    match OVER_CHORD.with(|c| c.borrow().clone()) {
        Some(pcs) => {
            let abs: Vec<i32> = pcs.iter().map(|x| (root + x).rem_euclid(12)).collect();
            degree_in(m.rem_euclid(12), root, &abs).0
        }
        None => degree(m, root).0,
    }
}

/// A note's role in a chord with this root, and its colour: the root red, thirds blue, fifths green, sixths and
/// sevenths orange, the colour notes (9, 11) purple.
pub(crate) fn degree(pc: i32, root: i32) -> (&'static str, Color32) {
    const NAMES: [&str; 12] = ["R", "\u{266D}9", "9", "\u{266D}3", "3", "11", "\u{266D}5", "5", "#5", "6", "\u{266D}7", "7"];
    let s = (pc - root).rem_euclid(12) as usize;
    // over a seventh the sixth is the 13th (G13b9: its E is the 13)
    if s == 9 && SEVENTH.with(|c| c.get() == Some(root.rem_euclid(12))) {
        return ("13", Color32::from_rgb(255, 150, 40));
    }
    let col = match s {
        0 => Color32::from_rgb(235, 64, 64),
        3 | 4 => Color32::from_rgb(64, 140, 255),
        6..=8 => Color32::from_rgb(56, 196, 90),
        9..=11 => Color32::from_rgb(255, 150, 40),
        _ => Color32::from_rgb(178, 96, 255),
    };
    (NAMES[s], col)
}

/// A note's degree named the way the chord spells it: with a major third a raised fourth is #11 and a minor
/// third is #9; with a fifth a raised fifth is b13; with a seventh a sixth is 13; with no third a 2 or 4 is a
/// suspension.
pub(crate) fn degree_in(pc: i32, root: i32, pcs: &[i32]) -> (&'static str, Color32) {
    let has = |x: i32| pcs.iter().any(|p| (p - root).rem_euclid(12) == x);
    let (name, col) = degree(pc, root);
    let s = (pc - root).rem_euclid(12);
    let name = match s {
        6 if has(7) => "#11",
        3 if has(4) => "#9",
        8 if has(7) => "\u{266D}13",
        9 if has(10) || has(11) => "13",
        // a diminished 7th: its 7th is a double-flat 7th, not a 6th
        9 if has(3) && has(6) && !has(7) => "\u{266D}\u{266D}7",
        5 if !has(3) && !has(4) => "4",
        2 if !has(3) && !has(4) && !has(10) && !has(11) => "2",
        _ => name,
    };
    (name, col)
}

/// A two-octave piano from C with `lit` notes (MIDI) marked: in the degree colours, named by degree, when the
/// chord's `root` is known. When `click` is on, a click returns that key's pitch class.
pub(crate) fn piano(ui: &mut egui::Ui, rect: Rect, lit: &[i32], click: bool, id: &str, root: Option<i32>) -> Option<i32> {
    piano_span(ui, rect, lit, click, id, root, 48, 2)
}

/// A piano of `octaves` octaves from the C at MIDI `base`, with `lit` notes marked (a note outside the range
/// shows an octave in).
#[allow(clippy::too_many_arguments)]
pub(crate) fn piano_span(
    ui: &mut egui::Ui,
    rect: Rect,
    lit: &[i32],
    click: bool,
    id: &str,
    root: Option<i32>,
    base: i32,
    octaves: i32,
) -> Option<i32> {
    let p = ui.painter().clone();
    let whites: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
    let nw = 7 * octaves;
    let ww = rect.width() / nw as f32;
    let span = 12 * octaves;
    let lit_pc = |m: i32| lit.iter().any(|x| base + (x - base).rem_euclid(span) == m);
    let mut hit = None;
    // white keys
    for k in 0..nw {
        let m = base + (k / 7) * 12 + whites[(k % 7) as usize];
        let r = Rect::from_min_size(Pos2::new(rect.min.x + k as f32 * ww, rect.min.y), Vec2::new(ww - 1.0, rect.height()));
        let on = lit_pc(m);
        let fill = match (on, root) {
            (true, Some(rt)) => degree(m, rt).1,
            (true, None) => pal().red,
            _ => Color32::from_gray(238),
        };
        p.rect_filled(r, 2.0, fill);
        p.rect_stroke(r, 2.0, Stroke::new(1.0_f32, Color32::from_gray(90)));
        if let (true, Some(rt)) = (on, root) {
            ptext(&p, Pos2::new(r.center().x, r.max.y - 10.0), Align::Center, key_name(m, rt), 1.0, Color32::from_gray(10));
        }
        if click && ui.interact(r, ui.id().with((id, "w", k)), Sense::click()).clicked() {
            hit = Some(m.rem_euclid(12));
        }
    }
    // black keys, on top
    for k in 0..nw {
        let pc = whites[(k % 7) as usize];
        if matches!(pc, 4 | 11) {
            continue;
        }
        let m = base + (k / 7) * 12 + pc + 1;
        let r = Rect::from_min_size(
            Pos2::new(rect.min.x + (k as f32 + 1.0) * ww - ww * 0.32, rect.min.y),
            Vec2::new(ww * 0.64, rect.height() * 0.6),
        );
        let on = lit_pc(m);
        let fill = match (on, root) {
            (true, Some(rt)) => degree(m, rt).1,
            (true, None) => pal().red,
            _ => Color32::from_gray(25),
        };
        p.rect_filled(r, 2.0, fill);
        if let (true, Some(rt)) = (on, root) {
            ptext(&p, Pos2::new(r.center().x, r.max.y - 9.0), Align::Center, degree(m, rt).0, 1.0, Color32::from_gray(10));
        }
        if click && ui.interact(r, ui.id().with((id, "b", k)), Sense::click()).clicked() {
            hit = Some(m.rem_euclid(12));
        }
    }
    hit
}

/// A chord's notes laid out on the piano: the bass, then the rest climbing from it.
pub(crate) fn piano_voicing(c: &theory::ChordNotes) -> Vec<i32> {
    let mut out = vec![48 + c.bass];
    let mut last = 48 + c.bass;
    for pc in c.pcs.iter().filter(|p| **p != c.bass) {
        let mut m = 48 + pc;
        while m <= last {
            m += 12;
        }
        out.push(m);
        last = m;
    }
    out
}

/// A chord box for a guitar shape: six strings standing up, five frets, dots, x for a string not played and o
/// for an open one, the starting fret at the side.
pub(crate) fn chord_box_labelled(p: &egui::Painter, rect: Rect, shape: &[Option<u8>; 6], label: &dyn Fn(i32) -> String) {
    chord_box_coloured(p, rect, shape, &|m| (label(m), Color32::from_gray(245)));
}

/// A chord box whose dots each have their own colour as well as their label (the label in black).
pub(crate) fn chord_box_coloured(
    p: &egui::Painter,
    rect: Rect,
    shape: &[Option<u8>; 6],
    label: &dyn Fn(i32) -> (String, Color32),
) {
    let fretted: Vec<u8> = shape.iter().flatten().filter(|f| **f > 0).copied().collect();
    let lo = fretted.iter().min().copied().unwrap_or(1);
    let hi = fretted.iter().max().copied().unwrap_or(0);
    let start = if hi <= 4 { 1 } else { lo };
    // five frets, or more for a wide grip, so every dot sits inside the box
    let rows = (hi.saturating_sub(start) as usize + 1).max(5);
    let top = rect.min.y + 14.0;
    // (room at the side for the starting fret's number, clear of the low string's dots)
    let grid = Rect::from_min_max(Pos2::new(rect.min.x + 22.0, top), Pos2::new(rect.max.x - 8.0, rect.max.y - 4.0));
    let sx = grid.width() / 5.0;
    let fy = grid.height() / rows as f32;
    let line = Stroke::new(1.0_f32, pal().ink2);
    for s in 0..6 {
        let x = grid.min.x + s as f32 * sx;
        p.line_segment([Pos2::new(x, grid.min.y), Pos2::new(x, grid.max.y)], line);
    }
    for f in 0..=rows {
        let y = grid.min.y + f as f32 * fy;
        let w: f32 = if f == 0 && start == 1 { 3.0 } else { 1.0 };
        p.line_segment([Pos2::new(grid.min.x, y), Pos2::new(grid.max.x, y)], Stroke::new(w, pal().ink2));
    }
    if start > 1 {
        let r = (sx.min(fy) * 0.46).max(6.0);
        ptext(p, Pos2::new(grid.min.x - r - 3.0, grid.min.y + fy * 0.5), Align::Max, &start.to_string(), 1.0, pal().ink2);
    }
    for (s, f) in shape.iter().enumerate() {
        let x = grid.min.x + s as f32 * sx;
        match f {
            None => {
                ptext(p, Pos2::new(x, rect.min.y + 6.0), Align::Center, "x", 1.0, pal().dim);
            }
            Some(0) => {
                let (txt, fill) = label(theory::STANDARD[s]);
                if txt.is_empty() {
                    p.circle_stroke(Pos2::new(x, rect.min.y + 7.0), 3.0, Stroke::new(1.0_f32, pal().ink));
                } else {
                    // an open string: a hollow ring above the nut (not a dot - nothing is fretted), its degree inside
                    // (a coloured note - a cluster's red or blue - keeps its colour on the ring)
                    let ring = if fill == Color32::from_gray(245) { pal().ink } else { fill };
                    p.circle_stroke(Pos2::new(x, rect.min.y + 6.0), 7.0, Stroke::new(1.5_f32, ring));
                    ptext(p, Pos2::new(x, rect.min.y + 6.0), Align::Center, &txt, 1.0, ring);
                }
            }
            Some(f) => {
                let row = (*f as f32 - start as f32) + 0.5;
                let c = Pos2::new(x, grid.min.y + row * fy);
                let (txt, fill) = label(theory::STANDARD[s] + *f as i32);
                if txt.is_empty() {
                    p.circle_filled(c, (sx.min(fy) * 0.36).max(3.0), pal().red);
                } else {
                    // a white dot with the note's degree in black
                    p.circle_filled(c, (sx.min(fy) * 0.46).max(6.0), fill);
                    p.circle_stroke(c, (sx.min(fy) * 0.46).max(6.0), Stroke::new(1.0_f32, Color32::from_gray(60)));
                    ptext(p, c, Align::Center, &txt, 1.0, Color32::from_gray(10));
                }
            }
        }
    }
}

/// A guitar neck lying down (high string on top), frets 0 to `frets`. `shape` marks a fret per string; when
/// `click` is on, a click returns (string, fret) - fret 0 left of the first fret wire is the open string, and a
/// click on the marked fret again clears that string.
#[allow(clippy::too_many_arguments)]
pub(crate) fn neck(
    ui: &mut egui::Ui,
    rect: Rect,
    shape: &[Option<u8>; 6],
    frets: u8,
    click: bool,
    id: &str,
    root: Option<i32>,
    ghost: &[i32],
    ghost_full: bool,
) -> Option<(usize, Option<u8>)> {
    neck_alpha(ui, rect, shape, frets, click, id, root, ghost, ghost_full, 0.7)
}

/// A small warning triangle (a chord that might not work).
pub(crate) fn warn_triangle(p: &egui::Painter, r: Rect) {
    let c = r.center();
    let s = r.width().min(r.height()) * 0.42;
    let pts = vec![Pos2::new(c.x, c.y - s), Pos2::new(c.x + s, c.y + s * 0.8), Pos2::new(c.x - s, c.y + s * 0.8)];
    p.add(egui::Shape::convex_polygon(pts, Color32::from_rgb(255, 196, 0), Stroke::new(1.0_f32, Color32::BLACK)));
    p.line_segment([Pos2::new(c.x, c.y - s * 0.45), Pos2::new(c.x, c.y + s * 0.25)], Stroke::new(1.5_f32, Color32::BLACK));
    p.circle_filled(Pos2::new(c.x, c.y + s * 0.55), 1.0, Color32::BLACK);
}

/// The neck, with the faint rings at strength `alpha` (Preferences).
#[allow(clippy::too_many_arguments)]
pub(crate) fn neck_alpha(
    ui: &mut egui::Ui,
    rect: Rect,
    shape: &[Option<u8>; 6],
    frets: u8,
    click: bool,
    id: &str,
    root: Option<i32>,
    ghost: &[i32],
    ghost_full: bool,
    alpha: f32,
) -> Option<(usize, Option<u8>)> {
    let p = ui.painter().clone();
    let open_w = 26.0;
    let fw = (rect.width() - open_w) / frets as f32;
    let sh = rect.height() / 6.0;
    let wood = pal().lcd;
    p.rect_filled(Rect::from_min_max(Pos2::new(rect.min.x + open_w, rect.min.y), rect.max), 2.0, wood);
    // fret wires and the nut
    for f in 0..=frets {
        let x = rect.min.x + open_w + f as f32 * fw;
        let w: f32 = if f == 0 { 4.0 } else { 1.5 };
        p.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], Stroke::new(w, pal().ink2));
    }
    // inlays
    for f in [3u8, 5, 7, 9, 15] {
        if f <= frets {
            let x = rect.min.x + open_w + (f as f32 - 0.5) * fw;
            p.circle_filled(Pos2::new(x, rect.center().y), 4.0, pal().lcd_ghost);
        }
    }
    if 12 <= frets {
        let x = rect.min.x + open_w + 11.5 * fw;
        p.circle_filled(Pos2::new(x, rect.min.y + sh * 2.0), 4.0, pal().lcd_ghost);
        p.circle_filled(Pos2::new(x, rect.min.y + sh * 4.0), 4.0, pal().lcd_ghost);
    }
    // fret numbers
    for f in [3u8, 5, 7, 9, 12, 15] {
        if f <= frets {
            let x = rect.min.x + open_w + (f as f32 - 0.5) * fw;
            ptext(&p, Pos2::new(x, rect.max.y + 9.0), Align::Center, &f.to_string(), 1.0, pal().dim);
        }
    }
    let mut hit = None;
    for row in 0..6 {
        // row 0 is the high string (string index 5)
        let s = 5 - row;
        let y = rect.min.y + (row as f32 + 0.5) * sh;
        p.line_segment(
            [Pos2::new(rect.min.x + open_w, y), Pos2::new(rect.max.x, y)],
            Stroke::new(1.0 + (s as f32 * 0.3).min(1.5), pal().ink.gamma_multiply(0.6)),
        );
        // the chord's other notes wherever they fall on this string: faint, or full in the finder's map
        if let Some(rt) = root {
            for f in 0..=frets {
                let m = theory::STANDARD[s] + f as i32;
                if !ghost.contains(&m.rem_euclid(12)) || shape[s] == Some(f) {
                    continue;
                }
                let (txt, col) = degree(m, rt);
                let c = if f == 0 {
                    Pos2::new(rect.min.x + open_w * 0.45, y)
                } else {
                    Pos2::new(rect.min.x + open_w + (f as f32 - 0.5) * fw, y)
                };
                if ghost_full {
                    p.circle_filled(c, sh * 0.34, col);
                    ptext(&p, c, Align::Center, txt, 1.0, Color32::from_gray(10));
                } else {
                    p.circle_stroke(c, sh * 0.3, Stroke::new(1.5_f32, col.gamma_multiply(alpha)));
                }
            }
        }
        // each dot says its degree in the chord (in its colour) once the chord is known, else its note
        let label = |f: u8| {
            let m = theory::STANDARD[s] + f as i32;
            match root {
                Some(rt) => degree(m, rt),
                None => (theory::note_name(m, false), pal().red),
            }
        };
        match shape[s] {
            Some(0) => {
                let (txt, col) = label(0);
                p.circle_stroke(Pos2::new(rect.min.x + open_w * 0.45, y), sh * 0.34, Stroke::new(2.5_f32, col));
                ptext(&p, Pos2::new(rect.min.x + open_w * 0.45, y), Align::Center, txt, 1.0, pal().ink);
            }
            Some(f) => {
                let (txt, col) = label(f);
                let c = Pos2::new(rect.min.x + open_w + (f as f32 - 0.5) * fw, y);
                p.circle_filled(c, sh * 0.4, col);
                ptext(&p, c, Align::Center, txt, 1.0, Color32::from_gray(10));
                // the note's name, small, under its degree
                let note = theory::note_name(theory::STANDARD[s] + f as i32, false);
                if root.is_some() {
                    ptext(&p, Pos2::new(c.x + sh * 0.55, c.y - sh * 0.3), Align::Min, note, 1.0, pal().dim);
                }
            }
            None if !ghost_full => {
                ptext(&p, Pos2::new(rect.min.x + open_w * 0.45, y), Align::Center, "x", 1.0, pal().dim);
            }
            None => {}
        }
        if click {
            for f in 0..=frets {
                let r = if f == 0 {
                    Rect::from_min_size(Pos2::new(rect.min.x, y - sh / 2.0), Vec2::new(open_w, sh))
                } else {
                    Rect::from_min_size(Pos2::new(rect.min.x + open_w + (f - 1) as f32 * fw, y - sh / 2.0), Vec2::new(fw, sh))
                };
                let resp =
                    ui.interact(r, ui.id().with((id, s, f)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.clicked() {
                    hit = Some((s, if shape[s] == Some(f) { None } else { Some(f) }));
                }
            }
        }
    }
    hit
}

pub(crate) fn grip_notes(g: &[Option<u8>; 6]) -> Vec<i32> {
    g.iter().enumerate().filter_map(|(i, f)| f.map(|f| theory::STANDARD[i] + f as i32)).collect()
}

/// A neck lying down (high string on top) with several grips on it: the picked one bold, its notes named by
/// `label`, the others faint, each grip's notes joined by a line. A click on a dot picks that grip.
/// With `whole`, every note of those pitch classes on every string and fret is lit too, under the shapes, so you
/// see how they sit in the whole neck.
pub(crate) fn fret_map(
    ui: &mut egui::Ui,
    grips: &[[Option<u8>; 6]],
    sel: usize,
    label: &dyn Fn(i32) -> (String, Color32),
    whole: Option<&[i32]>,
) -> Option<usize> {
    let w = ui.available_width().min(940.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 170.0), Sense::click());
    let neck = Rect::from_min_max(rect.min + Vec2::new(30.0, 10.0), rect.max - Vec2::new(8.0, 26.0));
    let p = ui.painter();
    p.rect_filled(neck, 2.0, pal().lcd);
    let frets = 15.0_f32;
    let fw = neck.width() / frets;
    let sh = neck.height() / 6.0;
    let x_of = |f: u8| if f == 0 { neck.min.x - 14.0 } else { neck.min.x + (f as f32 - 0.5) * fw };
    let y_of = |s: usize| neck.min.y + (5 - s) as f32 * sh + sh / 2.0;
    for f in 0..=15 {
        let x = neck.min.x + f as f32 * fw;
        p.line_segment(
            [Pos2::new(x, neck.min.y), Pos2::new(x, neck.max.y)],
            Stroke::new(if f == 0 { 4.0_f32 } else { 1.2 }, pal().ink2),
        );
    }
    for f in [3u8, 5, 7, 9, 12, 15] {
        ptext(p, Pos2::new(x_of(f), neck.max.y + 12.0), Align::Center, &f.to_string(), 1.0, pal().dim);
    }
    for s in 0..6 {
        p.line_segment(
            [Pos2::new(neck.min.x, y_of(s)), Pos2::new(neck.max.x, y_of(s))],
            Stroke::new(1.0_f32, pal().ink.gamma_multiply(0.5)),
        );
    }
    // the whole neck: each note of the chord wherever it falls, ringed in its colour
    if let Some(pcs) = whole {
        for s in 0..6 {
            for f in 0..=15u8 {
                let m = theory::STANDARD[s] + f as i32;
                if !pcs.contains(&m.rem_euclid(12)) {
                    continue;
                }
                let c = Pos2::new(x_of(f), y_of(s));
                let (txt, col) = label(m);
                p.circle_filled(c, sh * 0.36, pal().lcd);
                p.circle_stroke(c, sh * 0.36, Stroke::new(1.5_f32, col));
                ptext(p, c, Align::Center, &txt, 1.0, col);
            }
        }
    }
    let mut dots: Vec<(Pos2, usize)> = Vec::new();
    // the faint ones first, the picked one on top
    let order: Vec<usize> =
        (0..grips.len()).filter(|i| *i != sel).chain(std::iter::once(sel).filter(|s| *s < grips.len())).collect();
    for gi in order {
        let g = &grips[gi];
        let on = gi == sel;
        let pts: Vec<Pos2> = (0..6).filter_map(|s| g[s].map(|f| Pos2::new(x_of(f), y_of(s)))).collect();
        p.add(egui::Shape::line(
            pts.clone(),
            Stroke::new(if on { 2.5_f32 } else { 1.0 }, pal().ink2.gamma_multiply(if on { 0.9 } else { 0.35 })),
        ));
        for s in 0..6 {
            let Some(f) = g[s] else { continue };
            let c = Pos2::new(x_of(f), y_of(s));
            let (txt, col) = label(theory::STANDARD[s] + f as i32);
            if on {
                p.circle_filled(c, sh * 0.42, col);
                p.circle_stroke(c, sh * 0.42, Stroke::new(1.0_f32, Color32::from_gray(40)));
                ptext(p, c, Align::Center, &txt, 1.0, Color32::from_gray(10));
            } else {
                p.circle_filled(c, sh * 0.3, col.gamma_multiply(0.45));
            }
            dots.push((c, gi));
        }
    }
    let clicked = resp.clicked();
    let hit = resp.on_hover_cursor(egui::CursorIcon::PointingHand).interact_pointer_pos().filter(|_| clicked);
    hit.and_then(|pos| {
        dots.iter()
            .filter(|d| d.0.distance(pos) < sh * 0.6)
            .min_by(|a, b| a.0.distance(pos).partial_cmp(&b.0.distance(pos)).unwrap())
            .map(|d| d.1)
    })
}
