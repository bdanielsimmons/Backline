//! Winamp skins drawn for real (shared by the desktop and the phone app): the worn skin's pictures as textures,
//! its window frames, and the skin's fonts (the playlist font and the smooth text the skins use).

use super::*;

/// The worn skin's pictures as textures, with its visualizer and equalizer-curve colours.
pub(crate) struct WaTex {
    pub(crate) sheets: HashMap<String, (egui::TextureHandle, Vec2)>,
    pub(crate) vis: Vec<Color32>,
    pub(crate) graph: Vec<Color32>,
    /// the playlist font the skin asks for
    pub(crate) font: Option<String>,
    /// the EQ and PL buttons need help to be seen: outlined and lettered in this colour
    pub(crate) eqpl_ink: Option<Color32>,
    /// the balance picture is the volume one: drawn following the knob
    pub(crate) bal_is_vol: bool,
    /// the balance pictures that make a smooth run (empty: all)
    pub(crate) bal_frames: Vec<usize>,
}

impl WaTex {
    pub(crate) fn new(ctx: &egui::Context, art: crate::winamp::Art) -> WaTex {
        let mut sheets = HashMap::new();
        for (name, size, rgba) in art.sheets {
            let img = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
            let tex = ctx.load_texture(format!("winamp_{}", name), img, egui::TextureOptions::NEAREST);
            sheets.insert(name, (tex, Vec2::new(size[0] as f32, size[1] as f32)));
        }
        WaTex {
            sheets,
            vis: art.vis,
            graph: art.graph,
            font: art.font,
            eqpl_ink: art.eqpl_ink,
            bal_is_vol: art.bal_is_vol,
            bal_frames: art.bal_frames,
        }
    }

    pub(crate) fn size(&self, sheet: &str) -> Option<Vec2> {
        self.sheets.get(sheet).map(|s| s.1)
    }

    /// One sprite (x, y, w, h in the sheet) into `dst`. False when the skin lacks that sheet.
    pub(crate) fn spr(&self, p: &egui::Painter, sheet: &str, src: (f32, f32, f32, f32), dst: Rect) -> bool {
        self.spr_flip(p, sheet, src, dst, false)
    }

    /// A sprite, mirrored left to right if `flip`.
    pub(crate) fn spr_flip(&self, p: &egui::Painter, sheet: &str, src: (f32, f32, f32, f32), dst: Rect, flip: bool) -> bool {
        let Some((tex, size)) = self.sheets.get(sheet) else { return false };
        // a sheet drawn smaller than Winamp's: only the part of the picture that is in it (drawing past its edge
        // would smear the edge pixels); mostly missing, nothing - the caller draws a plain stand-in
        let (w2, h2) = ((size.x - src.0).min(src.2), (size.y - src.1).min(src.3));
        if w2 <= 0.0 || h2 <= 0.0 || w2 * h2 < 0.5 * src.2 * src.3 {
            return false;
        }
        let dst = Rect::from_min_size(dst.min, Vec2::new(dst.width() * w2 / src.2, dst.height() * h2 / src.3));
        let src = (src.0, src.1, w2, h2);
        // pull the edges in by a hair: sprites sit side by side in a sheet (the digits 0 to 9...), and sampling
        // right on the border would pick up a sliver of the neighbour
        const E: f32 = 0.02;
        let uv = Rect::from_min_max(
            Pos2::new((src.0 + E) / size.x, (src.1 + E) / size.y),
            Pos2::new((src.0 + src.2 - E) / size.x, (src.1 + src.3 - E) / size.y),
        );
        let uv = if flip { Rect::from_min_max(Pos2::new(uv.max.x, uv.min.y), Pos2::new(uv.min.x, uv.max.y)) } else { uv };
        p.image(tex.id(), dst, uv, Color32::WHITE);
        true
    }
}

/// A plain stand-in for a control the skin has no picture for: a small plate in the skin's colours with a label,
/// lit when on.
pub(crate) fn plate(p: &egui::Painter, r: Rect, text: &str, lit: bool) {
    let pal = crate::skin::pal();
    let (bg, fg) = if lit { (pal.ink, pal.lcd) } else { (pal.lcd.gamma_multiply(0.9), pal.ink) };
    p.rect_filled(r, 1.0, bg);
    p.rect_stroke(r, 1.0, Stroke::new(1.0_f32, pal.ink.gamma_multiply(0.8)));
    if !text.is_empty() {
        p.text(r.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::monospace((r.height() * 0.62).max(7.0)), fg);
    }
}

/// Winamp's 275 x 116 window placed on screen: `o` the top-left corner, `s` screen points per skin pixel.
#[derive(Clone, Copy)]
pub(crate) struct Place {
    pub(crate) o: Pos2,
    pub(crate) s: f32,
}

impl Place {
    pub(crate) fn fit(area: Rect) -> Place {
        let ppp = font::ppp();
        let fit = (area.width() / 275.0).min(area.height() / 116.0);
        // whole screen pixels per skin pixel, so the art stays crisp; in a panel too small for even one, it shrinks
        // to fit rather than spilling over the edges
        let s = if fit * ppp >= 1.0 { (fit * ppp).floor() / ppp } else { fit.max(0.2) };
        // the corner on a whole screen pixel too, so every skin pixel lands exactly on screen pixels
        let snap = |v: f32| (v * ppp).round() / ppp;
        Place { o: Pos2::new(snap(area.center().x - 137.5 * s), snap(area.center().y - 58.0 * s)), s }
    }
    pub(crate) fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.o + Vec2::new(x, y) * self.s, Vec2::new(w, h) * self.s)
    }
}

/// text.bmp: which cell (row, column) each character is in; cells are 5 x 6.
pub(crate) const FONT: &[(char, u8, u8)] = &[
    ('a', 0, 0),
    ('b', 0, 1),
    ('c', 0, 2),
    ('d', 0, 3),
    ('e', 0, 4),
    ('f', 0, 5),
    ('g', 0, 6),
    ('h', 0, 7),
    ('i', 0, 8),
    ('j', 0, 9),
    ('k', 0, 10),
    ('l', 0, 11),
    ('m', 0, 12),
    ('n', 0, 13),
    ('o', 0, 14),
    ('p', 0, 15),
    ('q', 0, 16),
    ('r', 0, 17),
    ('s', 0, 18),
    ('t', 0, 19),
    ('u', 0, 20),
    ('v', 0, 21),
    ('w', 0, 22),
    ('x', 0, 23),
    ('y', 0, 24),
    ('z', 0, 25),
    ('"', 0, 26),
    ('@', 0, 27),
    (' ', 0, 30),
    ('0', 1, 0),
    ('1', 1, 1),
    ('2', 1, 2),
    ('3', 1, 3),
    ('4', 1, 4),
    ('5', 1, 5),
    ('6', 1, 6),
    ('7', 1, 7),
    ('8', 1, 8),
    ('9', 1, 9),
    ('…', 1, 10),
    ('.', 1, 11),
    (':', 1, 12),
    ('(', 1, 13),
    (')', 1, 14),
    ('-', 1, 15),
    ('\'', 1, 16),
    ('!', 1, 17),
    ('_', 1, 18),
    ('+', 1, 19),
    ('\\', 1, 20),
    ('/', 1, 21),
    ('[', 1, 22),
    (']', 1, 23),
    ('^', 1, 24),
    ('&', 1, 25),
    ('%', 1, 26),
    (',', 1, 27),
    ('=', 1, 28),
    ('$', 1, 29),
    ('#', 1, 30),
    ('Å', 2, 0),
    ('Ö', 2, 1),
    ('Ä', 2, 2),
    ('?', 2, 3),
    ('*', 2, 4),
    ('<', 1, 22),
    ('>', 1, 23),
    ('{', 1, 22),
    ('}', 1, 23),
];

