//! Skins: Backline's own looks (each three colours: its panels, its display, its accent; or your Custom one), the
//! palette worked out from them, and the colour helpers the whole UI draws with.

use super::*;

// ------------------------------------------------------------------- skins
#[derive(Clone, Copy)]
pub(crate) struct Pal {
    pub(crate) app_bg: Color32,
    /// the accent: lit writing on the edge colour (column heads), a look's own accent
    pub(crate) accent: Color32,
    /// the panels (menus, frames, the library's side), lighter and darker shades, and the hover tone
    pub(crate) panel: Color32,
    pub(crate) panel_lt: Color32,
    pub(crate) panel_dk: Color32,
    pub(crate) panel_hover: Color32,
    /// the display: lists and boxes (a playlist's colour), and a faint mark on it
    pub(crate) display: Color32,
    pub(crate) display_ghost: Color32,
    pub(crate) groove: Color32,
    pub(crate) sel: Color32,
    pub(crate) ink: Color32,
    pub(crate) ink2: Color32,
    pub(crate) dim: Color32,
    pub(crate) red: Color32,
    pub(crate) btn_face: Color32,
    pub(crate) btn_hi: Color32,
    pub(crate) row_alt: Color32,
    pub(crate) row_sel: Color32,
    pub(crate) bar_txt: Color32,
    pub(crate) edge: Color32,
}

