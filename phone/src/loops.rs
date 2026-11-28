//! LOOPS (in My Tracks), the desktop's Loops page as a narrow window has it: every song you've saved loops or
//! bookmarks on, the one practised last first, under Recent and each month before it. Tap a song for its loops and
//! bookmarks (< ALL SONGS goes back); tap a loop to loop it, a bookmark to play from there.

use crate::*;

impl App {
    pub(crate) fn loops_ui(&mut self, ui: &mut egui::Ui) {
        let mut rows: Vec<(Song, Kept)> =
            self.kept.iter().filter(|(_, k)| !k.loops.is_empty() || !k.marks.is_empty()).map(|(vid, k)| (self.song_for(vid, k), k.clone())).collect();
        rows.sort_by(|a, b| b.1.at.cmp(&a.1.at).then_with(|| a.0.title.cmp(&b.0.title)));
        if let Some(open) = self.loops_pick.clone() {
            if let Some((s, k)) = rows.iter().find(|r| r.0.vid == open).cloned() {
                self.loop_song(ui, s, k);
                return;
            }
            self.loops_pick = None;
        }
        if rows.is_empty() {
            crate::mine::hint(ui, "No loops or bookmarks yet. In Practice, set A and B and press SAVE LOOP, or add a bookmark.");
            return;
        }
        let n = rows.len();
        dim_line(ui, &format!("{} {} with loops or bookmarks", n, if n == 1 { "song" } else { "songs" }), 1.0, pal().ink2);
        // (the month each song was last practised in, on the phone's own clock)
        let off = takes::local_offset();
        let day = |secs: u64| if secs == 0 { 0 } else { (secs as i64 + off).div_euclid(86400) };
        let today = day(now_secs());
        let heads: Vec<String> = rows.iter().map(|r| when_heading(day(r.1.at), today)).collect();
        let cur = self.cur.as_ref().map(|c| c.vid.clone());
        let mut open = None;
        egui::ScrollArea::vertical().id_salt("loops_scroll").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 2.0);
            for (i, (s, k)) in rows.iter().enumerate() {
                if i == 0 || heads[i] != heads[i - 1] {
                    if i > 0 {
                        ui.add_space(10.0);
                    }
                    when_head(ui, &heads[i], heads[i..].iter().take_while(|h| **h == heads[i]).count());
                }
                let (who, name) = parts(s);
                let mut sub = vec![];
                if !who.is_empty() {
                    sub.push(who);
                }
                for (n, one, many) in [(k.loops.len(), "loop", "loops"), (k.marks.len(), "bookmark", "bookmarks")] {
                    if n > 0 {
                        sub.push(format!("{} {}", n, if n == 1 { one } else { many }));
                    }
                }
                let pic = self.thumb(s);
                if crate::songs::list_row(ui, pic, "note", &name, &sub.join(" · "), cur.as_deref() == Some(s.vid.as_str())).clicked() {
                    open = Some(s.vid.clone());
                }
            }
            ui.add_space(12.0);
        });
        if open.is_some() {
            self.loops_pick = open;
        }
    }

    /// A song kept with its loops (or, kept before songs were, found in your lists).
    fn song_for(&self, vid: &str, k: &Kept) -> Song {
        if !k.song.vid.is_empty() {
            return k.song.clone();
        }
        self.songs
            .iter()
            .chain(self.liked.iter())
            .chain(self.device_tracks.iter())
            .find(|s| s.vid == vid)
            .cloned()
            .unwrap_or(Song { vid: vid.to_string(), title: "Unknown song".to_string(), ..Default::default() })
    }

    /// One song's loops and bookmarks: the song, its map (loops as bands, bookmarks as ticks; tap it to play from
    /// there), your notes on it, then each loop and bookmark in the song's order.
    fn loop_song(&mut self, ui: &mut egui::Ui, s: Song, k: Kept) {
        let playing = self.cur.as_ref().is_some_and(|c| c.vid == s.vid);
        let (la, lb, on, now) = (self.loop_a, self.loop_b, self.loop_on, self.pos());
        let looping = |a: f32, b: f32| playing && on && la.is_some_and(|x| (x - a).abs() < 0.05) && lb.is_some_and(|x| (x - b).abs() < 0.05);
        let mut back = false;
        let mut go: Option<(f32, f32)> = None;
        ui.horizontal(|ui| {
            if retro_btn_w(ui, "< ALL SONGS", 124.0, false).clicked() {
                back = true;
            }
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if retro_btn_w(ui, "PLAY", 64.0, false).clicked() {
                    go = Some((0.0, -1.0));
                }
            });
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical().id_salt(("loop_song", &s.vid)).auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 2.0);
            // ---- the song
            let (who, name) = parts(&s);
            let len = if s.secs > 1.0 {
                s.secs
            } else {
                let far = k.loops.iter().map(|l| l.b).chain(k.marks.iter().map(|m| m.t)).fold(0.0f32, f32::max);
                far * 1.08 + 5.0
            };
            let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::hover());
            let sub = if who.is_empty() { fmt_time(len) } else { format!("{} · {}", who, fmt_time(len)) };
            font::keep_caps(|| {
                let p = ui.painter();
                ptext(p, Pos2::new(r.min.x + 2.0, r.min.y + 14.0), Align::Min, &fit(&name, 1.8, r.width() - 4.0), 1.8, pal().ink);
                ptext(p, Pos2::new(r.min.x + 2.0, r.min.y + 36.0), Align::Min, &fit(&sub, 1.1, r.width() - 4.0), 1.1, pal().ink2);
            });
            // ---- the map: the song end to end, loops as bands, bookmarks as ticks, where it's playing
            let (all, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 58.0), Sense::click());
            let sr = Rect::from_min_size(all.min + Vec2::new(0.0, 4.0), Vec2::new(all.width(), 36.0));
            let inner = sr.shrink(3.0);
            let at = |t: f32| inner.min.x + (t / len).clamp(0.0, 1.0) * inner.width();
            {
                let p = ui.painter();
                inset(p, sr, pal().display);
                for l in &k.loops {
                    let band = Rect::from_min_max(Pos2::new(at(l.a), inner.min.y), Pos2::new(at(l.b).max(at(l.a) + 3.0), inner.max.y));
                    p.rect_filled(band, 0.0, pal().ink.gamma_multiply(if looping(l.a, l.b) { 0.6 } else { 0.3 }));
                }
                for m in &k.marks {
                    let x = at(m.t);
                    p.line_segment([Pos2::new(x, inner.min.y), Pos2::new(x, inner.max.y)], Stroke::new(2.0_f32, pal().ink));
                }
                if playing {
                    let x = at(now);
                    p.line_segment([Pos2::new(x, sr.min.y - 2.0), Pos2::new(x, sr.max.y + 2.0)], Stroke::new(1.5_f32, pal().ink));
                }
                for (f, al) in [(0.0, Align::Min), (0.5, Align::Center), (1.0, Align::Max)] {
                    ptext(p, Pos2::new(sr.min.x + sr.width() * f, sr.max.y + 9.0), al, &fmt_time(len * f), 1.0, pal().dim);
                }
            }
            if resp.clicked() {
                if let Some(pp) = resp.interact_pointer_pos() {
                    go = Some((((pp.x - inner.min.x) / inner.width()).clamp(0.0, 1.0) * len, -1.0));
                }
            }
            // ---- your notes on the song
            if !k.notes.trim().is_empty() {
                ui.add_space(2.0);
                crate::mine::hint(ui, k.notes.trim());
            }
            ui.add_space(6.0);
            // ---- each loop and bookmark, in the song's order
            enum Spot<'a> {
                L(&'a Loop),
                M(&'a Mark),
            }
            let mut spots: Vec<(f32, Spot)> = k.loops.iter().map(|l| (l.a, Spot::L(l))).collect();
            spots.extend(k.marks.iter().map(|m| (m.t, Spot::M(m))));
            spots.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (i, (_, sp)) in spots.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::click());
                let lit = matches!(sp, Spot::L(l) if looping(l.a, l.b));
                let p = ui.painter();
                if lit {
                    p.rect_filled(r, 0.0, pal().row_sel);
                } else if resp.is_pointer_button_down_on() {
                    p.rect_filled(r, 0.0, pal().row_alt);
                }
                let cy = r.center().y;
                let (name, meta, spot) = match sp {
                    Spot::L(l) => {
                        p.rect_filled(Rect::from_center_size(Pos2::new(r.min.x + 14.0, cy), Vec2::new(16.0, 9.0)), 0.0, pal().ink.gamma_multiply(0.65));
                        (l.name.clone(), format!("{} - {}", fmt_time(l.a), fmt_time(l.b)), (l.a, l.b))
                    }
                    Spot::M(m) => {
                        p.rect_filled(Rect::from_center_size(Pos2::new(r.min.x + 14.0, cy), Vec2::new(3.0, 16.0)), 0.0, pal().ink);
                        (if m.name.is_empty() { "Bookmark".to_string() } else { m.name.clone() }, fmt_time(m.t), (m.t, -1.0))
                    }
                };
                let mw = ptext(p, Pos2::new(r.max.x - 6.0, cy), Align::Max, &meta, 1.1, pal().ink2);
                font::keep_caps(|| {
                    ptext(p, Pos2::new(r.min.x + 32.0, cy), Align::Min, &fit(&name, 1.35, r.width() - mw - 50.0), 1.35, pal().ink);
                });
                if i + 1 < spots.len() {
                    p.line_segment([Pos2::new(r.min.x + 32.0, r.max.y), Pos2::new(r.max.x, r.max.y)], Stroke::new(1.0_f32, pal().row_alt));
                }
                if resp.clicked() {
                    go = Some(spot);
                }
            }
            ui.add_space(12.0);
        });
        if back {
            self.loops_pick = None;
        }
        if let Some((a, b)) = go {
            self.play_spot(s, a, b);
        }
    }
}

