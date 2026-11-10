//! The library window (left): sign-in, search box, the source lists, and the log / EQ / storage views.

use super::*;

/// The side list's (and Now Playing's) shade in a skin's library: between the list's colour and the window's
/// ground, so it sits a step apart from the page in any skin.
pub(crate) fn side_tone() -> Color32 {
    pal().display.lerp_to_gamma(pal().app_bg, 0.55)
}

/// A fine line between parts of one surface: the text colour, faint (it shows on light and dark skins alike).
pub(crate) fn divider() -> Color32 {
    pal().ink2.gamma_multiply(0.22)
}

/// A small rounded tag on a picture, its bottom-left corner at `at`.
fn tag(p: &egui::Painter, at: Pos2, text: &str, bg: Color32, fg: Color32) {
    let sc = crate::font::ui_scale();
    let w = crate::font::text_w(text, 1.0) + 16.0 * sc;
    let r = Rect::from_min_max(Pos2::new(at.x, at.y - 20.0 * sc), Pos2::new(at.x + w, at.y));
    p.rect_filled(r, r.height() / 2.0, bg);
    crate::font::ptext(p, r.center(), Align::Center, text, 1.0, fg);
}

/// A five-pointed star of radius `r`: filled with `fill` (None: an outline only), edged in `edge`.
fn star_mark(p: &egui::Painter, c: Pos2, r: f32, fill: Option<Color32>, edge: Color32) {
    let pts: Vec<Pos2> = (0..10)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
            let rr = if k % 2 == 0 { r } else { r * 0.45 };
            c + Vec2::new(a.cos() * rr, a.sin() * rr)
        })
        .collect();
    if let Some(f) = fill {
        for k in 0..10 {
            p.add(egui::Shape::convex_polygon(vec![c, pts[k], pts[(k + 1) % 10]], f, Stroke::NONE));
        }
    }
    p.add(egui::Shape::closed_line(pts, Stroke::new(1.4_f32, edge)));
}

impl App {
    pub(crate) fn login_ui(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let full = ui.max_rect();
        let inner = Rect::from_center_size(full.center(), Vec2::new(480.0, 270.0));
        window_deco(ui, inner, "BACKLINE");
        let p = ui.painter();
        let c = inner.center();
        // the name with a mark either side of it, sized to the name (not to a fixed spot)
        let (title, tpx, ty) = ("BACKLINE", 4.0, c.y - 88.0);
        let half = crate::font::text_w(title, tpx) / 2.0;
        let mpx = 2.4;
        let m = mpx * 9.0;
        logo_mark(p, Pos2::new(c.x - half - 14.0 - m, ty - m / 2.0), mpx);
        logo_mark(p, Pos2::new(c.x + half + 14.0, ty - m / 2.0), mpx);
        ptext(p, Pos2::new(c.x, ty), Align::Center, title, tpx, pal().ink);
        if PRACTICE {
            ptext(p, c + Vec2::new(0.0, -56.0), Align::Center, "A retro player for Tidal and more", 2.0, pal().ink2);
            ptext(p, c + Vec2::new(0.0, -36.0), Align::Center, "PRACTICE EDITION: LOOP, SLOW DOWN, LEARN TUNES", 1.0, pal().dim);
        } else {
            ptext(p, c + Vec2::new(0.0, -50.0), Align::Center, "A retro player for Tidal and more", 2.0, pal().ink2);
        }
        match self.auth {
            Auth::LoggingIn => match &self.login_code {
                Some((code, url)) => {
                    ptext(p, c + Vec2::new(0.0, -6.0), Align::Center, "Approve the login in your browser", 2.0, pal().ink);
                    ptext(p, c + Vec2::new(0.0, 34.0), Align::Center, code, 5.0, pal().ink);
                    let b = Rect::from_center_size(c + Vec2::new(0.0, 86.0), Vec2::new(300.0, 30.0));
                    let r =
                        ui.interact(b, ui.id().with("relink"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                    raised_h(ui.painter(), b, r.is_pointer_button_down_on(), r.hovered());
                    ptext(ui.painter(), b.center(), Align::Center, "Open link again", 2.0, pal().ink);
                    if r.clicked() {
                        let _ = webbrowser::open(url);
                    }
                }
                None => {
                    ptext(p, c + Vec2::new(0.0, 24.0), Align::Center, "Contacting Tidal...", 2.0, pal().ink);
                }
            },
            _ => {
                let b = Rect::from_center_size(c + Vec2::new(0.0, 30.0), Vec2::new(320.0, 44.0));
                let r = ui.interact(b, ui.id().with("login"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                raised_h(ui.painter(), b, r.is_pointer_button_down_on(), r.hovered());
                ptext(ui.painter(), b.center(), Align::Center, "LOG IN WITH TIDAL", 3.0, pal().ink);
                if r.clicked() {
                    acts.push(Action::StartLogin);
                }
                let b2 = Rect::from_center_size(c + Vec2::new(0.0, 80.0), Vec2::new(320.0, 30.0));
                let r2 = ui.interact(b2, ui.id().with("skip"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                raised_h(ui.painter(), b2, r2.is_pointer_button_down_on(), r2.hovered());
                let skip = "USE WITHOUT TIDAL";
                ptext(ui.painter(), b2.center(), Align::Center, skip, 2.0, pal().ink2);
                if r2.clicked() {
                    acts.push(Action::Offline(true));
                }
                if !self.login_err.is_empty() {
                    let er = Rect::from_center_size(c + Vec2::new(0.0, 116.0), Vec2::new(430.0, 22.0));
                    inset(ui.painter(), er, pal().display);
                    marquee(ui, er, &self.login_err, 2.0, pal().red);
                }
            }
        }
    }

    /// The Tidal search box: the same text box as everywhere (select, Ctrl+A, copy and paste), Enter searches.
    pub(crate) fn search_input(&mut self, ui: &mut egui::Ui, rect: Rect, acts: &mut Vec<Action>) {
        const F_SEARCH: u32 = crate::tools::F_SEARCH;
        // something else took the keyboard away (the command palette opening)
        if !self.search_focus && self.ed.id == F_SEARCH {
            self.ed.id = 0;
        }
        let o = crate::views::field(ui, &mut self.ed, F_SEARCH, &mut self.search, rect, "Search Tidal", false);
        self.search_focus = self.ed.id == F_SEARCH;
        if o.enter {
            acts.push(Action::Search(self.search.clone()));
        }
    }

    pub(crate) fn log_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            if retro_btn(ui, "COPY LOG", false).clicked() {
                acts.push(Action::CopyLog);
            }
            if retro_btn(ui, "CLOSE", false).clicked() {
                self.side = None;
            }
        });
        ui.add_space(4.0);
        let w = ui.available_width() - 24.0;
        let text = crate::api::log_text();
        let mut lines: Vec<(String, bool)> = Vec::new();
        for l in text.lines() {
            let bad = l.contains("ERROR") || l.contains("PANIC") || l.contains("DECODE") || l.contains("failed");
            for piece in wrap(l, 2.0, w) {
                lines.push((piece, bad));
            }
        }
        let rh = 20.0;
        egui::ScrollArea::vertical().auto_shrink([false, false]).stick_to_bottom(true).show_rows(
            ui,
            rh,
            lines.len(),
            |ui, range| {
                for i in range {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), rh), Sense::hover());
                    let (txt, bad) = &lines[i];
                    ptext(
                        ui.painter(),
                        Pos2::new(rect.min.x + 6.0, rect.center().y),
                        Align::Min,
                        txt,
                        2.0,
                        if *bad { pal().red } else { pal().ink },
                    );
                }
            },
        );
    }