pub(crate) const fn c(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// One of Backline's own looks: its panels' colour, its display's (the lists wear it too) and the accent that lights
/// the display, the sliders and the playing song. Everything else is worked out from these (`look_pal`, genskin).
pub(crate) struct Look {
    pub(crate) name: &'static str,
    pub(crate) panel: Color32,
    pub(crate) display: Color32,
    pub(crate) accent: Color32,
}

const fn look(name: &'static str, panel: Color32, display: Color32, accent: Color32) -> Look {
    Look { name, panel, display, accent }
}

/// Backline's looks: the neutral ones, then around the colour wheel, then paper ones, then the two for the eyes.
/// (Graphite, Midnight, Blue Hour and High Contrast are the old Dark, Sleek Light, Sleek Dark and High Contrast Dark,
/// exactly.) Greyscale is calm on purpose: no colour anywhere, the covers in grey too (`mono`).
pub(crate) const LOOKS: [Look; 29] = [
    look("GRAPHITE", c(43, 43, 44), c(13, 13, 13), c(169, 172, 176)),
    look("GREYSCALE", c(64, 64, 64), c(30, 30, 30), c(196, 196, 196)),
    look("STORM", c(40, 44, 52), c(12, 14, 18), c(255, 214, 90)),
    look("SLATE", c(52, 60, 70), c(16, 20, 26), c(170, 190, 215)),
    look("CLASSIC", c(52, 52, 68), c(0, 0, 0), c(0, 226, 0)),
    look("MIDNIGHT", c(36, 45, 67), c(10, 14, 22), c(206, 212, 228)),
    look("BLUE HOUR", c(40, 48, 69), c(12, 15, 23), c(120, 160, 255)),
    look("COBALT", c(30, 44, 86), c(8, 14, 32), c(110, 170, 255)),
    look("DUSK", c(52, 40, 78), c(16, 11, 28), c(196, 164, 255)),
    look("ORCHID", c(60, 30, 62), c(18, 8, 20), c(238, 140, 255)),
    look("NEON", c(34, 24, 44), c(10, 6, 16), c(255, 92, 214)),
    look("ROSEWOOD", c(72, 38, 50), c(24, 10, 16), c(255, 150, 182)),
    look("CRIMSON", c(70, 24, 30), c(22, 6, 8), c(255, 96, 96)),
    look("EMBER", c(70, 40, 28), c(24, 11, 6), c(255, 146, 74)),
    look("COFFEE", c(56, 42, 34), c(18, 12, 8), c(222, 180, 140)),
    look("AMBER", c(60, 48, 30), c(20, 14, 4), c(255, 190, 70)),
    look("OLIVE", c(58, 58, 36), c(18, 18, 8), c(214, 206, 120)),
    look("FOREST", c(30, 50, 34), c(8, 18, 10), c(230, 200, 90)),
    look("MOSS", c(38, 56, 40), c(10, 20, 12), c(140, 226, 140)),
    look("SEA GLASS", c(26, 58, 62), c(6, 20, 22), c(104, 228, 214)),
    look("PAPER", c(228, 220, 204), c(247, 243, 233), c(52, 74, 140)),
    look("SAND", c(232, 220, 196), c(250, 244, 232), c(150, 84, 30)),
    look("SAKURA", c(240, 222, 228), c(252, 244, 246), c(196, 48, 104)),
    look("LAVENDER", c(226, 220, 240), c(248, 246, 253), c(96, 64, 170)),
    look("DAYLIGHT", c(208, 216, 226), c(245, 248, 252), c(0, 96, 180)),
    look("ARCTIC", c(214, 228, 236), c(246, 251, 253), c(0, 120, 150)),
    look("SPRING", c(214, 236, 224), c(245, 252, 248), c(0, 128, 80)),
    look("HIGH CONTRAST", c(29, 29, 29), c(8, 8, 8), c(255, 255, 0)),
    look("HIGH CONTRAST LIGHT", c(230, 230, 230), c(255, 255, 255), c(0, 0, 0)),
];

/// The looks' names, in order.
pub(crate) const SKIN_NAMES: [&str; LOOKS.len()] = {
    let mut out = [""; LOOKS.len()];
    let mut i = 0;
    while i < LOOKS.len() {
        out[i] = LOOKS[i].name;
        i += 1;
    }
    out
};

/// The look first worn (the very first run).
pub(crate) const FIRST_LOOK: usize = 0;

/// A look by its name (as kept in the settings: the list can grow and change order).
pub(crate) fn look_named(name: &str) -> Option<usize> {
    LOOKS.iter().position(|l| l.name.eq_ignore_ascii_case(name.trim()))
}

/// Where a look kept by number goes: 9.356 kept the number in its list of fourteen.
pub(crate) fn look_from_356(n: usize) -> usize {
    const AT_356: [&str; 14] = [
        "GRAPHITE", "MIDNIGHT", "COBALT", "DUSK", "NEON", "ROSEWOOD", "EMBER", "AMBER", "MOSS", "SEA GLASS", "PAPER",
        "DAYLIGHT", "HIGH CONTRAST", "HIGH CONTRAST LIGHT",
    ];
    AT_356.get(n).and_then(|name| look_named(name)).unwrap_or(FIRST_LOOK)
}

/// Where a look kept by an older version (the seventeen palettes before 9.356) went.
pub(crate) fn look_from_old(old: usize) -> usize {
    // olive, aqua, dark, amber, paper, plum, slate, clay, aero glass, sleek dark, sleek light, neon, sunset, ocean,
    // citrus, high contrast dark, high contrast light
    const TO: [&str; 17] = [
        "OLIVE", "SEA GLASS", "GRAPHITE", "AMBER", "PAPER", "DUSK", "SLATE", "EMBER", "SEA GLASS", "BLUE HOUR", "MIDNIGHT",
        "NEON", "ROSEWOOD", "SEA GLASS", "MOSS", "HIGH CONTRAST", "HIGH CONTRAST LIGHT",
    ];
    TO.get(old).and_then(|name| look_named(name)).unwrap_or(FIRST_LOOK)
}

/// Greyscale worn: nothing in colour, the covers in grey too (and hearts).
static MONO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn mono() -> bool {
    MONO.load(Ordering::Relaxed)
}

/// Set as a look goes on: whether it's the calm grey one.
pub(crate) fn set_mono(look: Option<usize>) {
    MONO.store(look.is_some_and(|n| LOOKS.get(n).is_some_and(|l| l.name == "GREYSCALE")), Ordering::Relaxed);
}

/// A whole palette from a look's three colours (what's worn before its skin is: the skin then gives its own).
pub(crate) fn look_pal(panel: Color32, display: Color32, accent: Color32) -> Pal {
    let light = lum(display) > 0.5;
    let lt = |x: Color32, t: f32| x.lerp_to_gamma(Color32::WHITE, t);
    let dk = |x: Color32, t: f32| x.lerp_to_gamma(Color32::BLACK, t);
    let ink = if light { c(22, 22, 26) } else { c(226, 229, 236) };
    Pal {
        app_bg: if light { dk(panel, 0.12) } else { dk(display, 0.4) },
        accent,
        panel,
        panel_lt: lt(panel, 0.1),
        panel_dk: dk(panel, 0.2),
        panel_hover: lt(panel, 0.05),
        display,
        display_ghost: display.lerp_to_gamma(accent, 0.12),
        groove: dk(panel, 0.35),
        sel: display.lerp_to_gamma(accent, 0.25),
        ink,
        ink2: ink.lerp_to_gamma(display, 0.25),
        dim: ink.lerp_to_gamma(display, 0.45),
        red: c(220, 70, 60),
        btn_face: lt(panel, 0.08),
        btn_hi: lt(panel, 0.22),
        row_alt: display.lerp_to_gamma(panel, 0.2),
        row_sel: display.lerp_to_gamma(accent, 0.35),
        bar_txt: ink,
        edge: if light { dk(panel, 0.6) } else { c(6, 6, 6) },
    }
}

/// Your Custom look: its background and accent, and its display and writing when you've picked them (else worked out
/// from the background). Kept in step with the built-in looks: the background eased to a panel dark enough for light
/// writing (or, picked light, a paper one for dark writing); the writing always reads on the display.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct CustomLook {
    pub(crate) background: Color32,
    pub(crate) accent: Color32,
    pub(crate) display: Option<Color32>,
    pub(crate) text: Option<Color32>,
}

