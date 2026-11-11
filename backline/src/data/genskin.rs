//! Winamp-style skins drawn by Backline itself, from a look's three colours (`crate::skin::LOOKS`, or your Custom
//! one): every picture a classic skin has (main window, buttons, digits, font, sliders, equalizer, playlist frame) is
//! painted here in code, so nothing is borrowed from anyone's skin. Soft gradients, brushed panels and a glowing
//! display, in the spirit of Winamp 5; dark panels get light writing, light (paper) ones dark.
//! The result is an ordinary .wsz file, worn through the same path as a downloaded skin.

use crate::skin::Pal;
use crate::winamp::{darker, lighter, lum, mix, readable};
use eframe::egui::Color32;
use image::{Rgba, RgbaImage};
use std::io::Write;

/// The colours a skin paints with.
struct Th {
    face: Color32,
    face_hi: Color32,
    face_lo: Color32,
    edge: Color32,
    /// writing on the panels (labels like KBPS)
    face_ink: Color32,
    display: Color32,
    display_ink: Color32,
    display_dim: Color32,
    accent: Color32,
    btn: Color32,
    btn_hi: Color32,
    btn_lo: Color32,
    btn_ink: Color32,
    /// paper panels (dark writing) rather than dark ones
    light: bool,
}

/// A look's colours (its `panel`, its `display`, its `accent`) worked out into all a skin needs:
/// the accent always lit enough to read on the display.
fn theme(p: &Pal) -> Th {
    let (base, lcd) = (p.panel, p.display);
    let glow = readable(p.accent, lcd, 7.0);
    if lum(base) > 0.4 {
        Th {
            face: base,
            face_hi: lighter(base, 0.45),
            face_lo: darker(base, 0.14),
            edge: darker(base, 0.62),
            face_ink: readable(darker(base, 0.6), base, 4.5),
            display: lcd,
            display_ink: glow,
            display_dim: mix(lcd, glow, 0.13),
            accent: glow,
            btn: lighter(base, 0.25),
            btn_hi: lighter(base, 0.6),
            btn_lo: darker(base, 0.06),
            btn_ink: Color32::from_gray(30),
            light: true,
        }
    } else {
        Th {
            face: base,
            face_hi: lighter(base, 0.14),
            face_lo: darker(base, 0.35),
            edge: Color32::from_gray(6),
            face_ink: Color32::from_gray(170),
            display: lcd,
            display_ink: glow,
            display_dim: mix(lcd, glow, 0.13),
            accent: glow,
            btn: lighter(base, 0.08),
            btn_hi: lighter(base, 0.22),
            btn_lo: darker(base, 0.2),
            btn_ink: Color32::from_gray(225),
            light: false,
        }
    }
}

// ------------------------------------------------------------------ painting

struct Pic(RgbaImage);

fn rgba(c: Color32) -> Rgba<u8> {
    Rgba([c.r(), c.g(), c.b(), 255])
}

impl Pic {
    fn new(w: u32, h: u32, bg: Color32) -> Pic {
        Pic(RgbaImage::from_pixel(w, h, rgba(bg)))
    }
    fn dot(&mut self, x: i32, y: i32, c: Color32) {
        if x >= 0 && y >= 0 && (x as u32) < self.0.width() && (y as u32) < self.0.height() {
            self.0.put_pixel(x as u32, y as u32, rgba(c));
        }
    }
    fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.dot(xx, yy, c);
            }
        }
    }
    /// Top-to-bottom gradient.
    fn grad(&mut self, x: i32, y: i32, w: i32, h: i32, top: Color32, bot: Color32) {
        for i in 0..h {
            let t = if h > 1 { i as f32 / (h - 1) as f32 } else { 0.0 };
            self.fill(x, y + i, w, 1, mix(top, bot, t));
        }
    }
    /// One-pixel bevel: light on top and left, dark on bottom and right.
    fn bevel(&mut self, x: i32, y: i32, w: i32, h: i32, hi: Color32, lo: Color32) {
        self.fill(x, y, w, 1, hi);
        self.fill(x, y, 1, h, hi);
        self.fill(x, y + h - 1, w, 1, lo);
        self.fill(x + w - 1, y, 1, h, lo);
    }
    fn outline(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color32) {
        self.bevel(x, y, w, h, c, c);
    }
    /// A small ASCII picture ('#' = a pixel).
    fn art(&mut self, x: i32, y: i32, rows: &[&str], c: Color32) {
        for (j, r) in rows.iter().enumerate() {
            for (i, ch) in r.chars().enumerate() {
                if ch == '#' {
                    self.dot(x + i as i32, y + j as i32, c);
                }
            }
        }
    }
    /// Text in Backline's own 5 x 6 font.
    fn text(&mut self, x: i32, y: i32, s: &str, c: Color32) {
        for (i, ch) in s.chars().enumerate() {
            if let Some(g) = glyph(ch) {
                self.art(x + i as i32 * 5, y, &g, c);
            }
        }
    }
    fn bmp(&self) -> Vec<u8> {
        let rgb = image::DynamicImage::ImageRgba8(self.0.clone()).to_rgb8();
        let mut out = std::io::Cursor::new(Vec::new());
        let _ = rgb.write_to(&mut out, image::ImageFormat::Bmp);
        out.into_inner()
    }
}

