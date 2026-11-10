//! The library's navigation (the section bar, ALL PAGES, the queue window's header) and the music pages: FILES,
//! YOUTUBE, SOUNDCLOUD, your lists, MY TRACKS, CONTINUE. The practice pages are in the files beside it, declared
//! here so they share its helpers: tunes.rs, diary.rs, loops_page.rs, practice.rs (the practice panel),
//! text_field.rs (the text box) and notes_box.rs.

use crate::api::Track;
use crate::extras::{Knob, CHAN_NAMES};
use crate::font::{ptext, ptext_fit, spx, text_w};
use crate::sources;
use crate::stems;
use crate::store::{self, Ext};
use crate::tools::{F_LOOK, F_LOOP_A, F_LOOP_B, F_SPEED, F_TR_LOOPS, F_TR_STEP};
pub(crate) use crate::widgets::{dim_line, form_label, label};
use crate::{
    bh, cache, chip, col_header, col_on, fill_rect, fmt_t, fmt_time, inset, lcd_box, list_row, menu_item, outline, pal, para,
    play_buttons, retro_btn, retro_btn_w, section_header, tab_row, table_header, title_line, track_cells, window_deco, Action,
    App, Ed, RowState, Sec, Side, Tip, PRACTICE,
};
use crate::{check_box, dropdown};
use eframe::egui::{self, Align, Pos2, Rect, Sense, Vec2};
use egui::{Color32, Stroke};
use std::collections::HashMap;
use std::sync::atomic::Ordering;

#[path = "text_field.rs"]
mod text_field;
pub(crate) use text_field::*;
#[path = "notes_box.rs"]
mod notes_box;
pub(crate) use notes_box::*;
#[path = "diary.rs"]
mod diary;
pub(crate) use diary::*;
#[path = "loops_page.rs"]
mod loops_page;
pub(crate) use loops_page::*;
#[path = "tunes.rs"]
mod tunes;
#[path = "practice.rs"]
mod practice;

/// "45 SEC", "12 MIN", "1 H 05 MIN"
/// 0 = a file, 2 = SoundCloud, 1 = YouTube.
fn heart_group(e: &Ext) -> u8 {
    if e.kind == "file" {
        0
    } else if e.src.contains("soundcloud.com") {
        2
    } else {
        1
    }
}

fn mins_text(secs: u32) -> String {
    match secs {
        0..=59 => format!("{} SEC", secs),
        60..=3599 => format!("{} MIN", secs / 60),
        _ => format!("{} H {:02} MIN", secs / 3600, secs % 3600 / 60),
    }
}

fn src_is_link(s: &str) -> bool {
    s.starts_with("http")
}

// text-field ids (only one field has the keyboard at a time)
const F_NEW_TUNE: u32 = 1;
const F_T_NAME: u32 = 2;
const F_T_KEY: u32 = 3;
const F_T_NOTES: u32 = 4;
const F_V_NOTE: u32 = 5;
const F_YT: u32 = 6;
const F_TUNE: u32 = 7;
const F_MINS: u32 = 8;
const F_BPM: u32 = 9;
const F_NOTE: u32 = 10;
const F_SEC: u32 = 11;
pub(crate) const F_CHART: u32 = 12;
pub(crate) const F_IREAL: u32 = 13;
const F_SC: u32 = 14;
const F_NEW_PL: u32 = 50;

/// "LABEL  [ text field ]" on one row (or a taller multi-line box).
pub(crate) fn field_row(
    ui: &mut egui::Ui,
    ed: &mut Ed,
    id: u32,
    label: &str,
    text: &mut String,
    hint: &str,
    tall: Option<f32>,
) -> FieldOut {
    let h = tall.unwrap_or(bh());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let ly = if tall.is_some() { rect.min.y + 13.0 } else { rect.center().y };
    ptext(ui.painter(), Pos2::new(rect.min.x + 4.0, ly), Align::Min, label, 2.0, pal().ink2);
    let fr = Rect::from_min_max(Pos2::new(rect.min.x + 76.0, rect.min.y), rect.max);
    field(ui, ed, id, text, fr, hint, tall.is_some())
}

fn stars_text(n: u8) -> String {
    let n = n.min(5) as usize;
    format!("{}{}", "*".repeat(n), ".".repeat(5 - n))
}

/// The lighter second row under a tab row: plain words, the open one underlined. Squeezes to fit a narrow window.
fn sub_tabs(ui: &mut egui::Ui, items: &[(Sec, &str)], cur: usize) -> Option<usize> {
    if crate::winamp_ui::chrome_on() {
        let names: Vec<&str> = items.iter().map(|s| s.1).collect();
        return crate::segments(ui, &names, cur);
    }
    crate::font::keep_caps(|| {
        let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::hover());
        let pad = 18.0;
        let total: f32 = items.iter().map(|s| if s.1 == "|" { 14.0 } else { text_w(s.1, 2.0) + pad }).sum();
        let shrink = ((row.width() - 8.0) / total).min(1.0);
        let (mut x, mut hit) = (row.min.x + 4.0, None);
        for (i, (_, name)) in items.iter().enumerate() {
            // "|" is a bar between groups of tabs, not a tab
            if *name == "|" {
                let bx = x + 6.0 * shrink;
                fill_rect(
                    ui.painter(),
                    Rect::from_min_max(Pos2::new(bx, row.min.y + 4.0), Pos2::new(bx + 2.0, row.max.y - 4.0)),
                    pal().ink2,
                );
                x += 14.0 * shrink;
                continue;
            }
            let w = (text_w(name, 2.0) + pad) * shrink;
            let r = Rect::from_min_max(Pos2::new(x, row.min.y), Pos2::new(x + w, row.max.y));
            let resp = ui
                .interact(r, ui.id().with(("subtab", i, *name)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let on = i == cur;
            // a small raised button each, a gap between; the one you are on pressed in and lit
            let b = r.shrink2(Vec2::new(2.0, 1.0));
            if on {
                crate::inset(ui.painter(), b, pal().display);
            } else {
                crate::raised_h(ui.painter(), b, resp.is_pointer_button_down_on(), resp.hovered());
            }
            let col = if on {
                pal().bar_txt
            } else if resp.hovered() {
                pal().ink
            } else {
                pal().ink2
            };
            crate::ui_text(ui.painter(), b.center(), Align::Center, name, 2.0, w - 8.0, col);
            if resp.clicked() {
                hit = Some(i);
            }
            x += w;
        }
        hit
    })
}

/// A page's group tabs (CHORDS, LINES): each group's name, or its page's own name when it has only the one, and
/// for hovering, the pages in it.
pub(crate) fn group_tabs(groups: &[(&'static str, &[(u8, &'static str)])]) -> (Vec<&'static str>, Vec<String>) {
    let names = groups.iter().map(|g| if g.1.len() == 1 { g.1[0].1 } else { g.0 }).collect();
    let tips = groups
        .iter()
        .map(|g| if g.1.len() == 1 { String::new() } else { g.1.iter().map(|p| p.1).collect::<Vec<_>>().join(", ") })
        .collect();
    (names, tips)
}

/// The button that lists every PRACTICE page (not "GO TO": the search box's button is GO).
const PAGES: &str = "ALL PAGES";

/// The room the ALL PAGES button takes in a row, the gap before it included.
pub(crate) fn pages_w() -> f32 {
    text_w(PAGES, 2.0) + 70.0
}

/// Where a line of the ALL PAGES list takes you: the PRACTICE tab, then the page's own number
/// (CHORDS: the page, then one of its own tabs, or `ANY` to leave it on the one it was last on).
#[derive(Clone, Copy, PartialEq)]
enum Go {
    Tunes(u8),
    Diary(u8),
    Loops,
    Reading(u8),
    Chords(u8, u8),
    Ear,
    Lines(u8),
    Melodies(u8),
    Chart,
}

const ANY: u8 = 255;

/// The ALL PAGES list: every PRACTICE page under its tab's name (as headings), a page's own tabs set in under it.
fn go_list(plan: bool) -> (Vec<&'static str>, Vec<(usize, &'static str)>, Vec<Go>) {
    use Go::*;
    let mut diary = vec![("DIARY", Diary(0)), ("ROUTINE", Diary(1)), ("LISTENING", Diary(2))];
    if plan {
        diary.push(("MY PLAN", Diary(3)));
    }
    let groups: Vec<(&str, Vec<(&str, Go)>)> = vec![
        ("TUNES", vec![("YOUR TUNES", Tunes(0)), ("RECOMMENDED", Tunes(1)), ("CHART", Chart)]),
        ("DIARY", diary),
        ("LOOPS", vec![("LOOPS & BOOKMARKS", Loops)]),
        ("READING", vec![("THE BOOK", Reading(0)), ("MADE-UP LINES", Reading(1))]),
        (
            "CHORDS",
            vec![
                ("FIND A CHORD", Chords(0, ANY)),
                ("CHORD ANALYZER", Chords(1, ANY)),
                ("TRIADS", Chords(2, ANY)),
                ("  MAP", Chords(2, 0)),
                ("  VOICE LEADING", Chords(2, 1)),
                ("  OVER A BASS", Chords(2, 2)),
                ("VOICINGS", Chords(3, ANY)),
                ("  INVERSIONS", Chords(3, 0)),
                ("  WALK THE CHANGES", Chords(3, 1)),
                ("CLUSTERS", Chords(4, ANY)),
                ("  SCALES", Chords(4, 0)),
                ("  FIXED CLUSTER", Chords(4, 1)),
                ("  FIXED BASS", Chords(4, 2)),
                ("  IN A TUNE", Chords(4, 3)),
                ("  BUILD", Chords(4, 4)),
                ("  MY CHORDS", Chords(4, 5)),
                ("SCALE HARMONY", Chords(6, ANY)),
                ("PROGRESSIONS", Chords(8, ANY)),
                ("PASSING CHORDS", Chords(5, ANY)),
                ("COMPING MOVES", Chords(7, ANY)),
                ("  GOSPEL", Chords(7, 0)),
                ("  QUARTAL & CLUSTERS", Chords(7, 1)),
            ],
        ),
        ("EAR", vec![("EAR TRAINING", Ear)]),
        (
            "LINES",
            vec![
                ("SLONIMSKY'S DIARY", Lines(0)),
                ("ATONAL", Lines(5)),
                ("SCALE FINGERINGS", Lines(1)),
                ("ARPEGGIOS", Lines(4)),
                ("PENTATONICS & MORE", Lines(2)),
                ("SUPERIMPOSE", Lines(3)),
                ("TRIAD PAIRS", Lines(6)),
            ],
        ),
        ("CHORD MELODIES", vec![("SONGS", Melodies(0)), ("MOVES", Melodies(1))]),
    ];
    let (mut items, mut heads, mut gos) = (Vec::new(), Vec::new(), Vec::new());
    for (head, pages) in groups {
        heads.push((items.len(), head));
        for (name, g) in pages {
            items.push(name);
            gos.push(g);
        }
    }
    (items, heads, gos)
}