pub(crate) fn glyph(c: char) -> (u8, u8) {
    // the skin's font has plain letters only: Shōga is drawn as Shoga
    let c = crate::font::plain(c).to_ascii_lowercase();
    FONT.iter().find(|f| f.0 == c).map_or((0, 30), |f| (f.1, f.2))
}

/// Text in the skin's own font, from skin pixel (x, y).
pub(crate) fn wa_text(t: &WaTex, p: &egui::Painter, pl: Place, x: f32, y: f32, s: &str) {
    for (i, c) in s.chars().enumerate() {
        let (row, col) = glyph(c);
        t.spr(p, "text", (col as f32 * 5.0, row as f32 * 6.0, 5.0, 6.0), pl.r(x + i as f32 * 5.0, y, 5.0, 6.0));
    }
}

// ---------------------------------------------------------------- window frames
// With a skin on, every Backline window wears the skin's own frame (the pieces of its playlist window) and its
// title in the skin's font. Kept here for the free function that draws window frames.
thread_local! {
    static CHROME: std::cell::RefCell<Option<((egui::TextureHandle, Vec2), Option<(egui::TextureHandle, Vec2)>)>> =
        const { std::cell::RefCell::new(None) };
}

/// Use this skin's frames for every window (None: Backline's own again).
pub(crate) fn set_chrome(t: Option<&WaTex>) {
    let v = t.and_then(|t| Some((t.sheets.get("pledit")?.clone(), t.sheets.get("text").cloned())));
    CHROME.with(|c| *c.borrow_mut() = v);
}

/// A window's frame and title from the worn skin. False when no skin is worn (Backline draws its own).
pub(crate) fn draw_frame(p: &egui::Painter, outer: Rect, inner: Rect, title: &str) -> bool {
    CHROME.with(|c| {
        let c = c.borrow();
        let Some(((tex, size), text)) = c.as_ref() else { return false };
        let piece = |p: &egui::Painter, tex: &egui::TextureHandle, size: Vec2, src: (f32, f32, f32, f32), dst: Rect| {
            const E: f32 = 0.02;
            let uv = Rect::from_min_max(
                Pos2::new((src.0 + E) / size.x, (src.1 + E) / size.y),
                Pos2::new((src.0 + src.2 - E) / size.x, (src.1 + src.3 - E) / size.y),
            );
            p.image(tex.id(), dst, uv, Color32::WHITE);
        };
        let top_h = inner.min.y - outer.min.y;
        let k = top_h / 20.0; // the skin's 20-pixel title bar fills our title strip
        let tw = 25.0 * k;
        // the title bar: corners at the ends, tiles between
        let top = Rect::from_min_max(outer.min, Pos2::new(outer.max.x, inner.min.y));
        let clip = p.with_clip_rect(top);
        let mut x = outer.min.x + tw;
        while x < outer.max.x - tw {
            piece(
                &clip,
                tex,
                *size,
                (127.0, 0.0, 25.0, 20.0),
                Rect::from_min_size(Pos2::new(x, top.min.y), Vec2::new(tw, top_h)),
            );
            x += tw;
        }
        piece(p, tex, *size, (0.0, 0.0, 25.0, 20.0), Rect::from_min_size(top.min, Vec2::new(tw, top_h)));
        // the right end is a plain piece: the skin's own corner has minimize / close buttons, which would do
        // nothing on these windows
        piece(
            p,
            tex,
            *size,
            (127.0, 0.0, 25.0, 20.0),
            Rect::from_min_size(Pos2::new(outer.max.x - tw, top.min.y), Vec2::new(tw, top_h)),
        );
        // the sides: the skin's edge tiles, repeated down
        let (lw, rw, bh) = (inner.min.x - outer.min.x, outer.max.x - inner.max.x, outer.max.y - inner.max.y);
        for (x0, w, src) in [(outer.min.x, lw, (0.0, 42.0, 12.0, 29.0)), (inner.max.x, rw, (39.0, 42.0, 12.0, 29.0))] {
            let col = Rect::from_min_max(Pos2::new(x0, inner.min.y), Pos2::new(x0 + w, inner.max.y));
            let clip = p.with_clip_rect(col);
            let th = 29.0 * (w / 12.0);
            let mut y = col.min.y;
            while y < col.max.y {
                piece(&clip, tex, *size, src, Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, th)));
                y += th;
            }
        }
        // the bottom: the lowest rows of the skin's bottom tile, repeated across
        let rows = (bh / k).clamp(4.0, 38.0);
        let bot = Rect::from_min_max(Pos2::new(outer.min.x, inner.max.y), outer.max);
        let clip = p.with_clip_rect(bot);
        let mut x = outer.min.x;
        while x < outer.max.x {
            piece(
                &clip,
                tex,
                *size,
                (179.0, 38.0 - rows, 25.0, rows),
                Rect::from_min_size(Pos2::new(x, bot.min.y), Vec2::new(tw, bh)),
            );
            x += tw;
        }
        bottom_edge(p, outer, bot.min.y);
        // the title on a dark plate in light text, so it reads over any decoration in any skin
        let size = top_h * 0.46;
        let label = title.to_uppercase();
        let tw2 = sans_width(&label, size);
        let at = Pos2::new(outer.min.x + tw + 6.0, top.center().y);
        p.rect_filled(
            Rect::from_min_max(
                Pos2::new(at.x - 5.0, top.min.y + top_h * 0.18),
                Pos2::new(at.x + tw2 + 5.0, top.max.y - top_h * 0.18),
            ),
            2.0,
            Color32::from_black_alpha(190),
        );
        sans_text(p, at, Align::Min, &label, size, outer.width() * 0.5, Color32::from_gray(235));
        round_corners(p, outer, true);
        let _ = text;
        true
    })
}