fn text_w(s: &str) -> i32 {
    s.chars().count() as i32 * 5 - 1
}

/// A panel: a soft vertical sheen.
fn panel(p: &mut Pic, t: &Th, x: i32, y: i32, w: i32, h: i32) {
    // one flat colour, as every window's frame is: the windows read as one window, no seams where they meet
    p.fill(x, y, w, h, t.face);
}

/// A sunken display box.
fn lcd_box(p: &mut Pic, t: &Th, x: i32, y: i32, w: i32, h: i32) {
    p.fill(x, y, w, h, t.display);
    p.bevel(x - 1, y - 1, w + 2, h + 2, t.face_lo, t.face_hi);
    p.outline(x - 1, y - 1, w + 2, h + 2, t.edge);
}

/// A button face (pressed: pushed in).
fn button(p: &mut Pic, t: &Th, x: i32, y: i32, w: i32, h: i32, down: bool) {
    let (a, b) = if down { (t.btn_lo, t.btn) } else { (t.btn_hi, t.btn_lo) };
    p.grad(x, y, w, h, a, b);
    p.outline(x, y, w, h, t.edge);
    if !down {
        p.fill(x + 1, y + 1, w - 2, 1, lighter(t.btn_hi, 0.15));
    }
    // rounded corners
    for (cx, cy) in [(x, y), (x + w - 1, y), (x, y + h - 1), (x + w - 1, y + h - 1)] {
        p.dot(cx, cy, t.face_lo);
    }
}

/// A button with a word on it, and a small lamp at the left when `lamp` is Some(lit).
fn label_button(p: &mut Pic, t: &Th, x: i32, y: i32, w: i32, h: i32, down: bool, label: &str, lamp: Option<bool>) {
    button(p, t, x, y, w, h, down);
    let d = if down { 1 } else { 0 };
    let lamp_w = if lamp.is_some() { 5 } else { 0 };
    let tx = x + (w - text_w(label) - lamp_w) / 2 + lamp_w + d;
    let ty = y + (h - 5) / 2 + d;
    if let Some(on) = lamp {
        let c = if on { t.accent } else { mix(t.btn, t.edge, 0.45) };
        p.fill(tx - 5, ty + 1, 3, 3, c);
        if on {
            p.dot(tx - 4, ty + 2, lighter(c, 0.5));
        }
    }
    p.text(tx, ty, label, t.btn_ink);
}

/// A title strip with `title` in the middle.
fn title_strip(p: &mut Pic, t: &Th, x: i32, y: i32, w: i32, h: i32, title: &str) {
    p.fill(x, y, w, h, t.face);
    // the title on the display's colours, as every window's name is written (Library, Practice)
    let tw = text_w(title);
    let tx = x + (w - tw) / 2;
    p.fill(tx - 4, y + 1, tw + 8, h - 2, t.display);
    p.text(tx, y + (h - 5) / 2, title, t.display_ink);
}

/// A tiny square title-bar button with a picture on it.
fn tiny_button(p: &mut Pic, t: &Th, x: i32, y: i32, pic: &[&str], down: bool) {
    button(p, t, x, y, 9, 9, down);
    let d = if down { 1 } else { 0 };
    p.art(x + 2 + d, y + 2 + d, pic, t.btn_ink);
}

const PIC_OPT: [&str; 5] = ["#####", ".....", "#####", ".....", "#####"];
const PIC_MIN: [&str; 5] = [".....", ".....", ".....", ".....", "#####"];
const PIC_SHADE: [&str; 5] = ["#####", "#...#", "#####", ".....", "....."];
const PIC_CLOSE: [&str; 5] = ["#...#", ".#.#.", "..#..", ".#.#.", "#...#"];

// ------------------------------------------------------------------ the font

