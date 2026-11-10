//! The PRACTICE panel: the waveform with its loops and marks, the A-B loop and speed, saved loops and bookmarks
//! with their notes, and the tools behind its tabs (trainer, pitch, stems, record).

use super::*;

// little 7x7 pictures for the stems: drums, bass (a woofer), other (keys), vocals (a mic)
pub(crate) const STEM_ICONS: [[&str; 7]; 4] = [
    [".#####.", "#######", "#.....#", "#.....#", "#.....#", ".#####.", "#.....#"],
    [".#####.", "#.....#", "#.###.#", "#.###.#", "#.###.#", "#.....#", ".#####."],
    ["#######", "#.#.#.#", "#.#.#.#", "#.#.#.#", "#.#.#.#", "#.#.#.#", "#######"],
    [".#####.", "#######", "#######", ".#####.", "...#...", "...#...", ".#####."],
];

/// Icon button for one stem: pressed in (bright) while the stem is playing.
pub(crate) fn stem_btn(ui: &mut egui::Ui, i: usize, on: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(44.0, bh()), Sense::click());
    crate::raised_h(ui.painter(), rect, resp.is_pointer_button_down_on() || on, resp.hovered() && !on);
    let dy = if on { 1.0 } else { 0.0 };
    let o = Pos2::new((rect.center().x - 7.0).round(), (rect.center().y - 7.0 + dy).round());
    crate::pixmap(ui.painter(), o, 2.0, &STEM_ICONS[i][..], if on { pal().ink } else { pal().dim });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

