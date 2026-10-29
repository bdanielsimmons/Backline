//! New Playlist: a panel to name a new playlist (in Backline or on your Tidal account), say what it is for, give it
//! a cover (a ready-made one, or a picture of your own, picked or dropped on the panel) and see it before making it.
//! Opened from a song's right-click menu, the playlist picker and the LISTS pages.

use crate::api::Track;
use crate::credits::panel;
use crate::{pal, para, retro_btn, retro_btn_w, title_line, Action, App, Tip};
use eframe::egui::{self, Sense, Vec2};

const F_NEW_NAME: u32 = 103;
const F_NEW_DESC: u32 = 104;

/// What the panel holds while it is open.
#[derive(Clone, Default)]
pub(crate) struct NewPl {
    /// the song it starts with (None: an empty playlist)
    pub track: Option<Track>,
    /// a loop of that song, when the playlist starts with a loop: (from, to, its name)
    pub loop_ab: Option<(f32, f32, String)>,
    pub name: String,
    /// a few words on what it is for
    pub desc: String,
    /// the picture picked (a file on this computer), empty for none
    pub art: String,
    /// on your Tidal account rather than in Backline
    pub tidal: bool,
}

/// A picture for a playlist, kept in Backline's folder (so it stays when the original moves): its new path.
pub(crate) fn keep_art(from: &str) -> Option<String> {
    let src = std::path::Path::new(from);
    let bytes = std::fs::read(src).ok()?;
    image::load_from_memory(&bytes).ok()?;
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("png").to_lowercase();
    let mut h: u64 = 0xcbf29ce484222325;
    for b in &bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let dir = crate::api::config_dir().join("playlist_art");
    std::fs::create_dir_all(&dir).ok()?;
    let to = dir.join(format!("{:016x}.{}", h, ext));
    std::fs::write(&to, &bytes).ok()?;
    Some(to.to_string_lossy().into_owned())
}

/// Ask for a picture file.
pub(crate) fn pick_art() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("Pick a picture for the playlist")
        .add_filter("Pictures", &["jpg", "jpeg", "png", "webp", "gif", "bmp"])
        .pick_file()
        .map(|p| p.to_string_lossy().into_owned())
}

/// How the image loader names a picture file.
pub(crate) fn art_url(path: &str) -> String {
    if path.is_empty() {
        String::new()
    } else {
        format!("img:{}", path)
    }
}

/// Is this file a picture (by its name)?
pub(crate) fn is_picture(p: &std::path::Path) -> bool {
    let e = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    ["jpg", "jpeg", "png", "webp", "gif", "bmp"].contains(&e.as_str())
}

impl App {
    /// Open the New Playlist panel, the name box ready to type in.
    pub(crate) fn open_new_playlist(&mut self, d: NewPl) {
        self.ed.id = F_NEW_NAME;
        self.ed.cur = d.name.chars().count();
        self.ed.anchor = Some(0);
        self.new_pl_dlg = Some(d);
    }

    /// Your pictures on the Tidal playlists you gave one: in a page's lists, and on the playlist's own page.
    pub(crate) fn tidal_art_on(&self, p: &mut crate::api::Page) {
        if self.store.tidal_art.is_empty() {
            return;
        }
        for (_, row) in p.rows.iter_mut() {
            for c in row.iter_mut().filter(|c| c.kind == crate::api::Kind::Playlist) {
                if let Some(a) = self.store.tidal_art.get(&c.id) {
                    c.image = art_url(a);
                }
            }
        }
        if let Some(a) = p.card.as_ref().filter(|c| c.kind == crate::api::Kind::Playlist).and_then(|c| self.store.tidal_art.get(&c.id)) {
            p.image = art_url(a);
        }
    }