/// The 5 x 6 font: each character's rows (up to 5 wide, 5 tall; the sixth row is the gap below).
fn glyph(c: char) -> Option<[&'static str; 5]> {
    Some(match c.to_ascii_lowercase() {
        'a' => [".##.", "#..#", "####", "#..#", "#..#"],
        'b' => ["###.", "#..#", "###.", "#..#", "###."],
        'c' => [".###", "#...", "#...", "#...", ".###"],
        'd' => ["###.", "#..#", "#..#", "#..#", "###."],
        'e' => ["####", "#...", "###.", "#...", "####"],
        'f' => ["####", "#...", "###.", "#...", "#..."],
        'g' => [".###", "#...", "#.##", "#..#", ".###"],
        'h' => ["#..#", "#..#", "####", "#..#", "#..#"],
        'i' => ["###", ".#.", ".#.", ".#.", "###"],
        'j' => ["..##", "...#", "...#", "#..#", ".##."],
        'k' => ["#..#", "#.#.", "##..", "#.#.", "#..#"],
        'l' => ["#...", "#...", "#...", "#...", "####"],
        'm' => ["#...#", "##.##", "#.#.#", "#...#", "#...#"],
        'n' => ["#..#", "##.#", "#.##", "#..#", "#..#"],
        'o' => [".##.", "#..#", "#..#", "#..#", ".##."],
        'p' => ["###.", "#..#", "###.", "#...", "#..."],
        'q' => [".##.", "#..#", "#..#", "#.#.", ".#.#"],
        'r' => ["###.", "#..#", "###.", "#.#.", "#..#"],
        's' => [".###", "#...", ".##.", "...#", "###."],
        't' => ["###", ".#.", ".#.", ".#.", ".#."],
        'u' => ["#..#", "#..#", "#..#", "#..#", ".##."],
        'v' => ["#..#", "#..#", "#..#", ".##.", ".##."],
        'w' => ["#...#", "#...#", "#.#.#", "##.##", "#...#"],
        'x' => ["#..#", "#..#", ".##.", "#..#", "#..#"],
        'y' => ["#.#", "#.#", ".#.", ".#.", ".#."],
        'z' => ["####", "...#", ".##.", "#...", "####"],
        'å' => [".##.", "....", "####", "#..#", "#..#"],
        'ä' => ["#..#", "....", "####", "#..#", "#..#"],
        'ö' => ["#..#", "....", ".##.", "#..#", ".##."],
        '0' => [".##.", "#..#", "#..#", "#..#", ".##."],
        '1' => [".#.", "##.", ".#.", ".#.", "###"],
        '2' => ["###.", "...#", ".##.", "#...", "####"],
        '3' => ["###.", "...#", ".##.", "...#", "###."],
        '4' => ["#..#", "#..#", "####", "...#", "...#"],
        '5' => ["####", "#...", "###.", "...#", "###."],
        '6' => [".##.", "#...", "###.", "#..#", ".##."],
        '7' => ["####", "...#", "..#.", ".#..", ".#.."],
        '8' => [".##.", "#..#", ".##.", "#..#", ".##."],
        '9' => [".##.", "#..#", ".###", "...#", ".##."],
        '"' => ["#.#", "#.#", "...", "...", "..."],
        '@' => [".##.", "#.##", "#.##", "#...", ".##."],
        '…' => ["....", "....", "....", "....", "#.#.#"],
        '.' => ["..", "..", "..", "..", "#."],
        ':' => ["..", "#.", "..", "#.", ".."],
        '(' => [".#", "#.", "#.", "#.", ".#"],
        ')' => ["#.", ".#", ".#", ".#", "#."],
        '-' => ["...", "...", "###", "...", "..."],
        '\'' => ["#", "#", ".", ".", "."],
        '!' => ["#", "#", "#", ".", "#"],
        '_' => ["....", "....", "....", "....", "####"],
        '+' => ["...", ".#.", "###", ".#.", "..."],
        '\\' => ["#..", "#..", ".#.", "..#", "..#"],
        '/' => ["..#", "..#", ".#.", "#..", "#.."],
        '[' => ["##", "#.", "#.", "#.", "##"],
        ']' => ["##", ".#", ".#", ".#", "##"],
        '^' => [".#.", "#.#", "...", "...", "..."],
        '&' => [".#..", "#.#.", ".#..", "#.#.", ".#.#"],
        '%' => ["#..#", "..#.", ".#..", "#...", "#..#"],
        ',' => ["..", "..", "..", ".#", "#."],
        '=' => ["...", "###", "...", "###", "..."],
        '$' => [".###", "##..", ".##.", "..##", "###."],
        '#' => [".#.#", "####", ".#.#", "####", ".#.#"],
        '?' => ["###.", "...#", ".##.", "....", ".#.."],
        '*' => ["...", "#.#", ".#.", "#.#", "..."],
        _ => return None,
    })
}

/// text.bmp: three rows of 31 cells, in Winamp's order.
fn text_sheet(t: &Th) -> Pic {
    let mut p = Pic::new(155, 18, t.display);
    let rows = ["abcdefghijklmnopqrstuvwxyz\"@", "0123456789….:()-'!_+\\/[]^&%,=$#", "åöä?*"];
    for (r, chars) in rows.iter().enumerate() {
        for (i, ch) in chars.chars().enumerate() {
            if let Some(g) = glyph(ch) {
                p.art(i as i32 * 5, r as i32 * 6, &g, t.display_ink);
            }
        }
    }
    p
}

