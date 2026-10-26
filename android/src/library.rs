//! The LIBRARY window: TIDAL (search, your liked songs, your playlists), FILES on the phone, and YT (paste a link).
//! Tap a song to play it: the list it is in becomes the queue.

use crate::deck::song_row;
use crate::practice::text_box;
use crate::*;

impl App {
    /// MUSIC, the Tidal app's way: MY TRACKS first (your liked songs up front), HOME (new releases, albums, mixes)
    /// and SEARCH, with the phone's files and YouTube links as two icons at the right. A page opened from any of
    /// them (an album, an artist...) shows over it, with a way back.
    pub(crate) fn library_ui(&mut self, ui: &mut egui::Ui) {
        if self.lib_tab == 4 {
            self.page_ui(ui);
            return;
        }
        if self.lib_tab > 5 {
            self.lib_tab = 3;
        }
        let cur = match self.lib_tab {
            3 => 0,
            5 => 1,
            0 => 2,
            _ => 9,
        };
        let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
        if let Some(i) = seg_tabs(ui, Rect::from_min_max(row.min, Pos2::new(row.max.x - 138.0, row.max.y)), &["My tracks", "Home", "Search"], cur, "music_seg") {
            self.list_open = None;
            self.lib_tab = [3, 5, 0][i];
            if i == 0 {
                self.tidal_load(self.my_page.min(1) + 1);
            }
        }
        // files on the phone and YouTube links, as two icons
        for (k, (tab, icon)) in [(1u8, 1usize), (2, 2)].iter().enumerate() {
            let c = Pos2::new(row.max.x - 112.0 + k as f32 * 44.0, row.center().y);
            let r = Rect::from_center_size(c, Vec2::splat(40.0));
            let resp = ui.interact(r, ui.id().with(("src_icon", k)), Sense::click());
            let on = self.lib_tab == *tab;
            if on || resp.is_pointer_button_down_on() {
                ui.painter().rect_filled(r, 20.0, pal().beige_lt);
            }
            src_icon(ui.painter(), c, *icon, if on { pal().red } else { pal().ink2 });
            if resp.clicked() {
                self.lib_tab = *tab;
                if *tab == 1 {
                    self.scan_files();
                }
            }
        }
        let g = Pos2::new(row.max.x - 20.0, row.center().y);
        gear_glyph(ui.painter(), g, pal().ink2);
        if ui.interact(Rect::from_center_size(g, Vec2::splat(40.0)), ui.id().with("music_gear"), Sense::click()).clicked() {
            self.settings_open = true;
        }
        ui.add_space(4.0);
        match self.lib_tab {
            1 => self.files_ui(ui),
            2 => self.yt_ui(ui),
            3 => self.my_tracks_ui(ui),
            5 => self.home_ui(ui),
            _ => self.tidal_ui(ui),
        }
    }

    /// HOME: what you played lately, then Tidal's home page (new releases, albums, mixes, playlists).
    fn home_ui(&mut self, ui: &mut egui::Ui) {
        if !self.logged_in {
            self.login_ui(ui);
            return;
        }
        if self.home.is_none() && !self.busy {
            self.busy = true;
            self.spawn(|api| {
                Msg::Library(api.home().map(|mut p| {
                    p.title = "home".to_string();
                    p
                }))
            });
        }
        if self.busy && self.home.is_none() {
            dim(ui, "Loading...", 1.0, pal().ink2);
        }
        let rows = self.home.as_ref().map(|p| p.rows.clone()).unwrap_or_default();
        let recent = self.recent.clone();
        egui::ScrollArea::vertical().id_salt("home_all").auto_shrink([false, false]).show(ui, |ui| {
            if !recent.is_empty() {
                dim(ui, "Played lately", 1.0, pal().ink2);
                let playing = self.cur_track().map(|t| t.id);
                let mut play = None;
                for (i, t) in recent.iter().take(6).enumerate() {
                    let (r, dots) = song_row(ui, i, t, playing == Some(t.id));
                    if dots {
                        self.song_menu = Some((t.clone(), None));
                        self.menu_pl = None;
                    } else if r.clicked() {
                        play = Some(i);
                    }
                }
                if let Some(i) = play {
                    self.queue = recent.clone();
                    self.play(i);
                }
            }
            self.card_rows_inline(ui, &rows);
        });
    }