impl Default for CustomLook {
    fn default() -> CustomLook {
        CustomLook { background: c(36, 45, 67), accent: c(110, 170, 255), display: None, text: None }
    }
}

impl CustomLook {
    /// Starting from one of Backline's looks.
    pub(crate) fn from_look(n: usize) -> CustomLook {
        let l = &LOOKS[n % LOOKS.len()];
        CustomLook { background: l.panel, accent: l.accent, display: Some(l.display), text: None }
    }

    /// The panel, display and accent it is worn with, and its writing (None: as a built-in look's).
    pub(crate) fn colors(&self) -> (Color32, Color32, Color32, Option<Color32>) {
        let (h, s, v) = rgb_to_hsv(self.background);
        let light = lum(self.background) > 0.45;
        let panel = if light {
            // paper: light, a hint of the colour
            hsv_to_rgb(h, (s * 0.35).min(0.18), v.max(0.86))
        } else {
            // dark: the colour kept, deep enough that the writing on it reads (as dark as you like: black is black)
            hsv_to_rgb(h, s.min(0.6), v.min(0.32))
        };
        let display = self.display.unwrap_or_else(|| {
            if light {
                hsv_to_rgb(h, (s * 0.15).min(0.06), 0.97)
            } else {
                hsv_to_rgb(h, s.min(0.7), (v * 0.34).clamp(0.03, 0.11))
            }
        });
        let text = self.text.map(|t| crate::winamp::readable(t, display, 4.5));
        (panel, display, self.accent, text)
    }

    /// Its palette (what's worn before its skin is).
    pub(crate) fn pal(&self) -> Pal {
        let (panel, display, accent, _) = self.colors();
        look_pal(panel, display, accent)
    }
}

/// The looks' palettes, in order (worked out once).
pub(crate) static PALS: std::sync::LazyLock<Vec<Pal>> =
    std::sync::LazyLock::new(|| LOOKS.iter().map(|l| look_pal(l.panel, l.display, l.accent)).collect());

pub(crate) static SKIN: AtomicUsize = AtomicUsize::new(0);

/// Preferences > ACCESSIBILITY: loops in orange (told apart from green and red by nearly everyone), and less motion.
pub(crate) static LOOP_SAFE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub(crate) static CALM: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The colour of loops and their A / B markers.
pub(crate) fn loop_col() -> Color32 {
    if LOOP_SAFE.load(Ordering::Relaxed) {
        Color32::from_rgb(230, 159, 0)
    } else {
        pal().red
    }
}