/// Round a window's top (or bottom) corners: outside each quarter circle the app's background again, a dark edge
/// along the curve - so windows end softly and meet cleanly.
pub(crate) fn round_corners(p: &egui::Painter, outer: Rect, top: bool) {
    let r = 7.0_f32;
    let bg = crate::pal().app_bg;
    let cy = if top { outer.min.y + r } else { outer.max.y - r };
    let dir = if top { -1.0 } else { 1.0 };
    let arc = |cx: f32, side: f32| -> Vec<Pos2> {
        (0..=8)
            .map(|i| {
                let a = i as f32 / 8.0 * std::f32::consts::FRAC_PI_2;
                Pos2::new(cx + side * r * a.cos(), cy + dir * r * a.sin())
            })
            .collect()
    };
    let mut mesh = egui::Mesh::default();
    for (cx, side) in [(outer.min.x + r, -1.0_f32), (outer.max.x - r, 1.0)] {
        let corner = Pos2::new(cx + side * r, if top { outer.min.y } else { outer.max.y });
        for w in arc(cx, side).windows(2) {
            let i = mesh.vertices.len() as u32;
            for q in [corner, w[0], w[1]] {
                mesh.colored_vertex(q, bg);
            }
            mesh.add_triangle(i, i + 1, i + 2);
        }
    }
    p.add(egui::Shape::mesh(mesh));
    for (cx, side) in [(outer.min.x + r, -1.0_f32), (outer.max.x - r, 1.0)] {
        p.add(egui::Shape::line(arc(cx, side), egui::Stroke::new(1.5_f32, Color32::from_black_alpha(220))));
    }
}

/// The bottom of a window, finished like its top: a highlight where the frame's floor begins, the two bottom
/// corners rounded off (outside the curve is the app's background again) and a dark edge around them, so the window
/// ends on a clear line whatever colour the skin's frame is.
pub(crate) fn bottom_edge(p: &egui::Painter, outer: Rect, band_top: f32) {
    let r = 7.0_f32;
    p.line_segment(
        [Pos2::new(outer.min.x + 2.0, band_top + 0.5), Pos2::new(outer.max.x - 2.0, band_top + 0.5)],
        egui::Stroke::new(1.0_f32, Color32::from_white_alpha(30)),
    );
    let bg = crate::pal().app_bg;
    let cy = outer.max.y - r;
    let arc = |cx: f32, side: f32| -> Vec<Pos2> {
        (0..=8)
            .map(|i| {
                let a = i as f32 / 8.0 * std::f32::consts::FRAC_PI_2;
                Pos2::new(cx + side * r * a.cos(), cy + r * a.sin())
            })
            .collect()
    };
    let mut mesh = egui::Mesh::default();
    for (cx, side) in [(outer.min.x + r, -1.0_f32), (outer.max.x - r, 1.0)] {
        let corner = Pos2::new(cx + side * r, outer.max.y);
        let pts = arc(cx, side);
        for w in pts.windows(2) {
            let i = mesh.vertices.len() as u32;
            for q in [corner, w[0], w[1]] {
                mesh.colored_vertex(q, bg);
            }
            mesh.add_triangle(i, i + 1, i + 2);
        }
    }
    p.add(egui::Shape::mesh(mesh));
    // the edge: up the left corner, along the bottom, up the right corner
    let mut edge: Vec<Pos2> = arc(outer.min.x + r, -1.0);
    edge.extend(arc(outer.max.x - r, 1.0).into_iter().rev());
    p.add(egui::Shape::line(edge, egui::Stroke::new(1.5_f32, Color32::from_black_alpha(220))));
}

/// Winamp's own visualizer, 76 x 16 skin pixels from `o` at `s` points per pixel, in the skin's colours
/// (viscolor.txt): 19 bars (2 = their top colour ... 17 = the bottom, 23 = the peaks), the oscilloscope (18 to
/// 22, brightest at the centre line), or both. `mode`: 0 bars, 1 wave, 2 both.
#[allow(clippy::too_many_arguments)]
pub(crate) fn skin_viz(
    p: &egui::Painter,
    o: Pos2,
    s: f32,
    vis: &[Color32],
    bands: &[f32],
    peaks: &[f32],
    wave: &[f32],
    mode: u8,
) {
    if vis.len() < 24 {
        return;
    }
    let px = |x: f32, y: f32, w: f32, h: f32, c: Color32| {
        p.rect_filled(Rect::from_min_size(o + Vec2::new(x, y) * s, Vec2::new(w, h) * s), 0.0, c);
    };
    if mode != 1 && !bands.is_empty() {
        for b in 0..19usize {
            let i = b * bands.len() / 19;
            let h = (bands[i].clamp(0.0, 1.0) * 16.0).round() as usize;
            for row in 0..h {
                let top = 15 - row;
                px(b as f32 * 4.0, top as f32, 3.0, 1.0, vis[2 + top]);
            }
            if let Some(pk) = peaks.get(i) {
                let ph = (pk.clamp(0.0, 1.0) * 16.0).round() as usize;
                if ph > 0 {
                    px(b as f32 * 4.0, (16 - ph) as f32, 3.0, 1.0, vis[23]);
                }
            }
        }
    }
    if mode != 0 && !wave.is_empty() {
        // one dot per column, joined to the last one, as Winamp's line scope draws
        let y_at = |x: usize| {
            let v = wave[x * wave.len() / 76];
            (7.5 - v.clamp(-1.0, 1.0) * 8.0).round().clamp(0.0, 15.0) as i32
        };
        let mut prev = y_at(0);
        for x in 0..76usize {
            let y = y_at(x);
            let (a, b) = if prev < y { (prev, y) } else { (y, prev) };
            for yy in a..=b {
                let k = ((yy - 8).unsigned_abs() as usize / 2).min(4);
                px(x as f32, yy as f32, 1.0, 1.0, vis[18 + k]);
            }
            prev = y;
        }
    }
}

// ---------------------------------------------------------------- the playlist font
// Winamp's playlist is written in an ordinary font, and each skin names its own (Tahoma, Verdana, Arial...).
// That one is used when the computer has it; the playlist's MISC menu offers others, classic Windows fonts and
// free retro ones (open-licensed, from Google Fonts) fetched once and kept.

