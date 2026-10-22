//! Classic Winamp skins from the Winamp Skin Museum (skins.webamp.org): list and search them, download one,
//! and turn it into a Backline palette. The colours come from the skin itself: its playlist colours (pledit.txt)
//! for the lists and screens, its visualizer colours (viscolor.txt) for the accent, and the overall tone of its
//! main window (main.bmp) for the panels, adjusted so text always stays readable.
//! Skins belong to their makers: nothing is bundled, each one is fetched when you pick it and kept on disk.

use crate::skin::Pal;
use eframe::egui::Color32;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

const API: &str = "https://api.webamp.org/graphql";

#[derive(Clone, Debug, PartialEq)]
pub struct WaSkin {
    pub md5: String,
    /// readable name from the file name ("Link-ZeldAMP_Version_2.wsz" -> "Link ZeldAMP Version 2")
    pub name: String,
    pub shot: String,
    pub download: String,
}

fn http() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Backline (desktop music player)")
        .build()
        .map_err(|e| e.to_string())
}

pub fn pretty_name(file: &str) -> String {
    let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
    stem.replace(['_', '-'], " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The skins in one answer from the museum (a page of the list, or search results).
fn parse_skins(v: &Value) -> Vec<WaSkin> {
    let d = &v["data"];
    let nodes = d["skins"]["nodes"].as_array().or_else(|| d["search_skins"].as_array());
    nodes
        .map(|a| {
            a.iter()
                .filter_map(|n| {
                    Some(WaSkin {
                        md5: n["md5"].as_str()?.to_string(),
                        name: pretty_name(n["filename"].as_str().unwrap_or("skin")),
                        shot: n["screenshot_url"].as_str()?.to_string(),
                        download: n["download_url"].as_str().unwrap_or("").to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A page of skins, best first (the museum's own order), or the results of a search.
pub fn list(query: &str, offset: usize) -> Result<Vec<WaSkin>, String> {
    const FIELDS: &str = "md5 filename screenshot_url download_url";
    let q = query.trim();
    let gql = if q.is_empty() {
        format!("{{ skins(first: 30, offset: {}, sort: MUSEUM) {{ nodes {{ {} }} }} }}", offset, FIELDS)
    } else {
        let safe: String = q.chars().filter(|c| !matches!(c, '"' | '\\')).collect();
        format!("{{ search_skins(query: \"{}\", first: 30, offset: {}) {{ {} }} }}", safe, offset, FIELDS)
    };
    let body = serde_json::json!({ "query": gql });
    let v: Value = http()?.post(API).json(&body).send().map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())?;
    if let Some(e) = v["errors"][0]["message"].as_str() {
        return Err(e.to_string());
    }
    Ok(parse_skins(&v))
}

pub fn dir() -> PathBuf {
    crate::api::config_dir().join("skins")
}

/// The skin file, from disk if it was fetched before.
pub fn fetch(s: &WaSkin) -> Result<Vec<u8>, String> {
    let path = dir().join(format!("{}.wsz", s.md5));
    if let Ok(b) = std::fs::read(&path) {
        return Ok(b);
    }
    let url = if s.download.is_empty() { format!("https://r2.webampskins.org/skins/{}.wsz", s.md5) } else { s.download.clone() };
    let b = http()?
        .get(&url)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .map_err(|e| e.to_string())?;
    let _ = std::fs::create_dir_all(dir());
    let _ = std::fs::write(&path, &b);
    Ok(b.to_vec())
}

/// A skin already on disk (to bring it back at start-up).
pub fn load_saved(md5: &str) -> Option<Vec<u8>> {
    std::fs::read(dir().join(format!("{}.wsz", md5))).ok()
}

/// The few files we read from a skin, by lower-case name (skins come from many tools: names and folders vary).
fn files(wsz: &[u8]) -> Result<HashMap<String, Vec<u8>>, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(wsz)).map_err(|e| format!("not a skin file ({})", e))?;
    let mut out = HashMap::new();
    for i in 0..zip.len() {
        let Ok(mut f) = zip.by_index(i) else { continue };
        let name = f.name().rsplit(['/', '\\']).next().unwrap_or("").to_ascii_lowercase();
        if name.ends_with(".bmp") || ["pledit.txt", "viscolor.txt"].contains(&name.as_str()) {
            let mut b = Vec::new();
            if f.read_to_end(&mut b).is_ok() {
                out.insert(name, b);
            }
        }
    }
    Ok(out)
}

fn hex(s: &str) -> Option<Color32> {
    let h = s.trim().trim_start_matches('#');
    if h.len() < 6 {
        return None;
    }
    let p = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    Some(Color32::from_rgb(p(0)?, p(2)?, p(4)?))
}

/// pledit.txt: Normal, Current, NormalBG, SelectedBG (lower-case keys).
fn pledit(text: &str) -> HashMap<String, Color32> {
    text.lines()
        .filter_map(|l| {
            let (k, v) = l.split_once('=')?;
            Some((k.trim().to_ascii_lowercase(), hex(v)?))
        })
        .collect()
}

/// viscolor.txt: 24 lines of "r,g,b" (anything after the numbers is a comment).
fn viscolor(text: &str) -> Vec<Color32> {
    text.lines()
        .filter_map(|l| {
            let n: Vec<u8> =
                l.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).take(3).filter_map(|s| s.parse().ok()).collect();
            (n.len() == 3).then(|| Color32::from_rgb(n[0], n[1], n[2]))
        })
        .collect()
}

/// `c`, if brighter than `max`, brought down to it: the same hue, just less glare.
fn soften(c: Color32, max: f32) -> Color32 {
    let l = lum(c);
    if l <= max {
        return c;
    }
    let k = max / l;
    Color32::from_rgb((c.r() as f32 * k) as u8, (c.g() as f32 * k) as u8, (c.b() as f32 * k) as u8)
}

pub(crate) fn lum(c: Color32) -> f32 {
    (0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32) / 255.0
}

pub use crate::skin::contrast;

/// `c` moved toward white or black (whichever ends up further from `other`) until the two read at `min`
/// contrast, changing it as little as possible.
fn apart(c: Color32, other: Color32, min: f32) -> Color32 {
    if contrast(c, other) >= min {
        return c;
    }
    let to_white = contrast(Color32::WHITE, other) >= contrast(Color32::BLACK, other);
    let mut out = c;
    for i in 1..=20 {
        let t = i as f32 / 20.0;
        out = if to_white { lighter(c, t) } else { darker(c, t) };
        if contrast(out, other) >= min {
            break;
        }
    }
    out
}

/// A text colour that reads on `bg`.
pub(crate) fn readable(fg: Color32, bg: Color32, min: f32) -> Color32 {
    apart(fg, bg, min)
}

/// A background (panel, button) that `fg` reads on.
fn backing(bg: Color32, fg: Color32, min: f32) -> Color32 {
    // a dark panel stays dark (a light one light) when it can still read that way: darkened or lightened just enough
    // - only when it can't does it cross over
    let dark = lum(bg) < 0.5;
    for i in 0..=20 {
        let t = i as f32 / 20.0;
        let c = if dark { darker(bg, t) } else { lighter(bg, t) };
        // (never lightened into a glare: past soft paper, it goes the other way instead)
        if !dark && lum(c) > 0.8 {
            break;
        }
        if contrast(c, fg) >= min {
            return c;
        }
    }
    // the other way: darker for a light one, lighter for a dark one
    for i in 0..=20 {
        let t = i as f32 / 20.0;
        let c = if dark { lighter(bg, t) } else { darker(bg, t) };
        if contrast(c, fg) >= min && lum(c) <= 0.8 {
            return c;
        }
    }
    apart(bg, fg, min)
}

pub(crate) fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

pub(crate) fn lighter(c: Color32, t: f32) -> Color32 {
    mix(c, Color32::WHITE, t)
}

pub(crate) fn darker(c: Color32, t: f32) -> Color32 {
    mix(c, Color32::BLACK, t)
}

/// Main window tones: (average, a dark one, a light one), from main.bmp.
fn tones(bmp: &[u8]) -> Option<(Color32, Color32, Color32)> {
    let img = image::load_from_memory_with_format(bmp, image::ImageFormat::Bmp).ok()?.to_rgb8();
    let mut px: Vec<Color32> = img.pixels().map(|p| Color32::from_rgb(p[0], p[1], p[2])).collect();
    if px.is_empty() {
        return None;
    }
    let n = px.len() as f32;
    let (r, g, b) = px.iter().fold((0f32, 0f32, 0f32), |a, c| (a.0 + c.r() as f32, a.1 + c.g() as f32, a.2 + c.b() as f32));
    let avg = Color32::from_rgb((r / n) as u8, (g / n) as u8, (b / n) as u8);
    px.sort_by(|a, b| lum(*a).total_cmp(&lum(*b)));
    Some((avg, px[px.len() / 20], px[px.len() * 9 / 10]))
}

/// The skin's own pictures, ready to draw: each sheet (by name, e.g. "main", "cbuttons") as RGBA pixels, plus
/// its visualizer colours and the colours of the equalizer's curve.
pub struct Art {
    pub sheets: Vec<(String, [usize; 2], Vec<u8>)>,
    pub vis: Vec<Color32>,
    pub graph: Vec<Color32>,
    /// the playlist font the skin asks for (pledit.txt Font=), e.g. "Tahoma"
    pub font: Option<String>,
    /// the EQ and PL buttons are blank or melt into the window behind them: drawn with a plain outline and
    /// letters in this colour so they can be found (None: the skin's own are fine)
    pub eqpl_ink: Option<Color32>,
    /// the balance picture is the volume one (copied in, or none at all): a strip filled from the left
    pub bal_is_vol: bool,
    /// the balance pictures that make a smooth run from the middle out (empty: all of them)
    pub bal_frames: Vec<usize>,
}

/// The balance pictures to show, from the middle out: a few stray ones that look nothing like the run around them
/// (a flash between the steps, drawn for Winamp's coarse slider, which never lands on them) are left out, so moving
/// it is a smooth run. Empty when every picture belongs (most skins), or when most would be left out (then the
/// pictures are an animation in their own right).
fn balance_frames(sheets: &[(String, [usize; 2], Vec<u8>)]) -> Vec<usize> {
    let Some(b) = sheets.iter().find(|s| s.0 == "balance") else { return Vec::new() };
    let n = ((b.1[1] + 2) / 15).min(28);
    if b.1[0] < 47 || n < 3 {
        return Vec::new();
    }
    // the share of the picture (x 9..47, 13 high) that differs between two frames
    let changed = |f: usize, g: usize| -> f32 {
        let mut d = 0;
        for y in 0..13 {
            for x in 9..47 {
                let (i, j) = (((f * 15 + y) * b.1[0] + x) * 4, ((g * 15 + y) * b.1[0] + x) * 4);
                let diff: i32 = (0..3).map(|k| (b.2[i + k] as i32 - b.2[j + k] as i32).abs()).sum();
                if diff > 60 {
                    d += 1;
                }
            }
        }
        d as f32 / (13.0 * 38.0)
    };
    let mut keep = vec![0];
    for f in 1..n {
        if changed(*keep.last().unwrap_or(&0), f) <= 0.35 {
            keep.push(f);
        }
    }
    // only strays scattered through the run (at least three, each with kept pictures either side) - a different
    // last picture or a single odd one is the artist's choice
    let last = *keep.last().unwrap_or(&0);
    let strays = (1..last).filter(|f| !keep.contains(f)).count();
    if strays < 3 || keep.len() * 2 < n {
        Vec::new()
    } else {
        keep
    }
}

/// Is the balance picture really the volume one - missing, or copied in (most of its level pictures the same)?
fn balance_is_volume(sheets: &[(String, [usize; 2], Vec<u8>)]) -> bool {
    let get = |name: &str| sheets.iter().find(|s| s.0 == name);
    let (Some(b), Some(v)) = (get("balance"), get("volume")) else { return true };
    // the part both draw from (x 9..47 of the level pictures)
    let h = b.1[1].min(v.1[1]).min(420);
    if b.1[0] < 47 || v.1[0] < 47 || h < 30 {
        return false;
    }
    let (mut same, mut all) = (0u32, 0u32);
    for y in (0..h).step_by(3) {
        for x in 9..47 {
            let (i, j) = ((y * b.1[0] + x) * 4, (y * v.1[0] + x) * 4);
            all += 1;
            if b.2[i..i + 3] == v.2[j..j + 3] {
                same += 1;
            }
        }
    }
    same * 10 >= all * 9
}

/// Can the EQ and PL buttons be seen? None if so; if not, a colour that stands out on the window behind them.
fn faint_eqpl(sheets: &[(String, [usize; 2], Vec<u8>)]) -> Option<Color32> {
    let get = |name: &str| sheets.iter().find(|s| s.0 == name);
    let main = get("main")?;
    let px = |s: &(String, [usize; 2], Vec<u8>), x: usize, y: usize| -> Option<[u8; 4]> {
        if x >= s.1[0] || y >= s.1[1] {
            return None;
        }
        let i = (y * s.1[0] + x) * 4;
        Some([s.2[i], s.2[i + 1], s.2[i + 2], s.2[i + 3]])
    };
    // the window behind the two buttons: light or dark
    let mut sum = 0.0;
    let mut n = 0.0;
    for y in 58..70 {
        for x in 219..265 {
            if let Some(c) = px(main, x, y) {
                sum += lum(Color32::from_rgb(c[0], c[1], c[2]));
                n += 1.0;
            }
        }
    }
    let ink = if n > 0.0 && sum / n > 0.5 { Color32::BLACK } else { Color32::WHITE };
    let Some(shuf) = get("shufrep") else { return Some(ink) };
    // each button (off): with next to no light and dark in it (a flat or near-flat patch) it is lost
    for sx in [0usize, 23] {
        let (mut lo, mut hi) = (1.0f32, 0.0f32);
        for y in 0..12 {
            for x in 0..23 {
                let Some(c) = px(shuf, sx + x, 61 + y) else { return Some(ink) };
                let l = lum(Color32::from_rgb(c[0], c[1], c[2]));
                lo = lo.min(l);
                hi = hi.max(l);
            }
        }
        if hi - lo < 0.08 {
            return Some(ink);
        }
    }
    None
}

/// The sheets the main window and the equalizer are drawn from.
const SHEETS: [&str; 14] = [
    "main", "cbuttons", "titlebar", "numbers", "nums_ex", "text", "posbar", "volume", "balance", "shufrep", "playpaus",
    "monoster", "eqmain", "pledit",
];

/// Colours and pictures from a skin file.
pub fn load(wsz: &[u8]) -> Result<(Pal, Art), String> {
    let f = files(wsz)?;
    let pal = palette_from(&f)?;
    let mut sheets = Vec::new();
    let mut graph = Vec::new();
    // pictures a skin leaves out are borrowed, as Winamp borrows from its base skin: from Backline's own skin made in
    // this skin's colours, so a skin with no equalizer or playlist still gets whole ones that match it
    let lacks = |n: &str| !f.contains_key(&format!("{}.bmp", n)) && !(n == "numbers" && f.contains_key("nums_ex.bmp"));
    let base: HashMap<String, Vec<u8>> = if SHEETS.iter().any(|n| *n != "nums_ex" && *n != "monoster" && lacks(n)) {
        files(&crate::genskin::make(&pal, 1)).unwrap_or_default()
    } else {
        HashMap::new()
    };
    for name in SHEETS {
        let own = f.get(&format!("{}.bmp", name));
        let Some(b) = own.or_else(|| if lacks(name) { base.get(&format!("{}.bmp", name)) } else { None }) else { continue };
        let Ok(img) = image::load_from_memory_with_format(b, image::ImageFormat::Bmp) else { continue };
        let mut img = img.to_rgba8();
        if name == "text" && img.width() > 150 {
            // the font comes on a solid background (the colour of the space character): make it see-through,
            // so the letters sit on anything
            let bg = *img.get_pixel(150, 0);
            for px in img.pixels_mut() {
                if px[0] == bg[0] && px[1] == bg[1] && px[2] == bg[2] {
                    px[3] = 0;
                }
            }
        }
        let size = [img.width() as usize, img.height() as usize];
        if name == "eqmain" && img.width() > 115 && img.height() >= 313 {
            // the curve's colours: a one-pixel column, top to bottom
            graph = (294..313)
                .map(|y| {
                    let p = img.get_pixel(115, y);
                    Color32::from_rgb(p[0], p[1], p[2])
                })
                .collect();
        }
        sheets.push((name.to_string(), size, img.into_raw()));
    }
    let mut vis = f.get("viscolor.txt").map(|b| viscolor(&String::from_utf8_lossy(b))).unwrap_or_default();
    if vis.len() < 24 {
        // Winamp's own visualizer colours, as Winamp uses when a skin brings none
        const DEFAULT: [(u8, u8, u8); 24] = [
            (0, 0, 0),
            (24, 33, 41),
            (239, 49, 16),
            (206, 41, 16),
            (214, 90, 0),
            (214, 102, 0),
            (214, 115, 0),
            (198, 123, 8),
            (222, 165, 24),
            (214, 181, 33),
            (189, 222, 41),
            (148, 222, 33),
            (41, 206, 16),
            (50, 190, 16),
            (57, 181, 16),
            (49, 156, 8),
            (41, 148, 0),
            (24, 132, 8),
            (255, 255, 255),
            (214, 214, 222),
            (181, 189, 189),
            (160, 170, 175),
            (148, 156, 165),
            (150, 150, 150),
        ];
        vis = DEFAULT.iter().map(|c| Color32::from_rgb(c.0, c.1, c.2)).collect();
    }
    let font = f.get("pledit.txt").and_then(|b| {
        String::from_utf8_lossy(b).lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim().eq_ignore_ascii_case("font") && !v.trim().is_empty()).then(|| v.trim().to_string())
        })
    });
    let eqpl_ink = faint_eqpl(&sheets);
    let bal_is_vol = balance_is_volume(&sheets);
    let bal_frames = balance_frames(&sheets);
    Ok((pal, Art { sheets, vis, graph, font, eqpl_ink, bal_is_vol, bal_frames }))
}

/// A Backline palette from a skin file.
#[cfg(test)]
pub fn palette(wsz: &[u8]) -> Result<Pal, String> {
    palette_from(&files(wsz)?)
}

fn palette_from(f: &HashMap<String, Vec<u8>>) -> Result<Pal, String> {
    if f.is_empty() {
        return Err("this skin has none of the usual files".to_string());
    }
    let pl = f.get("pledit.txt").map(|b| pledit(&String::from_utf8_lossy(b))).unwrap_or_default();
    let vis = f.get("viscolor.txt").map(|b| viscolor(&String::from_utf8_lossy(b))).unwrap_or_default();
    // Winamp's own defaults when a skin leaves something out
    let normal = pl.get("normal").copied().unwrap_or(Color32::from_rgb(0, 255, 0));
    let current = pl.get("current").copied().unwrap_or(Color32::WHITE);
    // (a white list background is fine in a small playlist; spread over the library and the panels it glares: the big
    // areas keep the skin's tint but never come brighter than a soft paper grey)
    let normal_bg = soften(pl.get("normalbg").copied().unwrap_or(Color32::BLACK), 0.80);
    let sel_bg = pl.get("selectedbg").copied().unwrap_or(Color32::from_rgb(0, 0, 198));
    let (chrome, dark, light) = f.get("main.bmp").and_then(|b| tones(b)).unwrap_or((
        Color32::from_gray(58),
        Color32::from_gray(16),
        Color32::from_gray(150),
    ));
    // text first: the list text and the playing song's text must read on the list background (and the playing
    // song's also on the selection colour), whatever the skin chose
    let ink = readable(normal, normal_bg, 4.5);
    let bar_txt = readable(current, normal_bg, 4.5);
    // the selection colour bends to the playing text, not the other way round
    let sel_bg = backing(sel_bg, bar_txt, 4.5);
    // panels and buttons take the skin's tones, moved just far enough from the text to read
    let beige = soften(backing(chrome, ink, 4.5), 0.78);
    // (softened, a panel might sit too close to the text again)
    let beige = backing(beige, ink, 4.5);
    let accent = vis.get(2).copied().filter(|c| lum(*c) > 0.15).unwrap_or(current);
    let (btn, btn_light) = match f.get("cbuttons.bmp").and_then(|b| tones(b)) {
        Some((avg, _, light)) => {
            let b = backing(avg, ink, 4.5);
            (b, mix(light, b, 0.3))
        }
        None => {
            let b = backing(darker(beige, 0.1), ink, 4.5);
            (b, mix(Color32::WHITE, b, 0.7))
        }
    };
    let row_alt = if lum(normal_bg) < 0.5 { lighter(normal_bg, 0.05) } else { darker(normal_bg, 0.05) };
    let edge = darker(dark, 0.4);
    // header text (trim) sits on the dark edge colour
    let trim = readable(mix(light, beige, 0.4), edge, 4.5);
    Ok(Pal {
        app_bg: darker(chrome, 0.8),
        trim,
        beige,
        beige_lt: lighter(beige, 0.12),
        beige_dk: darker(beige, 0.15),
        beige_h: lighter(beige, 0.06),
        lcd: normal_bg,
        lcd_ghost: mix(normal_bg, normal, 0.12),
        groove: mix(normal_bg, dark, 0.5),
        sel: mix(sel_bg, normal_bg, 0.5),
        ink,
        ink2: readable(mix(ink, normal_bg, 0.3), normal_bg, 4.5),
        dim: readable(mix(ink, normal_bg, 0.55), normal_bg, 3.0),
        red: accent,
        btn_face: btn,
        btn_hi: btn_light,
        row_alt,
        row_sel: sel_bg,
        bar_txt,
        edge,
    })
}

#[cfg(test)]
mod tests {

    /// One skin's whole palette ($BACKLINE_SKIN = a .wsz), each colour's brightness (run with --ignored --nocapture).
    #[test]
    #[ignore]
    fn print_palette() {
        let Ok(f) = std::env::var("BACKLINE_SKIN") else { return };
        let (p, _) = load(&std::fs::read(f).unwrap()).unwrap();
        for (n, c) in [
            ("app_bg", p.app_bg),
            ("beige", p.beige),
            ("beige_lt", p.beige_lt),
            ("beige_dk", p.beige_dk),
            ("beige_h", p.beige_h),
            ("lcd", p.lcd),
            ("lcd_ghost", p.lcd_ghost),
            ("groove", p.groove),
            ("sel", p.sel),
            ("ink", p.ink),
            ("ink2", p.ink2),
            ("dim", p.dim),
            ("btn_face", p.btn_face),
            ("btn_hi", p.btn_hi),
            ("row_alt", p.row_alt),
            ("row_sel", p.row_sel),
            ("bar_txt", p.bar_txt),
            ("edge", p.edge),
            ("trim", p.trim),
        ] {
            println!("{:10} {:?} {:.2}", n, c, lum(c));
        }
    }

    /// Every skin in $BACKLINE_SKINS (a folder of .wsz): what each lacks of the windows Backline draws, and how bright
    /// its big areas come out (run with --ignored --nocapture).
    #[test]
    #[ignore]
    fn audit_skins() {
        let Ok(dir) = std::env::var("BACKLINE_SKINS") else { return };
        let need =
            ["main", "titlebar", "cbuttons", "eqmain", "pledit", "text", "numbers", "posbar", "volume", "shufrep", "playpaus"];
        let (mut n, mut bad) = (0, 0);
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let b = std::fs::read(e.path()).unwrap();
            n += 1;
            match load(&b) {
                Err(err) => {
                    bad += 1;
                    println!("FAIL {:?}: {}", e.file_name(), err);
                }
                Ok((pal, art)) => {
                    let have: Vec<&str> = art.sheets.iter().map(|s| s.0.as_str()).collect();
                    let miss: Vec<&str> = need
                        .iter()
                        .copied()
                        .filter(|x| !have.contains(x) && !(*x == "numbers" && have.contains(&"nums_ex")))
                        .collect();
                    // as worn: after the readability fix every big surface stays on the list's side
                    let worn = crate::skin::readable(pal, true);
                    let dark = lum(worn.lcd) < 0.5;
                    // (a mid-grey panel is a skin's own choice; white behind a dark list is a flip. The space round
                    // the windows is always dark, so it isn't counted for light skins)
                    let flipped = [worn.app_bg, worn.beige, worn.row_alt, worn.sel, worn.row_sel, worn.btn_face]
                        .iter()
                        .enumerate()
                        .any(|(i, c)| if dark { lum(*c) > 0.62 } else { i > 0 && lum(*c) < 0.2 });
                    let glare = lum(worn.lcd) > 0.82 || lum(worn.beige) > 0.85 || lum(worn.app_bg) > 0.85 || flipped;
                    if !miss.is_empty() || glare {
                        bad += 1;
                        println!(
                            "{:?}: missing {:?} lcd {:.2} panel {:.2} back {:.2}{}",
                            e.file_name(),
                            miss,
                            lum(pal.lcd),
                            lum(pal.beige),
                            lum(pal.app_bg),
                            if glare { "  GLARE" } else { "" }
                        );
                    }
                }
            }
        }
        println!("{} skins, {} with something to look at", n, bad);
    }
    use super::*;
    use std::io::Write;

    fn wsz(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            for (name, data) in files {
                z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
                z.write_all(data).unwrap();
            }
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn names() {
        assert_eq!(pretty_name("Link-ZeldAMP_Version_2.wsz"), "Link ZeldAMP Version 2");
        assert_eq!(pretty_name("base-2.91.wsz"), "base 2.91");
    }

    #[test]
    fn reads_museum_answers() {
        let v = serde_json::json!({"data": {"search_skins": [
            {"md5": "abc", "filename": "Cool_Skin.wsz", "screenshot_url": "http://s/abc.png", "download_url": "http://d/abc.wsz"},
            {"md5": "nope"}
        ]}});
        let s = parse_skins(&v);
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].name.as_str(), s[0].shot.as_str()), ("Cool Skin", "http://s/abc.png"));
        let v = serde_json::json!({"data": {"skins": {"nodes": [{"md5": "x", "filename": "a.wsz", "screenshot_url": "u"}]}}});
        assert_eq!(parse_skins(&v)[0].md5, "x");
    }

    #[test]
    fn colour_files() {
        let pl = pledit("[Text]\r\nNormal=#00FF00\r\nCurrent=#FFFFFF\nNormalBG=#000000\nSelectedBG=#0000C6\nFont=Arial");
        assert_eq!(pl["normal"], Color32::from_rgb(0, 255, 0));
        assert_eq!(pl["selectedbg"], Color32::from_rgb(0, 0, 198));
        let v = viscolor("0,0,0, // background\n24,33,41, // dots\n239,49,16, // top of spectrum\nbad line");
        assert_eq!(v, vec![Color32::BLACK, Color32::from_rgb(24, 33, 41), Color32::from_rgb(239, 49, 16)]);
    }

    #[test]
    fn every_text_colour_reads() {
        // the hard cases real skins have: mid-red text, text the same as its background, a pale selection
        // behind white "playing" text, dark text on dark panels
        for (normal, current, bg, sel) in [
            ("#C80000", "#FFFFFF", "#000000", "#E0E0E0"),
            ("#202020", "#202020", "#202020", "#202020"),
            ("#808080", "#FFFF00", "#7F7F7F", "#FFFFFF"),
            ("#00FF00", "#00C000", "#00FF00", "#00FF00"),
            ("#000080", "#FFFFFF", "#F0F0F0", "#000080"),
        ] {
            let pledit = format!(
                "[Text]
Normal={}
Current={}
NormalBG={}
SelectedBG={}",
                normal, current, bg, sel
            );
            let p = palette(&wsz(&[("pledit.txt", pledit.as_bytes())])).unwrap();
            for (what, fg, back, min) in [
                ("list text", p.ink, p.lcd, 4.5),
                ("playing on list", p.bar_txt, p.lcd, 4.5),
                ("playing on selection", p.bar_txt, p.row_sel, 4.5),
                ("panel text", p.ink, p.beige, 4.5),
                ("button text", p.ink, p.btn_face, 4.5),
                ("header text", p.trim, p.edge, 4.5),
                ("second text", p.ink2, p.lcd, 3.0),
            ] {
                assert!(contrast(fg, back) >= min, "{} ({}): {:.2}", what, normal, contrast(fg, back));
            }
        }
        assert!((contrast(Color32::WHITE, Color32::BLACK) - 21.0).abs() < 0.1);
    }

    #[test]
    fn palette_from_a_skin() {
        // folder names and upper case vary between skins
        let f = wsz(&[
            ("MySkin/PLEDIT.TXT", b"[Text]\nNormal=#00E000\nCurrent=#FFFFFF\nNormalBG=#101010\nSelectedBG=#203080"),
            ("MySkin/VISCOLOR.TXT", b"0,0,0\n10,10,10\n240,60,20\n"),
        ]);
        let p = palette(&f).unwrap();
        assert_eq!(p.lcd, Color32::from_rgb(16, 16, 16));
        assert_eq!(p.ink, Color32::from_rgb(0, 224, 0));
        assert_eq!(p.row_sel, Color32::from_rgb(32, 48, 128));
        assert_eq!(p.red, Color32::from_rgb(240, 60, 20), "the spectrum's top colour is the accent");
        assert!(contrast(p.ink, p.beige) >= 4.5, "panel text stays readable");
        assert!(palette(&wsz(&[("readme.txt", b"hi")])).is_err());
        assert!(palette(b"not a zip").is_err());
    }
}