/// Reduce motion: no spinning record, no tilting cover, no scrolling titles.
pub(crate) fn calm() -> bool {
    CALM.load(Ordering::Relaxed)
}

/// A palette made from a Winamp skin (winamp.rs), worn instead of the built-in ones while set. Each one is
/// leaked on purpose (a few bytes per skin picked), so `pal()` can hand out a plain reference.
static CUSTOM: std::sync::atomic::AtomicPtr<Pal> = std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

/// How light a colour looks (WCAG relative luminance, 0 black to 1 white).
fn lum(c: Color32) -> f32 {
    let ch = |v: u8| {
        let s = v as f32 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
}

/// How far apart two colours read (WCAG contrast ratio, 1 to 21).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `c` moved a step at a time toward black (or white) until it is `min` apart from `against`.
fn push(c: Color32, against: Color32, min: f32, to_dark: bool) -> Color32 {
    let target = if to_dark { Color32::BLACK } else { Color32::WHITE };
    let mut out = c;
    for k in 1..=10 {
        if contrast(out, against) >= min {
            break;
        }
        out = c.lerp_to_gamma(target, k as f32 / 10.0);
    }
    out
}

/// A skin's colours made readable: every text colour clear of every background it sits on - the main text at
/// least 4.5 : 1 (the usual standard), the second text 3 : 1, the faint text 2.2 : 1. Backgrounds behind light text
/// go darker (rather than the text going grey), dark text gets lighter backgrounds. Skins that are already clear
/// don't change.
pub(crate) fn readable(mut p: Pal, app_bg_too: bool) -> Pal {
    // which side the skin is on comes from its list background, never from its text: a dark skin stays dark (and a
    // light one light) whatever colour its text is. Text that can't read on that side - deep red, blue or purple on
    // dark - is lifted (or deepened) itself, rather than the backgrounds flipping to white.
    let dark = lum(p.display) < 0.5;
    let far = if dark { Color32::BLACK } else { Color32::WHITE };
    let toward = if dark { Color32::WHITE } else { Color32::BLACK };
    for (fg, min) in [(&mut p.ink, 5.5f32), (&mut p.bar_txt, 5.0), (&mut p.ink2, 4.0), (&mut p.dim, 3.0)] {
        for k in 0..=20 {
            let c = fg.lerp_to_gamma(toward, k as f32 / 20.0);
            if contrast(c, far) >= min || k == 20 {
                *fg = c;
                break;
            }
        }
    }
    let light_ink = dark;
    if app_bg_too {
        p.app_bg = push(p.app_bg, p.ink, 4.5, light_ink);
    }
    for bg in [
        &mut p.display,
        &mut p.panel,
        &mut p.panel_lt,
        &mut p.panel_dk,
        &mut p.panel_hover,
        &mut p.btn_face,
        &mut p.row_alt,
        &mut p.sel,
        &mut p.row_sel,
    ] {
        *bg = push(*bg, p.ink, 4.5, light_ink);
    }
    for (fg, min) in [(&mut p.ink2, 3.0), (&mut p.dim, 2.2)] {
        for bg in if app_bg_too { vec![p.app_bg, p.display, p.panel] } else { vec![p.display, p.panel] } {
            *fg = push(*fg, bg, min, !light_ink);
        }
    }
    p.bar_txt = push(p.bar_txt, p.row_sel, 4.5, lum(p.row_sel) > 0.4);
    p
}

pub(crate) fn set_custom(p: Option<Pal>) {
    // a worn skin's colours, kept readable whatever the skin's own choices
    let p = p.map(|p| readable(p, true));
    let ptr = p.map_or(std::ptr::null_mut(), |p| Box::into_raw(Box::new(p)));
    CUSTOM.store(ptr, Ordering::Release);
}

/// Rounded fill (modern skins).
pub(crate) fn rfill(p: &egui::Painter, r: Rect, rad: f32, c: Color32) {
    p.rect_filled(r, Rounding::same(rad), c);
}

pub(crate) fn rline(p: &egui::Painter, r: Rect, rad: f32, w: f32, c: Color32) {
    p.rect_stroke(r, Rounding::same(rad), Stroke::new(w, c));
}

/// Vertical gradient inside a rounded rectangle, drawn as thin horizontal slices clipped by the corner radius.
pub(crate) fn rgrad(p: &egui::Painter, r: Rect, rad: f32, top: Color32, bot: Color32) {
    let n = (r.height() / 2.0).ceil().max(1.0) as usize;
    let lerp = |a: u8, b: u8, t: f32| (a as f32 + (b as f32 - a as f32) * t) as u8;
    for i in 0..n {
        let t = i as f32 / (n.max(2) - 1) as f32;
        let y0 = r.min.y + r.height() * i as f32 / n as f32;
        let y1 = r.min.y + r.height() * (i + 1) as f32 / n as f32;
        let col = Color32::from_rgb(lerp(top.r(), bot.r(), t), lerp(top.g(), bot.g(), t), lerp(top.b(), bot.b(), t));
        // inset each slice near the corners so the gradient keeps the rounded outline
        let dy = (y0 + y1) * 0.5 - r.min.y;
        let from_edge = dy.min(r.height() - dy);
        let k = if from_edge < rad { rad - (rad * rad - (rad - from_edge) * (rad - from_edge)).max(0.0).sqrt() } else { 0.0 };
        p.rect_filled(Rect::from_min_max(Pos2::new(r.min.x + k, y0), Pos2::new(r.max.x - k, y1 + 0.5)), Rounding::ZERO, col);
    }
}

pub(crate) fn rgb_to_hsv(c: Color32) -> (f32, f32, f32) {
    let (r, g, b) = (c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let d = mx - mn;
    let h = if d == 0.0 {
        0.0
    } else if mx == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if mx == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    (h, if mx == 0.0 { 0.0 } else { d / mx }, mx)
}

pub(crate) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color32 {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - f * s), v * (1.0 - (1.0 - f) * s));
    let (r, g, b) = match (i as i32).rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

pub(crate) fn pal() -> &'static Pal {
    let c = CUSTOM.load(Ordering::Acquire);
    if !c.is_null() {
        // SAFETY: set_custom only stores pointers from Box::into_raw that are never freed
        return unsafe { &*c };
    }
    // the built-in skins, kept readable the same way (text where text is drawn; their own looks otherwise)
    static FIXED: std::sync::OnceLock<Vec<Pal>> = std::sync::OnceLock::new();
    let fixed = FIXED.get_or_init(|| PALS.iter().map(|p| readable(*p, false)).collect());
    &fixed[SKIN.load(Ordering::Relaxed) % fixed.len()]
}