pub(crate) enum FontSrc {
    /// the one the skin asks for
    Skin,
    /// a font installed on the computer, by family name
    Sys(&'static str),
    /// fetched once from Google Fonts: (path in google/fonts, the pixel size it is drawn at)
    Web(&'static str, f32),
}

pub(crate) const PL_FONTS: [(&str, FontSrc); 14] = [
    ("THE SKIN'S OWN", FontSrc::Skin),
    ("TAHOMA", FontSrc::Sys("tahoma")),
    ("MS SANS SERIF", FontSrc::Sys("microsoft sans serif")),
    ("VERDANA", FontSrc::Sys("verdana")),
    ("ARIAL", FontSrc::Sys("arial")),
    ("LUCIDA CONSOLE", FontSrc::Sys("lucida console")),
    ("VT323 (TERMINAL)", FontSrc::Web("ofl/vt323/VT323-Regular.ttf", 14.0)),
    ("PIXELIFY SANS", FontSrc::Web("ofl/pixelifysans/PixelifySans%5Bwght%5D.ttf", 12.0)),
    ("SILKSCREEN", FontSrc::Web("ofl/silkscreen/Silkscreen-Regular.ttf", 8.0)),
    ("SHARE TECH MONO", FontSrc::Web("ofl/sharetechmono/ShareTechMono-Regular.ttf", 11.0)),
    ("PRESS START 2P", FontSrc::Web("ofl/pressstart2p/PressStart2P-Regular.ttf", 8.0)),
    ("JERSEY 10", FontSrc::Web("ofl/jersey10/Jersey10-Regular.ttf", 12.0)),
    ("MICRO 5", FontSrc::Web("ofl/micro5/Micro5-Regular.ttf", 10.0)),
    ("LEXEND", FontSrc::Web("ofl/lexend/Lexend%5Bwght%5D.ttf", 12.0)),
];

/// Windows' list of installed fonts: family name (lower case, e.g. "arial narrow", "trebuchet ms italic") to file.
pub(crate) static FONT_MAP: std::sync::OnceLock<HashMap<String, std::path::PathBuf>> = std::sync::OnceLock::new();

/// Whether the list of installed fonts has been read (asking Windows for it takes a moment, so it is done in
/// the background and nothing waits for it on screen).
pub(crate) fn fonts_ready() -> bool {
    FONT_MAP.get().is_some()
}

/// Read the list of installed fonts now (call off the main thread).
pub(crate) fn warm_fonts() {
    let _ = installed_fonts();
}

pub(crate) fn installed_fonts() -> &'static HashMap<String, std::path::PathBuf> {
    FONT_MAP.get_or_init(|| {
        let mut map = HashMap::new();
        if !cfg!(windows) {
            return map;
        }
        // read straight from the registry (not with reg.exe: starting that fails on some PCs)
        for (name, file) in registry_fonts() {
            let file = file.trim();
            let lower = file.to_ascii_lowercase();
            // only outline fonts can be drawn (the old .fon bitmap ones cannot)
            if !(lower.ends_with(".ttf") || lower.ends_with(".otf") || lower.ends_with(".ttc")) {
                continue;
            }
            let path = if file.contains('\\') {
                std::path::PathBuf::from(file)
            } else {
                std::path::Path::new("C:\\Windows\\Fonts").join(file)
            };
            let name = name.trim().trim_end_matches("(TrueType)").trim_end_matches("(OpenType)").trim();
            for n in name.split(" & ") {
                map.entry(n.trim().to_ascii_lowercase()).or_insert_with(|| path.clone());
            }
        }
        map
    })
}

/// The installed fonts as Windows lists them (machine-wide, then yours): (name, file).
#[cfg(windows)]
fn registry_fonts() -> Vec<(String, String)> {
    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(key: isize, sub: *const u16, opts: u32, sam: u32, out: *mut isize) -> i32;
        fn RegEnumValueW(
            key: isize,
            index: u32,
            name: *mut u16,
            name_len: *mut u32,
            reserved: *mut u32,
            kind: *mut u32,
            data: *mut u8,
            data_len: *mut u32,
        ) -> i32;
        fn RegCloseKey(key: isize) -> i32;
    }
    const KEY_READ: u32 = 0x20019;
    // HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER
    let roots = [0x8000_0002u32 as i32 as isize, 0x8000_0001u32 as i32 as isize];
    let sub: Vec<u16> = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts".encode_utf16().chain([0]).collect();
    let mut out = Vec::new();
    for root in roots {
        let mut key: isize = 0;
        if unsafe { RegOpenKeyExW(root, sub.as_ptr(), 0, KEY_READ, &mut key) } != 0 {
            continue;
        }
        let mut name = vec![0u16; 16384];
        let mut data = vec![0u8; 8192];
        for index in 0u32.. {
            let (mut nlen, mut dlen, mut kind) = (name.len() as u32, data.len() as u32, 0u32);
            let r = unsafe {
                RegEnumValueW(
                    key,
                    index,
                    name.as_mut_ptr(),
                    &mut nlen,
                    std::ptr::null_mut(),
                    &mut kind,
                    data.as_mut_ptr(),
                    &mut dlen,
                )
            };
            // 259: no more values
            if r == 259 {
                break;
            }
            // text values only (REG_SZ, REG_EXPAND_SZ); one too long for the buffers is skipped
            if r != 0 || !(kind == 1 || kind == 2) {
                continue;
            }
            let n = String::from_utf16_lossy(&name[..nlen as usize]);
            let wide: Vec<u16> = data[..dlen as usize].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            let f = String::from_utf16_lossy(&wide).trim_end_matches('\0').to_string();
            out.push((n, f));
        }
        unsafe { RegCloseKey(key) };
    }
    out
}

#[cfg(not(windows))]
fn registry_fonts() -> Vec<(String, String)> {
    Vec::new()
}

/// A font installed on this computer, by the name a skin gives (Windows' font list first, then the usual files
/// and the Mac font folders).
pub(crate) fn system_font(name: &str) -> Option<Vec<u8>> {
    let n = name.trim().to_ascii_lowercase();
    if let Some(b) = installed_fonts().get(&n).and_then(|p| std::fs::read(p).ok()) {
        return Some(b);
    }
    let file = match n.as_str() {
        "arial" => "arial.ttf",
        "tahoma" => "tahoma.ttf",
        "verdana" => "verdana.ttf",
        "trebuchet ms" | "trebuchet" => "trebuc.ttf",
        "lucida console" => "lucon.ttf",
        "ms sans serif" | "microsoft sans serif" | "sans serif" => "micross.ttf",
        "courier new" | "courier" => "cour.ttf",
        "times new roman" | "times" => "times.ttf",
        "georgia" => "georgia.ttf",
        "comic sans ms" => "comic.ttf",
        "segoe ui" => "segoeui.ttf",
        "consolas" => "consola.ttf",
        _ => "",
    };
    let squashed = format!("{}.ttf", n.replace(' ', ""));
    let mac = name.trim().to_string();
    let mut tries: Vec<String> = Vec::new();
    for f in [file, squashed.as_str()] {
        if !f.is_empty() {
            tries.push(format!("C:\\Windows\\Fonts\\{}", f));
        }
    }
    tries.push(format!("/System/Library/Fonts/Supplemental/{}.ttf", mac));
    tries.push(format!("/Library/Fonts/{}.ttf", mac));
    tries.iter().find_map(|p| std::fs::read(p).ok())
}

