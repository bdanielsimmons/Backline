//! READING (under PRACTICE): sight reading, the way the classic guitar reading courses teach it - read in one
//! position at a time, one key at a time, with the rhythms getting harder step by step, and keep going without
//! looking at the neck. Every line is made up fresh (nothing is copied from a book): notes from the key that fall
//! in the position, moving mostly by step, rhythms from the level picked, ending home. It is written out properly
//! (key and time signatures, rests, beams, dots, triplets), the tab hidden until you want to check, and it plays
//! with a count-in and a cursor - or just clicks while you play it.

use super::*;
use crate::theory;

/// Keys in the order the courses bring them in: (name, tonic, sharps (+) or flats (-) in the signature).
const KEYS: [(&str, i32, i32); 10] = [
    ("C MAJOR", 0, 0),
    ("G MAJOR", 7, 1),
    ("F MAJOR", 5, -1),
    ("D MAJOR", 2, 2),
    ("Bb MAJOR", 10, -2),
    ("A MAJOR", 9, 3),
    ("Eb MAJOR", 3, -3),
    ("E MAJOR", 4, 4),
    ("Ab MAJOR", 8, -4),
    ("Db MAJOR", 1, -5),
];
/// Positions: where the first finger sits (0 = open position).
const POSITIONS: [(&str, u8); 9] = [
    ("OPEN POSITION", 0),
    ("2ND POSITION", 2),
    ("3RD POSITION", 3),
    ("4TH POSITION", 4),
    ("5TH POSITION", 5),
    ("7TH POSITION", 7),
    ("8TH POSITION", 8),
    ("9TH POSITION", 9),
    ("12TH POSITION", 12),
];
const LEVELS: [&str; 6] = [
    "1 - WHOLES, HALVES, QUARTERS",
    "2 - + EIGHTHS",
    "3 - + DOTTED NOTES",
    "4 - + SYNCOPATION",
    "5 - + TRIPLETS",
    "6 - + SIXTEENTHS",
];
const BARS: [usize; 3] = [4, 8, 16];
/// a beat is 12 units, so eighths (6), triplets (4) and sixteenths (3) are all whole numbers
const BEAT: u16 = 12;

/// One note (or rest) of a line: its pitch (None for a rest) and its length in twelfths of a beat.
#[derive(Clone)]
pub(crate) struct RNote {
    pub midi: Option<i32>,
    pub dur: u16,
}

/// A line to read, and how it is written.
#[derive(Clone)]
pub(crate) struct Reading {
    pub notes: Vec<RNote>,
    pub beats: u16,
    pub key: usize,
    pub pos: u8,
}

/// A small random number maker (the same line comes back from the same seed).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn pick_weighted(&mut self, w: &[u32]) -> usize {
        let total: u32 = w.iter().sum();
        let mut r = (self.next() % total.max(1) as u64) as u32;
        for (i, x) in w.iter().enumerate() {
            if r < *x {
                return i;
            }
            r -= x;
        }
        0
    }
}

/// The frets a position covers (a stretch either side allowed).
fn window(pos: u8) -> (u8, u8) {
    if pos == 0 {
        (0, 4)
    } else {
        (pos - 1, pos + 4)
    }
}

/// The notes of a key that can be played in a position, low to high.
fn position_notes(key: usize, pos: u8) -> Vec<i32> {
    let scale = [0, 2, 4, 5, 7, 9, 11];
    let tonic = KEYS[key].1;
    let (lo, hi) = window(pos);
    let mut out: Vec<i32> = Vec::new();
    for s in 0..6 {
        for f in lo..=hi {
            let m = theory::STANDARD[s] + f as i32;
            if scale.contains(&(m - tonic).rem_euclid(12)) && !out.contains(&m) {
                out.push(m);
            }
        }
    }
    out.sort();
    out
}

/// Where a note is played in a position: (string, fret), the first finger's frets first.
fn finger(m: i32, pos: u8) -> Option<(usize, u8)> {
    let (lo, hi) = window(pos);
    let core = |f: u8| if pos == 0 { f <= 3 } else { f >= pos && f <= pos + 3 };
    let mut best: Option<(usize, u8, i32)> = None;
    for s in 0..6 {
        let f = m - theory::STANDARD[s];
        if f < lo as i32 || f > hi as i32 {
            continue;
        }
        let f = f as u8;
        let cost = if core(f) { 0 } else { 10 } + (f as i32 - pos as i32 - 1).abs();
        if best.is_none_or(|b| cost < b.2) {
            best = Some((s, f, cost));
        }
    }
    best.map(|b| (b.0, b.1))
}

