//! SONGS: find a song (search YouTube, or paste a link: Share > Copy link in the YouTube app, and the app offers it
//! as you come back), then your songs, newest first. Tap one to practise it.

use crate::*;

/// A text box as the desktop draws one in Studio: sunk into the screen, the typewriter face.
pub(crate) fn text_box(ui: &mut egui::Ui, text: &mut String, w: f32, hint: &str, id: &str) -> egui::Response {
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
    inset(ui.painter(), r, pal().display);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(Vec2::new(6.0, 2.0))).layout(egui::Layout::left_to_right(Align::Center)));
    child.visuals_mut().override_text_color = Some(pal().ink);
    child.visuals_mut().extreme_bg_color = pal().display;
    child.visuals_mut().selection.bg_fill = pal().row_sel;
    child.visuals_mut().text_cursor.stroke = Stroke::new(2.0_f32, pal().ink);
    child.add(
        egui::TextEdit::singleline(text)
            .id_salt(id)
            .frame(false)
            .hint_text(egui::RichText::new(hint).color(pal().dim))
            .desired_width(r.width() - 12.0)
            .font(egui::FontId::proportional(15.5)),
    )
}

impl App {
    pub(crate) fn songs_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
        // ---- search or paste a link
        ui.horizontal(|ui| {
            let w = ui.available_width() - 58.0 - 70.0 - 12.0;
            let r = text_box(ui, &mut self.query, w, "Search YouTube", "q");
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if retro_btn_w(ui, if self.searching { "..." } else { "GO" }, 58.0, false).clicked() || enter {
                ui.memory_mut(|m| m.surrender_focus(r.id));
                self.go();
            }
            if retro_btn_w(ui, "PASTE", 70.0, false).clicked() {
                match clipboard() {
                    Some(t) if video_id(&t).is_some() => {
                        self.clip_seen = video_id(&t).unwrap_or_default();
                        self.clip_offer = None;
                        self.query = t;
                        self.go();
                    }
                    Some(t) if !t.trim().is_empty() => self.query = t.trim().to_string(),
                    _ => self.set_note("Nothing to paste. In YouTube, tap Share, then Copy link."),
                }
            }
        });
        // ---- a link copied in YouTube, offered
        if let Some(vid) = self.clip_offer.clone() {
            ui.add_space(2.0);
            centered_row(ui, "clip", |ui| {
                label(ui, "ADD THE YOUTUBE LINK YOU COPIED?", 0.0);
                if retro_btn_w(ui, "ADD IT", 70.0, true).clicked() {
                    self.clip_offer = None;
                    self.add_link(vid.clone());
                }
                if retro_btn_w(ui, "NO", 44.0, false).clicked() {
                    self.clip_offer = None;
                }
            });
        }
        ui.add_space(6.0);
        egui::ScrollArea::vertical().id_salt("songs_scroll").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 2.0);
            // ---- a search's results
            if !self.results.is_empty() {
                // (what was searched stays in the box above)
                ui.horizontal(|ui| ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if retro_btn(ui, "CLEAR", false).clicked() {
                        self.results.clear();
                        self.query.clear();
                    }
                    heading(ui, "RESULTS", ui.available_width());
                }));
                let res = self.results.clone();
                for s in res {
                    let hit = self.song_row(ui, &s, false, "", false, true);
                    if hit.save {
                        self.save_song(&s);
                    } else if hit.tap {
                        self.results.clear();
                        self.query.clear();
                        self.add_and_play(s);
                        return;
                    }
                }
                ui.add_space(12.0);
            }
            // ---- your songs (and the way to the music already on the phone)
            ui.horizontal(|ui| ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if retro_btn(ui, "MY FILES", false).clicked() {
                    self.view = 3;
                    self.mine_tab = 2;
                    self.mine_filter.clear();
                    self.rescan_files();
                    self.save_settings();
                }
                heading(ui, "YOUR SONGS", ui.available_width());
            }));
            if self.songs.is_empty() {
                ui.add_space(4.0);
                for line in [
                    "Search for a song above, or copy a link in YouTube (Share, then Copy link) and come back.",
                    "Music already on your phone: MY FILES, then pick a folder.",
                    "Then loop the hard part, slow it down, change its pitch, and bookmark the spots.",
                ] {
                    for l in wrap(line, 1.3, ui.available_width() - 8.0) {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
                        ptext(ui.painter(), Pos2::new(r.min.x + 2.0, r.center().y), Align::Min, &l, 1.3, pal().ink2);
                    }
                    ui.add_space(6.0);
                }
                return;
            }
            let list = self.songs.clone();
            let cur = self.cur.as_ref().map(|c| c.vid.clone());
            for s in list {
                let asked = self.del_ask.as_deref() == Some(s.vid.as_str());
                let hit = self.song_row(ui, &s, cur.as_deref() == Some(s.vid.as_str()), if asked { "ASK" } else { "X" }, true, true);
                let (tap, x) = (hit.tap, hit.side);
                if hit.heart {
                    self.toggle_like(&s);
                    continue;
                }
                if hit.save {
                    self.save_song(&s);
                    continue;
                }
                if x {
                    if asked {
                        self.remove_song(&s.vid);
                        self.del_ask = None;
                        return;
                    }
                    self.del_ask = Some(s.vid.clone());
                } else if tap {
                    if asked {
                        self.del_ask = None;
                        continue;
                    }
                    self.del_ask = None;
                    self.view = 1;
                    self.play_list = self.songs.clone();
                    self.load(s, true);
                    self.save_settings();
                }
            }
            ui.add_space(12.0);
        });
    }

    /// A song as a row: its picture, its name, who and how long. Along its end: `heart` a key to like it; `save` (a
    /// YouTube song) its SAVE key (how far while it saves, SAVED once it's on the phone); `side` "X" a remove key
    /// ("ASK" asks first), other words a dim note. Returns what was tapped.
    pub(crate) fn song_row(&mut self, ui: &mut egui::Ui, s: &Song, playing: bool, side: &str, heart: bool, save: bool) -> RowHit {
        let w = ui.available_width();
        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 46.0), Sense::click());
        let p = ui.painter().clone();
        if playing {
            p.rect_filled(r, 0.0, pal().row_sel);
        } else if resp.is_pointer_button_down_on() {
            p.rect_filled(r, 0.0, pal().row_alt);
        }
        let th = Rect::from_min_size(Pos2::new(r.min.x + 2.0, r.min.y + 5.0), Vec2::splat(36.0));
        match self.thumb(s) {
            Some(t) => {
                p.image(t, th, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            }
            None => {
                p.rect_filled(th, 0.0, studio::ghost());
                // (a phone track with no art: a note)
                if crate::device::is_file(&s.vid) {
                    crate::vicon::named(&p, th.shrink(9.0), "note", studio::dim());
                }
            }
        }
        p.rect_stroke(th, 0.0, Stroke::new(1.0_f32, pal().ink));
        // ---- the keys along the end, from the right
        let mut x = r.max.x - 2.0;
        let (mut removed, mut hearted, mut saved_hit) = (false, false, false);
        match side {
            "X" | "ASK" => {
                let on = side == "ASK";
                let (kr, h) = row_key(ui, r, &mut x, if on { 88.0 } else { 36.0 }, ("song_x", &s.vid));
                studio::key(&p, kr, on, h.is_pointer_button_down_on(), false);
                if on {
                    ptext(&p, kr.center(), Align::Center, "REMOVE", 1.1, studio::key_fg(true));
                } else {
                    crate::vicon::named(&p, Rect::from_center_size(kr.center(), Vec2::splat(12.0)), "close", studio::key_fg(false));
                }
                removed = h.clicked();
            }
            "" => {}
            t => {
                let tw = ptext(&p, Pos2::new(x - 4.0, r.center().y), Align::Max, t, 1.0, pal().dim);
                x -= tw + 10.0;
            }
        }
        if save && !crate::device::is_file(&s.vid) {
            // SAVE: on the phone next; how far while it comes; SAVED (lit) once it's here
            let mine = self.cur.as_ref().is_some_and(|c| c.vid == s.vid);
            let (label, lit) = if saved(&s.vid) {
                ("SAVED".to_string(), true)
            } else if self.saving.as_deref() == Some(s.vid.as_str()) {
                (pct_word(self.save_pct.load(Ordering::Relaxed)), false)
            } else if mine && self.loading && self.keep {
                (pct_word(self.dl_pct.load(Ordering::Relaxed)), false)
            } else if self.save_queue.contains(&s.vid) {
                ("...".to_string(), false)
            } else {
                ("SAVE".to_string(), false)
            };
            let (kr, h) = row_key(ui, r, &mut x, 56.0, ("song_save", &s.vid));
            studio::key(&p, kr, lit, h.is_pointer_button_down_on(), false);
            ptext(&p, kr.center(), Align::Center, &label, 1.05, studio::key_fg(lit));
            saved_hit = h.clicked();
        }
        if heart {
            // the heart: a key, lit when you like the song
            let (kr, h) = row_key(ui, r, &mut x, 36.0, ("song_heart", &s.vid));
            let on = self.is_liked(&s.vid);
            studio::key(&p, kr, on, h.is_pointer_button_down_on(), false);
            crate::vicon::named(&p, Rect::from_center_size(kr.center(), Vec2::splat(13.0)), "heart", studio::key_fg(on));
            hearted = h.clicked();
        }
        // ---- the name, and who and how long under it (as YouTube writes them, cut with ... to fit, never squeezed)
        let tx = th.max.x + 10.0;
        let tw = x - tx - 2.0;
        let (who, name) = parts(s);
        font::keep_caps(|| {
            ptext(&p, Pos2::new(tx, r.min.y + 15.0), Align::Min, &fit(&name, 1.35, tw), 1.35, pal().ink);
        });
        // (a length not known yet, as a phone file Android hasn't looked inside: left off)
        let len = if s.secs > 0.5 { fmt_time(s.secs) } else { String::new() };
        let sub = [who, len].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
        font::keep_caps(|| {
            ptext(&p, Pos2::new(tx, r.min.y + 33.0), Align::Min, &fit(&sub, 1.05, tw), 1.05, pal().ink2);
        });
        // (a fine line between rows)
        p.line_segment([Pos2::new(tx, r.max.y), Pos2::new(r.max.x, r.max.y)], Stroke::new(1.0_f32, pal().row_alt));
        RowHit { tap: resp.clicked() && !removed && !hearted && !saved_hit, side: removed, heart: hearted, save: saved_hit }
    }

    /// A song off your list, with its kept audio and practice (asked first).
    pub(crate) fn remove_song(&mut self, vid: &str) {
        self.songs.retain(|s| s.vid != vid);
        save_json("songs.json", &self.songs);
        self.kept.remove(vid);
        self.save_kept();
        let _ = std::fs::remove_file(audio_file(vid));
        if self.cur.as_ref().is_some_and(|c| c.vid == vid) {
            self.stop();
            self.cur = None;
            self.wave.clear();
            self.audio = None;
            self.dur = 0.0;
        }
    }
}

