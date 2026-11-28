//! MY TRACKS, as the desktop keeps yours: LIKED (the songs you've hearted, YouTube songs and the phone's own tracks),
//! SAVED (your YouTube songs kept on the phone, playing with no connection), LOOPS (every song you've saved loops or
//! bookmarks on: loops.rs) and FILES (the music files on the phone, folder by folder, from Android's own list). A
//! filter for long lists; tap a song to practise it.

use crate::*;

/// The lists in the order shown, and each one's number (kept in Settings).
const LISTS: [(&str, u8); 4] = [("Liked", 0), ("Saved", 1), ("Loops", 3), ("Files", 2)];

impl App {
    pub(crate) fn mine_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
        let at = LISTS.iter().position(|l| l.1 == self.mine_tab).unwrap_or(0);
        if let Some(i) = segments(ui, &LISTS.map(|l| l.0), at) {
            self.mine_tab = LISTS[i].1;
            self.mine_filter.clear();
            if self.mine_tab == 2 {
                self.rescan_files();
            }
            self.save_settings();
        }
        if self.mine_tab == 3 {
            self.loops_ui(ui);
            return;
        }
        // ---- the list picked
        let files = self.mine_tab == 2;
        let list: Vec<Song> = match self.mine_tab {
            0 => self.liked.clone(),
            1 => self.songs.iter().filter(|s| saved(&s.vid)).cloned().collect(),
            _ => {
                if !self.phone_music(ui) {
                    return;
                }
                self.device_tracks.clone()
            }
        };
        // a filter, once a list is long enough to want one (in Files it looks through every folder)
        if list.len() > 8 || !self.mine_filter.is_empty() {
            let w = ui.available_width();
            let hint = if files { "Find a song in every folder" } else { "Filter by name or artist" };
            crate::songs::text_box(ui, &mut self.mine_filter, w, hint, "mine_filter");
        }
        let f = self.mine_filter.trim().to_lowercase();
        if files && f.is_empty() && !list.is_empty() {
            self.folders_ui(ui, &list);
            return;
        }
        let shown: Vec<Song> = if f.is_empty() {
            list
        } else {
            list.into_iter().filter(|s| s.title.to_lowercase().contains(&f) || s.channel.to_lowercase().contains(&f)).collect()
        };
        let what = match self.mine_tab {
            0 => "liked",
            1 => "saved on this phone",
            _ => "on this phone",
        };
        dim_line(ui, &format!("{} {} {}", shown.len(), if shown.len() == 1 { "song" } else { "songs" }, what), 1.0, pal().ink2);
        if shown.is_empty() {
            hint(
                ui,
                match self.mine_tab {
                    0 if f.is_empty() => "Tap the heart on a song to keep it here.",
                    1 if f.is_empty() && !self.keep => "Press SAVE on a song to keep it on this phone (Saving is off in Settings).",
                    1 if f.is_empty() => "Songs you add are saved here as they come in, or press SAVE on one.",
                    // (an iPhone shows an app only its own folder)
                    2 if f.is_empty() && cfg!(target_os = "ios") => "Put songs in the Files app, in On My iPhone > Backline. Folders show here as folders.",
                    2 if f.is_empty() => "No music files found on this phone.",
                    _ => "Nothing matches.",
                },
            );
            return;
        }
        self.song_rows(ui, shown, ("mine_scroll", self.mine_tab), !files);
    }

    /// Songs as rows (only the rows in view are drawn: a phone can hold thousands); tap one to practise it, Previous
    /// and Next going through the same list.
    fn song_rows(&mut self, ui: &mut egui::Ui, list: Vec<Song>, id: impl std::hash::Hash, save: bool) {
        let cur = self.cur.as_ref().map(|c| c.vid.clone());
        let (mut tapped, mut hearted, mut saving) = (None, None, None);
        egui::ScrollArea::vertical().id_salt(id).auto_shrink([false, false]).show_rows(ui, 46.0, list.len(), |ui, rows| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 2.0);
            for i in rows {
                let s = &list[i];
                let hit = self.song_row(ui, s, cur.as_deref() == Some(s.vid.as_str()), "", true, save);
                if hit.heart {
                    hearted = Some(s.clone());
                } else if hit.save {
                    saving = Some(s.clone());
                } else if hit.tap {
                    tapped = Some(s.clone());
                }
            }
        });
        if let Some(s) = hearted {
            self.toggle_like(&s);
        }
        if let Some(s) = saving {
            self.save_song(&s);
        }
        if let Some(s) = tapped {
            self.play_list = list;
            self.view = 1;
            self.load(s, true);
            self.save_settings();
        }
    }

    /// FILES, folder by folder: where you are (UP goes back), the folders in it with how many songs each holds, then
    /// its songs by file name.
    fn folders_ui(&mut self, ui: &mut egui::Ui, all: &[Song]) {
        let here = self.folder.clone();
        let mut subs: std::collections::BTreeMap<String, (String, usize)> = Default::default();
        let mut songs: Vec<Song> = Vec::new();
        for s in all {
            let f = device::folder_of(&s.path);
            if f == here {
                songs.push(s.clone());
            } else if let Some(rest) = under(&f, &here) {
                let first = rest.split('/').next().unwrap_or(rest).to_string();
                subs.entry(first.to_lowercase()).or_insert((first, 0)).1 += 1;
            }
        }
        // (a folder gone since: back to the top)
        if subs.is_empty() && songs.is_empty() && !here.is_empty() {
            self.folder.clear();
            self.save_settings();
            return;
        }
        ui.horizontal(|ui| {
            if !here.is_empty() && retro_btn_w(ui, "< UP", 64.0, false).clicked() {
                self.folder = here.rsplit_once('/').map(|(a, _)| a.to_string()).unwrap_or_default();
                self.save_settings();
            }
            let path = if here.is_empty() { format!("{} {} on this phone", all.len(), if all.len() == 1 { "song" } else { "songs" }) } else { here.replace('/', " / ") };
            dim_line_short(ui, &path, pal().ink2);
        });
        songs.sort_by_key(|s| file_name(&s.path).to_lowercase());
        let subs: Vec<(String, usize)> = subs.into_values().collect();
        if subs.is_empty() {
            self.song_rows(ui, songs, ("files", &here), false);
            return;
        }
        let cur = self.cur.as_ref().map(|c| c.vid.clone());
        let (mut open, mut tapped, mut hearted) = (None, None, None);
        egui::ScrollArea::vertical().id_salt(("files", &here)).auto_shrink([false, false]).show_rows(ui, 46.0, subs.len() + songs.len(), |ui, rows| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 2.0);
            for i in rows {
                if let Some((name, n)) = subs.get(i) {
                    if crate::songs::list_row(ui, None, "folder", name, &format!("{} {}", n, if *n == 1 { "song" } else { "songs" }), false).clicked() {
                        open = Some(name.clone());
                    }
                } else {
                    let s = &songs[i - subs.len()];
                    let hit = self.song_row(ui, s, cur.as_deref() == Some(s.vid.as_str()), "", true, false);
                    if hit.heart {
                        hearted = Some(s.clone());
                    } else if hit.tap {
                        tapped = Some(s.clone());
                    }
                }
            }
        });
        if let Some(n) = open {
            self.folder = if here.is_empty() { n } else { format!("{}/{}", here, n) };
            self.save_settings();
        }
        if let Some(s) = hearted {
            self.toggle_like(&s);
        }
        if let Some(s) = tapped {
            self.play_list = songs;
            self.view = 1;
            self.load(s, true);
            self.save_settings();
        }
    }

    /// The phone's music read again in the background (Files opened again: songs added since show up), the list
    /// kept meanwhile.
    pub(crate) fn rescan_files(&mut self) {
        if self.device_state == 3 {
            self.spawn(|| Msg::DeviceTracks(device::scan()));
        }
    }

    /// FILES' way in: Android's permission asked for (once), then the phone's music read. Returns whether the list is
    /// ready to show.
    fn phone_music(&mut self, ui: &mut egui::Ui) -> bool {
        if self.device_state == 3 {
            if !self.device_err.is_empty() {
                ui.horizontal(|ui| {
                    dim_line_short(ui, &self.device_err.clone(), pal().red);
                    if retro_btn(ui, "TRY AGAIN", false).clicked() {
                        self.device_state = 2;
                        self.spawn(|| Msg::DeviceTracks(device::scan()));
                    }
                });
            }
            return true;
        }
        if self.device_state == 2 {
            dim_centered(ui, "Reading the music on this phone...");
            return false;
        }
        if device::allowed() {
            self.device_state = 2;
            self.spawn(|| Msg::DeviceTracks(device::scan()));
            dim_centered(ui, "Reading the music on this phone...");
            return false;
        }
        // not allowed yet: say what it's for, and ask (Android's own question), then wait for the yes
        hint(ui, "Practise with the music files on your phone, folder by folder.");
        ui.add_space(4.0);
        centered_row(ui, "allow_music", |ui| {
            if retro_btn(ui, "SHOW MY FILES", true).clicked() {
                self.device_state = 1;
                device::ask();
            }
        });
        if self.device_state == 1 {
            let now = ui.input(|i| i.time);
            if now - self.device_asked_at > 0.5 {
                self.device_asked_at = now;
                if device::allowed() {
                    self.device_state = 0;
                }
            }
            ui.ctx().request_repaint_after(Duration::from_millis(500));
            dim_centered(ui, "Allow Backline to see your music (or turn it on in the phone's settings)");
        }
        false
    }
}