/// The rhythm cells a level allows: groups of (length, rest?) that fill one, two, three or four beats.
fn cells(level: usize) -> Vec<(Vec<(u16, bool)>, u32)> {
    let n = |d: u16| (d, false);
    let r = |d: u16| (d, true);
    let mut c: Vec<(Vec<(u16, bool)>, u32)> = vec![(vec![n(12)], 8), (vec![n(24)], 4), (vec![n(48)], 1), (vec![r(12)], 1)];
    if level >= 1 {
        c.push((vec![n(6), n(6)], 7));
    }
    if level >= 2 {
        c.push((vec![n(18), n(6)], 4));
        c.push((vec![n(36)], 1));
        c.push((vec![r(24)], 1));
    }
    if level >= 3 {
        c.push((vec![n(6), n(12), n(6)], 4));
        c.push((vec![r(6), n(6)], 2));
        c.push((vec![n(6), r(6)], 1));
    }
    if level >= 4 {
        c.push((vec![n(4), n(4), n(4)], 4));
    }
    if level >= 5 {
        c.push((vec![n(3), n(3), n(3), n(3)], 4));
        c.push((vec![n(6), n(3), n(3)], 3));
        c.push((vec![n(3), n(3), n(6)], 3));
        c.push((vec![n(9), n(3)], 3));
    }
    c
}

/// A fresh line: the rhythm a bar at a time from the level's cells, then the pitches - mostly steps through the
/// key, a leap now and then, starting and ending on the key note.
pub(crate) fn make_line(seed: u64, key: usize, pos: u8, level: usize, bars: usize, beats: u16) -> Reading {
    let mut rng = Rng(seed | 1);
    let bar_len = beats * BEAT;
    let all = cells(level);
    let mut rhythm: Vec<(u16, bool)> = Vec::new();
    for b in 0..bars {
        let mut at = 0u16;
        let last = b + 1 == bars;
        while at < bar_len {
            let left = bar_len - at;
            // the last bar ends on a long note
            if last && left <= 2 * BEAT && at > 0 {
                rhythm.push((left, false));
                break;
            }
            let fit: Vec<usize> = (0..all.len())
                .filter(|i| {
                    let len: u16 = all[*i].0.iter().map(|x| x.0).sum();
                    // long notes start on a strong beat; a whole note fills a 4/4 bar
                    len <= left && (len < 2 * BEAT || at % (2 * BEAT) == 0) && (len != 4 * BEAT || at == 0)
                })
                .collect();
            let w: Vec<u32> = fit.iter().map(|i| all[*i].1).collect();
            let cell = &all[fit[rng.pick_weighted(&w)]].0;
            // a rest never opens the line
            if rhythm.is_empty() && cell[0].1 {
                continue;
            }
            for x in cell {
                rhythm.push(*x);
            }
            at += cell.iter().map(|x| x.0).sum::<u16>();
        }
    }
    let avail = position_notes(key, pos);
    let tonic = KEYS[key].1;
    let tonics: Vec<usize> = (0..avail.len()).filter(|i| (avail[*i] - tonic).rem_euclid(12) == 0).collect();
    let mid = avail.len() / 2;
    let mut i = tonics.iter().copied().min_by_key(|t| (*t as i32 - mid as i32).abs()).unwrap_or(mid);
    let n_notes = rhythm.iter().filter(|x| !x.1).count();
    let mut k = 0;
    let mut notes = Vec::new();
    for (dur, rest) in rhythm {
        if rest {
            notes.push(RNote { midi: None, dur });
            continue;
        }
        if k > 0 {
            if k + 1 == n_notes {
                // home: the key note closest to here
                i = tonics.iter().copied().min_by_key(|t| (*t as i32 - i as i32).abs()).unwrap_or(i);
            } else {
                const MOVES: [i32; 9] = [0, 1, -1, 2, -2, 3, -3, 4, -4];
                let w: [u32; 9] = [4, 30, 30, 11, 11, 4, 4, 2 + level as u32, 2 + level as u32];
                let mut step = MOVES[rng.pick_weighted(&w)];
                // turn back at the edges of the position
                if (i as i32 + step) < 0 || (i as i32 + step) >= avail.len() as i32 {
                    step = -step;
                }
                i = (i as i32 + step).clamp(0, avail.len() as i32 - 1) as usize;
            }
        }
        notes.push(RNote { midi: avail.get(i).copied(), dur });
        k += 1;
    }
    let _ = rng.below(1);
    Reading { notes, beats, key, pos }
}