pub(crate) fn font_dir() -> std::path::PathBuf {
    crate::api::config_dir().join("fonts")
}

/// The font for a menu choice when it is at hand (installed, or fetched before). None: it needs fetching
/// (or the skin's font is not on this computer).
pub(crate) fn font_now(choice: usize, skin_font: Option<&str>) -> Option<Vec<u8>> {
    match &PL_FONTS.get(choice)?.1 {
        FontSrc::Skin => skin_font.and_then(system_font).or_else(|| system_font("arial")).or_else(|| system_font("tahoma")),
        FontSrc::Sys(n) => system_font(n),
        FontSrc::Web(path, _) => std::fs::read(font_dir().join(path.rsplit('/').next()?)).ok(),
    }
}

/// Fetch a free font once and keep it (call off the main thread).
pub(crate) fn fetch_font(choice: usize) -> Result<Vec<u8>, String> {
    let FontSrc::Web(path, _) = &PL_FONTS.get(choice).ok_or("no such font")?.1 else { return Err("not a download".into()) };
    let url = format!("https://github.com/google/fonts/raw/main/{}", path);
    let b = reqwest::blocking::get(&url).and_then(|r| r.error_for_status()).and_then(|r| r.bytes()).map_err(|e| e.to_string())?;
    let _ = std::fs::create_dir_all(font_dir());
    let _ = std::fs::write(font_dir().join(path.rsplit('/').next().unwrap_or("font.ttf")), &b);
    Ok(b.to_vec())
}

/// The playlist font, drawn the way Windows drew it for Winamp: the font's own hinting snaps every stroke onto
/// whole pixels (so stems keep one even width instead of breaking up), and each pixel is fully on or off, like
/// Windows text before ClearType. A font's built-in bitmaps for small sizes win when it has them, as on Windows.
/// Lines are drawn straight at the size they show on screen, once, and kept.
/// A font's hand-tuned pixel widths of every letter at one size (its `hdmx` table), when it has them - as Windows
/// spaced small text. None: work them out from the outlines.
fn hdmx_widths<'a>(font: &swash::FontRef<'a>, ppem: u8) -> Option<&'a [u8]> {
    let d = font.table(swash::tag_from_bytes(b"hdmx"))?;
    let n = i16::from_be_bytes([*d.get(2)?, *d.get(3)?]).max(0) as usize;
    let rec = u32::from_be_bytes([*d.get(4)?, *d.get(5)?, *d.get(6)?, *d.get(7)?]) as usize;
    if rec < 3 {
        return None;
    }
    (0..n).map(|i| 8 + i * rec).find(|&o| d.get(o) == Some(&ppem)).and_then(|o| d.get(o + 2..o + rec))
}

pub(crate) struct PixFont {
    pub(crate) bytes: std::sync::Arc<Vec<u8>>,
    /// the font's own size (Winamp's 10 pixels for ordinary fonts, a pixel font's native size)
    pub(crate) px: f32,
    pub(crate) scaler: swash::scale::ScaleContext,
    pub(crate) cache: HashMap<(String, u32), (egui::TextureHandle, Vec2)>,
}

impl PixFont {
    pub(crate) fn new(bytes: Vec<u8>, px: f32) -> Option<PixFont> {
        swash::FontRef::from_index(&bytes, 0)?;
        Some(PixFont { bytes: std::sync::Arc::new(bytes), px, scaler: swash::scale::ScaleContext::new(), cache: HashMap::new() })
    }

    pub(crate) fn font(&self) -> swash::FontRef<'_> {
        swash::FontRef::from_index(&self.bytes, 0).expect("checked in new")
    }

    /// Does this font have every character of `s`?
    pub(crate) fn covers(&self, s: &str) -> bool {
        let map = self.font().charmap();
        s.chars().all(|c| c.is_whitespace() || map.map(c) != 0)
    }

    /// Each character's glyph and its advance in whole pixels at `size` (whole pixels, as Windows lays text out).
    pub(crate) fn glyphs(&self, s: &str, size: f32) -> Vec<(swash::GlyphId, f32)> {
        let font = self.font();
        let (map, metrics) = (font.charmap(), font.glyph_metrics(&[]).scale(size));
        let tuned = hdmx_widths(&font, size.round() as u8);
        s.chars()
            .map(|ch| {
                let id = map.map(ch);
                let adv = tuned.and_then(|w| w.get(id as usize)).map(|w| *w as f32);
                (id, adv.unwrap_or_else(|| metrics.advance_width(id).round()))
            })
            .collect()
    }

    /// Width in screen pixels at `size` pixels.
    pub(crate) fn width_at(&self, s: &str, size: f32) -> f32 {
        self.glyphs(s, size).iter().map(|g| g.1).sum()
    }

    #[cfg(test)]
    pub(crate) fn width(&self, s: &str) -> f32 {
        self.width_at(s, self.px)
    }

    /// The line's pixels at `size`: width, height and one coverage byte per pixel (0 or 255).
    pub(crate) fn raster(&mut self, s: &str, size: f32) -> (usize, usize, Vec<u8>) {
        use swash::scale::{Render, Source, StrikeWith};
        let glyphs = self.glyphs(s, size);
        let bytes = self.bytes.clone();
        let font = swash::FontRef::from_index(&bytes, 0).expect("checked in new");
        let m = font.metrics(&[]).scale(size);
        let asc = m.ascent.round() as i32;
        let h = (m.ascent + m.descent).ceil() as usize + 2;
        let w = glyphs.iter().map(|g| g.1).sum::<f32>() as usize + 4;
        let mut px = vec![0u8; w * h];
        let mut scaler = self.scaler.builder(font).size(size).hint(true).build();
        let mut x = 1i32;
        for (id, adv) in glyphs {
            let img = Render::new(&[Source::Bitmap(StrikeWith::ExactSize), Source::Outline])
                .format(swash::zeno::Format::Alpha)
                .render(&mut scaler, id);
            if let Some(img) = img {
                let pl = img.placement;
                let embedded = matches!(img.source, Source::Bitmap(_));
                for gy in 0..pl.height as i32 {
                    for gx in 0..pl.width as i32 {
                        let c = img.data[(gy * pl.width as i32 + gx) as usize];
                        // hinted outlines sit on the pixel grid; a stroke a little off it still counts (a quarter
                        // covered), so light fonts like Courier keep every stroke at small sizes
                        // (half covered or more, as black-and-white hinting draws it - less fattens every stroke)
                        if c >= if embedded { 1 } else { 128 } {
                            let (dx, dy) = (x + pl.left + gx, 1 + asc - pl.top + gy);
                            if dx >= 0 && dy >= 0 && (dx as usize) < w && (dy as usize) < h {
                                px[dy as usize * w + dx as usize] = 255;
                            }
                        }
                    }
                }
            }
            x += adv as i32;
        }
        (w, h, px)
    }

    /// The line drawn at `size` screen pixels (cached).
    pub(crate) fn line(&mut self, ctx: &egui::Context, s: &str, size: f32) -> (egui::TextureId, Vec2) {
        let key = (s.to_string(), size.round() as u32);
        if let Some((t, sz)) = self.cache.get(&key) {
            return (t.id(), *sz);
        }
        if self.cache.len() > 900 {
            self.cache.clear();
        }
        let (w, h, cov) = self.raster(s, size.round());
        let pixels = cov.iter().map(|&v| Color32::from_rgba_premultiplied(v, v, v, v)).collect();
        let tex = ctx.load_texture("playlist_line", egui::ColorImage { size: [w, h], pixels }, egui::TextureOptions::NEAREST);
        let sz = Vec2::new(w as f32, h as f32);
        let id = tex.id();
        self.cache.insert(key, (tex, sz));
        (id, sz)
    }

    /// The size in screen pixels for text meant to look about `size` points tall: a whole multiple of the
    /// font's own size (pixel fonts stay pixel-perfect), never smaller than it.
    pub(crate) fn screen_px(&self, size: f32, ppp: f32) -> f32 {
        let want = size * ppp;
        if self.px >= 10.0 {
            want.round().max(self.px)
        } else {
            (want / self.px).round().max(1.0) * self.px
        }
    }
}

