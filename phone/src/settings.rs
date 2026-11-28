//! SETTINGS, as the desktop's Preferences has them: STORAGE (SAVING on or off, the saved songs, Clear Saved Songs),
//! YOUTUBE (yt-dlp's version, Update), SCREEN (Studio's screen colours).

use crate::*;

impl App {
    pub(crate) fn settings_ui(&mut self, ui: &mut egui::Ui) {
        // the saved songs counted now and then (not every frame)
        let now = ui.input(|i| i.time);
        if now - self.stats_at > 2.0 {
            self.stats = saved_stats();
            self.stats_at = now;
        }
        egui::ScrollArea::vertical().id_salt("settings_scroll").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            section_header(ui, "STORAGE");
            crate::mine::hint(ui, "Saving on: every song you add is kept on this phone and plays with no connection. Off: only the songs you press SAVE on.");
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, if self.keep { "SAVING: ON" } else { "SAVING: OFF" }, self.keep).clicked() {
                    self.keep = !self.keep;
                    self.saver_failed.clear();
                    self.save_settings();
                    let unsaved = self.songs.iter().filter(|s| !saved(&s.vid)).count();
                    self.set_note(&if !self.keep {
                        "Not saving new songs".to_string()
                    } else if unsaved > 0 {
                        format!("Saving songs to the phone. Your {} other songs are saved in the background", unsaved)
                    } else {
                        "Saving songs to the phone".to_string()
                    });
                }
                if self.ask_clear {
                    if retro_btn(ui, "DELETE THEM", true).clicked() {
                        clear_saved();
                        self.ask_clear = false;
                        self.saver_failed.clear();
                        self.stats_at = -10.0;
                        self.set_note("Saved songs deleted (your songs, loops and bookmarks stay)");
                    }
                    if retro_btn(ui, "KEEP THEM", false).clicked() {
                        self.ask_clear = false;
                    }
                } else if retro_btn(ui, "CLEAR SAVED SONGS", false).clicked() {
                    self.ask_clear = true;
                }
            });
            let (n, bytes) = self.stats;
            let busy = if self.saver_busy { " Saving another now." } else { "" };
            para(ui, &format!("{} of your {} songs saved, {} used.{}", n.min(self.songs.len().max(n)), self.songs.len(), fmt_size(bytes), busy), pal().ink);

            ui.add_space(8.0);
            // (an iPhone has no yt-dlp: songs come straight from YouTube, and your own files through the Files app)
            if cfg!(target_os = "ios") {
                section_header(ui, "YOUR FILES");
                crate::mine::hint(ui, "Put songs in the Files app, in On My iPhone > Backline (folders and all). They show in My Tracks > Files.");
            } else {
            section_header(ui, "YOUTUBE");
            if self.ytdlp_ver.is_none() && !self.ytdlp_busy {
                // (asked once each time Settings opens after a change)
                self.ytdlp_busy = true;
                self.spawn(|| Msg::YtdlpVersion(ytdlp::version()));
            }
            let ver = match &self.ytdlp_ver {
                Some(Ok(v)) => format!("yt-dlp {}, updated by itself daily.", v),
                Some(Err(e)) => format!("yt-dlp isn't working: {}", e),
                None => "Getting yt-dlp ready (the first time takes a little while)...".to_string(),
            };
            if matches!(self.ytdlp_ver, Some(Err(_))) {
                para(ui, &ver, pal().red);
            } else {
                crate::mine::hint(ui, &ver);
            }
            ui.horizontal_wrapped(|ui| {
                let busy = self.ytdlp_updating;
                if retro_btn(ui, if busy { "UPDATING..." } else { "UPDATE YT-DLP" }, false).clicked() && !busy {
                    self.ytdlp_busy = true;
                    self.ytdlp_updating = true;
                    self.ytdlp_note = "Looking for a newer yt-dlp...".to_string();
                    self.spawn(|| Msg::YtdlpUpdated(ytdlp::update()));
                }
            });
            if !self.ytdlp_note.is_empty() {
                para(ui, &self.ytdlp_note.clone(), pal().ink);
            }
            }

            ui.add_space(8.0);
            section_header(ui, "SCREEN");
            let names: Vec<&str> = skin::STUDIO_SCREENS.iter().map(|s| s.0).collect();
            if let Some(i) = segments(ui, &names, self.screen as usize) {
                self.screen = i as u8;
                skin::STUDIO_SCREEN.store(self.screen, Ordering::Relaxed);
                self.wear();
                self.save_settings();
            }
            ui.add_space(16.0);
        });
    }
}