/// Where each note starts (in beats from the top).
fn onsets(r: &Reading) -> Vec<f32> {
    let mut at = 0u32;
    r.notes
        .iter()
        .map(|n| {
            let o = at as f32 / BEAT as f32;
            at += n.dur as u32;
            o
        })
        .collect()
}

/// The staff step of a written note (guitar is written an octave above it sounds); the bottom line E is 30.
fn step_of(m: i32, sharps: bool) -> i32 {
    let (letter, _, oct) = crate::lines::spell(m + 12, sharps);
    oct * 7 + letter
}

/// Draw a line on staves, as many bars to a row as fit. `lit` is the note sounding now. With `tab` the tab is
/// under each staff; with `names` the note names are.
fn draw_reading(ui: &mut egui::Ui, r: &Reading, lit: Option<usize>, tab: bool, names: bool) {
    let g = 7.0_f32;
    let sig = KEYS[r.key].2;
    let sharps = sig > 0;
    let bar_len = r.beats * BEAT;
    // the notes of each bar
    let mut bars: Vec<Vec<usize>> = Vec::new();
    let mut at = 0u16;
    for (i, n) in r.notes.iter().enumerate() {
        if at % bar_len == 0 {
            bars.push(Vec::new());
        }
        if let Some(b) = bars.last_mut() {
            b.push(i);
        }
        at += n.dur;
    }
    let slot = |d: u16| 20.0 + d as f32 * 1.7;
    let bar_w = |b: &Vec<usize>| 18.0 + b.iter().map(|i| slot(r.notes[*i].dur)).sum::<f32>();
    let w = ui.available_width().max(300.0);
    let head_w = |first: bool| 40.0 + sig.unsigned_abs() as f32 * 9.0 + if first { 26.0 } else { 0.0 };
    // rows of bars
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut used = 0.0;
    for (bi, b) in bars.iter().enumerate() {
        let need = bar_w(b);
        if rows.is_empty() || used + need > w - 12.0 {
            rows.push(Vec::new());
            used = head_w(rows.len() == 1);
        }
        if let Some(row) = rows.last_mut() {
            row.push(bi);
        }
        used += need;
    }
    let above = 46.0;
    let staff_h = 4.0 * g;
    let below = 44.0;
    let tab_h = if tab { 58.0 } else { 0.0 };
    let names_h = if names { 18.0 } else { 0.0 };
    let row_h = above + staff_h + below + tab_h + names_h;
    let starts = onsets(r);
    for (ri, row) in rows.iter().enumerate() {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, row_h), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect.shrink2(Vec2::new(0.0, 2.0)), 3.0, pal().beige_lt);
        let top = rect.min.y + above;
        let bottom = top + staff_h;
        let y_of = |step: i32| bottom - (step - 30) as f32 * g / 2.0;
        let line = Stroke::new(1.0_f32, pal().ink2);
        let ink = pal().ink;
        // how wide this row really is: its bars stretched to fill the paper
        let natural: f32 = row.iter().map(|b| bar_w(&bars[*b])).sum();
        let x_head = rect.min.x + head_w(ri == 0);
        let last_row = ri + 1 == rows.len();
        let stretch =
            if last_row && natural < (rect.max.x - 10.0 - x_head) * 0.6 { 1.0 } else { (rect.max.x - 10.0 - x_head) / natural };
        let x_end = x_head + natural * stretch;
        for k in 0..5 {
            let y = top + k as f32 * g;
            p.line_segment([Pos2::new(rect.min.x + 6.0, y), Pos2::new(x_end, y)], line);
        }
        pixmap(
            p,
            Pos2::new(rect.min.x + 10.0, top - 1.7 * g),
            7.4 * g / crate::lines_ui::CLEF.len() as f32,
            &crate::lines_ui::CLEF,
            ink,
        );
        // the key signature
        const SHARP_STEPS: [i32; 7] = [38, 35, 39, 36, 33, 37, 34];
        const FLAT_STEPS: [i32; 7] = [34, 37, 33, 36, 32, 35, 31];
        for k in 0..sig.unsigned_abs() as usize {
            let st = if sharps { SHARP_STEPS[k] } else { FLAT_STEPS[k] };
            crate::lines_ui::accidental(
                p,
                Pos2::new(rect.min.x + 40.0 + k as f32 * 9.0, y_of(st)),
                if sharps { 1 } else { -1 },
                g,
                ink,
            );
        }
        // the time signature, on the first row
        if ri == 0 {
            let tx = rect.min.x + 40.0 + sig.unsigned_abs() as f32 * 9.0 + 12.0;
            ptext(p, Pos2::new(tx, top + g), Align::Center, &r.beats.to_string(), 3.0, ink);
            ptext(p, Pos2::new(tx, top + 3.0 * g), Align::Center, "4", 3.0, ink);
        }
        // the bar number
        if let Some(b0) = row.first() {
            ptext(p, Pos2::new(rect.min.x + 8.0, top - 2.6 * g), Align::Min, &(b0 + 1).to_string(), 1.0, pal().dim);
        }
        let tab_top = bottom + below;
        let tab_y = |s: usize| tab_top + (5 - s) as f32 * 9.0;
        if tab {
            for s in 0..6 {
                p.line_segment([Pos2::new(rect.min.x + 6.0, tab_y(s)), Pos2::new(x_end, tab_y(s))], line);
            }
        }
        let mut x = x_head;
        for bi in row {
            let b = &bars[*bi];
            x += 9.0 * stretch;
            // where each note sits, and its stem direction (beamed groups agree)
            let mut heads: Vec<(usize, f32, Option<i32>)> = Vec::new();
            let mut xx = x;
            for i in b {
                let n = &r.notes[*i];
                let cx = xx + slot(n.dur) * stretch * 0.35;
                heads.push((*i, cx, n.midi.map(|m| step_of(m, sharps))));
                xx += slot(n.dur) * stretch;
            }
            // beam groups: short notes within one beat
            let mut groups: Vec<Vec<usize>> = Vec::new();
            {
                let mut cur: Vec<usize> = Vec::new();
                let mut cur_beat = u32::MAX;
                for (k, h) in heads.iter().enumerate() {
                    let n = &r.notes[h.0];
                    let beat = (starts[h.0] * 1000.0) as u32 / 1000;
                    let short = n.dur < BEAT && n.midi.is_some();
                    if short && beat == cur_beat && !cur.is_empty() {
                        cur.push(k);
                    } else {
                        if !cur.is_empty() {
                            groups.push(std::mem::take(&mut cur));
                        }
                        if short {
                            cur.push(k);
                            cur_beat = beat;
                        }
                    }
                }
                if !cur.is_empty() {
                    groups.push(cur);
                }
            }
            let in_group = |k: usize| groups.iter().position(|gr| gr.contains(&k));
            let up_of = |steps: &[i32]| (steps.iter().sum::<i32>() as f32 / steps.len().max(1) as f32) < 34.0;
            let mut stem_tip: Vec<Option<(f32, f32, bool)>> = vec![None; heads.len()];
            for gr in &groups {
                if gr.len() < 2 {
                    continue;
                }
                let steps: Vec<i32> = gr.iter().filter_map(|k| heads[*k].2).collect();
                let up = up_of(&steps);
                let ys: Vec<f32> = steps.iter().map(|s| y_of(*s)).collect();
                let tip = if up {
                    ys.iter().cloned().fold(f32::INFINITY, f32::min) - 3.4 * g
                } else {
                    ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max) + 3.4 * g
                };
                for k in gr {
                    stem_tip[*k] = Some((tip, 0.0, up));
                }
            }
            for (k, (i, cx, st)) in heads.iter().enumerate() {
                let n = &r.notes[*i];
                let on = lit == Some(*i);
                let col = if on { pal().red } else { ink };
                if on {
                    p.rect_filled(
                        Rect::from_min_max(Pos2::new(cx - 10.0, rect.min.y + 6.0), Pos2::new(cx + 12.0, bottom + below - 4.0)),
                        2.0,
                        pal().red.gamma_multiply(0.15),
                    );
                }
                let Some(st) = *st else {
                    draw_rest(p, *cx, bottom, g, n.dur, col);
                    continue;
                };
                let y = y_of(st);
                // ledger lines
                let mut s = 28;
                while s >= st {
                    p.line_segment([Pos2::new(cx - 9.0, y_of(s)), Pos2::new(cx + 9.0, y_of(s))], line);
                    s -= 2;
                }
                let mut s = 40;
                while s <= st {
                    p.line_segment([Pos2::new(cx - 9.0, y_of(s)), Pos2::new(cx + 9.0, y_of(s))], line);
                    s += 2;
                }
                let open = n.dur >= 2 * BEAT;
                let rad = g * 0.58;
                if open {
                    p.circle_stroke(Pos2::new(*cx, y), rad - 0.6, Stroke::new(1.8_f32, col));
                } else {
                    p.circle_filled(Pos2::new(*cx, y), rad, col);
                }
                // a dot for dotted notes, in a space
                if matches!(n.dur, 9 | 18 | 36) {
                    let dy = if st % 2 == 0 { -g / 2.0 } else { 0.0 };
                    p.circle_filled(Pos2::new(cx + rad + 4.0, y + dy), 1.8, col);
                }
                if n.dur < 4 * BEAT {
                    let (tip, up) = match stem_tip[k] {
                        Some((tip, _, up)) => (tip, up),
                        None => {
                            let up = st < 34;
                            (if up { y - 3.4 * g } else { y + 3.4 * g }, up)
                        }
                    };
                    let sx = if up { cx + rad - 0.5 } else { cx - rad + 0.5 };
                    p.line_segment([Pos2::new(sx, y), Pos2::new(sx, tip)], Stroke::new(1.3_f32, col));
                    // a lone eighth or sixteenth gets its flags
                    if n.dur < BEAT && in_group(k).is_none_or(|gi| groups[gi].len() < 2) {
                        let flags = if n.dur <= 3 { 2 } else { 1 };
                        for f in 0..flags {
                            let fy = tip + if up { f as f32 * 6.0 } else { -(f as f32) * 6.0 };
                            let d = if up { 1.0 } else { -1.0 };
                            let pts =
                                vec![Pos2::new(sx, fy), Pos2::new(sx + 6.0, fy + 7.0 * d), Pos2::new(sx + 5.0, fy + 13.0 * d)];
                            p.add(egui::Shape::line(pts, Stroke::new(1.6_f32, col)));
                        }
                    }
                }
                if tab {
                    if let Some((s, f)) = finger(n.midi.unwrap_or(0), r.pos) {
                        let txt = f.to_string();
                        p.rect_filled(
                            Rect::from_center_size(Pos2::new(*cx, tab_y(s)), Vec2::new(text_w(&txt, 1.0) + 6.0, 9.0)),
                            0.0,
                            pal().beige_lt,
                        );
                        ptext(p, Pos2::new(*cx, tab_y(s)), Align::Center, &txt, 1.0, col);
                    }
                }
                if names {
                    let (letter, acc, _) = crate::lines::spell(n.midi.unwrap_or(0), sharps);
                    let nm = format!(
                        "{}{}",
                        ["C", "D", "E", "F", "G", "A", "B"][letter as usize],
                        match acc {
                            1 => "#",
                            -1 => "b",
                            _ => "",
                        }
                    );
                    ptext(p, Pos2::new(*cx, rect.max.y - 10.0), Align::Center, &nm, 1.0, if on { pal().red } else { pal().dim });
                }
            }
            // beams: one for eighths, a second for sixteenths, a 3 over triplets
            for gr in &groups {
                if gr.len() < 2 {
                    continue;
                }
                let Some((tip, _, up)) = stem_tip[gr[0]] else { continue };
                let sx = |k: usize| heads[k].1 + if up { g * 0.58 - 0.5 } else { -g * 0.58 + 0.5 };
                let (a, z) = (sx(gr[0]), sx(gr[gr.len() - 1]));
                let thick = 3.0;
                let band = |y: f32| Rect::from_min_max(Pos2::new(a, y.min(y + thick)), Pos2::new(z, y.max(y + thick)));
                p.rect_filled(band(if up { tip } else { tip - thick }), 0.0, ink);
                let y2 = if up { tip + 5.0 } else { tip - 5.0 - thick };
                for (j, k) in gr.iter().enumerate() {
                    if r.notes[heads[*k].0].dur > 3 {
                        continue;
                    }
                    // a sixteenth: the second beam to its neighbour if that is one too, else a stub
                    let next16 = gr.get(j + 1).is_some_and(|kk| r.notes[heads[*kk].0].dur <= 3);
                    let x0 = sx(*k);
                    let x1 = if next16 {
                        sx(gr[j + 1])
                    } else if j + 1 < gr.len() {
                        x0 + 8.0
                    } else {
                        x0 - 8.0
                    };
                    p.rect_filled(Rect::from_min_max(Pos2::new(x0.min(x1), y2), Pos2::new(x0.max(x1), y2 + thick)), 0.0, ink);
                }
                if r.notes[heads[gr[0]].0].dur == 4 {
                    let ty = if up { tip - 9.0 } else { tip + 9.0 };
                    ptext(p, Pos2::new((a + z) / 2.0, ty), Align::Center, "3", 1.0, pal().ink2);
                }
            }
            x += (bar_w(b) - 9.0) * stretch;
            // the bar line (a double one at the end)
            let end = *bi + 1 == bars.len();
            p.line_segment([Pos2::new(x, top), Pos2::new(x, bottom)], Stroke::new(if end { 3.0_f32 } else { 1.0 }, ink));
            if end {
                p.line_segment([Pos2::new(x - 4.0, top), Pos2::new(x - 4.0, bottom)], line);
            }
        }
        ui.add_space(6.0);
    }
}