thread_local! {
    static PIX: std::cell::RefCell<Option<PixFont>> = const { std::cell::RefCell::new(None) };
}

/// Make `bytes` the playlist font (None: back to plain text).
pub(crate) fn set_playlist_font(_ctx: &egui::Context, bytes: Option<Vec<u8>>, choice: usize) {
    let px = match PL_FONTS.get(choice).map(|f| &f.1) {
        Some(FontSrc::Web(_, px)) => *px,
        _ => 11.0, // Winamp's playlist size: 8 points at 96 dpi
    };
    PIX.with(|c| *c.borrow_mut() = bytes.and_then(|b| PixFont::new(b, px)));
}

/// Whether a skin's frames are on (lists then use the playlist font and colours).
pub(crate) fn chrome_on() -> bool {
    CHROME.with(|c| c.borrow().is_some())
}

/// Whether text should take the skin's playlist font (a skin is worn and its font is ready).
#[allow(dead_code)]
pub(crate) fn text_override() -> bool {
    chrome_on() && PIX.with(|c| c.borrow().is_some())
}

/// A label written in capitals ("PICK ONE...", "RHYTHM CHANGES"), as it reads in an ordinary font: a short one in
/// Title Case, a longer one as a sentence. Words with a digit (chords, times) and abbreviations stay as they are;
/// text that has any lower case already is left alone.
/// In a sentence, a run of words in capitals (a button's name: "+ MY TUNES keeps it") reads as a name: "+ My Tunes
/// keeps it". Single letters and abbreviations stay.
pub(crate) fn caps_words(text: &str) -> String {
    let caps = |w: &str| {
        let letters: Vec<char> = w.chars().filter(|c| c.is_alphabetic()).collect();
        letters.len() >= 2 && letters.iter().all(|c| c.is_uppercase())
    };
    let words: Vec<&str> = text.split(' ').collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if caps(words[i]) {
            let mut j = i;
            while j < words.len() && (caps(words[j]) || (j > i && words[j] == "&")) {
                j += 1;
            }
            out.push(display_case(&words[i..j].join(" ")).into_owned());
            i = j;
        } else {
            out.push(words[i].to_string());
            i += 1;
        }
    }
    out.join(" ")
}