impl App {
    /// ALL PAGES: every PRACTICE page in one list, the one you are on lit; picking one takes you straight there.
    pub(crate) fn go_to(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let (items, heads, gos) = go_list(crate::plan::have_plan());
        let sub = |t: u8| match t {
            2 => self.tr_page,
            3 => self.vc_page,
            4 => self.cl_view,
            7 => self.mv_page,
            _ => ANY,
        };
        let now = match self.sec {
            Sec::Tunes => Go::Tunes(self.tunes_tab),
            Sec::Diary => Go::Diary(self.diary_tab),
            Sec::Loops => Go::Loops,
            Sec::Reading => Go::Reading(self.rd_mode),
            Sec::Chords => Go::Chords(self.chords_tab, sub(self.chords_tab)),
            Sec::Ear => Go::Ear,
            Sec::Lines => Go::Lines(self.ln_tab),
            Sec::Chart => Go::Chart,
            _ => Go::Melodies(self.cm_tab),
        };
        let sel = gos.iter().position(|g| *g == now).unwrap_or(usize::MAX);
        // the button names the page you are on (cut short to fit), so it reads as where you are and the way to
        // the others; a page's own tab with its page ("Triads: Map")
        let face = match items.get(sel) {
            Some(it) => match it.strip_prefix("  ") {
                Some(sub) => {
                    let parent = items[..sel].iter().rev().find(|s| !s.starts_with("  ")).copied().unwrap_or("");
                    format!("{}: {}", parent, sub)
                }
                None => it.to_string(),
            },
            None => PAGES.to_string(),
        };
        let at = ui.cursor().min;
        let picked = crate::menu_list_w(ui, "practice_go", &face, &items, sel, &heads, pages_w());
        let _ = ui
            .interact(Rect::from_min_size(at, Vec2::new(pages_w(), bh())), ui.id().with("practice_go_tip"), Sense::hover())
            .tip("The page you're on. Click to go to any practice page.");
        let Some(i) = picked else { return };
        let g = gos[i];
        let sec = match g {
            Go::Tunes(_) => Sec::Tunes,
            Go::Diary(_) => Sec::Diary,
            Go::Loops => Sec::Loops,
            Go::Reading(_) => Sec::Reading,
            Go::Chords(..) => Sec::Chords,
            Go::Ear => Sec::Ear,
            Go::Lines(_) => Sec::Lines,
            Go::Melodies(_) => Sec::Melodies,
            Go::Chart => Sec::Chart,
        };
        if sec != self.sec {
            acts.push(Action::Section(sec));
        }
        match g {
            Go::Tunes(t) => self.tunes_tab = t,
            Go::Diary(t) => self.diary_tab = t,
            Go::Reading(t) => {
                // (as its own tabs do: what was playing for the other one stops)
                if self.rd_mode != t {
                    self.rd_mode = t;
                    self.player.send(crate::player::Cmd::StopOnce);
                    self.rd_play = None;
                }
            }
            Go::Chords(t, s) => {
                self.chords_tab = t;
                if s != ANY {
                    match t {
                        2 => self.tr_page = s,
                        3 => self.vc_page = s,
                        4 => self.cl_view = s,
                        _ => self.mv_page = s,
                    }
                }
            }
            Go::Lines(t) => self.ln_tab = t,
            Go::Melodies(t) => self.cm_tab = t,
            Go::Loops | Go::Ear | Go::Chart => {}
        }
    }

    // ------------------------------------------------------------ switchers
    /// Two levels: MUSIC (the sources and your lists) and PRACTICE (tunes, diary) on top, that group's lists under it.
    /// Each group comes back to the list you last had open in it. The plain build has only MUSIC, so only its row shows.
    pub(crate) fn section_bar(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        const PRACTICE_SECS: [(Sec, &str); 9] = [
            (Sec::Tunes, "TUNES"),
            (Sec::Chart, "CHART"),
            (Sec::Diary, "DIARY"),
            (Sec::Loops, "LOOPS"),
            (Sec::Reading, "READING"),
            (Sec::Chords, "CHORDS"),
            (Sec::Ear, "EAR"),
            (Sec::Lines, "LINES"),
            (Sec::Melodies, "CHORD MELODIES"),
        ];
        // the BACKLINE tab covers its own page and the pages opened from it
        let side = self.side.is_some();
        let in_practice = PRACTICE_SECS.iter().any(|s| s.0 == self.sec);
        let tabs: &[&str] = if PRACTICE { &["MUSIC", "PRACTICE", "BACKLINE"] } else { &["MUSIC", "BACKLINE"] };
        let last = tabs.len() - 1;
        let cur = if side { last } else { (PRACTICE && in_practice) as usize };
        let top = ui.cursor().min.y;
        if let Some(i) = tab_row(ui, tabs, cur) {
            if i == last {
                acts.push(Action::BacklinePage);
            } else if i != cur {
                let to_practice = PRACTICE && i == 1;
                let music: Vec<Sec> = [Sec::Mine, Sec::Lists, Sec::Tidal, Sec::Files, Sec::Sc, Sec::Yt]
                    .into_iter()
                    .filter(|s| *s != Sec::Tidal || !self.offline)
                    .collect();
                let back = if to_practice { self.last_practice } else { self.last_music };
                let ok = if to_practice { PRACTICE_SECS.iter().any(|s| s.0 == back) } else { music.contains(&back) };
                let to = if ok {
                    back
                } else if to_practice {
                    Sec::Tunes
                } else {
                    music[0]
                };
                acts.push(Action::Section(to));
            }
        }
        let r = ui.min_rect();
        crate::tour::mark("APPMENU", Rect::from_min_max(Pos2::new(r.min.x, top), Pos2::new(r.max.x, ui.cursor().min.y)));
        if side {
            // the BACKLINE tab's own pages
            let items = [(Sec::Tidal, "SETTINGS"), (Sec::Tidal, "SKINS"), (Sec::Tidal, "LOG")];
            let cur = match self.side {
                Some(Side::Skins) => 1,
                Some(Side::Log) => 2,
                _ => 0,
            };
            if let Some(i) = sub_tabs(ui, &items, cur) {
                if i != cur {
                    acts.push(match i {
                        0 => Action::BacklinePage,
                        1 => Action::WaOpen,
                        _ => Action::ShowLog,
                    });
                }
            }
            return;
        }
        if PRACTICE && in_practice {
            let cur = PRACTICE_SECS.iter().position(|s| s.0 == self.sec).unwrap_or(0);
            // the tabs, then ALL PAGES at the end of the row: every page in one list
            let go_w = pages_w();
            let mut hit = None;
            ui.horizontal(|ui| {
                let w = (ui.available_width() - go_w).max(120.0);
                ui.allocate_ui(Vec2::new(w, 24.0), |ui| hit = sub_tabs(ui, &PRACTICE_SECS, cur));
                self.go_to(ui, acts);
            });
            if let Some(i) = hit {
                acts.push(Action::Section(PRACTICE_SECS[i].0));
            }
            return;
        }
        // everything you keep (MY TRACKS, LISTS), then each source on its own
        // each source, then a bar, then everything you keep (MY TRACKS, LISTS)
        let mut items: Vec<(Sec, &str)> = vec![(Sec::Tidal, "TIDAL")];
        items.extend([
            (Sec::Files, "FILES"),
            (Sec::Sc, "SOUNDCLOUD"),
            (Sec::Yt, "YT"),
            (Sec::Mine, "|"),
            (Sec::Mine, "MY TRACKS"),
            (Sec::Lists, "LISTS"),
        ]);
        let cur = items.iter().position(|s| s.0 == self.sec && s.1 != "|").unwrap_or(0);
        if let Some(i) = sub_tabs(ui, &items, cur) {
            match items[i].0 {
                // TIDAL goes back to the Tidal page you had
                Sec::Tidal if self.page.is_none() => acts.push(Action::Home),
                s => acts.push(Action::Section(s)),
            }
        }
        // Tidal's own space: its home and your Tidal library, nothing from other sources mixed in
        if self.sec == Sec::Tidal {
            let on_library = self.page.as_ref().is_some_and(|p| p.library);
            let sub = [(Sec::Tidal, "HOME"), (Sec::Tidal, "MY LIBRARY")];
            if let Some(i) = sub_tabs(ui, &sub, on_library as usize) {
                acts.push(if i == 1 { Action::Library } else { Action::Home });
            }
        }
    }

