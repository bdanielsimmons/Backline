//! The LIBRARY window: TIDAL (search, your liked songs, your playlists), FILES on the phone, and YT (paste a link).
//! Tap a song to play it: the list it is in becomes the queue.

use crate::deck::song_row;
use crate::practice::text_box;
use crate::*;

impl App {
    pub(crate) fn library_ui(&mut self, ui: &mut egui::Ui) {
        if self.lib_tab == 9 {
            self.music_home(ui);
            return;
        }
        // inside a source: a way back to the tiles, and its name
        let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
        let name = ["Tidal", "Files", "YouTube"][(self.lib_tab as usize).min(2)];
        ptext(ui.painter(), Pos2::new(r.min.x + 2.0, r.center().y), Align::Min, &format!("<  {}", name), 1.8, pal().ink);
        if resp.clicked() {
            self.lib_tab = 9;
            return;
        }
        ui.add_space(4.0);
        match self.lib_tab {
            1 => self.files_ui(ui),
            2 => self.yt_ui(ui),
            _ => self.tidal_ui(ui),
        }
    }

    /// MUSIC's home: four big tiles (Tidal, Files, YouTube, Liked), then what was played lately.
    fn music_home(&mut self, ui: &mut egui::Ui) {
        let tiles = [("Tidal", "Search, liked, playlists"), ("Files", "On this phone"), ("YouTube", "Paste a link"), ("Liked", "Your hearts")];
        let w = (ui.available_width() - 10.0) / 2.0;
        let h = 92.0;
        let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 2.0 * h + 10.0), Sense::hover());
        for (k, (name, sub)) in tiles.iter().enumerate() {
            let r = Rect::from_min_size(area.min + Vec2::new((k % 2) as f32 * (w + 10.0), (k / 2) as f32 * (h + 10.0)), Vec2::new(w, h));
            let resp = ui.interact(r, ui.id().with(("src", k)), Sense::click());
            let p = ui.painter();
            p.rect_filled(r, 10.0, if resp.is_pointer_button_down_on() { pal().beige_dk } else { pal().beige_lt });
            src_icon(p, Pos2::new(r.center().x, r.min.y + 26.0), k, pal().red);
            ptext(p, Pos2::new(r.center().x, r.min.y + 56.0), Align::Center, name, 1.7, pal().ink);
            ptext_fit(p, Pos2::new(r.center().x, r.min.y + 76.0), Align::Center, sub, 1.1, w - 12.0, pal().dim);
            if resp.clicked() {
                match k {
                    3 => {
                        self.lib_tab = 0;
                        self.tidal_page = 1;
                    }
                    _ => {
                        self.lib_tab = k as u8;
                        if k == 1 {
                            self.scan_files();
                        }
                    }
                }
            }
        }
        ui.add_space(14.0);
        ptext(ui.painter(), ui.cursor().min + Vec2::new(2.0, 6.0), Align::Min, "RECENT", 1.1, pal().ink2);
        ui.add_space(18.0);
        if self.recent.is_empty() {
            para(ui, "What you play shows up here.", pal().dim);
        } else {
            let recent = self.recent.clone();
            self.song_list(ui, &recent, "recent");
        }
    }

    /// A list of songs; tap one to play the list from it.
    fn song_list(&mut self, ui: &mut egui::Ui, tracks: &[Track], id: &str) {
        let playing = self.cur_track().map(|t| t.id);
        let mut play = None;
        egui::ScrollArea::vertical().id_salt(id).auto_shrink([false, false]).show(ui, |ui| {
            for (i, t) in tracks.iter().enumerate() {
                let (r, dots) = song_row(ui, i, t, playing == Some(t.id));
                if dots {
                    self.song_menu = Some((t.clone(), None));
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
        if let Some(i) = tab_row(ui, &["SEARCH", "LIKED", "PLAYLISTS"], self.tidal_page as usize) {
            self.tidal_page = i as u8;
            self.list_open = None;
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
        ui.add_space(4.0);
        if self.busy {
            dim_line(ui, "LOADING...", 1.0, pal().ink2);
        }
        match self.tidal_page {
            1 => {
                let l = self.liked.clone();
                self.song_list(ui, &l, "liked");
            }
            2 => {
                if let Some((name, tracks)) = self.list_open.clone() {
                    ui.horizontal(|ui| {
                        if icon_btn_w(ui, &IC_BACK, false, pal().ink, 40.0).clicked() {
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
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H + 14.0), Sense::click());
                        let p = ui.painter();
                        if i % 2 == 1 {
                            fill_rect(p, r, pal().row_alt);
                        }
                        ptext_fit(p, Pos2::new(r.min.x + 8.0, r.min.y + 11.0), Align::Min, &c.title.to_uppercase(), 2.0, r.width() - 16.0, pal().ink);
                        ptext_fit(p, Pos2::new(r.min.x + 8.0, r.min.y + 27.0), Align::Min, &c.subtitle.to_uppercase(), 1.0, r.width() - 16.0, pal().ink2);
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
                    if (retro_btn_w(ui, "GO", 48.0, false).clicked() || enter) && !self.query.trim().is_empty() {
                        let q = self.query.trim().to_string();
                        self.searching = true;
                        self.spawn(move |api| Msg::Found(api.search(&q).map(|p| p.tracks)));
                    }
                });
                if self.searching {
                    dim_line(ui, "SEARCHING...", 1.0, pal().ink2);
                }
                let r = self.results.clone();
                self.song_list(ui, &r, "search");
            }
        }
    }

    fn login_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        para(ui, "Sign in with your own Tidal subscription. Files on this phone and YouTube work without it.", pal().ink2);
        ui.add_space(6.0);
        match self.code.clone() {
            None => {
                if retro_btn(ui, "SIGN IN TO TIDAL", false).clicked() {
                    self.code = Some((String::new(), String::new()));
                    self.spawn(|api| Msg::Code(api.start_login()));
                }
            }
            Some((code, url)) if !code.is_empty() => {
                para(ui, "Approve Backline in the page that opened, or go to", pal().ink2);
                lcd_box(ui, &url, ui.available_width(), pal().ink);
                para(ui, "and enter the code", pal().ink2);
                title_line(ui, &code, 4.0, pal().ink);
            }
            Some(_) => dim_line(ui, "ASKING TIDAL...", 1.0, pal().ink2),
        }
    }

    /// The songs on the phone: Backline's own Music folder (no permission needed), and the phone's Music and
    /// Download folders where Android lets us read them.
    pub(crate) fn scan_files(&mut self) {
        let mut dirs = vec![music_dir()];
        dirs.extend(["/sdcard/Music", "/sdcard/Download"].iter().map(std::path::PathBuf::from));
        self.files.clear();
        for d in dirs {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("").to_lowercase();
                if !["mp3", "m4a", "flac", "wav", "ogg", "aac"].contains(&ext.as_str()) {
                    continue;
                }
                let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("?").to_string();
                let id = -(1 + (hash(&p.to_string_lossy()) & 0x7fff_ffff_ffff) as i64);
                let (artist, title) = match name.split_once(" - ") {
                    Some((a, b)) => (a.to_string(), b.to_string()),
                    None => (String::new(), name.clone()),
                };
                self.files.push((Track { id, title, artist, ..Default::default() }, p));
            }
        }
        self.files.sort_by(|a, b| a.0.title.to_lowercase().cmp(&b.0.title.to_lowercase()));
    }

    fn files_ui(&mut self, ui: &mut egui::Ui) {
        if retro_btn(ui, "LOOK AGAIN", false).clicked() {
            self.scan_files();
        }
        if self.files.is_empty() {
            para(ui, &format!("No songs found. Put audio files in your Music or Download folder, or in {}", music_dir().display()), pal().ink2);
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
            if retro_btn_w(ui, "PASTE", 62.0, false).clicked() {
                if let Some(s) = clipboard() {
                    self.yt_link = s.trim().to_string();
                }
            }
            if retro_btn_w(ui, "ADD", 50.0, false).clicked() && !self.busy {
                self.yt_add(self.yt_link.trim().to_string(), None);
            }
        });
        if self.busy {
            dim_line(ui, "READING THE LINK...", 1.0, pal().ink2);
        }
        // a big playlist: only with a yes
        if let Some((link, name, n)) = self.yt_ask.clone() {
            para(ui, &format!("\"{}\" has {} videos.", name, n), pal().ink);
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, &format!("ADD ALL {}", n), false).clicked() {
                    self.yt_ask = None;
                    self.yt_add(link.clone(), Some(n as usize));
                }
                if retro_btn(ui, "FIRST 25", false).clicked() {
                    self.yt_ask = None;
                    self.yt_add(link.clone(), Some(25));
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.yt_ask = None;
                }
            });
        }
        if self.yt.is_empty() {
            para(ui, "Copy a link in the YouTube app (Share > Copy link), then PASTE and ADD.", pal().ink2);
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