thread_local! {
    static TITLE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Labels drawn inside `f` (buttons, tabs) read in Title Case however many words they have ("Pick Up Where I Left
/// Off"), unless they are a sentence.
pub(crate) fn as_title<R>(f: impl FnOnce() -> R) -> R {
    let before = TITLE.with(|c| c.replace(true));
    let r = f();
    TITLE.with(|c| c.set(before));
    r
}

pub(crate) fn display_case(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.chars().any(|c| c.is_uppercase()) || text.chars().any(|c| c.is_lowercase()) {
        return std::borrow::Cow::Borrowed(text);
    }
    const KEEP: [&str; 27] = [
        "EQ", "PL", "BPM", "YT", "OK", "DJ", "CD", "ID", "TV", "AI", "MP3", "WAV", "MIDI", "A-B", "II", "III", "IV", "VI", "VII",
        "PDF", "USB", "UK", "US", "BACKLINE", "MB", "KB", "GB",
    ];
    let words: Vec<&str> = text.split(' ').collect();
    let n = words.iter().filter(|w| !w.is_empty()).count();
    let sentence = text.contains(|c: char| matches!(c, '.' | ',' | ';' | '?' | '!')) && !text.ends_with("...");
    let title = n <= 4 || (TITLE.with(|c| c.get()) && !sentence && n <= 8);
    let mut out = String::with_capacity(text.len());
    let mut first = true;
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        if w.is_empty() {
            continue;
        }
        let core = w.trim_matches(|c: char| !c.is_alphanumeric());
        // a single letter: a note or a loop end ("A AND B", "TO B") stays a capital; otherwise it is the word "a"
        let next = words.get(i + 1).copied().unwrap_or("");
        let prev = if i > 0 { words[i - 1] } else { "" };
        // (B to G alone are always notes; A is one beside "and", "or", a dash, after a colon or before a bracket)
        let letter_name = core.chars().count() == 1
            && (matches!(core, "B" | "C" | "D" | "E" | "F" | "G")
                || next.is_empty()
                || next.starts_with('(')
                || prev.ends_with(':')
                || matches!(next, "AND" | "OR" | "-" | "&")
                || matches!(prev, "AND" | "OR" | "TO" | "&" | "-"));
        // names spelled their own way
        const NAMES: [(&str, &str); 5] =
            [("SOUNDCLOUD", "SoundCloud"), ("YOUTUBE", "YouTube"), ("IREAL", "iReal"), ("TIDAL", "Tidal"), ("YT-DLP", "yt-dlp")];
        // roman numerals of a progression ("I-IV-V", "II-V-I") stay as they are
        let roman = !core.is_empty()
            && core.split('-').all(|p| !p.is_empty() && p.chars().all(|c| matches!(c, 'I' | 'V' | 'X')))
            && core.contains('-');
        // an ordinal ("4TH", "2ND") reads 4th
        let ordinal = {
            // (and plural: "7THS" reads 7ths)
            let o = core.strip_suffix('S').filter(|o| o.ends_with("TH")).unwrap_or(core);
            o.is_ascii()
                && o.len() > 2
                && o[..o.len() - 2].chars().all(|c| c.is_ascii_digit())
                && matches!(&o[o.len() - 2..], "ST" | "ND" | "RD" | "TH")
        };
        // small words stay small inside a title ("Set A and B", "Pick One of Each")
        const SMALL: [&str; 15] =
            ["A", "AN", "AND", "OR", "OF", "TO", "IN", "ON", "AT", "THE", "FOR", "WITH", "BY", "FROM", "AS"];
        if let Some((_, name)) = NAMES.iter().find(|(caps, _)| *caps == core) {
            out.push_str(&w.replacen(core, name, 1));
        } else if core == "I" || w.starts_with("I'") || w.starts_with("I\u{2019}") {
            // the word "I" (I'M, I'LL...) is always a capital
            out.push('I');
            out.push_str(&w[1..].to_lowercase());
        } else if ordinal {
            out.push_str(&w.to_lowercase());
        } else if roman || w.chars().any(|c| c.is_ascii_digit()) || KEEP.contains(&core) || (letter_name && !first) {
            out.push_str(w);
        } else if title && !first && SMALL.contains(&core) {
            out.push_str(&w.to_lowercase());
        } else {
            let mut up = title || first;
            let mut after_apos = false;
            for c in w.chars() {
                if c.is_alphabetic() {
                    if up && !after_apos {
                        out.extend(c.to_uppercase());
                        up = false;
                    } else {
                        out.extend(c.to_lowercase());
                    }
                    after_apos = false;
                } else {
                    after_apos = c == APOS;
                    // a new part after a dash or slash ("UP-TEMPO") starts high again in a title
                    if title && (c == '-' || c == '/') {
                        up = true;
                    }
                    out.push(c);
                }
            }
        }
        if !core.is_empty() {
            first = false;
        }
        // a new sentence after a full stop, question or exclamation mark starts with a capital
        if w.ends_with(['.', '?', '!']) && !w.ends_with("...") {
            first = true;
        }
    }
    std::borrow::Cow::Owned(out)
}

const APOS: char = '\u{27}';

/// A line of the playlist in its font as Winamp drew it: at the font's own size (11 pixels for an ordinary font, a
/// pixel font's native one), every pixel `k` points square - a whole number of screen pixels - so the stack's text
/// grows with the stack, crisp. Cut with "..." to `max_w`; `pos` is the left / centre / right middle point. Returns
/// how wide it came out.
pub(crate) fn pl_text(p: &egui::Painter, pos: Pos2, align: Align, text: &str, k: f32, max_w: f32, col: Color32) -> f32 {
    // the skin's own font, 11 skin pixels - Windows' 8 points - tall
    let font = egui::FontId::new(11.0 * k, egui::FontFamily::Name("skin".into()));
    let fits = |s: &str| p.layout_no_wrap(s.to_string(), font.clone(), col).size().x <= max_w;
    let mut s = text.to_string();
    if !fits(&s) {
        while !s.is_empty() && !fits(&format!("{}...", s)) {
            s.pop();
        }
        s = format!("{}...", s.trim_end());
    }
    let g = p.layout_no_wrap(s, font, col);
    let w = g.size().x;
    let x = match align {
        Align::Min => pos.x,
        Align::Center => pos.x - w / 2.0,
        Align::Max => pos.x - w,
    };
    let ppp = font::ppp();
    let snap = |v: f32| (v * ppp).round() / ppp;
    p.galley(Pos2::new(snap(x), snap(pos.y - g.size().y / 2.0)), g, col);
    w
}

/// How wide a line comes out in the playlist font at `size`.
pub(crate) fn sans_width(text: &str, size: f32) -> f32 {
    PIX.with(|c| {
        let c = c.borrow();
        match c.as_ref() {
            Some(pf) => mixed_w(pf, text, size),
            None => text.chars().count() as f32 * size * 0.5,
        }
    })
}

/// Split text into runs the skin's font can draw (true) and runs it can't (false).
pub(crate) fn runs(pf: &PixFont, text: &str) -> Vec<(bool, String)> {
    let mut out: Vec<(bool, String)> = Vec::new();
    let mut buf = [0u8; 4];
    for ch in text.chars() {
        let ok = pf.covers(ch.encode_utf8(&mut buf));
        match out.last_mut() {
            Some((k, s)) if *k == ok || ch.is_whitespace() => s.push(ch),
            _ => out.push((ok, ch.to_string())),
        }
    }
    out
}

/// Width of a line drawn in the skin's font, with the characters it lacks in the smooth font.
pub(crate) fn mixed_w(pf: &PixFont, text: &str, size: f32) -> f32 {
    let ppp = font::ppp();
    let spx = pf.screen_px(size, ppp);
    runs(pf, text).iter().map(|(ok, s)| if *ok { pf.width_at(s, spx) / ppp } else { font::smooth_w(s, size) }).sum()
}

/// One line in the playlist font, cut with "..." to `max_w`; `pos` is the left / centre / right middle point and
/// `size` about how tall it should come out. Returns how wide it came out.
pub(crate) fn sans_text(p: &egui::Painter, pos: Pos2, align: Align, text: &str, size: f32, max_w: f32, col: Color32) -> f32 {
    let drawn = PIX.with(|c| {
        let mut c = c.borrow_mut();
        let pf = c.as_mut()?;
        let ppp = font::ppp();
        // characters the skin's font lacks (Japanese, ...): only those in the smooth font, the rest stays the skin's
        if !pf.covers(text) {
            let mut s = text.to_string();
            if mixed_w(pf, &s, size) > max_w {
                while !s.is_empty() && mixed_w(pf, &format!("{}\u{2026}", s), size) > max_w {
                    s.pop();
                }
                s = format!("{}\u{2026}", s.trim_end());
            }
            let w = mixed_w(pf, &s, size);
            let mut x = match align {
                Align::Min => pos.x,
                Align::Center => pos.x - w / 2.0,
                Align::Max => pos.x - w,
            };
            let spx = pf.screen_px(size, ppp);
            let snap = |v: f32| (v * ppp).round() / ppp;
            for (ok, run) in runs(pf, &s) {
                if ok {
                    let (tex, sz) = pf.line(p.ctx(), &run, spx);
                    let (rw, rh) = (sz.x / ppp, sz.y / ppp);
                    let r = Rect::from_min_size(Pos2::new(snap(x), snap(pos.y - rh / 2.0)), Vec2::new(rw, rh));
                    p.image(tex, r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), col);
                    x += rw;
                } else {
                    let g = p.layout_no_wrap(run, egui::FontId::proportional(size), col);
                    let gw = g.size().x;
                    p.galley(Pos2::new(snap(x), snap(pos.y - g.size().y / 2.0)), g, col);
                    x += gw;
                }
            }
            return Some(w);
        }
        let spx = pf.screen_px(size, ppp);
        let mut s = text.to_string();
        if pf.width_at(&s, spx) / ppp > max_w {
            while !s.is_empty() && pf.width_at(&format!("{}\u{2026}", s), spx) / ppp > max_w {
                s.pop();
            }
            s = format!("{}\u{2026}", s.trim_end());
        }
        let (tex, sz) = pf.line(p.ctx(), &s, spx);
        let (w, h) = (sz.x / ppp, sz.y / ppp);
        let x = match align {
            Align::Min => pos.x,
            Align::Center => pos.x - w / 2.0,
            Align::Max => pos.x - w,
        };
        let snap = |v: f32| (v * ppp).round() / ppp;
        let r = Rect::from_min_size(Pos2::new(snap(x), snap(pos.y - h / 2.0)), Vec2::new(w, h));
        p.image(tex, r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), col);
        Some(w)
    });
    drawn.unwrap_or_else(|| {
        // no font yet: plain smooth text
        let mut job = egui::text::LayoutJob::single_section(
            text.to_string(),
            egui::TextFormat { font_id: egui::FontId::proportional(size), color: col, ..Default::default() },
        );
        job.wrap = egui::text::TextWrapping {
            max_width: max_w.max(8.0),
            max_rows: 1,
            break_anywhere: true,
            overflow_character: Some('\u{2026}'),
        };
        let g = p.layout_job(job);
        let w = g.size().x;
        let x = match align {
            Align::Min => pos.x,
            Align::Center => pos.x - w / 2.0,
            Align::Max => pos.x - w,
        };
        p.galley(Pos2::new(x, pos.y - g.size().y / 2.0), g, col);
        w
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn skins_fonts_are_found_by_name() {
        // the names skins really give, styles included
        for name in [
            "Arial",
            "Arial Narrow",
            "Arial Bold",
            "Trebuchet MS Italic",
            "Verdana Italic",
            "Tahoma",
            "Comic Sans MS",
            "MS Sans Serif",
        ] {
            assert!(system_font(name).is_some(), "{} not found", name);
        }
        assert!(system_font("Adelaide CE").is_none(), "a font that is not installed is None (Arial stands in)");
    }

    #[cfg(windows)]
    #[test]
    fn lines_are_crisp_pixels() {
        let mut pf = PixFont::new(system_font("Arial").unwrap(), 10.0).unwrap();
        let w = pf.width("1. Kanye West - FML");
        assert!(w > 60.0 && w < 140.0, "about Winamp's width at 10 px: {}", w);
        assert!(pf.width("WWW") > pf.width("iii"), "proportional");
        assert!(pf.screen_px(20.0, 1.0) >= 20.0 && pf.screen_px(4.0, 1.0) >= 10.0, "never below its own size");
        let pixel = PixFont::new(system_font("Arial").unwrap(), 8.0).unwrap();
        assert_eq!(pixel.screen_px(15.0, 1.0), 16.0, "a pixel font comes in whole multiples of its size");
        // hinted: every upright stroke comes out the same width at every size, the cure for grainy text
        for size in [10.0, 12.0, 13.0, 15.0, 17.0, 20.0] {
            let (w, h, px) = pf.raster("lllllll", size);
            let mut widths = std::collections::BTreeSet::new();
            let y = (0..h).max_by_key(|y| (0..w).filter(|x| px[y * w + x] > 0).count()).unwrap();
            let mut run = 0;
            for x in 0..w {
                if px[y * w + x] > 0 {
                    run += 1;
                } else if run > 0 {
                    widths.insert(run);
                    run = 0;
                }
            }
            assert_eq!(widths.len(), 1, "even stems at {} px: {:?}", size, widths);
        }
    }

    #[cfg(windows)]
    #[test]
    fn installed_fonts_read_from_the_registry() {
        let fonts = registry_fonts();
        assert!(fonts.len() > 20, "only {} fonts", fonts.len());
        assert!(installed_fonts().contains_key("arial"), "no arial");
    }

    #[test]
    fn capitals_read_as_words() {
        assert_eq!(display_case("PICK ONE..."), "Pick One...");
        assert_eq!(display_case("BILLIE'S BOUNCE"), "Billie's Bounce");
        assert_eq!(display_case("UP-TEMPO"), "Up-Tempo");
        assert_eq!(display_case("EQ"), "EQ");
        assert_eq!(display_case("1 OF 10 KINDS IN YOUR SET"), "1 of 10 kinds in your set");
        assert_eq!(display_case("Already mixed"), "Already mixed");
        assert_eq!(display_case("PICK UP WHERE I LEFT OFF"), "Pick up where I left off");
        assert_eq!(as_title(|| display_case("PICK UP WHERE I LEFT OFF").into_owned()), "Pick Up Where I Left Off");
        assert_eq!(display_case("I'M HERE"), "I'm Here");
        assert_eq!(display_case("GET YT-DLP"), "Get yt-dlp");
        assert_eq!(display_case("I-IV-V"), "I-IV-V");
        assert_eq!(display_case("7THS"), "7ths");
        assert_eq!(display_case("START ON: C (AS WRITTEN)"), "Start on: C (as written)");
        assert_eq!(display_case("START ON: A (AS WRITTEN)"), "Start on: A (as written)");
        assert_eq!(display_case("B♭7 OVER E♭"), "B♭7 Over E♭");
        assert_eq!(caps_words("Triads, then 7THS."), "Triads, then 7ths.");
        assert_eq!(as_title(|| display_case("UP A 4TH - 12 KEYS").into_owned()), "Up a 4th - 12 Keys");
        assert_eq!(as_title(|| display_case("GET STEMS TOOL (170 MB)").into_owned()), "Get Stems Tool (170 MB)");
        assert_eq!(display_case("SAVE AS A MOVE"), "Save as a Move");
        assert_eq!(
            display_case("DELETE THIS GROUP? ITS LOOPS STAY, WITH NO GROUP"),
            "Delete this group? Its loops stay, with no group"
        );
        assert_eq!(caps_words("Click one; + MY TUNES keeps it at 120 BPM."), "Click one; + My Tunes keeps it at 120 BPM.");
        assert_eq!(caps_words("No book? MADE-UP LINES works"), "No book? Made-Up Lines works");
        assert_eq!(display_case("PLAY A TRACK TO START"), "Play a track to start");
        assert_eq!(display_case("SET A AND B"), "Set A and B");
        assert_eq!(display_case("SOUNDCLOUD"), "SoundCloud");
    }
}