/// numbers.bmp: the big time digits, as a seven-segment display with the unlit segments faintly showing.
fn numbers(t: &Th) -> Pic {
    let mut p = Pic::new(108, 13, t.display);
    // segments: top, upper right, lower right, bottom, lower left, upper left, middle
    const ON: [u8; 10] =
        [0b0111111, 0b0000110, 0b1011011, 0b1001111, 0b1100110, 0b1101101, 0b1111101, 0b0000111, 0b1111111, 0b1101111];
    let seg = |p: &mut Pic, x: i32, s: usize, c: Color32| match s {
        0 => p.fill(x + 2, 0, 5, 2, c),
        1 => p.fill(x + 7, 2, 2, 4, c),
        2 => p.fill(x + 7, 7, 2, 4, c),
        3 => p.fill(x + 2, 11, 5, 2, c),
        4 => p.fill(x, 7, 2, 4, c),
        5 => p.fill(x, 2, 2, 4, c),
        _ => p.fill(x + 2, 6, 5, 1, c),
    };
    for d in 0..12 {
        let x = d as i32 * 9;
        for s in 0..7 {
            let lit = match d {
                0..=9 => ON[d] >> s & 1 == 1,
                11 => s == 6, // the minus sign
                _ => false,   // blank
            };
            seg(&mut p, x, s, if lit { t.display_ink } else { t.display_dim });
        }
    }
    p
}

// ------------------------------------------------------------------ the sheets

fn titlebar(t: &Th) -> Pic {
    let mut p = Pic::new(344, 87, t.face);
    // the title strip, its buttons drawn in
    title_strip(&mut p, t, 27, 0, 275, 14, "BACKLINE");
    for (x, pic) in [(6, &PIC_OPT), (244, &PIC_MIN), (254, &PIC_SHADE), (264, &PIC_CLOSE)] {
        tiny_button(&mut p, t, 27 + x, 3, pic, false);
    }
    // the same strip again, unfocused (Winamp keeps both)
    title_strip(&mut p, t, 27, 15, 275, 14, "BACKLINE");
    // pressed title-bar buttons: options, minimize, close; shade below
    tiny_button(&mut p, t, 0, 9, &PIC_OPT, true);
    tiny_button(&mut p, t, 9, 9, &PIC_MIN, true);
    tiny_button(&mut p, t, 18, 9, &PIC_CLOSE, true);
    tiny_button(&mut p, t, 0, 27, &PIC_SHADE, true);
    // the clutter bar (O A I D V), and lit copies in every column below, one per item
    let clutter = |p: &mut Pic, x: i32, y: i32, lit: bool| {
        p.fill(x, y, 8, 43, if lit { t.btn_lo } else { t.face_lo });
        p.outline(x, y, 8, 43, t.edge);
        for (i, ch) in "oaidv".chars().enumerate() {
            let gy = [3, 11, 18, 25, 33][i] + 1;
            if let Some(g) = glyph(ch) {
                p.art(x + 2, y + gy, &g, if lit { t.accent } else { t.face_ink });
            }
        }
    };
    clutter(&mut p, 304, 0, false);
    for k in 0..5 {
        clutter(&mut p, 304 + k * 8, 44, true);
    }
    p
}

fn main_window(t: &Th) -> Pic {
    let mut p = Pic::new(275, 116, t.face);
    panel(&mut p, t, 0, 0, 275, 116);
    // the clock and visualizer display, the title marquee, and the bitrate boxes
    lcd_box(&mut p, t, 21, 23, 84, 38);
    p.fill(72, 30, 2, 2, t.display_ink);
    p.fill(72, 35, 2, 2, t.display_ink);
    lcd_box(&mut p, t, 109, 23, 157, 8);
    lcd_box(&mut p, t, 109, 42, 17, 8);
    lcd_box(&mut p, t, 154, 42, 12, 8);
    p.text(129, 43, "KBPS", t.face_ink);
    p.text(169, 43, "KHZ", t.face_ink);
    // the slider wells, the seek groove and the button bay
    p.bevel(106, 56, 70, 15, t.face_lo, t.face_hi);
    p.bevel(176, 56, 40, 15, t.face_lo, t.face_hi);
    p.bevel(15, 71, 250, 12, t.face_lo, t.face_hi);
    p.bevel(14, 86, 118, 22, t.face_lo, t.face_hi);
    // Backline's gem in the corner, where Winamp has its lightning bolt
    let gem = ["...#...", "..###..", ".#####.", "#######", ".#####.", "..###..", "...#..."];
    p.art(250, 91, &gem, t.accent);
    p.art(251, 92, &["..#..", ".###.", "#####", ".###.", "..#.."], lighter(t.accent, 0.4));
    p
}

