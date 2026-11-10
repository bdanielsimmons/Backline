//! Backline's own looks worn as skins (`wear_look`), your Custom look (`wear_custom`) and its editor: your colors
//! (Background, Accent, and the Display and Text when you want them yours too), everything else worked out from them so
//! the styling stays in step (`skin::CustomLook`). The editor wears your picks as you go; Cancel puts back what you
//! wore before. On the Skins page each look shows its own deck in little (`look_deck`).

use super::*;
use crate::credits::panel;
use crate::skin::CustomLook;

/// The Custom look's editor, open: what was worn when it opened (Cancel puts it back), which color is being picked,
/// and when the look was last drawn (picks while dragging are drawn a few times a second, not every frame).
pub(crate) struct LookEdit {
    before: (Option<(String, String)>, usize, CustomLook, bool),
    role: usize,
    drawn_at: f64,
    pending: bool,
}

/// The colors you can pick, in the editor's order.
const ROLES: [&str; 4] = ["Background", "Display", "Accent", "Text"];

/// The skin's own id for Backline's looks ("gen:" + the look's number in older versions) and for your Custom one.
pub(crate) const LOOK_MD5: &str = "gen:2";
pub(crate) const CUSTOM_MD5: &str = "gen:custom";

/// Whether what's worn is one of Backline's own looks (or your Custom one), rather than a Winamp skin.
pub(crate) fn backline_look(worn: Option<&(String, String)>) -> bool {
    worn.is_some_and(|w| w.0.starts_with("gen:"))
}

/// Whether what's worn is your Custom look.
pub(crate) fn custom_worn(worn: Option<&(String, String)>) -> bool {
    worn.is_some_and(|w| w.0 == CUSTOM_MD5)
}

/// A look in little: its panel, its display as a band, its accent as a dot.
pub(crate) fn swatch(p: &egui::Painter, r: Rect, panel: Color32, display: Color32, accent: Color32) {
    p.rect_filled(r, 3.0, panel);
    p.rect_filled(Rect::from_min_max(Pos2::new(r.min.x + r.width() * 0.45, r.min.y + 3.0), r.max - Vec2::new(3.0, 3.0)), 2.0, display);
    p.circle_filled(Pos2::new(r.min.x + r.width() * 0.25, r.center().y), 3.0, accent);
    p.rect_stroke(r, 3.0, Stroke::new(1.0_f32, pal().edge));
}

/// A look's row in a menu: its swatch, then its name, marked when worn. True when clicked.
pub(crate) fn look_item(ui: &mut egui::Ui, n: usize, on: bool) -> bool {
    let l = &LOOKS[n % LOOKS.len()];
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width().max(190.0), 24.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(r, 4.0, pal().sel);
    }
    swatch(p, Rect::from_min_size(Pos2::new(r.min.x + 6.0, r.center().y - 8.0), Vec2::new(30.0, 16.0)), l.panel, l.display, l.accent);
    let name = crate::winamp_ui::display_case(l.name);
    let col = if on || resp.hovered() { pal().ink } else { pal().ink2 };
    crate::font::ptext_fit(p, Pos2::new(r.min.x + 44.0, r.center().y), Align::Min, &name, 2.0, r.width() - 64.0, col);
    if on {
        p.circle_filled(Pos2::new(r.max.x - 10.0, r.center().y), 3.5, pal().accent);
    }
    resp.clicked()
}

thread_local! {
    /// Each look's deck in little (drawn as it's first shown on the Skins page), and your Custom look's (with the
    /// colors it was drawn from), and the frame the last one was drawn in (one a frame, so the page never stalls).
    static DECKS: std::cell::RefCell<HashMap<usize, egui::TextureHandle>> = std::cell::RefCell::new(HashMap::new());
    static CUSTOM_DECK: std::cell::RefCell<Option<(CustomLook, egui::TextureHandle)>> = const { std::cell::RefCell::new(None) };
    static DRAWN_IN: std::cell::Cell<u64> = const { std::cell::Cell::new(u64::MAX) };
}

