//! The setup wizard, on the very first run: how it looks, listening or practice, your music, where the program
//! lives (it can copy itself somewhere proper and make shortcuts), then the tour. RUN SETUP AGAIN in Preferences
//! brings it back.

use super::*;

const STEPS: usize = 6;

/// Where Backline installs itself: a folder of its own among the user's programs.
pub(crate) fn install_dir() -> PathBuf {
    #[cfg(windows)]
    {
        dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("Programs").join("Backline")
    }
    #[cfg(not(windows))]
    {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join("Applications").join("Backline")
    }
}

/// The program's file name where it is installed.
fn exe_name() -> &'static str {
    if cfg!(windows) {
        "backline.exe"
    } else {
        "backline"
    }
}

/// Is this copy already the installed one?
pub(crate) fn installed() -> bool {
    let here = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    match (here, std::fs::canonicalize(install_dir()).ok()) {
        (Some(h), Some(i)) => std::fs::canonicalize(h).is_ok_and(|h| h == i),
        _ => false,
    }
}

/// Copy the program into its folder and make shortcuts to it (Start Menu, and the desktop if asked). Returns the
/// installed program's path.
pub(crate) fn install(desktop: bool) -> Result<PathBuf, String> {
    let from = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = install_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let to = dir.join(exe_name());
    std::fs::copy(&from, &to).map_err(|e| format!("could not copy the program: {}", e))?;
    #[cfg(windows)]
    {
        let mut links = Vec::new();
        if let Some(start) = dirs::data_dir() {
            links.push(start.join("Microsoft").join("Windows").join("Start Menu").join("Programs").join("Backline.lnk"));
        }
        if desktop {
            if let Some(d) = dirs::desktop_dir() {
                links.push(d.join("Backline.lnk"));
            }
        }
        for link in links {
            shortcut(&to, &link);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = desktop;
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&to, std::fs::Permissions::from_mode(0o755));
    }
    Ok(to)
}

/// A Windows shortcut to `target` at `link` (made by the shell, quietly).
#[cfg(windows)]
fn shortcut(target: &std::path::Path, link: &std::path::Path) {
    use std::os::windows::process::CommandExt;
    const NO_WINDOW: u32 = 0x0800_0000;
    let q = |p: &std::path::Path| p.display().to_string().replace('\'', "''");
    let script = format!(
        "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{}'); $s.TargetPath='{}'; $s.WorkingDirectory='{}'; $s.Description='Backline'; $s.Save()",
        q(link),
        q(target),
        q(target.parent().unwrap_or(target))
    );
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(NO_WINDOW)
        .status();
}