#[cfg(test)]
mod contrast_tests {
    use super::*;

    /// A look as it is worn: its skin drawn, the palette read from it, made readable.
    fn worn(n: usize) -> Pal {
        let (p, _) = crate::genskin::wear(&PALS[n]).expect("skin");
        readable(p, true)
    }

    #[test]
    fn every_look_reads_where_text_sits() {
        for (n, name) in SKIN_NAMES.iter().enumerate() {
            let p = worn(n);
            for (what, fg, bg, min) in [
                ("text on lists", p.ink, p.display, 4.5),
                ("text on panels", p.ink, p.panel, 4.5),
                ("button text", p.ink, p.btn_face, 4.5),
                ("second text", p.ink2, p.display, 3.0),
                ("playing row", p.bar_txt, p.row_sel, 4.5),
            ] {
                assert!(contrast(fg, bg) >= min, "{} {}: {:.1}", name, what, contrast(fg, bg));
            }
        }
    }

    #[test]
    fn the_looks_are_told_apart() {
        // no two looks share a panel and a display (the old palettes all came out the same graphite)
        for a in 0..LOOKS.len() {
            for b in a + 1..LOOKS.len() {
                let d = |x: Color32, y: Color32| {
                    (x.r() as i32 - y.r() as i32).abs() + (x.g() as i32 - y.g() as i32).abs() + (x.b() as i32 - y.b() as i32).abs()
                };
                let apart = d(LOOKS[a].panel, LOOKS[b].panel) + d(LOOKS[a].accent, LOOKS[b].accent);
                assert!(apart >= 40, "{} and {} look alike", LOOKS[a].name, LOOKS[b].name);
            }
        }
    }

