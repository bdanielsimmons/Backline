//! Winamp skins drawn for real: the main window and the equalizer from the skin's own bitmaps, at Winamp's own
//! positions (sprite and layout map from Webamp, the open-source Winamp remake), scaled in whole screen pixels.
//! Every control works and drives Backline: transport, seek, volume, shuffle, repeat, the equalizer.

use super::*;

pub(crate) use crate::winamp_draw::*;

impl App {
    /// The mini player: the deck, and under it - as Winamp stacked them - the skin's own equalizer and playlist when
    /// its EQ and PL buttons open them, all at the deck's width and scale. Without a skin (or with neither open),
    /// just the deck.
    pub(crate) fn mini_window(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let stack = (self.show_eq, !self.queue_hidden);
        if self.wa_art.is_none() || stack == (false, false) {
            self.player_window(ui, acts);
            return;
        }
        let full = ui.max_rect();
        let k = (full.width() / 275.0).max(0.5);
        let deck = Rect::from_min_size(full.min, Vec2::new(full.width(), 116.0 * k));
        let mut c = ui.new_child(egui::UiBuilder::new().id_salt("mini_deck").max_rect(deck));
        self.player_window(&mut c, acts);
        let mut y = deck.max.y;
        if stack.0 {
            let r = Rect::from_min_size(Pos2::new(full.min.x, y), Vec2::new(full.width(), 116.0 * k));
            let mut c = ui.new_child(egui::UiBuilder::new().id_salt("mini_eq").max_rect(r));
            self.winamp_eq(&mut c, r);
            y = r.max.y;
        }
        if stack.1 {
            // the rest of the window (never less than the playlist's frame while the window is still growing)
            let r = Rect::from_min_max(Pos2::new(full.min.x, y), Pos2::new(full.max.x, full.max.y.max(y + 58.0 * k)));
            let mut c = ui.new_child(egui::UiBuilder::new().id_salt("mini_pl").max_rect(r));
            if self.wa_art.as_ref().is_some_and(|t| t.sheets.contains_key("pledit")) {
                self.winamp_playlist(&mut c, r, acts);
            } else {
                self.playlist_ui(&mut c, acts);
            }
        }
        ui.allocate_rect(full, Sense::hover());
    }

    /// The mini window's height follows its stack: the deck, then the equalizer (as tall as the deck), then the
    /// playlist (twice that), all at the deck's scale.
    pub(crate) fn mini_stack_size(&mut self, ctx: &egui::Context) {
        if self.wa_art.is_none() {
            return;
        }
        let want = (self.show_eq, !self.queue_hidden);
        if want == self.mini_stack {
            return;
        }
        self.mini_stack = want;
        // the deck's width: the one just asked for when the mini player has only now come on
        let w = self.mini_size_at.map_or(ctx.screen_rect().width(), |(_, s)| s.x);
        let k = w / 275.0;
        let h = 116.0 * k + if want.0 { 116.0 * k } else { 0.0 } + if want.1 { 232.0 * k } else { 0.0 };
        let size = Vec2::new(w, h);
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        if self.mini_size_at.is_some() {
            self.mini_size_at = Some((std::time::Instant::now(), size));
        }
    }

    /// The visualizer settings' preview of the player screen while a skin is worn: the skin's own visualizer,
    /// as large as fits in `r`. False when no skin is worn.
    pub(crate) fn skin_viz_preview(
        &self,
        p: &egui::Painter,
        r: Rect,
        bands: &[f32],
        peaks: &[f32],
        wave: &[f32],
        mode: u8,
    ) -> bool {
        let Some(t) = self.wa_art.as_ref() else { return false };
        let ppp = font::ppp();
        let s = (((r.width() / 76.0).min(r.height() / 16.0) * ppp).floor().max(1.0)) / ppp;
        let o = Pos2::new(r.center().x - 38.0 * s, r.center().y - 8.0 * s);
        p.rect_filled(Rect::from_min_size(o, Vec2::new(76.0, 16.0) * s), 0.0, t.vis[0]);
        skin_viz(p, o, s, &t.vis, bands, peaks, wave, mode);
        true
    }

