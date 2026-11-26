//! Now Playing: the cover big (or the visuals filling the window), the song under it, a side panel (Lyrics, Up
//! Next, About, Loops), the controls at the foot fading when the mouse is still; a practice screen while practising;
//! each style's own backdrop (Hardware's is an amp, in hw_amp.rs).

use super::*;

/// What Now Playing can show of the song (`App.cap_show`).
const CAP_SHOW: [&str; 3] = ["ART AND INFO", "INFO ONLY", "NOTHING"];

impl App {
    /// Now Playing (the owner's picks of 2026-11-25): the cover big (or the visuals filling the window, the song in a
    /// card at the bottom left), the song under it with no plate, a side panel with tabs (Lyrics, Up Next, About,
    /// Loops) whose lyrics have no box and fade at its edges, switches for them and the visuals at the top right, the
    /// main controls at the foot and everything else in a "..." menu; the controls fade when the mouse is still and
    /// come back when it moves. Practising, it's a practice screen (the waveform big). Each style dresses it: the
    /// blurred cover behind (Classic, Ambient on glass), the album's colour and huge bold lyrics (Streaming), a lit
    /// analyzer and VU meters (Hardware).
    pub(crate) fn art_ui(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        use crate::skin::{style, STYLE_STREAMING};
        let whole = ui.max_rect();
        let own_bar = crate::app_bar() && self.art_view && !self.mini && !self.fullscreen;
        let full = Rect::from_min_max(whole.min + Vec2::new(0.0, if own_bar { crate::NP_BAR_H - 8.0 } else { 0.0 }), whole.max);
        let p = ui.painter().clone();
        let dt = ui.input(|i| i.stable_dt).min(0.05);
        let now = ui.input(|i| i.time);
        let hover = ui.input(|i| i.pointer.hover_pos());
        let track = self.cur_track();
        let pos = self.pos();
        let dur = track.as_ref().map(|t| t.duration).unwrap_or(0.0);
        let active = self.cur.is_some() && !self.stopped;
        let st = style();
        let prac = practice_tools() && self.practice;
        // (the old queue button: the panel's Up Next tab now)
        if self.art_queue {
            self.art_queue = false;
            self.show_lyrics = true;
            self.np_tab = 1;
        }

        // ---- the controls show while the mouse moves and a few seconds after (always while practising)
        if ui.input(|i| i.pointer.delta() != Vec2::ZERO || i.pointer.any_down() || i.raw_scroll_delta != Vec2::ZERO) {
            self.np_move_at = now;
        }
        let idle = (now - self.np_move_at) as f32;
        let menu_open = ui.ctx().memory(|m| m.any_popup_open());
        // (a test copy with BACKLINE_NP_UI keeps them showing, for pictures)
        let pinned = std::env::var_os("BACKLINE_NP_UI").is_some();
        let show = if prac || menu_open || pinned { 1.0 } else { (1.0 - (idle - 3.0) / 0.6).clamp(0.0, 1.0) };
        if idle < 3.8 {
            ui.ctx().request_repaint();
        }

        // ---- layout
        let bar_h = 96.0;
        let area = Rect::from_min_max(full.min + Vec2::new(28.0, 58.0), Pos2::new(full.max.x - 28.0, full.max.y - bar_h - 12.0));
        // (Hardware's amp has its own Up Next and lyrics)
        let panel_on = self.show_lyrics && area.width() > 640.0 && !prac && !crate::hardware();
        // Streaming's lyrics take the whole page (a small cover and the song at the top left)
        let st_lyrics = st == STYLE_STREAMING && panel_on && self.np_tab == 0;
        let art_zone = if panel_on && !st_lyrics {
            Rect::from_min_max(area.min, Pos2::new(area.min.x + area.width() * 0.5 - 10.0, area.max.y))
        } else {
            area
        };
        let panel = if st_lyrics { area } else { Rect::from_min_max(Pos2::new(area.min.x + area.width() * 0.5 + 10.0, area.min.y), area.max) };
        let cap_k = [0.8, 1.0, 1.35][self.cap_size.min(2) as usize];
        let caption_h = if st == STYLE_STREAMING { 92.0 } else { 70.0 } * cap_k;
        let size_k = if st == STYLE_STREAMING { 0.66 } else { 0.82 };
        let a = (art_zone.width().min(art_zone.height() - caption_h) * size_k).max(120.0);
        let center = Pos2::new(art_zone.center().x, art_zone.min.y + (art_zone.height() - caption_h) / 2.0);

        // ---- the backdrop
        let url = track.as_ref().map(|t| cover_url(&t.cover, 640)).unwrap_or_default();
        let (tex, prev, fade) = self.images.held("album", &url, false, now);
        if fade < 1.0 {
            ui.ctx().request_repaint();
        }
        self.np_backdrop(&p, whole, dt, &url, now);

        // ---- visuals filling the window, under everything
        // (Hardware shows the visuals inside its screen, not over the whole window)
        let vis = self.vis_in_np() && !prac && !crate::hardware();
        if vis {
            self.vis_paint(ui, whole, 1);
            let name = self.vis_name();
            if self.vis_seen != name {
                self.vis_seen = name.clone();
                self.np_move_at = now;
            }
            if show > 0.0 {
                let label = format!("VISUALS: {}", name);
                let w = crate::text_w(&label, 1.2) + 32.0;
                let bar = Rect::from_min_size(full.min + Vec2::new(24.0, 16.0), Vec2::new(w, 28.0));
                p.rect_filled(bar, 6.0, Color32::from_black_alpha((150.0 * show) as u8));
                crate::ptext(&p, bar.center(), Align::Center, &label, 1.2, Color32::from_white_alpha((235.0 * show) as u8));
            }
        }

        // ---- the soft visualizer behind the cover (not with the visuals, not practising)
        if self.show_spec && !vis && !prac && !crate::hardware() {
            let zone = if panel_on && self.spec_lyr == 0 { Rect::from_min_max(full.min, Pos2::new(art_zone.max.x + 14.0, full.max.y)) } else { full };
            let half = (zone.width() * self.spec_w / 2.0).max(40.0);
            let mid = zone.center().x;
            let r = Rect::from_min_max(
                Pos2::new((mid - half).max(zone.min.x + 6.0), zone.max.y - zone.height() * self.spec_h),
                Pos2::new((mid + half).min(zone.max.x - 6.0), zone.max.y),
            );
            let (vb, vp, vw) = self.viz_data(1);
            let soft = self.spec_frame == 1;
            let r = if soft { Rect::from_min_max(r.min, r.max - Vec2::new(0.0, 12.0)) } else { r };
            crate::viz::set_edge_fade(soft);
            viz_draw(&p, r, &vb, &vp, &vw, self.viz[1].mode, self.viz[1].w, true, self.spec_op, self.viz_tint(self.viz[1].color));
            crate::viz::set_edge_fade(false);
            if self.cur.is_some() && !self.paused && !self.stopped && crate::onscreen::seen() {
                ui.ctx().request_repaint();
            }
        }

        // ---- what's in the middle: Hardware, its amp (holding the practice tools while practising); practising, the
        // practice screen; else the cover
        if crate::studio::on() {
            self.np_studio(ui, area, dt, acts);
        } else if crate::hardware() && !vis {
            self.np_amp(ui, area, tex, dt, acts);
        } else if prac {
            self.np_practice(ui, Rect::from_min_max(full.min + Vec2::new(28.0, 20.0), Pos2::new(full.max.x - 28.0, full.max.y - bar_h - 8.0)), tex, acts);
        } else if !vis && !st_lyrics {
            self.np_cover(ui, &p, center, a, hover, dt, tex, prev, fade, &url, active, acts);
            // the song under the cover: no plate, a soft spot under the words so they read over anything
            if let Some(tr) = track.as_ref().filter(|_| self.cap_show < 2) {
                let cx = art_zone.center().x;
                let y0 = art_zone.max.y - caption_h + 16.0 * cap_k;
                let w = art_zone.width() - 40.0;
                let sub = if tr.album.is_empty() { tr.artists_text() } else { format!("{} · {}", tr.artists_text(), tr.album) };
                let (tp, sp) = if st == STYLE_STREAMING { (4.4 * cap_k, 2.2 * cap_k) } else { (3.0 * cap_k, 2.0 * cap_k) };
                let ink = if self.cap_dark { Color32::from_rgb(24, 24, 24) } else { OVER };
                let tw = text_w(&tr.title, tp).min(w);
                if !self.cap_dark {
                    soft_spot(&p, Rect::from_center_size(Pos2::new(cx, y0 + 12.0 * cap_k), Vec2::new(tw + 10.0, 30.0 * cap_k)), 22.0, 0.22);
                }
                ptext_fit(&p, Pos2::new(cx, y0), Align::Center, &tr.title, tp, w, ink);
                if st == STYLE_STREAMING {
                    // (bold, as a streaming app's title)
                    ptext_fit(&p, Pos2::new(cx + 0.8, y0), Align::Center, &tr.title, tp, w, ink);
                }
                let sy = y0 + if st == STYLE_STREAMING { 40.0 } else { 30.0 } * cap_k;
                ptext_fit(&p, Pos2::new(cx, sy), Align::Center, &sub, sp, w, ink.gamma_multiply(0.75));
                let cap_r = Rect::from_center_size(Pos2::new(cx, (y0 + sy) / 2.0), Vec2::new(tw + 30.0, sy - y0 + 40.0));
                self.np_caption_menu(ui, cap_r, now);
            } else if track.is_none() {
                let t = now as f32;
                let breath = ((t * std::f32::consts::TAU / 5.0).sin() + 1.0) / 2.0;
                let level = (breath * 4.0).round() / 4.0;
                ptext(&p, art_zone.center(), Align::Center, "NO TRACK PLAYING", 3.0, OVER.gamma_multiply(0.2 + 0.8 * level));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
            }
        }
        // the visuals on: the song in a card at the bottom left, while the mouse moves (or a new song starts)
        if vis {
            if let Some(tr) = &track {
                if self.cap_song != tr.id {
                    self.cap_song = tr.id;
                    self.np_move_at = now;
                }
                if show > 0.0 {
                    let card = Rect::from_min_size(Pos2::new(full.min.x + 28.0, full.max.y - bar_h - 118.0), Vec2::new(360.0_f32.min(full.width() * 0.45), 96.0));
                    p.rect_filled(card, 12.0, Color32::from_black_alpha((150.0 * show) as u8));
                    let cs = card.height() - 20.0;
                    let cr = Rect::from_min_size(card.min + Vec2::splat(10.0), Vec2::splat(cs));
                    if let Some(id) = tex {
                        egui::Image::new(egui::load::SizedTexture::new(id, cr.size()))
                            .rounding(Rounding::same(6.0))
                            .tint(Color32::from_white_alpha((255.0 * show) as u8))
                            .paint_at(ui, cr);
                    }
                    let tx = cr.max.x + 14.0;
                    let room = card.max.x - tx - 10.0;
                    let a8 = |c: Color32| c.gamma_multiply(show);
                    ptext_fit(&p, Pos2::new(tx, card.center().y - 11.0), Align::Min, &tr.title, 2.2, room, a8(OVER));
                    ptext_fit(&p, Pos2::new(tx, card.center().y + 13.0), Align::Min, &tr.artists_text(), 1.6, room, a8(OVER.gamma_multiply(0.7)));
                }
            }
        }

        // ---- the side panel: Lyrics, Up Next, About, Loops
        if panel_on {
            self.np_panel(ui, &p, panel, st_lyrics, hover, dt, pos, tex, acts);
        }

        // ---- the switches at the top right: Lyrics, Up Next, Visuals
        if show > 0.0 && !prac && !crate::hardware() {
            let items = [("Visuals", self.vis_np), ("Up Next", panel_on && self.np_tab == 1), ("Lyrics", panel_on && self.np_tab == 0)];
            let mut x = full.max.x - 28.0;
            let y = full.min.y + 14.0;
            for (n, (name, on)) in items.into_iter().enumerate() {
                let w = text_w(name, 1.5) + 24.0;
                let r = Rect::from_min_size(Pos2::new(x - w, y), Vec2::new(w, 28.0));
                x = r.min.x - 6.0;
                let h = ui.interact(r, ui.id().with(("np_switch", n)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                let fill = if on { 0.22 } else if h.hovered() { 0.14 } else { 0.08 };
                // (the foot's soft shade under the word, so it reads over a pale cover or visuals)
                soft_spot(&p, r.shrink2(Vec2::new(10.0, 9.0)), 10.0, 0.24 * show);
                p.rect_filled(r, 6.0, Color32::from_rgba_premultiplied((255.0 * fill * show) as u8, (255.0 * fill * show) as u8, (255.0 * fill * show) as u8, (255.0 * fill * show) as u8));
                ptext(&p, r.center(), Align::Center, name, 1.5, OVER.gamma_multiply(if on || h.hovered() { show } else { 0.75 * show }));
                if h.clicked() {
                    match n {
                        0 => {
                            self.vis_np = !self.vis_np;
                            self.dirty = true;
                        }
                        k => {
                            let tab = if k == 1 { 1 } else { 0 };
                            if panel_on && self.np_tab == tab {
                                self.show_lyrics = false;
                            } else {
                                self.show_lyrics = true;
                                self.np_tab = tab;
                                self.q_jump |= tab == 1;
                            }
                            self.dirty = true;
                        }
                    }
                }
                if n == 0 {
                    h.context_menu(|ui| self.vis_menu(ui));
                }
            }
        }

        // ---- the foot: how far in, the main controls, the volume, and "..." for everything else
        if show > 0.0 {
            let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(whole));
            cu.set_opacity(show);
            self.np_foot(&mut cu, full, bar_h, active, pos, dur, acts);
        }
        if own_bar {
            crate::window_bar(ui, Rect::from_min_size(whole.min, Vec2::new(whole.width(), crate::NP_BAR_H)), 1.0, 70);
        }
    }

    /// Now Playing's backdrop for the style worn: Streaming the album's colour, Hardware its body, the others the cover
    /// itself blown up and blurred (the cover's average colour until that's ready), darkened, with a soft vignette.
    fn np_backdrop(&mut self, p: &egui::Painter, whole: Rect, dt: f32, url: &str, now: f64) {
        use crate::skin::{style, STYLE_HARDWARE, STYLE_STREAMING};
        match style() {
            STYLE_STREAMING => {
                let t = self.amb_shown(now);
                let c = t[0].lerp_to_gamma(t[1], 0.5).lerp_to_gamma(t[2], 0.33);
                let top = c.lerp_to_gamma(Color32::BLACK, 0.25);
                let bot = c.lerp_to_gamma(Color32::BLACK, 0.72);
                let mut m = egui::Mesh::default();
                for (pt, col) in [(whole.left_top(), top), (whole.right_top(), top), (whole.right_bottom(), bot), (whole.left_bottom(), bot)] {
                    m.colored_vertex(pt, col);
                }
                m.add_triangle(0, 1, 2);
                m.add_triangle(0, 2, 3);
                p.add(m);
                return;
            }
            STYLE_HARDWARE => {
                crate::hw_amp::room(p, whole);
                return;
            }
            crate::skin::STYLE_STUDIO => {
                p.rect_filled(whole, 0.0, crate::studio::BODY);
                return;
            }
            _ => {}
        }
        let blur = self.np_blur.as_ref().filter(|_| !self.np_blur_for.is_empty() && self.np_blur_for.contains(url.split('/').next_back().unwrap_or(""))).map(|t| t.id());
        if let Some(id) = blur {
            // the cover blown up to fill the window (its middle, kept square), darkened
            let (w, h) = (whole.width(), whole.height());
            let (u0, u1, v0, v1) = if w > h { (0.0, 1.0, 0.5 - h / w / 2.0, 0.5 + h / w / 2.0) } else { (0.5 - w / h / 2.0, 0.5 + w / h / 2.0, 0.0, 1.0) };
            let mut m = egui::Mesh::with_texture(id);
            // (bright enough that the cover's colours show, dark enough that white reads over them)
            let tint = Color32::from_gray(if crate::skin::style() == crate::skin::STYLE_AMBIENT { 205 } else { 185 });
            for (pt, uv) in [
                (whole.left_top(), Pos2::new(u0, v0)),
                (whole.right_top(), Pos2::new(u1, v0)),
                (whole.right_bottom(), Pos2::new(u1, v1)),
                (whole.left_bottom(), Pos2::new(u0, v1)),
            ] {
                m.vertices.push(egui::epaint::Vertex { pos: pt, uv, color: tint });
            }
            m.add_triangle(0, 1, 2);
            m.add_triangle(0, 2, 3);
            p.add(m);
        } else {
            let avg = self.images.avg.get(url).copied();
            if let Some(tg) = avg {
                let k = 1.0 - (-dt * 4.0).exp();
                let mix = |a: Color32, b: Color32| {
                    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
                    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
                };
                let base = pal().app_bg;
                let c = self.bg_now.unwrap_or([base, base]);
                self.bg_now = Some([mix(c[0], tg[0]), mix(c[1], tg[1])]);
            }
            let [top, bot] = self.bg_now.unwrap_or([pal().app_bg, pal().app_bg]);
            let deep = |c: Color32, cap: f32| {
                let mut h = egui::ecolor::Hsva::from(c);
                h.s = (h.s * 1.1).min(0.75);
                h.v = (h.v * 0.38).min(cap);
                Color32::from(h)
            };
            let mut m = egui::Mesh::default();
            for (pt, col) in [(whole.left_top(), deep(top, 0.19)), (whole.right_top(), deep(top, 0.14)), (whole.right_bottom(), deep(bot, 0.04)), (whole.left_bottom(), deep(bot, 0.06))] {
                m.colored_vertex(pt, col);
            }
            m.add_triangle(0, 1, 2);
            m.add_triangle(0, 2, 3);
            p.add(m);
        }
        // a vignette: clear round the middle, easing into near-black at the corners
        let edge = Color32::from_rgb(4, 4, 6);
        let c = whole.center();
        let (rx, ry) = (whole.width() * 0.5 * 1.42, whole.height() * 0.5 * 1.42);
        const SEG: u32 = 64;
        const RINGS: u32 = 24;
        let uv = egui::epaint::WHITE_UV;
        let mut v = egui::Mesh::default();
        for ring in 0..=RINGS {
            let f = ring as f32 / RINGS as f32;
            let s = ((f - 0.42) / 0.58).clamp(0.0, 1.0);
            let al = s * s * (3.0 - 2.0 * s) * 0.85;
            let col = Color32::from_rgba_unmultiplied(edge.r(), edge.g(), edge.b(), (al * 255.0) as u8);
            for k in 0..SEG {
                let ang = k as f32 / SEG as f32 * std::f32::consts::TAU;
                v.vertices.push(egui::epaint::Vertex { pos: Pos2::new(c.x + ang.cos() * rx * f, c.y + ang.sin() * ry * f), uv, color: col });
            }
        }
        for ring in 0..RINGS {
            for k in 0..SEG {
                let (a0, a1) = (ring * SEG + k, ring * SEG + (k + 1) % SEG);
                let (b0, b1) = (a0 + SEG, a1 + SEG);
                v.indices.extend_from_slice(&[a0, a1, b1, a0, b1, b0]);
            }
        }
        p.add(egui::Shape::mesh(v));
    }

    /// The cover, big, tilting a little under the mouse; right-click for its border (and, on a dev build only, the
    /// experimental record).
    #[allow(clippy::too_many_arguments)]
    fn np_cover(
        &mut self,
        ui: &mut egui::Ui,
        p: &egui::Painter,
        center: Pos2,
        a: f32,
        hover: Option<Pos2>,
        dt: f32,
        tex: Option<egui::TextureId>,
        prev: Option<egui::TextureId>,
        fade: f32,
        url: &str,
        active: bool,
        acts: &mut Vec<Action>,
    ) {
        let reach = a / 2.0;
        let calm = crate::skin::calm();
        let target = match hover.filter(|_| !calm) {
            Some(pp) if (pp.x - center.x).abs() < reach && (pp.y - center.y).abs() < reach => {
                Vec2::new(((pp.x - center.x) / reach).clamp(-1.0, 1.0), ((pp.y - center.y) / reach).clamp(-1.0, 1.0)) * 0.35
            }
            _ => Vec2::ZERO,
        };
        let k = 1.0 - (-dt * 12.0).exp();
        self.art_tilt += (target - self.art_tilt) * k;
        let tilt = self.art_tilt;
        let (ky, kx) = (-tilt.x * 0.28, -tilt.y * 0.28);
        let (cy, cx) = (ky.cos(), kx.cos());
        let dist = a * 4.0;
        let push = Vec2::new(-tilt.x, -tilt.y) * 6.0;
        let xf = move |x: f32, y: f32| -> Pos2 {
            let z = x * ky.sin() + y * kx.sin();
            let sc = dist / (dist - z);
            Pos2::new(center.x + push.x + x * cy * sc, center.y + push.y + y * cx * sc)
        };
        let corners = |h: f32| [xf(-h, -h), xf(h, -h), xf(h, h), xf(-h, h)];
        let h = a / 2.0;
        // (the record is an experiment, only on a dev build: the owner never used it)
        let dev = crate::update::alpha() > 0;
        let hit = ui.interact(Rect::from_center_size(center, Vec2::splat(a)), ui.id().with("np_cover"), Sense::click());
        hit.context_menu(|ui| {
            for (i, name) in ["NONE", "SUBTLE", "SKIN COLOR"].iter().enumerate() {
                let mark = if self.art_border as usize == i { "> " } else { "   " };
                if menu_item(ui, &format!("{}BORDER: {}", mark, name)) {
                    self.art_border = i as u8;
                    self.dirty = true;
                    ui.close_menu();
                }
            }
            if dev {
                let on = VINYL.load(Ordering::Relaxed);
                if menu_item(ui, &format!("{}Record (Experimental, Dev Build)", if on { "> " } else { "   " })) {
                    acts.push(Action::ToggleVinyl);
                    ui.close_menu();
                }
            }
        });
        if dev && VINYL.load(Ordering::Relaxed) && tex.is_some() {
            let off = Vec2::new(-tilt.x * 22.0, -tilt.y * 12.0 + 16.0);
            p.circle_filled(center + off, a * 0.5 + 4.0, Color32::from_black_alpha((60.0 * self.art_op) as u8));
            let tempo = if self.bpm > 0 { self.bpm as f32 / 133.3 } else { 1.0 };
            let spin = vinyl_angle(ui, if active && !self.paused { self.speed as f32 / 100.0 * tempo } else { 0.0 });
            paint_vinyl(p, Rect::from_center_size(center, Vec2::splat(a)), tex, spin, self.art_op.min(0.8));
            return;
        }
        let off = Vec2::new(-tilt.x * 14.0, -tilt.y * 8.0 + 14.0);
        let see = self.art_op < 0.99;
        for (grow, al) in [(16.0, 14u8), (10.0, 22), (5.0, 34)] {
            let sh: Vec<Pos2> = corners(h + grow).iter().map(|c| *c + off).collect();
            p.add(egui::Shape::convex_polygon(sh, Color32::from_black_alpha(if see { al / 2 } else { al }), Stroke::NONE));
        }
        let cs = corners(h);
        match tex {
            Some(id) => {
                if let Some(old) = prev {
                    cover_grid(p, &xf, h, old, self.art_op);
                }
                if prev.is_none() && fade < 1.0 {
                    p.add(egui::Shape::convex_polygon(cs.to_vec(), Color32::from_black_alpha((70.0 * (1.0 - fade)) as u8), Stroke::NONE));
                }
                cover_grid(p, &xf, h, id, self.art_op * fade);
                let edge = match self.art_border {
                    1 => Some(Stroke::new(1.0_f32, Color32::from_white_alpha(30))),
                    2 => Some(Stroke::new(2.0_f32, pal().accent.gamma_multiply(self.art_op))),
                    _ => None,
                };
                if let Some(s) = edge {
                    p.add(egui::Shape::closed_line(cs.to_vec(), s));
                }
            }
            None => {
                p.add(egui::Shape::convex_polygon(cs.to_vec(), Color32::from_black_alpha(70), Stroke::NONE));
                let loading = !url.is_empty() && !self.images.failed(url);
                let now = ui.input(|i| i.time);
                let since_id = egui::Id::new(("cover_wait", url));
                let since: f64 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(since_id, || now));
                let slow = now - since > 1.5;
                if !loading || slow {
                    let spin = (loading && slow).then_some(now as f32);
                    paint_record(p, Rect::from_center_size(center, Vec2::splat(a)), spin);
                }
                if loading {
                    ui.ctx().request_repaint();
                }
            }
        }
    }