    /// The box that filters your library down as you type (every word must match; typos forgiven), with X to
    /// clear it. One filter for the Tidal library, MY TRACKS and LISTS.
    pub(crate) fn filter_box(&mut self, ui: &mut egui::Ui, hint: &str) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            let (ic, _) = ui.allocate_exact_size(Vec2::new(22.0, bh()), Sense::hover());
            crate::icon_in(ui.painter(), ic.shrink(4.0), &crate::IC_FILTER, pal().ink2);
            // (the full width; room for the clear button only once there is something to clear)
            let w = (ui.available_width() - if self.lib_filter.is_empty() { 8.0 } else { 44.0 }).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let _ = field(ui, &mut self.ed, crate::tools::F_LIB_FILTER, &mut self.lib_filter, rect, hint, false);
            if !self.lib_filter.is_empty() && retro_btn_w(ui, "X", 30.0, false).tip("Clear the filter.").clicked() {
                self.lib_filter.clear();
            }
        });
        ui.add_space(4.0);
    }

    /// MY TRACKS: every song you liked, from every source (your Tidal likes, and the files, SoundCloud and YouTube
    /// songs you hearted), with a SOURCES row to leave any of them out.
    pub(crate) fn mine_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "MY TRACKS", 3.0, pal().ink);
        title_line(
            ui,
            if self.mine_stems { "every song split into stems, liked or not" } else { "every song you liked, from every source" },
            2.0,
            pal().ink2,
        );
        ui.horizontal_wrapped(|ui| {
            // the sources in a small list of ticks under one button, so the row never wraps
            let n_on = crate::SOURCES.iter().filter(|(k, _)| !self.mine_off.iter().any(|o| o == k)).count();
            let what = if n_on == crate::SOURCES.len() { "all".to_string() } else { format!("{} of {}", n_on, crate::SOURCES.len()) };
            let src = retro_btn(ui, &crate::font::as_label(&format!("Sources: {}", what)), n_on < crate::SOURCES.len())
                .tip("Choose which sources to show: Tidal, files, SoundCloud, YouTube.");
            let pop = ui.id().with("mine_sources");
            if src.clicked() {
                ui.memory_mut(|m| m.toggle_popup(pop));
            }
            egui::popup_below_widget(ui, pop, &src, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                ui.set_min_width(170.0);
                for (key, name) in crate::SOURCES {
                    let on = !self.mine_off.iter().any(|k| k == key);
                    if crate::check_box(ui, name, on).clicked() {
                        if on {
                            self.mine_off.push(key.to_string());
                        } else {
                            self.mine_off.retain(|k| k != key);
                        }
                        self.dirty = true;
                    }
                }
            });
            ui.add_space(8.0);
            if crate::check_box(ui, "WITH STEMS", self.mine_stems)
                .tip("Only the songs already split into drums, bass, vocals and the rest, liked or not.")
                .clicked()
            {
                self.mine_stems = !self.mine_stems;
                self.dirty = true;
            }
        });
        // the songs, made again only when what they come from changes (not every frame: thousands of them)
        let filter = self.lib_filter.trim().to_string();
        let key = (
            self.lib_page.as_ref().map_or(0, |p| std::sync::Arc::as_ptr(p) as usize ^ p.tracks.len().rotate_left(40)),
            self.store.hearts.len(),
            self.serial,
            (self.mine_stems, self.go_offline, self.offline),
            self.mine_off.join(","),
            filter.clone(),
        );
        if self.mine_cache.as_ref().is_none_or(|c| c.0 != key) {
            let built = self.mine_tracks(&filter);
            self.mine_cache = Some((key, std::sync::Arc::new(built)));
        }
        let Some((_, m)) = self.mine_cache.clone() else { return };
        let (tracks, all, waiting, unnamed) = (&m.0, m.1, m.2, m.3);
        if self.mine_stems && unnamed > 0 {
            dim_line(ui, &format!("{} MORE SPLIT SONGS SHOW HERE ONCE YOU PLAY THEM AGAIN", unnamed), 1.0, pal().dim);
        }
        if self.mine_stems && all == 0 {
            para(ui, "No songs split yet. Play a song, open PRACTICE > STEMS and press SPLIT THIS TRACK.", pal().ink2);
            return;
        }
        if self.go_offline {
            dim_line(ui, "OFFLINE MODE: ONLY THE SONGS ON THIS COMPUTER", 1.0, pal().dim);
        }
        ui.add_space(4.0);
        if waiting {
            dim_line(ui, "YOUR TIDAL LIKES SHOW ONCE TIDAL > MY LIBRARY HAS LOADED", 1.0, pal().dim);
        }
        if all == 0 {
            para(ui, "Nothing here yet. Click the heart on any song and it shows up here. That works for Tidal, your files, SoundCloud and YouTube.", pal().ink2);
            return;
        }
        self.filter_box(ui, "Filter by title, artist or album");
        if !filter.is_empty() {
            title_line(ui, &format!("{} of {} songs match", tracks.len(), all), 2.0, pal().ink2);
            if tracks.is_empty() {
                title_line(ui, "No songs match.", 2.0, pal().ink2);
                return;
            }
        } else {
            title_line(ui, &format!("{} songs", tracks.len()), 2.0, pal().ink2);
        }
        crate::play_buttons(ui, tracks, acts, false);
        ui.add_space(4.0);
        let playing = self.cur_track().map(|t| t.id);
        crate::tracks_list(ui, tracks, playing, &self.liked, None, &self.srcmap, acts);
        ui.add_space(20.0);
    }

    /// MY TRACKS' songs (the sources ticked, or the songs split into stems), filtered: (the songs shown, how many
    /// before the filter, still waiting for Tidal's, split songs not known by name yet).
    fn mine_tracks(&self, filter: &str) -> (Vec<Track>, usize, bool, usize) {
        let shows = |key: &str| !self.mine_off.iter().any(|k| k == key);
        let mut tracks: Vec<Track>;
        let (mut waiting, mut unnamed) = (false, 0);
        if self.mine_stems {
            // every split song this PC knows by name (one split before songs were remembered gets its name when played)
            tracks = Vec::new();
            for id in stems::split_ids() {
                match self.store.songs.get(&id) {
                    Some(v) if shows(&v.kind) => tracks.push(v.to_track()),
                    Some(_) => {}
                    None => unnamed += 1,
                }
            }
            tracks.sort_by_cached_key(|t| (t.artists_text().to_lowercase(), t.title.clone()));
        } else {
            tracks = self.store.hearts.iter().filter(|e| shows(&e.kind)).map(|e| e.to_track()).collect();
        }
        if shows("tidal") && !self.offline && !self.mine_stems {
            match &self.lib_page {
                Some(p) => tracks.extend(p.tracks.iter().cloned()),
                None => waiting = true,
            }
        }
        // offline mode: only what is on this computer (your files, and the songs kept)
        if self.go_offline {
            tracks.retain(|t| cache::has(t.id) || matches!(self.srcmap.get(&t.id), Some(crate::Src::File(_))));
        }
        let all = tracks.len();
        if !filter.is_empty() {
            let keep = crate::filtered(filter, &tracks, |t| vec![t.title.clone(), t.artists_text(), t.album.clone()], 3);
            tracks = keep.into_iter().map(|i| tracks[i].clone()).collect();
        }
        (tracks, all, waiting, unnamed)
    }

    /// "Pick up where you left off" in one press.
    pub(crate) fn continue_bar(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if !PRACTICE || !(self.cur.is_none() || self.stopped) {
            return;
        }
        let Some(l) = &self.store.last else { return };
        let txt = format!(
            "CONTINUE: {} - {}  @ {}  {}%",
            l.version.artist,
            l.version.title,
            fmt_time(l.pos),
            if l.speed == 0 { 100 } else { l.speed }
        );
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click());
        crate::raised_h(ui.painter(), rect, resp.is_pointer_button_down_on(), resp.hovered());
        if crate::winamp_ui::chrome_on() {
            // Continue: the song by the artist (a link to them, from Tidal) - where you were, and the speed if not 100%
            let p = ui.painter();
            let speed = if l.speed == 0 || l.speed == 100 { String::new() } else { format!(" at {}%", l.speed) };
            let parts = [
                ("Continue: ".to_string(), pal().ink2),
                (l.version.title.clone(), pal().ink),
                (" by ".to_string(), pal().ink2),
                (l.version.artist.clone(), pal().ink),
                (format!(" - {}{}", fmt_time(l.pos), speed), pal().ink2),
            ];
            let mut x = rect.min.x + 14.0;
            let mut artist = None;
            for (i, (s, col)) in parts.iter().enumerate() {
                let room = rect.max.x - 10.0 - x;
                if room < 20.0 {
                    break;
                }
                // (cut with "..." when it doesn't fit, at the same size as the rest of the line)
                crate::row_text(p, Pos2::new(x, rect.center().y), Align::Min, s, room, *col);
                let w = p
                    .layout_no_wrap(
                        s.to_string(),
                        egui::FontId::proportional((crate::ROW_H * 0.62 * crate::font::ui_scale()).max(11.0)),
                        *col,
                    )
                    .size()
                    .x
                    .min(room);
                if i == 3 && l.version.kind == "tidal" {
                    artist = Some(Rect::from_min_size(Pos2::new(x, rect.min.y + 4.0), Vec2::new(w, rect.height() - 8.0)));
                }
                x += w;
            }
            let mut on_artist = false;
            if let Some(ar) = artist {
                let a = ui
                    .interact(ar, ui.id().with("continue_artist"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                let y = ar.center().y + 0.42 * crate::font::text_px(2.0);
                let ucol = if a.hovered() { pal().bar_txt } else { pal().ink2 };
                ui.painter().line_segment([Pos2::new(ar.min.x, y), Pos2::new(ar.max.x, y)], egui::Stroke::new(1.0_f32, ucol));
                if a.clicked() {
                    acts.push(Action::GoTo(l.version.id, crate::GoTo::Artist));
                    on_artist = true;
                }
            }
            if !on_artist && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                acts.push(Action::Continue);
            }
            return;
        }
        ptext_fit(ui.painter(), rect.center(), Align::Center, &txt, 2.0, rect.width() - 16.0, pal().ink);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            acts.push(Action::Continue);
        }
    }

    fn right_tabs(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let inner = ui.max_rect();
        let chip_h = 16.0;
        let mut right = crate::title_right(inner);
        let mut place = |w: f32| {
            let r = crate::title_slot(inner, right, w, chip_h);
            right -= w + 6.0;
            r
        };
        let r_chart = place(48.0);
        let r_queue = place(48.0);
        if chip(ui, r_chart, "CHART", self.rtab == 1, "c_rchart").tip("The chord changes of the open tune.").clicked() {
            acts.push(Action::RightTab(1));
        }
        if chip(ui, r_queue, "QUEUE", self.rtab == 0, "c_rqueue").clicked() {
            acts.push(Action::RightTab(0));
        }
    }

    /// Called at the top of the queue window: in focus mode (no library, so no Chart page), the QUEUE / CHART chips
    /// in the window's title strip (`ui`, the panel's), the lead sheet on its page (`page`).
    pub(crate) fn right_header(&mut self, ui: &mut egui::Ui, page: &mut egui::Ui, acts: &mut Vec<Action>) -> bool {
        if !PRACTICE || !self.lib_hidden {
            return false;
        }
        self.right_tabs(ui, acts);
        if self.rtab == 1 {
            self.chart_ui(page, acts);
            true
        } else {
            false
        }
    }

    // ---------------------------------------------------------------- FILES
    /// `which`: 0 files, 1 YouTube clips, 2 kept SoundCloud tracks, 3 SoundCloud search results
    fn ext_list(&self, ui: &mut egui::Ui, acts: &mut Vec<Action>, which: u8) {
        let yt = which != 0 && which != 4;
        let hearts: Vec<Ext>;
        let list = match which {
            0 => &self.store.files,
            1 => &self.store.yt,
            2 => &self.store.sc,
            4..=6 => {
                hearts = self.store.hearts.iter().filter(|e| heart_group(e) == which - 4).cloned().collect();
                &hearts
            }
            7 => &self.yt_results,
            _ => &self.sc_results,
        };
        let playing = self.cur_track().map(|t| t.id);
        // the column names once a page: not again over the liked songs when the page's own list has them above
        let above = match which {
            4 => !self.store.files.is_empty(),
            5 => !self.store.yt.is_empty(),
            6 => !self.store.sc.is_empty(),
            _ => false,
        };
        let art = crate::row_arts_on();
        if !above {
            col_header(ui, true, art);
        }
        for (i, e) in list.iter().enumerate() {
            let state = if playing == Some(e.id) { RowState::Playing } else { RowState::Normal };
            let stored = if yt { cache::has(e.id) } else { true };
            crate::row_heart(self.liked.contains(&e.id));
            crate::row_stems(e.id);
            if art {
                crate::row_art(&store::file_cover(&e.kind, &e.src, &e.cover));
            }
            let r = list_row(
                ui,
                i,
                Some(i + 1),
                || track_cells(e.id, &e.title, &e.artist, "", &e.kind),
                || {
                    if e.dur == 0.0 && e.cover.is_empty() && (which == 2 || which == 3 || which == 6) {
                        "(loading info)".to_string()
                    } else if col_on(2) {
                        fmt_time(e.dur)
                    } else {
                        String::new()
                    }
                },
                state,
                false,
                stored,
            );
            if crate::heart_clicked() {
                acts.push(Action::ToggleLike(e.to_track()));
            }
            if let Some(r) = r {
                if r.clicked() {
                    let tracks: Vec<Track> = list.iter().map(|x: &Ext| x.to_track()).collect();
                    acts.push(Action::Play(tracks, i));
                }
                let liked = self.liked.contains(&e.id);
                r.context_menu(|ui| {
                    // (Edit info... comes with the song's own items)
                    crate::song_items(ui, acts, &e.to_track(), liked, false);
                    crate::link_item(ui, acts, crate::track_link(&e.to_track(), Some(e)));
                    if (4..=6).contains(&which) {
                        // (taking it out of LIKED is Remove from My Tracks, above)
                    } else if which == 3 || which == 7 {
                        // a saved playlist's page: take a song out of it (kept out when it's read again)
                        let open = if which == 7 { self.yt_cur.as_ref() } else { self.sc_cur.as_ref() };
                        if let Some(url) = open.filter(|u| self.store.lists.iter().any(|l| l.url == **u)) {
                            if menu_item(ui, "Remove from This Playlist") {
                                acts.push(Action::RemoveFromList(url.clone(), e.id));
                                ui.close_menu();
                            }
                        }
                        if which == 3 && menu_item(ui, "Save to My SoundCloud") {
                            acts.push(Action::ScKeep(e.id));
                            ui.close_menu();
                        }
                        if which == 7 && menu_item(ui, "Save to My Clips") {
                            acts.push(Action::YtKeep(e.id));
                            ui.close_menu();
                        }
                    } else if menu_item(ui, "Remove from This List") {
                        acts.push(Action::RemoveExt(e.id));
                        ui.close_menu();
                    }
                });
            }
        }
    }

    /// Saved playlist links of one kind ("sc" / "yt"); click opens, right-click removes.
    fn saved_lists(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, kind: &str) {
        let mine: Vec<(usize, store::SavedList)> =
            self.store.lists.iter().enumerate().filter(|(_, l)| l.kind == kind).map(|(i, l)| (i, l.clone())).collect();
        if mine.is_empty() {
            return;
        }
        section_header(ui, "PLAYLISTS");
        crate::cards_header(ui, "LENGTH", false);
        let mut queue: Option<(usize, bool)> = None;
        for (k, (i, l)) in mine.iter().enumerate() {
            let length = || {
                if l.items.is_empty() {
                    String::new()
                } else {
                    let total: f32 = l.items.iter().map(|e| e.dur).sum();
                    let n = l.items.len();
                    let time = if total > 0.0 { format!(", {}", crate::fmt_long(total)) } else { String::new() };
                    format!("{} song{}{}", n, if n == 1 { "" } else { "s" }, time)
                }
            };
            if let Some(r) = list_row(ui, k, Some(k + 1), || l.name.clone(), length, RowState::Normal, false, false) {
                if r.clicked() {
                    self.open_saved_list(l, acts);
                }
                r.context_menu(|ui| {
                    if !l.items.is_empty() {
                        if menu_item(ui, "Play Next") {
                            queue = Some((k, true));
                            ui.close_menu();
                        }
                        if menu_item(ui, "Add to Queue") {
                            queue = Some((k, false));
                            ui.close_menu();
                        }
                    }
                    crate::link_item(ui, acts, Some(l.url.clone()));
                    if menu_item(ui, "Remove from My Playlists") {
                        acts.push(Action::RemoveList(*i));
                        ui.close_menu();
                    }
                });
            }
        }
        // its songs, known to the player first, then into the queue
        if let Some((k, next)) = queue {
            let l = &mine[k].1;
            let items = l.shown();
            for e in &items {
                self.register(&e.to_version());
            }
            self.keep_all(items.iter().map(|e| e.id).collect());
            acts.push(Action::QueueTracks(l.name.clone(), items.iter().map(|e| e.to_track()).collect(), next));
        }
    }

    /// Open a saved playlist on its page: at once, from the songs kept with it (read from the web only the first
    /// time, or with REFRESH).
    fn open_saved_list(&mut self, l: &store::SavedList, acts: &mut Vec<Action>) {
        if l.items.is_empty() {
            self.read_list_again(&l.kind, &l.url, acts);
            return;
        }
        let items = l.shown();
        for e in &items {
            self.register(&e.to_version());
        }
        self.keep_all(items.iter().map(|e| e.id).collect());
        if l.kind == "sc" {
            self.sc_cur = Some(l.url.clone());
            self.sc_results = l.shown();
        } else {
            self.yt_cur = Some(l.url.clone());
            self.yt_title = l.name.clone();
            self.yt_results = l.shown();
        }
        self.list_ask = None;
    }

    /// Read a playlist from the web again (songs may have been added to it).
    fn read_list_again(&mut self, kind: &str, url: &str, acts: &mut Vec<Action>) {
        if kind == "sc" {
            acts.push(Action::ScOpenSet(url.to_string()));
        } else {
            self.yt_in = url.to_string();
            acts.push(Action::AddYt);
        }
    }

    /// A YouTube or SoundCloud playlist on its own page: a way back, its name and length, PLAY and SHUFFLE, all its
    /// songs, and taking it off your playlists (asked first).
    fn list_page(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, kind: &str) {
        let yt = kind == "yt";
        let url = if yt { self.yt_cur.clone() } else { self.sc_cur.clone() }.unwrap_or_default();
        let list: Vec<store::Ext> = if yt { self.yt_results.clone() } else { self.sc_results.clone() };
        let name =
            self.store.lists.iter().find(|l| l.url == url).map(|l| l.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(
                || if yt && !self.yt_title.is_empty() { self.yt_title.clone() } else { sources::list_name(&url) },
            );
        let back = |me: &mut Self| {
            if yt {
                me.yt_results.clear();
                me.yt_title.clear();
                me.yt_cur = None;
            } else {
                me.sc_results.clear();
                me.sc_cur = None;
            }
            me.list_ask = None;
        };
        ui.add_space(6.0);
        if crate::ibtn(ui, &crate::IC_BACK, "PLAYLISTS", false).tip("Back to your playlists.").clicked() {
            back(self);
            return;
        }
        ui.add_space(4.0);
        title_line(ui, &name, 3.0, pal().ink);
        let tracks: Vec<Track> = list.iter().map(|e| e.to_track()).collect();
        let total: f32 = list.iter().map(|e| e.dur).sum();
        title_line(
            ui,
            &format!(
                "{} songs{}",
                tracks.len(),
                if total > 0.0 { format!(", {}", crate::fmt_long(total)) } else { String::new() }
            ),
            2.0,
            pal().ink2,
        );
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            play_buttons(ui, &tracks, acts, false);
            if retro_btn(ui, "REFRESH", false).tip("Reload the playlist to get newly added songs.").clicked() {
                self.read_list_again(kind, &url, acts);
            }
            if self.list_ask.as_deref() != Some(url.as_str())
                && retro_btn(ui, "REMOVE PLAYLIST...", false)
                    .tip("Remove it from your playlists. Songs you saved stay.")
                    .clicked()
            {
                self.list_ask = Some(url.clone());
            }
        });
        if self.list_ask.as_deref() == Some(url.as_str()) {
            ui.horizontal_wrapped(|ui| {
                label(ui, "REMOVE THIS PLAYLIST?", 240.0);
                if retro_btn(ui, "REMOVE", false).clicked() {
                    if let Some(i) = self.store.lists.iter().position(|l| l.url == url) {
                        acts.push(Action::RemoveList(i));
                    }
                    back(self);
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.list_ask = None;
                }
            });
        }
        ui.add_space(4.0);
        dim_line(ui, "RIGHT-CLICK A SONG TO SAVE IT, QUEUE IT OR ADD IT TO A PLAYLIST", 1.0, pal().dim);
        self.ext_list(ui, acts, if yt { 7 } else { 3 });
    }

    /// The hearts you gave to this kind of source (0 files, 1 YouTube, 2 SoundCloud); stored on this PC only.
    fn liked_block(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, group: u8) {
        let n = self.store.hearts.iter().filter(|e| heart_group(e) == group).count();
        if n == 0 {
            return;
        }
        section_header(ui, "LIKED (ON THIS PC)");
        let tracks: Vec<Track> = self.store.hearts.iter().filter(|e| heart_group(e) == group).map(|e| e.to_track()).collect();
        ui.add_space(4.0);
        play_buttons(ui, &tracks, acts, false);
        ui.add_space(4.0);
        self.ext_list(ui, acts, 4 + group);
    }

    pub(crate) fn files_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "MY FILES", 3.0, pal().ink);
        // linked with another computer: its songs too, to copy the ones you want (they never come over by themselves)
        if let Some(name) = crate::peer_files::peer_name() {
            // SOURCE: this computer's songs, the linked computer's, or both in one list
            ui.horizontal_wrapped(|ui| {
                label(ui, "SOURCE", 0.0);
                if crate::check_box(ui, "THIS COMPUTER", self.files_here).tip("The songs on this computer.").clicked() {
                    // (one of the two always stays on)
                    self.files_here = !self.files_here || !self.files_there;
                }
                ui.add_space(6.0);
                if crate::check_box(ui, &name.to_uppercase(), self.files_there)
                    .tip(format!("The songs in Backline on {}, to copy the ones you want.", name))
                    .clicked()
                {
                    self.files_there = !self.files_there || !self.files_here;
                    if self.files_there {
                        self.peer.list = None;
                    }
                }
            });
            ui.add_space(4.0);
            if self.files_there {
                let with_here = self.files_here;
                self.peer_files_view(ui, acts, &name, with_here);
                return;
            }
        }
        let n = self.store.files.len();
        // how many, then one ADD with its choices under it (the count short, so they share a line)
        ui.horizontal_wrapped(|ui| {
            label(ui, &format!("{} {}", n, if n == 1 { "file" } else { "files" }), 0.0);
            let add = retro_btn(ui, "+ ADD", false).tip("Add files or a folder, or rescan your folders. Nothing is copied: they play from where they are.");
            let pop = ui.id().with("files_add");
            if add.clicked() {
                ui.memory_mut(|m| m.toggle_popup(pop));
            }
            egui::popup_below_widget(ui, pop, &add, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(220.0);
                if menu_item(ui, "Add Files...") {
                    acts.push(Action::AddFiles);
                }
                if menu_item(ui, "Add a Folder...") {
                    acts.push(Action::AddFolder);
                }
                if menu_item(ui, "Rescan Folders") {
                    acts.push(Action::Rescan);
                }
            });
        });
        if self.files_busy {
            let stage = crate::extras::FILES_STAGE.load(Ordering::Relaxed);
            dim_line(
                ui,
                if stage == 1 { "CHOOSE FILES OR A FOLDER IN THE DIALOG..." } else { "SCANNING FILES..." },
                2.0,
                pal().ink2,
            );
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        }
        if self.store.files.is_empty() && !self.files_busy {
            ui.add_space(6.0);
            let tail = if PRACTICE { " Loops, speed and sections all work on them." } else { "" };
            para(ui, &format!("Drag audio files or folders onto this window, or use + ADD. Nothing is copied: files stay where they are.{}", tail), pal().ink2);
        }
        if !self.store.folders.is_empty() {
            section_header(ui, "WATCHED FOLDERS (CLICK TO STOP)");
            for (i, f) in self.store.folders.iter().enumerate() {
                if let Some(r) = list_row(ui, i, None, || f.clone(), || "FORGET".to_string(), RowState::Normal, false, false) {
                    if r.clicked() {
                        acts.push(Action::ForgetFolder(i));
                    }
                }
            }
        }
        if !self.store.files.is_empty() {
            section_header(ui, "FILES");
            let tracks: Vec<Track> = self.store.files.iter().map(|e| e.to_track()).collect();
            ui.add_space(4.0);
            play_buttons(ui, &tracks, acts, false);
            ui.add_space(4.0);
            self.ext_list(ui, acts, 0);
        }
        self.liked_block(ui, acts, 0);
        ui.add_space(20.0);
    }

    // -------------------------------------------------------------- YOUTUBE
    pub(crate) fn yt_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "YOUTUBE", 3.0, pal().ink);
        title_line(ui, "audio only, for practice", 2.0, pal().ink2);
        ui.add_space(4.0);
        let have = sources::ytdlp_path().is_some();
        ui.horizontal_wrapped(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(150.0, bh()), Sense::hover());
            ptext(
                ui.painter(),
                Pos2::new(rect.min.x + 2.0, rect.center().y),
                Align::Min,
                if have { "YT-DLP: READY" } else { "YT-DLP: MISSING" },
                2.0,
                if have { pal().ink } else { pal().red },
            );
            if retro_btn(ui, if have { "UPDATE YT-DLP" } else { "GET YT-DLP" }, !have)
                .tip("Downloads the free yt-dlp tool from its official GitHub page.")
                .clicked()
            {
                acts.push(Action::GetYtDlp);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let w = (ui.available_width() - 78.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_YT, &mut self.yt_in, rect, "Paste a YouTube link or playlist", false);
            if o.enter || retro_btn_w(ui, "ADD", 70.0, false).clicked() {
                acts.push(Action::AddYt);
            }
        });
        if self.yt_busy || !self.yt_msg.is_empty() {
            let col = if self.yt_msg.starts_with("ERROR") || self.yt_msg.starts_with("PRESS") || self.yt_msg.starts_with("THAT") {
                pal().red
            } else {
                pal().ink2
            };
            let n = crate::sources::YT_READ.load(std::sync::atomic::Ordering::Relaxed);
            let msg = if self.yt_busy && self.yt_msg.starts_with("READING THE PLAYLIST") && n > 0 {
                format!("READING THE PLAYLIST... {} SO FAR", n)
            } else {
                self.yt_msg.clone()
            };
            if self.yt_busy {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
            }
            para(ui, &msg, col);
        }
        // a playlist open: its own page (back to the rest with the arrow)
        if self.yt_cur.is_some() && !self.yt_results.is_empty() {
            self.list_page(ui, acts, "yt");
            ui.add_space(20.0);
            return;
        }
        self.saved_lists(ui, acts, "yt");
        if self.store.yt.is_empty() {
            para(ui, "One-time setup: press GET YT-DLP. Then paste a link and press ADD.", pal().ink2);
            if PRACTICE {
                para(ui, "Play-alongs, lessons, live takes: anything you want to loop and slow down.", pal().ink2);
            }
            para(
                ui,
                "Using YouTube this way may go against its terms. It is for personal use only, at your own discretion.",
                pal().dim,
            );
        } else {
            section_header(ui, "MY CLIPS");
            let tracks: Vec<Track> = self.store.yt.iter().map(|e| e.to_track()).collect();
            ui.add_space(4.0);
            play_buttons(ui, &tracks, acts, false);
            ui.add_space(4.0);
            self.ext_list(ui, acts, 1);
        }
        self.liked_block(ui, acts, 1);
        ui.add_space(20.0);
    }

    // ----------------------------------------------------------- SOUNDCLOUD
    pub(crate) fn sc_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "SOUNDCLOUD", 3.0, pal().ink);
        title_line(ui, "search, paste a link, or type a user and press USER PLAYLISTS", 2.0, pal().ink2);
        ui.add_space(4.0);
        let have = sources::ytdlp_path().is_some();
        if !have {
            ui.horizontal_wrapped(|ui| {
                label(ui, "ONE-TIME SETUP", 150.0);
                if retro_btn(ui, "GET YT-DLP", true).tip("Downloads the free yt-dlp tool, which also handles SoundCloud.").clicked()
                {
                    acts.push(Action::GetYtDlp);
                }
            });
            if self.yt_busy || !self.yt_msg.is_empty() {
                para(ui, &self.yt_msg.clone(), pal().ink2);
            }
        }
        ui.horizontal_wrapped(|ui| {
            let w = (ui.available_width() - 78.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_SC, &mut self.sc_in, rect, "Search SoundCloud, or paste a link", false);
            if o.enter || retro_btn_w(ui, "GO", 70.0, false).clicked() {
                acts.push(Action::ScGo);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "USER PLAYLISTS", false)
                .tip("Lists all public playlists of a user. Type their SoundCloud name or profile link above first.")
                .clicked()
            {
                acts.push(Action::ScSets);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let n = sources::SC_BROWSER.load(Ordering::Relaxed) as usize;
            label(ui, "SIGN-IN FROM", 130.0);
            if retro_btn_w(ui, sources::SC_BROWSERS[n], 110.0, n > 0)
                .tip("Uses the SoundCloud sign-in of a browser on this PC, for private playlists and Go+. Click to change. Firefox works best.")
                .clicked()
            {
                acts.push(Action::ScBrowser);
            }
        });
        if !self.sc_msg.is_empty() {
            let col = if self.sc_msg.starts_with("ERROR") || self.sc_msg.starts_with("PRESS") { pal().red } else { pal().ink2 };
            para(ui, &self.sc_msg.clone(), col);
        }
        if self.sc_results.is_empty() && self.store.sc.is_empty() {
            para(ui, "Paste your likes link or a playlist link and press GO. Your likes link is soundcloud.com/YOU/likes. Public lists show right here; click a track to play it.", pal().ink2);
            para(ui, "Without a sign-in, private and Go+ tracks may play as previews or not at all. Using SoundCloud this way may go against its terms. Practice use is at your own discretion.", pal().dim);
        }
        // a playlist open: its own page
        let sc_list = self.sc_cur.as_ref().is_some_and(|u| self.store.lists.iter().any(|l| &l.url == u));
        if sc_list && !self.sc_results.is_empty() {
            self.list_page(ui, acts, "sc");
            ui.add_space(20.0);
            return;
        }
        self.saved_lists(ui, acts, "sc");
        if !self.sc_sets.is_empty() {
            section_header(ui, "PLAYLISTS (CLICK TO OPEN)");
            for (i, (title, url)) in self.sc_sets.iter().enumerate() {
                if let Some(r) =
                    list_row(ui, i, Some(i + 1), || title.clone(), || "OPEN".to_string(), RowState::Normal, false, false)
                {
                    if r.clicked() {
                        acts.push(Action::ScOpenSet(url.clone()));
                    }
                }
            }
        }
        if !self.sc_results.is_empty() {
            section_header(ui, "RESULTS (RIGHT-CLICK TO SAVE)");
            let tracks: Vec<Track> = self.sc_results.iter().map(|e| e.to_track()).collect();
            ui.add_space(4.0);
            play_buttons(ui, &tracks, acts, false);
            ui.add_space(4.0);
            self.ext_list(ui, acts, 3);
        }
        if !self.store.sc.is_empty() {
            section_header(ui, "MY SOUNDCLOUD");
            let tracks: Vec<Track> = self.store.sc.iter().map(|e| e.to_track()).collect();
            ui.add_space(4.0);
            play_buttons(ui, &tracks, acts, false);
            ui.add_space(4.0);
            self.ext_list(ui, acts, 2);
        }
        self.liked_block(ui, acts, 2);
        ui.add_space(20.0);
    }

    /// WHAT'S NEW, over the dimmed app: the notes of this version and the few before it, newest first.
    pub(crate) fn news_panel(&mut self, ctx: &egui::Context) {
        let Some(notes) = self.news.clone() else { return };
        let screen = ctx.screen_rect();
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("news_dim"))).rect_filled(
            screen,
            0.0,
            egui::Color32::from_black_alpha(170),
        );
        let me = crate::update::build();
        let mut close = false;
        let mut go_on = false;
        // (what was playing before the update: carried on from here, as the Continue bar behind this would)
        let resume = self
            .store
            .last
            .as_ref()
            .filter(|_| crate::PRACTICE && (self.cur.is_none() || self.stopped))
            .map(|l| (l.version.title.clone(), l.version.artist.clone(), crate::fmt_time(l.pos)));
        egui::Area::new(egui::Id::new("news_panel"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Frame::none().fill(pal().app_bg).stroke(egui::Stroke::new(2.0_f32, pal().edge)).inner_margin(16.0).show(
                    ui,
                    |ui| {
                        let w = 560.0_f32.min(screen.width() - 40.0);
                        ui.set_width(w);
                        title_line(ui, "WHAT'S NEW", 3.0, pal().ink);
                        egui::ScrollArea::vertical().max_height((screen.height() * 0.45).max(160.0)).show(ui, |ui| {
                            ui.set_width(w - 16.0);
                            // the areas' tags in one column, as wide as the widest shown, so the words line up
                            let all: Vec<(&String, Vec<(Option<String>, String)>)> =
                                notes.iter().map(|(t, b)| (t, news_lines(b))).collect();
                            let tag_w = all
                                .iter()
                                .flat_map(|(_, l)| l.iter().filter_map(|(a, _)| a.as_ref()))
                                .map(|a| crate::font::text_w(&a.to_uppercase(), NEWS_TAG_PX) + 16.0)
                                .fold(0.0_f32, f32::max);
                            for (tag, lines) in all {
                                // a version that changed nothing you'd notice isn't listed
                                if lines.is_empty() {
                                    continue;
                                }
                                let this = tag.rsplit('.').next().and_then(|n| n.parse::<u64>().ok()) == Some(me);
                                section_header(
                                    ui,
                                    &format!("{}{}", tag.to_uppercase(), if this { "  (THIS VERSION)" } else { "" }),
                                );
                                for (area, text) in &lines {
                                    news_line(ui, area.as_deref(), text, tag_w);
                                }
                                ui.add_space(6.0);
                            }
                        });
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            if let Some((title, artist, at)) = &resume {
                                let who = if artist.is_empty() { String::new() } else { format!(" by {}", artist) };
                                if retro_btn(ui, &format!("Continue: {}{}", title, who), true)
                                    .tip(format!("Play it again from {}, where it was.", at))
                                    .clicked()
                                {
                                    go_on = true;
                                }
                            }
                            if retro_btn(ui, "GOT IT", resume.is_none()).clicked() {
                                close = true;
                            }
                        });
                    },
                );
            });
        if go_on || close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.news = None;
            self.seen_build = crate::update::build();
            self.dirty = true;
        }
        if go_on {
            self.apply(Action::Continue);
        }
    }

    /// A pack being opened, over the dimmed app: each piece in it, ticked (one that would replace yours starts
    /// unticked, and says so), then UNLOCK or cancel. Its loops and playlists come after, in the import window.
    pub(crate) fn pack_panel(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        let Some(mut p) = self.pack_in.take() else { return };
        // the practice version's download, for this computer
        const PRACTICE_FILE: &str = if cfg!(target_os = "macos") { "BACKLINE-PRACTICE-MAC.TAR.GZ" } else { "BACKLINE-PRACTICE.EXE" };
        let screen = ctx.screen_rect();
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("pack_dim"))).rect_filled(
            screen,
            0.0,
            egui::Color32::from_black_alpha(170),
        );
        let mut close = false;
        egui::Area::new(egui::Id::new("pack_panel")).order(egui::Order::Tooltip).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO).show(
            ctx,
            |ui| {
                egui::Frame::none().fill(pal().app_bg).stroke(egui::Stroke::new(2.0_f32, pal().edge)).inner_margin(16.0).show(ui, |ui| {
                    ui.set_width(440.0_f32.min(screen.width() - 40.0));
                    title_line(ui, "OPEN A PACK", 3.0, pal().ink);
                    para(ui, "Check what to import. Each item goes where it belongs and turns on its part of Backline.", pal().ink2);
                    ui.add_space(4.0);
                    for part in p.parts.iter_mut() {
                        let label = if part.replaces { format!("{}  (REPLACES YOURS)", part.label) } else { part.label.clone() };
                        if crate::check_box(ui, &label, part.on).clicked() {
                            part.on = !part.on;
                        }
                    }
                    if p.share.is_some() {
                        para(ui, "LOOPS & PLAYLISTS: you pick which ones next. They're added to yours, nothing is replaced.", pal().ink2);
                    }
                    if p.parts.iter().any(|x| x.replaces && x.on) {
                        para(ui, "A checked item marked REPLACES YOURS replaces the one you have.", pal().red);
                    }
                    // the plain player has no DIARY, MY PLAN, LINES or READING: say so, rather than unlock into nothing
                    if !crate::PRACTICE && !p.parts.is_empty() {
                        crate::views::dim_line(
                            ui,
                            "THIS IS THE PLAIN PLAYER: MY PLAN, THE BOOKS, LINES AND CHORD MELODIES ARE IN THE PRACTICE VERSION.",
                            1.0,
                            pal().red,
                        );
                        crate::views::dim_line(
                            ui,
                            &format!("GET {} FROM THE RELEASES PAGE. WHAT YOU UNLOCK HERE SHOWS UP THERE.", PRACTICE_FILE),
                            1.0,
                            pal().red,
                        );
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if retro_btn(ui, "UNLOCK", false).clicked() {
                            acts.push(Action::PackUnlock);
                        }
                        if retro_btn(ui, "CANCEL", false).clicked() {
                            close = true;
                        }
                    });
                });
            },
        );
        if !(close || ctx.input(|i| i.key_pressed(egui::Key::Escape))) {
            self.pack_in = Some(p);
        }
    }

    /// The export / import preview, over the dimmed app: what is in the file (songs, loops, playlists) and, for an
    /// import, what is new to you and what can't play here - then go ahead or cancel.
    pub(crate) fn share_panel(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        let Some((importing, s)) = self.share_view.clone() else {
            self.share_pick = None;
            return;
        };
        // everything ticked to begin with
        if self.share_pick.is_none() {
            let songs = s.loops.iter().filter(|(_, l)| !l.is_empty()).map(|(id, _)| *id).collect();
            let lists = s.playlists.iter().map(|p| p.name.clone()).collect();
            self.share_pick = Some((songs, lists));
        }
        let screen = ctx.screen_rect();
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("share_dim"))).rect_filled(
            screen,
            0.0,
            egui::Color32::from_black_alpha(170),
        );
        let mut songs: Vec<(i64, String, usize)> = s
            .loops
            .iter()
            .filter(|(_, l)| !l.is_empty())
            .map(|(id, l)| {
                let name = s.songs.get(id).map(|v| format!("{} - {}", v.artist, v.title)).unwrap_or_else(|| "a song".into());
                (*id, name, l.len())
            })
            .collect();
        songs.sort_by(|a, b| a.1.cmp(&b.1));
        let lists: Vec<(String, usize)> = s.playlists.iter().map(|p| (p.name.clone(), p.items.len())).collect();
        let (picked_songs, picked_lists) = self.share_pick.clone().unwrap_or_default();
        let n_loops: usize = songs.iter().filter(|x| picked_songs.contains(&x.0)).map(|x| x.2).sum();
        let n_songs = songs.iter().filter(|x| picked_songs.contains(&x.0)).count();
        let n_lists = lists.iter().filter(|x| picked_lists.contains(&x.0)).count();
        // what an import would add, counting only what's ticked
        let took = if importing {
            let mut t = s.clone();
            t.loops.retain(|id, _| picked_songs.contains(id));
            t.songs.retain(|id, _| picked_songs.contains(id));
            t.playlists.retain(|p| picked_lists.contains(&p.name));
            Some(self.store.preview_share(&t))
        } else {
            None
        };
        let mut pick = self.share_pick.take().unwrap_or_default();
        let newer = s.version > store::SHARE_VERSION;
        let mut close = false;
        // above the dimming (which is on the foreground layer)
        egui::Area::new(egui::Id::new("share_panel"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Frame::none().fill(pal().app_bg).stroke(egui::Stroke::new(2.0_f32, pal().edge)).inner_margin(16.0).show(
                    ui,
                    |ui| {
                        ui.set_width(520.0_f32.min(screen.width() - 40.0));
                        title_line(
                            ui,
                            if importing { "IMPORT LOOPS & PLAYLISTS" } else { "EXPORT LOOPS & PLAYLISTS" },
                            3.0,
                            pal().ink,
                        );
                        para(ui, &format!("SELECTED: {} LOOPS ON {} SONGS, {} PLAYLISTS", n_loops, n_songs, n_lists), pal().ink);
                        ui.horizontal_wrapped(|ui| {
                            let all_songs: std::collections::HashSet<i64> = songs.iter().map(|x| x.0).collect();
                            let all_lists: std::collections::HashSet<String> = lists.iter().map(|x| x.0.clone()).collect();
                            if retro_btn(ui, "EVERYTHING", false).clicked() {
                                pick = (all_songs.clone(), all_lists.clone());
                            }
                            if retro_btn(ui, "ALL LOOPS", false).tip("Every loop, no playlists.").clicked() {
                                pick = (all_songs.clone(), std::collections::HashSet::new());
                            }
                            if retro_btn(ui, "ALL PLAYLISTS", false).tip("Every playlist, no loops.").clicked() {
                                pick = (std::collections::HashSet::new(), all_lists.clone());
                            }
                            if retro_btn(ui, "NONE", false).clicked() {
                                pick = Default::default();
                            }
                        });
                        if let Some(t) = &took {
                            para(
                                ui,
                                &format!(
                                    "NEW: {} LOOPS, {} PLAYLISTS, {} SONGS FOR YOUR EXISTING PLAYLISTS",
                                    t.loops, t.playlists, t.items
                                ),
                                pal().ink,
                            );
                            if t.relinked > 0 {
                                para(ui, &format!("{} FILES WILL BE MATCHED TO YOUR OWN COPIES", t.relinked), pal().ink2);
                            }
                            if t.missing > 0 {
                                para(
                                    ui,
                                    &format!("{} FILES AREN'T ON THIS COMPUTER. THEIR LOOPS ARE IMPORTED BUT WON'T PLAY.", t.missing),
                                    pal().ink2,
                                );
                            }
                            if t.tidal > 0 && !self.api.has_token() {
                                para(ui, &format!("{} TIDAL SONGS NEED A TIDAL LOGIN TO PLAY", t.tidal), pal().ink2);
                            }
                        }
                        if newer {
                            para(
                                ui,
                                "MADE BY A NEWER VERSION OF BACKLINE, SO SOME ITEMS MAY BE SKIPPED. UPDATE TO IMPORT EVERYTHING.",
                                pal().red,
                            );
                        }
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical().max_height((screen.height() * 0.45).max(120.0)).show(ui, |ui| {
                            if !songs.is_empty() {
                                section_header(ui, "LOOPS");
                                for (id, name, n) in &songs {
                                    let on = pick.0.contains(id);
                                    let label = format!("{}  ({} {})", name, n, if *n == 1 { "LOOP" } else { "LOOPS" });
                                    if crate::check_box(ui, &label, on).clicked() {
                                        if on {
                                            pick.0.remove(id);
                                        } else {
                                            pick.0.insert(*id);
                                        }
                                    }
                                }
                            }
                            if !lists.is_empty() {
                                section_header(ui, "PLAYLISTS");
                                for (name, n) in &lists {
                                    let on = pick.1.contains(name);
                                    if crate::check_box(ui, &format!("{}  ({} SONGS)", name, n), on).clicked() {
                                        if on {
                                            pick.1.remove(name);
                                        } else {
                                            pick.1.insert(name.clone());
                                        }
                                    }
                                }
                            }
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let go = if importing { "IMPORT" } else { "SAVE..." };
                            if retro_btn(ui, go, false).clicked() {
                                acts.push(if importing { Action::ShareTake } else { Action::ShareSave });
                            }
                            if retro_btn(ui, "CANCEL", false).clicked() {
                                close = true;
                            }
                        });
                    },
                );
            });
        self.share_pick = Some(pick);
        if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.share_view = None;
            self.share_pick = None;
        }
    }

    /// Take your loops and playlists to another computer: a file to save, send however you like, and open there.
    fn share_buttons(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "EXPORT LOOPS & PLAYLISTS...", false)
                .tip("Saves every loop and playlist to one file. Email it or put it on a USB stick or OneDrive, then import it on the other computer.")
                .clicked()
            {
                acts.push(Action::ShareExport);
            }
            if retro_btn(ui, "IMPORT...", false)
                .tip("Imports a file from another computer. It adds the loops and playlists you don't have and leaves yours unchanged.")
                .clicked()
            {
                acts.push(Action::ShareImport);
            }
        });
    }

    // ------------------------------------------------------------ PLAYLISTS
    pub(crate) fn playlists_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        if let Some(i) = self.pl_open.filter(|&i| i < self.store.playlists.len()) {
            self.playlist_detail(ui, acts, i);
            return;
        }
        title_line(ui, "PLAYLISTS", 3.0, pal().ink);
        title_line(ui, "your own lists, mixing Tidal, SoundCloud, YouTube and files", 2.0, pal().ink2);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let w = (ui.available_width() - 78.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_NEW_PL, &mut self.new_pl, rect, "New playlist name", false);
            // the New Playlist panel: a name, a picture, in Backline or on Tidal
            if o.enter || retro_btn_w(ui, "NEW", 70.0, false).tip("Create a playlist").clicked() {
                acts.push(Action::PlaylistNew(None));
            }
        });
        self.share_buttons(ui, acts);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "SAVE QUEUE AS PLAYLIST", false)
                .tip("Turns the queue into a playlist, named after what it was started from (a radio, a mix). You can change the name.")
                .clicked()
            {
                acts.push(Action::PlaylistFromQueue);
            }
        });
        if self.store.playlists.is_empty() {
            para(ui, "Right-click any song and choose ADD TO PLAYLIST. Playlists live on this PC only, and one list can mix every source.", pal().ink2);
        }
        ui.add_space(4.0);
        // your playlists, filtered by name as you type
        let filter = self.lib_filter.trim().to_string();
        if !self.store.playlists.is_empty() {
            self.filter_box(ui, "Filter your playlists");
        }
        let order: Vec<usize> = if filter.is_empty() {
            (0..self.store.playlists.len()).collect()
        } else {
            crate::filtered(&filter, &self.store.playlists, |p| vec![p.name.clone()], 4)
        };
        if !filter.is_empty() && order.is_empty() && !self.store.playlists.is_empty() {
            title_line(ui, "No playlists match.", 2.0, pal().ink2);
        }
        for (row, &i) in order.iter().enumerate() {
            let pl = &self.store.playlists[i];
            let n = pl.items.len();
            if let Some(r) =
                list_row(ui, row, Some(i + 1), || pl.name.clone(), || format!("{} tracks", n), RowState::Normal, false, false)
            {
                if r.clicked() {
                    acts.push(Action::PlaylistOpen(Some(i)));
                }
                r.context_menu(|ui| {
                    // (the panel has its description and cover too)
                    if menu_item(ui, "Rename...") {
                        acts.push(Action::PlaylistEdit(i));
                        ui.close_menu();
                    }
                    if menu_item(ui, "Delete This Playlist") {
                        acts.push(Action::PlaylistDelete(i));
                        ui.close_menu();
                    }
                });
            }
        }
        ui.add_space(20.0);
    }

    fn playlist_detail(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, pi: usize) {
        let name = self.store.playlists[pi].name.clone();
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "< ALL PLAYLISTS", false).clicked() {
                acts.push(Action::PlaylistOpen(None));
            }
            if retro_btn(ui, "RENAME / EDIT", false).tip("Its name, description and cover").clicked() {
                acts.push(Action::PlaylistEdit(pi));
            }
        });
        // its cover (a record while it has none) beside the name; a click picks one
        ui.add_space(4.0);
        let url = crate::new_playlist::art_url(&self.store.playlists[pi].art);
        let desc = self.store.playlists[pi].desc.clone();
        ui.horizontal_top(|ui| {
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(140.0), Sense::click());
            crate::paint_page_art(ui, &mut self.images, &url, r, false);
            if resp.hovered() {
                let band = Rect::from_min_max(egui::pos2(r.min.x, r.max.y - 24.0), r.max);
                ui.painter().rect_filled(band, 0.0, egui::Color32::from_black_alpha(170));
                ptext_fit(ui.painter(), band.center(), Align::Center, if url.is_empty() { "ADD A COVER" } else { "CHANGE COVER" }, 1.4, r.width() - 10.0, egui::Color32::WHITE);
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                acts.push(Action::PlaylistEdit(pi));
            }
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.add_space(8.0);
                title_line(ui, &name.to_uppercase(), 3.0, pal().ink);
                if !desc.is_empty() {
                    para(ui, &desc, pal().ink2);
                }
            });
        });
        let items = self.store.playlists[pi].items.clone();
        title_line(ui, &format!("{} tracks (right-click one to move or remove it)", items.len()), 2.0, pal().ink2);
        ui.add_space(4.0);
        if items.is_empty() {
            para(ui, "Empty. Right-click any song and choose ADD TO A BACKLINE PLAYLIST.", pal().ink2);
            return;
        }
        let tracks: Vec<Track> = items.iter().map(|v| v.to_track()).collect();
        play_buttons(ui, &tracks, acts, false);
        ui.add_space(4.0);
        let playing = self.cur_track().map(|t| t.id);
        let art = crate::row_arts_on();
        col_header(ui, true, art);
        for (k, v) in items.iter().enumerate() {
            let state = if playing == Some(v.id) { RowState::Playing } else { RowState::Normal };
            let stored = v.kind == "file" || cache::has(v.id);
            crate::row_heart(self.liked.contains(&v.id));
            crate::row_stems(v.id);
            if art {
                crate::row_art(&store::file_cover(&v.kind, &v.src, &v.cover));
            }
            // a loop shows as its name and where it is, the song after it
            let title = match v.loop_ab {
                Some((a, b)) => format!("LOOP: {} ({}-{})  {}", v.loop_name, fmt_time(a), fmt_time(b), v.title),
                None => v.title.clone(),
            };
            let len = v.loop_ab.map_or(v.dur, |(a, b)| b - a);
            let r = list_row(
                ui,
                k,
                Some(k + 1),
                || track_cells(v.id, &title, &v.artist, &v.album, &v.kind),
                || if col_on(2) { fmt_time(len) } else { String::new() },
                state,
                false,
                stored,
            );
            if crate::heart_clicked() {
                acts.push(Action::ToggleLike(v.to_track()));
            }
            if let Some(r) = r {
                if r.clicked() {
                    match v.loop_ab {
                        // a loop plays looping, from its start
                        Some(ab) => acts.push(Action::PlaySaved(v.clone(), Some(ab))),
                        None => acts.push(Action::Play(tracks.clone(), k)),
                    }
                }
                let tidal = self.ext_of(v.id).is_none();
                let liked = self.liked.contains(&v.id);
                r.context_menu(|ui| {
                    let t = v.to_track();
                    if v.loop_ab.is_some() && menu_item(ui, "Play the Whole Song") {
                        acts.push(Action::PlaySaved(v.clone(), None));
                        ui.close_menu();
                    }
                    crate::song_items(ui, acts, &t, liked, tidal);
                    if k > 0 && menu_item(ui, "Move Up") {
                        acts.push(Action::PlaylistMove(pi, k, -1));
                        ui.close_menu();
                    }
                    if k + 1 < items.len() && menu_item(ui, "Move Down") {
                        acts.push(Action::PlaylistMove(pi, k, 1));
                        ui.close_menu();
                    }
                    let ext = self.ext_of(v.id);
                    crate::link_item(ui, acts, crate::track_link(&t, ext.as_ref()));
                    if menu_item(ui, "Remove from This Playlist") {
                        acts.push(Action::PlaylistRemove(pi, k));
                        ui.close_menu();
                    }
                });
            }
        }
        ui.add_space(20.0);
    }

}

