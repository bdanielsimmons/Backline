//! The library window (left): sign-in, search box, the source lists, and the log / EQ / storage views.

use super::*;

impl App {
    pub(crate) fn login_ui(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let full = ui.max_rect();
        let inner = Rect::from_center_size(full.center(), Vec2::new(480.0, 270.0));
        window_deco(ui, inner, "BACKLINE");
        let p = ui.painter();
        let c = inner.center();
        logo_mark(p, c + Vec2::new(-214.0, -102.0), 4.0);
        logo_mark(p, c + Vec2::new(178.0, -102.0), 4.0);
        ptext(p, c + Vec2::new(0.0, -84.0), Align::Center, "BACKLINE", 7.0, pal().ink);
        if PRACTICE {
            ptext(p, c + Vec2::new(0.0, -50.0), Align::Center, "A retro player for Tidal and more", 2.0, pal().ink2);
            ptext(p, c + Vec2::new(0.0, -32.0), Align::Center, "PRACTICE EDITION: LOOP, SLOW DOWN, LEARN TUNES", 1.0, pal().dim);
        } else {
            ptext(p, c + Vec2::new(0.0, -40.0), Align::Center, "A retro player for Tidal and more", 2.0, pal().ink2);
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
                    inset(ui.painter(), er, pal().lcd);
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
                self.show_log = false;
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
                                if menu_item(ui, "Delete this curve") {
                                    drop = Some(i);
                                    ui.close_menu();
                                }
                            });
                        }
                    }
                    // + : save the curve you have
                    let (card, r) = ui.allocate_exact_size(Vec2::new(card_h, card_h), Sense::click());
                    let r = r.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Save this curve as a preset of your own.");
                    inset(ui.painter(), card, if r.hovered() || self.eq_naming { pal().sel } else { pal().lcd });
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
            self.set_note(&format!("EQUALIZER CURVE {} DELETED", name));
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
                crate::para(ui, "NAME THIS CURVE", pal().ink2);
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(170.0, bh()), Sense::hover());
                    let o = crate::views::field(
                        ui,
                        &mut self.ed,
                        crate::tools::F_EQ_NAME,
                        &mut self.eq_name,
                        rect,
                        "My curve",
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
            self.set_note(&format!("EQUALIZER CURVE {} SAVED", name));
        }
        changed
    }

    /// One preset card, `w` wide: its name and a preview of its curve, lit when it is the curve on.
    fn eq_card(&self, ui: &mut egui::Ui, name: &str, g: &[f32; 10], w: f32, h: f32) -> egui::Response {
        let (card, r) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
        let r = r.on_hover_cursor(egui::CursorIcon::PointingHand).tip(format!("{} preset", name));
        let on = self.eq_on && self.eq_gains == *g;
        inset(ui.painter(), card, if on || r.hovered() { pal().sel } else { pal().lcd });
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
        let xr = Rect::from_min_size(Pos2::new(inner.max.x - 34.0, inner.min.y - 26.0), Vec2::new(30.0, 22.0));
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(xr), |ui| {
            if retro_btn_w(ui, "X", 30.0, false).tip("Close the equalizer.").clicked() {
                self.show_eq = false;
            }
        });
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
                .tip("Remembers the EQ for each song and brings it back when it plays.")
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
        inset(ui.painter(), graph, pal().lcd);
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

    /// The skins, one section under another: ORIGINAL (Backline's palettes as sleek Winamp-style skins) and
    /// WINAMP (the one you wear, the ones worn lately, and the way to the browser). The one worn is marked.
    fn skin_lists(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let look = crate::gen_look(self.wa_worn.as_ref());
        let museum = self.wa_worn.clone().filter(|_| look == 0);
        let cur = SKIN.load(Ordering::Relaxed) % PALS.len();
        for (gi, (title, g)) in [(0usize, ("ORIGINAL", 2usize))] {
            ui.add_space(6.0);
            // folded up: its name and the one worn from it; open to pick another
            let worn_here = (if g == 0 { self.wa_worn.is_none() } else { look == g }).then(|| SKIN_NAMES[cur]);
            let head = format!(
                "{} {}{}",
                if self.skins_open[gi] { "-" } else { "+" },
                title,
                worn_here.map(|n| format!(":  {}", n)).unwrap_or_default()
            );
            if menu_item(ui, &head) {
                self.skins_open[gi] = !self.skins_open[gi];
            }
            if !self.skins_open[gi] {
                continue;
            }
            // in a box of its own, so opening it doesn't push the page away
            crate::views::contained(ui, ("skin_list", 0), |ui| {
                for (n, name) in SKIN_NAMES.iter().enumerate() {
                    let on = n == cur && if g == 0 { self.wa_worn.is_none() } else { look == g };
                    if menu_item(ui, &format!("{}{}", if on { "> " } else { "  " }, name)) {
                        acts.push(Action::Theme(n, g));
                    }
                }
            });
        }
        ui.add_space(6.0);
        let head = format!(
            "{} WINAMP{}",
            if self.skins_open[2] { "-" } else { "+" },
            museum.as_ref().map(|w| format!(":  {}", w.1.to_uppercase())).unwrap_or_default()
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
            for (g, (title, list, fav)) in [("FAVOURITES", favs, true), ("WORN LATELY", recent, false)].into_iter().enumerate() {
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
                        let tip = if fav { "Take it out of your favourites." } else { "Take it off this list." };
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
                if retro_btn(ui, "PREFERENCES", false).tip("Updates, sound output, the opening sound, size and keys.").clicked() {
                    acts.push(Action::TogglePrefs);
                }
                if retro_btn(ui, "WHAT'S NEW", false).tip("What changed in this version and the few before it.").clicked() {
                    acts.push(Action::News);
                }
                if retro_btn(ui, "HELP AND TOURS", false).tip("Every key and tip, and the guided tours  (F1)").clicked() {
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
            // old screens, on top of any skin
            ui.horizontal_wrapped(|ui| {
                crate::views::label(ui, "OLD SCREEN", 120.0);
                const FX: [(&str, &str); 6] = [
                    ("SCANLINES", "The dark lines between a CRT's rows of light."),
                    ("TUBE EDGES", "Darker corners, where the glass curves away."),
                    ("GHOST SEGMENTS", "The unlit 8s of the time readout, faint behind the digits (Winamp skins)."),
                    ("BURN-IN", "The player's name worn into the glass behind the scrolling title (Winamp skins)."),
                    ("GLOW", "The lit digits and the title glow, as phosphor does (Winamp skins)."),
                    ("AFTERGLOW", "A changed digit lingers and fades, and the moving title leaves a trail (Winamp skins)."),
                ];
                for (i, (n, tip)) in FX.iter().enumerate() {
                    if crate::check_box(ui, n, self.crt[i]).tip(*tip).clicked() {
                        self.crt[i] = !self.crt[i];
                        self.dirty = true;
                    }
                }
            });
            // how strong the glow and afterglow are
            if self.crt[4] || self.crt[5] {
                ui.horizontal(|ui| {
                    crate::views::label(ui, "GLOW STRENGTH", 120.0);
                    if retro_btn_w(ui, "-", 28.0, false).clicked() {
                        self.crt_glow = self.crt_glow.saturating_sub(1).max(1);
                        self.dirty = true;
                    }
                    lcd_box(ui, &format!("{}", self.crt_glow), 40.0, pal().ink);
                    if retro_btn_w(ui, "+", 28.0, false).clicked() {
                        self.crt_glow = (self.crt_glow + 1).min(10);
                        self.dirty = true;
                    }
                    crate::views::dim_line(ui, "1 FAINT, 4 AS MADE, 10 BLAZING", 1.0, pal().dim);
                });
            }
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
        section_header(ui, "PLAYED IN BACKLINE");
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
        let recent = self.store.recent.clone();
        for v in recent.iter().map(|r| &r.0) {
            self.register(v);
        }
        let playing = self.cur_track().map(|t| t.id);
        let mut i = 0;
        while i < recent.len() {
            let label = day(recent[i].1);
            let n = recent[i..].iter().take_while(|r| day(r.1) == label).count();
            dim_line(ui, label, 1.0, pal().ink2);
            let tracks: Vec<Track> = recent[i..i + n].iter().map(|r| r.0.to_track()).collect();
            crate::tracks_list(ui, &tracks, playing, &self.liked, acts);
            ui.add_space(6.0);
            i += n;
        }
        ui.add_space(20.0);
    }

    /// Winamp skins from the Skin Museum: search, a grid of screenshots, click one to wear it.
    pub(crate) fn winamp_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "WINAMP SKINS", 3.0, pal().ink);
        // under the title: the ones worn lately and your favourites, each a drop-down (a little darker, set apart)
        {
            let favs = self.wa_favs.clone();
            if !self.wa_recent.is_empty() || !favs.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    // your favourites: kept for good, until you take them off
                    if !favs.is_empty() {
                        let fd = retro_btn(ui, "FAVOURITES  v", true).tip("Your favourite skins.");
                        let fmenu = ui.id().with("wa_favs");
                        if fd.clicked() {
                            ui.memory_mut(|m| m.toggle_popup(fmenu));
                        }
                        let worn_now = self.wa_worn.as_ref().map(|w| w.0.clone());
                        egui::popup::popup_below_widget(ui, fmenu, &fd, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                            ui.set_min_width(260.0);
                            for (md5, name) in favs.clone() {
                                ui.horizontal(|ui| {
                                    if retro_btn_w(ui, "X", 24.0, false).tip("Take it out of your favourites.").clicked() {
                                        acts.push(Action::WaFav(md5.clone(), name.clone()));
                                    }
                                    let on = worn_now.as_deref() == Some(md5.as_str());
                                    if menu_item(ui, &format!("{}{}", if on { "> " } else { "  " }, name.to_uppercase())) {
                                        acts.push(Action::WaApply(crate::winamp::WaSkin {
                                            md5,
                                            name,
                                            shot: String::new(),
                                            download: String::new(),
                                        }));
                                        ui.memory_mut(|m| m.close_popup());
                                    }
                                });
                            }
                        });
                    }
                    if self.wa_recent.is_empty() {
                        return;
                    }
                    let dd = retro_btn(ui, "WORN LATELY  v", true).tip("Skins you wore lately.");
                    let menu = ui.id().with("worn_lately");
                    if dd.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(menu));
                    }
                    let worn_now = self.wa_worn.as_ref().map(|w| w.0.clone());
                    egui::popup::popup_below_widget(ui, menu, &dd, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                        ui.set_min_width(260.0);
                        for (md5, name) in self.wa_recent.clone() {
                            ui.horizontal(|ui| {
                                if retro_btn_w(ui, "X", 24.0, false).tip("Take it off this list.").clicked() {
                                    acts.push(Action::WaForget(md5.clone()));
                                }
                                let on = worn_now.as_deref() == Some(md5.as_str());
                                if menu_item(ui, &format!("{}{}", if on { "> " } else { "  " }, name.to_uppercase())) {
                                    acts.push(Action::WaApply(crate::winamp::WaSkin {
                                        md5,
                                        name,
                                        shot: String::new(),
                                        download: String::new(),
                                    }));
                                    ui.memory_mut(|m| m.close_popup());
                                }
                            });
                        }
                    });
                });
            }
        }
        para(ui, "From the Winamp Skin Museum. Click one to wear it. The back arrow takes you back.", pal().ink2);
        if let Some((md5, name)) = self.wa_worn.clone().filter(|w| !w.0.starts_with("gen:")) {
            ui.horizontal(|ui| {
                para(ui, &format!("WEARING: {}", name.to_uppercase()), pal().ink);
                let fav = self.wa_favs.iter().any(|f| f.0 == md5);
                if retro_btn(ui, if fav { "* FAVOURITE" } else { "+ FAVOURITE" }, fav)
                    .tip(if fav { "In your favourites. Click to take it out." } else { "Keep this skin in your favourites." })
                    .clicked()
                {
                    acts.push(Action::WaFav(md5.clone(), name.clone()));
                }
            });
        } else if let Some((_, name)) = &self.wa_worn {
            para(ui, &format!("WEARING: {}", name.to_uppercase()), pal().ink);
        }
        ui.horizontal_wrapped(|ui| {
            let go_w = 54.0;
            // (measured against what shows, so GO and BEST never end up past the panel's edge)
            let seen = (ui.clip_rect().max.x - ui.cursor().min.x).min(ui.available_width());
            let w = (seen - go_w - 70.0 - 3.0 * ui.spacing().item_spacing.x - 12.0).max(120.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = crate::views::field(
                ui,
                &mut self.ed,
                crate::tools::F_WINAMP,
                &mut self.wa_q,
                rect,
                "Search skins (zelda, metal, anime)",
                false,
            );
            if o.enter || retro_btn_w(ui, "GO", go_w, false).clicked() {
                acts.push(Action::WaSearch);
            }
            if retro_btn_w(ui, "BEST", 70.0, false).tip("The museum's favourites.").clicked() {
                self.wa_q.clear();
                acts.push(Action::WaSearch);
            }
        });
        ui.add_space(8.0);
        let list = self.wa_list.clone();
        let worn = self.wa_worn.as_ref().map(|w| w.0.clone());
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // cards: the screenshot (main window, equalizer and playlist) and the name
            let (cw, ch) = (150.0, 150.0 * 348.0 / 275.0);
            let gap = 12.0;
            let seen = (ui.clip_rect().max.x - ui.cursor().min.x).min(ui.available_width());
            let per_row = ((seen + gap) / (cw + gap)).floor().max(1.0) as usize;
            for row in list.chunks(per_row) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for s in row {
                        let (cell, r) = ui.allocate_exact_size(Vec2::new(cw, ch + 22.0), Sense::click());
                        let r = r
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .tip(format!("{}. Click to wear it, right-click to make it a favourite.", s.name));
                        let fav = self.wa_favs.iter().any(|f| f.0 == s.md5);
                        r.context_menu(|ui| {
                            if menu_item(ui, if fav { "Remove from favourites" } else { "Add to favourites" }) {
                                acts.push(Action::WaFav(s.md5.clone(), s.name.clone()));
                                ui.close_menu();
                            }
                        });
                        let shot = Rect::from_min_size(cell.min, Vec2::new(cw, ch));
                        match self.images.get(&s.shot) {
                            Some(id) => {
                                egui::Image::new(egui::load::SizedTexture::new(id, shot.size())).paint_at(ui, shot);
                            }
                            None => {
                                ui.painter().rect_filled(shot, 0.0, pal().lcd);
                                ui.ctx().request_repaint();
                            }
                        }
                        let on = worn.as_deref() == Some(s.md5.as_str());
                        if r.hovered() || on {
                            ui.painter().rect_stroke(
                                shot.expand(2.0),
                                0.0,
                                Stroke::new(2.0_f32, if on { pal().red } else { pal().ink }),
                            );
                        }
                        let label = crate::font::fit(&s.name, 1.0, cw);
                        crate::font::ptext(
                            ui.painter(),
                            Pos2::new(cell.min.x, cell.max.y - 9.0),
                            Align::Min,
                            &label,
                            1.0,
                            pal().ink2,
                        );
                        if r.clicked() {
                            acts.push(Action::WaApply(s.clone()));
                        }
                    }
                });
                ui.add_space(gap);
            }
            if self.wa_busy {
                para(ui, "LOADING SKINS...", pal().ink2);
            } else if list.is_empty() {
                para(ui, "No skins found. Try another word, or BEST.", pal().ink2);
            } else if !self.wa_end && retro_btn(ui, "MORE SKINS", false).clicked() {
                acts.push(Action::WaMore);
            }
        });
    }

    pub(crate) fn cache_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if self.cache_at.elapsed() > Duration::from_secs(1) {
            self.cache_stats = cache::stats();
            self.cache_at = Instant::now();
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            if retro_btn(ui, if self.keep_cache { "SAVING: ON" } else { "SAVING: OFF" }, self.keep_cache).clicked() {
                acts.push(Action::ToggleKeep);
            }
            if retro_btn(ui, "OPEN FOLDER", false).clicked() {
                acts.push(Action::OpenCache);
            }
        });
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            if retro_btn(ui, "CLEAR ALL", false).clicked() {
                acts.push(Action::ClearCache);
            }
            if retro_btn(ui, "CLEAR STEMS", false).tip("Delete all separated stems to free disk space.").clicked() {
                acts.push(Action::StemClear);
            }
            if retro_btn(ui, "CLOSE", false).clicked() {
                acts.push(Action::ToggleCacheView);
            }
        });
        ui.add_space(10.0);
        let w = ui.available_width() - 20.0;
        let (n, bytes) = self.cache_stats;
        para(ui, "TRACKS ARE SAVED IN THIS FOLDER:", pal().ink2);
        for line in chunk(&cache::dir().display().to_string(), 2.0, w) {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
            ptext(ui.painter(), Pos2::new(rect.min.x + 8.0, rect.center().y), Align::Min, &line, 2.0, pal().ink);
        }
        ui.add_space(8.0);
        para(ui, &format!("{} tracks stored, {} used", n, cache::fmt_size(bytes)), pal().ink);
        para(ui, &format!("Separated stems use {}", cache::fmt_size(stems::stems_size())), pal().ink2);
        para(
            ui,
            "A check mark means the track is stored on this computer. It starts instantly and plays with no internet.",
            pal().ink2,
        );
        para(ui, "Looping and slow-down always play from the stored copy. Loops, speed and seeks stay on this PC. Backline never reports plays to Tidal, so your listening statistics are untouched.", pal().ink2);
    }

    /// The BACKLINE menu, next to HOME and LIBRARY: skins (and Winamp skins), preferences, help, storage, the log.
    /// What the BACKLINE menu holds (also opened by a Winamp skin's own options button). True once something
    /// was picked, so the menu can close.
    pub(crate) fn app_menu_items(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) -> bool {
        ui.set_min_width(240.0);
        let before = acts.len();
        let mut done = false;
        // what the player's chips do, for a skin's deck (it has none)
        if self.wa_art.is_some() && !self.mini {
            if menu_item(ui, "Now Playing (the song big)") {
                acts.push(Action::ToggleArt);
            }
            if PRACTICE
                && menu_item(ui, if self.practice { "Just listen (practice tools off)" } else { "Practice (loop, slow down)" })
            {
                acts.push(Action::TogglePractice);
            }
            if PRACTICE && menu_item(ui, if self.focus_mode { "Focus  (on)" } else { "Focus (hide the lists)" }) {
                acts.push(Action::ToggleFocus);
            }
            if menu_item(ui, if self.wa_guide { "Button guide  (on)" } else { "Button guide (name each button)" }) {
                self.wa_guide = !self.wa_guide;
                self.dirty = true;
            }
            ui.add_space(4.0);
        }
        // the window: keep it on top, the mini player and back
        if menu_item(ui, if self.go_offline { "Go online" } else { "Go offline (only what's on this computer)" }) {
            acts.push(Action::GoOffline(!self.go_offline));
        }
        if menu_item(ui, if self.pinned { "Keep on top  (on)" } else { "Keep on top" }) {
            acts.push(Action::TogglePin);
        }
        if menu_item(ui, if self.mini { "Back to the full window" } else { "Mini player" }) {
            acts.push(Action::ToggleMini);
        }
        if menu_item(ui, if self.fullscreen { "Leave full screen  (F11)" } else { "Full screen  (F11)" }) {
            acts.push(Action::ToggleFullscreen);
        }
        if menu_item(ui, "Preferences") {
            acts.push(Action::TogglePrefs);
        }
        if menu_item(ui, "Help and tours  (F1)") {
            acts.push(Action::ToggleHelp);
        }
        if menu_item(ui, if self.show_cache { "Where tracks are stored  (close)" } else { "Where tracks are stored" }) {
            acts.push(Action::ToggleCacheView);
        }
        if menu_item(ui, if self.show_log { "Log  (close)" } else { "Log" }) {
            self.show_log = !self.show_log;
            if self.show_log {
                self.show_eq = false;
                self.show_cache = false;
            }
            done = true;
        }
        // the Winamp skin worn (or the way in to them): opens the skin browser
        let wa_label = match &self.wa_worn {
            Some((md5, name)) if !md5.starts_with("gen:") => format!("> WINAMP: {}", name.to_uppercase()),
            _ => "> WINAMP SKINS".to_string(),
        };
        if menu_item(ui, &wa_label) && !self.show_winamp {
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
            .tip("Now Playing: the song big  (A)");
        let p = ui.painter().clone();
        inset(&p, bar, pal().lcd);
        if resp.hovered() {
            p.rect_filled(bar.shrink(2.0), 0.0, pal().row_sel);
        }
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
            crate::Repeat::One => "Repeat: this song",
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
            let d = if n == 2 { 34.0 } else if n == 0 || n == 4 { 24.0 } else { 28.0 };
            let r = Rect::from_center_size(c, Vec2::splat(d));
            let h = ui.interact(r, id.with(("ctl", n)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
            if n == 2 {
                p.circle_filled(c, d / 2.0 + if h.hovered() { 1.0 } else { 0.0 }, pal().ink);
                let off = if playing { 0.0 } else { 1.5 };
                crate::vicon::named(&p, Rect::from_center_size(c + Vec2::new(off, 0.0), Vec2::splat(d * 0.42)), icon, pal().lcd);
            } else {
                let lit = (n == 0 && self.shuffle) || (n == 4 && rep_on);
                let col = if lit { pal().bar_txt } else if h.hovered() { pal().ink } else { pal().ink2 };
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
            let all = ui.max_rect();
            let sw = (190.0 * font::ui_scale()).clamp(140.0, (all.width() * 0.3).max(140.0));
            let side = Rect::from_min_max(all.min + Vec2::new(0.0, 8.0), Pos2::new(all.min.x + sw, all.max.y - 6.0));
            self.side_nav(ui, side, acts);
            tour::mark("TABS", side);
            tour::mark("APPMENU", side);
            // (a margin all round: nothing touches the window's edges)
            let main = Rect::from_min_max(Pos2::new(side.max.x + 12.0, all.min.y + 8.0), Pos2::new(all.max.x - 4.0, all.max.y));
            let mut body = ui.new_child(egui::UiBuilder::new().max_rect(main).layout(egui::Layout::top_down(Align::Min)));
            self.library_body(&mut body, acts, false);
            ui.allocate_rect(all, Sense::hover());
            return;
        }
        logo(ui);
        ui.add_space(4.0);
        self.library_body(ui, acts, true);
    }

    /// The places, down the side of the library (a skin worn): home, your music by where it is, the practice
    /// tools, and Backline's own pages at the foot.
    fn side_nav(&mut self, ui: &mut egui::Ui, r: Rect, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        let sc = font::ui_scale();
        p.rect_filled(r, 6.0, pal().lcd);
        // the brand at the top
        let gem = Rect::from_min_size(r.min + Vec2::new(12.0, 12.0), Vec2::splat(22.0 * sc));
        crate::vicon::gem(&p, gem, pal().ink, pal().ink2);
        ptext(&p, Pos2::new(gem.max.x + 8.0, gem.center().y), Align::Min, "Backline", 2.2, pal().ink);
        let gem =
            Rect::from_min_max(gem.min, Pos2::new(gem.max.x, gem.max.y + 8.0 + self.window_icons(ui, r, gem.max.y + 8.0, acts)));

        let side = self.show_tl || self.show_winamp || self.show_log || self.show_cache;
        let on_lib = self.page.as_ref().is_some_and(|p| p.library);
        let here = |s: Sec| !side && self.sec == s;
        // (icon, name, lit, what it does); no icon: a group's heading
        let mut rows: Vec<(&str, &str, bool, u8)> = vec![
            ("home", "Home", here(Sec::Tidal) && !on_lib && !self.offline, 1),
            ("", "Music", false, 0),
            ("heart", "My Tracks", here(Sec::Mine), 2),
            ("list", "Lists", here(Sec::Lists), 3),
            ("tidal", "Tidal", here(Sec::Tidal) && (on_lib || self.offline), 4),
            ("folder", "Files", here(Sec::Files), 5),
            ("cloud", "SoundCloud", here(Sec::Sc), 6),
            ("youtube", "YouTube", here(Sec::Yt), 7),
        ];
        if PRACTICE {
            rows.extend([
                ("", "Practice", false, 0),
                ("note", "Tunes", here(Sec::Tunes), 10),
                ("diary", "Diary", here(Sec::Diary), 11),
                ("loop", "Loops", here(Sec::Loops), 12),
                ("book", "Reading", here(Sec::Reading), 13),
                ("chords", "Chords", here(Sec::Chords), 14),
                ("lines", "Lines", here(Sec::Lines), 15),
                ("melody", "Chord Melodies", here(Sec::Melodies), 16),
            ]);
        }
        let foot = [
            ("gear", "Settings", side && !self.show_winamp && !self.show_log, 20u8),
            ("skins", "Winamp Skins", self.show_winamp, 21),
            ("log", "Log", self.show_log, 22),
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
                p.rect_filled(rr, 5.0, pal().beige.lerp_to_gamma(pal().lcd, 0.3));
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
            Stroke::new(1.0_f32, pal().beige_dk),
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
            Some(5) => acts.push(Action::Section(Sec::Files)),
            Some(6) => acts.push(Action::Section(Sec::Sc)),
            Some(7) => acts.push(Action::Section(Sec::Yt)),
            Some(10) => acts.push(Action::Section(Sec::Tunes)),
            Some(11) => acts.push(Action::Section(Sec::Diary)),
            Some(12) => acts.push(Action::Section(Sec::Loops)),
            Some(13) => acts.push(Action::Section(Sec::Reading)),
            Some(14) => acts.push(Action::Section(Sec::Chords)),
            Some(15) => acts.push(Action::Section(Sec::Lines)),
            Some(16) => acts.push(Action::Section(Sec::Melodies)),
            Some(20) => acts.push(Action::BacklinePage),
            Some(21) => acts.push(Action::WaOpen),
            Some(22) => acts.push(Action::ShowLog),
            _ => {}
        }
    }

    /// The window's own switches as a row of icons (a skin worn): online, focus, practice, the mini player, on top, the
    /// skin's guide. Returns the row's height.
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
            items.push(("focus", self.focus_mode, "Focus: just the player, your tools and the chart.".into(), 2));
            items.push((
                "guitar",
                self.practice,
                if self.practice { "Practice mode is on. Click to just listen." } else { "Practice mode: loop and slow down." }
                    .into(),
                3,
            ));
        }
        items.push(("mini", false, "Mini player.".into(), 4));
        items.push(("pin", self.pinned, "Keep this window on top.".into(), 5));
        items.push(("guide", self.wa_guide, "Outline and name each of this skin's buttons.".into(), 6));
        let d = 26.0 * font::ui_scale();
        let mut x = r.min.x + 10.0;
        for (icon, on, tip, key) in items {
            let b = Rect::from_min_size(Pos2::new(x, y), Vec2::splat(d));
            x += d + 3.0;
            if b.max.x > r.max.x - 6.0 {
                break;
            }
            let h = ui
                .interact(b, ui.id().with(("win_icon", key)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .tip(tip);
            if on {
                p.rect_filled(b, 5.0, pal().beige.lerp_to_gamma(pal().lcd, 0.3));
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
                    2 => acts.push(Action::ToggleFocus),
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
        d
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
        let side = self.show_tl || self.show_winamp || self.show_log || self.show_cache;
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
                let in_practice =
                    matches!(self.sec, Sec::Tunes | Sec::Diary | Sec::Loops | Sec::Reading | Sec::Chords | Sec::Lines);
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
            if !self.offline && !side {
                let go_w = 54.0;
                let w = (ui.available_width() - go_w - ui.spacing().item_spacing.x).max(80.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                self.search_input(ui, rect, acts);
                if retro_btn_w(ui, "GO", go_w, false).clicked() {
                    acts.push(Action::Search(self.search.clone()));
                }
            }
        });

        // library tabs
        let is_lib = self.sec == Sec::Tidal && self.page.as_ref().map(|p| p.library).unwrap_or(false);
        if is_lib && !self.show_log && !self.show_cache && !self.show_winamp && !self.show_tl {
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
            inset(ui.painter(), rect, pal().lcd);
            marquee(ui, rect, &self.status, 2.0, pal().red);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                acts.push(Action::ClearStatus);
            }
        }
        // (not on the BACKLINE pages: settings, skins, the log)
        if !(self.show_tl || self.show_winamp || self.show_log || self.show_cache) {
            self.continue_bar(ui, acts);
        }
        ui.add_space(4.0);

        // list well: it stops a little above the window's bottom edge, so its own border shows and the last row
        // ends inside it instead of running into the frame
        let avail = ui.available_rect_before_wrap();
        let mut avail = Rect::from_min_max(avail.min, Pos2::new(avail.max.x, avail.max.y - 6.0));
        // with a skin and a song: the now-playing bar along the foot (click it for NOW PLAYING)
        if self.wa_art.is_some() && !self.mini && avail.height() > 200.0 {
            if let Some(tr) = self.cur_track() {
                let bar = Rect::from_min_max(Pos2::new(avail.min.x, avail.max.y - 52.0), avail.max);
                avail.max.y = bar.min.y - 6.0;
                self.now_bar(ui, bar, &tr, acts);
            }
        }
        if avail.height() < 30.0 {
            return;
        }
        inset(ui.painter(), avail, pal().lcd);
        let area = if crate::winamp_ui::chrome_on() { avail.shrink2(Vec2::new(12.0, 8.0)) } else { avail.shrink(thick(3.0)) };
        let page = self.page.clone();
        let serial = self.serial;
        let loading = self.loading;
        let tab = self.lib_tab;
        let playing_id = self.cur_track().map(|t| t.id);
        let (show_log, show_cache, show_winamp) = (self.show_log, self.show_cache, self.show_winamp);
        let sec = self.sec;
        let sec_i = sec as u8;
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(area), |ui| {
            // (a skin worn: room between the things on a page; the song tables stay flush, list_row sees to that)
            ui.spacing_mut().item_spacing = Vec2::new(8.0, if crate::winamp_ui::chrome_on() { GAP } else { 0.0 });
            if show_log {
                self.log_view(ui, acts);
                return;
            }
            if show_cache {
                self.cache_view(ui, acts);
                return;
            }
            if show_winamp {
                self.winamp_view(ui, acts);
                return;
            }
            if self.show_tl {
                self.backline_page(ui, acts);
                return;
            }
            if sec == Sec::Melodies {
                ui.push_id((serial, sec_i), |ui| self.melodies_view(ui, acts));
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
                            para(ui, "You are using Backline without Tidal. Your files, YouTube, SoundCloud and every practice tool still work. Log in to search and play Tidal's catalogue, with your liked songs, playlists and artists.", pal().ink2);
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
                                                .tip("Makes it on your Tidal account.")
                                                .clicked()
                                        {
                                            acts.push(Action::TidalListNew(self.tidal_new_name.clone()));
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
                        Sec::Melodies => {}
                        Sec::Lists => self.playlists_view(ui, acts),
                        Sec::Mine => self.mine_view(ui, acts),
                    });
                });
        });
    }
}