    /// The song's words under the cover: right-click for their size, colour and what's shown.
    fn np_caption_menu(&mut self, ui: &mut egui::Ui, r: Rect, now: f64) {
        ui.interact(r, ui.id().with("caption"), Sense::click()).context_menu(|ui| {
            for (i, name) in ["SMALL", "MEDIUM", "LARGE"].iter().enumerate() {
                let mark = if self.cap_size as usize == i { "> " } else { "   " };
                if menu_item(ui, &format!("{}TEXT: {}", mark, name)) {
                    self.cap_size = i as u8;
                    self.dirty = true;
                    ui.close_menu();
                }
            }
            for (dark, name) in [(false, "SOFT WHITE"), (true, "SOFT BLACK")] {
                let mark = if self.cap_dark == dark { "> " } else { "   " };
                if menu_item(ui, &format!("{}COLOR: {}", mark, name)) {
                    self.cap_dark = dark;
                    self.dirty = true;
                    ui.close_menu();
                }
            }
            for (i, name) in CAP_SHOW.iter().enumerate() {
                let mark = if self.cap_show as usize == i { "> " } else { "   " };
                if menu_item(ui, &format!("{}SHOW: {}", mark, name)) {
                    self.cap_show = i as u8;
                    self.cap_mode_at = now;
                    self.dirty = true;
                    ui.close_menu();
                }
            }
        });
    }

