//! DIARY: the day's notes first (the journal, flipping day to day, ALL NOTES), the totals for a period with their
//! bars, the calendar with stars and tags, and a song's history.

use super::*;

// ------------------------------------------------------------ the diary's summaries

/// A starred loop's star.
pub(crate) const STAR_COL: Color32 = Color32::from_rgb(232, 190, 70);

/// The colours of the top five songs in a summary (the rest is OTHER, in the dim ink), and of reading.
pub(crate) const SONG_COLS: [Color32; 5] = [
    Color32::from_rgb(217, 164, 65),
    Color32::from_rgb(95, 168, 211),
    Color32::from_rgb(214, 104, 83),
    Color32::from_rgb(111, 191, 115),
    Color32::from_rgb(167, 125, 194),
];
pub(crate) const READING_COL: Color32 = Color32::from_rgb(80, 190, 180);

/// A stretch of days for a summary: its name and its columns (a day each, or a month each for a year).
pub(crate) struct Period {
    pub(crate) name: String,
    pub(crate) cols: Vec<(String, Vec<i64>)>,
}

impl Period {
    /// The week (Monday to Sunday), month or year around a day.
    pub(crate) fn around(day: i64, span: u8) -> Period {
        match span {
            0 => {
                let mon = day - (day + 3).rem_euclid(7);
                const D: [&str; 7] = ["M", "T", "W", "T", "F", "S", "S"];
                Period {
                    name: format!("WEEK OF {}", store::date_str(mon)),
                    cols: (0..7).map(|k| (D[k as usize].to_string(), vec![mon + k])).collect(),
                }
            }
            1 => {
                let (y, m, _) = store::civil_from_days(day);
                let first = store::days_from_civil(y, m, 1);
                let next = if m == 12 { store::days_from_civil(y + 1, 1, 1) } else { store::days_from_civil(y, m + 1, 1) };
                Period {
                    name: format!("{:04}-{:02}", y, m),
                    cols: (first..next)
                        .map(|d| {
                            let n = d - first + 1;
                            (if n == 1 || n % 5 == 0 { n.to_string() } else { String::new() }, vec![d])
                        })
                        .collect(),
                }
            }
            _ => {
                let (y, _, _) = store::civil_from_days(day);
                const M: [&str; 12] = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"];
                Period {
                    name: format!("{}", y),
                    cols: (1..=12u32)
                        .map(|m| {
                            let first = store::days_from_civil(y, m, 1);
                            let next =
                                if m == 12 { store::days_from_civil(y + 1, 1, 1) } else { store::days_from_civil(y, m + 1, 1) };
                            (M[m as usize - 1].to_string(), (first..next).collect())
                        })
                        .collect(),
                }
            }
        }
    }
}

/// One column of a summary: its label, its days, its time.
pub(crate) struct Col {
    pub(crate) label: String,
    pub(crate) days: Vec<i64>,
    pub(crate) total: u32,
}

/// What a stretch of days adds up to.
pub(crate) struct Summary {
    pub(crate) cols: Vec<Col>,
    pub(crate) total: u32,
    pub(crate) tunes: u32,
    pub(crate) free: u32,
    pub(crate) reading: u32,
    /// the songs, most time first
    pub(crate) songs: Vec<(i64, u32)>,
    /// the loops (song, start, end, seconds, passes), most time first
    pub(crate) loops: Vec<(i64, f32, f32, u32, u32)>,
    pub(crate) days_practised: usize,
    pub(crate) days_so_far: usize,
    pub(crate) with_reading: bool,
}

impl Summary {
    pub(crate) fn of(app: &App, period: &Period, with_reading: bool) -> Summary {
        let today = store::today_days();
        let day_of = |d: i64| app.store.days.iter().find(|x| x.date == store::date_str(d));
        let read_secs = |d: &store::Day| d.secs.get("Sight reading").copied().unwrap_or(0);
        let day_total = |d: &store::Day| {
            let all: u32 = d.secs.values().sum();
            if with_reading {
                all
            } else {
                all - read_secs(d)
            }
        };
        let (mut tunes, mut free, mut reading) = (0, 0, 0);
        let mut songs: std::collections::HashMap<i64, u32> = Default::default();
        let mut loops: std::collections::HashMap<String, (u32, u32)> = Default::default();
        let (mut practised, mut so_far) = (0, 0);
        let mut cols = Vec::new();
        for (label, days) in &period.cols {
            let mut total = 0;
            for &d in days {
                if d <= today {
                    so_far += 1;
                }
                let Some(day) = day_of(d) else { continue };
                let t = day_total(day);
                total += t;
                if t > 0 {
                    practised += 1;
                }
                for (k, s) in &day.secs {
                    match k.as_str() {
                        "Sight reading" => reading += s,
                        "Free practice" => free += s,
                        _ => tunes += s,
                    }
                }
                for (id, s) in &day.songs {
                    *songs.entry(*id).or_insert(0) += s;
                }
                for (k, (s, n)) in &day.loop_time {
                    let e = loops.entry(k.clone()).or_insert((0, 0));
                    e.0 += s;
                    e.1 += n;
                }
            }
            cols.push(Col { label: label.clone(), days: days.clone(), total });
        }
        if !with_reading {
            reading = 0;
        }
        let mut songs: Vec<(i64, u32)> = songs.into_iter().filter(|s| s.1 > 0).collect();
        // (ties in a fixed order, so the lists never swap places between frames)
        songs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut loops: Vec<(i64, f32, f32, u32, u32)> = loops
            .into_iter()
            .filter_map(|(k, (s, n))| store::parse_loop_key(&k).map(|(id, a, b)| (id, a, b, s, n)))
            .filter(|l| l.3 > 0)
            .collect();
        loops.sort_by(|a, b| b.3.cmp(&a.3).then(a.0.cmp(&b.0)).then(a.1.total_cmp(&b.1)).then(a.2.total_cmp(&b.2)));
        let total = cols.iter().map(|c| c.total).sum();
        Summary {
            cols,
            total,
            tunes,
            free,
            reading,
            songs,
            loops,
            days_practised: practised,
            days_so_far: so_far.max(1),
            with_reading,
        }
    }

    /// A song's colour here: one of five for the top songs, the dim ink for the rest.
    pub(crate) fn colour(&self, id: i64) -> Color32 {
        self.songs.iter().take(5).position(|s| s.0 == id).map_or(pal().dim, |i| SONG_COLS[i])
    }

    /// Some days as (name, seconds, colour) pieces: the top songs, OTHER (the rest, and time with no song), and
    /// reading.
    pub(crate) fn segments_for(&self, app: &App, days: &[i64]) -> Vec<(String, u32, Color32)> {
        let mut top: Vec<(String, u32, Color32)> = Vec::new();
        let (mut other, mut reading) = (0u32, 0u32);
        let mut per: std::collections::HashMap<i64, u32> = Default::default();
        for &d in days {
            let Some(day) = app.store.days.iter().find(|x| x.date == store::date_str(d)) else { continue };
            let read = day.secs.get("Sight reading").copied().unwrap_or(0);
            let practice: u32 = day.secs.values().sum::<u32>() - read;
            let with_song: u32 = day.songs.values().sum();
            other += practice.saturating_sub(with_song);
            if self.with_reading {
                reading += read;
            }
            for (id, s) in &day.songs {
                *per.entry(*id).or_insert(0) += s;
            }
        }
        for (i, (id, _)) in self.songs.iter().take(5).enumerate() {
            if let Some(s) = per.remove(id).filter(|s| *s > 0) {
                top.push((app.song_name(*id), s, SONG_COLS[i]));
            }
        }
        other += per.values().sum::<u32>();
        if other > 0 {
            top.push(("OTHER".to_string(), other, pal().dim));
        }
        if reading > 0 {
            top.push(("READING".to_string(), reading, READING_COL));
        }
        top
    }
}

/// The width there is to see from here (a list wider than the page never pushes these past its edge).
pub(crate) fn seen_w(ui: &egui::Ui) -> f32 {
    ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 4.0).max(60.0)
}

/// A bar the width of the page, split into its pieces by time.
pub(crate) fn stacked_bar(ui: &mut egui::Ui, segs: &[(String, u32, Color32)], h: f32) {
    let w = seen_w(ui);
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let total: u32 = segs.iter().map(|s| s.1).sum::<u32>().max(1);
    let mut x = r.min.x;
    for (_, s, c) in segs {
        let sw = *s as f32 / total as f32 * r.width();
        ui.painter().rect_filled(Rect::from_min_max(Pos2::new(x, r.min.y), Pos2::new(x + sw, r.max.y)), 0.0, *c);
        x += sw;
    }
}

