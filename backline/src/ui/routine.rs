//! DIARY > ROUTINE, LISTENING and MASTERS: the spine of a practice week. One tune at a time, listened to, learned
//! by ear, played with the band, a lick through every key, a solo recorded - each step a button that opens the
//! right tool with the right settings. The lists live in data/canon.rs.

use super::*;
use crate::canon;

/// The steps of a day: (name, what to do, minutes, a key for the day's done-list).
pub(crate) const STEPS: [(&str, &str, u32); 6] = [
    ("LISTEN TO THE RECORD", "Loop the head on the reference recording. Twice through, nothing in your hands.", 5),
    ("MELODY BY EAR", "Play the melody from the record, no sheet. Then in two other keys.", 10),
    (
        "CHANGES WITH THE BAND",
        "Comp the changes with the band, then arpeggios through them. Shells on the 6th and 5th strings.",
        10,
    ),
    (
        "ONE LICK THROUGH 12 KEYS",
        "Loop 4 to 8 bars you love from the record. The trainer moves it up a 4th every few passes; play along by ear.",
        10,
    ),
    ("SOLO, RECORDED", "Three choruses over the band, recorded. Don't listen back today.", 10),
    ("LISTEN BACK", "Sunday: hear this week's takes. Write one thing to fix in the diary.", 10),
];

impl App {
    /// The tune of the week's index in TUNES, if it is there.
    fn routine_tune_i(&self) -> Option<usize> {
        let name = self.store.routine_tune.trim();
        if name.is_empty() {
            return None;
        }
        self.find_tune(name)
    }

    /// The next tune from the session list (jazz or gospel): kinds in turn, so the weeks vary; one you have not
    /// marked READY and have not skipped.
    pub(crate) fn routine_next_name(&self) -> Option<String> {
        let kinds = if self.store.routine_gospel { canon::GOSPEL_TUNES } else { canon::SESSION_TUNES };
        let ready = |n: &str| self.find_tune(n).is_some_and(|i| self.store.tunes[i].status >= 2);
        let skipped = |n: &str| self.store.skipped_tunes.iter().any(|s| s.eq_ignore_ascii_case(n));
        let start = self.store.routine_count as usize;
        for k in 0..kinds.len() {
            let (_, list) = kinds[(start + k) % kinds.len()];
            if let Some((name, _)) =
                list.iter().find(|(n, _)| !ready(n) && !skipped(n) && !n.eq_ignore_ascii_case(&self.store.routine_tune))
            {
                return Some(name.to_string());
            }
        }
        None
    }

