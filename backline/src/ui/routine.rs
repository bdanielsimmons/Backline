//! DIARY > ROUTINE and LISTENING. ROUTINE: your set (one tune of each kind you are working on - a blues,
//! rhythm changes, a ballad...) and suggestions for a session with how to do each in Backline. The lists live in
//! data/canon.rs.

use super::*;
use crate::canon;

/// What to practise, and how to do it here: (what, how in Backline). A checkbox each, for today.
pub(crate) const SUGGESTIONS: [(&str, &str); 7] = [
    ("LISTEN BEFORE YOU PLAY", "Play a recording of a tune from your set, twice, guitar down. Click the tune, then a recording in TUNES. Loop the head with A and B in the PRACTICE panel."),
    ("LEARN THE MELODY BY EAR", "Slow the loop down until you can play along without the sheet. Use SPEED in the PRACTICE panel, and the pitch stays the same. Then move it up a 4th in PITCH and find it again."),
    ("PLAY THE CHANGES WITH THE BAND", "Comp shells with the band, then run arpeggios through the tune. Click a tune in your set to put its chart on the right. Press PLAY BAND on the lead sheet. For arpeggios, go to LINES > ARPEGGIOS."),
    ("ONE LICK THROUGH 12 KEYS", "Loop 4 to 8 bars you love from a record and take them through every key. In TRAINER, set THEN to UP A 4TH to move the loop every few passes. Play it back in each key by ear."),
    ("RECORD A SOLO", "Record three choruses with the band. Go to PRACTICE > RECORD and turn on WITH THE BAND. Don't listen back today."),
    ("LISTEN BACK (ONCE A WEEK)", "Hear the week's takes with ALL TAKES in the RECORD tab. Write down one thing to fix with LOG PRACTICE in the DIARY."),
    ("LISTEN TO A RECORD", "Listen closely to one album, start to finish. Pick it in DIARY > LISTENING."),
];

