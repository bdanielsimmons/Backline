//! The main window's layout: the library on one side and, with a skin, the stack on the other (player, equalizer,
//! playlist, practice panel) sized the way Winamp drew it (whole skin pixels where it can), or Backline's own
//! columns without one; the drag handles between them.

use super::*;

impl App {
    /// The library and the player's column (not in the mini player, Now Playing or the full-window visuals).
    pub(crate) fn main_layout(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        let screen = ctx.screen_rect();
        // library width is a share of the window, so it scales with it
        // a worn skin: the stack (player, equalizer, playlist) the way Winamp drew it - every skin pixel a whole
        // number of screen pixels (as large as the window has room for: 1 to 4), the column exactly that wide,
        // the library the rest
        let ppp = ctx.pixels_per_point();
        let skin_k: Option<f32> = self.wa_art.is_some().then(|| {
            let by_w = ((screen.width() - 640.0) * ppp / 275.0).floor();
            let by_h = (screen.height() * 0.45 * ppp / 116.0).floor();
            // at most Winamp's double size (more on a high-density screen): beside the library, never a giant
            if self.stack_w > 0.0 {
                // dragged: as wide as you made it (the deck's pixels then not all the same size), the deck no
                // taller than half the window, the library (with its side list) at least 640 points wide
                let max_k = (screen.height() * 0.5 / 116.0).max(1.0);
                (self.stack_w / 275.0).clamp(1.0, max_k).min(((screen.width() - 640.0) / 275.0).max(1.0))
            } else {
                by_w.min(by_h).clamp(1.0, (2.0 * ppp).round().max(2.0)) / ppp
            }
        });
        // practising with a skin: the deck and playlist stop growing at about a quarter of the window's height,
        // so the practice panel gets the room; the column is then just as wide as the deck (the library takes
        // the rest), so the deck always fills it, edge to edge - never a gap beside it. (Never narrower than
        // the practice panel's tabs need.)
        let skin_k = skin_k.map(|k| {
            if self.practice && !self.focus_mode {
                k.min((screen.height() * 0.25 / 116.0).max(PRACTICE_COL_MIN / 275.0))
            } else {
                k
            }
        });
        if let Some(k) = skin_k {
            self.stack_k = k;
        }
        // focus with a skin: the stack keeps its place, and the big side holds the practice tools (or the
        // song big, when just listening) instead of the lists
        let focus_skin = self.focus_mode && skin_k.is_some();
        let lib_w = if self.focus_mode && !focus_skin {
            0.0
        } else if let Some(k) = skin_k {
            (screen.width() - 275.0 * k).max(280.0)
        } else {
            (self.lib_frac * screen.width()).clamp(280.0, (screen.width() - 560.0).max(280.0))
        };
        let right_w = screen.width() - lib_w;
        let inner_w = right_w - 28.0;
        // the practice panel's height and the player's are yours to drag (double-click a handle: back to auto)
        // (0: as tall as it can be, the playlist keeping a few songs)
        let mut practice_h = if !self.practice || focus_skin {
            0.0
        } else if self.prac_h <= 0.0 {
            if skin_k.is_some() {
                screen.height()
            } else {
                (screen.height() * 0.6).max(120.0)
            }
        } else {
            self.prac_h.clamp(120.0, (screen.height() * 0.6).max(120.0))
        };
        let max_inner_h = ((screen.height() - practice_h) * 0.5 - 44.0).max(120.0);
        let auto_s = (inner_w / 300.0).min(max_inner_h / 138.0).clamp(0.8, 3.0);
        let mut s = if self.stack_frac >= 0.99 {
            auto_s
        } else {
            let want = (self.stack_frac * screen.height() - 44.0).max(0.0);
            (inner_w / 300.0).min(want / 138.0).clamp(0.6, 3.0)
        };
        // the whole column fits in the window, the playlist always with room for a good few songs: when
        // player, equalizer, playlist and practice panel add up to more, the practice panel gives first (down
        // to its least), then an equalizer you sized, then the player scales down
        const PLAYLIST_MIN: f32 = 200.0;
        let mut eq_fit = self.eq_h_user;
        {
            // the parts of the column that grow with the player's scale, and the ones that don't
            let skin_eq = self.show_eq && self.wa_art.is_some() && eq_fit.is_none();
            let per_s = if self.queue_hidden { 0.0 } else { 138.0 } + if skin_eq { 116.0 } else { 0.0 };
            let base = if self.queue_hidden { 160.0 } else { 44.0 + PLAYLIST_MIN };
            let eq_part = |eq: Option<f32>| {
                if !self.show_eq {
                    0.0
                } else if let Some(h) = eq {
                    h
                } else if skin_eq {
                    104.0
                } else {
                    340.0
                }
            };
            let h = screen.height();
            if self.practice && skin_k.is_none() {
                let room = h - base - eq_part(eq_fit) - per_s * s;
                if practice_h > room {
                    practice_h = room.max(120.0);
                }
            }
            if let Some(e) = eq_fit.filter(|_| self.show_eq) {
                let room = h - base - practice_h - per_s * s;
                if e > room {
                    eq_fit = Some(room.max(140.0));
                }
            }
            if per_s > 0.0 {
                s = s.min((h - base - eq_part(eq_fit) - practice_h) / per_s).max(0.4);
            }
        }
        // (a skin's deck: its own height, a little room above it, resting on the playlist)
        let player_h = skin_k.map_or(138.0 * s + 44.0, |k| 116.0 * k);

        if focus_skin {
            egui::SidePanel::left("focus_side")
                .exact_width(lib_w)
                .resizable(false)
                .show_separator_line(false)
                .frame(panel_frame())
                .show(ctx, |ui| {
                    let r = ui.max_rect();
                    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(r));
                    if self.practice {
                        self.practice_ui(&mut inner, acts);
                    } else {
                        self.art_ui(&mut inner, acts);
                    }
                    ui.allocate_rect(r, Sense::hover());
                });
        } else if !self.focus_mode {
            egui::SidePanel::left("library")
                .exact_width(lib_w)
                .resizable(false)
                .show_separator_line(false)
                .frame(panel_frame())
                .show(ctx, |ui| {
                    // drawn in a child that does not stretch the panel: something too wide for the column
                    // (long words in a wide font) is cut off at the edge instead of pushing the player away
                    let r = ui.max_rect();
                    self.lib_rect = r;
                    // its own zoom: ctrl + the mouse wheel, or ctrl + / - / 0, with the mouse over it
                    if ui.rect_contains_pointer(r) {
                        let (zd, plus, minus, zero) = ui.input_mut(|i| {
                            let ctrl = i.modifiers.command;
                            let plus = ctrl
                                && (i.consume_key(i.modifiers, egui::Key::Plus)
                                    | i.consume_key(i.modifiers, egui::Key::Equals));
                            let minus = ctrl && i.consume_key(i.modifiers, egui::Key::Minus);
                            let zero = ctrl && i.consume_key(i.modifiers, egui::Key::Num0);
                            (i.zoom_delta(), plus, minus, zero)
                        });
                        let before = self.lib_zoom;
                        if zd != 1.0 {
                            self.lib_zoom *= zd;
                        }
                        if plus {
                            self.lib_zoom += 0.1;
                        }
                        if minus {
                            self.lib_zoom -= 0.1;
                        }
                        if zero {
                            self.lib_zoom = 1.0;
                        }
                        self.lib_zoom = self.lib_zoom.clamp(0.8, 2.5);
                        if self.lib_zoom != before {
                            self.dirty = true;
                        }
                    }
                    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(r));
                    let z = self.lib_zoom;
                    font::with_scale(font::ui_scale() * z, || self.library_ui(&mut inner, acts));
                    ui.allocate_rect(r, Sense::hover());
                });
        }
        // the equalizer docks under the player: the skin's at the player's own scale, or Backline's
        let eq_h = eq_fit.unwrap_or(match (skin_k, self.wa_art.is_some()) {
            // a skin's stack: the equalizer at the deck's own size, its frame and preset cards round it
            (Some(k), _) => 116.0 * k + 44.0 + 60.0,
            (None, true) => 116.0 * s + 44.0 + 60.0,
            _ => 340.0,
        });
        // a skin's stack: deck, equalizer and practice panel never leave the playlist less than a few songs
        // (the practice panel gives way, then the equalizer)
        let (practice_h, eq_h) = if skin_k.is_some() && !focus_skin && !self.queue_hidden {
            // the playlist's frame (58 skin pixels tall) and four songs
            let k = skin_k.unwrap_or(1.0);
            // (practising: two songs, the practice panel getting the rest)
            let songs = if self.practice { 2.0 } else { 4.0 };
            let min_pl = 58.0 * k + songs * 13.0 * k.min(2.0) + 8.0;
            let eq_now = if self.show_eq { eq_h } else { 0.0 };
            let mut p = practice_h;
            if self.practice {
                p = p.min((screen.height() - player_h - eq_now - min_pl).max(160.0));
            }
            let e = if self.show_eq { eq_h.min((screen.height() - player_h - p - min_pl).max(120.0)) } else { eq_h };
            (p, e)
        } else {
            (practice_h, eq_h)
        };
        // the column, top down: the player (PL off with no equalizer: it fills the column instead, below), the
        // equalizer right under it, the practice panel along the foot (under the playlist, so player, equalizer and
        // playlist stay one stack), the playlist in what's left
        if !self.queue_hidden || self.show_eq {
            egui::TopBottomPanel::top("player")
                .exact_height(player_h)
                .resizable(false)
                .show_separator_line(false)
                .frame(panel_frame())
                .show(ctx, |ui| self.player_window(ui, acts));
        }
        if self.show_eq {
            egui::TopBottomPanel::top("eq_dock")
                .exact_height(eq_h)
                .resizable(false)
                .show_separator_line(false)
                .frame(panel_frame())
                .show(ctx, |ui| self.eq_dock(ui));
        }
        if self.practice && !focus_skin {
            egui::TopBottomPanel::bottom("practice")
                .exact_height(practice_h)
                .resizable(false)
                .show_separator_line(false)
                .frame(panel_frame())
                .show(ctx, |ui| {
                    // (in a child: contents taller than the panel are cut off instead of pushing the playlist up)
                    let r = ui.max_rect();
                    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(r));
                    self.practice_ui(&mut inner, acts);
                    ui.allocate_rect(r, Sense::hover());
                });
        }
        let rest = egui::CentralPanel::default();
        if !self.queue_hidden {
            rest.frame(panel_frame()).show(ctx, |ui| self.playlist_ui(ui, acts));
        } else if self.show_eq {
            // PL off, the equalizer open: the room left under them
            rest.frame(egui::Frame::none().fill(pal().app_bg)).show(ctx, |_| {});
        } else {
            rest.frame(panel_frame()).show(ctx, |ui| self.player_window(ui, acts));
        }

        // drag handles between the windows: under the player (or the equalizer under it), its height;
        // above the practice panel, the panel's height
        // the equalizer's own height: dragged at its bottom (or its top, when it sits at the bottom of the
        // column); double-click for its size again
        if self.show_eq && skin_k.is_none() {
            let y = screen.min.y + player_h + eq_h;
            let r = Rect::from_min_size(Pos2::new(lib_w, y - 7.0), Vec2::new(right_w, 14.0));
            match splitter(ctx, "split_eq", r, false) {
                Some(Some(py)) => {
                    let h = py - screen.min.y - player_h;
                    self.eq_h_user = Some(h.clamp(140.0, (screen.height() * 0.6).max(140.0)));
                    self.dirty = true;
                }
                Some(None) => {
                    self.eq_h_user = None;
                    self.dirty = true;
                }
                None => {}
            }
        }
        if (!self.queue_hidden || self.show_eq) && skin_k.is_none() {
            // with the equalizer open, the player's handle is between the two
            let y = screen.min.y + player_h;
            let r = Rect::from_min_size(Pos2::new(lib_w, y - 7.0), Vec2::new(right_w, 14.0));
            match splitter(ctx, "split_h", r, false) {
                Some(Some(py)) => {
                    let ph = (py - screen.min.y).max(120.0);
                    self.stack_frac = (ph / screen.height()).clamp(0.1, 0.98);
                    self.dirty = true;
                }
                Some(None) => {
                    self.stack_frac = 1.0;
                    self.dirty = true;
                }
                None => {}
            }
        }
        if self.practice && !focus_skin {
            let y = screen.max.y - practice_h;
            let r = Rect::from_min_size(Pos2::new(lib_w, y - 7.0), Vec2::new(right_w, 14.0));
            match splitter(ctx, "split_p", r, false) {
                Some(Some(py)) => {
                    let most = if skin_k.is_some() {
                        // a skin's stack: never past the playlist's few songs
                        (screen.height()
                            - player_h
                            - if self.show_eq { eq_h } else { 0.0 }
                            - 58.0 * self.stack_k
                            - 2.0 * 13.0 * self.stack_k.min(2.0)
                            - 8.0)
                            .max(160.0)
                    } else {
                        (screen.height() * 0.6).max(220.0)
                    };
                    self.prac_h = (screen.max.y - py).clamp(120.0, most);
                    self.dirty = true;
                }
                Some(None) => {
                    self.prac_h = 0.0;
                    self.dirty = true;
                }
                None => {}
            }
        }
        if !self.focus_mode || focus_skin {
            let r = Rect::from_min_size(Pos2::new(lib_w - 7.0, screen.min.y), Vec2::new(14.0, screen.height()));
            match splitter(ctx, "split_v", r, true) {
                Some(Some(x)) if skin_k.is_some() => {
                    self.stack_w = (screen.width() - x).max(275.0);
                    self.dirty = true;
                }
                Some(None) if skin_k.is_some() => {
                    self.stack_w = 0.0;
                    self.dirty = true;
                }
                Some(Some(x)) => {
                    self.lib_frac = (x / screen.width()).clamp(0.2, 0.7);
                    self.dirty = true;
                }
                Some(None) => self.lib_frac = 0.34,
                None => {}
            }
        }
    }
}

// ------------------------------------------------------------------- update
pub(crate) fn panel_frame() -> egui::Frame {
    egui::Frame::none().fill(pal().app_bg).inner_margin(egui::Margin { left: 14.0, right: 14.0, top: 30.0, bottom: 14.0 })
}

/// The skin's column while practising is never narrower than this (points): the practice panel's tabs fit in it.
pub(crate) const PRACTICE_COL_MIN: f32 = 560.0;