/// The parts of the app a note can be about: a note written "Area: what changed" shows its area as a tag.
const NEWS_AREAS: [&str; 25] = [
    "App", "Library", "Player", "Playlist", "Queue", "Practice", "Chart", "Now Playing", "Visuals", "Skins", "Looks", "Window",
    "Tunes", "Loops", "Diary", "Lines", "Chords", "Ear Training", "Reading", "Settings", "Sync", "Files", "Tidal",
    "Mac", "Mini Player",
];

/// A release's notes as What's New shows them: one line a change, (area, words), from the notes written for GitHub
/// (the details under each, the headings and the download instructions left out).
pub(crate) fn news_lines(body: &str) -> Vec<(Option<String>, String)> {
    let mut out = Vec::new();
    for line in body.lines() {
        if line.trim().trim_start_matches('#').trim().eq_ignore_ascii_case("downloads") {
            break;
        }
        if line.trim_start().starts_with('#') || (line.starts_with(char::is_whitespace) && line.trim_start().starts_with("- ")) {
            continue;
        }
        let l = line.trim().replace(['`'], "").replace("**", "");
        if l.is_empty() || l.starts_with("Full Changelog") || l.starts_with("<!--") {
            continue;
        }
        let l = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")).unwrap_or(&l).trim();
        let l = l.strip_prefix("New: ").unwrap_or(l);
        let area = NEWS_AREAS.iter().find(|a| l.strip_prefix(**a).is_some_and(|r| r.starts_with(": ")));
        match area {
            Some(a) => out.push((Some(a.to_string()), l[a.len() + 2..].to_string())),
            None => out.push((None, l.to_string())),
        }
    }
    out
}