    /// The New Playlist panel, while it is open.
    pub(crate) fn new_playlist_panel(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        let Some(mut d) = self.new_pl_dlg.take() else { return };
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut make = false;
        let tidal_ok = self.api.has_token() && d.loop_ab.is_none() && d.track.as_ref().map_or(true, |t| t.id > 0);
        if !tidal_ok {
            d.tidal = false;
        }
        let covers = samples();
        let url = art_url(&d.art);
        let pic = self.images.get(&url);
        let loading = !url.is_empty() && pic.is_none() && !self.images.failed(&url);
        let thumbs: Vec<Option<egui::TextureId>> = covers.iter().map(|c| self.images.get(&art_url(c))).collect();
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        let now = ctx.input(|i| i.time) as f32;
        panel(ctx, "new_playlist", 620.0, |ui| {
            title_line(ui, if d.tidal { "NEW TIDAL PLAYLIST" } else { "NEW PLAYLIST" }, 3.0, pal().ink);
            if let Some(t) = &d.track {
                let what = match &d.loop_ab {
                    Some((_, _, n)) => format!("Starts with the loop \"{}\" of {}", n, t.title),
                    None => format!("Starts with {} - {}", t.artists_text(), t.title),
                };
                para(ui, &what, pal().ink2);
            }
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                // the cover, as it will look; a picture can be dropped on it (or anywhere on the window)
                let side = 200.0;
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
                let p = ui.painter().clone();
                match pic {
                    Some(id) => {
                        egui::Image::new(egui::load::SizedTexture::new(id, r.size())).paint_at(ui, r);
                    }
                    None if loading => {
                        crate::paint_record(&p, r, Some(now));
                        ui.ctx().request_repaint();
                    }
                    None => {
                        p.rect_filled(r, 0.0, pal().ink2.gamma_multiply(0.12));
                        let mid = r.center();
                        crate::ptext_fit(&p, mid - Vec2::new(0.0, 10.0), egui::Align::Center, "DROP A PICTURE", 1.6, side - 20.0, pal().ink2);
                        crate::ptext_fit(&p, mid + Vec2::new(0.0, 12.0), egui::Align::Center, "or pick a cover below", 1.2, side - 20.0, pal().dim);
                    }
                }
                // a dashed edge while there is no cover yet, bright while a file is dragged over the window
                if pic.is_none() || hovering {
                    let col = if hovering || resp.hovered() { pal().ink } else { pal().edge };
                    let corners = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
                    for w in corners.windows(2) {
                        let (a, b) = (w[0], w[1]);
                        let n = ((b - a).length() / 8.0) as usize;
                        for k in (0..n).step_by(2) {
                            let (f0, f1) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
                            p.line_segment([a + (b - a) * f0, a + (b - a) * f1], egui::Stroke::new(1.5_f32, col));
                        }
                    }
                } else {
                    p.rect_stroke(r, 0.0, egui::Stroke::new(1.0_f32, pal().edge));
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Pick a picture from this computer").clicked() {
                    if let Some(p) = pick_art() {
                        d.art = p;
                    }
                }
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    let w = ui.available_width();
                    crate::dim_line(ui, "NAME", 1.0, pal().ink2);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, crate::bh()), Sense::hover());
                    if crate::views::field(ui, &mut self.ed, F_NEW_NAME, &mut d.name, rect, "Playlist name", false).enter {
                        make = true;
                    }
                    ui.add_space(8.0);
                    crate::dim_line(ui, "DESCRIPTION", 1.0, pal().ink2);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 92.0), Sense::hover());
                    crate::views::field(ui, &mut self.ed, F_NEW_DESC, &mut d.desc, rect, "What it is for (optional)", true);
                    if tidal_ok {
                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            if retro_btn(ui, "IN BACKLINE", !d.tidal).tip("Kept on this computer; can mix every source.").clicked()
                            {
                                d.tidal = false;
                            }
                            if retro_btn(ui, "ON TIDAL", d.tidal).tip("Made on your Tidal account.").clicked() {
                                d.tidal = true;
                            }
                        });
                        if d.tidal && !d.art.is_empty() {
                            crate::dim_line(ui, "Name and description go to Tidal; the cover shows in Backline only.", 1.0, pal().dim);
                        }
                    }
                });
            });
            // ready-made covers, and your own
            ui.add_space(12.0);
            crate::dim_line(ui, "COVERS", 1.0, pal().ink2);
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(7.0);
                for (c, tex) in covers.iter().zip(&thumbs) {
                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(52.0), Sense::click());
                    match tex {
                        Some(id) => {
                            egui::Image::new(egui::load::SizedTexture::new(*id, r.size())).paint_at(ui, r);
                        }
                        None => {
                            ui.painter().rect_filled(r, 0.0, pal().ink2.gamma_multiply(0.15));
                            ui.ctx().request_repaint();
                        }
                    }
                    let on = d.art == *c;
                    if on || resp.hovered() {
                        let col = if on { pal().ink } else { pal().edge };
                        ui.painter().rect_stroke(r.expand(2.5), 1.0, egui::Stroke::new(2.0_f32, col));
                    }
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        d.art = c.clone();
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "YOUR OWN PICTURE...", false).tip("Or drop one on the window").clicked() {
                    if let Some(p) = pick_art() {
                        d.art = p;
                    }
                }
                if !d.art.is_empty() && retro_btn(ui, "NO COVER", false).clicked() {
                    d.art.clear();
                }
            });
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if retro_btn_w(ui, "MAKE IT", 120.0, true).clicked() {
                    make = true;
                }
                if retro_btn_w(ui, "CANCEL", 96.0, false).clicked() {
                    close = true;
                }
            });
        });
        if make {
            if d.tidal && d.name.trim().is_empty() {
                self.set_note("TYPE A NAME FOR THE PLAYLIST FIRST");
            } else {
                acts.push(Action::PlaylistMake(d));
                self.ed.id = 0;
                return;
            }
        }
        if close {
            self.ed.id = 0;
            return;
        }
        self.new_pl_dlg = Some(d);
    }
}

/// The ready-made covers: drawn once into Backline's folder, then just files (picked like any picture).
pub(crate) fn samples() -> Vec<String> {
    let dir = crate::api::config_dir().join("playlist_art").join("covers");
    (0..SAMPLES)
        .map(|k| {
            let path = dir.join(format!("cover{}.png", k + 1));
            if !path.exists() {
                let _ = std::fs::create_dir_all(&dir);
                let _ = draw_sample(k).save(&path);
            }
            path.to_string_lossy().into_owned()
        })
        .collect()
}

