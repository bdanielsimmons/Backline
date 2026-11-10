//! The app's own look outside any page: its window icon and the fonts it writes in (kept for its other windows).

use super::*;

pub(crate) fn app_icon() -> egui::IconData {
    let n = 32usize;
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let dx = x as f32 + 0.5 - 16.0;
            let dy = y as f32 + 0.5 - 16.0;
            let d = (dx * dx + dy * dy).sqrt();
            let mut c: Option<[u8; 3]> = None;
            // black outside, white inside, a bright shine at the top left and a black play triangle
            if d <= 15.5 {
                c = Some([8, 8, 10]);
            }
            if d <= 14.0 {
                c = Some([250, 250, 250]);
            }
            if d <= 12.5 {
                c = Some([222, 222, 224]);
            }
            if d <= 9.0 && dy < -3.0 && dx < 0.0 && d > 6.0 {
                c = Some([255, 255, 255]);
            }
            let fx = x as f32 + 0.5;
            if (12.0..=23.0).contains(&fx) && dy.abs() <= (23.5 - fx) * 0.78 {
                c = Some([16, 16, 18]);
            }
            if let Some(c) = c {
                let i = (y * n + x) * 4;
                rgba[i] = c[0];
                rgba[i + 1] = c[1];
                rgba[i + 2] = c[2];
                rgba[i + 3] = 255;
            }
        }
    }
    egui::IconData { rgba, width: n as u32, height: n as u32 }
}

/// The fonts the app's window took last (a window of its own, drawn on another thread, reads in the same ones).
pub(crate) static FONTS: Mutex<Option<egui::FontDefinitions>> = Mutex::new(None);

/// `fonts` for the app's window (kept for its other windows).
pub(crate) fn set_fonts(ctx: &egui::Context, fonts: egui::FontDefinitions) {
    *FONTS.lock().unwrap_or_else(|e| e.into_inner()) = Some(fonts.clone());
    ctx.set_fonts(fonts);
}

/// The fonts the app's window took last.
pub(crate) fn fonts_now() -> egui::FontDefinitions {
    FONTS.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_else(|| smooth_fonts(None))
}

/// The smooth fonts: Chakra Petch (or `main`, e.g. Lexend, in front of it), then the system's fonts for any
/// character they lack.
pub(crate) fn smooth_fonts(main: Option<&'static [u8]>) -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    let chakra: &'static [u8] = include_bytes!("../../assets/ChakraPetch-Regular.ttf");
    fonts.font_data.insert("chakra".to_string(), egui::FontData::from_static(chakra));
    if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        f.insert(0, "chakra".to_string());
    }
    // a skin worn: its own font draws its playlist ("skin"); everything else reads in Inter, a face made for screens
    // (a skin's Arial and friends come out ragged at these sizes without Windows' hinting)
    let inter: &'static [u8] = include_bytes!("../../assets/Inter-Regular.ttf");
    let mut skin_fam = vec!["chakra".to_string()];
    if let Some(m) = main {
        fonts.font_data.insert("main".to_string(), egui::FontData::from_static(m));
        fonts.font_data.insert("inter".to_string(), egui::FontData::from_static(inter));
        if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            f.insert(0, "inter".to_string());
        }
        skin_fam.insert(0, "main".to_string());
    }
    let base = if main.is_some() { inter } else { chakra };
    for (name, bytes) in font::system_fallbacks() {
        // sit its letters on the same baseline as the main font's
        let tweak = font::baseline_tweak(base, bytes);
        fonts.font_data.insert(name.clone(), egui::FontData::from_static(bytes).tweak(tweak));
        for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            if let Some(f) = fonts.families.get_mut(&fam) {
                f.push(name.clone());
            }
        }
        skin_fam.push(name.clone());
    }
    fonts.families.insert(egui::FontFamily::Name("skin".into()), skin_fam);
    fonts
}
