//! Song lists: the rows (`list_row`) and the table they sit in (columns you choose and size, covers, hearts, a
//! sticky header), the cards view, the play / save buttons over a list, the right-click song menus, and a Tidal
//! page (album, artist, playlist, mix) with its bio.

use super::*;

/// One list row. Text is only built (and drawn) when the row is on screen.
pub(crate) fn list_row(
    ui: &mut egui::Ui,
    idx: usize,
    num: Option<usize>,
    left: impl FnOnce() -> String,
    right: impl FnOnce() -> String,
    state: RowState,
    drag: bool,
    stored: bool,
) -> Option<egui::Response> {
    // taken even when the row is off screen, so it never lands on the next one
    let heart = ROW_HEART.with(|h| h.take());
    let split = ROW_STEMS.with(|s| s.take());
    let art = ROW_ART.with(|a| a.borrow_mut().take());
    let gutter = ROW_GUTTER.with(|g| g.take());
    let w = ui.available_width();
    // (a list too narrow for the cover goes without, as its header does)
    let fit = row_fit(w, num.is_some(), art.is_some());
    let art = art.filter(|_| fit.art);
    let (num, heart) = (num.filter(|_| fit.num), heart.filter(|_| fit.heart));
    // (taller with the library's zoom, and with a cover)
    let row_h = if art.is_some() { ROW_ART_H } else { ROW_H } * font::ui_scale();
    let probe = Rect::from_min_size(ui.cursor().min, Vec2::new(w, row_h));
    // rows sit flush, whatever gap the page puts between other things
    let flush = |ui: &mut egui::Ui| {
        let gap = ui.spacing().item_spacing.y;
        if gap > 0.0 {
            ui.add_space(-gap);
        }
    };
    if !ui.is_rect_visible(probe) {
        // (a cover a screen away is asked for already, so it is there when the row scrolls into view)
        if let Some(url) = art.filter(|u| !u.is_empty()) {
            let clip = ui.clip_rect();
            if clip.expand2(Vec2::new(0.0, clip.height())).intersects(probe) {
                ART_WANT.with(|w| w.borrow_mut().push(url));
            }
        }
        ui.allocate_space(Vec2::new(w, row_h));
        flush(ui);
        return None;
    }
    let sense = if drag { Sense::click_and_drag() } else { Sense::click() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, row_h), sense);
    flush(ui);
    let p = ui.painter();
    let bg = if state == RowState::Playing {
        Some(pal().row_sel)
    } else if resp.hovered() {
        Some(pal().sel)
    } else if idx % 2 == 1 {
        Some(pal().row_alt)
    } else {
        None
    };
    if let Some(c) = bg {
        fill_rect(p, rect, c);
    }
    // the song playing stands out in any skin: a bar across the row and a stripe down its left edge
    if state == RowState::Playing {
        fill_rect(p, rect, pal().bar_txt.gamma_multiply(0.16));
        fill_rect(p, Rect::from_min_size(rect.min, Vec2::new(thick(3.0), rect.height())), pal().bar_txt);
    }
    let (fg, fg2) = match state {
        RowState::Playing => (pal().bar_txt, pal().bar_txt),
        RowState::Played => (pal().dim, pal().dim),
        RowState::Normal => (pal().ink, pal().ink2),
    };
    let cy = rect.center().y;
    let rtxt = right();
    let mut rw = if let Some((a, b)) = rtxt.split_once('\t') {
        // two right-hand columns, lined up with `table_header`
        row_text(p, Pos2::new(rect.max.x - 10.0, cy), Align::Max, b, 200.0, fg2);
        let w = row_text(p, Pos2::new(rect.max.x - 10.0 - TBL_W2, cy), Align::Max, a, 200.0, fg2);
        TBL_W2 + w
    } else {
        row_text(p, Pos2::new(rect.max.x - 10.0, cy), Align::Max, &rtxt, 300.0, fg2)
    };
    if stored {
        pixmap(p, Pos2::new(rect.max.x - 10.0 - rw - 16.0, cy - 4.0), 2.0, &CHECK[..], fg2);
        rw += 16.0;
    }
    // split into stems: four little faders
    if split {
        let x = rect.max.x - 10.0 - rw - 16.0;
        let mr = Rect::from_min_size(Pos2::new(x - 2.0, rect.min.y), Vec2::new(16.0, rect.height()));
        for (k, h) in [6.0, 9.0, 5.0, 8.0].iter().enumerate() {
            let bx = x + k as f32 * 3.0;
            fill_rect(p, Rect::from_min_max(Pos2::new(bx, cy + 4.0 - h), Pos2::new(bx + 2.0, cy + 4.0)), fg2);
        }
        ui.interact(mr, ui.id().with(("stems_mark", idx, rect.min.y as i32)), Sense::hover())
            .tip("Stems ready: drums, bass, vocals and the rest can be turned on and off.");
        rw += 16.0;
    }
    // a heart, when the caller asked for one
    let lraw = left();
    let left_has_cells = lraw.contains('\t');
    if let Some(liked) = heart {
        let heart_w = HEART_W * font::ui_scale();
        let slot = if left_has_cells {
            rect.max.x - 10.0 - fit.right + if fit.src { SRC_W * font::ui_scale() } else { 0.0 }
        } else {
            rect.max.x - 10.0 - rw - heart_w
        };
        let hr = Rect::from_min_size(Pos2::new(slot, rect.min.y), Vec2::new(heart_w, rect.height()));
        if draw_heart(ui, hr, liked, resp.hovered(), ui.id().with(("heart", idx, rect.min.y as i32))) {
            HEART_HIT.with(|h| h.set(true));
        }
        if !left_has_cells {
            rw += heart_w;
        }
    }
    let x0 = rect.min.x + 8.0;
    let mut tx = x0;
    if let Some(n) = num {
        if state == RowState::Playing {
            pixmap(p, Pos2::new(x0 + 2.0, cy - 5.0), 2.0, &PLAY_S[..], pal().bar_txt);
        } else {
            row_text(p, Pos2::new(x0 + 34.0, cy), Align::Max, &n.to_string(), 34.0, fg2);
        }
        tx = x0 + 44.0 * font::ui_scale();
    }
    if let Some(url) = &art {
        let a = ART_PX * font::ui_scale();
        row_cover(ui, Rect::from_min_size(Pos2::new(tx, cy - a / 2.0), Vec2::splat(a)), url);
        tx += ART_W * font::ui_scale();
    } else if gutter {
        tx += GUT_W * font::ui_scale();
    }
    if left_has_cells {
        // cell layout shared with the column header
        let cw = col_widths(fit.avail);
        let cells: Vec<&str> = lraw.split('\t').collect();
        let mut x = tx;
        for (i, c) in cells.iter().enumerate().take(3) {
            if cw[i] > 0.0 {
                row_text(p, Pos2::new(x, cy), Align::Min, c, (cw[i] - 12.0).max(20.0), if i == 0 { fg } else { fg2 });
                x += cw[i];
            }
        }
        if let (true, Some(src)) = (fit.src, cells.get(4)) {
            let sc = font::ui_scale();
            row_text(p, Pos2::new(rect.max.x - 10.0 - fit.right, cy), Align::Min, src, (SRC_W - 10.0) * sc, fg2);
        }
        if let (true, Some((key, bpm))) = (fit.kb, cells.get(3).and_then(|c| c.split_once('|'))) {
            let sc = font::ui_scale();
            let kx = rect.max.x - 10.0 - (80.0 + KB_W) * sc;
            row_text(p, Pos2::new(kx, cy), Align::Min, key, (KB_GAP - 6.0) * sc, fg2);
            row_text(p, Pos2::new(kx + KB_GAP * sc, cy), Align::Min, bpm, 50.0 * sc, fg2);
        }
    } else {
        let avail = (rect.max.x - 10.0 - rw - 16.0) - tx;
        row_text(p, Pos2::new(tx, cy), Align::Min, &lraw, avail.max(20.0), fg);
    }
    Some(resp.on_hover_cursor(egui::CursorIcon::PointingHand))
}