impl App {
    /// ROUTINE: your set (one tune of each kind you are working on), then suggestions for a session.
    pub(crate) fn routine_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        // ---- your set: one tune of each kind
        section_header(ui, "YOUR SET");
        para(ui, "One tune of each kind keeps you ready for any session. That means a blues, rhythm changes, a ballad and so on. Pick one in each row, and click its name to see the chart on the right.", pal().ink2);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "JAZZ", !self.store.routine_gospel).clicked() {
                self.store.routine_gospel = false;
                self.store_dirty = true;
            }
            if retro_btn(ui, "GOSPEL", self.store.routine_gospel).clicked() {
                self.store.routine_gospel = true;
                self.store_dirty = true;
            }
        });
        let kinds = if self.store.routine_gospel { canon::GOSPEL_TUNES } else { canon::SESSION_TUNES };
        let showing =
            self.prac_chart.as_ref().map(|c| c.0.clone()).or_else(|| self.chart_tune().map(|i| self.store.tunes[i].name.clone()));
        let mut covered = 0;
        // the kinds' names in one column, as wide as the longest, so the choices line up
        let kind_w = kinds.iter().map(|(k, _)| text_w(&crate::winamp_ui::caps_words(k), 2.0)).fold(0.0, f32::max) + 16.0;
        for (kind, list) in kinds.iter() {
            let picked = self.store.routine_set.get(*kind).cloned().filter(|s| !s.is_empty());
            if picked.is_some() {
                covered += 1;
            }
            ui.horizontal_wrapped(|ui| {
                crate::views::label(ui, kind, kind_w / crate::font::ui_scale());
                if let Some(name) = &picked {
                    let on = showing.as_deref().is_some_and(|s| s.eq_ignore_ascii_case(name));
                    if retro_btn(ui, &crate::font::as_label(name), on).tip("Shows its chart on the right.").clicked() {
                        acts.push(Action::PreviewTune(name.clone()));
                    }
                }
                // the kind's tunes; the ones already in your tunes marked
                let mut items: Vec<String> = vec![if picked.is_some() { "Change".to_string() } else { "Pick one".to_string() }];
                items.extend(
                    list.iter().map(|(n, _)| if self.find_tune(n).is_some() { format!("{}  *", n) } else { n.to_string() }),
                );
                if picked.is_some() {
                    items.push("NONE".to_string());
                }
                let items: Vec<&str> = items.iter().map(String::as_str).collect();
                if let Some(i) = dropdown(ui, &format!("set_{}", kind), "", &items, 0, 0.0) {
                    if i > 0 {
                        let name = if i <= list.len() { list[i - 1].0.to_string() } else { String::new() };
                        acts.push(Action::SetTune(kind.to_string(), name));
                    }
                }
            });
        }
        crate::views::dim_line(
            ui,
            &format!("{} OF {} KINDS IN YOUR SET   * = ALREADY IN YOUR TUNES", covered, kinds.len()),
            1.0,
            pal().ink,
        );

        // ---- suggestions for a session, with how to do each here
        ui.add_space(8.0);
        section_header(ui, "A SESSION");
        para(
            ui,
            "Ideas for today, from how the masters practiced. Do what you have time for, and tick what you did.",
            pal().ink2,
        );
        let date = store::today_str();
        let done: Vec<u8> = self.store.routine_done.get(&date).cloned().unwrap_or_default();
        for (k, (what, how)) in SUGGESTIONS.iter().enumerate() {
            let ticked = done.contains(&(k as u8));
            ui.horizontal(|ui| {
                let w = ui.available_width() - 34.0;
                ui.vertical(|ui| {
                    ui.set_width(w);
                    // (a step's name as a title, not shouted)
                    let what = if crate::winamp_ui::chrome_on() {
                        crate::winamp_ui::as_title(|| crate::winamp_ui::display_case(what).into_owned())
                    } else {
                        what.to_string()
                    };
                    title_line(ui, &what, 2.0, if ticked { pal().ink2 } else { pal().ink });
                    para(ui, how, pal().ink2);
                });
                // the checkbox, on the right
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
                let p = ui.painter();
                p.rect_stroke(r.shrink(2.0), 2.0, Stroke::new(1.5_f32, if resp.hovered() { pal().ink } else { pal().ink2 }));
                if ticked {
                    p.line_segment(
                        [Pos2::new(r.min.x + 5.0, r.center().y), Pos2::new(r.center().x - 1.0, r.max.y - 5.0)],
                        Stroke::new(2.0_f32, pal().ink),
                    );
                    p.line_segment(
                        [Pos2::new(r.center().x - 1.0, r.max.y - 5.0), Pos2::new(r.max.x - 4.0, r.min.y + 4.0)],
                        Stroke::new(2.0_f32, pal().ink),
                    );
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Did it today.").clicked() {
                    acts.push(Action::RoutineDone(k as u8));
                }
            });
        }
        ui.add_space(20.0);
    }

    /// LISTENING: records by kind, jazz or gospel; a click finds the album on Tidal; LISTENED counts it.
    pub(crate) fn listening_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Close listening was half of how the masters practiced. They played a few records over and over. Click one to play it, and each play is counted.", pal().ink2);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "JAZZ", !self.listen_gospel).clicked() {
                self.listen_gospel = false;
            }
            if retro_btn(ui, "GOSPEL", self.listen_gospel).clicked() {
                self.listen_gospel = true;
            }
            if !self.listen_gospel {
                ui.add_space(10.0);
                if retro_btn(ui, "GUITARISTS ONLY", self.listen_guitar).clicked() {
                    self.listen_guitar = !self.listen_guitar;
                }
            }
        });
        let kinds = if self.listen_gospel { canon::LISTEN_GOSPEL } else { canon::LISTEN_JAZZ };
        let guitar = self.listen_guitar && !self.listen_gospel;
        let playing_album = self.cur_track().map(|t| t.album.to_lowercase());
        for (era, list) in kinds.iter() {
            let shown: Vec<&(&str, &str)> =
                list.iter().filter(|(a, _)| !guitar || canon::GUITARISTS.iter().any(|g| a.contains(g))).collect();
            if shown.is_empty() {
                continue;
            }
            section_header(ui, era);
            for (artist, album) in shown {
                let key = format!("{} - {}", artist, album);
                let n = self.store.listened.get(&key).copied().unwrap_or(0);
                // a row like a song's: click it to play the record
                let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::click());
                let on = playing_album.as_deref().is_some_and(|a| a.starts_with(&album.to_lowercase()));
                let p = ui.painter();
                if resp.hovered() || on {
                    p.rect_filled(r, 0.0, pal().sel);
                }
                let y = r.center().y;
                let w = r.width();
                // names as written with the skinned look (all caps would come back as "Ella fitzgerald"), else in capitals
                let cap = |t: &str| if crate::winamp_ui::chrome_on() { t.to_string() } else { t.to_uppercase() };
                crate::font::ptext_fit(
                    p,
                    Pos2::new(r.min.x + 8.0, y),
                    Align::Min,
                    &cap(artist),
                    2.0,
                    w * 0.38,
                    pal().ink2,
                );
                crate::font::ptext_fit(
                    p,
                    Pos2::new(r.min.x + 8.0 + w * 0.4, y),
                    Align::Min,
                    &cap(album),
                    2.0,
                    w * 0.48,
                    pal().ink,
                );
                if n > 0 {
                    ptext(p, Pos2::new(r.max.x - 8.0, y), Align::Max, &format!("{}x", n), 1.0, pal().dim);
                }
                if resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .tip("Play it from Tidal. Each play counts as a listen.")
                    .clicked()
                {
                    acts.push(Action::PlayRecord(artist.to_string(), album.to_string()));
                }
            }
        }
        ui.add_space(20.0);
    }
}