    /// A list of songs; tap one to play the list from it. Over a list of a few: PLAY, SHUFFLE, PLAY NEXT and ADD TO
    /// QUEUE for all of it, and a box to filter a long one.
    fn song_list(&mut self, ui: &mut egui::Ui, tracks: &[Track], id: &str) {
        self.song_list_in(ui, tracks, id, None);
    }

    pub(crate) fn song_list_in(&mut self, ui: &mut egui::Ui, tracks: &[Track], id: &str, playlist: Option<usize>) {
        let playing = self.cur_track().map(|t| t.id);
        let mut play = None;
        if tracks.len() > 1 {
            ui.horizontal_wrapped(|ui| {
                if pill(ui, "Play", false).clicked() {
                    self.queue = tracks.to_vec();
                    self.shuffle = false;
                    play = Some(usize::MAX);
                }
                if pill(ui, "Shuffle", false).clicked() {
                    let mut q = tracks.to_vec();
                    for i in (1..q.len()).rev() {
                        q.swap(i, (rand() % (i as u64 + 1)) as usize);
                    }
                    self.queue = q;
                    play = Some(usize::MAX);
                }
                if pill(ui, "Play next", false).clicked() {
                    let at = self.cur.map_or(0, |c| c + 1).min(self.queue.len());
                    for (k, t) in tracks.iter().enumerate() {
                        self.queue.insert(at + k, t.clone());
                    }
                    self.set_note(&format!("{} songs play next", tracks.len()));
                }
                if pill(ui, "Add to queue", false).clicked() {
                    self.queue.extend(tracks.iter().cloned());
                    self.set_note(&format!("{} songs added", tracks.len()));
                }
            });
        }
        if let Some(usize::MAX) = play {
            self.play(0);
            return;
        }
        let long = tracks.len() > 30;
        if long {
            let w = ui.available_width();
            text_box(ui, &mut self.list_filter, w, "Filter this list");
        }
        let f = self.list_filter.trim().to_lowercase();
        egui::ScrollArea::vertical().id_salt(id).auto_shrink([false, false]).show(ui, |ui| {
            for (i, t) in tracks.iter().enumerate() {
                if long && !f.is_empty() && !format!("{} {} {}", t.title, t.artists_text(), t.album).to_lowercase().contains(&f) {
                    continue;
                }
                let (r, dots) = song_row(ui, i, t, playing == Some(t.id));
                if dots {
                    self.song_menu = Some((t.clone(), None));
                    self.menu_pl = playlist.map(|p| (p, i));
                } else if r.clicked() {
                    play = Some(i);
                }
            }
        });
        if let Some(i) = play {
            self.queue = tracks.to_vec();
            self.play(i);
        }
    }

    fn tidal_ui(&mut self, ui: &mut egui::Ui) {
        if !self.logged_in {
            self.login_ui(ui);
            return;
        }
        self.tidal_page = 0;
        self.tidal_body(ui);
    }