/// Whether a deck may be drawn this frame (one a frame; the rest come in the frames after).
fn may_draw(ctx: &egui::Context) -> bool {
    let now = ctx.cumulative_pass_nr();
    if DRAWN_IN.with(|d| d.get()) == now {
        ctx.request_repaint();
        return false;
    }
    DRAWN_IN.with(|d| d.set(now));
    true
}

/// Look `n`'s own deck (the player as that look draws it), for the Skins page's cards; None until it's drawn.
pub(crate) fn look_deck(ctx: &egui::Context, n: usize) -> Option<egui::TextureId> {
    let n = n % LOOKS.len();
    if let Some(id) = DECKS.with(|d| d.borrow().get(&n).map(|t| t.id())) {
        return Some(id);
    }
    if !may_draw(ctx) {
        return None;
    }
    let (_, art) = genskin::wear(&PALS[n]).ok()?;
    let t = ctx.load_texture(format!("look_deck_{}", n), deck_image(&art)?, egui::TextureOptions::NEAREST);
    let id = t.id();
    DECKS.with(|d| d.borrow_mut().insert(n, t));
    Some(id)
}

/// Your Custom look's deck, drawn again when its colors change.
pub(crate) fn custom_deck(ctx: &egui::Context, look: &CustomLook) -> Option<egui::TextureId> {
    if let Some(id) = CUSTOM_DECK.with(|d| d.borrow().as_ref().filter(|(l, _)| l == look).map(|(_, t)| t.id())) {
        return Some(id);
    }
    if !may_draw(ctx) {
        return None;
    }
    let (_, _, _, text) = look.colors();
    let (_, art) = genskin::wear_with(&look.pal(), text).ok()?;
    let t = ctx.load_texture("custom_deck", deck_image(&art)?, egui::TextureOptions::NEAREST);
    let id = t.id();
    CUSTOM_DECK.with(|d| *d.borrow_mut() = Some((*look, t)));
    Some(id)
}