fn cbuttons(t: &Th) -> Pic {
    let mut p = Pic::new(136, 36, t.face);
    let prev = ["#..#..#", "#.##.##", "#######", "#.##.##", "#..#..#"];
    let play = ["#...", "##..", "###.", "####", "###.", "##..", "#..."];
    let pause = ["##.##", "##.##", "##.##", "##.##", "##.##", "##.##", "##.##"];
    let stop = ["######", "######", "######", "######", "######", "######"];
    let next = ["#..#..#", "##.##.#", "#######", "##.##.#", "#..#..#"];
    let eject = ["...#...", "..###..", ".#####.", "#######", ".......", "#######"];
    let icons: [(&[&str], i32, i32); 5] = [(&prev, 0, 23), (&play, 23, 23), (&pause, 46, 23), (&stop, 69, 23), (&next, 92, 22)];
    for down in [false, true] {
        let y = if down { 18 } else { 0 };
        let d = if down { 1 } else { 0 };
        for (pic, x, w) in icons {
            button(&mut p, t, x, y, w, 18, down);
            let (iw, ih) = (pic[0].len() as i32, pic.len() as i32);
            p.art(x + (w - iw) / 2 + d, y + (18 - ih) / 2 + d, pic, t.btn_ink);
        }
        let y = if down { 16 } else { 0 };
        button(&mut p, t, 114, y, 22, 16, down);
        p.art(114 + 8 + d, y + 5 + d, &eject, t.btn_ink);
    }
    p
}

fn shufrep(t: &Th) -> Pic {
    let mut p = Pic::new(92, 85, t.face);
    for (row, (on, down)) in [(false, false), (false, true), (true, false), (true, true)].into_iter().enumerate() {
        let y = row as i32 * 15;
        label_button(&mut p, t, 0, y, 28, 15, down, "REP", Some(on));
        label_button(&mut p, t, 28, y, 47, 15, down, "SHUFFLE", Some(on));
    }
    for (x, label, down) in [(0, "EQ", false), (23, "PL", false), (46, "EQ", true), (69, "PL", true)] {
        label_button(&mut p, t, x, 61, 23, 12, down, label, Some(false));
        label_button(&mut p, t, x, 73, 23, 12, down, label, Some(true));
    }
    p
}

fn playpaus(t: &Th) -> Pic {
    let mut p = Pic::new(42, 9, t.display);
    p.art(2, 1, &["#...", "##..", "###.", "####", "###.", "##..", "#..."], t.display_ink);
    p.art(11, 1, &["##.##", "##.##", "##.##", "##.##", "##.##", "##.##", "##.##"], t.display_ink);
    p.fill(19, 1, 7, 7, t.display_ink);
    p
}

fn monoster(t: &Th) -> Pic {
    let mut p = Pic::new(58, 24, t.display);
    // in narrow letters, each word in the middle of its own slot (STEREO 29 wide, MONO 27), so the two sit evenly
    for (y, c) in [(0, t.display_ink), (12, t.display_dim)] {
        small_text(&mut p, (29 - small_w("STEREO")) / 2, y + 3, "STEREO", c);
        small_text(&mut p, 29 + (27 - small_w("MONO")) / 2, y + 3, "MONO", c);
    }
    p
}

/// The narrow letters of MONO and STEREO (as Winamp's own lights have them), each its own width.
fn small_glyph(c: char) -> &'static [&'static str] {
    match c {
        'S' => &[".##", "#..", ".#.", "..#", "##."],
        'T' => &["###", ".#.", ".#.", ".#.", ".#."],
        'E' => &["###", "#..", "##.", "#..", "###"],
        'R' => &["##.", "#.#", "##.", "#.#", "#.#"],
        'O' => &[".#.", "#.#", "#.#", "#.#", ".#."],
        'M' => &["#...#", "##.##", "#.#.#", "#...#", "#...#"],
        'N' => &["#..#", "##.#", "#.##", "#..#", "#..#"],
        _ => &[],
    }
}

/// Narrow letters, a pixel between them.
fn small_text(p: &mut Pic, x: i32, y: i32, s: &str, c: Color32) {
    let mut x = x;
    for ch in s.chars() {
        let g = small_glyph(ch);
        p.art(x, y, g, c);
        x += g.first().map_or(0, |r| r.len() as i32) + 1;
    }
}

fn small_w(s: &str) -> i32 {
    s.chars().map(|c| small_glyph(c).first().map_or(0, |r| r.len() as i32) + 1).sum::<i32>() - 1
}