/// Your own files' numbers (as the lists know them), for what only a file on this computer can do.
pub(crate) static FILE_IDS: std::sync::Mutex<std::collections::BTreeSet<i64>> = std::sync::Mutex::new(std::collections::BTreeSet::new());

pub(crate) fn file_id(id: i64, on: bool) {
    let mut s = FILE_IDS.lock().unwrap_or_else(|e| e.into_inner());
    if on {
        s.insert(id);
    } else {
        s.remove(&id);
    }
}

/// A song with a file on this computer: one of yours, or a saved copy of a streamed one.
pub(crate) fn on_disk(id: i64) -> bool {
    cache::has(id) || FILE_IDS.lock().unwrap_or_else(|e| e.into_inner()).contains(&id)
}

/// "Go to album / artist / track radio" and the credits, for a Tidal track (files, YouTube and SoundCloud have none).
/// A track with several artists gets one entry per artist, so you pick which one.
pub(crate) fn goto_items(ui: &mut egui::Ui, acts: &mut Vec<Action>, t: &Track, tidal: bool) {
    // your files, YouTube and SoundCloud songs can be renamed; a Tidal song's names are Tidal's, but its key and
    // tempo can be set (kept on this computer)
    if menu_item(ui, if tidal { "Key and BPM..." } else { "Edit Info..." }) {
        acts.push(Action::EditInfo(t.id, t.title.clone(), t.artist.clone()));
        ui.close_menu();
    }
    // a file of yours, or a song saved on this computer: see the file itself
    if on_disk(t.id)
        && menu_item(ui, "Show in Explorer") {
            acts.push(Action::ShowFile(t.id));
            ui.close_menu();
        }
    if !tidal {
        return;
    }
    let mut go = |ui: &mut egui::Ui, label: &str, a: Action| {
        if menu_item(ui, label) {
            acts.push(a);
            ui.close_menu();
        }
    };
    go(ui, "Go to album", Action::GoTo(t.id, GoTo::Album));
    if t.artists.len() > 1 {
        go(ui, "Go to artist...", Action::PickArtist(t.artists.clone()));
    } else {
        go(ui, "Go to artist", Action::GoTo(t.id, GoTo::Artist));
    }
    go(ui, "Go to track radio", Action::GoTo(t.id, GoTo::Radio));
    go(ui, "Show credits", Action::Credits(t.id, format!("{} - {}", t.artists_text(), t.title)));
}

/// Right-click menu for loop points (shared by the seek bar and the practice timeline).
pub(crate) fn loop_menu(ui: &mut egui::Ui, acts: &mut Vec<Action>, t: f32, both: bool, loop_on: bool, practice: bool) {
    if !PRACTICE {
        return;
    }
    if menu_item(ui, &format!("Loop start (A) here  {}", fmt_t(t))) {
        acts.push(Action::SetAAt(t));
        ui.close_menu();
    }
    if menu_item(ui, &format!("Loop end (B) here  {}", fmt_t(t))) {
        acts.push(Action::SetBAt(t));
        ui.close_menu();
    }
    if both {
        if menu_item(ui, if loop_on { "Loop Off" } else { "Loop On" }) {
            acts.push(Action::LoopToggle);
            ui.close_menu();
        }
        if menu_item(ui, "Clear Loop") {
            acts.push(Action::LoopClear);
            ui.close_menu();
        }
    }
    if menu_item(ui, if practice { "Back to Listening Mode" } else { "Practice Mode" }) {
        acts.push(Action::TogglePractice);
        ui.close_menu();
    }
}

/// Width of the last right-hand column in two-column rows.
pub(crate) const TBL_W2: f32 = 140.0;

/// Header bar for a list: a title on the left and up to two right-aligned column titles.
pub(crate) fn table_header(ui: &mut egui::Ui, left: &str, mid: &str, last: &str, numbered: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 20.0), Sense::hover());
    fill_rect(ui.painter(), rect, pal().edge);
    let cy = rect.center().y;
    let p = ui.painter();
    if numbered {
        ptext(p, Pos2::new(rect.min.x + 8.0, cy), Align::Min, "#", 2.0, pal().accent);
    }
    ptext(p, Pos2::new(rect.min.x + 8.0 + if numbered { 44.0 } else { 0.0 }, cy), Align::Min, left, 2.0, pal().accent);
    ptext(p, Pos2::new(rect.max.x - 10.0 - TBL_W2, cy), Align::Max, mid, 2.0, pal().accent);
    ptext(p, Pos2::new(rect.max.x - 10.0, cy), Align::Max, last, 2.0, pal().accent);
}

// ------------------------------------------------------------------ list columns
/// Which extra columns the track lists show: bit0 artist, bit1 album, bit2 length, bit3 key and tempo
/// (name is always shown).
pub(crate) static COLS: AtomicU32 = AtomicU32::new(0b11101);
pub(crate) const COL_NAMES: [&str; 5] = ["ARTIST", "ALBUM", "LENGTH", "KEY / BPM", "SOURCE"];
/// width of the SOURCE slot (Tidal, SoundCloud, YouTube, File), left of the heart
pub(crate) const SRC_W: f32 = 100.0;
/// width of the KEY / BPM slot, just left of LENGTH; the BPM sits `KB_GAP` in (room for "C#m?")
pub(crate) const KB_W: f32 = 120.0;
pub(crate) const KB_GAP: f32 = 64.0;
/// how the NAME, ARTIST and ALBUM columns share the room (drag the lines between them in the header)
pub(crate) static COL_W: Mutex<[f32; 3]> = Mutex::new([0.4, 0.3, 0.3]);
/// the columns were changed (shown, hidden or resized): the settings need saving
pub(crate) static COLS_CHANGED: AtomicBool = AtomicBool::new(false);

/// The library's songs after a look (`newest`: Tidal's newest hundred, newest first; `total`: its count now): the
/// newest as Tidal has them on top (new ones, and any taken out and liked again: moved up, never twice), the rest as
/// they were. None when the count doesn't add up (some were taken out, or more came than the look sees): then it's
/// read again whole. Also how many are new.
pub(crate) fn liked_top(old: &[Track], had: usize, newest: &[Track], total: usize) -> Option<(Vec<Track>, usize)> {
    let have: HashSet<i64> = old.iter().map(|t| t.id).collect();
    let added = newest.iter().filter(|t| !have.contains(&t.id)).count();
    if total != had + added {
        return None;
    }
    let top: HashSet<i64> = newest.iter().map(|t| t.id).collect();
    let mut v: Vec<Track> = newest.to_vec();
    v.extend(old.iter().filter(|t| !top.contains(&t.id)).cloned());
    Some((v, added))
}

/// How long ago a moment was, as people say it: "just now", "1 min ago", "5 mins ago", "1 hr ago", "3 hrs ago",
/// "2 days ago" (`then` and `now` in seconds since 1970).
pub(crate) fn ago_text(then: u64, now: u64) -> String {
    let d = now.saturating_sub(then);
    let plural = |n: u64, one: &str, many: &str| format!("{} {} ago", n, if n == 1 { one } else { many });
    match d {
        0..=59 => "just now".to_string(),
        60..=3599 => plural(d / 60, "min", "mins"),
        3600..=86399 => plural(d / 3600, "hr", "hrs"),
        _ => plural(d / 86400, "day", "days"),
    }
}

/// Your liked songs as they load in the background: 0 loading, 1 all read, 2 the reading stopped (an error).
pub(crate) static LIB_LOAD: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// How a track row (and the header over it) fits a list `w` wide.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RowFit {
    /// where the cells start, from the row's left (after the number and the cover)
    pub(crate) lead: f32,
    /// the room for NAME, ARTIST and ALBUM
    pub(crate) avail: f32,
    /// the room kept on the right for the heart, LENGTH and KEY / BPM
    pub(crate) right: f32,
    /// KEY / BPM shown
    pub(crate) kb: bool,
    /// SOURCE shown (where the song comes from), at the left of the right-hand slots
    pub(crate) src: bool,
    /// the cover shown
    pub(crate) art: bool,
    /// the row's number shown
    pub(crate) num: bool,
    /// the heart shown
    pub(crate) heart: bool,
}