impl App {
    /// The wizard, while it is open.
    pub(crate) fn setup_overlay(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        let Some(step) = self.setup_step else { return };
        let mut go: i32 = 0;
        let mut finish = false;
        let mut tour = false;
        crate::credits::panel(ctx, "setup", 600.0, |ui| {
            title_line(ui, &format!("SET UP BACKLINE   {}/{}", step + 1, STEPS), 3.0, pal().ink);
            ui.add_space(4.0);
            match step {
                0 => {
                    para(ui, "Welcome to Backline: a retro player for Tidal, your own files, YouTube and SoundCloud, made for practising music.", pal().ink);
                    para(
                        ui,
                        "A few quick choices and you're set. Everything here can be changed later in Preferences.",
                        pal().ink2,
                    );
                }
                1 => {
                    section_header(ui, "HOW IT LOOKS");
                    para(ui, "Pick a skin (more live under the BACKLINE tab, Winamp skins too).", pal().ink2);
                    ui.horizontal_wrapped(|ui| {
                        let cur = SKIN.load(Ordering::Relaxed);
                        for (i, name) in SKIN_NAMES.iter().enumerate() {
                            if retro_btn(ui, name, cur == i).clicked() {
                                acts.push(Action::Theme(i, 0));
                            }
                        }
                    });
                    section_header(ui, "SIZE OF EVERYTHING");
                    ui.horizontal_wrapped(|ui| {
                        let now = (ctx.zoom_factor() * 100.0).round() as i32;
                        for pct in [90, 100, 115, 130, 150] {
                            if retro_btn_w(ui, &format!("{}%", pct), 64.0, now == pct).clicked() {
                                ctx.set_zoom_factor(pct as f32 / 100.0);
                                self.dirty = true;
                            }
                        }
                    });
                }
                2 => {
                    section_header(ui, "LISTENING OR PRACTISING");
                    if PRACTICE {
                        para(ui, "PRACTICE opens the tools for learning from recordings: loops, slow-down, pitch, the metronome, tunes, chords and more. LISTEN keeps it a plain player. P switches any time.", pal().ink2);
                        ui.horizontal_wrapped(|ui| {
                            if retro_btn(ui, "LISTEN", !self.practice).clicked() && self.practice {
                                acts.push(Action::TogglePractice);
                            }
                            if retro_btn(ui, "PRACTICE", self.practice).clicked() && !self.practice {
                                acts.push(Action::TogglePractice);
                            }
                        });
                    } else {
                        para(
                            ui,
                            "This is the plain player. The practice edition adds loops, slow-down, tunes and chords.",
                            pal().ink2,
                        );
                    }
                    section_header(ui, "OPENING SOUND");
                    ui.horizontal_wrapped(|ui| {
                        for (m, name) in [(0u8, "OFF"), (1, "BACKLINE CHIME")] {
                            if retro_btn(ui, name, self.intro == m).clicked() {
                                acts.push(Action::Intro(m));
                            }
                        }
                    });
                }
                3 => {
                    section_header(ui, "YOUR MUSIC");
                    let signed = self.auth == Auth::In;
                    para(
                        ui,
                        if signed {
                            "Tidal: signed in."
                        } else {
                            "Tidal: not signed in. The library window has the sign-in; Backline works without it too (your files, YouTube, SoundCloud)."
                        },
                        pal().ink2,
                    );
                    para(ui, "Have music on this computer? Add its folder and it shows under FILES.", pal().ink2);
                    ui.horizontal_wrapped(|ui| {
                        if retro_btn(ui, "ADD A MUSIC FOLDER...", false).clicked() {
                            acts.push(Action::AddFolder);
                        }
                    });
                    if !self.store.folders.is_empty() {
                        for f in self.store.folders.clone() {
                            para(ui, &f, pal().dim);
                        }
                    }
                }
                4 => {
                    section_header(ui, "WHERE BACKLINE LIVES");
                    let dir = install_dir();
                    if installed() {
                        para(ui, &format!("Backline is installed in {}.", dir.display()), pal().ink2);
                    } else {
                        para(
                            ui,
                            &format!(
                                "Backline is one program file. INSTALL copies it to {} {} and opens it from there; updates arrive there on their own.",
                                dir.display(),
                                if cfg!(windows) { "with a Start Menu shortcut," } else { "" }
                            ),
                            pal().ink2,
                        );
                        if cfg!(windows) && check_box(ui, "A SHORTCUT ON THE DESKTOP TOO", self.setup_desktop).clicked() {
                            self.setup_desktop = !self.setup_desktop;
                        }
                        ui.horizontal_wrapped(|ui| {
                            if retro_btn(ui, "INSTALL", true).clicked() {
                                match install(self.setup_desktop) {
                                    Ok(to) => {
                                        // the setup carries on in the installed copy where it left off
                                        self.setup_step = Some(5);
                                        self.save_settings();
                                        self.store.save();
                                        if std::process::Command::new(&to).spawn().is_ok() {
                                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                        }
                                    }
                                    Err(e) => self.setup_note = e,
                                }
                            }
                            crate::views::dim_line(ui, "OR NEXT TO KEEP IT WHERE IT IS", 1.0, pal().dim);
                        });
                        if !self.setup_note.is_empty() {
                            para(ui, &self.setup_note, pal().red);
                        }
                    }
                    para(
                        ui,
                        &format!("Your settings and library are kept in {}.", crate::api::config_dir().display()),
                        pal().dim,
                    );
                }
                _ => {
                    para(
                        ui,
                        "All set. The tour shows each part working, with a song so you can hear it (about two minutes).",
                        pal().ink,
                    );
                    ui.horizontal_wrapped(|ui| {
                        if retro_btn(ui, "TAKE THE TOUR", true).clicked() {
                            finish = true;
                            tour = true;
                        }
                        if retro_btn(ui, "START", false).clicked() {
                            finish = true;
                        }
                    });
                }
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if step > 0 && retro_btn(ui, "BACK", false).clicked() {
                    go = -1;
                }
                if step + 1 < STEPS as u8 && retro_btn(ui, "NEXT", true).clicked() {
                    go = 1;
                }
                if step + 1 < STEPS as u8 && retro_btn(ui, "SKIP SETUP", false).clicked() {
                    finish = true;
                }
            });
        });
        if finish {
            self.setup_step = None;
            self.tour_seen = true;
            self.dirty = true;
            if tour {
                self.tour_start(0);
            }
        } else if go != 0 {
            self.setup_step = Some((step as i32 + go).clamp(0, STEPS as i32 - 1) as u8);
            self.dirty = true;
        }
    }
}
