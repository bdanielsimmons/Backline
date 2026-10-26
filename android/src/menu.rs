//! The song menu (a song held or its three dots): play it now or next, queue it, move it, like it, put it on a
//! playlist (yours on the phone, or a Tidal one), go to its album or artist or a radio of it, its credits, copy its
//! name or link. Also the songs kept between runs: the queue (to carry on where you left off), what you played
//! lately, and your own playlists.

use crate::*;

/// A song as kept in the phone's files.
#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub(crate) struct QTrack {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub artists: Vec<(i64, String)>,
    pub album: String,
    pub cover: String,
    pub duration: f32,
}

impl QTrack {
    pub(crate) fn of(t: &Track) -> QTrack {
        QTrack {
            id: t.id,
            title: t.title.clone(),
            artist: t.artist.clone(),
            artists: t.artists.clone(),
            album: t.album.clone(),
            cover: t.cover.clone(),
            duration: t.duration,
        }
    }

    pub(crate) fn track(&self) -> Track {
        Track {
            id: self.id,
            title: self.title.clone(),
            artist: self.artist.clone(),
            artists: self.artists.clone(),
            album: self.album.clone(),
            cover: self.cover.clone(),
            duration: self.duration,
            ..Default::default()
        }
    }
}

/// One of your playlists, kept on the phone.
#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub(crate) struct Playlist {
    pub name: String,
    pub tracks: Vec<QTrack>,
}

/// Where you were: the queue, the song in it, and how far in.
#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub(crate) struct Session {
    pub queue: Vec<QTrack>,
    pub cur: Option<usize>,
    pub pos: f32,
    pub recent: Vec<QTrack>,
}

/// Put text on the phone's clipboard.
pub(crate) fn set_clipboard(text: &str) -> bool {
    #[cfg(target_os = "android")]
    {
        use jni::objects::JObject;
        let run = || -> Option<()> {
            let ctx = ndk_context::android_context();
            let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
            let mut env = vm.attach_current_thread().ok()?;
            let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
            let name = env.new_string("clipboard").ok()?;
            let cm = env.call_method(&activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&name).into()]).ok()?.l().ok()?;
            let label = env.new_string("Backline").ok()?;
            let body = env.new_string(text).ok()?;
            let clip = env
                .call_static_method(
                    "android/content/ClipData",
                    "newPlainText",
                    "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                    &[(&label).into(), (&body).into()],
                )
                .ok()?
                .l()
                .ok()?;
            env.call_method(&cm, "setPrimaryClip", "(Landroid/content/ClipData;)V", &[(&clip).into()]).ok()?;
            Some(())
        };
        return run().is_some();
    }
    #[allow(unreachable_code)]
    {
        let _ = text;
        false
    }
}

impl App {
    /// Keep where you are (the queue, the song, how far in) and what you played lately.
    pub(crate) fn save_session(&self) {
        let s = Session {
            queue: self.queue.iter().map(QTrack::of).collect(),
            cur: self.cur,
            pos: self.pos(),
            recent: self.recent.iter().map(QTrack::of).collect(),
        };
        save_json("where.json", &s);
    }

    /// Back to where you were at the last run: the queue and the song in it, paused where you left it.
    pub(crate) fn load_session(&mut self) {
        let s: Session = load_json("where.json");
        self.recent = s.recent.iter().map(QTrack::track).collect();
        if s.queue.is_empty() {
            return;
        }
        self.queue = s.queue.iter().map(QTrack::track).collect();
        self.cur = s.cur.filter(|c| *c < self.queue.len());
        self.stopped = true;
        self.resume_at = (s.pos > 1.0).then_some(s.pos);
    }

    pub(crate) fn save_playlists(&self) {
        save_json("playlists.json", &self.playlists);
    }