/// A skin's deck put together as Winamp lays it out: the window, its title bar, the buttons and sliders, a clock, the
/// song's name and a visualizer playing.
fn deck_image(art: &winamp::Art) -> Option<egui::ColorImage> {
    const W: usize = 275;
    const H: usize = 116;
    let mut px = vec![0u8; W * H * 4];
    art.sheets.iter().find(|s| s.0 == "main")?;
    let blit = |px: &mut Vec<u8>, name: &str, (sx, sy, w, h): (usize, usize, usize, usize), (dx, dy): (usize, usize)| {
        let Some((_, size, rgba)) = art.sheets.iter().find(|s| s.0 == name) else { return };
        for y in 0..h {
            for x in 0..w {
                let (ix, iy, ox, oy) = (sx + x, sy + y, dx + x, dy + y);
                if ix >= size[0] || iy >= size[1] || ox >= W || oy >= H {
                    continue;
                }
                let (i, o) = ((iy * size[0] + ix) * 4, (oy * W + ox) * 4);
                let Some(src) = rgba.get(i..i + 4) else { continue };
                let a = src[3] as u32;
                for k in 0..3 {
                    px[o + k] = ((src[k] as u32 * a + px[o + k] as u32 * (255 - a)) / 255) as u8;
                }
                px[o + 3] = 255;
            }
        }
    };
    blit(&mut px, "main", (0, 0, W, H), (0, 0));
    blit(&mut px, "titlebar", (27, 0, 275, 14), (0, 0));
    // previous, play, pause, stop, next, eject
    for (k, x) in [0, 23, 46, 69, 92].into_iter().enumerate() {
        blit(&mut px, "cbuttons", (x, 0, if k == 4 { 22 } else { 23 }, 18), (16 + x, 88));
    }
    blit(&mut px, "cbuttons", (114, 0, 22, 16), (136, 89));
    // a song a third of the way in, the volume up most of the way
    blit(&mut px, "posbar", (0, 0, 248, 10), (16, 72));
    blit(&mut px, "posbar", (248, 0, 29, 10), (16 + 70, 72));
    blit(&mut px, "volume", (0, 20 * 15, 68, 13), (107, 57));
    blit(&mut px, "volume", (15, 422, 14, 11), (107 + 40, 58));
    blit(&mut px, "balance", (9, 0, 38, 13), (177, 57));
    blit(&mut px, "balance", (15, 422, 14, 11), (177 + 12, 58));
    blit(&mut px, "shufrep", (28, 0, 47, 15), (164, 89));
    blit(&mut px, "shufrep", (0, 0, 28, 15), (210, 89));
    blit(&mut px, "shufrep", (0, 61, 23, 12), (219, 58));
    blit(&mut px, "shufrep", (23, 61, 23, 12), (242, 58));
    blit(&mut px, "monoster", (29, 12, 27, 12), (212, 41));
    blit(&mut px, "monoster", (0, 0, 29, 12), (239, 41));
    blit(&mut px, "playpaus", (0, 0, 9, 9), (26, 28));
    // the clock at 2:47, the song's name
    for (d, x) in [(0, 48), (2, 60), (4, 78), (7, 90)] {
        blit(&mut px, "numbers", (d * 9, 0, 9, 13), (x, 26));
    }
    for (k, ch) in "backline".chars().enumerate() {
        blit(&mut px, "text", ((ch as usize - 'a' as usize) * 5, 0, 5, 6), (111 + k * 5, 27));
    }
    // the visualizer: bars in the skin's own colors (top to bottom), a peak over each
    let mut put = |x: usize, y: usize, c: Color32| {
        let o = (y * W + x) * 4;
        px[o..o + 4].copy_from_slice(&[c.r(), c.g(), c.b(), 255]);
    };
    const BARS: [usize; 19] = [9, 12, 14, 11, 8, 10, 13, 15, 12, 9, 7, 10, 8, 11, 6, 8, 5, 7, 4];
    for (b, &h) in BARS.iter().enumerate() {
        for row in 16 - h..16 {
            for x in 0..3 {
                put(24 + b * 4 + x, 43 + row, art.vis.get(2 + row).copied().unwrap_or(Color32::GRAY));
            }
        }
        if h < 15 {
            for x in 0..3 {
                put(24 + b * 4 + x, 43 + 15 - h, art.vis.get(23).copied().unwrap_or(Color32::GRAY));
            }
        }
    }
    Some(egui::ColorImage::from_rgba_unmultiplied([W, H], &px))
}