/// A narrow list gives way before the names get squeezed past reading: ALBUM, then ARTIST (`col_widths`), then
/// SOURCE, KEY / BPM, then the covers; a very narrow one the numbers, then the hearts.
pub(crate) fn row_fit(w: f32, numbered: bool, art: bool) -> RowFit {
    let sc = font::ui_scale();
    let fit = |src: bool, kb: bool, art: bool, num: bool, heart: bool| {
        let lead = 8.0 + if num { 44.0 * sc } else { 0.0 } + if art { ART_W * sc } else { 0.0 };
        let right =
            (80.0 + if heart { HEART_W } else { 0.0 } + if kb { KB_W } else { 0.0 } + if src { SRC_W } else { 0.0 }) * sc;
        RowFit { lead, avail: w - 10.0 - right - lead, right, kb, src, art, num, heart }
    };
    let (min, least) = (110.0 * sc, 60.0 * sc);
    let mut f = fit(col_on(4), col_on(3), art, numbered, true);
    if f.avail < min && f.src {
        f = fit(false, f.kb, f.art, f.num, true);
    }
    if f.avail < min && f.kb {
        f = fit(false, false, f.art, f.num, true);
    }
    if f.avail < min && f.art {
        f = fit(false, false, false, f.num, true);
    }
    if f.avail < least && f.num {
        f = fit(false, false, false, false, true);
    }
    if f.avail < least {
        f = fit(false, false, false, false, false);
    }
    f.avail = f.avail.max(40.0);
    f
}

/// the heart's slot in a track row, left of KEY / BPM and LENGTH
pub(crate) const HEART_W: f32 = 24.0;

thread_local! {
    /// set just before `list_row` to give that row a heart (Some(liked)); a click on it is left in HEART_HIT
    static ROW_HEART: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
    static HEART_HIT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// set just before `list_row` when that row's song is split into stems
    static ROW_STEMS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// set just before `list_row`: the row's cover (a picture's address; "" none) to show at its left
    static ROW_ART: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    /// set just before `list_row`: without a cover, the row still starts after the header's covers switch
    static ROW_GUTTER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// the rows' covers asked for this frame (on screen, or a screen away), and from last frame's, those ready to
    /// draw (None: there is no picture)
    pub(crate) static ART_WANT: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    pub(crate) static ART_TEX: std::cell::RefCell<HashMap<String, Option<egui::TextureId>>> = std::cell::RefCell::new(HashMap::new());
}

/// A track row's height with covers on, the cover's size, and the room it takes at the row's left.
pub(crate) const ROW_ART_H: f32 = 40.0;
pub(crate) const ART_PX: f32 = 32.0;
pub(crate) const ART_W: f32 = 42.0;
/// the room for the covers switch at the left of a list of albums, playlists or artists with covers off
pub(crate) const GUT_W: f32 = 26.0;

/// Give the next `list_row` a cover at its left, when covers are on (`cover` as a track keeps it: a Tidal picture's
/// id, a web address or "file:<path>"; empty for none).
pub(crate) fn row_art(cover: &str) {
    if row_arts_on() {
        let url = if cover.is_empty() { String::new() } else { format!("thumb:{}", cover_url(cover, 80)) };
        ROW_ART.with(|a| *a.borrow_mut() = Some(url));
    }
}

/// A row's cover: the picture once it has come in (loaded in the background), a quiet square till then, the gem when
/// there is none.
pub(crate) fn row_cover(ui: &egui::Ui, r: Rect, url: &str) {
    let tex = if url.is_empty() {
        Some(None)
    } else {
        ART_WANT.with(|w| w.borrow_mut().push(url.to_string()));
        ART_TEX.with(|t| t.borrow().get(url).copied())
    };
    match tex {
        Some(Some(id)) => {
            egui::Image::new(egui::load::SizedTexture::new(id, r.size())).rounding(Rounding::same(3.0)).paint_at(ui, r);
        }
        Some(None) => paint_record(ui.painter(), r, None),
        None => {
            ui.painter().rect_filled(r, Rounding::same(3.0), Color32::from_black_alpha(45));
        }
    }
}

/// The covers drawing rows asked for: an icon of a picture (the switch in a track list's header).
pub(crate) fn picture_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    let r = Rect::from_center_size(c, Vec2::new(14.0, 11.0));
    let s = Stroke::new(1.3f32, col);
    p.rect_stroke(r, 1.5, s);
    p.line_segment([Pos2::new(r.min.x + 2.0, r.max.y - 2.0), Pos2::new(r.min.x + 6.0, r.min.y + 5.0)], s);
    p.line_segment([Pos2::new(r.min.x + 6.0, r.min.y + 5.0), Pos2::new(r.min.x + 9.0, r.max.y - 3.0)], s);
    p.line_segment([Pos2::new(r.min.x + 8.0, r.min.y + 7.0), Pos2::new(r.max.x - 2.0, r.max.y - 2.0)], s);
    p.circle_filled(Pos2::new(r.max.x - 3.5, r.min.y + 3.5), 1.4, col);
}

/// Mark the next `list_row` when its song has stems.
pub(crate) fn row_stems(id: i64) {
    ROW_STEMS.with(|s| s.set(stems::is_split(id)));
}

/// Give the next `list_row` a heart: filled when the song is liked, a faint outline when not.
pub(crate) fn row_heart(liked: bool) {
    ROW_HEART.with(|h| h.set(Some(liked)));
    HEART_HIT.with(|h| h.set(false));
}

/// Was the heart of the row just drawn clicked?
pub(crate) fn heart_clicked() -> bool {
    HEART_HIT.with(|h| h.replace(false))
}

/// A heart in `r`, clickable (like / unlike).
pub(crate) fn draw_heart(ui: &egui::Ui, r: Rect, liked: bool, row_hot: bool, id: egui::Id) -> bool {
    let resp = ui.interact(r, id, Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let resp = resp.tip(if liked { "Remove from My Tracks" } else { "Add to My Tracks" });
    let col = if liked {
        // (the Greyscale look: no colour, the heart too)
        if crate::skin::mono() { pal().ink } else { Color32::from_rgb(235, 70, 90) }
    } else if resp.hovered() {
        pal().ink
    } else if row_hot {
        pal().ink2
    } else {
        pal().dim.gamma_multiply(0.6)
    };
    let o = Pos2::new(r.center().x - 7.0, r.center().y - 6.0);
    if liked || resp.hovered() {
        pixmap(ui.painter(), o, 2.0, &HEART[..], col);
    } else {
        pixmap(ui.painter(), o, 2.0, &HEART_O[..], col);
    }
    resp.clicked()
}

pub(crate) fn col_on(i: usize) -> bool {
    COLS.load(Ordering::Relaxed) >> i & 1 == 1
}

/// Widths of the NAME, ARTIST and ALBUM cells inside `avail` (0 for hidden ones).
pub(crate) fn col_widths(avail: f32) -> [f32; 3] {
    let w = *COL_W.lock().unwrap();
    let mut on = [true, col_on(0), col_on(1)];
    // too little room for every column (a narrow library, or zoomed in): ALBUM goes, then ARTIST, before the names
    // get squeezed past reading
    let min = 110.0 * font::ui_scale();
    for drop in [2usize, 1] {
        let shown = on.iter().filter(|o| **o).count() as f32;
        if avail / shown < min && on[drop] {
            on[drop] = false;
        }
    }
    let sum: f32 = (0..3).filter(|&i| on[i]).map(|i| w[i]).sum();
    let mut out = [0.0; 3];
    for i in (0..3).filter(|&i| on[i]) {
        out[i] = avail * w[i] / sum.max(1e-6);
    }
    out
}

/// A list's header row that stays at the top of the list while it scrolls: where it would go, and the painter to
/// draw it with (over the rows).
pub(crate) fn sticky_head(ui: &egui::Ui, rect: Rect, key: &str) -> (Rect, egui::Painter) {
    let clip = ui.clip_rect();
    let rect = if rect.min.y < clip.min.y { rect.translate(Vec2::new(0.0, clip.min.y - rect.min.y)) } else { rect };
    let p = ui.painter().clone().with_layer_id(egui::LayerId::new(egui::Order::Middle, ui.id().with(key))).with_clip_rect(clip);
    (rect, p)
}

/// The header of a list of albums, playlists or artists: NAME, and what the right-hand column holds. It stays at
/// the top while the list scrolls. `art`: the rows have covers (or the room for the covers switch, at the left).
pub(crate) fn cards_header(ui: &mut egui::Ui, right: &str, art: bool) {
    let w = ui.available_width();
    let sc = font::ui_scale();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 20.0 * sc), Sense::hover());
    let (rect, p) = sticky_head(ui, rect, "cards_head");
    fill_rect(&p, rect, pal().edge);
    let cy = rect.center().y;
    let mut x = rect.min.x + 8.0;
    if art {
        // as the rows have it: the covers' column, or a narrow one for the switch when they're off (or don't fit)
        let shown = row_arts_on() && row_fit(w, false, true).art;
        let col = if shown { ART_W } else { GUT_W } * sc;
        art_switch(ui, &p, Pos2::new(x + if shown { ART_PX * sc / 2.0 } else { 10.0 }, cy), rect.height(), "cards_head_art");
        x += col;
    }
    ui_text(&p, Pos2::new(x, cy), Align::Min, "NAME", 2.0, 400.0, pal().accent);
    ui_text(&p, Pos2::new(rect.max.x - 10.0, cy), Align::Max, right, 2.0, 400.0, pal().accent);
}