    /// The side panel: its tabs (Lyrics, Up Next, About, Loops) along its top, then the one picked, with no box (on
    /// Ambient, a pane of glass). Streaming's lyrics fill the page under a small cover and the song.
    #[allow(clippy::too_many_arguments)]
    fn np_panel(
        &mut self,
        ui: &mut egui::Ui,
        p: &egui::Painter,
        r: Rect,
        st_lyrics: bool,
        hover: Option<Pos2>,
        dt: f32,
        pos: f32,
        tex: Option<egui::TextureId>,
        acts: &mut Vec<Action>,
    ) {
        use crate::skin::{style, STYLE_AMBIENT};
        if style() == STYLE_AMBIENT {
            p.rect_filled(r.expand(10.0), 16.0, Color32::from_rgba_premultiplied(26, 26, 26, 26));
        }
        // the tabs
        let tabs = ["Lyrics", "Up Next", "About", "Loops"];
        let mut x = if st_lyrics { r.max.x - tabs.iter().map(|t| text_w(t, 1.7) + 22.0).sum::<f32>() } else { r.min.x + 4.0 };
        let ty = r.min.y + 10.0;
        for (i, t) in tabs.iter().enumerate() {
            let w = text_w(t, 1.7) + 22.0;
            let tr = Rect::from_min_size(Pos2::new(x, ty - 12.0), Vec2::new(w, 24.0));
            x += w;
            let h = ui.interact(tr, ui.id().with(("np_tab", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let on = self.np_tab as usize == i;
            ptext(p, Pos2::new(tr.min.x + 4.0, ty), Align::Min, t, 1.7, OVER.gamma_multiply(if on || h.hovered() { 1.0 } else { 0.55 }));
            if on {
                p.rect_filled(Rect::from_min_size(Pos2::new(tr.min.x + 4.0, ty + 11.0), Vec2::new(text_w(t, 1.7), 2.0)), 1.0, OVER);
            }
            if h.clicked() {
                // (Up Next, opened or clicked again: the list goes to the song playing)
                self.q_jump |= i == 1;
                self.np_tab = i as u8;
                self.dirty = true;
            }
        }
        let body = Rect::from_min_max(Pos2::new(r.min.x, r.min.y + 34.0), r.max);
        match self.np_tab {
            0 => {
                let lr = if st_lyrics {
                    // Streaming: a small cover and the song at the top left, the lyrics huge under them
                    let head = Rect::from_min_size(r.min, Vec2::new(r.width() * 0.6, 64.0));
                    let cs = Rect::from_min_size(head.min, Vec2::splat(64.0));
                    if let Some(id) = tex {
                        egui::Image::new(egui::load::SizedTexture::new(id, cs.size())).rounding(Rounding::same(6.0)).paint_at(ui, cs);
                    }
                    if let Some(tr) = self.cur_track() {
                        ptext_fit(p, Pos2::new(cs.max.x + 14.0, cs.center().y - 10.0), Align::Min, &tr.title, 2.4, head.width() - 80.0, OVER);
                        ptext_fit(p, Pos2::new(cs.max.x + 14.6, cs.center().y - 10.0), Align::Min, &tr.title, 2.4, head.width() - 80.0, OVER);
                        ptext_fit(p, Pos2::new(cs.max.x + 14.0, cs.center().y + 14.0), Align::Min, &tr.artists_text(), 1.7, head.width() - 80.0, OVER.gamma_multiply(0.7));
                    }
                    Rect::from_min_max(Pos2::new(r.min.x, r.min.y + 84.0), r.max)
                } else {
                    body
                };
                self.np_lyrics(ui, p, lr, st_lyrics, hover, dt, pos, acts);
            }
            1 => self.style_queue(ui, body, acts),
            2 => self.np_about(ui, p, body, OVER),
            _ => self.np_loops(ui, p, body, acts, OVER),
        }
    }

    /// The lyrics with no box: the line being sung big and bright, the rest dim, fading out towards the panel's top and
    /// foot; the wheel scrolls them a while, a click on a line (synced lyrics) jumps there. Streaming's huge and bold,
    /// from the left.
    #[allow(clippy::too_many_arguments)]
    fn np_lyrics(&mut self, ui: &mut egui::Ui, p: &egui::Painter, ly: Rect, big: bool, hover: Option<Pos2>, dt: f32, pos: f32, acts: &mut Vec<Action>) {
        let cp = p.with_clip_rect(ly);
        let lh = if big { 62.0 } else { 44.0 };
        let cyl = ly.min.y + ly.height() * if big { 0.35 } else { 0.45 };
        let wheel = ui.input(|i| i.raw_scroll_delta.y);
        let over = hover.map(|pp| ly.contains(pp)).unwrap_or(false);
        let mut scroll = self.lyric_scroll;
        let track_id = self.cur_track().map(|t| t.id);
        let (align, x) = if big { (Align::Min, ly.min.x + 4.0) } else { (Align::Center, ly.center().x) };
        let mut msg: Option<&str> = None;
        match &self.lyrics {
            None => msg = Some("Lyrics show here"),
            Some(l) => {
                if track_id != Some(l.id) || l.status == 0 {
                    msg = Some("Loading lyrics...");
                } else if l.status == 2 {
                    msg = Some("No lyrics for this track.");
                } else {
                    let n = l.lines.len();
                    let idx: Option<usize> = if l.synced { l.lines.iter().rposition(|(tt, _)| *tt <= pos + 0.25) } else { None };
                    self.lyric_free = (self.lyric_free - dt).max(0.0);
                    if over && wheel.abs() > 0.0 {
                        scroll = (scroll - wheel / lh).within(0.0, (n.max(1) - 1) as f32);
                        self.lyric_free = 4.0;
                    } else if l.synced && self.lyric_free <= 0.0 {
                        let tgt = idx.unwrap_or(0) as f32;
                        scroll += (tgt - scroll) * (1.0 - (-dt * 7.0).exp());
                    }
                    let mut hover_i: Option<usize> = None;
                    if l.synced && over {
                        if let Some(pp) = hover {
                            let k = (scroll + (pp.y - cyl) / lh).round();
                            if k >= 0.0 && (k as usize) < n && !l.lines[k as usize].1.is_empty() {
                                hover_i = Some(k as usize);
                            }
                        }
                    }
                    let lr = ui.interact(ly, ui.id().with("lyric_click"), Sense::click());
                    if hover_i.is_some() {
                        lr.clone().on_hover_cursor(egui::CursorIcon::PointingHand);
                    }
                    if lr.clicked() {
                        if let Some(i) = hover_i {
                            acts.push(Action::Seek(l.lines[i].0));
                            self.lyric_free = 0.0;
                        }
                    }
                    let edge = ly.height() * 0.22;
                    for (i, (_, txt)) in l.lines.iter().enumerate() {
                        if txt.is_empty() {
                            continue;
                        }
                        let d = i as f32 - scroll;
                        let y = cyl + d * lh;
                        if y < ly.min.y - lh || y > ly.max.y + lh {
                            continue;
                        }
                        // (fading out towards the panel's top and foot: no hard edge)
                        let to_edge = (y - ly.min.y).min(ly.max.y - y);
                        let ef = (to_edge / edge).clamp(0.0, 1.0);
                        let ef = ef * ef * (3.0 - 2.0 * ef);
                        let now_line = idx == Some(i);
                        let (px, a) = if now_line {
                            (if big { 4.6 } else { 3.3 }, 1.0)
                        } else if hover_i == Some(i) {
                            (if big { 3.8 } else { 2.5 }, 0.85)
                        } else {
                            (if big { 3.8 } else { 2.5 }, if big { 0.42 } else { 0.38 })
                        };
                        let col = OVER.gamma_multiply(a * ef);
                        let w = ly.width() - 20.0;
                        ptext_fit(&cp, Pos2::new(x, y), align, txt, px, w, col);
                        if big || now_line {
                            // (a little weight: bold for Streaming's, for the line being sung)
                            ptext_fit(&cp, Pos2::new(x + 0.7, y), align, txt, px, w, col);
                        }
                    }
                }
            }
        }
        if let Some(m) = msg {
            ptext_fit(&cp, Pos2::new(x, cyl), align, m, 2.0, ly.width() - 30.0, OVER.gamma_multiply(0.6));
        }
        self.lyric_scroll = scroll;
    }

    /// About the song: its name, who plays, the album, how long, and what Backline knows of it (key, tempo, tuning).
    pub(crate) fn np_about(&mut self, _ui: &mut egui::Ui, p: &egui::Painter, r: Rect, ink: Color32) {
        let Some(tr) = self.cur_track() else { return };
        let info = crate::meta::get(tr.id).unwrap_or_default();
        let mut rows: Vec<(&str, String)> = vec![("Song", tr.title.clone()), ("Artist", tr.artists_text())];
        if !tr.album.is_empty() {
            rows.push(("Album", tr.album.clone()));
        }
        rows.push(("Length", fmt_time(tr.duration)));
        if info.key.is_some() {
            rows.push(("Key", crate::meta::key_text(&info)));
        }
        if let Some(b) = info.bpm {
            rows.push(("Tempo", format!("{} BPM", b)));
        }
        if let Some(a4) = info.a4 {
            rows.push(("Tuning", format!("A = {:.1} Hz", a4)));
        }
        let src = match self.ext_of(tr.id) {
            None => "Tidal",
            Some(e) if e.kind == "yt" => "YouTube",
            Some(e) if e.kind == "sc" => "SoundCloud",
            _ => "A file on this computer",
        };
        rows.push(("From", src.to_string()));
        let mut y = r.min.y + 16.0;
        for (k, v) in rows {
            ptext(p, Pos2::new(r.min.x + 4.0, y), Align::Min, k, 1.5, ink.gamma_multiply(0.55));
            ptext_fit(p, Pos2::new(r.min.x + 110.0, y), Align::Min, &v, 1.9, r.width() - 120.0, ink);
            y += 36.0;
            if y > r.max.y - 10.0 {
                break;
            }
        }
    }

    /// The song's saved loops and bookmarks: a click on a loop plays it (looping), on a bookmark plays from there.
    pub(crate) fn np_loops(&mut self, ui: &mut egui::Ui, p: &egui::Painter, r: Rect, acts: &mut Vec<Action>, ink: Color32) {
        let Some(tr) = self.cur_track() else { return };
        let loops = self.store.sections.get(&tr.id).cloned().unwrap_or_default();
        let marks = self.store.marks.get(&tr.id).cloned().unwrap_or_default();
        if loops.is_empty() && marks.is_empty() {
            ptext_fit(p, Pos2::new(r.min.x + 4.0, r.min.y + 18.0), Align::Min, "No loops or bookmarks yet. Make them in Practice.", 1.6, r.width() - 8.0, ink.gamma_multiply(0.6));
            return;
        }
        let mut y = r.min.y + 4.0;
        let row = |ui: &mut egui::Ui, y: f32, id: (u8, usize), name: &str, when: &str| -> bool {
            let rr = Rect::from_min_size(Pos2::new(r.min.x, y), Vec2::new(r.width(), 40.0));
            let h = ui.interact(rr, ui.id().with(("np_loop", id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if h.hovered() {
                p.rect_filled(rr, 6.0, Color32::from_rgba_premultiplied(16, 16, 16, 16));
            }
            ptext_fit(p, Pos2::new(rr.min.x + 10.0, rr.center().y), Align::Min, name, 1.9, rr.width() - 140.0, ink);
            ptext(p, Pos2::new(rr.max.x - 10.0, rr.center().y), Align::Max, when, 1.5, ink.gamma_multiply(0.6));
            h.clicked()
        };
        for (i, s) in loops.iter().enumerate() {
            let name = if s.name.is_empty() { format!("Loop {}", i + 1) } else { s.name.clone() };
            if row(ui, y, (0, i), &name, &format!("{} – {}", fmt_t(s.a), fmt_t(s.b))) {
                acts.push(Action::GoSection(i));
            }
            y += 42.0;
        }
        if !marks.is_empty() {
            y += 8.0;
            ptext(p, Pos2::new(r.min.x + 10.0, y + 8.0), Align::Min, "Bookmarks", 1.4, ink.gamma_multiply(0.55));
            y += 24.0;
        }
        let v = self.version_of(&tr);
        for (k, m) in marks.iter().enumerate() {
            if y > r.max.y - 40.0 {
                break;
            }
            let name = if m.note.is_empty() { format!("Bookmark {}", k + 1) } else { m.note.clone() };
            if row(ui, y, (1, k), &name, &fmt_t(m.t)) {
                acts.push(Action::PlayAt(v.clone(), m.t));
            }
            y += 42.0;
        }
    }

    /// Practising: the cover small with the song and what's known of it at the top, then the practice tools (the
    /// waveform big, its loops and bookmarks on it, zoom, the A-B loop and the speed), as the practice window has them.
    fn np_practice(&mut self, ui: &mut egui::Ui, r: Rect, tex: Option<egui::TextureId>, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        let cs = Rect::from_min_size(r.min, Vec2::splat(84.0));
        if let Some(id) = tex {
            egui::Image::new(egui::load::SizedTexture::new(id, cs.size())).rounding(Rounding::same(6.0)).paint_at(ui, cs);
        }
        if let Some(tr) = self.cur_track() {
            // the time at the right, big (no tenths); a click shows what's left
            let dur = tr.duration.max(self.track_len());
            let pos = if self.cur.is_some() { self.pos() } else { 0.0 };
            let now_t = if self.time_left { format!("-{}", fmt_time((dur - pos).max(0.0))) } else { fmt_time(pos) };
            let total = format!("/ {}", fmt_time(dur));
            let (w1, w2) = (text_w(&now_t, 3.4), text_w(&total, 1.4));
            ptext(&p, Pos2::new(r.max.x, cs.min.y + 26.0), Align::Max, &total, 1.4, OVER.gamma_multiply(0.7));
            ptext(&p, Pos2::new(r.max.x - w2 - 8.0, cs.min.y + 22.0), Align::Max, &now_t, 3.4, OVER);
            let tr_r = Rect::from_min_max(Pos2::new(r.max.x - w2 - w1 - 12.0, cs.min.y), Pos2::new(r.max.x, cs.min.y + 44.0));
            if ui.interact(tr_r, ui.id().with("np_prac_time"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Click for the time left").clicked() {
                self.time_left = !self.time_left;
                self.dirty = true;
            }
            let tx = cs.max.x + 16.0;
            let name_w = (r.width() - 140.0 - w1 - w2 - 30.0).max(80.0);
            ptext_fit(&p, Pos2::new(tx, cs.min.y + 16.0), Align::Min, &tr.title, 2.6, name_w, OVER);
            ptext_fit(&p, Pos2::new(tx + 0.6, cs.min.y + 16.0), Align::Min, &tr.title, 2.6, name_w, OVER);
            ptext_fit(&p, Pos2::new(tx, cs.min.y + 42.0), Align::Min, &tr.artists_text(), 1.7, name_w, OVER.gamma_multiply(0.7));
            // what's known of it, as tags
            let info = crate::meta::get(tr.id).unwrap_or_default();
            let mut tags: Vec<String> = Vec::new();
            if info.key.is_some() {
                tags.push(format!("Key {}", crate::meta::key_text(&info)));
            }
            if let Some(b) = info.bpm {
                tags.push(format!("{} BPM", b));
            }
            if let Some(a4) = info.a4 {
                tags.push(format!("A = {:.0}", a4));
            }
            let nl = self.store.sections.get(&tr.id).map_or(0, |l| l.len());
            let nm = self.store.marks.get(&tr.id).map_or(0, |l| l.len());
            if nl + nm > 0 {
                tags.push(format!("{} loop{} · {} mark{}", nl, if nl == 1 { "" } else { "s" }, nm, if nm == 1 { "" } else { "s" }));
            }
            let mut x = tx;
            for t in tags {
                let w = text_w(&t, 1.4) + 18.0;
                let tr = Rect::from_min_size(Pos2::new(x, cs.min.y + 58.0), Vec2::new(w, 22.0));
                p.rect_filled(tr, 5.0, Color32::from_rgba_premultiplied(26, 26, 26, 26));
                ptext(&p, tr.center(), Align::Center, &t, 1.4, OVER);
                x = tr.max.x + 8.0;
            }
        }
        let body = Rect::from_min_max(Pos2::new(r.min.x, cs.max.y + 14.0), r.max);
        // (the tour points at the practice tools)
        tour::mark("ARTPRACTICE", body);
        self.practice_stack(ui, body, acts, false);
    }

    /// The foot: how far in (the loops on it when practising), the main controls in the middle, the volume at the
    /// left, and at the right the speed, practice, the sound output, "..." (everything else) and Close.
    #[allow(clippy::too_many_arguments)]
    fn np_foot(&mut self, ui: &mut egui::Ui, full: Rect, bar_h: f32, active: bool, pos: f32, dur: f32, acts: &mut Vec<Action>) {
        let p = ui.painter().clone();
        // (Studio: the foot on its black body in the screen's label colour, the big button a key of glass)
        let studio = crate::studio::on();
        let over = if studio { crate::studio::label() } else { OVER };
        // ---- how far in
        let sb = Rect::from_min_size(Pos2::new(full.min.x + 28.0, full.max.y - bar_h), Vec2::new(full.width() - 56.0, 10.0));
        let sr = ui.interact(sb.expand2(Vec2::new(0.0, 4.0)), ui.id().with("art_seek"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand);
        if active && dur > 0.0 && (sr.clicked() || sr.drag_stopped()) {
            if let Some(pp) = sr.interact_pointer_pos() {
                acts.push(Action::Seek(((pp.x - sb.min.x) / sb.width()).clamp(0.0, 1.0) * dur));
            }
        }
        let line = Rect::from_center_size(sb.center(), Vec2::new(sb.width(), 4.0));
        if active && dur > 0.0 {
            crate::widgets::seek_label(ui, &sr, line, |x| ((x - sb.min.x) / sb.width()).clamp(0.0, 1.0) * dur);
        }
        // (a soft shade under the bar, the time, the volume and every button, so they read over a pale cover or
        // visuals as well as a dark one)
        soft_spot(&p, line, 8.0, 0.22);
        p.rect_filled(line, 2.0, Color32::from_rgba_premultiplied(60, 60, 60, 60));
        let frac = if active && dur > 0.0 { (pos / dur).clamp(0.0, 1.0) } else { 0.0 };
        p.rect_filled(Rect::from_min_size(line.min, Vec2::new(line.width() * frac, line.height())), 2.0, over);
        if sr.hovered() {
            p.circle_filled(Pos2::new(line.min.x + line.width() * frac, line.center().y), 6.0, over);
        }
        if practice_tools() && self.practice && active && dur > 0.0 {
            let x_at = |t: f32| line.min.x + line.width() * (t / dur).clamp(0.0, 1.0);
            if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
                let r = Rect::from_min_max(Pos2::new(x_at(a), line.min.y - 2.0), Pos2::new(x_at(b).max(x_at(a) + 3.0), line.max.y + 2.0));
                p.rect_filled(r, 2.0, crate::skin::loop_col().gamma_multiply(if self.loop_on { 0.8 } else { 0.45 }));
            }
        }
        // (the length at the right: a click shows what's left instead, as the decks' time does)
        let right = if self.time_left && active { format!("-{}", fmt_time((dur - pos).max(0.0))) } else { fmt_time(dur) };
        for (x, align, t) in [(sb.min.x, Align::Min, fmt_time(if active { pos } else { 0.0 })), (sb.max.x, Align::Max, right)] {
            let at = Pos2::new(x, sb.max.y + 12.0);
            let w = text_w(&t, 1.7);
            let x0 = if align == Align::Min { at.x } else { at.x - w };
            soft_spot(&p, Rect::from_min_max(Pos2::new(x0 + 2.0, at.y - 5.0), Pos2::new(x0 + w - 2.0, at.y + 5.0)), 11.0, 0.28);
            ptext(&p, at, align, &t, 1.7, over);
            if align == Align::Max {
                let tr = Rect::from_min_max(Pos2::new(x0 - 4.0, at.y - 10.0), Pos2::new(at.x + 4.0, at.y + 10.0));
                if ui.interact(tr, ui.id().with("np_time_left"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Click for the time left").clicked() {
                    self.time_left = !self.time_left;
                    self.dirty = true;
                }
            }
        }
        // ---- the main controls, in the middle
        let cy = (sb.max.y + full.max.y) / 2.0 + 6.0;
        let playing = active && !self.paused;
        let liked_now = self.cur_track().map(|t| self.liked.contains(&t.id)).unwrap_or(false);
        let cur_t = self.cur_track();
        let gap = 44.0;
        // (the transport in the middle; a narrow window: left of the keys at the right, clear of them, and then the
        // volume slider goes, its headphones staying)
        let half = 2.5 * gap + 34.0;
        let row_left = full.max.x - 28.0 - self.np_row_w.max(100.0) - 12.0;
        let x0 = full.center().x.min(row_left - half);
        let vol_room = x0 - half - 12.0 >= full.min.x + 170.0;
        let x0 = if vol_room { x0 } else { x0.max(full.min.x + 60.0 + half) };
        // (narrower still: previous, play and next alone, shuffle, repeat and the heart left to the keyboard and menus)
        let slim = x0 + half + 12.0 > row_left;
        let x0 = if slim { (full.min.x + 60.0 + 1.5 * gap + 34.0).max(row_left - 1.5 * gap - 46.0) } else { x0 };
        let mut n = 0usize;
        let mut bare = |ui: &mut egui::Ui, icon: &str, on: bool, big: bool, tip: &str, x: f32| -> bool {
            n += 1;
            let d = if big { 44.0 } else { 26.0 };
            let r = Rect::from_center_size(Pos2::new(x, cy), Vec2::splat(d));
            let resp = ui.interact(r, ui.id().with(("np_ctl", n)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
            let p = ui.painter();
            if big && studio {
                let k = Rect::from_center_size(r.center(), Vec2::splat(d - 6.0));
                p.rect_filled(k, 0.0, if resp.hovered() { crate::studio::key_col() } else { crate::studio::scr() });
                p.rect_stroke(k, 0.0, Stroke::new(1.0_f32, crate::studio::ink()));
                crate::vicon::named(p, Rect::from_center_size(k.center(), Vec2::splat(d * 0.38)), icon, crate::studio::ink());
            } else if big {
                let rad = d / 2.0 + if resp.hovered() { 1.0 } else { 0.0 };
                soft_spot(p, Rect::from_center_size(r.center(), Vec2::splat(2.0 * rad)), 9.0, 0.24);
                p.circle_filled(r.center(), rad, over);
                let off = if icon == "play" { 1.5 } else { 0.0 };
                crate::vicon::named(p, Rect::from_center_size(r.center() + Vec2::new(off, 0.0), Vec2::splat(d * 0.42)), icon, Color32::from_rgb(20, 20, 22));
            } else {
                let col = if resp.hovered() { Color32::WHITE } else { over };
                soft_spot(p, Rect::from_center_size(r.center(), Vec2::splat(d * 0.5)), 10.0, 0.28);
                crate::vicon::named(p, Rect::from_center_size(r.center(), Vec2::splat(d * 0.62)), icon, col);
                if on {
                    p.circle_filled(Pos2::new(r.center().x, r.max.y + 3.0), 2.0, col);
                }
            }
            resp.clicked()
        };
        if !slim && bare(ui, "shuffle", self.shuffle, false, if self.shuffle { "Shuffle: on" } else { "Shuffle: off" }, x0 - 2.5 * gap) {
            acts.push(Action::Shuffle);
        }
        if bare(ui, "prev", false, false, "Previous", x0 - 1.5 * gap) {
            acts.push(Action::Prev);
        }
        if bare(ui, if playing { "pause" } else { "play" }, false, true, if playing { "Pause  (SPACE)" } else { "Play  (SPACE)" }, x0 - 0.5 * gap + 4.0) {
            acts.push(Action::Toggle);
        }
        if bare(ui, "skip", false, false, "Next", x0 + 0.5 * gap + 8.0) {
            acts.push(Action::Next);
        }
        let rep_tip = match self.repeat {
            Repeat::Off => "Repeat: off  (click for all)",
            Repeat::All => "Repeat: all  (click for one)",
            Repeat::One => "Repeat: one  (click for off)",
        };
        if !slim && bare(ui, "loop", self.repeat != Repeat::Off, false, rep_tip, x0 + 1.5 * gap + 8.0) {
            acts.push(Action::Repeat);
        }
        if !slim && bare(ui, "heart", liked_now, false, if liked_now { "Remove from My Tracks  (H)" } else { "Add to My Tracks  (H)" }, x0 + 2.5 * gap + 8.0) {
            if let Some(t) = &cur_t {
                acts.push(Action::ToggleLike(t.clone()));
            }
        }
        // ---- the volume, at the left
        let vol = Rect::from_min_max(Pos2::new(full.min.x + 56.0, cy - 7.0), Pos2::new(full.min.x + 170.0, cy + 7.0));
        let spk = Rect::from_center_size(Pos2::new(full.min.x + 38.0, cy), Vec2::splat(24.0));
        let sh = ui.interact(spk, ui.id().with("np_mute"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(if self.volume <= 0.0 { "Unmute" } else { "Mute" });
        soft_spot(&p, Rect::from_center_size(spk.center(), Vec2::splat(12.0)), 10.0, 0.28);
        crate::vicon::named(&p, spk.shrink(4.0), "headphones", if sh.hovered() { Color32::WHITE } else { over.gamma_multiply(if self.volume <= 0.0 { 0.45 } else { 0.9 }) });
        if sh.clicked() {
            if self.volume > 0.0 {
                self.vol_before = self.volume;
                acts.push(Action::Volume(0.0));
            } else {
                acts.push(Action::Volume(self.vol_before.max(0.3)));
            }
        }
        // (a narrow window: the slider goes, the headphones stay)
        if vol_room {
            let vr = ui.interact(vol, ui.id().with("np_vol"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(format!("Volume {}%", (self.volume * 100.0).round()));
            if vr.dragged() || vr.clicked() {
                if let Some(pp) = vr.interact_pointer_pos() {
                    acts.push(Action::Volume(((pp.x - vol.min.x) / vol.width()).clamp(0.0, 1.0)));
                }
            }
            let wheel = ui.input(|i| i.smooth_scroll_delta.y);
            if vr.hovered() && wheel != 0.0 {
                acts.push(Action::Volume((self.volume + wheel * 0.002).clamp(0.0, 1.0)));
            }
            let vl = Rect::from_center_size(vol.center(), Vec2::new(vol.width(), 4.0));
            soft_spot(&p, vl, 8.0, 0.22);
            p.rect_filled(vl, 2.0, Color32::from_rgba_premultiplied(60, 60, 60, 60));
            p.rect_filled(Rect::from_min_size(vl.min, Vec2::new(vl.width() * self.volume.clamp(0.0, 1.0), 4.0)), 2.0, over);
        }
        // ---- at the right: speed, practice, the output, "...", Close
        let row = Rect::from_min_max(Pos2::new(x0 + 2.5 * gap + 30.0, cy - bh() / 2.0), Pos2::new(full.max.x - 28.0, cy + bh() / 2.0));
        // (the tour points at the controls along the foot)
        tour::mark("ARTBTNS", Rect::from_min_max(Pos2::new(full.min.x + 28.0, cy - bh() / 2.0), row.max));
        let used = ui
            .allocate_new_ui(egui::UiBuilder::new().max_rect(row), |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if retro_btn(ui, "CLOSE", false).tip("Back to the library  (ESC)").clicked() {
                    acts.push(Action::ToggleArt);
                }
                let more = foot_icon(ui, "more", over, "More: the visualizer, the visuals, what's shown, full screen...");
                let id = ui.id().with("np_more");
                if more.clicked() {
                    ui.memory_mut(|m| m.toggle_popup(id));
                }
                egui::popup::popup_above_or_below_widget(ui, id, &more, egui::AboveOrBelow::Above, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                    ui.set_min_width(260.0);
                    self.np_more_items(ui, acts);
                });
                let ob = foot_icon(ui, "headphones", over, "Sound output: speakers, headphones or Bluetooth");
                self.output_popup(ui, &ob, acts);
                if practice_tools() && retro_btn(ui, "PRACTICE", self.practice).tip("Practice mode: the waveform, loops and speed  (P)").clicked() {
                    acts.push(Action::TogglePractice);
                }
                let spd = retro_btn(ui, &crate::tools::speed_label(self.speed), self.speed != 100)
                    .tip("Playback speed (pitch unchanged). Click to cycle, right-click to pick or type one  (UP and DOWN keys)");
                if spd.clicked() {
                    acts.push(Action::Speed(crate::tools::next_speed(self.speed)));
                }
                spd.context_menu(|ui| self.speed_menu(ui, acts));
                })
                .response
                .rect
            })
            .inner;
        // (how wide the keys at the right came out: next frame the transport keeps clear of them)
        self.np_row_w = used.width();
    }

    /// The "..." menu's items: the visualizer behind the cover and its settings, the visuals, popping them out,
    /// what's shown of the song, its text size, full screen.
    fn np_more_items(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let mark = |on: bool| if on { "> " } else { "   " };
        // (Hardware's amp has no cover to dress: what's under its panel, the visuals, full screen)
        if crate::studio::on() {
            // (the cover's color or ink: a click on the cover)
            if menu_item(ui, &format!("   {}", if self.fullscreen { "Exit Full Screen  (F)" } else { "Full Screen  (F)" })) {
                acts.push(Action::ToggleFullscreen);
                ui.close_menu();
            }
            return;
        }
        if crate::skin::style() == crate::skin::STYLE_HARDWARE {
            for (i, name) in ["Meters", "Visuals", "Lyrics"].iter().enumerate() {
                if menu_item(ui, &format!("{}Under the Panel: {}", mark(self.hw_show as usize == i), name)) {
                    self.hw_show = i as u8;
                    self.dirty = true;
                    ui.close_menu();
                }
            }
            crate::sub_menu(ui, "   Pick the Visuals", |ui| self.vis_menu(ui));
            ui.separator();
            if menu_item(ui, &format!("   {}", if self.fullscreen { "Exit Full Screen  (F)" } else { "Full Screen  (F)" })) {
                acts.push(Action::ToggleFullscreen);
                ui.close_menu();
            }
            return;
        }
        if menu_item(ui, &format!("{}Visualizer Behind the Cover", mark(self.show_spec))) {
            acts.push(Action::ToggleSpec);
            ui.close_menu();
        }
        if menu_item(ui, "   Visualizer Settings...") {
            self.viz_panel = Some(1);
            ui.close_menu();
        }
        if menu_item(ui, &format!("{}Visuals Instead of the Cover", mark(self.vis_np))) {
            self.vis_np = !self.vis_np;
            self.dirty = true;
            ui.close_menu();
        }
        crate::sub_menu(ui, "   Pick the Visuals", |ui| self.vis_menu(ui));
        if crate::popout::CAN && menu_item(ui, "   Pop Out the Visuals") {
            self.pop_toggle();
            ui.close_menu();
        }
        ui.separator();
        for (i, name) in ["Art and Info", "Info Only", "Nothing"].iter().enumerate() {
            if menu_item(ui, &format!("{}Show: {}", mark(self.cap_show as usize == i), name)) {
                self.cap_show = i as u8;
                self.cap_mode_at = ui.input(|i| i.time);
                self.dirty = true;
                ui.close_menu();
            }
        }
        for (i, name) in ["Small", "Medium", "Large"].iter().enumerate() {
            if menu_item(ui, &format!("{}Text: {}", mark(self.cap_size as usize == i), name)) {
                self.cap_size = i as u8;
                self.dirty = true;
                ui.close_menu();
            }
        }
        ui.separator();
        if menu_item(ui, &format!("   {}", if self.fullscreen { "Exit Full Screen  (F)" } else { "Full Screen  (F)" })) {
            acts.push(Action::ToggleFullscreen);
            ui.close_menu();
        }
    }
}

/// The one colour of what's drawn straight over the picture (the bare symbols and the times): near-white on a soft
/// dark spot (`soft_spot`), so it reads over any cover or visual without changing with them.
const OVER: Color32 = Color32::from_rgb(240, 240, 238);

/// A soft dark spot under something drawn straight over the picture: `inner` (a rounded shape, as round as its
/// shorter side allows: a circle for a square) filled at `a`, fading smoothly to nothing over `spread` all round, the
/// same in every direction. One gradient (no steps, no edge), so the symbol on it reads over any visual.
/// An icon key along Now Playing's foot, in the foot's own colour (on the cover's backdrop or Studio's black body,
/// not a page's ink), a soft shade under it as the other controls have; full strength under the pointer.
fn foot_icon(ui: &mut egui::Ui, icon: &str, col: Color32, tip: &str) -> egui::Response {
    let h = bh();
    let (r, resp) = ui.allocate_exact_size(Vec2::new(h + 2.0, h), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
    let p = ui.painter();
    soft_spot(p, Rect::from_center_size(r.center(), Vec2::splat(h * 0.4)), 10.0, 0.28);
    let c = if resp.hovered() { col } else { col.gamma_multiply(0.82) };
    crate::vicon::named(p, Rect::from_center_size(r.center(), Vec2::splat(h * 0.58)), icon, c);
    resp
}

fn soft_spot(p: &egui::Painter, inner: Rect, spread: f32, a: f32) {
    let rr = inner.width().min(inner.height()) / 2.0;
    // the corner arcs' centres, each with the angle its quarter starts at (going round)
    let corners = [
        (Pos2::new(inner.max.x - rr, inner.max.y - rr), 0.0f32),
        (Pos2::new(inner.min.x + rr, inner.max.y - rr), 90.0),
        (Pos2::new(inner.min.x + rr, inner.min.y + rr), 180.0),
        (Pos2::new(inner.max.x - rr, inner.min.y + rr), 270.0),
    ];
    const K: usize = 10;
    const RINGS: usize = 10;
    let per = corners.len() * (K + 1);
    let uv = egui::epaint::WHITE_UV;
    let mut m = egui::Mesh::default();
    m.vertices.push(egui::epaint::Vertex { pos: inner.center(), uv, color: Color32::from_black_alpha((a * 255.0) as u8) });
    for ring in 0..=RINGS {
        let t = ring as f32 / RINGS as f32;
        // (eases out: full at the shape's edge, a long gentle tail)
        let f = 1.0 - t * t * (3.0 - 2.0 * t);
        let col = Color32::from_black_alpha((a * f * 255.0) as u8);
        let r = rr + spread * t;
        for (c, start) in corners {
            for j in 0..=K {
                let ang = (start + 90.0 * j as f32 / K as f32).to_radians();
                m.vertices.push(egui::epaint::Vertex { pos: c + Vec2::new(ang.cos(), ang.sin()) * r, uv, color: col });
            }
        }
    }
    let at = |ring: usize, i: usize| (1 + ring * per + i % per) as u32;
    for i in 0..per {
        m.indices.extend_from_slice(&[0, at(0, i), at(0, i + 1)]);
    }
    for ring in 0..RINGS {
        for i in 0..per {
            let (a0, a1, b0, b1) = (at(ring, i), at(ring, i + 1), at(ring + 1, i), at(ring + 1, i + 1));
            m.indices.extend_from_slice(&[a0, a1, b1, a0, b1, b0]);
        }
    }
    p.add(egui::Shape::mesh(m));
}

/// The cover as a fine grid of points, each placed through the same 3D tilt, so the picture stays a flat,
/// stiff card at any angle (stretched over just two triangles it would bend along the diagonal).
fn cover_grid(p: &egui::Painter, xf: &impl Fn(f32, f32) -> Pos2, h: f32, id: egui::TextureId, alpha: f32) {
    const N: u32 = 12;
    const E: f32 = 0.006;
    let color = Color32::from_white_alpha((alpha.clamp(0.0, 1.0) * 255.0) as u8);
    let mut mesh = egui::Mesh::with_texture(id);
    for j in 0..=N {
        for i in 0..=N {
            let (u, v) = (i as f32 / N as f32, j as f32 / N as f32);
            let pos = xf(-h + 2.0 * h * u, -h + 2.0 * h * v);
            let uv = Pos2::new(E + (1.0 - 2.0 * E) * u, E + (1.0 - 2.0 * E) * v);
            mesh.vertices.push(egui::epaint::Vertex { pos, uv, color });
        }
    }
    for j in 0..N {
        for i in 0..N {
            let a = j * (N + 1) + i;
            let b = a + N + 1;
            mesh.indices.extend_from_slice(&[a, a + 1, b + 1, a, b + 1, b]);
        }
    }
    p.add(egui::Shape::mesh(mesh));
}