/// The pieces of a bar named under it: a swatch, the name, the time.
pub(crate) fn legend(ui: &mut egui::Ui, segs: &[(String, u32, Color32)]) {
    // laid out by hand, a line at a time, so nothing runs past the page's edge
    let w = seen_w(ui);
    let items: Vec<(String, Color32)> =
        segs.iter().map(|(n, s, c)| (format!("{}  {}", clip_text(n, 1.0, 220.0), mins_text(*s)), *c)).collect();
    let mut lines: Vec<Vec<(String, Color32)>> = vec![Vec::new()];
    let mut x = 0.0;
    for (t, c) in items {
        let iw = 16.0 + text_w(&t, 1.0) + 14.0;
        if x + iw > w && !lines.last().is_some_and(|l| l.is_empty()) {
            lines.push(Vec::new());
            x = 0.0;
        }
        x += iw;
        if let Some(l) = lines.last_mut() {
            l.push((t, c));
        }
    }
    for line in lines {
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 18.0), Sense::hover());
        let p = ui.painter();
        let mut x = r.min.x;
        for (t, c) in line {
            p.rect_filled(Rect::from_center_size(Pos2::new(x + 5.0, r.center().y), Vec2::splat(10.0)), 1.0, c);
            ptext(p, Pos2::new(x + 16.0, r.center().y), Align::Min, &t, 1.0, pal().ink2);
            x += 16.0 + text_w(&t, 1.0) + 14.0;
        }
    }
}

/// The text cut to fit `max_w` (with ... at the end), at one size - not shrunk.
pub(crate) fn clip_text(text: &str, px: f32, max_w: f32) -> String {
    if text_w(text, px) <= max_w {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let t: String = chars.iter().collect::<String>().trim_end().to_string() + "...";
        if text_w(&t, px) <= max_w {
            return t;
        }
    }
    String::new()
}

/// A row with a name, a bar as long as its share of the longest, and the time on the right.
pub(crate) fn bar_row(ui: &mut egui::Ui, name: &str, secs: u32, most: u32, col: Color32, right: &str) {
    let w = seen_w(ui);
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::hover());
    let p = ui.painter();
    let name_w = (r.width() * 0.45).min(320.0);
    let right_w = text_w(right, 1.0) + 8.0;
    ptext(p, Pos2::new(r.min.x, r.center().y), Align::Min, &clip_text(name, 1.0, name_w - 8.0), 1.0, pal().ink);
    let bar =
        Rect::from_min_max(Pos2::new(r.min.x + name_w, r.center().y - 4.0), Pos2::new(r.max.x - right_w, r.center().y + 4.0));
    p.rect_filled(bar, 0.0, pal().dim.gamma_multiply(0.25));
    let f = secs as f32 / most.max(1) as f32;
    p.rect_filled(Rect::from_min_size(bar.min, Vec2::new(bar.width() * f, bar.height())), 0.0, col);
    ptext(p, Pos2::new(r.max.x, r.center().y), Align::Max, right, 1.0, pal().ink2);
}

/// How many days' notes ALL NOTES shows at a time.
pub(crate) const SKIM: usize = 30;

/// The days with notes written, in order (days since 1970).
pub(crate) fn noted_days(days: &[store::Day]) -> Vec<i64> {
    let mut v: Vec<i64> = days.iter().filter(|d| !d.note.trim().is_empty()).filter_map(|d| store::parse_date(&d.date)).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// The day a year, a month (`mode` 0, 1) or a day (2) before (`dir` -1) or after `d`: the same day of the month,
/// or the month's last when it's shorter (Jan 31 to Feb 28).
pub(crate) fn period_step(d: i64, mode: u8, dir: i8) -> i64 {
    let (y, m, dd) = store::civil_from_days(d);
    let (y, m) = match mode {
        0 => (y + dir as i32, m),
        1 => {
            let k = y * 12 + (m as i32 - 1) + dir as i32;
            (k.div_euclid(12), (k.rem_euclid(12) + 1) as u32)
        }
        _ => return d + dir as i64,
    };
    let next = if m == 12 { store::days_from_civil(y + 1, 1, 1) } else { store::days_from_civil(y, m + 1, 1) };
    let last = (next - store::days_from_civil(y, m, 1)) as u32;
    store::days_from_civil(y, m, dd.min(last))
}

/// The nearest day with notes after `from` (`later`), or before it.
pub(crate) fn note_step(noted: &[i64], from: i64, later: bool) -> Option<i64> {
    if later {
        noted.iter().copied().find(|&d| d > from)
    } else {
        noted.iter().copied().rev().find(|&d| d < from)
    }
}

/// The start of a note on one line, for the calendar's hover.
pub(crate) fn note_preview(note: &str) -> String {
    let flat = crate::notes_fmt::plain(note);
    match flat.char_indices().nth(160) {
        Some((i, _)) => format!("{}...", flat[..i].trim_end()),
        None => flat,
    }
}

/// Slide, flipping to the next day with notes (`later`) or back: over the notes box (`rect`) the old day's words slide
/// out and fade as the new day's slide in, formatted as they read; `t` 0..1.
pub(crate) fn slide_notes(p: &egui::Painter, rect: Rect, px: f32, old: &str, new: &str, t: f32, later: bool) {
    let face = rect.shrink(2.0);
    let inner = rect.shrink2(Vec2::new(8.0, 3.0));
    // the box's face again, over the words drawn there
    p.rect_filled(face, 0.0, pal().lcd);
    let cp = p.with_clip_rect(face);
    let dx = if later { -40.0 } else { 40.0 };
    let out = (t * 1.25).min(1.0).powi(2);
    let came = 1.0 - (1.0 - t).powi(3);
    let w = inner.width();
    if out < 1.0 {
        draw_notes(&cp, inner.min + Vec2::new(dx * out, 0.0), w, &lay_notes(old, px, w, &[]), 1.0 - out, None, "");
    }
    draw_notes(&cp, inner.min + Vec2::new(-dx * (1.0 - came), 0.0), w, &lay_notes(new, px, w, &[]), came, None, "");
}

/// Room before the formatting tools on a row, so they end it.
pub(crate) fn tools_at_end(ui: &mut egui::Ui) {
    let n = crate::notes_fmt::TOOLS.len() as f32;
    let tw = n * TOOL_W * crate::font::ui_scale() + (n - 1.0) * 2.0 + 8.0;
    let room = ui.available_size_before_wrap().x - tw;
    if room > 0.0 {
        ui.add_space(room);
    }
}

/// A little pencil (a calendar day with notes): `at` its top-left corner, `size` across.
pub(crate) fn pencil(p: &egui::Painter, at: Pos2, size: f32, col: Color32) {
    let k = size / 16.0;
    let pt = |x: f32, y: f32| Pos2::new(at.x + x * k, at.y + y * k);
    let stroke = Stroke::new(1.3f32, col);
    p.add(egui::Shape::closed_line(vec![pt(3.0, 13.0), pt(4.0, 9.5), pt(11.5, 2.0), pt(14.0, 4.5), pt(6.5, 12.0)], stroke));
    p.line_segment([pt(10.0, 3.5), pt(12.5, 6.0)], stroke);
    p.line_segment([pt(3.0, 13.0), pt(6.5, 12.0)], stroke);
}

/// A #tag's chip: room each side of its word.
pub(crate) const TAG_PAD: f32 = 6.0;

/// Stars on days that clicked.
pub(crate) const GOLD: Color32 = Color32::from_rgb(232, 184, 74);

/// The search marked in notes.
pub(crate) const MARK: Color32 = Color32::from_rgba_premultiplied(110, 86, 34, 110);

/// Months, for dates read out.
pub(crate) const MONTH_NAMES: [&str; 12] =
    ["JANUARY", "FEBRUARY", "MARCH", "APRIL", "MAY", "JUNE", "JULY", "AUGUST", "SEPTEMBER", "OCTOBER", "NOVEMBER", "DECEMBER"];

/// A five-pointed star at `c`, `r` to its points: filled, or drawn round its edge.
pub(crate) fn star(p: &egui::Painter, c: Pos2, r: f32, filled: bool, col: Color32) {
    let pts: Vec<Pos2> = (0..10)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
            c + Vec2::angled(a) * if k % 2 == 0 { r } else { r * 0.45 }
        })
        .collect();
    if filled {
        let mut m = egui::Mesh::default();
        m.colored_vertex(c, col);
        for q in &pts {
            m.colored_vertex(*q, col);
        }
        for k in 0..10u32 {
            m.add_triangle(0, 1 + k, 1 + (k + 1) % 10);
        }
        p.add(egui::Shape::mesh(m));
    } else {
        p.add(egui::Shape::closed_line(pts, Stroke::new(1.3f32, col)));
    }
}

/// A button with a star on it (lit gold when on).
pub(crate) fn star_btn(ui: &mut egui::Ui, on: bool) -> egui::Response {
    let r = retro_btn_w(ui, " ", 30.0, false);
    star(ui.painter(), r.rect.center(), 7.0, on, if on { GOLD } else { pal().ink2 });
    r
}