/// The size of an area's tag in What's New.
const NEWS_TAG_PX: f32 = 1.25;

/// One change in What's New: its area as a tag in a column `tag_w` wide (so the words line up), then the words.
fn news_line(ui: &mut egui::Ui, area: Option<&str>, text: &str, tag_w: f32) {
    // (each change starts with a capital, whatever its note)
    let mut cs = text.chars();
    let text = cs.next().map(|c| c.to_uppercase().chain(cs).collect::<String>()).unwrap_or_default();
    ui.horizontal_top(|ui| {
        if tag_w > 0.0 {
            let (r, _) = ui.allocate_exact_size(Vec2::new(tag_w, 24.0), Sense::hover());
            if let Some(a) = area {
                let w = crate::font::text_w(&a.to_uppercase(), NEWS_TAG_PX) + 14.0;
                let pill = Rect::from_min_size(Pos2::new(r.min.x, r.min.y + 3.0), Vec2::new(w, 19.0));
                ui.painter().rect_filled(pill, 9.5, pal().display.lerp_to_gamma(pal().accent, 0.22));
                crate::font::keep_caps(|| {
                    ptext_fit(ui.painter(), pill.center(), Align::Center, &a.to_uppercase(), NEWS_TAG_PX, w - 8.0, pal().ink)
                });
            }
        }
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            crate::wrapped(ui, &text, pal().ink2);
        });
    });
}