/// The covers switch in a list's header: a little picture, lit when covers are on; a click turns them on or off
/// (in every list).
pub(crate) fn art_switch(ui: &egui::Ui, p: &egui::Painter, c: Pos2, h: f32, key: &str) {
    let on = row_arts_on();
    let ir = Rect::from_center_size(c, Vec2::new(24.0, h));
    let ihit = ui.interact(ir, ui.id().with(key), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let col = if ihit.hovered() { pal().ink } else if on { pal().accent } else { pal().accent.gamma_multiply(0.45) };
    picture_icon(p, ir.center(), col);
    if ihit.tip(if on { "Covers on. Click to hide them." } else { "Covers off. Click to show them." }).clicked() {
        ROW_ARTS.store(!on, Ordering::Relaxed);
        COLS_CHANGED.store(true, Ordering::Relaxed);
    }
}

/// Header row over a track list; right-click it to choose the columns.
/// `art`: the rows below have covers (room kept for them; the switch at the left turns them on and off).
pub(crate) fn col_header(ui: &mut egui::Ui, numbered: bool, art: bool) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 20.0 * font::ui_scale()), Sense::hover());
    // it stays at the top while the list scrolls (and is clicked where it shows)
    let (rect, painter) = sticky_head(ui, rect, "col_head");
    let resp = ui.interact(rect, ui.id().with("col_head_hit"), Sense::click());
    fill_rect(&painter, rect, pal().edge);
    let cy = rect.center().y;
    let fit = row_fit(w, numbered, art);
    let tx = rect.min.x + fit.lead;
    let avail = fit.avail;
    let cw = col_widths(avail);
    let p = &painter;
    if fit.num {
        // covers on or off: a little picture, lit when they're on
        art_switch(ui, p, Pos2::new(rect.min.x + 18.0, cy), rect.height(), "col_head_art");
    }
    if fit.src {
        let sc = font::ui_scale();
        ui_text(p, Pos2::new(rect.max.x - 10.0 - fit.right, cy), Align::Min, "SOURCE", 2.0, (SRC_W - 10.0) * sc, pal().accent);
    }
    if fit.kb {
        let sc = font::ui_scale();
        let kx = rect.max.x - 10.0 - (80.0 + KB_W) * sc;
        ui_text(p, Pos2::new(kx, cy), Align::Min, "KEY", 2.0, (KB_GAP - 6.0) * sc, pal().accent);
        ui_text(p, Pos2::new(kx + KB_GAP * sc, cy), Align::Min, "BPM", 2.0, 50.0 * sc, pal().accent);
    }
    // each heading kept inside its column
    ui_text(p, Pos2::new(tx, cy), Align::Min, "NAME", 2.0, (cw[0] - 12.0).max(20.0), pal().accent);
    let mut x = tx + cw[0];
    for (i, name) in COL_NAMES[..2].iter().enumerate() {
        if cw[i + 1] > 0.0 {
            ui_text(p, Pos2::new(x, cy), Align::Min, name, 2.0, (cw[i + 1] - 12.0).max(20.0), pal().accent);
            x += cw[i + 1];
        }
    }
    if col_on(2) {
        ui_text(p, Pos2::new(rect.max.x - 10.0, cy), Align::Max, "LENGTH", 2.0, 76.0 * font::ui_scale(), pal().accent);
    }
    // drag the line between two columns to share the room differently
    let vis: Vec<usize> = (0..3).filter(|&i| cw[i] > 0.0).collect();
    let mut bx = tx;
    for k in 0..vis.len().saturating_sub(1) {
        bx += cw[vis[k]];
        let hit = Rect::from_center_size(Pos2::new(bx - 6.0, cy), Vec2::new(12.0, rect.height()));
        let r =
            ui.interact(hit, ui.id().with(("col_split", k)), Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if r.hovered() || r.dragged() {
            fill_rect(&painter, Rect::from_center_size(hit.center(), Vec2::new(2.0, rect.height() - 4.0)), pal().accent);
        }
        if r.dragged() {
            let mut w = COL_W.lock().unwrap();
            let (a, b) = (vis[k], vis[k + 1]);
            let sum: f32 = vis.iter().map(|&i| w[i]).sum();
            let min = 0.1 * sum;
            let d = (r.drag_delta().x / avail * sum).clamp(min - w[a], w[b] - min);
            w[a] += d;
            w[b] -= d;
            COLS_CHANGED.store(true, Ordering::Relaxed);
        }
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Right-click to choose columns.").context_menu(|ui| {
        for (i, name) in COL_NAMES.iter().enumerate() {
            let mark = if col_on(i) { "[x] " } else { "[ ] " };
            if menu_item(ui, &format!("{}{}", mark, name)) {
                COLS.fetch_xor(1 << i, Ordering::Relaxed);
                COLS_CHANGED.store(true, Ordering::Relaxed);
            }
        }
        let on = row_arts_on();
        if menu_item(ui, if on { "[x] COVERS" } else { "[ ] COVERS" }) {
            ROW_ARTS.store(!on, Ordering::Relaxed);
            COLS_CHANGED.store(true, Ordering::Relaxed);
        }
    });
}

/// One track row's text, split into cells with tabs for `list_row`: then key and tempo (if known), then where the song
/// comes from (`kind`: tidal, file, yt, sc).
pub(crate) fn track_cells(id: i64, title: &str, artist: &str, album: &str, kind: &str) -> String {
    let i = meta::get(id).unwrap_or_default();
    let key = meta::key_text(&i);
    let bpm = i.bpm.map_or(String::new(), |b| b.to_string());
    format!("{}\t{}\t{}\t{}|{}\t{}", title, artist, album, key, bpm, source_label(kind))
}

/// Where a track comes from ("tidal", "file", "yt", "sc"), by what it plays from: a song not on Tidal is in `src`;
/// one that isn't (and isn't a Tidal song) says nothing.
pub(crate) fn track_kind(src: &HashMap<i64, crate::Src>, id: i64) -> &'static str {
    match src.get(&id) {
        Some(crate::Src::File(_)) => "file",
        Some(crate::Src::Yt(v)) if v.starts_with("http") => "sc",
        Some(crate::Src::Yt(_)) => "yt",
        None if id > 0 => "tidal",
        None => "",
    }
}

/// Where a song comes from, as the SOURCE column says it.
pub(crate) fn source_label(kind: &str) -> &'static str {
    match kind {
        "tidal" => "Tidal",
        "file" => "File",
        "sc" => "SoundCloud",
        "yt" => "YouTube",
        _ => "",
    }
}

