//! Backline's own looks worn as skins (`wear_look`), your Custom look (`wear_custom`) and its editor: two colours you
//! pick (Background, Accent), everything else worked out from them so the styling stays in step (`skin::custom_look`).
//! The editor wears your picks as you go; Cancel puts back what you wore before.

use super::*;
use crate::credits::panel;

/// The Custom look's editor, open: what was worn when it opened (Cancel puts it back), and when the look was last
/// drawn (picks while dragging are drawn a few times a second, not every frame).
pub(crate) struct LookEdit {
    before: (Option<(String, String)>, usize, (Color32, Color32), bool),
    drawn_at: f64,
    pending: bool,
}

/// Quick picks under each colour: the built-in looks' own.
const QUICK_BACKGROUND: [Color32; 8] = [
    Color32::from_rgb(43, 43, 44),
    Color32::from_rgb(36, 45, 67),
    Color32::from_rgb(52, 40, 78),
    Color32::from_rgb(72, 38, 50),
    Color32::from_rgb(70, 40, 28),
    Color32::from_rgb(38, 56, 40),
    Color32::from_rgb(26, 58, 62),
    Color32::from_rgb(228, 220, 204),
];
const QUICK_ACCENT: [Color32; 8] = [
    Color32::from_rgb(206, 212, 228),
    Color32::from_rgb(110, 170, 255),
    Color32::from_rgb(196, 164, 255),
    Color32::from_rgb(255, 92, 214),
    Color32::from_rgb(255, 146, 74),
    Color32::from_rgb(255, 190, 70),
    Color32::from_rgb(140, 226, 140),
    Color32::from_rgb(104, 228, 214),
];

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

impl App {
    /// Look `n` as the skin to wear.
    pub(crate) fn gen_skin(&self, n: usize) -> winamp::WaSkin {
        winamp::WaSkin { md5: LOOK_MD5.into(), name: SKIN_NAMES[n % LOOKS.len()].into(), shot: String::new(), download: String::new() }
    }

    /// Your Custom look's palette, from its two colours.
    pub(crate) fn custom_pal(&self) -> Pal {
        let (panel, display, accent) = crate::skin::custom_look(self.custom_cols.0, self.custom_cols.1);
        crate::skin::look_pal(panel, display, accent)
    }

    /// Wear Backline's look `n` (drawn now, worn before the next frame).
    pub(crate) fn wear_look(&mut self, n: usize) {
        let n = n % LOOKS.len();
        SKIN.store(n, Ordering::Relaxed);
        self.look_custom = false;
        self.dirty = true;
        let s = self.gen_skin(n);
        let _ = self.tx.send(Msg::WaSkin(genskin::wear(&PALS[n]).map(|(p, art)| (s, p, art))));
        self.ctx.request_repaint();
    }

    /// Wear your Custom look, as its colours are now.
    pub(crate) fn wear_custom(&mut self) {
        self.look_custom = true;
        self.dirty = true;
        let s = winamp::WaSkin { md5: CUSTOM_MD5.into(), name: "CUSTOM".into(), shot: String::new(), download: String::new() };
        let _ = self.tx.send(Msg::WaSkin(genskin::wear(&self.custom_pal()).map(|(p, art)| (s, p, art))));
        self.ctx.request_repaint();
    }

    /// Open the Custom look's editor, wearing it straight away.
    pub(crate) fn edit_look(&mut self) {
        let now = self.ctx.input(|i| i.time);
        self.look_edit = Some(LookEdit {
            before: (self.wa_worn.clone(), SKIN.load(Ordering::Relaxed), self.custom_cols, self.look_custom),
            drawn_at: now,
            pending: false,
        });
        self.wear_custom();
    }

    /// The Custom look's editor: Background and Accent, each a colour picker with quick picks under it; Done keeps it,
    /// Cancel (or Esc) puts back what you wore before.
    pub(crate) fn look_panel(&mut self, ctx: &egui::Context) {
        let Some(edit) = self.look_edit.as_ref() else { return };
        let (mut done, mut cancel) = ctx.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        let mut cols = self.custom_cols;
        let mut changed = false;
        let pending = edit.pending;
        panel(ctx, "look_edit", 470.0, |ui| {
            title_line(ui, "CUSTOM LOOK", 3.0, pal().ink);
            ui.add_space(4.0);
            para(ui, "Pick two colors. The rest is worked out from them, so everything stays in step.", pal().ink2);
            ui.add_space(8.0);
            for (k, (name, quick)) in [("Background", &QUICK_BACKGROUND), ("Accent", &QUICK_ACCENT)].into_iter().enumerate() {
                let col = if k == 0 { &mut cols.0 } else { &mut cols.1 };
                ui.horizontal(|ui| {
                    crate::views::label(ui, name, 110.0);
                    changed |= egui::color_picker::color_edit_button_srgba(ui, col, egui::color_picker::Alpha::Opaque).changed();
                    ui.add_space(10.0);
                    for q in quick.iter() {
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        ui.painter().rect_filled(r.shrink(1.0), 4.0, *q);
                        let edge = if *q == *col { pal().ink } else if resp.hovered() { pal().ink2 } else { pal().edge };
                        ui.painter().rect_stroke(r.shrink(1.0), 4.0, Stroke::new(if *q == *col { 2.0_f32 } else { 1.0 }, edge));
                        if resp.clicked() {
                            *col = *q;
                            changed = true;
                        }
                    }
                });
                ui.add_space(6.0);
            }
            ui.add_space(8.0);
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
        if changed {
            self.custom_cols = cols;
            if let Some(e) = self.look_edit.as_mut() {
                e.pending = true;
            }
        }
        // (the look drawn again a few times a second while the colours move, and once more when they stop)
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
            self.custom_cols = cols;
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
