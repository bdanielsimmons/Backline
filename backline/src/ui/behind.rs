//! What's behind symbols drawn straight on a picture (Now Playing's controls over the visuals or the cover's colours):
//! the colour already drawn under each spot, read back from the graphics card a few times a second, so each symbol
//! can take a colour that stands out from it (`stand_out`) instead of sitting on a box.

use eframe::egui::{self, Color32, Rect};
use eframe::egui_glow;
use eframe::glow::{self, HasContext};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// each spot's colour as last read (linear RGB), and as shown now (easing toward the last read)
static READ: Mutex<Option<HashMap<egui::Id, [f32; 3]>>> = Mutex::new(None);
static SHOWN: Mutex<Option<HashMap<egui::Id, [f32; 3]>>> = Mutex::new(None);
/// when the last read was asked for
static ASKED: Mutex<f64> = Mutex::new(f64::NEG_INFINITY);

/// Read what's been drawn so far under each spot, at most 4 times a second. Called after the picture and before the
/// symbols, on the painter they're drawn with (the read happens where it falls among the things drawn).
pub(crate) fn probe(p: &egui::Painter, now: f64, spots: Vec<(egui::Id, Rect)>) {
    {
        let Ok(mut asked) = ASKED.lock() else { return };
        if (0.0..0.25).contains(&(now - *asked)) {
            return;
        }
        *asked = now;
    }
    let Some(area) = spots.iter().map(|s| s.1).reduce(|a, b| a.union(b)) else { return };
    let cb = egui_glow::CallbackFn::new(move |info, painter| {
        let got = unsafe { read(painter.gl(), &info, &spots) };
        if let Ok(mut r) = READ.lock() {
            r.get_or_insert_with(HashMap::new).extend(got);
        }
    });
    p.add(egui::PaintCallback { rect: area, callback: Arc::new(cb) });
}

/// Each spot's average colour (linear RGB), from the picture egui is drawing into.
unsafe fn read(gl: &glow::Context, info: &egui::PaintCallbackInfo, spots: &[(egui::Id, Rect)]) -> Vec<(egui::Id, [f32; 3])> {
    let ppp = info.pixels_per_point;
    let [sw, sh] = info.screen_size_px.map(|v| v as i32);
    let mut out = Vec::new();
    for (id, r) in spots {
        let x0 = ((r.min.x * ppp).floor() as i32).max(0);
        let x1 = ((r.max.x * ppp).ceil() as i32).min(sw);
        let y0 = ((r.min.y * ppp).floor() as i32).max(0);
        let y1 = ((r.max.y * ppp).ceil() as i32).min(sh);
        let (w, h) = (x1 - x0, y1 - y0);
        if w <= 0 || h <= 0 {
            continue;
        }
        let mut px = vec![0u8; (w * h * 4) as usize];
        // (rows count up from the bottom)
        gl.read_pixels(x0, sh - y1, w, h, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(&mut px));
        let mut sum = [0f32; 3];
        for c in px.chunks_exact(4) {
            for k in 0..3 {
                sum[k] += lin(c[k]);
            }
        }
        let n = (w * h) as f32;
        out.push((*id, sum.map(|s| s / n)));
    }
    out
}

/// The colour under a spot, easing toward the last read (so it never jumps), and whether it's still on its way.
/// None until a spot has been read.
pub(crate) fn behind(id: egui::Id, dt: f32) -> Option<(Color32, bool)> {
    let target = READ.lock().ok()?.as_ref()?.get(&id).copied()?;
    let mut shown = SHOWN.lock().ok()?;
    let s = shown.get_or_insert_with(HashMap::new).entry(id).or_insert(target);
    let k = 1.0 - (-dt * 5.0).exp();
    let mut moving = false;
    for i in 0..3 {
        s[i] += (target[i] - s[i]) * k;
        if (target[i] - s[i]).abs() > 0.002 {
            moving = true;
        } else {
            s[i] = target[i];
        }
    }
    Some((Color32::from_rgb(enc(s[0]), enc(s[1]), enc(s[2])), moving))
}

fn lin(v: u8) -> f32 {
    let c = v as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn enc(l: f32) -> u8 {
    let l = l.clamp(0.0, 1.0);
    let c = if l <= 0.0031308 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 };
    (c * 255.0).round() as u8
}

/// How bright a colour looks (relative luminance, 0 black to 1 white).
fn lum(c: Color32) -> f32 {
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

/// How much two colours stand apart: 1 (the same) to 21 (black on white). 3 is the least for a symbol to read,
/// 4.5 for text.
pub(crate) fn contrast(a: Color32, b: Color32) -> f32 {
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `ink` as it is when it stands out from `bg` by `need`; else moved just far enough toward black or white (whichever
/// stands out more from `bg`) that it does, so it keeps as much of its own colour as it can.
pub(crate) fn stand_out(ink: Color32, bg: Color32, need: f32) -> Color32 {
    if contrast(ink, bg) >= need {
        return ink;
    }
    let to = if contrast(Color32::WHITE, bg) >= contrast(Color32::BLACK, bg) { Color32::WHITE } else { Color32::BLACK };
    let mix = |t: f32| {
        let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Color32::from_rgb(m(ink.r(), to.r()), m(ink.g(), to.g()), m(ink.b(), to.b()))
    };
    // the least step that's enough (or all the way, when even black or white isn't)
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..14 {
        let m = (lo + hi) / 2.0;
        if contrast(mix(m), bg) >= need {
            hi = m;
        } else {
            lo = m;
        }
    }
    mix(hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_stand_out_from_any_backdrop() {
        let ink2 = Color32::from_rgb(170, 170, 176);
        // on the dark backdrop: left as the skin has it
        assert_eq!(stand_out(ink2, Color32::from_rgb(12, 12, 16), 4.5), ink2);
        // on light fog: darkened just enough
        let fog = Color32::from_rgb(196, 198, 200);
        let c = stand_out(ink2, fog, 4.5);
        assert!(contrast(c, fog) >= 4.5 && lum(c) < lum(ink2));
        // a dark skin's ink on a dark picture: lightened
        let dark_ink = Color32::from_rgb(40, 40, 60);
        let night = Color32::from_rgb(20, 20, 30);
        let c = stand_out(dark_ink, night, 4.5);
        assert!(contrast(c, night) >= 4.5 && lum(c) > lum(dark_ink));
        // the hardest grey still gets there, one way or the other
        for v in 0..=255u8 {
            let bg = Color32::from_rgb(v, v, v);
            assert!(contrast(stand_out(ink2, bg, 4.5), bg) >= 4.5, "grey {}", v);
        }
    }

    #[test]
    fn colours_round_trip() {
        for v in 0..=255u8 {
            assert_eq!(enc(lin(v)), v);
        }
    }
}
