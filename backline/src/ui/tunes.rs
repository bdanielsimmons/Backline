//! TUNES: your repertoire (how well you know each, names to look up), a tune's own page (progress, key, tempo,
//! notes, its versions and the diary's mentions) and the recommended tunes.

use super::*;

pub(crate) const COMFORT: [&str; 4] = ["LEARNING", "OK", "GOOD", "COMFORTABLE"];

/// How well you know a tune: four pips, click one to set the level.
pub(crate) fn comfort_meter(ui: &mut egui::Ui, level: u8) -> Option<u8> {
    let level = level.min(3);
    let mut hit = None;
    ui.horizontal_wrapped(|ui| {
        label(ui, "PROGRESS", 74.0);
        for n in 0..4u8 {
            let (r, resp) = ui.allocate_exact_size(Vec2::new(30.0, bh()), Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(COMFORT[n as usize]);
            let on = n <= level;
            let p = ui.painter();
            inset(p, r, pal().panel_dk);
            if on {
                fill_rect(p, r.shrink(5.0), if level == 3 { pal().ink } else { pal().red });
            }
            if resp.clicked() {
                hit = Some(n);
            }
        }
        ui.add_space(8.0);
        lcd_box(ui, COMFORT[level as usize], 130.0, pal().ink);
    });
    hit
}

impl App {
    // ---------------------------------------------------------------- TUNES
    /// While a tune name is typed: cards for the songs it could be (the recommended tunes, your own, and the songs in
    /// your library and queue), spelled right - accents and capitals - so a name typed fast comes out right. Click one
    /// to add it under that name, or KEEP AS TYPED.
    pub(crate) fn tune_suggestions(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let typed = self.new_tune.trim().to_string();
        // only the cards asked for (Enter or ADD), and only while that name is still in the box
        if typed.is_empty() || TUNE_ASKED.with(|a| *a.borrow() != typed) {
            return;
        }
        let found = TUNE_FOUND.with(|f| f.borrow().clone());
        // nothing to offer, or exactly what was typed: no cards
        if found.is_empty() || (found.len() == 1 && found[0].1 == typed) {
            return;
        }
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
            for (_, n, who) in &found {
                let w = (text_w(n, 2.0).max(text_w(who, 1.3)) + 28.0).clamp(120.0, 300.0);
                let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 44.0), Sense::click());
                let hot = resp.hovered();
                ui.painter().rect_filled(r, crate::corner(), if hot { pal().panel } else { pal().panel.lerp_to_gamma(pal().display, 0.35) });
                ui.painter().rect_stroke(r, crate::corner(), egui::Stroke::new(1.0_f32, pal().panel_hover));
                crate::font::keep_caps(|| {
                    crate::row_text(ui.painter(), Pos2::new(r.min.x + 12.0, r.min.y + 15.0), Align::Min, n, w - 20.0, pal().ink);
                    crate::row_text(
                        ui.painter(),
                        Pos2::new(r.min.x + 12.0, r.min.y + 32.0),
                        Align::Min,
                        who,
                        w - 20.0,
                        pal().ink2,
                    );
                });
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Add with this name.").clicked() {
                    self.new_tune = n.clone();
                    TUNE_ASKED.with(|a| a.borrow_mut().clear());
                    acts.push(Action::NewTune);
                }
            }
            if retro_btn(ui, "KEEP AS TYPED", false)
                .tip(format!("Add \"{}\" as typed  (Enter)", typed))
                .clicked()
            {
                TUNE_ASKED.with(|a| a.borrow_mut().clear());
                acts.push(Action::NewTune);
            }
        });
        ui.add_space(4.0);
    }

    /// The tune names closest to `typed`, best first (at most 5): (score, name, who).
    pub(crate) fn tune_matches(&self, typed: &str) -> Vec<(u32, String, String)> {
        // (name, who) from every place a tune name could come from
        let mut pool: Vec<(String, String)> = Vec::new();
        for kinds in [crate::canon::SESSION_TUNES, crate::canon::GOSPEL_TUNES] {
            for (_, list) in kinds.iter() {
                for (n, who) in list.iter() {
                    pool.push((n.to_string(), who.to_string()));
                }
            }
        }
        for tn in &self.store.tunes {
            pool.push((tn.name.clone(), "In your tunes".to_string()));
        }
        let mut songs: Vec<&crate::api::Track> = self.queue.iter().collect();
        if let Some(pg) = &self.page {
            songs.extend(pg.tracks.iter());
        }
        for tr in songs {
            let name = crate::chart::clean_title(&tr.title);
            if !name.is_empty() {
                pool.push((name, tr.artists_text()));
            }
        }
        // the closest first, each name once
        let mut found: Vec<(u32, String, String)> = Vec::new();
        for (n, who) in pool {
            if let Some(s) = crate::filter_score(typed, &[&n]) {
                if !found.iter().any(|f| f.1.eq_ignore_ascii_case(&n)) {
                    found.push((s, n, who));
                }
            }
        }
        found.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.len().cmp(&b.1.len())));
        found.truncate(5);
        found
    }

    /// TUNES > RECOMMENDED: the tunes that get called, by kind (jazz or gospel). Click one to see its chart on the
    /// lead sheet - it only becomes one of yours with + MY TUNES.
    pub(crate) fn recommended_tunes(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "The tunes called at sessions, sorted by kind. Knowing a mix of kinds keeps you ready for anything. Click one to see its chart on the right, and press + MY TUNES to add it.", pal().ink2);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "JAZZ", !self.tunes_gospel).clicked() {
                self.tunes_gospel = false;
            }
            if retro_btn(ui, "GOSPEL", self.tunes_gospel).clicked() {
                self.tunes_gospel = true;
            }
        });
        let kinds = if self.tunes_gospel { crate::canon::GOSPEL_TUNES } else { crate::canon::SESSION_TUNES };
        let showing = self.prac_chart.as_ref().map(|c| c.0.clone());
        for (kind, list) in kinds.iter() {
            section_header(ui, kind);
            for (name, who) in list.iter() {
                let mine = self.find_tune(name).is_some();
                let on = showing.as_deref().is_some_and(|s| s.eq_ignore_ascii_case(name));
                ui.horizontal_wrapped(|ui| {
                    if retro_btn(ui, &crate::font::as_label(name), on)
                        .tip(format!("{}. Shows its chart on the lead sheet.", who))
                        .clicked()
                    {
                        acts.push(Action::PreviewTune(name.to_string()));
                    }
                    if mine {
                        label(ui, "IN YOUR TUNES", 0.0);
                    } else if retro_btn(ui, "+ MY TUNES", false).tip("Add it to your repertoire.").clicked() {
                        acts.push(Action::AddRecommended(name.to_string()));
                    }
                    if self.store.routine_set.get(*kind).is_some_and(|s| s.eq_ignore_ascii_case(name)) {
                        label(ui, "IN YOUR SET", 0.0);
                    } else if retro_btn(ui, "MY SET", false)
                        .tip("Set as your tune for this kind in DIARY > ROUTINE.")
                        .clicked()
                    {
                        acts.push(Action::SetTune(kind.to_string(), name.to_string()));
                    }
                });
            }
        }
        ui.add_space(20.0);
    }

    pub(crate) fn tunes_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if let Some(i) = self.tune_open {
            if i < self.store.tunes.len() && self.pick.is_none() {
                self.tune_detail(ui, acts, i);
                return;
            }
        }
        ui.add_space(6.0);
        if let Some(t) = self.pick.clone() {
            para(ui, &format!("ADD \"{} - {}\" TO WHICH TUNE?", t.artist, t.title), pal().ink);
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "+ NEW TUNE FROM THIS", false).clicked() {
                    acts.push(Action::NewTuneFrom(t.clone()));
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.pick = None;
                }
            });
            ui.add_space(6.0);
        } else {
            title_line(ui, "TUNES", 3.0, pal().ink);
            // (how many: on the tab)
            let yours = format!("YOUR TUNES · {}", self.store.tunes.len());
            if let Some(i) = tab_row(ui, &[yours.as_str(), "RECOMMENDED"], self.tunes_tab as usize) {
                self.tunes_tab = i as u8;
            }
            ui.add_space(4.0);
            if self.tunes_tab == 1 {
                self.recommended_tunes(ui, acts);
                return;
            }
        }
        ui.horizontal_wrapped(|ui| {
            let w = (ui.available_width() - 78.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_NEW_TUNE, &mut self.new_tune, rect, "Add a tune, or paste an iReal link", false);
            let go = o.enter
                || retro_btn_w(ui, "ADD", 70.0, false)
                    .tip("Type a tune's name, or paste an iReal Pro link or playlist: every song in it becomes a tune with its chart.")
                    .clicked();
            if go && self.new_tune.trim().to_ascii_lowercase().starts_with("ireal") {
                // an iReal link: imported, not added as a name
                self.ireal_in = std::mem::take(&mut self.new_tune).trim().to_string();
                acts.push(Action::ImportIreal(None));
            } else if go {
                // looked up once, on Enter or ADD: a better spelling shows as cards to pick from; none, and it's added
                let typed = self.new_tune.trim().to_string();
                let found = if typed.chars().count() >= 3 { self.tune_matches(&typed) } else { Vec::new() };
                let exact = found.iter().any(|f| f.1 == typed);
                if found.is_empty() || exact || TUNE_ASKED.with(|a| *a.borrow() == typed) {
                    TUNE_ASKED.with(|a| a.borrow_mut().clear());
                    acts.push(Action::NewTune);
                } else {
                    TUNE_ASKED.with(|a| *a.borrow_mut() = typed.clone());
                    TUNE_FOUND.with(|f| *f.borrow_mut() = found);
                }
            }
        });
        self.tune_suggestions(ui, acts);
        ui.add_space(4.0);
        if self.store.tunes.is_empty() {
            para(ui, "Your repertoire, with the recordings you study for each tune. To add a recording, right-click a song in any list and choose ADD TO A TUNE. Or play it and press ADD CURRENT inside a tune.", pal().ink2);
        }
        // your tunes under the kind each is (a blues, rhythm changes, a ballad...): the kind it has in your set, else
        // the kind the session lists give it, else OTHER; the ones in your set marked
        let kinds: Vec<&str> = crate::canon::SESSION_TUNES.iter().chain(crate::canon::GOSPEL_TUNES.iter()).map(|k| k.0).collect();
        let kind_of = |name: &str| -> Option<&str> {
            if let Some((k, _)) = self.store.routine_set.iter().find(|(_, v)| v.eq_ignore_ascii_case(name)) {
                return kinds.iter().copied().find(|x| x == k);
            }
            crate::canon::SESSION_TUNES
                .iter()
                .chain(crate::canon::GOSPEL_TUNES.iter())
                .find(|(_, l)| l.iter().any(|(n, _)| n.eq_ignore_ascii_case(name)))
                .map(|k| k.0)
        };
        let mut order: Vec<(Option<&str>, usize)> =
            self.store.tunes.iter().enumerate().map(|(i, t)| (kind_of(&t.name), i)).collect();
        let rank = |k: &Option<&str>| k.map_or(usize::MAX, |k| kinds.iter().position(|x| *x == k).unwrap_or(usize::MAX - 1));
        order.sort_by_key(|(k, i)| (rank(k), *i));
        let mut last: Option<Option<&str>> = None;
        for (kind, i) in order {
            if last != Some(kind) {
                section_header(ui, kind.unwrap_or("OTHER"));
                last = Some(kind);
            }
            let t = &self.store.tunes[i];
            let in_set = self.store.routine_set.values().any(|v| v.eq_ignore_ascii_case(&t.name));
            let mins = self.store.tune_secs(&t.name) / 60;
            let r = list_row(
                ui,
                i,
                None,
                || {
                    if crate::winamp_ui::chrome_on() {
                        // (with a skin: how well you know it is the dots drawn on the row, not text)
                        format!("        {}", t.name)
                    } else {
                        format!(
                            "[{}] {}",
                            "#".repeat(t.status.min(3) as usize + 1) + &"-".repeat(3 - t.status.min(3) as usize),
                            t.name
                        )
                    }
                },
                // what you have for it, in words: in your set, recordings, time practised
                || {
                    let mut bits: Vec<String> = Vec::new();
                    if in_set {
                        bits.push("In your set".to_string());
                    }
                    bits.push(match t.versions.len() {
                        0 => "No recordings yet".to_string(),
                        1 => "1 recording".to_string(),
                        n => format!("{} recordings", n),
                    });
                    if mins > 0 {
                        bits.push(mins_text(mins * 60).to_lowercase());
                    }
                    bits.join(" · ")
                },
                RowState::Normal,
                false,
                false,
            );
            if let Some(r) = r.as_ref().filter(|_| crate::winamp_ui::chrome_on()) {
                // how well you know it: four small dots, filled up to its level
                let lv = t.status.min(3) as usize;
                let x0 = r.rect.min.x + 12.0;
                for k in 0..4 {
                    let c = Pos2::new(x0 + k as f32 * 7.0, r.rect.center().y);
                    if k <= lv {
                        ui.painter().circle_filled(c, 2.6, pal().bar_txt);
                    } else {
                        ui.painter().circle_stroke(c, 2.4, egui::Stroke::new(1.0_f32, pal().ink2));
                    }
                }
            }
            let r = r.map(|r| r.on_hover_text(COMFORT[t.status.min(3) as usize]));
            if let Some(r) = r {
                if r.clicked() {
                    match &self.pick {
                        Some(p) => acts.push(Action::AddToTune(i, p.clone())),
                        None => acts.push(Action::OpenTune(Some(i))),
                    }
                }
            }
        }
        ui.add_space(20.0);
    }

    /// Under a tune's name, after LOOK UP: its proper title when the name looks short of it or misspelled (from the
    /// recordings found, and the tunes Backline knows), to take with RENAME or leave with KEEP.
    pub(crate) fn name_guess_line(&mut self, ui: &mut egui::Ui, i: usize) {
        let name = self.store.tunes[i].name.clone();
        if self.look_busy || self.look_for != name || self.name_kept.contains(&name) {
            return;
        }
        if self.name_guess.0 != name || self.name_guess.1 != self.look_tracks.len() {
            let titles: Vec<&str> = self.look_tracks.iter().map(|t| t.0.title.as_str()).collect();
            let known: Vec<&str> = crate::canon::SESSION_TUNES
                .iter()
                .chain(crate::canon::GOSPEL_TUNES.iter())
                .flat_map(|k| k.1.iter().map(|t| t.0))
                .collect();
            let guess = crate::likely_title(&name, &titles, &known)
                // not one of your other tunes' names already
                .filter(|g| !self.store.tunes.iter().any(|t| t.name.eq_ignore_ascii_case(g)));
            self.name_guess = (name.clone(), self.look_tracks.len(), guess);
        }
        let Some(guess) = self.name_guess.2.clone() else { return };
        ui.horizontal_wrapped(|ui| {
            ui.add_space(76.0);
            label(ui, &format!("Did you mean: {}?", guess), 0.0);
            if retro_btn(ui, "RENAME", false).tip("Give the tune this name, everywhere it is used.").clicked() {
                self.store.rename_tune(&name, &guess);
                self.look_for = guess.clone();
                self.store_dirty = true;
            }
            if retro_btn(ui, "KEEP", false).tip("Keep the name as it is.").clicked() {
                self.name_kept.push(name.clone());
                self.dirty = true;
            }
        });
    }

    pub(crate) fn tune_detail(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, i: usize) {
        ui.add_space(6.0);
        let (status, nvers, secs, tname) = {
            let t = &self.store.tunes[i];
            (t.status, t.versions.len(), self.store.tune_secs(&t.name), t.name.clone())
        };
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "< TUNES", false).clicked() {
                acts.push(Action::OpenTune(None));
            }
            if retro_btn(ui, "CHART", false).tip("Its chord changes and the band, on the Chart page. Back comes here.").clicked() {
                acts.push(Action::OpenChart);
            }
        });
        ui.add_space(4.0);
        if let Some(n) = comfort_meter(ui, status) {
            acts.push(Action::Status(i, n));
        }
        let o = field_row(ui, &mut self.ed, F_T_NAME, "NAME", &mut self.store.tunes[i].name, "Tune name", None);
        self.store_dirty |= o.changed;
        self.name_guess_line(ui, i);
        let o = field_row(ui, &mut self.ed, F_T_KEY, "KEY", &mut self.store.tunes[i].key, "Bb, Dm", None);
        self.store_dirty |= o.changed;
        ui.horizontal_wrapped(|ui| {
            label(ui, "BPM", 74.0);
            if retro_btn_w(ui, "-", 28.0, false).clicked() {
                let t = &mut self.store.tunes[i];
                t.bpm = if t.bpm == 0 { 100 } else { t.bpm.saturating_sub(5).max(30) };
                self.store_dirty = true;
            }
            let b = self.store.tunes[i].bpm;
            lcd_box(ui, &if b == 0 { "---".to_string() } else { b.to_string() }, 64.0, pal().ink);
            if retro_btn_w(ui, "+", 28.0, false).clicked() {
                let t = &mut self.store.tunes[i];
                t.bpm = if t.bpm == 0 { 100 } else { (t.bpm + 5).min(300) };
                self.store_dirty = true;
            }
            ui.add_space(8.0);
            lcd_box(ui, &format!("{} MIN PRACTICED", secs / 60), 150.0, pal().ink2);
        });
        // its notes, with the formatting tools (as the diary's)
        let mut tooled = false;
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(70.0, bh()), Sense::hover());
            ptext(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, "NOTES", 2.0, pal().ink2);
            tools_at_end(ui);
            tooled = note_tools(ui, &mut self.ed, F_T_NOTES, &mut self.store.tunes[i].notes);
        });
        let hint = "What to work on, the form, tricky bars";
        let tunes = self.tune_names();
        let (o, _, asked) = notes_box(ui, &mut self.ed, F_T_NOTES, &mut self.store.tunes[i].notes, hint, 84.0, 300.0, &tunes);
        self.store_dirty |= o.changed || tooled;
        match asked {
            Some(NoteClick::Tag(t)) => self.open_notes(Some(t), "", acts),
            Some(NoteClick::Tune(j)) => self.open_tune(j, acts),
            None => {}
        }
        // the days your diary names this tune: the newest few (a click opens one), all of them in ALL NOTES
        let name = self.store.tunes[i].name.trim().to_string();
        let mut named: Vec<(i64, String)> = self
            .store
            .days
            .iter()
            .filter(|d| !name.is_empty() && crate::notes_fmt::names_tune(&d.note, &name))
            .filter_map(|d| {
                let line = d.note.lines().find(|l| crate::notes_fmt::names_tune(l, &name)).unwrap_or("");
                store::parse_date(&d.date).map(|x| (x, crate::notes_fmt::plain(line)))
            })
            .collect();
        named.sort_by_key(|a| std::cmp::Reverse(a.0));
        if !named.is_empty() {
            section_header(ui, &format!("FROM YOUR DIARY  -  {} {}", named.len(), if named.len() == 1 { "DAY" } else { "DAYS" }));
            let mut open = None;
            for (k, (d, line)) in named.iter().take(4).enumerate() {
                let (_, m, dd) = store::civil_from_days(*d);
                let when = format!("{} {} {}", store::weekday(*d), &MONTH_NAMES[m as usize - 1][..3], dd);
                let r = list_row(ui, k, None, || format!("{}   {}", when, line), String::new, RowState::Normal, false, false);
                if r.is_some_and(|r| r.tip("Open this day in the diary.").clicked()) {
                    open = Some(*d);
                }
            }
            if named.len() > 4 && retro_btn(ui, &format!("ALL {} IN THE DIARY", named.len()), false).tip("Every day that names this tune.").clicked() {
                self.open_notes(None, &name, acts);
            }
            if let Some(d) = open {
                self.open_diary_day(d, acts);
            }
            ui.add_space(4.0);
        }
        // who wrote it, when, where from: looked up once for you (MusicBrainz), yours to fix
        {
            let t = &self.store.tunes[i];
            if t.written_by.is_empty() || t.year.is_empty() {
                acts.push(Action::TuneDetails(i));
            }
        }
        // who wrote it, when, from where, the style: one line, the boxes to fix them under DETAILS
        ui.horizontal_wrapped(|ui| {
            let t = &self.store.tunes[i];
            let bits: Vec<&str> =
                [&t.written_by, &t.year, &t.from, &t.style].iter().map(|v| v.trim()).filter(|v| !v.is_empty()).collect();
            let line = if bits.is_empty() { "Composer, year, origin".to_string() } else { bits.join(" · ") };
            let room = (ui.available_width() - 110.0).max(60.0);
            let (r, _) = ui.allocate_exact_size(Vec2::new(room.min(text_w(&line, 2.0) + 12.0), bh()), Sense::hover());
            ptext_fit(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, &line, 2.0, room - 8.0, pal().ink2);
            if retro_btn(ui, if self.tune_more { "HIDE DETAILS" } else { "DETAILS" }, self.tune_more)
                .tip("Edit the composer, year, origin and style.")
                .clicked()
            {
                self.tune_more = !self.tune_more;
            }
        });
        if self.tune_more {
            for (fid, lab, hint) in [
                (crate::tools::F_T_WRITER, "WRITTEN BY", "Composer and lyricist"),
                (crate::tools::F_T_YEAR, "YEAR", "When it was written or first recorded"),
                (crate::tools::F_T_FROM, "FROM", "The show, film or album it comes from"),
                (crate::tools::F_T_STYLE, "STYLE", "Swing, ballad, bossa, waltz"),
            ] {
                let t = &mut self.store.tunes[i];
                let val = match fid {
                    crate::tools::F_T_WRITER => &mut t.written_by,
                    crate::tools::F_T_YEAR => &mut t.year,
                    crate::tools::F_T_FROM => &mut t.from,
                    _ => &mut t.style,
                };
                // a wider label column than the rows above: "WRITTEN BY" needs it
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bh()), Sense::hover());
                let lw = (text_w("WRITTEN BY", 2.0) + 14.0).max(76.0);
                ptext(ui.painter(), Pos2::new(rect.min.x + 4.0, rect.center().y), Align::Min, lab, 2.0, pal().ink2);
                let fr = Rect::from_min_max(Pos2::new(rect.min.x + lw, rect.min.y), rect.max);
                let o = field(ui, &mut self.ed, fid, val, fr, hint, false);
                self.store_dirty |= o.changed;
            }
        }
        ui.add_space(4.0);
        // what to do with it: one row, the rest under "..."
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "PLAY ALL", false).clicked() {
                acts.push(Action::PlayTune(i, false));
            }
            if retro_btn(ui, "SHUFFLE", false).clicked() {
                acts.push(Action::PlayTune(i, true));
            }
            if crate::ibtn(ui, &crate::IC_SEARCH, if self.look_busy { "SEARCHING..." } else { "LOOK UP" }, self.look_busy)
                .tip("Finds the composer and the most popular recordings on Tidal. Searches only when you press it.")
                .clicked()
            {
                acts.push(Action::LookUp(i));
            }
            if retro_btn(ui, "ADD CURRENT", false).tip("Add the recording that is playing now.").clicked() {
                acts.push(Action::AddCurrentToTune(i));
            }
            let more = retro_btn_w(ui, "...", 34.0, false).tip("More options: LOOK UP style, play-alongs, remove tune.");
            let pop = ui.id().with("tune_more_menu");
            if more.clicked() {
                ui.memory_mut(|m| m.toggle_popup(pop));
            }
            egui::popup_below_widget(ui, pop, &more, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(240.0);
                dim_line(ui, "LOOK UP FAVORS", 1.0, pal().dim);
                for (k, name) in ["Auto (jazz for a known standard)", "Jazz", "Gospel", "Any"].iter().enumerate() {
                    let mark = if self.look_style as usize % 4 == k { "> " } else { "   " };
                    if menu_item(ui, &format!("{}{}", mark, name)) {
                        self.look_style = k as u8;
                    }
                }
                ui.separator();
                if menu_item(ui, "   Play-alongs on YouTube...") {
                    acts.push(Action::PlayAlong(tname.clone()));
                }
                if menu_item(ui, "   Remove This Tune...")
                    && !self.del_arm {
                        acts.push(Action::RemoveTune(i));
                    }
            });
        });
        // asked before it goes
        if self.del_arm {
            ui.horizontal_wrapped(|ui| {
                label(ui, &format!("Remove {}, with its recordings?", tname), 0.0);
                if retro_btn(ui, "REMOVE", true).clicked() {
                    acts.push(Action::RemoveTune(i));
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.del_arm = false;
                }
            });
        }
        // a search of your own, once LOOK UP has been used
        if self.look_busy || self.look_for == tname || !self.look_q.is_empty() {
            ui.horizontal_wrapped(|ui| {
                let w = (ui.available_width() - 4.0).max(120.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                let o = field(
                    ui,
                    &mut self.ed,
                    F_LOOK,
                    &mut self.look_q,
                    rect,
                    "Look up a title, or title and artist (empty: this tune)",
                    false,
                );
                if o.enter {
                    acts.push(Action::LookUp(i));
                }
            });
        }

        // ---- the recordings
        section_header(ui, &format!("RECORDINGS TO STUDY ({})", nvers));
        if nvers == 0 {
            para(
                ui,
                "Nothing here yet. Play a recording and press ADD CURRENT, or right-click any song and choose ADD TO A TUNE.",
                pal().ink2,
            );
        }
        let vtracks: Vec<Track> = self.store.tunes[i].versions.iter().map(|v| v.to_track()).collect();
        let playing = self.cur_track().map(|t| t.id);
        if nvers > 0 {
            table_header(ui, "TRACK", "LENGTH", "RATING", true);
        }
        for vi in 0..nvers {
            let (artist, title, kind, stars, note, id) = {
                let v = &self.store.tunes[i].versions[vi];
                (v.artist.clone(), v.title.clone(), v.kind.clone(), v.stars, v.note.clone(), v.id)
            };
            let state = if playing == Some(id) { RowState::Playing } else { RowState::Normal };
            let tag = match kind.as_str() {
                "yt" if src_is_link(&self.store.tunes[i].versions[vi].src) => "[SC] ",
                "yt" => "[YT] ",
                "file" => "[FILE] ",
                _ => "",
            };
            crate::row_stems(id);
            let r = list_row(
                ui,
                vi,
                Some(vi + 1),
                || format!("{}{} - {}", tag, artist, title),
                || format!("{}\t{}", fmt_time(vtracks[vi].duration), stars_text(stars)),
                state,
                false,
                cache::has(id),
            );
            if let Some(r) = r {
                if r.clicked() {
                    acts.push(Action::Play(vtracks.clone(), vi));
                }
                let liked = self.liked.contains(&vtracks[vi].id);
                r.context_menu(|ui| {
                    if menu_item(ui, "Rate / Add a Note") {
                        self.edit_ver = Some(vi);
                        ui.close_menu();
                    }
                    let tidal = self.ext_of(vtracks[vi].id).is_none();
                    crate::song_items(ui, acts, &vtracks[vi], liked, tidal);
                    crate::link_item(ui, acts, crate::track_link(&vtracks[vi], self.ext_of(vtracks[vi].id).as_ref()));
                    if menu_item(ui, "Remove from This Tune") {
                        acts.push(Action::RemoveVersion(i, vi));
                        ui.close_menu();
                    }
                });
            }
            if self.edit_ver == Some(vi) {
                ui.horizontal_wrapped(|ui| {
                    label(ui, "RATING", 74.0);
                    for n in 1..=5u8 {
                        if retro_btn_w(ui, &n.to_string(), 34.0, stars == n).clicked() {
                            self.store.tunes[i].versions[vi].stars = if stars == n { 0 } else { n };
                            self.store_dirty = true;
                        }
                    }
                    if retro_btn(ui, "DONE", false).clicked() {
                        self.edit_ver = None;
                        self.ed.id = 0;
                    }
                });
                let o = field_row(
                    ui,
                    &mut self.ed,
                    F_V_NOTE,
                    "NOTE",
                    &mut self.store.tunes[i].versions[vi].note,
                    "Why it's worth studying",
                    None,
                );
                self.store_dirty |= o.changed;
                ui.add_space(4.0);
            } else if !note.is_empty() {
                dim_line(ui, &format!("> {}", note), 2.0, pal().ink2);
            }
        }

        // ---- other versions (only after LOOK UP)
        if self.look_busy {
            dim_line(ui, "SEARCHING... (COMPOSER FIRST, THEN RECORDINGS. THIS TAKES A FEW SECONDS)", 2.0, pal().ink2);
        } else if self.look_for == tname && self.look_tracks.is_empty() {
            // looked up, and nothing came back worth studying
            let msg = if crate::sources::ytdlp_path().is_none() && !self.api.has_token() {
                "NO RECORDINGS FOUND. WITHOUT TIDAL, BACKLINE SEARCHES YOUTUBE: GET YT-DLP UNDER YOUTUBE (IT'S FREE)"
            } else {
                "NO RECORDINGS FOUND. TRY ITS FULL TITLE, OR TITLE AND ARTIST, IN THE BOX ABOVE"
            };
            dim_line(ui, msg, 1.0, pal().ink2);
        }
        if self.look_for == tname && !self.look_tracks.is_empty() {
            section_header(ui, "RECORDINGS TO STUDY");
            dim_line(
                ui,
                "SORTED BY TIDAL POPULARITY AND ARTISTS YOU RATE HIGHLY, WITHOUT KARAOKE OR COMPILATIONS. CLICK TO PLAY, RIGHT-CLICK TO ADD.",
                1.0,
                pal().dim,
            );
            let pops: Vec<u32> = self.look_tracks.iter().map(|x| x.1).collect();
            let found: Vec<Track> = self.look_tracks.iter().map(|x| x.0.clone()).collect();
            table_header(ui, "TRACK", "POPULARITY", "LENGTH", true);
            for (k, t) in found.iter().enumerate() {
                let state = if playing == Some(t.id) { RowState::Playing } else { RowState::Normal };
                crate::row_stems(t.id);
                let r = list_row(
                    ui,
                    k,
                    Some(k + 1),
                    || format!("{} - {}", t.artists_text(), t.title),
                    || format!("{}\t{}", pops[k], fmt_time(t.duration)),
                    state,
                    false,
                    cache::has(t.id),
                );
                if let Some(r) = r {
                    let r = r.tip(format!("From the album {}.", t.album));
                    if r.clicked() {
                        acts.push(Action::Play(found.clone(), k));
                    }
                    let liked = self.liked.contains(&t.id);
                    r.context_menu(|ui| {
                        if menu_item(ui, "Add to This Tune") {
                            acts.push(Action::AddToTune(i, t.clone()));
                            ui.close_menu();
                        }
                        crate::song_items(ui, acts, t, liked, true);
                    });
                }
            }
        }
        ui.add_space(20.0);
    }

    /// The names of your tunes (in the Tunes list's order), for linking them where notes name them.
    pub(crate) fn tune_names(&self) -> Vec<String> {
        self.store.tunes.iter().map(|t| t.name.clone()).collect()
    }

    /// A tune's page, opened from notes that name it.
    pub(crate) fn open_tune(&mut self, i: usize, acts: &mut Vec<Action>) {
        if i < self.store.tunes.len() {
            if self.sec != Sec::Tunes {
                acts.push(Action::Section(Sec::Tunes));
            }
            self.tune_open = Some(i);
            self.pick = None;
        }
    }
}