impl App {
    // ------------------------------------------------------------- PRACTICE
    /// Transcribing tools: waveform, A-B loop, saved sections, slow-down and the extras behind MORE.
    pub(crate) fn practice_ui(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let inner = ui.max_rect();
        window_deco(ui, inner, "PRACTICE");
        // the focus timer and the metronome: a pill each in the title strip (click the dot to start or stop, the words
        // to open its panel)
        let sc = crate::font::ui_scale();
        let (tw, mw, h) = (170.0 * sc, 140.0 * sc, 24.0);
        let mr = crate::title_slot(inner, crate::title_right(inner), mw, h);
        let tr = crate::title_slot(inner, mr.min.x - 8.0, tw, h);
        self.timer_pill(ui, tr, acts);
        self.metro_pill(ui, mr, acts);
        crate::tour::mark("TIMEBTNS", tr.union(mr));
        // the panel's contents scroll when the panel is short, rather than running past its edge
        // (no drag-to-scroll: dragging on the waveform sets loops)
        // everything inside grows and shrinks with the panel, gently (the square root of the size change), from a
        // little smaller to about twice as big
        // the page: PAGE_PAD in from the frame all round, so nothing runs into it
        let body = crate::page_rect(inner);
        let size = body.size();
        let s = (size.y / 250.0).min(size.x / 760.0).max(0.1).sqrt().clamp(0.85, 2.0);
        // the waveform's height is yours to drag (the handle under it)
        let wave_h = 54.0 * s * self.wv_h.clamp(0.6, 4.0);
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body), |ui| {
            crate::font::with_scale(s, || {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .drag_to_scroll(false)
                    .show(ui, |ui| self.practice_body(ui, acts, wave_h));
            });
        });
    }

    /// A bookmark on this song at `t`: the colour picked for them, or a random one of the swatches.
    pub(crate) fn add_mark(&mut self, id: i64, t: f32) {
        let col = self.mark_col.unwrap_or_else(|| {
            let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
            let used: Vec<[u8; 3]> = self.store.marks.get(&id).map(|l| l.iter().map(|m| m.col).collect()).unwrap_or_default();
            // a colour not on this song yet, when there is one
            let free: Vec<[u8; 3]> = MARK_COLS.iter().copied().filter(|c| !used.contains(c)).collect();
            let pool = if free.is_empty() { MARK_COLS.to_vec() } else { free };
            pool[n as usize % pool.len()]
        });
        let list = self.store.marks.entry(id).or_default();
        list.push(crate::store::Bookmark { t, col, ..Default::default() });
        list.sort_by(|a, b| a.t.total_cmp(&b.t));
        self.store_dirty = true;
        self.set_note(&format!("BOOKMARK AT {}", fmt_t(t)));
        self.keep_for_practice(id);
    }

    /// The note view in the waveform's place: the song by pitch (`notes::NoteMap`) as a heat map, a keyboard down the
    /// left lighting the loudest notes where it's playing, and the name of the note under the mouse.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn notes_strip(&mut self, ui: &egui::Ui, p: &egui::Painter, keys: Rect, inn: Rect, start: f32, span: f32, shown: f32, id: i64) {
        use crate::notes::{FPS, LO, ROWS};
        self.start_notes(id);
        let Some(m) = self.notes.2.clone().filter(|_| self.notes.0 == id) else {
            let msg = if self.notes_busy {
                "ANALYZING NOTES..."
            } else if self.notes.0 == id && self.notes.1 == 0 && stems::have_stems(id) {
                "TURN A STEM ON TO SEE ITS NOTES"
            } else {
                "NO NOTES TO SHOW"
            };
            ptext(p, inn.center(), Align::Center, msg, 1.0, pal().ink2);
            return;
        };
        const CH: usize = 2048;
        if self.notes_tex.is_empty() {
            // heat colours, the same on every skin: dark purple, through red and orange, to pale yellow
            const HEAT: [[f32; 3]; 8] = [
                [6.0, 4.0, 16.0],
                [40.0, 11.0, 84.0],
                [101.0, 21.0, 110.0],
                [159.0, 42.0, 99.0],
                [212.0, 72.0, 66.0],
                [245.0, 125.0, 21.0],
                [250.0, 193.0, 39.0],
                [252.0, 255.0, 164.0],
            ];
            let lut: Vec<egui::Color32> = (0..256)
                .map(|i| {
                    let v = i as f32 / 255.0 * 7.0;
                    let (k, f) = ((v as usize).min(6), v - (v as usize).min(6) as f32);
                    let mix = |j: usize| (HEAT[k][j] + (HEAT[k + 1][j] - HEAT[k][j]) * f) as u8;
                    egui::Color32::from_rgb(mix(0), mix(1), mix(2))
                })
                .collect();
            let lo = lut[0];
            for k in 0..m.cols.div_ceil(CH) {
                let (c0, c1) = (k * CH, ((k + 1) * CH).min(m.cols));
                let w = c1 - c0;
                let mut img = egui::ColorImage::new([w, ROWS], lo);
                for c in c0..c1 {
                    for r in 0..ROWS {
                        img.pixels[(ROWS - 1 - r) * w + (c - c0)] = lut[m.at(c, r) as usize];
                    }
                }
                self.notes_tex.push(ui.ctx().load_texture(format!("notes{}_{}", id, k), img, egui::TextureOptions::LINEAR));
            }
        }
        let (c0, c1) = (start * FPS, (start + span) * FPS);
        let x_of = |c: f32| inn.min.x + (c / FPS - start) / span * inn.width();
        for (k, tex) in self.notes_tex.iter().enumerate() {
            let w = tex.size()[0] as f32;
            let (k0, k1) = ((k * CH) as f32, (k * CH) as f32 + w);
            let (a, b) = (c0.max(k0), c1.min(k1));
            if b <= a {
                continue;
            }
            let r = Rect::from_min_max(Pos2::new(x_of(a), inn.min.y), Pos2::new(x_of(b), inn.max.y));
            let uv = Rect::from_min_max(Pos2::new((a - k0) / w, 0.0), Pos2::new((b - k0) / w, 1.0));
            p.image(tex.id(), r, uv, egui::Color32::WHITE);
        }
        // a faint line at every C; the keyboard, its keys lit where the sound is loudest now; and the names down the
        // side: every note when the rows are tall enough (drag the strip taller), else the C's and the notes lit
        const NAMES: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];
        let name_of = |midi: i32| format!("{}{}", NAMES[midi.rem_euclid(12) as usize], midi / 12 - 1);
        let lit_col = egui::Color32::from_rgb(252, 200, 60);
        let rh = inn.height() / ROWS as f32;
        let y_of = |r: usize| inn.max.y - (r + 1) as f32 * rh;
        let col = ((shown * FPS) as usize).min(m.cols.saturating_sub(1));
        let peak = (0..ROWS).map(|r| m.at(col, r)).max().unwrap_or(0) as u32;
        let name_x = keys.max.x + 3.0;
        let every = rh >= 9.0;
        let mut last_label = f32::MAX;
        for r in 0..ROWS {
            let midi = LO + r as i32;
            let y = y_of(r);
            if midi % 12 == 0 {
                fill_rect(p, Rect::from_min_max(Pos2::new(inn.min.x, y + rh - 0.5), Pos2::new(inn.max.x, y + rh)), egui::Color32::from_white_alpha(28));
            }
            let black = matches!(midi.rem_euclid(12), 1 | 3 | 6 | 8 | 10);
            let lit = peak > 120 && m.at(col, r) as u32 * 10 >= peak * 9;
            let w = if black { keys.width() * 0.6 } else { keys.width() };
            let c = if lit {
                lit_col
            } else if black {
                egui::Color32::from_gray(28)
            } else {
                egui::Color32::from_gray(200)
            };
            p.rect_filled(Rect::from_min_size(Pos2::new(keys.min.x, y), Vec2::new(w, (rh - 0.5).max(0.5))), 0.0, c);
            // (labels go bottom to top; one only where it won't sit on the last)
            if (every || lit || midi % 12 == 0) && last_label - (y + rh / 2.0) >= 9.0 {
                let col = if lit {
                    lit_col
                } else if midi % 12 == 0 {
                    egui::Color32::from_gray(225)
                } else {
                    egui::Color32::from_gray(if black { 110 } else { 160 })
                };
                ptext(p, Pos2::new(name_x, y + rh / 2.0), Align::Min, &name_of(midi), 0.7, col);
                last_label = y + rh / 2.0;
            }
        }
        // the note under the mouse
        if let Some(pp) = ui.input(|i| i.pointer.hover_pos()).filter(|pp| inn.contains(*pp) || keys.contains(*pp)) {
            let r = (((inn.max.y - pp.y) / rh) as i32).clamp(0, ROWS as i32 - 1);
            let name = name_of(LO + r);
            let at = Pos2::new(pp.x + 12.0, pp.y - 8.0);
            let w = crate::font::text_w(&name, 1.0) + 8.0;
            p.rect_filled(Rect::from_min_size(at, Vec2::new(w, 16.0)), 3.0, egui::Color32::from_black_alpha(200));
            ptext(p, Pos2::new(at.x + 4.0, at.y + 8.0), Align::Min, &name, 1.0, egui::Color32::from_gray(240));
        }
    }

    pub(crate) fn practice_body(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, wave_h: f32) {
        // the note view wants room for five octaves
        let notes_on = self.wv_notes;
        // (about half the panel, so a bigger panel gives fatter rows)
        let wave_h = if notes_on { wave_h.max(190.0).max(ui.clip_rect().height() * 0.5) } else { wave_h };
        let dur = self.track_len();
        let pos = self.pos();
        let tid = self.cur_track().map(|t| t.id);
        let active = self.cur.is_some() && !self.stopped && dur > 0.0;
        let shown = self.seek_drag.unwrap_or(pos);
        let both = self.loop_a.is_some() && self.loop_b.is_some();
        let shift = ui.input(|i| i.modifiers.shift);

        // ---- waveform timeline
        let (bar, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), wave_h), Sense::click_and_drag());
        // a handle along its bottom edge: drag it down for a taller waveform, up for a shorter one; double-click resets
        {
            let grip = Rect::from_min_max(Pos2::new(bar.min.x, bar.max.y - 2.0), Pos2::new(bar.max.x, bar.max.y + 5.0));
            let g = ui
                .interact(grip, ui.id().with("wv_height"), Sense::click_and_drag())
                .on_hover_cursor(egui::CursorIcon::ResizeVertical)
                .tip("Drag to resize the waveform. Double-click to reset.");
            if g.dragged() {
                let base = wave_h / self.wv_h.clamp(0.6, 4.0);
                self.wv_h = (self.wv_h + g.drag_delta().y / base.max(1.0)).clamp(0.6, 4.0);
                self.dirty = true;
            }
            if g.double_clicked() {
                self.wv_h = 1.0;
                self.dirty = true;
            }
            let shade = if g.hovered() || g.dragged() { pal().ink } else { pal().ink2.gamma_multiply(0.5) };
            ui.painter().rect_filled(
                Rect::from_center_size(Pos2::new(bar.center().x, bar.max.y + 1.5), Vec2::new(36.0, 3.0)),
                1.0,
                shade,
            );
        }
        let resp = resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .tip("Click to jump. Drag with the right mouse button (or shift + drag) to make a loop; shift + drag inside a loop slides it. Ctrl + drag A or B to move it. Alt + click to add or remove a bookmark. Right-click for more.");
        inset(ui.painter(), bar, pal().lcd);
        let mut inn = bar.shrink2(Vec2::new(5.0, 5.0));
        // (the note view's keyboard down the left)
        let keys = Rect::from_min_max(inn.min, Pos2::new(inn.min.x + 18.0, inn.max.y));
        if notes_on {
            inn.min.x += 48.0;
        }
        // zoom (like Audacity's): the mouse wheel over the waveform zooms in and out where the mouse is; + and -
        // zoom where the song is playing, FIT shows it all. Only the time gets finer - the height stays
        let dur1 = dur.max(0.001);
        let mut zoom = self.wv_zoom.clamp(1.0, 400.0);
        let mut start = self.wv_start;
        let wheel = if resp.hovered() && active { ui.input(|i| i.smooth_scroll_delta.y) } else { 0.0 };
        if wheel != 0.0 {
            // the wheel is the waveform's here, not the panel's scroll
            ui.input_mut(|i| {
                i.smooth_scroll_delta = Vec2::ZERO;
                i.raw_scroll_delta = Vec2::ZERO;
            });
            if let Some(m) = ui.input(|i| i.pointer.hover_pos()) {
                let frac = ((m.x - inn.min.x) / inn.width()).clamp(0.0, 1.0);
                let under = start + frac * dur1 / zoom;
                zoom = (zoom * (wheel / 120.0).exp2()).clamp(1.0, 400.0);
                start = under - frac * dur1 / zoom;
            }
        }
        // the zoom buttons, each as wide as its text (at any text size), one after the other
        let chip_w = |t: &str| (crate::font::text_w(t, 1.0) + 12.0).max(18.0);
        let chip_at = |k: f32, w: f32| Rect::from_min_size(Pos2::new(bar.min.x + 6.0 + k, bar.max.y - 24.0), Vec2::new(w, 16.0));
        let span = dur1 / zoom;
        // zoomed: the view keeps up with the song (turning the page as it reaches the edge), unless you are dragging
        let dragging = self.sel_anchor.is_some() || self.marker_drag != 0 || self.seek_drag.is_some() || wheel != 0.0;
        // clicking the waveform or zooming: the view follows the song again
        if resp.clicked() || resp.drag_started() || wheel != 0.0 {
            self.wv_free = false;
        }
        if zoom > 1.0 && !dragging && !self.wv_free && (shown < start || shown > start + span) {
            start = shown - span * 0.05;
        }
        let start = start.clamp(0.0, (dur1 - span).max(0.0));
        self.wv_zoom = zoom;
        self.wv_start = start;
        let at = |t: f32| inn.min.x + inn.width() * ((t - start) / span).clamp(0.0, 1.0);
        let time_at = |x: f32| start + ((x - inn.min.x) / inn.width()).clamp(0.0, 1.0) * span;
        if active {
            let p = ui.painter().clone();
            // waveform
            // zoomed in to under ~40 ms a pixel: the fine outline, every pick attack its own spike; the part already
            // played a little brighter
            let played = |x: f32| if x < at(shown) { pal().ink2.gamma_multiply(0.85) } else { pal().ink2.gamma_multiply(0.55) };
            let fine = match (tid, &self.wave_fine) {
                _ if notes_on => None,
                (Some(id), (fid, Some(f))) if *fid == id && f.len() > 2 && span / inn.width().max(1.0) < 0.04 => Some(f),
                _ => None,
            };
            if let (true, Some(id)) = (notes_on, tid) {
                self.notes_strip(ui, &p, keys, inn, start, span, shown, id);
            } else if let Some(f) = fine {
                let cols = inn.width().max(1.0) as usize;
                let (mid, half) = (inn.center().y, inn.height() * 0.5);
                let n = f.len();
                for c in 0..cols {
                    let a = (((start + span * c as f32 / cols as f32) * 1000.0) as usize).min(n - 1);
                    let b = (((start + span * (c + 1) as f32 / cols as f32) * 1000.0) as usize).clamp(a + 1, n);
                    let (lo, hi) = f[a..b].iter().fold((0i8, 0i8), |(l, h), p| (l.min(p[0]), h.max(p[1])));
                    let x = inn.min.x + c as f32;
                    let (y0, y1) = (mid - hi as f32 / 127.0 * half, mid - lo as f32 / 127.0 * half);
                    fill_rect(&p, Rect::from_min_max(Pos2::new(x, y0.min(mid - 0.5)), Pos2::new(x + 1.0, y1.max(mid + 0.5))), played(x));
                }
            } else if let (Some(id), (wid, Some(w))) = (tid, &self.wave) {
                if *wid == id && w.len() > 2 {
                    let cols = (inn.width() / 2.0).max(1.0) as usize;
                    let mid = inn.center().y;
                    // the part of the song in view
                    let n = w.len() as f32;
                    let (v0, v1) = (start / dur1 * n, (start + span) / dur1 * n);
                    for c in 0..cols {
                        let a = (v0 + (v1 - v0) * c as f32 / cols as f32) as usize;
                        let b = ((v0 + (v1 - v0) * (c + 1) as f32 / cols as f32) as usize).max(a + 1).min(w.len());
                        let m = w[a.min(w.len() - 1)..b].iter().copied().max().unwrap_or(0);
                        let h = (m as f32 / 255.0) * (inn.height() * 0.5);
                        let x = inn.min.x + c as f32 * 2.0;
                        fill_rect(
                            &p,
                            Rect::from_min_max(Pos2::new(x, mid - h - 0.5), Pos2::new(x + 1.5, mid + h + 0.5)),
                            played(x),
                        );
                    }
                }
            }
            // saved sections as faint bands along the bottom
            if let Some(list) = tid.filter(|id| !self.is_demo(*id)).and_then(|id| self.store.sections.get(&id)) {
                for s in list {
                    fill_rect(
                        &p,
                        Rect::from_min_max(Pos2::new(at(s.a), inn.max.y - 3.0), Pos2::new(at(s.b).max(at(s.a) + 1.0), inn.max.y)),
                        pal().ink.gamma_multiply(0.6),
                    );
                }
            }
            if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
                let col = if self.loop_on { crate::skin::loop_col().gamma_multiply(0.35) } else { pal().ink.gamma_multiply(0.2) };
                fill_rect(&p, Rect::from_min_max(Pos2::new(at(a), inn.min.y), Pos2::new(at(b), inn.max.y)), col);
            }
            for (v, lab) in [(self.loop_a, "A"), (self.loop_b, "B")] {
                if let Some(t) = v {
                    fill_rect(
                        &p,
                        Rect::from_min_max(Pos2::new(at(t) - 1.0, bar.min.y + 2.0), Pos2::new(at(t) + 1.0, bar.max.y - 2.0)),
                        crate::skin::loop_col(),
                    );
                    ptext(&p, Pos2::new(at(t) + 4.0, inn.min.y + 5.0), Align::Min, lab, 1.0, crate::skin::loop_col());
                }
            }
            fill_rect(
                &p,
                Rect::from_min_max(Pos2::new(at(shown) - 1.0, bar.min.y + 2.0), Pos2::new(at(shown) + 1.0, bar.max.y - 2.0)),
                pal().ink,
            );
            ptext(
                &p,
                Pos2::new(bar.max.x - 8.0, bar.min.y + 8.0),
                Align::Max,
                &format!("{} / {}", fmt_t(shown), fmt_t(dur)),
                1.0,
                pal().ink,
            );
            // time marks along the top, as close as the zoom allows (a second, a tenth of one...)
            let step = [0.1f32, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0]
                .into_iter()
                .find(|s| span / s <= 9.0)
                .unwrap_or(600.0);
            let mut t = (start / step).ceil() * step;
            // (no labels under the time readout in the corner)
            let readout = bar.max.x - 8.0 - crate::font::text_w(&format!("{} / {}", fmt_t(shown), fmt_t(dur)), 1.0) - 40.0;
            while t <= start + span + 1e-3 {
                let x = at(t);
                fill_rect(&p, Rect::from_min_max(Pos2::new(x, bar.min.y + 2.0), Pos2::new(x + 1.0, bar.min.y + 7.0)), pal().ink);
                if x < readout {
                    let (m, s) = ((t / 60.0).floor() as u32, t % 60.0);
                    let lab = if step < 1.0 { format!("{}:{:04.1}", m, s) } else { format!("{}:{:02}", m, s.round() as u32) };
                    let lw = crate::font::text_w(&lab, 0.7);
                    p.rect_filled(
                        Rect::from_min_size(Pos2::new(x + 1.5, bar.min.y + 2.0), Vec2::new(lw + 4.0, 10.0)),
                        1.0,
                        egui::Color32::from_black_alpha(190),
                    );
                    ptext(&p, Pos2::new(x + 3.0, bar.min.y + 7.0), Align::Min, &lab, 0.7, egui::Color32::from_gray(235));
                }
                t += step;
            }
            // zoomed: a scrollbar along the bottom to move through the song (drag it, or click where to look)
            if self.wv_zoom > 1.0 {
                let track = Rect::from_min_max(Pos2::new(inn.min.x, bar.max.y - 7.0), Pos2::new(inn.max.x, bar.max.y - 3.0));
                let r = ui
                    .interact(track.expand2(Vec2::new(0.0, 4.0)), ui.id().with("wv_scroll"), Sense::click_and_drag())
                    .on_hover_cursor(egui::CursorIcon::Grab)
                    .tip("Drag to scroll through the song. Playback continues.");
                fill_rect(&p, track, pal().ink.gamma_multiply(0.15));
                let w = (track.width() * span / dur1).max(14.0);
                let x0 = track.min.x + (track.width() - w) * (start / (dur1 - span).max(0.001));
                let thumb = Rect::from_min_size(Pos2::new(x0, track.min.y - 1.0), Vec2::new(w, track.height() + 2.0));
                fill_rect(&p, thumb, if r.hovered() || r.dragged() { pal().ink } else { pal().ink2 });
                if let Some(pp) = r.interact_pointer_pos().filter(|_| r.dragged() || r.clicked()) {
                    let frac = ((pp.x - track.min.x - w / 2.0) / (track.width() - w).max(1.0)).clamp(0.0, 1.0);
                    self.wv_start = frac * (dur1 - span).max(0.0);
                    self.wv_free = true;
                }
            }
            // zoom buttons, over the waveform's corner (they take effect from the next frame)
            let zoom_by = |me: &mut Self, by: f32| {
                let frac = ((shown - start) / span).clamp(0.0, 1.0);
                me.wv_zoom = (me.wv_zoom * by).clamp(1.0, 400.0);
                me.wv_start = shown - frac * dur1 / me.wv_zoom;
                me.wv_free = false;
            };
            let (w1, w2, w3) = (chip_w("-"), chip_w("+"), chip_w("FIT"));
            // bookmarks: + MARK drops one where the song is; right-click it for the colour they get
            if let Some(id) = tid.filter(|id| !self.is_demo(*id)) {
                let w4 = chip_w("+ MARK");
                let mb = crate::chip(ui, chip_at(w1 + w2 + w3 + 9.0, w4), "+ MARK", false, "wv_mark").tip(
                    "Adds a bookmark at the current position (or press K, or alt + click the waveform). Right-click to set their color. Click a flag to jump to it, or right-click it to recolor or remove it.",
                );
                if mb.clicked() {
                    self.add_mark(id, shown);
                }
                let one = self.mark_col;
                mb.context_menu(|ui| {
                    if menu_item(ui, if one.is_none() { "* Random Colors" } else { "  Random Colors" }) {
                        self.mark_col = None;
                        self.dirty = true;
                        ui.close_menu();
                    }
                    crate::views::dim_line(ui, "OR ALL ONE COLOR", 1.0, pal().dim);
                    if let Some(c) = mark_swatches(ui, one) {
                        self.mark_col = Some(c);
                        self.dirty = true;
                        ui.close_menu();
                    }
                    if self.store.marks.get(&id).is_some_and(|m| !m.is_empty()) && menu_item(ui, "Remove This Song's Bookmarks") {
                        self.store.marks.remove(&id);
                        self.store_dirty = true;
                        ui.close_menu();
                    }
                });
                // each bookmark: a line in its colour and a numbered flag along the bottom
                let marks = self.store.marks.get(&id).cloned().unwrap_or_default();
                let mut drop = None;
                let mut recolour = None;
                let mut rename = None;
                for (k, m) in marks.iter().enumerate() {
                    if m.t < start || m.t > start + span {
                        continue;
                    }
                    let col = egui::Color32::from_rgb(m.col[0], m.col[1], m.col[2]);
                    let x = at(m.t);
                    fill_rect(
                        &p,
                        Rect::from_min_max(Pos2::new(x - 1.0, bar.min.y + 12.0), Pos2::new(x + 1.0, bar.max.y - 2.0)),
                        col,
                    );
                    // a short flag (the number, or the name cut to fit, never past the next bookmark); the whole
                    // name is in its tip
                    let next_x = marks.get(k + 1).map_or(inn.max.x, |n| at(n.t));
                    let room = (next_x - x - 3.0).clamp(14.0, 70.0);
                    let full = if m.name.is_empty() { (k + 1).to_string() } else { m.name.clone() };
                    let lab = crate::font::fit(&full, 1.0, room - 8.0);
                    let fw = (crate::font::text_w(&lab, 1.0) + 8.0).min(room);
                    let flag = Rect::from_min_size(Pos2::new(x, bar.max.y - 40.0), Vec2::new(fw, 13.0));
                    p.rect_filled(flag, 1.0, col);
                    ptext(&p, flag.center(), Align::Center, &lab, 1.0, egui::Color32::from_gray(12));
                    let fr = ui
                        .interact(flag.expand(2.0), ui.id().with(("wv_bm", k)), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .tip(mark_tip(m, k, &fmt_t(m.t)));
                    if fr.clicked() && ui.input(|i| i.modifiers.alt) {
                        drop = Some(k);
                    } else if fr.clicked() {
                        acts.push(Action::Seek(m.t));
                    }
                    fr.context_menu(|ui| {
                        if menu_item(ui, "Rename / Add a Note...") {
                            rename = Some(k);
                            ui.close_menu();
                        }
                        if let Some(c) = mark_swatches(ui, Some(m.col)) {
                            recolour = Some((k, c));
                            ui.close_menu();
                        }
                        if menu_item(ui, "Remove This Bookmark") {
                            drop = Some(k);
                            ui.close_menu();
                        }
                    });
                }
                // rename / a note: the BOOKMARKS tab, its note open to write in (its name is on its row)
                if let Some(k) = rename {
                    self.more = true;
                    self.mtab = 3;
                    if let Some(m) = marks.get(k) {
                        self.edit_mark(id, k, m, true);
                    }
                }
                if let Some(list) = self.store.marks.get_mut(&id) {
                    if let Some((k, c)) = recolour {
                        list[k].col = c;
                        self.store_dirty = true;
                    }
                    if let Some(k) = drop {
                        list.remove(k);
                        self.store_dirty = true;
                    }
                }
            }
            if crate::chip(ui, chip_at(0.0, w1), "-", false, "wv_out").tip("Zoom out.").clicked() {
                zoom_by(self, 0.5);
            }
            if crate::chip(ui, chip_at(w1 + 3.0, w2), "+", false, "wv_in")
                .tip("Zoom in at the playhead, or use the mouse wheel on the waveform.")
                .clicked()
            {
                zoom_by(self, 2.0);
            }
            // the note view, on or off
            let nx = w1 + w2 + w3 + 12.0 + chip_w("+ MARK");
            if crate::chip(ui, chip_at(nx, chip_w("NOTES")), "NOTES", self.wv_notes, "wv_notes")
                .tip("Shows the song by pitch instead of loudness: brighter means louder at that note. The lowest bright bar is usually the note; fainter copies above it are its overtones. On a split song it uses only the stems you have on, so a solo or bass line stands out. Hover over it to see the note's name.")
                .clicked()
            {
                self.wv_notes = !self.wv_notes;
                self.dirty = true;
            }
            if self.wv_zoom > 1.0
                && crate::chip(ui, chip_at(w1 + w2 + 6.0, w3), "FIT", false, "wv_fit").tip("Show the whole song again.").clicked()
            {
                self.wv_zoom = 1.0;
                self.wv_start = 0.0;
                self.wv_free = false;
            }

            // shift + drag (or a drag with the right button) selects a loop; plain click / drag seeks
            if resp.drag_started_by(egui::PointerButton::Secondary) || (resp.drag_started() && shift) {
                if let Some(pp) = resp.interact_pointer_pos() {
                    self.sel_anchor = Some(time_at(pp.x));
                }
            }
            // ctrl + drag near A or B moves that marker
            let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
            if resp.drag_started() && ctrl {
                if let Some(pp) = resp.interact_pointer_pos() {
                    self.marker_drag = [(1u8, self.loop_a), (2u8, self.loop_b)]
                        .into_iter()
                        .filter_map(|(k, v)| v.map(|t| (k, (at(t) - pp.x).abs())))
                        .filter(|c| c.1 < 14.0)
                        .min_by(|a, b| a.1.total_cmp(&b.1))
                        .map_or(0, |c| c.0);
                }
            }
            // shift + drag inside the loop slides the whole loop, keeping its length
            if resp.drag_started() && shift && !ctrl {
                if let (Some(a), Some(b), Some(pp)) = (self.loop_a, self.loop_b, resp.interact_pointer_pos()) {
                    let t = time_at(pp.x);
                    if t > a && t < b {
                        self.marker_drag = 3;
                        self.pan_off = t - a;
                        self.pan_len = b - a;
                        self.sel_anchor = None;
                    }
                }
            }
            if self.marker_drag != 0 {
                if let Some(pp) = ui.input(|i| i.pointer.hover_pos()) {
                    let t = time_at(pp.x);
                    if self.marker_drag == 3 {
                        let a = (t - self.pan_off).clamp(0.0, (dur - self.pan_len).max(0.0));
                        acts.push(Action::SetAAt(a));
                        acts.push(Action::SetBAt(a + self.pan_len));
                    } else if self.marker_drag == 1 {
                        acts.push(Action::SetAAt(self.loop_b.map_or(t, |b| t.min(b - 0.2))));
                    } else {
                        acts.push(Action::SetBAt(self.loop_a.map_or(t, |a| t.max(a + 0.2))));
                    }
                }
                if resp.drag_stopped() {
                    self.marker_drag = 0;
                }
            } else if let Some(anc) = self.sel_anchor {
                if let Some(pp) = ui.input(|i| i.pointer.hover_pos()) {
                    let (x1, x2) = (at(anc), pp.x.clamp(inn.min.x, inn.max.x));
                    let (lo, hi) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
                    fill_rect(
                        &p,
                        Rect::from_min_max(Pos2::new(lo, inn.min.y), Pos2::new(hi, inn.max.y)),
                        crate::skin::loop_col().gamma_multiply(0.3),
                    );
                }
                if resp.drag_stopped() {
                    if let Some(pp) = resp.interact_pointer_pos() {
                        let t = time_at(pp.x);
                        let (a, b) = if anc < t { (anc, t) } else { (t, anc) };
                        if b - a > 0.3 {
                            acts.push(Action::SetAAt(a));
                            acts.push(Action::SetBAt(b));
                        }
                    }
                    self.sel_anchor = None;
                }
            } else if resp.clicked() && ui.input(|i| i.modifiers.alt) {
                // alt + click: a bookmark there - or, on one, takes it away (the song plays on)
                if let (Some(id), Some(pp)) = (tid.filter(|id| !self.is_demo(*id)), resp.interact_pointer_pos()) {
                    let near = self.store.marks.get(&id).and_then(|l| l.iter().position(|m| (at(m.t) - pp.x).abs() <= 6.0));
                    match near {
                        Some(k) => {
                            if let Some(l) = self.store.marks.get_mut(&id) {
                                l.remove(k);
                            }
                            self.store_dirty = true;
                            self.set_note("BOOKMARK REMOVED");
                        }
                        None => self.add_mark(id, time_at(pp.x)),
                    }
                }
            } else {
                if resp.dragged() || resp.clicked() {
                    if let Some(pp) = resp.interact_pointer_pos() {
                        self.seek_drag = Some(time_at(pp.x));
                    }
                }
                if resp.drag_stopped() || resp.clicked() {
                    if let Some(t) = self.seek_drag.take() {
                        acts.push(Action::Seek(t));
                    }
                }
            }
            if resp.secondary_clicked() {
                if let Some(pp) = resp.interact_pointer_pos() {
                    self.menu_t = time_at(pp.x);
                }
            }
            let (mt, lo, pr) = (self.menu_t, self.loop_on, self.practice);
            let mut mark_here = false;
            resp.context_menu(|ui| {
                crate::loop_menu(ui, acts, mt, both, lo, pr);
                if menu_item(ui, &format!("Bookmark Here  {}", fmt_t(mt))) {
                    mark_here = true;
                    ui.close_menu();
                }
            });
            if let Some(id) = tid.filter(|id| mark_here && !self.is_demo(*id)) {
                self.add_mark(id, mt);
            }
        } else {
            ptext(ui.painter(), bar.center(), Align::Center, "PLAY A TRACK TO START", 2.0, pal().dim);
        }
        ui.add_space(6.0);

        // ---- the panel's tabs: the loop and speed tools first, then the bigger tools (TRAINER, PITCH & EAR, STEMS)
        let cur = if self.more { self.mtab as usize + 1 } else { 0 };
        // (short names, so the six fit on one row)
        if let Some(i) = tab_row(ui, &["LOOP", "TRAINER", "PITCH", "STEMS", "BOOKMARKS", "RECORD"], cur) {
            self.more = i > 0;
            if i > 0 {
                self.mtab = (i - 1) as u8;
            }
        }
        ui.add_space(4.0);
        if self.more {
            let top = ui.cursor().min.y;
            self.more_tools(ui, acts);
            let r = ui.min_rect();
            crate::tour::mark("MOREBOX", Rect::from_min_max(Pos2::new(r.min.x, top), Pos2::new(r.max.x, ui.cursor().min.y)));
            return;
        }

        // ---- A and B, then the loop's own buttons  (each block reports where it is, for the practice tour)
        let row_mark = |ui: &egui::Ui, key: &str, top: f32| {
            let r = ui.min_rect();
            crate::tour::mark(key, Rect::from_min_max(Pos2::new(r.min.x, top), Pos2::new(r.max.x, ui.cursor().min.y)));
        };
        let ab_top = ui.cursor().min.y;
        let a_txt = self.loop_a.map(fmt_t).unwrap_or_else(|| "-:--.-".to_string());
        let b_txt = self.loop_b.map(fmt_t).unwrap_or_else(|| "-:--.-".to_string());
        // three rows, each centred: A and B; the loop's buttons and moving about; the speed
        centered_row(ui, "ab_row", |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if retro_btn_w(ui, "A", 34.0, false).tip("Set loop start  ([ key)").clicked() && active {
                acts.push(Action::SetAAt(pos));
            }
            self.num_step(
                ui,
                acts,
                F_LOOP_A,
                78.0,
                Knob::LoopA,
                a_txt,
                "Nudge earlier or later, or click and type a time like 1:23.5.",
            );
            divider(ui);
            if retro_btn_w(ui, "B", 34.0, false).tip("Set loop end  (] key)").clicked() && active {
                acts.push(Action::SetBAt(pos));
            }
            self.num_step(
                ui,
                acts,
                F_LOOP_B,
                78.0,
                Knob::LoopB,
                b_txt,
                "Nudge earlier or later, or click and type a time like 2:05.",
            );
        });
        ui.add_space(6.0);
        centered_row(ui, "loop_btns", |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let ink = pal().ink;
            if crate::icon_btn_w(ui, &crate::IC_REW, false, ink, 34.0).tip("Back two seconds  (, key)").clicked() {
                acts.push(Action::SeekRel(-2.0));
            }
            if crate::icon_btn_w(ui, &crate::IC_PREV, false, ink, 34.0)
                .tip("Restart from the loop start (or track start)  (. key)")
                .clicked()
            {
                acts.push(Action::Seek(self.loop_a.unwrap_or(0.0)));
            }
            divider(ui);
            if crate::icon_btn_w(ui, &crate::IC_REP, self.loop_on && both, ink, 34.0).tip("Loop on / off  (\\ key)").clicked() {
                acts.push(Action::LoopToggle);
            }
            if crate::icon_btn_w(ui, &crate::IC_X, false, ink, 34.0).tip("Clear the loop.").clicked() {
                acts.push(Action::LoopClear);
            }
            divider(ui);
            // the whole loop on to the next phrase, or back to the one before (same length)
            if crate::icon_btn_w(ui, &crate::IC_BACK, false, ink, 34.0).tip("Shift the loop back by its own length.").clicked()
                && both
            {
                acts.push(Action::ShiftLoop(false));
            }
            if crate::icon_btn_w(ui, &crate::IC_FWD, false, ink, 34.0)
                .tip("Shift the loop forward by its own length, to the next phrase.")
                .clicked()
                && both
            {
                acts.push(Action::ShiftLoop(true));
            }
        });
        row_mark(ui, "LOOPROW", ab_top);
        ui.add_space(6.0);

        // ---- the speed
        let speed_top = ui.cursor().min.y;
        centered_row(ui, "speed_row", |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            label(ui, "SPEED", 0.0);
            // the usual speeds in a drop-down, and - / + for 1% at a time (click the number to type any, 25 to 250)
            let sp = retro_btn_w(ui, "v", 34.0, false).tip("Pick a speed.");
            let menu = ui.id().with("speed_menu");
            if sp.clicked() {
                ui.memory_mut(|m| m.toggle_popup(menu));
            }
            egui::popup::popup_below_widget(ui, menu, &sp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(110.0);
                for pct in [25u32, 30, 40, 50, 60, 70, 75, 80, 85, 90, 95, 100, 110, 125, 150] {
                    let mark = if self.speed == pct { "> " } else { "   " };
                    if menu_item(ui, &format!("{}{}%", mark, pct)) {
                        acts.push(Action::Speed(pct));
                    }
                }
            });
            self.num_step(
                ui,
                acts,
                F_SPEED,
                64.0,
                Knob::Speed,
                format!("{}%", self.speed),
                "1% slower or faster, or click and type any speed from 25 to 250.",
            );
        });
        row_mark(ui, "SPEEDROW", speed_top);
        ui.add_space(14.0);

        // ---- saved loops for this track: which one is looping now (or that it isn't saved yet)
        if let (Some(a), Some(b), Some(id)) = (self.loop_a, self.loop_b, tid) {
            let saved = self
                .store
                .sections
                .get(&id)
                .and_then(|l| l.iter().find(|s| (s.a - a).abs() < 0.05 && (s.b - b).abs() < 0.05))
                .map(|s| s.name.to_uppercase());
            let what = match (&saved, self.loop_on) {
                (Some(n), true) => format!("LOOPING: {}", n),
                (Some(n), false) => format!("LOOP SET: {} (OFF)", n),
                (None, true) => "LOOPING, UNSAVED. NAME IT AND PRESS SAVE LOOP.".to_string(),
                (None, false) => "LOOP SET, OFF, UNSAVED".to_string(),
            };
            dim_line(ui, &what, 1.0, if saved.is_some() { pal().ink } else { pal().ink2 });
        }
        ui.horizontal_wrapped(|ui| {
            // the save button's real width; if the name box cannot keep a usable size, the button drops to the next line
            let bw = 10.0 + 18.0 + 8.0 + text_w("SAVE LOOP", 2.0) + 10.0;
            let w = (ui.available_width() - bw - 12.0).max(150.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let o = field(ui, &mut self.ed, F_SEC, &mut self.sec_name, rect, "Name this loop (bridge, head, lick)", false);
            if o.enter {
                acts.push(Action::SaveSection);
            }
            if crate::ibtn(ui, &crate::IC_BOOK, "SAVE LOOP", false).tip("Save this A-B loop with the track.").clicked() {
                acts.push(Action::SaveSection);
            }
        });
        self.saved_loops(ui, acts, tid);

        ui.add_space(6.0);
    }

    /// The practice panel's saved loops of this song, by group: click one to loop it; right-click to rename it, add a
    /// note, put it in a group or delete it (asked first).
    pub(crate) fn saved_loops(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, tid: Option<i64>) {
        let Some(id) = tid.filter(|id| !self.is_demo(*id)) else { return };
        let secs: Vec<crate::store::Section> = self.store.sections.get(&id).cloned().unwrap_or_default();
        if secs.is_empty() {
            label(ui, "SAVED LOOPS OF THIS TRACK SHOW UP HERE", 300.0);
            return;
        }
        // groups in the order they first appear, loops with no group first
        let mut groups: Vec<String> = vec![String::new()];
        for s in &secs {
            if !groups.contains(&s.group) {
                groups.push(s.group.clone());
            }
        }
        let mut named: Vec<String> = self.store.groups();
        for g in groups.iter().filter(|g| !g.is_empty()) {
            if !named.contains(g) {
                named.push(g.clone());
            }
        }
        for g in &groups {
            let members: Vec<usize> = (0..secs.len()).filter(|i| secs[*i].group == *g).collect();
            if members.is_empty() {
                continue;
            }
            let noted: Vec<bool> = members.iter().map(|&i| !secs[i].note.trim().is_empty()).collect();
            // (the group's name before its first loops; the rest of the group lines up after it)
            let mut lab_w = 0.0;
            for (n, run) in note_runs(&noted).into_iter().enumerate() {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                    if !g.is_empty() {
                        if n == 0 {
                            let x0 = ui.cursor().min.x;
                            label(ui, &g.to_uppercase(), (text_w(&g.to_uppercase(), 1.0) + 16.0).min(160.0));
                            lab_w = ui.cursor().min.x - x0;
                        } else {
                            ui.add_space(lab_w - 6.0);
                        }
                    }
                    for &i in &members[run.clone()] {
                        self.loop_pill(ui, acts, id, i, &secs[i], &named);
                    }
                });
                if run.len() == 1 && noted[run.start] {
                    let i = members[run.start];
                    if note_under(ui, &secs[i].note, lab_w + 10.0).clicked() {
                        self.edit_loop(id, i, &secs[i], true);
                    }
                }
                if self.edit_in_practice && self.clip_edit.is_some_and(|(lid, k)| lid == id && members[run.clone()].contains(&k)) {
                    self.loop_editor(ui, acts);
                }
            }
        }
        if let Some((_, i)) = self.loop_ask.filter(|(lid, _)| *lid == id) {
            ui.horizontal_wrapped(|ui| {
                let name = secs.get(i).map(|s| s.name.to_uppercase()).unwrap_or_default();
                label(ui, &format!("DELETE {}?", name), (text_w(&name, 1.0) + 90.0).min(320.0));
                if retro_btn(ui, "DELETE", false).clicked() {
                    acts.push(Action::DeleteSection(i));
                    self.loop_ask = None;
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.loop_ask = None;
                }
            });
        }
    }

    /// One saved loop in the practice panel: a pill to play it, right-click to rename, note, group or delete it.
    pub(crate) fn loop_pill(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, id: i64, i: usize, s: &store::Section, named: &[String]) {
        {
            {
                {
                    let w = (text_w(&s.name, 2.0) + 20.0).clamp(44.0, 180.0);
                    let on = self.loop_on
                        && self.loop_a.is_some_and(|x| (x - s.a).abs() < 0.05)
                        && self.loop_b.is_some_and(|x| (x - s.b).abs() < 0.05);
                    let tip = if s.note.is_empty() {
                        format!("{} to {}. Right-click to rename, add a note, group or delete.", fmt_t(s.a), fmt_t(s.b))
                    } else {
                        format!("{} to {}. Right-click to rename, edit the note, group or delete.", fmt_t(s.a), fmt_t(s.b))
                    };
                    let r = retro_btn_w(ui, &s.name, w, on).tip(tip);
                    if r.clicked() {
                        acts.push(Action::GoSection(i));
                    }
                    r.context_menu(|ui| {
                        if menu_item(ui, if s.note.is_empty() { "Rename / Add a Note..." } else { "Rename / Edit the Note..." }) {
                            self.edit_loop(id, i, s, true);
                            ui.close_menu();
                        }
                        for other in named.iter().filter(|o| **o != s.group) {
                            if menu_item(ui, &format!("Move to {}", other)) {
                                acts.push(Action::LoopGroup(id, i, other.clone()));
                                ui.close_menu();
                            }
                        }
                        if menu_item(ui, "Move to a New Group...") {
                            self.edit_loop(id, i, s, true);
                            self.clip_group = String::new();
                            self.ed.id = crate::tools::F_CLIP_GROUP;
                            self.ed.anchor = None;
                            ui.close_menu();
                        }
                        if !s.group.is_empty() && menu_item(ui, "Remove from Group") {
                            acts.push(Action::LoopGroup(id, i, String::new()));
                            ui.close_menu();
                        }
                        if menu_item(ui, "Delete This Loop...") {
                            self.loop_ask = Some((id, i));
                            ui.close_menu();
                        }
                    });
                }
            }
        }
    }

    /// Opens a loop's editor (its name, note and group), its note ready to write in; `in_practice`: in the practice
    /// panel, else on the LOOPS page.
    pub(crate) fn edit_loop(&mut self, id: i64, i: usize, s: &store::Section, in_practice: bool) {
        self.mark_edit = None;
        self.clip_edit = Some((id, i));
        self.edit_in_practice = in_practice;
        self.clip_name = s.name.clone();
        self.clip_note = s.note.clone();
        self.clip_group = s.group.clone();
        self.write_note();
    }

    /// Opens a bookmark's editor (its name and note), its note ready to write in.
    pub(crate) fn edit_mark(&mut self, id: i64, k: usize, m: &store::Bookmark, in_practice: bool) {
        self.clip_edit = None;
        self.mark_edit = Some((id, k));
        self.edit_in_practice = in_practice;
        self.clip_name = m.name.clone();
        self.clip_note = m.note.clone();
        self.write_note();
    }

    /// The note box takes the keys, the caret at its end.
    pub(crate) fn write_note(&mut self) {
        self.ed.id = crate::tools::F_CLIP_NOTE;
        self.ed.cur = self.clip_note.chars().count();
        self.ed.anchor = None;
    }

    /// Editing a loop: its name, a note, the group it's in (with this song's other groups to pick from).
    pub(crate) fn loop_editor(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let Some((id, i)) = self.clip_edit else { return };
        let o1 = field_row(
            ui,
            &mut self.ed,
            crate::tools::F_CLIP_NAME,
            "NAME",
            &mut self.clip_name,
            "funky riff, the lick at 2:10",
            None,
        );
        note_row(ui, &mut self.ed, crate::tools::F_CLIP_NOTE, &mut self.clip_note, "what it is, what to work on");
        let o3 = field_row(
            ui,
            &mut self.ed,
            crate::tools::F_CLIP_GROUP,
            "GROUP",
            &mut self.clip_group,
            "solo licks, head, comping (empty: none)",
            None,
        );
        let mut groups: Vec<String> = self.store.groups();
        for s in self.store.sections.get(&id).into_iter().flatten() {
            if !s.group.is_empty() && !groups.contains(&s.group) {
                groups.push(s.group.clone());
            }
        }
        let mut done = o1.enter || o3.enter || save_keys(ui);
        ui.horizontal_wrapped(|ui| {
            ui.add_space(12.0);
            for g in &groups {
                if retro_btn(ui, &crate::font::as_label(g), *g == self.clip_group).tip("Put it in this group.").clicked() {
                    self.clip_group = g.clone();
                }
            }
            if retro_btn(ui, "SAVE", false).clicked() {
                done = true;
            }
            if retro_btn(ui, "CANCEL", false).clicked() {
                self.clip_edit = None;
                self.ed.id = 0;
            }
        });
        if done {
            acts.push(Action::EditLoop(id, i, self.clip_name.clone(), self.clip_note.clone(), self.clip_group.clone()));
            self.clip_edit = None;
            self.ed.id = 0;
        }
        ui.add_space(6.0);
    }

    /// BOOKMARKS: this song's bookmarks in time order - jump to one, name it, recolour or remove it.
    pub(crate) fn bookmarks_tab(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let Some(id) = self.cur_track().map(|t| t.id).filter(|id| !self.is_demo(*id)) else {
            crate::views::dim_line(ui, "PLAY A SONG TO BOOKMARK IT", 1.0, pal().dim);
            return;
        };
        let now = self.pos();
        if let Some(t) = self.cur_track() {
            let song = if t.artist.is_empty() { t.title.clone() } else { format!("{} - {}", t.artist, t.title) };
            dim_line(ui, &format!("BOOKMARKS IN {}", song.to_uppercase()), 2.0, pal().ink);
        }
        centered_row(ui, "marks_add", |ui| {
            if retro_btn(ui, &format!("+ BOOKMARK AT {}", fmt_t(now)), false)
                .tip(format!(
                    "Adds a bookmark at the current position (or press {}, or alt + click the waveform).",
                    self.key_text(crate::prefs::Cmd::Bookmark)
                ))
                .clicked()
            {
                self.add_mark(id, now);
            }
            let n = self.store.marks.get(&id).map_or(0, |l| l.len());
            if n > 0 && retro_btn(ui, "REMOVE ALL", false).tip("Remove all bookmarks from this song.").clicked() {
                self.store.marks.remove(&id);
                self.store_dirty = true;
            }
        });
        ui.add_space(4.0);
        let n = self.store.marks.get(&id).map_or(0, |l| l.len());
        if n == 0 {
            para(ui, "No bookmarks on this song yet. They mark places to find your way: the solo, the key change, the bar you keep missing.", pal().ink2);
            return;
        }
        let mut drop = None;
        let mut recolour = None;
        for k in 0..n {
            let m = self.store.marks[&id][k].clone();
            let mut write = false;
            ui.horizontal(|ui| {
                let col = egui::Color32::from_rgb(m.col[0], m.col[1], m.col[2]);
                // its colour: right-click for another
                let (sw, sr) = ui.allocate_exact_size(Vec2::new(22.0, bh()), Sense::click());
                ui.painter().rect_filled(sw.shrink2(Vec2::new(2.0, 4.0)), 2.0, col);
                ptext(ui.painter(), sw.center(), Align::Center, &(k + 1).to_string(), 1.0, egui::Color32::from_gray(12));
                let note_item = if m.note.is_empty() { "Add a note..." } else { "Edit the note..." };
                sr.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Right-click to add a note or change the color.").context_menu(|ui| {
                    if menu_item(ui, note_item) {
                        write = true;
                        ui.close_menu();
                    }
                    if let Some(c) = mark_swatches(ui, Some(m.col)) {
                        recolour = Some((k, c));
                        ui.close_menu();
                    }
                });
                let tb = retro_btn_w(ui, &fmt_t(m.t), 70.0, false).tip("Jump here. Right-click to add a note or remove it.");
                if tb.clicked() {
                    acts.push(Action::Seek(m.t));
                }
                tb.context_menu(|ui| {
                    if menu_item(ui, note_item) {
                        write = true;
                        ui.close_menu();
                    }
                    if menu_item(ui, "Remove This Bookmark") {
                        drop = Some(k);
                        ui.close_menu();
                    }
                });
                let w = (ui.available_width() - 40.0).clamp(80.0, 720.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                let fid = 5000 + k as u32;
                if let Some(list) = self.store.marks.get_mut(&id) {
                    let o = field(ui, &mut self.ed, fid, &mut list[k].name, r, "Name it (solo, bridge, key change)", false);
                    if o.changed {
                        self.store_dirty = true;
                    }
                }
                if retro_btn_w(ui, "X", 28.0, false).tip("Remove this bookmark.").clicked() {
                    drop = Some(k);
                }
            });
            // its note under it, lined up with its name: a click edits it
            let indent = 22.0 + 70.0 + ui.spacing().item_spacing.x * 2.0 + 8.0;
            if !m.note.trim().is_empty() && note_under(ui, &m.note, indent).clicked() {
                write = true;
            }
            if write {
                self.edit_mark(id, k, &m, true);
            }
            if self.edit_in_practice && self.mark_edit == Some((id, k)) {
                self.mark_editor(ui, false);
            }
        }
        if let Some(list) = self.store.marks.get_mut(&id) {
            if let Some((k, c)) = recolour {
                list[k].col = c;
                self.store_dirty = true;
            }
            if let Some(k) = drop {
                list.remove(k);
                self.store_dirty = true;
            }
        }
    }

    /// The practice panel's other tabs: TRAINER (0), PITCH (1), STEMS (2), BOOKMARKS (3).
    pub(crate) fn more_tools(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let tid = self.cur_track().map(|t| t.id);
        if self.mtab == 3 {
            self.bookmarks_tab(ui, acts);
            return;
        }
        if self.mtab == 4 {
            self.record_tab(ui, acts);
            return;
        }
        match self.mtab {
            0 => {
                // the count-off is its own thing: clicks before every pass of the loop, with or without the trainer
                centered_row(ui, "trainer_0", |ui| {
                    label(ui, "COUNT-OFF", 92.0);
                    let ci = if self.count_in == 0 { "OFF".to_string() } else { format!("{} BEATS", self.count_in) };
                    if retro_btn_w(ui, &ci, 92.0, self.count_in > 0)
                        .tip(
                            "Clicks before every pass of the loop, at the song's tempo. Off, 2 or 4 beats, with or without the trainer.",
                        )
                        .clicked()
                    {
                        acts.push(Action::CountIn);
                    }
                    // the song's tempo the count-off clicks at: found by itself, or set here if that is wrong
                    ui.add_space(8.0);
                    label(ui, "BPM", 36.0);
                    let s = if self.bpm > 0 { self.bpm.to_string() } else { "---".to_string() };
                    self.num_step(
                        ui,
                        acts,
                        crate::tools::F_PR_BPM,
                        46.0,
                        Knob::Bpm,
                        s,
                        "The song's tempo, detected automatically. If it's wrong, adjust it or type it.",
                    );
                    if retro_btn_w(ui, "TAP", 52.0, false).tip("Tap along with the music to find the tempo.").clicked() {
                        acts.push(Action::TapTempo);
                    }
                });
                ui.add_space(8.0);
                centered_row(ui, "trainer_1", |ui| {
                    label(ui, "TRAIN", 56.0);
                    if retro_btn_w(ui, if self.trainer { "ON" } else { "OFF" }, 56.0, self.trainer)
                        .tip("Plays the loop a set number of times, then speeds up by a set percent.")
                        .clicked()
                    {
                        acts.push(Action::Trainer);
                    }
                    if self.trainer {
                        lcd_box(ui, &format!("PASS {}/{}", self.passes, self.trainer_n), 100.0, pal().ink);
                        if self.trainer_keys {
                            lcd_box(ui, &format!("KEY {}/12", self.keys_done + 1), 90.0, pal().ink);
                        }
                    }
                });
                centered_row(ui, "trainer_2", |ui| {
                    label(ui, "THEN", 60.0);
                    if retro_btn(ui, "FASTER", !self.trainer_keys).tip("After the passes, the speed goes up.").clicked() {
                        acts.push(Action::TrainerKeys(false));
                    }
                    if retro_btn(ui, "UP A 4TH - 12 KEYS", self.trainer_keys)
                        .tip("After the passes, the loop moves up a 4th. Learn a lick from the record in every key, by ear.")
                        .clicked()
                    {
                        acts.push(Action::TrainerKeys(true));
                    }
                });
                centered_row(ui, "trainer_3", |ui| {
                    label(ui, "EVERY", 60.0);
                    self.num_step(
                        ui,
                        acts,
                        F_TR_LOOPS,
                        44.0,
                        Knob::Loops,
                        self.trainer_n.to_string(),
                        "Passes to play before each step. Or type it.",
                    );
                    label(ui, "LOOPS", 56.0);
                    self.num_step(
                        ui,
                        acts,
                        F_TR_STEP,
                        52.0,
                        Knob::Step,
                        format!("+{}%", self.trainer_step),
                        "How much faster each step is. Or type it.",
                    );
                });
                dim_centered(ui, "STARTS AT THE SPEED SET ABOVE AND STOPS AT 100%");
                ui.add_space(8.0);
                centered_row(ui, "trainer_4", |ui| {
                    if retro_btn_w(ui, "FOCUS", 76.0, self.focus_mode)
                        .tip("Hides the lists, leaving just the player and these tools.")
                        .clicked()
                    {
                        acts.push(Action::ToggleFocus);
                    }
                });
            }
            1 => {
                centered_row(ui, "pitch_0", |ui| {
                                        label(ui, "PITCH", 56.0);
                                        if retro_btn_w(ui, "-", 28.0, false)
                                            .tip("Lower the pitch a semitone. The speed stays.")
                                            .clicked()
                                        {
                                            acts.push(Action::Transpose(-100));
                                        }
                                        if retro_btn_w(ui, "-10c", 44.0, false)
                                            .tip("Lower the pitch 10 cents, a tenth of a semitone. For records not tuned to A440.")
                                            .clicked()
                                        {
                                            acts.push(Action::Transpose(-10));
                                        }
                                        // the pitch, to read or to type: semitones (+0.25, -2) or cents (30c, -15c); Enter sets it
                                        if self.ed.id != crate::tools::F_PITCH {
                                            self.pitch_text = format!("{:+.2}", self.semis as f32 / 100.0);
                                        }
                                        let (pr, _) = ui.allocate_exact_size(Vec2::new(78.0, bh()), Sense::hover());
                                        let o = field(ui, &mut self.ed, crate::tools::F_PITCH, &mut self.pitch_text, pr, "+0.00", false);
                                        if o.enter {
                                            let s = self.pitch_text.trim().to_lowercase().replace(' ', "");
                                            // semitones, cents ("30c"), or the A to hear the record at ("445hz")
                                            let rec_a = tid.and_then(crate::meta::get).and_then(|i| i.a4).unwrap_or(440.0);
                                            let cents = if s.ends_with("hz") {
                                                crate::meta::parse_hz(&s).map(|hz| crate::meta::cents_between(rec_a, hz))
                                            } else {
                                                match s.strip_suffix('c') {
                                                    Some(c) => c.parse::<f32>().ok(),
                                                    None => s.parse::<f32>().ok().map(|x| x * 100.0),
                                                }
                                            };
                                            if let Some(c) = cents {
                                                let want = (c.round() as i32).clamp(-1200, 1200);
                                                acts.push(Action::Transpose(want - self.semis));
                                            }
                                            self.ed.id = 0;
                                        }
                                        if retro_btn_w(ui, "+10c", 44.0, false)
                                            .tip("Raise the pitch 10 cents.")
                                            .clicked()
                                        {
                                            acts.push(Action::Transpose(10));
                                        }
                                        if retro_btn_w(ui, "+", 28.0, false).tip("Raise the pitch a semitone. The speed stays.").clicked() {
                                            acts.push(Action::Transpose(100));
                                        }
                                    });
                // the tuning: the A the record is at (found, or typed), the A you play at, and the shift between
                let cur_id = self.cur_track().map(|t| t.id);
                let rec_a = cur_id.and_then(crate::meta::get).and_then(|i| i.a4);
                let unclear = matches!(self.tuning, Some((id, _, conf)) if Some(id) == cur_id && conf < 0.0);
                let hz = |a: f32| if (a - a.round()).abs() < 0.05 { format!("{:.0}", a) } else { format!("{:.1}", a) };
                centered_row(ui, "pitch_t0", |ui| {
                    label(ui, "RECORD A =", 100.0);
                    if self.ed.id != crate::tools::F_REC_A {
                        self.rec_a_text = rec_a.map_or(String::new(), hz);
                    }
                    let (r, _) = ui.allocate_exact_size(Vec2::new(64.0, bh()), Sense::hover());
                    let o = field(ui, &mut self.ed, crate::tools::F_REC_A, &mut self.rec_a_text, r, "---", false);
                    if o.enter {
                        // a typed A (452), or nothing to forget it
                        if let Some(id) = cur_id {
                            let t = self.rec_a_text.trim().to_string();
                            if t.is_empty() {
                                crate::meta::set_a4(id, None);
                                self.store_dirty = true;
                            } else if let Some(a) = crate::meta::parse_hz(&t) {
                                crate::meta::set_a4(id, Some(a));
                                self.store_dirty = true;
                            }
                        }
                        self.ed.id = 0;
                    }
                    label(ui, "HZ", 28.0);
                    let find = if self.tuning_busy {
                        "LISTENING..."
                    } else if unclear {
                        "UNCLEAR: AGAIN"
                    } else {
                        "FIND"
                    };
                    if retro_btn_w(ui, find, if find == "FIND" { 60.0 } else { 130.0 }, false)
                        .tip("Listens to the record and finds the A it's tuned to. If you know it (A = 452), type it in the box instead.")
                        .clicked()
                    {
                        acts.push(Action::FindTuning);
                    }
                });
                centered_row(ui, "pitch_t1", |ui| {
                    label(ui, "YOUR A =", 100.0);
                    if self.ed.id != crate::tools::F_MY_A {
                        self.my_a_text = hz(self.my_a);
                    }
                    let (r, _) = ui.allocate_exact_size(Vec2::new(64.0, bh()), Sense::hover());
                    let o = field(ui, &mut self.ed, crate::tools::F_MY_A, &mut self.my_a_text, r, "440", false);
                    if o.enter {
                        if let Some(a) = crate::meta::parse_hz(&self.my_a_text) {
                            self.my_a = a;
                            self.dirty = true;
                        }
                        self.ed.id = 0;
                    }
                    label(ui, "HZ", 28.0);
                    match rec_a {
                        Some(a) => {
                            let c = crate::meta::cents_between(a, self.my_a).round() as i32;
                            let txt = if c == 0 { "IN TUNE".to_string() } else { format!("MATCH ({:+}c)", c) };
                            let tip = format!(
                                "Moves the record from A = {} to your A = {}: {:+} cents. The speed stays.",
                                hz(a),
                                hz(self.my_a),
                                c
                            );
                            if retro_btn_w(ui, &txt, 124.0, self.semis == c).tip(&tip).clicked() {
                                acts.push(Action::Transpose(c - self.semis));
                            }
                        }
                        None => {
                            retro_btn_w(ui, "MATCH", 80.0, false).tip("Find the record's A, or type it, first.");
                        }
                    }
                });
                // what you hear now, in cents and in Hz
                let line = match rec_a {
                    Some(a) => format!(
                        "Now {:+}c: the record's A {} sounds at {} Hz",
                        self.semis,
                        hz(a),
                        hz(a * 2f32.powf(self.semis as f32 / 1200.0))
                    ),
                    None => format!("Now {:+}c. Type 445hz in Pitch to hear the record at A = 445 Hz", self.semis),
                };
                dim_centered(ui, &line);
                ui.add_space(4.0);
                // back to the record's pitch, the ear mode and saving it
                centered_row(ui, "pitch_1", |ui| {
                                        if self.semis != 0 && retro_btn_w(ui, "0", 28.0, false).tip("Reset to the original pitch.").clicked() {
                                            acts.push(Action::Transpose(-self.semis));
                                        }
                                        ui.add_space(8.0);
                                        if retro_btn_w(ui, CHAN_NAMES[self.chan as usize % 6], 100.0, self.chan != 0)
                                            .tip("Ear mode. LEFT or RIGHT CHANNEL plays just that side in both ears. Old jazz records often put the bass or piano on one side, while modern mixes barely differ. NO CENTER removes vocals, and BASS ONLY keeps the low end.")
                                            .clicked()
                                        {
                                            acts.push(Action::Chan);
                                        }
                                        if retro_btn_w(ui, if self.exporting { "SAVING..." } else { "EXPORT WAV" }, 100.0, false)
                                            .tip("Saves the song as you hear it to a WAV file, with stems, speed, pitch, ear mode and equalizer. It saves the A-B loop if there is one, or else the whole song.")
                                            .clicked()
                                        {
                                            acts.push(Action::Export);
                                        }
                                    });
            }
            _ => {
                let mut stem_note: Option<String> = None;
                let mut stem_files = None;
                centered_row(ui, "stems_0", |ui| {
                    label(ui, "STEMS", 56.0);
                    if self.stem_busy == 1 {
                    lcd_box(ui, &stems::stage_text(), 300.0, pal().ink2);
                } else if self.stem_busy == 2 {
                    let (d, t) = (stems::DONE.load(Ordering::Relaxed), stems::TOTAL.load(Ordering::Relaxed));
                    lcd_box(ui, &format!("{} {}/{}", stems::stage_text(), d, t), 240.0, pal().ink2);
                    if retro_btn(ui, "CANCEL", false).clicked() {
                        acts.push(Action::StemCancel);
                    }
                } else if tid.is_some_and(|id| self.is_demo(id)) {
                    // the tour's song shows the button you would see on any song not split yet
                    if retro_btn_w(ui, "SPLIT THIS TRACK", 190.0, false)
                        .tip("Splits the track into drums, bass, vocals and other. Not during the tour.")
                        .clicked()
                    {
                        self.set_note("NOT DURING THE TOUR");
                    }
                } else if !stems::tool_ready() {
                    if retro_btn_w(ui, "GET STEMS TOOL (170 MB)", 230.0, false)
                        .tip("A one-time download of the tool that splits drums, bass, vocals and the rest. It runs on this PC.")
                        .clicked()
                    {
                        acts.push(Action::StemGet);
                    }
                } else if tid.is_some_and(stems::have_stems) {
                    for (i, name) in stems::NAMES.iter().enumerate() {
                        let r = stem_btn(ui, i, self.stem_on[i])
                            .tip(format!("{}. Click to mute or unmute, right-click to solo.", name));
                        if r.clicked() {
                            acts.push(Action::StemToggle(i));
                        }
                        r.context_menu(|ui| {
                            if menu_item(ui, "Solo This Stem") {
                                acts.push(Action::StemSolo(i));
                                ui.close_menu();
                            }
                        });
                    }
                    if retro_btn_w(ui, "ALL", 44.0, false).clicked() {
                        acts.push(Action::StemAll);
                    }
                    // what they take on this computer
                    if let Some(id) = tid {
                        let (this, all) = stems::sizes(id);
                        stem_note = Some(format!("THIS SONG'S STEMS: {}   ALL STEMS: {}", cache::fmt_size(this), cache::fmt_size(all)));
                        stem_files = Some(id);
                    }
                } else {
                    let len = self.track_len();
                    if retro_btn_w(ui, "SPLIT THIS TRACK", 190.0, false)
                        .tip("Splits the track into drums, bass, vocals and other, on this PC. The stems are saved.")
                        .clicked()
                    {
                        acts.push(Action::StemSplit);
                    }
                    // what it will cost before you press it
                    let t = stems::time_guess(len);
                    let time = if t < 90 { format!("{} SEC", t.div_ceil(5) * 5) } else { format!("{} MIN", t.div_ceil(60)) };
                    stem_note = Some(format!("ABOUT {} AND {} ON DISK", time, cache::fmt_size(stems::size_guess(len))));
                }
                });
                if let Some(t) = stem_note {
                    dim_centered(ui, &t);
                }
                // this song's stem files: show them, or delete them (a second click to be sure)
                if let Some(id) = stem_files {
                    let sure_key = egui::Id::new("stem_del_sure");
                    let now = ui.input(|i| i.time);
                    let sure = ui.ctx().data(|d| d.get_temp::<(i64, f64)>(sure_key)).is_some_and(|(i, t)| i == id && now - t < 4.0);
                    centered_row(ui, "stems_files", |ui| {
                        if retro_btn_w(ui, "OPEN FOLDER", 130.0, false)
                            .tip("Shows this song's four stem files (drums, bass, vocals, other) on your computer.")
                            .clicked()
                        {
                            acts.push(Action::StemOpen(id));
                        }
                        let text = if sure { "CLICK AGAIN TO DELETE" } else { "DELETE THESE STEMS" };
                        if retro_btn_w(ui, text, 210.0, sure)
                            .tip("Deletes this song's stems to free space. The song itself stays. You can split it again any time.")
                            .clicked()
                        {
                            if sure {
                                ui.ctx().data_mut(|d| d.remove::<(i64, f64)>(sure_key));
                                acts.push(Action::StemDelete(id));
                            } else {
                                ui.ctx().data_mut(|d| d.insert_temp(sure_key, (id, now)));
                            }
                        }
                    });
                    if sure {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
                    }
                }
            }
        }
    }
}