    /// The menu of a song held down (the desktop's right-click), sliding up from the bottom.
    pub(crate) fn song_menu_ui(&mut self, ctx: &egui::Context) {
        let Some((t, qi)) = self.song_menu.clone() else { return };
        let screen = ctx.screen_rect();
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("menu_dim")))
            .rect_filled(screen, 0.0, Color32::from_black_alpha(150));
        let mut close = false;
        let k = ctx.animate_bool_with_time(egui::Id::new("song_menu_in"), true, 0.18);
        let drop_px = (1.0 - k) * 320.0;
        if ctx.input(|i| i.pointer.any_click()) && !ctx.is_pointer_over_area() {
            close = true;
        }
        let tidal = t.id > 0 && !self.files.iter().any(|f| f.0.id == t.id) && !self.yt.iter().any(|y| yt_id(&y.vid) == t.id);
        egui::Area::new(egui::Id::new("song_menu")).order(egui::Order::Tooltip).anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, drop_px)).show(ctx, |ui| {
            egui::Frame::none().fill(pal().beige).rounding(egui::Rounding { nw: 16.0, ne: 16.0, sw: 0.0, se: 0.0 }).inner_margin(egui::Margin { left: 14.0, right: 14.0, top: 14.0, bottom: 34.0 }).show(ui, |ui| {
                ui.set_width(screen.width() - 28.0);
                ui.set_max_height(screen.height() * 0.8);
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                title_line(ui, &fit(&t.title, 2.0, screen.width() - 80.0), 2.0, pal().ink);
                dim(ui, &t.artists_text(), 1.0, pal().ink2);
                let w = ui.available_width();
                egui::ScrollArea::vertical().id_salt("menu_scroll").max_height(screen.height() * 0.65).show(ui, |ui| {
                    match self.menu_mode {
                        1 => close |= self.menu_playlists(ui, &t, tidal, w),
                        3 => close |= self.menu_edit(ui, &t, w),
                        2 => {
                            if self.credits.is_empty() {
                                dim(ui, "Loading the credits...", 1.0, pal().ink2);
                            }
                            for (role, name) in self.credits.clone() {
                                dim(ui, &role, 1.0, pal().ink2);
                                dim(ui, &name, 1.4, pal().ink);
                            }
                            if pill_w(ui, "Back", w, false).clicked() {
                                self.menu_mode = 0;
                            }
                        }
                        _ => close |= self.menu_main(ui, &t, qi, tidal, w),
                    }
                });
            });
        });
        if close {
            self.song_menu = None;
            self.menu_mode = 0;
            self.menu_pl = None;
            ctx.animate_bool_with_time(egui::Id::new("song_menu_in"), false, 0.0);
        }
    }

    /// The menu's first page: a line icon and a word per row. Returns true when it should close.
    fn menu_main(&mut self, ui: &mut egui::Ui, t: &Track, qi: Option<usize>, tidal: bool, w: f32) -> bool {
        let mut close = false;
        let cur = self.cur;
        if mrow(ui, w, 0, "Play now") {
            match qi {
                Some(i) => self.play(i),
                None => {
                    let at = cur.map_or(0, |c| c + 1).min(self.queue.len());
                    self.queue.insert(at, t.clone());
                    self.play(at);
                }
            }
            close = true;
        }
        if mrow(ui, w, 1, "Play next") {
            let at = cur.map_or(0, |c| c + 1).min(self.queue.len());
            if let Some(i) = qi {
                let s = self.queue.remove(i);
                let at = if i < at { at - 1 } else { at };
                self.queue.insert(at, s);
                self.fix_cur(i, at);
            } else {
                self.queue.insert(at, t.clone());
            }
            self.set_note("Plays next");
            close = true;
        }
        if qi.is_none() && mrow(ui, w, 2, "Add to the queue") {
            self.queue.push(t.clone());
            self.set_note("Added to the queue");
            close = true;
        }
        if let Some(i) = qi {
            if mrow(ui, w, 11, "Move up") && i > 0 {
                self.queue.swap(i, i - 1);
                self.fix_cur(i, i - 1);
                self.song_menu = Some((t.clone(), Some(i - 1)));
            }
            if mrow(ui, w, 12, "Move down") && i + 1 < self.queue.len() {
                self.queue.swap(i, i + 1);
                self.fix_cur(i, i + 1);
                self.song_menu = Some((t.clone(), Some(i + 1)));
            }
            if mrow(ui, w, 13, "Take out of the queue") {
                self.queue.remove(i);
                match self.cur {
                    Some(c) if c == i => {
                        self.stop();
                        self.cur = None;
                    }
                    Some(c) if c > i => self.cur = Some(c - 1),
                    _ => {}
                }
                close = true;
            }
        }
        if let Some((p, k)) = self.menu_pl {
            if mrow(ui, w, 13, "Take out of this playlist") {
                if let Some(pl) = self.playlists.get_mut(p) {
                    if k < pl.tracks.len() {
                        pl.tracks.remove(k);
                    }
                }
                self.save_playlists();
                close = true;
            }
        }
        if tidal && self.logged_in {
            let liked = self.liked_ids.contains(&t.id);
            if mrow(ui, w, 3, if liked { "Unlike" } else { "Like" }) {
                self.set_like(t, !liked);
            }
        }
        if mrow(ui, w, 4, "Add to a playlist") {
            self.menu_mode = 1;
            if tidal && self.logged_in && self.own_lists.is_empty() {
                self.spawn(|api| Msg::OwnLists(api.own_playlists()));
            }
        }
        if tidal && self.logged_in {
            for (icon, name, to) in [(5u8, "Go to album", api::GoTo::Album), (6, "Go to artist", api::GoTo::Artist), (7, "Song radio", api::GoTo::Radio)] {
                if mrow(ui, w, icon, name) {
                    let id = t.id;
                    self.page_busy = true;
                    self.page_pending = None;
                    self.spawn(move |api| Msg::Page(api.track_page(id, to)));
                    self.lib_tab = 4;
                    self.tab = Tab::Library;
                    close = true;
                }
            }
            if mrow(ui, w, 8, "Credits") {
                self.credits.clear();
                self.menu_mode = 2;
                let id = t.id;
                self.spawn(move |api| Msg::Credits(api.credits(id)));
            }
        }
        if mrow(ui, w, 8, "Edit info") {
            let m = meta::get(t.id).unwrap_or_default();
            self.edit = (
                t.title.clone(),
                t.artists_text(),
                m.key.map_or(String::new(), |k| meta::key_short(k).to_string()),
                m.bpm.map_or(String::new(), |b| b.to_string()),
            );
            self.menu_mode = 3;
        }
        if mrow(ui, w, 9, "Copy name") {
            let ok = set_clipboard(&format!("{} - {}", t.artists_text(), t.title));
            self.set_note(if ok { "Copied" } else { "Couldn't copy" });
            close = true;
        }
        let link = if let Some(y) = self.yt.iter().find(|y| yt_id(&y.vid) == t.id) {
            Some(format!("https://youtu.be/{}", y.vid))
        } else if tidal {
            Some(format!("https://tidal.com/track/{}", t.id))
        } else {
            None
        };
        if let Some(l) = link {
            if mrow(ui, w, 10, "Copy link") {
                let ok = set_clipboard(&l);
                self.set_note(if ok { "Link copied" } else { "Couldn't copy" });
                close = true;
            }
        }
        if let Some(k) = self.yt.iter().position(|y| yt_id(&y.vid) == t.id) {
            if mrow(ui, w, 13, "Take off the YouTube list") {
                self.yt.remove(k);
                save_json("yt.json", &self.yt);
                close = true;
            }
        }
        close
    }

    /// The menu's EDIT INFO page: the song's title and artist as you want them shown, its key and tempo.
    fn menu_edit(&mut self, ui: &mut egui::Ui, t: &Track, w: f32) -> bool {
        dim(ui, "Title", 1.0, pal().ink2);
        crate::practice::text_box(ui, &mut self.edit.0, w, "Title");
        dim(ui, "Artist", 1.0, pal().ink2);
        crate::practice::text_box(ui, &mut self.edit.1, w, "Artist");
        dim(ui, "Key and tempo", 1.0, pal().ink2);
        ui.horizontal(|ui| {
            let hw = (w - 6.0) / 2.0;
            crate::practice::text_box(ui, &mut self.edit.2, hw, "Key: Bb, F#m");
            crate::practice::text_box(ui, &mut self.edit.3, hw, "BPM");
        });
        let mut close = false;
        ui.horizontal(|ui| {
            if pill(ui, "Save", false).clicked() {
                let (title, artist) = (self.edit.0.trim().to_string(), self.edit.1.trim().to_string());
                if !title.is_empty() {
                    self.names.insert(t.id, (title.clone(), artist.clone()));
                    save_json("names.json", &self.names);
                    let id = t.id;
                    for list in [&mut self.queue, &mut self.recent, &mut self.liked, &mut self.results] {
                        for x in list.iter_mut().filter(|x| x.id == id) {
                            x.title = title.clone();
                            x.artist = artist.clone();
                            x.artists.clear();
                        }
                    }
                    for f in self.files.iter_mut().filter(|f| f.0.id == id) {
                        f.0.title = title.clone();
                        f.0.artist = artist.clone();
                    }
                }
                let key = meta::parse_typed_key(&self.edit.2);
                let bpm = self.edit.3.trim().parse::<u16>().ok().filter(|b| (20..=400).contains(b));
                meta::set_mine(t.id, bpm, key);
                save_json("meta.json", &meta::saved());
                self.set_note("Saved");
                close = true;
            }
            if pill(ui, "Back", false).clicked() {
                self.menu_mode = 0;
            }
        });
        close
    }

    /// A song shown with the name you gave it, if you did.
    pub(crate) fn renamed(&self, t: &mut Track) {
        if let Some((title, artist)) = self.names.get(&t.id) {
            t.title = title.clone();
            t.artist = artist.clone();
            t.artists.clear();
        }
    }

    /// The menu's playlist page: your playlists on the phone (and a new one), then your Tidal playlists.
    fn menu_playlists(&mut self, ui: &mut egui::Ui, t: &Track, tidal: bool, w: f32) -> bool {
        let mut close = false;
        dim(ui, "On this phone", 1.0, pal().ink2);
        for k in 0..self.playlists.len() {
            let name = self.playlists[k].name.clone();
            if pill_w(ui, &name, w, false).clicked() {
                self.playlists[k].tracks.push(QTrack::of(t));
                self.save_playlists();
                self.set_note(&format!("Added to {}", name));
                close = true;
            }
        }
        ui.horizontal(|ui| {
            let bw = 70.0;
            crate::practice::text_box(ui, &mut self.menu_new, w - bw - 6.0, "New playlist name");
            if pill_w(ui, "Make", bw, false).clicked() && !self.menu_new.trim().is_empty() {
                let name = self.menu_new.trim().to_string();
                self.playlists.push(Playlist { name: name.clone(), tracks: vec![QTrack::of(t)] });
                self.save_playlists();
                self.menu_new.clear();
                self.set_note(&format!("Made {}", name));
                close = true;
            }
        });
        if tidal && self.logged_in {
            ui.add_space(6.0);
            dim(ui, "On Tidal", 1.0, pal().ink2);
            if self.own_lists.is_empty() {
                dim(ui, "Loading...", 1.0, pal().dim);
            }
            for (uuid, name) in self.own_lists.clone() {
                if pill_w(ui, &name, w, false).clicked() {
                    let id = t.id;
                    self.spawn(move |api| Msg::Done(api.playlist_add(&uuid, id).map(|_| format!("Added to {}", name))));
                    close = true;
                }
            }
            if pill_w(ui, "New Tidal playlist (named from the box above)", w, false).clicked() && !self.menu_new.trim().is_empty() {
                let (name, id) = (self.menu_new.trim().to_string(), t.id);
                self.menu_new.clear();
                self.own_lists.clear();
                self.spawn(move |api| Msg::Done(api.playlist_create(&name).and_then(|u| api.playlist_add(&u, id)).map(|_| format!("Made {} on Tidal", name))));
                close = true;
            }
        }
        if pill_w(ui, "Back", w, false).clicked() {
            self.menu_mode = 0;
        }
        close
    }

    /// Like or unlike a Tidal song (the heart on Now Playing and in the menu).
    pub(crate) fn set_like(&mut self, t: &Track, like: bool) {
        if like {
            self.liked_ids.insert(t.id);
            if !self.liked.iter().any(|l| l.id == t.id) {
                self.liked.insert(0, t.clone());
            }
        } else {
            self.liked_ids.remove(&t.id);
            self.liked.retain(|l| l.id != t.id);
        }
        let (api, id) = (self.api.clone(), t.id);
        std::thread::spawn(move || {
            let _ = api.set_liked(id, like);
        });
    }
}