const SAMPLES: usize = 10;

/// One ready-made cover, 400 pixels square: record-sleeve shapes in two or three colours.
fn draw_sample(k: usize) -> image::RgbImage {
    const N: u32 = 400;
    type C = [f32; 3];
    fn mix(a: C, b: C, t: f32) -> C {
        let t = t.clamp(0.0, 1.0);
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
    }
    fn hex(h: u32) -> C {
        [((h >> 16) & 255) as f32, ((h >> 8) & 255) as f32, (h & 255) as f32]
    }
    // how much of a shape covers a pixel, from its signed distance in pixels (a soft edge)
    fn inside(d: f32) -> f32 {
        (0.5 - d).clamp(0.0, 1.0)
    }
    let mut img = image::RgbImage::new(N, N);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let (fx, fy) = (x as f32, y as f32);
        let (u, v) = (fx / N as f32, fy / N as f32);
        let r = ((fx - 200.0).powi(2) + (fy - 200.0).powi(2)).sqrt();
        let c: C = match k {
            // midnight: deep blue, a gold moon
            0 => {
                let m = ((fx - 270.0).powi(2) + (fy - 130.0).powi(2)).sqrt() - 60.0;
                mix(mix(hex(0x0b1d3a), hex(0x1f4e79), v), hex(0xf2c14e), inside(m))
            }
            // sunset: orange to purple, a low sun cut by stripes
            1 => {
                let bg = mix(hex(0xff8c42), hex(0x5b2a86), v);
                let stripe = fy > 220.0 && ((fy - 220.0) as u32 / 14) % 2 == 1;
                if !stripe {
                    mix(bg, hex(0xffd166), inside(r - 120.0))
                } else {
                    bg
                }
            }
            // vinyl: a record on a red sleeve
            2 => {
                let bg = hex(0xc0392b);
                let groove = if (r as u32 / 5) % 2 == 0 { 0.0 } else { 0.1 };
                let rec = mix(hex(0x111111), hex(0x3a3a3a), groove);
                let c = mix(mix(bg, rec, inside(r - 170.0)), hex(0xf4e1c1), inside(r - 55.0));
                mix(c, bg, inside(r - 8.0))
            }
            // stripes: diagonal, three colours
            3 => [hex(0x1b998b), hex(0xede6e3), hex(0xe84855)][((fx + fy) / 40.0) as usize % 3],
            // halftone: dots growing down the page
            4 => {
                let (gx, gy) = ((fx % 20.0) - 10.0, (fy % 20.0) - 10.0);
                mix(hex(0xf7f3e3), hex(0x2d3047), inside((gx * gx + gy * gy).sqrt() - (1.0 + 8.0 * v)))
            }
            // waves: rolling lines on teal
            5 => {
                let w = (fy + (u * 12.0).sin() * 18.0 + (u * 5.0 + v * 3.0).sin() * 10.0).rem_euclid(32.0);
                mix(mix(hex(0x006d77), hex(0x83c5be), v), hex(0xedf6f9), inside((w - 16.0).abs() - 3.0))
            }
            // night sky: stars on violet
            6 => {
                let h = (x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663)) % 1000;
                if h < 3 {
                    hex(0xffffff)
                } else {
                    mix(hex(0x10002b), hex(0x5a189a), v * v)
                }
            }
            // blocks: a circle, a quarter and a square, Bauhaus style
            7 => {
                let mut c = if fx < 200.0 && fy > 200.0 { hex(0x2e294e) } else { hex(0xf1e9da) };
                c = mix(c, hex(0xd90368), inside(((fx - 400.0).powi(2) + fy.powi(2)).sqrt() - 200.0));
                mix(c, hex(0xffd400), inside(((fx - 120.0).powi(2) + (fy - 120.0).powi(2)).sqrt() - 70.0))
            }
            // smoke: soft clouds of colour
            8 => {
                let a = (u * 6.0 + (v * 4.0).sin()).sin() * 0.5 + 0.5;
                let b = (v * 5.0 + (u * 3.0).cos() * 1.5).sin() * 0.5 + 0.5;
                mix(mix(hex(0x0f2027), hex(0x2c5364), a), hex(0xd4a373), b * b * 0.6)
            }
            // spotlight: a cone of light on a dark stage
            _ => {
                let d = ((u - 0.5).abs() * 2.0 - v * 0.9).max(0.0);
                let light = (1.0 - d * 3.0).clamp(0.0, 1.0) * (0.35 + 0.65 * v);
                let floor = inside((fx - 200.0).abs() + (fy - 360.0).abs() * 4.0 - 150.0) * 0.4;
                mix(hex(0x0a0a0a), hex(0xf6e7b4), light.max(floor))
            }
        };
        *px = image::Rgb([c[0] as u8, c[1] as u8, c[2] as u8]);
    }
    img
}