#[cfg(test)]
mod summary_tests {
    use super::*;

    #[test]
    fn whats_new_reads_a_change_list() {
        let body = "## Changes
- Chart: a page of its own
- New: Skins: fourteen looks
- A line with no area
  - a detail

## Downloads
- **Windows:** the installer
";
        let l = news_lines(body);
        assert_eq!(l.len(), 3);
        assert_eq!(l[0], (Some("Chart".to_string()), "a page of its own".to_string()));
        assert_eq!(l[1], (Some("Skins".to_string()), "fourteen looks".to_string()));
        assert_eq!(l[2], (None, "A line with no area".to_string()));
        // a release that changed nothing you'd notice has no lines (and isn't listed)
        assert!(news_lines("## Changes

## Downloads
- **Windows:** x
").is_empty());
    }

    #[test]
    fn a_loop_with_a_note_gets_a_line_of_its_own() {
        let r = note_runs(&[false, false, true, false, true, true]);
        assert_eq!(r, vec![0..2, 2..3, 3..4, 4..5, 5..6]);
        assert!(note_runs(&[]).is_empty());
        assert_eq!(note_runs(&[false, false]), vec![0..2]);
    }

    #[test]
    fn a_bookmark_keeps_a_note() {
        // one saved before notes were kept still loads, with none
        let old: store::Bookmark = serde_json::from_str(r#"{"t":12.5,"col":[1,2,3],"name":"bridge"}"#).unwrap();
        assert_eq!((old.name.as_str(), old.note.as_str()), ("bridge", ""));
        let m = store::Bookmark { t: 70.0, name: "Solo".into(), note: "watch the turnaround".into(), ..Default::default() };
        let tip = mark_tip(&m, 1, "1:10");
        assert!(tip.starts_with("Solo\nwatch the turnaround\nBookmark 2 at 1:10."), "{}", tip);
        let plain = mark_tip(&store::Bookmark::default(), 0, "0:00");
        assert!(plain.starts_with("Bookmark 1 at 0:00."), "{}", plain);
    }

    #[test]
    fn skimming_goes_day_to_day_with_notes() {
        let day = |date: &str, note: &str| store::Day { date: date.into(), note: note.into(), ..Default::default() };
        let days = [day("2026-11-05", "Comping, tried drop 2"), day("2026-10-30", "  "), day("2026-11-01", "Blues in F"), day("2026-10-28", "x")];
        let noted = noted_days(&days);
        let at = |s: &str| store::parse_date(s).unwrap();
        assert_eq!(noted, [at("2026-10-28"), at("2026-11-01"), at("2026-11-05")]);
        // from a day without notes, and from one with them
        assert_eq!(note_step(&noted, at("2026-11-06"), false), Some(at("2026-11-05")));
        assert_eq!(note_step(&noted, at("2026-11-01"), false), Some(at("2026-10-28")));
        assert_eq!(note_step(&noted, at("2026-11-01"), true), Some(at("2026-11-05")));
        assert_eq!(note_step(&noted, at("2026-11-05"), true), None);
        assert_eq!(note_step(&noted, at("2026-10-28"), false), None);
        // the calendar's steps: a day, a month (the day kept, or the month's last), a year
        assert_eq!(period_step(at("2026-11-08"), 2, -1), at("2026-11-07"));
        assert_eq!(period_step(at("2026-11-08"), 1, -1), at("2026-10-08"));
        assert_eq!(period_step(at("2026-01-31"), 1, 1), at("2026-02-28"));
        assert_eq!(period_step(at("2026-12-15"), 1, 1), at("2027-01-15"));
        assert_eq!(period_step(at("2026-01-10"), 1, -1), at("2025-12-10"));
        assert_eq!(period_step(at("2028-02-29"), 0, 1), at("2029-02-28"));
        assert_eq!(period_step(at("2026-11-08"), 0, -1), at("2025-11-08"));
        assert_eq!(note_preview("Two\nlines"), "Two lines");
        assert!(note_preview(&"word ".repeat(100)).ends_with("...") && note_preview(&"word ".repeat(100)).len() < 170);
    }

    #[test]
    fn a_group_of_one_page_shows_its_name() {
        const G: [(&str, &[(u8, &str)]); 2] = [("SCALES", &[(1, "SCALE FINGERINGS"), (4, "ARPEGGIOS")]), ("OVER CHORDS", &[(3, "SUPERIMPOSE")])];
        let (names, tips) = group_tabs(&G);
        assert_eq!(names, ["SCALES", "SUPERIMPOSE"]);
        assert_eq!(tips, ["SCALE FINGERINGS, ARPEGGIOS", ""]);
    }

    #[test]
    fn all_pages_lists_every_page_once() {
        let (items, heads, gos) = go_list(true);
        assert_eq!(items.len(), gos.len());
        let names: Vec<&str> = heads.iter().map(|h| h.1).collect();
        assert_eq!(names, ["TUNES", "DIARY", "LOOPS", "READING", "CHORDS", "EAR", "LINES", "CHORD MELODIES"]);
        for (i, g) in gos.iter().enumerate() {
            assert!(!gos[..i].contains(g), "{} twice", items[i]);
        }
        assert!(gos.contains(&Go::Lines(3)) && gos.contains(&Go::Diary(3)));
        assert!(!go_list(false).2.contains(&Go::Diary(3)));
        // a page's own tabs only ever sit under it
        for (i, it) in items.iter().enumerate() {
            if it.starts_with("  ") {
                let Go::Chords(t, s) = gos[i] else { panic!("{it}") };
                assert!(s != ANY && matches!(gos[i - 1], Go::Chords(p, _) if p == t));
            }
        }
    }

    #[test]
    fn periods_cover_the_right_days() {
        let wed = store::parse_date("2026-10-14").unwrap();
        let week = Period::around(wed, 0);
        assert_eq!(week.cols.len(), 7);
        assert_eq!(store::date_str(week.cols[0].1[0]), "2026-10-12", "weeks start on Monday");
        assert_eq!(store::date_str(week.cols[6].1[0]), "2026-10-18");
        assert_eq!(Period::around(wed, 1).cols.len(), 31);
        let year = Period::around(wed, 2);
        assert_eq!(year.cols.len(), 12);
        assert_eq!(year.cols.iter().map(|c| c.1.len()).sum::<usize>(), 365);
    }
}

/// A thin upright line between groups of controls in a row.
/// A dim line of words centred across the page or panel.
pub(crate) fn dim_centered(ui: &mut egui::Ui, text: &str) {
    let text = if crate::winamp_ui::chrome_on() { crate::winamp_ui::caps_words(text) } else { text.to_string() };
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 7.0 * spx(1.5) + 8.0), Sense::hover());
    ptext_fit(ui.painter(), r.center(), Align::Center, &text, 1.5, r.width() - 12.0, pal().ink2);
}

/// A row of controls centred across the page or panel (its width is taken from the frame before). Too wide for the
/// room, it wraps from the left instead.
pub(crate) fn centered_row(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    let key = ui.id().with(("centered_row", id));
    let w: f32 = ui.ctx().data(|d| d.get_temp(key)).unwrap_or(0.0);
    let room = ui.available_width();
    if w > room {
        let r = ui.horizontal_wrapped(add).response.rect;
        // (it fits on one line again, with room to spare: centre it from the next frame; a row that only just fits
        // stays as it is, so it never flips back and forth)
        if r.height() <= bh() + 6.0 && r.width() < room - 8.0 {
            ui.ctx().data_mut(|d| d.insert_temp(key, r.width()));
            ui.ctx().request_repaint();
        }
        return;
    }
    ui.horizontal(|ui| {
        let pad = ((room - w) / 2.0).max(0.0).floor();
        ui.add_space(pad);
        let x0 = ui.cursor().min.x;
        add(ui);
        let used = ui.min_rect().max.x - x0;
        if (used - w).abs() > 0.5 {
            ui.ctx().data_mut(|d| d.insert_temp(key, used));
            ui.ctx().request_repaint();
        }
    });
}

fn divider(ui: &mut egui::Ui) {
    let h = crate::bh();
    let (r, _) = ui.allocate_exact_size(Vec2::new(9.0, h), Sense::hover());
    ui.painter().line_segment(
        [Pos2::new(r.center().x, r.min.y + 4.0), Pos2::new(r.center().x, r.max.y - 4.0)],
        egui::Stroke::new(1.0_f32, pal().panel_lt),
    );
}

thread_local! {
    /// The tune name looked up with Enter or ADD (its cards show while it is still in the box), and what was found.
    static TUNE_ASKED: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    static TUNE_FOUND: std::cell::RefCell<Vec<(u32, String, String)>> = const { std::cell::RefCell::new(Vec::new()) };
}
