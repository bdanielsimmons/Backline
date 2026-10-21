//! The help / about overlay (the ? button or F1): what Backline is, how it is meant to be used, the keys,
//! and a little about where it came from. Short chunks on purpose - read only what you need.

use crate::{pal, para, retro_btn, section_header, tab_row, title_line, Action, App, PRACTICE};
use eframe::egui;

fn points(ui: &mut egui::Ui, items: &[&str]) {
    for s in items {
        para(ui, &format!("- {}", s), pal().ink);
    }
}

impl App {
    pub(crate) fn help_overlay(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        if !self.show_help {
            return;
        }
        let mut tour: Option<u8> = None;
        let mut open_prefs = false;
        let binds = self.binds.clone();
        // dim everything behind it
        crate::credits::block_behind(ctx, "help");
        let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("help_dim")));
        p.rect_filled(ctx.screen_rect(), 0.0, egui::Color32::from_black_alpha(150));
        let size = egui::vec2(640.0_f32.min(ctx.screen_rect().width() - 40.0), 520.0_f32.min(ctx.screen_rect().height() - 40.0));
        egui::Area::new(egui::Id::new("help_area"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let frame = egui::Frame::none()
                    .fill(pal().beige)
                    .stroke(egui::Stroke::new(2.0_f32, pal().edge))
                    .rounding(if crate::style() != 0 { 8.0 } else { 0.0 })
                    .inner_margin(12.0);
                frame.show(ui, |ui| {
                    ui.set_width(size.x - 24.0);
                    ui.set_height(size.y - 24.0);
                    title_line(ui, "HELP", 3.0, pal().ink);
                    let tabs: Vec<&str> = if PRACTICE {
                        vec!["TOUR", "START", "KEYS", "PRACTICE", "ABOUT"]
                    } else {
                        vec!["TOUR", "START", "KEYS", "ABOUT"]
                    };
                    let cur = self.help_tab.min(tabs.len() - 1);
                    if let Some(i) = tab_row(ui, &tabs, cur) {
                        self.help_tab = i;
                    }
                    let name = tabs[cur];
                    egui::ScrollArea::vertical().auto_shrink([false, false]).max_height(size.y - 150.0).show(
                        ui,
                        |ui| match name {
                            "TOUR" => tour_tab(ui, &mut tour),
                            "START" => start_tab(ui),
                            "KEYS" => keys_tab(ui, &binds, &mut open_prefs),
                            "PRACTICE" => practice_tab(ui),
                            _ => about_tab(ui),
                        },
                    );
                    ui.add_space(6.0);
                    if retro_btn(ui, "CLOSE  (ESC)", false).clicked() {
                        acts.push(Action::ToggleHelp);
                    }
                });
            });
        if let Some(k) = tour {
            self.tour_start(k);
        }
        if open_prefs {
            acts.push(Action::TogglePrefs);
        }
    }
}

fn tour_tab(ui: &mut egui::Ui, pick: &mut Option<u8>) {
    section_header(ui, "WANT A TOUR?");
    para(ui, "A one-minute guided look at the screen. It shows one piece at a time. Press ESC to stop.", pal().ink);
    ui.add_space(4.0);
    if retro_btn(ui, "TAKE THE TOUR", true).clicked() {
        *pick = Some(0);
    }
    if PRACTICE {
        ui.add_space(8.0);
        para(ui, "The practice tools have their own tour. Take it when you want to loop and slow down music.", pal().ink);
        if retro_btn(ui, "PRACTICE TOUR", false).clicked() {
            *pick = Some(1);
        }
    }
    ui.add_space(8.0);
    para(ui, "Prefer to read? The other tabs say the same things.", pal().ink2);
}

fn start_tab(ui: &mut egui::Ui) {
    section_header(ui, "WHAT THIS IS");
    para(
        ui,
        "A small player made for practicing music. It plays Tidal, SoundCloud, YouTube and your own files. The retro look is on purpose. Nothing here is required, so skip to what you need.",
        pal().ink,
    );
    section_header(ui, "THE 30-SECOND TOUR");
    points(
        ui,
        &[
            "The left window is the LIBRARY. The tabs at the top switch the source.",
            "The middle window is the PLAYER. Play, skip, seek and set the volume. Click the cover to see the song big.",
            "The right window is the QUEUE. Drag a row to reorder it.",
            "Right-click almost anything for more: songs, the seek bar, the skin button, the visualizer.",
            "Right-click a song to choose PLAY NEXT, ADD TO QUEUE, add it to a playlist or copy its link.",
        ],
    );
    section_header(ui, "NOW PLAYING");
    para(
        ui,
        "Click the cover or press A for the big view. It shows large art, lyrics and a visualizer, made for listening.",
        pal().ink,
    );
    points(
        ui,
        &[
            "Press L to show lyrics next to the cover. Click a line to jump there.",
            "The spectrum icon turns the visualizer on and off. Right-click it for size, bars and colors.",
            "The skin button is here too, so you can change the look without leaving.",
            "In practice mode your loop shows on the seek bar. Right-click the bar to set one.",
            "The small visualizer in the player window has its own settings. Right-click it to change them.",
        ],
    );
    section_header(ui, "WHERE THE MUSIC COMES FROM");
    points(
        ui,
        &[
            "TIDAL: sign in with your own subscription. The sign-in stays on this computer.",
            "SOUNDCLOUD and YOUTUBE: press GET YT-DLP once to fetch a free helper. Then search or paste links.",
            "FILES: drag audio files or folders onto the window.",
            "LISTS: your own playlists. One list can mix songs from every source.",
        ],
    );
    section_header(ui, "MAKE IT YOURS");
    points(
        ui,
        &[
            "Skins, PREFERENCES, storage and the log are in the BACKLINE menu at the top of the library.",
            "In the big view, right-click the spectrum icon for bars, waveform, colors and size.",
            "Everything saves by itself.",
        ],
    );
}

