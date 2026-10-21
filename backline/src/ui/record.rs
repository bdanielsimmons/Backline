//! RECORD (a tab of the practice panel): record yourself through the interface or the mic while the band, a loop
//! or a song plays, with the sound on a big live waveform. Every take is kept with the tune (you over the band, and
//! you alone), and plays back with all the practice tools.

use super::*;
use crate::record;

impl App {
    pub(crate) fn record_tab(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let on = self.rec_buf.is_some();
        if !on && self.rec_devs.is_empty() {
            self.rec_devs = record::input_devices();
        }
        para(ui, "Record yourself over the band, a loop or a song. Each take is kept with its tune. Listen back at the end of the week, the way you would study a record. Monitor yourself through your interface, not through Backline.", pal().ink2);
        // ---- the input, the tune it goes with, the band
        ui.horizontal_wrapped(|ui| {
            if self.rec_devs.is_empty() {
                crate::views::label(ui, "NO INPUT FOUND", 0.0);
            } else {
                let names: Vec<&str> = self.rec_devs.iter().map(String::as_str).collect();
                let sel = self.rec_dev.as_ref().and_then(|d| names.iter().position(|n| n == d)).unwrap_or(0);
                if let Some(i) = dropdown(ui, "rec_dev", "INPUT", &names, sel, 0.0) {
                    self.rec_dev = Some(self.rec_devs[i].clone());
                }
            }
            let mut tunes: Vec<String> = vec!["FREE PLAY".to_string()];
            tunes.extend(self.store.tunes.iter().map(|t| t.name.to_uppercase()));
            let tunes: Vec<&str> = tunes.iter().map(String::as_str).collect();
            let sel = (self.rec_tune as usize).min(tunes.len() - 1);
            if let Some(i) = dropdown(ui, "rec_tune", "TUNE", &tunes, sel, 0.0) {
                self.rec_tune = i as u32;
            }
            if retro_btn(ui, "WITH THE BAND", self.rec_band).tip("The band plays the tune's chart when you press REC.").clicked()
            {
                self.rec_band = !self.rec_band;
            }
        });
        // ---- REC, the time, the level
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            // REC: a dot and the word, red and blinking while the take runs
            let (r, resp) = ui.allocate_exact_size(Vec2::new(86.0 * crate::font::ui_scale(), bh()), Sense::click());
            let p = ui.painter();
            crate::raised_h(p, r, resp.is_pointer_button_down_on() || on, resp.hovered());
            let red = Color32::from_rgb(230, 80, 70);
            let blink = !on || (ui.input(|i| i.time) * 2.0) as i64 % 2 == 0;
            let dot = Pos2::new(r.min.x + 14.0, r.center().y);
            if blink {
                p.circle_filled(dot, 5.0, if on { red } else { pal().ink });
            } else {
                p.circle_stroke(dot, 5.0, Stroke::new(1.5_f32, red));
            }
            ptext(
                p,
                Pos2::new(r.min.x + 26.0, r.center().y),
                Align::Min,
                if on { "STOP" } else { "REC" },
                2.0,
                if on { red } else { pal().ink },
            );
            if resp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .tip(if on { "Stop and keep the take." } else { "Start a take." })
                .clicked()
            {
                acts.push(Action::RecToggle);
            }
            if on {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
            }
            let secs = self.rec_t0.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0);
            crate::lcd_box(ui, &fmt_time(secs), 70.0, if on { Color32::from_rgb(230, 80, 70) } else { pal().ink });
            // the level: how loud the last moment was, falling back down between peaks
            let peak = self
                .rec_buf
                .as_ref()
                .and_then(|b| {
                    b.lock().ok().map(|mut b| {
                        let pk = b.peak;
                        b.peak = pk * 0.85;
                        pk
                    })
                })
                .unwrap_or(0.0);
            let (r, _) = ui.allocate_exact_size(Vec2::new(160.0, bh()), Sense::hover());
            let bar = r.shrink2(Vec2::new(0.0, 6.0));
            ui.painter().rect_filled(bar, 2.0, pal().lcd);
            let w = bar.width() * peak.clamp(0.0, 1.0);
            let col = if peak > 0.9 {
                Color32::from_rgb(230, 80, 70)
            } else if peak > 0.6 {
                Color32::from_rgb(217, 164, 65)
            } else {
                Color32::from_rgb(111, 191, 115)
            };
            ui.painter().rect_filled(Rect::from_min_size(bar.min, Vec2::new(w, bar.height())), 2.0, col);
            crate::views::label(ui, "LEVEL", 0.0);
            if self.rec_ask {
                ui.add_space(10.0);
                crate::views::label(ui, "KEEP THE LAST TAKE?", 0.0);
                if retro_btn(ui, "KEEP", false).clicked() {
                    acts.push(Action::RecLast(true));
                }
                if retro_btn(ui, "DELETE", false).clicked() {
                    acts.push(Action::RecLast(false));
                }
            }
        });
        if let Some(e) = self.rec_buf.as_ref().and_then(|b| b.lock().ok().map(|b| b.err.clone())).filter(|e| !e.is_empty()) {
            crate::views::dim_line(ui, &e, 1.0, Color32::from_rgb(230, 80, 70));
        }
        // ---- the sound coming in: the last eight seconds, scrolling
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 150.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(r, 2.0, pal().lcd);
        let mid = r.center().y;
        p.line_segment([Pos2::new(r.min.x, mid), Pos2::new(r.max.x, mid)], Stroke::new(1.0_f32, pal().dim.gamma_multiply(0.5)));
        if let Some(b) = self.rec_buf.as_ref().and_then(|b| b.lock().ok()) {
            let span = (b.rate as usize * 8).max(1);
            let n = b.samples.len();
            let start = n.saturating_sub(span);
            let cols = r.width().max(1.0) as usize;
            let per = span / cols.max(1);
            for c in 0..cols {
                let a = start + c * per;
                let z = (a + per).min(n);
                if a >= z {
                    continue;
                }
                let (mut lo, mut hi) = (0i16, 0i16);
                for s in &b.samples[a..z] {
                    lo = lo.min(*s);
                    hi = hi.max(*s);
                }
                let x = r.min.x + c as f32;
                let h = r.height() / 2.0 - 4.0;
                let y0 = mid - hi as f32 / 32768.0 * h;
                let y1 = mid - lo as f32 / 32768.0 * h;
                p.line_segment([Pos2::new(x, y0), Pos2::new(x, y1.max(y0 + 1.0))], Stroke::new(1.0_f32, pal().bar_txt));
            }
            ui.ctx().request_repaint();
        } else {
            ptext(p, r.center(), Align::Center, "PRESS REC AND PLAY", 2.0, pal().dim);
        }
        // ---- the last few takes; ALL TAKES for the rest
        if !self.store.takes.is_empty() {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                mini_header(ui, "TAKES");
            });
            let playing = self.cur_track().map(|t| t.id);
            let last: Vec<crate::store::Take> = self.store.takes.iter().rev().take(5).cloned().collect();
            for t in &last {
                ui.horizontal_wrapped(|ui| {
                    let tune = if t.tune.is_empty() { "FREE PLAY".to_string() } else { t.tune.to_uppercase() };
                    let label = format!(
                        "{}   {}   {}{}",
                        t.stamp,
                        tune,
                        fmt_time(t.secs),
                        if t.tag.is_empty() { String::new() } else { format!("   {}", t.tag.to_uppercase()) }
                    );
                    if retro_btn(ui, &label, playing == Some(t.id)).tip("Play it back, with the loop and speed tools.").clicked()
                    {
                        acts.push(Action::RecPlay(t.id, false));
                    }
                });
            }
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "ALL TAKES", self.takes_open).tip("Play, tag or delete every take.").clicked() {
                    acts.push(Action::TakesPanel(!self.takes_open));
                }
            });
        }
    }

    /// ALL TAKES: every take by tune, newest first - play the mix or you alone, give it a tag, delete it.
    pub(crate) fn takes_panel(&mut self, ctx: &egui::Context) {
        if !self.takes_open {
            return;
        }
        let mut acts: Vec<Action> = Vec::new();
        let close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let playing = self.cur_track().map(|t| t.id);
        crate::credits::panel(ctx, "takes", 640.0, |ui| {
            title_line(ui, "YOUR TAKES", 3.0, pal().ink);
            para(ui, "Your takes by tune, newest first. Click a take to hear yourself over what played. Click YOU ALONE to hear only you. Type any tag you like, such as solo, comping or keeper.", pal().ink2);
            // the order: by tune, newest first inside each
            let mut order: Vec<usize> = (0..self.store.takes.len()).collect();
            order.sort_by(|a, b| {
                let (x, y) = (&self.store.takes[*a], &self.store.takes[*b]);
                x.tune.to_lowercase().cmp(&y.tune.to_lowercase()).then(y.stamp.cmp(&x.stamp))
            });
            let mut last_tune: Option<String> = None;
            egui::ScrollArea::vertical().max_height(ctx.screen_rect().height() * 0.6).show(ui, |ui| {
                for i in order {
                    let (id, tune, stamp, secs, dry_ok) = {
                        let t = &self.store.takes[i];
                        (t.id, t.tune.clone(), t.stamp.clone(), t.secs, !t.dry.is_empty())
                    };
                    if last_tune.as_ref() != Some(&tune) {
                        section_header(ui, if tune.is_empty() { "FREE PLAY" } else { &tune });
                        last_tune = Some(tune.clone());
                    }
                    ui.horizontal_wrapped(|ui| {
                        if retro_btn(ui, &format!("{}  {}", stamp, fmt_time(secs)), playing == Some(id))
                            .tip("Play it back.")
                            .clicked()
                        {
                            acts.push(Action::RecPlay(id, false));
                        }
                        if dry_ok && retro_btn(ui, "YOU ALONE", false).tip("Just you, without the band.").clicked() {
                            acts.push(Action::RecPlay(id, true));
                        }
                        let (r, _) = ui.allocate_exact_size(Vec2::new(160.0, bh()), Sense::hover());
                        let tag = &mut self.store.takes[i].tag;
                        if crate::views::field(ui, &mut self.ed, crate::tools::F_TAKE_TAG + i as u32, tag, r, "tag", false)
                            .changed
                        {
                            self.store_dirty = true;
                        }
                        if self.take_ask == Some(id) {
                            crate::views::label(ui, "DELETE IT?", 0.0);
                            if retro_btn(ui, "YES", false).clicked() {
                                acts.push(Action::TakeDelete(id));
                            }
                            if retro_btn(ui, "NO", false).clicked() {
                                self.take_ask = None;
                            }
                        } else if retro_btn(ui, "DELETE", false).clicked() {
                            self.take_ask = Some(id);
                        }
                    });
                }
            });
            ui.add_space(6.0);
            if retro_btn(ui, "CLOSE", false).clicked() {
                acts.push(Action::TakesPanel(false));
            }
        });
        if close {
            acts.push(Action::TakesPanel(false));
        }
        for a in acts {
            self.apply(a);
        }
    }
}