/// volume.bmp / balance.bmp: 28 level pictures 15 pixels apart, then the knob (pressed, normal) at the bottom.
fn slider_sheet(t: &Th, balance: bool) -> Pic {
    let mut p = Pic::new(68, 433, t.face);
    for i in 0..28 {
        let y = i * 15;
        let f = i as f32 / 27.0;
        let (x0, w) = if balance { (9, 38) } else { (0, 68) };
        p.fill(x0, y, w, 13, t.face);
        let gy = y + 4;
        p.fill(x0 + 2, gy, w - 4, 5, t.display);
        p.bevel(x0 + 1, gy - 1, w - 2, 7, t.face_lo, t.face_hi);
        let lit = mix(mix(t.display, t.accent, 0.35), t.accent, f);
        if balance {
            // a centred bar that warms as the balance leaves the middle
            p.fill(x0 + 2, gy + 1, w - 4, 3, mix(t.display_dim, lit, 0.3 + 0.7 * f));
        } else {
            let n = ((w - 4) as f32 * f).round() as i32;
            for k in 0..n {
                let c = mix(mix(t.display, t.accent, 0.3), t.accent, k as f32 / (w - 4) as f32);
                p.fill(x0 + 2 + k, gy + 1, 1, 3, c);
            }
        }
    }
    for (x, down) in [(0, true), (15, false)] {
        button(&mut p, t, x, 422, 14, 11, down);
        for k in 0..3 {
            p.fill(x + 4 + k * 2, 425, 1, 5, if down { t.btn_lo } else { t.btn_ink });
        }
    }
    p
}

fn posbar(t: &Th) -> Pic {
    let mut p = Pic::new(307, 10, t.face);
    p.fill(0, 0, 248, 10, t.face);
    p.fill(1, 3, 246, 4, t.display);
    p.bevel(0, 2, 248, 6, t.face_lo, t.face_hi);
    for (x, down) in [(248, false), (278, true)] {
        button(&mut p, t, x, 0, 29, 10, down);
        for k in 0..4 {
            p.fill(x + 10 + k * 3, 3, 1, 4, if down { t.accent } else { t.btn_ink });
        }
    }
    p
}

fn eqmain(t: &Th) -> Pic {
    let mut p = Pic::new(275, 315, t.face);
    panel(&mut p, t, 0, 0, 275, 116);
    // labels: the dB scale, PREAMP and the bands
    p.text(56, 39, "+12", t.face_ink);
    p.text(61, 66, "0", t.face_ink);
    p.text(56, 92, "-12", t.face_ink);
    p.text(14, 106, "PREAMP", t.face_ink);
    for (i, hz) in ["31", "62", "125", "250", "500", "1K", "2K", "4K", "8K", "16K"].iter().enumerate() {
        let cx = 78 + i as i32 * 18 + 7;
        p.text(cx - text_w(hz) / 2, 106, hz, t.face_ink);
    }
    p.bevel(85, 16, 115, 21, t.face_lo, t.face_hi);
    // the title strip, with its close button; and the close button pressed
    title_strip(&mut p, t, 0, 134, 275, 14, "EQUALIZER");
    tiny_button(&mut p, t, 264, 137, &PIC_CLOSE, false);
    p.fill(0, 116, 275, 18, t.face);
    tiny_button(&mut p, t, 0, 125, &PIC_CLOSE, true);
    // ON and AUTO, off then on
    label_button(&mut p, t, 10, 119, 26, 12, false, "ON", Some(false));
    label_button(&mut p, t, 36, 119, 32, 12, false, "AUTO", Some(false));
    label_button(&mut p, t, 69, 119, 26, 12, false, "ON", Some(true));
    label_button(&mut p, t, 95, 119, 32, 12, false, "AUTO", Some(true));
    // PRESETS, normal and pressed
    label_button(&mut p, t, 224, 164, 44, 12, false, "PRESETS", None);
    label_button(&mut p, t, 224, 176, 44, 12, true, "PRESETS", None);
    // the slider knob, normal and pressed
    for (y, down) in [(164, false), (176, true)] {
        button(&mut p, t, 0, y, 11, 11, down);
        p.fill(2, y + 5, 7, 1, if down { t.accent } else { t.btn_ink });
    }
    // the 28 slider pictures: a groove whose fill warms with the level
    for n in 0..28 {
        let (x, y) = (13 + (n % 14) * 15, 164 + (n / 14) * 65);
        p.fill(x, y, 14, 63, t.face);
        let f = n as f32 / 27.0;
        p.fill(x + 5, y + 2, 4, 59, t.display);
        p.bevel(x + 4, y + 1, 6, 61, t.face_lo, t.face_hi);
        let c = mix(mix(t.display, t.accent, 0.25), t.accent, (f - 0.5).abs() * 2.0);
        let fill_top = y + 2 + ((1.0 - f) * 58.0) as i32;
        let mid = y + 31;
        let (a, b) = if fill_top < mid { (fill_top, mid) } else { (mid, fill_top) };
        p.fill(x + 6, a, 2, (b - a).max(1), c);
    }
    // the curve's box (a centre line on the display), and its colours down one column
    p.fill(0, 294, 113, 19, t.display);
    for x in (0..113).step_by(2) {
        p.dot(x, 294 + 9, t.display_dim);
    }
    for y in 0..19 {
        let f = y as f32 / 18.0;
        let c = mix(lighter(t.accent, 0.3), t.accent, f);
        p.dot(115, 294 + y, c);
    }
    p
}