/// The quick picks for a color: the built-in looks' own (each once), or a set of writing colors.
fn quick_picks(role: usize) -> Vec<Color32> {
    if role == 3 {
        // writing: whites and greys, creams, a few tints, and dark ones for paper
        return [
            (240, 240, 240), (210, 210, 210), (170, 170, 170), (236, 226, 200), (206, 212, 228), (190, 220, 255),
            (200, 240, 200), (255, 220, 160), (255, 200, 220), (230, 210, 255), (40, 40, 40), (20, 30, 60),
            (60, 30, 20), (20, 50, 30), (70, 20, 60), (0, 0, 0),
        ]
        .iter()
        .map(|&(r, g, b)| Color32::from_rgb(r, g, b))
        .collect();
    }
    let mut out: Vec<Color32> = Vec::new();
    for l in LOOKS.iter() {
        let c = match role {
            0 => l.panel,
            1 => l.display,
            _ => l.accent,
        };
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

impl App {
    /// Look `n` as the skin to wear.
    pub(crate) fn gen_skin(&self, n: usize) -> winamp::WaSkin {
        winamp::WaSkin { md5: LOOK_MD5.into(), name: SKIN_NAMES[n % LOOKS.len()].into(), shot: String::new(), download: String::new() }
    }

    /// Wear Backline's look `n` (drawn now, worn before the next frame).
    pub(crate) fn wear_look(&mut self, n: usize) {
        let n = n % LOOKS.len();
        SKIN.store(n, Ordering::Relaxed);
        crate::skin::set_mono(Some(n));
        self.look_custom = false;
        self.dirty = true;
        let s = self.gen_skin(n);
        let _ = self.tx.send(Msg::WaSkin(genskin::wear(&PALS[n]).map(|(p, art)| (s, p, art))));
        self.ctx.request_repaint();
    }

    /// Your Custom look as a skin, from its colors as they are now.
    pub(crate) fn custom_skin(&self) -> Result<(winamp::WaSkin, Pal, winamp::Art), String> {
        let s = winamp::WaSkin { md5: CUSTOM_MD5.into(), name: "CUSTOM".into(), shot: String::new(), download: String::new() };
        let (_, _, _, text) = self.custom.colors();
        genskin::wear_with(&self.custom.pal(), text).map(|(p, art)| (s, p, art))
    }

    /// Wear your Custom look, as its colors are now.
    pub(crate) fn wear_custom(&mut self) {
        crate::skin::set_mono(None);
        self.look_custom = true;
        self.dirty = true;
        let _ = self.tx.send(Msg::WaSkin(self.custom_skin()));
        self.ctx.request_repaint();
    }

    /// Open the Custom look's editor, wearing it straight away.
    pub(crate) fn edit_look(&mut self) {
        let now = self.ctx.input(|i| i.time);
        self.look_edit = Some(LookEdit {
            before: (self.wa_worn.clone(), SKIN.load(Ordering::Relaxed), self.custom, self.look_custom),
            role: 0,
            drawn_at: now,
            pending: false,
        });
        self.wear_custom();
    }

    /// The Custom look's editor: Start From a look; Background, Display, Accent and Text each picked right here (the
    /// picker in the panel, so it never runs off a small window), with quick picks beside it; Display and Text are
    /// worked out until you pick them (Auto puts them back). Done keeps it, Cancel (or Esc) puts back what you wore.
    pub(crate) fn look_panel(&mut self, ctx: &egui::Context) {
        let Some(edit) = self.look_edit.as_ref() else { return };
        let (mut done, mut cancel) = ctx.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        let mut look = self.custom;
        let mut role = edit.role;
        let pending = edit.pending;
        let mut changed = false;
        let (_, shown_display, _, shown_text) = look.colors();
        panel(ctx, "look_edit", 560.0, |ui| {
            title_line(ui, "CUSTOM LOOK", 3.0, pal().ink);
            ui.add_space(2.0);
            para(ui, "Pick your colors. The rest is worked out from them, so everything stays readable.", pal().ink2);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                crate::views::label(ui, "Start From", 100.0);
                let cur = SKIN.load(Ordering::Relaxed) % LOOKS.len();
                if let Some(n) = crate::dropdown(ui, "look_start", "", &SKIN_NAMES, cur, 220.0) {
                    look = CustomLook::from_look(n);
                    changed = true;
                }
            });
            ui.add_space(8.0);
            // the four colors as chips: the one being picked lit
            ui.horizontal(|ui| {
                for (k, name) in ROLES.iter().enumerate() {
                    let col = match k {
                        0 => look.background,
                        1 => look.display.unwrap_or(shown_display),
                        2 => look.accent,
                        _ => look.text.or(shown_text).unwrap_or(pal().ink),
                    };
                    let auto = (k == 1 && look.display.is_none()) || (k == 3 && look.text.is_none());
                    let label = if auto { format!("{} (auto)", name) } else { name.to_string() };
                    let w = crate::font::text_w(&label, 1.6) + 40.0;
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::click());
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let p = ui.painter();
                    let lit = k == role;
                    p.rect_filled(r, 14.0, if lit { pal().btn_hi } else if resp.hovered() { pal().sel } else { pal().btn_face });
                    p.circle_filled(Pos2::new(r.min.x + 14.0, r.center().y), 7.0, col);
                    p.circle_stroke(Pos2::new(r.min.x + 14.0, r.center().y), 7.0, Stroke::new(1.0_f32, pal().edge));
                    crate::font::ptext(p, Pos2::new(r.min.x + 28.0, r.center().y), Align::Min, &label, 1.6, pal().ink);
                    if resp.clicked() {
                        role = k;
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                // the picker itself, here in the panel
                let mut c = match role {
                    0 => look.background,
                    1 => look.display.unwrap_or(shown_display),
                    2 => look.accent,
                    _ => look.text.or(shown_text).unwrap_or(pal().ink),
                };
                let before = c;
                ui.vertical(|ui| {
                    ui.spacing_mut().slider_width = 230.0;
                    egui::color_picker::color_picker_color32(ui, &mut c, egui::color_picker::Alpha::Opaque);
                });
                ui.add_space(12.0);
                // quick picks: the looks' own, a grid of them
                let mut auto = false;
                ui.vertical(|ui| {
                    for row in quick_picks(role).chunks(6) {
                        ui.horizontal(|ui| {
                            for q in row {
                                let (r, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                                ui.painter().rect_filled(r.shrink(1.0), 5.0, *q);
                                let on = *q == c;
                                let edge = if on { pal().ink } else if resp.hovered() { pal().ink2 } else { pal().edge };
                                ui.painter().rect_stroke(r.shrink(1.0), 5.0, Stroke::new(if on { 2.0_f32 } else { 1.0 }, edge));
                                if resp.clicked() {
                                    c = *q;
                                }
                            }
                        });
                    }
                    // the display and the writing can go back to being worked out
                    if (role == 1 && look.display.is_some()) || (role == 3 && look.text.is_some()) {
                        ui.add_space(6.0);
                        auto = retro_btn(ui, "Auto", false).tip("Worked out from the background again.").clicked();
                    }
                });
                if auto {
                    if role == 1 {
                        look.display = None;
                    } else {
                        look.text = None;
                    }
                    changed = true;
                } else if c != before {
                    match role {
                        0 => look.background = c,
                        1 => look.display = Some(c),
                        2 => look.accent = c,
                        _ => look.text = Some(c),
                    }
                    changed = true;
                }
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if retro_btn_w(ui, "DONE", 96.0, true).clicked() {
                    done = true;
                }
                if retro_btn_w(ui, "CANCEL", 96.0, false).clicked() {
                    cancel = true;
                }
            });
        });
        let now = ctx.input(|i| i.time);
        if let Some(e) = self.look_edit.as_mut() {
            e.role = role;
        }
        if changed && look != self.custom {
            self.custom = look;
            if let Some(e) = self.look_edit.as_mut() {
                e.pending = true;
            }
        }
        // (the look drawn again a few times a second while the colors move, and once more when they stop)
        let due = self.look_edit.as_ref().is_some_and(|e| e.pending && now - e.drawn_at > 0.15);
        if due || (pending && !changed && !ctx.input(|i| i.pointer.any_down())) {
            if let Some(e) = self.look_edit.as_mut() {
                e.pending = false;
                e.drawn_at = now;
            }
            self.wear_custom();
        }
        if self.look_edit.as_ref().is_some_and(|e| e.pending) {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
        if cancel {
            let Some(e) = self.look_edit.take() else { return };
            let (worn, n, cols, custom) = e.before;
            self.custom = cols;
            match worn {
                Some(w) if custom_worn(Some(&w)) || (custom && backline_look(Some(&w))) => self.wear_custom(),
                Some(w) if backline_look(Some(&w)) => self.wear_look(n),
                Some((md5, name)) => {
                    self.apply(Action::WaApply(winamp::WaSkin { md5, name, shot: String::new(), download: String::new() }))
                }
                None => self.wear_look(n),
            }
        } else if done {
            if self.look_edit.as_ref().is_some_and(|e| e.pending) {
                self.wear_custom();
            }
            self.look_edit = None;
            self.dirty = true;
        }
    }
}