/// The tags used in all the days' notes, most used first (then by name), with how many days have each.
pub(crate) fn tag_counts(days: &[store::Day]) -> Vec<(String, usize)> {
    let mut n: HashMap<String, usize> = HashMap::new();
    for d in days {
        for t in crate::notes_fmt::tags(&d.note) {
            *n.entry(t).or_default() += 1;
        }
    }
    let mut v: Vec<(String, usize)> = n.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// What a click on a day's card in ALL NOTES asks for.
pub(crate) enum CardClick {
    Open(i64),
    Star(i64),
    Tag(String),
    Tune(usize),
    Link(String),
}

/// A desk calendar's page (ALL NOTES' year, month or day): the band at the top, the big word, a small one under it;
/// `s` how much of its height shows (it turns over), its words only when it's nearly flat.
pub(crate) fn paint_page(p: &egui::Painter, r: Rect, s: f32, txt: &[String; 3]) {
    const PAPER: Color32 = Color32::from_rgb(236, 231, 221);
    const BAND: Color32 = Color32::from_rgb(168, 68, 58);
    let h = (r.height() * s).max(1.0);
    let cr = Rect::from_center_size(r.center(), Vec2::new(r.width(), h));
    p.rect_filled(cr.translate(Vec2::new(0.0, 3.0)), 7.0, Color32::from_black_alpha(90));
    p.rect_filled(cr, 7.0, PAPER);
    let band = Rect::from_min_size(cr.min, Vec2::new(cr.width(), (18.0 * s).min(cr.height())));
    p.rect_filled(band, egui::Rounding { nw: 7.0, ne: 7.0, sw: 0.0, se: 0.0 }, BAND);
    for x in [cr.min.x + 16.0, cr.max.x - 16.0] {
        p.circle_filled(Pos2::new(x, band.min.y + 5.0 * s), 2.6 * s, Color32::from_rgb(58, 23, 20));
    }
    if s > 0.75 {
        crate::font::keep_caps(|| {
            ptext(p, Pos2::new(cr.center().x, band.center().y + 1.0), Align::Center, &txt[0], 1.0, Color32::WHITE);
            let big_y = if txt[2].is_empty() { (band.max.y + cr.max.y) / 2.0 } else { band.max.y + (cr.max.y - band.max.y) * 0.4 };
            ptext_fit(p, Pos2::new(cr.center().x, big_y), Align::Center, &txt[1], 2.4, cr.width() - 12.0, Color32::from_rgb(27, 27, 27));
            if !txt[2].is_empty() {
                ptext(p, Pos2::new(cr.center().x, cr.max.y - 8.0), Align::Center, &txt[2], 0.9, Color32::from_rgb(106, 100, 92));
            }
        });
    }
}

/// A desk calendar's page as it turns (`flip`: when it started, the page it turns from): how much of it shows and
/// which words (the old page's going down, the new one's coming up), and whether it's still turning.
pub(crate) fn page_turn(flip: &Option<(f64, i8, [String; 3])>, now: f64, words: &[String; 3]) -> (f32, [String; 3], bool) {
    match flip {
        Some((t0, _, old)) => {
            let t = ((now - t0) / 0.3) as f32;
            if t >= 1.0 {
                (1.0, words.clone(), false)
            } else if t < 0.5 {
                ((std::f32::consts::PI * t).cos(), old.clone(), true)
            } else {
                ((std::f32::consts::PI * t).cos().abs(), words.clone(), true)
            }
        }
        None => (1.0, words.clone(), false),
    }
}

/// The page for a period of ALL NOTES ("2026", "2026-11" or "2026-11-05"), with how many days of notes it has (`empty`:
/// none).
pub(crate) fn page_words(key: &str, empty: bool, n: usize) -> [String; 3] {
    let days = if empty { "NO NOTES".to_string() } else { format!("{} {}", n, if n == 1 { "DAY" } else { "DAYS" }) };
    match key.len() {
        4 => ["YEAR".into(), key.to_string(), days],
        7 => [key[..4].to_string(), title_case(MONTH_NAMES[key[5..7].parse::<usize>().unwrap_or(1).clamp(1, 12) - 1]), days],
        _ => {
            let d = store::parse_date(key).unwrap_or(0);
            let (y, m, dd) = store::civil_from_days(d);
            let under = if empty { format!("{}  NO NOTES", store::weekday(d)) } else { store::weekday(d).to_string() };
            [format!("{} {}", &MONTH_NAMES[m as usize - 1][..3], y), dd.to_string(), under]
        }
    }
}

/// "NOVEMBER" as "November".
pub(crate) fn title_case(w: &str) -> String {
    let mut c = w.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase())
}

/// A period's name, for the arrows' hints.
pub(crate) fn period_name(key: &str) -> String {
    match key.len() {
        4 => key.to_string(),
        7 => format!("{} {}", title_case(MONTH_NAMES[key[5..7].parse::<usize>().unwrap_or(1).clamp(1, 12) - 1]), &key[..4]),
        _ => {
            let d = store::parse_date(key).unwrap_or(0);
            let (_, m, dd) = store::civil_from_days(d);
            format!("{} {} {}", title_case(store::weekday(d)), title_case(&MONTH_NAMES[m as usize - 1][..3]), dd)
        }
    }
}

/// One day of ALL NOTES as a card: its date (bright) and minutes, its star, its notes formatted (`find` marked, the
/// tunes `tunes` linked); `slide` its offset across and how much it shows (coming in after the page turns).
#[allow(clippy::too_many_arguments)]
pub(crate) fn note_card(ui: &mut egui::Ui, d: i64, day: &store::Day, px: f32, find: &str, tunes: &[String], slide: (f32, f32)) -> Option<CardClick> {
    let w = ui.available_width();
    let pad = 12.0;
    let top = 30.0;
    let laid = lay_notes(day.note.trim(), px, w - 2.0 * pad, tunes);
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, top + laid.h + 12.0), Sense::click());
    ui.add_space(6.0);
    if !ui.is_rect_visible(r) {
        return None;
    }
    let rr = r.translate(Vec2::new(slide.0, 0.0));
    let fade = |c: Color32| if slide.1 < 1.0 { c.gamma_multiply(slide.1) } else { c };
    let o = Pos2::new(rr.min.x + pad, rr.min.y + top);
    let hit = resp.hover_pos().and_then(|m| note_hit(&laid, m - o, px));
    let pc = hit.map(|h| &laid.pieces[h.0]);
    let url = pc.and_then(|p| p.url.clone());
    let tag = pc.filter(|p| p.look.tag).map(|p| p.text.to_lowercase());
    let tune = pc.and_then(|p| p.tune);
    let sr = Rect::from_center_size(Pos2::new(rr.max.x - 20.0, rr.min.y + 15.0), Vec2::splat(26.0));
    let star_hot = resp.hover_pos().is_some_and(|m| sr.contains(m));
    let p = ui.painter();
    let face = if resp.hovered() { lerp_col(pal().row_alt, pal().ink, 0.05) } else { pal().row_alt };
    p.rect_filled(rr, 6.0, fade(face));
    p.rect_stroke(rr, 6.0, Stroke::new(1.0f32, fade(pal().edge)));
    // the date, bright; the minutes, dim
    let (y, m, dd) = store::civil_from_days(d);
    let date = format!("{} · {} {}, {}", store::weekday(d), &MONTH_NAMES[m as usize - 1][..3], dd, y);
    let mins: u32 = day.secs.values().sum();
    crate::font::keep_caps(|| {
        let dw = ptext(p, Pos2::new(rr.min.x + pad, rr.min.y + 15.0), Align::Min, &date, 1.3, fade(pal().ink));
        if mins >= 60 {
            ptext(p, Pos2::new(rr.min.x + pad + dw + 12.0, rr.min.y + 15.0), Align::Min, &mins_text(mins), 1.0, fade(pal().dim));
        }
    });
    star(p, sr.center(), 7.5, day.star, fade(if day.star { GOLD } else if star_hot { pal().ink } else { pal().ink2 }));
    p.hline(rr.min.x + pad..=rr.max.x - pad, rr.min.y + top - 5.0, Stroke::new(1.0f32, fade(pal().edge)));
    draw_notes(&p.with_clip_rect(rr), o, w - 2.0 * pad, &laid, slide.1, hit.map(|h| h.0), find);
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let resp = if star_hot {
        resp.tip(if day.star { "Starred. Click to take the star off." } else { "Star this day." })
    } else if let Some(u) = &url {
        resp.tip(u)
    } else if let Some(t) = &tag {
        resp.tip(format!("Every day with {}.", t))
    } else if let Some(i) = tune {
        resp.tip(format!("Open {}.", tunes[i]))
    } else {
        resp.tip("Open this day.")
    };
    if !resp.clicked() {
        return None;
    }
    Some(if star_hot {
        CardClick::Star(d)
    } else if let Some(u) = url {
        CardClick::Link(u)
    } else if let Some(t) = tag {
        CardClick::Tag(t)
    } else if let Some(i) = tune {
        CardClick::Tune(i)
    } else {
        CardClick::Open(d)
    })
}