/// A rest of `dur` twelfths of a beat, centred at `x` on the staff whose bottom line is at `bottom`.
fn draw_rest(p: &egui::Painter, x: f32, bottom: f32, g: f32, dur: u16, col: Color32) {
    let mid = bottom - 2.0 * g;
    match dur {
        48 => {
            let y = bottom - 3.0 * g;
            p.rect_filled(Rect::from_min_max(Pos2::new(x - g * 0.7, y), Pos2::new(x + g * 0.7, y + g * 0.5)), 0.0, col);
        }
        24 | 36 => {
            p.rect_filled(Rect::from_min_max(Pos2::new(x - g * 0.7, mid - g * 0.5), Pos2::new(x + g * 0.7, mid)), 0.0, col);
        }
        12 | 18 => {
            let k = g / 7.0;
            let pts: Vec<Pos2> = [(-2.0, -10.0), (3.0, -5.0), (-2.0, 0.0), (3.0, 5.0), (-1.0, 5.0), (1.0, 10.0)]
                .iter()
                .map(|(dx, dy)| Pos2::new(x + dx * k * 1.2, mid + dy * k * 1.2))
                .collect();
            p.add(egui::Shape::line(pts, Stroke::new(2.0_f32, col)));
        }
        _ => {
            // an eighth (one hook) or sixteenth (two)
            let hooks = if dur <= 3 { 2 } else { 1 };
            p.line_segment([Pos2::new(x + 3.0, mid - g * 0.8), Pos2::new(x - 1.0, mid + g * 1.4)], Stroke::new(1.6_f32, col));
            for h in 0..hooks {
                p.circle_filled(Pos2::new(x - 2.0 - h as f32, mid - g * 0.6 + h as f32 * g), 2.0, col);
            }
        }
    }
}