/// A folder's path under the one open ("" the top), or none when it isn't under it.
fn under<'a>(f: &'a str, here: &str) -> Option<&'a str> {
    if here.is_empty() {
        return (!f.is_empty()).then_some(f);
    }
    f.strip_prefix(here)?.strip_prefix('/')
}

/// A file's name, from its path.
fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Words wrapped across the window, dim, shown whole (an explanation, or what a list is for while it's empty; the
/// desktop's `para` keeps one line and puts the rest under the pointer, and a phone has no pointer).
pub(crate) fn hint(ui: &mut egui::Ui, text: &str) {
    for line in text.lines() {
        for l in wrap(line, 1.3, ui.available_width() - 8.0) {
            let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
            ptext(ui.painter(), Pos2::new(r.min.x + 2.0, r.center().y), Align::Min, &l, 1.3, pal().ink2);
        }
    }
}

/// A short line, as wide as its words (cut with ... when the room is less).
fn dim_line_short(ui: &mut egui::Ui, text: &str, col: Color32) {
    let w = (text_w(text, 1.3) + 10.0).min(ui.available_width() - 4.0).max(40.0);
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
    ptext_fit(ui.painter(), Pos2::new(r.min.x + 2.0, r.center().y), Align::Min, text, 1.3, w - 4.0, col);
}