/// A row of the song menu: a line icon and a word, a hairline above. Returns true when tapped.
fn mrow(ui: &mut egui::Ui, w: f32, icon: u8, text: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 46.0), Sense::click());
    let p = ui.painter();
    if resp.is_pointer_button_down_on() {
        p.rect_filled(r, 6.0, pal().beige_lt);
    }
    p.line_segment([Pos2::new(r.min.x, r.min.y), Pos2::new(r.max.x, r.min.y)], Stroke::new(1.0, pal().beige_lt));
    let c = Pos2::new(r.min.x + 16.0, r.center().y);
    let col = pal().ink;
    let s = Stroke::new(1.8, col);
    let at = |x: f32, y: f32| c + Vec2::new(x, y);
    match icon {
        0 => play_glyph(p, c, false, col),
        1 => skip_glyph(p, c, true, col),
        2 | 4 => {
            for y in [-6.0, 0.0, 6.0] {
                p.line_segment([at(-9.0, y), at(4.0, y)], s);
            }
            if icon == 4 {
                p.line_segment([at(9.0, 2.0), at(9.0, 10.0)], s);
                p.line_segment([at(5.0, 6.0), at(13.0, 6.0)], s);
            }
        }
        3 => src_icon(p, c, 4, col),
        5 => {
            p.circle_stroke(c, 9.0, s);
            p.circle_filled(c, 2.5, col);
        }
        6 => {
            p.circle_stroke(at(0.0, -4.0), 4.0, s);
            p.add(egui::Shape::line(vec![at(-8.0, 9.0), at(-6.0, 3.0), at(6.0, 3.0), at(8.0, 9.0)], s));
        }
        7 => src_icon(p, c, 0, col),
        8 => {
            for (y, l) in [(-6.0, 12.0), (0.0, 8.0), (6.0, 12.0)] {
                p.line_segment([at(-8.0, y), at(-8.0 + l, y)], s);
            }
        }
        9 => {
            p.rect_stroke(Rect::from_min_max(at(-8.0, -5.0), at(3.0, 9.0)), 2.0, s);
            p.rect_stroke(Rect::from_min_max(at(-3.0, -10.0), at(8.0, 4.0)), 2.0, s);
        }
        10 => src_icon(p, c, 2, col),
        11 | 12 => {
            let d = if icon == 11 { -1.0 } else { 1.0 };
            p.line_segment([at(0.0, -8.0 * d), at(0.0, 8.0 * d)], s);
            p.line_segment([at(-5.0, 3.0 * d), at(0.0, 8.0 * d)], s);
            p.line_segment([at(5.0, 3.0 * d), at(0.0, 8.0 * d)], s);
        }
        _ => {
            p.line_segment([at(-6.0, -6.0), at(6.0, 6.0)], s);
            p.line_segment([at(6.0, -6.0), at(-6.0, 6.0)], s);
        }
    }
    ptext(p, Pos2::new(r.min.x + 42.0, r.center().y), Align::Min, text, 1.4, col);
    resp.clicked()
}