/// The notes of the key in a position, on a little neck (fret numbers along the bottom).
fn position_map(ui: &mut egui::Ui, key: usize, pos: u8, sharps: bool) {
    let (lo, hi) = window(pos);
    let n = (hi - lo + 1) as f32;
    let w = (n * 56.0 + 40.0).min(ui.available_width());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 130.0), Sense::hover());
    let neck = Rect::from_min_max(rect.min + Vec2::new(30.0, 8.0), rect.max - Vec2::new(8.0, 22.0));
    let p = ui.painter();
    p.rect_filled(neck, 2.0, pal().lcd);
    let fw = neck.width() / n;
    let sh = neck.height() / 6.0;
    for f in 0..=(hi - lo + 1) {
        let x = neck.min.x + f as f32 * fw;
        p.line_segment(
            [Pos2::new(x, neck.min.y), Pos2::new(x, neck.max.y)],
            Stroke::new(if lo == 0 && f == 1 { 3.0_f32 } else { 1.0 }, pal().ink2),
        );
    }
    for f in lo..=hi {
        ptext(
            p,
            Pos2::new(neck.min.x + (f - lo) as f32 * fw + fw / 2.0, neck.max.y + 11.0),
            Align::Center,
            &f.to_string(),
            1.0,
            pal().dim,
        );
    }
    let tonic = KEYS[key].1;
    for s in 0..6 {
        let y = neck.min.y + (5 - s) as f32 * sh + sh / 2.0;
        p.line_segment([Pos2::new(neck.min.x, y), Pos2::new(neck.max.x, y)], Stroke::new(1.0_f32, pal().ink.gamma_multiply(0.5)));
        for f in lo..=hi {
            let m = theory::STANDARD[s] + f as i32;
            if ![0, 2, 4, 5, 7, 9, 11].contains(&(m - tonic).rem_euclid(12)) {
                continue;
            }
            let c = Pos2::new(neck.min.x + (f - lo) as f32 * fw + fw / 2.0, y);
            let home = (m - tonic).rem_euclid(12) == 0;
            p.circle_filled(c, sh * 0.4, if home { pal().red } else { Color32::from_gray(235) });
            ptext(p, c, Align::Center, theory::note_name(m, sharps), 1.0, Color32::from_gray(10));
        }
    }
}