/// Partway from `a` to `b`.
pub(crate) fn lerp_col(a: Color32, b: Color32, k: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

impl App {
    /// The summary under the day: where the time went over the week (month, year) - a column a day (a month)
    /// stacked by song, the split between tunes, free practice and reading, the top songs and the top loops. The
    /// top five songs get colours; the rest is OTHER, so a lot of practice never makes it busier.
    pub(crate) fn diary_summary(&mut self, ui: &mut egui::Ui, period: &Period, sum: &Summary) {
        ui.horizontal_wrapped(|ui| {
            const SPANS: [&str; 3] = ["THIS WEEK", "THIS MONTH", "THIS YEAR"];
            if let Some(i) = dropdown(ui, "diary_span", "SUMMARY", &SPANS, self.diary_span as usize, 130.0) {
                self.diary_span = i as u8;
                self.dirty = true;
            }
            ui.add_space(10.0);
            if check_box(ui, "INCLUDE READING", self.diary_reading)
                .tip("Include sight-reading time in these charts and totals.")
                .clicked()
            {
                self.diary_reading = !self.diary_reading;
                self.dirty = true;
            }
        });
        ui.add_space(4.0);
        title_line(ui, &format!("{}  -  {}", period.name, mins_text(sum.total)), 1.5, pal().ink2);
        dim_line(ui, &format!("PRACTICED ON {} OF {} DAYS", sum.days_practised, sum.days_so_far), 1.0, pal().dim);
        if sum.total == 0 {
            dim_line(ui, "NO PRACTICE IN THIS PERIOD YET", 1.0, pal().dim);
            return;
        }
        // a column a day (a month), stacked by song
        ui.add_space(4.0);
        let w = seen_w(ui);
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 96.0), Sense::hover());
        let p = ui.painter();
        let n = sum.cols.len().max(1) as f32;
        let gap = if n > 14.0 { 2.0 } else { 6.0 };
        let cw = ((r.width() - gap * (n - 1.0)) / n).max(2.0);
        let chart_h = r.height() - 16.0;
        let top = sum.cols.iter().map(|c| c.total).max().unwrap_or(1).max(1) as f32;
        for (k, c) in sum.cols.iter().enumerate() {
            let x = r.min.x + k as f32 * (cw + gap);
            let base = r.min.y + chart_h;
            p.rect_filled(Rect::from_min_size(Pos2::new(x, base - 1.0), Vec2::new(cw, 1.0)), 0.0, pal().dim.gamma_multiply(0.5));
            let mut y = base;
            for (_, secs, col) in sum.segments_for(self, &c.days) {
                let h = secs as f32 / top * chart_h;
                p.rect_filled(Rect::from_min_max(Pos2::new(x, y - h), Pos2::new(x + cw, y)), 0.0, col);
                y -= h;
            }
            if !c.label.is_empty() {
                ptext(p, Pos2::new(x + cw / 2.0, r.max.y - 6.0), Align::Center, &c.label, 1.0, pal().dim);
            }
        }
        // the split: tunes, free practice, reading
        ui.add_space(12.0);
        let split: Vec<(String, u32, Color32)> = [
            ("TUNES", sum.tunes, Color32::from_rgb(217, 164, 65)),
            ("FREE PRACTICE", sum.free, Color32::from_rgb(95, 168, 211)),
            ("READING", sum.reading, READING_COL),
        ]
        .into_iter()
        .filter(|s| s.1 > 0)
        .map(|(n, s, c)| (n.to_string(), s, c))
        .collect();
        dim_line(ui, "PRACTICE BY TYPE", 1.0, pal().dim);
        stacked_bar(ui, &split, 10.0);
        legend(ui, &split);
        // the top songs, and the top loops
        ui.add_space(6.0);
        dim_line(ui, "TOP SONGS", 1.0, pal().dim);
        let most = sum.songs.first().map_or(1, |s| s.1).max(1);
        for (id, secs) in sum.songs.iter().take(5) {
            let name = self.song_name(*id);
            bar_row(ui, &name, *secs, most, sum.colour(*id), &mins_text(*secs));
        }
        if !sum.loops.is_empty() {
            ui.add_space(4.0);
            dim_line(ui, "TOP LOOPS", 1.0, pal().dim);
            let most = sum.loops.first().map_or(1, |l| l.3).max(1);
            for (id, a, b, secs, passes) in sum.loops.iter().take(5) {
                let loop_name = self
                    .store
                    .sections
                    .get(id)
                    .and_then(|s| s.iter().find(|s| (s.a - a).abs() < 0.06 && (s.b - b).abs() < 0.06))
                    .map(|s| s.name.clone())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| format!("{}-{}", fmt_time(*a), fmt_time(*b)));
                let song = self.store.songs.get(id).map(|v| v.title.clone()).unwrap_or_default();
                let _ = passes;
                let right = mins_text(*secs);
                bar_row(ui, &format!("{}  ({})", loop_name, song), *secs, most, sum.colour(*id), &right);
            }
        }
    }

    /// "Artist - Title" of a song in the diary.
    pub(crate) fn song_name(&self, id: i64) -> String {
        self.store.songs.get(&id).map(|v| format!("{} - {}", v.artist, v.title)).unwrap_or_else(|| "A SONG".to_string())
    }

    /// One song of a diary day: back to the day's songs, its name and time, a small PLAY, and each loop worked on
    /// that day - its name, where it is, the time on it and the passes through it (click one to play that loop).
    pub(crate) fn diary_song_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, date: &str, id: i64, secs: u32) {
        let v = self.store.songs.get(&id).cloned();
        let name = v.as_ref().map(|v| format!("{} - {}", v.artist, v.title)).unwrap_or_else(|| "A SONG".to_string());
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if crate::ibtn(ui, &crate::IC_BACK, "SONGS", false).tip("Back to this day's songs.").clicked() {
                self.diary_song = None;
            }
            if let Some(v) = v.clone() {
                if retro_btn(ui, "PLAY", false).tip("Play the song.").clicked() {
                    acts.push(Action::PlaySaved(v, None));
                }
            }
        });
        title_line(ui, &name, 2.5, pal().ink);
        title_line(ui, &format!("{} practiced on {}", mins_text(secs), date), 1.5, pal().ink2);
        ui.add_space(4.0);
        let saved = self.store.sections.get(&id).cloned().unwrap_or_default();
        let name_of = |a: f32, b: f32| {
            saved
                .iter()
                .find(|s| (s.a - a).abs() < 0.06 && (s.b - b).abs() < 0.06)
                .map(|s| s.name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| "LOOP".to_string())
        };
        let mut loops: Vec<(f32, f32, u32, u32)> = self
            .store
            .days
            .iter()
            .find(|d| d.date == date)
            .map(|d| {
                d.loop_time
                    .iter()
                    .filter_map(|(k, (s, n))| store::parse_loop_key(k).filter(|x| x.0 == id).map(|x| (x.1, x.2, *s, *n)))
                    .collect()
            })
            .unwrap_or_default();
        loops.sort_by(|a, b| b.2.cmp(&a.2).then(b.3.cmp(&a.3)));
        if loops.is_empty() {
            dim_line(ui, "NO LOOP TIMES FOR THIS DAY (TRACKED SINCE 2026-10-14)", 1.0, pal().dim);
            if !saved.is_empty() {
                ui.add_space(4.0);
                dim_line(ui, "THE SONG'S SAVED LOOPS", 1.0, pal().dim);
                loops = saved.iter().map(|s| (s.a, s.b, 0, 0)).collect();
            }
        } else {
            dim_line(ui, "LOOPS PRACTICED (CLICK TO PLAY)", 1.0, pal().dim);
        }
        for (k, (a, b, s, n)) in loops.iter().enumerate() {
            let label = format!("{}   {}-{}", name_of(*a, *b), fmt_time(*a), fmt_time(*b));
            let _ = n;
            let right = if *s == 0 { String::new() } else { mins_text(*s) };
            if let Some(r) = list_row(ui, k, None, || label.clone(), || right.clone(), RowState::Normal, false, false) {
                if let Some(v) = v.clone() {
                    if r.tip("Play this loop.").clicked() {
                        acts.push(Action::PlaySaved(v, Some((*a, *b))));
                    }
                }
            }
        }
    }

    /// A day of the diary, opened (from ALL NOTES or a tune's page).
    pub(crate) fn open_diary_day(&mut self, d: i64, acts: &mut Vec<Action>) {
        if self.sec != Sec::Diary {
            acts.push(Action::Section(Sec::Diary));
        }
        self.diary_tab = 0;
        self.diary_all = 0;
        self.diary_sel = d;
        let (y, m, _) = store::civil_from_days(d);
        self.diary_ym = (y, m);
    }

    /// ALL NOTES, showing the days with `tag` (or, `find`, the days a search finds).
    pub(crate) fn open_notes(&mut self, tag: Option<String>, find: &str, acts: &mut Vec<Action>) {
        if self.sec != Sec::Diary {
            acts.push(Action::Section(Sec::Diary));
        }
        self.diary_tab = 0;
        self.diary_all = SKIM;
        self.notes_filter = tag;
        self.notes_q = find.to_string();
    }

    /// Whether day `d`'s notes fit ALL NOTES' filter (starred, or a tag) and the search `find` (lowercase: in the
    /// words, or the date as written or read out).
    pub(crate) fn note_fits(&self, day: &store::Day, d: i64, find: &str) -> bool {
        match self.notes_filter.as_deref() {
            Some("*") if !day.star => return false,
            Some(t) if t != "*" && !crate::notes_fmt::tags(&day.note).iter().any(|g| g == t) => return false,
            _ => {}
        }
        if find.is_empty() {
            return true;
        }
        let (_, m, _) = store::civil_from_days(d);
        let when = format!("{} {} {}", store::weekday(d), day.date, MONTH_NAMES[m as usize - 1]).to_lowercase();
        when.contains(find) || crate::notes_fmt::plain(&day.note).to_lowercase().contains(find)
    }

    /// ALL NOTES, a page of its own: BACK (or Esc, or the DIARY tab) to the diary; a search; ALL, STARRED and your tags
    /// (the five used most, the rest behind ...); a year, a month or a day at a time, a desk calendar's page on the
    /// diary's own day, turned by the calendar with < and > (TODAY back; a click on the page goes in: year to months,
    /// month to days, a day to the diary; a page without notes points to the nearest either side); the days as cards (a
    /// click opens one, its star stars it, a tag on it shows that tag's days, a tune named on it opens the tune's page).
    pub(crate) fn all_notes_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let px = if crate::winamp_ui::chrome_on() { 1.7 } else { 2.0 };
        let back = ui.horizontal(|ui| retro_btn(ui, "< BACK", false).tip("Back to the diary (Esc).").clicked()).inner;
        let esc = !self.fullscreen && !self.art_view && self.ed.id == 0 && !self.panel_open() && ui.input(|i| i.key_pressed(egui::Key::Escape));
        if back || esc {
            self.diary_all = 0;
            ui.ctx().request_repaint();
            return;
        }
        let noted = noted_days(&self.store.days);
        // (with a search or a filter on: how many of them fit, worked out below and shown the next frame)
        if self.notes_q.trim().is_empty() && self.notes_filter.is_none() {
            section_header(ui, &format!("ALL NOTES  -  {} DAYS", noted.len()));
        } else {
            section_header(ui, &format!("ALL NOTES  -  {} OF {} DAYS", self.notes_fit, noted.len()));
        }
        // the search
        let (sr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bh()), Sense::hover());
        if field(ui, &mut self.ed, crate::tools::F_NOTES_SEARCH, &mut self.notes_q, sr, "Search your notes", false).changed {
            self.diary_all = SKIM;
        }
        ui.add_space(4.0);
        // the filters: everything, the starred days, your tags (the five used most, then ... for the rest)
        let tags = tag_counts(&self.store.days);
        let starred = self.store.days.iter().filter(|d| d.star && !d.note.trim().is_empty()).count();
        let mut pick: Option<Option<String>> = None;
        ui.horizontal_wrapped(|ui| {
            let cur = self.notes_filter.clone();
            if retro_btn(ui, "ALL", cur.is_none()).tip("Every day with notes.").clicked() {
                pick = Some(None);
            }
            let on = cur.as_deref() == Some("*");
            let r = retro_btn(ui, &format!("    STARRED {}", starred), on).tip("The days you starred.");
            star(ui.painter(), Pos2::new(r.rect.min.x + 15.0, r.rect.center().y), 6.0, true, GOLD);
            if r.clicked() {
                pick = Some(if on { None } else { Some("*".to_string()) });
            }
            for (k, (g, n)) in tags.iter().enumerate() {
                let on = cur.as_deref() == Some(g.as_str());
                if !self.notes_tags_all && k >= 5 && !on {
                    continue;
                }
                if retro_btn(ui, &format!("{} {}", g, n), on).tip(format!("Every day with {}.", g)).clicked() {
                    pick = Some(if on { None } else { Some(g.clone()) });
                }
            }
            if tags.len() > 5 {
                let (label, tip) = if self.notes_tags_all {
                    ("LESS", "Only the tags you use most.".to_string())
                } else {
                    ("...", format!("{} more tags", tags.len() - 5))
                };
                if retro_btn(ui, label, false).tip(tip).clicked() {
                    self.notes_tags_all = !self.notes_tags_all;
                }
            }
        });
        if let Some(f) = pick {
            self.notes_filter = f;
            self.diary_all = SKIM;
        }
        ui.add_space(4.0);

        // the days that fit, newest first
        let find = self.notes_q.trim().to_lowercase();
        let tunes = self.tune_names();
        let at: HashMap<&str, usize> = self.store.days.iter().enumerate().map(|(i, d)| (d.date.as_str(), i)).collect();
        let days: Vec<(i64, usize)> = noted
            .iter()
            .rev()
            .filter_map(|&d| at.get(store::date_str(d).as_str()).map(|&i| (d, i)))
            .filter(|&(d, i)| self.note_fits(&self.store.days[i], d, &find))
            .collect();
        if self.notes_fit != days.len() {
            self.notes_fit = days.len();
            ui.ctx().request_repaint();
        }

        // the period: a year, a month or a day, on the diary's own day (where the diary is when you go back), turned
        // by the calendar with < and > (a page without notes too: it says so, with the nearest notes either side)
        let mode = self.notes_mode.min(2);
        let key = |d: i64| -> String {
            let s = store::date_str(d);
            match mode {
                0 => s[..4].to_string(),
                1 => s[..7].to_string(),
                _ => s,
            }
        };
        let today = store::today_days();
        if self.diary_sel == 0 {
            self.diary_sel = today;
        }
        let cur = key(self.diary_sel);
        let in_p: Vec<(i64, usize)> = days.iter().copied().filter(|&(d, _)| key(d) == cur).collect();
        // the nearest days with notes (that fit) before and after this page
        let near_old = days.iter().map(|&(d, _)| d).find(|&d| key(d) < cur);
        let near_new = days.iter().map(|&(d, _)| d).filter(|&d| key(d) > cur).next_back();
        let words = page_words(&cur, in_p.is_empty(), in_p.len());
        let now = ui.input(|i| i.time);
        let mut go: Option<(i64, i8)> = None;
        ui.horizontal_wrapped(|ui| {
            for (k, name) in ["YEAR", "MONTH", "DAY"].iter().enumerate() {
                let tip = ["A year at a time.", "A month at a time.", "A day at a time."][k];
                if retro_btn(ui, name, mode == k as u8).tip(tip).clicked() {
                    self.notes_mode = k as u8;
                    self.diary_all = SKIM;
                }
            }
        });
        ui.add_space(4.0);
        let mut deeper = false;
        // (a row as tall as the page, so the arrows and TODAY sit level with its middle)
        ui.allocate_ui_with_layout(Vec2::new(ui.available_width(), 66.0), egui::Layout::left_to_right(Align::Center), |ui| {
            for (label, dir) in [("<", -1i8), (">", 1i8)] {
                if dir > 0 {
                    // the calendar's page, turning when it changes; a click goes in (a year to its months, a month to
                    // its days, a day to the diary)
                    let (pr, presp) = ui.allocate_exact_size(Vec2::new(128.0, 66.0), Sense::click());
                    let (s, shown, _) = page_turn(&self.notes_flip, now, &words);
                    paint_page(ui.painter(), pr, s.max(0.02), &shown);
                    let tip = ["Show this year's months.", "Show this month's days.", "Open this day in the diary."][mode as usize];
                    if presp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(tip).clicked() {
                        deeper = true;
                    }
                }
                let to = period_step(self.diary_sel, mode, dir);
                let tip = format!("{}: {}", if dir < 0 { "Earlier" } else { "Later" }, period_name(&key(to)));
                if retro_btn_w(ui, label, 34.0, false).tip(tip).clicked() {
                    go = Some((to, dir));
                }
            }
            if key(today) != cur && retro_btn(ui, "TODAY", false).tip("Back to today.").clicked() {
                go = Some((today, if today < self.diary_sel { -1 } else { 1 }));
            }
        });
        if deeper {
            if mode == 2 {
                self.open_diary_day(self.diary_sel, acts);
                ui.ctx().request_repaint();
                return;
            }
            self.notes_mode = mode + 1;
            self.diary_all = SKIM;
            ui.ctx().request_repaint();
        }
        let turn_to = |app: &mut App, d: i64, dir: i8| {
            app.diary_sel = d;
            let (y, m, _) = store::civil_from_days(d);
            app.diary_ym = (y, m);
            app.notes_flip = Some((now, dir, words.clone()));
            app.diary_all = SKIM;
        };
        if let Some((d, dir)) = go {
            turn_to(self, d, dir);
            ui.ctx().request_repaint();
        }
        // the cards come in as the page turns
        let mut slide = (0.0, 1.0);
        let mut done = false;
        if let Some((t0, dir, _)) = &self.notes_flip {
            let t = ((now - t0) / 0.3) as f32;
            if t >= 1.0 {
                done = true;
            } else {
                let e = 1.0 - (1.0 - t).powi(3);
                slide = (*dir as f32 * 30.0 * (1.0 - e), e);
                ui.ctx().request_repaint();
            }
        }
        if done {
            self.notes_flip = None;
        }
        ui.add_space(6.0);

        // the list: a year by its months (a month's name opens it), a month's days, or the one day; a page without
        // notes says so, with the way to the nearest notes either side (and, a day, to write some)
        if in_p.is_empty() {
            let when = period_name(&cur);
            let what = match (find.is_empty() && self.notes_filter.is_none(), mode) {
                (true, 2) => format!("No notes on {}", when),
                (true, _) => format!("No notes in {}", when),
                (false, _) if !find.is_empty() => format!("No notes match \"{}\" in {}", self.notes_q.trim(), when),
                (false, _) => format!("None in {}", when),
            };
            dim_line(ui, &what, 2.0, pal().ink2);
            ui.add_space(4.0);
            let mut jump: Option<(i64, i8)> = None;
            ui.horizontal_wrapped(|ui| {
                if let Some(d) = near_old {
                    if retro_btn(ui, &format!("< {}", period_name(&store::date_str(d))), false).tip("The notes before this.").clicked() {
                        jump = Some((d, -1));
                    }
                }
                if let Some(d) = near_new {
                    if retro_btn(ui, &format!("{} >", period_name(&store::date_str(d))), false).tip("The notes after this.").clicked() {
                        jump = Some((d, 1));
                    }
                }
                if mode == 2 && retro_btn(ui, "WRITE ONE", false).tip("Open this day in the diary, to write its notes.").clicked() {
                    self.open_diary_day(self.diary_sel, acts);
                }
            });
            if let Some((d, dir)) = jump {
                turn_to(self, d, dir);
                ui.ctx().request_repaint();
            }
            return;
        }
        let mut asked: Option<CardClick> = None;
        let mut month_open: Option<i64> = None;
        let mut last_month = String::new();
        let limit = if mode == 2 { 1 } else { self.diary_all.max(SKIM) };
        for &(d, i) in in_p.iter().take(limit) {
            if mode == 0 {
                let k = store::date_str(d)[..7].to_string();
                if k != last_month {
                    let n = in_p.iter().filter(|&&(x, _)| store::date_str(x).starts_with(&k)).count();
                    let (hr, hresp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), Sense::click());
                    let hot = hresp.hovered();
                    let m = k[5..7].parse::<usize>().unwrap_or(1).clamp(1, 12);
                    crate::font::keep_caps(|| {
                        let tw = ptext(ui.painter(), Pos2::new(hr.min.x + 4.0, hr.center().y), Align::Min, MONTH_NAMES[m - 1], 1.4, if hot { pal().ink } else { pal().ink2 });
                        ptext(ui.painter(), Pos2::new(hr.min.x + tw + 14.0, hr.center().y), Align::Min, &format!("{} {}", n, if n == 1 { "DAY" } else { "DAYS" }), 1.0, pal().dim);
                    });
                    if hresp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("Show this month.").clicked() {
                        month_open = Some(d);
                    }
                    last_month = k;
                }
            }
            if let Some(c) = note_card(ui, d, &self.store.days[i], px, &find, &tunes, slide) {
                asked = Some(c);
            }
        }
        if mode != 2 && in_p.len() > limit {
            ui.add_space(4.0);
            if retro_btn(ui, "SHOW MORE", false).tip("Older notes.").clicked() {
                self.diary_all = limit + SKIM;
            }
        }
        if let Some(d) = month_open {
            self.notes_mode = 1;
            self.diary_sel = d;
            let (y, m, _) = store::civil_from_days(d);
            self.diary_ym = (y, m);
            ui.ctx().request_repaint();
        }
        match asked {
            Some(CardClick::Open(d)) => {
                self.open_diary_day(d, acts);
                ui.ctx().request_repaint();
            }
            Some(CardClick::Star(d)) => {
                let date = store::date_str(d);
                if let Some(day) = self.store.days.iter_mut().find(|x| x.date == date) {
                    day.star = !day.star;
                    self.store_dirty = true;
                }
            }
            Some(CardClick::Tag(t)) => {
                self.notes_filter = Some(t);
                self.diary_all = SKIM;
            }
            Some(CardClick::Tune(i)) => self.open_tune(i, acts),
            Some(CardClick::Link(u)) => open_link(&u),
            None => {}
        }
    }

    /// While a #tag is being typed in the day's notes: the tags you've used that start the same way (the most used
    /// first), to pick. True when one was picked.
    pub(crate) fn diary_tag_pick(&mut self, ui: &mut egui::Ui) -> bool {
        let id = crate::tools::F_DIARY_NOTE;
        if self.ed.id != id {
            return false;
        }
        let chars: Vec<char> = self.diary_note.1.chars().collect();
        let cur = self.ed.cur.min(chars.len());
        let Some((at, typed)) = crate::notes_fmt::tag_at(&chars, cur) else { return false };
        let fits: Vec<String> =
            tag_counts(&self.store.days).into_iter().map(|t| t.0).filter(|g| g.starts_with(&typed) && *g != typed).take(8).collect();
        if fits.is_empty() {
            return false;
        }
        let mut pick = None;
        let row = ui.horizontal_wrapped(|ui| {
            let (lr, _) = ui.allocate_exact_size(Vec2::new(90.0, bh()), Sense::hover());
            ptext(ui.painter(), Pos2::new(lr.min.x + 4.0, lr.center().y), Align::Min, "YOUR TAGS", 1.0, pal().dim);
            for g in &fits {
                if retro_btn(ui, g, false).tip("Use this tag.").clicked() {
                    pick = Some(g.clone());
                }
            }
        });
        if row.response.contains_pointer() {
            self.ed.hold_next = id;
        }
        let Some(g) = pick else { return false };
        let ins: Vec<char> = format!("{} ", g).chars().collect();
        let mut c = chars;
        c.splice(at..cur, ins.iter().copied());
        self.diary_note.1 = c.iter().collect();
        self.ed.cur = at + ins.len();
        self.ed.anchor = Some(self.ed.cur);
        true
    }

    pub(crate) fn diary_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        title_line(ui, "PRACTICE DIARY", 3.0, pal().ink);
        // MY PLAN only when there is a plan file of your own
        let mine = crate::plan::have_plan();
        let tabs: &[&str] = if mine { &["DIARY", "ROUTINE", "LISTENING", "MY PLAN"] } else { &["DIARY", "ROUTINE", "LISTENING"] };
        if self.diary_tab as usize >= tabs.len() {
            self.diary_tab = 0;
        }
        if let Some(i) = tab_row(ui, tabs, self.diary_tab as usize) {
            self.diary_tab = i as u8;
            self.diary_all = 0;
        }
        ui.add_space(6.0);
        match self.diary_tab {
            3 => return self.plan_view(ui, acts),
            1 => return self.routine_view(ui, acts),
            2 => return self.listening_view(ui, acts),
            _ if self.diary_all > 0 => return self.all_notes_view(ui, acts),
            _ => {}
        }
        let today = store::today_days();
        if self.diary_sel == 0 {
            self.diary_sel = today;
        }
        if self.diary_ym.0 == 0 {
            let (y, m, _) = store::civil_from_days(self.diary_sel);
            self.diary_ym = (y, m);
        }
        let live = self.acc as u32;
        let day_secs = |app: &App, d: i64| app.store.secs_on(&store::date_str(d)) + if d == today { live } else { 0 };

        // ---- the day's notes, first: a journal in your own words, for the day picked on the calendar (today at first),
        // saved as you type (a day is only added to the diary once something is written)
        let note_date = store::date_str(self.diary_sel);
        if self.diary_note.0 != note_date {
            let text = self.store.days.iter().find(|d| d.date == note_date).map(|d| d.note.clone()).unwrap_or_default();
            self.diary_note = (note_date.clone(), text);
        }
        let noted = noted_days(&self.store.days);
        let px = if crate::winamp_ui::chrome_on() { 1.7 } else { 2.0 };
        section_header(ui, &format!("NOTES  -  {} {}", store::weekday(self.diary_sel), note_date));
        // skimming: the day before or after that has notes, or all of them to read through (a page of their own)
        // and the formatting tools at the end of the row
        let (mut jump, mut turn, mut tooled) = (None, None, false);
        ui.horizontal_wrapped(|ui| {
            if !noted.is_empty() {
                for (label, up, tip) in [("<", false, "Previous day with notes."), (">", true, "Next day with notes.")] {
                    let to = note_step(&noted, self.diary_sel, up);
                    let r = ui
                        .scope(|ui| {
                            if to.is_none() {
                                ui.set_opacity(0.35);
                            }
                            retro_btn_w(ui, label, 34.0, false)
                        })
                        .inner;
                    if to.is_some() && r.tip(tip).clicked() {
                        (jump, turn) = (to, Some(up));
                    }
                }
                if retro_btn(ui, "ALL NOTES", false).tip("Search your notes, by year, month or day.").clicked() {
                    self.diary_all = SKIM;
                }
            }
            // a star for the day (once it has notes)
            let starred = self.store.days.iter().find(|d| d.date == note_date).filter(|d| !d.note.trim().is_empty()).map(|d| d.star);
            if let Some(on) = starred {
                if star_btn(ui, on).tip(if on { "Starred. Click to take the star off." } else { "Star this day: one that clicked." }).clicked() {
                    if let Some(d) = self.store.days.iter_mut().find(|d| d.date == note_date) {
                        d.star = !on;
                        self.store_dirty = true;
                    }
                }
            }
            tools_at_end(ui);
            tooled = note_tools(ui, &mut self.ed, crate::tools::F_DIARY_NOTE, &mut self.diary_note.1);
        });
        ui.add_space(4.0);
        {
            // (the box grows with what's written: every line in sight, and one more to write on)
            let hint = "How it went, what clicked, what's next... (#tags sort them)";
            let tunes = self.tune_names();
            let (o, nr, asked) = notes_box(ui, &mut self.ed, crate::tools::F_DIARY_NOTE, &mut self.diary_note.1, hint, 100.0, 400.0, &tunes);
            // typing a #tag: the ones you have, to pick
            let picked = self.diary_tag_pick(ui);
            match asked {
                Some(NoteClick::Tag(t)) => self.open_notes(Some(t), "", acts),
                Some(NoteClick::Tune(i)) => self.open_tune(i, acts),
                None => {}
            }
            if o.changed || tooled || picked {
                let text = self.diary_note.1.clone();
                match self.store.days.iter_mut().find(|d| d.date == note_date) {
                    Some(d) => d.note = text,
                    None if !text.trim().is_empty() => self.store.day_mut(&note_date).note = text,
                    None => {}
                }
                self.store_dirty = true;
            }
            // flipped to another day: the old day's words slide out as the new day's slide in
            let mut done = false;
            if let Some((t0, later, old)) = &self.diary_flip {
                let t = ((ui.input(|i| i.time) - t0) / 0.3) as f32;
                if t >= 1.0 {
                    done = true;
                } else {
                    slide_notes(ui.painter(), nr, px, old, &self.diary_note.1, t, *later);
                    ui.ctx().request_repaint();
                }
            }
            if done {
                self.diary_flip = None;
            }
        }
        if let Some(d) = jump {
            self.diary_sel = d;
            let (y, m, _) = store::civil_from_days(d);
            self.diary_ym = (y, m);
            let old = self.diary_note.1.clone();
            self.diary_flip = turn.map(|later| (ui.input(|i| i.time), later, old));
            ui.ctx().request_repaint();
        }
        ui.add_space(8.0);

        // ---- summary
        let week: u32 = (0..7).map(|k| day_secs(self, today - k)).sum();
        let (cy, cm, _) = store::civil_from_days(today);
        let month_prefix = format!("{:04}-{:02}", cy, cm);
        let month: u32 = self
            .store
            .days
            .iter()
            .filter(|d| d.date.starts_with(&month_prefix))
            .map(|d| d.secs.values().sum::<u32>())
            .sum::<u32>()
            + live;
        let total: u32 = self.store.days.iter().map(|d| d.secs.values().sum::<u32>()).sum::<u32>() + live;
        let practised = (0..7).filter(|k| day_secs(self, today - k) > 0).count();
        // three tiles, roomy; the days practised and all time under the pointer
        let tiles = [
            ("TODAY", mins_text(day_secs(self, today))),
            ("7 DAYS", mins_text(week)),
            ("THIS MONTH", mins_text(month)),
        ];
        let w = ui.available_width();
        let tw = ((w - 2.0 * 6.0) / 3.0).floor().max(60.0);
        let (sr, sresp) = ui.allocate_exact_size(Vec2::new(w, 46.0), Sense::hover());
        sresp.on_hover_text(format!(
            "Practiced on {} of the last 7 days. All time: {}.",
            practised,
            mins_text(total).to_lowercase()
        ));
        for (k, (name, val)) in tiles.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(sr.min.x + k as f32 * (tw + 6.0), sr.min.y), Vec2::new(tw, 46.0));
            inset(ui.painter(), r, pal().lcd);
            ptext(ui.painter(), Pos2::new(r.center().x, r.min.y + 13.0), Align::Center, name, 1.0, pal().dim);
            ptext_fit(ui.painter(), Pos2::new(r.center().x, r.min.y + 31.0), Align::Center, val, 2.0, tw - 8.0, pal().ink);
        }

        // ---- calendar
        ui.add_space(8.0);
        let (y, m) = self.diary_ym;
        // the month as a desk calendar's page (as ALL NOTES has it): turned by < and >, its practice time under the
        // name; a click shows the month's notes
        let ym_key = format!("{:04}-{:02}", y, m);
        let month_secs: u32 = self
            .store
            .days
            .iter()
            .filter(|d| d.date.starts_with(&ym_key))
            .map(|d| d.secs.values().sum::<u32>())
            .sum::<u32>()
            + if (y, m) == (cy, cm) { live } else { 0 };
        let words = [
            y.to_string(),
            title_case(MONTH_NAMES[(m as usize - 1) % 12]),
            if month_secs > 0 { mins_text(month_secs) } else { "NO PRACTICE".to_string() },
        ];
        let now = ui.input(|i| i.time);
        let (mut to, mut notes) = (None::<((i32, u32), i8)>, false);
        ui.allocate_ui_with_layout(Vec2::new(ui.available_width(), 66.0), egui::Layout::left_to_right(Align::Center), |ui| {
            if retro_btn_w(ui, "<", 34.0, false).tip("Previous month.").clicked() {
                to = Some((if m == 1 { (y - 1, 12) } else { (y, m - 1) }, -1));
            }
            let (pr, presp) = ui.allocate_exact_size(Vec2::new(128.0, 66.0), Sense::click());
            let (s, shown, turning) = page_turn(&self.diary_cal_flip, now, &words);
            paint_page(ui.painter(), pr, s.max(0.02), &shown);
            if turning {
                ui.ctx().request_repaint();
            }
            if presp.on_hover_cursor(egui::CursorIcon::PointingHand).tip("This month's notes.").clicked() {
                notes = true;
            }
            if retro_btn_w(ui, ">", 34.0, false).tip("Next month.").clicked() {
                to = Some((if m == 12 { (y + 1, 1) } else { (y, m + 1) }, 1));
            }
            if retro_btn(ui, "TODAY", false).tip("Jump to today.").clicked() {
                self.diary_sel = today;
                to = Some(((cy, cm), if (cy, cm) < (y, m) { -1 } else { 1 }));
            }
        });
        if let Some((ym, dir)) = to {
            if ym != (y, m) {
                self.diary_cal_flip = Some((now, dir, words.clone()));
            }
            self.diary_ym = ym;
            ui.ctx().request_repaint();
        }
        if self.diary_cal_flip.as_ref().is_some_and(|f| now - f.0 >= 0.3) {
            self.diary_cal_flip = None;
        }
        if notes {
            // (on the day picked, if it's in this month; else the month's first)
            let (sy, sm, _) = store::civil_from_days(self.diary_sel);
            if (sy, sm) != (y, m) {
                self.diary_sel = store::days_from_civil(y, m, 1);
            }
            self.notes_mode = 1;
            self.diary_all = SKIM;
            ui.ctx().request_repaint();
            return;
        }
        ui.add_space(4.0);
        let first = store::days_from_civil(y, m, 1);
        let next = if m == 12 { store::days_from_civil(y + 1, 1, 1) } else { store::days_from_civil(y, m + 1, 1) };
        let ndays = next - first;
        let lead = (first + 4).rem_euclid(7);
        let gw = ui.available_width().min(7.0 * 84.0);
        let cw = (gw / 7.0).floor();
        let ch = 46.0;
        let rows = (lead + ndays + 6) / 7;
        let (gr, _) = ui.allocate_exact_size(Vec2::new(cw * 7.0, 16.0 + rows as f32 * ch), Sense::hover());
        for (k, n) in ["S", "M", "T", "W", "T", "F", "S"].iter().enumerate() {
            ptext(ui.painter(), Pos2::new(gr.min.x + k as f32 * cw + cw / 2.0, gr.min.y + 7.0), Align::Center, n, 1.0, pal().dim);
        }
        let busiest = (0..ndays).map(|k| day_secs(self, first + k)).max().unwrap_or(0).max(1);
        // (days with notes get a pencil, a starred one a gold star; the note shows under the pointer)
        let notes: HashMap<String, (String, bool)> = self
            .store
            .days
            .iter()
            .filter(|d| !d.note.trim().is_empty())
            .map(|d| (d.date.clone(), (note_preview(&d.note), d.star)))
            .collect();
        for k in 0..ndays {
            let d = first + k;
            let pos = lead + k;
            let r = Rect::from_min_size(
                Pos2::new(gr.min.x + (pos % 7) as f32 * cw + 2.0, gr.min.y + 16.0 + (pos / 7) as f32 * ch + 2.0),
                Vec2::new(cw - 4.0, ch - 4.0),
            );
            let secs = day_secs(self, d);
            let resp =
                ui.interact(r, ui.id().with(("diaryday", d)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let level = if secs == 0 { 0 } else { 1 + (secs * 3 / busiest).min(2) };
            let face = match level {
                0 => pal().lcd,
                1 => pal().row_alt,
                2 => pal().ink2,
                _ => pal().ink,
            };
            inset(ui.painter(), r, face);
            if d == self.diary_sel {
                outline(ui.painter(), r, 2.0, pal().red);
            } else if d == today {
                outline(ui.painter(), r, 1.0, pal().ink);
            }
            let txt_col = if level >= 2 { pal().lcd } else { pal().ink };
            ptext(ui.painter(), Pos2::new(r.min.x + 5.0, r.min.y + 8.0), Align::Min, &format!("{}", k + 1), 1.0, txt_col);
            if secs >= 60 {
                ptext(ui.painter(), Pos2::new(r.max.x - 4.0, r.min.y + 8.0), Align::Max, &format!("{}m", secs / 60), 1.0, txt_col);
            }
            let resp = match notes.get(&store::date_str(d)) {
                Some((preview, starred)) => {
                    if *starred {
                        star(ui.painter(), Pos2::new(r.min.x + 12.0, r.max.y - 11.0), 7.0, true, GOLD);
                    } else {
                        pencil(ui.painter(), Pos2::new(r.min.x + 5.0, r.max.y - 18.0), 14.0, txt_col.gamma_multiply(0.8));
                    }
                    resp.tip(preview)
                }
                None => resp,
            };
            if resp.clicked() {
                self.diary_sel = d;
            }
        }

        // ---- the picked day
        let sel = self.diary_sel;
        let date = store::date_str(sel);
        ui.add_space(6.0);
        section_header(ui, &format!("{} {}  -  {}", store::weekday(sel), date, mins_text(day_secs(self, sel))));
        let (mut rows_d, mut loops, mut pomos) = (Vec::new(), 0, 0);
        if let Some(d) = self.store.days.iter().find(|d| d.date == date) {
            rows_d = d.secs.iter().map(|(k, v)| (k.clone(), *v)).collect::<Vec<_>>();
            (loops, pomos) = (d.loops, d.pomos);
        }
        rows_d.sort_by(|a, b| b.1.cmp(&a.1));
        let tot: u32 = rows_d.iter().map(|r| r.1).sum();
        // the day's times changed, or added by hand
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "EDIT TIMES", self.diary_edit).tip("Edit this day's times, or add practice done away from the computer.").clicked() {
                self.diary_edit = !self.diary_edit;
            }
        });
        if self.diary_edit {
            let mut change: Option<(String, i64)> = None;
            for (name, s) in &rows_d {
                ui.horizontal(|ui| {
                    if retro_btn(ui, "X", false).tip("Remove from this day.").clicked() {
                        change = Some((name.clone(), -(*s as i64)));
                    }
                    if retro_btn(ui, "-5", false).clicked() {
                        change = Some((name.clone(), -300));
                    }
                    if retro_btn(ui, "+5", false).clicked() {
                        change = Some((name.clone(), 300));
                    }
                    dim_line(ui, &format!("{}  {}", name, mins_text(*s)), 2.0, pal().ink);
                });
            }
            let mut add = false;
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 150.0).max(120.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
                add |= field(ui, &mut self.ed, crate::tools::F_DIARY_ADD_NAME, &mut self.diary_add.0, r, "what you practiced", false).enter;
                let (r, _) = ui.allocate_exact_size(Vec2::new(60.0, bh()), Sense::hover());
                add |= field(ui, &mut self.ed, crate::tools::F_DIARY_ADD_MIN, &mut self.diary_add.1, r, "min", false).enter;
                add |= retro_btn(ui, "ADD", false).tip("Add this many minutes to the day.").clicked();
            });
            if add {
                let name = self.diary_add.0.trim().to_string();
                if let (false, Ok(m)) = (name.is_empty(), self.diary_add.1.trim().parse::<u32>()) {
                    change = Some((name, m as i64 * 60));
                    self.diary_add = (String::new(), String::new());
                }
            }
            if let Some((name, by)) = change {
                let d = self.store.day_mut(&date);
                let v = (*d.secs.get(&name).unwrap_or(&0) as i64 + by).max(0) as u32;
                if v == 0 {
                    d.secs.remove(&name);
                } else {
                    d.secs.insert(name, v);
                }
                self.store_dirty = true;
            }
        }
        if rows_d.is_empty() {
            dim_line(ui, "NO PRACTICE TIME TRACKED THIS DAY", 2.0, pal().ink2);
        }
        for (k, (name, s)) in rows_d.iter().take(12).enumerate() {
            let _ = list_row(
                ui,
                k,
                None,
                || name.clone(),
                || format!("{}  {}%", mins_text(*s), s * 100 / tot.max(1)),
                RowState::Normal,
                false,
                false,
            );
        }
        // time spent looping (how many times round says little), and focus blocks when there were some
        let looped: u32 = self.store.days.iter().find(|d| d.date == date).map_or(0, |d| d.loop_time.values().map(|v| v.0).sum());
        let _ = loops;
        let mut bits = Vec::new();
        if looped > 0 {
            bits.push(format!("TIME LOOPED {}", mins_text(looped)));
        }
        if pomos > 0 {
            bits.push(format!("FOCUS BLOCKS {}", pomos));
        }
        if !bits.is_empty() {
            dim_line(ui, &bits.join("   "), 2.0, pal().ink2);
        }
        // the day in one bar, in the summary's colours
        let span = self.diary_span;
        let period = Period::around(sel, span);
        let sum = Summary::of(self, &period, self.diary_reading);
        let segs = sum.segments_for(self, &[sel]);
        if !segs.is_empty() {
            ui.add_space(4.0);
            stacked_bar(ui, &segs, 12.0);
            legend(ui, &segs);
        }
        // the songs worked on that day (tunes or free practice); click one to play it
        let mut songs: Vec<(i64, u32)> = self
            .store
            .days
            .iter()
            .find(|d| d.date == date)
            .map(|d| d.songs.iter().map(|(k, v)| (*k, *v)).collect())
            .unwrap_or_default();
        songs.sort_by(|a, b| b.1.cmp(&a.1));
        // a song opened: the loops worked on that day and for how long, and PLAY if you want to hear it
        if let Some(id) = self.diary_song.as_ref().filter(|(d, _)| *d == date).map(|(_, id)| *id) {
            self.diary_song_view(ui, acts, &date, id, songs.iter().find(|s| s.0 == id).map_or(0, |s| s.1));
        } else if !songs.is_empty() {
            ui.add_space(4.0);
            dim_line(ui, "SONGS PRACTICED", 1.0, pal().dim);
            for (k, (id, s)) in songs.iter().enumerate() {
                let v = self.store.songs.get(id).cloned();
                let name = v.as_ref().map(|v| format!("{} - {}", v.artist, v.title)).unwrap_or_else(|| "A SONG".to_string());
                // its time, and how much of it was looping
                let looped: u32 = self.store.days.iter().find(|d| d.date == date).map_or(0, |d| {
                    d.loop_time
                        .iter()
                        .filter(|(k, _)| store::parse_loop_key(k).is_some_and(|x| x.0 == *id))
                        .map(|(_, v)| v.0)
                        .sum()
                });
                let r = list_row(
                    ui,
                    k,
                    None,
                    || name.clone(),
                    || {
                        if looped > 0 {
                            format!("{}  ({} LOOPED)", mins_text(*s), mins_text(looped))
                        } else {
                            mins_text(*s)
                        }
                    },
                    RowState::Normal,
                    false,
                    false,
                );
                if let Some(r) = r {
                    if r.tip("The loops you worked on, and for how long.").clicked() {
                        self.diary_song = Some((date.clone(), *id));
                    }
                }
                let _ = v;
            }
        }

        // ---- the week (or month, or year) around the day
        ui.add_space(8.0);
        self.diary_summary(ui, &period, &sum);

        let mins_hint = format!("{}", day_secs(self, sel) / 60);
        // ---- journal for that day
        ui.add_space(4.0);
        if self.show_form {
            let hint_t = self.practice_label();
            let _ = field_row(ui, &mut self.ed, F_TUNE, "TUNE", &mut self.f_tune, &hint_t, None);
            let _ = field_row(ui, &mut self.ed, F_MINS, "MINUTES", &mut self.f_mins, &mins_hint, None);
            let _ = field_row(ui, &mut self.ed, F_BPM, "BPM", &mut self.f_bpm, "The tempo you reached", None);
            let _ = field_row(ui, &mut self.ed, F_NOTE, "NOTE", &mut self.f_note, "What went well? What next?", Some(70.0));
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "SAVE ENTRY", false).clicked() {
                    acts.push(Action::SaveEntry);
                }
                if retro_btn(ui, "CANCEL", false).clicked() {
                    self.show_form = false;
                    self.entry_edit = None;
                    self.ed.id = 0;
                }
            });
        } else if retro_btn(ui, &format!("LOG PRACTICE FOR {}", date), false)
            .tip("Write down what you worked on. It works for past days too.")
            .clicked()
        {
            self.show_form = true;
        }
        ui.add_space(6.0);
        let mut any = false;
        let mut pick: Option<(usize, bool)> = None;
        for (i, e) in self.store.entries.iter().enumerate().rev().filter(|(_, e)| e.date == date) {
            any = true;
            ui.horizontal(|ui| {
                if retro_btn(ui, "EDIT", self.entry_edit == Some(i)).tip("Change this entry.").clicked() {
                    pick = Some((i, false));
                }
                if retro_btn(ui, "X", false).tip("Delete this entry.").clicked() {
                    pick = Some((i, true));
                }
            });
            let head = format!(
                "{}{}{}",
                e.tune,
                if e.mins > 0 { format!("  {}m", e.mins) } else { String::new() },
                if e.bpm > 0 { format!("  {} bpm", e.bpm) } else { String::new() }
            );
            dim_line(ui, &head, 2.0, pal().ink);
            if !e.note.is_empty() {
                para(ui, &e.note, pal().ink2);
            }
        }
        match pick {
            Some((i, true)) => {
                self.store.entries.remove(i);
                self.entry_edit = None;
                self.show_form = false;
                self.store_dirty = true;
            }
            Some((i, false)) => {
                let e = self.store.entries[i].clone();
                self.f_tune = e.tune;
                self.f_mins = if e.mins > 0 { e.mins.to_string() } else { String::new() };
                self.f_bpm = if e.bpm > 0 { e.bpm.to_string() } else { String::new() };
                self.f_note = e.note;
                self.entry_edit = Some(i);
                self.show_form = true;
            }
            None => {}
        }
        if !any && self.store.entries.is_empty() {
            para(ui, "Practice time is tracked automatically while you work on a loop in PRACTICE. To write down what you worked on and the tempo you reached, pick a day above and press LOG PRACTICE.", pal().ink2);
        }
        ui.add_space(20.0);
    }
}
