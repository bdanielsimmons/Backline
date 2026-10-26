//! Notes on a staff, shared by the desktop and the phone: the treble clef, an accidental, and a chord written on
//! a staff with its notes named and its tab.

use crate::font::{ptext, text_w};
use crate::lines;
use crate::skin::pal;
use crate::icons::pixmap;
use eframe::egui::{self, Align, Color32, Pos2, Rect, Stroke, Vec2};

/// A small treble clef in dots.
pub(crate) const CLEF: [&str; 18] = [
    "...##...", "..#..#..", "..#..#..", "..#.#...", "...##...", "...#....", "..##....", ".#.#....", "#..###..", "#.#.#.#.",
    "#.#.#.#.", ".#..#.#.", "..###...", "....#...", "....#...", "..#.#...", "..##....", "........",
];

/// A sharp (two upright strokes crossed by two rising ones) or a flat (an upright stroke with a bowl at its foot),
/// drawn beside a note head at `c`, sized to the staff gap `g`.
pub(crate) fn accidental(p: &egui::Painter, c: Pos2, acc: i32, g: f32, col: Color32) {
    let s = Stroke::new(1.1_f32, col);
    if acc > 0 {
        for dx in [-0.35, 0.35] {
            p.line_segment([Pos2::new(c.x + dx * g, c.y - 1.2 * g), Pos2::new(c.x + dx * g, c.y + 1.2 * g)], s);
        }
        for dy in [-0.4, 0.4] {
            p.line_segment(
                [Pos2::new(c.x - 0.75 * g, c.y + dy * g + 0.2 * g), Pos2::new(c.x + 0.75 * g, c.y + dy * g - 0.2 * g)],
                Stroke::new(1.8_f32, col),
            );
        }
    } else if acc == 0 {
        // a natural
        p.line_segment([Pos2::new(c.x - 0.3 * g, c.y - 1.2 * g), Pos2::new(c.x - 0.3 * g, c.y + 0.5 * g)], s);
        p.line_segment([Pos2::new(c.x + 0.3 * g, c.y - 0.5 * g), Pos2::new(c.x + 0.3 * g, c.y + 1.2 * g)], s);
        for dy in [-0.35, 0.35] {
            p.line_segment(
                [Pos2::new(c.x - 0.3 * g, c.y + dy * g + 0.15 * g), Pos2::new(c.x + 0.3 * g, c.y + dy * g - 0.15 * g)],
                Stroke::new(1.8_f32, col),
            );
        }
    } else {
        let x = c.x - 0.35 * g;
        p.line_segment([Pos2::new(x, c.y - 1.6 * g), Pos2::new(x, c.y + 0.5 * g)], s);
        let bowl = vec![
            Pos2::new(x, c.y + 0.5 * g),
            Pos2::new(x + 0.75 * g, c.y - 0.05 * g),
            Pos2::new(x + 0.55 * g, c.y - 0.45 * g),
            Pos2::new(x, c.y - 0.15 * g),
        ];
        p.add(egui::Shape::line(bowl, Stroke::new(1.3_f32, col)));
    }
}