impl App {
    /// The READING page.
    pub(crate) fn reading_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let _ = acts;
        ui.add_space(6.0);
        title_line(ui, "SIGHT READING", 3.0, pal().ink);
        para(ui, "Read in one position, in one key, without looking at the neck. Pick the position, the key and how hard the rhythms get, then press NEW LINE. Count yourself in and keep going: a wrong note is fine, stopping isn't. HEAR IT plays the line; PLAY ALONG only clicks, with a cursor, while you play. Every line is new.", pal().ink2);
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            let keys: Vec<&str> = KEYS.iter().map(|k| k.0).collect();
            if let Some(i) = dropdown(ui, "rd_key", "", &keys, self.rd_key, 150.0) {
                self.rd_key = i;
                changed = true;
            }
            let pos: Vec<&str> = POSITIONS.iter().map(|p| p.0).collect();
            if let Some(i) = dropdown(ui, "rd_pos", "", &pos, self.rd_pos, 170.0) {
                self.rd_pos = i;
                changed = true;
            }
            if let Some(i) = dropdown(ui, "rd_lvl", "RHYTHMS", &LEVELS, self.rd_level, 280.0) {
                self.rd_level = i;
                changed = true;
            }
        });
        ui.horizontal_wrapped(|ui| {
            let bars: Vec<String> = BARS.iter().map(|b| format!("{} BARS", b)).collect();
            let bars: Vec<&str> = bars.iter().map(|s| s.as_str()).collect();
            if let Some(i) = dropdown(ui, "rd_bars", "", &bars, self.rd_bars, 110.0) {
                self.rd_bars = i;
                changed = true;
            }
            if let Some(i) = dropdown(ui, "rd_time", "TIME", &["4/4", "3/4"], self.rd_time, 120.0) {
                self.rd_time = i;
                changed = true;
            }
            ui.add_space(10.0);
            crate::views::label(ui, "TEMPO", 60.0);
            if retro_btn_w(ui, "-", 28.0, false).clicked() {
                self.rd_bpm = (self.rd_bpm - 5).max(30);
            }
            crate::lcd_box(ui, &format!("{} BPM", self.rd_bpm), 80.0, pal().ink);
            if retro_btn_w(ui, "+", 28.0, false).clicked() {
                self.rd_bpm = (self.rd_bpm + 5).min(220);
            }
        });
        let new_line = |app: &mut App| {
            let seed =
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
            let beats = if app.rd_time == 1 { 3 } else { 4 };
            app.rd_line = Some(make_line(seed, app.rd_key, POSITIONS[app.rd_pos].1, app.rd_level, BARS[app.rd_bars], beats));
            app.rd_play = None;
        };
        if changed || self.rd_line.is_none() {
            new_line(self);
        }
        // a play-through running: the cursor, and counting it once it is done
        let lit = self.rd_lit(ui);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "NEW LINE", false).tip("A fresh line with these settings").clicked() {
                self.player.send(Cmd::StopOnce);
                new_line(self);
            }
            let playing = self.rd_play.is_some();
            if retro_btn(ui, "HEAR IT", playing && self.rd_play.as_ref().is_some_and(|p| !p.2))
                .tip("A bar to count you in, then the line")
                .clicked()
            {
                self.rd_start(false);
            }
            if retro_btn(ui, "PLAY ALONG", playing && self.rd_play.as_ref().is_some_and(|p| p.2))
                .tip("Just the clicks and the cursor - you play it")
                .clicked()
            {
                self.rd_start(true);
            }
            if playing && retro_btn(ui, "STOP", false).clicked() {
                self.player.send(Cmd::StopOnce);
                self.rd_play = None;
            }
            ui.add_space(10.0);
            if retro_btn(ui, "SHOW TAB", self.rd_tab).tip("Check yourself: where each note is played in this position").clicked()
            {
                self.rd_tab = !self.rd_tab;
            }
            if retro_btn(ui, "NOTE NAMES", self.rd_names).clicked() {
                self.rd_names = !self.rd_names;
            }
            if retro_btn(ui, "THE POSITION", self.rd_map).tip("The key's notes in this position, on the neck").clicked() {
                self.rd_map = !self.rd_map;
            }
        });
        let today = crate::store::today_str();
        let read = self.store.days.iter().find(|d| d.date == today).map_or(0, |d| d.reads);
        crate::views::dim_line(ui, &format!("LINES READ TODAY: {}   (PLAY ALONG TO THE END TO COUNT ONE)", read), 1.0, pal().dim);
        ui.add_space(6.0);
        if self.rd_map {
            position_map(ui, self.rd_key, POSITIONS[self.rd_pos].1, KEYS[self.rd_key].2 > 0);
            ui.add_space(6.0);
        }
        if let Some(r) = self.rd_line.clone() {
            draw_reading(ui, &r, lit, self.rd_tab, self.rd_names);
        }
        ui.add_space(10.0);
        section_header(ui, "HOW TO WORK");
        for tip in [
            "Before you play, look the line over: the key, the highest and lowest notes, the hardest rhythm.",
            "Count a bar in out loud. Play slower than you think you need to; then raise the tempo 5 at a time.",
            "Eyes on the page, not the neck. The position keeps your hand in place: one finger per fret.",
            "New position? Turn on THE POSITION, learn where the key's notes are, then turn it off and read.",
            "Read a few new lines every day rather than one line many times - the point is reading, not memorising.",
        ] {
            para(ui, tip, pal().ink2);
        }
        ui.add_space(20.0);
    }

    /// Start a play-through: the notes (or only the clicks), with a bar to count in.
    fn rd_start(&mut self, along: bool) {
        let Some(r) = self.rd_line.clone() else { return };
        let spb = 60.0 / self.rd_bpm.max(30) as f32;
        let starts = onsets(&r);
        let notes: Vec<(Option<i32>, f32, f32)> =
            r.notes.iter().zip(&starts).map(|(n, s)| (n.midi, s * spb, n.dur as f32 / BEAT as f32 * spb)).collect();
        let count = r.beats as u32;
        self.player.send(Cmd::StopOnce);
        self.player.send(Cmd::Once(crate::band::reading_sound(&notes, r.beats as u32, count, spb, !along), 0.9));
        let lead = count as f32 * spb;
        let total = lead + notes.iter().map(|n| n.1 + n.2).fold(0.0, f32::max);
        self.rd_play = Some((Instant::now(), starts.iter().map(|s| lead + s * spb).collect(), along, total));
    }

    /// The note sounding now in a play-through; a play-along that reaches the end is counted in the diary.
    fn rd_lit(&mut self, ui: &egui::Ui) -> Option<usize> {
        let (t0, starts, along, total) = self.rd_play.clone()?;
        let el = t0.elapsed().as_secs_f32();
        if el > total + 0.3 {
            self.rd_play = None;
            if along {
                let date = crate::store::today_str();
                let d = self.store.day_mut(&date);
                d.reads += 1;
                *d.secs.entry("Sight reading".to_string()).or_insert(0) += total as u32;
                self.store_dirty = true;
            }
            return None;
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
        starts.iter().rposition(|s| el >= *s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every bar adds up, every note is in the key and playable in the position, and the line ends home.
    #[test]
    fn lines_add_up() {
        for level in 0..6 {
            for (key, pos) in [(0usize, 0u8), (1, 2), (3, 5), (4, 7), (9, 9)] {
                for beats in [3u16, 4] {
                    let r = make_line(level as u64 * 7919 + key as u64 * 31 + pos as u64 + 1, key, pos, level, 8, beats);
                    let total: u32 = r.notes.iter().map(|n| n.dur as u32).sum();
                    assert_eq!(total, 8 * beats as u32 * BEAT as u32, "level {} key {} pos {}", level, key, pos);
                    // no note runs over a bar line
                    let mut at = 0u32;
                    for n in &r.notes {
                        let bar = (beats * BEAT) as u32;
                        assert!(at / bar == (at + n.dur as u32 - 1) / bar, "a note crosses a bar line");
                        at += n.dur as u32;
                        assert!([3, 4, 6, 9, 12, 18, 24, 36, 48].contains(&n.dur), "odd length {}", n.dur);
                    }
                    let tonic = KEYS[key].1;
                    for m in r.notes.iter().filter_map(|n| n.midi) {
                        assert!([0, 2, 4, 5, 7, 9, 11].contains(&(m - tonic).rem_euclid(12)), "out of key");
                        assert!(finger(m, pos).is_some(), "not in the position");
                    }
                    let last = r.notes.iter().rev().find_map(|n| n.midi).unwrap();
                    assert_eq!((last - tonic).rem_euclid(12), 0, "ends home");
                    assert!(r.notes[0].midi.is_some(), "starts on a note");
                }
            }
        }
    }
}
