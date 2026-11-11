//! LOOPS: every song you've worked on with its loops and bookmarks (minutes, stars, notes under each), the
//! groups you sort them into, and the editors for a loop's or a bookmark's name and note.

use super::*;

/// The LOOPS page's songs, filtered and in order.
pub(crate) struct SpotData {
    pub(crate) songs: Vec<SpotSong>,
}

/// One song on the LOOPS page: its loops (with the time spent on each) and bookmarks, and its minutes.
pub(crate) struct SpotSong {
    pub(crate) id: i64,
    pub(crate) v: Option<store::Version>,
    pub(crate) loops: Vec<(usize, store::Section, u32)>,
    pub(crate) marks: Vec<(usize, store::Bookmark)>,
    /// practised this week, and in all (with its loops), in seconds
    pub(crate) mins: u32,
    pub(crate) total: u32,
    /// the day its newest loop was saved
    pub(crate) newest: String,
    /// the last day it was practiced, played or looped (days since 1970; 0: never dated)
    pub(crate) last: i64,
}

/// ALL's headings, as a game library has them: Recent (this month), then each month before it (this year's by name,
/// older ones with their year), then Earlier (songs never dated).
pub(crate) fn when_heading(day: i64, today: i64) -> String {
    const MONTHS: [&str; 12] =
        ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    if day <= 0 {
        return "Earlier".to_string();
    }
    let (y, m, _) = store::civil_from_days(day);
    let (this_year, this_month, _) = store::civil_from_days(today);
    if (y, m) >= (this_year, this_month) {
        return "Recent".to_string();
    }
    let name = MONTHS[(m as usize).clamp(1, 12) - 1];
    if y == this_year {
        name.to_string()
    } else {
        format!("{} {}", name, y)
    }
}

/// A heading over ALL's songs: the month (or Recent), and how many songs are under it.
fn when_head(ui: &mut egui::Ui, head: &str, n: usize) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
    let p = ui.painter();
    let y = r.center().y - 2.0;
    ptext(p, Pos2::new(r.min.x + 10.0, y), Align::Min, head, 2.0, pal().ink);
    ptext(p, Pos2::new(r.max.x - 10.0, y + 1.0), Align::Max, &format!("{} {}", n, if n == 1 { "song" } else { "songs" }), 1.2, pal().ink2);
    p.line_segment([Pos2::new(r.min.x + 10.0, r.max.y - 3.0), Pos2::new(r.max.x - 10.0, r.max.y - 3.0)], Stroke::new(2.0_f32, pal().ink2.gamma_multiply(0.6)));
}

/// A song's waveform as kept on disk (50 peaks a second), read once and kept for the LOOPS page.
pub(crate) fn song_wave(id: i64) -> Option<std::rc::Rc<Vec<u8>>> {
    thread_local! {
        static WAVES: std::cell::RefCell<HashMap<i64, Option<std::rc::Rc<Vec<u8>>>>> = std::cell::RefCell::new(HashMap::new());
    }
    WAVES.with(|w| {
        w.borrow_mut()
            .entry(id)
            .or_insert_with(|| {
                std::fs::read(crate::api::config_dir().join("waves").join(format!("{}.pk", id)))
                    .ok()
                    .filter(|b| b.len() > 50)
                    .map(std::rc::Rc::new)
            })
            .clone()
    })
}

/// A loop's or bookmark's note under it: a size down from its name.
pub(crate) const NOTE_PX: f32 = 1.5;

/// The note's lines, wrapped to `w` (each line break kept).
pub(crate) fn note_lines(note: &str, w: f32) -> Vec<String> {
    let note = note.trim();
    if note.is_empty() {
        return Vec::new();
    }
    crate::font::wrap(note, NOTE_PX, w.max(60.0))
}

/// The height of one of a note's lines.
pub(crate) fn note_lh() -> f32 {
    (crate::font::text_px(NOTE_PX) * 1.35).max(15.0)
}

/// A note's lines, the first one's top at `top`.
pub(crate) fn draw_note_lines(p: &egui::Painter, x: f32, top: f32, lines: &[String], col: Color32) {
    let lh = note_lh();
    for (k, line) in lines.iter().enumerate() {
        ptext(p, Pos2::new(x, top + lh * (k as f32 + 0.5)), Align::Min, line, NOTE_PX, col);
    }
}

/// A loop's or bookmark's note, shown whole under it, `indent` in from the left. A click or right-click on it
/// opens it to edit.
pub(crate) fn note_under(ui: &mut egui::Ui, note: &str, indent: f32) -> egui::Response {
    let lines = note_lines(note, ui.available_width() - indent - 12.0);
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), lines.len() as f32 * note_lh() + 4.0), Sense::click());
    let col = if resp.hovered() { pal().ink } else { pal().ink2 };
    draw_note_lines(ui.painter(), r.min.x + indent, r.min.y, &lines, col);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Click to edit the note.")
}

/// "NOTE  [ box ]" for a loop's or bookmark's note: a few lines tall, and taller as you write (Enter starts a new
/// line).
pub(crate) fn note_row(ui: &mut egui::Ui, ed: &mut Ed, id: u32, text: &mut String, hint: &str) -> FieldOut {
    let px = if crate::winamp_ui::chrome_on() { 1.7 } else { 2.0 };
    let lh = 7.0 * spx(px) + 6.0;
    let chars: Vec<char> = text.chars().collect();
    let lines = layout_lines(&chars, px, ui.available_width() - 76.0 - 16.0).len().max(1);
    let h = ((lines + 1) as f32 * lh + 6.0).max(3.0 * lh + 6.0);
    field_row(ui, ed, id, "NOTE", text, hint, Some(h))
}

/// Ctrl + Enter (Cmd on a Mac): save what's being edited.
pub(crate) fn save_keys(ui: &egui::Ui) -> bool {
    ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter))
}

/// Loops in a row as pills, a loop with a note on its own (its note under it): the runs, as ranges of `noted`.
pub(crate) fn note_runs(noted: &[bool]) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut k = 0;
    while k < noted.len() {
        let start = k;
        k += 1;
        if !noted[start] {
            while k < noted.len() && !noted[k] {
                k += 1;
            }
        }
        out.push(start..k);
    }
    out
}