    /// The presets as little cards, each with a preview of its curve - Backline's, then yours (right-click one of
    /// yours to delete it), then a + card to keep the curve you have as your own (it asks for a name). The cards
    /// narrow to fit, and the row scrolls past that. True when one was picked.
    pub(crate) fn eq_presets(&mut self, ui: &mut egui::Ui, card_h: f32) -> bool {
        let mut changed = false;
        let all: Vec<(String, [f32; 10])> =
            EQ_PRESETS.iter().map(|(n, g)| (n.to_string(), *g)).chain(self.eq_mine.iter().cloned()).collect();
        let row_w = ui.available_width().max(120.0);
        // as wide as fits, from 96 down to 60, then the row scrolls
        let card_w = ((row_w - 8.0) / (all.len() + 1) as f32 - 6.0).clamp(60.0, 96.0);
        let mut drop: Option<usize> = None;
        let mut plus: Option<egui::Response> = None;
        ui.allocate_ui(Vec2::new(row_w, card_h + 14.0), |ui| {
            egui::ScrollArea::horizontal().id_salt("eq_presets").max_width(row_w).show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (k, (name, g)) in all.iter().enumerate() {
                        let r = self.eq_card(ui, name, g, card_w, card_h);
                        if r.clicked() {
                            self.eq_gains = *g;
                            self.eq_on = true;
                            changed = true;
                        }
                        if let Some(i) = k.checked_sub(EQ_PRESETS.len()) {
                            r.context_menu(|ui| {
                                if menu_item(ui, "Delete Preset") {
                                    drop = Some(i);
                                    ui.close_menu();
                                }
                            });
                        }
                    }
                    // + : save the curve you have
                    let (card, r) = ui.allocate_exact_size(Vec2::new(card_h, card_h), Sense::click());
                    let r = r.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Save as preset");
                    inset(ui.painter(), card, if r.hovered() || self.eq_naming { pal().sel } else { pal().display });
                    let c = card.center();
                    let st = Stroke::new(2.0_f32, pal().ink);
                    ui.painter().line_segment([c - Vec2::new(7.0, 0.0), c + Vec2::new(7.0, 0.0)], st);
                    ui.painter().line_segment([c - Vec2::new(0.0, 7.0), c + Vec2::new(0.0, 7.0)], st);
                    plus = Some(r);
                });
            });
        });
        if let Some(i) = drop {
            let name = self.eq_mine.remove(i).0;
            self.set_note(&format!("EQUALIZER PRESET {} DELETED", name));
            self.dirty = true;
        }
        let Some(plus) = plus else { return changed };
        let pop = ui.id().with("eq_name_pop");
        if plus.clicked() {
            self.eq_naming = !self.eq_naming;
            if self.eq_naming {
                ui.memory_mut(|m| m.open_popup(pop));
                // the name box takes the keyboard at once
                self.ed.id = crate::tools::F_EQ_NAME;
                self.ed.anchor = None;
                self.ed.cur = self.eq_name.chars().count();
            } else {
                ui.memory_mut(|m| m.close_popup());
            }
        }
        if !ui.memory(|m| m.is_popup_open(pop)) {
            self.eq_naming = false;
        }
        let mut save = false;
        egui::popup::popup_above_or_below_widget(
            ui,
            pop,
            &plus,
            egui::AboveOrBelow::Above,
            egui::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                ui.set_min_width(260.0);
                crate::para(ui, "PRESET NAME", pal().ink2);
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(170.0, bh()), Sense::hover());
                    let o = crate::views::field(
                        ui,
                        &mut self.ed,
                        crate::tools::F_EQ_NAME,
                        &mut self.eq_name,
                        rect,
                        "My preset",
                        false,
                    );
                    save = o.enter || retro_btn_w(ui, "SAVE", 70.0, false).clicked();
                });
            },
        );
        if save {
            let name = if self.eq_name.trim().is_empty() {
                format!("MY EQ {}", self.eq_mine.len() + 1)
            } else {
                self.eq_name.trim().to_uppercase()
            };
            // the same name again: that curve is replaced
            match self.eq_mine.iter_mut().find(|m| m.0 == name) {
                Some(m) => m.1 = self.eq_gains,
                None => self.eq_mine.push((name.clone(), self.eq_gains)),
            }
            self.eq_name.clear();
            self.eq_naming = false;
            self.ed.id = 0;
            ui.memory_mut(|m| m.close_popup());
            self.dirty = true;
            self.set_note(&format!("EQUALIZER PRESET {} SAVED", name));
        }
        changed
    }

    /// One preset card, `w` wide: its name and a preview of its curve, lit when it is the curve on.
    fn eq_card(&self, ui: &mut egui::Ui, name: &str, g: &[f32; 10], w: f32, h: f32) -> egui::Response {
        let (card, r) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
        let r = r.on_hover_cursor(egui::CursorIcon::PointingHand).tip(format!("{} preset", name));
        let on = self.eq_on && self.eq_gains == *g;
        inset(ui.painter(), card, if on || r.hovered() { pal().sel } else { pal().display });
        crate::ui_text(ui.painter(), Pos2::new(card.min.x + 6.0, card.min.y + 8.0), Align::Min, name, 1.0, w - 12.0, pal().ink);
        let gr = Rect::from_min_max(Pos2::new(card.min.x + 6.0, card.min.y + 15.0), card.max - Vec2::new(6.0, 5.0));
        let pts: Vec<Pos2> = (0..10)
            .map(|i| Pos2::new(gr.min.x + gr.width() * i as f32 / 9.0, gr.center().y - g[i] / 12.0 * gr.height() * 0.5))
            .collect();
        // (a low card: just its name)
        if gr.height() >= 6.0 {
            ui.painter().add(egui::Shape::line(pts, Stroke::new(1.5_f32, pal().red)));
        }
        r
    }

    /// The equalizer, docked under the player (as Winamp stacks it): the skin's own with a skin on, the
    /// preset cards under it; Backline's otherwise. X (or the skin's close button) puts it away.
    pub(crate) fn eq_dock(&mut self, ui: &mut egui::Ui) {
        let inner = ui.max_rect();
        window_deco(ui, inner, "EQUALIZER");
        // the close button, in the title strip
        // (drawn to the slot's height: a full-size button would stand taller than the strip's band)
        let xr = crate::title_slot(inner, crate::title_right(inner), 30.0, 22.0);
        let xb = ui
            .interact(xr, ui.id().with("eq_close"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("Close the equalizer.");
        raised_h(ui.painter(), xr, xb.is_pointer_button_down_on(), xb.hovered());
        crate::winamp_ui::as_title(|| ui_text(ui.painter(), xr.center(), Align::Center, "X", 2.0, xr.width() - 6.0, pal().ink));
        if xb.clicked() {
            self.show_eq = false;
        }
        // the page, clear of the frame all round
        let mut page = crate::page_ui(ui, inner);
        let ui = &mut page;
        if self.wa_art.is_some() {
            let full = ui.available_rect_before_wrap();
            // the preset cards under it give way as you size the equalizer up (as the cover beside the player
            // does): lower, then gone; they come back as it gets smaller
            let strip_h = match self.eq_zoom {
                Some(z) => (full.height() - 116.0 * z - 6.0).clamp(0.0, 56.0),
                None => 56.0,
            };
            let strip_h = if strip_h < 30.0 { 0.0 } else { strip_h };
            let eq_area =
                Rect::from_min_max(full.min, Pos2::new(full.max.x, full.max.y - strip_h - if strip_h > 0.0 { 4.0 } else { 0.0 }));
            self.winamp_eq(ui, eq_area);
            let mut changed = false;
            if strip_h > 0.0 {
                let strip = Rect::from_min_max(Pos2::new(full.min.x, full.max.y - strip_h), full.max);
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(strip), |ui| {
                    ui.horizontal(|ui| {
                        changed = self.eq_presets(ui, strip_h - 14.0);
                    });
                });
            }
            if changed {
                self.apply_eq();
                self.eq_remember();
                self.dirty = true;
            }
        } else {
            self.eq_view(ui);
        }
    }

    pub(crate) fn eq_view(&mut self, ui: &mut egui::Ui) {
        const LABELS: [&str; 10] = ["31", "62", "125", "250", "500", "1K", "2K", "4K", "8K", "16K"];
        ui.add_space(6.0);
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.add_space(4.0);
            if retro_btn_w(ui, if self.eq_on { "EQ ON" } else { "EQ OFF" }, 84.0, self.eq_on).clicked() {
                self.eq_on = !self.eq_on;
                changed = true;
            }
            if retro_btn_w(ui, "AUTO", 64.0, self.eq_auto)
                .tip("Remembers the EQ for each song and restores it when it plays.")
                .clicked()
            {
                self.eq_auto = !self.eq_auto;
                self.dirty = true;
            }
            ui.add_space(8.0);
            changed |= self.eq_presets(ui, 40.0);
        });
        ui.add_space(8.0);
        // Winamp-style: the response curve on a little screen, then the sliders, each filled green to red
        // from the middle line to its knob
        let heat = |g: f32| -> Color32 {
            // -12 dB green, 0 yellow-green, +12 red
            let f = ((g + 12.0) / 24.0).clamp(0.0, 1.0);
            let (r, gr) = if f < 0.5 { (f * 2.0 * 230.0, 210.0) } else { (230.0, 210.0 - (f - 0.5) * 2.0 * 170.0) };
            Color32::from_rgb(r as u8, gr as u8, 40)
        };
        let eq_on = self.eq_on;
        let col_on = move |c: Color32| if eq_on { c } else { c.gamma_multiply(0.35) };
        let area = ui.available_rect_before_wrap();
        let scale_w = 40.0;
        // the curve
        let (graph, _) = ui.allocate_exact_size(Vec2::new(area.width(), 46.0), Sense::hover());
        inset(ui.painter(), graph, pal().display);
        let gi = graph.shrink2(Vec2::new(10.0, 6.0));
        let gx = |i: usize| gi.min.x + scale_w + (gi.width() - scale_w) * (i as f32 + 0.5) / 10.0;
        let gy = |g: f32| gi.center().y - g / 12.0 * gi.height() * 0.5;
        ui.painter().line_segment(
            [Pos2::new(gi.min.x + scale_w, gi.center().y), Pos2::new(gi.max.x, gi.center().y)],
            Stroke::new(1.0_f32, pal().dim.gamma_multiply(0.6)),
        );
        // a smooth line through the ten bands (each piece coloured by its height)
        let pts: Vec<(f32, f32)> = (0..10).map(|i| (gx(i), self.eq_gains[i])).collect();
        for w in 0..9 {
            let (x0, g0) = pts[w];
            let (x1, g1) = pts[w + 1];
            for s in 0..8 {
                let (u0, u1) = (s as f32 / 8.0, (s + 1) as f32 / 8.0);
                let ease = |u: f32| u * u * (3.0 - 2.0 * u);
                let ga = g0 + (g1 - g0) * ease(u0);
                let gb = g0 + (g1 - g0) * ease(u1);
                ui.painter().line_segment(
                    [Pos2::new(x0 + (x1 - x0) * u0, gy(ga)), Pos2::new(x0 + (x1 - x0) * u1, gy(gb))],
                    Stroke::new(2.0_f32, col_on(heat((ga + gb) / 2.0))),
                );
            }
        }
        ptext(ui.painter(), Pos2::new(gi.min.x, gi.center().y), Align::Min, "CURVE", 1.0, pal().ink2);
        ui.add_space(8.0);

        let area = ui.available_rect_before_wrap();
        let h = (area.height() - 30.0).clamp(80.0, 220.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(area.width(), h), Sense::hover());
        let bands = Rect::from_min_max(Pos2::new(rect.min.x + scale_w, rect.min.y), rect.max);
        let col_w = bands.width() / 10.0;
        let (top, bot) = (rect.min.y + 18.0, rect.max.y - 18.0);
        let p = ui.painter().clone();
        let y0 = top + (bot - top) * 0.5;
        // the dB scale
        for (g, s) in [(12.0f32, "+12"), (0.0, "0 DB"), (-12.0, "-12")] {
            let y = top + (bot - top) * (0.5 - g / 24.0);
            ptext(&p, Pos2::new(rect.min.x + scale_w - 8.0, y), Align::Max, s, 1.0, pal().ink2);
        }
        fill_rect(&p, Rect::from_min_max(Pos2::new(bands.min.x + 4.0, y0), Pos2::new(bands.max.x - 4.0, y0 + 1.0)), pal().dim);
        for i in 0..10 {
            let cx = bands.min.x + col_w * (i as f32 + 0.5);
            let col = Rect::from_min_max(Pos2::new(cx - col_w / 2.0, rect.min.y), Pos2::new(cx + col_w / 2.0, rect.max.y));
            let r = ui.interact(col, ui.id().with(("eq_band", i)), Sense::click_and_drag());
            if r.dragged() || r.clicked() {
                if let Some(pp) = r.interact_pointer_pos() {
                    let f = ((pp.y - top) / (bot - top)).clamp(0.0, 1.0);
                    let mut g = ((12.0 - 24.0 * f) * 2.0).round() / 2.0;
                    if g.abs() < 0.75 {
                        g = 0.0;
                    }
                    self.eq_gains[i] = g;
                    self.eq_on = true;
                    changed = true;
                }
            }
            if r.double_clicked() {
                self.eq_gains[i] = 0.0;
                changed = true;
            }
            let g = self.eq_gains[i];
            let tw = (col_w * 0.28).clamp(6.0, 12.0);
            inset(&p, Rect::from_min_max(Pos2::new(cx - tw / 2.0, top), Pos2::new(cx + tw / 2.0, bot)), pal().groove);
            let y = top + (bot - top) * (0.5 - g / 24.0);
            // the fill: from the middle line to the knob, in the colour of how far it is pushed
            let (fy0, fy1) = if y < y0 { (y, y0) } else { (y0, y) };
            if fy1 - fy0 > 1.0 {
                fill_rect(
                    &p,
                    Rect::from_min_max(Pos2::new(cx - tw / 2.0 + 2.0, fy0), Pos2::new(cx + tw / 2.0 - 2.0, fy1)),
                    col_on(heat(g)),
                );
            }
            raised(&p, Rect::from_center_size(Pos2::new(cx, y), Vec2::new((col_w - 10.0).clamp(12.0, 22.0), 8.0)), r.dragged());
            let gt = if g.abs() < 0.05 { "0".to_string() } else { format!("{:+}", g.round() as i32) };
            ptext(&p, Pos2::new(cx, rect.min.y + 8.0), Align::Center, &gt, 1.0, if self.eq_on { pal().ink } else { pal().dim });
            ptext(&p, Pos2::new(cx, rect.max.y - 8.0), Align::Center, LABELS[i], 1.0, pal().ink2);
        }
        ui.add_space(4.0);
        para(ui, "Drag a slider, or double-click it to reset. The range is +/- 12 dB.", pal().dim);
        if changed {
            self.apply_eq();
            self.eq_remember();
            self.dirty = true;
        }
    }

    /// The skins, one section under another: BACKLINE (Backline's own looks and your Custom one) and WINAMP (the one
    /// you wear, the ones worn lately, and the way to the browser). The one worn is marked.
    fn skin_lists(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let worn = self.wa_worn.clone();
        let custom = crate::custom_worn(worn.as_ref());
        let ours = crate::backline_look(worn.as_ref()) && !custom;
        let museum = worn.clone().filter(|w| !crate::backline_look(Some(w)));
        let cur = SKIN.load(Ordering::Relaxed) % LOOKS.len();
        ui.add_space(6.0);
        // folded up: its name and the look worn from it; open to pick another
        let worn_here = if custom { Some("CUSTOM") } else { ours.then(|| SKIN_NAMES[cur]) };
        let head = format!(
            "{} {}",
            if self.skins_open[0] { "-" } else { "+" },
            crate::font::as_label(&format!(
                "Backline skins{}",
                worn_here.map(|n| format!("  ·  {}", crate::winamp_ui::display_case(n))).unwrap_or_default()
            ))
        );
        if menu_item(ui, &head) {
            self.skins_open[0] = !self.skins_open[0];
        }
        if self.skins_open[0] {
            // in a box of its own, so opening it doesn't push the page away
            crate::views::contained(ui, ("skin_list", 0), |ui| {
                for (n, name) in SKIN_NAMES.iter().enumerate() {
                    if menu_item(ui, &format!("{}{}", if ours && n == cur { "> " } else { "  " }, name)) {
                        acts.push(Action::Look(n));
                    }
                }
                if menu_item(ui, &format!("{}CUSTOM...", if custom { "> " } else { "  " })) {
                    acts.push(Action::EditLook);
                }
            });
        }
        ui.add_space(6.0);
        let head = format!(
            "{} {}",
            if self.skins_open[2] { "-" } else { "+" },
            crate::font::as_label(&format!(
                "Winamp skins{}",
                museum.as_ref().map(|w| format!("  ·  {}", w.1)).unwrap_or_default()
            ))
        );
        if menu_item(ui, &head) {
            self.skins_open[2] = !self.skins_open[2];
        }
        if self.skins_open[2] {
            // two groups, each folding open: your favourites, and the ones worn lately (X takes one off: a favourite
            // out of your favourites, the others off the list)
            let favs: Vec<(String, String)> = self.wa_favs.clone();
            let mut recent: Vec<(String, String)> = Vec::new();
            if let Some(w) = museum.clone().filter(|w| !favs.iter().any(|f| f.0 == w.0)) {
                recent.push(w);
            }
            for r in &self.wa_recent {
                if !recent.iter().any(|l| l.0 == r.0) && !favs.iter().any(|f| f.0 == r.0) {
                    recent.push(r.clone());
                }
            }
            for (g, (title, list, fav)) in [("FAVORITES", favs, true), ("RECENT", recent, false)].into_iter().enumerate() {
                if list.is_empty() {
                    continue;
                }
                let open = self.wa_groups_open[g];
                if menu_item(ui, &format!("    {} {}  ({})", if open { "-" } else { "+" }, title, list.len())) {
                    self.wa_groups_open[g] = !open;
                }
                if !open {
                    continue;
                }
                for (md5, name) in list {
                    let on = museum.as_ref().is_some_and(|w| w.0 == md5);
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        let tip = if fav { "Remove from favorites" } else { "Remove from this list" };
                        if retro_btn_w(ui, "X", 24.0, false).tip(tip).clicked() {
                            acts.push(if fav { Action::WaFav(md5.clone(), name.clone()) } else { Action::WaForget(md5.clone()) });
                        }
                        if menu_item(ui, &format!("{}{}", if on { "> " } else { "  " }, name.to_uppercase())) && !on {
                            acts.push(Action::WaApply(crate::winamp::WaSkin {
                                md5: md5.clone(),
                                name: name.clone(),
                                shot: String::new(),
                                download: String::new(),
                            }));
                        }
                    });
                }
            }
            if menu_item(ui, "  BROWSE WINAMP SKINS...") {
                acts.push(Action::WaOpen);
            }
        }
    }

    /// The BACKLINE tab: the skins, then everything about the app itself (preferences, help and tours, where
    /// tracks are stored, the log, your Tidal account).
    pub(crate) fn backline_page(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(6.0);
            title_line(ui, "BACKLINE", 3.0, pal().ink);
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "PREFERENCES", false).tip("Updates, sound output, startup sound, zoom and shortcuts.").clicked() {
                    acts.push(Action::TogglePrefs);
                }
                if retro_btn(ui, "WHAT'S NEW", false).tip("Changes in this version and recent ones.").clicked() {
                    acts.push(Action::News);
                }
                if retro_btn(ui, "HELP AND TOURS", false).tip("Help and tours  (F1)").clicked() {
                    acts.push(Action::ToggleHelp);
                }
            });
            ui.add_space(14.0);
            title_line(ui, "TIDAL ACCOUNT", 3.0, pal().ink);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if self.offline {
                    if retro_btn(ui, "LOG IN TO TIDAL", false)
                        .tip("Add your Tidal account. Everything else works without it.")
                        .clicked()
                    {
                        acts.push(Action::Offline(false));
                    }
                } else if retro_btn(ui, "LOG OUT", false).clicked() {
                    acts.push(Action::Logout);
                }
            });
            ui.add_space(14.0);
            title_line(ui, "SKINS", 3.0, pal().ink);
            ui.add_space(6.0);
            self.skin_lists(ui, acts);
            ui.add_space(20.0);
        });
    }

    /// RECENT: the songs played in Backline, newest first, by day - every one, cached or not (Tidal's own history
    /// only hears of what streams from it, and isn't open to Backline).
    fn recent_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "RECENT", 3.0, pal().ink);
        // played here, by day
        section_header(ui, "RECENTLY PLAYED");
        if self.store.recent.is_empty() {
            dim_line(ui, "NOTHING YET. SONGS SHOW UP HERE AS YOU PLAY THEM", 1.0, pal().dim);
        }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let day = |at: u64| -> &'static str {
            let ago = now.saturating_sub(at);
            if ago < 86_400 {
                "TODAY"
            } else if ago < 2 * 86_400 {
                "YESTERDAY"
            } else if ago < 7 * 86_400 {
                "THIS WEEK"
            } else {
                "EARLIER"
            }
        };
        let recent = &self.store.recent;
        let playing = self.cur_track().map(|t| t.id);
        let mut i = 0;
        while i < recent.len() {
            let label = day(recent[i].1);
            let n = recent[i..].iter().take_while(|r| day(r.1) == label).count();
            dim_line(ui, label, 1.0, pal().ink2);
            let tracks: Vec<Track> = recent[i..i + n].iter().map(|r| r.0.to_track()).collect();
            crate::tracks_list(ui, &tracks, playing, &self.liked, None, acts);
            ui.add_space(6.0);
            i += n;
        }
        ui.add_space(20.0);
    }

    /// Winamp skins from the Skin Museum: the one you wear (keep it, or back to Backline's look), then BROWSE (search
    /// the museum, its best when nothing's typed, more as you scroll), FAVORITES and RECENT, each a grid of
    /// screenshots. A click shows the skin big first (`wa_preview_panel`); WEAR IT puts it on.
    pub(crate) fn winamp_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let sc = crate::font::ui_scale();
        ui.add_space(6.0);
        title_line(ui, "WINAMP SKINS", 3.0, pal().ink);
        para(ui, "From the Winamp Skin Museum.", pal().ink2);
        self.wa_wearing(ui, acts);
        ui.add_space(8.0);
        let tabs = [
            "Browse".to_string(),
            format!("Favorites  {}", self.wa_favs.len()),
            format!("Recent  {}", self.wa_recent.len()),
        ];
        let tabs: Vec<&str> = tabs.iter().map(|t| t.as_str()).collect();
        if let Some(k) = crate::segments(ui, &tabs, self.wa_tab as usize) {
            self.wa_tab = k as u8;
        }
        ui.add_space(8.0);
        if self.wa_tab == 0 {
            self.wa_search_row(ui, acts);
            ui.add_space(4.0);
        }
        let q = self.wa_q.trim().to_string();
        let caption = match self.wa_tab {
            1 => "Kept until you take them off. Click one to see it big.".to_string(),
            2 => "The skins you wore lately, newest first.".to_string(),
            _ if q.is_empty() => "The museum's best. More load as you scroll.".to_string(),
            _ => format!("Skins matching \"{}\".", q),
        };
        crate::views::dim_line(ui, &caption, 1.0, pal().ink2);
        ui.add_space(6.0);
        let list = self.wa_tab_list();
        let worn = self.wa_worn.as_ref().map(|w| w.0.clone());
        let tab = self.wa_tab;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // cards as wide as the row allows (no gap left at the right), 150 at least
            let gap = 16.0 * sc;
            let seen = (ui.clip_rect().max.x - ui.cursor().min.x).min(ui.available_width()) - 4.0;
            let per_row = ((seen + gap) / (150.0 * sc + gap)).floor().max(1.0) as usize;
            let cw = ((seen - gap * (per_row - 1) as f32) / per_row as f32).min(260.0 * sc);
            for row in list.chunks(per_row) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for s in row {
                        self.wa_card(ui, acts, s, cw, worn.as_deref() == Some(s.md5.as_str()), tab);
                    }
                });
                ui.add_space(gap);
            }
            match tab {
                1 if list.is_empty() => para(ui, "No favorites yet. Point at a skin and click its star.", pal().ink2),
                2 if list.is_empty() => para(ui, "The skins you wear show up here.", pal().ink2),
                0 if self.wa_busy => para(ui, "Loading skins...", pal().ink2),
                0 if list.is_empty() => para(ui, "No skins found. Try another word, or clear the search for the museum's best.", pal().ink2),
                0 if !self.wa_end => {
                    // the end of the grid in sight: the next page comes
                    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
                    if ui.is_rect_visible(r) {
                        acts.push(Action::WaMore);
                    }
                }
                _ => {}
            }
            ui.add_space(20.0);
        });
    }

    /// The skins the tab shows: the museum's (BROWSE), your favorites, or the ones worn lately.
    pub(crate) fn wa_tab_list(&self) -> Vec<crate::winamp::WaSkin> {
        let kept = |l: &[(String, String)]| -> Vec<crate::winamp::WaSkin> {
            l.iter()
                .map(|(md5, name)| crate::winamp::WaSkin {
                    md5: md5.clone(),
                    name: name.clone(),
                    shot: crate::winamp::shot_url(md5),
                    download: String::new(),
                })
                .collect()
        };
        match self.wa_tab {
            1 => kept(&self.wa_favs),
            2 => kept(&self.wa_recent),
            _ => self.wa_list.clone(),
        }
    }

    /// WEARING: the museum skin on now (its picture and name), a star to keep it, and the way back to Backline's
    /// own look. Its buttons wrap under it when the page is narrow, never off the edge.
    fn wa_wearing(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let Some((md5, name)) = self.wa_worn.clone().filter(|w| !w.0.starts_with("gen:")) else { return };
        let sc = crate::font::ui_scale();
        let fav = self.wa_favs.iter().any(|f| f.0 == md5);
        ui.add_space(4.0);
        egui::Frame::none()
            .fill(pal().row_alt)
            .stroke(Stroke::new(1.0_f32, pal().groove))
            .rounding(10.0)
            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(14.0, 8.0);
                    let th = 64.0 * sc;
                    let (r, _) = ui.allocate_exact_size(Vec2::new(th * 275.0 / 348.0, th), Sense::hover());
                    match self.images.get(&crate::winamp::shot_url(&md5)) {
                        Some(id) => {
                            egui::Image::new(egui::load::SizedTexture::new(id, r.size())).paint_at(ui, r);
                        }
                        None => {
                            ui.painter().rect_filled(r, 3.0, pal().display);
                        }
                    }
                    let nw = (crate::font::text_w(&name, 2.4) + 10.0).clamp(140.0, 360.0);
                    ui.vertical(|ui| {
                        ui.set_width(nw);
                        crate::views::dim_line(ui, "WEARING", 1.0, pal().ink2);
                        let (r, _) = ui.allocate_exact_size(Vec2::new(nw, 26.0 * sc), Sense::hover());
                        crate::font::ptext_fit(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, &name, 2.4, nw - 8.0, pal().ink);
                    });
                    if retro_btn(ui, if fav { "In Favorites" } else { "Add to Favorites" }, fav)
                        .tip(if fav { "In your favorites. Click to take it out." } else { "Keep it in your favorites." })
                        .clicked()
                    {
                        acts.push(Action::WaFav(md5.clone(), name.clone()));
                    }
                    if retro_btn(ui, "Backline Look", false).tip("Take the skin off: Backline's own look.").clicked() {
                        acts.push(if self.look_custom {
                            Action::EditLook
                        } else {
                            Action::Look(crate::skin::SKIN.load(std::sync::atomic::Ordering::Relaxed))
                        });
                    }
                });
            });
        ui.add_space(4.0);
    }

    /// BROWSE's search: a box you can see (a round edge and a magnifier), Enter to search, x to clear (back to the
    /// museum's best).
    fn wa_search_row(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let sc = crate::font::ui_scale();
        let h = bh() + 6.0 * sc;
        let w = (ui.clip_rect().max.x - ui.cursor().min.x).min(ui.available_width()) - 6.0;
        let (pill, _) = ui.allocate_exact_size(Vec2::new(w.max(160.0), h), Sense::hover());
        let p = ui.painter();
        p.rect_filled(pill, h / 2.0, pal().display);
        let focused = self.ed.id == crate::tools::F_WINAMP;
        p.rect_stroke(pill, h / 2.0, Stroke::new(1.0_f32, if focused { pal().ink2 } else { pal().groove.gamma_multiply(1.6) }));
        let ir = Rect::from_center_size(Pos2::new(pill.min.x + 18.0 * sc, pill.center().y), Vec2::splat(16.0 * sc));
        crate::vicon::named(p, ir, "search", pal().ink2);
        let clear_w = if self.wa_q.is_empty() { 0.0 } else { 30.0 * sc };
        let fr = Rect::from_min_max(
            Pos2::new(ir.max.x + 4.0, pill.center().y - bh() / 2.0),
            Pos2::new(pill.max.x - 12.0 - clear_w, pill.center().y + bh() / 2.0),
        );
        let o = crate::views::field_bare(ui, &mut self.ed, crate::tools::F_WINAMP, &mut self.wa_q, fr, "Search the museum (zelda, metal, anime)");
        if o.enter {
            acts.push(Action::WaSearch);
        }
        if clear_w > 0.0 {
            let cr = Rect::from_center_size(Pos2::new(pill.max.x - 12.0 - clear_w / 2.0, pill.center().y), Vec2::splat(22.0 * sc));
            let x = ui.interact(cr, ui.id().with("wa_clear"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Clear: the museum's best");
            crate::vicon::named(ui.painter(), cr.shrink(4.0 * sc), "close", if x.hovered() { pal().ink } else { pal().ink2 });
            if x.clicked() {
                self.wa_q.clear();
                self.ed.id = 0;
                acts.push(Action::WaSearch);
            }
        }
    }

    /// One skin in the grid: its screenshot and name. The one you wear has a lit ring and a WEARING tag; the
    /// pointer over it shows a star (always shown, filled, on a favorite). A click shows it big; right-click for
    /// the rest.
    fn wa_card(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, s: &crate::winamp::WaSkin, cw: f32, on: bool, tab: u8) {
        let sc = crate::font::ui_scale();
        let ch = cw * 348.0 / 275.0;
        let name_h = 26.0 * sc;
        let (cell, r) = ui.allocate_exact_size(Vec2::new(cw, ch + name_h), Sense::click());
        if !ui.is_rect_visible(cell) {
            return;
        }
        let shot = Rect::from_min_size(cell.min, Vec2::new(cw, ch));
        let fav = self.wa_favs.iter().any(|f| f.0 == s.md5);
        let p = ui.painter().clone();
        match self.images.get(&s.shot) {
            Some(id) => {
                egui::Image::new(egui::load::SizedTexture::new(id, shot.size())).paint_at(ui, shot);
            }
            None => {
                p.rect_filled(shot, 4.0, pal().display);
            }
        }
        let hover = r.hovered();
        if on {
            p.rect_stroke(shot.expand(2.0), 4.0, Stroke::new(2.0_f32, pal().bar_txt));
            p.rect_stroke(shot.expand(5.0), 6.0, Stroke::new(3.0_f32, pal().bar_txt.gamma_multiply(0.25)));
        } else if hover {
            p.rect_stroke(shot.expand(1.5), 4.0, Stroke::new(1.5_f32, pal().ink2));
        }
        if on {
            tag(&p, Pos2::new(shot.min.x + 8.0, shot.max.y - 8.0), "Wearing", pal().bar_txt, pal().display);
        }
        if self.wa_loading.as_deref() == Some(s.name.as_str()) {
            p.rect_filled(shot, 4.0, Color32::from_black_alpha(140));
            tag(&p, Pos2::new(shot.min.x + 8.0, shot.max.y - 8.0), "Putting it on...", pal().ink, pal().display);
        }
        // the star: on the pointer, or always on a favorite
        let star = Rect::from_min_size(Pos2::new(shot.max.x - 34.0 * sc, shot.min.y + 6.0 * sc), Vec2::splat(28.0 * sc));
        let sr = ui.interact(star, ui.id().with(("wa_star", &s.md5)), Sense::click());
        if hover || fav || sr.hovered() {
            p.circle_filled(star.center(), star.width() / 2.0, Color32::from_black_alpha(200));
            star_mark(&p, star.center(), 8.5 * sc, if fav { Some(pal().bar_txt) } else { None }, if fav { pal().bar_txt } else { Color32::from_gray(235) });
        }
        let sr = sr.on_hover_cursor(egui::CursorIcon::PointingHand).tip(if fav { "In your favorites. Click to take it out." } else { "Add to favorites" });
        if sr.clicked() {
            acts.push(Action::WaFav(s.md5.clone(), s.name.clone()));
        }
        let label = crate::font::fit(&s.name, 1.5, cw);
        crate::font::ptext(&p, Pos2::new(cell.min.x + 2.0, cell.max.y - name_h / 2.0 + 2.0), Align::Min, &label, 1.5, if on { pal().ink } else { pal().ink2 });
        let r = r.on_hover_cursor(egui::CursorIcon::PointingHand).tip(format!("{}. Click to see it big.", s.name));
        if r.clicked() && !sr.hovered() {
            self.wa_preview = Some(s.clone());
        }
        r.context_menu(|ui| {
            if menu_item(ui, "Wear It") {
                acts.push(Action::WaApply(s.clone()));
                ui.close_menu();
            }
            if menu_item(ui, if fav { "Remove from Favorites" } else { "Add to Favorites" }) {
                acts.push(Action::WaFav(s.md5.clone(), s.name.clone()));
                ui.close_menu();
            }
            if tab == 2 && menu_item(ui, "Remove from This List") {
                acts.push(Action::WaForget(s.md5.clone()));
                ui.close_menu();
            }
        });
    }

    /// A skin seen big before it goes on: its screenshot drawn twice size or as big as fits (crisp pixels), WEAR IT
    /// (Enter), the star, PREVIOUS / NEXT through the grid it came from (the arrow keys), CLOSE (Esc).
    pub(crate) fn wa_preview_panel(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        let Some(s) = self.wa_preview.clone() else { return };
        let list = self.wa_tab_list();
        let at = list.iter().position(|x| x.md5 == s.md5);
        let (mut wear, mut close, mut step) = ctx.input(|i| {
            let step = if i.key_pressed(egui::Key::ArrowLeft) {
                -1
            } else if i.key_pressed(egui::Key::ArrowRight) {
                1
            } else {
                0
            };
            (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape), step)
        });
        let fav = self.wa_favs.iter().any(|f| f.0 == s.md5);
        let worn = self.wa_worn.as_ref().is_some_and(|w| w.0 == s.md5);
        let screen = ctx.screen_rect();
        // twice the museum's picture, or as big as the window lets it be
        let h = (348.0_f32 * 2.0).min(screen.height() - 240.0).max(240.0);
        let w = h * 275.0 / 348.0;
        let tex = self.images.get(&s.shot);
        crate::credits::panel(ctx, "wa_preview", w + 24.0, |ui| {
            title_line(ui, &crate::font::as_label(&s.name), 3.0, pal().ink);
            ui.add_space(6.0);
            {
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
                match tex {
                    Some(id) => {
                        egui::Image::new(egui::load::SizedTexture::new(id, r.size())).paint_at(ui, r);
                    }
                    None => {
                        ui.painter().rect_filled(r, 4.0, pal().display);
                        crate::font::ptext(ui.painter(), r.center(), Align::Center, "Loading the picture...", 1.5, pal().ink2);
                    }
                }
            }
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, if worn { "WEARING IT" } else { "WEAR IT" }, true).tip("Put this skin on  (Enter)").clicked() {
                    wear = true;
                }
                if retro_btn(ui, if fav { "In Favorites" } else { "Add to Favorites" }, fav).clicked() {
                    acts.push(Action::WaFav(s.md5.clone(), s.name.clone()));
                }
                if at.is_some_and(|k| k > 0) && retro_btn(ui, "Previous", false).tip("The skin before  (Left)").clicked() {
                    step = -1;
                }
                if at.is_some_and(|k| k + 1 < list.len()) && retro_btn(ui, "Next", false).tip("The next skin  (Right)").clicked() {
                    step = 1;
                }
                if retro_btn(ui, "Close", false).tip("Esc").clicked() {
                    close = true;
                }
            });
        });
        if wear {
            if !worn {
                acts.push(Action::WaApply(s.clone()));
            }
            self.wa_preview = None;
        } else if close {
            self.wa_preview = None;
        } else if step != 0 {
            if let Some(k) = at.map(|k| k as i64 + step).filter(|k| *k >= 0 && (*k as usize) < list.len()) {
                self.wa_preview = Some(list[k as usize].clone());
            }
        }
    }

    /// The BACKLINE menu, next to HOME and LIBRARY: skins (and Winamp skins), preferences, help, storage, the log.
    /// What the BACKLINE menu holds (also opened by a Winamp skin's own options button). True once something
    /// was picked, so the menu can close.
    pub(crate) fn app_menu_items(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) -> bool {
        ui.set_min_width(240.0);
        let before = acts.len();
        let mut done = false;
        if menu_item(ui, "Visuals, full screen  (G)") {
            acts.push(Action::Visuals);
        }
        // what the player's chips do, for a skin's deck (it has none)
        if self.wa_art.is_some() && !self.mini {
            if menu_item(ui, "Now Playing") {
                acts.push(Action::ToggleArt);
            }
            if PRACTICE
                && menu_item(ui, if self.practice { "Listen (practice tools off)" } else { "Practice (loop, slow down)" })
            {
                acts.push(Action::TogglePractice);
            }
            if menu_item(ui, if self.lib_hidden { "Show Library" } else { "Hide Library" }) {
                acts.push(Action::ToggleLibrary);
            }
            if menu_item(ui, if self.wa_guide { "Button Guide  (on)" } else { "Button Guide (name each button)" }) {
                self.wa_guide = !self.wa_guide;
                self.dirty = true;
            }
            if menu_item(ui, if self.wa_outline { "Button Outlines  (on)" } else { "Button Outlines (when pointing)" }) {
                self.wa_outline = !self.wa_outline;
                self.dirty = true;
            }
            ui.add_space(4.0);
        }
        // the window: keep it on top, the mini player and back
        if menu_item(ui, if self.go_offline { "Go Online" } else { "Go Offline (only what's on this computer)" }) {
            acts.push(Action::GoOffline(!self.go_offline));
        }
        if menu_item(ui, if self.pinned { "Always on Top  (on)" } else { "Always on Top" }) {
            acts.push(Action::TogglePin);
        }
        if menu_item(ui, if self.mini { "Exit Mini Player" } else { "Mini Player" }) {
            acts.push(Action::ToggleMini);
        }
        if menu_item(ui, if self.fullscreen { "Exit full screen  (F11)" } else { "Full screen  (F11)" }) {
            acts.push(Action::ToggleFullscreen);
        }
        if menu_item(ui, "Preferences") {
            acts.push(Action::TogglePrefs);
        }
        if menu_item(ui, "Help and tours  (F1)") {
            acts.push(Action::ToggleHelp);
        }
        if menu_item(ui, "Storage") {
            acts.push(Action::ToggleCacheView);
        }
        let log_on = self.side == Some(Side::Log);
        if menu_item(ui, if log_on { "Log  (close)" } else { "Log" }) {
            if log_on {
                self.side = None;
            } else {
                self.side = Some(Side::Log);
                self.show_eq = false;
            }
            done = true;
        }
        // the Winamp skin worn (or the way in to them): opens the skin browser
        let wa_label = match &self.wa_worn {
            Some((md5, name)) if !md5.starts_with("gen:") => format!("> WINAMP: {}", name.to_uppercase()),
            _ => "> WINAMP SKINS".to_string(),
        };
        if menu_item(ui, &wa_label) && self.side != Some(Side::Skins) {
            acts.push(Action::WaOpen);
        }
        ui.add_space(4.0);
        let worn = self.wa_worn.clone();
        let recent = self.wa_recent.clone();
        skin_menu(ui, acts, worn.as_ref(), &recent);
        done || acts.len() != before
    }

    /// The now-playing bar (a skin worn): the cover, the song and who, a heart, how far in - in the skin's colours.
    /// Click it for NOW PLAYING.
    fn now_bar(&mut self, ui: &mut egui::Ui, bar: Rect, tr: &crate::api::Track, acts: &mut Vec<Action>) {
        let id = ui.id().with("now_bar");
        let resp = ui
            .interact(bar, id, Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("Now Playing  (A)");
        let p = ui.painter().clone();
        // the window's foot: a shade apart from the page, a fine line over it
        p.rect_filled(bar, 0.0, if resp.hovered() { side_tone().lerp_to_gamma(pal().row_sel, 0.5) } else { side_tone() });
        p.line_segment([bar.left_top(), bar.right_top()], Stroke::new(1.0_f32, divider()));
        let cover = Rect::from_min_size(bar.min + Vec2::new(6.0, 6.0), Vec2::splat(bar.height() - 12.0));
        if tr.cover.is_empty() {
            crate::paint_page_art(ui, &mut self.images, "", cover, false);
        } else {
            crate::paint_held(ui, &mut self.images, "now_bar", &crate::cover_url(&tr.cover, 160), cover, false);
        }
        let tx = cover.max.x + 10.0;
        let heart = Rect::from_min_size(Pos2::new(bar.max.x - 30.0, bar.center().y - 10.0), Vec2::new(crate::HEART_W, 20.0));
        // the controls, in the middle (or after the words when the bar is narrow)
        let playing = self.cur.is_some() && !self.stopped && !self.paused;
        let cw = 44.0;
        let cx = bar.center().x.max(tx + 220.0 + cw).min(heart.min.x - 2.5 * cw - 20.0);
        let mut clicked_control = false;
        // shuffle, previous, a round PLAY / PAUSE, next, repeat: smooth icons, as over the lists (shuffle and repeat
        // lit when on)
        let rep = match self.repeat {
            crate::Repeat::Off => "Repeat: off",
            crate::Repeat::All => "Repeat: all",
            crate::Repeat::One => "Repeat: one",
        };
        let rep_on = !matches!(self.repeat, crate::Repeat::Off);
        let rep_one = matches!(self.repeat, crate::Repeat::One);
        for (n, (icon, a, tip)) in [
            ("shuffle", Action::Shuffle, if self.shuffle { "Shuffle: on" } else { "Shuffle: off" }),
            ("prev", Action::Prev, "Previous"),
            (
                if playing { "pause" } else { "play" },
                if playing { Action::PauseBtn } else { Action::PlayBtn },
                if playing { "Pause" } else { "Play" },
            ),
            ("skip", Action::Next, "Next"),
            ("loop", Action::Repeat, rep),
        ]
        .into_iter()
        .enumerate()
        {
            let c = Pos2::new(cx + (n as f32 - 2.0) * cw, bar.center().y - 2.0);
            let d = if n == 2 {
                34.0
            } else if n == 0 || n == 4 {
                24.0
            } else {
                28.0
            };
            let r = Rect::from_center_size(c, Vec2::splat(d));
            let h = ui.interact(r, id.with(("ctl", n)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
            if n == 2 {
                p.circle_filled(c, d / 2.0 + if h.hovered() { 1.0 } else { 0.0 }, pal().ink);
                let off = if playing { 0.0 } else { 1.5 };
                crate::vicon::named(&p, Rect::from_center_size(c + Vec2::new(off, 0.0), Vec2::splat(d * 0.42)), icon, pal().display);
            } else {
                let lit = (n == 0 && self.shuffle) || (n == 4 && rep_on);
                let col = if lit {
                    pal().bar_txt
                } else if h.hovered() {
                    pal().ink
                } else {
                    pal().ink2
                };
                crate::vicon::named(&p, Rect::from_center_size(c, Vec2::splat(d * 0.6)), icon, col);
                if lit {
                    p.circle_filled(Pos2::new(c.x, c.y + d * 0.55), 1.8, col);
                }
                if n == 4 && rep_one {
                    crate::font::keep_caps(|| ptext(&p, Pos2::new(c.x + d * 0.42, c.y - d * 0.3), Align::Center, "1", 1.0, col));
                }
            }
            if h.clicked() {
                acts.push(a);
                clicked_control = true;
            }
        }
        let room = (cx - 2.5 * cw - 12.0) - tx;
        crate::row_text(&p, Pos2::new(tx, bar.min.y + 17.0), Align::Min, &tr.title, room, pal().bar_txt);
        crate::row_text(&p, Pos2::new(tx, bar.min.y + 35.0), Align::Min, &tr.artists_text(), room, pal().ink2);
        // how far in: a thin line along the foot - click or drag it to move in the song
        let f = if tr.duration > 0.0 { (self.pos() / tr.duration).clamp(0.0, 1.0) } else { 0.0 };
        let line = Rect::from_min_max(Pos2::new(bar.min.x + 2.0, bar.max.y - 4.0), Pos2::new(bar.max.x - 2.0, bar.max.y - 2.0));
        let seek = ui
            .interact(line.expand2(Vec2::new(0.0, 4.0)), id.with("seek"), Sense::click_and_drag())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let mut seeking = false;
        if seek.clicked() || seek.dragged() {
            if let (Some(pp), true) = (seek.interact_pointer_pos(), tr.duration > 0.0) {
                acts.push(Action::Seek(((pp.x - line.min.x) / line.width()).clamp(0.0, 1.0) * tr.duration));
                seeking = true;
            }
        }
        if crate::draw_heart(ui, heart, self.liked.contains(&tr.id), resp.hovered(), id.with("heart")) {
            acts.push(Action::ToggleLike(tr.clone()));
        } else if resp.clicked() && !clicked_control && !seeking {
            acts.push(Action::ToggleArt);
        }
        p.rect_filled(line, 0.0, pal().dim.gamma_multiply(0.4));
        p.rect_filled(Rect::from_min_size(line.min, Vec2::new(line.width() * f, line.height())), 0.0, pal().bar_txt);
        if self.cur.is_some() && !self.stopped && !self.paused {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    pub(crate) fn library_ui(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        window_deco(ui, ui.max_rect(), "LIBRARY");
        // a skin's deck has no title line: its chips (NOW PLAYING, the guide, online...) sit on the library's
        if self.wa_art.is_some() && !self.mini && !crate::winamp_ui::chrome_on() {
            self.title_chips(ui, ui.max_rect(), acts);
        }
        // a skin worn: the places down a list at the side (as Spotify has them), the page beside it
        if self.wa_art.is_some() && !self.mini && crate::winamp_ui::chrome_on() {
            // one surface inside the frame, as Spotify's and Tidal's are: the places down the side a shade apart and
            // split off by a fine line, the page straight on the list's colour, Now Playing along the foot - no
            // boxes inside boxes
            let all = crate::frame_body(ui.max_rect());
            let p = ui.painter().clone();
            p.rect_filled(all, 0.0, pal().display);
            // (the places get the window's whole height; Now Playing is the page's foot)
            let sw = (212.0 * font::ui_scale()).clamp(160.0, (all.width() * 0.34).max(160.0));
            let side = Rect::from_min_max(all.min, Pos2::new(all.min.x + sw, all.max.y));
            p.rect_filled(side, 0.0, side_tone());
            p.line_segment([side.right_top(), side.right_bottom()], Stroke::new(1.0_f32, divider()));
            self.side_nav(ui, side, acts);
            tour::mark("TABS", side);
            tour::mark("APPMENU", side);
            let mut rest = Rect::from_min_max(Pos2::new(side.max.x, all.min.y), all.max);
            if let Some(tr) = self.cur_track().filter(|_| all.height() > 300.0) {
                let bar = Rect::from_min_max(Pos2::new(rest.min.x, rest.max.y - 64.0 * font::ui_scale()), rest.max);
                rest.max.y = bar.min.y;
                self.now_bar(ui, bar, &tr, acts);
            }
            let main = rest.shrink2(Vec2::new(20.0, 14.0));
            let mut body = ui.new_child(egui::UiBuilder::new().max_rect(main).layout(egui::Layout::top_down(Align::Min)));
            self.library_body(&mut body, acts, false);
            ui.allocate_rect(all, Sense::hover());
            return;
        }
        let all = ui.max_rect();
        let mut page = crate::page_ui(ui, all);
        logo(&mut page);
        page.add_space(4.0);
        self.library_body(&mut page, acts, true);
        ui.allocate_rect(all, Sense::hover());
    }

    /// The places, down the side of the library (a skin worn): home, your music by where it is, the practice
    /// tools, and Backline's own pages at the foot.
    fn side_nav(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        let sc = font::ui_scale();
        // the brand at the top
        let gem = Rect::from_min_size(r.min + Vec2::new(12.0, 12.0), Vec2::splat(22.0 * sc));
        crate::vicon::gem(&p, gem, pal().ink, pal().ink2);
        ptext(&p, Pos2::new(gem.max.x + 8.0, gem.center().y), Align::Min, "Backline", 2.2, pal().ink);
        let gem =
            Rect::from_min_max(gem.min, Pos2::new(gem.max.x, gem.max.y + 8.0 + self.mode_switch(ui, r, gem.max.y + 8.0, acts)));
        let gem =
            Rect::from_min_max(gem.min, Pos2::new(gem.max.x, gem.max.y + 6.0 + self.window_icons(ui, r, gem.max.y + 6.0, acts)));

        let side = self.side.is_some();
        let on_lib = self.page.as_ref().is_some_and(|p| p.library);
        let here = |s: Sec| !side && self.sec == s;
        // the computer this one is linked with (SYNC): its songs, under LINKED DEVICES
        let peer = crate::peer_files::peer_name();
        let there = here(Sec::Files) && self.files_there && !self.files_here && peer.is_some();
        // (icon, name, lit, what it does); no icon: a group's heading
        let mut rows: Vec<(&str, &str, bool, u8)> = vec![
            ("home", "Home", here(Sec::Tidal) && !on_lib && !self.offline, 1),
            ("", "Music", false, 0),
            ("heart", "My Tracks", here(Sec::Mine), 2),
            ("list", "Lists", here(Sec::Lists), 3),
            ("tidal", "Tidal", here(Sec::Tidal) && (on_lib || self.offline), 4),
            ("folder", "Files", here(Sec::Files) && !there, 5),
            ("cloud", "SoundCloud", here(Sec::Sc), 6),
            ("youtube", "YouTube", here(Sec::Yt), 7),
        ];
        if PRACTICE {
            rows.extend([
                // practice in three groups: what you play, how it works, every day
                ("", "Repertoire", false, 0),
                ("note", "Tunes", here(Sec::Tunes), 10),
                ("chart", "Chart", here(Sec::Chart), 18),
                ("loop", "Loops", here(Sec::Loops), 12),
                ("melody", "Chord Melodies", here(Sec::Melodies), 16),
                ("", "Theory", false, 0),
                ("chords", "Chords", here(Sec::Chords), 14),
                ("lines", "Lines", here(Sec::Lines), 15),
                ("ear", "Ear Training", here(Sec::Ear), 17),
                ("", "Daily", false, 0),
                ("diary", "Diary", here(Sec::Diary), 11),
                ("book", "Reading", here(Sec::Reading), 13),
            ]);
        }
        if let Some(name) = peer.as_deref() {
            rows.extend([("", "Linked devices", false, 0), ("globe", name, there, 8)]);
        }
        let foot = [
            ("gear", "Settings", self.side == Some(Side::Backline), 20u8),
            ("skins", "Winamp Skins", self.side == Some(Side::Skins), 21),
            ("log", "Log", self.side == Some(Side::Log), 22),
        ];
        let top = gem.max.y + 14.0;
        let heads = rows.iter().filter(|r| r.0.is_empty()).count() as f32;
        let items = (rows.len() + foot.len()) as f32 - heads;
        let rh = (28.0 * sc).min((r.max.y - top - 12.0) / (items + heads * 0.9 + 0.5)).max(16.0);
        let mut y = top;
        let mut hit: Option<u8> = None;
        let row = |y: f32, (icon, name, on, key): (&str, &str, bool, u8), hit: &mut Option<u8>| {
            let rr = Rect::from_min_size(Pos2::new(r.min.x + 6.0, y), Vec2::new(r.width() - 12.0, rh - 2.0));
            let resp =
                ui.interact(rr, ui.id().with(("side_nav", key)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if on {
                p.rect_filled(rr, 5.0, pal().panel.lerp_to_gamma(pal().display, 0.3));
                p.rect_filled(Rect::from_min_size(rr.min, Vec2::new(3.0, rr.height())), 1.5, pal().bar_txt);
            } else if resp.hovered() {
                p.rect_filled(rr, 5.0, Color32::from_white_alpha(10));
            }
            let col = if on || resp.hovered() { pal().ink } else { pal().ink2 };
            let ic =
                Rect::from_center_size(Pos2::new(rr.min.x + 18.0 * sc, rr.center().y), Vec2::splat((16.0 * sc).min(rh - 8.0)));
            crate::vicon::named(&p, ic, icon, col);
            let tx = rr.min.x + 36.0 * sc;
            ptext_fit(&p, Pos2::new(tx, rr.center().y), Align::Min, name, 1.8, rr.max.x - tx - 4.0, col);
            if resp.clicked() {
                *hit = Some(key);
            }
        };
        for it in rows {
            if it.0.is_empty() {
                // a group's heading: small, dim
                y += rh * 0.3;
                font::keep_caps(|| {
                    ptext(&p, Pos2::new(r.min.x + 14.0, y + rh * 0.3), Align::Min, &it.1.to_uppercase(), 1.25, pal().dim)
                });
                y += rh * 0.6;
                continue;
            }
            row(y, it, &mut hit);
            y += rh;
        }
        // Backline's own pages, at the foot
        let mut y = (r.max.y - 8.0 - foot.len() as f32 * rh).max(y + rh * 0.5);
        p.line_segment(
            [Pos2::new(r.min.x + 12.0, y - 6.0), Pos2::new(r.max.x - 12.0, y - 6.0)],
            Stroke::new(1.0_f32, pal().panel_dk),
        );
        for it in foot {
            row(y, it, &mut hit);
            y += rh;
        }
        match hit {
            Some(1) => acts.push(if self.offline { Action::Section(Sec::Files) } else { Action::Home }),
            Some(2) => acts.push(Action::Section(Sec::Mine)),
            Some(3) => acts.push(Action::Section(Sec::Lists)),
            Some(4) => acts.push(if self.offline { Action::Section(Sec::Tidal) } else { Action::Library }),
            Some(5) => {
                self.files_there = false;
                self.files_here = true;
                acts.push(Action::Section(Sec::Files));
            }
            // the linked computer: its songs, to copy the ones you want
            Some(8) => {
                self.files_there = true;
                self.files_here = false;
                self.peer.list = None;
                acts.push(Action::Section(Sec::Files));
            }
            Some(6) => acts.push(Action::Section(Sec::Sc)),
            Some(7) => acts.push(Action::Section(Sec::Yt)),
            Some(10) => acts.push(Action::Section(Sec::Tunes)),
            Some(11) => acts.push(Action::Section(Sec::Diary)),
            Some(12) => acts.push(Action::Section(Sec::Loops)),
            Some(13) => acts.push(Action::Section(Sec::Reading)),
            Some(14) => acts.push(Action::Section(Sec::Chords)),
            Some(15) => acts.push(Action::Section(Sec::Lines)),
            Some(16) => acts.push(Action::Section(Sec::Melodies)),
            Some(17) => acts.push(Action::Section(Sec::Ear)),
            Some(18) => acts.push(Action::OpenChart),
            Some(20) => acts.push(Action::BacklinePage),
            Some(21) => acts.push(Action::WaOpen),
            Some(22) => acts.push(Action::ShowLog),
            _ => {}
        }
    }

    /// The window's own switches as a row of icons (a skin worn): online, focus, practice, the mini player, on top, the
    /// skin's guide. Returns the row's height.
    /// Listen / Practice: the app's two modes as a switch with words, under the name (returns its height).
    fn mode_switch(&mut self, ui: &mut egui::Ui, r: Rect, y: f32, acts: &mut Vec<Action>) -> f32 {
        if !PRACTICE {
            return 0.0;
        }
        let h = 30.0 * font::ui_scale();
        let area = Rect::from_min_max(Pos2::new(r.min.x + 10.0, y), Pos2::new(r.max.x - 10.0, y + h));
        let p = ui.painter().clone();
        p.rect_filled(area, h / 2.0, pal().panel.lerp_to_gamma(pal().display, 0.35));
        p.rect_stroke(area, h / 2.0, Stroke::new(1.0_f32, pal().panel_hover));
        let half = area.width() / 2.0;
        for (k, (icon, word, on)) in
            [("headphones", "LISTEN", !self.practice), ("wave", "PRACTICE", self.practice)].into_iter().enumerate()
        {
            let cell = Rect::from_min_size(Pos2::new(area.min.x + k as f32 * half, area.min.y), Vec2::new(half, h)).shrink(3.0);
            let resp = ui
                .interact(cell, ui.id().with(("mode", k)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .tip(if k == 0 {
                    "Listen: no loop, no slow-down."
                } else {
                    "Practice: loop, slow down, the band and your tools.  (P)"
                });
            if on {
                p.rect_filled(cell, cell.height() / 2.0, pal().ink);
            } else if resp.hovered() {
                p.rect_filled(cell, cell.height() / 2.0, Color32::from_white_alpha(14));
            }
            let col = if on { pal().display } else { pal().ink2 };
            let tw = text_w(word, 1.4);
            let x0 = cell.center().x - (tw + 18.0) / 2.0;
            crate::vicon::named(&p, Rect::from_center_size(Pos2::new(x0 + 7.0, cell.center().y), Vec2::splat(12.0)), icon, col);
            crate::winamp_ui::as_title(|| {
                crate::ui_text(
                    &p,
                    Pos2::new(x0 + 18.0, cell.center().y),
                    Align::Min,
                    word,
                    1.4,
                    cell.width() - (x0 + 18.0 - cell.min.x),
                    col,
                )
            });
            if k == 1 {
                tour::mark("CHIP", cell);
            }
            if resp.clicked() && !on {
                acts.push(Action::TogglePractice);
            }
        }
        h
    }

    fn window_icons(&mut self, ui: &mut egui::Ui, r: Rect, y: f32, acts: &mut Vec<Action>) -> f32 {
        let p = ui.painter().clone();
        let off = self.go_offline || self.net_down;
        let mut items: Vec<(&str, bool, String, u8)> = vec![(
            "globe",
            !off,
            if self.go_offline {
                "Offline. Only what's on this computer plays. Click to go online.".into()
            } else if self.net_down {
                "No internet right now. Click to stay offline.".into()
            } else {
                "Online. Click to go offline.".into()
            },
            1,
        )];
        if PRACTICE {
            items.push(("focus", self.lib_hidden, "Hide the library: just the player and your tools.".into(), 2));
        }
        items.push(("mini", false, "Mini player.".into(), 4));
        items.push(("pin", self.pinned, "Always on top".into(), 5));
        items.push(("guide", self.wa_guide, "Label this skin's buttons.".into(), 6));
        let d = 26.0 * font::ui_scale();
        let names = self.icon_names;
        // the icons spread evenly across the list's width (with names: a word under each)
        let n = items.len() as f32;
        let step = ((r.width() - 20.0) / n).max(d + 3.0);
        let mut x = r.min.x + 10.0 + (step - d) / 2.0;
        for (icon, on, tip, key) in items {
            let b = Rect::from_min_size(Pos2::new(x, y), Vec2::splat(d));
            x += step;
            if names {
                let word = match key {
                    1 => {
                        if self.go_offline || self.net_down {
                            "Offline"
                        } else {
                            "Online"
                        }
                    }
                    2 => "Focus",
                    4 => "Mini",
                    5 => "On top",
                    _ => "Guide",
                };
                font::keep_caps(|| {
                    ptext_fit(&p, Pos2::new(b.center().x, b.max.y + 7.0), Align::Center, word, 0.95, step - 4.0, pal().ink2)
                });
            }
            if b.max.x > r.max.x - 6.0 {
                break;
            }
            let h = ui
                .interact(b, ui.id().with(("win_icon", key)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .tip(tip);
            if on {
                p.rect_filled(b, 5.0, pal().panel.lerp_to_gamma(pal().display, 0.3));
            } else if h.hovered() {
                p.rect_filled(b, 5.0, Color32::from_white_alpha(10));
            }
            let col = if on {
                pal().bar_txt
            } else if h.hovered() {
                pal().ink
            } else {
                pal().ink2
            };
            crate::vicon::named(&p, b.shrink(d * 0.22), icon, col);
            if key == 3 {
                tour::mark("CHIP", b);
            }
            if h.clicked() {
                match key {
                    1 => acts.push(Action::GoOffline(!self.go_offline)),
                    2 => acts.push(Action::ToggleLibrary),
                    3 => acts.push(Action::TogglePractice),
                    4 => acts.push(Action::ToggleMini),
                    5 => acts.push(Action::TogglePin),
                    _ => {
                        self.wa_guide = !self.wa_guide;
                        self.dirty = true;
                    }
                }
            }
        }
        if self.icon_names {
            d + 14.0
        } else {
            d
        }
    }

    /// The library's page: the places' tabs (`tabs`: when there is no side list), the search row, the page itself.
    fn library_body(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, tabs: bool) {
        // which list: Tidal / my files / YouTube / tunes / diary
        if tabs {
            let tabs_top = ui.cursor().min.y;
            self.section_bar(ui, acts);
            let lr = ui.min_rect();
            tour::mark("TABS", Rect::from_min_max(Pos2::new(lr.min.x, tabs_top), Pos2::new(lr.max.x, ui.cursor().min.y)));
        }

        // back, forward, HOME and the search box: one row
        let side = self.side.is_some();
        ui.horizontal(|ui| {
            for back in [true, false] {
                let ok = self.can_go(back);
                let col = if ok { pal().ink } else { pal().dim.gamma_multiply(0.6) };
                let icon: &[&str] = if back { &IC_BACK } else { &IC_FWD };
                let r = icon_btn_w(ui, icon, false, col, 38.0).tip(if back {
                    "Back. The mouse back button works too."
                } else {
                    "Forward. The mouse forward button works too."
                });
                if r.clicked() && ok {
                    acts.push(if back { Action::Back } else { Action::Forward });
                }
            }
            // HOME: Tidal home in MUSIC (your files when not signed in), your tunes in PRACTICE
            if !side && tabs {
                let in_practice = self.sec.practice();
                let tip = if in_practice {
                    "Your tunes."
                } else if self.offline {
                    "Your files."
                } else {
                    "Tidal home."
                };
                if ibtn(ui, &IC_HOME, "HOME", false).tip(tip).clicked() {
                    if in_practice {
                        acts.push(Action::Section(Sec::Tunes));
                        acts.push(Action::OpenTune(None));
                    } else if self.offline {
                        acts.push(Action::Section(Sec::Files));
                    } else {
                        acts.push(Action::Home);
                    }
                }
            }
            // on a practice page with the side list (no tab row to hold it): ALL PAGES, at the end of this row
            let pages = crate::PRACTICE
                && !tabs
                && !side
                && self.sec.practice();
            let pages_w = if pages { crate::views::pages_w() } else { 0.0 };
            if !self.offline && !side {
                let go_w = 54.0;
                let w = (ui.available_width() - go_w - pages_w - ui.spacing().item_spacing.x).max(80.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                self.search_input(ui, rect, acts);
                if retro_btn_w(ui, "GO", go_w, false).clicked() {
                    acts.push(Action::Search(self.search.clone()));
                }
            }
            if pages {
                self.go_to(ui, acts);
            }
        });

        // library tabs
        let is_lib = self.sec == Sec::Tidal && self.page.as_ref().map(|p| p.library).unwrap_or(false);
        if is_lib && self.side.is_none() {
            const TABS: [(Tab, &str); 5] = [
                (Tab::Tracks, "MY TRACKS"),
                (Tab::Recent, "RECENT"),
                (Tab::Lists, "LISTS"),
                (Tab::Albums, "ALBUMS"),
                (Tab::Artists, "ARTISTS"),
            ];
            let names: Vec<&str> = TABS.iter().map(|t| t.1).collect();
            let cur = TABS.iter().position(|t| t.0 == self.lib_tab).unwrap_or(0);
            if let Some(i) = tab_row(ui, &names, cur) {
                acts.push(Action::Tab(TABS[i].0));
            }
        }

        // persistent error line
        if self.status_err && !self.status.is_empty() {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::click());
            inset(ui.painter(), rect, pal().display);
            marquee(ui, rect, &self.status, 2.0, pal().red);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                acts.push(Action::ClearStatus);
            }
        }
        // (on Home only: the player at the foot already shows the song everywhere else)
        let home = self.sec == Sec::Tidal && !self.page.as_ref().is_some_and(|p| p.library);
        if home && self.side.is_none() {
            self.continue_bar(ui, acts);
        }
        ui.add_space(4.0);

        // list well: down to the page's foot (PAGE_PAD above the frame), so its own border shows and the last row
        // ends inside it instead of running into the frame
        let mut avail = ui.available_rect_before_wrap();
        // (a skin worn, the library one surface: no well, and Now Playing is the window's own foot)
        let flat = !tabs && crate::winamp_ui::chrome_on();
        // with a skin and a song: the now-playing bar along the foot (click it for NOW PLAYING)
        if self.wa_art.is_some() && !self.mini && avail.height() > 200.0 && !flat {
            if let Some(tr) = self.cur_track() {
                let bar = Rect::from_min_max(Pos2::new(avail.min.x, avail.max.y - 52.0), avail.max);
                avail.max.y = bar.min.y - 6.0;
                self.now_bar(ui, bar, &tr, acts);
            }
        }
        if avail.height() < 30.0 {
            return;
        }
        let area = if flat {
            avail
        } else {
            inset(ui.painter(), avail, pal().display);
            if crate::winamp_ui::chrome_on() {
                avail.shrink2(Vec2::new(12.0, 8.0))
            } else {
                avail.shrink(thick(3.0))
            }
        };
        let page = self.page.clone();
        let serial = self.serial;
        let loading = self.loading;
        let tab = self.lib_tab;
        let playing_id = self.cur_track().map(|t| t.id);
        let side = self.side;
        let sec = self.sec;
        let sec_i = sec as u8;
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(area), |ui| {
            // (a skin worn: room between the things on a page; the song tables stay flush, list_row sees to that)
            ui.spacing_mut().item_spacing = Vec2::new(8.0, if crate::winamp_ui::chrome_on() { GAP } else { 0.0 });
            match side {
                Some(Side::Log) => return self.log_view(ui, acts),
                Some(Side::Skins) => return self.winamp_view(ui, acts),
                Some(Side::Backline) => return self.backline_page(ui, acts),
                None => {}
            }
            if sec == Sec::Melodies {
                ui.push_id((serial, sec_i), |ui| self.melodies_view(ui, acts));
                return;
            }
            // the lead sheet fills the page (it fits the chart to the room, the band along its foot)
            if sec == Sec::Chart {
                ui.push_id((serial, sec_i), |ui| self.chart_ui(ui, acts));
                return;
            }
            // the scrollbar's room is always kept: a page about as tall as the window would otherwise gain and lose
            // its scrollbar every frame (the width changes, rows wrap, the height changes) and flicker
            // (a test copy can open scrolled down: BACKLINE_SCROLL=points; the chord finder scrolls itself)
            let mut sa = egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible);
            if let Some(y) = std::env::var("BACKLINE_SCROLL").ok().and_then(|s| s.parse::<f32>().ok()).filter(|_| sec != Sec::Chords) {
                sa = sa.vertical_scroll_offset(y);
            }
            sa.show(ui, |ui| {
                    ui.push_id((serial, sec_i), |ui| match sec {
                        // used without Tidal: the tab is still here, and logs you in
                        Sec::Tidal if self.offline => {
                            ui.add_space(10.0);
                            title_line(ui, "TIDAL", 3.0, pal().ink);
                            para(ui, "You are using Backline without Tidal. Your files, YouTube, SoundCloud and every practice tool still work. Log in to search and play Tidal's catalog, with your liked songs, playlists and artists.", pal().ink2);
                            if retro_btn(ui, "LOG IN WITH TIDAL", false).clicked() {
                                acts.push(Action::Offline(false));
                                acts.push(Action::StartLogin);
                            }
                        }
                        Sec::Tidal => {
                            if loading {
                                ui.add_space(6.0);
                                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 16.0), Sense::hover());
                                ptext(
                                    ui.painter(),
                                    Pos2::new(rect.min.x + 8.0, rect.center().y),
                                    Align::Min,
                                    "LOADING...",
                                    2.0,
                                    pal().ink2,
                                );
                            }
                            if let Some(pg) = &page {
                                // your Tidal playlists: make a new one here
                                if pg.library && tab == Tab::Lists {
                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(6.0);
                                        let w = (ui.available_width() - 190.0).max(80.0);
                                        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                                        let o = crate::views::field(
                                            ui,
                                            &mut self.ed,
                                            crate::tools::F_TIDAL_PL,
                                            &mut self.tidal_new_name,
                                            rect,
                                            "New Tidal playlist name",
                                            false,
                                        );
                                        if o.enter
                                            || retro_btn(ui, "+ NEW PLAYLIST", false)
                                                .tip("Create a playlist on Tidal")
                                                .clicked()
                                        {
                                            let name = std::mem::take(&mut self.tidal_new_name).trim().to_string();
                                            self.open_new_playlist(crate::new_playlist::NewPl {
                                                name,
                                                tidal: true,
                                                ..Default::default()
                                            });
                                        }
                                    });
                                    ui.add_space(4.0);
                                }
                                // RECENT: what you played here
                                if pg.library && tab == Tab::Recent {
                                    self.recent_view(ui, acts);
                                    return;
                                }
                                // your library: a box that filters it down as you type (every word must match)
                                if pg.library {
                                    let hint = match tab {
                                        Tab::Tracks => "Filter by title, artist or album",
                                        Tab::Lists => "Filter your playlists",
                                        Tab::Albums => "Filter your albums",
                                        _ => "Filter your artists",
                                    };
                                    self.filter_box(ui, hint);
                                }
                                let filter = if pg.library { self.lib_filter.clone() } else { String::new() };
                                page_view(ui, &mut self.images, pg, playing_id, tab, &self.liked, &filter, acts);
                            }
                        }
                        Sec::Files => self.files_view(ui, acts),
                        Sec::Yt => self.yt_view(ui, acts),
                        Sec::Sc => self.sc_view(ui, acts),
                        Sec::Tunes => self.tunes_view(ui, acts),
                        Sec::Diary => self.diary_view(ui, acts),
                        Sec::Loops => self.loops_view(ui, acts),
                        Sec::Reading => self.reading_view(ui, acts),
                        Sec::Chords => self.chords_view(ui, acts),
                        Sec::Lines => self.lines_view(ui, acts),
                        Sec::Melodies | Sec::Chart => {}
                        Sec::Ear => self.ear_view(ui, acts),
                        Sec::Lists => self.playlists_view(ui, acts),
                        Sec::Mine => self.mine_view(ui, acts),
                    });
                });
        });
        self.draw_toast(ui, avail);
        self.draw_copying(ui, avail);
    }

    /// The note over the library for a moment ("Added to ..."): a pill at the foot of the page, fading out.
    fn draw_toast(&mut self, ui: &egui::Ui, area: Rect) {
        let Some((text, at)) = self.toast.clone() else { return };
        const SHOW: f32 = 2.6;
        const FADE: f32 = 0.5;
        let age = (ui.input(|i| i.time) - at) as f32;
        if age > SHOW + FADE {
            self.toast = None;
            return;
        }
        let a = if age < 0.15 { age / 0.15 } else if age > SHOW { 1.0 - (age - SHOW) / FADE } else { 1.0 }.clamp(0.0, 1.0);
        let sc = font::ui_scale();
        let px = 2.0;
        let tw = font::text_w(&text, px).min((area.width() - 90.0 * sc).max(60.0));
        let (w, h) = (tw + 58.0 * sc, 34.0 * sc);
        let y = area.max.y - 18.0 * sc - h / 2.0 + (1.0 - a) * 6.0;
        let r = Rect::from_center_size(Pos2::new(area.center().x, y), Vec2::new(w, h));
        let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("lib_toast")));
        p.rect_filled(r.translate(Vec2::new(0.0, 3.0)), h / 2.0, egui::Color32::from_black_alpha((110.0 * a) as u8));
        p.rect_filled(r, h / 2.0, pal().bar_txt.gamma_multiply(a));
        // a tick, then the words
        let ink = pal().display.gamma_multiply(a);
        let c = Pos2::new(r.min.x + 20.0 * sc, r.center().y);
        let s = egui::Stroke::new(2.2 * sc, ink);
        p.line_segment([c + Vec2::new(-5.0, 0.0) * sc, c + Vec2::new(-1.5, 3.5) * sc], s);
        p.line_segment([c + Vec2::new(-1.5, 3.5) * sc, c + Vec2::new(5.5, -4.0) * sc], s);
        crate::ptext_fit(&p, Pos2::new(r.min.x + 36.0 * sc, r.center().y), Align::Min, &text, px, tw, ink);
        ui.ctx().request_repaint();
    }
}