    /// MY TRACKS: your liked songs and playlists on Tidal, and the songs kept on this phone (they play without a
    /// connection).
    fn my_tracks_ui(&mut self, ui: &mut egui::Ui) {
        // four tiles, Spotify's library: Liked, Playlists, Albums (and artists), Kept (and the phone's files)
        let liked_n = if self.liked.is_empty() { String::new() } else { format!(" {}", self.liked.len()) };
        let tiles = [(format!("Liked{}", liked_n), 4usize, 0u8), ("Playlists".to_string(), 5, 1), ("Albums".to_string(), 6, 2), ("Kept".to_string(), 1, 4)];
        let w = ui.available_width();
        let tw = (w - 8.0) / 2.0;
        let (area, _) = ui.allocate_exact_size(Vec2::new(w, 2.0 * 48.0 + 8.0), Sense::hover());
        let mut hit = None;
        for (n, (name, icon, page)) in tiles.iter().enumerate() {
            let r = Rect::from_min_size(area.min + Vec2::new((n % 2) as f32 * (tw + 8.0), (n / 2) as f32 * 56.0), Vec2::new(tw, 48.0));
            let resp = ui.interact(r, ui.id().with(("mt_tile", n)), Sense::click());
            let on = self.my_page == *page || (*page == 2 && self.my_page == 3);
            ui.painter().rect_filled(r, 10.0, if on { pal().ink } else if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige_lt });
            let col = if on { pal().beige } else { pal().ink };
            src_icon(ui.painter(), Pos2::new(r.min.x + 22.0, r.center().y), *icon, if on { col } else { pal().red });
            ptext_fit(ui.painter(), Pos2::new(r.min.x + 42.0, r.center().y), Align::Min, name, 1.3, tw - 50.0, col);
            if resp.clicked() {
                hit = Some(*page as usize);
            }
        }
        if let Some(i) = hit {
            self.my_page = i as u8;
            self.list_open = None;
            self.pl_open = None;
            self.list_filter.clear();
            if i < 2 {
                self.tidal_load(i as u8 + 1);
            } else if i < 4 && self.lib_page.is_none() && self.logged_in && !self.busy {
                self.busy = true;
                self.spawn(|api| Msg::Library(api.library()));
            }
        }
        ui.add_space(4.0);
        // liked songs and playlists load the first time they show
        if self.my_page < 2 && !self.busy && ((self.my_page == 0 && self.liked.is_empty()) || (self.my_page == 1 && self.lists.is_empty())) {
            self.tidal_load(self.my_page + 1);
        }
        // albums and artists you saved, and new from the artists you follow
        if self.my_page == 2 || self.my_page == 3 {
            if self.busy {
                dim(ui, "Loading...", 1.0, pal().ink2);
            }
            let rows = self.lib_page.as_ref().map(|p| p.rows.clone()).unwrap_or_default();
            let want: &[&str] = &["Albums", "Artists", api::NEW_FROM_FOLLOWED];
            let rows: Vec<(String, Vec<api::Card>)> = rows.into_iter().filter(|r| want.contains(&r.0.as_str())).collect();
            self.card_rows(ui, &rows, "lib_rows");
            return;
        }
        // your playlists on the phone, above the Tidal ones
        if self.my_page == 1 {
            if let Some(p) = self.pl_open {
                let Some(pl) = self.playlists.get(p).cloned() else {
                    self.pl_open = None;
                    return;
                };
                ui.horizontal(|ui| {
                    if pill_w(ui, "<", 40.0, false).clicked() {
                        self.pl_open = None;
                    }
                    title_line(ui, &pl.name, 2.0, pal().ink);
                    if pill(ui, "Delete playlist", false).clicked() {
                        self.playlists.remove(p);
                        self.save_playlists();
                        self.pl_open = None;
                    }
                });
                if self.pl_open.is_some() {
                    let tracks: Vec<Track> = pl.tracks.iter().map(QTrack::track).collect();
                    self.song_list_in(ui, &tracks, "my_pl", Some(p));
                }
                return;
            }
            if self.list_open.is_none() && !self.playlists.is_empty() {
                dim(ui, "On this phone", 1.0, pal().ink2);
                for k in 0..self.playlists.len() {
                    let label = format!("{}   ({})", self.playlists[k].name, self.playlists[k].tracks.len());
                    if pill_w(ui, &label, ui.available_width(), false).clicked() {
                        self.pl_open = Some(k);
                    }
                }
                ui.add_space(6.0);
                dim(ui, "On Tidal", 1.0, pal().ink2);
            }
        }
        if self.my_page == 4 {
            if pill(ui, "Files on this phone", false).clicked() {
                self.lib_tab = 1;
                self.scan_files();
                return;
            }
            let mut kept = cache::tagged();
            kept.sort_by(|a, b| a.artist.cmp(&b.artist).then(a.title.cmp(&b.title)));
            if kept.is_empty() {
                note_text(ui, "Songs you play are kept here, so they play again without a connection.", pal().dim);
            }
            self.song_list(ui, &kept, "kept");
            return;
        }
        if !self.logged_in {
            self.login_ui(ui);
            return;
        }
        self.tidal_page = self.my_page.min(1) + 1;
        self.tidal_body(ui);
    }

    /// Fetch your liked songs (1) or playlists (2) the first time they're wanted.
    fn tidal_load(&mut self, i: u8) {
        if !self.logged_in {
            return;
        }
        {
            if i == 1 && self.liked.is_empty() && !self.busy {
                self.busy = true;
                self.spawn(|api| {
                    // every liked song, a page at a time
                    let mut all = Vec::new();
                    let mut r = Ok(());
                    loop {
                        match api.fav_tracks(all.len()) {
                            Ok((t, _, total)) => {
                                let n = t.len();
                                all.extend(t);
                                if n == 0 || all.len() >= total || all.len() >= 3000 {
                                    break;
                                }
                            }
                            Err(e) => {
                                r = Err(e);
                                break;
                            }
                        }
                    }
                    Msg::Liked(r.map(|_| all))
                });
            }
            if i == 2 && self.lists.is_empty() && !self.busy {
                self.busy = true;
                self.spawn(|api| Msg::Lists(api.my_playlists()));
            }
        }
    }

    /// The Tidal list showing: search (0), liked (1) or playlists (2).
    fn tidal_body(&mut self, ui: &mut egui::Ui) {
        if self.busy {
            dim(ui, "Loading...", 1.0, pal().ink2);
        }
        match self.tidal_page {
            1 => {
                let l = self.liked.clone();
                self.song_list(ui, &l, "liked");
            }
            2 => {
                if let Some((name, tracks)) = self.list_open.clone() {
                    ui.horizontal(|ui| {
                        if pill_w(ui, "<", 40.0, false).clicked() {
                            self.list_open = None;
                        }
                        title_line(ui, &name.to_uppercase(), 2.0, pal().ink);
                    });
                    self.song_list(ui, &tracks, "list");
                    return;
                }
                let lists = self.lists.clone();
                let mut open = None;
                egui::ScrollArea::vertical().id_salt("lists").auto_shrink([false, false]).show(ui, |ui| {
                    for (i, c) in lists.iter().enumerate() {
                        // a dense row: the name, then how many songs, dimmer, on the same line
                        let _ = i;
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::click());
                        let p = ui.painter();
                        if resp.is_pointer_button_down_on() {
                            p.rect_filled(r, 6.0, pal().beige_lt);
                        }
                        let sw = text_w(&c.subtitle, 1.1) + 12.0;
                        ptext_fit(p, Pos2::new(r.min.x + 10.0, r.center().y), Align::Min, &c.title, 1.3, r.width() - sw - 20.0, pal().ink);
                        ptext(p, Pos2::new(r.max.x - 8.0, r.center().y), Align::Max, &c.subtitle.to_lowercase(), 1.1, pal().dim);
                        if resp.clicked() {
                            open = Some(c.clone());
                        }
                    }
                });
                if let Some(c) = open {
                    self.busy = true;
                    self.spawn(move |api| Msg::ListTracks(c.title.clone(), api.open(&c).map(|p| p.tracks)));
                }
            }
            _ => {
                ui.horizontal(|ui| {
                    let w = ui.available_width() - 56.0;
                    let r = text_box(ui, &mut self.query, w, "Search Tidal");
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if (pill_w(ui, "Go", 48.0, false).clicked() || enter) && !self.query.trim().is_empty() {
                        let q = self.query.trim().to_string();
                        if let Some(p) = self.search_cache.get(&q.to_lowercase()) {
                            self.results = p.tracks.clone();
                            self.result_rows = p.rows.clone();
                        } else {
                            self.searching = true;
                            self.search_q = q.to_lowercase();
                            self.spawn(move |api| Msg::Found(api.search(&q)));
                        }
                    }
                });
                if self.searching {
                    dim(ui, "Searching...", 1.0, pal().ink2);
                }
                let r = self.results.clone();
                if r.is_empty() && self.result_rows.is_empty() {
                    // nothing searched yet: the searches made before, to tap again
                    let mut past: Vec<String> = self.search_cache.keys().cloned().collect();
                    past.sort();
                    if !past.is_empty() {
                        dim(ui, "Searched before", 1.0, pal().ink2);
                        ui.horizontal_wrapped(|ui| {
                            for q in past {
                                if pill(ui, &q, false).clicked() {
                                    if let Some(p) = self.search_cache.get(&q) {
                                        self.results = p.tracks.clone();
                                        self.result_rows = p.rows.clone();
                                    }
                                    self.query = q.clone();
                                }
                            }
                        });
                    }
                    return;
                }
                let rows = self.result_rows.clone();
                let mut sub = ui.new_child(egui::UiBuilder::new().max_rect(ui.available_rect_before_wrap()).layout(egui::Layout::top_down(Align::Min)));
                egui::ScrollArea::vertical().id_salt("search_all").auto_shrink([false, false]).show(&mut sub, |ui| {
                    let playing = self.cur_track().map(|t| t.id);
                    let mut play = None;
                    for (i, t) in r.iter().enumerate() {
                        let (resp, dots) = song_row(ui, i, t, playing == Some(t.id));
                        if dots {
                            self.song_menu = Some((t.clone(), None));
                            self.menu_pl = None;
                        } else if resp.clicked() {
                            play = Some(i);
                        }
                    }
                    if let Some(i) = play {
                        self.queue = r.clone();
                        self.play(i);
                    }
                    self.card_rows_inline(ui, &rows);
                });
            }
        }
    }

    /// Rows of cards (albums, artists, playlists, mixes) in their own scroll; tap one to open it.
    fn card_rows(&mut self, ui: &mut egui::Ui, rows: &[(String, Vec<api::Card>)], id: &str) {
        egui::ScrollArea::vertical().id_salt(id).auto_shrink([false, false]).show(ui, |ui| {
            self.card_rows_inline(ui, rows);
        });
    }

    fn card_rows_inline(&mut self, ui: &mut egui::Ui, rows: &[(String, Vec<api::Card>)]) {
        let mut open = None;
        for (title, cards) in rows {
            ui.add_space(6.0);
            dim(ui, title, 1.0, pal().ink2);
            for c in cards {
                let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
                let p = ui.painter();
                if resp.is_pointer_button_down_on() {
                    p.rect_filled(r, 6.0, pal().beige_lt);
                }
                let kind = match c.kind {
                    api::Kind::Album => "Album",
                    api::Kind::Artist => "Artist",
                    api::Kind::Playlist => "Playlist",
                    api::Kind::Mix => "Mix",
                };
                ptext_fit(p, Pos2::new(r.min.x + 8.0, r.min.y + 15.0), Align::Min, &c.title, 1.5, r.width() - 16.0, pal().ink);
                let sub = if c.subtitle.is_empty() { kind.to_string() } else { format!("{}  ·  {}", kind, c.subtitle) };
                ptext_fit(p, Pos2::new(r.min.x + 8.0, r.min.y + 33.0), Align::Min, &sub, 1.1, r.width() - 16.0, pal().dim);
                if resp.clicked() {
                    open = Some(c.clone());
                }
            }
        }
        if let Some(c) = open {
            self.open_card(c);
        }
    }

    /// Open a card's page (an album, an artist, a playlist, a mix).
    pub(crate) fn open_card(&mut self, c: api::Card) {
        self.page_busy = true;
        self.page_pending = Some(c.clone());
        self.lib_tab = 4;
        self.spawn(move |api| Msg::Page(api.open(&c)));
    }

    /// A Tidal page (an album, artist, playlist, mix, or a song's album / artist / radio): its songs and its rows,
    /// SAVE for an album or playlist, FOLLOW and RADIO for an artist; back goes to the page before.
    fn page_ui(&mut self, ui: &mut egui::Ui) {
        let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
        let title = self.pages.last().map(|p| p.title.clone()).unwrap_or_else(|| "Loading...".to_string());
        ptext_fit(ui.painter(), Pos2::new(r.min.x + 2.0, r.center().y), Align::Min, &format!("<  {}", title), 1.8, r.width() - 4.0, pal().ink);
        if resp.clicked() {
            if self.page_busy {
                self.page_busy = false;
                self.page_pending = None;
            } else {
                self.pages.pop();
                self.page_cards.pop();
            }
            if self.pages.is_empty() {
                self.lib_tab = 3;
            }
            return;
        }
        if self.page_busy {
            dim(ui, "Loading...", 1.0, pal().ink2);
            return;
        }
        let Some(page) = self.pages.last().cloned() else {
            self.lib_tab = 3;
            return;
        };
        // the header: the cover big, the title and what it is
        if !page.image.is_empty() {
            let url = api::cover_url(&page.image, 320);
            if self.page_cover.as_ref().map(|c| &c.0) != Some(&url) {
                self.page_cover = Some((url.clone(), None));
                self.spawn(move |api| {
                    let img = api.fetch(&url).ok().and_then(|b| image::load_from_memory(&b).ok()).map(|i| {
                        let i = i.to_rgba8();
                        egui::ColorImage::from_rgba_unmultiplied([i.width() as usize, i.height() as usize], i.as_raw())
                    });
                    Msg::Cover(url, img)
                });
            }
            let (hr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 108.0), Sense::hover());
            let cv = Rect::from_min_size(hr.min, Vec2::splat(104.0));
            match self.page_cover.as_ref().and_then(|c| c.1.as_ref()) {
                Some(tex) => {
                    let mut img = egui::epaint::RectShape::filled(cv, 8.0, Color32::WHITE);
                    img.fill_texture_id = tex.id();
                    img.uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                    ui.painter().add(img);
                }
                None => {
                    ui.painter().rect_filled(cv, 8.0, pal().row_sel);
                }
            }
            let tx = cv.max.x + 12.0;
            let tw = hr.max.x - tx;
            ptext_fit(ui.painter(), Pos2::new(tx, hr.min.y + 60.0), Align::Min, &page.title, 1.8, tw, pal().ink);
            ptext_fit(ui.painter(), Pos2::new(tx, hr.min.y + 84.0), Align::Min, &page.subtitle, 1.1, tw, pal().dim);
            ui.add_space(6.0);
        } else if !page.subtitle.is_empty() {
            dim(ui, &page.subtitle, 1.0, pal().dim);
        }
        let card = self.page_cards.last().cloned().flatten();
        if let Some(c) = card {
            ui.horizontal_wrapped(|ui| match c.kind {
                api::Kind::Artist => {
                    if pill(ui, "Follow", false).clicked() {
                        let id = c.id.clone();
                        self.spawn(move |api| Msg::Done(api.follow_artist(&id, true).map(|_| "Following".to_string())));
                    }
                    if pill(ui, "Unfollow", false).clicked() {
                        let id = c.id.clone();
                        self.spawn(move |api| Msg::Done(api.follow_artist(&id, false).map(|_| "Not following".to_string())));
                    }
                }
                api::Kind::Album | api::Kind::Playlist => {
                    if pill(ui, "Save to my library", false).clicked() {
                        let c2 = c.clone();
                        self.spawn(move |api| Msg::Done(api.save_card(&c2, true).map(|_| "Saved".to_string())));
                    }
                    if pill(ui, "Remove from my library", false).clicked() {
                        let c2 = c.clone();
                        self.spawn(move |api| Msg::Done(api.save_card(&c2, false).map(|_| "Removed".to_string())));
                    }
                }
                _ => {}
            });
        }
        if page.tracks.is_empty() {
            self.card_rows(ui, &page.rows, "page_rows");
        } else {
            let tracks = page.tracks.clone();
            let rows = page.rows.clone();
            let h = if rows.is_empty() { ui.available_height() } else { ui.available_height() * 0.6 };
            let mut top = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), h))).layout(egui::Layout::top_down(Align::Min)));
            self.song_list(&mut top, &tracks, "page_tracks");
            if !rows.is_empty() {
                ui.add_space(h);
                self.card_rows(ui, &rows, "page_rows");
            }
        }
    }

    fn login_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        note_text(ui, "Sign in with your own Tidal subscription. Files on this phone and YouTube work without it.", pal().ink2);
        ui.add_space(6.0);
        match self.code.clone() {
            None => {
                if pill(ui, "Sign in to Tidal", false).clicked() {
                    self.code = Some((String::new(), String::new()));
                    self.spawn(|api| Msg::Code(api.start_login()));
                }
            }
            Some((code, url)) if !code.is_empty() => {
                note_text(ui, "Approve Backline in the page that opened, or go to", pal().ink2);
                lcd_box(ui, &url, ui.available_width(), pal().ink);
                note_text(ui, "and enter the code", pal().ink2);
                title_line(ui, &code, 4.0, pal().ink);
            }
            Some(_) => dim(ui, "Asking Tidal...", 1.0, pal().ink2),
        }
    }

    /// The songs on the phone: Backline's own Music folder (no permission needed), and the phone's Music and
    /// Download folders where Android lets us read them.
    /// Look for songs on the phone (the app's Music folder, Music and Download, and the folders in them), with the
    /// names in their tags, and the playlist files (.m3u, .m3u8, .pls) there - in the background.
    pub(crate) fn scan_files(&mut self) {
        if self.files_busy {
            return;
        }
        self.files_busy = true;
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let mut dirs = vec![music_dir()];
            dirs.extend(["/sdcard/Music", "/sdcard/Download"].iter().map(std::path::PathBuf::from));
            let mut songs = Vec::new();
            let mut lists = Vec::new();
            for d in dirs {
                walk(&d, 0, &mut songs, &mut lists);
            }
            songs.sort_by(|a: &(Track, std::path::PathBuf), b| a.0.title.to_lowercase().cmp(&b.0.title.to_lowercase()));
            let lists: Vec<(String, Vec<std::path::PathBuf>)> = lists.iter().filter_map(|p| read_playlist_file(p)).filter(|l| !l.1.is_empty()).collect();
            let _ = tx.send(Msg::Files(songs, lists));
            ctx.request_repaint();
        });
    }

    fn files_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if pill(ui, if self.files_busy { "Looking..." } else { "Look again" }, false).clicked() {
                self.scan_files();
            }
        });
        // playlist files found, each a button that plays it
        let lists = self.file_lists.clone();
        if !lists.is_empty() {
            dim(ui, "Playlist files", 1.0, pal().ink2);
            ui.horizontal_wrapped(|ui| {
                for (name, paths) in &lists {
                    if pill(ui, &format!("{} ({})", name, paths.len()), false).clicked() {
                        let q: Vec<Track> = paths.iter().filter_map(|p| self.files.iter().find(|f| &f.1 == p).map(|f| f.0.clone())).collect();
                        if !q.is_empty() {
                            self.queue = q;
                            self.play(0);
                        }
                    }
                }
            });
        }
        if self.files.is_empty() {
            note_text(ui, &format!("No songs found. Put audio files in your Music or Download folder, or in {}", music_dir().display()), pal().ink2);
            return;
        }
        let tracks: Vec<Track> = self.files.iter().map(|f| f.0.clone()).collect();
        self.song_list(ui, &tracks, "files");
    }

    /// YT: paste a YouTube link (a video or a playlist) and its audio plays like any song.
    fn yt_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let w = ui.available_width() - 132.0;
            text_box(ui, &mut self.yt_link, w.max(80.0), "A YouTube link");
            if pill_w(ui, "Paste", 62.0, false).clicked() {
                if let Some(s) = clipboard() {
                    self.yt_link = s.trim().to_string();
                }
            }
            if pill_w(ui, "Add", 50.0, false).clicked() && !self.busy {
                self.yt_add(self.yt_link.trim().to_string(), None);
            }
        });
        if self.busy {
            dim(ui, "Reading the link...", 1.0, pal().ink2);
        }
        // a big playlist: only with a yes
        if let Some((link, name, n)) = self.yt_ask.clone() {
            note_text(ui, &format!("\"{}\" has {} videos.", name, n), pal().ink);
            ui.horizontal_wrapped(|ui| {
                if pill(ui, &format!("ADD ALL {}", n), false).clicked() {
                    self.yt_ask = None;
                    self.yt_add(link.clone(), Some(n as usize));
                }
                if pill(ui, "First 25", false).clicked() {
                    self.yt_ask = None;
                    self.yt_add(link.clone(), Some(25));
                }
                if pill(ui, "Cancel", false).clicked() {
                    self.yt_ask = None;
                }
            });
        }
        if self.yt.is_empty() {
            note_text(ui, "Copy a link in the YouTube app (Share > Copy link), then PASTE and ADD.", pal().ink2);
            return;
        }
        let playing = self.cur_track().map(|t| t.id);
        let items = self.yt.clone();
        let mut play = None;
        let mut drop = None;
        egui::ScrollArea::vertical().id_salt("yt").auto_shrink([false, false]).show(ui, |ui| {
            for (i, y) in items.iter().enumerate() {
                let t = y.track();
                let (r, dots) = song_row(ui, i, &t, playing == Some(t.id));
                if dots {
                    self.song_menu = Some((t.clone(), None));
                } else if r.clicked() {
                    play = Some(i);
                }
            }
        });
        if let Some(i) = drop {
            self.yt.remove(i);
            save_json("yt.json", &self.yt);
        } else if let Some(i) = play {
            self.queue = self.yt.iter().map(|y| y.track()).collect();
            self.play(i);
        }
    }

    /// Read a link: a video is added; a playlist is added up to `limit` videos, asking first when it is big.
    fn yt_add(&mut self, link: String, limit: Option<usize>) {
        if link.is_empty() {
            return;
        }
        self.busy = true;
        self.spawn(move |_| {
            let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
                Ok(r) => r,
                Err(e) => return Msg::YtAdded(Err(e.to_string())),
            };
            rt.block_on(async move {
                let rp = match yt_client() {
                    Ok(r) => r,
                    Err(e) => return Msg::YtAdded(Err(e)),
                };
                if let Some(list) = list_id(&link) {
                    let mut pl = match rp.query().playlist(&list).await {
                        Ok(p) => p,
                        Err(e) => return Msg::YtAdded(Err(e.to_string())),
                    };
                    let n = pl.video_count;
                    let Some(limit) = limit.or((n <= 25).then_some(25)) else {
                        return Msg::YtAsk(link.clone(), pl.name.clone(), n);
                    };
                    let _ = pl.videos.extend_limit(rp.query(), limit).await;
                    let items = pl
                        .videos
                        .items
                        .iter()
                        .take(limit)
                        .map(|v| YtItem {
                            vid: v.id.clone(),
                            title: v.name.clone(),
                            channel: v.channel.as_ref().map(|c| c.name.clone()).unwrap_or_default(),
                            secs: v.duration.unwrap_or(0) as f32,
                        })
                        .collect();
                    return Msg::YtAdded(Ok(items));
                }
                let Some(vid) = video_id(&link) else {
                    return Msg::YtAdded(Err("that doesn't look like a YouTube link".to_string()));
                };
                match rp.query().player(&vid).await {
                    Ok(p) => Msg::YtAdded(Ok(vec![YtItem {
                        vid,
                        title: p.details.name.clone().unwrap_or_default(),
                        channel: p.details.channel_name.clone().unwrap_or_default(),
                        secs: p.details.duration as f32,
                    }])),
                    Err(e) => Msg::YtAdded(Err(e.to_string())),
                }
            })
        });
    }
}