    #[test]
    fn light_looks_are_light_and_dark_ones_dark() {
        for (n, l) in LOOKS.iter().enumerate() {
            let p = worn(n);
            assert_eq!(lum(p.display) > 0.5, lum(l.display) > 0.5, "{}", l.name);
        }
    }

    #[test]
    fn a_custom_look_stays_in_step() {
        // any colours make a look whose writing reads, light or dark, with or without its own display and writing
        for bg in [c(255, 0, 0), c(0, 0, 0), c(255, 255, 255), c(128, 128, 128), c(30, 200, 90), c(250, 240, 180)] {
            for accent in [c(255, 255, 0), c(0, 0, 0), c(20, 20, 200), c(255, 255, 255)] {
                for (display, text) in [(None, None), (Some(c(40, 40, 40)), Some(c(60, 60, 60))), (Some(c(250, 250, 250)), None)] {
                    let look = CustomLook { background: bg, accent, display, text };
                    let (_, display, _, text) = look.colors();
                    let (p, _) = crate::genskin::wear_with(&look.pal(), text).expect("skin");
                    let p = readable(p, true);
                    assert!(contrast(p.ink, p.display) >= 4.5, "{:?} {:?} {:?}", bg, accent, display);
                    assert!(contrast(p.ink, p.panel) >= 4.5, "{:?} {:?} panel", bg, accent);
                }
            }
        }
    }

    #[test]
    fn an_old_look_finds_its_new_one() {
        assert_eq!(SKIN_NAMES[look_from_old(10)], "MIDNIGHT");
        assert_eq!(SKIN_NAMES[look_from_old(9)], "BLUE HOUR");
        assert_eq!(SKIN_NAMES[look_from_old(2)], "GRAPHITE");
        assert_eq!(SKIN_NAMES[look_from_old(16)], "HIGH CONTRAST LIGHT");
        assert_eq!(look_from_old(99), FIRST_LOOK);
        // 9.356 kept a number in its own list of fourteen
        assert_eq!(SKIN_NAMES[look_from_356(13)], "HIGH CONTRAST LIGHT");
        assert_eq!(SKIN_NAMES[look_from_356(1)], "MIDNIGHT");
        assert_eq!(look_named("sea glass").map(|n| SKIN_NAMES[n]), Some("SEA GLASS"));
    }
}

#[cfg(test)]
mod readable_tests {
    use super::*;

    #[test]
    fn light_text_on_light_menus_becomes_readable() {
        let grey = c(200, 200, 205);
        let p = Pal {
            app_bg: c(20, 20, 24),
            accent: grey,
            panel: c(245, 245, 245),
            panel_lt: c(240, 240, 240),
            panel_dk: c(220, 220, 220),
            panel_hover: c(230, 230, 230),
            display: c(10, 10, 12),
            display_ghost: grey,
            groove: grey,
            sel: c(235, 235, 235),
            ink: c(190, 195, 215),
            ink2: c(60, 60, 70),
            dim: c(40, 40, 45),
            red: c(220, 60, 60),
            btn_face: c(225, 225, 225),
            btn_hi: grey,
            row_alt: c(25, 25, 30),
            row_sel: c(60, 60, 80),
            bar_txt: c(70, 70, 90),
            edge: grey,
        };
        let r = readable(p, true);
        for bg in [r.app_bg, r.display, r.panel, r.panel_lt, r.btn_face, r.sel] {
            assert!(contrast(r.ink, bg) >= 4.5, "{:?} on {:?}", r.ink, bg);
        }
        assert!(contrast(r.ink2, r.app_bg) >= 3.0 && contrast(r.dim, r.display) >= 2.2);
        assert!(contrast(r.bar_txt, r.row_sel) >= 4.5);
    }
}