/// Loops' headings, as the desktop's: Recent (this month), then each month before it (this year's by name, older
/// ones with their year), then Earlier (songs never dated).
fn when_heading(day: i64, today: i64) -> String {
    const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    if day <= 0 {
        return "Earlier".to_string();
    }
    let (y, m, _) = takes::civil(day);
    let (ty, tm, _) = takes::civil(today);
    if (y, m) >= (ty, tm) {
        return "Recent".to_string();
    }
    let name = MONTHS[(m as usize).clamp(1, 12) - 1];
    if y == ty {
        name.to_string()
    } else {
        format!("{} {}", name, y)
    }
}

/// A heading over Loops' songs: the month (or Recent), and how many songs are under it.
fn when_head(ui: &mut egui::Ui, head: &str, n: usize) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::hover());
    let p = ui.painter();
    spaced_caps(p, Pos2::new(r.min.x + 2.0, r.center().y), &head.to_uppercase(), 1.1, pal().ink);
    ptext(p, Pos2::new(r.max.x - 2.0, r.center().y), Align::Max, &format!("{} {}", n, if n == 1 { "song" } else { "songs" }), 1.0, pal().ink2);
    p.line_segment([Pos2::new(r.min.x + 2.0, r.max.y - 3.0), Pos2::new(r.max.x - 2.0, r.max.y - 3.0)], Stroke::new(1.0_f32, pal().ink2.gamma_multiply(0.6)));
}