    /// The main window, drawn from the worn skin. Everything is clickable and drives Backline.
    pub(crate) fn winamp_main(&mut self, ui: &mut egui::Ui, area: Rect, acts: &mut Vec<Action>) {
        let Some(t) = self.wa_art.take() else { return };
        // the player sits in the middle, in line with the equalizer under it; the album cover is small, in the
        // margin to its right, framed in the skin's colours; a click on it opens the album view
        // mini: the window is the player - the deck fills it, no cover beside it
        // (the deck is the top of the stack: as wide as the playlist under it, no cover beside it - that is in NOW
        // PLAYING)
        let mut side = 0.0;
        let skin_area = if self.mini { area } else { area.shrink2(Vec2::new(side + 20.0, 0.0)) };
        // mini: the deck fills the window edge to edge (the window has its shape); otherwise whole pixels, centred -
        // or the size you gave it (dragging its corner, or ctrl + the mouse wheel over it), up to the whole window
        let pl = if self.mini {
            Place { o: skin_area.min, s: (skin_area.width() / 275.0).min(skin_area.height() / 116.0) }
        } else if let Some(z) = self.deck_zoom {
            // never past what shows of the window (its edges may be cut by the panel). When Backline opens the
            // cover beside it always shows (the size kept from last time stops at it); sized up now, the cover
            // steps aside to make room
            let shown = area.intersect(ui.clip_rect());
            let limit = if self.deck_free { shown } else { skin_area.intersect(shown) };
            let s = z.min((limit.width() / 275.0).min(limit.height() / 116.0)).max(0.5);
            let room = if 275.0 * s > skin_area.width() { shown } else { skin_area.intersect(shown) };
            if 275.0 * s > skin_area.width() {
                side = 0.0;
            }
            let snap = |v: f32| (v * font::ppp()).round() / font::ppp();
            Place { o: Pos2::new(snap(room.center().x - 137.5 * s), snap(room.center().y - 58.0 * s)), s }
        } else {
            // the stack's size; the panel's whole width (its frame margins too), its foot on the playlist below; centred
            // when the column is wider than the deck (practising keeps the deck smaller)
            let s = self.stack_k;
            let snap = |v: f32| (v * font::ppp()).round() / font::ppp();
            let x = area.min.x - 14.0 + ((area.width() + 28.0 - 275.0 * s) / 2.0).max(0.0);
            Place { o: Pos2::new(snap(x), snap(area.max.y + 14.0 - 116.0 * s)), s }
        };
        if !self.mini {
            let deck = Rect::from_min_size(pl.o, Vec2::new(275.0, 116.0) * pl.s);
            // ctrl + the mouse wheel over the deck sizes it
            if ui.rect_contains_pointer(deck) {
                let zd = ui.input(|i| if i.modifiers.command { i.zoom_delta() } else { 1.0 });
                if zd != 1.0 {
                    self.deck_zoom = Some((pl.s * zd).clamp(0.5, 8.0));
                    self.deck_free = true;
                    self.dirty = true;
                }
            }
            // its corner: drag to size it (it stays centred), double-click for as large as fits
            let grip = Rect::from_min_size(deck.max - Vec2::splat(12.0), Vec2::splat(12.0));
            let gh = ui
                .interact(grip, ui.id().with("deck_grip"), Sense::click_and_drag())
                .on_hover_cursor(egui::CursorIcon::ResizeNwSe)
                .tip("Drag to size the player, or use ctrl + mouse wheel. Double-click to make it as large as fits.");
            if gh.double_clicked() {
                self.deck_zoom = None;
                self.dirty = true;
            } else if gh.dragged() {
                if let Some(pp) = gh.interact_pointer_pos() {
                    let c = deck.center();
                    let s = ((pp.x - c.x) * 2.0 / 275.0).max((pp.y - c.y) * 2.0 / 116.0);
                    self.deck_zoom = Some(s.clamp(0.5, 8.0));
                    self.deck_free = true;
                    self.dirty = true;
                }
            }
            if gh.hovered() || gh.dragged() {
                let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, ui.id().with("deck_grip_mark")));
                for k in 0..3 {
                    let d = 3.0 + k as f32 * 3.0;
                    p.line_segment(
                        [Pos2::new(grip.max.x - d, grip.max.y - 1.0), Pos2::new(grip.max.x - 1.0, grip.max.y - d)],
                        Stroke::new(1.5_f32, pal().ink),
                    );
                }
            }
        }
        let right = pl.o.x + 275.0 * pl.s;
        let cover = Rect::from_center_size(Pos2::new((right + area.max.x) / 2.0, area.center().y), Vec2::splat(side));
        // the tour points at the skinned player and its cover too
        crate::tour::note("PLAYER", Rect::from_min_size(pl.o, Vec2::new(275.0 * pl.s, 116.0 * pl.s)));
        // the deck's title-bar buttons. The middle one is the window's size, as Windows' own: one square goes
        // fullscreen, two overlapping squares (while fullscreen) come back; in mini it goes back to the full window.
        // The mini player has its own button beside them. In mini the deck is the window, so minimize and close work
        // as Winamp's did
        let tid = ui.id().with("wa_titlebar");
        let at = |x: f32| Rect::from_min_size(pl.o + Vec2::new(x, 3.0) * pl.s, Vec2::new(9.0, 9.0) * pl.s);
        if self.mini {
            // no system title bar in mini: the deck's own one moves the window (its buttons, drawn after, win)
            let bar = Rect::from_min_size(pl.o, Vec2::new(275.0, 14.0) * pl.s);
            if ui.interact(bar, tid.with("drag"), Sense::drag()).drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
        }
        // (in the full window the middle one is the skin's own button below, which goes fullscreen)
        {
            if self.mini
                && ui
                    .interact(at(254.0), tid.with("size"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .tip("Back to the full window.")
                    .clicked()
            {
                acts.push(Action::ToggleMini);
            }
            if !self.mini
                && ui
                    .interact(at(234.0), tid.with("mini"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .tip("Switch to the mini player.")
                    .clicked()
            {
                acts.push(Action::ToggleMini);
            }
        }
        if self.mini {
            for (k, x, tip) in [(0, 244.0, "Minimize"), (2, 264.0, "Close Backline")] {
                if ui
                    .interact(at(x), tid.with(k), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .tip(tip)
                    .clicked()
                {
                    match k {
                        0 => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                        _ => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                    }
                }
            }
        }
        crate::tour::mark("COVER", cover);
        if self.mini || side < 1.0 {
            // (no cover in mini, or with the player sized to fill the window)
        } else if let Some(tr) = self.cur_track() {
            let url = cover_url(&tr.cover, 320);
            // framed in the skin's colours
            raised(ui.painter(), cover.expand(4.0), false);
            inset(ui.painter(), cover.expand(1.0), pal().lcd);
            paint_held(ui, &mut self.images, "player", &url, cover, false);
        } else {
            paint_record(ui.painter(), cover, None);
        }
        let ch = ui
            .interact(cover, ui.id().with("winamp_cover"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("Now Playing  (A)");
        if ch.clicked() {
            acts.push(Action::ToggleArt);
        }
        let p = ui.painter().clone();
        let track = self.cur_track();
        let pos = self.seek_drag.unwrap_or(self.pos());
        let dur = track.as_ref().map(|t| t.duration).unwrap_or(0.0);
        let active = self.cur.is_some() && !self.stopped;
        let playing = active && !self.paused;
        let rate = self.player.shared.lock().unwrap().rate;
        let id = ui.id().with("winamp_main");
        let hit = |ui: &mut egui::Ui, name: &str, r: Rect, drag: bool| {
            ui.interact(r, id.with(name), if drag { Sense::click_and_drag() } else { Sense::click() })
                .on_hover_cursor(egui::CursorIcon::PointingHand)
        };

        t.spr(&p, "main", (0.0, 0.0, 275.0, 116.0), pl.r(0.0, 0.0, 275.0, 116.0));
        t.spr(&p, "titlebar", (27.0, 0.0, 275.0, 14.0), pl.r(0.0, 0.0, 275.0, 14.0));
        // the title bar's own buttons: options menu, minimize, shade (MINI), close
        let tb = |ui: &mut egui::Ui, name: &str, x: f32, src: (f32, f32), tip: &str| -> egui::Response {
            let r = pl.r(x, 3.0, 9.0, 9.0);
            let h = ui.interact(r, id.with(name), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip);
            if h.is_pointer_button_down_on() {
                t.spr(&p, "titlebar", (src.0, src.1 + 9.0, 9.0, 9.0), r);
            }
            h
        };
        let opt = tb(ui, "opt", 6.0, (0.0, 0.0), "Options: skins, preferences and help.");
        let opt_menu = id.with("opt_menu");
        if opt.clicked() {
            ui.memory_mut(|m| m.toggle_popup(opt_menu));
        }
        egui::popup::popup_below_widget(ui, opt_menu, &opt, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
            // a menu's width, however wide the things in it would like to be
            ui.set_max_width(320.0 * crate::font::ui_scale());
            // the menu can be taller than the mini player: a way out at the top, and it scrolls in what room there is
            if menu_item(ui, "x  Close this menu") {
                ui.memory_mut(|m| m.close_popup());
            }
            let room = (ui.ctx().screen_rect().max.y - ui.cursor().min.y - 12.0).max(80.0);
            egui::ScrollArea::vertical().max_height(room).show(ui, |ui| {
                if self.app_menu_items(ui, acts) {
                    ui.memory_mut(|m| m.close_popup());
                }
            });
        });
        if tb(ui, "min", 244.0, (9.0, 0.0), "Minimize").clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
        if !self.mini
            && tb(ui, "shade", 254.0, (0.0, 18.0), if self.fullscreen { "Leave fullscreen" } else { "Fullscreen" }).clicked()
        {
            acts.push(Action::ToggleFullscreen);
        }
        if tb(ui, "close", 264.0, (18.0, 0.0), "Quit Backline").clicked() {
            acts.push(Action::QuitApp);
        }
        // the clutter bar: Options, Always on top, song Info, Double size, Visualizer
        if t.size("titlebar").map_or(false, |s| s.x >= 344.0) {
            t.spr(&p, "titlebar", (304.0, 0.0, 8.0, 43.0), pl.r(10.0, 22.0, 8.0, 43.0));
            let zoomed = ui.ctx().zoom_factor() > 1.4;
            let items: [(&str, f32, f32, f32, bool, &str); 5] = [
                ("o", 304.0, 3.0, 8.0, false, "Options"),
                ("a", 312.0, 11.0, 7.0, self.pinned, "Always on top"),
                ("i", 320.0, 18.0, 7.0, false, "Song info (credits)"),
                ("d", 328.0, 25.0, 8.0, zoomed, "Double size"),
                ("v", 336.0, 33.0, 7.0, false, "Visualizer settings"),
            ];
            for (name, sx, y, h, on, tip) in items {
                let r = pl.r(10.0, 22.0 + y, 8.0, h);
                let hr = ui
                    .interact(r, id.with(("clutter", name)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .tip(tip);
                if on || hr.is_pointer_button_down_on() {
                    t.spr(&p, "titlebar", (sx, 44.0 + y, 8.0, h), r);
                }
                if hr.clicked() {
                    match name {
                        "o" => ui.memory_mut(|m| m.toggle_popup(opt_menu)),
                        "a" => acts.push(Action::TogglePin),
                        "i" => {
                            if let Some(tr) = self.cur_track().filter(|t| self.ext_of(t.id).is_none()) {
                                acts.push(Action::Credits(tr.id, format!("{} - {}", tr.artists_text(), tr.title)));
                            }
                        }
                        "d" => {
                            ui.ctx().set_zoom_factor(if zoomed { 1.0 } else { 2.0 });
                            self.dirty = true;
                        }
                        _ => self.viz_panel = Some(0),
                    }
                }
            }
        }
        // playing / paused / stopped
        let st = if playing {
            0.0
        } else if active {
            9.0
        } else {
            18.0
        };
        t.spr(&p, "playpaus", (st, 0.0, 9.0, 9.0), pl.r(26.0, 28.0, 9.0, 9.0));
        // the time, in the skin's big digits
        let digits = if t.size("nums_ex").is_some() { "nums_ex" } else { "numbers" };
        let secs = pos.max(0.0) as u32;
        let (m, s) = ((secs / 60) % 100, secs % 60);
        if active {
            for (x, d) in [(48.0, m / 10), (60.0, m % 10), (78.0, s / 10), (90.0, s % 10)] {
                t.spr(&p, digits, (d as f32 * 9.0, 0.0, 9.0, 13.0), pl.r(x, 26.0, 9.0, 13.0));
            }
        }
        // the scrolling title; while a slider is hovered or dragged it says where that slider is, as Winamp does
        let hint_id = id.with("hint");
        let hint: Option<String> = ui.ctx().data_mut(|d| d.remove_temp(hint_id));
        let title = match (&hint, &track) {
            (Some(h), _) => h.clone(),
            (None, Some(tr)) => {
                // the key and tempo ride along, once they are known
                let info = crate::meta::get(tr.id).unwrap_or_default();
                let mut extra = String::new();
                if info.key.is_some() {
                    extra += &format!("  *  KEY {}", crate::meta::key_text(&info));
                }
                if let Some(b) = info.bpm {
                    extra += &format!("  *  {} BPM", b);
                }
                format!("{} - {} ({}){}", tr.artists_text(), tr.title, fmt_time(tr.duration), extra)
            }
            (None, None) => "BACKLINE - A RETRO PLAYER FOR TIDAL".to_string(),
        };
        let marquee = pl.r(111.0, 24.0, 154.0, 6.0);
        let w = title.chars().count() as f32 * 5.0;
        let shift = if w > 154.0 && hint.is_none() && !crate::skin::calm() {
            ((ui.input(|i| i.time) as f32 * 22.0) % (w + 25.0)).floor()
        } else {
            0.0
        };
        let clip = p.with_clip_rect(marquee);
        // the title
        wa_text(&t, &clip, pl, 111.0 - shift, 24.0, &title);
        if w > 154.0 && shift > 0.0 {
            wa_text(&t, &clip, pl, 111.0 - shift + w + 25.0, 24.0, &title);
            // it moves one skin pixel at a time (22 a second): redrawn that often, not every frame
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(45));
        }
        // kbps, kHz, mono / stereo
        if active {
            wa_text(&t, &p, pl, 111.0, 43.0, &format!("{:>3}", self.kbps.min(999)));
            wa_text(&t, &p, pl, 156.0, 43.0, &format!("{:>2}", (rate / 1000).min(99)));
        }
        t.spr(&p, "monoster", (29.0, 12.0, 27.0, 12.0), pl.r(212.0, 41.0, 27.0, 12.0));
        t.spr(&p, "monoster", (0.0, if active { 0.0 } else { 12.0 }, 29.0, 12.0), pl.r(239.0, 41.0, 29.0, 12.0));
        // the visualizer, in the skin's own colours (2 = top of the bars ... 17 = bottom, 23 = peaks);
        // click it for bars / wave / both, right-click for all its settings
        let vz = hit(ui, "viz", pl.r(24.0, 43.0, 76.0, 16.0), false)
            .tip("Visualizer. Click for bars, wave, both or visuals; right-click for settings.");
        if vz.clicked() {
            self.viz[0].mode = (self.viz[0].mode + 1) % 4;
            self.dirty = true;
        }
        if vz.secondary_clicked() {
            self.viz_panel = Some(0);
        }
        let (bands, peaks, wave) = self.viz_data(0);
        if active && self.viz[0].mode == 3 {
            self.vis_paint(ui, pl.r(24.0, 43.0, 76.0, 16.0), 2);
        } else if active {
            skin_viz(&p, pl.o + Vec2::new(24.0, 43.0) * pl.s, pl.s, &t.vis, &bands, &peaks, &wave, self.viz[0].mode);
            // Winamp's own visualizer ran at about 30 frames a second; so does this one (half the work of every frame)
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        }
        // volume: the strip picture follows the level; right-click it for the sound output
        let vr = pl.r(107.0, 57.0, 68.0, 13.0);
        // as many level pictures as this skin has (usually 28), 15 pixels apart
        let frames = |sheet: &str| t.size(sheet).map_or(28.0, |s| ((s.y + 2.0) / 15.0).floor().clamp(1.0, 28.0));
        let frame = (self.volume * (frames("volume") - 1.0)).round();
        t.spr(&p, "volume", (0.0, frame * 15.0, 68.0, 13.0), vr);
        let vh = hit(ui, "volume", vr, true)
            .tip(format!("Volume {}%. Right-click for the sound output.", (self.volume * 100.0).round()));
        if t.size("volume").map_or(false, |s| s.y >= 433.0) {
            let src = if vh.dragged() { (0.0, 422.0, 14.0, 11.0) } else { (15.0, 422.0, 14.0, 11.0) };
            t.spr(&p, "volume", src, pl.r(107.0 + self.volume * 54.0, 58.0, 14.0, 11.0));
        }
        if vh.dragged() || vh.clicked() {
            if let Some(pp) = vh.interact_pointer_pos() {
                // the knob's middle follows the mouse, as in Winamp
                acts.push(Action::Volume(((pp.x - vr.min.x - pl.s * 7.0) / (pl.s * 54.0)).clamp(0.0, 1.0)));
            }
        }
        let mut say: Option<String> = None;
        if vh.hovered() || vh.dragged() {
            say = Some(format!("VOLUME: {}%", (self.volume * 100.0).round()));
        }
        vh.context_menu(|ui| self.output_menu(ui, acts));
        // balance: left / right; double-click to centre it again
        let br = pl.r(177.0, 57.0, 38.0, 13.0);
        // skins without a balance picture borrow the volume one, as Winamp does
        let bal = if t.size("balance").is_some() { "balance" } else { "volume" };
        if t.bal_is_vol && t.size(bal).is_some_and(|s| s.x >= 68.0) {
            // a volume-style strip (filled from the left): it follows the knob, the filled part ending under it,
            // rather than Winamp's left-right mirror, which only suits a picture drawn for it
            let at = (self.balance + 100) as f32 / 200.0;
            let f = (at * (frames(bal) - 1.0)).round();
            let edge = 7.0 + f / (frames(bal) - 1.0).max(1.0) * 54.0;
            let x = (edge - (7.0 + at * 24.0)).clamp(0.0, 30.0).round();
            t.spr(&p, bal, (x, f * 15.0, 38.0, 13.0), br);
        } else {
            let mut bframe = (self.balance.unsigned_abs() as f32 / 100.0 * (frames(bal) - 1.0)).round();
            // only the pictures of the smooth run (the nearest of them)
            if bal == "balance" {
                if let Some(k) = t.bal_frames.iter().min_by_key(|k| (**k as f32 - bframe).abs() as i32) {
                    bframe = *k as f32;
                }
            }
            // one set of pictures for both sides: to the left it is mirrored, so a picture that leans one way
            // (a row of hearts emptying from one end) leans the way you pushed it
            t.spr_flip(&p, bal, (9.0, bframe * 15.0, 38.0, 13.0), br, self.balance < 0);
        }
        let bh = hit(ui, "balance", br, true).tip(match self.balance {
            0 => "Balance: centre. Drag to change it, double-click to centre.".to_string(),
            b if b < 0 => format!("Balance: {}% left. Double-click to centre.", -b),
            b => format!("Balance: {}% right. Double-click to centre.", b),
        });
        if t.size(bal).map_or(false, |s| s.y >= 433.0) {
            let src = if bh.dragged() { (0.0, 422.0, 14.0, 11.0) } else { (15.0, 422.0, 14.0, 11.0) };
            t.spr(&p, bal, src, pl.r(177.0 + (self.balance + 100) as f32 / 200.0 * 24.0, 58.0, 14.0, 11.0));
        }
        if bh.hovered() || bh.dragged() {
            say = Some(match self.balance {
                0 => "BALANCE: CENTER".to_string(),
                b if b < 0 => format!("BALANCE: {}% LEFT", -b),
                b => format!("BALANCE: {}% RIGHT", b),
            });
        }
        if bh.double_clicked() {
            acts.push(Action::Balance(0));
        } else if bh.dragged() || bh.clicked() {
            if let Some(pp) = bh.interact_pointer_pos() {
                let f = ((pp.x - br.min.x - pl.s * 7.0) / (pl.s * 24.0)).clamp(0.0, 1.0);
                let b = (f * 200.0 - 100.0).round() as i32;
                // it clicks into the middle
                acts.push(Action::Balance(if b.abs() < 10 { 0 } else { b }));
            }
        }
        // EQ and playlist buttons
        let eq = pl.r(219.0, 58.0, 23.0, 12.0);
        let eqh = hit(ui, "eq", eq, false).tip("Equalizer");
        let eq_src = match (self.show_eq, eqh.is_pointer_button_down_on()) {
            (false, false) => (0.0, 61.0),
            (true, false) => (0.0, 73.0),
            (false, true) => (46.0, 61.0),
            (true, true) => (46.0, 73.0),
        };
        t.spr(&p, "shufrep", (eq_src.0, eq_src.1, 23.0, 12.0), eq);
        if eqh.clicked() {
            acts.push(Action::ToggleEq);
        }
        // PL: shows or hides the playlist (the queue)
        let plr = pl.r(242.0, 58.0, 23.0, 12.0);
        let plh = hit(ui, "pl", plr, false).tip(if self.queue_hidden { "Show the queue." } else { "Hide the queue." });
        let pl_src = match (!self.queue_hidden, plh.is_pointer_button_down_on()) {
            (false, false) => (23.0, 61.0),
            (true, false) => (23.0, 73.0),
            (false, true) => (69.0, 61.0),
            (true, true) => (69.0, 73.0),
        };
        t.spr(&p, "shufrep", (pl_src.0, pl_src.1, 23.0, 12.0), plr);
        // a skin whose EQ and PL buttons can't be seen: outlined and lettered
        if let Some(ink) = t.eqpl_ink {
            for (r, name, on) in [(eq, "EQ", self.show_eq), (plr, "PL", !self.queue_hidden)] {
                let r = r.shrink(pl.s * 0.5);
                p.rect_stroke(r, 0.0, egui::Stroke::new(pl.s.max(1.0), ink.gamma_multiply(if on { 1.0 } else { 0.6 })));
                p.text(r.center(), egui::Align2::CENTER_CENTER, name, egui::FontId::monospace(8.0 * pl.s), ink);
            }
        }
        if plh.clicked() {
            self.queue_hidden = !self.queue_hidden;
        }
        // the seek bar
        let pr = pl.r(16.0, 72.0, 248.0, 10.0);
        t.spr(&p, "posbar", (0.0, 0.0, 248.0, 10.0), pr);
        let ph = hit(ui, "seek", pr, true).tip("Drag or click to move in the song.");
        if active && dur > 0.0 {
            let f = (pos / dur).clamp(0.0, 1.0);
            let src = if ph.dragged() { (278.0, 0.0, 29.0, 10.0) } else { (248.0, 0.0, 29.0, 10.0) };
            let kr = pl.r(16.0 + f * 219.0, 72.0, 29.0, 10.0);
            if !t.spr(&p, "posbar", src, kr) {
                plate(&p, kr.shrink2(Vec2::new(pl.s, 2.0 * pl.s)), "", ph.dragged());
            }
            if ph.dragged() || ph.clicked() {
                if let Some(pp) = ph.interact_pointer_pos() {
                    self.seek_drag = Some(((pp.x - pr.min.x - 14.5 * pl.s) / (219.0 * pl.s)).clamp(0.0, 1.0) * dur);
                }
            }
            if ph.dragged() || ph.hovered() {
                let at = self.seek_drag.unwrap_or(pos);
                say = Some(format!("SEEK TO: {}/{} ({}%)", fmt_time(at), fmt_time(dur), (at / dur * 100.0).round()));
            }
            if ph.drag_stopped() || ph.clicked() {
                if let Some(s) = self.seek_drag.take() {
                    acts.push(Action::Seek(s));
                }
            }
        }
        if let Some(s) = say {
            ui.ctx().data_mut(|d| d.insert_temp(hint_id, s));
            ui.ctx().request_repaint();
        }
        // transport
        let buttons: [(&str, f32, f32, f32, Action, &str); 5] = [
            ("prev", 16.0, 0.0, 23.0, Action::Prev, "Previous"),
            ("play", 39.0, 23.0, 23.0, Action::PlayBtn, "Play  (Space)"),
            ("pause", 62.0, 46.0, 23.0, Action::PauseBtn, "Pause  (Space)"),
            ("stop", 85.0, 69.0, 23.0, Action::StopBtn, "Stop"),
            ("next", 108.0, 92.0, 22.0, Action::Next, "Next"),
        ];
        for (name, x, sx, w, a, tip) in buttons {
            let r = pl.r(x, 88.0, w, 18.0);
            let h = hit(ui, name, r, false).tip(tip);
            let down = if h.is_pointer_button_down_on() { 18.0 } else { 0.0 };
            if !t.spr(&p, "cbuttons", (sx, down, w, 18.0), r) {
                let glyph = match name {
                    "prev" => "|<",
                    "play" => ">",
                    "pause" => "||",
                    "stop" => "[]",
                    _ => ">|",
                };
                plate(&p, r.shrink(pl.s), glyph, down > 0.0);
            }
            if h.clicked() {
                acts.push(a);
            }
        }
        let ej = pl.r(136.0, 89.0, 22.0, 16.0);
        let ejh = hit(ui, "eject", ej, false).tip("Open files");
        if !t.spr(&p, "cbuttons", (114.0, if ejh.is_pointer_button_down_on() { 16.0 } else { 0.0 }, 22.0, 16.0), ej) {
            plate(&p, ej.shrink(pl.s), "^", ejh.is_pointer_button_down_on());
        }
        if ejh.clicked() {
            acts.push(Action::AddFiles);
        }
        // shuffle and repeat
        let sh = pl.r(164.0, 89.0, 47.0, 15.0);
        let shh = hit(ui, "shuffle", sh, false).tip("Shuffle");
        let sy = if self.shuffle { 30.0 } else { 0.0 } + if shh.is_pointer_button_down_on() { 15.0 } else { 0.0 };
        if !t.spr(&p, "shufrep", (28.0, sy, 47.0, 15.0), sh) {
            plate(&p, sh.shrink(pl.s), "SHUFFLE", self.shuffle);
        }
        if shh.clicked() {
            acts.push(Action::Shuffle);
        }
        let rp = pl.r(210.0, 89.0, 28.0, 15.0);
        let rph = hit(ui, "repeat", rp, false).tip(match self.repeat {
            Repeat::Off => "Repeat: off. Click to repeat all.",
            Repeat::All => "Repeat: all. Click to repeat this song.",
            _ => "Repeat: this song. Click to turn it off.",
        });
        let ry = if self.repeat != Repeat::Off { 30.0 } else { 0.0 } + if rph.is_pointer_button_down_on() { 15.0 } else { 0.0 };
        if !t.spr(&p, "shufrep", (0.0, ry, 28.0, 15.0), rp) {
            plate(&p, rp.shrink(pl.s), "REP", self.repeat != Repeat::Off);
        }
        // Winamp's button has only on / off: repeating one song gets a small "1" in the skin's own font, in the
        // button's corner, so every skin shows which it is
        if self.repeat == Repeat::One {
            p.rect_filled(pl.r(231.0, 90.0, 6.0, 7.0), 0.0, t.vis.first().copied().unwrap_or(Color32::BLACK));
            wa_text(&t, &p, pl, 232.0, 90.5, "1");
        }
        if rph.clicked() {
            acts.push(Action::Repeat);
        }
        // the size buttons' pictures, drawn over the skin's own (after the deck, so they show): one square, or two
        // overlapping squares while fullscreen; and the mini player's button, a little window in the corner
        {
            let p = ui.painter();
            let face = |r: Rect| {
                p.rect_filled(r, 0.0, Color32::from_gray(28));
                p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::from_gray(90)));
            };
            let ink = Stroke::new((pl.s * 0.8).max(1.0), Color32::from_gray(225));
            if !self.mini {
                let r = at(234.0);
                face(r);
                let m = r.width() * 0.2;
                let w = Vec2::new(r.width() * 0.4, r.height() * 0.3);
                p.rect_filled(Rect::from_min_size(r.max - Vec2::splat(m) - w, w), 0.0, Color32::from_gray(225));
            }
            let r = at(254.0);
            face(r);
            let m = r.width() * 0.2;
            if self.mini || !self.fullscreen {
                p.rect_stroke(r.shrink(m), 0.0, ink);
            } else {
                let w = r.width() * 0.45;
                let back = Rect::from_min_size(Pos2::new(r.max.x - m - w, r.min.y + m), Vec2::splat(w));
                let front = Rect::from_min_size(Pos2::new(r.min.x + m, r.max.y - m - w), Vec2::splat(w));
                p.rect_stroke(back, 0.0, ink);
                p.rect_filled(front, 0.0, Color32::from_gray(28));
                p.rect_stroke(front, 0.0, ink);
            }
        }
        // where the buttons are: outlined and named for a moment after a skin is put on, and while Alt is held over
        // the player - some skins hide them in their picture
        {
            let deck = Rect::from_min_size(pl.o, Vec2::new(275.0, 116.0) * pl.s);
            let (now, alt) = ui.input(|i| (i.time, i.modifiers.alt));
            let show = self.wa_guide || now < self.wa_map_until || (alt && ui.rect_contains_pointer(deck));
            let a = ui.ctx().animate_bool_with_time(id.with("button_map"), show, 0.3);
            if a > 0.0 {
                if now < self.wa_map_until {
                    ui.ctx().request_repaint();
                }
                let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, id.with("button_map_layer")));
                let spots: [(&str, (f32, f32, f32, f32)); 13] = [
                    ("PREV", (16.0, 88.0, 23.0, 18.0)),
                    ("PLAY", (39.0, 88.0, 23.0, 18.0)),
                    ("PAUSE", (62.0, 88.0, 23.0, 18.0)),
                    ("STOP", (85.0, 88.0, 23.0, 18.0)),
                    ("NEXT", (108.0, 88.0, 22.0, 18.0)),
                    ("OPEN", (136.0, 89.0, 22.0, 16.0)),
                    ("SHUFFLE", (164.0, 89.0, 47.0, 15.0)),
                    ("REPEAT", (210.0, 89.0, 28.0, 15.0)),
                    ("EQ", (219.0, 58.0, 23.0, 12.0)),
                    ("PL", (242.0, 58.0, 23.0, 12.0)),
                    ("VOLUME", (107.0, 57.0, 68.0, 13.0)),
                    ("BAL", (177.0, 57.0, 38.0, 13.0)),
                    ("SEEK", (16.0, 72.0, 248.0, 10.0)),
                ];
                let font = egui::FontId::monospace((5.5 * pl.s).max(9.0));
                for (name, (x, y, w, h)) in spots {
                    let r = pl.r(x, y, w, h);
                    // dark under light, so it shows on any skin
                    p.rect_stroke(r, 0.0, Stroke::new(3.0_f32, Color32::BLACK.gamma_multiply(0.6 * a)));
                    p.rect_stroke(r, 0.0, Stroke::new(1.5_f32, Color32::WHITE.gamma_multiply(0.9 * a)));
                    let g = p.layout_no_wrap(name.to_string(), font.clone(), Color32::WHITE.gamma_multiply(a));
                    let bg = Rect::from_center_size(r.center(), g.size() + Vec2::new(4.0, 0.0));
                    p.rect_filled(bg, 2.0, Color32::from_black_alpha((190.0 * a) as u8));
                    p.galley(bg.center() - g.size() / 2.0, g, Color32::WHITE);
                }
            }
        }
        // the top of the stack, rounded
        if !self.mini {
            crate::winamp_ui::round_corners(&p, pl.r(0.0, 0.0, 275.0, 116.0), true);
        }
        self.wa_art = Some(t);
    }

    /// The equalizer, drawn from the worn skin: ON, PRESETS, the curve and the ten sliders.
    pub(crate) fn winamp_eq(&mut self, ui: &mut egui::Ui, area: Rect) {
        let Some(t) = self.wa_art.take() else { return };
        let id = ui.id().with("winamp_eq");
        let pl = if self.mini {
            // the mini player's stack: right under the deck, at its width; its title bar moves the window
            let pl = Place { o: area.min, s: area.width() / 275.0 };
            if ui.interact(pl.r(0.0, 0.0, 275.0, 14.0), id.with("drag"), Sense::drag()).drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            pl
        } else {
            let (pl, z) = sized_place(ui, area, self.eq_zoom, id.with("size"), "the equalizer");
            if let Some(z) = z {
                self.eq_zoom = z;
                self.dirty = true;
            }
            pl
        };
        let p = ui.painter().clone();
        let mut changed = false;
        t.spr(&p, "eqmain", (0.0, 0.0, 275.0, 116.0), pl.r(0.0, 0.0, 275.0, 116.0));
        t.spr(&p, "eqmain", (0.0, 134.0, 275.0, 14.0), pl.r(0.0, 0.0, 275.0, 14.0));
        // ON
        let on_r = pl.r(14.0, 18.0, 26.0, 12.0);
        let on_h = ui
            .interact(on_r, id.with("on"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("Turn the equalizer on or off.");
        let on_src = if self.eq_on { (69.0, 119.0) } else { (10.0, 119.0) };
        if !t.spr(&p, "eqmain", (on_src.0, on_src.1, 26.0, 12.0), on_r) {
            plate(&p, on_r.shrink(pl.s), "ON", self.eq_on);
        }
        if on_h.clicked() {
            self.eq_on = !self.eq_on;
            changed = true;
        }
        // AUTO: remember the EQ for each song and bring it back when that song plays
        let au_r = pl.r(40.0, 18.0, 32.0, 12.0);
        let au = ui
            .interact(au_r, id.with("auto"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("AUTO remembers the EQ for each song and brings it back when it plays.");
        if !t.spr(&p, "eqmain", (if self.eq_auto { 95.0 } else { 36.0 }, 119.0, 32.0, 12.0), au_r) {
            plate(&p, au_r.shrink(pl.s), "AUTO", self.eq_auto);
        }
        if au.clicked() {
            self.eq_auto = !self.eq_auto;
            self.dirty = true;
        }
        // the title bar's close button closes the equalizer
        let cl = pl.r(264.0, 3.0, 9.0, 9.0);
        let clh = ui.interact(cl, id.with("close"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Close");
        if clh.is_pointer_button_down_on() {
            t.spr(&p, "eqmain", (0.0, 125.0, 9.0, 9.0), cl);
        }
        if clh.clicked() {
            self.show_eq = false;
        }
        // PRESETS: click for the list
        let pr_r = pl.r(217.0, 18.0, 44.0, 12.0);
        let pr_h =
            ui.interact(pr_r, id.with("presets"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Presets");
        if !t.spr(&p, "eqmain", (224.0, if pr_h.is_pointer_button_down_on() { 176.0 } else { 164.0 }, 44.0, 12.0), pr_r) {
            plate(&p, pr_r.shrink(pl.s), "PRESETS", pr_h.is_pointer_button_down_on());
        }
        let menu_id = id.with("presets_menu");
        if pr_h.clicked() {
            ui.memory_mut(|m| m.toggle_popup(menu_id));
        }
        egui::popup::popup_below_widget(ui, menu_id, &pr_h, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(140.0);
            // Backline's, then yours
            let all: Vec<(String, [f32; 10])> =
                EQ_PRESETS.iter().map(|(n, g)| (n.to_string(), *g)).chain(self.eq_mine.iter().cloned()).collect();
            for (name, g) in all {
                if menu_item(ui, &name) {
                    self.eq_gains = g;
                    self.eq_on = true;
                    changed = true;
                }
            }
        });
        // the curve, in the skin's own curve colours
        let has_graph = t.spr(&p, "eqmain", (0.0, 294.0, 113.0, 19.0), pl.r(86.0, 17.0, 113.0, 19.0));
        if !has_graph {
            p.rect_filled(pl.r(86.0, 17.0, 113.0, 19.0), 1.0, Color32::from_black_alpha(150));
        }
        for x in 0..112 {
            let f = x as f32 / 111.0 * 9.0;
            let i = (f.floor() as usize).min(8);
            let u = f - i as f32;
            let (a, b) = (self.eq_gains[i], self.eq_gains[i + 1]);
            let g = a + (b - a) * (u * u * (3.0 - 2.0 * u));
            let y = (26.0 - g / 12.0 * 9.0).round().clamp(17.0, 35.0);
            let c = if has_graph { t.graph.get((y - 17.0) as usize).copied() } else { None }.unwrap_or(pal().ink);
            p.rect_filled(pl.r(86.0 + x as f32, y, 1.0, 1.0), 0.0, c);
        }
        // the preamp slider (a plain gain before the bands) and the ten bands
        let band = |p: &egui::Painter, x: f32, value: f32, down: bool| {
            let n = (value / 100.0 * 27.0).round().clamp(0.0, 27.0) as i32;
            let (sx, sy) = ((n % 14) as f32 * 15.0, (n / 14) as f32 * 65.0);
            let ty = 38.0 + (1.0 - value / 100.0) * 51.0;
            if !t.spr(p, "eqmain", (13.0 + sx, 164.0 + sy, 14.0, 63.0), pl.r(x, 38.0, 14.0, 63.0)) {
                // no slider pictures in this skin: a plain groove, filled from the middle to the knob
                p.rect_filled(pl.r(x + 2.0, 38.0, 10.0, 63.0), 2.0, Color32::from_black_alpha(140));
                p.rect_filled(pl.r(x + 6.0, 41.0, 2.0, 57.0), 0.0, pal().ink.gamma_multiply(0.35));
                let (a, b) = (ty + 5.5, 69.5);
                p.rect_filled(pl.r(x + 6.0, a.min(b), 2.0, (a - b).abs()), 0.0, pal().ink2);
            }
            if !t.spr(p, "eqmain", (0.0, if down { 176.0 } else { 164.0 }, 11.0, 11.0), pl.r(x + 1.0, ty, 11.0, 11.0)) {
                plate(p, pl.r(x + 1.5, ty + 2.5, 11.0, 6.0), "", down);
            }
        };
        let pr = pl.r(21.0, 38.0, 14.0, 63.0);
        let ph = ui.interact(pr, id.with("preamp"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(
            format!("Preamp: {:+} dB. It sets the level before the bands. Double-click to reset.", self.eq_pre.round() as i32),
        );
        if ph.dragged() || ph.clicked() {
            if let Some(pp) = ph.interact_pointer_pos() {
                let f = ((pp.y - pr.min.y - 5.5 * pl.s) / (51.0 * pl.s)).clamp(0.0, 1.0);
                let mut g = ((12.0 - 24.0 * f) * 2.0).round() / 2.0;
                if g.abs() < 0.75 {
                    g = 0.0;
                }
                self.eq_pre = g;
                self.eq_on = true;
                changed = true;
            }
        }
        if ph.double_clicked() {
            self.eq_pre = 0.0;
            changed = true;
        }
        band(&p, 21.0, (self.eq_pre + 12.0) / 24.0 * 100.0, ph.dragged());
        for i in 0..10 {
            let x = 78.0 + i as f32 * 18.0;
            let r = pl.r(x, 38.0, 14.0, 63.0);
            const HZ: [&str; 10] = ["31", "62", "125", "250", "500", "1K", "2K", "4K", "8K", "16K"];
            let h = ui
                .interact(r, id.with(("band", i)), Sense::click_and_drag())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .tip(format!("{} Hz: {:+} dB. Double-click to reset.", HZ[i], self.eq_gains[i].round() as i32));
            if h.dragged() || h.clicked() {
                if let Some(pp) = h.interact_pointer_pos() {
                    let f = ((pp.y - r.min.y - 5.5 * pl.s) / (51.0 * pl.s)).clamp(0.0, 1.0);
                    let mut g = ((12.0 - 24.0 * f) * 2.0).round() / 2.0;
                    if g.abs() < 0.75 {
                        g = 0.0;
                    }
                    self.eq_gains[i] = g;
                    self.eq_on = true;
                    changed = true;
                }
            }
            if h.double_clicked() {
                self.eq_gains[i] = 0.0;
                changed = true;
            }
            band(&p, x, (self.eq_gains[i] + 12.0) / 24.0 * 100.0, h.dragged());
        }
        self.wa_art = Some(t);
        if changed {
            self.apply_eq();
            self.eq_remember();
            self.dirty = true;
        }
    }

    /// The sound-output button's pop-up (shared by the player and the album view).
    pub(crate) fn output_popup(&mut self, ui: &mut egui::Ui, r: &egui::Response, acts: &mut Vec<Action>) {
        let id = r.id.with("outputs");
        if r.clicked() {
            ui.memory_mut(|m| m.toggle_popup(id));
        }
        egui::popup::popup_below_widget(ui, id, r, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
            ui.set_min_width(300.0);
            self.output_menu(ui, acts);
        });
    }

    /// The sound outputs as a menu.
    pub(crate) fn output_menu(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "SOUND OUTPUT", pal().ink2);
        let mark = |on: bool| if on { "> " } else { "   " };
        if menu_item(ui, &format!("{}System default", mark(self.out_device.is_none()))) {
            acts.push(Action::SetDevice(None));
            ui.close_menu();
        }
        for name in self.out_list.clone() {
            let on = self.out_device.as_deref() == Some(name.as_str());
            if menu_item(ui, &format!("{}{}", mark(on), name)) {
                acts.push(Action::SetDevice(Some(name)));
                ui.close_menu();
            }
        }
    }
}

/// A skinned window (275 x 116) placed in `area` at the size you gave it - `zoom`, up to what shows; None: as large
/// as fits in whole pixels - with its corner to drag and ctrl + the mouse wheel over it to size it (double-click the
/// corner: as large as fits). Returns the place, and the new size when you changed it.
pub(crate) fn sized_place(
    ui: &mut egui::Ui,
    area: Rect,
    zoom: Option<f32>,
    id: egui::Id,
    what: &str,
) -> (Place, Option<Option<f32>>) {
    let shown = area.intersect(ui.clip_rect());
    let pl = match zoom {
        Some(z) => {
            let s = z.min((shown.width() / 275.0).min(shown.height() / 116.0)).max(0.5);
            let snap = |v: f32| (v * font::ppp()).round() / font::ppp();
            Place { o: Pos2::new(snap(shown.center().x - 137.5 * s), snap(shown.center().y - 58.0 * s)), s }
        }
        None => Place::fit(area),
    };
    let win = Rect::from_min_size(pl.o, Vec2::new(275.0, 116.0) * pl.s);
    let mut out = None;
    if ui.rect_contains_pointer(win) {
        let zd = ui.input(|i| if i.modifiers.command { i.zoom_delta() } else { 1.0 });
        if zd != 1.0 {
            out = Some(Some((pl.s * zd).clamp(0.5, 8.0)));
        }
    }
    let grip = Rect::from_min_size(win.max - Vec2::splat(12.0), Vec2::splat(12.0));
    let gh = ui
        .interact(grip, id, Sense::click_and_drag())
        .on_hover_cursor(egui::CursorIcon::ResizeNwSe)
        .tip(format!("Drag to size {}, or use ctrl + mouse wheel. Double-click to make it as large as fits.", what));
    if gh.double_clicked() {
        out = Some(None);
    } else if gh.dragged() {
        if let Some(pp) = gh.interact_pointer_pos() {
            let c = win.center();
            let s = ((pp.x - c.x) * 2.0 / 275.0).max((pp.y - c.y) * 2.0 / 116.0);
            out = Some(Some(s.clamp(0.5, 8.0)));
        }
    }
    if gh.hovered() || gh.dragged() {
        let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, id.with("mark")));
        for k in 0..3 {
            let d = 3.0 + k as f32 * 3.0;
            p.line_segment(
                [Pos2::new(grip.max.x - d, grip.max.y - 1.0), Pos2::new(grip.max.x - 1.0, grip.max.y - d)],
                Stroke::new(1.5_f32, pal().ink),
            );
        }
    }
    (pl, out)
}

impl App {
    /// Put the chosen playlist font in place (fetching a free one first if it is not here yet).
    pub(crate) fn apply_pl_font(&mut self) {
        let skin_font = self.wa_art.as_ref().and_then(|t| t.font.clone());
        // the skin's font comes from the list of installed fonts: if that is not read yet, read it in the
        // background and come back (the window never stalls on it)
        if !fonts_ready() && matches!(PL_FONTS.get(self.pl_font).map(|f| &f.1), Some(FontSrc::Skin | FontSrc::Sys(_))) {
            let (tx, ctx, choice) = (self.tx.clone(), self.ctx.clone(), self.pl_font);
            std::thread::spawn(move || {
                warm_fonts();
                let _ = tx.send(Msg::FontReady(choice, Ok(Vec::new())));
                ctx.request_repaint();
            });
            return;
        }
        // the skin's own font, when it names one of the free fonts (SLEEK asks for Silkscreen), comes from there
        let mut choice = self.pl_font;
        if matches!(PL_FONTS.get(choice).map(|f| &f.1), Some(FontSrc::Skin)) {
            if let Some(i) = skin_font
                .as_deref()
                .and_then(|n| PL_FONTS.iter().position(|f| matches!(f.1, FontSrc::Web(..)) && f.0.eq_ignore_ascii_case(n)))
            {
                choice = i;
            }
        }
        match font_now(choice, skin_font.as_deref()) {
            Some(b) => {
                // the same font, smooth, for everything outside the stack (kept for the run: a skin is chosen rarely)
                let leaked: &'static [u8] = Box::leak(b.clone().into_boxed_slice());
                self.ctx.set_fonts(crate::smooth_fonts(Some(leaked)));
                set_playlist_font(&self.ctx, Some(b), choice)
            }
            None if matches!(PL_FONTS.get(choice).map(|f| &f.1), Some(FontSrc::Web(..))) => {
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                self.set_note("FETCHING THE FONT...");
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::FontReady(choice, fetch_font(choice)));
                    ctx.request_repaint();
                });
            }
            None => set_playlist_font(&self.ctx, None, self.pl_font),
        }
    }

    /// AUTO on: keep the EQ for the song playing.
    pub(crate) fn eq_remember(&mut self) {
        if self.eq_auto {
            if let Some(id) = self.cur_track().map(|t| t.id).filter(|id| !self.is_demo(*id)) {
                self.eq_songs.insert(id, self.eq_gains);
            }
        }
    }

    /// The queue as a real Winamp playlist window, drawn from the worn skin: its frame with the title piece, the
    /// tracks in the playlist font and colours, its scrollbar, and the bottom bar with working buttons.
    pub(crate) fn winamp_playlist(&mut self, ui: &mut egui::Ui, outer: Rect, acts: &mut Vec<Action>) {
        let Some(t) = self.wa_art.take() else { return };
        let p = ui.painter().clone();
        let ppp = font::ppp();
        // the window's pieces and the text grow and shrink with the playlist (its width and its height; whole screen
        // pixels, so the skin stays crisp)
        // fluid (no steps), and gentle: the square root of the size change
        // the stack's size (the mini player's playlist: as wide as its deck)
        let k = if self.mini { (outer.width() / 275.0).max(0.5) } else { self.stack_k };
        let _ = ppp;
        let id = ui.id().with("winamp_playlist");
        if self.mini {
            // in the mini player's stack its title bar moves the window (its buttons, made after, win)
            let bar = Rect::from_min_size(outer.min, Vec2::new(outer.width(), 20.0 * k));
            if ui.interact(bar, id.with("drag"), Sense::drag()).drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
        }
        let piece = |src: (f32, f32, f32, f32), dst: Rect| {
            t.spr(&p, "pledit", src, dst);
        };
        let tiled = |src: (f32, f32, f32, f32), area: Rect, horizontal: bool| {
            let clip = p.with_clip_rect(area);
            let (tw, th) = (src.2 * k, src.3 * k);
            if horizontal {
                let mut x = area.min.x;
                while x < area.max.x {
                    t.spr(&clip, "pledit", src, Rect::from_min_size(Pos2::new(x, area.min.y), Vec2::new(tw, th)));
                    x += tw;
                }
            } else {
                let mut y = area.min.y;
                while y < area.max.y {
                    t.spr(&clip, "pledit", src, Rect::from_min_size(Pos2::new(area.min.x, y), Vec2::new(tw, th)));
                    y += th;
                }
            }
        };
        let (top, left, right, bottom) = (20.0 * k, 12.0 * k, 20.0 * k, 38.0 * k);
        let list = Rect::from_min_max(outer.min + Vec2::new(left, top), outer.max - Vec2::new(right, bottom));
        // the list's own background, in the skin's playlist colour
        p.rect_filled(list, 0.0, pal().lcd);

        // ---- the frame
        tiled((127.0, 0.0, 25.0, 20.0), Rect::from_min_max(outer.min, Pos2::new(outer.max.x, outer.min.y + top)), true);
        piece((0.0, 0.0, 25.0, 20.0), Rect::from_min_size(outer.min, Vec2::new(25.0, 20.0) * k));
        piece(
            (153.0, 0.0, 25.0, 20.0),
            Rect::from_min_size(Pos2::new(outer.max.x - 25.0 * k, outer.min.y), Vec2::new(25.0, 20.0) * k),
        );
        piece(
            (26.0, 0.0, 100.0, 20.0),
            Rect::from_center_size(Pos2::new(outer.center().x, outer.min.y + top / 2.0), Vec2::new(100.0, 20.0) * k),
        );
        tiled(
            (0.0, 42.0, 12.0, 29.0),
            Rect::from_min_max(Pos2::new(outer.min.x, list.min.y), Pos2::new(list.min.x, list.max.y)),
            false,
        );
        let rcol = Rect::from_min_max(Pos2::new(list.max.x, list.min.y), Pos2::new(outer.max.x, list.max.y));
        tiled((31.0, 42.0, 20.0, 29.0), rcol, false);
        let bot = Rect::from_min_max(Pos2::new(outer.min.x, list.max.y), outer.max);
        tiled((179.0, 0.0, 25.0, 38.0), bot, true);
        piece((0.0, 72.0, 125.0, 38.0), Rect::from_min_size(bot.min, Vec2::new(125.0, 38.0) * k));
        let br = Rect::from_min_size(Pos2::new(outer.max.x - 150.0 * k, bot.min.y), Vec2::new(150.0, 38.0) * k);
        piece((126.0, 72.0, 150.0, 38.0), br);

        // ---- the tracks
        let queue = self.queue.clone();
        let cur = self.cur;
        // the rows and their text stop growing at double size: a stack dragged wide keeps a readable list
        let tk = k.min(2.0);
        let row_h = 13.0 * tk;
        let rows = ((list.height() - 4.0) / row_h).floor().max(1.0) as usize;
        let max_top = queue.len().saturating_sub(rows) as f32;
        let mut first = self.wa_pl_top.clamp(0.0, max_top);
        if self.q_scroll {
            if let Some(c) = cur {
                first = (c as f32 - rows as f32 / 2.0).clamp(0.0, max_top);
            }
            self.q_scroll = false;
        }
        // the wheel anywhere over the list (the rows on top take the hover, so it is the pointer that counts)
        let _ = ui.interact(list, id.with("list"), Sense::hover());
        if ui.input(|i| i.pointer.hover_pos()).is_some_and(|pp| list.contains(pp)) {
            let wheel = ui.input(|i| i.smooth_scroll_delta.y);
            first = (first - wheel / row_h).clamp(0.0, max_top);
        }
        let digits = format!("{}", queue.len()).len();
        let clip = p.with_clip_rect(list);
        let mut rects: Vec<(usize, Rect)> = Vec::new();
        for (n, i) in (first.floor() as usize..queue.len()).take(rows + 1).enumerate() {
            let tr = &queue[i];
            let y = list.min.y + 2.0 + n as f32 * row_h - (first.fract() * row_h);
            let r = Rect::from_min_size(Pos2::new(list.min.x, y), Vec2::new(list.width(), row_h));
            if r.max.y > list.max.y + row_h {
                break;
            }
            let h = ui
                .interact(r.intersect(list), id.with(("row", i)), Sense::click_and_drag())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                // the song in full (a long title is cut short in the row)
                .tip(format!(
                    "{} - {}{}",
                    tr.artists_text(),
                    tr.title,
                    if tr.album.is_empty() { String::new() } else { format!("   ({})", tr.album) }
                ));
            rects.push((i, r));
            if h.drag_started() {
                self.q_drag = Some(i);
            }
            if h.hovered() {
                clip.rect_filled(r, 0.0, pal().row_sel);
            }
            // the song playing: a bar across the row, a stripe at its left and an arrow before its name
            let now = cur == Some(i);
            if now {
                clip.rect_filled(r, 0.0, pal().bar_txt.gamma_multiply(0.22));
                clip.rect_filled(Rect::from_min_size(r.min, Vec2::new(3.0 * tk, r.height())), 0.0, pal().bar_txt);
            }
            let col = if now { pal().bar_txt } else { pal().ink };
            let label = if now {
                format!("{:>w$}. > {} - {}", i + 1, tr.artists_text(), tr.title, w = digits)
            } else if self.queued(i) {
                format!("{:>w$}. QUEUED  {} - {}", i + 1, tr.artists_text(), tr.title, w = digits)
            } else {
                format!("{:>w$}. {} - {}", i + 1, tr.artists_text(), tr.title, w = digits)
            };
            let dur = fmt_time(tr.duration);
            let mut dw = pl_text(&clip, Pos2::new(r.max.x - 3.0 * tk, r.center().y), Align::Max, &dur, tk, 80.0 * tk, col);
            // a heart left of the length: like / unlike from the playlist
            let hr = Rect::from_min_size(
                Pos2::new(r.max.x - 4.0 - dw - 4.0 - crate::HEART_W, r.min.y),
                Vec2::new(crate::HEART_W, r.height()),
            );
            if list.contains_rect(hr) {
                if crate::draw_heart(ui, hr, self.liked.contains(&tr.id), h.hovered(), id.with(("heart", i))) {
                    acts.push(Action::ToggleLike(tr.clone()));
                }
                dw += crate::HEART_W + 4.0;
            }
            pl_text(&clip, Pos2::new(r.min.x + 3.0 * tk, r.center().y), Align::Min, &label, tk, r.width() - dw - 10.0 * tk, col);
            if h.clicked() {
                acts.push(Action::PlayIndex(i));
            }
            let tidal = self.ext_of(tr.id).is_none();
            let link = track_link(tr, self.ext_of(tr.id).as_ref());
            let tr = tr.clone();
            let liked = self.liked.contains(&tr.id);
            h.context_menu(|ui| {
                crate::player_win::queue_moves(ui, acts, i, cur);
                // the same as every song's menu: album / artist, My Tracks, tunes, playlists (Backline and Tidal)
                crate::song_items(ui, acts, &tr, liked, tidal);
                link_item(ui, acts, link.clone());
                if menu_item(ui, "Remove from the queue") {
                    acts.push(Action::Remove(i));
                    ui.close_menu();
                }
            });
        }
        // drag to reorder: a line where it will land, moved on release
        if let Some(from) = self.q_drag {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let released = ui.input(|i| i.pointer.any_released());
            if let Some(pp) = ui.input(|i| i.pointer.hover_pos()) {
                let ins = rects
                    .iter()
                    .find(|(_, rc)| pp.y < rc.center().y)
                    .map(|(i, rc)| (*i, rc.min.y))
                    .or_else(|| rects.last().map(|(i, rc)| (*i + 1, rc.max.y)));
                if let Some((ins_i, y)) = ins {
                    clip.rect_filled(
                        Rect::from_min_size(Pos2::new(list.min.x, y - 1.0), Vec2::new(list.width(), 2.0)),
                        0.0,
                        pal().bar_txt,
                    );
                    if released {
                        acts.push(Action::MoveQueue(from, ins_i));
                    }
                }
            }
            if released || !ui.input(|i| i.pointer.any_down()) {
                self.q_drag = None;
            }
        }
        // the scrollbar: the skin's handle, riding the right edge
        let track = Rect::from_min_max(Pos2::new(rcol.min.x + 5.0 * k, rcol.min.y), Pos2::new(rcol.min.x + 13.0 * k, rcol.max.y));
        let hh = 18.0 * k;
        let f = if max_top > 0.0 { first / max_top } else { 0.0 };
        let handle = Rect::from_min_size(Pos2::new(track.min.x, track.min.y + f * (track.height() - hh)), Vec2::new(8.0 * k, hh));
        let sh = ui.interact(track, id.with("scroll"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand);
        piece(if sh.dragged() { (61.0, 53.0, 8.0, 18.0) } else { (52.0, 53.0, 8.0, 18.0) }, handle);
        if sh.dragged() || sh.clicked() {
            if let Some(pp) = sh.interact_pointer_pos() {
                first = ((pp.y - track.min.y - hh / 2.0) / (track.height() - hh)).clamp(0.0, 1.0) * max_top;
            }
        }
        self.wa_pl_top = first;

        // ---- the bottom bar: ADD and REM work; LIST opens the lead sheet
        let bl = |x: f32| Rect::from_min_size(bot.min + Vec2::new(x, 8.0) * k, Vec2::new(22.0, 18.0) * k);
        let add = ui
            .interact(bl(14.0), id.with("add"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("Add files.");
        if add.clicked() {
            acts.push(Action::AddFiles);
        }
        let rem = ui
            .interact(bl(43.0), id.with("rem"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("Clear the queue.");
        if rem.clicked() {
            acts.push(Action::ClearQueue);
        }
        let sel = ui
            .interact(bl(72.0), id.with("sel"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("Shuffle the queue.");
        if sel.clicked() {
            acts.push(Action::QueueOp(0));
        }
        // MISC: playlists (save the queue as one, a new one, open one), sort the queue, and the playlist font
        let misc = ui
            .interact(bl(101.0), id.with("misc"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("Playlists, sorting and the playlist font.");
        let misc_menu = id.with("misc_menu");
        if misc.clicked() {
            ui.memory_mut(|m| m.toggle_popup(misc_menu));
        }
        egui::popup::popup_below_widget(ui, misc_menu, &misc, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
            ui.set_min_width(220.0);
            let before = acts.len();
            para(ui, "PLAYLISTS", pal().ink2);
            if menu_item(ui, "Save the queue as a playlist") {
                acts.push(Action::PlaylistFromQueue);
            }
            if menu_item(ui, "New playlist...") {
                acts.push(Action::PlaylistNew(None));
            }
            for (i, pl) in self.store.playlists.iter().enumerate().take(12) {
                if menu_item(ui, &format!("Open  {}", pl.name)) {
                    acts.push(Action::Section(crate::Sec::Lists));
                    acts.push(Action::PlaylistOpen(Some(i)));
                }
            }
            ui.add_space(4.0);
            para(ui, "QUEUE", pal().ink2);
            for (name, op) in [("Sort by artist", 1u8), ("Sort by title", 2), ("Reverse", 3)] {
                if menu_item(ui, name) {
                    acts.push(Action::QueueOp(op));
                }
            }
            ui.add_space(4.0);
            para(ui, "PLAYLIST FONT", pal().ink2);
            for (i, (name, _)) in PL_FONTS.iter().enumerate() {
                if menu_item(ui, &format!("{}{}", if self.pl_font == i { "> " } else { "   " }, name)) {
                    acts.push(Action::PlFont(i));
                }
            }
            if acts.len() != before {
                ui.memory_mut(|m| m.close_popup());
            }
        });
        // the title bar's close (and shade) hide the queue
        for (name, x) in [("pl_close", 11.0), ("pl_shade", 21.0)] {
            let r = Rect::from_min_size(Pos2::new(outer.max.x - x * k, outer.min.y + 3.0 * k), Vec2::new(9.0, 9.0) * k);
            if ui
                .interact(r, id.with(name), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .tip("Hide the queue.")
                .clicked()
            {
                self.queue_hidden = true;
            }
        }
        // a small visualizer in the skin's own panel for it, when the bar is wide enough
        if bot.width() >= (125.0 + 75.0 + 150.0) * k {
            let vr = Rect::from_min_size(Pos2::new(br.min.x - 75.0 * k, bot.min.y), Vec2::new(75.0, 38.0) * k);
            piece((205.0, 0.0, 75.0, 38.0), vr);
            let (bands, peaks, wave) = self.viz_data(0);
            let inner = Rect::from_min_size(vr.min + Vec2::new(3.0, 12.0) * k, Vec2::new(72.0, 16.0) * k);
            if self.cur.is_some() && !self.stopped && self.viz[0].mode == 3 {
                self.vis_paint(ui, inner, 3);
            } else if self.cur.is_some() && !self.stopped {
                // the same style as the player's (bars, wave or both), in the skin's colours
                skin_viz(&p.with_clip_rect(inner), inner.min, k, &t.vis, &bands, &peaks, &wave, self.viz[0].mode);
            }
        }
        let list_btn = Rect::from_min_size(br.min + Vec2::new(106.0, 8.0) * k, Vec2::new(22.0, 18.0) * k);
        let lb = ui
            .interact(list_btn, id.with("list_btn"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("The lead sheet");
        if lb.clicked() {
            self.rtab = 1;
        }
        // running time (where we are / the whole queue) and the mini clock, in the skin's font
        let total: f32 = queue.iter().map(|t| t.duration).sum();
        let pos = self.pos();
        let text_at = |x: f32, y: f32, s: &str| {
            for (n, ch) in s.chars().enumerate() {
                let (row, col) = glyph(ch);
                t.spr(
                    &p,
                    "text",
                    (col as f32 * 5.0, row as f32 * 6.0, 5.0, 6.0),
                    Rect::from_min_size(br.min + Vec2::new(x + n as f32 * 5.0, y) * k, Vec2::new(5.0, 6.0) * k),
                );
            }
        };
        if self.cur.is_some() {
            text_at(7.0, 10.0, &format!("{}/{}", fmt_time(pos), fmt_long(total)));
            text_at(66.0, 23.0, &format!("{:>5}", fmt_time(pos)));
        }
        // the mini transport
        let acts_row = [Action::Prev, Action::PlayBtn, Action::PauseBtn, Action::StopBtn, Action::Next, Action::AddFiles];
        const MINI_TIPS: [&str; 6] = ["Previous", "Play", "Pause", "Stop", "Next", "Open files"];
        for (n, a) in acts_row.into_iter().enumerate() {
            let r = Rect::from_min_size(br.min + Vec2::new(3.0 + n as f32 * 10.0, 22.0) * k, Vec2::new(10.0, 10.0) * k);
            if ui
                .interact(r, id.with(("mini", n)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .tip(MINI_TIPS[n])
                .clicked()
            {
                acts.push(a);
            }
        }
        // the foot of the stack, rounded
        crate::winamp_ui::round_corners(&p, outer, false);
        self.wa_art = Some(t);
    }
}