    pub(crate) fn routine_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "One tune a week, the way the masters learned: listen hard, learn it by ear, play it with the band, take one lick through every key, record a solo. Each step opens the tool set up for it. Change the minutes, turn steps off, make it yours.", pal().ink2);
        // ---- the tune of the week
        let today = store::today_days();
        let tune_i = self.routine_tune_i();
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, "TUNE OF THE WEEK", 150.0);
            if self.store.routine_tune.is_empty() {
                crate::views::label(ui, "NONE YET - PRESS NEXT", 0.0);
            } else {
                let days = store::parse_date(&self.store.routine_since).map(|d| today - d + 1).unwrap_or(1);
                title_line(ui, &format!("{}   (DAY {})", self.store.routine_tune.to_uppercase(), days.max(1)), 2.5, pal().ink);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "NEXT", false).tip("The next session tune, a different kind each week").clicked() {
                if let Some(n) = self.routine_next_name() {
                    acts.push(Action::RoutineTune(n));
                }
            }
            if !self.store.routine_tune.is_empty()
                && retro_btn(ui, "SKIP IT", false).tip("Not this one: it won't come up again").clicked()
            {
                let n = self.store.routine_tune.clone();
                self.store.skipped_tunes.push(n);
                if let Some(next) = self.routine_next_name() {
                    acts.push(Action::RoutineTune(next));
                }
            }
            let kinds = if self.store.routine_gospel { canon::GOSPEL_TUNES } else { canon::SESSION_TUNES };
            let labels: Vec<String> =
                kinds.iter().flat_map(|(k, l)| l.iter().map(move |(n, _)| format!("{}: {}", k, n))).collect();
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            let mut pick = vec!["PICK ONE..."];
            pick.extend(labels.iter().copied());
            if let Some(i) = dropdown(ui, "rt_pick", "", &pick, 0, 160.0) {
                if i > 0 {
                    let n = labels[i - 1].split_once(": ").map(|x| x.1).unwrap_or(labels[i - 1]).to_string();
                    acts.push(Action::RoutineTune(n));
                }
            }
            let mut mine = vec!["FROM MY TUNES..."];
            let names: Vec<String> = self.store.tunes.iter().map(|t| t.name.clone()).collect();
            mine.extend(names.iter().map(String::as_str));
            if let Some(i) = dropdown(ui, "rt_mine", "", &mine, 0, 160.0) {
                if i > 0 {
                    acts.push(Action::RoutineTune(names[i - 1].clone()));
                }
            }
            if retro_btn(ui, if self.store.routine_gospel { "GOSPEL" } else { "JAZZ" }, false)
                .tip("Which list NEXT draws from")
                .clicked()
            {
                self.store.routine_gospel = !self.store.routine_gospel;
                self.store_dirty = true;
            }
        });
        if let Some(i) = tune_i {
            if self.store.tunes[i].chart.trim().is_empty() {
                crate::views::dim_line(ui, "NO CHART YET - OPEN THE TUNE AND FIND ONE, OR TYPE IT", 1.0, pal().dim);
            }
        }

        // ---- today's steps
        ui.add_space(6.0);
        let date = store::date_str(today);
        let sunday = store::weekday(today) == "SUN";
        let done: Vec<u8> = self.store.routine_done.get(&date).cloned().unwrap_or_default();
        if self.store.routine_mins.len() != STEPS.len() {
            self.store.routine_mins = STEPS.iter().map(|s| s.2).collect();
        }
        let mut n_done = 0;
        let mut n_on = 0;
        let mut total = 0;
        for (k, (name, about, _)) in STEPS.iter().enumerate() {
            let off = self.store.routine_off.contains(&(k as u8));
            if k == 5 && !sunday && !off {
                // listening back is a Sunday thing
                continue;
            }
            let is_done = done.contains(&(k as u8));
            if !off {
                n_on += 1;
                total += self.store.routine_mins[k];
                if is_done {
                    n_done += 1;
                }
            }
            // one row: done, the step (a button that opens its tool), its minutes (or OFF) in one small menu
            ui.horizontal_wrapped(|ui| {
                if retro_btn_w(ui, if is_done { "DONE" } else { "  " }, 44.0, is_done).tip("Mark it done for today").clicked() {
                    acts.push(Action::RoutineDone(k as u8));
                }
                if off {
                    crate::views::label(ui, name, 0.0);
                } else if retro_btn(ui, name, false).tip(about).clicked() {
                    acts.push(Action::RoutineGo(k as u8));
                }
                const MINS: [u32; 7] = [0, 5, 10, 15, 20, 30, 45];
                let items: Vec<String> =
                    MINS.iter().map(|m| if *m == 0 { "OFF".to_string() } else { format!("{} MIN", m) }).collect();
                let items: Vec<&str> = items.iter().map(String::as_str).collect();
                let cur = if off { 0 } else { MINS.iter().position(|m| *m == self.store.routine_mins[k]).unwrap_or(2) };
                if let Some(i) = dropdown(ui, &format!("rt_min{}", k), "", &items, cur, 0.0) {
                    if i == 0 {
                        if !off {
                            self.store.routine_off.push(k as u8);
                        }
                    } else {
                        self.store.routine_off.retain(|x| *x != k as u8);
                        self.store.routine_mins[k] = MINS[i];
                    }
                    self.store_dirty = true;
                }
            });
            if !off {
                crate::views::dim_line(ui, &about.to_uppercase(), 1.0, pal().ink2);
            }
        }
        ui.add_space(4.0);
        crate::views::dim_line(ui, &format!("{} OF {} DONE TODAY   -   ABOUT {} MINUTES", n_done, n_on, total), 1.0, pal().ink);
        crate::views::dim_line(
            ui,
            "THE FOCUS TIMER (TOMATO, TOP RIGHT OF PRACTICE) KEEPS EACH STEP TO ITS MINUTES.",
            1.0,
            pal().dim,
        );
    }

    /// LISTENING: records by kind, jazz or gospel; a click finds the album on Tidal; LISTENED counts it.
    pub(crate) fn listening_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Close listening was half of how the masters practised: a few records, over and over. Click one to find it on Tidal; LISTENED keeps count.", pal().ink2);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "JAZZ", !self.listen_gospel).clicked() {
                self.listen_gospel = false;
            }
            if retro_btn(ui, "GOSPEL", self.listen_gospel).clicked() {
                self.listen_gospel = true;
            }
        });
        let kinds = if self.listen_gospel { canon::LISTEN_GOSPEL } else { canon::LISTEN_JAZZ };
        for (kind, list) in kinds.iter() {
            section_header(ui, kind);
            for (artist, album) in list.iter() {
                let key = format!("{} - {}", artist, album);
                let n = self.store.listened.get(&key).copied().unwrap_or(0);
                ui.horizontal_wrapped(|ui| {
                    if retro_btn(ui, &key.to_uppercase(), false).tip("Find it on Tidal").clicked() {
                        acts.push(Action::Search(format!("{} {}", artist, album)));
                    }
                    let lab = if n == 0 { "LISTENED".to_string() } else { format!("LISTENED x{}", n) };
                    if retro_btn(ui, &lab, n > 0).tip("One more close listen").clicked() {
                        *self.store.listened.entry(key.clone()).or_insert(0) += 1;
                        self.store_dirty = true;
                    }
                });
            }
        }
        ui.add_space(20.0);
    }

    /// MASTERS: how they worked, and the matching tool here.
    pub(crate) fn masters_view(&mut self, ui: &mut egui::Ui) {
        para(ui, "What is on the record about how they practised - and what in Backline does the same job. The pattern: tunes, close listening, small things taken everywhere, people, and the melody over theory.", pal().ink2);
        for (name, how, tool) in canon::MASTERS.iter() {
            section_header(ui, name);
            para(ui, how, pal().ink);
            crate::views::dim_line(ui, &format!("HERE: {}", tool.to_uppercase()), 1.0, pal().ink2);
        }
        ui.add_space(20.0);
    }
}