const AUDIO: [&str; 7] = ["mp3", "m4a", "flac", "wav", "ogg", "aac", "opus"];

/// Songs and playlist files in a folder and the folders in it (four deep).
fn walk(d: &std::path::Path, depth: u8, songs: &mut Vec<(Track, std::path::PathBuf)>, lists: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(d) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth < 4 && !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
                walk(&p, depth + 1, songs, lists);
            }
            continue;
        }
        let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("").to_lowercase();
        if ["m3u", "m3u8", "pls"].contains(&ext.as_str()) {
            lists.push(p);
            continue;
        }
        if !AUDIO.contains(&ext.as_str()) {
            continue;
        }
        let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("?").to_string();
        let id = -(1 + (hash(&p.to_string_lossy()) & 0x7fff_ffff_ffff) as i64);
        let (mut artist, mut title) = match name.split_once(" - ") {
            Some((a, b)) => (a.to_string(), b.to_string()),
            None => (String::new(), name.clone()),
        };
        let mut album = String::new();
        if let Some((ta, tt, tal)) = tag_names(&p) {
            if !tt.is_empty() {
                title = tt;
            }
            if !ta.is_empty() {
                artist = ta;
            }
            album = tal;
        }
        if songs.iter().any(|s| s.1 == p) {
            continue;
        }
        songs.push((Track { id, title, artist, album, ..Default::default() }, p));
    }
}