fn pledit(t: &Th) -> Pic {
    let mut p = Pic::new(280, 186, t.face);
    // the title bar: corners, the title piece and the plain tile, focused (top row) and not (second row)
    for row in [0, 21] {
        let strip = |p: &mut Pic, x: i32, w: i32| p.fill(x, row, w, 20, t.face);
        strip(&mut p, 0, 25);
        strip(&mut p, 26, 100);
        let title = "PLAYLIST";
        let tx = 26 + (100 - text_w(title)) / 2;
        p.fill(tx - 4, row + 4, text_w(title) + 8, 12, t.display);
        p.text(tx, row + 7, title, t.display_ink);
        strip(&mut p, 127, 25);
        strip(&mut p, 153, 25);
        tiny_button(&mut p, t, 153 + 4, row + 3, &PIC_SHADE, false);
        tiny_button(&mut p, t, 153 + 14, row + 3, &PIC_CLOSE, false);
    }
    // the sides: left (12 wide), right (20 wide, with the scroll groove)
    // (flat, as the title strips: the frame is one colour all round)
    p.fill(0, 42, 12, 29, t.face);
    p.fill(31, 42, 20, 29, t.face);
    p.fill(36, 42, 8, 29, t.display);
    // the scroll handle, normal and pressed
    button(&mut p, t, 52, 53, 8, 18, false);
    button(&mut p, t, 61, 53, 8, 18, true);
    for (x, c) in [(52, t.btn_ink), (61, t.accent)] {
        for k in 0..3 {
            p.fill(x + 2, 53 + 7 + k * 2, 4, 1, c);
        }
    }
    // the bottom: the plain tile (its last rows also edge every other window), then the two end pieces
    let bottom = |p: &mut Pic, x: i32, y: i32, w: i32| p.fill(x, y, w, 38, t.face);
    bottom(&mut p, 179, 0, 25);
    bottom(&mut p, 0, 72, 125);
    for (i, label) in ["ADD", "REM", "SEL", "MISC"].iter().enumerate() {
        label_button(&mut p, t, 14 + i as i32 * 29, 72 + 8, 22, 18, false, label, None);
    }
    bottom(&mut p, 126, 72, 150);
    // running time, the mini clock, the mini transport and LIST
    lcd_box(&mut p, t, 126 + 5, 72 + 8, 78, 9);
    lcd_box(&mut p, t, 126 + 64, 72 + 21, 27, 9);
    let minis: [&[&str]; 6] = [
        &["#..#", "#.##", "####", "#.##", "#..#"],
        &["#..", "##.", "###", "##.", "#.."],
        &["#.#", "#.#", "#.#", "#.#", "#.#"],
        &["###", "###", "###", "###", "###"],
        &["#..#", "##.#", "####", "##.#", "#..#"],
        &[".#.", "###", "...", "###", "..."],
    ];
    for (n, pic) in minis.iter().enumerate() {
        let x = 126 + 3 + n as i32 * 10;
        p.art(x + (10 - pic[0].len() as i32) / 2, 72 + 24, pic, t.face_ink);
    }
    label_button(&mut p, t, 126 + 106, 72 + 8, 22, 18, false, "LIST", None);
    // a resize grip in the corner
    for k in 0..3 {
        for j in 0..=k {
            p.dot(126 + 146 - k * 3 + j * 3, 72 + 33 - j * 3, t.face_lo);
        }
    }
    // the little visualizer panel
    bottom(&mut p, 205, 0, 75);
    lcd_box(&mut p, t, 205 + 3, 12, 72, 16);
    p
}

// ------------------------------------------------------------------ the file

fn hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