/// A chord written on a staff, as guitar music is (an octave above how it sounds): its notes stacked, a note a
/// step from the one below set beside it, the accidentals kept clear of each other; each note named with its
/// degree in a column beside it, and the chord's tab when it has one ((string, fret) per note). `label` names a
/// note's degree.
pub(crate) fn chord_staff(
    ui: &mut egui::Ui,
    rect: Rect,
    notes: &[(i32, Option<(usize, u8)>)],
    sharps: bool,
    label: &dyn Fn(i32) -> (String, Color32),
) {
    let p = ui.painter();
    p.rect_filled(rect, 3.0, pal().beige_lt);
    if notes.is_empty() {
        return;
    }
    let g = 7.0_f32;
    let mut ns: Vec<(i32, Option<(usize, u8)>)> = notes.to_vec();
    ns.sort_by_key(|n| n.0);
    ns.dedup_by_key(|n| n.0);
    // the staff, upright in the middle of the box
    let top = rect.center().y - 2.0 * g;
    let bottom = top + 4.0 * g;
    let line = Stroke::new(1.0_f32, pal().ink2);
    let staff_w = 120.0_f32.min(rect.width() * 0.4);
    for k in 0..5 {
        let y = top + k as f32 * g;
        p.line_segment([Pos2::new(rect.min.x + 6.0, y), Pos2::new(rect.min.x + staff_w, y)], line);
    }
    pixmap(p, Pos2::new(rect.min.x + 10.0, top - 1.7 * g), 7.4 * g / CLEF.len() as f32, &CLEF, pal().ink);
    let x = rect.min.x + staff_w * 0.72;
    let mut last: Option<(i32, bool)> = None;
    let mut acc_cols: Vec<(i32, usize)> = Vec::new();
    let ink = pal().ink;
    for (m, _) in &ns {
        let (letter, acc, oct) = lines::spell(m + 12, sharps);
        let step = oct * 7 + letter;
        let y = bottom - (step - 30) as f32 * g / 2.0;
        // a second above the note under it: set to the right of the stem side
        let beside = last.is_some_and(|(s, b)| step - s == 1 && !b);
        let hx = if beside { x + g * 1.15 } else { x };
        last = Some((step, beside));
        let mut s = 28;
        while s >= step {
            let ly = bottom - (s - 30) as f32 * g / 2.0;
            p.line_segment([Pos2::new(x - 8.0, ly), Pos2::new(x + 8.0 + if beside { g } else { 0.0 }, ly)], line);
            s -= 2;
        }
        let mut s = 40;
        while s <= step {
            let ly = bottom - (s - 30) as f32 * g / 2.0;
            p.line_segment([Pos2::new(x - 8.0, ly), Pos2::new(x + 8.0 + if beside { g } else { 0.0 }, ly)], line);
            s += 2;
        }
        p.circle_filled(Pos2::new(hx, y), g * 0.55, ink);
        if acc != 0 {
            // an accidental within a 6th of one already placed goes a column further left
            let col = acc_cols.iter().filter(|(s, _)| (step - s).abs() < 6).map(|(_, c)| c + 1).max().unwrap_or(0);
            acc_cols.push((step, col));
            accidental(p, Pos2::new(x - g * 1.8 - col as f32 * g * 1.3, y), acc, g, ink);
        }
    }
    // the names, high to low, in a column
    let names_x = rect.min.x + staff_w + 14.0;
    let row_h = ((rect.height() - 12.0) / ns.len().max(1) as f32).min(16.0);
    let col_top = rect.center().y - row_h * ns.len() as f32 / 2.0 + row_h / 2.0;
    for (k, (m, _)) in ns.iter().rev().enumerate() {
        let y = col_top + k as f32 * row_h;
        let (letter, acc, _) = lines::spell(*m, sharps);
        let name = format!(
            "{}{}",
            ["C", "D", "E", "F", "G", "A", "B"][letter as usize],
            match acc {
                1 => "#",
                -1 => "\u{266D}",
                _ => "",
            }
        );
        let (deg, col) = label(*m);
        ptext(p, Pos2::new(names_x, y), Align::Min, &name, 1.5, pal().ink);
        ptext(p, Pos2::new(names_x + 34.0, y), Align::Min, &deg, 1.5, col);
    }
    // the tab: high E on top, the frets in a column
    if ns.iter().any(|n| n.1.is_some()) {
        let tx = names_x + 80.0;
        let tg = (rect.height() - 24.0) / 6.0;
        let ty0 = rect.center().y - 2.5 * tg;
        let tab_y = |s: usize| ty0 + (5 - s) as f32 * tg;
        for s in 0..6 {
            p.line_segment([Pos2::new(tx, tab_y(s)), Pos2::new((tx + 46.0).min(rect.max.x - 6.0), tab_y(s))], line);
        }
        for (_, sf) in &ns {
            if let Some((s, f)) = sf {
                let txt = f.to_string();
                let w = text_w(&txt, 1.5) + 4.0;
                let c = Pos2::new(tx + 23.0, tab_y(*s));
                p.rect_filled(Rect::from_center_size(c, Vec2::new(w, tg * 0.9)), 0.0, pal().beige_lt);
                ptext(p, c, Align::Center, &txt, 1.5, pal().ink);
            }
        }
    }
}