// ------------------------------------------------------------------ page view
/// `own`: the id of your own Tidal playlist these songs are, for taking one out of it. `src`: where each song not on
/// Tidal plays from (the SOURCE column).
pub(crate) fn tracks_list(
    ui: &mut egui::Ui,
    tracks: &[Track],
    playing_id: Option<i64>,
    liked: &HashSet<i64>,
    own: Option<&str>,
    src: &HashMap<i64, crate::Src>,
    acts: &mut Vec<Action>,
) {
    // covers beside the songs (not on an album's own page: they would all be the same one)
    let one_album = tracks.len() > 1 && tracks.iter().all(|t| t.cover == tracks[0].cover);
    let art = row_arts_on() && !one_album;
    col_header(ui, true, art);
    // only the rows on screen (and a screen either side, so covers are asked for ahead) do any work; the rest are
    // one stretch of room each, above and below (thousands of songs cost nothing off screen)
    let (first, last) = row_window(ui, tracks.len(), art);
    for (i, t) in tracks.iter().enumerate().take(last).skip(first) {
        // (a song Tidal can't stream any more, with no copy here: greyed, and it doesn't play)
        let gone = t.gone && !cache::has(t.id);
        let state = if playing_id == Some(t.id) {
            RowState::Playing
        } else if gone {
            RowState::Played
        } else {
            RowState::Normal
        };
        row_heart(liked.contains(&t.id));
        row_stems(t.id);
        if art {
            row_art(&t.cover);
        }
        let r = list_row(
            ui,
            i,
            Some(i + 1),
            || track_cells(t.id, &t.title, &t.artists_text(), &t.album, track_kind(src, t.id)),
            || if col_on(2) { fmt_time(t.duration) } else { String::new() },
            state,
            false,
            cache::has(t.id),
        );
        if heart_clicked() {
            acts.push(Action::ToggleLike(t.clone()));
        }
        if let Some(r) = r {
            let r = if gone { r.on_hover_text("No longer available on Tidal.") } else { r };
            if r.clicked() && !gone {
                acts.push(Action::Play(tracks.to_vec(), i));
            }
            r.context_menu(|ui| {
                song_items(ui, acts, t, liked.contains(&t.id), true);
                link_item(ui, acts, track_link(t, None));
                if let Some(uuid) = own {
                    if menu_item(ui, "Remove from This Playlist") {
                        // (which time it is there: a song can be in a playlist twice)
                        let nth = tracks[..i].iter().filter(|x| x.id == t.id).count();
                        acts.push(Action::TidalListRemove(uuid.to_string(), t.id, nth));
                        ui.close_menu();
                    }
                }
            });
        }
    }
    row_room(ui, tracks.len() - last, art);
}

/// The rows of a long list worth drawing (on screen, or a screen either side), as (first, past the last); the room
/// of the rows above them is taken here (`row_room` takes the rest's, after them). Rows are all one height.
pub(crate) fn row_window(ui: &mut egui::Ui, n: usize, art: bool) -> (usize, usize) {
    let h = row_height(ui, art);
    let clip = ui.clip_rect();
    let top = ui.cursor().min.y;
    let ahead = clip.height().max(200.0);
    let first = (((clip.min.y - ahead - top) / h).floor().max(0.0) as usize).min(n);
    let last = ((((clip.max.y + ahead) - top) / h).ceil().max(0.0) as usize).clamp(first, n);
    row_room(ui, first, art);
    (first, last)
}

/// Room for `k` rows not drawn (flush, as `list_row` keeps them).
pub(crate) fn row_room(ui: &mut egui::Ui, k: usize, art: bool) {
    if k == 0 {
        return;
    }
    let h = row_height(ui, art);
    ui.allocate_space(Vec2::new(ui.available_width(), k as f32 * h));
    let gap = ui.spacing().item_spacing.y;
    if gap > 0.0 {
        ui.add_space(-gap);
    }
}

/// A track row's height, as `list_row` makes it (taller with a cover, when the cover fits the list's width).
pub(crate) fn row_height(ui: &egui::Ui, art: bool) -> f32 {
    let w = ui.available_width();
    let covered = art && row_fit(w, true, true).art;
    (if covered { ROW_ART_H } else { ROW_H }) * font::ui_scale()
}

/// The public link for a track: Tidal by id, YouTube / SoundCloud from the saved source. None for files.
pub(crate) fn track_link(t: &Track, ext: Option<&store::Ext>) -> Option<String> {
    if t.id > 0 {
        return Some(format!("https://tidal.com/browse/track/{}", t.id));
    }
    let e = ext?;
    if e.kind == "file" || e.src.is_empty() {
        return None;
    }
    Some(if e.src.starts_with("http") { e.src.clone() } else { format!("https://www.youtube.com/watch?v={}", e.src) })
}

/// What a song's right-click menu offers wherever the song is shown: play it next or later, go to its album or
/// artist, keep it in My Tracks, put it in a tune or a playlist. `queued`: it is in the queue already.
pub(crate) fn song_items(ui: &mut egui::Ui, acts: &mut Vec<Action>, t: &Track, liked: bool, tidal: bool) {
    // a new copy of the song each time, even from the queue itself
    if menu_item(ui, "Play Next") {
        acts.push(Action::PlayNext(t.clone()));
        ui.close_menu();
    }
    if menu_item(ui, "Add to Queue") {
        acts.push(Action::Enqueue(t.clone()));
        ui.close_menu();
    }
    goto_items(ui, acts, t, tidal);
    if menu_item(ui, if liked { "Remove from My Tracks" } else { "Add to My Tracks" }) {
        acts.push(Action::ToggleLike(t.clone()));
        ui.close_menu();
    }
    if PRACTICE && menu_item(ui, "Add to a Tune...") {
        acts.push(Action::PickTune(t.clone()));
        ui.close_menu();
    }
    // (a window of its own, to find one among many: you stay where you are)
    if menu_item(ui, "Add to Playlist...") {
        acts.push(Action::PlaylistPicker(t.clone(), None));
        ui.close_menu();
    }
    if menu_item(ui, "Copy Name") {
        let who = t.artists_text();
        acts.push(Action::CopyName(if who.is_empty() { t.title.clone() } else { format!("{} - {}", who, t.title) }));
        ui.close_menu();
    }
    // (a file of yours, or a streamed song's saved copy: nothing to delete otherwise)
    if on_disk(t.id) && menu_item(ui, "Delete from This Computer") {
        acts.push(Action::DeleteSong(t.clone()));
        ui.close_menu();
    }
}

/// What free-standing menus need to know of your Tidal playlists: whether you're logged in (and online), and your
/// own playlists (id, name; None until they're loaded, which a menu asks for in `want_tidal`). Kept in step with the
/// app at the start of each frame.
#[derive(Default)]
pub(crate) struct PlMenu {
    pub tidal_on: bool,
    pub tidal: Option<Vec<(String, String)>>,
    pub want_tidal: bool,
}

thread_local! {
    pub(crate) static PL_MENU: std::cell::RefCell<PlMenu> = std::cell::RefCell::new(PlMenu::default());
}

/// Menu entry that shows the link and copies it. Nothing is shown when there is no link (local files).
pub(crate) fn link_item(ui: &mut egui::Ui, acts: &mut Vec<Action>, link: Option<String>) {
    if let Some(l) = link {
        let short: String =
            if l.chars().count() > 46 { format!("{}...", l.chars().take(44).collect::<String>()) } else { l.clone() };
        if menu_item(ui, &format!("Copy Link   {}", short.trim_start_matches("https://"))) {
            acts.push(Action::CopyLink(l));
            ui.close_menu();
        }
    }
}