/// A bookmark's flag tip: its name and note (when it has them), then where it is and what clicks do.
pub(crate) fn mark_tip(m: &store::Bookmark, k: usize, at: &str) -> String {
    let mut s = String::new();
    for line in [&m.name, &m.note] {
        if !line.is_empty() {
            s.push_str(line);
            s.push('\n');
        }
    }
    format!("{}Bookmark {} at {}. Click to jump, alt + click to remove, right-click to rename, add a note or recolor.", s, k + 1, at)
}

/// The bookmark colours: bright enough to read a dark number on, apart enough to tell apart.
pub(crate) const MARK_COLS: [[u8; 3]; 8] = [
    [235, 80, 80],
    [245, 150, 50],
    [240, 215, 70],
    [110, 210, 100],
    [80, 200, 210],
    [90, 140, 245],
    [175, 120, 240],
    [240, 120, 190],
];

/// The bookmark colours as a row of swatches in a menu, the current one outlined; the one clicked.
pub(crate) fn mark_swatches(ui: &mut egui::Ui, cur: Option<[u8; 3]>) -> Option<[u8; 3]> {
    let mut out = None;
    ui.horizontal(|ui| {
        for c in MARK_COLS {
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
            ui.painter().rect_filled(r.shrink(2.0), 2.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
            if cur == Some(c) || resp.hovered() {
                ui.painter().rect_stroke(r, 2.0, egui::Stroke::new(1.5_f32, pal().ink));
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                out = Some(c);
            }
        }
    });
    // any colour: a pixel colour wheel (round the wheel the hue, out from the middle the strength)
    let cells = 15;
    let cell = 8.0_f32;
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(cells as f32 * cell), Sense::click());
    let c0 = r.center();
    let rad = cells as f32 * cell / 2.0;
    let col_at = |p: Pos2| -> Option<[u8; 3]> {
        let d = p - c0;
        let dist = d.length() / rad;
        if dist > 1.0 {
            return None;
        }
        let hue = (d.y.atan2(d.x) / std::f32::consts::TAU).rem_euclid(1.0);
        let c = egui::Color32::from(egui::ecolor::Hsva::new(hue, (dist * 1.15).min(1.0) * 0.85, 1.0, 1.0));
        Some([c.r(), c.g(), c.b()])
    };
    for y in 0..cells {
        for x in 0..cells {
            let cr = Rect::from_min_size(r.min + Vec2::new(x as f32 * cell, y as f32 * cell), Vec2::splat(cell));
            if let Some(c) = col_at(cr.center()) {
                ui.painter().rect_filled(cr, 0.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
            }
        }
    }
    if let Some(pp) =
        resp.on_hover_cursor(egui::CursorIcon::Crosshair).interact_pointer_pos().filter(|_| ui.input(|i| i.pointer.any_click()))
    {
        // the cell's own colour, as drawn
        let cx = ((pp.x - r.min.x) / cell).floor() * cell + r.min.x + cell / 2.0;
        let cy = ((pp.y - r.min.y) / cell).floor() * cell + r.min.y + cell / 2.0;
        if let Some(c) = col_at(Pos2::new(cx, cy)) {
            out = Some(c);
        }
    }
    out
}

impl App {
    /// LOOPS > GROUPS: your groups as a list (name and how many loops); open one for its own page: its loops, RENAME
    /// and DELETE, and a way back. + NEW GROUP makes one.
    pub(crate) fn groups_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if let Some(g) = self.grp_open.clone().filter(|g| self.store.groups().contains(g) || g.is_empty()) {
            self.group_page(ui, acts, &g);
            return;
        }
        self.grp_open = None;
        let groups = self.store.groups();
        let count = |g: &str| self.store.sections.values().flatten().filter(|s| s.group == g).count();
        let mut rows: Vec<(String, usize)> = groups.iter().map(|g| (g.clone(), count(g))).collect();
        let loose = count("");
        if loose > 0 {
            rows.push((String::new(), loose));
        }
        for (k, (g, n)) in rows.iter().enumerate() {
            let title = if g.is_empty() { "NO GROUP".to_string() } else { g.to_uppercase() };
            let sub = format!("{} {}", n, if *n == 1 { "LOOP" } else { "LOOPS" });
            if let Some(r) = list_row(ui, k, None, || title.clone(), || sub.clone(), RowState::Normal, false, false) {
                if r.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Open this group.").clicked() {
                    self.grp_open = Some(g.clone());
                    self.grp_edit = None;
                    self.grp_ask = None;
                }
            }
        }
        self.new_group_row(ui);
        ui.add_space(20.0);
    }

    /// One group's own page: its loops (click to play, right-click to rename, note, move or forget), RENAME and
    /// DELETE (asked first; its loops stay), and back to the list.
    pub(crate) fn group_page(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, g: &str) {
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "< ALL GROUPS", false).clicked() {
                self.grp_open = None;
                self.grp_edit = None;
                self.grp_ask = None;
            }
            if !g.is_empty() {
                if retro_btn(ui, "RENAME", self.grp_edit.is_some()).clicked() {
                    self.grp_edit = Some(g.to_string());
                    self.grp_name = g.to_string();
                    self.ed.id = crate::tools::F_GROUP_NAME;
                    self.ed.anchor = None;
                }
                if retro_btn(ui, "DELETE...", self.grp_ask.is_some()).clicked() {
                    self.grp_ask = Some(g.to_string());
                }
            }
        });
        title_line(ui, &if g.is_empty() { "NO GROUP".to_string() } else { g.to_uppercase() }, 3.0, pal().ink);
        if self.grp_edit.as_deref() == Some(g) {
            ui.horizontal_wrapped(|ui| {
                label(ui, "NEW NAME", 90.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(200.0, bh()), Sense::hover());
                let o = field(ui, &mut self.ed, crate::tools::F_GROUP_NAME, &mut self.grp_name, r, "new name", false);
                if (o.enter || retro_btn(ui, "SAVE", false).clicked()) && !self.grp_name.trim().is_empty() {
                    let new = self.grp_name.trim().to_string();
                    self.store.rename_group(g, &new);
                    self.store_dirty = true;
                    self.grp_edit = None;
                    self.grp_open = Some(new);
                    self.ed.id = 0;
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.grp_edit = None;
                    self.ed.id = 0;
                }
            });
        }
        if self.grp_ask.as_deref() == Some(g) {
            ui.horizontal_wrapped(|ui| {
                label(ui, "DELETE THIS GROUP? ITS LOOPS WON'T BE DELETED", 360.0);
                if retro_btn(ui, "DELETE", false).clicked() {
                    self.store.rename_group(g, "");
                    self.store_dirty = true;
                    self.grp_ask = None;
                    self.grp_open = None;
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.grp_ask = None;
                }
            });
        }
        let groups = self.store.groups();
        let mut members: Vec<(i64, usize, store::Section)> = self
            .store
            .sections
            .iter()
            .flat_map(|(id, l)| l.iter().enumerate().map(move |(i, s)| (*id, i, s.clone())))
            .filter(|x| x.2.group == g)
            .collect();
        members.sort_by(|a, b| a.2.name.cmp(&b.2.name).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
        if members.is_empty() {
            para(ui, "No loops in it yet. Right-click a loop in LOOPS or the PRACTICE panel to move it here.", pal().ink2);
            return;
        }
        for (k, (id, i, s)) in members.iter().enumerate() {
            let v = self.store.songs.get(id).cloned();
            let song = v.as_ref().map(|v| format!("{} - {}", v.artist, v.title)).unwrap_or_default();
            let title = format!("{}   {}-{}", s.name.to_uppercase(), fmt_time(s.a), fmt_time(s.b));
            let sub = song.clone();
            if let Some(r) = list_row(ui, k, None, || title.clone(), || sub.clone(), RowState::Normal, false, false) {
                let r = r.on_hover_cursor(egui::CursorIcon::PointingHand).tip(if s.note.is_empty() {
                    "Click to play this loop.".to_string()
                } else {
                    s.note.clone()
                });
                if r.clicked() {
                    if let Some(v) = &v {
                        acts.push(Action::PlaySaved(v.clone(), Some((s.a, s.b))));
                    }
                }
                r.context_menu(|ui| {
                    if menu_item(ui, "Rename / Add a Note...") {
                        self.edit_loop(*id, *i, s, false);
                        ui.close_menu();
                    }
                    for other in groups.iter().filter(|o| **o != s.group) {
                        if menu_item(ui, &format!("Move to {}", other)) {
                            acts.push(Action::LoopGroup(*id, *i, other.clone()));
                            ui.close_menu();
                        }
                    }
                    if !s.group.is_empty() && menu_item(ui, "Remove from Group") {
                        acts.push(Action::LoopGroup(*id, *i, String::new()));
                        ui.close_menu();
                    }
                    if menu_item(ui, "Delete This Loop") {
                        acts.push(Action::ForgetLoop(*id, *i));
                        ui.close_menu();
                    }
                });
            }
            if !s.note.trim().is_empty() && note_under(ui, &s.note, 16.0).clicked() {
                self.edit_loop(*id, *i, s, false);
            }
            if self.clip_edit == Some((*id, *i)) && !self.edit_in_practice {
                self.loop_editor(ui, acts);
            }
        }
        ui.add_space(20.0);
    }

    /// + NEW GROUP, opening a name box.
    pub(crate) fn new_group_row(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if !self.grp_adding {
                if retro_btn(ui, "+ NEW GROUP", false).clicked() {
                    self.grp_adding = true;
                    self.ed.id = crate::tools::F_GROUP_NAME + 100;
                    self.ed.anchor = None;
                }
                return;
            }
            let (r, _) = ui.allocate_exact_size(Vec2::new(200.0, bh()), Sense::hover());
            let o =
                field(ui, &mut self.ed, crate::tools::F_GROUP_NAME + 100, &mut self.grp_new, r, "group name", false);
            if (o.enter || retro_btn(ui, "ADD", false).clicked()) && !self.grp_new.trim().is_empty() {
                let mut l = self.store.groups();
                if !l.iter().any(|g| g == self.grp_new.trim()) {
                    l.push(self.grp_new.trim().to_string());
                }
                self.store.loop_groups = Some(l);
                self.grp_new.clear();
                self.grp_adding = false;
                self.ed.id = 0;
                self.store_dirty = true;
            }
            if retro_btn(ui, "CANCEL", false).clicked() {
                self.grp_adding = false;
                self.grp_new.clear();
                self.ed.id = 0;
            }
        });
    }

    // ---------------------------------------------------------------- DIARY
    /// LOOPS (under PRACTICE): every song you saved a loop on or practised, without making it a tune - this week's
    /// first, or all of them. Click a loop to jump straight to it, looping.
    pub(crate) fn loops_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        self.name_old_loops();
        let data = self.spot_data();
        // the song picked: one asked for, else the song playing, else the first
        let playing = self.cur_track().map(|t| t.id);
        let has = |id: i64| data.songs.iter().any(|s| s.id == id);
        let pick = self.loops_pick.filter(|p| has(*p)).or(playing.filter(|p| has(*p))).or(data.songs.first().map(|s| s.id));
        if ui.available_width() >= 640.0 {
            // two panes, each scrolling on its own: the songs on the left (under the filter, which stays), the one
            // picked on the right (its waveform stays in sight as the list moves)
            ui.add_space(6.0);
            title_line(ui, "LOOPS", 3.0, pal().ink);
            let full = ui.available_rect_before_wrap();
            let lw = 250.0;
            let left = Rect::from_min_max(full.min, Pos2::new(full.min.x + lw, full.max.y));
            let right = Rect::from_min_max(Pos2::new(left.max.x + 14.0, full.min.y), full.max);
            ui.painter().line_segment(
                [Pos2::new(left.max.x + 6.0, full.min.y), Pos2::new(left.max.x + 6.0, full.max.y - 8.0)],
                Stroke::new(1.0_f32, pal().row_alt),
            );
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(left), |ui| {
                ui.set_width(lw);
                self.loops_controls(ui, acts);
                ui.add_space(6.0);
                // (a test copy can open the list scrolled down: BACKLINE_SCROLL=points)
                let mut sa = egui::ScrollArea::vertical().id_salt("loops_songs").auto_shrink([false, false]);
                if let Some(y) = std::env::var("BACKLINE_SCROLL").ok().and_then(|s| s.parse::<f32>().ok()) {
                    sa = sa.vertical_scroll_offset(y);
                }
                sa.show(ui, |ui| {
                    self.loops_songs(ui, &data, pick);
                    ui.add_space(20.0);
                });
            });
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right), |ui| {
                egui::ScrollArea::vertical().id_salt("loops_detail").auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    self.loops_detail(ui, acts, &data, pick);
                    ui.add_space(20.0);
                });
            });
            ui.allocate_rect(full, Sense::hover());
            return;
        }
        // narrow: the page scrolls as one
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| self.loops_narrow(ui, acts, &data));
    }

    /// The Loops page in a narrow window: the songs, or the one opened (the list a click away).
    fn loops_narrow(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, data: &SpotData) {
        ui.add_space(6.0);
        title_line(ui, "LOOPS", 3.0, pal().ink);
        let has = |id: i64| data.songs.iter().any(|s| s.id == id);
        if let Some(open) = self.loops_pick.filter(|p| has(*p)) {
            // narrow: one song at a time, the list a click away
            if retro_btn(ui, "< ALL SONGS", false).clicked() {
                self.loops_pick = None;
            }
            ui.add_space(8.0);
            self.loops_detail(ui, acts, data, Some(open));
        } else {
            self.loops_controls(ui, acts);
            ui.add_space(6.0);
            self.loops_songs(ui, data, None);
            if self.loops_mode == 2 {
                ui.add_space(14.0);
                self.groups_view(ui, acts);
            }
        }
        ui.add_space(20.0);
    }

    /// The filter, which week, and the rest (what to show, the order, export / import) in "...".
    pub(crate) fn loops_controls(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let w = ui.available_width().min(250.0);
        ui.horizontal(|ui| {
            let (ic, _) = ui.allocate_exact_size(Vec2::new(22.0, bh()), Sense::hover());
            crate::icon_in(ui.painter(), ic.shrink(4.0), &crate::IC_FILTER, pal().ink2);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w - 30.0, bh()), Sense::hover());
            let _ = field(ui, &mut self.ed, crate::tools::F_LOOPS_FILTER, &mut self.loops_filter, rect, "Filter", false);
        });
        ui.horizontal(|ui| {
            if retro_btn(ui, "THIS WEEK", !self.loops_all).tip("Songs practiced or looped in the last 7 days.").clicked() {
                self.loops_all = false;
            }
            if retro_btn(ui, "ALL", self.loops_all).tip("Every song with a loop or a bookmark.").clicked() {
                self.loops_all = true;
            }
            let more = retro_btn_w(ui, "...", 34.0, false).tip("More options: show, sort, export and import.");
            let pop = ui.id().with("loops_more");
            if more.clicked() {
                ui.memory_mut(|m| m.toggle_popup(pop));
            }
            egui::popup_below_widget(ui, pop, &more, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(230.0);
                dim_line(ui, "SHOW", 1.0, pal().dim);
                for (i, name) in ["Loops and bookmarks", "Loops", "Bookmarks"].iter().enumerate() {
                    let mark = if self.loops_show as usize == i { "> " } else { "   " };
                    if menu_item(ui, &format!("{}{}", mark, name)) {
                        self.loops_show = i as u8;
                        self.dirty = true;
                    }
                }
                dim_line(ui, "SORT", 1.0, pal().dim);
                let cur = if self.loops_mode == 2 { 3 } else { self.loops_sort.min(2) as usize };
                for (i, name) in ["Recent", "A-Z", "Time spent", "By group"].iter().enumerate() {
                    let mark = if cur == i { "> " } else { "   " };
                    if menu_item(ui, &format!("{}{}", mark, name)) {
                        if i == 3 {
                            self.loops_mode = 2;
                        } else {
                            self.loops_mode = 0;
                            self.loops_sort = i as u8;
                        }
                        self.dirty = true;
                    }
                }
                dim_line(ui, "SHARE", 1.0, pal().dim);
                if menu_item(ui, "   Export Loops & Playlists...") {
                    acts.push(Action::ShareExport);
                }
                if menu_item(ui, "   Import from Another Computer...") {
                    acts.push(Action::ShareImport);
                }
            });
        });
    }

    /// The songs, filtered and in order, with their loops and bookmarks.
    pub(crate) fn spot_data(&self) -> SpotData {
        let today = store::today_days();
        let week_start = store::date_str(today - 6);
        let mut week: HashMap<i64, u32> = HashMap::new();
        for d in self.store.days.iter().filter(|d| d.date >= week_start) {
            for (id, s) in &d.songs {
                *week.entry(*id).or_default() += s;
            }
        }
        // time spent on each loop, from the diary
        let mut spent: HashMap<(i64, i32, i32), u32> = HashMap::new();
        for d in &self.store.days {
            for (k, v) in &d.loop_time {
                if let Some((id, a, b)) = store::parse_loop_key(k) {
                    *spent.entry((id, (a * 10.0).round() as i32, (b * 10.0).round() as i32)).or_default() += v.0;
                }
            }
        }
        let spent_on = |id: i64, s: &store::Section| {
            spent.get(&(id, (s.a * 10.0).round() as i32, (s.b * 10.0).round() as i32)).copied().unwrap_or(0)
        };
        // the last day each song was practiced (the diary) or played (what you played lately), for ALL's months
        let touched = self.last_days();
        let (show_loops, show_marks) = (self.loops_show != 2, self.loops_show != 1);
        // the songs: practised this week, or with a loop saved this week (ALL: every song with a loop or bookmark)
        let mut ids: Vec<i64> = week.keys().copied().collect();
        for (id, l) in &self.store.sections {
            if !l.is_empty() && (self.loops_all || l.iter().any(|s| s.saved >= week_start)) && !ids.contains(id) {
                ids.push(*id);
            }
        }
        if self.loops_all {
            for (id, l) in &self.store.marks {
                if !l.is_empty() && !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        let filter = self.loops_filter.trim().to_string();
        let mut songs: Vec<SpotSong> = Vec::new();
        for id in &ids {
            let v = self.store.songs.get(id).cloned();
            let song = v.as_ref().map(|v| format!("{} {} {}", v.artist, v.title, v.album)).unwrap_or_default();
            let song_hit = filter.is_empty() || crate::filter_score(&filter, &[&song]).is_some();
            let hit = |texts: &[&str]| song_hit || crate::filter_score(&filter, texts).is_some();
            let loops: Vec<(usize, store::Section, u32)> = if show_loops {
                self.store
                    .sections
                    .get(id)
                    .into_iter()
                    .flatten()
                    .enumerate()
                    .filter(|(_, s)| hit(&[&s.name, &s.group, &s.note]))
                    .map(|(i, s)| (i, s.clone(), spent_on(*id, s)))
                    .collect()
            } else {
                Vec::new()
            };
            let marks: Vec<(usize, store::Bookmark)> = if show_marks {
                self.store.marks.get(id).into_iter().flatten().cloned().enumerate().filter(|(_, m)| hit(&[&m.name, &m.note])).collect()
            } else {
                Vec::new()
            };
            // a song only practised shows when nothing narrows the list
            if loops.is_empty() && marks.is_empty() && (!song_hit || !filter.is_empty() || self.loops_show == 2) {
                continue;
            }
            let mins = week.get(id).copied().unwrap_or(0);
            let total = loops.iter().map(|l| l.2).sum::<u32>() + mins;
            let newest = loops.iter().map(|l| l.1.saved.clone()).max().unwrap_or_default();
            // (any loop of the song's counts, also one the filter hides)
            let saved = self.store.sections.get(id).into_iter().flatten().filter_map(|s| store::parse_date(&s.saved)).max().unwrap_or(0);
            let marked = self.store.marks.get(id).into_iter().flatten().filter_map(|m| store::parse_date(&m.saved)).max().unwrap_or(0);
            let saved = saved.max(marked);
            let last = touched.get(id).copied().unwrap_or(0).max(saved);
            songs.push(SpotSong { id: *id, v, loops, marks, mins, total, newest, last });
        }
        let name_of = |s: &SpotSong| s.v.as_ref().map(|v| v.title.to_lowercase()).unwrap_or_default();
        match self.loops_sort {
            1 => songs.sort_by(|a, b| name_of(a).cmp(&name_of(b)).then(a.id.cmp(&b.id))),
            2 => songs.sort_by(|a, b| b.total.cmp(&a.total).then(a.id.cmp(&b.id))),
            // ALL, most recent first: the newest touched on top, under the months' headings
            _ if self.loops_all => songs.sort_by(|a, b| b.last.cmp(&a.last).then(b.newest.cmp(&a.newest)).then(a.id.cmp(&b.id))),
            _ => songs.sort_by(|a, b| b.mins.cmp(&a.mins).then(b.newest.cmp(&a.newest)).then(a.id.cmp(&b.id))),
        }
        SpotData { songs }
    }

    /// The left pane: one two-line row per song.
    pub(crate) fn loops_songs(&mut self, ui: &mut egui::Ui, data: &SpotData, pick: Option<i64>) {
        let playing = self.cur_track().map(|t| t.id);
        let row = |ui: &mut egui::Ui, key: i64, title: &str, sub: &str| -> bool {
            let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
            let p = ui.painter();
            let on = pick == Some(key);
            if on {
                p.rect_filled(r, 6.0, pal().row_sel);
            } else if resp.hovered() {
                p.rect_filled(r, 6.0, pal().sel);
            }
            let x = r.min.x + 10.0;
            let col = if playing == Some(key) { pal().bar_txt } else { pal().ink };
            let clip = Rect::from_min_max(r.min, Pos2::new(r.max.x - 8.0, r.max.y)).intersect(p.clip_rect());
            ptext(&p.with_clip_rect(clip), Pos2::new(x, r.min.y + 15.0), Align::Min, title, 1.6, col);
            ptext_fit(p, Pos2::new(r.min.x + 10.0, r.min.y + 32.0), Align::Min, sub, 1.0, r.width() - 20.0, pal().ink2);
            resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
        };
        if data.songs.is_empty() {
            let msg = if !self.loops_filter.trim().is_empty() {
                "Nothing matches."
            } else if self.loops_all {
                "No loops or bookmarks yet. In PRACTICE, set A and B and press SAVE LOOP, or press + MARK."
            } else {
                "Nothing this week yet. Practice a song or save a loop and it shows up here. ALL has every song."
            };
            para(ui, msg, pal().ink2);
            return;
        }
        // ALL, most recent first: under headings, Recent, then each month, with how many songs are under each
        let months = self.loops_all && self.loops_sort == 0 && self.loops_mode != 2;
        let today = store::today_days();
        let heads: Vec<String> = if months { data.songs.iter().map(|s| when_heading(s.last, today)).collect() } else { Vec::new() };
        for (k, s) in data.songs.iter().enumerate() {
            if months && (k == 0 || heads[k] != heads[k - 1]) {
                let n = heads[k..].iter().take_while(|h| **h == heads[k]).count();
                if k > 0 {
                    ui.add_space(16.0);
                }
                when_head(ui, &heads[k], n);
                ui.add_space(4.0);
            }
            let (title, artist) = match &s.v {
                Some(v) => (v.title.clone(), v.artist.clone()),
                None => ("Unknown song".to_string(), String::new()),
            };
            let spots = s.loops.len() + s.marks.len();
            let mut sub = artist;
            if spots > 0 {
                sub = format!("{}{}{} {}", sub, if sub.is_empty() { "" } else { "  ·  " }, spots, if spots == 1 { "spot" } else { "spots" });
            }
            if s.mins >= 60 {
                // (lower case, as "spots" is: the line is mixed case and stays as written)
                sub = format!("{}  ·  {}", sub, mins_text(s.mins).to_lowercase());
            }
            if row(ui, s.id, &title, &sub) {
                self.loops_pick = Some(s.id);
            }
        }
    }

    /// The right pane: the song picked (its map, then each loop and bookmark), or the groups.
    pub(crate) fn loops_detail(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, data: &SpotData, pick: Option<i64>) {
        if self.loops_mode == 2 {
            self.groups_view(ui, acts);
            return;
        }
        let Some(c) = data.songs.iter().find(|s| Some(s.id) == pick) else { return };
        let playing = self.cur_track().map(|t| t.id) == Some(c.id);
        let (la, lb, now) = (self.loop_a, self.loop_b, self.pos());
        // ---- the song, and a button to play it
        let (title, artist) = match &c.v {
            Some(v) => (v.title.clone(), v.artist.clone()),
            None => ("Unknown song".to_string(), String::new()),
        };
        let (hr, hresp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
        let play_r = Rect::from_center_size(Pos2::new(hr.max.x - 18.0, hr.center().y), Vec2::splat(30.0));
        {
            let p = ui.painter();
            let room = (hr.width() - 60.0).max(40.0);
            let clip = Rect::from_min_max(hr.min, Pos2::new(hr.min.x + room, hr.max.y)).intersect(p.clip_rect());
            let tw = ptext(&p.with_clip_rect(clip), Pos2::new(hr.min.x, hr.center().y), Align::Min, &title, 2.6, if playing { pal().bar_txt } else { pal().ink });
            if !artist.is_empty() && tw < room && room - tw > 70.0 {
                ptext_fit(p, Pos2::new(hr.min.x + tw + 12.0, hr.center().y + 2.0), Align::Min, &artist, 1.4, room - tw - 16.0, pal().ink2);
            }
            // (a song never found has nothing to play: no button)
            if c.v.is_some() {
                p.circle_filled(play_r.center(), 15.0, pal().bar_txt);
                let c0 = play_r.center();
                p.add(egui::Shape::convex_polygon(
                    vec![c0 + Vec2::new(-4.0, -6.5), c0 + Vec2::new(7.0, 0.0), c0 + Vec2::new(-4.0, 6.5)],
                    pal().app_bg,
                    Stroke::NONE,
                ));
            }
        }
        if c.v.is_none() {
            hresp.clone().tip("Backline doesn't know which song this is: it was bookmarked before songs were kept with their bookmarks.");
        }
        if let Some(v) = &c.v {
            let pr = ui.interact(play_r, ui.id().with(("spot_play", c.id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip("Play the song.");
            if pr.clicked() {
                acts.push(Action::PlaySaved(v.clone(), None));
            }
            let liked = self.liked.contains(&v.id);
            hresp.context_menu(|ui| {
                crate::song_items(ui, acts, &v.to_track(), liked, v.kind == "tidal");
            });
        }
        ui.add_space(8.0);
        // ---- the map: the song end to end, bookmarks named above their ticks, loops as bands, time below
        let len = c
            .v
            .as_ref()
            .map(|v| v.dur)
            .filter(|d| *d > 1.0)
            .or_else(|| song_wave(c.id).map(|w| w.len() as f32 / 50.0).filter(|d| *d > 1.0))
            .unwrap_or_else(|| {
                let lmax = c.loops.iter().map(|l| l.1.b).fold(0.0f32, f32::max);
                let mmax = c.marks.iter().map(|m| m.1.t).fold(0.0f32, f32::max);
                lmax.max(mmax) * 1.08 + 5.0
            });
        let (all, sresp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 16.0 + 60.0 + 16.0), Sense::click());
        let sr = Rect::from_min_size(Pos2::new(all.min.x, all.min.y + 16.0), Vec2::new(all.width(), 60.0));
        let at = |t: f32| sr.min.x + (t / len).clamp(0.0, 1.0) * sr.width();
        {
            let p = ui.painter();
            p.rect_filled(sr, 5.0, pal().row_alt);
            if let Some(w) = song_wave(c.id) {
                let n = sr.width().max(1.0) as usize;
                let wl = (w.len() as f32 / 50.0).max(0.1);
                let mut x = sr.min.x;
                for k in (0..n).step_by(2) {
                    let (t0, t1) = (k as f32 / n as f32 * len, (k + 2) as f32 / n as f32 * len);
                    let (i0, i1) = ((t0 / wl * w.len() as f32) as usize, (t1 / wl * w.len() as f32) as usize);
                    // the slice's average, not its loudest peak, and spread out: a loud mastered song still shows its shape
                    let slice = w.get(i0.min(w.len())..i1.max(i0 + 1).min(w.len())).unwrap_or(&[]);
                    let avg = slice.iter().map(|&x| x as f32).sum::<f32>() / slice.len().max(1) as f32 / 255.0;
                    let h = avg.powi(4) * (sr.height() / 2.0 - 3.0) + 1.0;
                    p.line_segment(
                        [Pos2::new(x, sr.center().y - h), Pos2::new(x, sr.center().y + h)],
                        Stroke::new(1.0_f32, pal().ink2.gamma_multiply(0.45)),
                    );
                    x += 2.0;
                }
            }
            for (_, s, _) in &c.loops {
                let band = Rect::from_min_max(Pos2::new(at(s.a), sr.min.y), Pos2::new(at(s.b).max(at(s.a) + 3.0), sr.max.y));
                let on = playing && la.is_some_and(|a| (a - s.a).abs() < 0.05) && lb.is_some_and(|b| (b - s.b).abs() < 0.05);
                p.rect_filled(band, 2.0, pal().bar_txt.gamma_multiply(if on { 0.6 } else { 0.32 }));
            }
            // bookmark names above their ticks, skipping one that would land on the last
            let mut last_end = f32::MIN;
            for (_, m) in &c.marks {
                let col = Color32::from_rgb(m.col[0], m.col[1], m.col[2]);
                let x = at(m.t);
                p.line_segment([Pos2::new(x, sr.min.y), Pos2::new(x, sr.max.y)], Stroke::new(2.0_f32, col));
                if !m.name.is_empty() && x - 2.0 > last_end {
                    let w = ptext(p, Pos2::new(x - 2.0, all.min.y + 7.0), Align::Min, &m.name, 1.0, col);
                    last_end = x + w + 6.0;
                }
            }
            if playing {
                let x = at(now);
                p.line_segment([Pos2::new(x, sr.min.y - 2.0), Pos2::new(x, sr.max.y + 2.0)], Stroke::new(1.5_f32, pal().ink));
            }
            for k in 0..4 {
                let t = len * k as f32 / 3.0;
                let al = match k {
                    0 => Align::Min,
                    3 => Align::Max,
                    _ => Align::Center,
                };
                ptext(p, Pos2::new(sr.min.x + sr.width() * k as f32 / 3.0, sr.max.y + 9.0), al, &fmt_time(t), 1.0, pal().dim);
            }
        }
        if let Some(v) = &c.v {
            let sresp = sresp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if let Some(pp) = sresp.hover_pos().filter(|pp| sr.contains(*pp)) {
                let t = ((pp.x - sr.min.x) / sr.width()).clamp(0.0, 1.0) * len;
                egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), ui.id().with(("strip_tip", c.id)), |ui| {
                    ui.label(format!("Play from {}", fmt_time(t)));
                });
                if sresp.clicked() {
                    acts.push(Action::PlayAt(v.clone(), t));
                }
            }
        }
        ui.add_space(10.0);
        // ---- each loop and bookmark: starred loops first, the rest in song order
        if c.loops.is_empty() && c.marks.is_empty() {
            para(ui, "Practiced, nothing saved yet. Set A and B in PRACTICE and press SAVE LOOP, or press + MARK.", pal().ink2);
            return;
        }
        enum Row<'a> {
            Loop(usize, &'a store::Section, u32),
            Mark(usize, &'a store::Bookmark),
        }
        let mut rows: Vec<(bool, f32, Row)> = c.loops.iter().map(|(i, s, w)| (s.fav, s.a, Row::Loop(*i, s, *w))).collect();
        rows.extend(c.marks.iter().map(|(i, m)| (false, m.t, Row::Mark(*i, m))));
        rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)));
        let mut drop_mark = None;
        for (_, _, row) in &rows {
            match row {
                Row::Loop(i, s, w) => {
                    let on = playing && la.is_some_and(|a| (a - s.a).abs() < 0.05) && lb.is_some_and(|b| (b - s.b).abs() < 0.05);
                    let nl = note_lines(&s.note, ui.available_width() - 80.0);
                    let h = if nl.is_empty() { 32.0 } else { 30.0 + nl.len() as f32 * note_lh() + 6.0 };
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::click());
                    let p = ui.painter();
                    if on {
                        p.rect_filled(r, 6.0, pal().row_sel);
                    } else if resp.hovered() {
                        p.rect_filled(r, 6.0, pal().sel);
                    }
                    let cy = r.min.y + 16.0;
                    p.rect_filled(Rect::from_center_size(Pos2::new(r.min.x + 18.0, cy), Vec2::new(14.0, 8.0)), 2.0, pal().bar_txt.gamma_multiply(0.65));
                    let mut meta = format!("{}-{}", fmt_time(s.a), fmt_time(s.b));
                    if *w >= 60 {
                        meta = format!("{}   {}", meta, mins_text(*w));
                    }
                    if !s.group.is_empty() {
                        meta = format!("{}   {}", s.group.to_uppercase(), meta);
                    }
                    let mw = ptext(p, Pos2::new(r.max.x - 40.0, cy), Align::Max, &meta, 1.0, pal().ink2);
                    ptext_fit(p, Pos2::new(r.min.x + 36.0, cy), Align::Min, &s.name, 1.6, (r.width() - mw - 90.0).max(20.0), if on { pal().bar_txt } else { pal().ink });
                    draw_note_lines(p, r.min.x + 36.0, r.min.y + 28.0, &nl, pal().ink2);
                    let star_r = Rect::from_center_size(Pos2::new(r.max.x - 18.0, cy), Vec2::splat(24.0));
                    let sh = ui
                        .interact(star_r, ui.id().with(("loop_star", c.id, *i)), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .tip(if s.fav { "Starred. Click to unstar." } else { "Star it as one that matters, like a part for a gig coming up." });
                    let star_col = if s.fav {
                        STAR_COL
                    } else if sh.hovered() || resp.hovered() {
                        pal().dim
                    } else {
                        pal().dim.gamma_multiply(0.35)
                    };
                    crate::icon_in(ui.painter(), star_r.shrink(6.0), &crate::IC_STAR, star_col);
                    if sh.clicked() {
                        if let Some(x) = self.store.sections.get_mut(&c.id).and_then(|l| l.get_mut(*i)) {
                            x.fav = !x.fav;
                            self.store_dirty = true;
                        }
                    }
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Play this loop. Right-click to rename or add a note.");
                    if resp.clicked() && !sh.clicked() {
                        if let Some(v) = &c.v {
                            acts.push(Action::PlaySaved(v.clone(), Some((s.a, s.b))));
                        }
                    }
                    resp.context_menu(|ui| {
                        if menu_item(ui, if s.note.is_empty() { "Rename / Add a Note..." } else { "Rename / Edit the Note..." }) {
                            self.edit_loop(c.id, *i, s, false);
                            ui.close_menu();
                        }
                        if let Some(v) = &c.v {
                            if menu_item(ui, "Add This Loop to a Playlist...") {
                                acts.push(Action::PlaylistPicker(v.to_track(), Some((s.a, s.b, s.name.clone()))));
                                ui.close_menu();
                            }
                        }
                        if menu_item(ui, "Delete This Loop") {
                            acts.push(Action::ForgetLoop(c.id, *i));
                            ui.close_menu();
                        }
                    });
                    if self.clip_edit == Some((c.id, *i)) && !self.edit_in_practice {
                        self.loop_editor(ui, acts);
                    }
                }
                Row::Mark(i, m) => {
                    let nl = note_lines(&m.note, ui.available_width() - 80.0);
                    let h = if nl.is_empty() { 32.0 } else { 30.0 + nl.len() as f32 * note_lh() + 6.0 };
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::click());
                    let p = ui.painter();
                    if resp.hovered() {
                        p.rect_filled(r, 6.0, pal().sel);
                    }
                    let cy = r.min.y + 16.0;
                    let col = Color32::from_rgb(m.col[0], m.col[1], m.col[2]);
                    p.rect_filled(Rect::from_center_size(Pos2::new(r.min.x + 18.0, cy), Vec2::new(3.0, 14.0)), 1.0, col);
                    let mw = ptext(p, Pos2::new(r.max.x - 40.0, cy), Align::Max, &fmt_time(m.t), 1.0, pal().ink2);
                    let name = if m.name.is_empty() { "Bookmark" } else { &m.name };
                    ptext_fit(p, Pos2::new(r.min.x + 36.0, cy), Align::Min, name, 1.6, (r.width() - mw - 90.0).max(20.0), if m.name.is_empty() { pal().ink2 } else { pal().ink });
                    draw_note_lines(p, r.min.x + 36.0, r.min.y + 28.0, &nl, pal().ink2);
                    let resp = resp
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .tip("Play from this bookmark. Right-click to rename, add a note or remove it.");
                    if resp.clicked() {
                        if let Some(v) = &c.v {
                            acts.push(Action::PlayAt(v.clone(), m.t));
                        }
                    }
                    resp.context_menu(|ui| {
                        if menu_item(ui, if m.note.is_empty() { "Rename / Add a Note..." } else { "Rename / Edit the Note..." }) {
                            self.edit_mark(c.id, *i, m, false);
                            ui.close_menu();
                        }
                        if menu_item(ui, "Remove This Bookmark") {
                            drop_mark = Some(*i);
                            ui.close_menu();
                        }
                    });
                    if self.mark_edit == Some((c.id, *i)) && !self.edit_in_practice {
                        self.mark_editor(ui, true);
                    }
                }
            }
        }
        if let Some(k) = drop_mark {
            if let Some(l) = self.store.marks.get_mut(&c.id) {
                if k < l.len() {
                    l.remove(k);
                    self.store_dirty = true;
                }
            }
            self.mark_edit = None;
        }
    }

    /// Renaming a bookmark or giving it a note (as a loop's editor, without the group); `with_name`: the name too
    /// (the BOOKMARKS tab has it on the bookmark's row). Ctrl + Enter saves.
    pub(crate) fn mark_editor(&mut self, ui: &mut egui::Ui, with_name: bool) {
        let Some((id, k)) = self.mark_edit else { return };
        let o1 = if with_name {
            field_row(ui, &mut self.ed, crate::tools::F_CLIP_NAME, "NAME", &mut self.clip_name, "solo, bridge, key change", None)
        } else {
            FieldOut { changed: false, enter: false }
        };
        note_row(ui, &mut self.ed, crate::tools::F_CLIP_NOTE, &mut self.clip_note, "what happens here, what to work on");
        let mut done = o1.enter || save_keys(ui);
        ui.horizontal_wrapped(|ui| {
            ui.add_space(12.0);
            if retro_btn(ui, "SAVE", false).clicked() {
                done = true;
            }
            if retro_btn(ui, "CANCEL", false).clicked() {
                self.mark_edit = None;
                self.ed.id = 0;
            }
        });
        if done {
            if let Some(m) = self.store.marks.get_mut(&id).and_then(|l| l.get_mut(k)) {
                if with_name {
                    m.name = self.clip_name.trim().to_string();
                }
                m.note = self.clip_note.trim().to_string();
                self.store_dirty = true;
            }
            self.mark_edit = None;
            self.ed.id = 0;
        }
        ui.add_space(6.0);
    }

    /// Loops and bookmarks saved before their songs were remembered: name them from wherever the song is known
    /// (tunes, files, playlists, what you played, the queue). Each song is looked for once a run.
    /// The last day each song was practiced (the diary) or played here (what you played lately), as days since 1970.
    fn last_days(&self) -> HashMap<i64, i64> {
        let mut last: HashMap<i64, i64> = HashMap::new();
        for d in &self.store.days {
            if let Some(n) = store::parse_date(&d.date) {
                for id in d.songs.keys() {
                    let e = last.entry(*id).or_insert(0);
                    *e = (*e).max(n);
                }
            }
        }
        for (v, at) in &self.store.recent {
            let e = last.entry(v.id).or_insert(0);
            *e = (*e).max((*at / 86_400) as i64);
        }
        last
    }

    /// Bookmarks set before their day was kept: dated from the song's last play this computer knows, so ALL's months
    /// place them - here, and (synced) on the other computer, whose list of what was played lately is its own. Once a
    /// run.
    fn date_old_marks(&mut self) {
        thread_local! {
            static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
        }
        if DONE.with(|d| d.replace(true)) {
            return;
        }
        let last = self.last_days();
        let mut changed = false;
        for (id, l) in self.store.marks.iter_mut() {
            let Some(day) = last.get(id).copied().filter(|d| *d > 0) else { continue };
            for m in l.iter_mut().filter(|m| m.saved.is_empty()) {
                m.saved = store::date_str(day);
                changed = true;
            }
        }
        if changed {
            self.store_dirty = true;
        }
    }

    pub(crate) fn name_old_loops(&mut self) {
        self.date_old_marks();
        thread_local! {
            static TRIED: std::cell::RefCell<std::collections::HashSet<i64>> = Default::default();
        }
        let marked = self.store.marks.iter().filter(|(_, l)| !l.is_empty()).map(|(id, _)| id);
        let missing: Vec<i64> = self
            .store
            .sections
            .keys()
            .chain(marked)
            .copied()
            .filter(|id| !self.store.songs.contains_key(id) && TRIED.with(|t| t.borrow_mut().insert(*id)))
            .collect();
        for id in missing {
            let found = self
                .store
                .tunes
                .iter()
                .flat_map(|t| t.versions.iter())
                .find(|v| v.id == id)
                .cloned()
                .or_else(|| {
                    self.store
                        .files
                        .iter()
                        .chain(&self.store.yt)
                        .chain(&self.store.sc)
                        .chain(&self.store.hearts)
                        .find(|e| e.id == id)
                        .map(|e| e.to_version())
                })
                .or_else(|| self.store.playlists.iter().flat_map(|p| p.items.iter()).find(|v| v.id == id).cloned())
                .or_else(|| self.store.recent.iter().find(|(v, _)| v.id == id).map(|(v, _)| v.clone()))
                .or_else(|| self.store.last.as_ref().and_then(|l| l.queue.iter().find(|v| v.id == id).cloned()))
                .or_else(|| self.queue.iter().find(|t| t.id == id).map(|t| self.version_of(t)));
            if let Some(v) = found {
                self.store.songs.insert(id, v);
                self.store_dirty = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn months_as_headings() {
        let today = store::days_from_civil(2026, 11, 10);
        assert_eq!(when_heading(today, today), "Recent");
        assert_eq!(when_heading(today - 6, today), "Recent");
        assert_eq!(when_heading(store::days_from_civil(2026, 11, 1), today), "Recent", "all of this month");
        assert_eq!(when_heading(store::days_from_civil(2026, 10, 31), today), "October");
        assert_eq!(when_heading(store::days_from_civil(2026, 1, 1), today), "January");
        assert_eq!(when_heading(store::days_from_civil(2025, 12, 31), today), "December 2025");
        assert_eq!(when_heading(0, today), "Earlier");
    }
}