/// A complete skin (.wsz) from a look `p` (its panels, display and accent).
/// `text`: the lists' writing (your Custom look's), else the display's accent softened.
pub fn make(p: &Pal, text: Option<Color32>) -> Vec<u8> {
    let t = theme(p);
    let mut files: Vec<(&str, Vec<u8>)> = vec![
        ("main.bmp", main_window(&t).bmp()),
        ("titlebar.bmp", titlebar(&t).bmp()),
        ("cbuttons.bmp", cbuttons(&t).bmp()),
        ("numbers.bmp", numbers(&t).bmp()),
        ("text.bmp", text_sheet(&t).bmp()),
        ("posbar.bmp", posbar(&t).bmp()),
        ("volume.bmp", slider_sheet(&t, false).bmp()),
        ("balance.bmp", slider_sheet(&t, true).bmp()),
        ("shufrep.bmp", shufrep(&t).bmp()),
        ("playpaus.bmp", playpaus(&t).bmp()),
        ("monoster.bmp", monoster(&t).bmp()),
        ("eqmain.bmp", eqmain(&t).bmp()),
        ("pledit.bmp", pledit(&t).bmp()),
    ];
    // the list colours: the display's (its words a little quieter than the playing song's)
    let quiet = if t.light { Color32::from_gray(40) } else { Color32::from_gray(200) };
    let normal = text.unwrap_or_else(|| mix(t.display_ink, quiet, 0.5));
    let (current, bg, sel) = (t.display_ink, t.display, mix(t.display, t.accent, 0.3));
    let pl = format!(
        "[Text]\r\nNormal={}\r\nCurrent={}\r\nNormalBG={}\r\nSelectedBG={}\r\nFont={}\r\n",
        hex(normal),
        hex(current),
        hex(bg),
        hex(sel),
        // Arial, Winamp's own (the deck keeps the skin's pixel font)
        "Arial"
    );
    files.push(("pledit.txt", pl.into_bytes()));
    // visualizer: background, dots, 16 bar colours (top to bottom), 5 for the wave, the peaks
    let mut vis = vec![t.display, t.display_dim];
    for i in 0..16 {
        let f = i as f32 / 15.0;
        vis.push(mix(lighter(t.accent, 0.35), darker(t.accent, 0.35), f));
    }
    for i in 0..5 {
        vis.push(mix(t.display_ink, t.display, i as f32 * 0.12));
    }
    vis.push(t.display_ink);
    let vc: String = vis.iter().map(|c| format!("{},{},{},\r\n", c.r(), c.g(), c.b())).collect();
    files.push(("viscolor.txt", vc.into_bytes()));

    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        for (name, data) in &files {
            if z.start_file(*name, zip::write::SimpleFileOptions::default()).is_ok() {
                let _ = z.write_all(data);
            }
        }
        let _ = z.finish();
    }
    buf.into_inner()
}

/// Backline's skin in a downloaded skin's colours (graphite tinted with them, its accent on the display), for the
/// pictures that skin leaves out (winamp.rs borrows them, as Winamp borrows from its base skin).
pub fn borrowed(p: &Pal) -> Vec<u8> {
    let base = mix(Color32::from_gray(34), p.btn_face, 0.16);
    let lcd = mix(Color32::from_gray(8), p.btn_face, 0.06);
    let mut glow = if lum(p.btn_hi) > lum(p.accent) { p.btn_hi } else { p.accent };
    if lum(glow) < 0.55 {
        glow = lighter(glow, 0.35);
    }
    make(&crate::skin::look_pal(base, lcd, glow), None)
}

/// A look worn as a skin: the skin, and the palette the rest of the app reads from it (as from any downloaded skin).
pub fn wear(p: &Pal) -> Result<(Pal, crate::winamp::Art), String> {
    wear_with(p, None)
}

/// A look worn with its own writing colour for the lists (your Custom look's), when it has one.
pub fn wear_with(p: &Pal, text: Option<Color32>) -> Result<(Pal, crate::winamp::Art), String> {
    crate::winamp::load(&make(p, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_look_makes_a_whole_skin() {
        for (n, pal) in crate::skin::PALS.iter().enumerate() {
            let (_, art) = wear(pal).unwrap_or_else(|e| panic!("look {}: {}", n, e));
            for sheet in [
                "main", "cbuttons", "titlebar", "numbers", "text", "posbar", "volume", "balance", "shufrep", "playpaus",
                "monoster", "eqmain", "pledit",
            ] {
                assert!(art.sheets.iter().any(|s| s.0 == sheet), "look {} lacks {}", n, sheet);
            }
            assert!(art.vis.len() >= 24 && art.graph.len() == 19);
            assert_eq!(art.font.as_deref(), Some("Arial"));
        }
    }

    #[test]
    fn display_and_button_text_read() {
        for (n, pal) in crate::skin::PALS.iter().enumerate() {
            let t = theme(pal);
            assert!(crate::winamp::contrast(t.display_ink, t.display) >= 4.5, "look {} display text", n);
            assert!(crate::winamp::contrast(t.btn_ink, t.btn) >= 3.0, "look {} button text", n);
            assert!(crate::winamp::contrast(t.face_ink, t.face) >= 3.0, "look {} panel text", n);
        }
    }

    #[test]
    fn every_character_has_a_picture() {
        for ch in "abcdefghijklmnopqrstuvwxyz0123456789.:()-'!_+\\/[]^&%,=$#?*\"@…".chars() {
            assert!(glyph(ch).is_some(), "{}", ch);
        }
    }
}