/// `head`: under a `cards_header` with the covers switch (the rows keep its room when covers are off).
pub(crate) fn cards_list(ui: &mut egui::Ui, cards: &[Card], tags: bool, head: bool, acts: &mut Vec<Action>) {
    for (i, c) in cards.iter().enumerate() {
        // its cover at the left, as the songs have (covers on)
        row_art(&c.image);
        if head {
            ROW_GUTTER.with(|g| g.set(true));
        }
        let r = list_row(
            ui,
            i,
            None,
            || {
                if tags {
                    let tag = match c.kind {
                        Kind::Album => "ALBUM",
                        Kind::Playlist => "LIST",
                        Kind::Artist => "ARTIST",
                        Kind::Mix => "MIX",
                    };
                    format!("[{}]  {}", tag, c.title)
                } else {
                    c.title.clone()
                }
            },
            || c.subtitle.clone(),
            RowState::Normal,
            false,
            false,
        );
        if let Some(r) = r {
            if r.clicked() {
                acts.push(Action::Open(c.clone()));
            }
            // an album, playlist or mix: all its songs into the queue, next or at the end
            let songs = c.kind != Kind::Artist;
            if songs || !c.image.is_empty() {
                r.context_menu(|ui| {
                    if songs && menu_item(ui, "Play Next") {
                        acts.push(Action::QueueCard(c.clone(), true));
                        ui.close_menu();
                    }
                    if songs && menu_item(ui, "Add to Queue") {
                        acts.push(Action::QueueCard(c.clone(), false));
                        ui.close_menu();
                    }
                    if !c.image.is_empty() && menu_item(ui, "View Art") {
                        acts.push(Action::ArtPreview(c.image.clone(), c.title.clone(), c.kind == Kind::Artist));
                        ui.close_menu();
                    }
                    // one of your own Tidal playlists (known once your playlists are in): it can be renamed
                    if c.kind == Kind::Playlist {
                        let own = PL_MENU.with(|m| {
                            let mut m = m.borrow_mut();
                            m.want_tidal |= m.tidal.is_none() && m.tidal_on;
                            m.tidal.as_ref().is_some_and(|l| l.iter().any(|(u, _)| *u == c.id))
                        });
                        if own && menu_item(ui, "Rename...") {
                            acts.push(Action::TidalListRename(c.id.clone(), c.title.clone()));
                            ui.close_menu();
                        }
                    }
                });
            }
        }
    }
}

/// PLAY and SHUFFLE for a list; `refresh`: and REFRESH, to load it from Tidal again (your library).
pub(crate) fn play_buttons(ui: &mut egui::Ui, tracks: &[Track], acts: &mut Vec<Action>, refresh: bool) {
    play_buttons_save(ui, tracks, acts, refresh, None);
}

/// "Save as a Tidal playlist..." and "Save as a Backline playlist...": these songs, under `name` (which can be
/// changed before it's made). Tidal only for songs that are all Tidal's, when you're logged in.
pub(crate) fn save_items(ui: &mut egui::Ui, acts: &mut Vec<Action>, tracks: &[Track], name: &str) {
    let tidal = !tracks.is_empty() && tracks.iter().all(|t| t.id > 0) && PL_MENU.with(|m| m.borrow().tidal_on);
    if tidal && menu_item(ui, "Save as a Tidal Playlist...") {
        acts.push(Action::PlaylistNewFrom(name.to_string(), tracks.to_vec(), true));
    }
    if menu_item(ui, "Save as a Backline Playlist...") {
        acts.push(Action::PlaylistNewFrom(name.to_string(), tracks.to_vec(), false));
    }
}

/// The same, and (`save`: the list's name) a way to keep these songs as a playlist of your own.
pub(crate) fn play_buttons_save(ui: &mut egui::Ui, tracks: &[Track], acts: &mut Vec<Action>, refresh: bool, save: Option<&str>) {
    if winamp_ui::chrome_on() {
        play_icons(ui, tracks, acts, refresh, save);
        return;
    }
    ui.horizontal(|ui| {
        if ibtn(ui, &IC_PLAY, "PLAY", false).clicked() {
            acts.push(Action::Play(tracks.to_vec(), 0));
        }
        if ibtn(ui, &IC_SHUF, "SHUFFLE", false).clicked() {
            acts.push(Action::PlayShuffled(tracks.to_vec()));
        }
        if retro_btn(ui, "PLAY ALL NEXT", false).tip("Plays all of these right after the current song.").clicked() {
            acts.push(Action::QueueTracks("THIS LIST".into(), tracks.to_vec(), true));
        }
        if retro_btn(ui, "ADD ALL TO QUEUE", false).tip("Adds all of these to the end of the queue.").clicked() {
            acts.push(Action::QueueTracks("THIS LIST".into(), tracks.to_vec(), false));
        }
        if refresh && ibtn(ui, &IC_REP, "REFRESH", false).tip("Reload your library from Tidal.").clicked() {
            acts.push(Action::RefreshLibrary);
        }
        if let Some(name) = save {
            let b = retro_btn(ui, "SAVE AS PLAYLIST...", false).tip("Keep these songs as a playlist, on Tidal or in Backline.");
            let id = ui.id().with("list_save");
            if b.clicked() {
                ui.memory_mut(|m| m.toggle_popup(id));
            }
            egui::popup_below_widget(ui, id, &b, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(200.0 * font::ui_scale());
                save_items(ui, acts, tracks, name);
            });
        }
    });
}

/// The same, a skin worn: a big round PLAY, SHUFFLE and ADD TO QUEUE as icons (names on hover), and "..." for the
/// rest.
pub(crate) fn play_icons(ui: &mut egui::Ui, tracks: &[Track], acts: &mut Vec<Action>, refresh: bool, save: Option<&str>) {
    let sc = font::ui_scale();
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        // PLAY: a circle in the accent colour
        let d = 40.0 * sc;
        let (r, resp) = ui.allocate_exact_size(Vec2::splat(d), Sense::click());
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Play");
        let grow = if resp.hovered() { 1.0 } else { 0.0 };
        ui.painter().circle_filled(r.center(), d / 2.0 + grow, pal().bar_txt);
        vicon::named(
            ui.painter(),
            Rect::from_center_size(r.center() + Vec2::new(1.5, 0.0), Vec2::splat(d * 0.42)),
            "play",
            pal().display,
        );
        if resp.clicked() {
            acts.push(Action::Play(tracks.to_vec(), 0));
        }
        ui.add_space(6.0);
        let icon = |ui: &mut egui::Ui, name: &str, tip: &str| -> egui::Response {
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(34.0 * sc), Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.hovered() {
                ui.painter().circle_filled(r.center(), r.width() / 2.0, Color32::from_white_alpha(12));
            }
            let col = if resp.hovered() { pal().ink } else { pal().ink2 };
            vicon::named(ui.painter(), r.shrink(8.0 * sc), name, col);
            resp.tip(tip)
        };
        if icon(ui, "shuffle", "Shuffle").clicked() {
            acts.push(Action::PlayShuffled(tracks.to_vec()));
        }
        // (all of them: one song goes in from its own right-click menu)
        let all_n = format!("Add all {} to the queue", commas(tracks.len()));
        if icon(ui, "queue", &all_n).clicked() {
            acts.push(Action::QueueTracks("THIS LIST".into(), tracks.to_vec(), false));
        }
        // save as a playlist: a plus in a ring, its two choices under it (on Tidal, in Backline)
        if let Some(name) = save {
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(34.0 * sc), Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.hovered() {
                ui.painter().circle_filled(r.center(), r.width() / 2.0, Color32::from_white_alpha(12));
            }
            let col = if resp.hovered() { pal().ink } else { pal().ink2 };
            let (c, rad, s) = (r.center(), 8.5 * sc, Stroke::new(1.6 * sc, col));
            ui.painter().circle_stroke(c, rad, s);
            ui.painter().line_segment([c - Vec2::new(rad * 0.5, 0.0), c + Vec2::new(rad * 0.5, 0.0)], s);
            ui.painter().line_segment([c - Vec2::new(0.0, rad * 0.5), c + Vec2::new(0.0, rad * 0.5)], s);
            let resp = resp.tip("Save as a playlist, on Tidal or in Backline");
            let id = ui.id().with("list_save");
            if resp.clicked() {
                ui.memory_mut(|m| m.toggle_popup(id));
            }
            egui::popup_below_widget(ui, id, &resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(200.0 * sc);
                save_items(ui, acts, tracks, name);
            });
        }
        let more = icon(ui, "more", "More");
        let id = ui.id().with("list_more");
        if more.clicked() {
            ui.memory_mut(|m| m.toggle_popup(id));
        }
        egui::popup_below_widget(ui, id, &more, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(170.0 * sc);
            if menu_item(ui, "Play All Next") {
                acts.push(Action::QueueTracks("THIS LIST".into(), tracks.to_vec(), true));
            }
            if menu_item(ui, "Add All to Queue") {
                acts.push(Action::QueueTracks("THIS LIST".into(), tracks.to_vec(), false));
            }
            if refresh && menu_item(ui, "Refresh from Tidal") {
                acts.push(Action::RefreshLibrary);
            }
            if let Some(name) = save {
                save_items(ui, acts, tracks, name);
            }
        });
    });
    ui.add_space(4.0);
}