fn keys_tab(ui: &mut egui::Ui, binds: &[Option<egui::Key>], open_prefs: &mut bool) {
    para(ui, "Keys work whenever you are not typing in a box. You can change them in PREFERENCES.", pal().ink2);
    para(ui, "CTRL+K (CMD+K on a Mac) opens the command palette. Type what you want to do and press ENTER. The keys 1 2 3 4 switch between LIBRARY, PLAYER, ALBUM and MINI.", pal().ink);
    if retro_btn(ui, "CHANGE SHORTCUTS", false).clicked() {
        *open_prefs = true;
    }
    for (g, name) in ["PLAYING", "NOW PLAYING", "PRACTICE"].iter().enumerate() {
        if g == 2 && !PRACTICE {
            continue;
        }
        section_header(ui, name);
        for (i, c) in crate::prefs::CMDS.iter().enumerate() {
            if c.group as usize == g {
                let k = binds.get(i).copied().flatten().map_or("(none)".to_string(), crate::prefs::key_label);
                para(ui, &format!("- {}  {}", k, c.label), pal().ink);
            }
        }
        if g == 0 {
            para(ui, "- F1  this help", pal().ink);
        }
        if g == 1 {
            para(ui, "- ESC  back", pal().ink);
        }
    }
}

fn practice_tab(ui: &mut egui::Ui) {
    section_header(ui, "THE IDEA");
    para(ui, "Loop a hard spot, slow it down, then speed it up as it gets clean. Turn practice mode on with the PRACTICE chip or the P key.", pal().ink);
    section_header(ui, "THE FOUR TOOLS");
    points(
        ui,
        &[
            "LOOP: [ and ] set the ends, and \\ turns it on. Right-click the seek bar for more. Each track remembers its loops.",
            "SPEED: the UP and DOWN arrows change it 5 percent at a time. The pitch stays the same.",
            "PITCH: move by semitones or 10-cent steps. FIND TUNING lines up tracks that aren't at A440.",
            "TIME: a metronome with count-in, and a focus timer for work blocks with short rests.",
        ],
    );
    section_header(ui, "KEEPING TRACK");
    points(
        ui,
        &[
            "TUNES is your repertoire, with charts and recordings. DIARY shows what you practiced.",
            "Practice time counts only while a loop plays in practice mode.",
        ],
    );
}

fn about_tab(ui: &mut egui::Ui) {
    section_header(ui, "VERSION");
    let b = crate::update::build();
    let v = if b > 0 {
        format!("Backline {} build {}", crate::VERSION, b)
    } else {
        format!("Backline {} (local build)", crate::VERSION)
    };
    para(ui, &v, pal().ink);
    para(ui, "Backline checks for updates by itself. It installs them the next time you open it, or when you click the bar at the top.", pal().ink2);
    section_header(ui, "WHO MADE IT");
    para(ui, "Built by Daniel Simmons.", pal().ink);
    section_header(ui, "WHY IT EXISTS");
    para(
        ui,
        "Built for personal practice. It is a fast, steady player with loops, slow-down and a calm screen that stays out of your way. Written in Rust.",
        pal().ink,
    );
    section_header(ui, "PRIVACY");
    points(
        ui,
        &[
            "Loops, speed, seeks, playlists and your diary stay on this computer.",
            "Nothing is reported to Tidal, SoundCloud or YouTube about what you play.",
            "The only thing sent out on its own is the update check to GitHub.",
        ],
    );
    section_header(ui, "WHERE YOUR STUFF IS");
    para(ui, &crate::api::config_dir().display().to_string(), pal().ink2);
    para(ui, "The file library.json holds your tunes, loops, playlists and diary. Back it up to keep them safe.", pal().ink2);
    section_header(ui, "FINE PRINT");
    para(
        ui,
        "Backline is unofficial and not tied to Tidal, SoundCloud or YouTube. You need your own Tidal subscription. Using YouTube or SoundCloud through Backline may go against their terms. Personal use is at your own discretion.",
        pal().dim,
    );
    section_header(ui, "BUILT WITH");
    para(ui, "egui, symphonia, rodio, yt-dlp, and the HT-Demucs model for stem splitting.", pal().ink2);
}