/// What was tapped on a song's row: the row, the key at its end (remove), the heart, SAVE.
pub(crate) struct RowHit {
    pub tap: bool,
    pub side: bool,
    pub heart: bool,
    pub save: bool,
}

/// How far a song's saving is, on its key ("Saving" while yt-dlp gets ready, before the download starts).
fn pct_word(p: u32) -> String {
    if p == 0 {
        "Saving".to_string()
    } else {
        format!("{}%", p.min(99))
    }
}

/// A key along a row's end, `w` wide, left of `x` (which moves on past it).
fn row_key(ui: &mut egui::Ui, r: Rect, x: &mut f32, w: f32, id: impl std::hash::Hash) -> (Rect, egui::Response) {
    let kr = Rect::from_min_size(Pos2::new(*x - w, r.center().y - 14.0), Vec2::new(w, 28.0));
    *x -= w + 6.0;
    (kr, ui.interact(kr, ui.id().with(id), Sense::click()))
}

/// A row as a song's is drawn: a picture (or an icon in its place), a name, a dim line under it, ">" at the end.
pub(crate) fn list_row(ui: &mut egui::Ui, pic: Option<egui::TextureId>, icon: &str, title: &str, sub: &str, on: bool) -> egui::Response {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 46.0), Sense::click());
    let p = ui.painter().clone();
    if on {
        p.rect_filled(r, 0.0, pal().row_sel);
    } else if resp.is_pointer_button_down_on() {
        p.rect_filled(r, 0.0, pal().row_alt);
    }
    let th = Rect::from_min_size(Pos2::new(r.min.x + 2.0, r.min.y + 5.0), Vec2::splat(36.0));
    match pic {
        Some(t) => {
            p.image(t, th, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        None => {
            p.rect_filled(th, 0.0, studio::ghost());
            crate::vicon::named(&p, th.shrink(8.0), icon, studio::ink());
        }
    }
    p.rect_stroke(th, 0.0, Stroke::new(1.0_f32, pal().ink));
    let tx = th.max.x + 10.0;
    let tw = r.max.x - tx - 26.0;
    font::keep_caps(|| {
        ptext(&p, Pos2::new(tx, r.min.y + 15.0), Align::Min, &fit(title, 1.35, tw), 1.35, pal().ink);
        ptext(&p, Pos2::new(tx, r.min.y + 33.0), Align::Min, &fit(sub, 1.05, tw), 1.05, pal().ink2);
    });
    ptext(&p, Pos2::new(r.max.x - 6.0, r.center().y), Align::Max, ">", 1.5, pal().ink2);
    p.line_segment([Pos2::new(tx, r.max.y), Pos2::new(r.max.x, r.max.y)], Stroke::new(1.0_f32, pal().row_alt));
    resp
}

/// A heading in the window: small spaced capitals with an underline (the desktop's headings with a skin worn).
pub(crate) fn heading(ui: &mut egui::Ui, text: &str, w: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(w.max(40.0), 26.0), Sense::hover());
    let tw = spaced_caps(ui.painter(), Pos2::new(r.min.x + 2.0, r.center().y), text, 1.1, pal().ink);
    ui.painter().line_segment([Pos2::new(r.min.x + 2.0, r.max.y - 3.0), Pos2::new(r.min.x + 2.0 + tw.min(w), r.max.y - 3.0)], Stroke::new(2.0_f32, pal().ink));
}