pub(crate) fn page_view(
    ui: &mut egui::Ui,
    images: &mut Images,
    page: &Page,
    playing_id: Option<i64>,
    tab: Tab,
    liked: &HashSet<i64>,
    filter: &str,
    src: &HashMap<i64, crate::Src>,
    acts: &mut Vec<Action>,
) {
    ui.add_space(6.0);

    // ---- "Your Library": one tab at a time, MY TRACKS first
    if page.library {
        match tab {
            Tab::Tracks => {
                title_line(ui, "MY TRACKS", 3.0, pal().ink);
                // filtered by the words typed above (title, artist or album)
                // (offline mode: only the songs on this computer)
                let here: Vec<Track>;
                let mut liked_total = page.total.max(page.tracks.len());
                let all: &[Track] = if crate::api::OFFLINE.load(Ordering::Relaxed) {
                    // every liked song stored on this computer, also the ones the library hadn't loaded yet (from the
                    // list of liked songs kept from last time, read once), then every other Tidal song stored here
                    // (saved from a playlist, an album, a search: their names kept as they were played)
                    thread_local! {
                        static SAVED: std::cell::OnceCell<Vec<Track>> = const { std::cell::OnceCell::new() };
                    }
                    let saved = SAVED.with(|s| s.get_or_init(crate::api::load_liked).clone());
                    // (Tidal songs only: YouTube and SoundCloud copies have their own lists)
                    let others: Vec<Track> = cache::tagged().into_iter().filter(|t| t.id > 0).collect();
                    liked_total = liked_total.max(saved.len());
                    let mut seen: HashSet<i64> = HashSet::new();
                    here = page
                        .tracks
                        .iter()
                        .chain(saved.iter())
                        .chain(others.iter())
                        .filter(|t| cache::has(t.id) && seen.insert(t.id))
                        .cloned()
                        .collect();
                    &here
                } else {
                    &page.tracks
                };
                let shown: Vec<Track>;
                let tracks: &[Track] = if filter.trim().is_empty() {
                    all
                } else {
                    shown = filtered(filter, all, |t| vec![t.title.clone(), t.artists_text(), t.album.clone()], 1)
                        .into_iter()
                        .map(|i| all[i].clone())
                        .collect();
                    &shown
                };
                let sub = if !filter.trim().is_empty() {
                    format!("{} of {} songs match", tracks.len(), all.len())
                } else if crate::api::OFFLINE.load(Ordering::Relaxed) {
                    format!("Offline: {} of {} songs are on this computer", commas(all.len()), commas(liked_total))
                } else if page.total > page.tracks.len() && LIB_LOAD.load(Ordering::Relaxed) == 0 {
                    format!("Loading your songs: {} of {}", commas(page.tracks.len()), commas(page.total))
                } else {
                    format!("{} songs", commas(page.tracks.len()))
                };
                title_line(ui, &sub, 2.0, pal().ink2);
                // all read: the songs Tidal can't play any more (greyed), and why there may be fewer than Tidal counts
                // (or that the reading stopped)
                if filter.trim().is_empty() && !crate::api::OFFLINE.load(Ordering::Relaxed) {
                    let state = LIB_LOAD.load(Ordering::Relaxed);
                    let gone = page.tracks.iter().filter(|t| t.gone && !cache::has(t.id)).count();
                    let gap = page.total.saturating_sub(page.tracks.len());
                    let mut why: Vec<String> = Vec::new();
                    if gone > 0 && state != 0 {
                        why.push(format!("{} no longer on Tidal, greyed out.", commas(gone)));
                    }
                    match state {
                        1 if gap > 0 => why.push(format!(
                            "Tidal counts {}: the other {} are videos or entries it can't play, so they're left out.",
                            commas(page.total),
                            commas(gap)
                        )),
                        2 if gap > 0 => why.push(format!("{} more couldn't be loaded just now. Refresh tries again.", commas(gap))),
                        _ => {}
                    }
                    if !why.is_empty() {
                        para(ui, &why.join(" "), pal().ink2);
                    }
                }
                ui.add_space(4.0);
                play_buttons(ui, tracks, acts, true);
                ui.add_space(8.0);
                if page.tracks.is_empty() {
                    title_line(ui, "No liked songs found.", 2.0, pal().ink2);
                } else if tracks.is_empty() {
                    title_line(ui, "No songs match.", 2.0, pal().ink2);
                }
                tracks_list(ui, tracks, playing_id, liked, None, src, acts);
            }
            t => {
                let (key, name) = match t {
                    Tab::Lists => ("Playlists", "PLAYLISTS"),
                    Tab::Albums => ("Albums", "ALBUMS"),
                    _ => ("Artists", "ARTISTS"),
                };
                title_line(ui, name, 3.0, pal().ink);
                ui.add_space(4.0);
                // the same library load brings these, so the same REFRESH
                if ibtn(ui, &IC_REP, "REFRESH", false).tip("Load your library from Tidal again.").clicked() {
                    acts.push(Action::RefreshLibrary);
                }
                ui.add_space(6.0);
                // new albums and singles from the artists you follow, above them
                if key == "Artists" && filter.trim().is_empty() {
                    if let Some((_, new)) = page.rows.iter().find(|r| r.0 == api::NEW_FROM_FOLLOWED) {
                        section_header(ui, "NEW FROM ARTISTS YOU FOLLOW");
                        cards_list(ui, &new[..new.len().min(8)], false, false, acts);
                        ui.add_space(6.0);
                        section_header(ui, "ARTISTS YOU FOLLOW");
                    }
                }
                let mut any = false;
                for (title, cards) in &page.rows {
                    if title == key {
                        any = true;
                        let shown: Vec<Card>;
                        let cards: &[Card] = if filter.trim().is_empty() {
                            cards
                        } else {
                            shown = filtered(filter, cards, |c| vec![c.title.clone(), c.subtitle.clone()], 2)
                                .into_iter()
                                .map(|i| cards[i].clone())
                                .collect();
                            &shown
                        };
                        title_line(ui, &format!("{} items", cards.len()), 2.0, pal().ink2);
                        ui.add_space(4.0);
                        cards_header(
                            ui,
                            match t {
                                Tab::Lists => "TRACKS",
                                Tab::Albums => "ARTIST",
                                _ => "",
                            },
                            true,
                        );
                        cards_list(ui, cards, false, true, acts);
                    }
                }
                if !any {
                    title_line(ui, "Nothing here yet.", 2.0, pal().ink2);
                }
            }
        }
        ui.add_space(20.0);
        return;
    }

    // ---- album / playlist / artist / search / home
    let has_header = !page.image.is_empty() || !page.subtitle.is_empty();
    if has_header {
        ui.horizontal(|ui| {
            // the page's picture, square and framed like the rest of the skin; click it to look at it big
            let (cr, cresp) = ui.allocate_exact_size(Vec2::splat(104.0), Sense::click());
            inset(ui.painter(), cr, pal().edge);
            paint_page_art(ui, images, &page.image, cr.shrink(2.0), false);
            if !page.image.is_empty()
                && cresp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Show the picture big.").clicked()
            {
                acts.push(Action::ArtPreview(page.image.clone(), page.title.clone(), page.artist));
            }
            ui.vertical(|ui| {
                title_line(ui, &page.title, 3.0, pal().ink);
                match &page.by {
                    // an album's artist: their name opens their page
                    Some(by) => {
                        let top = ui.cursor().min;
                        title_line(ui, &page.subtitle, 2.0, pal().ink2);
                        let w = crate::font::text_w(&page.subtitle, 2.0).min(ui.available_width());
                        let rect = Rect::from_min_max(top, Pos2::new(top.x + w, ui.cursor().min.y));
                        let r = ui.interact(rect, ui.id().with("page_by"), Sense::click());
                        if r.hovered() {
                            let y = r.rect.max.y - 1.0;
                            ui.painter().hline(r.rect.x_range(), y, egui::Stroke::new(1.0_f32, pal().ink2));
                        }
                        if r.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Go to the artist.").clicked() {
                            acts.push(Action::Open(by.clone()));
                        }
                    }
                    None => {
                        title_line(ui, &page.subtitle, 2.0, pal().ink2);
                    }
                }
                ui.add_space(6.0);
                // (a radio, a mix, an album: its songs can be kept as a playlist of yours, under its name or another)
                if !page.tracks.is_empty() {
                    play_buttons_save(ui, &page.tracks, acts, false, Some(&page.title));
                }
                // one of your own Tidal playlists: rename it
                if let Some(c) = page.card.as_ref().filter(|c| c.kind == Kind::Playlist && page.saved.is_none()) {
                    if retro_btn(ui, "RENAME", false).tip("Give this playlist a new name on Tidal.").clicked() {
                        acts.push(Action::TidalListRename(c.id.clone(), page.title.clone()));
                    }
                }
                // an album or someone else's playlist: keep it in your Tidal albums / playlists
                if let (Some(c), Some(saved)) = (&page.card, page.saved) {
                    let (on_txt, off_txt, tip) = if c.kind == Kind::Album {
                        ("IN MY ALBUMS", "+ ADD TO MY ALBUMS", "Keep this album in your albums on Tidal.")
                    } else {
                        ("IN MY PLAYLISTS", "+ ADD TO MY PLAYLISTS", "Keep this playlist in your playlists on Tidal.")
                    };
                    let tip = if saved { "In your Tidal library. Click to remove it." } else { tip };
                    if retro_btn(ui, if saved { on_txt } else { off_txt }, saved).tip(tip).clicked() {
                        acts.push(Action::SaveCard(c.clone(), !saved));
                    }
                }
                if page.artist && !page.artist_id.is_empty() {
                    ui.horizontal(|ui| {
                        let tip =
                            if page.following { "You follow them on Tidal. Click to unfollow." } else { "Follow them on Tidal." };
                        if retro_btn(ui, if page.following { "FOLLOWING" } else { "+ FOLLOW" }, page.following).tip(tip).clicked()
                        {
                            acts.push(Action::Follow(page.artist_id.clone(), !page.following));
                        }
                        if let Some(r) = &page.radio {
                            if retro_btn(ui, "RADIO", false).tip("Their radio: music like theirs.").clicked() {
                                acts.push(Action::Open(r.clone()));
                            }
                        }
                    });
                }
            });
        });
        if !page.bio.is_empty() {
            ui.add_space(4.0);
            bio_view(ui, &page.bio, &page.artist_id, acts);
        }
    } else {
        title_line(ui, &page.title, 3.0, pal().ink);
    }
    ui.add_space(6.0);

    if page.tracks.is_empty() && page.rows.is_empty() {
        title_line(ui, "Nothing to show.", 2.0, pal().ink2);
    }
    // songs first, as in any music player, then the albums / artists / playlists
    if !page.tracks.is_empty() {
        if !page.rows.is_empty() {
            section_header(ui, "SONGS");
        }
        // one of your own playlists (nothing to save): its songs can be taken out of it
        let own = page.card.as_ref().filter(|c| c.kind == Kind::Playlist && page.saved.is_none()).map(|c| c.id.as_str());
        tracks_list(ui, &page.tracks, playing_id, liked, own, src, acts);
    }
    for (title, cards) in &page.rows {
        section_header(ui, title);
        cards_list(ui, cards, true, false, acts);
    }
    ui.add_space(20.0);
}