/// The artist, title and album in a file's tags (the desktop's).
fn tag_names(path: &std::path::Path) -> Option<(String, String, String)> {
    use lofty::prelude::*;
    let tagged = lofty::read_from_path(path).ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    Some((
        tag.artist().map(|s| s.to_string()).unwrap_or_default(),
        tag.title().map(|s| s.to_string()).unwrap_or_default(),
        tag.album().map(|s| s.to_string()).unwrap_or_default(),
    ))
}

/// A file's cover picture: the one in its tags, else a cover image in its folder (the desktop's).
pub(crate) fn file_art(path: &std::path::Path) -> Option<Vec<u8>> {
    use lofty::prelude::*;
    if let Ok(tagged) = lofty::read_from_path(path) {
        for tag in tagged.tags() {
            if let Some(p) = tag.pictures().first() {
                return Some(p.data().to_vec());
            }
        }
    }
    let dir = path.parent()?;
    for name in ["cover", "folder", "front", "album", "Cover", "Folder", "Front", "AlbumArt"] {
        for ext in ["jpg", "jpeg", "png"] {
            if let Ok(b) = std::fs::read(dir.join(format!("{}.{}", name, ext))) {
                return Some(b);
            }
        }
    }
    None
}

/// The songs a playlist file lists, in order (paths beside it work as well as full ones), and its name (the
/// desktop's).
fn read_playlist_file(p: &std::path::Path) -> Option<(String, Vec<std::path::PathBuf>)> {
    let bytes = std::fs::read(p).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let dir = p.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
    let pls = p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("pls"));
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        let entry = if pls {
            match line.split_once('=') {
                Some((k, v)) if k.to_ascii_lowercase().starts_with("file") => v.trim(),
                _ => continue,
            }
        } else if line.is_empty() || line.starts_with('#') {
            continue;
        } else {
            line
        };
        let entry = entry.strip_prefix("file:///").or_else(|| entry.strip_prefix("file://")).unwrap_or(entry);
        let entry = entry.replace("%20", " ");
        let path = std::path::PathBuf::from(&entry);
        let path = if path.is_absolute() { path } else { dir.join(path) };
        if path.exists() {
            out.push(path);
        }
    }
    let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("Playlist").to_string();
    Some((name, out))
}