/// An artist's bio: two lines, then VIEW MORE (underlined) for the rest. The artists and albums it names are links.
pub(crate) fn bio_view(ui: &mut egui::Ui, bits: &[api::BioBit], id: &str, acts: &mut Vec<Action>) {
    const SHORT: usize = 2;
    let key = egui::Id::new(("bio_open", id.to_string()));
    let open = ui.ctx().data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    let (px, lh) = (2.0, 18.0);
    // as wide as what can be seen (the panel can be wider than its visible part)
    let seen = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x);
    let w = (seen - 24.0).max(160.0);
    let space = text_w(" ", px);
    // the words laid out in lines: (x, line, word, link)
    let mut words: Vec<(f32, usize, &str, Option<&Card>)> = Vec::new();
    let (mut x, mut line) = (0.0_f32, 0usize);
    for b in bits {
        // a piece that runs straight on from the last (the comma after a link) gets no space before it
        if x > 0.0 && !b.text.starts_with(char::is_whitespace) {
            x -= space;
        }
        for (k, part) in b.text.split('\n').enumerate() {
            if k > 0 && x > 0.0 {
                line += 1;
                x = 0.0;
            }
            for word in part.split_whitespace() {
                let ww = text_w(word, px);
                if x > 0.0 && x + ww > w {
                    line += 1;
                    x = 0.0;
                }
                words.push((x, line, word, b.link.as_ref()));
                x += ww + space;
            }
        }
    }
    let lines = words.last().map_or(0, |w| w.1 + 1);
    let shown = if open { lines } else { lines.min(SHORT) };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w + 16.0, shown as f32 * lh), Sense::hover());
    let p = ui.painter().clone();
    for (i, (wx, wl, word, link)) in words.iter().enumerate() {
        if *wl >= shown {
            break;
        }
        let pos = Pos2::new(rect.min.x + 8.0 + wx, rect.min.y + (*wl as f32 + 0.5) * lh);
        let col = if link.is_some() { pal().bar_txt } else { pal().ink2 };
        let ww = ptext(&p, pos, Align::Min, word, px, col);
        if let Some(c) = link {
            let r = Rect::from_min_size(Pos2::new(pos.x, pos.y - lh / 2.0), Vec2::new(ww, lh));
            let resp =
                ui.interact(r, ui.id().with(("bio_link", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.hovered() {
                p.line_segment([Pos2::new(r.min.x, pos.y + 7.0), Pos2::new(r.max.x, pos.y + 7.0)], Stroke::new(1.0_f32, col));
            }
            if resp.clicked() {
                acts.push(Action::Open((*c).clone()));
            }
        }
    }
    if lines > SHORT {
        let label = if open { "VIEW LESS" } else { "VIEW MORE" };
        let tw = text_w(label, px);
        let (r, resp) = ui.allocate_exact_size(Vec2::new(tw + 16.0, lh), Sense::click());
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        let col = if resp.hovered() { pal().ink } else { pal().ink2 };
        let at = Pos2::new(r.min.x + 8.0, r.center().y);
        ptext(&p, at, Align::Min, label, px, col);
        p.line_segment([Pos2::new(at.x, at.y + 7.0), Pos2::new(at.x + tw, at.y + 7.0)], Stroke::new(1.0_f32, col));
        // (set after the frame, so a page drawn twice in one frame cannot open and shut it again)
        if resp.clicked() {
            acts.push(Action::BioOpen(id.to_string(), !open));
        }
    }
    ui.add_space(4.0);
}

pub(crate) fn push_tracks(p: &mut Arc<Page>, v: &[Track]) {
    let pg = Arc::make_mut(p);
    let have: HashSet<i64> = pg.tracks.iter().map(|t| t.id).collect();
    pg.tracks.extend(v.iter().filter(|t| !have.contains(&t.id)).cloned());
}
